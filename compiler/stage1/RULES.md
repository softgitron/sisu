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
stage 0 (`types`, `sir`, `ownership`, `sir-tests`, `contracts-off`,
`contracts-preconditions`, `human`, `cases-check`, `cases-human`, the
`corpus-` classes, `fmt`, `fmt-check`, `doc`, `doc-output`) differ too,
for the programs the rule touches. `check.sh` accepts those differences
as it accepts DISCREPANCIES.md's: under each rule, a line ``- Differs:
`path`: outputs`` per case it changes, the path from the repository root
and the outputs among `tokens`, `tree`, `trivia-tree`, `ast`, `names`,
`types`, `sir`, `ownership`, `sir-tests`, `contracts-off`,
`contracts-preconditions`, `ids` (all of what `sisu check --format=ids`
prints), `human` (what `sisu check` prints), `fmt`, `fmt-check`, `doc`
and `doc-output`, `ws` (a worksheet as `sisu ws` shows it, R79),
`behaviour` for a runnable case a rule refuses (R21 on), which
`check.sh` then does not run, and, for stage 1's own package, `self` for
a module's SIR (the path of its file) and `fixed-sir0` for the fixed
point's comparisons with stage 0 (the path `compiler/stage1`, R124), and
`build` for the executable `check.sh`'s class `build` makes of a case
(R124). Any other difference fails the
check. What a program
does is checked against the conformance suite's expectations instead, by
the class `behaviour`, the primary oracle: every runnable program of the
corpus and of stage 1's cases, a rule's own among them, lowered by stage
1 and run from its SIR by the interpreter and natively, must print its
`.out` and `.err` and exit with its `.status`. Where no rule changed,
stage 0's SIR stays an oracle too, through the classes `sir` and
`corpus-sir`.

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

## R2. Unsafe code: the three unsafe operations and the markers that oblige (decision 271)

**The rule** (proposal/22-unsafe.md, "The decision" 1 to 3 and "Syntax";
D-080). An unsafe operation is permitted only inside an `unsafe:` block
or an `unsafe e` expression, and there are three: calling an `unsafe def`
(a function, a method, a trait's method), calling a foreign function (one
an `extern` block declares), and writing an `unsafe impl`.

- `unsafe def` marks a function whose callers take an obligation; its body
  is ordinary code, in which unsafe operations still need a block. It
  combines with `pure` (`pure unsafe def`). A call outside a block or
  expression is `E0981` (`unsafe.call`), labelled with the obligations the
  callee's `Safety:` list names and the help to say why they hold in a
  `# safety:` comment; a foreign function's is `E0982`. Neither may be a
  function value (`E0983`), since a call through the value would need no
  `unsafe`; a closure that calls it inside an `unsafe` expression can.
- `unsafe trait` marks a trait whose implementors keep a promise; `Send`
  and `Sync` are unsafe traits too. Its impl is written `unsafe impl`
  (`E0984`, with the fix that writes `unsafe `); `unsafe impl` of any other
  trait, or of no trait, promises nothing (`E0985`, with the fix that
  removes it), and `Copy` and `Escapable` are never asserted (`E0986`). An
  impl's method is `unsafe def` exactly when the trait's is (`E0234`, the
  signatures written with `unsafe`).
- An `extern "C":` block declares foreign functions: each is a function of
  the module, with no body, whose call lowers to the intrinsic
  `extern.c.NAME` (the library is the block's ABI string in lowercase),
  as stage 0's `sim.rs` has foreign calls marked; it is no longer
  `E0900`.
- The attributes only the standard library writes (`@intrinsic`,
  `@builtin`, `@structural`, `@keeps`, `@sealed`, `@marker`, `@internal`)
  are `E0225` (`resolve.std-only-attribute`) in the program's own modules,
  on a method as on an item: a program's function with an intrinsic body
  (`@intrinsic("mem.ptr.read")`) would make one of `std.mem`'s unsafe
  operations a safe call. Stage 0 checks an item's attributes and no
  method's, and stage 1 had not transcribed the check; one that the
  compiler writes (the `@internal` of an invariant's check) is not in the
  text and is not checked.
- Everything else is unchanged by `unsafe`: an `unsafe:` block is a block
  (its locals end with it), `unsafe e` is the expression it marks, and
  lowering passes both through, so the SIR of unsafe code is the SIR of
  the calls inside it; names inside both are resolved (stage 0 resolved
  none).

**Why.** 22 and D-080 decide it: the block is the only permission, and
the markers that oblige permit nothing, which Rust needed an edition to
reach; the operations are library functions, so the language gains no
dereference, address-of or pointer arithmetic. D-089 makes stage 1 the
front end, where the rule is implemented; stage 0 parsed `unsafe:` and
`unsafe e` and reported `E0900`, and parses neither `unsafe def`, `unsafe
trait` nor `unsafe impl`.

**In stage 1.** The parser (`parser.items`: `unsafe` before `def`,
`trait` and `impl`, after `pure`), the AST (`FnDef`, `TraitDef` and
`ImplDef.is_unsafe`, written by the dump only when set; `StmtKind.Unsafe`,
`ExprKind.Unsafe`, `TypeKind.PtrMut`), the name pass (`resolve`: the
blocks, expressions and `extern` declarations), the formatter (unchanged:
its rules are the tree's) and `sisu doc` (unchanged: headers as written).
The checker (`typeck.safety`): `Checker.enter_unsafe` and `exit_unsafe`
around a block or an expression, `unsafe_call` from `call_fn_with` (every
call of a function, a method or a trait's method), `unsafe_fn_value`;
`typeck.collect`: `FnInfo.unsafe_`, `foreign` and `obligations` (read from
the `Safety:` list above the declaration, `typeck.safety.safety_list`),
`TraitInfo.unsafe_`, `ImplInfo.unsafe_`, `check_unsafe_impl`,
`check_unsafe_method`, and the `extern` block's declarations
(`foreign_fns`, `extern_library`).

**What it changes.** No program of the corpus but those that write
`unsafe`: the conformance suite's `packages/trust.pkg` (its `unsafe:`
blocks are blocks now, so their `return` makes the line after them
unreachable, `W0004`, and the rules of R3 and R6 apply), the proposal's
`examples/ffi.sisu` (its foreign declarations and raw pointers are
checked now: `CStr.from`, `ptr.null` and `ptr.addr_of` are not the names
22 gives, which are `CString.new`, `ptr[T]?`'s `None` and `ptr.of_mut`), and stage
1's own cases that showed `unsafe` as a later phase's (`later_phases`,
`statement_mistakes`, `expected_block_labels`). The cases of
`cases/unsafe/` show it: `operations.sisu` runs (its `.out`, interpreted
and natively from stage 1's SIR), `refusals.sisu` has each refusal, and
`reserved_attributes.sisu` the attributes a program may not write (stage
0 reports the items', stage 1 the methods' too); every case of
`cases/unsafe/` has its `.diag`, which the class `expected` holds stage
1's `check --format=ids` to.

- Differs: `compiler/conformance/cases/packages/trust.pkg/app`: types, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/packages/trust.pkg/fastmath/src/lib.sisu`: ast
- Differs: `compiler/conformance/cases/packages/trust.pkg/rawio/src/lib.sisu`: ast
- Differs: `proposal/examples/ffi.sisu`: ast, names, types, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/later_phases.sisu`: ast, names, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/statement_mistakes.sisu`: ast, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/expected_block_labels.sisu`: ast, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/unsafe/operations.sisu`: tree, trivia-tree, ast, names, ids, human, doc, doc-output, fmt-check
- Differs: `compiler/stage1/cases/unsafe/refusals.sisu`: tree, trivia-tree, ast, names, ids, human, doc, doc-output, fmt-check
- Differs: `compiler/stage1/cases/unsafe/reserved_attributes.sisu`: ids, human, doc, doc-output

**Documents to bring along** (for the coordinator): the conformance
suite's `packages/trust.diag`, which stage 1 now answers as
`compiler/stage1/RULES.md`'s R2, R3 and R6 decide (stage 1's output of
`sisu check --format=ids` there is the expectation to adopt); 02's
`unsafe` syntax (`unsafe def`, `unsafe trait`, `unsafe impl`); 04's
"Unsafe" section, which 22 replaces; docs/architecture.md, decision 271.

**Decision 271**, for docs/architecture.md:

271. Unsafe code (22; D-080) is implemented in stage 1, the front end:
     an unsafe operation, which is a call of an `unsafe def` (a function,
     a method or a trait's method), a call of a foreign function, or an
     `unsafe impl`, is permitted only inside an `unsafe:` block or an
     `unsafe e` expression. A call outside one is `E0981` (`E0982` for a
     foreign function), labelled with the obligations of the callee's
     `Safety:` list; neither kind of function may be a function value
     (`E0983`). `Send`, `Sync` and the traits declared `unsafe trait` are
     implemented with `unsafe impl` (`E0984`, with the fix that writes
     it); `unsafe impl` of another trait or of none is `E0985`, of `Copy`
     or `Escapable` `E0986`; an impl's method is `unsafe def` exactly when
     the trait's is (`E0234`). An `unsafe def`'s body is no unsafe
     context. An `extern` block declares foreign functions, whose calls
     lower to the intrinsic `extern.LIBRARY.NAME` (`extern.c.NAME` for
     `extern "C"`), which marks them in SIR as `sim.rs` reads them; the
     blocks and expressions lower as the blocks and expressions they mark,
     so SIR is unchanged by `unsafe` and the interpreter and the backend
     run unsafe code whose callees' bodies they know. `@intrinsic` and the
     other attributes only the standard library writes are `E0225` on a
     program's methods as on its items, since a function with an
     intrinsic body would be an unsafe operation that needs no `unsafe`.
     Stage 1 decides it (its RULES.md, R2, with `cases/unsafe/`); the
     frozen Rust front end parses `unsafe:` and `unsafe e` and reports
     them as `E0900`.

## R3. Safety comments and the lints of unsafe code (decision 272)

**The rule** (22, "Syntax" and "The audit and the lints"; D-083). A
`# safety:` line comment, continued on the comment lines under it,
stands immediately above the line where the statement holding an
`unsafe:` block or an `unsafe` expression starts, and above an `unsafe
impl` and an `extern` block (their attributes included); its parts are
named after the obligations they discharge (`valid, aligned: ...; pure:
...`, a name or names and a `:` at its start or after a `;` or a
sentence's `.`). An `unsafe def` and an `unsafe trait` name their
obligations in a `Safety:` list of their `##` comment, one ``- `name`:``
a line; the `unsafe def` of a trait's impl has the trait method's
obligations when it lists none of its own. The lints, each a warning:

- `W0009` (`unsafe.unused`): a block or expression with no unsafe
  operation, with the fix that removes the expression's `unsafe` (one with
  a call that did not resolve is not reported);
- `W0007` (`unsafe.no-safety-comment`): a block, an expression, an
  `unsafe impl` or an `extern` block without its `# safety:` comment;
- `W0008` (`unsafe.no-safety-list`): an `unsafe def` or `unsafe trait`
  without its `Safety:` list (not a trait impl's method, whose list is the
  trait's);
- `W0010` (`unsafe.unnamed-obligation`): a comment that names none of a
  callee's obligations (Q-72), once per callee;
- `W0013` (`unsafe.auto-trait-bound`): an `unsafe impl Send` or `Sync` of a
  generic type whose parameter lacks the matching bound (Rudra's check),
  with the fix `type.add-bound`;
- `W0014` (`unsafe.ambient-authority`): a public safe function, not `pure`,
  that calls a foreign function and takes no capability (a parameter of a
  capability type, a `Cap`, a `Lent`, a `dyn`, or what holds one), with
  the note that names clause 5.

`W0011` (R5) and `W0012` (R6) are the lints of `pure` and of a package's
level. The lints are the program's own modules', not the standard
library's (which `sisu1 audit --unsafe --std` shows) nor those the
compiler writes. `sisu publish`, which would turn `W0007` and `W0008` into
errors, is not stage 1's.

**Why.** 22 and D-083: the block is the unit the audit lists, with its
reason; the Linux kernel's rule and clippy's lints show that a comment per
block and a section per unsafe function are kept when a lint asks for
them, and Rudra's third bug pattern is the manual `Send` impl without its
bound.

**In stage 1.** `typeck.safety`: `safety_comment`, `named_parts`,
`doc_above`, `safety_list`, `missing_comment`, `Checker.unsafe_lints` and
`ambient_authority` (run after each body, `typeck.check_one` and
`check_value`); `typeck.collect`: `W0007` for `unsafe impl` and `extern`
blocks, `W0008`, `W0013`, and `check_unsafe_method`, which gives a trait
impl's `unsafe def` the trait method's obligations.

**What it changes.** No program of the corpus but those of R2. The cases:
`cases/unsafe/lints.sisu` (each lint once; it runs, its `.out`), and
`foreign_strings.sisu`, whose allocator's `release` takes `Allocator`'s
list.

- Differs: `compiler/stage1/cases/unsafe/lints.sisu`: tree, trivia-tree, ast, ids, human, doc, doc-output, fmt-check

**Decision 272**, for docs/architecture.md:

272. The `# safety:` comment of unsafe code (22, D-083) is the run of
     comment lines immediately above the line where the statement holding
     an `unsafe:` block or an `unsafe e` expression starts, or the
     `unsafe impl` or the `extern` block (its attributes included), from
     its line that starts with `# safety:`; its parts name the obligations
     they discharge, names and a `:` at its start or after a `;` or a
     sentence's `.`. An `unsafe def`'s and an `unsafe trait`'s obligations
     are the ``- `name`:`` items of the `Safety:` paragraph of their `##`
     comment, and a trait impl's `unsafe def` without one has the trait
     method's. Stage 1 reports, as warnings of the program's own modules: a
     block or expression with no unsafe operation (`W0009`, with the fix
     that removes an expression's `unsafe`); a block, an expression, an
     `unsafe impl` or an `extern` block without its comment (`W0007`); an
     `unsafe def` or `unsafe trait` without its list (`W0008`); a comment
     naming no obligation of a callee (`W0010`); an `unsafe impl Send` or
     `Sync` whose parameter lacks the bound (`W0013`, with `type.add-bound`);
     and a public safe function, not `pure`, that calls a foreign function
     and takes no capability (`W0014`). Stage 1 decides it (RULES.md, R3).

## R4. Raw pointers, `std.mem` and `std.ffi`: modules of the standard library only stage 1 reads (decision 273)

**The rule** (22, "Raw pointers", "Auto traits", "Plain data and
reinterpretation", "The standard library's layer"; D-081). `ptr[T]` and
`ptr mut T` are `std.mem`'s raw pointers, which a module imports as any
other name (`from std.mem import ptr`); `ptr mut T` is the mutable form of
the `ptr` in scope (`std.mem.ptr_mut`), a name the name pass reports
without the import (with the help that names it) and `E0987` when `ptr`
names another type. A raw pointer is `Copy`, `Escapable` exactly when
`T` is (it holds its argument), neither `Send` nor `Sync`, and prints as
`ptr` without its address. An `unsafe impl Send` or `Sync` asserts the
trait for the arguments its bounds admit, which the auto-trait solver
reads before it walks the type's fields (`Program.asserted_auto`).
`std.mem` declares the raw pointers and their `unsafe def`s with their
`Safety:` lists (`copy_from` copies slots of any `T` as bits, a value or
none each, with the obligation `moved` for a `T` that is not `Copy`), the
safe `as_ptr` of a slice, `Layout`, `size_of`, `align_of`, `align_up`, the
`Allocator` unsafe trait, `Global`, the bump `Region`, `RawBuf[T, A]`
(`Send` and `Sync` through its `unsafe impl`s, as 22 shows; `grow` is
safe, since its slots move to new memory as bits, each keeping what it
holds), `Plain` with its derive (a `@repr(c)` or one-field struct whose
fields are plain; `bit_cast`, `as_bytes`, `read_plain` and `view_plain`
safe for plain types; `transmute` unsafe), and `slice_from_raw`.
`RawBuf.with_capacity`, `grow` and its `Drop`, and `Region.bump` and its
`Drop`, are `pure`, so that `pure` code makes, grows and drops what is
built on them (22: "a `pure` function can push to a `list` that std builds
on `RawBuf`"; a `Drop` that is not `pure` is `E0393` where `pure` code
drops its value). `std.ffi` declares the C types, `CString` (text with a
NUL after it, refused by `new` when the text holds one, and the pointer a
foreign function reads) and `CStr` (a view of a NUL-terminated string that
foreign code owns, made by the `unsafe def` `from_ptr`, its bytes copied
out by `to_bytes` and `to_text`). The raw memory operations are `mem.*`
intrinsics, which stage 0 does not run yet.

Since the frozen Rust front end parses neither `unsafe def`, `unsafe
trait` nor `unsafe impl`, and formats and reads every file of
`compiler/std/`, these modules live in `compiler/stage1/std/` (`mem.sisu`,
`ffi.sisu`, `rt.sisu`), which stage 1 reads as the standard library's
(`typeck.std_sources.std_file`), until the Rust front end is retired; a
derive template may come from such a module (`DeriveSource.Std`), and a
derive's target shows reflection its attributes (`Target.attrs`).

**Why.** D-081 and 22's table: the raw pointer types move from `std.ffi`
to `std.mem`, never null, with strict provenance, and not `Send` or
`Sync` so that inference never makes a type holding one thread-safe;
`RawBuf` asserts both with the right bounds so that containers built on it
need no `unsafe impl` of their own. The brief keeps `compiler/std/` the
Rust front end's to read and format.

**In stage 1.** `typeck.program`: `ptr_mut_of`, `asserted_auto`, the
builtins `ptr` and `ptr_mut` in `auto_depth` and `is_copy_builtin`, their
display; `typeck.check` and `typeck.collect`: `TypeKind.PtrMut`;
`typeck.instances`: `DeriveSource.Std`, `is_plain_trait` (the derive of
`Plain` asks each field for `Plain`, `E0244`); `typeck.template`:
`Target.attrs`; `typeck.std_sources`: `stage1_only`, `std_file`, `mem` and
`ffi` among the modules `sisu doc --std` documents.

**What it changes.** `sisu doc --std` documents `std.ffi` and `std.mem`
after the others. The cases: `cases/unsafe/raw_pointers.sisu`,
`pointer_names.sisu`, `unimported_pointer.sisu`, `plain_data.sisu` (it
runs: the derive of `Plain` and the layouts, its `.out`) and
`plain_refusals.sisu`, and `foreign_strings.sisu` (a `CString` read by
a foreign function, a `CStr` viewing one, a `RawBuf` that a `pure`
function makes, grows and drops, an allocator of the program's own, a
slice's pointer; checked, not run).

- Differs: `--std`: doc, doc-output
- Differs: `compiler/stage1/std/mem.sisu`: tree, trivia-tree, ast, fmt-check
- Differs: `compiler/stage1/std/ffi.sisu`: tree, trivia-tree, ast, fmt-check
- Differs: `compiler/stage1/std/rt.sisu`: tree, trivia-tree, ast, fmt-check
- Differs: `compiler/stage1/cases/unsafe/raw_pointers.sisu`: tree, trivia-tree, ast, names, ids, human, doc, doc-output, fmt-check
- Differs: `compiler/stage1/cases/unsafe/pointer_names.sisu`: ast, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/unsafe/unimported_pointer.sisu`: ast, names, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/unsafe/plain_data.sisu`: names, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/unsafe/plain_refusals.sisu`: names, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/unsafe/foreign_strings.sisu`: tree, trivia-tree, ast, names, ids, human, doc, doc-output, fmt-check

The conformance suite's `cases/unsafe/` (stage 0's checked memory model,
decisions 301 to 304) imports `std.mem` or `std.ffi` in every program,
which stage 0's front end does not read, so each output that needs their
names differs from it; `foreign.sisu` and `pointers.sisu` also declare an
`unsafe def`, which stage 0 does not parse (R2). What they print is stage
1's SIR, run by stage 0 (`behaviour`).

- Differs: `compiler/conformance/cases/unsafe/bit_cast_sizes.sisu`: names, types, sir, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/unsafe/foreign.sisu`: tree, trivia-tree, ast, names, sir, ids, human, fmt-check, doc, doc-output
- Differs: `compiler/conformance/cases/unsafe/foreign_unknown.sisu`: ast, names, sir, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/unsafe/free_twice.sisu`: ast, names, sir, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/unsafe/gone.sisu`: ast, names, sir, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/unsafe/invalid_bits.sisu`: ast, names, sir, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/unsafe/leaked.sisu`: ast, names, sir, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/unsafe/layouts.sisu`: names, types, sir, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/unsafe/malloc_bounds.sisu`: ast, names, sir, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/unsafe/misaligned.sisu`: ast, names, sir, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/unsafe/mistyped.sisu`: ast, names, sir, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/unsafe/no_memory.sisu`: ast, names, sir, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/unsafe/out_of_bounds.sisu`: ast, names, sir, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/unsafe/overwrite.sisu`: ast, names, sir, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/unsafe/plain_casts.sisu`: ast, names, types, sir, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/unsafe/raw_buffer.sisu`: ast, names, sir, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/unsafe/pointers.sisu`: tree, trivia-tree, ast, names, sir, ids, human, fmt-check, doc, doc-output
- Differs: `compiler/conformance/cases/unsafe/read_plain_short.sisu`: names, types, sir, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/unsafe/reference_behind.sisu`: ast, names, sir, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/unsafe/release_layout.sisu`: ast, names, sir, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/unsafe/release_other.sisu`: ast, names, sir, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/unsafe/released.sisu`: ast, names, sir, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/unsafe/released_twice.sisu`: ast, names, sir, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/unsafe/shared_write.sisu`: ast, names, sir, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/unsafe/stale_pointer.sisu`: ast, names, sir, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/unsafe/uninitialised.sisu`: ast, names, sir, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/unsafe/unterminated.sisu`: ast, names, sir, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/unsafe/vector.sisu`: ast, names, sir, ids, human, doc, doc-output

**Documents to bring along:** 11's `std.mem` and `std.ffi` rows and 12's
C types, which D-081 amends; when the Rust front end is retired,
`compiler/stage1/std/` moves into `compiler/std/` and `std_file` goes.

**Decision 273**, for docs/architecture.md:

273. Raw pointers are `std.mem`'s (22, D-081): `ptr[T]` and `ptr mut T`,
     builtins `ptr` and `ptr_mut` that a module imports (`from std.mem
     import ptr`; `ptr mut T` is the mutable form of the `ptr` in scope,
     `E0987` when that is another type), `Copy`, `Escapable` when `T` is,
     never `Send` or `Sync`, printed without their address. An `unsafe
     impl Send` or `Sync` asserts the trait for the arguments its bounds
     admit. `std.mem` declares the pointers' `unsafe def`s (intrinsics
     `mem.*`), `Layout`, `size_of`, `align_of`, `align_up`, `Allocator`,
     `Global`, `Region`, `RawBuf[T, A]` (whose `grow` is safe, and whose
     making, growing and dropping are `pure`, as `Region`'s are) and
     `Plain`, whose derive asks `@repr(c)` (or one field) of a struct and
     `Plain` of each field; `std.ffi` the C types, `CString` and `CStr`.
     The modules whose syntax only stage 1 reads
     (`std.mem`, `std.ffi`, `std.rt`) are in `compiler/stage1/std/` until
     the frozen Rust front end, which reads and formats `compiler/std/`,
     is retired; a derive template may come from one, and a derive's
     target shows reflection its attributes. Stage 1 decides it (RULES.md,
     R4).

## R5. Unsafe code, `pure`, compile time and contracts (decision 274)

**The rule** (22, "Interaction with the rest of the language"; D-085).
An `unsafe` block does not make a function impure, and inside a `pure`
function a call of a function that is not `pure`, made inside an
`unsafe` block or expression, is accepted as its author's assertion that
the function stays observationally pure: the ownership checker's purity
check does not report it (`E0333` otherwise), and the block's comment
says why under a `pure:` part (`W0011`, `unsafe.unexplained-purity`,
otherwise). Observing an address is not `pure` (`ptr.addr`). A foreign
call and a machine intrinsic stop compile-time evaluation (`E0601`,
`comptime.io`, naming the foreign function or the intrinsic). Code that
compile-time evaluation writes (a derive's impls, a template's instance,
an unfolded value) holds no `unsafe` (`E0615`,
`comptime.unsafe-written`). A contract holds no `unsafe` block or
expression (`E0988`, `unsafe.in-contract`).

**Why.** D-085: without the assertion every function over a collection
written in Sisu on raw memory would be impure, and an allocator's `pure`
methods over `malloc` could not exist; the audit lists the assertion.
Comptime runs in a world that the foreign function is outside of, and
every unsafe block stays in a source file where the audit shows it.
Contracts can be compiled out (07), so unsafe code relies on none.

**In stage 1.** `typeck.safety`: `Checker.impure_call` (from
`call_fn_with`) records the call in the innermost region,
`asserted_regions` gives the ranges, which `TBody.asserted` carries to
lowering; `lower.func.lower_body` turns them into SIR locations
(`Ctx.asserted`), which `sir.Program.asserted` (not written) carries to
`borrowck.purity`, whose finding inside one goes (`Program.asserted_at`).
`interp.intrinsic` stops at `extern.*` and `rt.*`; `typeck.safety.
enter_unsafe` and `typeck.collect` report `E0615`;
`typeck.contracts.contract_clause` reports `E0988`. Comptime does not run
raw memory operations yet: the checked memory model of D-085 is the
interpreter's to build (the stage-0 round's), and stage 1's `interp` has
none, so a `comptime` that reaches a `mem.*` intrinsic is beyond the
slice of stage 1's lowering, as any intrinsic it lacks.

**What it changes.** No program of the corpus. The cases:
`cases/unsafe/operations.sisu` (a `pure` function asserting purity, run),
`address_purity.sisu`, `comptime_foreign.sisu`, `comptime_written.sisu`,
and `refusals.sisu` (`E0988`).

- Differs: `compiler/stage1/cases/unsafe/address_purity.sisu`: ast, names, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/unsafe/comptime_foreign.sisu`: ast, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/unsafe/comptime_written.sisu`: ast, ids, human, doc, doc-output

**Documents to bring along:** 02, 06 and 08, whose "a `pure` function
contains no `unsafe`" and comptime's "no `unsafe`" 22 amends.

**Decision 274**, for docs/architecture.md:

274. An `unsafe` block does not make a function impure (22, D-085):
     inside a `pure` function, a call of a function that is not `pure`
     made in an `unsafe` block or expression is its author's assertion
     that the function stays observationally pure, which the block's
     comment explains under a `pure:` part (`W0011` otherwise); the
     ranges of such blocks go from the typed body (`TBody.asserted`)
     through lowering into `sir.Program.asserted`, which SIR's text does
     not write, and the purity check reports no call inside one. A
     foreign call and a machine intrinsic stop compile-time evaluation
     (`E0601`); code that compile-time evaluation writes holds no
     `unsafe` (`E0615`); a contract holds none (`E0988`). Stage 1 decides
     it (RULES.md, R5).

## R6. The level a package declares, the root's consent, and the lockfile (decision 275)

**The rule** (22, "Packages: the manifest field"; D-082). A package's
manifest says `unsafe = "memory"` (raw memory: every unsafe operation but
foreign code) or `unsafe = "extern"` (foreign code too: an `extern` block
or a C header import); another value is `E1003`. A package with unsafe
code and no declaration is `E1019` (`package.unsafe-undeclared`), at its
first such code, naming the level it needs; foreign code in a package that
declares `memory` is `E1020`; a declaration above what the code uses is
`W0012` (`unsafe.level`), at the declaration; a dependency that declares a
level is `E1021` (`package.unsafe-untrusted`) unless the root's `[trust]
unsafe` lists it, with the help that writes the entry (its code is still
`cap.untrusted`, `E0703`, as before). The root package and every member
declare their own level; a program of one file has no manifest to declare
one in. The lockfile records each package's level (`unsafe = "memory"`
in its `[[package]]`), and a command that would rewrite the lockfile with
a package's level above what it records (none to `memory`, `memory` to
`extern`) stops with `E1022` (`package.unsafe-raised`) unless given
`--accept-unsafe` (`check`, `sir`, `build`, `run`, `doc`).

**Why.** D-082 (decided by the owner): consent per program, a declaration
per package so that the dependent sees it before installing, and an
update that raises the level is part of the authority delta that
admission prints; unattended runs never pass `--accept-unsafe`. Stage 1
has no `sisu update`, so the refusal is where it writes the lockfile.

**In stage 1.** `manifest`: `PackageInfo.unsafe_level`; `package`:
`Package.unsafe_level`, `lockfile_of`, `locked_levels`, `level_rank`;
`main`: `unsafe_levels` (from each package's tokens: `unsafe`, `extern`,
`import c "..."`), `raised_level`, `write_lockfile`'s `accept_unsafe`.
A foreign call to a function another package declares is not counted
toward the caller's level (open: it needs the checker's view of calls).

**What it changes.** The cases: `cases/packages/unsafe_levels.pkg` (each
diagnostic) and `unsafe_trusted.pkg` (declared and trusted; it runs, its
`.out`); the tests of `manifest` and `package` read the level and write
and read the lockfile's; `E1022` was checked by hand (a lockfile that
records `memory` and a manifest that declares `extern`). The conformance
suite's `testing/unsafe_level.pkg` declares `memory` for an `unsafe:`
block over `std.mem`'s pointers, which the Rust front end reads as none
of these (`W0005` for the key, `E0220` for the module, `E0900` for the
block), so its names, SIR, diagnostics and pages are stage 1's alone.

- Differs: `compiler/conformance/cases/testing/unsafe_level.pkg`: names, sir, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/testing/unsafe_level.pkg/src/main.sisu`: ast
- Differs: `compiler/stage1/cases/packages/unsafe_levels.pkg/app`: names, types, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/packages/unsafe_levels.pkg/app/src/main.sisu`: ast
- Differs: `compiler/stage1/cases/packages/unsafe_levels.pkg/cbind/src/lib.sisu`: ast
- Differs: `compiler/stage1/cases/packages/unsafe_levels.pkg/rawmem/src/lib.sisu`: tree, trivia-tree, ast, fmt-check
- Differs: `compiler/stage1/cases/packages/unsafe_levels.pkg/sneaky/src/lib.sisu`: tree, trivia-tree, ast, fmt-check
- Differs: `compiler/stage1/cases/packages/unsafe_trusted.pkg/app`: names, types, sir, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/packages/unsafe_trusted.pkg/app/src/main.sisu`: ast
- Differs: `compiler/stage1/cases/packages/unsafe_trusted.pkg/digits/src/lib.sisu`: tree, trivia-tree, ast, fmt-check

**Documents to bring along:** 09's manifest and `[trust]`, which D-082
amends; the lockfile's format in 09.

**Decision 275**, for docs/architecture.md:

275. A package declares its unsafe code in its manifest, `unsafe =
     "memory"` or `"extern"` (22, D-082), which stage 1 compares with the
     package's code (its `unsafe` and `extern` tokens and C header
     imports): code without a declaration is `E1019`, foreign code under
     `memory` `E1020`, a declaration above the code `W0012`; a dependency
     that declares a level needs the root's `[trust] unsafe` (`E1021`). The
     lockfile records each package's level, and a rewrite that would
     raise one stops with `E1022` unless the command has
     `--accept-unsafe`. Stage 1 decides it (RULES.md, R6).

## R7. `sisu1 audit --unsafe` (decision 276)

**The rule** (22, "The audit and the lints"; D-083). `audit --unsafe
[PATH]` lists, per package with its declared level and how the program
consents to it (the root package; `trusted in sisu.toml:LINE`, the line of
the root's `[trust]` entry; not trusted), every module that contains
unsafe code with its file, its counts and the hash of its text (64-bit
FNV-1a, the key of the review ledger), and in it every `unsafe:` block and
`unsafe` expression with the function it is in, every `unsafe def` and
`unsafe trait` with its `Safety:` list, every `unsafe impl` and `extern`
block with its comment, by line; then the package's foreign functions,
each with its callers and what each holds (a capability parameter, the
impl of a capability trait, the purity it asserts, or nothing); then the
lints and the rules of unsafe code that `check` reports, as `--format=ids`
prints them; last, the line that names `audit --unsafe --std`, which lists
the standard library's own unsafe code.

**In stage 1.** `audit` (`unsafe_report`, `module_hash`), `main.
audit_command`, the command `audit` of `sisu1`. The class `expected` of
`check.sh` holds the audit of a case with a `NAME.audit` to that file:
`cases/unsafe/operations.audit` and `cases/packages/unsafe_levels.audit`;
`audit`'s tests check its listing and its hash.

**Decision 276**, for docs/architecture.md:

276. `sisu audit --unsafe` (22, D-083) is stage 1's: per package, its
     declared level and the program's consent; per module with unsafe
     code, its counts and the FNV-1a hash of its text; per block,
     expression, `unsafe def`, `unsafe trait`, `unsafe impl` and `extern`
     block, its line, its function and its comment or `Safety:` list; the
     package's foreign functions with what their callers hold; and the
     lints, as `check --format=ids` prints them. `--std` lists the
     standard library's. Stage 1 decides it (RULES.md, R7).

## R8. `std.rt`, the runtime's machine layer (decision 277)

**The rule** (22, "Machine intrinsics"; D-086). `std.rt`, internal to
the standard library (a program's import of it is `E0701`,
`cap.internal`), declares the three machine intrinsics as `unsafe def`s
with their `Safety:` lists: `switch(save: ptr mut Context, load:
ptr[Context])` (`rt.switch`), `stack_pointer() -> int`
(`rt.stack_pointer`) and `thread_slot() -> ptr mut int`
(`rt.thread_slot`), with the `@repr(c)` struct `Context`. None runs today.

**What it changes.** The case `cases/unsafe/runtime_internal.sisu`.

- Differs: `compiler/stage1/cases/unsafe/runtime_internal.sisu`: ast, names, ids, human, doc, doc-output

**Decision 277**, for docs/architecture.md:

277. `std.rt` (22, D-086), internal to `std` (`E0701` for a program's
     import), declares the machine intrinsics `rt.switch`,
     `rt.stack_pointer` and `rt.thread_slot` as `unsafe def`s with their
     `Safety:` lists, and `Context`; compile-time evaluation stops at each.
     Stage 1 decides it (RULES.md, R8).

## R9. The runtime's first slice in Sisu: values by their parts (decision 278)

**The rule** (22, "How the runtime in Sisu uses it"; D-086). `std.mem`
holds the operations that `sisu-rt` performs by walking a type table
(`value.rs`, `dict.rs`) as generic code that compile-time reflection
takes apart per type, the first slice of the runtime in Sisu, which needs
no machine:

- `structural_hash[T](value, mut h: Hasher)` mixes the value's parts into
  the hasher in the order of decision 256, each as `Hasher.write_int`
  mixes an `int`: an `int` or a sized integer of up to 64 bits as itself
  (a `u64` as its word), a `char` as its code, a `bool` as 0 or 1, an
  `f64` as its bits (`f64_bits`, worked out from the value by scaling with
  two, which is exact; a NaN as the quiet NaN) and an `f32` as its bits; a
  text's or `bytes`' length, then each byte; a list's length, then its
  values; an option's variant (`Some` 0, `None` 1, as the prelude declares
  them), then its value; a struct's fields in order; an enum's variant
  index, then its fields; `()` as nothing;
- `structural_eq[T](a, b)` compares them part by part, the leaves as `==`
  does;
- `structural_copy[T](value)` builds a struct again from copies of its
  fields (`reflect.construct`), a list of copies of its values and an
  option of a copy of its value, and copies a leaf (and an enum, which
  reflection builds no variant of) as `.copy()` does;
- `drop[T](own value)` drops the value at once: the compiler's drop glue
  in SIR does that, its own `Drop` first and then its parts, as 22's table
  says ("drop glue is SIR already"), so the runtime needs no structural
  drop of its own, and a plain value has nothing to drop.

Reflection takes no tuple or array apart in stage 1, so `structural_hash`
refuses them at compile time and the others treat them as leaves.

**Why.** D-086 orders the runtime's port, `std.mem` first; 22's table
replaces `value.rs`'s and `dict.rs`'s structural operations by generic
functions instantiated per type, which is also what lets an optimiser
inline them. They are pure Sisu and run today, so they are checked
against the interpreter's answers now, before the checked memory model
exists. They are in `std.mem`, which a program may import, since `std.rt`
is internal to `std` and a case could not call them there; none reaches
the machine.

**In stage 1.** The last section of `stage1/std/mem.sisu`; nothing in the
compiler, since they are templates as `json.encode` is.

**What it changes.** Nothing but the case `cases/runtime/structural.sisu`:
values of every kind above, each hashed by `structural_hash` and through
its derived or the prelude's `Hash` (whose leaves are the interpreter's
`structural.hash`, decision 256), compared by `structural_eq` and by `==`,
copied by `structural_copy` (equal to the original and independent of
it), floats' bits against IEEE 754's, and `drop`'s order against a
scope's end; its `.out` holds the interpreter's answers, and the native
executable from the same SIR prints the same (the class `behaviour`).

- Differs: `compiler/stage1/cases/runtime/structural.sisu`: names, ids, human, doc, doc-output

**Decision 278**, for docs/architecture.md:

278. The first slice of the runtime in Sisu (22, D-086) is `std.mem`'s
     structural operations, generic code that compile-time reflection
     takes apart per type in place of `sisu-rt`'s walk over a type table:
     `structural_hash` mixes a value's parts in the order of decision 256
     (a float's bits worked out from its value, `f64_bits`),
     `structural_eq` compares them, `structural_copy` copies them anew,
     and `drop` drops a value at once through the compiler's drop glue,
     which already is the structural drop; tuples and arrays wait for
     reflection to take them apart. They are in `std.mem` rather than in
     the internal `std.rt`, since none reaches the machine. Stage 1's case
     `cases/runtime/structural.sisu` checks them against the interpreter's
     and the native runtime's answers. Stage 1 decides it (RULES.md, R9).

## R10. A non-exhaustive `match` names every pattern it misses (decision 291)

**The rule** (03, "Algebraic data types": "Missing-case errors list the
missing patterns"). `E0314` (`type.non-exhaustive`) lists the patterns no
arm covers, each in backquotes, separated by commas, in the order of the
constructors their types declare, up to eight; past eight the note
`type.more-missing` says the eight are the first of more. In a part of the
value where some arm names a constructor, each constructor no arm names is
a pattern of its own, with `_` for its fields (`Some(Spades)`, `None`); in
a part where no arm names any constructor, the part is `_` (`(Clubs, _)`,
`[False, _, *_]`). One missing pattern keeps the note
`type.missing-patterns` ("add an arm for `Amber`, or a `case _` arm");
several have `type.missing-patterns-each` ("add an arm for each of ...").

**Why.** 03 says the error lists the missing patterns; the manual's writers
found that it named one at a time (chapter 7), so a beginner fixed a
`match` by one arm per compile. The usefulness algorithm of Maranget's
paper gives every witness, not only the first; reporting `_` for a part no
arm names keeps the list to what the arms actually miss, as rustc reports
it (`[false, _, ..]`), where listing every constructor of such a part would
repeat the whole type.

**In stage 1.** `typeck.exhaustive`: `Checker.witnesses` (every witness,
up to a limit, in place of the first one `witness` found),
`check_exhaustive`, `MISSING_SHOWN`.

**What it changes.** A `match` that misses one pattern is reported as
before, but where no arm names any constructor of a part: the conformance
suite's `types/list_exhaustiveness` names `[False, _, *_]` where stage 0
names `[False, True, *_]`, one of the two patterns `[False, _, *_]` covers.
A `match` that misses several names them all. The case
`cases/findings/missing_patterns.sisu` shows each form, and its `.diag`.

- Differs: `compiler/conformance/cases/types/list_exhaustiveness.sisu`: types, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/findings/missing_patterns.sisu`: ids, human, doc, doc-output

**Documents to bring along:** the conformance suite's
`types/list_exhaustiveness.diag`, whose second line becomes stage 1's
(`missing="`[False, _, *_]`"`).

**Decision 291**, for docs/architecture.md:

291. A non-exhaustive `match` (`E0314`) names every pattern no arm covers
     (03), up to eight, in the order of the constructors their types
     declare, with the note `type.more-missing` past eight: where some arm
     names a constructor of a part of the value, each constructor no arm
     names is a pattern of its own with `_` for its fields; where none
     does, the part is `_`. Stage 1 decides it (RULES.md, R10); the frozen
     Rust front end names the first missing pattern only.

## R11. `sisu lint`, and `case _` over an enum of the program's own package (decision 292)

**The rule** (D-043; 03, "Algebraic data types": "A `_` wildcard is
allowed but the linter warns when matching on an enum from the same
package, since a new variant should be a compile error there"; 10, "One
binary": `sisu lint`, "lints beyond the compiler's warnings, with
fix-its"). `lint [--format=ids] [FEATURES] [PATH]` checks the program as
`check` does and prints its lints, the warnings `check` does not print, in
the human form or the conformance form; a program with errors has its
errors printed as `check` prints them, and fails. Its one lint today is
`W0015` (`lint.wildcard-own-enum`): a reachable `case _` arm of a `match`
whose value is an enum declared in a module of the same package as the
`match` (not the standard library's, and not code the compiler writes),
naming the variants the wildcard matches (those the arms above it leave),
with the fix `lint.list-variants` that writes them in its place
(`Square(_) | Line | Dot`). A wildcard nested in a pattern, a binding
(`case other:`), and an arm the arms above make unreachable (`W0003`) are
not linted.

**Why.** D-043 and 03 give the warning to the linter, and 10 makes the
linter `sisu lint`, beyond the compiler's warnings: as a warning of `sisu
check` it would fire on every such wildcard of every program, stage 1's
own 700 among them, which the documents did not ask for. The fix is the
change the warning asks for, so that a variant added later is an error at
the `match`.

**In stage 1.** `typeck.exhaustive`: `Checker.lint_wildcard`, from
`check_exhaustive`; `typeck`: `Checked.lints`, the lints taken out of the
checked diagnostics (`is_lint`), so that nothing but `lint` sees them;
`main`: `Report.lints` and the command `lint`. `check.sh`'s class
`expected` holds `lint --format=ids` of a case with a `NAME.lint` to that
file.

**What it changes.** Nothing that `check`, `sir`, `build` or `doc`
print. The case `cases/findings/wildcard_lint.sisu` (its `.lint`, its empty
`.diag`, and its `.out`: it runs).

**Documents to bring along:** 10's `sisu lint` row (its first lint);
port-debt.md's list of the driver's commands, where `sisu lint` is stage
1's `lint` (the driver may route it as it routes `check`).

**Decision 292**, for docs/architecture.md:

292. `sisu lint` (10) is stage 1's `lint`: the program checked as `sisu
     check` checks it, and its lints printed in either form, the warnings
     the compiler's own do not include; a program with errors has them
     reported and fails. Its first lint is D-043's (03): `W0015`, a
     reachable `case _` over an enum declared in the same package as the
     `match`, naming the variants it matches, with the fix that lists them
     in its place. The checker reports a lint with its diagnostics, and
     `Checked.lints` takes it out of them. Stage 1 decides it (RULES.md,
     R11).

## R12. Naming is enforced (decision 293)

**The rule** (02, "Lexical structure": "Naming follows PEP 8 and is
enforced"). A declaration whose name is not written in its kind's case is
an error at the name, with the name written so as the fix
(`resolve.rename`):

- `E0267` (`resolve.not-camel-case`): a struct, an enum, an error set, a
  trait, a type alias, an associated type, a variant, or a type
  parameter (a `comptime def`'s `T: type` ones too) whose name holds `_`
  or begins with a lowercase letter (`point` is `Point`, `http_server`
  is `HttpServer`);
- `E0268` (`resolve.not-snake-case`): a function, a method, a field, a
  parameter (a closure's too) or a variable (a `let`'s, a loop's, a
  pattern's, a `with`'s, an `else as`'s) whose name holds an uppercase
  letter (`firstValue` is `first_value`, `parseHTTPRequest` is
  `parse_http_request`);
- `E0269` (`resolve.not-screaming-case`): a constant, or a `const`
  generic parameter, whose name holds a lowercase letter (`max_size` is
  `MAX_SIZE`).

A name's case is its letters' (a name in a script without case breaks
none); a capitalised name in a pattern is a constant, not a binding, and
`_` is no name. The foreign functions of an `extern` block keep the names
their library gives them, and so do their parameters. Modules were
already named so (`E1012`), and the standard library's builtin types
(`int`, `str`, `list`) and C types (`c_int`) are its own.

**Why.** 02 says the convention is enforced, and gives the reason that
makes it a rule rather than a style: since variants are always
capitalised, a lowercase name in a pattern is always a binding and a
capitalised one a constant. The manual's writers found `struct point:`
accepted without a word (chapter 7). The check is the name pass's, over
the syntax tree, as `E0202` is.

**In stage 1.** `naming` (`Style`, `problem`, `breaks`, `to_camel`,
`to_snake`); `resolve`: `check_style` and `check_item_style`, called for
items, members, variants, generic parameters, parameters and every local
(`declare_local`, with `naming_kind`), but in an `extern` block
(`foreign`).

**What it changes.** No program of the corpus but two of stage 1's own
unsafe cases: `cases/unsafe/pointer_names.sisu` declares `struct ptr[T]`
(a type of the module named `ptr`, whose point is that it is no raw
pointer), which is now also `E0267`, and `cases/unsafe/refusals.sisu`'s
alias `type c_int = i32` is written `CInt`. The case
`cases/findings/naming.sisu` has each kind of name.

- Differs: `compiler/stage1/cases/findings/naming.sisu`: names, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/unsafe/pointer_names.sisu`: names

**Decision 293**, for docs/architecture.md:

293. Naming is enforced (02): the name pass reports a declaration whose
     name is not in its kind's case, with the name written so as the fix:
     `E0267` for a type, a trait, an associated type, a variant or a type
     parameter not in `CamelCase` (a `_` in it, or a lowercase first
     letter), `E0268` for a function, a method, a field, a parameter or a
     variable not in `snake_case` (an uppercase letter in it), `E0269` for
     a constant or a `const` generic parameter not in `SCREAMING_CASE` (a
     lowercase letter in it). A capitalised name in a pattern is a
     constant and not checked; an `extern` block's names are its
     library's. Stage 1 decides it (RULES.md, R12).

## R13. A trait is no value: a variant of its name is (decision 294)

**The rule** (03, "Algebraic data types": a variant is used unqualified
"wherever the expected type is known"; docs/architecture.md, "Names and
the standard library"). In an expression, a name that names a trait, the
prelude's (`Add`, `Mul`, `Neg`, `Num`, `Eq`) or the module's, is not its
value: where a variant of that name is visible (the module's own first),
the name is the variant, called (`Add(left, right)`) or bare (`Neg`). Where
none is, the name is the trait, as before (`E0309`, `E0350`). A trait
qualifying a member (`Add.add(a, b)`) is unchanged.

**Why.** docs/architecture.md resolves a name in an expression as "a
variant of the expected type first, then a declaration of that name, then
a variant declared in the current module", so a variant named like a
prelude trait, called where no type is expected (`let e = Add(Num(1),
Num(2))`), resolved to the trait and was "a value of type `Add` cannot be
called" (the manual's chapter 7). A trait can be neither called nor used
as a value, so passing it by loses nothing.

**In stage 1.** `typeck.call.call_expr` and `typeck.expr.name_expr`: a
trait found by `lookup` yields to `global_variant`.

**What it changes.** No program of the corpus. The case
`cases/findings/variants_named_like_traits.sisu` (its `.out`: it runs).

- Differs: `compiler/stage1/cases/findings/variants_named_like_traits.sisu`: ids, human, doc, doc-output

**Decision 294**, for docs/architecture.md:

294. A trait is not a value: in an expression, a name that names a trait
     and a visible variant is the variant (the module's own first), called
     or bare, so `Add(left, right)` makes a variant named like the
     prelude's `Add` where no type is expected; with no such variant the
     name is the trait, `E0309` called and `E0350` bare. Stage 1 decides
     it (RULES.md, R13).

## R14. An optional chain in an interpolation reads (decision 295)

**The rule.** An interpolation only reads its value, as `{u.name}` reads
the field, so an optional chain of fields written there,
`{u.team?.name}`, reads what it reaches through a place rather than
moving it out of the place: over a place of an optional type, each link
reads its field in place (`{u.team?.lead?.name}` is `u.team?.(lead?.name)`
over `u.team`), and the last field's value, when it is not `Copy`, is read
too, a `String` as a `str` view and another type through a reference (an
optional field as an optional reference, `Option[ref T]`); what it prints
is what the moved value printed. A chain whose base is no place, and a
chain anywhere else (`let n = u.team?.name`), moves what it reaches as
before.

**Why.** The manual's writers found `{u.team?.name}` moving `name` out of
`u.team`, so that the next use of `u` was `E0400` or `E0401`, where
`{u.name}` only reads (chapter 8): the interpolation formats the value
and drops it, so the move bought nothing. A chain over a reference
already reads, as `x?.f` through a `ref` borrows `f`; the rule extends
that to a place an interpolation names.

**In stage 1.** `typeck.expr`: `string_literal` sets `reading_chain` for a
part that is an optional chain; `opt_chain` hands it to
`reading_chain_of`, which builds the links over the place
(`reading_link`, `read_option`, `read_place`); `opt_chain_on` and
`chain_of` are the plain chain's parts. Lowering is unchanged: the chain
nests, and its value holds borrows that end with the statement.

**What it changes.** Programs whose interpolations hold optional chains
over places, whose SIR now borrows where it moved. The case
`cases/findings/interpolated_chains.sisu` (its `.out`: it runs; stage 0
refuses it, `E0400`, `E0401` and `E0419`).

- Differs: `compiler/stage1/cases/findings/interpolated_chains.sisu`: ids, human

**Decision 295**, for docs/architecture.md:

295. An interpolation reads its values: an optional chain of fields
     written in one (`{u.team?.name}`) reads what it reaches through a
     place, each link's field in place and the last one's value read
     when it is not `Copy` (a `String` as a `str`, an optional field as an
     optional reference), as `x?.f` through a reference borrows `f`; a
     chain over a value, or outside an interpolation, moves as before.
     Stage 1 decides it (RULES.md, R14).

## R15. An error set that hides the prelude's warns (decision 296)

**The rule** (D-024: shadowing an outer scope warns). An error set
declared with the name of one of the prelude's (`TimeError`, `IoError`,
`NetError`, ...) is `W0016` (`resolve.shadow-prelude-set`) at its name:
it hides the prelude's in its module, and the two are different sets with
one name, so what the prelude's functions return is not the program's
set.

**Why.** The manual's writers found `error TimeError` hiding the
prelude's without a word, after which `?` on a prelude function's result
failed with messages that named `main.TimeError` (chapter 8). D-024 warns
when a binding shadows an outer scope; the prelude is a module's
outermost scope, and an error set is where the shadow misleads, since
sets compose by their variants and read alike by their names. Other
declarations that hide the prelude's names (a program's `Path`, `check`,
`sum`) are left alone: the corpus has two dozen, stage 1 among them, and
none is a set.

**In stage 1.** `resolve`: `ModuleNames.error_sets`, `shadowed_set`.

**What it changes.** No program of the corpus. The case
`cases/findings/shadowed_error_set.sisu` (its `.diag`, `.out` and `.err`,
whose warning `sisu run` prints).

- Differs: `compiler/stage1/cases/findings/shadowed_error_set.sisu`: names, ids, human

**Decision 296**, for docs/architecture.md:

296. An error set named as one of the prelude's hides it in its module
     and warns (`W0016`, D-024), since the two are different sets with one
     name. Stage 1 decides it (RULES.md, R15).

## R16. The fixes and messages the manual's writers found missing (decision 297)

**The rule** (10, "Diagnostics": "Where possible, a suggested fix as a
diff the LSP can apply", "Missing-trait errors list the candidates", and
"Nothing after the first error in a dependency chain"; 02, "Departures
from Python"). Each of these diagnostics gains the fix or the message
that teaches what Sisu writes:

- `E0411` on a field whose type borrows only by `str` (`name: str`,
  `nickname: str?`, `list[str]`) gets the fix `borrow.own-string`, the
  type with `String` for each `str`, before `view struct`'s;
- `E0341` on a field changed through a method's plain `self` gets
  `type.take-mut` for `self`, which writes `mut ` before it;
- `E0347` and `E0368` for a missing `Eq`, `PartialOrd`, `Ord`, `Hash` or
  `Copy` of a struct or enum of the module being checked get
  `type.derive-trait`, which adds the trait with the supertraits the type
  lacks (`Ord` with `Eq` and `PartialOrd`, `Hash` with `Eq`) to its
  `@derive(...)`, or writes `@derive(...)` on a line above it, when every
  field has them;
- `E0300` for a borrowed collection whose items are not `Copy`, given
  where an `Iterable[T]` of owned items is expected (`sorted(hand)`), gets
  the note `type.borrowed-items` (the items are references, values only of
  a `Copy` type) and the fix `type.add-copy`;
- `E0300` for text where a `Path` is expected gets `type.make-path`,
  `Path.new(...)` around it;
- `E0320` for `?` on a result in a function that returns nothing and
  writes no result type gets `type.return-errors`: ` -> ! E` (the
  operand's set, or `!` when it is not known) in the header and `return
  Ok(())` at the body's end;
- `E0200` for Python's `open` gets the note `resolve.open-file`, which
  names `Fs` and `sys.fs.read_to_string`;
- `E0325` for a `for` over text gets `type.iterate-chars` and
  `type.iterate-bytes`;
- `E0369` for a character looked for in text (`c in "aeiou"`) gets
  `type.char-as-text` (`"{c}"` interpolated, a literal's own text);
- `E0305` for `print` or `print_err` given several values gets
  `type.interpolate`, the one text `print` takes (`print("{a} {b}")`, a
  literal's own text inline);
- an assignment to a name nothing declares (`greeting = "Hei"`), in a
  body, is `E0200` at it with `resolve.declare-with`, which writes `let `
  (`var ` when the body assigns the name again), and the name is then the
  local the fix makes, so its uses are not reported again;
- Python's `else` after a `for` or `while` is `E0127`
  (`parse.loop-else`), which says Sisu has none and shows `if let` on
  the search's result, where it was `E0103`, "expected an expression";
- an assignment on a `case` line (`case None: label = "x"`) is `E0118`,
  as a statement keyword there is, where it was `E0117`; both get
  `parse.arm-block`, which breaks the line after the `:` and indents the
  statement under the `case`;
- a typed hole whose value nothing uses (`todo("later")` as a statement)
  is a hole of type `()`, the statements still to write, where it was of
  type `_`.

A fix whose edit writes a line break shows its text up to the break in
the human form (`diag.fix_text`): the attribute line of
`type.derive-trait`, nothing of `parse.arm-block`'s.

**Why.** The manual's writers list each (manual/README.md, "What the
plan found in stage 0", chapters 1 and 4 to 9): diagnostics with no fix,
or with a fix a beginner does not want (`view struct` for a struct of
owned data), or that name the parser's expectation rather than the
mistake. 10 asks for fixes as diffs and for missing-trait errors that
list the candidates, 02 gives the Sisu spelling of each Python habit
(`print(x, y)` takes one argument, a declaration is `let`, no `else` on
loops, `if let` on the search's result), 03 makes `Copy` opt-in, and
D-061 makes a hole check against the type its position expects, which
for a statement whose value nothing uses is `()`.

**In stage 1.** `typeck.collect.owned_text_type` (`E0411`);
`typeck.check`: `self_param_at` (`E0341`), `derive_fix` and `derive_edit`
(`E0347` through `bound_not_satisfied`, `E0368` through
`check_comparison`), `check_expr_stmt` (the hole); `typeck.call`:
`borrowed_items_help`, `printed_together`; `typeck.expr`: `mismatch_fix`
(`Path`), `return_errors_fix` and `params_end`, `char_as_text_fix`, the
`for` over text; `resolve`: `use_name` (`open`), `declares_by_assignment`
and `assigned_later`; `parser.stmts.loop_else_reported`;
`parser.exprs.match_arm` and `arm_block_fix`; `diag.fix_text`.

**What it changes.** The conformance suite's programs with these
diagnostics, whose `ids` and `human` outputs gain the fixes and notes
(and whose `doc` outputs, which print `check`'s errors, with them), as
listed below. The cases `cases/findings/missing_fixes.sisu`,
`python_habits.sisu`, `loop_else.sisu`, `case_line_statements.sisu` and
`todo_statement.sisu` (each with its `.diag`; the last runs, its `.out`
and `.err`).

- Differs: `compiler/conformance/cases/generics/function_value_errors.sisu`: types, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/ownership/view_field.sisu`: ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/types/try_context.sisu`: types, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/wrong/view_in_struct.sisu`: ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/findings/missing_fixes.sisu`: ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/findings/python_habits.sisu`: names, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/findings/loop_else.sisu`: tree, ids, human, fmt-check, doc, doc-output
- Differs: `compiler/stage1/cases/findings/case_line_statements.sisu`: tree, ids, human, fmt-check, doc, doc-output
- Differs: `compiler/stage1/cases/findings/todo_statement.sisu`: ids, human
- Differs: `compiler/stage1/cases/later_phases.sisu`: tree, fmt-check
- Differs: `compiler/stage1/cases/statement_mistakes.sisu`: tree, fmt-check

**Documents to bring along:** the conformance suite's `.diag` files of
`generics/function_value_errors`, `ownership/view_field`,
`types/try_context` and `wrong/view_in_struct`, which gain the lines of
the fixes stage 1's `check --format=ids` now gives.

**Decision 297**, for docs/architecture.md:

297. The diagnostics the manual's writers found without a fix get one
     (10): `E0411` on a field that borrows by `str` owns a `String`;
     `E0341` through a method's `self` takes `mut self`; `E0347` and
     `E0368` for a missing `Eq`, `PartialOrd`, `Ord`, `Hash` or `Copy` of
     the module's struct or enum derive it with the supertraits it lacks;
     `E0300` for a borrowed collection of non-`Copy` items given as an
     `Iterable[T]` notes why and copies it, and for text where a `Path` is
     expected calls `Path.new`; `E0320` in a function that returns nothing
     returns `! E` and `Ok(())`; `E0325` over text iterates its characters
     or bytes; `E0369` for a character in text writes it as text; `E0305`
     for `print(a, b)` interpolates; `E0200` for `open` names `Fs`, and
     for an assignment that declares a name writes `let` (or `var`), the
     name declared from then on. Python's `else` on a loop is `E0127`, an
     assignment on a `case` line `E0118` with the block it needs, and a
     hole whose value nothing uses is of type `()`. Stage 1 decides it
     (RULES.md, R16).

## R17. An intrinsic's type arguments are written when a parameter cannot tell them (decision 298)

**The rule.** A call of an intrinsic writes its type arguments in SIR
(`call intrinsic NAME[T, ...](...)`) when one of its generic parameters
appears in its result and in no parameter, when a parameter is an
`Iterable[T]` (both as before, stage 0's `needs_type_args`), and now also
when one of the function's own generic parameters appears in no
parameter, in its result or nowhere: `std.mem`'s `size_of[T]() -> int`
and `align_of[T]() -> int` say nothing of `T` but by it, so `size_of[int]()`
lowers to `call intrinsic mem.size_of[int]()`, type argument 0 being `T`,
where it lowered to `call intrinsic mem.size_of()` and nothing after the
front end could know the type.

**Why.** The intrinsics of `std.mem` (R4) are the stage-0 round's to run,
in the interpreter and the native backend, and `RawBuf`, `Layout.of` and
`Layout.array` stand on `size_of` and `align_of`: an intrinsic whose type
argument is in neither its parameters nor its result is one whose meaning
is the type alone. The interpreter and the runtime treat an empty list
where a type argument is needed as an internal error naming the missing
argument.

**In stage 1.** `lower.exprs.needs_type_args`.

**What it changes.** No program of the corpus: of the standard library's
intrinsics, only `std.mem`'s `size_of` and `align_of` have a parameter
that appears nowhere but in the call (every other such parameter is in
its result, which wrote the arguments already), and no program of the
corpus calls either, nor reaches a function that does. The case
`cases/lowering/size_of_type_arguments.sisu` and its `.sir`, which
`check.sh`'s class `expected` holds stage 1's `sir` to.

- Differs: `compiler/stage1/cases/lowering/size_of_type_arguments.sisu`: names, ids, human, doc, doc-output

**Decision 298**, for docs/architecture.md:

298. An intrinsic's call writes its type arguments in SIR also when one
     of its own generic parameters appears in no parameter, in its result
     or nowhere (`mem.size_of[int]()`, `mem.align_of[(int, bool)]()`),
     besides when one appears in its result and in no parameter or a
     parameter is an `Iterable[T]`; type argument `k` is the `k`th
     generic parameter, the impl's first. Stage 1 decides it (RULES.md,
     R17).

## R18. An associated function takes explicit type arguments (decision 299)

**The rule.** A call of an associated function, a function of a type's
impl without `self`, may write the type arguments of the function's own
generic parameters after its name, `Type.function[Args](...)`, as a call of
a free function writes them, `f[Args](...)`: `Layout.of[u8]()`,
`Layout.array[int](3)`, `Shelf.empty_of[int]()`. The impl's generic
parameters come from the type as before, written
(`Pair[int].with_other[bool](1, True)`) or inferred from the arguments
(`Pair.with_other[str](2.5, "x")`); the brackets give the function's own,
in their order, as many as it has (`E0208` otherwise, and `E0349` for a
function that has none); the same holds for a trait's function called on
a type (`T.make[U]()`). A
body of the standard library that does not type-check is an internal
error, which names the module and the diagnostic, where its errors were
dropped and what they stood for lowered to `unreachable`.

**Why.** The resolution order is the one a call keeps already: a name's
static path first (a module, a type, a type parameter), then its member,
then the call's arguments, the brackets after a function's name being its
own type arguments (`x.method[T](...)` takes them so, and `f[T](...)`).
`Type.function[T]` was read as an index of the function value
(`E0360`, "a value of type `pure (int) -> mem.Layout?` cannot be
indexed") by both front ends, so `std.mem`'s `RawBuf`, which sizes its
buffer by `Layout.array[T](n)`, lowered that call to `unreachable`, and a
`RawBuf` of any capacity panicked: the standard library's diagnostics are
not a program's, and nothing reported them. A std body with an error is a
bug of the toolchain, never of the program that reaches it.

**In stage 1.** `typeck.call.call_expr`: `Type.function[Args](...)` calls
`static_member` with the brackets' types as `explicit_method_args`, which
its associated function and trait function paths pass to `call_fn` and
`call_fn_with` (`instantiate` puts them on the function's own generic
parameters, as for a method); `typeck.std_errors`, from `check_std_in`,
panics with `stage0.internal` naming the module and the diagnostic.

**What it changes.** No program of the corpus. The cases
`cases/calls/static_type_arguments.sisu` (it runs, its `.out`),
`static_type_argument_counts.sisu` (its `.diag`) and
`cases/unsafe/layout_type_arguments.sisu`, whose `.sir` shows
`mem.Layout.of[u8]()` and `mem.Layout.array[int](3)` called and
`mem.size_of[$0]()` inside them.

- Differs: `compiler/stage1/cases/calls/static_type_arguments.sisu`: ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/calls/static_type_argument_counts.sisu`: ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/unsafe/layout_type_arguments.sisu`: names, ids, human, doc, doc-output

**Decision 299**, for docs/architecture.md:

299. An associated function takes the type arguments of its own generic
     parameters after its name, `Type.function[Args](...)`, as a free
     function does: the type gives its impl's parameters, written or
     inferred, and the brackets the function's own (`E0208`, `E0349` for
     a function without them); a trait's function called on a type takes
     them the same way. A body of the standard library that does not
     type-check stops the compiler with an internal error naming the
     module and the diagnostic, where it lowered to `unreachable`. Stage 1
     decides it (RULES.md, R18).

## R19. Exported functions and blocking foreign calls (decision 300)

**The rule** (22, "Foreign calls"). An `@export`ed function is safe to
write because foreign code that calls it cannot hand it what Sisu keeps
alive: its signature holds no reference, view, `Shared` (or `Weak`),
closure or capability, and each parameter or result that does is `E0989`
(`unsafe.export-signature`). `@blocking` marks a declaration of an
`extern` block whose call may block its thread for an unbounded time, so
that the scheduler moves the call off the task threads; on anything but a
foreign function it is `E0990` (`unsafe.blocking-not-foreign`). Both
attributes were accepted without a word wherever they were written.

**Why.** 22 gives both: the export boundary's soundness argument is the
signature rule, and `@blocking` is a promise about a foreign call, which a
Sisu function's callers do not need (the scheduler sees its blocking
points). What the backend does with them (the export's C symbol and its
abort at the boundary, a blocking call's thread) is stage 2's, which SIR
does not carry yet; the front end checks what it can.

**In stage 1.** `typeck.collect`: `check_export_blocking` and
`unexportable`, from `resolve_fn`, for the program's own functions.

**What it changes.** No program of the corpus. The case
`cases/unsafe/exports.sisu` (its `.diag`).

- Differs: `compiler/stage1/cases/unsafe/exports.sisu`: ids, human, doc, doc-output

**Decision 300**, for docs/architecture.md:

300. An `@export`ed function's parameters and result hold no reference,
     view, `Shared`, `Weak`, closure or capability (22, "Foreign calls";
     `E0989` at each that does), since foreign code that calls it cannot
     keep them alive; `@blocking` marks a foreign function only (`E0990`
     elsewhere). Stage 1 decides it (RULES.md, R19); what the backend makes
     of either is stage 2's.

## R20. A float's `Debug` takes the whole format spec (decision 313)

**The rule** (02, "Literals": "`{expr=}` ... the value takes its `Debug`
rendering, as Python uses `repr`, and a format spec may follow, as in
`{x=:.2f}`"). The `Debug` impls of `f64` and `f32` that the front end
writes among `builtin_impls` (`std_sources.leaf_format`) apply a spec as
their `Display` impls do, through `Spec.float` and `Spec.float32`: the
precision, the type letter, the sign and grouping and a number's
alignment, where they padded the `Debug` text and dropped the rest. So
`{pi=:.1f}` prints `pi=3.1` and `{pi=:.2f}` `pi=3.14`, where both printed
`pi=3.14159`; a spec whose type is `?` (`{x:>8?}`) still writes the
`Debug` text padded, and so does a float without a spec. A float's `Debug`
text is the value as it is written, which is its `Display` text, the
shortest that reads back, and a leaf written with a spec applies the
whole spec itself (docs/architecture.md, "Formatting": "Forms" and "The
protocol"), so one spec means the same for both.

**Why.** The manual's writers found `{x=:.1f}` printing every digit
(chapter 1), which 02 rules out; the stage-0 round traced it to the
generated `Debug` text, which is the front end's, not the run time's.
The line the impl takes in `builtin_impls` is one, as before, so no other
function of that text moves.

**In stage 1.** `typeck.std_sources.leaf_format`.

**What it changes.** What a program prints that formats a float's
`Debug` with a spec other than `?`, which no program of the corpus does,
and the SIR of each program that reaches a float's `Debug` impl (43 of
the corpus, stage 1's cases and the proposal's examples), whose body
calls `Spec.float` where it padded the `Debug` text; the frozen Rust
front end keeps its own body, so their `sir` (the classes `sir` and
`corpus-sir`), `sir-tests` and `contracts-*` outputs differ from it, and
what they print does not (the class `behaviour`). The case
`cases/run/float_debug_precision.sisu` (its `.out`: it runs).

- Differs: `compiler/conformance/cases/capabilities/fakes.sisu`: sir
- Differs: `compiler/conformance/cases/concurrency/mutex.sisu`: sir, contracts-off, contracts-preconditions
- Differs: `compiler/conformance/cases/concurrency/sync_types.sisu`: sir
- Differs: `compiler/conformance/cases/contracts/checks.sisu`: sir, sir-tests, contracts-off, contracts-preconditions
- Differs: `compiler/conformance/cases/contracts/explain.sisu`: sir, sir-tests, contracts-off, contracts-preconditions
- Differs: `compiler/conformance/cases/contracts/invariants.sisu`: sir, sir-tests, contracts-off, contracts-preconditions
- Differs: `compiler/conformance/cases/contracts/properties.sisu`: sir-tests
- Differs: `compiler/conformance/cases/contracts/proposal_contracts.sisu`: sir, sir-tests, contracts-off, contracts-preconditions
- Differs: `compiler/conformance/cases/contracts/refinements.sisu`: sir-tests
- Differs: `compiler/conformance/cases/contracts/result_references.sisu`: sir, contracts-off, contracts-preconditions
- Differs: `compiler/conformance/cases/doc/items.sisu`: sir, sir-tests, contracts-off, contracts-preconditions
- Differs: `compiler/conformance/cases/examples/errors.sisu`: sir
- Differs: `compiler/conformance/cases/examples/traits.sisu`: sir
- Differs: `compiler/conformance/cases/generics/drop_generic.sisu`: sir
- Differs: `compiler/conformance/cases/generics/functions.sisu`: sir
- Differs: `compiler/conformance/cases/generics/types.sisu`: sir
- Differs: `compiler/conformance/cases/ownership/mutex_poison.sisu`: sir, contracts-off, contracts-preconditions
- Differs: `compiler/conformance/cases/ownership/mutex_unpoisoned.sisu`: sir, contracts-off, contracts-preconditions
- Differs: `compiler/conformance/cases/ownership/proposal_graph.sisu`: sir
- Differs: `compiler/conformance/cases/packages/failing_test.pkg`: sir-tests
- Differs: `compiler/conformance/cases/reflect/promotion.sisu`: sir
- Differs: `compiler/conformance/cases/reflect/proposal_comptime.sisu`: sir, sir-tests
- Differs: `compiler/conformance/cases/reflect/trait_methods.sisu`: sir
- Differs: `compiler/conformance/cases/replay/tokens.sisu`: sir-tests
- Differs: `compiler/conformance/cases/run/debug_forms.sisu`: sir
- Differs: `compiler/conformance/cases/run/format_specs.sisu`: sir
- Differs: `compiler/conformance/cases/run/proven_checks.sisu`: sir
- Differs: `compiler/conformance/cases/run/sized_integers.sisu`: sir
- Differs: `compiler/conformance/cases/std/json_decode.sisu`: sir
- Differs: `compiler/conformance/cases/std/pure_iterators.sisu`: sir
- Differs: `compiler/conformance/cases/std/search.sisu`: sir
- Differs: `compiler/conformance/cases/testing/proposal_testing.sisu`: sir-tests
- Differs: `compiler/conformance/cases/testing/refines.sisu`: sir, sir-tests
- Differs: `compiler/conformance/cases/traits/assoc_types.sisu`: sir
- Differs: `compiler/conformance/cases/traits/delegate.sisu`: sir
- Differs: `compiler/conformance/cases/traits/dyn_values.sisu`: sir
- Differs: `compiler/conformance/cases/traits/operators.sisu`: sir
- Differs: `compiler/conformance/cases/traits/own_eq_ord.sisu`: sir
- Differs: `compiler/stage1/cases/channels/generic_pipeline.sisu`: sir
- Differs: `compiler/stage1/cases/doc/layout.sisu`: sir, contracts-off, contracts-preconditions
- Differs: `compiler/stage1/cases/doc_example_generator.sisu`: sir-tests
- Differs: `compiler/stage1/cases/generator_in_template.sisu`: sir-tests
- Differs: `proposal/examples/ownership.sisu`: sir
- Differs: `proposal/examples/traits.sisu`: sir

The merge of stage 0's eleventh round gives `Zone` a `Debug` form that
writes a float (decision 684), which `std/time_zones` now reaches:

- Differs: `compiler/conformance/cases/std/time_zones.sisu`: sir

**Decision 313**, for docs/architecture.md:

313. The `Debug` impls of `f64` and `f32` that the front end writes apply a
     format spec as their `Display` impls do (`Spec.float`, `Spec.float32`),
     since the two texts are the same: `{x=:.2f}` rounds (02), where the
     precision was dropped and the `Debug` text padded; a spec of type `?`
     still pads the `Debug` text. Stage 1 decides it (RULES.md, R20).

## R21. A collection holds values or views, never references (decision 320)

**The rule** (04, "The decision", rule 3: references "cannot be stored in
ordinary structs, in `Shared`, in collections, or captured by closures
that escape"; rule 4: "View types are the escape hatch for storing
references"). A collection's items are values it owns, or views (`str`,
`Slice[T]`, a `view struct`, an iterator over a borrowed collection), and
never references: a `ref T` or `ref mut T` as the item type of `list`,
`set`, `dict` (its keys or its values) or `Array`, or of `std.collections`'
`Deque`, `Heap`, `SortedDict` or `SortedSet`, is `E0438`
(`borrow.ref-in-collection`, "`list[ref String]` would hold references,
and a collection holds values it owns or views, never references (04,
rule 3)"), wherever the program's code has it:

- written in a type (a parameter's, a result's or a field's, an
  annotation, a type argument): at the item's type, with the fix
  `borrow.own-items`, which takes the `ref` away ("hold the values
  themselves: `list[String]`");
- inferred in a body (`var names = []` that a `names.push(ref aino)`
  decides, `[ref a, ref b]`, `{"k": ref x}`, a `collect()` of an iterator
  over a borrowed collection, `xs.iter().filter(it > 4).collect()`): at
  the first place the body has the type, the local it is inferred for or
  the expression that makes it, once a body for each collection type,
  with the fix `borrow.copy-items` (`.map(it.copy())` before a
  `.collect()` that makes it) or the note `borrow.items-owned` (copies
  made with `.copy()` or `.to_string()`, or views such as `str`; a
  reference is kept only in a `view struct`'s `ref` field).

A shared borrow of a `str` is the `str` (a declaration's type reads `ref
str` so), so `list[str]` built from `d.keys()` of a `dict[str, V]` is a
list of views. And `E0300` for a reference where what it refers to is
expected, `names.push(ref aino)` into a `list[String]`, gets the fix
`type.add-copy` with the copy, `aino.copy()`, so that the owned program
the first fix leads to is one more fix away.

Code the compiler writes (a template's instances, derived impls) and the
standard library's bodies are not checked here: a generic body is checked
once, with its type parameters opaque (D-017), and a type parameter may
stand for a reference, as `Option[ref T]` and the iterator adapters of
the prelude need. A generic struct whose type argument is a reference
(`Bag[ref String]`) is view-like, as stage 0 has it (decision 257); what
its body makes of the argument is a collection of references only where a
program's body makes one (`Bag(items=[])`).

**Why.** The manual's writer of chapter 11 found that `var names:
list[ref String] = []` with `names.push(ref aino)` compiles and runs: the
frozen Rust front end treats a list holding a reference as a view, as it
treats a `list[str]`, so the list cannot outlive what it borrows, but the
program has a reference stored in a collection, which rule 3 rules out.
The view types are where a borrow is kept (rule 4, and 04's "Views": "the
list is also a view: it holds views"), each a type declared for it, with
one lifetime; a bare reference is bound to a local, passed, or returned,
and that is all, which is what keeps Sisu without lifetime names. The fix
names the owned alternative, as 10 asks of a diagnostic: the values
themselves, or copies of them.

**In stage 1.** `typeck.program`: `collection_name`, `ref_items` and
`is_ref_item`; `typeck.collect.written_ref_items` (declarations);
`typeck.check`: `written_ref_items` (annotations), `check_ref_items` (the
body's locals and expressions, after its bounds), `owned_items`;
`typeck.expr.mismatch_fix` (the copy).

**What it changes.** Three programs of the corpus make a collection of
references, which stage 1 now refuses: the conformance suite's
`run/iterators` (`xs.iter().filter(it > 4).collect()`, a `list[ref
int]`), `contracts/properties` (`d.keys().collect()` of a `dict[String,
int]`, a `list[ref String]`) and `generics/loan_through_own`
(`Bag[ref String]` made with `Bag(items=[])`, a `list[ref String]`). The
cases `cases/references/ref_items.sisu` (the refused forms, its `.diag`)
and `owned_items.sisu` (the owned program that replaces them, which runs:
its `.out`).

- Differs: `compiler/conformance/cases/contracts/properties.sisu`: ids, human
- Differs: `compiler/conformance/cases/generics/loan_through_own.sisu`: sir, ownership, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/references/ref_items.sisu`: ids, human, doc, doc-output

(`run/iterators` differed in every class, `behaviour` included, until the
suite's case took the owned spelling below; it matches stage 0 in every
class since.) The output `behaviour` is the class of that name: the
program a case expects to run is refused, so stage 1 lowers nothing and
the run is not made (`check.sh` counts it "refused as listed").

**Documents to bring along:** the conformance suite's `run/iterators`,
whose fourth line becomes `print(xs.iter().filter(it > 4).map(it.copy()).collect())`
with the same `.out`; `contracts/properties`, whose test "a dictionary's
keys" counts `d.keys().count()` (or collects copies); and
`generics/loan_through_own`, whose loan through an `own` argument a
`Bag[str]` keeps (a list of views, which is allowed, holding a view of
`s`), or which becomes an E0438 case with that `.diag`.

**Decision 320**, for docs/architecture.md:

320. A collection's items are values it owns or views, never references
     (04, rule 3): a `ref T` as the item type of `list`, `set`, `dict`,
     `Array` or a collection of `std.collections` is `E0438`, where a
     type writes it (with the fix that takes the `ref` away) and where a
     body's local or expression has it (with the fix that collects
     copies, `.map(it.copy())`, or the note naming copies and views),
     once a body for each collection type; a shared borrow of a `str` is
     the view. Generic bodies and the code the compiler writes are
     checked with their type parameters opaque, which may stand for a
     reference. Stage 1 decides it (RULES.md, R21); the frozen Rust front
     end treats such a collection as a view.

## R22. What the checker let through that the language refuses (decision 321)

**The rule.** Five programs the checker accepted are refused, each one
whose run would take one type for another or do what its text does not
say:

- an impl's method whose parameter or result types are not its trait's,
  with `Self` and the trait's arguments substituted and its associated
  types bound (`def price(self) -> f64` for a trait's `-> int`; `impl
  Mul[Vector] for f64` whose `mul` returns a `Vector` while `Output` is
  `f64`), is `E0234` (`resolve.method-signature`), as stage 0's resolver
  reports it, where stage 1 compared only the method's shape (its
  receiver, its parameters' number and modes, its own generic
  parameters);
- `impl Copy for Token:` written by hand is `E0270`
  (`resolve.copy-by-hand`, "`Copy` is derived, not implemented"), with
  the fix `resolve.derive-copy`, which writes `@derive(Copy)` on the type
  and takes the impl away: `Copy` is opt-in through `@derive(Copy)`
  (03, "Auto traits"), which checks the fields, and an impl the program
  writes left the type moved;
- a `dyn Trait` of a trait that cannot be one, in a type a body writes
  (`let shapes: list[Box[dyn Shape]]` with a method of `Shape` returning
  `Self`), is `E0241` (`resolve.not-object-safe`), as it is in a
  signature;
- a call of a trait's method whose impl nothing decides (`"{a.into()}"`,
  where nothing asks for a type) is `E0324` (`type.cannot-infer`, "the
  type of `a.into()` cannot be inferred here"), in a body without another
  error, as a local whose type nothing decides is;
- an `assert` whose condition moves a value (`assert m.into_inner() ==
  2`) does not show its parts when it fails, since showing them evaluates
  them again: it shows the two sides of its comparison, computed once
  (17, "Assertions": the capture records "each sub-expression", which a
  condition that only reads allows). Its run moved the value twice,
  `E0400` "moved in an earlier iteration of the loop" where there is no
  loop.

**Why.** The manual's writers found each (chapters 14, 15 and 20, and the
reference's R2): `sisu check` accepting an impl whose method returns an
`f64` for the trait's `int`, which the interpreter then added to an
`int` ("internal error in the interpreter: add on Float(2.5) and
Int(1)"); `impl Copy` compiling without a word and the type moving all
the same; a call through a `dyn` of a trait that is not object safe
stopping the interpreter (`Shape.area` on a `dyn` holding a struct); and
`x.into()` passing the checker, its type left `()`, and the interpreter
finding no impl of `Into` for `u8`. 03 makes the checker sound ("Static,
sound, no `any`"), `Copy` opt-in by derive, and a `dyn` of a trait that
cannot be one an error that says why; D-062 rules out a spelling that
means something else than it says.

**In stage 1.** `typeck.collect`: `check_method_types` (from the impl
checks, where the associated types are all bound),
`check_copy_by_hand` (from `resolve_impl`), `object_safety_problem`
(now `typeck.program`'s, shared with the checker); `typeck.check.resolve_ty`
(a `dyn` in a body's type); `typeck.Checker.check_bounds` and
`report_uninferred` (`undecided_call`); `typeck.purity.moves_place`.

**What it changes.** No program of the corpus. The cases
`cases/soundness/checker_holes.sisu` (the four refused forms, its
`.diag`), `checker_holes_fixed.sisu` (the programs the fixes leave, which
run: its `.out`) and `assert_moves.sisu` (an assertion that moves the
value it tests, once when it holds and once when it fails: its `.out`,
`.err` and `.status`).

- Differs: `compiler/stage1/cases/soundness/checker_holes.sisu`: names, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/soundness/assert_moves.sisu`: ids, human, doc, doc-output

**Decision 321**, for docs/architecture.md:

321. The checker refuses what it let through: an impl's method whose
     parameter or result types are not its trait's (`E0234`, as stage 0's
     resolver has it, where stage 1 compared the shape only); `impl Copy`
     written by hand (`E0270`, with the fix that derives it, since `Copy`
     is opt-in through `@derive(Copy)`, 03); a `dyn` of a trait that
     cannot be one in a body's types (`E0241`, as in a signature); a call
     of a trait's method whose impl nothing decides (`E0324`, the call
     named as the expression to annotate). An `assert` whose condition
     moves a value shows its comparison's sides, computed once, rather
     than its parts, which it would evaluate again. Stage 1 decides it
     (RULES.md, R22).

## R23. What lowering wrote that the program did not say (decision 322)

**The rule.** Two lowerings that gave a program another meaning than its
text:

- a sized integer's compound assignment is its operator's, at its width
  (03, "Numbers": overflow traps in every build mode, `+%` wraps and `+|`
  saturates): `level +%= 10` on a `u8` holding 250 leaves 4, `sat +|= 10`
  leaves 255, and `level += 10` traps, as `level = level +% 10`,
  `level = level +| 10` and `level = level + 10` do. Lowering wrote the
  `int` operations (`wadd`, `sadd`, `add`) for them, which the
  interpreter and the backend computed at the width of `int`, so a `u8`
  held 260;
- a borrowed parameter whose type holds the function's type parameters
  (`p: (T, T)`, `p: T?`) is taken by value exactly when their bounds
  make it `Copy`, and every call passes it so, whatever its type
  arguments: by reference for `(T, T)` with `T` unbounded, as for a bare
  `T`, even when the call's `(int, int)` is `Copy`; by value for `(T, T)`
  with `T: Copy`. The function took `ref (T, T)` while a call with a
  `Copy` instance passed the tuple itself, and the interpreter stopped at
  `p.0` ("dereferencing Tuple"), as both front ends lowered it.

**Why.** The manual's writers found both (chapter 2, `+%=` on a `u8`;
the reference's R2, a generic function over a tuple): each is a program
whose text says one thing and whose run did another, which the
specification rules out (03; D-062). A generic function is checked and
lowered once, with its type parameters opaque (D-017), so how it takes a
parameter is decided by its signature and its bounds, which every call
follows.

**In stage 1.** `lower.stmts.aug_assign` (the sized integer's
intrinsic, `lower.exprs.sized_op`, which the binary operators use);
`typeck.program.param_local_ty` (`is_copy_in` for any type that holds
the function's parameters) and `typeck.call.instantiate` (`opaque` for
a parameter that the function takes by reference and whose type has
`Copy` instances, which a call then borrows).

**What it changes.** No program of the corpus: none makes a compound
assignment on a sized integer, nor passes a `Copy` instance of a type
that holds a type parameter to a generic function that borrows it (the
corpus's calls of `sorted` and the other functions over an `Iterable[T]`
keep their arguments, since no instance of `Iterable[T]` is `Copy`). The
cases `cases/soundness/lowering_holes.sisu` (its `.out`) and
`compound_overflow.sisu` (the trap: its `.out`, `.err` and `.status`),
which run interpreted and natively.

**Decision 322**, for docs/architecture.md:

322. A sized integer's compound assignment (`+=`, `+%=`, `+|=` and the
     others) lowers to its operator's intrinsic at its width
     (`u8.wrap_add`), as the binary operator does, where it lowered to
     the `int` operation; and a borrowed parameter whose type holds the
     function's type parameters is taken by value exactly when their
     bounds make it `Copy`, every call passing it so: a call with a
     `Copy` instance of a type the function takes by reference (a tuple,
     an option, a type that derives `Copy`) borrows its argument. Stage 1
     decides it (RULES.md, R23).

## R24. The fixes and notes the manual's later chapters found missing (decision 323)

**The rule** (10, "Diagnostics": "a suggested fix as a diff the LSP can
apply"; 02, "Departures from Python"; 04, rule 1). Each of these
diagnostics gains the fix or the note that names what Sisu writes, or
loses the one that leads to another error:

- `E0304` for Python's `", ".join(names)` gets `type.join-items`, the
  call written as Sisu's, `names.join(", ")`; for `append`, `clone`,
  `strip`, `upper`, `lower`, `startswith`, `endswith` and `to_owned`,
  `type.sisu-spelling` with Sisu's name (`push`, `copy`, `trim`, ...),
  where the similar-name fix offered none or `iter` for `filter`; for an
  iterator's method on a collection (`xs.map(f)`), `type.iterate-first`,
  `.iter()` before it; and for a trait's method on a type parameter its
  bounds do not give it, `type.add-bound` (the trait, when one trait
  declares the method), as `E0347` offers. The arguments of a call that
  failed are no longer checked where they mention `it`, which made each
  `E0342`;
- `E0300` for text where a `String` is expected calls `.to_string()`, not
  the first conversion by name, `.to_lowercase()`;
- `E0340` at a call that passes `mut` for a parameter its function changes
  (the function's body has `E0341`, whose fix takes the parameter with
  `mut`) gets that same fix, `type.take-mut` at the parameter, in place of
  removing the call's `mut`: the two fixes undid each other. Where the
  function does not change it, the call's `mut` still goes;
- `E0413` for a result of text (`-> str`) gets `borrow.own-result` first,
  the result as owned text (`-> String`): text read from a file or made in
  the function borrows no parameter, and the `from` clause led to `E0407`;
- `E0407` for a returned view of text gets the note `borrow.return-text`:
  a copy of a `str` is the same view, so `.copy()`, which the note
  offered, gave `E0407` again; a `String` result owns its text;
- `E0400` and `E0401` get the note `borrow.move-remedies`, each remedy
  with what it costs: a copy makes a second value (an allocation for a list
  or a `String`), moving later costs nothing, and the third, a parameter
  taken without `own` where the function only reads, lends the value;
- `E0243` lists `Default` among the traits `@derive` takes (decision 238),
  in the catalog's text both front ends print;
- `from wordcount import words` of a name the package's root module
  re-exports (`pub from wordcount.count import words`) binds it, where it
  was `E0204` and each use `E0200`: a `from` import of a module of the
  program waits for that module's own imports (`typeck.collect`), and the
  name pass resolves the imports again once every module's are known.

**Why.** The manual's writers list each (manual/README.md, chapters 10 to
17 and the reference's R19): diagnostics with no fix, a fix that leads to
the next error (`.to_lowercase()` changing the text, `from fs, path` for a
`str` read from a file, `.copy()` of a view), two errors whose fixes undo
each other, a note that names remedies without what they cost, and an
import of a re-exported name, which 09 allows (`pub from` re-exports), that
failed while the module path to it worked.

**In stage 1.** `typeck.call`: `spelling_fix`, `sisu_spelling`,
`method_bound_fix`, `callee_takes_mut` (with `Checker.arg_callee`) and
`check_args_loosely`; `typeck.agree_mut_fixes`; `typeck.expr.mismatch_fix`
(`to_string`); `typeck.collect`: the owned result of `E0413`,
`import_waits` and `package_module`; `borrowck.loans.holds_text_view`;
`borrowck.init` (the note); `resolve.resolve_program` (the second pass of
imports).

**What it changes.** The conformance suite's programs with these
diagnostics, whose `ids` (for the fixes) and `human` outputs (and `doc`,
which prints `check`'s errors) differ, as listed below: `E0413`'s
`ownership/from_required` and `wrong/missing_from`, `E0304`'s
`traits/method_needs_bound`, and the dozen of `E0400` and `E0401`, whose
note is new. The cases `cases/habits/fixes.sisu` and `moves.sisu` (their
`.diag`), and `cases/packages/reexport_import.pkg`, which runs (its
`.out`).

- Differs: `compiler/conformance/cases/ownership/from_required.sisu`: ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/ownership/partial_move_use.sisu`: human
- Differs: `compiler/conformance/cases/ownership/rustc/reinit.sisu`: human
- Differs: `compiler/conformance/cases/ownership/use_after_move.sisu`: human
- Differs: `compiler/conformance/cases/ownership/use_after_move_loop.sisu`: human
- Differs: `compiler/conformance/cases/testing/parametrised_once_ownership.sisu`: human
- Differs: `compiler/conformance/cases/traits/method_needs_bound.sisu`: types, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/wrong/missing_from.sisu`: ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/wrong/moved_in_loop.sisu`: human
- Differs: `compiler/conformance/cases/wrong/moved_in_one_branch.sisu`: human
- Differs: `compiler/conformance/cases/wrong/moved_into_list.sisu`: human
- Differs: `compiler/conformance/cases/wrong/moved_into_struct.sisu`: human
- Differs: `compiler/conformance/cases/wrong/moved_twice_in_call.sisu`: human
- Differs: `compiler/conformance/cases/wrong/partial_move_then_whole.sisu`: human
- Differs: `compiler/conformance/cases/wrong/return_field_view_of_local.sisu`: human
- Differs: `compiler/conformance/cases/wrong/return_local_view.sisu`: human
- Differs: `compiler/conformance/cases/wrong/return_owned_param_view.sisu`: human
- Differs: `compiler/conformance/cases/wrong/use_after_move.sisu`: human
- Differs: `compiler/stage1/cases/habits/fixes.sisu`: ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/habits/moves.sisu`: human
- Differs: `compiler/stage1/cases/packages/reexport_import.pkg/app`: names, sir, ids, human, doc, doc-output

**Documents to bring along:** the manual's transcripts that show these
diagnostics (its `examples/*.console` with `E0243`'s list, `E0400`'s
note, `E0304`, `E0300`, `E0340` or `E0413`), which `sisu`, whose front
end is stage 1, now prints otherwise; and the conformance suite's `.diag`
files of `ownership/from_required`, `wrong/missing_from` and
`traits/method_needs_bound`, which gain the fixes.

**Decision 323**, for docs/architecture.md:

323. The fixes and notes the manual's later chapters found missing: `E0304`
     writes Sisu's spelling of Python's and Rust's methods (`names.join(",
     ")`, `push`, `copy`, `trim`), iterates a collection before an
     iterator's method, and bounds a type parameter by the trait whose
     method it lacks; `E0300` converts text with `.to_string()`; `E0340`
     at a call whose function changes the parameter offers the
     function's `mut`, as `E0341` does, where the two undid each other;
     `E0413` and `E0407` on text name a `String` result; `E0400`'s note
     names what each remedy costs and the borrowed parameter; `E0243`
     lists `Default`; and `from pkg import x` binds a name `pkg`
     re-exports, the import waiting for the module's own. Stage 1 decides
     it (RULES.md, R24).

## R25. Forms the documents give that the checker refused, and three more habits (decision 324)

**The rule.** Forms 02, 03 and 04 give, which the manual's writers found
refused:

- `with Booking.start("sauna"):`, without `as` (Python's `with lock:`),
  keeps its value without a name to the end of the block, where it was
  `E0100` ("expected `as`"): `with` is "a scoped binding whose value is
  dropped at the end of the block" (02);
- `ref` of a value that is no place, `show(ref Coffee(1))`, borrows a
  temporary that lives to the end of its statement, as an argument a
  parameter borrows does, where it was `E0313` ("this expression cannot be
  assigned to"); a borrow kept past the statement is `E0406`, with its note
  to bind the temporary first. `ref mut` of one stays `E0313`;
- a `Box` has the length of its value (`len(b)`, `impl[T: Len] Len for
  Box[T]`), whose methods it reaches, and a `Slice` prints as the list it
  views (`impl[T: Debug] Display for Slice[T]`), where both were `E0347`:
  two impls stage 1 adds after stage 0's text of `builtin_impls`;
- a `Key[T]` is a set element or a dict key: a derived `Hash` or `Eq`
  reads the type's fields, and a type argument only as a field holds it,
  so `Key[Ship]`, whose `T` no field holds, is structural, where `E0398`
  said it had an impl of its own;
- a trait method's type arguments are its own, `xs.iter().map[int](f)`,
  as an inherent method's are: they were dropped, so
  `collect[set[int]]()` compiled and made a list. A method that takes none
  is `E0349` (`collect[set[int]]()` and the tour's `collect[list[int]]()`
  alike, until the prelude has a `collect` over the collection it makes).

And three habits get a message that names what Sisu writes: Python's
`sorted(xs, key=len)` (`E0306`) the note `type.sort-by-key` (`sort_by`,
`max_by`, `min_by`), its `key=len` no longer `E0350`; a closure kept in a
`let` without its parameters' types, `(a, b) -> a + b`, `E0324` at the
operand (`type.annotate-params`) where `E0310` said "`+` cannot be applied
to `_` and `_`"; and `from std.collections import *` `E0128`
(`parse.glob-import`, Sisu has no glob imports) where `E0100` said
"expected a name, found `*`".

**Why.** 02 gives `with` as a scoped binding and Python's form is the
habit; 04's rule 3 lets a reference be passed, and a temporary is what
`ref` of a call's value borrows; 03 has a `Box` reach its value's methods
and `print` take what has `Display`, and decision 256 makes a derived
`Hash` agree with the structural one; a dropped type argument is a
spelling that means something else than it says (D-062), which a refusal
corrects until the prelude's `collect` takes the collection. The habits
are the manual's (chapters 13 to 17).

**In stage 1.** `parser.stmts.with_stmt` and `typeck.check` (`with`);
`typeck.expr` (`ref` of a temporary, the closure's operands);
`typeck.std_sources.builtin_impls` (the two impls); `typeck.program.structural_depth`;
`typeck.call.trait_call` (the type arguments) and the unknown keyword's
note; `parser.items.import_from`.

**What it changes.** Two programs of the corpus key a set or a dict by a
`Key[T]` (the conformance suite's `ownership/proposal_graph` and the
proposal's `examples/ownership.sisu`), whose SIR now hashes the keys as
the host does rather than through the derived impls, which print the
same. The cases `cases/habits/forms.sisu` (the forms, which run: its
`.out`), `refusals.sisu` and `glob_import.sisu` (their `.diag`). The
conformance suite's `run/shared_equality` (decision 464, stage 0's fifth
round) writes `d.get(b) == Some(ref 1)`, the borrow of a temporary, which
the frozen Rust front end refuses with `E0313` and stage 1 lowers; the
case expects stage 1's run.

- Differs: `compiler/conformance/cases/ownership/proposal_graph.sisu`: sir
- Differs: `compiler/conformance/cases/run/shared_equality.sisu`: types, sir, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/habits/forms.sisu`: tree, ids, human, fmt-check, doc, doc-output
- Differs: `compiler/stage1/cases/habits/glob_import.sisu`: tree, ids, human, fmt-check, doc, doc-output
- Differs: `compiler/stage1/cases/habits/refusals.sisu`: ids, human, doc, doc-output
- Differs: `proposal/examples/ownership.sisu`: sir

**Decision 324**, for docs/architecture.md:

324. `with e:` without `as` keeps the value without a name to the end of
     the block (02); `ref` of a value that is no place borrows a temporary
     to the end of its statement; a `Box` has its value's `Len` and a
     `Slice` a `Display`; a derived `Hash` or `Eq` is structural when the
     fields are, whatever the type arguments no field holds (`Key[T]` as a
     key); a trait method's type arguments are its own, and one that takes
     none is `E0349` (`collect[set[int]]()`, which made a list). Python's
     `sorted(xs, key=len)` names `sort_by`, `max_by` and `min_by`, a
     closure kept in a `let` without its parameters' types is `E0324`,
     and `import *` is `E0128`. Stage 1 decides it (RULES.md, R25).

## R26. A generic struct's construction: the type expected and the fields' defaults (decision 325)

**The rule** (02, "Declarations": "Struct fields may have defaults in any
position"; 03, "Type inference": information flows "from the expected
type to the expression"). Two rules of a generic struct's construction:

- the type expected gives the struct's type arguments before its fields
  are checked: `let r: Reading[f64, String] = Reading(21.5, "°C")` checks
  the text against `String` (a literal takes the type expected), where the
  fields decided the arguments first and the result, `Reading[f64, str]`,
  was `E0300`. An expected type holding a `dyn` (a `Box[dyn Shape]`) or a
  type not told yet gives nothing: the struct is made of its fields and
  coerced after, as before;
- a field's default in a generic struct is as generic as the struct: its
  function in SIR takes the struct's type parameters
  (`fn Shelf#items[T](0) value`), and each construction that leaves the
  field out calls it with the struct's type arguments
  (`Shelf#items[int]()`), its temporary of the field's type with them
  substituted. Lowering wrote the function without type parameters and
  the temporary with the struct's `$0`, which SIR's validation refused in
  the program's `main` (`E0501`, "the type of _2 names a type parameter
  the function does not have"); the Rust front end stops at it with
  `E0901`.

**Why.** The manual's writer of chapter 15 found both: the exercise of an
empty stack made its value in an associated function to avoid the
default, and a reading built with its type written still had to call
`.to_string()`. 03's bidirectional inference is what lets an annotation
pick a literal's type, and a default is a value of the field's type,
which in a generic struct holds the struct's parameters.

**In stage 1.** `typeck.call.construct` (the expected type);
`lower.func.lower_body` (the default's type parameters) and
`lower.exprs.into` (the call's type arguments).

**What it changes.** The SIR of one program of the corpus whose generic
struct has a field with a default (the conformance suite's
`comptime/derive_default`, whose `Wrapper[T]`'s default of `count` is
`Wrapper#count[T]`, called with `[$0]` from the derived `Default`), which
prints what it printed. The case `cases/habits/generic_structs.sisu`
(its `.out`: it runs, interpreted and natively).

- Differs: `compiler/conformance/cases/comptime/derive_default.sisu`: sir, sir-tests
- Differs: `compiler/stage1/cases/habits/generic_structs.sisu`: ids, human, doc, doc-output

**Decision 325**, for docs/architecture.md:

325. A generic struct's construction takes its type arguments from the
     type expected, when that type is the struct's and holds no `dyn` and
     nothing untold, before its fields are checked (`Reading(21.5, "°C")`
     as a `Reading[f64, String]`); and a field's default in a generic
     struct is a value function generic as the struct is, which each
     construction calls with the struct's type arguments, where lowering
     named the struct's parameter in the caller (`E0501`). Stage 1 decides
     it (RULES.md, R26).

## R27. Typed holes: a release build rejects them, a run shows their closure, and a blocked suite says so (decision 340)

**The rule** (D-061; 10, "Holes"; 17, "Assertions": "When it reaches a
hole, `todo("name")`, the interpreter stops with the hole's closure (the
`Debug` rendering of every binding in scope), and the runner reports the
test as blocked by that hole, neither passed nor failed. ... Release
builds and `sisu publish` reject holes, so blocked is never a shipped
state"). Three parts:

- **A release build rejects each typed hole.** `sisu build --native
  --release` (and `sisu run --native --release`, and `sisu1 build
  --release`) of a program with a hole is `E0445` at each
  (`type.hole-in-release`, "a release build rejects the typed hole
  `todo("area")` of type `f64`: blocked is never a shipped state"), with
  the note `type.hole-placeholder` ("fill the hole before the release
  build; a placeholder meant to ship is `panic("...")`"), in place of its
  warning `W0001`, then `D0006`, and nothing is built. `check` and a debug
  build still warn.
- **A run that reaches a hole prints its closure** before the run time
  stops at it: the line `in scope at the typed hole `todo("area")`:`
  (`panic.hole-closure`), then `  NAME = VALUE` for each binding in scope
  there, in the order the function declares them, its value written as
  `{NAME:?}` writes it, and a closure's captures after its own. A binding
  is shown when the initialisation dataflow finds it holding its whole
  value at the hole (one not yet assigned, moved away or out of scope is
  not), when it is a parameter or holds no loan (a reference or view kept
  in a local may outlive what it borrowed, which the program no longer
  uses there), when its `Debug` is the one the host writes (a type with a
  `Debug` of its own, or a type parameter, is left out), and when it holds
  no capability. A hole with nothing to show prints nothing more. A test
  blocked by a hole prints the same, so the runner's report says which
  hole each test waits on and the closure says with what.
- **A suite with a blocked test and no failed one exits with status 3**
  (17 names three outcomes and no status; this rule gives the third its
  own): 0 when every test passed, 1 when a test failed (whatever else is
  blocked) or the program has an error, 3 when none failed and one is
  blocked by a hole. Stage 1's `sisu1 test [OPTIONS] [PATH]` checks and
  lowers the program with its tests as `sisu test` builds it, prints its
  diagnostics as `sisu test` does (a program with an error is not run,
  `D0006`), runs its SIR with stage 0's runner (`sisu test --from-sir`,
  decision 285) with the runner's options (`--jobs`, `--filter`, `--tag`,
  `--skip-tag`, `--no-monitor`, `--sim`, `--faults`, `--update`,
  `--no-cache`, `--schedule-seed`, `--explore`, `--scheduler`,
  `--depth`), passes its report on with each replay line naming the
  program, and exits with the status. What `--from-sir` leaves out stays
  out until the runner takes it from stage 1 (`--replay`, transcripts, the
  cache and `--defer-borrow`), and `sisu test` itself, whose runner and
  front end are stage 0's, exits as before: the driver's routing of `test`
  through stage 1, or its runner's status, is the stage-0 round's.

**Why.** The manual's writers found all three (X13 and G26, "Typed
holes"): `sisu build --native --release` built a program with a hole with
W0001 and status 0, `sisu test` exited 0 on blocked tests, so nothing
stopped a hole from shipping, and a blocked test and a run that reached a
hole printed the hole's name only. 17's sentence that blocked is never a
shipped state needs a build that refuses and a status a CI job can tell
from success; the closure is what 10 and 17 say the interpreter stops with.

**In stage 1.** `main.build_program` and `main.release_report` (the
release refusal); `lower.func.FnLower.hole_bindings` (the bindings whose
`Debug` the host writes, `sir.Function.shown`) and `borrowck.holes` (the
closure's statements, written once the ownership checker is done, after
compile-time evaluation, so that nothing it adds changes what the checker
reports or what a `comptime` expression does); `main.test_command`,
`test_program` and `test_summary`, and `sisu1`'s `test`. `check.sh`
holds a case to its `NAME.release` (what `build --native --release`
prints) and its `NAME.tests` (what `test` prints, standard output then
standard error, the replay tokens' hashes written `HASH`), and, for a
conformance case whose expectation a rule changes, to the file under
`stage1/expected/` at the case's path, which RULES.md lists as its output
`behaviour`.

**What it changes.** The SIR of the programs of the corpus that reach a
hole, which print the closure (the conformance suite's `types/typed_hole`,
whose `.err` gains it, `stage1/expected/conformance/cases/types/typed_hole.err`;
`run/testing`'s tests); nothing a debug build or `check` prints. The cases
`cases/holes/release.sisu` (its `.release` and `.diag`), `closure.sisu`
(its `.out`, `.err` and `.status`: it runs, interpreted and natively),
`blocked.sisu` and `failed.sisu` (their `.tests`).

Since stage 0's decision 466 the runner captures what a test prints and
shows it only with a failure, so a blocked test's closure, which the run
time prints as the test's output, is dropped with that output: the two
`.tests` are blessed again without it (stage 1's seventh round). Stage
0's decision 507 shows a blocked test's captured output in a block of its
own after the summary's lines, as it shows a failed one's, so the closure
is back, and the two `.tests` are blessed again with it (stage 1's eighth
round); the conformance suite's `testing/blocked_closure` (decision 507)
is the program whose closure stage 1's SIR prints, as the others'.

- Differs: `compiler/conformance/cases/run/testing.sisu`: sir, sir-tests
- Differs: `compiler/conformance/cases/testing/blocked_closure.sisu`: sir
- Differs: `compiler/conformance/cases/types/typed_hole.sisu`: sir, behaviour

**Documents to bring along:** the conformance suite's
`types/typed_hole.err` (the file `stage1/expected/` holds); 10's and 17's
sentences on holes, which can name the status; the manual's reference of
`sisu test` ("The status is 1 if a test failed, 0 otherwise, even when a
test is blocked") and of `sisu build`, and its exercise checker
(`tools/check.py`), which runs exercises whose tests start blocked and
reads status 3 as waiting; and the stage-0 round's runner, whose status
and whose `--from-sir` (replay, transcripts, the cache) are the rest.

**Decision 340**, for docs/architecture.md:

340. Typed holes (D-061): a release build rejects each with `E0445`, which
     names the hole and its type and the `panic("...")` that may ship,
     where `check` and a debug build warn (`W0001`); a run that reaches a
     hole prints its closure first, `in scope at the typed hole
     `todo("name")`:` and a line `NAME = VALUE` (`Debug`) for each binding
     that holds its whole value there and holds no loan or is a parameter,
     whose `Debug` the host writes and which holds no capability, a
     closure's captures after its own; and `sisu test` exits with 3 when
     no test failed and one is blocked, 1 when one failed, 0 when all
     passed. The closure is written into the SIR after the ownership
     checker and compile-time evaluation, which it changes in nothing.
     Stage 1 decides it (RULES.md, R27).

## R28. What `pure` code may not do: read an `Atomic`, or change what a `par` loop's body does not own (decision 341)

**The rule** (06, "Effects": "The operations that observe the schedule are
errors in `pure` code: `select`, `Mutex`, `Atomic`, ..."; "Data
parallelism": a `par` body "is `pure` ...: it may mutate its own element
and nothing shared. It cannot do I/O, take a lock, wait on a channel").
Two holes close:

- every method of an `Atomic` is an impure call in a `pure` function,
  `load` among them, which the prelude lowered to `Cell.get`, a pure
  intrinsic: `E0333` names the method as the program calls it
  (`Atomic.load`, `Atomic.store`, `Atomic.swap`,
  `Atomic.compare_exchange`, where it named the intrinsic, `set` or
  `replace`), with the note `type.observes-schedule`. Making one
  (`Atomic(0)`) observes nothing and stays pure;
- the body of `for x in par.iter(xs):` is `pure`: a call of a function
  that is not is `E0446` (`type.par-body-impure`), and a change of a
  binding declared before the loop, by assignment, a `mut` argument (a
  `push`) or a `ref mut`, `E0447` (`type.par-body-changes`, which names
  `par.map` and a reducer). The loop's element and the body's own
  bindings are its own.

**Why.** The manual's writers found both (chapter 26 and G17): `a.load()`
compiled in a `pure` function, and a `par.iter` loop's body printed and
summed into a variable outside it, which 06 makes impossible so that a
parallel loop's result is its serial elision's.

**In stage 1.** `borrowck.purity` (`atomic_method`); `typeck.purity`
(`is_par_loop`, `check_par_body`, `par_change`, `block_stmts_deep`),
called from `typeck.check` where a `for` is checked.

**What it changes.** Nothing in the corpus, which has no such program.
The cases `cases/purity/atomic.sisu` and `par_body.sisu` (their `.diag`).

- Differs: `compiler/stage1/cases/purity/atomic.sisu`: types, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/purity/par_body.sisu`: types, ids, human, doc, doc-output

**Decision 341**, for docs/architecture.md:

341. Every method of an `Atomic` but its construction is impure in `pure`
     code (`E0333`, named as the program calls it, with a note that it
     observes the schedule), as 06 lists `Atomic` among the operations
     that observe the schedule; and the body of a `par` loop (`for x in
     par.iter(xs):`) is `pure`: it calls only `pure` functions (`E0446`)
     and changes nothing declared before the loop (`E0447`). Stage 1
     decides it (RULES.md, R28).

## R29. A trait's contract binds every impl (decision 342)

**The rule** (07, "Interaction with the rest of the language": "Trait
methods may declare contracts; an implementation may weaken the
precondition and strengthen the postcondition (Liskov), which the
compiler checks ... by runtime double-checking otherwise"). A trait's
method may begin with `requires` and `ensures` clauses, which are the
contract of every impl of the method:

- an impl's method checks the trait's `requires` on entry, before its
  own, and the trait's `ensures` on exit, after its own: runtime
  double-checking, so that an impl that strengthens the precondition or
  weakens the postcondition fails at the clause it broke. The trait's
  clauses are checked in the impl's body with the trait's names of the
  parameters naming the impl's (an impl may call `amount` `by`), and
  their failure names the trait's method (`precondition of `Counter.add`
  failed: amount > 0`), wherever the call comes from: the type itself, a
  generic bound or a `dyn`;
- a trait's method whose body is its contract alone has no default: it is
  required, as one without a body is (`E0232` names it when an impl leaves
  it out), where it was a provided method that did nothing (or `E0315`,
  for one with a result). A provided method with a contract and a body
  binds the impls that write their own;
- a mistake in a trait's clause is the trait's, reported once where the
  trait is checked, not in each impl;
- an impl in another module than the trait's is held to the trait's
  contract too since R39; `W0021` (`contract.trait-elsewhere`) stays for
  one whose trait's module stage 1 has no syntax of, which no module of a
  program is.

**Why.** The manual's writer of chapter 23 found that "contracts on a
trait's methods ... are accepted and never checked, in either front end
and through a generic call too: a trait's `requires x >= 0` lets
`push(-1)` through without a word"; 07 makes them binding.

**In stage 1.** `typeck.Checker.inherit_contract` (the trait's method's
clauses, its parameters' names and the name a failure gives),
`typeck.contracts` (`contract_only`, `inherited_clause`, the trait's
clauses in `check_contracts`), `ContractCheck.owner`, which
`lower.contracts` names a failure by, and `typeck.collect`'s
`E0232`.

**What it changes.** Nothing in the corpus, whose traits declare no
contracts. The cases `cases/contracts/trait_contracts.sisu` (its `.out`,
`.err` and `.status`: it runs, interpreted and natively) and
`trait_contract_refusals.sisu` (its `.diag`).

- Differs: `compiler/stage1/cases/contracts/trait_contracts.sisu`: sir, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/contracts/trait_contract_refusals.sisu`: ids, human, doc, doc-output

**Decision 342**, for docs/architecture.md:

342. A trait's method's `requires` and `ensures` bind every impl (07):
     an impl's method checks the trait's preconditions on entry before its
     own and the trait's postconditions on exit after its own, with the
     trait's names of the parameters, and a failure names the trait's
     method; a trait's method whose body is its contract alone is
     required, not a default. An impl in another module than the trait's
     is warned that stage 1 does not check it yet (`W0021`). Stage 1
     decides it (RULES.md, R29).

## R30. A deprecated item warns where it is used, and an attribute nothing reads warns (decision 343)

**The rule** (13, "Deprecation": "Deprecated items warn with the
replacement named"; 02 and D-046: attributes are "compile-time data,
not run-time calls", which the toolchain and compile-time code read):

- each call of a function or method marked `@deprecated("...")`, and
  each use of one as a value, is `W0023` (`type.deprecated`, "`parse` is
  deprecated: use `parse_v2`"), the attribute's text naming the
  replacement, or `W0025` (`type.deprecated-plain`) when it gives none,
  but in the item's own body;
- an attribute whose name Sisu does not read (the list in
  `resolve.READ_ATTRIBUTES`, the attributes the documents give: `derive`,
  `deprecated`, `cfg`, `inline`, `export`, `must_consume`, `tag`,
  `repr`, `float`, `diagnostic_name`, `fixture`, `no_alloc`, `stack`, ...,
  and those only the standard library writes) is `W0024`
  (`resolve.misspelt-attribute`) when it is within the similar-name
  distance of one Sisu reads, with the fix that writes that one
  (`@deprecatd` for `@deprecated`), and otherwise `W0022`
  (`resolve.unknown-attribute`) when no compile-time code of the program
  reads attributes (no `reflect.attributes_of`), since nothing can read it
  then. A program whose compile-time code does read them keeps its own
  attributes as data, without a word (`@route` of the conformance suite's
  `reflect/queries`).

**Why.** The manual's writers found both (chapter 28, X11 and the
reference's R14): `@deprecated("...")` compiled without a word and its
calls got no warning, and any attribute was accepted, so a misspelt one
was too.

**In stage 1.** `typeck.collect` (`FnInfo.deprecated`, from the
attribute), `typeck.Checker.deprecated_uses`; `resolve` (`check_attributes`,
`attribute`, `READ_ATTRIBUTES`, `read_attribute`), run by both the
program's and a single module's name pass.

**What it changes.** Nothing in the corpus, whose attributes Sisu reads
or are read by the program's compile-time code. The cases
`cases/attributes/deprecated.sisu` (its `.diag`, and its `.out` and
`.err`: it runs) and `unknown.sisu` (its `.diag`). What the warnings do
not reach yet: a deprecated type, constant or variant, and `@fix_inline`'s
rewrite (`sisu fix --deps`, 10), which is a command of its own.

- Differs: `compiler/stage1/cases/attributes/deprecated.sisu`: ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/attributes/unknown.sisu`: names, ids, human, doc, doc-output

**Decision 343**, for docs/architecture.md:

343. A call of a function or method marked `@deprecated("...")`, or a
     use of one as a value, warns with the attribute's text (`W0023`, or
     `W0025` without one), but in its own body (13); an attribute whose
     name Sisu does not read warns: with the fix to the attribute it
     misspells (`W0024`), or, when no compile-time code of the program
     reads attributes, because nothing can read it (`W0022`). Stage 1
     decides it (RULES.md, R30).

## R31. `embed` names a file from the package's root (decision 344)

**The rule** (08, "Compile-time evaluation": "`embed("path")` as a
compiler intrinsic with the path resolved relative to the package root").
In a package, every module's `embed("data/words.txt")` reads
`data/words.txt` beside the package's `sisu.toml`, whatever directory
the module is in; a single file's, from the file's directory, as before.
A path that leaves the root (`..`, or an absolute one) stays `E0612`.

**Why.** The manual's writer of G30 found that a module of a package read
`src/data/words.txt` and that `../data/words.txt`, the package's file, was
`E0612`.

**In stage 1.** `main.embedded_files` (the root it resolves from) and
`main.layout_input`.

**What it changes.** Nothing in the corpus, whose `embed`s are a single
file's. The case `cases/packages/embed_root.pkg` (its `.out`: it runs),
which the Rust front end refuses (`E0612`, the file not being under
`src/`).

- Differs: `compiler/stage1/cases/packages/embed_root.pkg`: sir, ids, human, doc, doc-output

**Decision 344**, for docs/architecture.md:

344. `embed("path")` in a module of a package resolves the path from the
     package's root, the directory of its `sisu.toml` (08), whatever
     directory the module is in; a single file's from its directory.
     Stage 1 decides it (RULES.md, R31).

## R32. `json.decode` validates refinements and invariants at the boundary (decision 345)

**The rule** (11: "`Decode` validates untrusted input against the type
(including refinement predicates) at the boundary, so that "parse, don't
validate" is the default"; 05: an expected failure, such as malformed
input, is an error value, a bug a panic). `json.decode[T]` and
`json.from_json[T]` decode a refinement's base and check its predicate,
and a struct's fields and then its `invariant`: text that breaks either is
`Err(Mismatch(path, expected))`, the path naming where (`$.hour`, `$[2]`)
and `expected` the type (``a `Hour` ``, ``a `Span` whose invariant
holds``), where a refinement was a type error in the decoder's instance
(`E0811` at `<std>/<from_json[Hour]>/json.sisu`) and an invariant a panic
that stopped the program. Untrusted text that breaks a struct's
invariant is malformed input, which a correct program must handle, not a
bug of the program that reads it.

Reflection gains three queries for it, which any compile-time code may
ask: `reflect.is_refinement(T)`, `reflect.base_of(T)` and
`reflect.has_invariant(T)`. The decoder's construction of a struct does
not check the invariant, which its code checks with the struct's
`__invariant`, as the generators of property tests do (17).

**Why.** The manual's writer of G7 found both: a field of a refinement
type stopped the build in a line of the decoder the reader cannot see,
and text breaking an invariant made `decode` panic, "so untrusted text can
stop the program".

**In stage 1.** `stage1/std/json.sisu`, stage 1's copy of the module
(`typeck.std_sources.stage1_copy`), since the Rust front end, which reads
`compiler/std/json.sisu`, cannot evaluate the new queries;
`typeck.template` (`TypeShape.base` and `invariant`, the queries) and
`typeck.instances.shape`; `lower.contracts.invariant_after_construction`.

**What it changes.** The SIR of the corpus's programs that decode, whose
instances of `from_json` construct a struct into a local first, and what
`sisu doc --std` writes of `std.json` (its `##!` lines), which R4's line
for `--std` already lists. The case `cases/json/decode_validates.sisu`
(its `.out`: it runs, interpreted and natively).

- Differs: `compiler/conformance/cases/reflect/json.sisu`: sir
- Differs: `compiler/stage1/cases/json/decode_validates.sisu`: ids, human, doc, doc-output

**Documents to bring along:** 11's row of `std.json`, which can say that
an invariant is checked too; and `compiler/std/json.sisu`, which takes
stage 1's copy once the Rust front end is retired.

**Decision 345**, for docs/architecture.md:

345. `json.decode[T]` validates the text at the boundary (11): a
     refinement's predicate and a struct's invariant that the text breaks
     are a `Mismatch` naming the path and the type, not a type error in
     the decoder or a panic (05: malformed input is an expected failure).
     Reflection gains `is_refinement`, `base_of` and `has_invariant`.
     Stage 1 decides it, in its own copy of `std.json` (RULES.md, R32).

## R33. Exact numbers as written, and the floating-point attributes (decision 346)

**The rule.** Four of 03's promises:

- a float literal reaches a `Decimal` as written (03, "Numbers": "a
  literal reaches them as written: `0.1` as a `Decimal` is exactly one
  tenth"), with the places it is written with: `let price: Decimal =
  3.50` is `3.50`, from the literal's text (`Decimal.of_literal`, which
  `std.num` gains), where it was `3.5`, read from the shortest digits of
  its `f64`;
- `/` between two `Decimal`s keeps `E0310` and gets the note
  `type.decimal-quotient`, which names `a.div(b, places, mode)` and
  `a.to_ratio()`: the quotient is rounded where it is made (D-076), and
  the places are the program's choice, so the note names them rather than
  a fix choosing them;
- `@float(fast)` permits contraction and reassociation in its function
  (03), which strict floating point satisfies, so it is accepted; any
  other argument is `E0448` (`resolve.float-attribute`);
- `@diagnostic_name(f)` waits for compile-time type construction (D-065:
  "once Q-27's type construction ships"), so it is `E0900`, where it was
  accepted and did nothing.

And `list[T].is_sorted()` where `T: Ord`, a method of the prelude's
`list` (07's `requires items.is_sorted()` on a list), which a slice had.

**Why.** The manual's writers found each (chapters 23 and 24, chapter 25,
the reference's R2, G31): a `Decimal` literal lost its places, `/` between
`Decimal`s named nothing, `@float(fast)` and `@diagnostic_name` were
accepted without a word, and `is_sorted` was a slice's only.

**In stage 1.** `typeck.expr.literal_via_trait` (the literal's text) and
`is_std_named`, the binary operator's note; `resolve.attribute`
(`@float` and `@diagnostic_name`); `compiler/std/num.sisu`
(`Decimal.of_literal`) and `compiler/std/prelude.sisu` (`list.is_sorted`),
which the Rust front end reads too, unchanged for it but for the new
methods it does not call.

**What it changes.** The SIR of the conformance suite's `std/num_exact`,
whose `0.075` as a `Decimal` is made from its text, printing what it
printed, and of `std/decimal_literals`, which the stage-0 round added to
hold the literals' places natively (decision 387). The cases `cases/numbers/decimal_literals.sisu` (its `.out`: it
runs) and `refusals.sisu` (its `.diag`).

- Differs: `compiler/conformance/cases/std/decimal_literals.sisu`: sir
- Differs: `compiler/conformance/cases/std/num_exact.sisu`: sir
- Differs: `compiler/stage1/cases/numbers/decimal_literals.sisu`: sir, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/numbers/refusals.sisu`: names, ids, human, doc, doc-output

**Decision 346**, for docs/architecture.md:

346. A float literal reaches a `Decimal` from its text, its places kept
     (`3.50` stays `3.50`, 03); `/` between `Decimal`s names `div` and
     `to_ratio` in a note (D-076); `@float(fast)` is accepted, strict
     floating point satisfying the permission it gives, and `@float` with
     any other argument is `E0448`; `@diagnostic_name` is `E0900` until
     compile-time type construction (Q-27); and a `list` of an `Ord` type
     has `is_sorted`. Stage 1 decides it (RULES.md, R33).

## R34. A plain `for` over a compile-time list gets `comptime for` (decision 347)

**The rule** (08, "Reflection": `comptime for` unrolls a loop over what
compile time knows). `E0609` for a plain `for` over a compile-time list
("a list is a compile-time value with no run-time form") gets the fix
`comptime.write-comptime-for`, which writes `comptime ` before the `for`,
when the template is in the call's file.

**Why.** The manual's writer of chapter 25 found the error without the fix
that names the loop the template meant.

**Decided against:** the same writer found that `reflect.fields_of(int)` is
an empty list, so a template that forgets `if comptime
reflect.is_struct(T)` compiles and makes empty output. 08 says what
reflection exposes but not what a type without fields gives, and the
conformance suite's `reflect/methods` holds `fields_of(str)` to an empty
list (its `count_fields[str]()` prints 0): a type declared without fields
has none, as a struct has no variants. Stage 1 keeps it.

**In stage 1.** `typeck.instances` (`plain_for_over`, the fix).

**What it changes.** Nothing in the corpus. The case
`cases/comptime/plain_for.sisu` (its `.diag`).

- Differs: `compiler/stage1/cases/comptime/plain_for.sisu`: ids, human, doc, doc-output

**Decision 347**, for docs/architecture.md:

347. `E0609` for a plain `for` over a compile-time list offers the fix
     that writes `comptime for` (08). Stage 1 decides it (RULES.md, R34).

## R35. `max` and `min` of a collection (decision 348)

**The rule** (02, "Prelude functions and intrinsics": Python's builtins
`min` and `max` are kept, "ordinary generic functions over traits").
`max(xs)` and `min(xs)`, one argument given by position, are the largest
and the smallest item of a collection or an iterator by its `Ord`, the
first of equal ones, as Python's are: the prelude's `max_of` and `min_of`,
which a call of `max` or `min` with one argument is. A collection with no
item panics (`` `max` of no items ``), as Python's raises. `max(a, b)`
and `min(a, b)` compare two values as before (03).

**Why.** The manual's writer of chapter 5 found `max(scores)` refused
with `E0308` and a fix that added a hole, though 02's list of prelude
functions is Python's.

**In stage 1.** `typeck.call.extreme_of`, from `call_def`; the prelude's
`max_of` and `min_of` (`compiler/std/prelude.sisu`), which the Rust front
end reads too and calls nowhere.

**What it changes.** Nothing in the corpus. The case
`cases/numbers/max_min.sisu` (its `.out`, `.err` and `.status`: it runs).

- Differs: `compiler/stage1/cases/numbers/max_min.sisu`: ids, human, doc, doc-output

**Decision 348**, for docs/architecture.md:

348. `max(xs)` and `min(xs)` with one argument are the largest and the
     smallest item of a collection, the first of equals, and panic for
     none, as Python's raise (02): the prelude's `max_of` and `min_of`.
     Stage 1 decides it (RULES.md, R35).

## R36. `@derive(Encode, Decode)` (decision 349)

**The rule** (11: `std.json` is an "encoder/decoder with
`@derive(Encode, Decode)`"). `std.json` declares the traits `Encode`
(`encode_json(self) -> String`) and `Decode` (`decode_json(j: Json, path:
str) -> Self ! JsonError`), and `encode` and `decode` write and read a
value of a type that implements one through it, inside other values too.
`@derive(Encode)` gives a struct or an enum, and `@derive(Decode)` a
struct, the form `encode` and `decode` give any type of its shape (by
fields, or by variant; `decode` checking refinements and the invariant,
R32), saying in the declaration that JSON is one of the type's forms; an
impl written by hand gives the type a form of its own (a `Money` as
`"12.50"`).

**Why.** The manual's writer of G7 found `@derive(Encode, Decode)` refused
(`E0243`).

**In stage 1.** `stage1/std/json.sisu` (the traits, whose methods reflect
so that each impl gets its own, `encode_shape` and `decode_struct`, and
the first branch of `encode` and `from_json`); `typeck.instances`, which
unfolds the derive of a std module that has templates from the text the
base keeps of it.

**What it changes.** The SIR of the corpus's programs that encode or
decode, whose instances take the new branches, and stage 1's case of the
bound on a template's instances in the standard library, whose label is
at `json.encode_shape` now. The case `cases/json/derive_encode_decode.sisu`
(its `.out`: it runs).

- Differs: `compiler/stage1/cases/instances_unbounded_in_std.sisu`: ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/json/derive_encode_decode.sisu`: names, ids, human, doc, doc-output

**Decision 349**, for docs/architecture.md:

349. `std.json` declares `Encode` and `Decode`, which `encode` and
     `decode` use for a type that implements them, inside other values
     too; `@derive(Encode, Decode)` gives a type the form of its shape and
     an impl written by hand a form of its own (11). Stage 1 decides it
     (RULES.md, R36).

## R37. A trait's associated constants (decision 361)

**The rule** (03, "Traits": "A trait declares required and provided
methods, associated types, and associated constants"). A trait's body may
declare constants, `const NAME: T`, which every impl gives, or `const
NAME: T = value`, whose value an impl may replace:

```sisu
trait Shape:
    const SIDES: int
    const CORNERS: int = Self.SIDES

impl Shape for Square:
    const SIDES: int = 4
```

- a constant is read through a type: `T.NAME` for a type parameter bounded
  by the trait (through a supertrait too), `Self.NAME` in the trait's and
  its impls' methods and in a default's value, and `Square.NAME` for a
  type that implements the trait, whose impl's value it is, or the
  trait's when the impl gives none; `Shape.SIDES`, through the trait, is
  `E0449` (`type.trait-constant`), since each impl gives its own;
- an impl gives each constant the trait declares without a value
  (`E0232`, whose note writes `const SIDES: int = ...`), and no other
  (`E0271`, `resolve.not-a-trait-constant`), with the trait's type
  (`E0272`, `resolve.constant-type`), as a constant: a constant written as
  a method, or a method as a constant, is `E0273`
  (`resolve.const-or-method`);
- a trait with a constant is no `dyn` type (`E0241`: a `dyn` value does
  not say which type's constant it would read), as one with an
  associated function is;
- a constant has a type (`E0219`) and a SCREAMING_CASE name (`E0269`, R12),
  in a trait, an impl or a type's body alike, where the name pass checked
  a module's constants only; its value is checked as the body of a
  `pure` function, which it is: the function that gives the constant,
  `pure def NAME() -> T: return value`, a method of the trait without
  `self` that each impl gives, so a read through a type parameter is the
  trait's call made for the instance (`call trait Shape.SIDES[$0]()` in
  SIR, `Square.Shape.SIDES` in the impl's table). A value is computed
  where it is read, as a module's constant's is.

**Why.** The manual's writer of chapter 14 found it E0100 without a value
and, with one, unreadable as `Self.SIDES` or `Square.SIDES` (E0351), and
its reference says "designed and missing" of it.

**In stage 1.** The parser (`Parser.in_trait_body`, `const_def_with`,
which leaves the value out in a trait's body); the AST's
`TraitDef.consts` (printed by `parse --ast` only when there are any);
`resolve.member_const` (the name, types and value of a constant of a
trait, an impl or a type's body); `typeck.collect` (`const_fn`, the
constants of a trait in `resolve_trait` and of an impl in
`resolve_impl`, `check_const_type` and the checks of `check_traits`,
`FnInfo.assoc_const`); `typeck.call` (the read in `static_member`,
`E0449` in `trait_path_call`); `typeck.program.object_safety_problem`;
and `doc.trait_section`, which lists a trait's constants.

**What it changes.** Nothing in the corpus, which declares no trait
constant. The cases `cases/traits/assoc_consts.sisu` (its `.out`: it
runs, interpreted and natively), `assoc_const_refusals.sisu` (its
`.diag`) and `cases/packages/assoc_consts.pkg` (the trait in one module,
an impl in another, the read in a third; its `.out`).

- Differs: `compiler/stage1/cases/packages/assoc_consts.pkg/src/shape.sisu`: tree, ast, fmt-check
- Differs: `compiler/stage1/cases/packages/assoc_consts.pkg`: types, sir, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/traits/assoc_const_refusals.sisu`: tree, ast, names, ids, human, fmt-check, doc, doc-output
- Differs: `compiler/stage1/cases/traits/assoc_consts.sisu`: tree, ast, ids, human, fmt-check, doc, doc-output

**Decision 361**, for docs/architecture.md:

361. A trait declares associated constants (03): `const NAME: T`, which
     every impl gives, or with a value an impl may replace; each is read
     through a type, `T.NAME` under a bound, `Self.NAME`, or `Type.NAME`,
     and is the trait's `pure` function without `self` that gives it, made
     for the type at the read. An impl gives each constant the trait
     leaves without a value, no other, with the trait's type and as a
     constant (`E0232`, `E0271` to `E0273`); a read through the trait is
     `E0449`, and a trait with a constant is no `dyn` type. Stage 1
     decides it (RULES.md, R37).

## R38. `expect` records its failure and the test goes on (decision 362)

**The rule** (17, "Assertions": "`expect expr` is the non-fatal form: the
test continues and fails at the end"; 02: "`expect cond` # in tests:
records the failure and continues"). In a program built with its tests
(`sisu test`, `sisu bench`, `sir --tests`), `expect cond` lowers as
`assert cond` does, its condition and the parts or the two sides it
shows, but a failure calls `expect.fail(text, message)`,
`expect.fail_compare(text, message, left, right)` or
`expect.fail_parts(text, message, parts)`, which take what `assert.fail`,
`fail_compare` and `fail_parts` take and return `()`: the run records the
failure in the running test's report, as `assert`'s would be rendered,
and goes on after the statement. The test fails at its end with every
failure it recorded, whatever it does after (it passes no other way, as a
test blocked by a hole is blocked whatever it does after, decision 359),
and `expect` in a helper the test calls records in the same test.
`assert` stops the test as before. Outside a build with its tests (`sisu
run`, `sisu build`, `sir`) no test is running to report at its end, so
`expect` stops the run as `assert` does, as it always did; and
compile-time code stops at a failed `expect` as at a failed `assert`.

**Why.** The manual's writer of chapter 19 found that `expect` stops a
test at its first failure, as `assert` does, and the runtime round, which
runs `sisu test` from stage 1's SIR (decision 357), named what it needs of
the front end (port-debt.md, "Tests and benchmarks from stage 1's SIR"):
`expect` lowered to intrinsics of its own that record the failure and
return.

**In stage 1.** `TStmt.Assert.soft` (`typeck.check`), `Program.with_tests`
(set from the resolver's `tests`), `lower.stmts.assert_` and
`lower.contracts.assert_showing_parts` (the intrinsics, and the jump to
the statement after where `assert`'s ends the block); `interp`, which
stops compile-time code at them. `check.sh` holds a case to its
`NAME.sir-tests`, what `sir --tests` prints of it.

**What it changes.** Nothing in the corpus, which writes no `expect`
statement. The case `cases/testing/expect.sisu`: its `.sir-tests`, the
SIR of its test with the three intrinsics, and its `.out` (its `main`
runs, interpreted and natively, where the `expect` holds).

**What the stage-0 side must do.** Stage 0's interpreter does not know
the three intrinsics yet, so a test whose `expect` fails stops with
"internal error in the interpreter: unknown intrinsic `expect.fail`" and
is reported failed: the interpreter renders each as `assert_fail`
renders its `assert` form and keeps it with the statement's location
(its frame's stack), the runner (`run_test`) reports a test that kept a
failure `Failed` with each failure in order, after the test returns or
before the panic that ended it, and the native table (`sisu_rt::abi`,
stage 2's) needs none, since a native build is no build with its tests.

**Decision 362**, for docs/architecture.md:

362. `expect cond` in a program built with its tests (`sisu test`) lowers
     as `assert cond` does but for the failure, which calls
     `expect.fail`, `expect.fail_compare` or `expect.fail_parts` with
     `assert`'s arguments: the run records the failure in the running
     test's report and goes on, and the runner fails the test at its end
     with every failure recorded (17). `assert` stops the test as before;
     outside a build with its tests, and at compile time, `expect` stops
     as `assert` does. Stage 1 decides it (RULES.md, R38); the interpreter
     and the runner implement the three intrinsics.

## R39. A trait's contract binds an impl of another module (decision 363)

**The rule** (07, as R29: a trait's method's contract binds "every
implementation"). An impl's method checks the trait's `requires` and
`ensures` wherever the impl is, in another module of the package or
another package's than the trait: the trait's clauses are checked in the
syntax of the trait's module, with the impl's parameters under the
trait's names of them, so a clause's names (a private helper of the
trait's module, a constant) are the trait's module's; `old(e)` saves `e`
on entry; a failure names the trait's method and shows the clause's
parts as the trait's text writes them; and the check's statements stand
at the impl's method's name, in the impl's file, whose lines the clause
is not on. `W0021` stays for an impl whose trait's module stage 1 has no
syntax of, which no module of a program is.

**Why.** R29 left it: "an impl in another module than the trait's is
warned that stage 1 does not check it yet (`W0021`)", since a checker of
stage 1 held the syntax of its own module only; the third round's "Still
open" list asks for the check.

**In stage 1.** `typeck.contract_syntax` (the trait's module's syntax
and text for a method of an impl of a trait of another module, given to
`check_one` beside the method's own), `Checker.inherit_contract`,
`check_contracts` and `inherited_clause`, which check the clause there
(`Checker.inherited_module`, `clause_module`), `ContractCheck.module` and
`TContracts.foreign_olds`; `lower.contracts` (`foreign_loc`,
`FnLower.fixed_loc` and `text_module`, which `emit` and `terminate`
read).

**What it changes.** Nothing in the corpus, whose traits declare no
contracts. The case `cases/packages/trait_contract_elsewhere.pkg` (its
`.out`, `.err` and `.status`: it runs, interpreted and natively), the
trait in one module, two impls in another, one of which breaks the
postcondition through a generic call.

- Differs: `compiler/stage1/cases/packages/trait_contract_elsewhere.pkg`: types, sir, ids, human, doc, doc-output

**Decision 363**, for docs/architecture.md:

363. A trait's contract binds an impl of any module (07): the impl's
     method checks the trait's clauses as R29's rule says, checked in the
     syntax of the trait's module with the impl's parameters under the
     trait's names, its failure naming the trait's method and showing the
     clause's parts, its statements standing at the impl's method's name.
     `W0021` remains only for a trait whose module's syntax the check
     does not have. Stage 1 decides it (RULES.md, R39).

## R40. Deprecated types, constants and variants; an attribute where it means nothing (decision 364)

**The rule** (13, "Deprecation": "Deprecated items warn with the
replacement named"; D-046: attributes are data the toolchain reads, for
the declarations they are written for). Three parts:

- **A deprecated type, constant or variant warns where it is used**, as a
  deprecated function does since R30 (`W0023` with the attribute's text,
  `W0025` without): a struct, an enum, an error set or a type alias (a
  refinement too) named in a function's signature, a field's or a
  variant's type, an annotation (`let q: Point`) or before `.`
  (`Point.origin()`), or built (`Point(x=1, y=2)`, at the name); a
  module's constant read; a variant built or matched (`Light.Lime`, `case
  Light.Lime:`), its name written `Light.Lime`. The type's own
  declaration may use it: its fields, its methods, its impls and their
  headers, as a function's own body may call it.
- **An attribute Sisu reads that stands where it means nothing warns**
  (`W0026`, `resolve.misplaced-attribute`, "`@derive` means something on a
  struct, an enum or an error set, so on a function it does nothing"):
  `@derive` and `@must_consume` belong on a struct, an enum or an error
  set, `@repr` and `@persisted` on a struct or an enum, `@delegate` on a
  struct, `@export`, `@inline`, `@float`, `@fix_inline`, `@fixture`,
  `@refines`, `@no_alloc`, `@no_block`, `@no_panic` and `@stack` on a
  function, `@comptime_budget` on a function or a constant, `@tag` on a
  test, a benchmark or a suite, `@deprecated` on a function, a type, a
  constant or a variant, and `@tile`, `@unroll` and `@vectorize` on a
  loop nest (which no declaration is). `@cfg` and `@preview` mean
  something anywhere; the attributes only the standard library writes
  (`@sealed`, `@marker`, `@intrinsic`, ...) are `E0225` in a program.
- **A suite's `@tag`s tag each of its tests** (17: "`@tag` selects and
  skips"), before a test's own, where they tagged none (`sir --tests`
  writes them on each test's line, which the runner selects by).

**Why.** The manual's writers found them (chapter 28, X11 and the
reference's R14): R30 warned at a deprecated function's calls only, and
an attribute Sisu reads was accepted wherever it stood, `@derive(Debug)`
or `@must_consume` on a function, `@comptime_budget` or `@export` on a
struct, and `@tag("slow")` on a suite, which tagged none of its tests.

**In stage 1.** `typeck.collect` (`deprecate`, `Program.deprecated` and
`deprecated_variants`, `deprecated_type_use` from `def_as_type`, with
`decl_owner` and `quiet_deprecation` for a type's own declaration and
its impls' headers; `suite_tags`); `typeck.Checker.deprecated_uses`
(`deprecated_item_use`, `deprecated_variant_use`), with the checker's
`resolve_ty` and `static_member`; the AST's `VariantDef.attrs` (printed by
`parse --ast` only when there are any); `resolve.attribute_places` and
`declaration_kind`, which the name pass's `check_attributes` reads.

**What it changes.** Nothing in the corpus, whose attributes stand where
they mean something and which deprecates nothing. The cases
`cases/attributes/deprecated_items.sisu` (its `.diag`, and its `.out`: it
runs), `misplaced.sisu` (its `.diag`) and `suite_tags.sisu` (its
`.sir-tests`, the tags on each test's line).

- Differs: `compiler/stage1/cases/attributes/deprecated_items.sisu`: ast, ids, human
- Differs: `compiler/stage1/cases/attributes/misplaced.sisu`: names, ids, human

**Decision 364**, for docs/architecture.md:

364. A type, a constant or a variant marked `@deprecated("...")` warns
     where it is used, as a deprecated function does (`W0023`, `W0025`):
     a type named or built, a constant read, a variant built or matched,
     but in the type's own declaration and impls (13). An attribute Sisu
     reads that stands on a declaration it means nothing on warns
     (`W0026`, naming where it means something); and a suite's `@tag`s
     tag each of its tests. Stage 1 decides it (RULES.md, R40).

## R41. `@derive(Arbitrary)` and an impl of `Arbitrary` written by hand (decision 365)

**The rule** (17, "Property-based testing and fuzzing": "generators are
derived for every type (`arbitrary[T]`; with `@derive(Arbitrary)` for
custom shapes, and refinement predicates honoured as filters)"; a
property test is `check(arbitrary[T]) do x:`, as 17 writes it). The
prelude declares the trait

```sisu
trait Arbitrary:
    def arbitrary(d: Draws) -> Self
```

- an impl written by hand draws a type its own way (`impl Arbitrary for
  Money:` building a value from `Draws.gen_in(d, 0, 10000)`), and
  `arbitrary[Money]` and the generator of every type of the module that
  holds a `Money` draw it through the impl, where they drew its fields;
- `@derive(Arbitrary)` gives a struct or an enum without type parameters
  the impl that draws its shape, the generator `arbitrary[T]` writes for
  any type (a struct's fields, then its invariant; an enum's variant, then
  its fields), so that a bound `T: Arbitrary` draws it too
  (`T.arbitrary(d)`); a type with a part that has no generator, or with
  type parameters, is `E0820` (`test.no-generator`) at the derive, which
  `E0243` refused;
- a type needs neither to be drawn: `arbitrary[T]` draws the shape of any
  type of the module, as before.

**Why.** The manual's writer of G11 found `@derive(Arbitrary)` refused
(E0243) and no way to draw a type its own way ("the generator of a custom
shape"), though `Gen.of(make)` builds one generator from a function.

**In stage 1.** `compiler/std/prelude.sisu` (the trait, which the Rust
front end reads too); `typeck.arbitrary.declare_generators` (the types an
impl written by hand draws, whose generator calls it, and the derived
impls, written beside the generators in `<arbitrary>/main.sisu`;
`derives_arbitrary`, `arbitrary_impl_of`, `Writer.shape_why`); and
`typeck.instances`, which leaves the derive to them.

**What it changes.** Nothing in the corpus, which derives no `Arbitrary`.
The cases `cases/testing/derive_arbitrary.sisu` (its `.tests`: three
properties hold, and one fails at the custom generator's boundary, `Money(cents=5000)`)
and `arbitrary_refusals.sisu` (its `.diag`).

- Differs: `compiler/stage1/cases/testing/arbitrary_refusals.sisu`: ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/testing/derive_arbitrary.sisu`: ids, human, doc, doc-output

**Decision 365**, for docs/architecture.md:

365. The prelude's `Arbitrary` (`def arbitrary(d: Draws) -> Self`) draws a
     type for property tests (17): an impl written by hand draws it its
     own way, which `arbitrary[T]` and the generators of the types that
     hold one use; `@derive(Arbitrary)` gives a struct or an enum without
     type parameters the impl that draws its shape, the generator
     `arbitrary[T]` writes for any type, and is `E0820` for one with a
     part that has no generator. Stage 1 decides it (RULES.md, R41).

## R42. A generator expression is a lazy `Iter` (decision 366)

**The rule** (02, "Comprehension": `(x for x in xs)` is a "lazy `Iter`",
beside the list, dict and set forms). `(f(x) for x in xs if c(x))` is an
`Iter[T]`, `T` the element's type: the generator whose body runs the
clauses as loops and tests around a `yield` of the element, as a
generator function's body runs (a task-like closure, D-056's generators),
so `sum((x for x in xs if x > 0))`, a `for` over it, its `collect()` and
any other `Iter` method take one item at a time. It borrows what it reads
of the function's own while it lives, as a loop over it would, and copies
what is `Copy`: what it reads cannot change meanwhile (`E0405`), and it
cannot outlive a local it reads (`E0407`), but it may borrow a parameter
(`def evens(xs: list[int]) -> Iter[int]: return (x for x in xs if ...)`).
Its clauses run when the `Iter` is first advanced, its first iterable
among them, which Python evaluates where the expression is. Python's bare
form as a call's one argument, `sum(x for x in xs)`, stays a syntax error
(`E0100`); the parentheses are 02's.

**Why.** The manual's writer of chapter 16 found it `E0900`, "as
planned", and then `E0304`, since the checker typed it as a list; the
third round named the missing piece, a closure that yields, which a
generator function's body already is.

**In stage 1.** `typeck.expr.generator_comprehension`, from
`comprehension`: the closure (`ClosureRole.Task`, its captures decided as
a task's), its loops, tests and `yield`, and `TExprKind.Generator`,
which lowering makes the `gen.new` of the closure, as for a generator
function.

**What it changes.** Nothing in the corpus, which writes no generator
expression. The cases `cases/generators/generator_expressions.sisu` (its
`.out`: it runs, interpreted and natively) and `generator_borrows.sisu`
(its `.diag`).

- Differs: `compiler/stage1/cases/generators/generator_borrows.sisu`: ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/generators/generator_expressions.sisu`: ids, human, doc, doc-output

**Decision 366**, for docs/architecture.md:

366. A comprehension in parentheses, `(f(x) for x in xs if c(x))`, is a
     lazy `Iter[T]` (02): a generator whose body runs the clauses as loops
     and tests around a `yield` of the element, task-like as a generator
     function's body is, borrowing what it reads of the function's own
     while it lives and copying what is `Copy`; its clauses run when it is
     first advanced. Stage 1 decides it (RULES.md, R42).

## R43. Forms the documents give that the checker refused, the manual's fourth round (decision 367)

**The rule.** Five forms the documents give, which stage 1 refused, and
the declarations it let through:

- **A slice bound counted from the end** (02: `a[i:^1]`): `xs[1:^1]`,
  `xs[^2:]` and `xs[:^3]` of a list are its items from or up to `len(xs) -
  k`, where `^` stood only inside an index (`E0383`).
- **A constant in an enum's body** (`const COUNT: int = 3`) is read as
  `Light.COUNT`, as a struct's is, where it was declared and unreadable
  (`E0351`).
- **A method of a `const` parameter's value**: `N.to_f32()`, `N` an `int`
  parameter of the function, is the method of the number, where it was
  looked up as an associated function of a type `N` (`E0351`).
- **Two sized integers compare as they add** (D-016): the narrower widens
  when that loses nothing, so `small < wide`, a `u8` and an `i32`, is
  `True` where it was `E0369`; a chain widens each pair.
- **A number or text on the left of an operator** reaches an impl of the
  operator for it when the right is a type of the program's own: `2.0 *
  v` with `impl Mul[Vector] for f64`, which `E0310` refused whatever the
  impls.
- **A supertrait cycle through other traits**, `trait A(B)` and `trait
  B(A)`, is `E0247` at each trait, as `trait A(A)` is, where it compiled.
- **A field or a variant declared twice** is `E0205` or `E0206`, as stage
  0 reports it, where stage 1 kept one of the two and said nothing. The
  second variant is left out of the derived impls, so stage 0's `W0003`
  at the arm its derived impl writes for it, at `<derive>/main.sisu`, a
  text the program does not have, has no counterpart.

**Why.** The manual's writers found each (chapters 5, 14 and 27, the
reference's R2 and R3; "Syntax and names", "Types and traits").

**In stage 1.** `typeck.expr` (`slice_bound`, `TExprKind.Slice.lo_end`
and `hi_end`; `compare_expr`; `operator_call`), `lower.exprs.from_end`;
the AST's `EnumDef.consts` (printed by `parse --ast` only when there are
any), `typeck.collect.declare_enum`, `resolve`'s enum members and
`doc.enum_section`; `typeck.call.static_path` (a `const` parameter is a
value); `typeck.collect.check_trait_cycles`, `resolve_struct` and
`resolve_enum`.

**What it changes.** Nothing in the corpus, which writes none of these.
The cases `cases/findings4/forms.sisu` (its `.out`: it runs, interpreted
and natively), `supertrait_cycle.sisu` and `duplicates.sisu` (their
`.diag`).

- Differs: `compiler/stage1/cases/findings4/duplicates.sisu`: ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/findings4/forms.sisu`: ast, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/findings4/supertrait_cycle.sisu`: ids, human, doc, doc-output

**Decision 367**, for docs/architecture.md:

367. A list's slice takes a bound counted from the end (`xs[1:^1]`, 02);
     an enum's body declares constants as a struct's does (`Light.COUNT`);
     a `const` parameter's value has the methods of its number
     (`N.to_f32()`); two sized integers compare as they add, the narrower
     widened when that loses nothing (D-016); a number or text on an
     operator's left reaches an impl of the operator for it when the right
     is a type of the program's (`2.0 * v`); a supertrait cycle through
     other traits is `E0247`; and a field or a variant declared twice is
     `E0205` or `E0206`, as in stage 0, the second variant left out of the
     derived impls. Stage 1 decides it (RULES.md, R43).

## R44. `@cfg` on the members of a struct or an enum (decision 368)

**The rule** (08, "Conditional compilation": `@cfg` on items; decision
243). A field, a variant or a method in a struct's or an enum's own body
whose `@cfg(...)` does not hold is left out of the build, as an item and
a method of an impl or a trait are: a build without its tests has no
`@cfg(test)` field, variant or method of the type, and its SIR declares
the type without them. A field left out takes its default nowhere; one
the build keeps is built and read as any other.

**Why.** The manual's writer of the reference's R14 found `@cfg(test)` on
a field, a variant or a method in a struct's or an enum's own body kept
in every build, where a method of an impl was left out.

**In stage 1.** `cfg.Filter.items` and `cfg.affected` (the struct's and
the enum's members), the AST's `FieldDef.attrs` (printed by `parse --ast`
only when there are any) and `VariantDef.attrs` (R40); the name pass
leaves out their nodes as it leaves out an item's (`Tree.without`).

**What it changes.** Nothing in the corpus, which puts no `@cfg` on a
member. The case `cases/findings4/cfg_members.sisu` (its `.out`, the run
of the build without its tests, interpreted and natively, and its
`.tests`, the build with them).

- Differs: `compiler/stage1/cases/findings4/cfg_members.sisu`: ast, doc, doc-output

**Decision 368**, for docs/architecture.md:

368. `@cfg` on a field, a variant or a method in a struct's or an enum's
     own body leaves it out of a build its condition does not hold in, as
     it leaves out an item and a method of an impl (08). Stage 1 decides
     it (RULES.md, R44).

## R45. The fixes and notes the manual's fourth round found missing (decision 369)

**The rule.** Each a fix or a note on an error stage 1 already reports,
as R16 and R24 gave theirs:

- `'Aino'`, Python's text in single quotes (`E0016`): the fix writes
  double quotes;
- `Light.Gren`, a CamelCase name after an enum or an error set without
  such a variant: `E0327` (`type.unknown-variant`), as in a pattern, with
  the variant it most likely means, where it was `E0351`;
- `int("42")`, `str(n)`, `float(x)` and `bool(x)`, Python's conversions
  (`E0200`): the note `resolve.python-conversion` names the methods
  (`s.parse_int()`, `n.to_string()`, `x.to_int()`, ...), and `uint` is no
  longer offered for `int`;
- a value where its option is expected, `let n: int? = 5` (`E0300`): the
  fix `Some(5)`;
- an `int` where a float is expected (`E0300`): the fix `.to_f64()` (or
  `.to_f32()`), where the note named an `=` that a `return` does not have;
- `"0" * 64` (`E0310`): the fix `"0".repeat(64)`;
- an operator on a type of the program's own without the impl (`E0310`):
  the note `type.operator-trait` names the trait and the impl that gives
  it (`impl Sub[Money] for Money`);
- a misspelt keyword argument, `prot=443` (`E0306`): the parameter it most
  likely means, as a misspelt name has;
- an argument by position past the positional parameters, where the next
  is keyword-only (`E0305`): the note `type.keyword-only` names it;
- `??` on a result (`E0371`): the fix `.or(...)`;
- `?.` after a result, Rust's `read(path)?.lines()` (`E0370`): the note
  `type.question-dot` (`?.` is one token; `(x?).lines()` passes the error
  on first);
- an assignment to a parameter of a `Copy` type (`E0312`): besides `mut`,
  the fix that changes a copy the body declares, `var n = n`;
- an item moved out of a list the function owns, `let first = queue[0]`
  (`E0338`): besides `.copy()`, the fix `queue.remove(0)`;
- a field of a `Shared` assigned (`E0341`): the note
  `type.shared-changes` (a `Cell` or a `Mutex` inside it), not the one about
  `ref mut`;
- a capture of an `own` closure that is not `Copy` (`E0423`): the note
  `type.capture-borrow`, a plain closure, where the note's copy made
  first was `E0423` again.

**Why.** The manual's writers found each ("Diagnostics without fixes",
chapters 2 to 4, 10, 13, 14, 16, 18, 21 and 22, the reference's R1 and
R17; "Ownership").

**In stage 1.** `lexer.scan.char_literal`; `typeck.call` (`missing_variant`,
`check_args`'s keyword arguments and count); `resolve.use_name`;
`typeck.expr` (`coerce`, `mismatch_fix`, `binary_result`, `coalesce`, the
optional chain, `check_movable`, `check_own_closures`); `typeck.check`
(`require_mutable_as`, `body_start`, `column_text`).

**What it changes.** The diagnostics of the corpus's programs that make
these mistakes and of stage 1's older cases that do (`lexical_mistakes`,
`unicode_escape_sign`, `type_calls`, `type_arguments_in_values`), as the
lines below list them: the tokens and the tree of a file whose
diagnostics carry `E0016`'s fix, which their dumps print, and its
formatter's verdict, which prints them too. The conformance suite's
`.diag` files of `syntax/unicode_escape_sign` (`E0016`'s fix) and
`types/argument_count` (the misspelt keyword's), which hold stage 1's
`check --format=ids`, are blessed again with the fix. The cases
`cases/findings4/fixes.sisu` and `fixes_lexical.sisu` (their `.diag`).

- Differs: `compiler/conformance/cases/ownership/own_closure_errors.sisu`: human, doc, doc-output
- Differs: `compiler/conformance/cases/ownership/shared_immutable.sisu`: human, doc, doc-output
- Differs: `compiler/conformance/cases/syntax/unicode_escape_sign.sisu`: tokens, tree, ids, human, fmt-check, doc, doc-output
- Differs: `compiler/conformance/cases/types/argument_count.sisu`: types, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/types/keyword_only_count.sisu`: human, doc, doc-output
- Differs: `compiler/conformance/cases/wrong/own_closure_captures_text.sisu`: human, doc, doc-output
- Differs: `compiler/stage1/cases/findings4/fixes.sisu`: names, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/findings4/fixes_lexical.sisu`: tokens, tree, ids, human, fmt-check, doc, doc-output
- Differs: `compiler/stage1/cases/lexical_mistakes.sisu`: tokens, tree, ids, human, fmt-check, doc, doc-output
- Differs: `compiler/stage1/cases/type_arguments_in_values.sisu`: names, ids, human
- Differs: `compiler/stage1/cases/type_calls.sisu`: human, doc, doc-output
- Differs: `compiler/stage1/cases/unicode_escape_sign.sisu`: tokens, tree, ids, human, fmt-check, doc, doc-output

**Documents to bring along:** the manual's transcripts of these errors,
which `sisu`, whose front end is stage 1, now prints with the fixes and
notes (`E0423`'s in chapter 16 and the reference's ownership and
diagnostics pages among them).

**Decision 369**, for docs/architecture.md:

369. The fixes and notes the manual's fourth round found missing: double
     quotes for text in single quotes (`E0016`); `E0327` with the closest
     variant for `Type.Name` an enum lacks; a note on Python's conversion
     functions (`E0200`); `Some(...)` for a value where its option is
     expected and `.to_f64()` for an `int` where a float is (`E0300`);
     `repeat` for text times a number and the operator's trait for a
     type's own (`E0310`); the closest parameter for a misspelt keyword
     (`E0306`); keyword-only parameters (`E0305`); `.or(...)` for `??` on a
     result (`E0371`); `?.` as one token after a result (`E0370`); a copy
     of a parameter the body changes (`E0312`); `remove` for an item moved
     out of a list (`E0338`); `Cell` or `Mutex` for a `Shared`'s field
     (`E0341`); and a plain closure for an `own` closure's capture
     (`E0423`). Stage 1 decides it (RULES.md, R45).

## R46. A build with its tests has no SIR of a program with an error (decision 370)

**The rule** (17: the runner runs the tests of a program that compiles;
decision 357: `sisu test` and `sisu bench` run from the SIR stage 1's
`sir --tests` prints). `sir --tests`, the program built with its tests,
prints no SIR of a program with an error of any phase: the front end's,
the ownership checker's, or a `comptime` expression's that fails (a
budget spent, `E0603`, a panic, a refinement a constant breaks). It
prints on standard error what `sisu test` prints of the program, its
diagnostics and `D0006` ("the program was not run: errors found"), or
for the ownership checker's errors `D0021`, which names `--defer-borrow`
(a run under the monitor, which the Rust front end serves), with the
front end's warnings before the latter two, and it fails (status 1).
`sir`, the program without its tests, prints the SIR of a program the
ownership checker rejects or whose `comptime` expression fails, as stage
0's `sisu sir` does ("`sisu sir` prints the SIR of a rejected program
too"), and nothing at an error of the front end, as before.

**Why.** The coordinator found, with the manual's exercises 11.4
(`winner`, `E0408`) and 25.2 (`journeys`, `E0603`) in their start
states, that `sisu test` ran the tests of a program `sisu check`
refuses: `sir --tests` printed the SIR of a program the ownership
checker rejects or whose `comptime` expression spent its budget, as
stage 0's `sisu sir --tests` does, and the driver ran it; at an error of
the front end it printed nothing and exited 0, which left the driver to
find out why.

**In stage 1.** `main`: `Lowering.refused`, set by `front_of` at the
front end's errors and by `finish_of` at the ownership checker's errors
and a failed `comptime` expression; `lowering_of`, which leaves out the
SIR of a refused program built with its tests (for `sir-batch --tests`
too); the `sir` command (`app_input`, `report_of`, and `refusal`, which
`test_program` shares). `check.sh` holds a case's `NAME.sir-tests` to
its standard error too when it prints any, after a line `--- stderr`.

**What it changes.** The SIR with its tests of the conformance case
`testing/parametrised_once_ownership` (`E0400`), which stage 1 no longer
prints (the class `sir-tests`), and what `sisu sir --tests` prints of a
refused program, which fails with its diagnostics where stage 0's prints
its SIR. The cases `cases/testing/refused_ownership.sisu`,
`refused_comptime.sisu` and `refused_types.sisu`: their `.sir-tests`,
no SIR and on standard error what stage 0's `sisu test` prints of each,
word for word; `sir.list` names the three, and the first two differ in
the class `sir-tests`:

- Differs: `compiler/conformance/cases/testing/parametrised_once_ownership.sisu`: sir-tests
- Differs: `compiler/stage1/cases/testing/refused_comptime.sisu`: sir-tests
- Differs: `compiler/stage1/cases/testing/refused_ownership.sisu`: sir-tests

**What the stage-0 side must do.** The driver (`cli.rs`, `sisu test` and
`sisu bench` served by stage 1): when `sir --tests` fails, show what it
printed on standard error and take its status, without running stage
1's `check` after it, which prints the diagnostics a second time (as the
driver is, the tests of a refused program no longer run, and its
diagnostics are printed twice).

**Decision 370**, for docs/architecture.md:

370. A program built with its tests (`sir --tests`, from which `sisu
     test` and `sisu bench` run, decision 357) has no SIR when it has an
     error of any phase: the front end's, the ownership checker's, or a
     failed `comptime` expression's. `sir --tests` prints the diagnostics
     and `D0006`, or `D0021` at the ownership checker's errors, as `sisu
     test` prints them, and fails. `sir`, the program without its tests,
     keeps printing the SIR of a program the ownership checker rejects,
     as stage 0's `sisu sir` does. Stage 1 decides it (RULES.md, R46).

## R47. `sir --tests` prints the warnings, and takes a build's options (decision 400)

**The rule** (17: the runner runs the tests of a program that compiles,
and `sisu test` prints the program's warnings before its report; decision
250: a package's features and the lockfile; decision 357: `sisu test` and
`sisu bench` run from what `sir --tests` prints). `sir --tests`, the
program built with its tests, checks the program once and prints, for a
program without an error, its SIR on standard output and its warnings on
standard error, as `check` prints them for the program built with its
tests: the human form, or the conformance form with `--format=ids`. A
program with an error is refused as R46 has it, in the same form. `sir`,
with its tests or without them, takes `--features NAMES` (any number of
times, `--features=NAMES` too), `--no-default-features` and `--locked` as
`check` does: the features choose what the root package's `@cfg`s keep,
`D0112` names a feature the package does not declare, and `--locked` stops
the command with `D0065` where the lockfile would change, before anything
is lowered. `--accept-unsafe` and `--contracts=LEVEL` stay.

**Why.** The driver runs stage 1 twice for each program `sisu test` runs:
`sir --tests`, and then `check` for the warnings (decision 380), about 0.6 s
each for a small file, and it keeps a run that asks for features on the
Rust front end, since stage 1's `sir` refused `--features` and `--locked` as
usage errors (port-debt.md, "A program's transcripts, `sisu test --std`,
`sisu test --defer-borrow` and `--features`").

**In stage 1.** `main`: the `sir` command (`feature_request`,
`unknown_feature` and `write_lockfile` as `check` has them, and the
feature request through `lowering_of` and `app_input`); `report_with`,
`report_of`'s report with the SIR of the program it checked, which `sir
--tests` and `test` print from one check where they checked twice;
`refusal` and `shown_report`, in either form.

**What it changes.** Nothing that `check.sh` compares with stage 0: the
SIR is the same (the class `sir-tests`), and stage 0's `sisu sir --tests`
prints no warnings. The case `cases/testing/warned_tests.sisu` (its
`.sir-tests`: the SIR, and the warning after `--- stderr`); `features.pkg`
of the conformance suite, its `--features verbose`, `--no-default-features`
and `--locked` after a change of its manifest, were checked by hand.

**What the driver may rely on**, so that `sisu test` and `sisu bench` run
stage 1 once per program (the stage-0 side's change to `cli.rs`): call
`sir --tests [--format=ids] [--locked] [--features NAMES]
[--no-default-features] [--accept-unsafe] [--contracts=LEVEL] PATH` for
each program (each member of a workspace, as now). Status 0: standard
output is the SIR (a program without tests prints its SIR all the same),
and standard error is exactly what `check` with the same options prints
of the program built with its tests, its warnings (empty when it has
none), to be shown before the report; there is no `beyond the slice` line
then, since a program stage 1 cannot follow is refused. Status 1:
standard output is empty, and standard error is what `sisu test` prints
of a program it does not run, to be shown as it is and the status taken:
its diagnostics then `D0006` or `D0021`, or `D0065` (`--locked`),
`package.unsafe-raised` (`E1022`) or `D0112` (an unknown feature). So the
driver drops its `check` run (`check_words`, `checked`) and its
`errors_line` after `sir --tests`, passes `--locked` and the features to
`sir --tests`, and `stage1_serves` no longer keeps `--features` and
`--no-default-features` on the Rust front end. Until it does, a program's
warnings are printed twice by `sisu test` (once from `check`, once from
`sir --tests`'s standard error, which the driver prints before the
report), which the conformance suite, comparing standard output, does not
see.

**Decision 400**, for docs/architecture.md:

400. `sir --tests` prints, beside the SIR of a program without an error,
     the program's warnings on standard error as `check` prints them (the
     human form, or `--format=ids`), from one check of the program, so
     that the driver runs stage 1 once for each program `sisu test` and
     `sisu bench` run; `sir` takes `--features`, `--no-default-features`
     and `--locked` as `check` does (decision 250). Stage 1 decides it
     (RULES.md, R47).

## R48. A bound from the end on a `Slice` and `bytes`; a module's type with its arguments; `E0413`'s order (decision 401)

**The rule.** Three of the forms the stage-0 round assigned to stage 1:

- **A bound counted from the end** (02: `a[i:^1]`; R43 for a list) reads a
  `Slice` and `bytes` as it reads a list: `xs[^1]` is the last value of a
  `Slice[int]` (a reference to it), `b[^2]` the byte before the last, and
  `xs[1:^1]` and `b[:^3]` cut from the end, where each was `E0383`
  ("appears only inside an index", though it stood in one). The base is
  evaluated once: the prelude's `Slice.at_end`, `Slice.cut_end`,
  `bytes.at_end` and `bytes.cut_end`, written in Sisu over `at` and `cut`,
  read its length, so `view(xs)[^2]` calls `view` once.
- **A module's type with its type arguments before `.`**,
  `collections.Deque[int].new()`, is the type: the name pass reads a
  scalar type or a type parameter among the arguments of `a.b[...]` as a
  type, where it read `int` as a value and reported `E0200` (with
  Python's conversions' note since R45); any other argument is a value as
  before (`cap.Fake[Trait]`).
- **`E0413`'s fixes** come in the order the body asks for: the `from`
  clause first when a `return` gives a borrowed parameter or a part of
  one (`return a`, `a.trim()`, `a[1:]`, `if c: a else: b`), which the
  clause fits and a `String` result copies; the `String` result first, as
  R24 has it, when the body returns text it makes.

**Why.** The stage-0 round's list (the maintenance round's findings
"`xs[^1]` with `xs` a `Slice[int]`", "`collections.Deque[int].new()`" and
"`E0413`'s order", which only stage 1 can settle).

**In stage 1.** `typeck.expr.index_expr` (the methods of a bound from the
end, `slice_bound`), `compiler/std/prelude.sisu` (`at_end` and `cut_end`
of `Slice` and `bytes`, `@internal`, which the Rust front end reads too and
no program of the corpus reaches), `resolve`'s `is_member_with_arguments`,
`member_with_arguments` and `member_argument`, and
`typeck.collect.returns_borrowed` (`stmt_returns_borrowed`,
`gives_param`).

**What it changes.** The fixes of the conformance suite's
`ownership/from_required` and `wrong/missing_from`, whose `longer` returns
`a` or `b`: their `.diag` files, which hold stage 1's `check --format=ids`,
are blessed again with `borrow.add-from` before `borrow.own-result`. The
cases `cases/findings5/slice_bounds.sisu` and `member_type_arguments.sisu`
(their `.out`: they run) and `from_order.sisu` (its `.diag`, both orders).

- Differs: `compiler/stage1/cases/findings5/from_order.sisu`: ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/findings5/slice_bounds.sisu`: ids, human, doc, doc-output

**Decision 401**, for docs/architecture.md:

401. A bound counted from the end reads a `Slice` and `bytes` as it reads
     a list (`xs[^1]`, `xs[1:^1]`, 02), the base evaluated once; a type of
     a module written with its arguments before `.`,
     `collections.Deque[int].new()`, has its scalar arguments read as
     types; and `E0413` offers the `from` clause first when the body
     returns a borrowed parameter or a part of one, the `String` result
     first when it returns text it makes. Stage 1 decides it (RULES.md,
     R48).

## R49. A generator expression's first iterable is evaluated where the expression is (decision 402)

**The rule** (02: a comprehension in parentheses is a lazy `Iter`, as in
Python, which evaluates the first iterable at once; R42). In `(f(x) for x
in it if c(x))` the first iterable, `it`, is evaluated where the
expression is, before the `Iter` exists, and the generator keeps its value:
a call, a literal, a slice or any value that is not a place is computed
there, into a local the generator's closure keeps by value (moved into it,
as an `own` closure's capture is), and a `?` in it passes its error on in
the function that writes the expression; a place (a name, `self`, a field
of one) is borrowed as before, which is the same as reading it there,
since nothing may change it while the generator lives (`E0405`). The
clauses after it, its other iterables and its tests, run as the
generator is advanced. A generator returned from a function owns what its
first iterable made (`return (len(w) for w in words())`), and one dropped
before its end frees it.

**Why.** R42 evaluated the first iterable when the `Iter` was first
advanced, which the fourth round left open: a call's side effects and
panics came at the first `next`, and `?` in the iterable was refused,
since it stood in the closure's body, which returns nothing.

**In stage 1.** `typeck.expr.generator_comprehension` (`names_place`,
`Checker.kept_iterables`, the `match` that binds the value the generator
keeps), `iterable_of`, which `check_iterable` now calls, and
`typeck.tasks.decide_captures`, which keeps that local by value.

**What it changes.** Nothing in the corpus, which writes no generator
expression; the SIR of R42's cases whose first iterable is a call
(`range(0, 3)` in `generator_expressions.sisu`), which still print what
they printed. The case `cases/generators/generator_site.sisu` (its `.out`:
it runs, interpreted and natively, with no allocation live at exit, a
generator dropped before its end among them).

- Differs: `compiler/stage1/cases/generators/generator_site.sisu`: ids, human, doc, doc-output

**Decision 402**, for docs/architecture.md:

402. A generator expression's first iterable is evaluated where the
     expression is (02, as Python's): a value that is not a place is
     computed there and kept by the generator, moved into its closure, a
     `?` in it passing its error on in the enclosing function; a place is
     borrowed as before. The other clauses run as the generator is
     advanced. Stage 1 decides it (RULES.md, R49).

## R50. A type's constant as a type's constant argument (decision 403)

**The rule** (03, "Const generics" and "Traits"; R37, R43). Where a type's
constant argument goes, `Array[int, Square.SIDES]`, a constant of a type
of the program is its value when folding computes it: the value an impl
of a trait gives the constant for the type, or the type's own body
(`Light.COUNT`, R43), an integer, a constant of the module or an operation
on them (`2 + 2`), with folding's rules and errors (`typeck.consts`), in
a signature, a field's type, a body's annotation and a type's arguments
before `.` (`Array[bool, Light.COUNT].filled(False)`). A trait's constant
read through a type parameter, `Array[int, T.SIDES]`, is not one: it is a
value of the instance, computed where the function runs (the trait's
`pure` function made for the type, R37), where a type's constant argument
is known where the type is written; `E0239` gets the note
`resolve.const-through-param`, which names the `const` parameter that
carries it (`[T: Shape, const N: int]`, given as `Square.SIDES` by a
caller).

**Why.** The fourth round's open item: "a trait's constant as a type's
constant argument (`Array[int, T.SIDES]`), which folding does not reach".
Through a type parameter it would need a type of SIR that calls a
function, which the interpreter and both lowerings would have to compute
where a type argument is needed (a type argument is a number, a type
parameter or an operation on them, `Type.Const` and `Type.ConstOp`), or a
hidden `const` parameter added to each generic function whose types read
one and to each of its callers': that is decided against here, and the
note says what to write instead.

**In stage 1.** `typeck.collect`: `assoc_const_arg` (an impl's or a type
body's constant, through `path_def`), `fold_const_args`, which records them
for the checker in `Program.const_args`, the note in `resolve_ty`;
`typeck.check.resolve_ty` and `typeck.call`'s type arguments, which read
them.

**What it changes.** Nothing in the corpus, which writes no such
argument. The cases `cases/traits/const_arguments.sisu` (its `.out`: it
runs) and `const_through_param.sisu` (its `.diag`), whose trees, with a
trait's constant without a value (R37), differ too.

- Differs: `compiler/stage1/cases/traits/const_arguments.sisu`: tree, ast, names, ids, human, fmt-check, doc, doc-output
- Differs: `compiler/stage1/cases/traits/const_through_param.sisu`: tree, ast, ids, human, fmt-check, doc, doc-output

**Decision 403**, for docs/architecture.md:

403. A constant of a type of the program, given by an impl or by the
     type's body, is a type's constant argument where folding computes its
     value (`Array[int, Square.SIDES]`, 03); a trait's constant through a
     type parameter is a value of the instance and no constant argument,
     `E0239` noting the `const` parameter that carries it. Stage 1 decides
     it (RULES.md, R50).

## R51. Deprecated traits and fields (decision 404)

**The rule** (13, "Deprecation": "every use of a deprecated item warns";
R30, R40). A trait or a field marked `@deprecated("...")` warns where it
is used, `W0023` with the attribute's text (`W0025` without), as a
deprecated type does since R40:

- a trait named in a bound (`T: Measured`), an impl's header (`impl
  Measured for Panel`), a supertraits list (`trait Ranked(Measured)`) or a
  `dyn` type, but in the trait's own declaration (its methods' bodies and
  signatures);
- a field read or changed (`p.width`, `p.width = 4`), given a value
  (`Panel(width=3)`, at the keyword) or matched (`case Panel(width=w)`), but
  in the struct's own methods and impls, as a deprecated type's own
  declaration may use it.

`@deprecated` on a trait or a field is no longer `W0026` (R40), whose list
of places now names them.

**Why.** The fourth round's open item: "a deprecated trait and field (R40
covers functions, types, constants and variants)".

**In stage 1.** `typeck.collect`: `deprecate` of a trait, `decl_owner` set
for a trait's declaration and its methods (`owner_def`), the use in
`resolve_bound`, `Program.deprecated_fields` from `resolve_struct`;
`typeck.Checker.deprecated_field_use`, from `deprecated_uses`;
`resolve.attribute_places`.

**What it changes.** Nothing in the corpus, which deprecates nothing. The
case `cases/attributes/deprecated_traits_fields.sisu` (its `.diag`, and
its `.out`: it runs), and the `places` of `W0026` in
`cases/attributes/misplaced.sisu` (its `.diag`, blessed again).

- Differs: `compiler/stage1/cases/attributes/deprecated_traits_fields.sisu`: ast, ids, human

**Decision 404**, for docs/architecture.md:

404. A trait or a field marked `@deprecated("...")` warns where it is used
     (13), as a deprecated type does (decision 364): a trait in a bound, an
     impl's header, a supertraits list or a `dyn` type, but in its own
     declaration; a field read, changed, given a value or matched, but in
     its struct's methods and impls. Stage 1 decides it (RULES.md, R51).

## R52. `@derive(Arbitrary)` of a generic type, and an impl of `Arbitrary` in another module (decision 405)

**The rule** (17, "Property-based testing"; R41). `@derive(Arbitrary)` on a
generic struct or enum gives the impl that draws its shape, each type
parameter drawn through its own `Arbitrary`:

```sisu
impl[T: Arbitrary] Arbitrary for Boxed[T]:
    def arbitrary(d: Draws) -> Boxed[T]:
        return __arbitrary_Boxed(d, ((e: Draws) -> T.arbitrary(e)))
```

a parameter's own bounds kept (`T: Arbitrary + Eq`), so that a bound `T:
Arbitrary` draws `Boxed[Money]`; a type with a part that has no generator
is `E0820` as before, and the reason `E0820` gave a generic type is gone.
An impl of `Arbitrary` written by hand in another module of the package
than its type's draws the type wherever it is drawn: the generators of the
type's own module (`arbitrary[Money]` there, and those of its types that
hold a `Money`) call the impl, and a site `arbitrary[Money]` in a module
that imports the type draws it through `Money.arbitrary`, as it does a
type of another module that derives `Arbitrary` (`arbitrary[Purchase]`).

**Why.** The fourth round's open item: "a generic type's
`@derive(Arbitrary)` and an impl of `Arbitrary` for a type of another
module (R41 draws a type of the module that writes the impl)".

**In stage 1.** `typeck.arbitrary`: the derived impl of a generic type
(`order_bounds`), `find_arbitrary_impls`, which records every module's
impls written by hand and derives before any module's generators are
written (`Resolver.arbitrary_by_hand`, `arbitrary_derived`), `path_def`,
`drawn_elsewhere` and the writer's `foreign` names; `check.sh` holds a
package case to its `NAME.pkg.tests`.

**What it changes.** Nothing in the corpus, which derives no `Arbitrary`.
The cases `cases/testing/generic_arbitrary.sisu` (its `.tests`: both
properties hold), `cases/packages/arbitrary_elsewhere.pkg` (its
`arbitrary_elsewhere.pkg.tests`: the impl in the module `draws` draws a
`Money` of the module `money` from `main`, and a property fails at the
impl's boundary, `Money(cents=5000)`), and `arbitrary_refusals.sisu`,
whose generic type now fails for its part without a generator (its
`.diag`).

- Differs: `compiler/stage1/cases/packages/arbitrary_elsewhere.pkg`: sir, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/testing/generic_arbitrary.sisu`: ids, human, doc, doc-output

**Decision 405**, for docs/architecture.md:

405. `@derive(Arbitrary)` on a generic type gives the impl that draws its
     shape with each type parameter drawn through its own `Arbitrary`
     (`impl[T: Arbitrary] Arbitrary for Boxed[T]`), and an impl of
     `Arbitrary` written in another module than its type's draws the type
     wherever it is drawn, a site in a module that imports the type
     drawing it through the impl. Stage 1 decides it (RULES.md, R52).

## R53. An attribute before a statement (decision 406)

**The rule** (02: an attribute stands on the line before a declaration;
19, N-22: checked directives on a loop nest are planned with the native
backend, "attributes sit on the line before an item, not on statements").
An attribute in a block before what is no declaration stands before a
statement, which no attribute applies to: `E0129`
(`parse.attribute-on-statement`), which names it, and the statement is
parsed as one. `@tile`, `@unroll` and `@vectorize` there get the note
`parse.loop-directive`: they are 19's planned directives, which hold or
fail to compile, and not in the language yet. On a declaration, where R40
warns that they mean nothing (`W0026`), they stay as they are.

**Why.** The fourth round's open item ("`@tile`, `@unroll` and
`@vectorize` on a loop, which the parser does not take before a
statement") and the reference's R17 finding: `E0106` for an attribute
before a statement said "expected a declaration, found `let`" and nothing
about the attribute. Taking the directives on a loop is N-22's, with the
dependence analysis that makes them hold, which the documents leave for
later; parsing them and checking nothing is what 19 rules out.

**In stage 1.** `parser.items.item` (`Parser.in_statements`, counted by
`statements`).

**What it changes.** Nothing in the corpus, which puts no attribute before
a statement. The case `cases/attributes/statement_attributes.sisu` (its
`.diag`); its tree and the formatter's verdict, which prints the
diagnostics, differ, and so do the formatter's of its deformed copies.

- Differs: `compiler/stage1/cases/attributes/statement_attributes.sisu`: tree, trivia-tree, ast, ids, human, fmt-check, doc, doc-output

**Decision 406**, for docs/architecture.md:

406. An attribute before a statement is `E0129`, naming the attribute,
     and the statement is parsed as one (02: attributes stand before
     declarations); `@tile`, `@unroll` and `@vectorize` there note that
     they are 19's planned directives (N-22), not in the language yet.
     Stage 1 decides it (RULES.md, R53).

## R54. The syntax the manual's writers found a parse error misnaming (decision 407)

**The rule** (02, "Departures from Python", 09, D-062; the manual's
findings, "Syntax and names" and "Diagnostics without fixes"). Where a
parse error stops at another language's spelling, the fix
`parse.sisu-spelling` writes Sisu's: `and` and `or` for `&&` and `||`,
`not` for `!x`, `ref` for `&x`, `i += 1` for `i++` (and `-=` for `--`),
`var` for `let mut`, `.to_f64()` for `as f64` (and each numeric type's
conversion), `print` for `println!`, `range(0, 3)` for `0..3` (`range(0, 3
+ 1)` for `0..=3`), `list[int]` for `Vec<int>` (`dict`, `set` for Rust's
maps and sets), `match` for `switch`, `.is_none()` and `.is_some()` for `is
None` and `is not None`, `:` for `=>` after a `case`'s pattern, `#` for a
line comment written `//`, and `None` for `nil`, `null` and `NULL`
(`E0200`). And four forms:

- an attribute on its declaration's line, `@derive(Eq) struct Point:`, is
  `E0131` (`parse.attribute-line`), with the fix that breaks the line, and
  the declaration is parsed, where `E0117` and `E0106` lost it;
- `pub(crate)`, or any name but `package` in `pub(...)`, is `E0130`
  (`parse.visibility-scope`), with the fix `pub(package)`, where every such
  name made the item `pub` to every package (D-062 rules the silent
  difference out);
- a one-line `if` without `else`, `if c: print(x)`, is a statement whose
  block is on its line (`E0102`), not an `if` expression without `else`
  (`E0116`);
- a keyword as a field's name, `type: str`, is `E0124` as a parameter's
  is, and the line declares no field, where it was `E0100`.

**Why.** The manual's findings: "Most other habits get a diagnostic that
does not name the Sisu spelling" (R19, G3, G4), "`switch day:` ... and
`i++`" (chapter 4), "an attribute on the line of its declaration" (R14),
"`pub(crate)` compiles and exports the item to every package" (R20), "a
one-line `if c: print(x)` is E0116" and "a keyword as a field's name is
E0100" (R1).

**In stage 1.** `parser.habits` (`Parser.with_habit`, from `error_found`,
`expect` and `end_stmt`), `parser.items` (`attribute`, `visibility`,
`struct_member`), `parser.stmts.no_else_after`, `resolve.use_name`.

**What it changes.** Nothing in the corpus, which writes none of these.
The cases `cases/findings5/habits.sisu` and `parse_fixes.sisu` (their
`.diag`); their trees, which print the parser's diagnostics, and the
formatter's verdicts differ.

- Differs: `compiler/stage1/cases/findings5/habits.sisu`: tree, names, ids, human, fmt-check, doc, doc-output
- Differs: `compiler/stage1/cases/findings5/parse_fixes.sisu`: tree, trivia-tree, ast, ids, human, fmt-check, doc, doc-output

**Decision 407**, for docs/architecture.md:

407. A parse error at another language's spelling gets the fix that
     writes Sisu's (`and`, `not`, `ref`, `i += 1`, `var`, `.to_f64()`,
     `print`, `range(0, 3)`, `list[int]`, `match`, `.is_none()`, `:`, `#`,
     `None`); an attribute on its declaration's line is `E0131` and the
     declaration is parsed; `pub(crate)` is `E0130` with `pub(package)`; a
     one-line `if` without `else` is `E0102`; a keyword as a field's name
     is `E0124`. Stage 1 decides it (RULES.md, R54).

## R55. The type checker's notes and fixes the manual's writers found missing (decision 408)

**The rule.** Each a note or a fix on an error stage 1 already reports,
as R16, R24 and R45 gave theirs:

- a `var` given a borrowed value, `var in_order = readings` with
  `readings` a borrowed parameter, then changed (`E0341`, "not declared
  with `var`", though it is): the note `type.var-holds-borrow` (it holds
  `ref list[int]`, which changes nothing of its own) and the fix `.copy()`
  at its value, where the note named `ref mut`;
- a borrowed parameter given to a parameter taken with `own` (`E0300`):
  the note `type.borrowed-to-own`, naming the callee and the move, and
  besides `.copy()` the fix `type.take-own`, the parameter taken with
  `own`;
- an `int` where a sized integer is expected (`E0300`): the note
  `type.narrow-int`, `.to_u8()` and the option it gives;
- `let Total = 10` (`E0327`): the fix `type.lowercase-binding`, the name in
  snake case, `total`;
- `E0417`, `E0806` and `E0810` without an article, which read "a
  `Opening`" and "a `Hour`": "a value of `{ty}`", "of type `{ty}`", in the
  catalog's text, which both front ends print.

**Why.** The manual's findings: "`var in_order = readings` ... is E0341
... though it is" (chapter 5), "a borrowed parameter passed to an `own`
parameter ... whose one fix copies" (chapter 10), "`let small: u8 =
large` is E0300 with no fix" and "`let Total = 10` gets E0327, E0329 and
E0200, none saying that a binding's name starts with a lowercase letter"
(chapter 2), and "E0806, E0810 and E0417 write `a {ty}` for every type"
(chapter 23, R14).

**In stage 1.** `typeck.check.require_mutable_as` (`is_param_local`,
`copied_init_end`), `typeck.expr.mismatch_fix` and `coerce`
(`Checker.arg_own`, set by `typeck.call.check_arg`), `typeck.pat`
(`snake_name`, `after_let`); `compiler/catalog/messages.catalog`.

**What it changes.** The human form of the corpus's programs with
`E0417`, `E0806` or `E0810`, which both front ends print from the same
catalog, and nothing else of the corpus. The case
`cases/findings5/notes.sisu` (its `.diag`).

**Documents to bring along:** the manual's transcripts of `E0417`,
`E0806` and `E0810`, whose text changes for every front end, and of the
other errors above, which `sisu` now prints with the notes and fixes.

- Differs: `compiler/stage1/cases/findings5/notes.sisu`: ids, human, doc, doc-output

**Decision 408**, for docs/architecture.md:

408. The type checker's notes and fixes the manual's writers found
     missing: a `var` that holds a borrow it was given (`E0341`), a
     borrowed parameter given to an `own` one, with the fix that takes it
     with `own` (`E0300`), the narrowing conversion of an `int` (`E0300`),
     a capitalised name a `let` binds (`E0327`), and `E0417`, `E0806` and
     `E0810` without an article. Stage 1 decides it (RULES.md, R55).

## R56. `sisu1 --version` prints stage 1's version (decision 440)

**The rule.** `sisu1 --version` and `sisu1 version` print one line,
`sisu1 0.6.0`, stage 1's version, and exit with 0, before anything is
read (the catalog, the standard library); any other argument after it is
the usage error it was. The version is the package's (`sisu.toml`, and
`main.VERSION`), whose minor number is the round of this file that last
changed what the driver may rely on: 0.6.0 is the sixth round's, whose
`sir --tests` reports the program's diagnostics itself (R46, R47).

**Why.** The driver asks which stage 1 it runs by a probe, `sir --tests
--locked` of an empty directory (decision 424), which reads the catalog
and the base; a version that says it is cheaper and says more.

**In stage 1.** `main.main` (`VERSION`), `sisu.toml` and its lockfile, and
stage 2's lockfile, which names stage 1's version.

**What it changes.** Nothing `check.sh` compares.

**What the stage-0 side must do.** `Stage1::sir_reports` may ask
`--version` instead of probing: a stage 1 that prints `sisu1 0.6.0` or a
later version reports from `sir --tests` (an older stage 1 stops at its
usage), and it may keep the answer beside the executable as now.

**Decision 440**, for docs/architecture.md:

440. `sisu1 --version` (and `sisu1 version`) prints `sisu1 VERSION`, stage
     1's version, one line, before it reads anything; its minor number is
     the round of stage 1's rules that last changed what the driver may
     rely on, 6 for the stage 1 whose `sir --tests` reports the program's
     diagnostics, so that the driver may ask it rather than probe `sir
     --tests --locked` (decision 424). Stage 1 decides it (RULES.md, R56).

## R57. A build that stops at errors says the program was not compiled (decision 441)

**The rule** (17 and the driver's messages: `D0006` says "the program was
not run", which only a run means). A build of a program with an error,
stage 1's `build` (`sisu build`, with or without `--native` and
`--release`) and what `sisu sir` prints of a program it has no SIR of,
ends with `D0132` (`cli.not-compiled`, "the program was not compiled:
errors found (N)"), at the front end's errors, the typed holes of a
release build, a failed `comptime` expression and the ownership
checker's errors alike, where it ended with `D0006` or `D0021`. A build
for a run, `build --for-run` (which the driver's `sisu run` and `sisu run
--native` ask for, and `sisu1 run`), ends as before, with `D0006`, or
`D0021` at the ownership checker's errors. `sir --tests` ends as `sisu
test` does (R46).

**Why.** The manual's finding: "`sisu build` and `sisu sir` of a program
with errors end with D0006, 'the program was not run'" (R16): a build
runs nothing. The driver's run prints what stage 1's build printed, so
the build that a run asks for keeps its line.

**In stage 1.** `main`: the `build` command (`--for-run`, `BuildHow`),
`build_command` and `build_program`; `sisu1` (`run`); the catalog's
`D0132`. The driver (`cli.rs`): `sisu run` and `sisu run --native` pass
`--for-run`.

**What it changes.** What `sisu build` and `sisu sir` print of a program
with errors, which no class of `check.sh` compares with stage 0 (whose
Rust front end keeps `D0006`); `sisu run` is as before (the conformance
suite's `run/stopped_by_errors`). The case
`cases/run/build_stopped.sisu` (its `.release`, what `build --native
--release` prints).

**Decision 441**, for docs/architecture.md:

441. A build of a program with an error, `sisu build` (with or without
     `--native`) and `sisu sir` of a program it has no SIR of, ends with
     `D0132`, "the program was not compiled: errors found (N)", where it
     ended with `D0006`, "the program was not run"; `sisu run` builds
     with stage 1's `build --for-run`, which ends as before with `D0006`,
     or `D0021` at the ownership checker's errors, and `sir --tests`
     ends as `sisu test` does. Stage 1 decides it (RULES.md, R57).

## R58. A hole's closure shows each `old(...)` once (decision 442)

**The rule** (17, typed holes: the closure shows the values in scope).
A hole's closure in a function with a contract shows each `old(...)`
value the contract keeps once, however many places its clauses write the
same text, where it showed one line for each (the value is the same: each
is taken on entry).

**Why.** The manual's finding from chapter 23: `withdraw`'s closure
showed `old(self.balance) = 1000` twice.

**In stage 1.** `borrowck.holes.close_function` (`old_shown`).

**What it changes.** The closure of a hole in a function whose contract
writes an `old(...)` more than once, which only stage 1 prints (R27); no
program of the corpus. The case `cases/holes/old_once.sisu` (its `.err`).

**Decision 442**, for docs/architecture.md:

442. A typed hole's closure shows each `old(...)` value of its
     function's contract once, by its text, however many clauses write
     it, where it showed a line for each place. Stage 1 decides it
     (RULES.md, R58).

## R59. `debug_assert` (decision 443)

**The rule** (02: "`assert`, `debug_assert` and `todo` are compiler
intrinsics ... `debug_assert` is the form that release builds strip").
`debug_assert cond` and `debug_assert cond, message` are statements, an
assertion whose first word is `debug_assert` (a name, not a keyword: as
`expect`, it is the statement only before an expression). Checked as
`assert` is, its parts shown when it fails, where the build checks every
contract (`--contracts=all`, the debug profile's and `sisu test`'s);
stripped, its condition not evaluated, where it checks fewer (`off` or
`preconditions`, the release profile's default, 07).

**Why.** The manual's finding (R15): "`debug_assert` ... is not a
statement: `debug_assert x == y` is E0117".

**In stage 1.** `parser.stmts.statement` (an `AssertStmt` that starts with
the name), `typeck.check` (`TStmt.Assert.debug`), `lower.stmts`.

**What it changes.** Programs that write `debug_assert`, which stage 0
refuses (`E0117`); none of the corpus. The case
`cases/findings6/debug_assert.sisu` (its `.out`, `.err` and `.status`).

- Differs: `compiler/stage1/cases/findings6/debug_assert.sisu`: tree, trivia-tree, ast, ids, human, fmt-check, doc, doc-output

**Decision 443**, for docs/architecture.md:

443. `debug_assert cond[, message]` is a statement, `assert`'s form that
     a build checking fewer than all contracts strips (02, 07): checked
     with every part shown where the build checks all of them (a debug
     build, `sisu test`), not evaluated where it checks `preconditions`
     or none (the release profile's default). `debug_assert` is a name,
     the statement only before an expression. Stage 1 decides it
     (RULES.md, R59).

## R60. `import a.b.Item` is `E0223` (decision 444)

**The rule** (09: `import` names a module; `from a.b import Item` an item
of one). An `import` whose path but its last name names a module, of the
program, a dependency or std, of which the last name is no module, is
`E0223` (`resolve.not-a-module`, "`std.json.JsonError` is not a module"),
with the fix `resolve.import-from-module` that writes `from std.json
import JsonError` (with `as` and `pub` as written); where std's module
declares no such item, `E0204` (`resolve.not-in-module`). The uses of
the name it would bind are not reported again.

**Why.** The manual's finding (R13): "`import atlas.geo.point.Point`, an
item named as a module, is E0203 ('there is no module') where the
catalog has E0223 for it", and for std's modules `E0220`, which names no
module either.

**In stage 1.** `typeck.collect.item_imported`, before `import_target`;
the name pass's `import_target` leaves a std module's item to it.

**What it changes.** Programs that import an item with `import`, which
stage 0 reports as `E0203` or `E0220`. The case
`cases/findings6/import_item.sisu` (its `.diag`).

- Differs: `compiler/stage1/cases/findings6/import_item.sisu`: names, ids, human, doc, doc-output

**Decision 444**, for docs/architecture.md:

444. `import a.b.Item`, whose path but its last name is a module that has
     no module `Item`, is `E0223` ("is not a module") with the fix that
     writes `from a.b import Item`, and `E0204` where a std module
     declares no such item, where it was `E0203` or `E0220`, which say
     there is no module. Stage 1 decides it (RULES.md, R60).

## R61. Forms the documents give: a `Slice` as an `Iterable`, `{x ?? "-"}`, a scalar type as a `comptime` argument (decision 445)

**The rule.**

- A `Slice[T]` and `bytes` given to an `Iterable[T]` parameter
  (`sorted(xs)`, `enumerate`, `zip`, `sum`) is iterated through its
  iterator, as a `for` loop iterates it, when its items decay to what the
  parameter takes (a `Copy` item); else the error names the view as
  before (11: `sorted` takes any iterable).
- In an interpolation, which only reads its values (R14), `place ??
  default` over an optional `String` place reads it as an optional `str`
  rather than moving the `String` out of the place:
  `{b.note ?? "-"}` with `b` the variable of a loop over a list of
  structs.
- A scalar type's name, `int`, `f64`, `bool`, `char`, `str`, given as a
  call's argument is the type checker's to read, as `String` is: a
  `comptime def` that takes `T: type` takes it (`comptime name(bool)`,
  08), and anywhere else it is `E0200` as before, from the type checker,
  with the note on Python's conversions.

**Why.** The manual's findings: "`sorted(xs)` with `xs` a `Slice[int]` is
E0300 ... though `for x in xs` works" (chapter 26); "`{b.note ?? "-"}`
... is E0419" (G7); "`comptime name(bool)` is E0200 ... though `String`,
`uint`, `u8` and the program's own types work and the Rust front end
takes every one" (R12), a regression of stage 1's name pass.

**In stage 1.** `typeck.expr.view_iter` (from `iterable_of`) and
`typeck.call.iterable_arg`; `typeck.expr.string_literal` and `coalesce`
(`Checker.reading_coalesce`, `read_option`); `resolve.call_argument` and
`typeck.expr.name_expr`.

**What it changes.** Programs with such forms, which stage 0 refuses but
the third (whose run is the same). The cases
`cases/findings6/read_forms.sisu` (its `.out`) and
`cases/findings6/scalar_values.sisu` (its `.diag`).

- Differs: `compiler/stage1/cases/findings6/read_forms.sisu`: ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/findings6/scalar_values.sisu`: names, ids, human, doc, doc-output

**Decision 445**, for docs/architecture.md:

445. A `Slice` and `bytes` given to an `Iterable[T]` parameter are
     iterated through their iterators, as a `for` loop iterates them; an
     interpolation's `place ?? default` over an optional `String` reads
     the place as an optional `str` (decision 295's reading); a scalar
     type's name given as an argument is a type a `comptime def` taking
     `T: type` takes, and `E0200` elsewhere. Stage 1 decides it (RULES.md,
     R61).

## R62. `@derive(Decode)` on an enum stops the derive (decision 446)

**The rule** (11; R36: `@derive(Decode)` gives a struct its form).
`@derive(Decode)` of `std.json` on an enum is `E0608`
(`comptime.derive-failed`, "deriving `Decode` for `Lake` stopped at
compile time: `json.Decode` is derived for a struct only; an enum
implements it with an `impl Decode` of its own") at the derive's
argument, where the derive was accepted, its method's `comptime panic`
went unseen and the run stopped at an invalid SIR (`E0501`) that `check`
did not report.

**Why.** The manual's finding from G7.

**In stage 1.** `typeck.instances` (the derives of a std module).

**What it changes.** Programs that derive `Decode` for an enum, which
stage 0 accepts. The case `cases/findings6/decode_enum.sisu` (its
`.diag`).

- Differs: `compiler/stage1/cases/findings6/decode_enum.sisu`: ids, human, doc, doc-output

**Decision 446**, for docs/architecture.md:

446. `@derive(Decode)` on an enum is `E0608` at the derive, since R36's
     derive gives a struct's form only, where it was accepted and the
     program's run stopped at an invalid SIR. Stage 1 decides it
     (RULES.md, R62).

## R63. The fixes and notes the manual's writers found missing, the fifth round's rest (decision 447)

**The rule.** Each a note or a fix on an error stage 1 already reports,
or the code of a parse error where a statement starts given inside an
expression too:

- a function that takes a `Copy` value given where one taking it by
  reference is expected, `xs.iter().map(doubled)` (`E0300`): the fix
  `type.call-with-value`, `x -> doubled(x)`;
- Python's `enumerate(xs, start=1)` (`E0306`): the note
  `type.enumerate-start`, that `enumerate` counts from 0;
- Python's `1.` and `.5` (`E0100`, `E0103`): the fix
  `parse.sisu-spelling`, `1.0` and `0.5`;
- Python's `a if c else b` and walrus inside an expression, where a
  bracket closes or a comma follows: `E0115` with its fix and `E0123`, as
  where a statement starts, where both were `E0100` ("expected `)`");
- a trait written as a parameter's whole type (`E0231`): the fix
  `resolve.add-ref-dyn`, `ref dyn Priced`, and the note
  `resolve.trait-generic`, the generic parameter, where the fix wrote
  `dyn Priced`, which is `E0395` there;
- a contract that calls a plain `def` of its module that is not `pure`
  (`E0801`): the fix `contract.make-pure`, `pure def`;
- a `[package]` key written with a bad value (`E1003`) is not also missing
  (`E1002`);
- `sisu audit --unsafe` names itself `sisu audit --unsafe --std`, where
  it wrote `sisu1`.

**Why.** The manual's findings (Diagnostics without fixes; chapters 14,
16 and 23; R1, R13, R19, G2).

**In stage 1.** `typeck.expr.by_value_closure`, `typeck.call`
(`check_args`), `parser.habits.with_habit` and `parser.state.expect_name`,
`parser.state.expect` and `python_conditional_in`,
`typeck.collect` (`param_type_at`), `typeck.purity.pure_def_at`,
`manifest`, `main.audit_command`; the catalog.

**What it changes.** The diagnostics of programs with these mistakes; of
the corpus, `contracts/contract_errors` (`E0801`'s fix) and
`packages/manifest_errors` (no `E1002`), whose `.diag` files are blessed
again. The cases `cases/findings6/fixes.sisu`, `python_conditional.sisu`,
`float_point.sisu`, `float_lead.sisu` and `pure_fix.sisu` (their
`.diag`).

- Differs: `compiler/conformance/cases/contracts/contract_errors.sisu`: ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/packages/manifest_errors.pkg`: ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/findings6/fixes.sisu`: ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/findings6/float_lead.sisu`: tree, ids, human, fmt-check, doc, doc-output
- Differs: `compiler/stage1/cases/findings6/float_point.sisu`: tree, ids, human, fmt-check, doc, doc-output
- Differs: `compiler/stage1/cases/findings6/pure_fix.sisu`: ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/findings6/python_conditional.sisu`: tree, ids, human, fmt-check, doc, doc-output

**Decision 447**, for docs/architecture.md:

447. The fixes and notes the manual's writers found missing: a closure
     that passes a value, `x -> f(x)`, for a function given where its
     argument is a reference; `enumerate`'s count from 0; `1.0` and `0.5`
     for Python's `1.` and `.5`; `E0115` and `E0123` inside an
     expression; `ref dyn T` and a generic parameter for a trait written
     as a parameter's type; `pure def` for a contract's impure callee; no
     `E1002` for a key written with a bad value. Stage 1 decides it
     (RULES.md, R63).

## R64. A variadic parameter and star unpacking are `E0900` (decision 448)

**The rule.** 02 gives `*xs: T` (variadic arguments collected into a
`Slice[T]`) and `let first, *rest = xs` (star unpacking), and 14 lists
"variadic functions" among what Sisu does not have; stage 1 lowers
neither. A variadic parameter is `E0900` (`stage0.unsupported`, "a
variadic parameter, `*xs: T`, is not implemented in stage 0 yet; it
arrives with the decision between 02, which gives it, and 14, which rules
variadic functions out ..."), where the checker stopped with "beyond the
slice", no code, and `check` exited 0; a `let` whose tuple pattern has a
star is one `E0900` that names `case [first, *rest]:` in a `match`, where
it was `E0326`, `E0329` and `E0387`, and its names are bound, of no type.

**Why.** The manual's findings (chapters 3 and 5, X1). The documents
disagree on variadics, which waits for the owner; star unpacking is 02's
and waits for its lowering (a refutable pattern without `else`, which 02
does not say how it fails).

**In stage 1.** `typeck.collect` (the parameters), `typeck.check.star_unpacking`.

**What it changes.** Programs with either form, none of the corpus. The
case `cases/findings6/designed_missing.sisu` (its `.diag`).

- Differs: `compiler/stage1/cases/findings6/designed_missing.sisu`: ids, human, doc, doc-output

**Decision 448**, for docs/architecture.md:

448. A variadic parameter is `E0900`, naming the disagreement between 02,
     which gives it, and 14, which rules variadic functions out, where the
     checker stopped without a code and `check` passed; star unpacking in
     a `let` is one `E0900` that names the list pattern of a `match`,
     where it was three errors of its pattern. Stage 1 decides it
     (RULES.md, R64).

## R65. A package's black-box tests: `tests/NAME.sisu` is a program whose dependency is the package (decision 480)

**The rule** (17: "A `tests/` directory holds black-box tests that see
only the public API"). A file directly in the `tests/` directory beside a
package's manifest (`[package]`), not a script, is compiled by every
command that takes a path (`check`, `sir`, `sir --tests`, `build`, `doc`,
`lint`) as a program of one module, the file, whose dependencies are the
package it tests, by the package's name, and the package's
dev-dependencies, by theirs: it imports the package as any dependency,
its `pub` items only (`from ledger import total` of a private `total` is
`E0255`), and the package is built without its own tests, as a
dependency is. The file has the package's features, each turning on the
package's own of the same name, so that `--features` and
`--no-default-features` mean what they mean for the package; the files'
names are relative to the package's directory (`tests/balance.sisu`,
`src/lib.sisu`); no lockfile is written for it, as for a file. A
package's dev-dependencies are the black-box file's, not the package's
code's.

`sisu1 test PKG` (and a workspace's members) runs, after the package's
own tests, each `tests/*.sisu` by name, a program each with its own
report and summary line, its replay token naming the file; the status is
the worst of them, as for a workspace's members.

What `sir --tests tests/NAME.sisu` prints is what it prints of any program
built with its tests (R46, R47): the SIR (`source 0 "tests/NAME.sisu"`,
then the package's sources and the dev-dependencies'), the file's tests'
lines (`test "NAME" test#K` with their tags) and its benchmarks, and none
of the package's tests; the program's warnings on standard error; a
program with an error is refused with its diagnostics and `D0006`.

Stage 1's version is 0.7.0 (R56): the driver may rely on it.

**Why.** port-debt.md, "A package's `tests/*.sisu` are not run" (the
manual's finding from chapter 19): stage 1 had to compile such a file as
it compiles a package's dependency before the driver can run it.

**In stage 1.** `package.tested_package` and `package.load_black_box`
(from `load_file_with`), the loader's `root` (whose dev-dependencies and
`[trust]` count) and its dev-dependencies given to the black-box file;
`main.test_command` and `main.black_box_files`; `main.VERSION`,
`sisu.toml` and the lockfiles.

**What it changes.** No program of the corpus has a `tests/` file. The
case `cases/packages/black_box.pkg` (a library, its unit test, a
dev-dependency and `tests/balance.sisu`; its `.tests`).

**What the stage-0 side must do.** `sisu test` of a package (or of a
workspace's members), after the package's own program: for each file of
its `tests/` directory, `*.sisu` sorted by name, ask stage 1 `sir --tests
PKG/tests/NAME.sisu` with the options it passes for the package
(`--locked`, `--features`, `--no-default-features`, `--accept-unsafe`,
`--contracts=...`), run that SIR as it runs the package's, and report it
as a program of its own (its lines, failures and summary; a replay token
naming the file); the status is the worst of the programs'. With
`--native` the same SIR is built natively. port-debt.md's item then goes.

**Decision 480**, for docs/architecture.md:

480. A file directly in a package's `tests/` directory is a program of
     one module whose dependencies are the package, by its name, and the
     package's dev-dependencies (17's black-box tests): it sees the
     package's `pub` items only, the package is built without its own
     tests, the file has the package's features, and its files are named
     relative to the package's directory. `sir --tests PKG/tests/NAME.sisu`
     prints its SIR with its own tests' lines, which `sisu test` runs after
     the package's own, a program each; `sisu1 test` does so, and stage
     1's version is 0.7.0. Stage 1 decides it (RULES.md, R65).

## R66. Stage 1's compile-time evaluator holds a `Shared` as its value (decision 481)

**The rule** (decision 464; 04, rule 5). The evaluator of compile-time
code holds a `Shared` value as the value it shares (`Shared.new`,
`Shared(x)`, `Shared.get`, a field or method reached through it, a
`copy` of it): `==` and `!=`, a set's element and a dict's key compare
and hash what it holds, as decision 464 has the run time do, and `{x}`
and `{x:?}` show it. Its identity, which a value has not, is beyond the
evaluator (`Shared.same`, `Shared.count`, `Shared.downgrade`); a `Shared`
a `comptime` expression would keep for run time is `E0604` ("a `Shared`
made at compile time cannot be kept"), as stage 0 has it.

**Why.** README "What is next", 10; it was "beyond the slice" at
`Shared.new`, where stage 0 evaluates the same expression.

**In stage 1.** `interp` (`SHARED`, the intrinsics `Shared.new` and
`Shared.get`, a dereference through a `Shared`, its display, and the
baker's refusal).

**What it changes.** No program of the corpus evaluates a `Shared` at
compile time. The case `cases/comptime/shared.sisu` (its `.out`, and its
SIR, stage 0's).

**Decision 481**, for docs/architecture.md:

481. Stage 1's compile-time evaluator holds a `Shared` as the value it
     shares, so `==`, `hash`, a set and a dict follow it to the value as
     decision 464 has the run time do; its identity (`same`, `count`, a
     `Weak`) is beyond the evaluator, and a `Shared` kept for run time is
     `E0604`. Stage 1 decides it (RULES.md, R66).

## R67. R54's spelling where the error expects a bracket or a colon, and a fix that names its replacement once (decision 482)

**The rule** (the manual's findings of step 16, "Diagnostics without
fixes"). R54's fix `parse.sisu-spelling` is given also where the parse
error is `E0100`, expecting the bracket of a call or the colon of a
header, at `as` or `..`: `print(n as f64)` gets `.to_f64()` and `for i in
0..3:` `range(0, 3)`. A `//` comment at the end of a line, `let n = 3 //
the count`, which divides by its first word and is `E0117` at its second,
gets the fix `#`; a one-word one after a value, `print(n) // done`, is a
division by a name, `E0200`, which gets the fix `#` when `//` follows a
space and the name ends the line. And a fix's message does not name the
replacement its edit shows: `parse.sisu-spelling` reads "Sisu writes it
so: `.to_f64()`" and `type.lowercase-binding` "a binding's name starts with
a lowercase letter: `kind`", where each named it twice.

**Why.** The manual's findings ("The fix of R54 comes only where the parse
error stops at the habit itself", "A fix whose message names its
replacement shows it twice"; chapter 2, R19, G3, G4).

**In stage 1.** `parser.habits` (`cast_or_range`, the comment's words),
`resolve.slash_comment`; the catalog's two fix texts.

**What it changes.** Nothing in the corpus, which writes none of these;
the human form of R54's cases and of `E0327`'s fix. The cases
`cases/findings7/spellings.sisu` and `slash_comment.sisu` (their `.diag`).

- Differs: `compiler/stage1/cases/findings7/spellings.sisu`: tree, ids, human, fmt-check, doc, doc-output
- Differs: `compiler/stage1/cases/findings7/slash_comment.sisu`: names, ids, human, doc, doc-output

**Documents to bring along:** the manual's transcripts that show
`parse.sisu-spelling` or `type.lowercase-binding` (about a dozen under
`manual/examples/`, the false friends' reference and the guides from Go
and Rust).

**Decision 482**, for docs/architecture.md:

482. R54's spelling fix is given also where the parse error expects a
     bracket or a colon (`print(n as f64)`, `for i in 0..3:`), and `#` for
     a `//` comment at the end of a line, also where it is a division by
     a name (`E0200`); a fix's message no longer names the replacement its
     edit shows. Stage 1 decides it (RULES.md, R67).

## R68. Messages without "a `{ty}`", and a trait's constant where a type goes (decision 483)

**The rule.** `E0418`, `E0420`, `E0812`, `R0069` and `R0103` write the
type without an article, as R55 has `E0417`, `E0806` and `E0810`: "a value
of `Opening` must be passed to an `own` parameter first", "`x` is of type
`Port`", "`0` is not of type `Port`". And a trait's constant read through a
type parameter where a type goes, `Array[int, T.SIDES]`, whose bound
declares the constant, is `E0274` (`resolve.const-not-type`, "`SIDES` is a
constant of `Shape`, which bounds `T`, not a type"), with R50's note,
where `E0239` said that `T` "has no associated type `SIDES`".

**Why.** The manual's findings ("E0418, E0420, E0812, R0069 and R0103
still write "a `{ty}`"", R17; "E0239 for a trait's constant ... says that
`T` "has no associated type `SIDES`"", R3, R17).

**In stage 1.** The catalog (shared with stage 0, whose front end and run
time print the five messages too); `typeck.collect.bound_constant`.

**What it changes.** The run time's `R0069` in four `.defer` expectations
of the conformance suite (`traits/must_consume_generic`,
`ownership/must_consume_dropped`, `must_consume_container` and
`must_consume_try`), blessed again; the case
`cases/traits/const_through_param.sisu` (its `.diag`, `E0274`).

**Documents to bring along:** the manual's transcripts of the five
messages (`e0418_left_unconsumed`, chapter 13's `drafts`, the diagnostics
reference) and of `E0239`'s for a constant.

**Decision 483**, for docs/architecture.md:

483. `E0418`, `E0420`, `E0812`, `R0069` and `R0103` write the type without
     an article ("a value of `T`", "of type `T`"); a trait's constant read
     through a type parameter where a type goes is `E0274`, which calls it
     a constant, where `E0239` called it an associated type. Stage 1
     decides it (RULES.md, R68).

## R69. `sisu lint`'s options, the audit of one file, and `fmt` of a file whose one error is `E0130` (decision 484)

**The rule.** `sisu lint` answers an option it does not take, `--help`
among them, with `D0009` ("unknown option `--help` for `sisu lint`") and
status 2, as the driver's commands do, where it read the option as a path
and stopped with `error: NotFound` and its trace. `sisu audit --unsafe` of
a program of one file heads it "no level", "a program of one file, which
may hold any unsafe code", where "no `unsafe`" read as if the file held
none above the blocks it lists. And `E0130` (`pub(crate)`, R54) is no
syntax error to the formatter: it is given of a tree the parser built
whole, a name `pub(...)` does not take, which `check` reports; `sisu fmt`
formats such a file as written, as it formats a file with any other error
that is not of syntax, and `parse --shape` gives its shape, where `fmt`
refused it (`D0075`) though it parses.

**Why.** The manual's findings ("`sisu lint --help` ... stops with
`error: NotFound`", R16; "a one-file program with an unsafe block is
headed "no `unsafe`"", R16, chapter 27, G19; "A file whose one error is
E0130 ... parses, and `sisu fmt` refuses it", R17, R19). The formatter
refuses a text whose tree is not the file's (02, "an AST-equality
self-check"); `E0130`'s is.

**In stage 1.** `main` (`lint`'s options, `unknown_option`; the audit's
file), `audit.unsafe_report`, `fmt.refuses` and `fmt.shape.shape`.

**What it changes.** The case `cases/unsafe/operations.audit` (its first
line). The formatter's verdict on `cases/findings5/parse_fixes.sisu` is
unchanged: it has other syntax errors.

**Decision 484**, for docs/architecture.md:

484. `sisu lint` answers an option it does not take with `D0009` and
     status 2; the audit heads a program of one file "no level"; a file
     whose only parse diagnostic is `E0130` is formatted, its tree being
     whole. Stage 1 decides it (RULES.md, R69).

## R70. A derived impl's diagnostics at the derive, and the derives' messages (decision 485)

**The rule.** A diagnostic in an impl that a derive writes (a trait's
`comptime def derive`, or the prelude's), which stage 0 places in the
module the compiler wrote, `<derive>/main.sisu`, is placed at the
derive's argument that asked for the impl (`@derive(Csv)`'s `Csv`), with
its message and parameters, the note `type.derived-code` ("in the impl of
`Csv` that the derive writes for `Sauna`: `cells.push("{self.stove}")`")
and neither the labels nor the fixes, which point into code the program
does not have (E0347's `:?`). The derives' messages: `@derive(Encode)`
(`Decode`, `Plain`) without its import is `E0243` with the note
`resolve.derive-import` ("`Encode` is a trait of `std.json`, whose derive
is known where it is imported: `from std.json import Encode`"); the fix
"derive it" is not offered for a type that derives the trait already,
`Pair[Plain]` of `@derive(Eq) struct Pair[T]`, but for the type argument
that lacks it, `Plain`; and `E0347` for a type of the module without
`Arbitrary` under a bound `T: Arbitrary` gets the fix `@derive(Arbitrary)`.

**Why.** The manual's findings ("A diagnostic in a derived impl ... is
placed in `<derive>/main.sisu` with no label at the `@derive`", chapter
25, R14; "E0347 for a field type without `Display`, at
`<derive>/main.sisu`, offers the fix `:?`", G21; "`@derive(Encode,
Decode)` without `from std.json import ...` is E0243", G7; "E0347 for
`Pair(Plain(1), Plain(2)) == ...` offers "derive it" for `Pair`", R14;
"E0347 for a type without `Arbitrary` ... has no fix", G11); the sixth
round's open item.

**In stage 1.** `typeck.instances` (`DeriveOrigin`, recorded where each
template is unfolded, and `E0243`'s note), `typeck.placed_at_derive` (where
a program's files' diagnostics are gathered), `typeck.check.derive_fix`
and `derive_edit`.

**What it changes.** The cases `cases/findings7/derive_placed.sisu` and
`derive_messages.sisu` (their `.diag`), and `cases/unsafe/comptime_written.sisu`
(R5), whose `E0615` in the derived impl is now at `@derive(Named)` (its
`.diag` blessed again; its differences are R5's).

- Differs: `compiler/stage1/cases/findings7/derive_placed.sisu`: ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/findings7/derive_messages.sisu`: ids, human, doc, doc-output

**Documents to bring along:** the manual's transcripts of a derive's
diagnostics (`guide-derive/stove`, chapter 25's `derive_heater`).

**Decision 485**, for docs/architecture.md:

485. A diagnostic in an impl a derive writes is placed at the derive's
     argument that asked for it, with a note giving the impl's line and
     without the labels and fixes that point into the generated text;
     `E0243` for a std module's derivable trait names its import; the fix
     "derive it" goes to the type argument that lacks the trait when the
     type derives it, and is offered for `Arbitrary`. Stage 1 decides it
     (RULES.md, R70).

## R71. A dependency that `when` excludes, and a call of a variadic function (decision 486)

**The rule.** An import of a dependency whose `when` excludes the target
is `E0275` (`resolve.dependency-excluded`, "cannot import `win`: the
dependency `win` of `app` is not built for this target, which its `when`
in that package's `sisu.toml` excludes"), where `E0260` asked to add to
`[dependencies]` what is there. And a call of a function with a variadic
parameter is not checked against the function's parameters, which R64's
`E0900` refuses: one error, where `E0305` counted the arguments against
the parameters before the variadic one.

**Why.** The manual's findings ("An import of a dependency whose `when`
excludes the target is E0260, whose message asks to add it to
`[dependencies]`", R13); the sixth round's open item (R64).

**In stage 1.** `main.layout_input` (each package's excluded
dependencies), `typeck.collect` (`SourcePackage.excluded`,
`PackageData.excluded`, the import's check; the variadic parameter's
`header_broken`).

**What it changes.** The cases `cases/packages/excluded_dependency.pkg`
(its `.diag`) and `cases/findings7/variadic_call.sisu` (its `.diag`).

- Differs: `compiler/stage1/cases/packages/excluded_dependency.pkg`: ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/findings7/variadic_call.sisu`: ids, human, doc, doc-output

**Decision 486**, for docs/architecture.md:

486. An import of a dependency whose `when` excludes the target is
     `E0275`, which says so, where `E0260` asked to add it to
     `[dependencies]`; a call of a function with a variadic parameter is
     not checked against its parameters, so R64's `E0900` is its one
     error. Stage 1 decides it (RULES.md, R71).

## R72. Habits a parse error stops at: `lambda`, `f"..."`, `class`, `try:`, `go f()`, a keyword as a name (decision 487)

**The rule.** Where a parse error stops at another language's spelling,
R54's fix writes Sisu's, or a note says what Sisu has instead: Python's
`lambda x: x + 1` (`E0117`) gets `x -> x + 1` (`(a, b) ->` for two
parameters); `f"n is {n}"` (`E0100`) the string without the `f`, since
every string interpolates; `class Point:` (`E0107`) `struct`; `try:`,
`except:` and `finally:` (`E0117`) the note `parse.no-exceptions` ("Sisu
has no `try`: an error is a value ..."); Go's `go f()` (`E0117`) the note
`parse.go-task` ("a task starts in a scope ..."); and a keyword where a
binding's name goes, `let type = "csv"` (`E0105`), the note
`parse.keyword-name` ("`type` is a keyword, which names no binding").

**Why.** The manual's findings ("Some habits still get a diagnostic that
does not name the Sisu spelling: `lambda`, `f"..."`, `class`, `try:` ...
and Go's `go f()`", R19, G3, G4; "A keyword used as a name ... gets E0105
and E0103, neither saying that the word is a keyword", chapter 2).

**In stage 1.** `parser.habits` (`with_habit`), `parser.items` (the
top-level statement's diagnostic through it).

**What it changes.** The case `cases/findings7/habits.sisu` (its `.diag`).

- Differs: `compiler/stage1/cases/findings7/habits.sisu`: tree, ids, human, fmt-check, doc, doc-output

**Decision 487**, for docs/architecture.md:

487. A parse error at Python's `lambda`, `f"..."` or `class` gets the fix
     that writes Sisu's spelling; at `try:`, `except:`, `finally:` and Go's
     `go f()` a note that says what Sisu has instead; at a keyword where a
     binding's name goes a note that it is a keyword. Stage 1 decides it
     (RULES.md, R72).

## R73. The name of a test, a suite or a benchmark is fixed text (decision 520)

**The rule.** The name of a `test`, a `suite` or a `bench` is a string
literal whose text is fixed when the declaration is: an interpolation in
it, `test "len({3, 1})":`, is `E0132` (`parse.interpolated-name`, "the name
of a test, a suite or a benchmark is fixed text, and `{` starts no
interpolation in it; a brace is written `{{` or `}}`"), at the braces,
with the fix that writes them as text (`{{3, 1}}`), and what the braces
hold is not parsed as an expression. An escape (`\"`) and a doubled brace
are text, as in every string literal (02).

**Why.** The manual's finding ("A test's name is interpolated, so `test
"len({3, 1})":` is E0103, and an escaped quote in a test's name is E0005",
chapter 5). 02 makes every ordinary string literal interpolate, so the
braces start an interpolation in a test's name too; but a name is what the
runner reports, filters and replays by, which no run-time value can be,
and stage 1 dropped the interpolation's value without a word
(`test "n {N}":` was the test "n "). The escaped quote's `E0005` was the
lexer's, inside the interpolation the braces began: with the braces
doubled it is text.

**In stage 1.** `parser.items` (`name_string`, for `test_def` and
`suite_def`).

**What it changes.** The cases `cases/findings8/test_names.sisu` (its
`.diag`) and `test_names_text.sisu` (its `.tests`).

- Differs: `compiler/stage1/cases/findings8/test_names.sisu`: tree, trivia-tree, ast, ids, human, fmt-check, doc, doc-output

**Decision 520**, for docs/architecture.md:

520. The name of a test, a suite or a benchmark is fixed text: an
     interpolation in it is `E0132`, with the fix that doubles its braces,
     and what the braces hold is not parsed. Stage 1 decides it (RULES.md,
     R73).

## R74. A module's name gives way to an import named like it; a local that hides a module (decision 521)

**The rule.** The name of a child module, which 09 binds in its parent
(the directory's module, or a package's root module for the files of
`src/`) without a declaration, gives way to an import of the same name
in the parent: `from bookings.clean import clean` in the root module of
the package `bookings` binds the function, where it was `E0202`
("defined more than once", "first defined here" at line 1, column 1) and
each call `E0309`. The module is still reached by its path, in that
module and every other (`from bookings.clean import ...`). A declaration
named like the child module stays `E0202` (`cases/packages/names.pkg`'s
`def point` beside `geo/point.sisu`): an expression's path `geo.point`
could not tell the two apart, where an import says which is meant. And a
local named like a module the module imports, `fs: Fs` beside `import
std.fs`, hides the module, as any local hides a module's item: `E0304`
for a method of the local's type it does not have (`fs.walk(...)`) gets
the note `type.hidden-module` ("`fs` here is a local, which hides the
module `std.fs` that this module imports: give the local another name").

**Why.** The manual's findings ("A module that imports a function named
like the module ... gets E0202 ... with its "first defined here" label at
line 1, column 1", chapters 17 and 22; "A parameter named like an imported
module ... hides the module, and each call such as `fs.walk(...)` is
E0304 with no word about the module", chapter 21). 09 makes a file a
module and names a module by its path from the package's name; it binds
no name for the module in its parent that an import there could collide
with, and a program that names its module and the function it imports
from it alike is ordinary Python.

**In stage 1.** `resolve.Resolver.bind_import` (the child's binding, at
0..0, replaced), `typeck.collect` (`ModuleData.children`, which an
import's path goes through, `child_link`, `bind_import`, `import_target`,
`package_module`, `item_imported`), `typeck.call.hidden_module_note`.

**What it changes.** The cases `cases/packages/module_named_item.pkg`
(its `.out`: it runs, interpreted and natively) and
`cases/findings8/hidden_module.sisu` (its `.diag`).

- Differs: `compiler/stage1/cases/packages/module_named_item.pkg`: names, types, sir, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/findings8/hidden_module.sisu`: human, doc, doc-output

**Decision 521**, for docs/architecture.md:

521. The name a child module has in its parent gives way to an import of
     that name there, and the module is still reached by its path; `E0304` on a local named like an imported module says the
     local hides it. Stage 1 decides it (RULES.md, R74).

## R75. A dev-dependency is imported only by the package's tests (decision 522)

**The rule.** A package's dev-dependency (`[dev-dependencies]`) is for its
`test` blocks, its benchmarks and its black-box tests (R65): its own code
imports it only by an import marked `@cfg(test)`, which a build without
the tests leaves out with the items that use it (08). Any other import of
it is `E0276` (`resolve.dev-dependency`, "cannot import `fixtures` here:
`fixtures` is a dev-dependency of `ledger`, which its tests import and its
code does not"), with the note `resolve.dev-dependency-tests` ("a
dev-dependency is for `test` blocks, benchmarks and the black-box tests
of `tests/`: import it under `@cfg(test)`, or list it under
`[dependencies]`"), in every build, so that `sisu check` (which checks the
tests too) says it as `sisu build` would. The black-box tests' file
imports the dev-dependencies freely, as before.

**Why.** The manual's finding ("A dev-dependency can be imported by the
package's own code, not only by its tests, in both front ends", R13) and
the seventh round's open item: a package built as a dependency is built
without its dev-dependencies (09's table and the lockfile have them for
the tests only), so code that imports one compiled in the package's own
checkout and could not be built by its users. 08 gives `@cfg(test)` on
items and imports; Sisu has no import inside a test block (an import
inside a function is `E0900`), so the mark is what says an import is the
tests'.

**In stage 1.** `package` (`Want.dev`, a package's own dev-dependency),
`main.layout_input` (`SourcePackage.dev_only`), `typeck.collect`
(`PackageData.dev_only`, `declare_import`'s `import_for_tests`, the check
in `import_target`).

**What it changes.** The case `cases/packages/dev_dependency_code.pkg`
(its `.diag`: a `@cfg(test)` import its unit test uses, and an import its
code uses). `cases/packages/black_box.pkg` is as it was.

- Differs: `compiler/stage1/cases/packages/dev_dependency_code.pkg`: sir, ids, human, doc, doc-output

**Decision 522**, for docs/architecture.md:

522. A package's code imports its dev-dependency only by an import marked
     `@cfg(test)`; any other import of it is `E0276`, in every build, and
     the black-box tests of `tests/` import it freely. Stage 1 decides it
     (RULES.md, R75).

## R76. The fixes the manual's writers found missing, of the round's diagnostics (decision 523)

**The rule.** Where a diagnostic missed the word or the fix that says what
Sisu writes:

- `self.save(x) else as e:` as a statement (`E0117` at `else`) gets the
  note `parse.else-needs-let` ("`else` belongs to a `let`, ...; a
  statement whose error alone is handled is `if let Err(e) = f():`") and,
  with `as NAME:`, the fix `if let Err(e) = save(4)`.
- Python's `d.get(k, default)` (`E0305`, of a `get` of one parameter that
  gives an option) gets the fix `type.get-default`, `d.get(k) ?? default`.
- A positional-only parameter passed by its name, `clamp(value=15, ...)`,
  is `E0306` with the note `type.positional-only` ("`value` is a
  positional-only parameter, which is passed without its name") and the
  fix that drops `value=`; the argument is taken as passed, so that no
  `E0308` follows with a fix that adds `value=todo("value")`, the same
  mistake.
- A std module's error set written alone, `JsonError` (`E0201`), is named
  through the module: with `import std.json`, the fix `json.JsonError`;
  without, the note `resolve.std-type-import` ("`JsonError` is a type of
  `std.json`, known where it is imported: `from std.json import
  JsonError`"); the similar name in scope (`IoError`) is no fix.
- `E0981` names a method of `ptr mut T` as the program writes the type,
  `(ptr mut T).read`, not by the builtin's name, `ptr_mut.read`.

**Why.** The manual's findings, "Diagnostics without fixes": "`self.save(x)
else as e:` as a statement is E0117 ... with no word that `else` belongs
to a `let`, nor the fix `if let Err(e) = ...`" (chapter 22); "`d.get(k,
default)` (E0305)" among the habits that name no Sisu spelling (R19, G3,
G4; the place the fifth round left of R72's); "A positional-only parameter
passed by name gets E0306 ... and E0308, whose fix adds
`value=todo("value")`, the same mistake again" (chapter 3); "`JsonError`
alone is E0201 with the fix `IoError`" (chapter 21); "E0981 names a builtin
by its internal name, `ptr_mut.read`" (chapter 27).

**In stage 1.** `parser.habits` (`with_habit`), `typeck.call.check_args`,
`resolve.Resolver.undefined_type` (`STD_ERROR_SETS`, the five `pub error`
sets of std's modules), `typeck.safety.unsafe_call`.

**What it changes.** The cases `cases/findings8/else_statement.sisu` and
`fixes.sisu` (their `.diag`), and `cases/type_calls.sisu`, whose
positional-only argument passed by name is no longer followed by
`E0308`.

- Differs: `compiler/stage1/cases/findings8/else_statement.sisu`: tree, ids, human, fmt-check, doc, doc-output
- Differs: `compiler/stage1/cases/findings8/fixes.sisu`: ast, names, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/type_calls.sisu`: types, ids

**Decision 523**, for docs/architecture.md:

523. `else` after a statement that is no `let` notes that it belongs to a
     `let` and fixes `else as e` to `if let Err(e) = ...`; `d.get(k,
     default)` is fixed to `d.get(k) ?? default`; a positional-only
     parameter passed by name is noted, fixed and taken as passed; a std
     module's error set written alone is named through its module; and
     `E0981` writes `ptr mut T` as the program does. Stage 1 decides it
     (RULES.md, R76).

## R77. A template's instance at its call, a derive's hole once, and the notes of the round (decision 524)

**The rule.** A diagnostic in a template's instance, which stage 0
places in the module the compiler wrote (`<make[User]>/main.sisu`), is
placed at the call that made the instance (through the calls in other
instances, to the first in a module the program wrote), with its message
and parameters, the note `type.instance-code` ("in the instance of
`make[User]` that this call makes: `return __T0(name=name)`") and
neither its labels nor its fixes, as R70 places a derived impl's; an
`E0300` that finds a reference where a value goes there gets the note
`type.instance-owns` (a parameter the template gives on, as to
`reflect.construct`, is taken `own`). This covers code that `comptime
for` unrolls. A typed hole in a derive's template is reported once, at
the hole when the module writes it there once, where R70 placed it at
each derive. An interpolated literal of compile-time values only, the
value of a `let` or a `var` without a type in a template, stays the
`String` it is in the template (02): the instance writes
`"User".to_string()`, where its plain literal was a `str` and `return out`
from a function that returns a `String` was `E0300` (where a type is
expected, a `return`'s value or an annotated binding's, the plain literal
takes it already, as stage 0's SIR has it). And the notes: `E0342` for `it` outside a call's
argument (a list of functions, `[heat, it - 15]`) says that `it` makes a
closure of a call's argument only, as 02 has it, and to write the closure;
`E0369` comparing a `T ! E` with a `T` (`ensures result > 0` of a function
that returns one) says what compares; `E0438` for what `sorted` gave
says that a list sorts itself in place (`xs.sort()`), where the note
about copies named nothing its caller can do; and a conflict with a
borrow that lasts through the loop the conflicting statement is in
(`E0405` for `xs.push` inside `for x in xs`) has the note
`borrow.loop-ends` ("the borrow lasts through the loop, which ends after
line 9: change `xs` after the loop, or loop over a copy"), where "move
this after line 9" named the loop's last line, inside the loop.

**Why.** The manual's findings: "An error in code that `comptime for`
unrolls is reported in a module the compiler writes ... and nothing
points at the loop's line or the call that instantiated it" (X7);
"`reflect.construct(T, {"name": name})` in a template whose `name` is a
plain `String` parameter is E0300 ... in the instance's module; an `own`
parameter works, and nothing says so" (R12); "a hole in a derive's
template is reported once for each type" (chapter 25, R14; assigned to
stage 1); "In a template, an interpolated literal made only of
compile-time values ... becomes a `str` in the instance" (R14); "`it` in a
list literal of functions ... is E0342, where an explicit closure works"
(chapter 26); "`ensures result > 0` is E0369 with no word that
`result.is_ok()` or `result == Ok(...)` is what compares" (R11); "`sorted`
of a list whose items are not `Copy` ... with a note about copies that a
caller of `sorted` cannot act on; `xs.sort()` works" (chapters 7 and 14);
"A change inside a loop over the same list or dict is E0405, whose note
says to move the change "after line N", N being the last line of the
loop's body, inside the loop" (chapter 11).

**In stage 1.** `typeck.collect` (`instance_sites`), `typeck.instances`
(recorded where an instance's module is declared), `typeck.placed_at_call`
and `once_at_template_hole` (where a program's files' diagnostics are
gathered), `typeck.template.kept_string`, `typeck.expr` (`E0342`'s and
`E0369`'s notes), `typeck.check.check_ref_items`,
`borrowck.loans.in_loop_of`.

**What it changes.** The cases `cases/findings8/instances.sisu`,
`notes.sisu` and `loop_change.sisu` (their `.diag`) and
`template_text.sisu` (its `.out`: it runs, interpreted and natively);
the conformance suite's `reflect/test_instances`, whose `E0807` and
`E0381` in a test template's instance are now at the test's name (its
`.diag` blessed again, and its comment saying so), and the four programs whose change inside a loop gets
the loop's note (`human`). What the names pass, the lexer and the parser
find in an instance is the template's own text and stays in the instance
(`cases/checker_names/template_names.sisu`, `instance_syntax_error.sisu`
as they were).

- Differs: `compiler/stage1/cases/findings8/instances.sisu`: ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/findings8/notes.sisu`: ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/findings8/loop_change.sisu`: human
- Differs: `compiler/stage1/cases/findings8/template_text.sisu`: ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/reflect/test_instances.sisu`: types, ids, human
- Differs: `compiler/conformance/cases/ownership/iterate_and_push.sisu`: human
- Differs: `compiler/conformance/cases/ownership/rustc/for_loop_head_linkage.sisu`: human
- Differs: `compiler/conformance/cases/wrong/dict_changed_while_iterating.sisu`: human
- Differs: `compiler/conformance/cases/wrong/push_while_iterating.sisu`: human

**Decision 524**, for docs/architecture.md:

524. A diagnostic in a template's instance is placed at the call that made
     it, with a note giving the instance's line and without the labels and
     fixes that point into the generated text; a typed hole of a derive's
     template is reported once; an untyped binding's interpolated literal
     of compile-time values stays a `String` in the instance; and
     `E0342`, `E0369`, `E0438` (of `sorted`) and a conflict inside a loop
     the borrow lasts through say what the program writes instead. Stage 1
     decides it (RULES.md, R77).

## R78. A literal beside a `BigInt` or a `Decimal`; an explicit discriminant (decision 525)

**The rule.** An integer or float literal that is an operand of an
arithmetic operator beside a value of a type that is no number and
implements `FromLiteral` for the literal's type, `f * 2` and `2 * f` of a
`BigInt` or `d + 0.25` of a `Decimal`, is a value of that type, made
through `FromLiteral` as a literal where that type is expected is (a
float reaching a `Decimal` from its text, R33), when the operator has no
impl that takes the literal as it is (`impl Mul[int] for Vector` keeps
its `int`). And an explicit discriminant of an enum's variant, `Red = 1`,
which 03 gives, is `E0900` ("an explicit discriminant of an enum's
variant is not implemented in stage 0 yet; it arrives with the layouts of
`@repr` enums (03, 12)"), where it was `E0117`; `@repr(u8)` stays accepted
as 03 allows it, its layout being the backend's.

**Why.** The manual's findings ("A literal beside a `BigInt` or a
`Decimal` does not take its type: `f * 2` and `d + 0.25` are E0310, though
D-076 gives the exact types literals through `FromLiteral` and the
ordinary operators", chapter 24; "`@repr(u8)` on an enum is accepted and
changes nothing, an explicit discriminant `Red = 1` is E0117", R2).

**In stage 1.** `typeck.expr.binary_expr` and `literal_beside`;
`parser.items.variant_member`.

**What it changes.** The cases `cases/findings8/literals.sisu` (its
`.out`: it runs, interpreted and natively) and `discriminants.sisu` (its
`.diag`).

- Differs: `compiler/stage1/cases/findings8/literals.sisu`: ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/findings8/discriminants.sisu`: tree, ids, human, fmt-check, doc, doc-output

**Decision 525**, for docs/architecture.md:

525. A literal operand beside a non-numeric type that implements
     `FromLiteral` for it becomes that type when the operator has no impl
     for the literal as it is; an explicit discriminant is `E0900` until
     the layouts of `@repr` enums are lowered. Stage 1 decides it
     (RULES.md, R78).

## R79. The programs of `sisu repl` and `sisu ws`: `sir --repl` and `sir --worksheet` (decision 580)

**The rule.** Stage 1 builds the program a REPL session or a worksheet
runs, as the driver's `repl.rs` writes it for the Rust front end, so that
the driver runs `sisu repl` and `sisu ws` on stage 1 and stage 0's
interpreter alone:

- `sisu1 sir --repl SESSION INPUT --kept OUT [--load]`. SESSION holds the
  session's program so far: its `#!` line (added when it has none, so a
  new session's SESSION is empty, or holds only the inline metadata the
  driver writes for a loaded package) and the inputs it kept, each as the
  OUT of its own `--repl` wrote it. INPUT is the new input as typed. The
  program is SESSION, the input (an input that is one expression
  statement wrapped to be shown, `__repl_show(INPUT\n)`, anything else as
  it is, each ending with a line break), `pass` and the function
  `__repl_show`, which prints a value's `Debug` form unless it is `()`.
  It is built as `sisu run` builds a script (without its tests, every
  contract checked): its SIR on standard output and status 0, OUT then
  holding SESSION and the input as the program keeps it (the driver makes
  OUT the next SESSION when the run keeps the input); or, when it does not
  build, the diagnostics of the phase that stopped it on standard error
  (the front end's, warnings with them, then the failed `comptime`
  expressions, then the ownership checker's, no `D0006` line after them)
  and `error: Failed`, status 1, OUT not written. Those inside the input
  are drawn in its text, as `<input>`, the others in the session's, as
  `<repl>`; a label outside the input is drawn in `<repl>` too. With
  `--load`, INPUT is a file loaded into the session (`sisu repl FILE`):
  its `#!` line and inline metadata lead the program and the rest is the
  input, kept as it is, its diagnostics drawn in the whole file under the
  name given. Paths in the program's inline metadata are relative to the
  working directory, as the Rust front end's are.
- `sisu1 sir --worksheet FILE`: the worksheet (a script, a `#!` line added
  when it has none), checked as written; if it does not build, the
  diagnostics as above, drawn in the file as written (where the Rust front
  end's lines counted the `#!` line it added), status 1. Else the
  worksheet with each top-level statement (not an item, an attribute or
  an error) preceded by `__ws_at(L)` and followed by `__ws_show(L, value)`
  (an expression statement's value, or each name a `let` or `var` binds),
  `L` the line the statement ends on in the file as written, and the
  helpers, `__ws_at` printing `\u{1}L`, `__ws_show` printing `\u{2}L VALUE`
  unless the value is `()`, and `__ws_fake`, which keeps `System.fake` in
  the SIR for the fake `System` a worksheet runs against: its SIR on
  standard output, status 0. The driver runs it (`run_main_faked`, or the
  real `System` with `--run`) and writes each line of the file with the
  values its markers give that line, then what was printed after a marker
  under that line, as now; it no longer needs the statements' spans.

**Why.** port-debt.md ("The Rust front end's roles still to replace"):
the REPL and worksheets need only a program checked and lowered per input,
and they ran on the frozen front end, so a worksheet lacked every rule
stage 1 has gained (the manual's finding, chapter 5 and R16: `max(scores)`,
`^` in a slice's bound, a generator expression and a trait's constant were
errors there). The driver kept the wrapping and the instrumenting, which
need a parser; stage 1 does both, so the driver needs no front end for
them. What the driver still parses itself is `:why NAME`'s reading of the
inputs it kept (which binds a name, what an initialiser reads, which input
changes it), which no check or lowering gives; `sisu1 parse --tree` gives
an input's tree in the form `sisu parse --tree` prints.

**In stage 1.** `session` (the wrapping, a loaded file's preamble, the
worksheet's markers), `main.repl_command`, `worksheet_command`,
`held_build` (a program held in memory, `held_layout_input`) and
`rebased`. `sisu1 --version` is 0.8.0: the driver may rely on both
commands.

**What it changes.** The sessions `cases/repl/session.repl` and
`loaded.repl` (their transcripts, `.repl.out`) and the worksheets
`cases/repl/sheet.ws.sisu`, `rules.ws.sisu` and `wrong.ws.sisu`, which the
classes `repl` and `ws` of `check.sh` run; `rules.ws.sisu` uses R16's `^`
and R61's `max` of a list, which the Rust front end refuses, and
`wrong.ws.sisu` draws its diagnostic in the file as written.

- Differs: `compiler/stage1/cases/repl/rules.ws.sisu`: ws
- Differs: `compiler/stage1/cases/repl/wrong.ws.sisu`: ws

**Decision 580**, for docs/architecture.md:

580. Stage 1 builds the programs of `sisu repl` and `sisu ws`: `sir --repl
     SESSION INPUT --kept OUT [--load]` gives the SIR of the session's
     program with the input after it (an expression alone shown), or the
     diagnostics of the phase that stopped it, those in the input drawn in
     its text; `sir --worksheet FILE` gives the worksheet's SIR with each
     top-level statement marked by the line it ends on in the file as
     written and its values shown, or its diagnostics in the file as
     written. `sisu1 --version` is 0.8.0. Stage 1 decides it (RULES.md,
     R79).

## R80. A sized integer shifted by an amount of any integer type keeps its type (decision 581)

**The rule.** `x << k` and `x >> k` (and `<<=`, `>>=`) of a sized integer
`x` (any of `u8` to `u128`, `i8` to `i128`, not `int`) by an amount `k` of
any integer type, `int` among them, has `x`'s type: the amount is a count
of bits, which neither widens to `x`'s type nor widens `x` to its own, and
an amount below 0 or not below the width traps (`panic.shift-out-of-range-
sized`), as it did for an amount of `x`'s type. A literal on the left of
a shift keeps its own type (`int`) beside a sized amount, where it took
the amount's. `int << k` with a sized `k` widens `k`, as before.

**Why.** The stage 0 round's item (and the manual's finding, chapter 24:
"A `u32` shifted by an `int`, `s << k`, widens to `int`, so `1u32 << 32`
with the amount in an `int` is 4294967296, where a `u32` amount panics";
and std has no `count_ones`, `leading_zeros` or `rotate_left`, which a
checksum reaches for): `u64 << k` with `k` an `int` was `E0310`, since
D-016's widening goes neither way between a `u64` and an `int`. 03 says
"Shifts by more than the width trap", which is the width of the value
shifted: a shift is not an arithmetic operator of two values of one type,
and D-016's implicit widening, which is lossless, has nothing to widen
here. The sixth round decided otherwise for `u32 << int` (the widening to
`int`), reading a shift as an operator as the others; with `u64`, which
has no wider type, that reading leaves the bit methods unwritable, so
this round reverses it.

**In stage 1.** `typeck.expr.binary_expr` (`shift`, `shifts_sized`) and
`binary_result`; lowering and both backends took a sized intrinsic's
amount of another integer type already (`u64.shl` reads its second operand
as an integer, natively by its own type's flag).

**What it changes.** The case `cases/findings9/shifts.sisu` (its `.out`:
it runs, interpreted and natively).

- Differs: `compiler/stage1/cases/findings9/shifts.sisu`: tree, trivia-tree, ast, names, types, sir, ownership, sir-tests, ids, human, fmt, fmt-check, doc, doc-output

**Decision 581**, for docs/architecture.md:

581. A sized integer shifted by an amount of any integer type keeps its
     own type, and an amount outside its width traps; a literal on a
     shift's left keeps `int`. This reverses the sixth round's reading of
     `u32 << int` as widening to `int`. Stage 1 decides it (RULES.md,
     R80).

## R81. `expect cond, "message"` (decision 582)

**The rule.** `expect` takes a message as `assert` does: `expect cond,
message`, the message a `str` or a `String` (an interpolated literal
among them), which a failure `expect` records shows under its first line
(`R0155`, decision 545's `run.expect-failed-message`), and which outside
a test is `assert`'s panic with its message.

**Why.** The stage 0 round's item (the manual's finding, chapter 19 and
R15): `expect cond, "message"`, the form `assert` takes, was `E0117`
("unexpected `,` after the end of the statement"), though 17 gives
`expect` as `assert`'s non-fatal form and the intrinsics a failed `expect`
lowers to take a message (R38).

**In stage 1.** `parser.stmts` (the `ExpectStmt`'s message), the AST's
`StmtKind.Expect(cond, message)`, `typeck.check` (`assert_message`), the
template's copy and walk.

**What it changes.** The case `cases/findings9/expect_message.sisu` (its
`.tests`).

- Differs: `compiler/stage1/cases/findings9/expect_message.sisu`: tree, trivia-tree, ast, names, types, sir, ownership, sir-tests, ids, human, fmt, fmt-check, doc, doc-output

**Decision 582**, for docs/architecture.md:

582. `expect` takes a message as `assert` does, `expect cond, message`,
     which its recorded failure shows. Stage 1 decides it (RULES.md, R81).

## R82. An assertion whose call takes a call shows its values (decision 583)

**The rule.** An `assert` or `expect` whose condition is a call of a
function or a method, and calls another function in an argument
(`temps.contains(average(temps))`), which the failure's parts cannot
evaluate again, evaluates each such argument once, into a temporary,
before the call, when its type needs no drop; the failure then shows those
arguments' values and the parts that only read (names and their fields),
drawn under the condition as a condition that only reads has them (17,
"Assertions"). Arguments are evaluated in their order, before the call,
as the call evaluates them.

**Why.** The stage 0 round's item (the manual's finding, chapter 19): "An
assertion whose parts call the program's own functions inside a method
call, `assert temps.contains(average(temps))`, prints no values, though 17
shows every sub-expression." Re-evaluating a call to show it could run its
effects twice, which is why such a condition showed nothing; evaluating it
once shows its value without that.

**In stage 1.** `lower.stmts.assert_`, `lower.contracts` (`evaluate_once`,
`read_again`, `assert_showing_parts`), `lower.exprs` (`FnLower.evaluated`,
read by `place_of` and `into`).

**What it changes.** The case `cases/findings9/assert_calls.sisu` (its
`.tests`), and `cases/testing/expect.sisu`, whose `.sir-tests` is blessed
again: its `expect even(double(3) + 1)` shows `double(3) + 1`.

- Differs: `compiler/stage1/cases/findings9/assert_calls.sisu`: tree, trivia-tree, ast, names, types, sir, ownership, sir-tests, ids, human, fmt, fmt-check, doc, doc-output

**Decision 583**, for docs/architecture.md:

583. An assertion whose condition is a call that takes a call evaluates
     that argument once and shows its value and the parts that only read
     when it fails. Stage 1 decides it (RULES.md, R82).

## R83. The test `@refines` makes stands at the attribute (decision 584)

**The rule.** The test that `@refines(reference)` adds, `test "f refines
reference"`, and the closure it checks its cases with, stand at the
`@refines` attribute in the file of the function it checks: its frames,
and a failure's `at test ... (FILE:LINE:COL)`, name the attribute's line,
where they named the module the compiler writes, `<arbitrary>/main.sisu`.

**Why.** The stage 0 round's item (the manual's finding, chapter 19): "The
test that `@refines` adds fails at `<arbitrary>/main.sisu:7:5`, in the
module the compiler writes, rather than at the `@refines` line." A reader
cannot open that module; the attribute is what asked for the test.

**In stage 1.** `typeck.arbitrary.declare_refinements`
(`Program.refines_at`), `lower` (`stand_at`).

**What it changes.** The case `cases/findings9/refines_place.sisu` (its
`.tests`), and the conformance suite's `testing/refines`, whose two
failures now name `refines.sisu:16:1` and `refines.sisu:34:1`.

- Differs: `compiler/stage1/cases/findings9/refines_place.sisu`: tree, trivia-tree, ast, names, types, sir, ownership, sir-tests, ids, human, fmt, fmt-check, doc, doc-output
- Differs: `compiler/conformance/cases/testing/refines.sisu`: sir-tests

**Decision 584**, for docs/architecture.md:

584. The test `@refines` adds stands at the attribute, in the file of the
     function it checks, with the closure it checks the cases with. Stage
     1 decides it (RULES.md, R83).

## R84. A typed hole reached at compile time leaves the expression to run time (decision 585)

**The rule.** A `comptime` expression whose evaluation reaches a typed
hole (`todo(...)`) is not an error: it has no value to bake, so it is left
in the program as a function the run calls, and reaching the hole there
blocks the test it is in (R27) or stops the run with the hole's panic, as
any hole does; the build warns of the hole (`W0001`), and a release build
refuses it (`E0445`), as for every hole. A `comptime` expression that uses
such a one reaches the hole too.

**Why.** The stage 0 round's item (the manual's finding, chapter 25): "A
typed hole reached at compile time is E0602, so a test that uses a
constant computed from a hole does not compile instead of being blocked."
D-061 makes a hole a state a program may be in while it is written: what
does not depend on it runs, and what does is blocked. A constant computed
from a hole is the second.

**In stage 1.** `interp` (`Stop.Hole`, and `visit`, which makes the
function a value function with its body).

**What it changes.** The case `cases/findings9/comptime_hole.sisu` (its
`.tests`: one test blocked, one passed).

- Differs: `compiler/stage1/cases/findings9/comptime_hole.sisu`: tree, trivia-tree, ast, names, types, sir, ownership, sir-tests, ids, human, fmt, fmt-check, doc, doc-output

**Decision 585**, for docs/architecture.md:

585. A `comptime` expression that reaches a typed hole is left to run time,
     where the hole blocks the test or the run that reaches it; the build
     warns of the hole. Stage 1 decides it (RULES.md, R84).

## R85. `E1022` ends with the line of the command that stopped (decision 586)

**The rule.** A lockfile whose `unsafe` level a package raises past
(`E1022`, without `--accept-unsafe`) is followed by the line of the command
it stops, as the manifests' other errors are (`E1019`, `E1021`):
`error[D0006]: the program was not run: errors found (1)` for `run`, `sir`
and `test`, `error[D0132]: the program was not compiled: errors found
(1)` for `build`; `check` and `doc` print none, as for those errors.

**Why.** The stage 0 round's item (the manual's finding, R13): "E1022 ends
a run without the line D0006 that the other errors of a package's
manifests end with."

**In stage 1.** `main.write_lockfile` (`stopped`).

**What it changes.** No case of the suite: a raised level needs a
lockfile that records a lower one, which `check.sh`'s runs rewrite; the
lines are the commands' own, as for `E1019`.

**Decision 586**, for docs/architecture.md:

586. `E1022` is followed by `D0006` or `D0132`, as the manifests' other
     errors are. Stage 1 decides it (RULES.md, R85).

## R86. `edition` takes an edition the compiler knows (decision 587)

**The rule.** The manifest's `edition` is an edition the compiler knows,
`"2027"`, the one 09's manifest names, the first: another year (`"1999"`)
is `E1003` (`package.bad-value`, "`edition` takes an edition this compiler
knows, `"2027"`, found `"1999"`"), and a value that is no year keeps its
`E1003` ("a year, as in `"2027"`").

**Why.** The stage 0 round's item (the manual's finding, chapter 28): "The
manifest's `edition` takes any text, `edition = "1999"` included, without
a word." 13 makes an edition what the compiler understands, every one of
them, and a new one comes every three years at most; one it does not know
it cannot build as written.

**In stage 1.** `manifest` (`EDITIONS`, `is_edition`).

**What it changes.** The case `cases/packages/unknown_edition.pkg` (its
`.diag`).

- Differs: `compiler/stage1/cases/packages/unknown_edition.pkg`: names, types, sir, ownership, ids, human, doc, doc-output

**Decision 587**, for docs/architecture.md:

587. A manifest's `edition` is one the compiler knows, `"2027"` so far;
     another year is `E1003`. Stage 1 decides it (RULES.md, R86).

## R87. `sisu fmt` keeps an unused import, which `sisu lint` reports (decision 588)

**The rule.** The formatter sorts imports and keeps each, used or not: it
changes a file's layout and nothing else. An import whose name the file
does not spell (outside the imports, or as a word of a doc comment), not a
re-export and holding no comment, is the lint `W0027`
(`lint.unused-import`, "`json` is imported and not used in this file") of
`sisu lint`, at the import (or at the name, in a `from` that keeps
others), with the fix `lint.remove-import` that removes it (the name and
its comma, in a `from`). `check` prints no lint (R11).

**Why.** The stage 0 round's item (the manual's finding, chapter 21 and
R13): "`sisu fmt` removes an import that a file does not use yet, so an
exercise cannot hand the learner the import its solution needs, and an
unused import gets no warning, though 09 says it warns." 10 makes the
formatter the parser's, with no options, and a formatter that removes
code changes what a file says; 09's table says Sisu warns and `sisu fmt`
removes unused imports, which this rule keeps as a lint with the fix that
removes it, which an editor or `sisu fix` applies. A warning of `check`
would be every program's that imports what it does not use yet.

**In stage 1.** `fmt` (`format_with`, `lines.format_tree`,
`shape.shape_of`: no import left out), `fmt.prune.lints`,
`main` (`lint`).

**What it changes.** The case `cases/findings9/unused_import.sisu` (its
`.lint`), and the `fmt` of every file with an unused import, which the
Rust formatter still removes.

- Differs: `compiler/stage1/cases/findings9/unused_import.sisu`: tree, trivia-tree, ast, names, types, sir, ownership, sir-tests, ids, human, fmt, fmt-check, doc, doc-output
- Differs: `compiler/stage1/cases/packages/names.pkg/app/src/main.sisu`: fmt, fmt-check
- Differs: `compiler/stage1/cases/packages/names.pkg/app/src/helpers.sisu`: fmt, fmt-check
- Differs: `compiler/stage1/cases/fmt/imports.sisu`: fmt, fmt-check
- Differs: `compiler/stage1/cases/findings6/import_item.sisu`: fmt, fmt-check
- Differs: `compiler/conformance/cases/wrong/unknown_module.sisu`: fmt, fmt-check
- Differs: `compiler/conformance/cases/packages/visibility.pkg/app/src/main.sisu`: fmt, fmt-check
- Differs: `compiler/conformance/cases/names/unknown_import.sisu`: fmt, fmt-check

**Decision 588**, for docs/architecture.md:

588. `sisu fmt` keeps an unused import; `sisu lint` reports it (`W0027`)
     with the fix that removes it. Stage 1 decides it (RULES.md, R87).

## R88. A task spawned in a loop that changes a captured value, and a task that borrows the body's value (decision 589)

**The rule.** Tasks a loop spawns that change a captured `var`
(`s.spawn do: count += 1`), the next borrowing it while the one before may
still run, get `E0402` naming that value, `count`, where it named the
closure's text as the value borrowed, with the note `task.shared-change`
("each task the loop spawns changes `count`, and the tasks run at once:
keep it in a `Mutex` (`Mutex(0)`, changed through `count.lock().value`),
or have each task return its part and combine them after the scope"),
where the note said to change the closure's text after the loop. And
`E0428` (a task that borrows what the scope's body made) has the note
`task.make-outside` ("make `names` before `task.scope`, outside the
scope's body, so that it outlives every task; `own () -> ...` (or `do
own:`) would move it into the first task and leave none for the next"),
where it offered `own` first; `E0429` keeps `task.use-own`.

**Why.** The stage 0 round's items (the manual's findings, chapter 20):
"Tasks spawned in a loop that change a captured `var` ... get E0402, whose
message names the closure's whole text as the value borrowed, says that it
"cannot be modified", and says nothing about a `Mutex`"; "E0428's note
offers `own () -> ...`, which moves the list into the first task and
leaves none for the next; making the value outside the scope's block,
which works, is not offered." 06 makes shared state a `Mutex` or an
`Atomic`.

**In stage 1.** `borrowck.loans` (`conflict`, `mut_capture`,
`check_tasks`).

**What it changes.** The cases `cases/findings9/shared_change.sisu` and
`body_local.sisu` (their `.diag`).

- Differs: `compiler/stage1/cases/findings9/shared_change.sisu`: tree, trivia-tree, ast, names, types, sir, ownership, sir-tests, ids, human, fmt, fmt-check, doc, doc-output
- Differs: `compiler/stage1/cases/findings9/body_local.sisu`: tree, trivia-tree, ast, names, types, sir, ownership, sir-tests, ids, human, fmt, fmt-check, doc, doc-output
- Differs: `compiler/conformance/cases/wrong/task_borrows_body_local.sisu`: human
- Differs: `compiler/conformance/cases/concurrency/task_borrows.sisu`: human

**Decision 589**, for docs/architecture.md:

589. `E0402` for tasks a loop spawns that change a captured value names
     the value and notes that the tasks run at once and a `Mutex` keeps
     it; `E0428`'s note offers making the value before the scope. Stage 1
     decides it (RULES.md, R88).

## R89. A `?` in a scope's body adds to the error set the function infers (decision 590)

**The rule.** In a function whose error set is inferred (`-> T !`), a
closure argument whose expected type mentions that set, a `task.scope`
body that the function returns, keeps the set open: each `?` in it adds
its error set to the function's, as a `?` in the function's own body
does, where the closure was checked against the set inferred so far and
each such `?` was `E0321`.

**Why.** The stage 0 round's item (the manual's finding, chapter 22): "An
inferred error set, `-> !`, leaves out what a `?` passes on inside a
`task.scope` body: each such `?` is E0321." 05 infers the set from what
the body passes on; the scope's body is part of the function's.

**In stage 1.** `typeck.infer.zonk_open`, `typeck.call.check_arg`.

**What it changes.** The case `cases/findings9/scope_errors.sisu` (its
`.out`: it runs, interpreted and natively).

- Differs: `compiler/stage1/cases/findings9/scope_errors.sisu`: tree, trivia-tree, ast, names, types, sir, ownership, sir-tests, ids, human, fmt, fmt-check, doc, doc-output

**Decision 590**, for docs/architecture.md:

590. A closure argument of a `T !` function keeps the inferred error set
     open, so a `?` in a scope's body adds to it. Stage 1 decides it
     (RULES.md, R89).

## R90. `E0335` names the operation responsible (decision 591)

**The rule.** `E0335` for a function passed where a `pure` one is required
names the operation responsible for its impurity, as 03 asks: the first
impure call of its body, followed through the program's own functions it
calls to the first that is not one (`print`), where it named the function
passed; when the way is longer than one call, the note `type.impure-path`
gives it ("`helper` calls `noisy_square`, which calls `print`").

**Why.** The eighth round's open item (the manual's finding, chapters 16
and 26): "E0335 names the function passed (`noisy_square`) as the impure
callee, where 03 asks for the operation responsible (`print`)." The check
of the call has the function's name, not its body; each body records its
first impure call, and once every body is checked the diagnostics follow
them.

**In stage 1.** `typeck.purity.body_impure`, `typeck.expr`
(`check_purity_coercion`, `PASSED`), `typeck.responsible`.

**What it changes.** The case `cases/findings9/impure_operation.sisu` (its
`.diag`), and the conformance cases with `E0335`.

- Differs: `compiler/stage1/cases/findings9/impure_operation.sisu`: tree, trivia-tree, ast, names, types, sir, ownership, sir-tests, ids, human, fmt, fmt-check, doc, doc-output

**Decision 591**, for docs/architecture.md:

591. `E0335` names the operation that makes the function passed impure,
     followed through the program's functions, with a note giving the way.
     Stage 1 decides it (RULES.md, R90).

## R91. `E0204` for an item a `@cfg` left out names the condition (decision 592)

**The rule.** `E0204` (`from M import X`, where `M` has no `X`) for an
item a `@cfg` of `M` left out of this build has the note
`resolve.cfg-left-out` ("`quick` is left out of this build by its
`@cfg(feature="fast")`, which does not hold"), the condition written as
the attribute has it for `feature="NAME"` and a name (`test`), `...`
otherwise.

**Why.** The eighth round's open item (the manual's finding, chapter 17):
"E0204 for an item that `@cfg(feature=...)` left out does not name the
feature." The filter that leaves the item out now records it with its
module.

**In stage 1.** `cfg` (`Filter.removed`, `item_name`, `condition_text`),
the AST's `Module.left_out`, `typeck.collect.import_from`.

**What it changes.** The case `cases/packages/cfg_feature_import.pkg` (its
`.diag`; the note is the human form's).

- Differs: `compiler/stage1/cases/packages/cfg_feature_import.pkg`: names, types, ids, human, doc, doc-output

**Decision 592**, for docs/architecture.md:

592. `E0204` for an item a `@cfg` left out notes the condition that did.
     Stage 1 decides it (RULES.md, R91).

## R92. A field's `ty` in a derive's template (decision 593)

**The rule.** In a derive's template a struct's field has a `ty`: the type
as the field's declaration writes it, its name that text
(`reflect.name_of(f.ty)`), and its kind as the prelude's names give it
(`int`, `f64`, `bool`, `char`, `str` and `String`, a `list` or an option
of a type so written, with its element, a tuple), so that `reflect.is_str`,
`is_list` and the others choose a form field by field; any other type's
kind is not known to a derive, which runs before the types are resolved,
and its `is_struct` is `False`. A variant's fields have no `ty` yet.

**Why.** The eighth round's open item (the manual's finding, G21):
"Inside a derive's template a field has no `ty` (E0608, "a field has no
`ty`"), though a generic function that reflects reads it, so a derive
cannot choose `Display` or `Debug` field by field."

**In stage 1.** `typeck.template` (`Target.field_types`, `written_shape`,
`TypeShape.of_target`), `typeck.instances`.

**What it changes.** The case `cases/findings9/derive_field_type.sisu`
(its `.out`: it runs, interpreted and natively).

- Differs: `compiler/stage1/cases/findings9/derive_field_type.sisu`: tree, trivia-tree, ast, names, types, sir, ownership, sir-tests, ids, human, fmt, fmt-check, doc, doc-output

**Decision 593**, for docs/architecture.md:

593. A derive's template sees a struct field's `ty` as its declaration
     writes it, with the kinds of the prelude's types. Stage 1 decides it
     (RULES.md, R92).

## R93. A function named `type` (decision 594)

**The rule.** `def type()` is `E0124` ("`type` is a keyword and cannot be
used as a name here"), as a parameter named by a keyword is: `type` starts
a type where an expression goes (`T: type`), so each call of such a
function was `E0103`.

**Why.** The manual's finding (R1): "A function declared with a keyword as
its name, `def type()`, compiles and cannot be called (E0103 at each
call), where a parameter's or a field's name gets E0124."

**In stage 1.** `parser.items.fn_def`.

**What it changes.** The case `cases/findings9/keyword_function.sisu`
(its `.diag`).

- Differs: `compiler/stage1/cases/findings9/keyword_function.sisu`: tree, trivia-tree, ast, names, types, sir, ownership, sir-tests, ids, human, fmt, fmt-check, doc, doc-output

**Decision 594**, for docs/architecture.md:

594. A function cannot be named `type`: `E0124`. Stage 1 decides it
     (RULES.md, R93).

## R94. `parse --shape` refuses what `fmt` refuses; an instance's note writes its type arguments as the source does (decision 595)

**The rule.** `sisu1 parse --shape FILE` prints what formatting keeps
(`fmt.shape`, the shape `fmt`'s self-check compares), or the file's syntax
diagnostics in the conformance form and status 1 when it has an error
`fmt` refuses: a test's name that interpolates (`E0132`, R73) among them,
so the two agree on which files are formatted. And the note
`type.instance-code` of a diagnostic placed at the call that made a
template's instance (R77) writes the instance's line with the type
arguments as the instance's name writes them, `return User(name=name)`,
where it showed the compiler's names for them, `return __T0(name=name)`.

**Why.** The manual round's findings: the conformance runner's format check
asks `sisu parse --shape` (the Rust front end's, which accepts `test "n
{N}":`) whether a file parses and `sisu fmt` (stage 1's, which refuses it
as `E0132`) to format it, so such a file failed the check and a
`.unformatted` did not excuse it; and a reader cannot know what `__T0`
names. The driver routes `parse --shape` to stage 1 (stage 0's side), as
it routes `parse --tree`.

**In stage 1.** `main` (`parse --shape`), `typeck.as_written`.

**What it changes.** The unit tests of `fmt.shape` and `typeck`, and the
human form of `cases/findings8/instances.sisu` (listed under R77). Where
stage 1's shape keeps an unused import (R87), the Rust front end's leaves
it out; the conformance runner compares the shape before and after
formatting, each from the same front end, so they agree either way.

- Differs: `compiler/stage1/cases/findings8/instances.sisu`: ids, human, doc, doc-output

**Decision 595**, for docs/architecture.md:

595. `parse --shape` refuses what `fmt` refuses, `E0132` among it, and a
     template instance's note at its call writes the type arguments as the
     source does. Stage 1 decides it (RULES.md, R94).

## R95. What the driver's REPL needs of stage 1: where the program stands, its name, and an input's bindings (decision 600)

**The rule.** Three additions to R79's commands, so that the driver runs
`sisu repl` and `sisu ws` on stage 1 with no front end of its own:

- `sir --repl SESSION INPUT --kept OUT [--load] [--dir DIR] [--name NAME]`
  and `sir --worksheet FILE [--dir DIR] [--name NAME]`. `--dir DIR` is the
  directory the program stands in: `embed("...")` resolves beside it and
  the inline metadata's paths are relative to it, whatever directory the
  files SESSION, INPUT and FILE were written to; without it, a session
  stands in the working directory and a worksheet in its file's directory,
  as before. `--name NAME` is what the diagnostics and the SIR's sources
  call the program: `<repl>` by default for a session (the input is still
  drawn as `<input>`, or as the loaded file's name with `--load`), the
  file as given for a worksheet. The driver may write the worksheet's text
  to a file of its own (the language server's unsaved sheet) and pass the
  user's name and directory.
- `parse --bindings INPUT`: what `:why NAME` reads of an input, the input
  parsed as the statements of a script (as `sir --repl` parses it). For
  each top-level statement or item, in order, a line `KIND START END`,
  `KIND` one of `let` (a `let` or a `var`), `assign`, `expr`, `item` and
  `other`, `START END` its bytes in INPUT; then, indented by two spaces,
  those of the lines that apply: `binds N...` (the names a `let` or `var`
  binds, its patterns' names in order), `initialiser START END` (the bytes
  of its value), `reads N...` (the names that value reads, each once, in
  order) and `changes N...` (the name an assignment's target starts from,
  `xs` of `xs[0].n = 1`, or each name a method called in an expression
  statement is called on, `xs` of `xs.push(3)`). Names are separated by a
  space; status 0. These are the driver's `bound_names`, `initialiser`,
  `names_read` and `changes`, which read the Rust parser's tree.

`sisu1 --version` is 0.9.0: the driver may rely on the three.

**Why.** The stage 0 round's items (decision 563: the driver built its REPL
on `sir FILE` and `check FILE`, its programs in a temporary directory):
`embed` in an input resolved beside the temporary copy, a program was
named by its temporary path, which the driver replaced in what it printed,
and `:why NAME` still parsed each input with the Rust parser.

**In stage 1.** `main.repl_command`, `worksheet_command` (`standing`,
`base_name`), `held_build` (the files `embed` names, beside where the
program stands), `session.bindings`.

**What it changes.** No output of the corpus; `session`'s unit test of
`bindings`.

**Decision 600**, for docs/architecture.md:

600. Stage 1's `sir --repl` and `sir --worksheet` take `--dir DIR`, where
     the program stands (`embed` and the inline metadata's paths), and
     `--name NAME`, what its diagnostics and SIR call it; `parse --bindings
     INPUT` gives each top-level statement of an input with what it binds,
     its initialiser's bytes, the names that reads and the names it
     changes, which `:why NAME` reads. `sisu1 --version` is 0.9.0. Stage 1
     decides it (RULES.md, R95).

## R96. A variant's fields' `ty` in a derive's template, and the kinds of the program's own types (decision 601)

**The rule.** R92 extended: in a derive's template the fields of an enum's
variants (`V.fields` of `reflect.variants_of(T)`) have a `ty`, the type as
the field's declaration writes it, as a struct's fields have; and a field
whose type is a struct or an enum the module declares or imports by name
has that kind (`reflect.is_struct(f.ty)`, `reflect.is_enum(f.ty)`), where
its kind was not known. Its own fields are still not looked into.

**Why.** The ninth round's open item (the manual's finding, G21, and "What
is next", 11): a variant's field stopped a derive with `E0608` ("a field
has no `ty`"), and the program's own types were neither structs nor
enums to it.

**In stage 1.** `typeck.template` (`TypeShape.of_target`, `written_shape`),
`typeck.instances` (`own_type_kinds`).

**What it changes.** The case `cases/findings10/derive_variant_type.sisu`
(its `.out`: it runs, interpreted and natively).

- Differs: `compiler/stage1/cases/findings10/derive_variant_type.sisu`: ids, human, doc, doc-output

**Decision 601**, for docs/architecture.md:

601. A derive's template sees the `ty` of a variant's fields as written,
     and the kinds of the structs and enums the module names. Stage 1
     decides it (RULES.md, R96).

## R97. `E0203` and `E0271` offer the similar name (decision 602)

**The rule.** `E0203` (`resolve.unknown-module`) for an import path whose
module is one letter off, `from atlas.geo.points import Point` beside the
module `atlas.geo.point`, has the fix `resolve.similar-module` that writes
the path with the similar module, as `E0220` has; and `E0271`
(`resolve.not-a-trait-constant`) for a constant the trait does not declare,
`WARMUP` where it declares `WARM_UP`, has the fix `resolve.similar-name`
with the trait's constant, as `E0306` has for a parameter's name.

**Why.** The manual's findings (R17), "Diagnostics without fixes".

**In stage 1.** `typeck.collect` (`import_target`, the impl's check).

**What it changes.** The cases `cases/findings10/similar_names.sisu` and
`cases/packages/similar_module.pkg` (their `.diag`).

- Differs: `compiler/stage1/cases/findings10/similar_names.sisu`: tree, ast, ids, human, fmt-check, doc, doc-output
- Differs: `compiler/stage1/cases/packages/similar_module.pkg`: ids, human, doc, doc-output

**Decision 602**, for docs/architecture.md:

602. `E0203` offers the path with a module of a similar name, and `E0271`
     the trait's constant of a similar name. Stage 1 decides it (RULES.md,
     R97).

## R98. `E0310` of two sized integers that an `int` both holds offers `.to_int()` (decision 603)

**The rule.** An arithmetic or bitwise operator between two sized integer
types neither of which widens to the other, but both to `int` (`i16` and
`u32`), is `E0310` as before, with the fix `type.convert-to-int` ("make the
`i16` an `int`, which holds the values of both") that writes `.to_int()`
after the left operand (parenthesised when it is not a name, a call or a
bracket already), beside which the right one widens (03, "Numbers").

**Why.** The manual's finding (chapter 24): "E0310 for an `i16` plus a
`u32` offers no conversion."

**In stage 1.** `typeck.expr` (`binary_result`, `method_edits`).

**What it changes.** The case `cases/findings10/mixed_sizes.sisu` (its
`.diag`).

- Differs: `compiler/stage1/cases/findings10/mixed_sizes.sisu`: ids, human, doc, doc-output

**Decision 603**, for docs/architecture.md:

603. `E0310` of two sized integers that an `int` both holds offers making
     the left one an `int`. Stage 1 decides it (RULES.md, R98).

## R99. `E0812` gives the copy that can change (decision 604)

**The rule.** `E0812` (a change of a refined value through its base) has
the note `refine.copy-as-base`: "a copy of `picks` is a `Picks` too: change
a copy typed as the base, `var copy: list[int] = picks.copy()`, and check
it again with `Picks.try_from(copy)`".

**Why.** The manual's finding (R17): "E0812 says to "change a copy of the
base", and the `.copy()` of a refined list is still the refinement, so the
copy is E0812 again". The message is the catalog's (the owner's); the note
says what the copy must be.

**In stage 1.** `typeck.contracts.check_protected_place`.

**What it changes.** The case `cases/findings10/refined_copy.sisu`, and the
human form of the conformance suite's `contracts/refinement_errors`.

- Differs: `compiler/stage1/cases/findings10/refined_copy.sisu`: human, doc, doc-output
- Differs: `compiler/conformance/cases/contracts/refinement_errors.sisu`: human, doc, doc-output

**Decision 604**, for docs/architecture.md:

604. `E0812` notes that the copy to change is one typed as the base. Stage
     1 decides it (RULES.md, R99).

## R100. `E0005` names a character that shows as a space or as nothing (decision 605)

**The rule.** `E0005` for a character that shows as a space (a no-break
space, the spaces of other widths, the line and paragraph separators) or as
nothing (a zero-width space or joiner, a soft hyphen, a word joiner, the
marks of the text's direction, a byte-order mark) writes it as its escape,
`` `\u{a0}` cannot appear ...``, with the note `lex.invisible-character`
naming it ("it is a no-break space, which looks like a space") and the fix
that writes a space (`lex.write-space`) or removes it
(`lex.remove-character`). A byte-order mark at the start of a file is named
as one ("a byte-order mark, which UTF-8 text does not need"); it is still
`E0005`, since 02's source is UTF-8 text and the token stream of the file
stays the Rust lexer's.

**Why.** The manual's finding (R1): "E0005 shows an invisible character as
it is, so the message for a no-break space reads "`` ` ` `` cannot
appear", and a byte-order mark at the start of a file is E0005".

**In stage 1.** `lexer.scan` (`invisible`).

**What it changes.** The cases `cases/findings10/invisible.sisu` and
`byte_order_mark.sisu` (their `.diag`).

- Differs: `compiler/stage1/cases/findings10/invisible.sisu`: tokens, tree, ids, human, fmt-check, doc, doc-output
- Differs: `compiler/stage1/cases/findings10/byte_order_mark.sisu`: tokens, tree, ids, human, fmt-check, doc, doc-output

**Decision 605**, for docs/architecture.md:

605. `E0005` writes a character that shows as a space or as nothing as its
     escape, names it, and offers a space or its removal. Stage 1 decides
     it (RULES.md, R100).

## R101. The fixes and notes the manual's writers found missing, the tenth round (decision 606)

**The rule** (the manual's "Diagnostics without fixes"):

- A value given to a `ref dyn Trait` parameter it implements, `total(b)`,
  is `E0300` with the fix `type.lend-it` that writes `ref ` (`mut ` for a
  `mut dyn`) before it.
- A reference to a `Copy` value of a refinement's base, a loop's `ref int`
  where a `Percent` goes, is `E0811` with its note about `try_from`, as the
  `int` is, where it was `E0300`.
- An integer literal beyond `int` (`E0374`) has the note
  `type.bigint-literal`, which names `BigInt.parse` and R78's literal
  beside a `BigInt`.
- `E0310` with a number on the left of a type of the program's own, `3.0 *
  step`, has the note `type.operator-trait-left`, which names the impl the
  operator needs (`impl Mul[Step] for f64`, the root package's alone) and
  the order that needs none.
- R70's fix that derives a trait for a type argument (`Plain` of
  `Pair[Plain]`) says which type: `type.derive-trait-for`, "derive it for
  `Plain`".
- A type nothing declares, or that a failed import would have bound, called
  with type arguments, `Deque[int]()`, is one `E0200`, its arguments not
  read as values (where `int` got a second `E0200` and the note about
  Python's conversions), in the name pass and the type checker alike.
- Python's `f"..."` after `=`, `let s = f"n is {n}"`, gets R72's fix, which
  only a call's argument had; a generator expression as a call's only
  argument without its own parentheses, `sum(p * p for p in primes)`, gets
  the note `parse.generator-parens` and the fix that adds them; `case not
  0:` gets the note `parse.pattern-not` (a pattern says what a value is;
  `case _:` or a guard), where R72's note said that `not` names no binding.
- `def MAX_HEAT(self)`, a method named in capitals, is `E0268` with the
  note `resolve.const-as-method` (a trait's constant is `const MAX_HEAT: T =
  ...`), where its fix renamed it `max_heat` and left the constant missing.
- `E0266` for a `@cfg` on the target, `@cfg(target.os == "linux")`, has the
  note `resolve.cfg-target`, which names `if comptime target.os ==
  "linux":`.
- The fixes that named their replacement besides showing it no longer do
  (R67's rule, for the rest the manual lists): `type.add-copy` reads "copy
  it", `type.convert-to-float` "convert the `int`",
  `parse.visibility-package` "make it visible to its package",
  `resolve.add-ref-dyn` "use the trait through a reference" and
  `borrow.declare-view` "declare it a view".

**Why.** The manual's findings (chapters 2, 12, 14, 15, 16, 17, 23; R2, R3,
R14, R17, R19, G2, G3), "Diagnostics without fixes".

**In stage 1.** `typeck.expr` (`mismatch_fix`, the refinement's coercion,
the integer literal, `binary_result`), `typeck.check.derive_fix`,
`typeck.call` (a call of an unknown name with type arguments), `resolve`
(`unknown_base`, `check_style`, `screaming`), `parser.habits` (`f_string`,
the generator, `case not`), `main.built_tree`, the catalog's texts.

**What it changes.** The cases `cases/findings10/fixes.sisu`,
`f_string_let.sisu`, `generator_argument.sisu`, `case_not.sisu`,
`constant_as_method.sisu` and `cfg_target.sisu` (their `.diag`); the human
form of every diagnostic with one of the five fixes, the same in both
front ends (they read the one catalog), and the conformance suite's
`ownership/rustc/move_out_of_vec_tail.defer` and
`overloaded_index_move.defer`, blessed again with `type.add-copy`'s text;
and stage 1's `cases/findings7/derive_messages.diag`
(`type.derive-trait-for`) and `cases/traits/assoc_const_refusals.diag` (no
rename of `def SIDES`), blessed again.

- Differs: `compiler/stage1/cases/findings10/fixes.sisu`: names, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/findings10/f_string_let.sisu`: tree, ids, human, fmt-check, doc, doc-output
- Differs: `compiler/stage1/cases/findings10/generator_argument.sisu`: tree, ids, human, fmt-check, doc, doc-output
- Differs: `compiler/stage1/cases/findings10/case_not.sisu`: human, fmt-check, doc, doc-output
- Differs: `compiler/stage1/cases/findings10/constant_as_method.sisu`: tree, ast, names, ids, human, fmt-check, doc, doc-output
- Differs: `compiler/stage1/cases/findings10/cfg_target.sisu`: human, doc, doc-output

**Documents to bring along:** the manual's transcripts that show the five
fixes (`10-ownership/first_in_line`, `12-views/program`,
`14-traits/not_a_type`, the reference's `e0338_move_out_of_borrow`,
`e0373_mixed_numbers`, `e0130_pub_crate`, `e0231_trait_as_type`,
`e0411_view_field` and the false friends' `pub_crate`).

**Decision 606**, for docs/architecture.md:

606. Stage 1 gives the fixes and notes the manual found missing: `ref`
     before a value lent to a `ref dyn` parameter, `E0811` for a `ref int`
     where a refinement goes, `BigInt` for a literal beyond `int`, the impl
     a number on an operator's left needs, the type a derive's fix is for,
     one `E0200` for `Deque[int]()`, the spellings of `f"..."` after `=`
     and of a generator as a call's only argument, `case not`, a method
     named in capitals, `@cfg` on the target; and five fixes no longer
     name their replacement. Stage 1 decides it (RULES.md, R101).

## R102. An iterator's adapter is not `pure`, whatever the function it is given (decision 607)

**The rule.** Purity stays a property of a function's type (03, "Function
types"; Q-17): `Iterator.map` takes `(T) -> U`, any function, and a
function that takes a plain function type is impure, "since the checker
cannot see what it will call", so `xs.iter().map(f).sum()` in a `pure`
function is `E0333` at `map` also when `f` is `pure`. Purity is not
computed over the monomorphised call graph: 03 and 06 keep that graph for
the resource attributes and closure capture kinds, which "never enter
function types", so that `pure` stays the one property a function type
carries. What changes is the diagnostic: `E0333` at an adapter of
`Iterator` (`map`, `filter`, `filter_map`, `flat_map`, `take_while`,
`skip_while`, `inspect`, `map_while`, `scan`) has the note
`type.pure-adapter`, which says why and names `fold` with a `pure`
function, which `Iter[T]` has (decision 564), and the loop.

**Why.** The stage 0 round's item (decision 564: "a `pure` chain through
`map` needs purity over the monomorphised call graph ... stage 1's, if
wanted"). The documents decide against it: inferring a call's purity from
its arguments is an effect polymorphism, which Q-17 rules out, and a `pure`
adapter in std (`map` taking `pure (T) -> U`) would refuse the impure
functions every other caller gives it.

**In stage 1.** `borrowck.purity` (`is_adapter`).

**What it changes.** The case `cases/findings10/pure_adapter.sisu` (its
`.diag`; the note is the human form's).

- Differs: `compiler/stage1/cases/findings10/pure_adapter.sisu`: human

**Decision 607**, for docs/architecture.md:

607. Purity is not inferred over the monomorphised call graph: an
     iterator's adapter takes any function and is not `pure`, and `E0333`
     at one notes `fold` with a `pure` function. Stage 1 decides it
     (RULES.md, R102).

## R103. `sisu lint` leaves a refused import to its error; a worksheet shows an option or a result (decision 608)

**The rule.** `sisu lint` does not report `W0027` for an import the
resolver refused (`E0204` for a name the module lacks, `E0203` or `E0220`
for a path): the name refused counts as used, and so does an import whose
path was refused, so each is reported once, by its error; the other
unused names of the import are still `W0027`. And a worksheet's top-level
expression statement whose value is an option or a result is shown
(`Some(1)`, `Err(Negative)`), where it was `E0318` in the worksheet as
written: the worksheet uses the value by showing it.

**Why.** The manual round's items: `sisu lint` printed `E0204` and then
`W0027` for the same name, and a worksheet showed an option only through a
`let`, as the Rust front end's worksheet did too.

**In stage 1.** `fmt.prune` (`unused`, `lints`, `overlaps`), `main`
(`lint`, `worksheet_command`), `session.shown_statements`.

**What it changes.** The cases `cases/findings10/refused_import.sisu`
(its `.lint`; its `fmt` differs as R87's cases do, the Rust formatter
removing the unused import) and `cases/repl/options.ws.sisu`, which the
Rust front end's worksheet refuses.

- Differs: `compiler/stage1/cases/findings10/refused_import.sisu`: fmt, fmt-check
- Differs: `compiler/stage1/cases/repl/options.ws.sisu`: ws

**Decision 608**, for docs/architecture.md:

608. `sisu lint` reports an import the resolver refused by its error
     alone, and a worksheet shows an expression statement's option or
     result. Stage 1 decides it (RULES.md, R103).

## R104. The fixes and notes the manual's writers found missing, the eleventh round (decision 660)

**The rule** (the manual's "Diagnostics without fixes", the tenth round's
open items, each checked against the manual's own program first):

- `E0338` for an item of a list the function owns, `let first = queue[0]`
  with `queue` a local `list[String]`, has the note
  `type.item-stays-in-list`: the list keeps the item and only lends it, so
  it is copied or taken out (the fix `type.take-out` of R45 where the list
  is a `var`). The message is the catalog's, shared with the case the
  words fit, a move out of a reference.
- `E0421` for a `Lent[Fs]` kept in a field has the note
  `borrow.lent-capability`, which names the capability itself or the work
  done inside the call, where the note about text (`.to_string()`) stood.
- A capability where a whole `System` is taken, a test's `TestClock`, is
  `E0300` with the note `type.narrow-capability`: the parameter narrowed
  to `Clock` (`clock: Clock`), and `sys.clock` passed where a `System` is
  at hand.
- `E0806` names the type's methods that take `mut self`
  (`contract.invariant-methods`), or, for a type with none, the method to
  write (`contract.invariant-method`).
- A script or a package whose manifest names a dependency by version is
  `E1010` alone: the `E0220` of its import, which names a `sisu.toml` a
  script does not have, is left out.
- An impl without an associated type the trait declares is `E0237` alone:
  the type is the error's from there on, so a use of it is not `E0347`
  again.
- `E0301` for `/` between two `int`s offers, besides `//`, the float
  division 03 and chapter 2 spell: the fix `type.float-division`, which
  makes both sides `f64` (`.to_f64()`, or `.0` after a literal).
- `E0402` at a binding the loop makes anew (its variable, `for turn in
  turns:`), while a borrow of the one before is still used (`who =
  turn.name`), has the note `borrow.bound-anew`, which names the copy that
  owns its value, where R77's note said to change the variable after the
  loop.
- `E0258` for a struct built from another module names an associated
  function of the type that gives one and that module may call
  (`resolve.made-by`: `Point.on_meridian(lat)`, `new` first).
- `E0204` for a variant imported by name, `from ledger import Taken`, has
  the note `resolve.variant-of-type`: a variant is reached through its
  type, `BookingError.Taken`.
- A dependency that declares its unsafe level and that `[trust]` leaves
  out is `E1021` alone, which names the fix: the `E0703`s at its `extern`
  and `unsafe` blocks are left out.
- `E0310` of a compound assignment names the operator written, `+=`, in
  code `comptime for` unrolls and elsewhere.

**Decided against:** `E0501` (`sir.invalid`) in `sisu check`: it is the
SIR verifier's, an internal error of lowering that stage 0's driver
reports when it reads SIR, which no source position names; a program that
reaches it is a compiler bug, reported as one. `E0333`'s `D0021` line is
stage 0's driver's.

**Why.** The manual's findings (chapters 10, 18 and 23; R17, G2, G19, G26,
G29, G32, X7, X8), "Diagnostics without fixes".

**In stage 1.** `typeck.expr` (`check_movable`, `mismatch_fix`,
`float_operand_edits`), `typeck.tasks.check_owners`,
`typeck.contracts.invariant_field`, `main` (`without_unreachable`, the
`[trust]` order in `layout_input`), `typeck.collect` (`check_traits`,
`variant_owner`), `typeck.check.check_assign`,
`borrowck.loans.conflict`, `typeck.call.maker_of`; the catalog's texts.

**What it changes.** The cases `cases/findings11/item_in_list.sisu`,
`lent_capability.sisu`, `narrow_clock.sisu`, `invariant_methods.sisu`,
`script_registry.sisu`, `assoc_cascade.sisu`, `float_division.sisu`,
`bound_anew.sisu` and `compound_operator.sisu`, and
`cases/packages/variant_import.pkg` and `trust_order.pkg`, each with its
`.diag` (the notes are the human form's; `variant_import`'s `fmt` differs
as R87's cases do, the Rust formatter removing the unused import); the
human form of the conformance suite's cases the notes reach. Blessed
again: the conformance suite's `types/integer_division.diag`,
`run/stopped_by_errors.diag` and `.err` (the float division's fix), and
stage 1's `cases/run/build_stopped.release` (the same) and
`cases/packages/unsafe_levels.diag` and `.audit` (no `E0703` beside
`cbind`'s `E1021`).

**Documents to bring along:** the manual's transcripts that show the
diagnostics this rule changes (`10-ownership/first_in_line`,
`18-capabilities/keep_lent`, `solutions/18-capabilities/days_left`,
`23-contracts/outside`, `guide-script/registry`,
`guide-c-library/untrusted`, `explanation-compile-time/total`,
`02-values/average`, the reference's `e0338_move_out_of_borrow`,
`e0301_division`, `e0806_invariant_field` and `e0258_private_field`, and
the false friends' `int_division`).

- Differs: `compiler/conformance/cases/capabilities/lent_rejected.sisu`: human, doc, doc-output
- Differs: `compiler/conformance/cases/ownership/rustc/overloaded_index_move.sisu`: human, doc, doc-output
- Differs: `compiler/conformance/cases/wrong/move_out_of_index.sisu`: human, doc, doc-output
- Differs: `compiler/conformance/cases/run/stopped_by_errors.sisu`: types, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/types/integer_division.sisu`: types, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/findings11/item_in_list.sisu`: human, doc, doc-output
- Differs: `compiler/stage1/cases/findings11/lent_capability.sisu`: human, doc, doc-output
- Differs: `compiler/stage1/cases/findings11/narrow_clock.sisu`: human
- Differs: `compiler/stage1/cases/findings11/invariant_methods.sisu`: human, doc, doc-output
- Differs: `compiler/stage1/cases/findings11/script_registry.sisu`: names, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/findings11/assoc_cascade.sisu`: ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/findings11/float_division.sisu`: ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/findings11/bound_anew.sisu`: human
- Differs: `compiler/stage1/cases/findings11/compound_operator.sisu`: ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/packages/variant_import.pkg`: human, doc, doc-output
- Differs: `compiler/stage1/cases/packages/variant_import.pkg/src/main.sisu`: fmt, fmt-check
- Differs: `compiler/stage1/cases/packages/trust_order.pkg`: names, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/packages/trust_order.pkg/cenv/src/lib.sisu`: ast
- Differs: `compiler/stage1/cases/run/build_stopped.sisu`: ids, human, doc, doc-output

**Decision 660**, for docs/architecture.md:

660. Stage 1 gives the fixes and notes the manual found missing in its
     eleventh round: a local list's item for `E0338`, a lent capability
     for `E0421`, the parameter narrowed to the capability a test gives,
     the methods `E0806` points to, `E1010` without `E0220`, `E0237`
     without its `E0347`s, the float division for `E0301`, a binding a
     loop makes anew for `E0402`, the associated function for `E0258`,
     a variant reached through its type for `E0204`, `E1021` without the
     `E0703`s, and `+=` named by `E0310`. Stage 1 decides it (RULES.md,
     R104).

## R105. An impl is the package's own when a type argument of its trait is (decision 661)

**The rule.** An impl of a trait of another package for a type of
another package, outside the root package, is coherent when one of the
trait's type arguments is a type of the package's own, and no type
parameter of the impl stands bare before it (the type implementing the
trait first, then the trait's arguments in order): `impl Mul[Vector] for
f64` in the package that declares `Vector`, which R43 lets `2.0 * v`
reach, so that a library gives the operator to its users. Any other is
`E0236` as before, with the note `resolve.orphan-local-type`, which says
which type would make the impl the package's. R101's note for `3.0 *
step` now names the package that declares the type on the right.

**Why.** Chapter 14's finding ("A library cannot give `2.0 * v` to its
users"). 03's coherence asks for at most one impl of a trait for a type,
globally, and gives the orphan rule for it; the first-local-type form is
Rust's own and keeps it: no other package can write `Mul[Vector] for
f64`, since only `Vector`'s package has `Vector`, and a bare parameter
before the local type (`impl[T] Mul[Vector] for T`) is refused, since
another package's impl of `Mul[Vector]` for its own type would overlap
it.

**In stage 1.** `typeck.collect.check_traits`, `local_argument`.

**What it changes.** The cases `cases/packages/coherence_operator.pkg`
(its `.out`, which the Rust front end refuses) and
`coherence_refused.pkg` (its `.diag`).

- Differs: `compiler/conformance/cases/packages/orphan.pkg/app`: human, doc, doc-output
- Differs: `compiler/stage1/cases/packages/coherence_operator.pkg`: types, sir, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/packages/coherence_refused.pkg`: human, doc, doc-output

**Decision 661**, for docs/architecture.md:

661. An impl of a foreign trait for a foreign type is the package's own
     when one of the trait's type arguments is a type of the package and
     no type parameter of the impl stands bare before it, `impl
     Mul[Vector] for f64` beside `Vector`; any other is `E0236`, whose
     note says which type would make it the package's. Stage 1 decides
     it (RULES.md, R105).

## R106. An import may test the target; an import in a body is `E0277` (decision 662)

**The rule.** 09 gives a dependency that only some targets build (`when
= { os = "windows" }`) and says nothing of how code that builds on every
target reaches it: `@cfg(...)` on the target was `E0266`, an import stands
outside `if comptime`, and an import of the excluded dependency is
`E0275`. Now:

- An import's `@cfg` may test a fact of the target, `@cfg(target.os ==
  "windows")` (`==` or `!=` on `os`, `arch` or `runtime`, under `not`,
  `all` and `any` as the other conditions). Where it holds, the import is
  as written; where it does not, it is left out of the build, as any item
  whose `@cfg` does not hold, and binds nothing (so no `E0275`). On any
  other item a condition on the target is `E0266` still, with R101's note
  that a body tests the target with `if comptime`.
- A name such an import would have bound is not reported inside an `if
  comptime` branch on the target that the build leaves out (the branch
  of a condition `target.FACT == "VALUE"` that does not hold, and those
  after one that does), where nothing could check it; anywhere else it is
  `E0200` with the note `resolve.cfg-import-absent`, which names the
  `@cfg` and the `if comptime` that keeps the use where the import is.
  08's rule that both branches type-check holds for everything else in
  them. Stage 1 checked a branch the build leaves out like any other,
  before resolution; this is the one exception, for the names the build
  does not have.
- An import inside a function's body, `E0900` ("a nested declaration")
  with an `E0200` at each use, is `E0277` (`resolve.import-in-body`):
  imports stand at the top level, and their names are the module's, with
  the fix `resolve.move-import` that moves it before the first line that
  is not a comment; the names it binds are not reported again.

**Why.** The manual's findings (R13, R17 under "Tooling and the driver";
R14's `E0266`; "An import inside a function is E0900").

**In stage 1.** `cfg` (`Filter.import_`, `target_condition`,
`import_names`, `discarded_branches`), `main` (`target_absences`,
`with_absences`, `NamedModule.absent` and `discarded`),
`typeck.collect` (the absent names are poisoned), `typeck.check`
(`move_import_edits`), `resolve.statement`.

**What it changes.** The cases `cases/packages/conditional_import.pkg`
(its `.out`), `conditional_import_use.pkg` and
`cases/findings11/import_in_body.sisu` (their `.diag`).

- Differs: `compiler/stage1/cases/packages/conditional_import.pkg`: sir, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/packages/conditional_import_use.pkg`: names, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/findings11/import_in_body.sisu`: names, ids, human, doc, doc-output

**Decision 662**, for docs/architecture.md:

662. An import's `@cfg` may test the target, `@cfg(target.os ==
     "windows")`, and binds its names only where the condition holds; a
     use of one is not reported in an `if comptime` branch on the target
     the build leaves out, and is `E0200` naming the `@cfg` elsewhere.
     An import in a function's body is `E0277`, with the fix that moves
     it to the top of the module. Stage 1 decides it (RULES.md, R106).

## R107. `arbitrary[T]` of a type parameter draws through its bound (decision 663)

**The rule.** `arbitrary[T]`, `T` a type parameter of the function bounded
by `Arbitrary`, is `Gen.of(d -> T.arbitrary(d))`, the generator the bound
gives, where it was `E0820`. Without the bound it is `E0820` still, whose
reason now names the bound (`a type parameter has the generator of its
bound, `T: Arbitrary``), where the message said twice that `T` has no
generator. `exhaustive[T]` of a type parameter is `E0820` as before:
`Arbitrary` gives random cases, not every one.

**Why.** G11's finding under "Testing" (17: a type with an `Arbitrary`
impl is drawn through it).

**In stage 1.** `typeck.arbitrary` (`write_bound_call`,
`Program.generator_bounds`), `typeck.contracts.generator_site`
(`bounded_by_arbitrary`, `is_type_param`).

**What it changes.** The cases `cases/findings11/arbitrary_bound.sisu`
(its `.out`; its test passes) and `arbitrary_unbounded.sisu` (its
`.diag`).

- Differs: `compiler/stage1/cases/findings11/arbitrary_bound.sisu`: ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/findings11/arbitrary_unbounded.sisu`: ids, human, doc, doc-output

**Decision 663**, for docs/architecture.md:

663. `arbitrary[T]` of a type parameter bounded by `Arbitrary` is the
     generator its bound gives; without the bound `E0820` names it.
     Stage 1 decides it (RULES.md, R107).

## R108. Types and traits: `F[_]`, a float's total order, the methods of 03, `@repr(c)` for `Plain` (decision 664)

**The rule.**

- `F[_]` in a generic parameter list is `E0100` with the note
  `parse.higher-kinded`: 03 reserves the syntax for higher-kinded types,
  which the first version does not have; the bracket is passed over, so
  the declaration is read on.
- The standard library has, of the methods the manual found missing beside
  03: `int.checked_add`, `checked_sub` and `checked_mul`, which give `None`
  on overflow; `f64.round_to[T]()`, the nearest value of an integer type
  (halves away from zero, made a `T` as `truncate_to` makes it);
  `f64.total_order()`, a `TotalF64` that is `Eq` and `Ord` (the numbers in
  order, `-0.0` before `0.0`, NaN after every number); and `max_by` and
  `min_by` over a `PartialOrd` key, which panic on two keys that are not
  ordered, as 03 asks. `E0347` for an `f64` that is not `Ord` has the note
  `type.total-order`, which names `total_order()`, where its fix wrote an
  `impl Ord for f64` that only the root package may write.
- `@derive(Plain)` of a struct of several fields asks for `@repr(c)`
  itself: an attribute named `repr` with another argument,
  `@repr(python)`, is refused as one without it.
- `@repr(u8)` on an enum without discriminants is kept: it names the
  tag's layout, which nothing a safe program does can observe until the
  layouts are lowered (R78 refuses the discriminants that would).

**Decided against, or left:** a padded `@repr(c)` struct under
`@derive(Plain)` (22 allows no padding): the derive's template has no
compile-time size of a field until the layouts are lowered, so it is
left open; `ptrsize`, `Sized`, `Decimal`'s `/`, `graphemes`,
`@derive(Into, From)`, the `Array` literal and `FromLiteral` for `Url`,
which are the runtime's, the design's or wait for the owner. A method
that takes `own self` and returns `Self` (`Gauge(level=4).filled(20)`) is
no exit that checks the invariant (R11's finding; 07 checks construction
and `mut self` methods): a question for the owner, with the lean that a
method of the type that returns `Self` checks the invariant on what it
returns, as construction does.

**Why.** The manual's "Types and traits" (R2, chapters 2, 14 and 27, R14).

**In stage 1.** `parser.items.generic_params`, `typeck.check.missing_impl_fix`;
`std/prelude.sisu` (each addition with its doc example), `std/mem.sisu`'s
derive of `Plain`.

**What it changes.** The cases `cases/findings11/higher_kinded.sisu`,
`float_order.sisu` and `plain_repr.sisu` (their `.diag`) and
`std_methods.sisu` (its `.out`); `cases/unsafe/plain_refusals.diag` is
blessed again, the derive's lines in `std/mem.sisu` one further down,
and so is the conformance suite's `std/doc_examples.select`, which runs
the seven new doc examples.

- Differs: `compiler/stage1/cases/findings11/higher_kinded.sisu`: tree, trivia-tree, ast, ids, human, fmt-check, doc, doc-output
- Differs: `compiler/stage1/cases/findings11/float_order.sisu`: ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/findings11/plain_repr.sisu`: names, ids, human, doc, doc-output

**Decision 664**, for docs/architecture.md:

664. `F[_]` is `E0100` with a note that 03 reserves it; the standard
     library has checked arithmetic as methods, `round_to`,
     `f64.total_order()` and `max_by` and `min_by` over a partially
     ordered key, which `E0347` for a float names; `@derive(Plain)` asks
     for `@repr(c)` itself; `@repr(u8)` without discriminants is kept.
     Stage 1 decides it (RULES.md, R108).

## R109. `E0235` at a derive labels the impl written beside it (decision 665)

**The rule.** `E0235` for an impl written beside `@derive(Eq)`, which
stands at the derive's `Eq`, labels the written impl (`resolve.first-impl`),
as `E0235` for two written impls labels the other one: a label of a
derived impl's diagnostic that points into a file of the program stays
when the diagnostic moves to the derive (R70 dropped every label).

**Left open:** a label at the line of the template that `comptime for`
unrolled (X7; the instance's text is not mapped back to its template's
lines); `@preview("...")`, which compiles without a word (the lean: a use
of a preview item warns as a deprecated one does, R30, with its text);
statements before a derive's `return impl` (E0607), which 08 neither
gives nor rules out.

**Why.** The manual's "Metaprogramming" (R14, R17).

**In stage 1.** `typeck.placed_at_derive`.

**What it changes.** The case `cases/findings11/derive_overlap.sisu` (its
`.diag`).

- Differs: `compiler/stage1/cases/findings11/derive_overlap.sisu`: ids, human, doc, doc-output

**Decision 665**, for docs/architecture.md:

665. `E0235` at a derive labels the impl written beside it. Stage 1
     decides it (RULES.md, R109).

## R110. A std template that asks `reflect` of its type parameters; `json.decode`'s expected type without an article (decision 666)

**The rule.** A generic method of the prelude whose body asks `reflect`
of its type parameters, `reflect.name_of(T)`, is unfolded for its
instances as a template of the program's own is, where stage 1 stopped
with an index out of bounds: the base keeps no std module's syntax, so
the instance reads the module's text parsed again (the prelude's text is
kept with the importable modules' for it), which gives the nodes the
base's parse gave. And `json.decode`'s `Mismatch` writes the expected type
without an article, ``expected="`Hour`"``, as R68 took the article out of
the compiler's messages.

**Why.** The stage 0 round's items: its `truncate_to` to the 128-bit
types, at the prelude's end, asks for the name of its type parameter;
and the manual's finding under "The standard library" (G7).

**In stage 1.** `typeck.instances` (`std_syntax`, `named_types` and
`value_types`, which pass by a body whose syntax is missing),
`typeck.collect` (the prelude's text in `std_texts`); `std/json.sisu`.

**What it changes.** `cases/json/decode_validates.out`, blessed again; no
case of stage 1's reaches the template yet (the prelude has no generic
method that asks `reflect`), which stage 0's `truncate_to` will be.

**Decision 666**, for docs/architecture.md:

666. A generic method of the standard library may ask `reflect` of its
     type parameters; `json.decode`'s `Mismatch` names the expected type
     without an article. Stage 1 decides it (RULES.md, R110).

## R111. The merge's differences: the prelude's constants are named as its methods are (decision 700)

**The rule.** An associated constant of a type the prelude declares is
named in the SIR by the type, `u8.MAX` and `i128.MIN`, as the type's
methods are (`u8.truncate_to`) and as stage 0 names it, where stage 1 wrote
`prelude.u8.MAX`: while the base is made the prelude is the root module,
and the name of a root item that the prelude also declares was qualified
by its module, which is right only for a program's own item that hides
the prelude's.

**The merge's other difference** is R87's: `std/instant_monotonic`
imports `std.cap` and uses only the names `from std.cap import` binds, an
import the Rust formatter removes and stage 1's keeps (R87), so its four
formatter classes differ as that rule lists (the deformed copies of a
file follow its `fmt-check`).

**Why.** The merge of stage 0's tenth round (decision 642: `MIN`, `MAX`
and `truncate_to` in the prelude's last block; decision 643's case).

**In stage 1.** `typeck.program.qualified_name`.

**What it changes.** The SIR of every program that reads a sized type's
`MIN` or `MAX`, which stage 0's is again: `std/int_bounds` matches.

- Differs: `compiler/conformance/cases/std/instant_monotonic.sisu`: fmt, fmt-check

**Decision 700**, for docs/architecture.md:

700. A prelude type's associated constant is named in the SIR by the
     type, `u8.MAX`, as its methods are. Stage 1 decides it (RULES.md,
     R111).

## R112. `sir --repl-serve`: the REPL's commands served by one process that keeps the base (decision 701)

**The rule.** `sisu1 sir --repl-serve` (with `--home DIR` and `--catalog
PATH` as every command takes them) makes the standard library's base once
and then serves requests, so that an input costs its own check and
lowering and not the base (decision 641). The protocol, which the driver
may rely on from 0.10.0:

- A **request** is one line of standard input: the arguments of one
  command, separated by tab characters (U+0009), exactly as they would
  follow `sisu1` on a command line (without `--home` or `--catalog`):
  `sir`, `--repl`, `SESSION`, `INPUT`, `--kept`, `OUT`, and `--load`,
  `--dir DIR`, `--name NAME` as R79 and R95 give them; or `sir`,
  `--worksheet`, `FILE`, `--dir DIR`, `--name NAME`; or `parse`,
  `--bindings`, `INPUT`. An argument holds no tab and no line break. An
  empty line, or the end of standard input, ends the server, status 0.
- An **answer** is written to **standard error**, one for each request in
  order: a line `answer STATUS OUT ERR`, where `STATUS` is the exit status
  the command gives when run alone (0 or 1) and `OUT` and `ERR` are
  lengths in bytes, then exactly `OUT` bytes, what the command alone
  prints on standard output (the SIR, or `parse --bindings`' lines), then
  exactly `ERR` bytes, what it prints on standard error: its diagnostics,
  ending with the line the run time prints when it fails, `error: Failed`
  (`error: Usage(...)` for arguments it does not take). An I/O error is
  `answer 1 0 N` with `error: ` and the error's `Debug` form, without the
  run time's line of the source. The files SESSION, INPUT, FILE and OUT are
  read and written as the command alone reads and writes them, so `OUT`
  holds the kept session when `STATUS` is 0.
- Standard output carries nothing. The answers go to standard error
  because the native run time writes it at once, where it keeps standard
  output until 64 KB are waiting or the program ends (`rt::stdout`); a run
  time that wrote standard output before a read of standard input would
  let a later version answer there.

The answers are byte for byte what the commands print alone (the
session of `cases/repl/session.repl`, its twelve inputs three times, gave
the same 36 answers both ways). `sisu1 --version` is 0.10.0: the driver
may rely on `sir --repl-serve` and on R116's `--target-runtime`.

**Why.** Decision 641 (stage 0's report): `sir --repl` costs 0.41 s per
input, 90 % of it `base_of`. Measured here on a machine with a load of 11
on four cores: 0.83 to 0.91 s per input as one process each (the session
above, natively), 0.041 s per input served, the first answer 1.46 s with
the base.

**In stage 1.** `main` (`repl_serve`, `Answer`, `Ending`, `answered`,
`repl_answer`, `worksheet_answer`, `bindings_answer`; `held_build` takes the
base).

**What it changes.** No output of the corpus; `main`'s unit test of the
frame.

**Decision 701**, for docs/architecture.md:

701. `sisu1 sir --repl-serve` serves `sir --repl`, `sir --worksheet` and
     `parse --bindings` from one process that makes the standard
     library's base once: each line of standard input is a request, the
     command's arguments separated by tabs, and each answer, on standard
     error, is `answer STATUS OUT ERR` and then the bytes the command alone
     prints on standard output and on standard error; an empty line or the
     end of the input ends it. An input costs 0.04 s served where it cost
     0.8 s as a process. `sisu1 --version` is 0.10.0. Stage 1 decides it
     (RULES.md, R112).

## R113. A `String` key is looked up by a `str`; an optional reference compared with an optional value (decision 702)

**The rule.**

- The prelude has `trait Lookup[K]` with `lookup_key(self) -> K`, and
  `impl Lookup[String] for str`. Where a lookup takes its key, the key
  parameter of `get`, `contains` and `remove` of a `dict` and of
  `contains` and `remove` of a `set`, and a `dict`'s index, a value whose
  type is not the key's but implements `Lookup[K]` is accepted, and the
  lookup borrows the key it gives: `recipes.get(dish)`, `prices[dish]`
  and `seen.remove(word)` with a `str` and `String` keys. `in` compared
  the text already. The key a `str` gives is a `String` made for the
  lookup and dropped after it, the allocation the `.to_string()` the user
  wrote made; the lookup without it is the runtime's (`in` and
  `dict.contains` already compare a `str` with the `String` keys there).
- `E0300` for an optional value compared with an optional reference to a
  `Copy` value, `prices.get("bun") == Some(280)`, has the fix
  `type.copy-optional`, `.copied()` after the left operand, in
  parentheses when it is not a name or a call chain.

**Why.** The manual's findings under "The standard library" (chapters
10, 19 and 20, G2). 03 and 11 say nothing of a key's type beyond `Eq` and
`Hash`; a lookup trait of the library that the checker consults, rather
than an implicit conversion everywhere a `String` is expected, keeps
D-016's "no implicit lossy conversions" whole, since the conversion is the
library's and named.

**In stage 1.** `typeck.call` (`is_lookup_key`, `lookup_arg`),
`typeck.expr` (`key_of`, `copied_fix`, the index of a `dict`);
`std/prelude.sisu` (`Lookup`, with its doc example).

**What it changes.** The cases `cases/findings12/str_lookup.sisu` (its
`.out`) and `copied_compare.sisu` (its `.diag`); `cases/habits/fixes.diag`
is blessed again (its `recipes.get(dish)` with a `str` is no longer
`E0300`), and so is the conformance suite's `std/doc_examples.select`,
which runs `Lookup`'s doc example.

- Differs: `compiler/stage1/cases/findings12/str_lookup.sisu`: ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/findings12/copied_compare.sisu`: ids, human, doc, doc-output

**Decision 702**, for docs/architecture.md:

702. A lookup in a `dict` or a `set` (`get`, `contains`, `remove`, an
     index) takes a value of a type that implements the prelude's
     `Lookup[K]`, a `str` for `String` keys, and borrows the key it gives;
     `E0300` for an optional reference compared with an optional value has
     the fix `.copied()`. Stage 1 decides it (RULES.md, R113).

## R114. The ownership checker's findings: `E0343` for a change through a `Shared` (decision 703)

**The rule.** A field assigned through a `Shared`, `c.n = 2`, is `E0343`
(`type.shared-not-mutable`, "cannot modify `c.n`: a `Shared` holds it,
and a shared value is not changed in place, even through a `var`"), with
the note `type.shared-changes` as before and without the fix that takes a
parameter with `mut`, where `E0341` said "it is not declared with `var`"
also of a `var`.

**Decided against:**

- `[p for p in zip([1, 2], ["a", "b"])]` (`E0406` for the temporaries),
  as the eighth round decided: 04 ends a temporary with its statement and
  `zip` borrows what it is given, and the `str` items of the list are
  views the checker ties to the list. Accepting it would take a view's
  origin through a collection: a `str` read out of a `list[str]` borrows
  what the list's items borrow (here nothing but literals), not the list,
  a change of 04's loans for views, which is the owner's.
- A loop over a `mut` parameter whose body reads it (known over-rejection
  15): the monitor of stage 0 agrees with the checker on the SIR both
  front ends write, where a read of the parameter passes the `ref mut` on
  as a copy, a `mut` reborrow (`len(copy _1)`). Accepting it takes the
  lowering the over-rejection names, a read as a shared reborrow
  (`ref (*_1)`), which changes the SIR of every function that reads a
  `mut` parameter as a whole, stage 1's own among them (`prog.display(types,
  t)` with `mut types`), so the fixed point would need stage 0's lowering
  to change too, or a fixed point that compares the two SIRs modulo the
  rule: the coordinator's to decide, with stage 0's frozen front end.

**Why.** The manual's "Ownership" (chapters 5 and 13).

**In stage 1.** `typeck.check` (`BehindSharedRef`).

**What it changes.** The case `cases/findings12/shared_field.sisu` (its
`.diag`); `cases/findings4/fixes.diag` and the conformance suite's
`ownership/shared_immutable.diag` are blessed again (`E0343` for `E0341`).

- Differs: `compiler/stage1/cases/findings12/shared_field.sisu`: ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/ownership/shared_immutable.sisu`: types, ids

**Decision 703**, for docs/architecture.md:

703. A change of what a `Shared` holds is `E0343`, which says so whether
     the binding is a `var` or not. Stage 1 decides it (RULES.md, R114).

## R115. `@preview` warns at a use; `truncate_to` to 128 bits is exact (decision 704)

**The rule.**

- A use of a function, a type or a constant marked `@preview("...")` is
  `W0028` (`type.preview`, "`schedule` is a preview, which may change or
  be removed before it is stable: TEXT"), or `W0029` without a text, where
  `@deprecated`'s `W0023` and `W0025` stand (R30, R40), but in the item's
  own declaration.
- `truncate_to[T]()` of a `u64`, `u128` or `i128` outside `int`'s range
  to a type of 128 bits is the value's bits read as that type's, exact
  (`u64.MAX.truncate_to[i128]()` is 18446744073709551615, `u128.MAX.
  truncate_to[i128]()` is -1), where it gave the value of its low 64 bits:
  the prelude's three bodies ask `reflect.name_of(T)`, which R110 lets a
  generic method of the prelude do, so they are templates, made for each
  type argument (`u64.truncate_to#u32`) as a program's are.

**Left open:** a label at the template's line that `comptime for` unrolls
(X7): the instance's text is made from the template's and not mapped back
to its lines, which would take an origin for each unrolled line; and
statements before a derive's `return impl` (E0607): 08 makes a derive a
`comptime` function and neither gives nor rules them out, and the
template engine unfolds the `return impl` alone, so a statement would need
the compile-time evaluator inside the unfolding (a check of the type's
shape would then stop the derive with its message, the lean).

**Why.** The eleventh round's open items (the manual's findings under
"Metaprogramming", chapter 28, X11, R14; and decision 642).

**In stage 1.** `typeck.collect` (`previews`, `preview_warning`,
`deprecated_type_use`), `typeck` (the uses of functions and items);
`std/prelude.sisu`'s last block (`truncate_to` of `u64`, `u128` and
`i128`, whose doc comments say it; the exact values are the case's, since
`sisu test --std` runs the doc examples on the Rust front end).

**What it changes.** The cases `cases/findings12/preview_use.sisu` (its
`.diag`) and `truncate_wide.sisu` (its `.out`).

- Differs: `compiler/stage1/cases/findings12/preview_use.sisu`: ids, human

**Decision 704**, for docs/architecture.md:

704. A use of an item marked `@preview` warns (`W0028`, `W0029`), as a
     use of a deprecated one does; `truncate_to` of a value outside
     `int`'s range to a type of 128 bits is exact. Stage 1 decides it
     (RULES.md, R115).

## R116. `--target-runtime`: the runtime a program is built for (decision 705)

**The rule.** Every command of stage 1 takes `--target-runtime native` or
`--target-runtime interpreter` (anything else is a usage error), the
runtime the program is built for, which `target.runtime` is to the
program's compile-time code: `if comptime target.runtime == "native":` is
decided by it. Without it the runtime is `interpreter`, as before, so the
driver passes `--target-runtime native` to `sir` and `build` for a native
build (`sisu run --native`, `sisu build --native`), which it may rely on
from 0.10.0. The base's cache does not depend on it. `@cfg` on an import
and a dependency's `when` still read stage 0's facts (`interpreter`):
open, since they are read before the build's options reach them.

**Why.** The manual's finding under "Metaprogramming" (chapter 25):
`target.runtime` is `interpreter` in a native build too. 12 names the
runtime profiles (`single`, `multi`); `target.runtime` names how the
program runs, as 08's facts do, and `native` is what the driver's builds
are.

**In stage 1.** `main` (`target_runtime`, `std_texts`, `base_cache_key`),
`typeck.std_sources.target_for`, `typeck.collect` and `typeck.base`.

**What it changes.** No output of the corpus (`check.sh` builds for the
interpreter); `std_sources`' unit test.

**Decision 705**, for docs/architecture.md:

705. Stage 1 takes `--target-runtime native|interpreter`, which
     `target.runtime` gives compile-time code; the driver passes `native`
     for a native build. Stage 1 decides it (RULES.md, R116).

## R117. A field's `##` comment is `f.doc`; `cli.help_text` prints it (decision 706)

**The rule.** A field that reflection gives, in a derive's template and in
a generic function that reflects (`reflect.fields_of(T)`), has `doc`: its
`##` comment, the lines above it without `##` and the space after it,
joined by line breaks, its attributes passed over; empty without one (a
variant's fields have none yet). `cli.help_text[T]()` writes each line of
a field's comment under its option, indented by six spaces, and under the
positional arguments.

**Why.** G6's finding under "The standard library": a tool's help said
what kind of value an option takes and not what it is for. 08: reflection
"exposes ... the documentation of items, as values"; an item's own
comment (`reflect.doc_of(T)`) is left for when a program asks for it.

**In stage 1.** `typeck.collect` (`doc_in`, `FieldInfo.doc`),
`typeck.instances`, `typeck.template` (`FieldShape.doc`, `Value.Field`'s
`doc`, `Target.field_docs`); `std/cli.sisu`.

**What it changes.** The case `cases/findings12/field_docs.sisu` (its
`.out`; its field's attribute is R51's, which the Rust AST does not
keep). The Rust front end has no `f.doc`, so it refuses `std.cli`'s
`help_text`, and the programs that call it differ as listed.

- Differs: `compiler/stage1/cases/findings12/field_docs.sisu`: ast, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/reflect/cli.sisu`: sir, contracts-off, contracts-preconditions, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/testing/proposal_testing.sisu`: ids, human

**Decision 706**, for docs/architecture.md:

706. A field has `doc`, its `##` comment, to compile-time code, and
     `cli.help_text` prints it under each option. Stage 1 decides it
     (RULES.md, R117).

## R118. `@export` asks for types C has a layout for (decision 707)

**The rule.** An exported function's parameters and result are numbers of
a width C has (`int`, `f64`, `u8` to `u64`, `i8` to `i32`, `f32`), `bool`,
`char`, nothing (`()`), raw pointers (`ptr[T]`, `ptr mut T`) or types
declared `@repr(c)`; any other is `E0991` (`unsafe.export-not-c`, at the
parameter or the result), after `E0989`, which a type that holds a
reference, a view, a `Shared`, a closure or a capability gets instead.

**Why.** R14's finding under "Metaprogramming", and 12 ("C interop"):
"`@export` requires `@repr(c)` types at the boundary; the compiler
reports what is not representable." Stage 0's round makes the exported
function a global symbol.

**In stage 1.** `typeck.collect` (`check_export_blocking`,
`c_representable`, the `@repr(c)` types in `Program.c_layout`).

**What it changes.** The case `cases/findings12/export_boundary.sisu`
(its `.diag`; it imports `std.mem`, which only stage 1 reads, R4).

- Differs: `compiler/stage1/cases/findings12/export_boundary.sisu`: names, ids, human, doc, doc-output

**Decision 707**, for docs/architecture.md:

707. An exported function's signature holds numbers, `bool`, `char`, raw
     pointers and `@repr(c)` types; any other is `E0991`. Stage 1 decides
     it (RULES.md, R118).

## R119. A file run alone does not import the module beside it: `E0220` says why (decision 708)

**The rule.** A program of one file is one module: `E0220` for its import
of a module whose file (`NAME.sisu`) or directory stands beside it has the
note `resolve.module-beside`, which says that a package (a directory with
a `sisu.toml` and the files in `src/`) makes them one program.

**Decided against:** the directory of a file run alone as a package
without a manifest. 09 makes a package a directory with a `sisu.toml` and
a `src/` tree and a file a module of one, so a directory of loose files
would be a third kind of program, whose name, dependencies and edition
come from nowhere. A fix that writes the manifest is no edit of the file
the diagnostic stands in, which is all a fix is; `sisu new` (10) is the
tool's. `sisu test FILE` compiling the file alone in a package's
directory is the driver's.

**Why.** Chapter 17's finding under "Tooling and the driver".

**In stage 1.** `main` (`beside_notes`, in `file_input`).

**What it changes.** The case `cases/findings12/beside_module.sisu` (its
`.diag`, and the note in its human form) and `beside_stove.sisu`, the
module beside it.

- Differs: `compiler/stage1/cases/findings12/beside_module.sisu`: human, doc, doc-output

**Decision 708**, for docs/architecture.md:

708. `E0220` for a file's import of a module beside it notes that only a
     package makes them one program; a directory without a manifest is no
     package. Stage 1 decides it (RULES.md, R119).

**Decided against, the twelfth round:**

- `Decimal` as `Copy` (G31): 03 makes a `Decimal` a scaled `BigInt`, of
  any size, which allocates, so it is no more `Copy` than a `BigInt` is;
  `E0400`'s fix `.copy()` stands. A question for the owner, with that
  lean: a fixed-size decimal type would be another type of `std.num`.
- The `zip` temporaries and known over-rejection 15 (R114), and the
  directory as a package (R119).

## R120. `@export` in the SIR, and the world of transcripts in `sir --tests` (decision 740)

**The rule.**

- `sir` writes a line `export "SYMBOL" FUNCTION` after the `entry` line
  for each function marked `@export("SYMBOL")`, in the order of their
  bodies, as the Rust lowering writes them (`Program::exports`, decision
  683), and keeps each such function, and what it reaches, as a root of
  what it lowers, after the entry and before the tests, whether or not
  `main` calls it.
- `sir --tests` of a program that has transcripts (a `.transcript` file
  in the directory `tests/` beside its root module's file, where the
  driver finds them) keeps `transcript_world` and `transcript_system` as
  roots after its tests, as the Rust front end's `sir_with` does for the
  build its transcripts run (decision 687), so that the driver can run
  the transcripts from the SIR its tests run from. Other commands, `check`
  among them, do not lower them.

**Why.** Stage 0's eleventh round (decisions 683 and 687): the symbol is
the backend's to make, from the SIR's line, and the transcripts waited
for stage 1's `sir --tests` to keep their world.

**In stage 1.** `typeck.collect` (`Program.exports`), `lower.lower` (the
exports and `extra` roots), `sir.Program.exports`, `sir.write`; `main`
(`transcript_roots`, `Input.roots`, `lower_of`); stage 2's `read.read`
gives the two new fields of `sir.Program` (stage 2 reads the `export`
lines beside it, as before).

**What it changes.** The case `cases/findings13/export_roots.sisu` (its
`.sir` and `.out`), whose `export` line is stage 0's too; a program of
several modules whose exported function is outside the root module and
not called, now lowered; and `sir --tests` of the conformance suite's
`testing/shop`, which has transcripts.

**Decision 740**, for docs/architecture.md:

740. Stage 1's `sir` writes `export "SYMBOL" FUNCTION` after the entry
     for each `@export`ed function and lowers it as a root; `sir --tests`
     of a program with transcripts keeps `transcript_world` and
     `transcript_system`, so that the driver runs its transcripts from that
     SIR. Stage 1 decides it (RULES.md, R120).

## R121. `@no_alloc`, `@no_block` and `@stack` warn that nothing checks them yet (decision 741)

**The rule.** A function marked `@no_alloc`, `@no_block` or `@stack(...)`
compiles as before, and each attribute is `W0030`
(`resolve.unchecked-attribute`, "`@no_alloc` is accepted and checked by
nothing yet, so nothing ensures that the function does not allocate"; "...
does not block"; "... the function's bound on its stack"), at its name,
where decision 343's warnings stand for an attribute nothing reads. On
another declaration the attribute is still R40's `W0026`. `@no_panic`,
which D-059 puts after 0.1, is not among them.

**Why.** Decision 693 (stage 0's eleventh round): the three attributes
compiled without a word and checked nothing; D-059 puts the first two in
v1 and `@stack` with the native backend, so a program that writes them
expects a check it does not get.

**In stage 1.** `resolve` (`unchecked_promise`, `Resolver.attribute`);
the catalog's `W0030`.

**What it changes.** The case `cases/findings13/unchecked_attributes.sisu`
(its `.diag` and `.out`). The manual's chapter 27 (`mixer.sisu`) and its
reference page of attributes, whose programs write `@no_alloc`, now show
the warning.

- Differs: `compiler/stage1/cases/findings13/unchecked_attributes.sisu`: names, ids, human

**Decision 741**, for docs/architecture.md:

741. `@no_alloc`, `@no_block` and `@stack(...)` on a function warn
     (`W0030`) that they are accepted and checked by nothing yet. Stage 1
     decides it (RULES.md, R121).

## R122. The manifest's `runtime`: `full`, `single`, and `freestanding` refused (decision 742)

**The rule.**

- `[package] runtime` takes `"full"` (the default), `"single"` or
  `"freestanding"`, 12's three profiles; any other value is `E1003`
  (`package.bad-value`, "`runtime` takes `"full"`, `"single"` or
  `"freestanding"`"), and `"freestanding"` is `E1023`
  (`package.freestanding`, "the runtime profile `freestanding` has no
  runtime yet: this compiler's runtime schedules tasks, so a program is
  built for `full` or `single`"), at the value.
- The root package's profile, when it is not `full`, is written to the
  SIR as the line `runtime single` right after the `entry` line, before
  the `export` lines (R120), which stage 0's twelfth round reads.
- `target.runtime_profile` is the profile to compile-time code, `"full"`
  or `"single"` (12 names no fact; `target.runtime` already names how the
  program runs, R116). It is read from the manifest of the package the
  command works on (the directory an argument names, or the nearest
  package at or above the working directory when none does; a file alone
  is `full`) before the base is made, as R116's runtime is; the base's
  cache does not depend on it.

What the driver may rely on from 0.11.0: the `runtime` line of the SIR,
and R120's `export` lines and transcript roots.

**Why.** Decision 694 and 12, "Runtime profiles": a package chooses its
profile in its manifest; stage 0's scheduler is `single`'s, so `full`
and `single` build the same runtime, and `freestanding` waits for a
runtime without the scheduler.

**In stage 1.** `manifest` (`PackageInfo.runtime`), `package`
(`Package.runtime`), `main` (`Input.runtime`, `lower_of`,
`runtime_profile`, `profile_of`), `sir.Program.runtime`, `sir.write`,
`typeck.std_sources.target_for`; the catalog's `E1023`.

**What it changes.** The cases `cases/packages/runtime_single.pkg` (its
SIR has the line; stage 0's reader does not read it yet, so the case has
no `.out`) and `cases/packages/runtime_freestanding.pkg` (its `.diag`);
stage 0 warns of the unknown key (`W0005`).

- Differs: `compiler/stage1/cases/packages/runtime_freestanding.pkg`: sir, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/packages/runtime_single.pkg`: sir, ids, human, doc, doc-output

**Decision 742**, for docs/architecture.md:

742. `[package] runtime` is `full`, `single` or `freestanding`; the last
     is `E1023` until there is a runtime without the scheduler; `single`
     is written to the SIR as `runtime single` after the entry, and
     `target.runtime_profile` names the profile. `sisu1 --version` is
     0.11.0. Stage 1 decides it (RULES.md, R122).

## R123. A REPL session's SIR names each input's lines as the input's own (decision 743)

**The rule.** `sir --repl` writes the session's program with each input
it holds as a source of its own: a function the loaded file declares
(`sisu repl FILE`) has the file's name and the file's own lines, a
function a typed input declares has `<input>` and the input's lines, and
`main`, whose statements are every input's, has the last input's source,
the input whose run the session adds, each of its statements at its own
input's lines. The session's other lines (its `#!` line, the trailer that
shows a value) stay `<repl>`'s. A panic in `sisu repl FILE` is then at the
file's line (`at scaled (load.sisu:8:5)`) and one in a typed input at
`<input>:1`, as the input's diagnostics are; the column of an expression
that is shown is its own, without the `__repl_show(` before it.

The session's program records where each input stands in its last lines,
`# sisu-repl-input: FIRST LAST SHIFT PREFIX NAME`, after the inputs and
before the trailer, so that no line of an input moves; the driver keeps
the session's text as `--kept` gives it, as before.

**Why.** Decision 690: the session's one source put the loaded file's
lines one below their own and a typed input's after them.

**In stage 1.** `session` (`Region`, `regions_of`, `region_lines`,
`name_regions`), `main` (`repl_answer`, `held_build`, `Input.regions`,
`report_with`).

**What it changes.** The case `cases/repl/loaded_panic.repl` (its
`.out`); `cases/repl/session.repl.out` is blessed again (a panic in `main`
is at `<input>:1:1` where it was `<repl>:14:1`), and so is the
conformance suite's `live/session.repl`, which `sisu repl` runs on stage
1 (`at main (<input>:1:1)` where it was `session.sisu:19:1`). The manual's REPL
transcripts that show a panic's line change the same way.

**Decision 743**, for docs/architecture.md:

743. `sir --repl` names a loaded file's functions by the file at its own
     lines and a typed input's by `<input>` at the input's, `main` by the
     last input. Stage 1 decides it (RULES.md, R123).

## R124. A read of a `mut` parameter is a shared reborrow; stage 0's SIR of the package is a report (decision 744)

**The rule.** Where a place that is a `mut` reference (a `mut`
parameter, `_1: ref mut list[int]`) is read through a shared reference,
as an argument of a borrowed parameter, a receiver taken by `self`, or an
operand that compares, formats or measures, lowering passes a shared
reborrow of what it refers to, `_4 = ref _1.*`, where it passed the
reference on as a copy, which decision 40 makes a `mut` reborrow that
ends every loan of it. So a loop over a `mut` parameter may read it, and
a method call on it may take its length:

```sisu
def walk(mut path: list[int]) -> int:
    var total = 0
    for i, k in enumerate(path):
        total += (i + k) * len(path)
    path.push(total)
    return total

def add(mut xs: list[int]):
    xs.push(len(xs))
```

Both were `E0405` (known over-rejection 15); a `mut` argument and a
`mut self` receiver still pass the reference on.

`bootstrap.sh` no longer fails when `A.sir` differs from what stage 0
prints of the package: it prints the sizes and digests of both and says
"differs from stage 0, as the rules list". The fixed point is D-078's,
`B.sir` equal to `A.sir` and stage1-B equal to stage1-A; the Rust front
end is an oracle that falls behind (decision 268), whose SIR of every
function that reads a `mut` parameter is now the old one. `check.sh`'s
class `fixed` accepts the pair `fixed-sir0` when this rule lists it, and
its class `self` accepts a module's SIR when this rule lists it (``-
Differs: `compiler/stage1/src/...`: self``), as any other class accepts a
listed difference. Since stage1-A is built from stage 0's SIR and
stage1-B from `A.sir`, the two executables are the same bytes only when
those SIRs are: both scripts compare them as a verdict then, and report
them otherwise (`fixed-exe`, listed here). The class `build` accepts an
executable of a case whose SIR a rule changes when the rule lists the
case's output `build`: `contracts/checks.sisu`, whose release build with
every contract reads a `mut` parameter.

**Why.** Known over-rejection 15 (docs/architecture.md, decision 263's
corpus found it) names this lowering as the fix; the twelfth round left
the choice to the coordinator, since stage 1's own SIR changes, and the
coordinator chose it.

**In stage 1.** `lower.exprs` (`shared_reborrow`, `ref_operand`,
`arg_operand`, `arg_operand_for`'s `lent`, `lent_params`); `bootstrap.sh`
and `check.sh` (`fixed_pair`, the class `self`).

**What it changes.** The case `cases/findings13/mut_param_loop.sisu` (its
`.out`), which stage 0's front end refuses; the SIR of every program that
reads a `mut` parameter (or a `mut` binding of a pattern) whole, listed
below, and of stage 1's own package, module by module.

- Differs: `compiler/conformance/cases/examples/concurrency.sisu`: sir
- Differs: `compiler/conformance/cases/examples/wordcount.sisu`: sir, sir-tests
- Differs: `compiler/conformance/cases/ownership/borrowed_results.sisu`: sir
- Differs: `compiler/conformance/cases/ownership/dict_entries.sisu`: sir
- Differs: `compiler/conformance/cases/ownership/reference_params.sisu`: sir
- Differs: `compiler/conformance/cases/ownership/shared_param_passed_on.sisu`: sir, sir-tests
- Differs: `compiler/conformance/cases/run/closures.sisu`: sir
- Differs: `compiler/conformance/cases/std/ord_impls.sisu`: sir
- Differs: `compiler/conformance/cases/std/regex.sisu`: sir
- Differs: `compiler/conformance/cases/std/sort.sisu`: sir
- Differs: `compiler/conformance/cases/wrong/store_view_into_caller.sisu`: sir
- Differs: `proposal/examples/wordcount.sisu`: sir, sir-tests
- Differs: `compiler/stage1/cases/findings13/mut_param_loop.sisu`: ids, human
- Differs: `compiler/stage1/<std>/prelude.sisu`: self
- Differs: `compiler/stage1/src/borrowck/consume.sisu`: self
- Differs: `compiler/stage1/src/borrowck/elaborate.sisu`: self
- Differs: `compiler/stage1/src/borrowck/holes.sisu`: self
- Differs: `compiler/stage1/src/borrowck/loans.sisu`: self
- Differs: `compiler/stage1/src/borrowck/shared.sisu`: self
- Differs: `compiler/stage1/src/borrowck/types.sisu`: self
- Differs: `compiler/stage1/src/cfg.sisu`: self
- Differs: `compiler/stage1/src/diff.sisu`: self
- Differs: `compiler/stage1/src/fmt/doc.sisu`: self
- Differs: `compiler/stage1/src/fmt/lines.sisu`: self
- Differs: `compiler/stage1/src/interp.sisu`: self
- Differs: `compiler/stage1/src/lexer/layout.sisu`: self
- Differs: `compiler/stage1/src/lower.sisu`: self
- Differs: `compiler/stage1/src/lower/contracts.sisu`: self
- Differs: `compiler/stage1/src/main.sisu`: self
- Differs: `compiler/stage1/src/package.sisu`: self
- Differs: `compiler/stage1/src/sir/simplify.sisu`: self
- Differs: `compiler/stage1/src/typeck.sisu`: self
- Differs: `compiler/stage1/src/typeck/arbitrary.sisu`: self
- Differs: `compiler/stage1/src/typeck/borrows.sisu`: self
- Differs: `compiler/stage1/src/typeck/collect.sisu`: self
- Differs: `compiler/stage1/src/typeck/exhaustive.sisu`: self
- Differs: `compiler/stage1/src/typeck/folding.sisu`: self
- Differs: `compiler/stage1/src/typeck/infer.sisu`: self
- Differs: `compiler/stage1/src/typeck/program.sisu`: self
- Differs: `compiler/stage1`: fixed-sir0, fixed-exe
- Differs: `compiler/conformance/cases/contracts/checks.sisu`: build

**Decision 744**, for docs/architecture.md:

744. A read of a `mut` reference through a shared one is lowered as a
     shared reborrow, `ref _1.*`, so a loop over a `mut` parameter may
     read it (known over-rejection 15 is gone from stage 1); the
     bootstrap's comparison of stage 1's SIR with stage 0's is a report,
     the fixed point `B.sir` equal to `A.sir`. Stage 1 decides it
     (RULES.md, R124).

## R125. The base cache keeps what collecting the base found of the std texts (decision 745)

**The rule.** The base cache's form is 2: besides the derived impls and
the promotions, it keeps which std modules a program may import hold
templates (`reflect.`) and `unsafe` code (`StdMarks`), which a base made
from the cache reads rather than searching every module's text again. A
base made from the cache finds the lines of a std text when lowering
first reaches a body of it (`std_lines`), not of every text at once (a
batch, whose base is made whole, and `sir --repl-serve`, which serves
many inputs, find them all once); the message catalog's lines are read
by their first four words, not split whole; the lexer looks punctuation
up by its bytes, not by a text made for each try. Nothing a command
prints changes; the write stays atomic (`Fs.rename`) and the key is the
same (the std texts' names, sizes and times, and the executable's).

**Measured** (callgrind, `sisu1 check` of a two-line program with the
cache): 4.64 G instructions before, 4.15 G after (−10.6 %); `base_of`
4.11 G before, 3.79 G after. What dominates is not the collecting but the
parsing of what the base declares: lexing and parsing the prelude, the
ambient modules, `target`, `builtin_impls` and the derived impls' text,
1.94 G (the lexer 1.03 G, about 3,500 instructions a byte of the 250 KB),
and building their ASTs, 0.56 G; then resolving them (0.37 G) and
checking the traits (0.15 G), with the copies and drops of the values
they make throughout.

**Decided against:** a cache of the collected items themselves, the
resolver's program and types and the ASTs of the base modules, written
and read back. Lowering reaches the std bodies through their ASTs, so the
cache would hold the whole syntax of the prelude and `cap` as well as
every definition and type the resolver made, a reader and writer for each
of the AST's and the resolver's types (about a hundred, with the
interner's numbering to keep), at a cost out of proportion to one round
beside the rest, and a text reader in Sisu at its present speed would
take back a part of what the parse costs. The saving is the parse, about
half of a command's start; the lexer's speed (3,500 instructions a byte)
is the cheaper half of it, and the next round's.

**In stage 1.** `main` (`BASE_CACHE_FORM`, `base_cache_text`,
`read_base_cache`, `repl_serve`), `typeck` (`BaseCache.marked`,
`base_cached`, `with_lines`), `typeck.collect` (`StdMarks`,
`Resolver.base_core`), `diag` (`first_words`), `lexer.scan`
(`spelling_key`).

**What it changes.** No output of the corpus.

**Decision 745**, for docs/architecture.md:

745. The base cache keeps which importable std modules hold templates and
     `unsafe` code, and a base made from it finds a std text's lines when
     lowering first reaches it: a command's start is 10 % shorter; the
     parse of the base modules, half of it, stays. Stage 1 decides it
     (RULES.md, R125).

## R126. The twelfth round's open items: `E0258` alone for a private field (decision 746)

**The rule.** A struct of another module built with a private field left
out, `Ledger(owner="me")` for `struct Ledger` with `bookings` private and
without a default, is `E0258` alone, whose note names the function that
makes one (`Ledger.new(owner)`, R104), where `E0337` followed it with the
fix that passes `bookings=todo("bookings")`, which `E0258` refuses again.
A public field left out is `E0337` as before.

**Decided against:**

- `TotalF64` as `Hash`, a key of a `dict` or a `set`: the runtime hashes
  and compares a key structurally (decision 256), so a key whose `Eq` is
  its own, as `TotalF64`'s is (every NaN equal), is `E0398`; an impl of
  `Hash` would make it that, not a key. The key waits for a runtime whose
  dict calls a key's impls, which is stage 0's.
- `Lookup`'s doc example with a lookup by `str` (`prices.get(dish)`):
  `sisu test --std` runs the prelude's doc examples on the Rust front end,
  which has no `Lookup` (R115), so the example calls `lookup_key()`.

**Left open:** the label at the template line that `comptime for`
unrolls (the instance's text is made whole, without an origin for each
unrolled line, R115); an item's own `##` text in reflection (`f.doc` of
a field is R117's; `reflect.doc_of(T)` waits for a program that asks);
`@cfg` on an import and a dependency's `when` under `--target-runtime
native`, which the package loader and the name pass read from
`manifest.target_facts()` before the build's options reach them (R116).

**Why.** The manual's finding under "Types and traits" and "Diagnostics"
(G32, R2 and chapter 24), and the twelfth round's open items.

**In stage 1.** `typeck.call` (the struct's fields in `check_struct`).

**What it changes.** The case `cases/packages/private_field_hole.pkg`
(its `.diag`).

- Differs: `compiler/stage1/cases/packages/private_field_hole.pkg`: types, ids, human, doc, doc-output

**Decision 746**, for docs/architecture.md:

746. A struct of another module built without a private field is
     `E0258` alone; `TotalF64` stays no key, since the runtime hashes keys
     structurally. Stage 1 decides it (RULES.md, R126).

## R127. The lexer looks nothing up, and the AST builder walks no tree twice (decision 760)

**The rule.** Nothing a command prints changes; the token stream is the
same, byte for byte (`check.sh`'s `tokens`, `tree`, `trivia-tree` and
`print` classes), and so is every AST. What changes is how the lexer and
the AST builder find what they find:

- The scanner dispatches on a punctuation's bytes (`scan.punctuation_of`:
  the first byte, then the next one or two, the longest of `PUNCTUATION`
  that the text starts with), where it looked a key made of the bytes up
  in a `dict` for a length of three, then two, then one; and it knows a
  keyword by its first byte and its length, then its bytes
  (`scan.keyword_at`), where it made a `String` of every name and looked
  it up. The scanner no longer builds the two tables for each text it
  scans. Two unit tests hold the dispatch to `PUNCTUATION` and `KEYWORDS`
  ("punctuation by its bytes", "keywords by their bytes"), each spelling
  alone and before a space, a name and another operator's first byte.
- The loops over a name, a comment, a run of spaces and a string's text
  read each byte once, in place, where each read was a call of
  `Scanner.byte` (and a triple-quoted string's text tried `"""` at each
  byte); a scan in code reads the mode only inside a string; the layout
  pass counts a space of the indentation without decoding a character,
  and finds the line breaks by a `match`.
- The AST builder knows whether a tree has an `ERROR` node at all, and a
  tree without one (every std text, and most programs) needs no walk of a
  function's body for `body_broken` or of its header for `header_broken`;
  and it finds an item among its parent's children by a binary search on
  where the children start (`B.element_start`, `B.child_index`) to read
  the doc comments before it, where it walked from the first child, which
  made a block of many items, an `impl` of the prelude's, quadratic.

**Measured** (callgrind, natively, the round's starting stage 1 and this
one built by the same stage 0): `sisu1 tokens` of `std/prelude.sisu`
(194 KB) lexes in 543 M instructions before, 284 M after (2,800 to 1,470
a byte; the scan 440 M to 196 M, the layout 103 M to 89 M); a cached
`sisu1 check --format=ids` of a two-line program executes 4.147 G before
(R125's 4.15 G) and 3.420 G after (−17.5 %): the lexer 1.028 G to
0.531 G, the AST builder 0.773 G to 0.542 G, the parser itself (without
the lexer) 0.908 G as before, `base_of` 3.793 G to 3.067 G.

**Why.** README's "Where the time goes" and R125: the lexer's 3,500
instructions a byte were the cheaper half of a command's start, and the
next round's.

**In stage 1.** `lexer.scan` (`punctuation_of`, `with_eq`, `doubled`,
`wrapping`, `keyword_at`, `Word`), `lexer.layout` (`split_lines`,
`indentation_width`), `lexer` (the two tests), `syntax.ast.build` (`B.errors`,
`docs_before`, `element_start`, `child_index`).

**What it changes.** No output of the corpus.

**Decision 760**, for docs/architecture.md:

760. Stage 1's lexer dispatches on a punctuation's and a keyword's bytes
     and reads each byte once, and its AST builder walks a tree without
     errors once and finds an item among its siblings by a binary search:
     a command's start is 17.5 % shorter (4.15 G instructions to 3.42 G
     for a two-line `check`, the lexer 1.03 G to 0.53 G), with the same
     tokens and trees. Stage 1 decides it (RULES.md, R127).

## R128. An import's `@cfg` and a dependency's `when` read the runtime the build is for (decision 761)

**The rule.** `target.runtime` in an import's `@cfg` (R106) and `runtime`
in a dependency's `when` are the runtime `--target-runtime` names (R116),
`native` or `interpreter`, as `if comptime target.runtime` in a body
already was: a build for the native runtime (`sisu run --native`, `sisu
build --native`, for which the driver passes `--target-runtime native`)
builds and imports a dependency whose `when = { runtime = "native" }`, and
one for the interpreter leaves it out, as `when = { os = "windows" }`
leaves one out on Linux. Without the option the runtime is `interpreter`,
as before.

**Why.** R116's open item: the package loader and the name pass read
`manifest.target_facts()`, the interpreter's, before the build's options
reached them, so a program that reached a dependency for the native
runtime only was refused by its native build.

**In stage 1.** `manifest` (`target_facts_for`), `package`
(`FeatureRequest.runtime`, `for_runtime`, `Layout.runtime`; the loader's
facts), `cfg` (`Features.runtime`, `for_runtime`, `facts`;
`target_condition`, `discarded_branches`), `main` (`build_request`,
`feature_request`, the `Features` of a file and of each package,
`target_absences`); the tests "a dependency's `when` reads the runtime
the build is for" (`package`) and "an import's `@cfg` reads the runtime
the build is for" (`cfg`).

**What it changes.** The case `cases/packages/runtime_import.pkg` (its
`.out`, `interpreted`, as `check.sh` builds it), whose native build
(`sisu run --native` in its directory, and `sisu1 sir --target-runtime
native` run from SIR) prints `native`; no other output of the corpus.

- Differs: `compiler/stage1/cases/packages/runtime_import.pkg`: sir, ids, human, doc, doc-output

**Decision 761**, for docs/architecture.md:

761. An import's `@cfg(target.runtime == ...)` and a dependency's `when =
     { runtime = ... }` read the runtime that `--target-runtime` names, so
     a native build reaches a dependency for the native runtime. Stage 1
     decides it (RULES.md, R128).

## R129. An item's own `##` is `reflect.doc_of(T)`, a variant's field's is `f.doc`, and `cli.about` gives a struct's (decision 762)

**The rule.**

- `reflect.doc_of(T)` is the `##` comment above the declaration of `T`
  (a struct, an enum, an error set), its attributes passed over, as R117
  reads a field's: the lines without `##` and the space after it, joined
  by line breaks; empty for a type without one, for a type of the
  standard library, whose text the base does not keep, and for any other
  type. It is a value of compile-time code, in a derive's template and in
  a generic function that reflects, as `reflect.name_of(T)` is.
- A variant's field that starts a line of its own has `f.doc`, the `##`
  comment above it, in `reflect.variants_of(T)`'s `v.fields`, as a
  struct's field has since R117; a field on the variant's own line,
  `Guessed(int)`, has none, since the comment above that line is the
  variant's.
- `cli.about[T]()` is the struct's own comment, which a tool prints
  before `cli.help_text[T]()`; `help_text` prints what it printed, so that
  the snapshots of its text stay (the conformance suite's
  `testing/proposal_testing` holds one of a struct with a comment).

**Why.** The manual's finding under "The standard library" (R12):
reflection gave a struct's fields their comment but not an item's own or
a variant's fields'; 08 has reflection expose "the documentation of
items, as values". R117 left `reflect.doc_of(T)` for when a program asked
for it, and a tool's help is the program.

**In stage 1.** `typeck.template` (`TypeShape.doc`, `VariantShape.docs`,
`Target.doc`, `reflect`'s `doc_of`, `variants_of`), `typeck.instances`
(a derive's target and a reflected type's shape), `typeck.collect`
(`variant_field_doc`, `own_line_doc`), `typeck.program`
(`VariantInfo.docs`); `std/cli.sisu` (`about`, added).

**What it changes.** The case `cases/findings14/item_docs.sisu` (its
`.out`), which the Rust front end refuses (it has no `reflect.doc_of`);
no other output of the corpus.

- Differs: `compiler/stage1/cases/findings14/item_docs.sisu`: ids, human, doc, doc-output

**Decision 762**, for docs/architecture.md:

762. `reflect.doc_of(T)` is the `##` comment of `T`'s declaration, a
     variant's field on a line of its own has `f.doc`, and `cli.about[T]()`
     gives a struct's own comment. Stage 1 decides it (RULES.md, R129).

## R130. A derive's template may check the type before its `return impl` (decision 763)

**The rule.** The body of a trait's `comptime def derive(T: type) -> impl
Trait for T` may begin with checks of the type before its `return impl
Trait for T:`: `comptime assert COND, "MESSAGE"`, `comptime panic("...")`,
and `comptime if` or `if comptime` whose branches hold only checks. The
unfolding runs them first, for each type the derive is for, and they
write no code; one that fails stops the derive there, `E0608` at the
`@derive` with the template's line where it stopped, as a failure inside
a method of the impl does:

```sisu
comptime def derive(T: type) -> impl Tabled for T:
    comptime if not reflect.is_struct(T):
        comptime panic("a table's row is a struct's fields")
    return impl Tabled for T:
        ...
```

Any other statement before or after the `return impl` is `E0607` as
before; when the body has a `return impl`, `E0607` now has the note
`comptime.derive-checks`, which says what may come before it.

**Decided against:** a `let` before the `return impl`, a value computed
once for the type that the impl's methods would read. The engine writes
the impl's methods as text for the type, and a name bound before them
would be a compile-time variable of every method, which it has only for
the variable of a `comptime for`; a method computes such a value itself,
at compile time, as before.

**Why.** G21's finding under "Metaprogramming": a derive's body must be
the `return impl` alone, so a check of the type's shape goes inside a
method. 08 makes the derive a `comptime def` like any other, whose
`return impl` is the one place it produces a declaration; a check before
it produces none.

**In stage 1.** `typeck.template` (`template_parts`, `template_checks`,
`is_check`, `checks_only`, `returns_impl`, `unfold`), `typeck.collect`
(`E0607`'s note); the catalog's `comptime.derive-checks`.

**What it changes.** The cases `cases/findings14/derive_checks.sisu` (its
`.out`) and `cases/findings14/derive_checks_refused.sisu` (its `.diag`),
both of which the Rust front end refuses with `E0607`.

- Differs: `compiler/stage1/cases/findings14/derive_checks.sisu`: ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/findings14/derive_checks_refused.sisu`: ids, human, doc, doc-output

**Decision 763**, for docs/architecture.md:

763. A derive's template may begin with compile-time checks of the type,
     `comptime assert`, `comptime panic` and `comptime if` of them, before
     its `return impl`; a failing check is `E0608` at the `@derive`, and
     `E0607` names what may come first. Stage 1 decides it (RULES.md,
     R130).

## R131. An error in an instance labels the template's line that wrote it (decision 764)

**The rule.** A diagnostic of a template's instance that R77 places at the
call that made the instance, with the instance's line in the note
`type.instance-code`, has the label `type.instance-line` ("the template's
line that wrote it") at the statement of the program's own template that
wrote that line, the innermost: the body of the `comptime for` that
unrolled it, the branch of the `if comptime` that chose it.

```text
error[E0310]: `+=` cannot be applied to `int` and `String`
  --> unrolled_line.sisu:16:11
   |
16 |     print(total(Sauna(heat=80, name="Aino")))
   |           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
12 |         n += v.[f]
   |         ---------- the template's line that wrote it
  = note: in the instance of `total[Sauna]` that this call makes: `n += v.name`
```

The unfolding records, for each statement of the template, the stretch of
the instance's code it wrote (`Unfold.origins`, `Traced`), and the label
is the statement whose stretch holds the diagnostic's start. A template
of the standard library has no label, since a program's diagnostics show
none of its text; one in another module than the call's is labelled in
its own file.

**Why.** The manual's finding under "Metaprogramming" (X7): an error in
unrolled code stood at the call with the unrolled line in a note, and
nothing pointed at the template's line. The twelfth and thirteenth rounds
left it open (R115, R126), since the instance's text was made whole.

**In stage 1.** `typeck.template` (`Unfold.origins`, `stmt`,
`unfold_fn_traced`, `Traced`, `origin_at`), `typeck.instances`,
`typeck.collect` (`Resolver.instance_origins`), `typeck` (the label at
the call); the catalog's `type.instance-line`.

**What it changes.** The case `cases/findings14/unrolled_line.sisu` (its
`.diag`), and every program whose instance has an error placed at its
call, which now has the label: the `.diag` files of
`cases/findings8/instances.sisu`, `cases/findings11/compound_operator.sisu`
and the conformance suite's `reflect/test_instances.sisu` are blessed
again (each differed from stage 0 already, as R77 and R104 list).

- Differs: `compiler/stage1/cases/findings14/unrolled_line.sisu`: ids, human, doc, doc-output

**Decision 764**, for docs/architecture.md:

764. An error of a template's instance placed at its call labels the
     statement of the program's template that wrote the instance's line.
     Stage 1 decides it (RULES.md, R131).

## R132. `check.sh`'s class `transcripts`: the transcripts' roots on every run (decision 765)

**The rule.** `check.sh` has the class `transcripts`: each program of the
conformance suite with transcripts (`NAME.files/tests/*.transcript`, so
far `testing/shop`), put in a sandbox as the conformance runner puts it
(the file and what its `.files/` holds), is lowered with its tests by
stage 0 (`sisu sir --tests NAME.sisu` there) and by stage 1, whose SIR
must hold `transcript_world` and `transcript_system` as functions and is
compared with stage 0's; and the driver's `sisu test --no-cache NAME.sisu`
runs there on stage 1 (`SISU_STAGE1_TRANSCRIPTS=1`; with `--native`,
`SISU_STAGE1` is the executable the run built), its tests and its
transcripts from stage 1's SIR (decision 722), and must print the case's
`NAME.test`. R120's roots, which the thirteenth round checked by hand, are
checked on every run.

**Why.** R120 kept the world of transcripts in `sir --tests`, and decision
722 runs transcripts from that SIR; no class ran it.

**In stage 1.** `check.sh` (the class, its summary line).

**What it changes.** No output; `testing/shop`'s SIR differs from stage
0's by R120's roots, as listed.

- Differs: `compiler/conformance/cases/testing/shop.sisu`: transcripts

**Decision 765**, for docs/architecture.md:

765. Stage 1's `check.sh` compares the SIR of each program with
     transcripts, built with its tests, with stage 0's and runs its
     transcripts through the driver from stage 1's SIR, against the
     case's `.test`. Stage 1 decides it (RULES.md, R132).

## R133. Unicode names: NFC and the confusable lints decided against for this round (decision 766)

**The rule.** Nothing changes: a name is the bytes it is written with, a
combining mark after a letter is `E0005`, and there is no confusable or
mixed-script lint (D-048; the manual's finding under "Syntax and names").

**Decided against**, for this round, with what it would take:

- **NFC.** A table of the canonical compositions whose result is a
  letter: 1,251 pairs (base, mark) in Unicode 15.1, with the combining
  classes of the marks to put several in canonical order first; the
  scanner would take a mark (category `Mn`) after a letter into the name,
  and every place that reads a name's text would read its NFC form:
  `ast.build`'s names, the name pass, which reads the syntax tree's
  tokens itself (`resolve`), and `sisu doc`, while the formatter keeps the
  text as written. A token's text and a name's would then differ, which
  the lossless tree and the name pass's labels, at a token's bytes, must
  both carry.
- **The confusable and mixed-script lints.** Unicode's `confusables.txt`
  maps about 6,300 characters to their skeletons; restricted to the
  scripts an identifier takes (Latin, Greek, Cyrillic and the rest of
  UAX 31's recommended set) it is still over a thousand entries, and the
  mixed-script lint needs the script of every letter (`Scripts.txt`, some
  2,000 ranges). Stage 1 reads its tables as constants compiled into it;
  tables of that size are a generated module, and the lints `sisu lint`'s
  (R11), over every name of a package.

Each is a round's work of its own, beside the lexer's speed (R127), which
a table read per non-ASCII character would give back in part; the
generated tables come first, made by a script from the Unicode data the
round names.

**Why.** D-048 decides NFC names and the lints; the manual's finding says
they "wait for the lexer's Unicode tables", which do not exist yet.

**What it changes.** Nothing.

**Decision 766**, for docs/architecture.md:

766. NFC names and the confusable and mixed-script lints of D-048 wait
     for generated Unicode tables (1,251 compositions; the confusables of
     the identifier scripts and their script ranges): not this round.
     Stage 1 decides it (RULES.md, R133).
