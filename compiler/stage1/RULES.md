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

## R134. A lookup by a `str` passes the `str` itself: no `String` is made for it (decision 800)

**The rule.** `get` and `remove` of a `dict[String, V]`, its index read as
a value, and `contains` and `remove` of a `set[String]`, given a `str`
(R113), lower to the intrinsics of decision 784 that take the `str`
itself, `dict.get_str`, `dict.remove_str`, `dict.index_str`,
`set.contains_str` and `set.remove_str`, where they lowered to the
`String` forms with the key `str.Lookup.lookup_key()` made, which a
program no longer calls (and whose function the SIR no longer holds). Each
`_str` intrinsic is declared with the facts of its `String` form
(`dict.get_str pure from 0`, `dict.remove_str pure inner 0 keeps 0`), and
`dict.index_str pure from 0`, whose reference is read as the index was.
What a program prints is what it printed, interpreted and natively, but
that a missing key's panic now stands at the index (`at main
(str_lookups.sisu:26:11)`), as decision 784's test has it, where it stood
at the statement that read the index (`26:5`). The compile-time evaluator
knows the five intrinsics (`interp`), as the interpreter does.

A string literal where a `String` key goes is still the `String` the type
checker makes of it (`String.new`), and `dict.contains` given a `str`
still takes the `String` `lookup_key` gives: `k in d`, which compared the
text already, is the form without one. A `d[k] = v` by a `str` needs the
`String` it inserts, and an index that is borrowed, the receiver of a
method (`d[k].byte_len()`) or a place changed through it (`d[k].n += 1`),
keeps the `String` form; the borrow by `dict.index_str`'s reference is
open.

**Measured** (callgrind; `compiler/target/s15/bench.sisu`, 10,000 each of
a `get`, an index and a `set`'s `contains` by a `str`, built by the
round's starting stage 1 and by this one): on the interpreter (`sisu run
--from-sir`) 370.0 M instructions before, 259.8 M after (−30 %); natively
(`sisu build --native --release`) 30,031 allocations before and 31 after,
but 97.8 M instructions before and 123.8 M after (+27 %): the backend
lowers `dict.get`, `dict.contains` and `set.contains` in place
(`sisu-codegen`'s `I::DictGet | I::DictContains | I::SetContains`), while
the `_str` intrinsics go through the runtime's dispatch
(`rt_dispatch_Call_word`, 22.4 M of the run, a chain of comparisons to
numbers 323 to 327); the same in-place lowering of the five is stage 0's
to add, after which the native run loses what the interpreter did.

**Why.** Decision 729: the lookup without its `String` is the front end's
lowering first; stage 0's thirteenth round gave the intrinsics (decision
784).

**In stage 1.** `lower.exprs` (`str_lookup`, `str_key`, `read_place_of`,
the call of an intrinsic, `operand`), `lower` (`intrinsic_decls`: the
facts of a `_str` intrinsic), `interp` (the five intrinsics).

**What it changes.** The SIR of a program that looks a `str` up (no
program of `sir.list`, whose 549 outputs are the same file for file); the
case `cases/findings15/str_lookups.sisu` (its `.out`, `.err` and
`.status`).

- Differs: `compiler/stage1/cases/findings15/str_lookups.sisu`: ids, human, doc, doc-output

**Decision 800**, for docs/architecture.md:

800. Stage 1 lowers `get`, `remove` and the read of an index of a
     `dict[String, V]`, and `contains` and `remove` of a `set[String]`,
     given a `str`, to decision 784's intrinsics that take the `str`, so
     that no `String` is made for the key; a missing key's panic stands at
     the index. On the interpreter a loop of 30,000 lookups takes 30 % fewer
     instructions; natively it allocates nothing but runs through the
     runtime's dispatch, 27 % more, until the backend lowers the five in
     place as it does `dict.get`. Stage 1 decides it (RULES.md, R134).

## R135. The parser makes its table of lines from its tokens and calls less per token (decision 801)

**The rule.** Nothing a command prints changes; every tree is the same,
byte for byte (`check.sh`'s `tree`, `trivia-tree` and `print` classes, and
3,000 mutants of the corpus and stage 1's cases compared by a check built
into a trial build, below). What changes is how the parser does its work:

- `Parser.new` finds where the lines start from the tokens that hold a
  line break (`LineBreak`, `Newline` and the texts of strings, raw and
  byte strings, characters, format specs and error tokens), as it walks
  them for the significant ones, where `Lines.of` read every byte of the
  text a second time after the lexer; `Lines.of_starts` makes the table,
  with no line known to be ASCII, so that `line_col` (which the parser's
  table never serves) counts characters.
- `current` and `at` read the token themselves, where each called `nth`;
  `bump` attaches the trivia before a token and the token in one call
  (`TreeBuilder.tokens_until`), where it called `flush_trivia`,
  `tokens_attached` for each trivia token, and `token`;
  `TreeBuilder.finish_node` takes a node's children in one pass.
- `render_ids_in` of no diagnostic makes no table of lines.

Two trials were left out: `list.with_capacity` for a node's children,
which goes through the runtime's dispatch natively and cost 73 M of the
prelude's parse where the growth it saves cost 18 M; and reading the
lines' bytes through `str.bytes()`, five times the cost of `byte_at`.

**Measured** (callgrind, natively, the round's starting stage 1 and this
one built by the same stage 0): `sisu1 parse --print std/prelude.sisu`
executes 988.3 M instructions before, 832.2 M after (−15.8 %), the parse
(`parse_lexed`) 463.6 M before and 352.9 M after (−23.9 %); a cached `sisu1
check --format=ids` of a two-line program 3.432 G before and 3.228 G after
(−5.9 %), the parser itself 0.908 G before (R127's 0.91 G) and 0.704 G
after (−22.5 %), the lexer 0.531 G as before. The batches (the better of
two runs each, at a load of 13 to 14 on four cores): `types-batch` of
`types.list` 7.33 s of user time before and 7.08 s after, `sir-batch` of
`sir.list` 25.3 s before and 26.3 s after (the front end 9.4 s and 9.6 s),
which the load's noise covers; their outputs are the same file for file.
The release profile of the native build was measured beside them, and is
not this round's: `sisu build --native --release` of stage 1 parses the
prelude in 373 M instructions where the debug profile, which `sisu1` and
`check.sh` build, takes 988 M; the profile stage 1 is built with is the
driver's and `sisu1`'s to choose.

**Why.** R127 left the parser itself at 0.91 G of the two-line `check`'s
3.42 G; the profile put a tenth of the prelude's parse in the second scan
of its bytes and most of the rest in calls of a few instructions each
(about 100 instructions a call in the native code of the debug profile).

**In stage 1.** `parser.state` (`Parser.new`, `current`, `at`, `bump`,
`flush_trivia`, `bump_eof`), `syntax.tree` (`TreeBuilder.tokens_until`,
`finish_node`), `diag` (`Lines.of_starts`, `render_ids_in`).

**What it changes.** No output of the corpus.

**Decision 801**, for docs/architecture.md:

801. Stage 1's parser finds the starts of lines from its tokens and calls
     less per token: a parse of the prelude takes 24 % fewer instructions
     and a two-line `check` 6 % fewer (3.43 G to 3.23 G, the parser 0.91 G
     to 0.70 G), with the same trees. Stage 1 decides it (RULES.md, R135).

## R136. Unicode names: NFC, the mixed-script lint `W0031` and the confusable lint `W0032` (decision 802)

**The rule.**

- A name may hold, after its letters, a mark that combines with the
  letter before it (a canonical combining class other than 0), where the
  mark was `E0005`; the token is the name as written.
- A name is read in NFC (02, "Lexical structure"): the AST's names
  (`Name`, a name expression) and the name pass's (`resolve`) are the
  name's characters decomposed, its marks put in their canonical order and
  composed again (UAX 15), so `cafe` with a combining acute accent and
  `café` written with one letter are one name. A name of ASCII is itself,
  at the cost of a look at its bytes. The formatter and the tree keep the
  text as written; a label stands at the token's bytes.
- `W0031` (`resolve.mixed-script`): a declared name whose letters are of
  more than one script, `pаrt` with a Cyrillic `а`, naming the scripts in
  the order the name uses them ("Latin and Cyrillic"). Digits, `_` and
  letters of no script (the mathematical alphabets) count for none, and
  the scripts of Chinese, Japanese and Korean names count as one.
- `W0032` (`resolve.confusable`): a declared name that reads as another
  name in scope (a module's item, a local or parameter of an enclosing
  scope), the two different, one of them beyond ASCII, and their skeletons
  the same: each letter written as the ASCII letter or digit it reads as.
  The label `resolve.confusable-here` stands at the other's declaration
  when it is the module's. Nothing is compared until a module declares a
  name beyond ASCII, so a program of ASCII names pays nothing more.

The tables are generated: `tools/unicode_tables.py` writes
`src/unicode_tables.sisu` from Python's `unicodedata` (Unicode 15.1.0),
821 lines: 1,251 compositions, 1,382 decompositions (the compatibility
ideographs left out, which a name keeps), 388 ranges of combining classes,
639 ranges of letters' scripts (a letter's script the first word of its
name, with the words of no script and the aliases the script lists) and
179 ranges of confusables (the letters whose NFKC form is one ASCII letter
or digit, and the usual Cyrillic, Greek and Latin look-alikes, a table in
the script). `check.sh` fails when the file is not what the script writes
(`python3 -I tools/unicode_tables.py --check`). The lookups are binary
searches over the tables' records, as `text.unicode`'s (`text.names`), on
the rare path of a character beyond ASCII. The scripts are an
approximation of Unicode's `Scripts.txt` from the characters' names, and
the confusables a subset of `confusables.txt`, as R133 expected; Hangul
syllables, which `unicodedata` decomposes by rule, are not composed.

**Why.** D-048 and 02 make names NFC with the two lints on by default; R133
said what they wait for, the generated tables, which are 821 lines.

**In stage 1.** `tools/unicode_tables.py`, `unicode_tables` (generated),
`text.names` (`nfc`, `scripts_of`, `skeleton`, `is_combining`, with the
tests "a name is read in NFC" and "a name's scripts and skeleton"),
`lexer.scan` (`ident`), `syntax.ast.build` (`name`, a name expression),
`resolve` (the names it reads, `check_style`, `check_confusable`,
`confusable`, `Resolver.unicode`), `diag` (`ResolveMixedScript`,
`ResolveConfusable`, `ResolveConfusableHere`); `check.sh` (the stale
check); the catalog's `W0031`, `W0032` and `resolve.confusable-here`.

**What it changes.** The cases `cases/findings15/nfc_names.sisu` (its
`.out`; the Rust lexer refuses the combining mark with `E0005`) and
`cases/findings15/confusable_names.sisu` (its `.diag` and `.out`); no
other output of the corpus.
The case `cases/unicode_names.sisu`, whose `café` with a combining accent
was a name and an `E0005`, is one name now, as in the tree.

- Differs: `compiler/stage1/cases/findings15/nfc_names.sisu`: tokens, tree, trivia-tree, ast, ids, human, fmt-check, doc, doc-output
- Differs: `compiler/stage1/cases/findings15/confusable_names.sisu`: names, ids, human
- Differs: `compiler/stage1/cases/unicode_names.sisu`: tokens, tree, trivia-tree, ast, ids, human, fmt-check, doc, doc-output

**Decision 802**, for docs/architecture.md:

802. A name may hold combining marks after its letters and is read in NFC;
     `W0031` warns of a name whose letters are of several scripts and
     `W0032` of a name that reads as another in scope, by generated tables
     (`tools/unicode_tables.py`, Unicode 15.1) that `check.sh` keeps
     fresh. Stage 1 decides it (RULES.md, R136; D-048).

## R137. The library's planned helpers: checked arithmetic of every integer, `or_else`, `try_map`, `try_collect` (decision 803)

**The rule.** The prelude has, appended at its end so that no function
before them moves:

- `checked_add`, `checked_sub` and `checked_mul` of each sized integer
  (`u8` to `u128`, `i8` to `i128`), `None` where the operation would trap,
  and `checked_pow(exp: int)` of `int` and of each sized integer
  (`requires exp >= 0`, as `num.checked_pow`); `u8.checked_add` was
  `E0304`.
- `Result.or_else(f)`: the value, or `f` of the error, 05's form
  `fallible().or_else(e -> recover(e))`, beside `or(default)`.
- `Iter.try_map(f)`: `f` of each item in a list, or the first error `f`
  gives (`Result[list[U], E]`), and `list[Result[T, E]].try_collect()`:
  the values, or the first error. 05 names them for chains that can fail,
  without their forms; an adapter's `map` is `Iterator`'s, whose trait a
  method appended at the prelude's end cannot join, so a chain collects
  first, `xs.iter().map(f).collect().try_collect()`, or maps with
  `try_map`.

Each has a doc example, which the Rust front end checks and runs (`u8`'s,
`i8`'s and `int`'s for the families).

**Why.** The manual's findings "as planned" under "Errors" (chapters 8 and
16) and "The standard library" (the checked methods of the sized
integers, `checked_pow` `std.num`'s function only).

**In stage 1.** Nothing but `std/prelude.sisu`.

**What it changes.** The conformance suite's `std/doc_examples.select` is
blessed again (six doc examples more, 191 where there were 185); no other
output of the corpus.

**Decision 803**, for docs/architecture.md:

803. The sized integers have `checked_add`, `checked_sub`, `checked_mul`
     and `checked_pow`, and `int` has `checked_pow`; `Result.or_else`,
     `Iter.try_map` and `list[Result[T, E]].try_collect` are the prelude's.
     Stage 1 decides it (RULES.md, R137).

## R138. A body with a syntax error reports the names outside its broken statements (decision 804)

**The rule.** A function's or a closure's body with an `ERROR` node is
resolved, where nothing in it was reported: a statement with an `ERROR`
node outside its blocks is passed over, none of its names reported, and
every name written in such a statement or in an `ERROR` node of the body
is taken as one the broken code may declare (`Resolver.broken`), never
reported as unknown where a later statement uses it, so the cascade stays
out. A header with a syntax error still leaves its body unreported, and
no diagnostic is kept on a line with a syntax error, as before. `go f()`
and `nil` in one body are now both reported (`E0117`, then `E0200` with
the fix `None`).

**Why.** G4's finding under "Diagnostics without fixes": a body with a
parse error reported nothing of its names, so the `nil` of a body that
also holds `go f()` went unreported until the parse was fixed.

**In stage 1.** `resolve` (`function`, the body of a closure or lambda,
`broken_in`, `error_names`, `statement`, `use_name`).

**What it changes.** The case `cases/findings15/names_despite_parse.sisu`
(its `.diag`), and the names of any body with a syntax error, as listed
below.
The name class of three of stage 1's cases of syntax errors reports the
names they do not know outside their broken statements, which `check`
then drops, each on a line with a syntax error (`ready`, `xs`, `open`).

- Differs: `compiler/stage1/cases/findings15/names_despite_parse.sisu`: names, ids, human, fmt-check, doc, doc-output
- Differs: `compiler/stage1/cases/expected_block_labels.sisu`: names
- Differs: `compiler/stage1/cases/expression_mistakes.sisu`: names
- Differs: `compiler/stage1/cases/statement_mistakes.sisu`: names

**Decision 804**, for docs/architecture.md:

804. A body with a syntax error has its names resolved but for the
     statements whose shape the error broke: none of their names is
     reported, and none of the names they write is reported later. Stage 1
     decides it (RULES.md, R138).

## R139. `sisu doc`'s values of an example: decided against for this round (decision 805)

**The rule.** Nothing changes: a doc example is shown as written.

**Decided against**, for this round, with what it would take: 10 has
`sisu doc` render each expression statement of an example with its value.
`sisu doc` is a `pure` function of the syntax trees, the resolved names
and the inferred error sets (decision 258); a value needs the example
type-checked and lowered as a function of its module's program, as `sisu
test` builds it, and then run. The compile-time evaluator (`interp`) runs
the `pure` code compile time needs, while examples call `print` and take
capabilities (`Fs.read_only`, `System.fake`), so most values would be left
as text; and the pages of every program with such an example would differ
from stage 0's `sisu doc`, which renders none, for the doc classes to
list. It is the worksheet's machinery (`sir --worksheet`'s `__ws_show`),
run by the interpreter, that would give each line its value, which makes
it a round of its own with the driver's `sisu doc`.

**What it changes.** Nothing.

**Decision 805**, for docs/architecture.md:

805. `sisu doc`'s rendering of an example's values waits for the example
     to be built and run as a worksheet is, which the compile-time
     evaluator cannot do for examples that print or take capabilities.
     Stage 1 decides it (RULES.md, R139).

## R140. Every type has its `Debug`, a public one too (decision 806)

**The rule.** Nothing changes in stage 1: every type, public or private,
has its `Debug` form without asking (the structural rendering), as both
front ends have always given it. A type that keeps its fields from a
reader writes its own `impl Debug`.

**Why.** Chapter 7's finding under "Types and traits": 10 makes `Debug`
opt-in for public types. The worksheet, the REPL, `sisu test`'s values
beside a failing `assert`, error return traces and the debugger show
values by their `Debug` form; a public type without one would show
nothing there, and a library's types are the ones a user's tests and
sessions meet most. The cost the opt-in avoided, a public type's fields
in output, is the impl a type that hides them writes.

**The sentence of 10** ("Debugging and profiling") the coordinator carries:
"A `Debug` rendering exists for every type (derived by default for private
types, opt-in for public ones)." becomes "A `Debug` rendering exists for
every type, derived by default for public and private types alike; a type
that hides its fields writes its own `impl Debug`."

**What it changes.** Nothing.

**Decision 806**, for docs/architecture.md:

806. Every type has a `Debug` form without asking, a public one too, and a
     type that hides its fields writes its own impl; 10's opt-in for
     public types is withdrawn. Stage 1 decides it (RULES.md, R140).

## R141. The merge's differences: the conformance suite's `runtime_single`, and a batch's runtime profile (decision 807)

**The rule.** Nothing a command prints changes. Stage 0's thirteenth round
added the conformance case `packages/runtime_single.pkg` (decision 783), a
copy of stage 1's own `cases/packages/runtime_single.pkg`, which R122
lists: the Rust front end refuses the manifest's `runtime` key (`W0005`)
and `target.runtime_profile` (`E0204`), so its SIR, diagnostics and pages
differ from stage 0's as those of stage 1's copy do. `check.sh`'s
`behaviour` class lowered it with `sir-batch`, whose one base is of the
profile `full` for every program of the batch (`main.runtime_profile`
reads the profile from a command's own path), so the batch's program
printed `profile: full` where the case expects `profile: single`; the
class now lowers a package whose manifest names another profile by the
single command, `sir PATH`, as the driver lowers it, and the program
prints `profile: single`, interpreted and natively. A batch's base for
each profile is open.

**Why.** The merge of stage 0's thirteenth round brought the case, and
`check.sh` found what it changes.

**In stage 1.** `check.sh` (the `behaviour` class).

**What it changes.** Nothing.

- Differs: `compiler/conformance/cases/packages/runtime_single.pkg`: sir, ids, human, doc, doc-output

**Decision 807**, for docs/architecture.md:

807. The conformance suite's `packages/runtime_single.pkg` differs from
     stage 0 as stage 1's own copy of it does (R122), and `check.sh` lowers
     a package of a runtime profile other than `full` by the single command,
     whose base is of its profile, where a batch's base is `full`. Stage 1
     decides it (RULES.md, R141).

## R142. `sir --tests --modules` names each test's module, for `sisu test src/MODULE.sisu` (decision 840)

**The rule.** `sir --tests --modules` writes, on each `test` line and each
`bench` line, after the function the line names, the word `in` and the
file of the module the test or benchmark is declared in, quoted as the
`source` lines quote it (the path from the package's root, or the file's
name for a program of one file):

```text
test "the main module's test" test#0 in "src/main.sisu"
test "heat/levels/eighty is hot" test#1 in "src/heat.sisu" tags "slow"
bench "heat/a level" bench#0 in "src/heat.sisu"
```

The file is the `source` of the function the line names, so a suite's
tests are their suite's module's and a parametrised test's cases its
own. Without `--modules` nothing changes: `sir --tests` writes the lines
it wrote, byte for byte, and `--modules` without `--tests` changes
nothing. Stage 1's version is 0.12.0.

**What the driver may rely on**, from `sisu1 0.12.0` on: `sisu1 sir
--tests --modules [OPTIONS] [PATH]` takes every option `sir --tests`
takes and prints the same SIR, but that each `test` and `bench` line
between the `entry` line and the first declaration has ` in "FILE"` right
after its function's name, before `tags`; FILE is quoted and escaped as a
`source` line's text is, and it is one of the `source` lines' texts. A
driver that runs one module's tests keeps the `test` lines whose FILE is
that module's path, relative to the package's root, and takes the words
out (or reads them) before stage 0's SIR reader reads the text, since
that reader drops a `test` line's `tags` after an unknown word.

**Why the option.** Decision 828 asks for each test's module in `sir
--tests`. The reader of stage 0 in this tree (`sisu-sir`'s `read.rs`)
refuses a line it does not know (`unknown line`) and reads the `tags` of a
`test` line only right after its function, so neither a line of its own
(`test FN FILE`, the lean) nor a word on the `test` line can be written by
default without breaking every `sisu test` of a program with tagged tests
that the driver runs through stage 1 today. Under the option, the SIR the
driver already reads is unchanged, and the driver asks for the words when
`sisu1 --version` says 0.12.0 and it has learnt to read them: the
reader's `test` arm takes ` in "FILE"` before `tags` (a line of the
reader's code), as the stage 2 reader would if it read tests.

**In stage 1.** `sir.write` (`write_as`, `module_word`), `main` (`sir
--tests --modules`, `report_with`, `VERSION`), `sisu.toml` and
`sisu.lock`; `check.sh` (`NAME.sir-modules`: the lines before the first
declaration of what `sir --tests --modules` prints of a case, a package
case's as `NAME.pkg.sir-modules`).

**What it changes.** No output of the corpus. The package case
`cases/packages/test_modules.pkg` (two modules, a suite with a tag, a
benchmark; its `.out` and its `.pkg.sir-modules`).

**Decision 840**, for docs/architecture.md:

840. Stage 1's `sir --tests --modules` writes ` in "FILE"` on each `test`
     and `bench` line after its function, FILE the source of the module
     that declares it, so that the driver can run one module's tests
     (`sisu test src/MODULE.sisu`, decision 828); `sir --tests` alone is
     unchanged, since stage 0's SIR reader drops the tags after a word it
     does not know. `sisu1 --version` says 0.12.0. Stage 1 decides it
     (RULES.md, R142).

## R143. Reflection on a type the prelude declares, and one error for what reflection cannot take (decision 841)

**The rule.**

- A `reflect` query in a function that takes no type parameter may name
  `int`, `f64`, `bool`, `char` and `str` (and, as before, `String`, the
  sized integers and the program's own types and traits):
  `comptime reflect.name_of(int)` is `"int"`, `reflect.is_int(int)` is
  `True`, `reflect.doc_of(int)` is empty. They were `E0900` ("`reflect` in
  this function is not implemented") and an `E0200` for `int` in
  `<main>/main.sisu`, the instance's module, which the program has no file
  for, with the note on converting numbers.
- A query whose argument compile time knows no type by, a run-time value
  (`reflect.name_of(degrees)`) or a name of nothing
  (`reflect.fields_of(Sauna)`), is one `E0609` at the function, whose
  reason says what reflection takes: "`reflect.name_of` takes a type (a
  type parameter, or a type or trait in scope), and compile time knows no
  type `degrees`", labelled at the argument in the program's own file. It
  was `E0900` with an `E0600` or an `E0200` in `<main>/main.sisu`.

**Why.** R12's finding under "Metaprogramming" (the manual's fourteenth
round): one diagnostic, in the right file, that says what reflection
takes; through a `comptime def` that takes the type it already worked.

**In stage 1.** `typeck.instances` (`named_type`: the primitive types),
`typeck.template` (`replacements`: a query whose argument is not known,
`query_of`).

**What it changes.** The cases `cases/findings16/reflect_builtin.sisu`
(its `.out`) and `cases/findings16/reflect_unknown.sisu` (its `.diag`).

- Differs: `compiler/stage1/cases/findings16/reflect_builtin.sisu`: ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/findings16/reflect_unknown.sisu`: names, ids, human, doc, doc-output

**Decision 841**, for docs/architecture.md:

841. A `reflect` query may name the prelude's scalar types (`int`, `f64`,
     `bool`, `char`, `str`) in any function, and a query given what
     compile time knows no type by is one `E0609` at its argument, saying
     what reflection takes, where it was `E0900` with an `E0200` in the
     instance's own module. Stage 1 decides it (RULES.md, R143).

## R144. A derive that `E0607` refuses is that one error (decision 842)

**The rule.** A trait whose `comptime def derive` is not the form `E0607`
names is that error, at the template, alone: a `@derive` of it is not
`E0243` ("cannot be derived", though the trait declares a derive), and a
type that names it in a `@derive` has no `E0351` for an associated
function (`Point.columns()`) or `E0304` for a method (`p.width()`) that a
derive of the trait would have written; their arguments are still
checked. The trait's own functions called through it, and a type that
does not name the derive, are reported as before.

**Why.** R12's finding under "Metaprogramming": one `E0607` mistake made
three errors and more, one at each `@derive` and each use.

**In stage 1.** `typeck.program` (`Program.refused_derives`,
`refused_derived`), `typeck.collect` (the trait's `E0607`),
`typeck.instances` (a `@derive` of a refused template), `typeck.call`
(`refused_derive_of`, at `E0351` and `E0304`).

**What it changes.** The case `cases/findings16/derive_refused_once.sisu`
(its `.diag`).

- Differs: `compiler/stage1/cases/findings16/derive_refused_once.sisu`: ids, human, doc, doc-output

**Decision 842**, for docs/architecture.md:

842. A derive template that `E0607` refuses is the one error: a `@derive`
     of its trait is not `E0243`, and the functions and methods the derive
     would have written are not `E0351` or `E0304` where a type that
     derives it uses them. Stage 1 decides it (RULES.md, R144).

## R145. `reflect.functions_of(module)`: a module's functions with their names, attributes and signatures (decision 843)

**The rule.** `reflect.functions_of(m)`, where `m` is a module of the
program that the code imports by name (`from web import routes`), is the
list of `m`'s functions in the order they are declared: its `def`s but
its `comptime def`s and generic functions, and of a module other than the
one the code is in, its public ones. Each is a descriptor with `name`,
`params` (each with `name`, `index`, `mode` and `ty`, as a method's) and
`ret` (its result type, a type compile time knows by its name), and the
queries `reflect.name_of(f)`, `reflect.doc_of(f)` (its `##` comment) and
`reflect.attributes_of(f)` (its attributes, each with `name` and `args`,
the text of each argument as written, quotes and all); in run-time code
the descriptor is the function itself, written `routes.NAME`, so
`f()` calls it and a list of them is a table of function values. A type
given to `functions_of` is `E0609` ("takes a module the code imports"),
as is a query of a type given a function. A module of the standard
library is none (`E0609` at the argument, R143).

```sisu
let table: list[(str, () -> String)] = [
    (a.args[0], f)
    for f in reflect.functions_of(routes)
    for a in reflect.attributes_of(f)
    if a.name == "route"
]
```

The table above is unfolded at compile time into the list of pairs it
holds; written `comptime [...]`, the list would be evaluated by the
compile-time evaluator after lowering, which keeps no function values,
so it is written without `comptime`.

**Why.** R14's finding under "Metaprogramming", and 08's table ("a
framework collects routed functions by scanning its own package at
compile time"): `reflect.attributes_of` reached a struct's, an enum's and
an error set's attributes only, and nothing listed a module's functions.

**In stage 1.** `typeck.template` (`Kind.Module`, `MethodShape.attrs` and
`doc`, `Value.Function`, `functions_of`, a function's fields and
queries, its literal), `typeck.instances` (`named_module`, beside
`named_type` and `named_trait` for an instance and a value).

**What it changes.** The package case
`cases/packages/routed_functions.pkg` (its `.out`), whose SIR and
diagnostics differ from stage 0's, which has no `functions_of`, and the
case `cases/findings16/functions_of_refused.sisu` (its `.diag`).

- Differs: `compiler/stage1/cases/packages/routed_functions.pkg`: types, sir, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/findings16/functions_of_refused.sisu`: ids, human, doc, doc-output

**Decision 843**, for docs/architecture.md:

843. `reflect.functions_of(m)` gives the functions of a module the code
     imports, each with its name, parameters, result type, attributes and
     comment, and stands in run-time code for the function itself, so a
     framework collects its routed functions into a table at compile time
     (08). Stage 1 decides it (RULES.md, R145).

## R146. `try_collect` after a `map`, and a borrowed index by a `str` through `dict.index_str` (decision 844)

**The rule.**

- The prelude has, appended at its end, `try_collect` of
  `Map[I, Result[T, E]]`, the result of an iterator's `map` to results:
  `xs.iter().map(f).try_collect()` is the values or the first error, after
  which no item is taken, where R137 had the chain collect first. 11 puts
  `try_collect` on the iterators; `Map` is the adapter of a chain that can
  fail, and the other adapters' items (`Filter`'s, `Take`'s) are their
  inner iterator's `Item`, which an inherent impl cannot ask to be a
  `Result` (a bound on an associated type's value is not in the language),
  so they collect first or map with `try_map` as before.
- A `dict[String, V]`'s index by a `str` that is only read through a
  reference, a method's receiver (`d[k].byte_len()`, `d[k].to_uppercase()`)
  or a `str` view of it, lowers to `dict.index_str` and reads the value
  through its reference, as R134's read of the index does: no `String` is
  made for the key, and a missing key's panic stands at the index. An
  index changed through it (`d[k].n += 1`, a `mut` receiver) keeps the
  `String` form.

`Map.try_collect` has a doc example, which `sisu test --std` runs (with
stage 1 as the front end; the Rust front end finds no `try_collect` on
`Map` and names no item for the impl, so its own run keeps 191).

**Why.** R137's and R134's open items.

**In stage 1.** `std/prelude.sisu` (appended), `lower.exprs`
(`borrow_operand`, a shared `Borrow`, `str_operand` and
`Coercion.StrView` read their place by `read_place_of`).

**What it changes.** The conformance suite's `std/doc_examples.select`
is blessed again (one doc example more, 192 where there were 191). The
cases `cases/findings16/try_collect_adapter.sisu`
(its `.out`) and `cases/findings16/str_index_borrowed.sisu` (its `.out`,
`.err` and `.status`), and the SIR of a program that borrows a `dict`'s
index by a `str` (none of `sir.list`'s, whose outputs are the same file
for file).

- Differs: `compiler/stage1/cases/findings16/str_index_borrowed.sisu`: ids, human, doc, doc-output

**Decision 844**, for docs/architecture.md:

844. `Map[I, Result[T, E]]` has `try_collect`, appended to the prelude,
     and a `dict`'s index by a `str` that is only borrowed (a method's
     receiver, a `str` view) reads through `dict.index_str` with no
     `String` made. Stage 1 decides it (RULES.md, R146).

## R147. The resolver's and the trait check's cost: the derives found among their trait's impls (decision 845)

**The rule.** Nothing a command prints changes; the SIR of every program
is the same file for file. What changes is how the resolver does its
work:

- `check_derives` looks for a derived impl among the impls of its trait
  (`Program.impls_of`), in their order, where it walked every impl of the
  program for each derive and compared two module names' prefixes first:
  quadratic in the base's 508 impls.
- `resolve_impl` pushes an impl onto its trait's list in place, where it
  copied the list and put it back for each impl.
- `check_unsafe_method` reads the two methods' `unsafe` flags through
  references, and copies their signatures only for the lint's message.

**Measured** (callgrind, natively, a cached `sisu1 check --format=ids` of
a two-line program, the cache written under valgrind first; this round's
stage 1 before and after these changes, built by the same stage 0): the
command 3.392 G instructions before, 3.341 G after (−1.5 %); the trait
check (`check_traits`) 150.2 M before, 102.1 M after (−32 %), its
`check_derives` 87.7 M before and 62.7 M after (the rest of it the
supertraits' `implements`, 46 M); resolving (`resolve_pending`) 383.7 M
before and 381.1 M after. What remains of resolving is spread over the
base's 1,323 functions' and 622 impls' signatures (155,000 instructions a
function: its types resolved at 37,000 each through `def_as_type` and
`Types.adt`'s key, `unfolds`' walk of its body, 27 M in all, and the copy
of its `FnInfo` for `Def.set_kind`), with no site above 3 % of the
command; the parse of the base's modules (the parser 0.70 G, the lexer
0.53 G, the AST builder 0.54 G, and 0.21 G for the cached text of the
derived impls) and the copies and drops of the values throughout are
the rest. R125's figures, 0.37 G and 0.15 G, were this round's 0.38 G and 0.15 G
before the change; the base grew by the prelude's appended helpers.

**Why.** Item 5 of the round (R125 and R135): what dominated after the
lexer and the parser.

**In stage 1.** `typeck.instances` (`check_derives`), `typeck.collect`
(`resolve_impl`, `check_unsafe_method`).

**What it changes.** No output of the corpus.

**Decision 845**, for docs/architecture.md:

845. Stage 1's trait check finds a derived impl among its trait's impls
     and the resolver keeps each trait's impls in place: a two-line
     `check` takes 3.34 G instructions where it took 3.39 G, the trait
     check 0.10 G where it took 0.15 G, resolving 0.38 G as before, its
     cost spread over the base's signatures. Stage 1 decides it
     (RULES.md, R147).

## R148. Hangul composition in names; `@derive(Into, From)` decided against (decision 846)

**The rule.**

- A name is read in NFC with Hangul composed and decomposed by rule
  (UAX 15, "Hangul"), which R136's tables leave out because Python's
  `unicodedata` gives syllables no decomposition: a leading consonant and
  a vowel jamo are the LV syllable, and an LV syllable and a trailing
  consonant the LVT one, so a name written `ᄀ` `ᅡ` `ᆨ` and the name `각`
  are one name. The arithmetic is twenty lines of `text.names`
  (`hangul_composition`, `decompose_into`); the generated tables are
  unchanged.
- **Decided against**, with the text that rules it out: `@derive(Into,
  From)`. 03 ("Type aliases and newtypes") names it as what makes a
  one-field struct cheaper, and 11 lists `Into`/`From` among the
  prelude's traits, but neither says what the derive writes, and the
  prelude has `Into[T]` only, documented as "a conversion that loses
  nothing", each sized integer into every wider one: there is no `From`
  to derive, and a newtype's `into` its field is not that trait's promise
  either way (a `Port` refined to `1..65535` into `int` loses nothing; an
  `int` into a `Port` can). A derive's template is `comptime def
  derive(T: type) -> impl Trait for T` (08, decision 251, `E0607`), with
  no argument of the trait, while `Into[F]` needs `F`, the field's type,
  computed at compile time; that is a new template form. With `@delegate`
  (which forwards a trait to the field) and the constructor and `.value`
  of the field, a newtype has both directions now. It waits for the
  owner to define `From` and what the two derives write.

**Why.** The manual's narrowed "missing beside 03" item, and R136's
Hangul left out.

**In stage 1.** `text.names` (`hangul_composition`, `decompose_into`, the
test "a name is read in NFC").

**What it changes.** The case `cases/findings16/hangul_names.sisu` (its
`.out`), whose two spellings stage 0 reads as two names.

- Differs: `compiler/stage1/cases/findings16/hangul_names.sisu`: ast, names, ids, human, doc, doc-output

**Decision 846**, for docs/architecture.md:

846. A name's NFC composes Hangul jamo into syllables by UAX 15's rule;
     `@derive(Into, From)` waits for the owner, since the prelude has no
     `From`, `Into` promises a lossless conversion, and a derive's
     template takes no trait argument. Stage 1 decides it (RULES.md,
     R148).

## R149. The merge's differences: the conformance suite's `unsafe/cstring` (decision 847)

**The rule.** Nothing a command prints changes. Stage 0's fourteenth round
added the conformance case `unsafe/cstring.sisu` (decision 823), a
`CString` handed to C's `strlen`, whose `unsafe` code and `extern` block
stage 1 checks as R2 to R9 have it and the Rust front end does not, so its
AST, names, SIR, diagnostics and pages differ from stage 0's as those of
the other cases of `unsafe/` do (R4 lists them). What it does is what its
`.out` says, from stage 1's SIR (`behaviour`).

**Why.** The merge of stage 0's fourteenth round brought the case, and
`check.sh` found what it changes.

**What it changes.** Nothing.

- Differs: `compiler/conformance/cases/unsafe/cstring.sisu`: ast, names, sir, ids, human, doc, doc-output

**Decision 847**, for docs/architecture.md:

847. The conformance suite's `unsafe/cstring` differs from stage 0's front
     end as the other cases of `unsafe/` do (R4). Stage 1 decides it
     (RULES.md, R149).

## R150. Stage 1's own builds in the release profile: `sisu1` and `check.sh` run the driver's (decision 880)

**The rule.** Nothing a command prints changes. `sisu1` no longer builds
`target/stage1/sisu1` itself: it runs the executable the driver builds
for every `sisu` command (decision 860: `sisu build --native --release
--contracts=all`, with the stamp `target/stage1/profile`), and when the
stamp does not name what is there (the profile, the executable's size and
its time), or a source of stage 1 is newer, it asks the driver for it by
starting a command that stage 1 serves (`sisu tokens` of a small std
file, with `SISU_STAGE1` and `SISU_FRONT_END` unset), under the driver's
lock. `SISU_STAGE1_PROFILE=debug` asks for the debug build, as the driver
takes it. So `sisu1` and `sisu` no longer build over each other's
executable: the driver rebuilt the debug build `sisu1` made at its next
command, and `sisu1` rebuilt the driver's whenever a source changed.

`check.sh --native` runs that build, copied into its work directory, for
every batch and the class `build`, and builds stage1-A, the debug build
of the fixed point (`sisu build --native`), once for the classes `self`
and `fixed`, which the fixed point is about (`stage1-B` is built from
stage1-A's SIR, and must be stage1-A byte for byte). Without `--native`,
`--self` builds stage1-A as before.

**Measured** (`check.sh --native --names --types --sir`, the round's
stage 1 before and after this change, on four cores that the other
agents' builds and conformance runs kept at a load of 10 to 30, so a
class's time varies by a third from run to run): the whole run took
1990 s before and 955 s after, `types` 10.6 s and 4.9 s, `corpus-types`
12.0 s and 5.8 s, `sir` 32.6 s and 17.2 s, `corpus-sir` 36.3 s and
19.1 s, `corpus-check` 23.7 s and 17.3 s, `doc` 58.2 s and 30.3 s,
`fmt-joined` 18.5 s and 10.3 s, `build` 34.2 s and 10.4 s, and `self-sir`,
the debug build in both, 75.0 s and 66.9 s; but the load fell between the
two runs (the behaviour class's runs, stage 0's, took 61.8 s and 29.9 s),
so most of that halving is the machine's. One build against the other on
the same, quieter machine (a load of 2 to 3, each batch run in turn,
twice): `types-batch` of `types.list`'s 549 programs takes 7.3 s and
7.9 s in the debug build, 5.2 s and 5.9 s in the release one (about a
quarter less), and `sir-batch` of `sir.list`'s 549 22.9 s and 23.9 s
against 17.5 s and 18.6 s (about a fifth less). The executable is 28.7 MB
where the debug one is 48.8 MB, and the driver builds it in its own time
(decision 860: 43 s where the debug one took 39 s), once a change of
stage 1's sources, rather than `check.sh` building one of its own; the
run builds stage1-A once, as it did.

**Why.** Decision 860's notes for stage 1, and R135's open item: the
release profile parses the prelude in 373 M instructions where the debug
one takes 988 M.

**In stage 1.** `sisu1` (`stamped`, the driver's build), `check.sh`
(`stage1_exe` the driver's build, `debug_stage1` and `stage1_debug` for
`self` and `fixed`).

**What it changes.** No output of the corpus; the batches' times.

**Decision 880**, for docs/architecture.md:

880. `stage1/sisu1` runs the stage 1 the driver builds (decision 860, the
     release profile with every contract checked, the stamp
     `target/stage1/profile`) and asks the driver to build it, so the two
     no longer build over each other's executable; `check.sh --native`
     runs it for its batches, and the classes `self` and `fixed` a debug
     stage1-A of their own. Stage 1 decides it (RULES.md, R150).

## R151. Compile-time tables of function values, and an attribute's arguments as values (decision 881)

**The rule.**

- A compile-time value that is a function, `routes.index` of
  `reflect.functions_of` (R145) or a function named in compile-time
  code, is kept in the result of a `comptime` expression evaluated after
  lowering as the constant that names it (`const fn
  sauna_web.routes.index`), so a table of routed functions written
  `comptime [...]` is evaluated by the compiler, where it was beyond the
  slice ("a failed `comptime` expression in a file of the standard
  library or one the compiler wrote"). A closure is still not kept; its
  error is the evaluator's, at the expression.
- `reflect.attributes_of`'s attributes give their arguments as values
  (08: "attributes are data"): `a.args` holds each argument's value, a
  string literal without interpolation its text without its quotes and
  escapes (`@route("/x")` gives `/x`), an integer literal its number,
  `True` and `False` booleans, and any other argument, a name among them
  (`@index(hour)`), the text written; a keyword argument gives the value
  after its `=`, and the new `a.keys` gives each argument's keyword, empty
  for one given by position (`@index(hour, unique=True)`: `args` is
  `[hour, True]`, `keys` is `["", "unique"]`). Where R145 gave the text as
  written, quotes and all, a framework compares a route with `p == path`
  rather than `"\"{path}\""`.
- An element of `a.args` or `a.keys` at a compile-time index,
  `a.args[0]`, is that value in the unfolded code, whatever the types of
  the others (`["bookings", 3][0]` was written before, which no list
  type holds).

**Why.** The sixteenth round's two open items (R145) and 08's framework
example; the manual's reference (R14, "Attributes are data") shows the
text form, which this changes.

**In stage 1.** `interp` (`scalar`: `Value.Fn` is `Const.Fn`),
`typeck.template` (`argument_value`, `argument_key`, `keyword_split`,
the fields `args` and `keys` of an attribute, `is_static` and `eval` of
an index of them).

**What it changes.** The conformance suite's `reflect/queries.out` is
blessed again (`route[/boxes, GET]` where it printed
`route["/boxes", method="GET"]`); `cases/packages/routed_functions.pkg`
compares a route by its text (its `.out` is the same); the new package
case `cases/packages/routed_comptime.pkg` (its `.out`), the table written
`comptime [...]`; and `cases/findings17/attribute_values.sisu` (its
`.out`).

- Differs: `compiler/conformance/cases/reflect/queries.sisu`: sir, contracts-off, contracts-preconditions
- Differs: `compiler/stage1/cases/packages/routed_comptime.pkg`: types, sir, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/findings17/attribute_values.sisu`: ids, human, doc, doc-output

**Decision 881**, for docs/architecture.md:

881. A `comptime` expression's function values are kept as the constants
     that name them, so a table of routed functions is evaluated at
     compile time, and an attribute's arguments are values (a string
     literal's text, an integer, a boolean, a name's text) with their
     keywords in `keys`, where they were their source text. Stage 1
     decides it (RULES.md, R151).

## R152. A missing key's panic stands at the index, whatever the key (decision 882)

**The rule.** A `dict`'s entry read by value inside a larger expression
(an argument, an operand, an interpolation) is read into a temporary at
the index's own position, so that a missing key's panic names the index
(`at main (missing_key.sisu:4:11)`), as R134's `dict.index_str` does for
a `str` key of `String` keys, where it named the start of the statement
that read it (`4:5`). A read that is the whole value of its statement
(`let x = d[k]`) was at the index already; a borrowed entry and an entry
changed through it are unchanged. What a program prints is the same; its
SIR reads such an entry one statement earlier.

**Why.** The manual's fifteenth round ("A missing key of a `dict[String,
V]` indexed by a string literal ... panics at the start of its
statement"), whose example is a `dict` of `str` keys: one place for both,
the index.

**In stage 1.** `lower.exprs` (`operand`, `keyed`).

**What it changes.** The conformance suite's `run/missing_key.err` is
blessed again (`4:11`); the case `cases/findings17/missing_key_literal.sisu`
(its `.out`, `.err` and `.status`); and the SIR of every program that reads
a `dict`'s entry inside an expression, listed below.

- Differs: `compiler/conformance/cases/run/collections.sisu`: sir
- Differs: `compiler/conformance/cases/run/list_keys.sisu`: sir
- Differs: `compiler/conformance/cases/run/missing_key.sisu`: sir
- Differs: `compiler/conformance/cases/std/modules.sisu`: sir
- Differs: `compiler/conformance/cases/wrong/dict_changed_while_iterating.sisu`: sir
- Differs: `compiler/stage1/cases/comptime/evaluator.sisu`: sir
- Differs: `compiler/stage1/cases/comptime/shared.sisu`: sir

**Decision 882**, for docs/architecture.md:

882. A `dict`'s entry read inside an expression is read at the index, so
     a missing key's panic names the index whether the key is a literal,
     a `str` or a `String`. Stage 1 decides it (RULES.md, R152).

## R153. The small types of 03: `ptrsize`, `uptrsize`, `Sized`, a list literal as an `Array`, a `Url` literal (decision 883)

**The rule.** Each is additive, appended where its item lives:

- `ptrsize` and `uptrsize`, the prelude's aliases of `i64` and `u64` (03:
  "`isize`/`usize` equivalents exist as `ptrsize`/`uptrsize` for FFI
  only"), the width of an address on every target Sisu builds for now.
- `Sized`, a marker trait of the prelude, which every type implements but
  a `dyn` one (03: "`dyn Display` is a type, unsized, used behind `ref` or
  `Box`"); since `E0395` keeps a `dyn` out of every other place, a bound
  `T: Sized` is met by every type argument, and an `unsafe impl` of it
  is `E0986` ("is never asserted"), as one of `Copy` or `Escapable` is.
- A list literal of `N` elements where an `Array[T, N]` is expected is
  that array (03, "Type inference": `let xs: Array[int, 3] = [1, 2, 3]`):
  lowered as `Array.filled` of the first element and the others stored in
  their places, as `a[i] = v` stores them. A literal of another length
  stays `E0300` (expected `Array[int, 3]`, found `list[int]`), as does
  `[]`.
- `FromLiteral[str] for Url` in `std.net` (11's `FromLiteral` for `Path`
  and `Url`): a string literal where a `Url` is expected is
  `Url.parse` of it; one that is not of the form stops the program with
  `expect`'s panic, naming the form.

**Why.** The manual's narrowed "missing beside 03" item (R2, chapters 5,
6 and 24).

**In stage 1.** `std/prelude.sisu` and `std/net.sisu` (appended),
`typeck.program` (`implements`: `Sized`), `typeck.collect` (`wk.sized`),
`typeck.expr` (the list literal), `lower.exprs` (`into` of the literal).

**What it changes.** The case `cases/findings17/small_types.sisu` (its
`.out`), which stage 0 refuses, and the conformance suite's
`std/doc_examples.select`, blessed again: `Url.from_literal`'s doc example
runs where `std.net`'s examples do, 194 where there were 192.

- Differs: `compiler/stage1/cases/findings17/small_types.sisu`: ids, human, doc, doc-output

**Decision 883**, for docs/architecture.md:

883. The prelude has `ptrsize` and `uptrsize` (`i64` and `u64` on every
     target now) and the marker trait `Sized`, which every type but a
     `dyn` one implements; a list literal of `N` elements where an
     `Array[T, N]` is expected is that array; and `std.net` has
     `FromLiteral[str] for Url`. Stage 1 decides it (RULES.md, R153).

## R154. `try_collect` after every adapter: decided against, with the spelling that works (decision 884)

**The rule.** Nothing changes. **Decided against**: `try_collect` on
`Filter`, `Take`, `Skip` and the other adapters whose item is their
inner iterator's `Item`. The forms tried, and why each fails:

- an inherent impl on `Filter[I]` for an `I` whose `Item` is a
  `Result[T, E]`: no bound says what an associated type is (03 has
  `where T: Bound`, never `I.Item == Result[T, E]`);
- a provided method of `Iterator` with a bound on `Item`, as `sum` has
  `where Item: Num`, through a trait of results with the value's and the
  error's types as its associated types (`Item.Value`): the checker
  resolves no associated type of an associated type (`E0201` "cannot find
  a type named `I.Item.Value`"), and a trait's associated type given as a
  `Result`'s parameter in its impl is `E0234` against its declaration, so
  both are type-system work of their own, and the method would sit in
  the middle of the prelude, which stays appended;
- impls for each adapter over each inner iterator (`Filter[Map[I,
  Result[T, E]]]`, `Take[...]`, ...) cover one depth of chain each and
  multiply with the adapters.

The spelling that works ends the chain in the `map` that makes the
results, `xs.iter().filter(p).map(f).try_collect()`, or in `map(r -> r)`
after the adapter, `xs.iter().map(f).take(2).map(r -> r).try_collect()`,
which is `Map[I, Result[T, E]]`'s `try_collect` (R146); `try_map` takes
the function itself.

**Why.** R146's open item.

**What it changes.** The case `cases/findings17/try_collect_spelling.sisu`
(its `.out`), both spellings.

**Decision 884**, for docs/architecture.md:

884. `try_collect` after an adapter whose item is its inner iterator's is
     decided against until the type system has a bound on an associated
     type's value or associated types of associated types; the chain ends
     in its `map`, or in `map(r -> r)`. Stage 1 decides it (RULES.md,
     R154).

## R155. Compile-time types: the first step, planned (decision 885)

**The rule.** Nothing changes; this round records the plan. 08 says that
reflection "exposes types ... as values" and that compile-time code
"can only produce values and types for use in expressions", never a
declaration; D-065 puts `@diagnostic_name(f)` after Q-27's type
construction, and `sisu eval` (08, "Configuration files") renders a
module's `const` items, which needs no new type but the evaluator over
the whole module. Today a `comptime def pick(wide: bool) -> type` is
`E0201` ("cannot find a type named `type`"), its `return i64` `E0350`, and
`let x: pick(True) = 3` a syntax error (`E0117`), since a type position
takes a path with its arguments only.

The first step is a type a `comptime` function returns:

1. `type` as the result type of a `comptime def`, whose body the
   template engine runs, as it runs a derive's (`Value.Type`, with
   `if`, `match` and the `reflect` queries on its parameters), returning a
   type a name, a type argument or a `reflect` query gives
   (`reflect.elem_of(T)`, `i64`);
2. a type position that is such a call, `comptime pick(True)`, written as
   a constant argument is (`Array[T, comptime n()]`): the parser's type
   grammar takes `comptime` and a call (02 has no such form, so the
   syntax is the owner's to confirm), the resolver evaluates it before the
   declaration that holds it is collected, and the checker sees the type
   it names, with an error at the call when it names none;
3. `@diagnostic_name(f)`, whose `f` is then a `comptime` function from a
   type to the name its diagnostics use.

The round left it: each of the three is a change of the parser, the
resolver's order and the template engine at once, larger than a round's
item, and its syntax waits for the owner.

**Why.** Item 6 of the round.

**What it changes.** Nothing.

**Decision 885**, for docs/architecture.md:

885. Compile-time code that makes types starts with a `comptime def ... ->
     type` that the template engine runs and a type position written
     `comptime f(args)`, whose syntax is the owner's; `@diagnostic_name`
     follows it, and `E0900` stays until then. Stage 1 decides it
     (RULES.md, R155).

## R156. `Decimal`'s `/` giving a `Ratio`: decided against for the round (decision 886)

**The rule.** Nothing changes. **Decided against**, with the text: 03
says of `Decimal` that "division yields a `Ratio`", and 11 that it
returns one, but `std.num`'s `Ratio` holds an `int` numerator and
denominator, so the quotient of two decimals of more digits than an
`int` holds has no `Ratio` (`Decimal.to_ratio()` gives `None` there),
and an `impl Div for Decimal` with `Output = Ratio` would panic where 03
promises an exact value. It waits for a `Ratio` of `BigInt`s, additive in
`std.num`; until then `a / b` between `Decimal`s is `E0310` with the note
`type.decimal-quotient`, which names `a.div(b, places, mode)` and
`a.to_ratio()` (R108).

**Why.** Item 7 of the round.

**What it changes.** Nothing.

**Decision 886**, for docs/architecture.md:

886. `Decimal`'s `/` giving a `Ratio` waits for a `Ratio` of `BigInt`s,
     since today's `Ratio` of `int`s cannot hold every quotient 03
     promises exactly. Stage 1 decides it (RULES.md, R156).

## R157. The merge's differences: the conformance suite's `unsafe/export_by_pointer` (decision 887)

**The rule.** Nothing a command prints changes. Stage 0's fifteenth round
added the conformance case `unsafe/export_by_pointer.sisu` (decision
864), a raw pointer to a `@repr(c)` struct handed to the program's own
export, whose `unsafe` code and `extern` block stage 1 checks as R2 to R9
have it and the Rust front end does not, so its AST, names, SIR,
diagnostics and pages differ from stage 0's as those of the other cases
of `unsafe/` do (R4, R149). What it does is what its `.out` says, from
stage 1's SIR (`behaviour`).

**Why.** The merge of stage 0's fifteenth round brought the case, and
`check.sh` found what it changes.

**What it changes.** Nothing.

- Differs: `compiler/conformance/cases/unsafe/export_by_pointer.sisu`: ast, names, sir, ids, human, doc, doc-output

**Decision 887**, for docs/architecture.md:

887. The conformance suite's `unsafe/export_by_pointer` differs from stage
     0's front end as the other cases of `unsafe/` do (R4). Stage 1
     decides it (RULES.md, R157).

## R158. `sir --repl-serve` serves every command that checks a program, in its request's working directory (decision 920)

**The rule.** The server of R112 answers, besides `sir --repl`, `sir
--worksheet` and `parse --bindings`, a request that is any `sir`,
`check`, `parse`, `fmt`, `lint`, `doc` or `tokens` command with its
arguments, so that a command pays its own check and not the standard
library's base. What the driver may rely on from 0.13.0:

- **A request** is one line of standard input, the command's arguments
  separated by tabs, as they would follow `sisu1` on a command line
  (without `--home` and `--catalog`, which make the request misused),
  with `--dir DIR` anywhere among them: the working directory the
  command runs in, absolute or relative to the server's. Without `--dir`
  it runs in the server's own. `--target-runtime` is the request's, as a
  command line's. `sir --repl` and `sir --worksheet` keep `--dir` as R95
  has it (where their program stands) and run in the server's directory,
  as before.
- **An answer** is R112's frame on standard error: `answer STATUS OUT
  ERR`, then exactly `OUT` bytes of what the command alone prints on
  standard output and `ERR` bytes of what it prints on standard error.
  `STATUS` is the status the executable `target/stage1/sisu1` gives when
  run alone, 0 or 1, and a failed command's standard error ends with the
  run time's last line: `error: Failed`, `error: Usage(...)`, `error:
  Exit(status=N)` (`fmt`, `doc`: the status `sisu1` exits with is `N`,
  as the script reads it) or `error: ` and an I/O error's form, without
  the lines of the source the run time adds after one. A command the
  server does not serve (`build`, `run`, `test`, `audit`, a batch,
  `sir --repl-serve` itself) is answered as misused.
- **The base** is made once and made again before a request whose key
  differs from the one it was made with: each std text's size and time,
  the catalog's, and the runtime and runtime profile the request builds
  for (R116, R122). A request never gets a base of other texts.
- **A stale server**: before each request the server compares its
  running executable's size and time, and those of
  `HOME/target/stage1/sisu1` when it was that build at its start, with
  what they were; when either changed (the driver built stage 1 again), it
  writes the line `stale` on standard error, answers nothing more and
  ends with status 0. The driver starts a new server and sends the
  request again.
- Files are read and written as the command alone reads and writes them,
  relative to `--dir`: a command's lockfile, `fmt`'s files and `doc
  --output`'s pages are written as when it runs alone.

`sisu1 --version` is 0.13.0: the driver may rely on these requests. One
server per working directory, and its lifetime, are the driver's to
choose (stage 0's next round); since a request names its directory, one
server serves many.

```text
$ printf 'check\ttwo.sisu\t--dir\t/work\n' | sisu1 sir --repl-serve
answer 0 0 0
```

**Measured** (a `check` of a two-line program, natively, the release
build, a load of 10 to 12 on four cores): 0.22 s of CPU time as a process
(the least of ten), 0.009 s per request served (the median of 50 after
the first, which makes the base, 0.44 s). Under callgrind a process takes
1.95 G instructions, and the server 1.75 G for the base once and 0.095 G
for each request after it (eleven requests, 3.02 G in all).

**Why.** Item 1 of the round: every command but the REPL's paid the base,
about 3.2 G instructions, where an input served cost 0.04 s (R112).

**In stage 1.** `main` (`run_command`, which `main` runs with an `Out`
that prints at once and the server with one that holds what the command
prints; `Served`, the base a command uses when it is given one;
`sir_with`, `check_with`, `lint_with`, `doc_with`, the commands after
their base; `repl_serve`, `serve_one`, `served_key`, `remade`,
`ServerStamp`, `request_system`, `InDir`, `held_frame`, `command_words`,
`served_command`), and `std/cap.sisu`'s appended `system_with` (a
`System` with another file system and environment, which only the
standard library may assemble), which `request_system` uses for an
environment whose arguments are the request's and whose `PWD` is its
directory.

**What it changes.** No output of the corpus; `main`'s unit tests of the
held frame, the request's words and the directory's paths. Each of 24
requests (`check`, `sir`, `tokens`, `parse`, `fmt`, `lint`, `doc` and `doc
--std`, of files and packages, with errors, a missing file, an unknown
command and option, `--target-runtime native`) was answered byte for byte
as the process printed it, and a server whose std text, catalog or
executable changed between requests made its base again or answered
`stale`.

**Decision 920**, for docs/architecture.md:

920. `sisu1 sir --repl-serve` serves any `sir`, `check`, `parse`, `fmt`,
     `lint`, `doc` or `tokens` request, run in the working directory its
     `--dir DIR` names, with R112's frame and the status the command gives
     alone; the base is made again when a std text, the catalog or the
     request's runtime changed, and a server whose executable was built
     again answers `stale` and ends. A `check` of a two-line program takes
     0.009 s served where it takes 0.22 s as a process. `sisu1 --version`
     is 0.13.0. Stage 1 decides it (RULES.md, R158).

## R159. An instance's syntax error is the template's, once; E0609 of a function without type parameters (decision 921)

**The rule.**

- A syntax error (a `lex.` or `parse.` diagnostic) in the instance of a
  program's own template is reported once, by the template's file: when
  the template's text has a syntax error itself (its header or body is
  broken), the instances' copies of it are that error and are not
  reported again; otherwise the error stands at the template's statement
  that wrote the instance's line (R131's origins), in the template's
  file, with its message and parameters. It was reported again for each
  unrolled copy, in `<show[Point]>/main.sisu`, a file the program does
  not have.
- Compile-time code of a function that takes no type parameters, which is
  made once and is no instance (`main`, with `reflect.name_of(degrees)`),
  that stops is the new `E0616`, `comptime.function-failed`: "the
  compile-time code of `main` stopped: ...", with the label
  `comptime.function-here` ("the compile-time code, where it stopped"),
  where it was `E0609`, "making `main`" and "the template", the words of
  an instance of a generic function, which stay `E0609`'s for those (the
  catalog gives each code one message, so the plain function's words have
  a code of their own).

**Why.** Two findings of the manual's sixteenth round (R12, R17).

**In stage 1.** `typeck` (the copies of a syntax error placed at the
template, `copied`), `typeck.collect` (`Resolver.broken_instances`),
`typeck.instances` (the instance's template broken, and the words of a
plain function), `diag` and the catalog (`E0616`,
`comptime.function-failed`, and `comptime.function-here`, after
`E0615`).

**What it changes.** The cases `cases/findings18/instance_syntax.sisu` and
`cases/findings18/plain_function_reflect.sisu` (their `.diag`), and the
conformance suite's `reflect/methods_rejected.diag`, blessed again: its
`count` and `unknown`, functions without type parameters, are `E0616`;
and the `.diag` files of `cases/findings16/functions_of_refused.sisu` and
`cases/findings16/reflect_unknown.sisu`, blessed again for the same.

- Differs: `compiler/stage1/cases/findings18/instance_syntax.sisu`: ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/findings18/plain_function_reflect.sisu`: ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/reflect/methods_rejected.sisu`: ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/expression_mistakes.sisu`: ids, human, doc, doc-output

**Decision 921**, for docs/architecture.md:

921. A syntax error in an instance of the program's template is reported
     once, in the template's file, and compile-time code of a function
     without type parameters that stops is `E0616`, in a plain function's
     words, where it was `E0609`'s instance words. Stage 1 decides it
     (RULES.md, R159).

## R160. A closure in a `comptime` result is kept when what it captures is constants (decision 922)

**The rule.** A closure in the result of a `comptime` expression is kept
as the closure of its function, as R151 keeps a function value as the
constant that names it, when it captures nothing or only numbers, text,
booleans and characters (an `own` closure's copies): `comptime [("add",
(x: int) -> x + 1)]` and `comptime scaled(3)` of a function that returns
`own (x: int) -> x * k` are evaluated by the compiler, their SIR the
closure written with its constant captures (`closure
scaled#closure0(const 3)`). A closure that captures more is `E0604`
naming what (`a closure that captures a tuple`, `... a binding` for a
reference into the compile-time code's frame), with the new note
`comptime.kept-functions`, which says what a result keeps of functions;
it was `E0604` "a function made at compile time cannot be kept", which
R151 had made untrue of a named function.

**Decided against**: keeping a closure that captures a tuple, a struct or
a list: the closure's captures are built at run time from constants only
where their types are written, which the evaluator's value does not
carry; a capture of a binding is a reference into a frame that is gone.

**Why.** Item 3 of the round, R151's open item.

**In stage 1.** `interp` (`Baker.build` of `Value.Closure`,
`captured_kind`), `main` (the note), `diag` and the catalog
(`comptime.kept-functions`, in `E0604`'s block).

**What it changes.** The cases `cases/findings18/kept_closures.sisu` (its
`.out`) and `cases/findings18/closure_captures_tuple.sisu` (its `.diag`),
and the conformance suite's `comptime/evaluation_errors.diag`, blessed
again: its `ADD`, `comptime adder()` of a closure that captures nothing,
is kept, and its `E0604` is gone.

- Differs: `compiler/stage1/cases/findings18/kept_closures.sisu`: ids, human
- Differs: `compiler/stage1/cases/findings18/closure_captures_tuple.sisu`: ids, human
- Differs: `compiler/conformance/cases/comptime/evaluation_errors.sisu`: sir, ids, human

**Decision 922**, for docs/architecture.md:

922. A `comptime` result keeps a closure that captures nothing or only
     constants, as the closure of its function; one that captures more is
     `E0604` with a note on what a result keeps of functions. Stage 1
     decides it (RULES.md, R160).

## R161. The attributes of fields, variants and methods in reflection (decision 923)

**The rule.** A field's and a variant's attributes are `f.attributes` and
`v.attributes`, a list of attributes as `reflect.attributes_of(T)` gives a
type's (each with `name`, `args` and `keys`, R151), for a type's fields
(`reflect.fields_of`), a derive's target's fields and variants, and
`reflect.variants_of`; a method's are `reflect.attributes_of(m)` (and
`m.attributes`) for `reflect.methods_of(T)`, as a module's function's
are (R145); and `x.[m]` of a method is `x`'s method, so that `s.[m]()`
calls the method a template routes to. A variant's field has none (the
parser takes no attribute there). The standard library's declarations
give none, as its types' do.

```sisu
def route[T](s: T, path: str) -> String:
    comptime for m in reflect.methods_of(T):
        comptime for a in reflect.attributes_of(m):
            comptime if a.name == "route":
                if path == a.args[0]:
                    return s.[m]()
    return "no route for {path}"
```

**Why.** Item 4 of the round, the manual's narrowed framework finding
(R14, R12, chapter 25).

**In stage 1.** `typeck.program` (`FieldInfo.attrs`, `VariantInfo.attrs`),
`typeck.collect` (`attr_data`, `fn_attrs`), `typeck.instances` (the
shapes' attributes, `method_shape`), `typeck.template` (`FieldShape`,
`VariantShape`, `Target.field_attrs` and `variant_attrs`, the values'
`attrs`, `attr_values`, `member_attrs`, `x.[m]`).

**What it changes.** The case `cases/findings18/member_attributes.sisu`
(its `.out`), columns named by a field's `@column`, variants labelled by
`@label` and handlers routed by a method's `@route`; and the conformance
suite's `comptime/derive_errors.diag`, blessed again, whose reason says
"`x.[f]` needs a field or a method". Stage 1's AST has carried a field's
attributes since R40, which the Rust front end's does not.

- Differs: `compiler/stage1/cases/findings18/member_attributes.sisu`: ast, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/comptime/derive_errors.sisu`: ids, human, doc, doc-output

**Decision 923**, for docs/architecture.md:

923. Reflection gives a field's and a variant's attributes as
     `f.attributes`, a method's as `reflect.attributes_of(m)`, and `x.[m]`
     names a method, so a template routes by a method's attribute. Stage 1
     decides it (RULES.md, R161).

## R162. The normalisation forms and graphemes in `std.string` (decision 924)

**The rule.** `std.string` (11: "Unicode scalar and grapheme iteration,
case mapping, normalisation forms") has, appended:

- the trait `Normalize`, implemented for `str`, with `nfc`, `nfd`, `nfkc`
  and `nfkd` (Unicode's UAX #15): the full canonical decomposition (the
  Hangul syllables computed), the compatibility one for `nfkd` and
  `nfkc`, the marks put in canonical order, and the canonical composition
  for `nfc` and `nfkc`, with the composition exclusions;
- the trait `Graphemes`, implemented for `str`, with `graphemes()` (03:
  "`s.graphemes()`" is an iterator), by the rules the tables carry: a base
  with the marks after it (`Mn`, `Mc`, `Me`) and the variation selectors,
  emoji modifiers and tag characters, `\r\n`, a syllable written in jamo,
  a pair of regional indicators, and a sequence joined by U+200D are each
  one grapheme, and a control character is one of its own. The full rules
  of UAX #29 would add `Prepend` (a character kept with what follows it),
  the Indic conjuncts joined across a virama (GB9c), a join after U+200D
  only between pictographs (`Extended_Pictographic`, GB11) and the few
  characters that extend a grapheme without being marks
  (`Other_Grapheme_Extend`, such as U+200C).

A method of `str` from `std.string` is called after `from std.string
import Normalize` (or `Graphemes`), as the trait's methods are; the
prelude is unchanged, so no program that does not import them parses the
tables. The tables (the canonical decompositions, the compositions, the
compatibility decompositions and their code points, the combining classes
and the marks, 170 KB of text) are written at the end of
`std/string.sisu` by `tools/unicode_tables.py` from Unicode 15.1.0, below
the line it rewrites, and `check.sh` fails when they are stale, as it does
for R136's. R136's tables, which hold only the decompositions of letters
and marks that a name needs, are the names' still.

**Why.** Item 5 of the round, the manual's "missing beside 03" item.

**In stage 1.** `std/string.sisu` (appended), `tools/unicode_tables.py`
(`std_tables`, `std_text`).

**What it changes.** The cases `cases/findings18/normalisation.sisu` and
`cases/findings18/graphemes.sisu` (their `.out`); the conformance suite's
`doc/std_pages.doc`, blessed again with `std.string`'s new items, and
`std/doc_examples.select`, with the examples of `Normalize` and
`Graphemes` (198 examples with R163's, where there were 194). Stage 0's
formatter removes `from std.string import Normalize` as unused, since no
name of it is written, where the methods need it; stage 1's resolves the
methods and keeps it, so the two cases differ in what `fmt` leaves.

- Differs: `compiler/stage1/cases/findings18/normalisation.sisu`: fmt, fmt-check
- Differs: `compiler/stage1/cases/findings18/graphemes.sisu`: fmt, fmt-check The four forms of
18,464 texts (every code point with a decomposition or a combining
class, every 97th Hangul syllable and 400 bases with marks in random
order) are Python's `unicodedata.normalize`'s, code point for code point.

**Decision 924**, for docs/architecture.md:

924. `std.string` has the normalisation forms `nfc`, `nfd`, `nfkc` and
     `nfkd` (`Normalize`) and `graphemes()` (`Graphemes`) for `str`, from
     generated Unicode tables; the graphemes keep a base's marks, `\r\n`,
     Hangul syllables, flags and joined sequences, short of UAX #29's
     `Prepend`, conjunct and pictograph rules. Stage 1 decides it
     (RULES.md, R162).

## R163. `BigRatio`, and `Decimal`'s `/` (decision 925)

**The rule.** 03 has `Ratio` as "exact fractions" and says that a
`Decimal`'s "division yields a `Ratio`"; `std.num`'s `Ratio` holds
`int`s, so a fraction of large parts overflows. Appended to `std.num`:
`BigRatio`, an exact fraction of `BigInt`s in lowest terms with a positive
denominator (`of`, `whole`, `of_ratio`, `is_whole`, `floor`, `to_ratio`,
`to_f64`, the operators `+`, `-`, `*`, `/` and unary `-`, the order,
`Display` and `Debug`), `Decimal.to_big_ratio`, and `impl Div for
Decimal` with `Output = BigRatio`: `a / b` of `Decimal`s is their exact
quotient at any size (`1.5 / 0.4` is `15/4`), and `a.div(b, places,
mode)` still rounds it to a `Decimal`. `Ratio` keeps its `int` parts,
since changing them would not be additive; the quotient is a `BigRatio`
rather than a `Ratio`, which the owner may merge (a `Ratio` of any size
would be one type). R156's `E0310` with the note `type.decimal-quotient`
no longer arises.

**Why.** Item 6 of the round, R156's open item.

**In stage 1.** `std/num.sisu` (appended).

**What it changes.** The case `cases/findings18/big_ratio.sisu` (its
`.out`), the conformance suite's `std/doc_examples.select`, blessed
again with the examples of `BigRatio` and `Decimal.to_big_ratio`, and
`cases/numbers/refusals.diag`, blessed again: its `Decimal` quotient is no
`E0310`.

**Decision 925**, for docs/architecture.md:

925. `std.num` has `BigRatio`, an exact fraction of `BigInt`s, and a
     `Decimal`'s `/` gives one, exactly; `Ratio` keeps its `int` parts.
     Stage 1 decides it (RULES.md, R163).

## R164. `query`: the language server's questions, answered by stage 1 and served (decision 960)

**The rule.** `sisu1 query KIND FILE [LINE COL] [--text PATH]` answers
the questions the driver's language server (`sisu lsp`, `lsp.rs` and
`assist.rs`) answers today from the Rust front end, so that the driver can
answer them from stage 1, and `sir --repl-serve` serves it as it serves
`check` (R158). What the driver may rely on from 0.14.0:

- **The program.** FILE is checked in the program that holds it, as
  `check` checks it, with its tests: the package of the nearest
  `sisu.toml` at or above its directory when FILE is one of that
  package's modules, else the file alone (with the dependencies its
  inline metadata names). With `--text PATH`, FILE's text is PATH's (the
  editor's buffer, which the driver writes to a file), everywhere FILE's
  would be read; the other modules are read from disk. A query writes no
  file, a lockfile neither.
- **A position** is LINE and COL, 1-based, the column counting
  characters, as `check --format=ids` writes them (a column past its
  line's last character is past its line break: the next line's start). In an answer a position is `LINE:COL` and
  a range `LINE:COL-LINE:COL`, in the file the line names. PATH in an
  answer is absolute.
- **An answer** is what the command prints on standard output: one line
  per item, its fields separated by tabs, each line ending in a line
  break, nothing when there is nothing to say (status 0 either way). A
  field of text has a backslash, a tab and a line break written `\\`, `\t`
  and `\n`, so that no field holds a tab or a line break. A file that
  cannot be read is the run time's I/O error (status 1), and a request
  that is not one of these (a kind, a missing or extra word, a LINE or COL
  that is not a number) is a usage error, as any command's.
- `query diagnostics FILE`: what `check` reports in FILE, in `check`'s
  order (the ownership checker's and the failed `comptime` expressions'
  among them, as `check` adds them): `RANGE\tCODE\tID\tSEVERITY\tMESSAGE`,
  the severity `error`, `warning` or `note`, and the message as the
  catalog renders it, which the human form prints after the code.
- `query symbols FILE`: FILE's declarations, from its syntax tree alone,
  so a file with a syntax error has all its whole items:
  `KIND\tNAME\tRANGE\tNAME_RANGE\tDEPTH`, in the order written, each
  member (a method, a field, a variant, an associated constant) after its
  item with depth 1, the items with depth 0. The kinds are `function`,
  `method`, `struct`, `enum`, `error`, `trait`, `impl` (named by its
  header, `impl Display for Point`), `const`, `type`, `test` (named by its
  string), `field` and `variant`. RANGE ends at the item's last character
  that is not a blank.
- `query hover FILE LINE COL`: at most one line,
  `RANGE\tSIGNATURE\tNOTES\tDOC`: the range of the name or expression
  there, and its signature: a definition's as it is declared (`def
  norm(p: Point) -> int`, a method's `def Point.moved(self, d: int) ->
  Point`, a struct with its fields and an enum with its variants on the
  lines after its own, `const N: int`, `trait T`, `module m`, `type A =
  B`), a local's `let x: T` or `var x: T`, a field's `x: T`, or else the
  expression's type. NOTES is what the checker knows besides (a `T !`
  function's `error set (inferred): ...`), DOC the `##` comment written
  above the declaration (its lines without `## `), or a field's.
- `query definition FILE LINE COL`: at most one line, `PATH\tRANGE`, the
  declaration of what the position names: a definition of the program or
  of the standard library (`HOME/std/NAME.sisu`), a field, a local. A
  position that names no declaration, or one the compiler wrote, has none.
- `query references FILE LINE COL`: `PATH\tRANGE\tKIND`, KIND
  `declaration` for the declaration's name, first, then `use` for each
  name written so that it means the same declaration, by module and
  position: a local's in its body, a definition's in every module of the
  program's own packages (a type's name written in a type among them); a
  name the standard library declares has its declaration only.
- `query completions FILE LINE COL`: `KIND\tLABEL\tDETAIL\tSNIPPET`,
  sorted by label, each label once, those that start with the name being
  written before the position. After a `.`, the members of the value
  before it (the program checked again without the `.` and the name, so
  that the value is an expression of its own): its struct's fields
  (`field`), then its type's methods and those of its trait impls
  (`method`). Elsewhere the names that may be written there: the locals
  of the innermost body declared before the position (`variable`), the
  module's own names and those it imports, the prelude's, the variants of
  the program's own enums by their bare names (`variant`), and the
  keywords (`keyword`); a definition's KIND is `function`, `struct`,
  `enum`, `const`, `module`, `trait` or `type`. DETAIL is the signature
  hover gives; SNIPPET is the protocol's snippet that writes a call or a
  construction with a hole per required argument (`norm(${1:p})`,
  `Point(x=${1:x}, y=${2:y})`), empty for the others.
- Served, a query is a request like any (R158): `query\tKIND\tFILE\t...`
  with `--dir DIR` among its words, answered with R112's frame.

```text
$ sisu1 query hover point.sisu 27 14
27:13-27:17	def norm(p: Point) -> int		The distance from the origin, squared.
$ sisu1 query completions point.sisu 28 20
variable	far	let far: Point	
```

What the driver keeps: the protocol's JSON, the conversion of positions
to UTF-16 columns where the editor asks for them, the documents it holds
(written for `--text`), rename and code actions (from `references` and
`diagnostics`), and the commands stage 1 does not answer (selection
ranges, formatting, which `fmt` answers, inlay hints and inline values).

**Measured** (natively, the release build, while `check.sh` ran beside
it, a load of 5 to 8 on four cores): a hover of the 30-line program of
`cases/findings19/query_program.sisu` takes 0.22 s of CPU time as a
process, and 0.009 s served (the median of 50 after the first request,
which makes the base, 0.36 s); a completion 0.013 s served, a search of
references 0.012 s.

**Why.** Item 1 of the round: the Rust front end still answers `sisu
lsp`'s questions (port-debt.md, "The Rust front end's roles still to
replace"), and stage 1's server answers every other command.

**In stage 1.** `query` (new: `Target`, `target_at`, `expr_at`,
`hover`, `signature`, `doc_above`, `definition`, `declared`,
`references`, `in_type`, `symbols`, `members`, `scope`, `receiver_at`,
`with_prefix`, `field`), and `main` (`query_command`, `query_answer`,
`query_input`, `query_module`, `query_files`, `query_diagnostics`;
`query` among the served commands).

**What it changes.** No output of the corpus. The cases are asked through
the server by `check.sh`'s new class `served`, which sends each
`cases/*/*.requests` file's lines to one `sir --repl-serve` and compares
its answers with the `.answers` beside it, the compiler's directory
written `COMPILER` and each frame's lengths left out:
`cases/findings19/query_program.requests` (every kind of a program,
`query_program.sisu`, and of `query_broken.sisu`, a file with a statement
left at `p.` and a call left open, whose symbols, members and hover text
are answered; `--text` with `query_buffer.text`; a usage error and a
missing file) and `cases/findings19/query_modules.requests` (the package
`cases/packages/query_modules.pkg`, with its `.out`: a name imported from
another module is declared and used there).

**Decision 960**, for docs/architecture.md:

960. `sisu1 query KIND FILE [LINE COL] [--text PATH]` answers the
     language server's questions from stage 1's check of the program that
     holds FILE: `diagnostics`, `symbols`, `hover`, `definition`,
     `references` and `completions`, one line per item of tab-separated
     fields, positions as `check --format=ids` writes them; `sir
     --repl-serve` serves it, a hover in 0.009 s. `sisu1 --version` is
     0.14.0. Stage 1 decides it (RULES.md, R164).

## R165. `eval`: a configuration module's constants, rendered (decision 961)

**The rule.** `sisu1 eval FILE [--to json|toml|yaml]` (08,
"Configuration files"; 10's `sisu eval <file>`) checks the module FILE as
`check` checks a file, without its tests, and evaluates each of its `const`
items, in the order written, with the compile-time evaluator, as a
`comptime` expression is evaluated (a budget of 1,000,000 steps, nothing
reaching the world), then writes them in the form `--to` names, JSON
without it (08 names the three, and the option `--to`): one JSON object,
one TOML table of `NAME = value` lines (a struct an inline table) or one
YAML mapping in block style, by the constants' names. A struct is a
mapping of its fields, in order; an enum's variant its name, or with
fields a mapping of its name to them (their mapping, or the one value);
`Some(x)` is `x` and `None` nothing (`null`, which TOML lacks: a field or
a constant that holds nothing is left out there, and nothing in a list is
`E0617`); a list, a set and a tuple a sequence; a dict whose keys are text
or integers a mapping; text in double quotes, JSON's escapes. The types,
refinements and invariants the module declares are the schema: a
constant whose evaluation stops is the evaluation's error at the
constant's name (`E0602` for a panic, an invariant's among them, `E0603`,
`E0601`, `E0810`), and one that holds what the form cannot write (a
function, a reference, an infinite float in JSON) is the new `E0617`,
`eval.no-form`: "`SQUARE` holds a function, which JSON cannot write".
Diagnostics are printed as `check` prints them, on standard error; with an
error nothing is written on standard output and the status is 1. A
constant whose evaluation needs what stage 1's evaluator does not run is
beyond the slice, said on standard error, status 1. `sir --repl-serve`
serves `eval` as any command. What the driver may rely on from 0.14.0:
the command, its options, its output and its status.

```text
$ sisu1 eval deploy/prod.sisu --to toml
SERVICES = [{ name = "web", port = 8080, replicas = 3, tier = "Free" }]
REGION = "eu-north"
```

**Decided against**: rendering through each type's `Encode` (08 "through
`Encode`", leaning, D-063), which would make every type of a
configuration derive it; the value's own structure is what an `Encode`
derive writes of it, and `eval` reads it whether or not the type derives
one. And the brief's `--format`: 08 spells the option `--to`.

**Why.** Item 2 of the round: the driver answers `sisu eval` with
`D0003` today; its routing of `sisu eval` to stage 1 is stage 0's next
round.

**In stage 1.** `config` (new: `Form`, `Data`, `Unwritable`, `constants`,
`data_of`, `render`, `json`, `toml`, `yaml_entry`), `interp`
(`value_of`, and `variant_info` made public), `main` (`eval_command`,
`eval_with`, `no_form`, and `report_program`, the part of `report_with`
that leaves the program checked and evaluated for `eval`), `diag` and the
catalog (`E0617`, `eval.no-form`, after `E0616`).

**What it changes.** No output of the corpus. The cases, asked through
the server by `cases/findings19/config.requests` (class `served`):
`config_services.sisu` (each form, and JSON without `--to`),
`config_refused.sisu` (a failed invariant and a function) and
`config_maybe.sisu` (nothing in a list: JSON and YAML write `null`, TOML
refuses), and `--to xml`, a usage error.

**Decision 961**, for docs/architecture.md:

961. `sisu1 eval FILE [--to json|toml|yaml]` evaluates a module's `const`
     items with the compile-time evaluator and writes them as JSON (the
     default), TOML or YAML; a constant whose evaluation stops is that
     error at its name, and one the form cannot write is `E0617`. Stage 1
     decides it (RULES.md, R165).

## R166. A template's compile-time code calls `range`, `zip`, `reversed`, `sorted`, `min`, `max`, `sum` and text's methods (decision 962)

**The rule.** In a template's `comptime for` and `comptime if` (and
wherever its compile-time code is evaluated), besides `reflect`,
`enumerate` and `len`: `range(stop)`, `range(start, stop)` and
`range(start, stop, step)` of integers (a step of 0, or more than 100,000
numbers, stops the instance); `zip(a, b)` of two lists of one length
(PEP 618, as the prelude's); `reversed(xs)`; `sorted(xs)`, `min` and
`max` of one list or of two or more values, and `sum(xs)`, of integers or
of text; and the methods of text the compile-time evaluator has,
`to_lowercase`, `to_uppercase`, `to_string` and `byte_len`. So `comptime
for i in range(len(a.args))` unrolls over an attribute's arguments. A call
of anything else is `E0609` as before, whose reason now lists what can be
called: "only `reflect`, `enumerate`, `len`, `range`, `zip`, `reversed`,
`sorted`, `min`, `max`, `sum` and the methods `to_lowercase`,
`to_uppercase`, `to_string` and `byte_len` of text can be called at
compile time here".

**Why.** Item 3 of the round, the manual's seventeenth round
(manual/README.md: "`comptime for i in range(len(a.args))` in a template
is E0609").

**In stage 1.** `typeck.template` (`is_builtin`, `is_text_method`,
`builtin`, `text_method`, `RANGE_LIMIT`; `Unfold.is_static` and
`Unfold.eval` of a call).

**What it changes.** The cases `cases/findings19/template_builtins.sisu`
(its `.out`: an attribute's arguments numbered by `range(len(a.args))`,
fields paired by `zip` and `reversed`, names `sorted` and in capitals, a
`min`, a `max` and a `sum` in conditions, a descending `range`) and
`cases/findings19/template_builtins_refused.sisu` (its `.diag`: `abs`,
with `E0609`'s new list, and `range` with a step of 0); stage 1's AST of
the first has its fields' attributes (R40). And the conformance suite's
`comptime/derive_user.sisu`, whose SIR differs from stage 0's though not
what it does: its derive's `names.push(f.name.to_string())` is evaluated
when the impl is unfolded, `f.name` being known then, so the impl holds
the text `"x"` where it held `"x".to_string()`.

- Differs: `compiler/stage1/cases/findings19/template_builtins.sisu`: ast, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/findings19/template_builtins_refused.sisu`: ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/comptime/derive_user.sisu`: sir

**Decision 962**, for docs/architecture.md:

962. A template's compile-time code may call `range`, `zip`, `reversed`,
     `sorted`, `min`, `max` and `sum` of lists and numbers and the methods
     `to_lowercase`, `to_uppercase`, `to_string` and `byte_len` of text,
     besides `reflect`, `enumerate` and `len`; `E0609`'s reason lists them.
     Stage 1 decides it (RULES.md, R166).

## R167. UAX #29's `Prepend`, conjunct and pictograph rules: decided against for the round, with the table they need (decision 963)

**The rule.** R162's graphemes stay as they are. Python's `unicodedata`
(Unicode 15.1.0, which `tools/unicode_tables.py` reads) has no
`Grapheme_Cluster_Break`, `Indic_Conjunct_Break` or
`Extended_Pictographic` property, and what it has does not carry them:

- `Prepend` is fifteen ranges of characters (the Arabic number signs
  U+0600 to U+0605, U+06DD, U+070F, U+0890 and U+0891, U+08E2, and some
  Brahmic prefixed vowels and repha forms, U+0D4E, U+110BD, U+110CD, ...)
  of the general categories `Cf` and `Lo`, beside thousands of characters
  of the same categories that are not; the East Asian width says nothing
  of them.
- GB9c's conjuncts need `Indic_Conjunct_Break`: its `Linker` is six
  viramas (Bengali, Devanagari, Gujarati, Malayalam, Oriya, Telugu), which
  a list could name, but its `Consonant` is
  `Indic_Syllabic_Category=Consonant` of those scripts, which a
  character's name and category only approximate (an independent vowel is
  a `Lo` letter of the script too, and a dead consonant is not one).
- GB11's `Extended_Pictographic` holds unassigned code points reserved for
  emoji, which have no category to read, beside `So` symbols that are not
  pictographs.
- `Other_Grapheme_Extend` (U+200C among them) is `Cf` or `Mc`, beside
  others of those categories that do not extend.

A table from Unicode's own files would add them: `Prepend` from
`GraphemeBreakProperty.txt` (15 records), `Indic_Conjunct_Break` from
`DerivedCoreProperties.txt` (its `Linker` 6 code points, its `Consonant`
and `Extend` a few dozen records), and `Extended_Pictographic` from
`emoji-data.txt` (about eighty records): a few kilobytes of text at the
end of `std/string.sisu`, with the three rules in `breaks_between`.

**Decided against** for the round: the files are not in the repository
and the tool's one source is `unicodedata`; and the change is to
`std/string.sisu`'s rules, not an append, while stage 0's agent edits
`std/`. An approximation from the categories would split or join graphemes
wrongly for whole scripts.

**Why.** Item 4 of the round, R162's open item.

**In stage 1.** Nothing.

**What it changes.** Nothing.

**Decision 963**, for docs/architecture.md:

963. `graphemes()` keeps R162's rules: UAX #29's `Prepend`, conjunct
     (GB9c) and pictograph (GB11) rules need Unicode's
     `GraphemeBreakProperty.txt`, `DerivedCoreProperties.txt` and
     `emoji-data.txt`, which Python's `unicodedata` does not carry; they
     wait for those files. Stage 1 decides it (RULES.md, R167).

## R168. The merge's differences: the conformance suite's `std/slices` (decision 964)

**The rule.** Nothing a command prints changes. Stage 0's seventeenth
round added the conformance case `std/slices.sisu` (decision 940), a
`Slice` that views its list in place, which imports `as_ptr` from
`std.mem` to show that two slices of one list point to one place;
`std.mem` is stage 1's (R5), and the Rust front end answers its import
with `E0220`, so the case's names, SIR, diagnostics and pages differ from
stage 0's as those of `unsafe/cstring` do (R149). What it does is what its
`.out` says, from stage 1's SIR (`behaviour`). Stage 0's decision 942,
the driver's side of R158's server, asks nothing of stage 1.

**Why.** Item 5 of the round: the merge of stage 0's seventeenth round
brought the case, and `check.sh` found what it changes.

**What it changes.** Nothing.

- Differs: `compiler/conformance/cases/std/slices.sisu`: names, sir, ids, human, doc, doc-output

**Decision 964**, for docs/architecture.md:

964. The conformance suite's `std/slices` differs from stage 0's front
     end, which has no `std.mem`, as `unsafe/cstring` does (R149). Stage 1
     decides it (RULES.md, R168).

## R169. `query`'s other questions: rename, actions, inlay hints, selection ranges, formatting and holes, and the names of import lines (decision 1000)

**The rule.** `sisu1 query` (R164) answers the rest of what the driver's
language server answers from the Rust front end, so that `sisu lsp` can
answer every request from stage 1: rename, code actions, inlay hints,
selection ranges and formatting, and, for 10's "Holes", a file's typed
holes. What the driver may rely on from 0.15.0, besides R164's forms,
which keep their program, positions, fields, escapes, usage errors and
serving:

- `query rename FILE LINE COL NEW`: the edits that rename what the
  position names to NEW, one line per line of `references` at the same
  position (its declaration first, then its uses), `PATH\tRANGE\tNEW`;
  or one line `refused\tREASON`: `refused\tnot-a-name` for a NEW that is
  not a name (a keyword among them), `refused\tnot-renamable` where the
  position names nothing the program's own packages declare (a name of
  the standard library, a module, which a file names, or nothing). The
  driver turns the lines into one workspace edit and the refusals into
  its own messages (`LSP_NOT_A_NAME`, `LSP_CANNOT_RENAME`).
- `query actions FILE LINE COL`: the fixes of the diagnostics `check`
  reports in FILE whose range holds the position (its ends included), in
  `check`'s order, each a line `fix\tK\tTITLE\tRANGE\tCODE\tID` (K the
  fix's index among its diagnostic's, 0 the one to prefer; TITLE the fix
  as the human form writes it after `= fix: `; RANGE, CODE and ID the
  diagnostic's) followed by one line `edit\tRANGE\tTEXT` per edit, in
  FILE. A fix without edits is left out. The driver asks it for the
  start of the protocol's range.
- `query inlay FILE`: the hints of FILE, by position, `POSITION\tKIND\tLABEL`:
  `type`, `: T` after the name of a local whose declaration does not
  write its type (a `let`'s, a `var`'s, a pattern's, a loop's, a
  closure's parameter; not a function's parameter, nor a name starting
  with `_`); `errors`, the error set a `T !` function infers, `{A, B}`,
  after its return type; `mode`, `mut` or `own` before an argument given
  to a parameter of that mode without the word, as the Rust front end's
  `inlay_hints` gives them. The driver keeps those in the editor's range.
- `query selection FILE LINE COL`: the ranges of the syntax around the
  position, from the file's syntax tree alone (a file with a syntax error
  has them too), innermost first, a line `RANGE` each, a node's range
  without the blanks and line breaks it ends with, each range once, the
  file's last (stage 0's `selection_ranges`).
- `query format FILE`: what `fmt` would write, as edits of FILE's text
  (`--text` for the buffer): one line per run of changed lines,
  `RANGE\tTEXT`, the run's lines in the text as it is (a range that is
  empty where lines are only inserted) and the lines that replace them,
  from the longest common subsequence of the lines (stage 0's
  `changed_runs`, so `rangeFormatting` keeps the runs inside its range);
  nothing for a text `fmt` leaves as it is; `refused\tsyntax` for a text
  with a syntax error and `refused\tunstable` for one the formatter
  refuses as changed (`D0075`, `D0076` of `fmt`).
- `query holes FILE`: FILE's typed holes, by position,
  `RANGE\tNAME\tTYPE\tFUNCTION\tSCOPE\tFITS` (R174).
- **The names of import lines** (R164's open item): a name of a `from`
  line's imported names means the definition it imports (not the alias
  after `as`, which declares a name of its own), and a name of an import
  path the module it names, so `references` lists them, `rename` renames
  them, and `hover` and `definition` answer at them. A module is declared
  by its file: its `definition` is `PATH\t1:1-1:1`.

```text
$ sisu1 query actions query_assist.sisu 21 12
fix	0	write `mut` before the argument: `mut `	21:10-21:16	E0339	type.mut-argument-missing
edit	21:10-21:10	mut 
$ sisu1 query rename query_assist.sisu 20 9 def
refused	not-a-name
```

What the driver keeps: the protocol's JSON and its UTF-16 columns, the
documents it holds, `prepareRename` (the `references` line at the
position), the worksheets' inline values (`sir --worksheet`'s SIR, R95,
run by stage 0's interpreter) and `sisu.explainBorrow`.

**Why.** Item 1 of the round: decision 983 left selection ranges,
formatting, rename, code actions and inlay hints to the Rust front end,
and R164 left the names of import lines out of `references`.

**In stage 1.** `query` (`target_of`, `in_import`, `path_names`,
`module_by_path`, `module_def`, `import_meaning`, `is_name`, `rename`,
`inlay`, `position`, `selection`, `split_lines`, `changed_runs`,
`format_edits`; `references`, `hover` and `definition` through
`target_of`; `declared` of a module), and `main` (`query_command`'s
kinds, `query_format`, `query_actions`). `sisu1 --version` is 0.15.0.

**What it changes.** No output of the corpus. The cases, asked through
the server (class `served`): `cases/findings20/query_assist.requests`
(`query_assist.sisu`, with its `.diag`: its hints, the fixes at two diagnostics and at a
line without one, a selection, a rename, two refused, the formatting of
the file and of `query_assist_broken.text`, and a usage error) and
`cases/findings20/query_imports.requests` (the package
`cases/packages/query_modules.pkg`: the references of `Bed` and
`starting` with their import line, a rename from the import line, the
hover there, the module `beds` of the import path, its refused rename).
`cases/findings19/query_program.answers` is blessed again (the
definition of `print` is at column 9 of the prelude's line, after R175's
`pub`, and two completions no longer offer the prelude's `failing`), and `cases/findings19/query_modules.answers` (the references of
`Bed` hold its import line's name). `query_assist.sisu`'s diagnostics
differ from stage 0's by the fix `type.result-or` that stage 1 gives
`E0371`.

- Differs: `compiler/stage1/cases/findings20/query_assist.sisu`: ids, human, doc, doc-output

**Decision 1000**, for docs/architecture.md:

1000. `sisu1 query` also answers `rename FILE LINE COL NEW` (an edit per
      reference, or `refused` with why), `actions` (each fix of the
      diagnostics at the position with its edits), `inlay` (the type of a
      local declared without one, an inferred error set, `mut` and `own`
      at arguments), `selection` (the syntax ranges around a position,
      innermost first), `format` (`fmt`'s change as runs of lines) and
      `holes`, and a name in an import line is a reference of what it
      imports, so that `sisu lsp` can answer every request from stage 1.
      `sisu1 --version` is 0.15.0. Stage 1 decides it (RULES.md, R169).

## R170. `build --for-run --emit-sir`: the SIR `sisu run` runs, served (decision 1001)

**The rule.** `sisu1 build --for-run --emit-sir [--release]
[--contracts=LEVEL] [--locked] [--features NAMES]
[--no-default-features] [PATH]` is `build --native --for-run` up to the
point where the build hands its backend the SIR, and writes that SIR on
standard output instead of building it: the lockfile written as the
build writes it, the program's diagnostics on standard error in the
build's order (the front end's errors with `D0006`, a release build's
holes, the warnings, a failed `comptime` expression's `D0006`, the
ownership checker's `D0021`, and why a program is beyond the slice), and
the status the build gives (1 at an error). A program without `main`
writes nothing on standard output, with status 0, which the driver
answers as it answers a SIR without an entry. `--native` is implied;
`--emit-sir` without `--for-run` is a usage error, and `--output` has no
file to name. `sir --repl-serve` serves this request (R158's frame, the
words and `--dir` as any), and answers any other `build` as misused, as
before. What the driver may rely on from 0.15.0: the request, its
standard output, which is the text the backend run with `SISU_KEEP_SIR`
would write (decision 268), and its standard error and status, which are
`build --native --for-run`'s.

```text
$ printf 'build\t--for-run\t--emit-sir\thi.sisu\t--dir\t/work\n' | sisu1 sir --repl-serve
answer 0 205 0
```

**Why.** Item 7 of the round: decision 981 left `sisu run` paying the
base, because R158 answers a `build` as misused and the SIR comes from
the backend's file.

**In stage 1.** `main` (`build_command` and `build_program` print through
the command's `Out`, take the served base, and stop at the SIR with
`BuildHow.emit_sir`; the served check of the words).

**What it changes.** No output of the corpus. The case:
`cases/findings20/query_assist.requests` asks it of
`slice_of_slice.sisu` (its SIR), of `prelude_internal.sisu` (its errors
and `D0006`) and without `--for-run` (a usage error).

**Decision 1001**, for docs/architecture.md:

1001. `sisu1 build --for-run --emit-sir [PATH]` writes on standard output
      the SIR `build --native --for-run` would hand its backend, with that
      build's diagnostics, order and status, and `sir --repl-serve` serves
      it, so that the driver can serve `sisu run` (decision 981). Stage 1
      decides it (RULES.md, R170).

## R171. `W0022` counts a member's `attributes` as code that reads attributes (decision 1002)

**The rule.** A program whose compile-time code reads attributes is not
warned with `W0022` of an attribute Sisu does not read (D-046): it is
data that code may read. Such code is a call of `reflect.attributes_of`,
and now also a member's `attributes`, `f.attributes` of a field and
`v.attributes` of a variant (R161), written `.attributes` not followed by
a letter, a digit or `_`. The message is as it was.

**Why.** Item 3 of the round, the manual's eighteenth round: a program
that reads attributes through `f.attributes` alone was warned of each
attribute it reads, "it means nothing here" (R164's open item).

**In stage 1.** `resolve` (`reads_attributes`, which `resolve` and
`resolve_program` ask of each module's text).

**What it changes.** The case `cases/findings20/member_attributes_read.sisu`
(its `.out`: the columns and labels a template reads through
`f.attributes` and `v.attributes`), which stage 0 refuses with `E0609` (a
field has no `attributes` there, R161).

- Differs: `compiler/stage1/cases/findings20/member_attributes_read.sisu`: ast, ids, human, doc, doc-output

**Decision 1002**, for docs/architecture.md:

1002. A member's `attributes` (`f.attributes`, `v.attributes`) is code
      that reads attributes, as a call of `reflect.attributes_of` is, so
      `W0022` does not warn of the attributes of a program that reads them
      that way. Stage 1 decides it (RULES.md, R171).

## R172. A slice of a slice made in one expression borrows the list (decision 1003)

**The rule.** A shared reference to a `Slice` lends what the slice views,
not the slice: a `Slice` holds no element of its own (decision 940: it
views its list in place), so what a call's result borrows through such
a reference (a slice of the slice, an element) is what the slice
borrows. `let first_two = towels.as_slice()[:2]` borrows `towels`, not
the temporary first slice, and lives as long as the loan of `towels`;
`towels.as_slice()[1:][:2]` and `for t in towels.as_slice()[2:]` too.
Where the first slice is a local, nothing changes. 04's rules ask that a
borrow not outlive what it borrows; the temporary is not what the second
slice borrows, so they allow it. A slice of a slice still cannot outlive
the list (`E0407` when returned) nor let it change while it lives
(`E0405`).

**Why.** Item 4 of the round, the manual's eighteenth round (chapter 12:
"`towels.as_slice()[:2]` is E0406 when used on a later line").

**In stage 1.** `borrowck.loans` (`through_views`, `lends_view`, which
`transfer` gives `call_result` for a call's operands).

**What it changes.** The case `cases/findings20/slice_of_slice.sisu` (its
`.out`), which stage 0 refuses with three `E0406`.

- Differs: `compiler/stage1/cases/findings20/slice_of_slice.sisu`: ids, human

**Decision 1003**, for docs/architecture.md:

1003. What a call's result borrows through a shared reference to a
      `Slice` is what the slice borrows, since a slice views its list in
      place (decision 940): a slice of a slice made in one expression
      borrows the list, not the temporary first slice, and is not
      `E0406` when used on a later line. Stage 1 decides it (RULES.md,
      R172).

## R173. The compile-time evaluator runs the prelude's functions a constant reaches for (decision 1004)

**The rule.** The compile-time evaluator (`interp`, which `eval`, a
`comptime` expression and a constant's value use) runs, as stage 0's
interpreter runs them:

- `min` and `max` of two values (`@structural` in the prelude): numbers,
  characters, text, tuples and lists, the first of two equal ones; NaN is
  `R0017`'s panic. `max(xs)` and `min(xs)` of one collection go through
  the prelude's `max_of` and `min_of` (R35), whose bodies now run with a
  derived `Ord`'s `structural.cmp` and `list.remove`.
- `abs` of an `int` (the least one overflows, `R0003`'s `abs(N)`), an
  `f64` and an `f32`.
- `any`, `all`, `sorted` (stable) and the prelude's `iterable_list`, of a
  list, a set, a range or an iterator's items.
- `enumerate`, `zip` (`R0012` for lengths that differ), `zip_shortest`
  and `reversed`, as iterators of their items made up front, so that a
  comprehension or a `for` takes them.

A pass over the prelude's `@intrinsic` and `@structural` functions found
no other one that a `const` or `comptime` expression of the corpus, the
manual's examples or the proposal calls; `range`, `len` and `sum` the
evaluator ran already. A template's compile-time code keeps R166's list.

**Why.** Item 5 of the round, R165's open item: `max` and `min` in a
constant that `eval` evaluates were beyond the slice.

**In stage 1.** `interp` (`call_intrinsic`'s `min`, `max`, `abs`, `any`,
`all`, `iterable_list`, `sorted`, `reversed`, `enumerate`, `zip`,
`zip_shortest`, `structural.cmp` and `list.remove`; `items_of`;
`InPlace.ListRemove`).

**What it changes.** No output of the corpus. The cases, asked through the
server: `cases/findings20/config_prelude.requests` (`eval` of
`config_prelude.sisu`, a constant per group, in JSON and TOML, and of
`config_prelude_refused.sisu`, `abs` of the least `int` and a `zip` of
two lengths, each `E0602` at its constant); stage 0 refuses
`max([4, 9, 2])` with `E0308`, a `max` of one list being R35's.

- Differs: `compiler/stage1/cases/findings20/config_prelude.sisu`: ids, human, doc, doc-output

**Decision 1004**, for docs/architecture.md:

1004. The compile-time evaluator runs the prelude's `min`, `max`, `abs`,
      `any`, `all`, `sorted`, `enumerate`, `zip`, `zip_shortest` and
      `reversed` (and `max_of`, `min_of` through a derived `Ord`), as the
      interpreter does, so a configuration's constants may call them.
      Stage 1 decides it (RULES.md, R173).

## R174. `sisu check --holes`: each typed hole with its type and the bindings that fit it (decision 1005)

**The rule.** `sisu1 check --holes FILE` (10, "Holes"; D-061) checks the
program that holds FILE (its package's, or the file alone, as `query`
finds it, R164) with its tests, and prints on standard output each typed
hole of the program's own modules, by module and position:

```text
holes.sisu:18:20: the typed hole `todo("risky")` of type `int`, in `Task.padded`
  in scope: `self: ref Task`, `risky: bool`, `base: int`
  of its type: `base`
```

the hole as `R0013` names it, the type its place needs, the function it
is written in, the bindings of its body declared before it with their
types (`D0172`, "nothing is in scope", for none) and those whose type,
past a reference, is the hole's (`D0174`, "nothing in scope is of its
type", for none). The new messages are `D0170` to `D0174` (`cli.hole`,
`cli.hole-scope`, `cli.hole-scope-none`, `cli.hole-fits`,
`cli.hole-fits-none`), numbered past stage 0's last so that the two
rounds do not meet. A program with an error is reported as `check`
reports it, status 1; another option is `D0009`, no path or more than
one a usage error. `query holes FILE` answers the same, a line per hole
(R169). `sir --repl-serve` serves both. What the driver may rely on from
0.15.0: the command, its output and its status; the routing of `sisu
check --holes` (`D0009` today) is stage 0's next round.

**Decided against**, for the round, of 10's report: the definitions of
the types the hole mentions, the `requires` and `ensures` around it, the
tests that reach it with their values, the expressions that fit to depth
two and the JSON form (the query's fields are the language server's).
The tests' values need the tests run, which is `sisu test`'s; the rest
is a round's work of its own over this report.

**Why.** Item 6 of the round: the manual's "Tooling" finding lists `sisu
check --holes` as `D0009`.

**In stage 1.** `query` (`Hole`, `holes`, `hole_order`, `function_name`,
`holes_answer`), `main` (`holes_command`, `query holes`) and the
catalog.

**What it changes.** No output of the corpus. The case
`cases/findings20/holes.sisu`, asked through the server by
`cases/findings20/query_assist.requests` (`query holes`, `check --holes`,
and `check --holes` without a path).

**Decision 1005**, for docs/architecture.md:

1005. `sisu1 check --holes FILE` lists each typed hole of the program
      with its position, the type its place needs, its function, the
      bindings in scope before it and those of its type (`D0170` to
      `D0174`), and `query holes` answers the same; 10's types,
      contracts, tests and fits to depth two wait. Stage 1 decides it
      (RULES.md, R174).

## R175. The prelude's visible names: what it marks `pub` (decision 1006)

**The rule.** 09 gives the prelude "a fixed, documented list of names, the
same in every module", and 11's `std.core` row adds the traits, the error
sets, the capability and task types and the functions; the manual's
reference ("Names every module sees") lists them. The prelude now marks
those declarations `pub`, 142 of them: the builtin and library types, the
capabilities and their operations' traits and types (`FsOps`,
`Metadata`, `Timer`, ...), the traits and the types their methods name
(`Formatter` and `Spec` of `Display` and `Debug`, `Hasher` of `Hash`,
`Draws` of `Arbitrary`), the error sets, the task and channel types, and
the functions 11 lists with `todo`, `unreachable`, `channel`,
`unbounded_channel`, `embed`, `arbitrary`, `exhaustive` and `check`, and
`iterable_list`, which the conformance suite's `run/iterable_rest` calls
(decision 235). The 43 others are its own, for the standard library's code: the calendar's
and the clock's helpers (`days_in_month`, `days_from_civil`,
`civil_from_days`, `time_digits`), the iterators' (`iter_from`,
`iter_of`, `max_of`, `min_of`, `list.sort_by`'s
`merge_runs`, `max_by`'s `ordered_keys`, `is_continuation`, `within_ulps`), the
property tests' (`DrawState`, `Failure`, `Outcome`, `Panicked`, `catch`,
`case_seed`, `replay_case`, `record_case`, `run_case`, `failing`,
`without`, `replaced`, `simpler`, `shrink`, `next_prefix`, `shown`), the
adapters' structs (`Take`, `Skip`, `Enumerate`), the host's objects
(`HostFs` to `HostListener`) and the transcripts' world
(`TranscriptWorld`, `transcript_world`, `transcript_system`).

A program's module (the program's packages and its dependencies; not the
standard library's, nor the modules the compiler writes) sees the
prelude's `pub` names only. A name it writes that the prelude declares
without `pub` is the new `E0278`, `resolve.prelude-internal`:
"`days_in_month` is the prelude's own, for the standard library's code:
a program sees only the names it marks `pub` (09)", where it was found
before; where a std module has the helper as a `pub` item
(`PRELUDE_HELPERS`), the fix imports it from there
(`resolve.import-from-std`, "import it from `std.time`", an import
before the module's first line of code). No std module has one yet: a
`pub` `days_in_month` appended to `std.time` changes that module's page,
which the conformance suite's `doc/std_pages` holds, so it waits for the
suite's owner. A prelude that marks no declaration `pub` (the unit
tests' small ones) shows them all, as before. Completions (R164) offer
the prelude's `pub` names only, so `DrawState` is no longer offered.
Lookups the compiler makes in the prelude by name (`iter_from`, `max_of`
of `max(xs)`, `Lookup`, ...) are not a program's and are unchanged.

**Decided against**: hiding `Formatter`, `Spec` and `Hasher`, which the
finding names: `Display.fmt`, `Debug.fmt_debug` and `Hash.hash` take
them, so a program that implements those traits writes them (13 cases of
the conformance suite do), and `std.fmt` names them as the prelude's.
And hiding `iterable_list`, whose conformance case would stop (the gate
of the suite holds it); and, for the round, a std module's `pub` copy of
a helper (above).

**Why.** Item 2 of the round, the manual's finding under "Syntax and
names" ("the prelude is wider than 09's fixed list") and R164's open
item (completions offered every prelude name).

**In stage 1.** `resolve` (`ModuleNames.internal`, `marks_pub` and
`visible`, `Environment.for_programs`, which `resolve` and
`resolve_program` resolve against, `internal_name`, `PRELUDE_HELPERS`,
`import_offset`, `E0278` where a value or a type name was `E0200` or
`E0201`), `query` (`scope`), `diag` and the catalog (`E0278` and its fix,
after `E0277`), and `std/prelude.sisu`'s `pub` words.

**What it changes.** The case `cases/findings20/prelude_internal.sisu`
(its `.diag`: `days_in_month`, `DrawState` as a type and `max_of`), and
what it broke in the corpus:

- `compiler/conformance/cases/run/iterable_rest.sisu` calls `iterable_list`
  itself to take what is left of an iterator given to an `Iterable[T]`
  parameter (decision 235): hidden, it was `E0278` and the suite's run
  of it failed, so `iterable_list` stays `pub` (above).
- `proposal/examples/concurrency.sisu` writes `list[Outcome]` for its own
  outcome type, which the example does not declare (its text names it),
  and stage 0 took it for the prelude's `Outcome` of the property tests:
  `E0278` there now, beside the example's `E0201`, `E0200` and `E0432`.
- The manual's examples name none of the helpers; the manual's reference
  ("Names every module sees", "Stage 0 differs") says what changed.

- Differs: `compiler/stage1/cases/findings20/prelude_internal.sisu`: names, ids, human, doc, doc-output
- Differs: `proposal/examples/concurrency.sisu`: names, ids, human, doc, doc-output

**Decision 1006**, for docs/architecture.md:

1006. The prelude marks `pub` the names 09's list and 11 give it, and a
      program's module sees only those: a name of the prelude's own
      helpers (`days_in_month`, `DrawState`, `max_of`, ...) is
      `E0278`, with the import of a std module that has it as the fix
      where one does, and completions offer the `pub` names. `Formatter`, `Spec` and `Hasher` stay visible: the traits
      take them. Stage 1 decides it (RULES.md, R175).

## R176. `fix`: each diagnostic's preferred fix applied to the file (decision 1040)

**The rule.** `sisu1 fix FILE [--dry-run] [--all]` (10: `sisu fix`
"apply compiler-suggested migrations") checks the program that holds
FILE as `query` finds it (R164), with its tests, and applies to FILE's
text the preferred fix of each diagnostic `check` reports in FILE: the
first of its fixes, the one `query actions` numbers 0 (R169), when it has
edits; a diagnostic whose first fix has none is left. With `--all` the
lints `lint` reports in FILE are fixed too (R11, the unused imports of
R87 among them). A fix whose edits touch those of a fix taken before it
in `check`'s order waits for the next round. The text is then checked
again and fixed again, until no fix applies, a round would bring back a
text an earlier round had (a fix that undoes another: `D0179`, and that
round is not applied), or 16 rounds are done. Each fix applied is a line
on standard output, `FILE:LINE:COL: CODE fixed: FIX` (`D0175`, FIX as the
human form writes it after `= fix: `), with each of its edits under it,
`  RANGE "TEXT"`, with `--dry-run`, whose positions are in the text of
its round; then `fixes applied to `FILE`: N` (`D0177`), the file written
once at the end, or with `--dry-run` `D0178` and the file left as it is;
`D0176` when nothing has a fix. Another option is `D0009` with status 2,
no path or more than one a usage error. `sir --repl-serve` serves it.

```text
$ sisu1 fix --all fix_assist.sisu
fix_assist.sisu:11:9: E0339 fixed: write `mut` before the argument: `mut `
fix_assist.sisu:13:11: E0200 fixed: a similar name is in scope: `count`
fix_assist.sisu:4:1: W0027 fixed: remove the import
fixes applied to `fix_assist.sisu`: 3
```

What the driver may rely on from 0.16.0: the command, its options, its
lines, the file it writes and its status; its routing of `sisu fix`
(`D0003` today) is stage 0's next round. 10's `--edition` and `--deps`
wait for editions and the registry; `--deps` would apply R177's fixes in
a dependent package.

**Why.** Item 1 of the round: 10 lists `sisu fix`, and R169's `query
actions` has every fix with its edits.

**In stage 1.** `main` (`fix_command`, `FIX_ROUNDS`, `Chosen`,
`fixes_to_apply`, `apply_edits`; `fix` among `served_command`'s), the
catalog (`D0175` to `D0179`), `sisu1` (its status), `check.sh` (the class
`fix`: a case with a `NAME.fixed` copied into a directory of its own and
fixed there as a process, what it printed and the file it left against
the `.fixed`). `sisu1 --version` is 0.16.0.

**What it changes.** No output of the corpus. The cases:
`cases/findings21/fix.requests` (served: `--dry-run` of `fix_assist.sisu`
with and without `--all`, of `fix_inline.sisu`, of `fix_none.sisu`, which
has nothing, an unknown option and no path), and `fix_assist.fixed` and
`fix_inline.fixed` (as a process, the class `fix`).

`fix_assist.sisu`'s unused import is one R87's formatter removes, which
stage 0's keeps.

- Differs: `compiler/stage1/cases/findings21/fix_assist.sisu`: fmt, fmt-check

**Decision 1040**, for docs/architecture.md:

1040. `sisu1 fix FILE [--dry-run] [--all]` applies the preferred fix (the
      first, with edits) of each diagnostic `check` reports in FILE, and
      with `--all` of each lint, checking the text again until no fix
      applies or a round would bring back an earlier text, says each fix
      it applied (`D0175` to `D0179`), and writes the file once, or with
      `--dry-run` prints the edits and leaves it. `sisu1 --version` is
      0.16.0. Stage 1 decides it (RULES.md, R176).

## R177. `@fix_inline`: a call rewritten with what the function returns (decision 1041)

**The rule.** 09, 10 and D-067: "a library deprecates an item by
rewriting its body in terms of the new API and marking it `@fix_inline`,
and `sisu fix --deps` replaces dependents' calls with that body". A
function marked `@fix_inline` is deprecated by it: a call of it warns
`W0023` with its `@deprecated` text, or `W0025` without one, as R30's
deprecated functions do. Where its body is one `return` of an expression,
it is a free function and its parameters take no `mut` or `own`, the
warning has the fix `type.inline-call`, "write what `twice` returns in
the call's place", whose edit replaces the call with the expression, each
parameter's uses replaced by the call's argument (in parentheses where
the argument is more than a name, a literal or a chain of calls, members
and indexes), the whole in parentheses where it is more than such a
chain:

```text
26:11-26:19 W0025 type.deprecated-plain name="twice"
  fix type.inline-call 26:11-26:19 "scaled(n, 2)"
```

The fix is given only where the rewrite means what the call meant: an
argument that is not a name, a member chain of one or a literal is used
exactly once, the arguments of that kind in their order, and before any
call of the expression has returned (so `twice(reading())` becomes
`scaled(reading(), 2)`, and `sum_sq(reading(), 1)` keeps its warning
without one); a parameter's name bound again inside the expression (a
closure's parameter, a comprehension's name) gives no fix; in another
module than the function's, only an expression that names nothing but
its parameters (and members and keyword names) has one, since its other
names are the function's module's; a call that leaves out an argument
with a default has none. A `@fix_inline` function whose body is more, a
method, or one with a `mut` or `own` parameter, is `W0033`,
`type.fix-inline-body`, at the attribute, and its calls warn without the
fix. `sisu fix` (R176) applies the fix.

**Decided against**, for the round: 10's ownership-aware inliner, which
moves a body of several statements, its borrows, `own` moves and `defer`
into the caller's scope; methods, whose receiver would take the place of
`self`; and `--deps`, which needs the registry's dependents.

**Why.** Item 1 of the round: the manual's finding "`@fix_inline`
compiles without a word and does nothing".

**In stage 1.** `typeck.fix_inline` (`InlineForm`, `form_of`, `inlined`,
`plain`, `primary`), `typeck.collect` (`inline_form`, `W0033`),
`typeck.program` (`Program.fix_inline`), `typeck` (`deprecated_uses`,
`inline_call`), `diag` and the catalog (`type.inline-call`, `W0033`).

**What it changes.** The case `cases/findings21/fix_inline.sisu` (its
`.out`, its `.diag`, its `.fixed`, and `fix --dry-run` of it in
`fix.requests`), which stage 0 checks without a word. The manual's
examples `explanation-evolution/migrate.sisu` and
`reference-attributes/designed.sisu`, outside `check.sh`, now show the fix
under their `W0023`.

- Differs: `compiler/stage1/cases/findings21/fix_inline.sisu`: ids, human

**Decision 1041**, for docs/architecture.md:

1041. A function marked `@fix_inline` is deprecated by it, and where its
      body is one `return` of an expression the warning at each call has
      the fix that writes the expression in the call's place with the
      arguments in their parameters' places, when that means what the
      call meant; another body is `W0033`. Stage 1 decides it (RULES.md,
      R177).

## R178. A constant is computed at compile time: `check` reports one that fails (decision 1042)

**The rule.** R3 of the manual, and 08: a constant's value is computed at
compile time. `check` (and every command that checks a program: `build`,
`run`, `test`, `eval`, `query diagnostics`) evaluates each `const` item of
the program's own modules (not the standard library's) with the
compile-time evaluator (`interp`, R165, R173), once the program has no
other error and its `comptime` expressions ran, within its
`@comptime_budget` or the default, and reports a failure at the
constant's name: a panic, a failed invariant or contract among them,
`E0602`; a refinement its value does not meet, `E0810`; the budget spent,
`E0603`; the world used, `E0601`. A constant that reaches a typed hole is
left alone, as a hole is (R27), and so is one whose evaluation needs what
the evaluator does not run (a `Beyond`), or whose value cannot be kept,
which the run computes as before. A build stops at such an error, as at a
failed `comptime` expression (R170's order).

The lowering does not fold the value: the SIR keeps a constant's
function, which the run calls where the program first reads it. A
constant's value may own memory (a list, a struct of `String`s), which a
folded constant would have to build at run time all the same, and
folding would change the SIR of every program with a constant, which the
classes `sir` and `behaviour` hold against stage 0's; the check is what
R3 asks of the compiler.

**Why.** Item 2 of the round, the manual's nineteenth round: a `const`
whose invariant fails passes `sisu check` and panics under `sisu run`.

**In stage 1.** `main` (`Constant`, `constants_of`, `const_failures`,
which `report_program` runs after `interp.evaluate`).

**What it changes.** The cases `cases/findings21/const_failures.sisu`
(its `.diag`: an invariant, an index and a budget) and
`cases/findings21/const_fine.sisu` (its `.out`); and
`cases/findings19/config.answers`, blessed again, since `eval` of
`config_refused.sisu` now stops at `WEB`'s `E0602` from the check, before
it renders, so its `E0617` for `SQUARE` no longer follows.

The diagnostics of the cases whose constants fail differ from stage 0's,
which checks them without a word: the conformance suite's `eval/stops`
(a `.diag` of its own now, `E0602` at `BAD`, whose assertion fails),
`cases/findings19/config_refused.sisu` and
`cases/findings20/config_prelude_refused.sisu`, whose constants `eval`
refused at their names and `check` now refuses there too.

- Differs: `compiler/stage1/cases/findings21/const_failures.sisu`: ids, human
- Differs: `compiler/conformance/cases/eval/stops.sisu`: ids, human
- Differs: `compiler/stage1/cases/findings19/config_refused.sisu`: ids, human
- Differs: `compiler/stage1/cases/findings20/config_prelude_refused.sisu`: ids, human

**Decision 1042**, for docs/architecture.md:

1042. Every command that checks a program evaluates each `const` of the
      program's modules at compile time, once the program has no other
      error, and reports a panic or a failed invariant (`E0602`), a
      refinement not met (`E0810`) or the budget spent (`E0603`) at the
      constant's name; the SIR keeps computing the value at run time.
      Stage 1 decides it (RULES.md, R178).

## R179. A keyword argument's name is a reference of its field (decision 1043)

**The rule.** In a struct's construction, `Bed(rows=2, crop=...)`, the
name before `=` names the field it gives a value: `query references` lists
it among the field's uses, `query rename` renames it, and `hover`,
`definition` and `references` asked at it answer for the field. A
keyword argument of a function's call is not a field's, and is not
counted; nor is a name in a struct pattern, which the query reads from
the expressions only.

**Why.** Item 4 of the round, R169's open item: a rename of a field missed
its keyword arguments.

**In stage 1.** `query` (`keyword_field`, which `references` and
`target_of` ask).

**What it changes.** The case `cases/findings21/keyword_refs.sisu` (its
`.out`), asked by `cases/findings21/queries.requests` (the references of
`rows` from its declaration and from a keyword, its rename);
`cases/findings19/query_program.answers` is blessed again (two keyword
names among a field's references).

**Decision 1043**, for docs/architecture.md:

1043. A keyword argument's name in a struct's construction is a reference
      of the field, which `query references` lists and `query rename`
      renames. Stage 1 decides it (RULES.md, R179).

## R180. TOML's tables, and `E0617` at the field (decision 1044)

**The rule.** `eval --to toml` (R165) writes a constant that is a struct,
a dict or an enum's variant with fields as a table of its own,
`[LIMITS]`, a field of it of that kind as a table under it,
`[LIMITS.retry]`, and a list whose every item is such a mapping as an
array of tables, `[[SERVICES]]`, one table an item, as TOML's own style
has it; a table that holds only tables is left to their headers
(`[SERVICES.tier.Paid]`). The values TOML writes on one line come first,
each `KEY = VALUE`, the constants and a table's fields in the order
written, then the tables, each after a blank line, since TOML reads every
line after a table's header as that table's. A list of values and a list
that mixes mappings with other values stay inline. JSON and YAML are as
they were. `E0617` names the field or item that holds what the form
cannot write by its path from the constant, `LIMITS.retry.backoff`,
`MAYBE[0]`, `TABLE["key"]`, where it named the constant.

**Why.** Item 5 of the round, the manual's nineteenth round: two services
wrote one line of 150 columns, and `E0617` named `LIMITS`, not
`timeout`.

**In stage 1.** `config` (`Unwritable.Holds`'s `path`, `data_of`, `json`,
`toml` with the path; `render`, `is_table`, `is_tables`, `toml_entry`,
`toml_fields`), `main` (`eval_with`, `no_form`).

**What it changes.** The cases `cases/findings21/config_tables.sisu` and
`config_unwritable.sisu`, asked in `queries.requests`; blessed again: the
conformance suite's `eval/config.eval` (its TOML), and
`cases/findings19/config.answers` (its TOML and `MAYBE[0]`).

**Decision 1044**, for docs/architecture.md:

1044. `eval --to toml` writes a struct or a mapping as a table and a list
      of them as an array of tables, after the values written inline,
      and `E0617` names the path of the field that holds what a form
      cannot write (`LIMITS.timeout`). Stage 1 decides it (RULES.md,
      R180).

## R181. The whole hole report: fits to depth two, the types, the contract, the tests, JSON (decision 1045)

**The rule.** `check --holes` and `query holes` (R174) give the rest of
10's report. For each hole:

- **What fits**, to depth two: the bindings of its type, as before, and a
  field of a binding (`self.hours`), a method of one that takes nothing
  else (`t.total()`), and a function of the program's own modules, but
  the hole's, of one argument that a binding is (`doubled(t)`), whose
  type is the hole's, sorted.
- **The types it mentions**: each struct and enum of the program's own
  modules that the hole's type names, through its arguments, references,
  tuples and functions, with its fields or variants: "`Size` is an enum
  of `Small`, `Large(hours: int)`" (`D0180` to `D0183`).
- **The contract in force**: the `requires` of its function as facts,
  "given: `t.hours >= 0`" (`D0184`), and its `ensures` as obligations,
  "to keep: `result >= t.hours`" (`D0185`); `result` is not among the
  bindings in scope.
- **The tests that reach it**: the tests whose bodies call its function,
  directly or through others, by name in the tests' order, the property
  test of its contract among them (`D0186`), or "no test reaches it"
  (`D0187`).
- **A JSON form**: `check --holes --format=json FILE`, an array of one
  object per hole with `at`, `name`, `type`, `function`, `scope`, `fits`,
  `types` (each `name`, `kind`, `members`), `requires`, `ensures` and
  `tests`; `--format=human` is the default.

`query holes` appends four fields to R169's six:
`TYPES\tREQUIRES\tENSURES\tTESTS`, the types as `struct Task {name:
String, hours: int}`, each list separated by `; `, the tests quoted.

**Decided against**: the values a test sees at the hole, which need the
tests run, `sisu test`'s work (R27 prints a hole's closure where a run
reaches it); the bindings' capabilities apart from their types, which
the types show; and fits through two calls, which would enumerate.

**Why.** Item 3 of the round, R174's decided-against part of 10's report.

**In stage 1.** `query` (`Hole`, `HoleType`, `holes`, `fits_deeper`,
`mentions`, `hole_type`, `tests_reaching`, `holes_answer`, `type_line`,
`hole_json`, `json_text`), `main` (`holes_command`) and the catalog.

**What it changes.** No output of the corpus. The case
`cases/findings21/holes_report.sisu`, asked in `queries.requests` (`check
--holes`, its JSON, `query holes`); `cases/findings20/query_assist.answers`
is blessed again (the holes' new lines and fields).

*Amended in the twenty-second round.* `D0181` (`cli.hole-struct-empty`)
and `D0183` (`cli.hole-enum-empty`) are kept, not retired: `holes_command`
now names each one whole, for a struct without fields and an enum without
variants. Before, it made the identifier by adding `-empty` to the
plain one, and the catalog's test of its entries (`every_entry_is_used`)
could not find them. The case is `cases/findings22/hole_empty.sisu`,
asked by `roles.requests`. Nothing it prints changes.

**Decision 1045**, for docs/architecture.md:

1045. `check --holes` and `query holes` also give the expressions that fit
      to depth two, the definitions of the program's types the hole's
      type names, its function's `requires` and `ensures`, the tests that
      reach it (`D0180` to `D0187`), and `--format=json`; the values a
      test sees wait for `sisu test`. Stage 1 decides it (RULES.md,
      R181).

## R182. `@derive(Plain)` of a `@repr(c)` struct with padding (decision 1046)

**The rule.** 22 allows no padding in a plain type, whose `as_bytes`
would read it. Stage 0's nineteenth round lays a declared struct's fields
out at C's offsets in both lowerings (decision 1021): each at the next
offset its alignment allows, a narrow number's width, a struct's widest
field's, a word for any other, the size a multiple of the widest. The type
checker computes the same offsets for a `@repr(c)` struct that derives
`Plain` (its fields plain: the integers, the floats and such structs),
and each gap is an error at the derive: `E0279`, "`Reading` cannot derive
`Plain`: C's layout leaves 3 bytes of padding between `id: u8` and
`value: i32`, at offsets 1 to 4", or `E0280` for one at the end, with the
note that a field `_pad1: Array[u8, 3]` there makes the bytes the
struct's, or that the fields ordered from the widest leave none. A struct
of one field, which needs no `@repr(c)`, has no gap. Stage 1 reads the
offsets from the types, as the lowerings' tables compute them; nothing
else needs them.

**Why.** Item 6 of the round, the manual's "Types and traits" finding,
which R108 left until the layouts were lowered.

**In stage 1.** `typeck.instances` (`check_derives`, `plain_gaps`,
`plain_layout`, `padding_note`), `diag` and the catalog (`E0279`,
`E0280`, `resolve.plain-padding-fill`).

**What it changes.** The case `cases/findings21/plain_padding.sisu` (its
`.diag`: `Reading`'s two gaps, `Packed` without one); no case of the
corpus derives `Plain` with padding.

The case imports `Plain` from `std.mem`, stage 1's (R5), which the Rust
front end answers with `E0220`.

- Differs: `compiler/stage1/cases/findings21/plain_padding.sisu`: names, ids, human, doc, doc-output

**Decision 1046**, for docs/architecture.md:

1046. `@derive(Plain)` of a `@repr(c)` struct whose fields C's layout
      (decision 1021) leaves a gap between or after is `E0279` or
      `E0280`, naming the bytes, their offsets and the fields around
      them. Stage 1 decides it (RULES.md, R182).

## R183. `std.time.days_in_month`, a home for the prelude's helper (decision 1047)

**The rule.** `std.time` has `pub pure def days_in_month(year, month)`,
the days of a month of the proleptic Gregorian calendar, with a doc
example; the prelude keeps its own copy, which the standard library's
code calls and a program does not see (R175). `E0278` for
`days_in_month` now has the fix `resolve.import-from-std`, "import it
from `std.time`", whose edit writes `from std.time import days_in_month`
before the module's first line of code (`PRELUDE_HELPERS`). The prelude's
other helpers are its own algorithms (`days_from_civil`,
`civil_from_days`, the iterators', the property tests') with no place in
a program's API, and keep none.

**Why.** Item 7 of the round, R175's decided-against item.

**In stage 1.** `resolve` (`PRELUDE_HELPERS`); `compiler/std/time.sisu`
(appended, so that the stage 0 round's edits of `std/` meet nothing).

**What it changes.** The case `cases/findings21/days_in_month.sisu` (its
`.out`); blessed again: `cases/findings20/prelude_internal.diag` (the fix)
and `query_assist.answers` (its human form), and the conformance suite's
`doc/std_pages.doc` (`std.time`'s page lists the function), which the
Rust front end, reading the same `std/`, writes alike, and
`std/doc_examples.select` (its doc example is one test more, 199).

**Decision 1047**, for docs/architecture.md:

1047. `std.time` has a `pub` `days_in_month`, which `E0278`'s fix imports
      for a program that names the prelude's own copy. Stage 1 decides it
      (RULES.md, R183).

## R184. The merge's differences: the conformance suite's `std/bytes`, `std/narrow` and `unsafe/narrow_pointer` (decision 1048)

**The rule.** Nothing a command prints changes. Stage 0's nineteenth
round added five conformance cases (decisions 1020 to 1022), of which
three differ from the Rust front end where a rule of stage 1's already
decides otherwise:

- `std/bytes.sisu` takes `xs[^3:]` of a `bytes`, a bound from the end,
  which R48 gives `bytes` and the Rust front end answers with `E0383`, so
  its types, SIR, diagnostics and pages differ.
- `std/narrow.sisu` reads `by[3]` of a `dict[i32, u16]` in an
  interpolation, which R152 reads into a temporary at the index, so its
  SIR reads the entry a statement earlier.
- `unsafe/narrow_pointer.sisu` imports `std.mem` and writes `unsafe` code,
  stage 1's (R4, R5), which the Rust front end answers with `E0220` and
  `E0900`, so its AST, names, SIR, diagnostics and pages differ as those
  of `unsafe/cstring` do (R149).

What each does is what its `.out` says, from stage 1's SIR
(`behaviour`). `eval/config` and `eval/stops`, of the new case kind
`NAME.eval`, are run by the suite, not by `check.sh`; `eval/config.eval`
is blessed again under R180. `unsafe/cstring`'s move into `native.list`
changes nothing here.

**Why.** Item 8 of the round: the merge of stage 0's nineteenth round
brought the cases, and `check.sh` found what they change.

**What it changes.** Nothing.

- Differs: `compiler/conformance/cases/std/bytes.sisu`: types, sir, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/std/narrow.sisu`: sir
- Differs: `compiler/conformance/cases/unsafe/narrow_pointer.sisu`: ast, names, sir, ids, human, doc, doc-output

**Decision 1048**, for docs/architecture.md:

1048. The conformance suite's `std/bytes`, `std/narrow` and
      `unsafe/narrow_pointer` differ from stage 0's front end under the
      rules that already decide their constructs (R48, R152, R4). Stage 1
      decides it (RULES.md, R184).

## R185. `sir --tests --std` and `test --std`: the standard library's doc examples as tests (decision 1080)

**The rule.** port-debt.md: "the standard library's doc examples are the
Rust front end's tests of `compiler/std`". With `--std`, `sir --tests`
lowers the examples of the standard library's `##` documentation as
tests, and so does stage 1's `test --std`. Each fenced example is a test,
as the root package's are (10, "Testing"; R138's `doc_examples`), named as
the Rust front end names it: the prelude's are `std.core/NAME (doc
example)`, a module's `std.NAME/ITEM (doc example)`, a method's
`Type.method`, a second example of an item `(doc example 2)`. They come
in stage 0's order, the prelude's first and then `doc --std`'s modules
(`DOCUMENTED`), and they come before the program's own tests. A module the base has not loaded is
loaded as an import loads it. The base keeps no syntax of its modules, so
the items' starts are read from each module's text. What an example
expects is checked as the root package's examples are, by the asserts
`doc_test_module` writes. The program's own tests come after them.
`--std` without `--tests` changes nothing.

Against the Rust front end's `sisu test --std` of a small program (the
conformance case `std/doc_examples`'s `.select`, 199 lines), stage 1's
`sir --tests --std` has 197 of the standard library's tests, every one
passing when `sisu test --from-sir` runs them. They have the same names,
and the two missing lines are `std.net/Url.parse (doc example)` and
`std.net/Url.from_literal (doc example)`, which the Rust front end lists
twice. It declares `std.net`'s examples once when `std.http`'s import loads
the module and again when its loop reaches `net`. Stage 1 declares each
example once, at its module's place in the order. The `.select` is the
driver's and stays as it is while the driver sends `test --std` to the
Rust front end. Once it sends it to stage 1, the case is blessed again
with 197 tests, and `std.net`'s two tests move after `std.math`'s.

What the driver may rely on from 0.17.0: `sir --tests --std [PATH]`, the
SIR `sir --tests` prints with the doc examples' tests among `test` lines.
The answer and statuses are those of `sir --tests` (R46, R47), and `sir
--repl-serve` serves it as it serves `sir`. `sisu1 --version` is 0.17.0
(the round's items R185 to R189 add a form each).

**Why.** Item 1 of the round: the last part of `sisu test` that only the
Rust front end could run.

**In stage 1.** `typeck.collect` (`std_doc_texts`,
`declare_std_doc_tests`, `std_module_named`, `declare_doc_tests_in`,
`item_starts`), `typeck` (`check_program`'s `std_tests`), `main`
(`Input.std_tests`, `sir_with`, `test_command`), `check.sh` (a `.tests`
case's `# test-options:` line).

**What it changes.** No output of the corpus. The case
`cases/findings22/std_doc_tests.sisu` (its `.tests`: `test --std`, 198
tests).

**Decision 1080**, for docs/architecture.md:

1080. `sir --tests --std` and `test --std` lower the standard library's
      doc examples as tests named `std.core/zip (doc example)`, in stage
      0's order before the program's own, each example once. `sisu1
      --version` is 0.17.0. Stage 1 decides it (RULES.md, R185).

## R186. `check --diary PATH`: the diagnostics as data (decision 1081)

**The rule.** 18, Phase 3, and 20, I-3. `check --diary PATH` (or
`--diary=PATH`) appends to PATH the records the driver's `diary.rs`
appends, in its order and form, one JSON object per line:

- a `check` record: the time (RFC 3339, UTC), the tool `sisu check`, the
  toolchain, the root package's own files each with the FNV-1a hash of its
  text, and the number of diagnostics;
- a `diagnostic` record per diagnostic in `check`'s order: its file and the
  file's hash, the code and catalog identifier, the severity, the message
  and its parameters, the span in lines and columns and in bytes, the
  labels, and the fixes with their edits;
- a `resolved` record for each diagnostic of a file's previous check (the
  last `check` record that names it) that this check no longer has at the
  place the change moved it to. It holds the diagnostic as it was, the one
  edit between the two texts (the bytes of the old text that differ and
  what replaced them), and the identifier of the fix it applied when that
  fix's edits make the new text, else `null`.

The texts are kept by their hashes in `PATH.d/`, made with the
directories above it when they are missing. A program with several files
hashes its own package's modules, and a workspace appends a check per
member. The one field that differs from the driver's is `toolchain`,
which is stage 1's version (`0.17.0`) where the driver writes its crate's
version. A diary written by either front end reads the same to the
other: the case below appended its checks to one diary started by the
Rust front end, and the records matched but for the time and the
toolchain. A diary that cannot be written is `D0013`, and the check
fails. `sir --repl-serve` serves the command, which writes the diary as it
does when it is run alone. Stage 1 reads a previous line with its own
JSON reader, and a line that is not JSON is skipped.

What the driver may rely on from 0.17.0: the option, the records, the
files in `PATH.d/`, and that `check`'s own output and status are what
they are without the option.

**Why.** Item 2 of the round: port-debt.md's `sisu check --diary`.

**In stage 1.** The new module `diary` (`timestamp`, `write`, `parse`,
`records`, `wanted`, `change`, `apply`), `main` (`check`'s `--diary`,
`check_with`, `append_diary`, `Report.ours`).

**What it changes.** No output of the corpus. Its tests are `diary`'s
own (`sisu1 test`): the time, JSON read back as written, the one change.
A case of `check.sh` would compare a time, so the round compared the
diaries by hand, a fix applied and a type error resolved, against the
driver's (above).

**Decision 1081**, for docs/architecture.md:

1081. `check --diary PATH` appends the driver's `check`, `diagnostic` and
      `resolved` records to PATH, keeping each text by its hash in
      `PATH.d/`, so that a diary either front end writes reads the same.
      Stage 1 decides it (RULES.md, R186).

## R187. `explain-borrow FILE:LINE` and `query explain FILE LINE` (decision 1082)

**The rule.** 10, "Language server", has the command "explain this
borrow". `explain-borrow FILE:LINE` prints every borrow live on the line,
as the driver's `explain_borrow` prints it (the form is `sisu-borrowck`'s
`explain.rs`), in the driver's words: `D0014` heads each function with a
statement on the line, then one line per loan. `D0015` names the loan's
kind (`D0019` or `D0020`), its place, where it starts, who holds it and
where it is last used, and `D0016` is for a loan only temporaries hold.
`D0017` says no borrow is live. The loans are those stage 1's ownership
checker finds (`Analysis.live_at_line`, a transcription of stage 0's) in
the program that holds FILE, as `query` finds it: before or after each of
the line's statements, ordered by where they start. The holders are the
named locals that hold a loan on the line, and a loan of a temporary that
only temporaries hold is left out. A program with an error of the front
end has its diagnostics reported as `check` reports them, then `D0006`,
with status 1. Ownership errors do not stop it, as they do not stop the
driver. Without `FILE:LINE` it is `D0018` with status 2, and a file that
cannot be read is `D0001` with status 1.

`query explain FILE LINE` gives the same loans as an answer, one line per
loan: `FUNCTION\tKIND\tPLACE\tSTART\tHOLDERS\tEND`, KIND `shared` or
`mut`, START and END `LINE:COL`, HOLDERS the holders separated by `, `
(empty for a temporary), as R164's fields are written. It is nothing for a
program with an error of the front end or for a line with no loan. `sir
--repl-serve` serves both.

What the driver may rely on from 0.17.0: both commands, their lines and
their statuses, for `sisu explain-borrow` and the language server's
`sisu.explainBorrow`.

**Why.** Item 3 of the round.

**In stage 1.** `borrowck.loans` (`live_at_line`, `LoanReport`,
`loc_order`), `borrowck` (`live_at`, `FunctionLoans`), `main`
(`explain_command`, `owned_program`, `explained`, `query_explain`,
`query_command`'s `explain`), and `sisu1` (the status).

**What it changes.** No output of the corpus. The case
`cases/findings22/explain_borrow.sisu` (its `.out`) is asked by
`roles.requests`: two lines with loans, one without, no line, a program
with an error, and `query explain`. Its answers are the driver's, byte
for byte, for each of its lines and for every line of `b.sisu` the round
tried.

**Decision 1082**, for docs/architecture.md:

1082. `explain-borrow FILE:LINE` prints the loans stage 1's ownership
      checker finds live on the line in the driver's words (`D0014` to
      `D0020`), and `query explain FILE LINE` answers them as
      tab-separated lines. Stage 1 decides it (RULES.md, R187).

## R188. `audit --ownership FILE...` and `audit --caps [FILE...]` (decision 1083)

**The rule.** 18, Phase 3, T1.1 and T1.2, and the driver's `audit.rs`.

- `audit --ownership FILE...` counts the escape hatches in each file from
  its tokens. These are `Shared`, `Weak`, `Cell`, `Arena` and `Key` where
  they mean std's types (not after a `.`, and not in a file that declares
  the name), the calls `.copy()`, and the keyword `unsafe`, and `lines`
  counts the lines on which a token starts that is not trivia or layout.
  Each file gets one line, `FILE lines=N Shared=N Weak=N Cell=N Arena=N
  Key=N copy=N unsafe=N`.
- `audit --caps [FILE...]` reads each program's SIR, built with its tests.
  For each of its own functions that does I/O it prints `caps=`, the
  number of parameters that hold a capability, with `io=direct` or
  `io=calls` and the depth from `main`. Then each use of a capability, the
  structs that bundle three or more capability types, and a summary.
  Every line starts with the file's name, and a test is named `test
  "NAME"` or `closure in test "NAME"`. Without a file it audits the
  package of the working directory, each workspace member's program
  named by its directory. A program with an error has its errors
  reported, then `D0006`, with status 1.

An option the driver's `audit` does not know is `D0009`, none or both of
the two is `D0022`, and `--ownership` without a file is `D0004`, all with
status 2. `audit --unsafe` (R7) is as it was. `sir --repl-serve` now
serves `audit`, every form of it.

The round compared both with the driver's. `--ownership` matched on the
1,112 `.sisu` files of the conformance suite, the standard library, stage
1's sources and cases and the proposal's examples, in one run each.
`--caps` matched on every conformance case (560 files) that the Rust
front end checks without an error, but `eval/stops`, whose failing
constant R178 makes an error. The cases that differ are those whose
errors differ under the rules of RULES.md (stage 1's diagnostics, R16,
R24 and the others) and those the Rust front end refuses where stage 1
accepts (R48, R152 and R4), which then print a summary of their own.

What the driver may rely on from 0.17.0: both forms, their lines, their
messages and statuses.

**Why.** Item 4 of the round.

**In stage 1.** `audit` (`ownership_line`, `declared_names`,
`caps_report`, `capability_types`, `cap_place_type`, `caps_name`,
`shown_fn`), `main` (`audit_counts`, `audit_caps`, `own_structs`;
`audit_command` writes to the command's output, so that it is served),
and `sisu1` (the status).

**What it changes.** No output of the corpus. The case
`cases/findings22/audit_counts.sisu` (its `.out`) is asked by
`roles.requests` with both forms and each misuse.

**Decision 1083**, for docs/architecture.md:

1083. `audit --ownership FILE...` and `audit --caps [FILE...]` print what
      the driver's `audit.rs` prints, from stage 1's tokens and SIR, with
      its messages and statuses. Stage 1 decides it (RULES.md, R188).

## R189. `--defer-borrow`: the SIR of a program the ownership checker rejects (decision 1084)

**The rule.** port-debt.md: "the deferral is the Rust ownership
checker's". `sir --defer-borrow [--tests] [PATH]`, and `build --for-run
--emit-sir --defer-borrow` (R170), print the SIR of a program whose only
errors are the ownership rules', as the driver's `run --defer-borrow`
and `test --defer-borrow` lower it for the counterexample mode. The
ownership rules' errors are the front end's `E04` codes and the ownership
checker's findings, but not purity's (`E0333`, `E0392`, `E0393`) or a
second assignment of a `let` (`E0367`), which the monitor does not
observe. The program is lowered and put through the ownership checker
despite those errors, simplified, and its holes closed. Its `comptime`
values are baked in when the checker found nothing, and a failed one
stops it. Standard error then holds every diagnostic of the program in
the conformance form (`check --format=ids`), and the status is 3 when an
error was deferred, 0 when the program has none. A program with any other
error, or one beyond the slice, prints no SIR. Its stopping errors go to
standard error, in the form `--format` asks for (the human one for
`build`'s, which takes no `--format`), then `D0006`, with status 1. A
program without `main` prints no SIR from `build --emit-sir`, as R170
says. The SIR is the Rust front end's for the case below, byte for byte,
and stage 0's monitor finds its conflict when it runs it (`sisu run
--monitor --from-sir`).

What the driver may rely on from 0.17.0: the option on both commands,
standard output, standard error and the statuses 0, 1 and 3, served by
`sir --repl-serve`.

**Why.** Item 5 of the round.

**In stage 1.** `main` (`Deferred`, `deferred_program`, `not_deferred`,
`deferred_answer`; `sir_with`, `build_command` and `BuildHow.deferring`),
and `sisu1` (the status of `sir` and `build` with the option).

**What it changes.** No output of the corpus. The cases
`cases/findings22/defer_borrow.sisu` and `defer_other.sisu` (their
`.diag`s) are asked by `roles.requests`: both commands, a program with a
type error, and one without an error.

**Decision 1084**, for docs/architecture.md:

1084. `sir --defer-borrow` and `build --for-run --emit-sir --defer-borrow`
      print the SIR of a program whose only errors are the ownership
      rules', with every diagnostic in the conformance form and status 3,
      for stage 0's counterexample mode. Stage 1 decides it (RULES.md,
      R189).

## R190. A named function in `filter`: the closure that calls it, which now reads `ref ref str` as `str` (decision 1085)

**The rule.** An adapter's function takes a reference to each item (11,
`filter(keep: (ref Item) -> bool)`). A function whose parameter is one
level of reference short of that, `def is_big(x: ref int)` for an
`Iter[ref int]`, is not a function of that type. It stays `E0300` with
R63's fix, `x -> is_big(x)`, a closure that calls it and lets the call
read through the reference. The fix is now offered for a function that
takes `str` too, `filter(is_long)` over a list of `str` (shown `(str) ->
bool`, since `ref str` is the view `str`). That closure now compiles: a
reference to a reference to text where `str` is expected is read through
both, as `ref ref int` is read as `ref int` where `ref int` is expected.

Decided against: accepting the function value itself. A function value
is the function, and taking one of another parameter type would need a
wrapper the program did not write, a second function behind the name.
04's references are written where they are taken, and a call's reborrow
is the closure's. The spelling that works is the fix's, `filter(x ->
is_big(x))`. The adapters' signatures stay 11's.

**Why.** Item 6 of the round, the manual's finding in "The standard
library": the closure the manual wrote for an `int` predicate was refused
for a `str` one.

**In stage 1.** `typeck.expr` (`coerce`: a `ref ref str` as `str`;
`by_value_closure`: a `str` parameter).

**What it changes.** The cases `cases/findings22/filter_named.sisu` (its
`.out`, both closures) and `filter_named_errors.sisu` (its `.diag`, both
fixes). The Rust front end refuses the first case's `str` closure (`E0300`) and gives no fix
for the second case's `str` function.

- Differs: `compiler/stage1/cases/findings22/filter_named.sisu`: ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/findings22/filter_named_errors.sisu`: ids, human, doc, doc-output

**Decision 1085**, for docs/architecture.md:

1085. A function named where an adapter expects a function of references
      to its parameter type is `E0300` with the closure that calls it as
      its fix, for a `str` parameter too, and a `ref ref str` is read as
      the `str` a call expects. Stage 1 decides it (RULES.md, R190).

## R191. Three diagnostics' words: `E0117`'s token, `E0241`'s member, one `E0333` per chain (decision 1086)

**The rule.** The manual's "Diagnostics without fixes":

- `E0117` names a kind of token without an article, "unexpected string
  after the end of the statement" (`found="string"`). Before, it read
  "unexpected a string". A token's own text is as it was ("unexpected
  `2`").
- `E0241` says "its member `SIDES` cannot be used through it". Before, it
  called a trait's constant (R37) a method that cannot be called. The
  catalog's text changes, so the driver's Rust front end, whose E0241 is
  always a method's, prints "member" too.
- In a `pure` function, the consumer or adapter after an impure adapter is
  the same chain. `count()` of `split_whitespace().filter(...).count()` is
  no second `E0333`: the adapter's error, with the note
  `type.pure-adapter` (R102), is the one.

**Why.** Item 7 of the round.

**In stage 1.** `parser.state` (`end_stmt`), the catalog (`E0241`, with a
comment), `borrowck.purity` (`adapted`).

**What it changes.** The cases `cases/findings22/f_string_trailing.sisu`,
`diagnostic_words.sisu` and `pure_chain.sisu` (their `.diag`s);
`cases/findings10/f_string_let.diag` (the parameter) and
`cases/findings10/pure_adapter.diag` (its `sum()` after `map`, no second
`E0333`) are blessed again. In `diagnostic_words.sisu` the Rust front end
does not know a trait's constant (R37), so its tree and AST differ too.
The manual's transcripts that show `E0117` for a kind of token or any
`E0241` are made again by the manual's writers.

- Differs: `compiler/stage1/cases/findings10/pure_adapter.sisu`: ids
- Differs: `compiler/stage1/cases/findings22/f_string_trailing.sisu`: tree, ids, human, fmt-check, doc, doc-output
- Differs: `compiler/stage1/cases/findings22/diagnostic_words.sisu`: tree, ast, ids, human, fmt-check, doc, doc-output
- Differs: `compiler/stage1/cases/findings22/pure_chain.sisu`: ids, human

**Decision 1086**, for docs/architecture.md:

1086. `E0117` names a kind of token without its article, `E0241` calls
      what a `dyn` value cannot use a member, and a `pure` function's
      chain after an impure adapter is one `E0333`, the adapter's. Stage
      1 decides it (RULES.md, R191).

## R192. A struct pattern's field names in `references` and `rename` (decision 1087)

**The rule.** R179's open item. In a struct pattern, `case Point(x=a,
y=b)`, the name before `=` names the field, as a construction's keyword
argument does (R179). `query references` lists it among the field's
uses, `query rename` renames it, and `hover`, `definition` and
`references` asked there answer for the field. The name is read from the
narrowest struct pattern of the program's bodies at it. A variant's
pattern names a variant's field, which has no reference of its own, and
is not counted.

**Why.** Item 8 of the round.

**In stage 1.** `query` (`pattern_field`, which `references` and
`target_of` ask).

**What it changes.** No output of the corpus. The case
`cases/findings22/pattern_refs.sisu` (its `.out`) is asked by
`roles.requests` (the references of `x`, and its rename from the
pattern).

**Decision 1087**, for docs/architecture.md:

1087. A field's name in a struct pattern is a reference of the field,
      which `query references` lists and `query rename` renames. Stage 1
      decides it (RULES.md, R192).

## R193. `@derive(Decode)` of a struct with a field of an enum (decision 1088)

**The rule.** This is the coordinator's item, the manual's G7. Take a
struct that derives `Decode` with a field whose type is (or holds, as a
list's or an option's element) an enum of the program. It passed `check`,
and `run` stopped before `main` with `E0501`, "invalid SIR in
`json.field#Mood`: bb3: calls missing function json.from_json". Now the
derive stops at compile time with `E0608`: "deriving `Decode` for `Day`
stopped at compile time: its field `mood` holds `Mood`, an enum, whose
variants `json.decode` cannot make".

The fault was the instances'. `json.from_json[Mood]` reaches the template's
`comptime panic` ("no JSON form for `Mood`"). The instance that failed was
asked first from the derived impl's module, `<derive>/NAME.sisu`, whose
errors the program does not show. Lowering then left the failed instance
out, while `json.field#Mood` still called it. The SIR's check of missing
functions was right to stop it, and the template (`std.json`'s) is as it
should be. Stage 1 now stops the derive itself, where the field is
written, as R62 stopped `Decode` of an enum. A call of `json.decode` of
such a struct then also has `E0609` at the call, which it had already
without the derive.

**Why.** The coordinator's item of the round.

**In stage 1.** `typeck.instances` (`check_derives`, `undecodable`).

**What it changes.** The case `cases/findings22/decode_enum_field.sisu`
(its `.diag`).

- Differs: `compiler/stage1/cases/findings22/decode_enum_field.sisu`: ids, human, fmt, fmt-check, doc, doc-output

**Decision 1088**, for docs/architecture.md:

1088. `@derive(Decode)` of a struct with a field that holds an enum is
      `E0608` at the derive, naming the field, where the run stopped at an
      invalid SIR. Stage 1 decides it (RULES.md, R193).

## R194. Narrowing methods of the sized integers: decided against (decision 1089)

**The rule.** The manual's finding: `to_u8()` of a `u32` is `E0304`, so
the manual narrows a `u32` through `to_int()`. Neither 03 nor 11 gives
`to_u8`, `to_i8`, `to_u16`, `to_i16`, `to_u32` or `to_i32`. 01's tour
spells every lossy conversion with its target as a type argument,
`x.truncate_to[u8]()`, which every sized integer has (R115), and a
widening one is implicit. Decided against: a method per target width
would be a second spelling of `truncate_to`, which 00's "one way to do
it" rules out. A checked narrowing is the comparison against the target's
bounds, `if x.to_int() <= u8.MAX.to_int(): x.truncate_to[u8]()`, written where the
program decides what an out-of-range value means.

**Why.** Item 9 of the round.

**What it changes.** Nothing.

**Decision 1089**, for docs/architecture.md:

1089. The sized integers narrow with `truncate_to[T]()` alone, with no
      `to_u8()` and its kin; a checked narrowing compares with the
      target's bounds first. Stage 1 decides it (RULES.md, R194).

## R195. `build --emit-sir` without `--for-run`: a served `sisu build`, the executable named; `check --holes` of a package (decision 1120)

**The rule.** Decision 1062: `sisu build` (with `--native` too) and `sisu
run --native` of a package stay a process of stage 1 each, since R170
serves a run's build alone and a package's executable goes where stage 1's
build puts it. Now:

- `sisu1 build --emit-sir [--release] [--contracts=LEVEL] [--output OUT]
  [--locked] [--features NAMES] [--no-default-features] [PATH]` is `build
  --native` up to the point where the build hands its backend the SIR. It
  writes the lockfile as the build writes it, and the program's
  diagnostics on standard error in the build's order, with the build's
  status (1 at an error, and `D0132`, "not compiled", after the errors).
  On standard output each program of PATH (a file's, a package's, each
  member's of a workspace, in turn) writes the line `# executable PATH`,
  the executable the backend would make (`build/PROFILE/NAME` beside a
  package's manifest, a file's name without `.sisu`, or `--output`'s),
  then its SIR. A program without `main`, a library, writes `# ` and the
  line the build prints of it, `# built `lib.sisu``. Each line is a
  comment of the SIR, which its readers pass by, so a file's answer is
  one SIR and a workspace's is split at those lines.
- `--native` is implied by `--emit-sir`, so `--release` and `--contracts`
  are taken without it, as R170 said and the build did not do (it
  answered `D0111` and `D0009`).
- `build --for-run --emit-sir` (R170), and with `--defer-borrow` (R189),
  write the same line before their SIR, so that a package's `run
  --native` finds its executable from the answer. It is a comment, so the
  driver's reader of R170's answer reads it as before.
- `sir --repl-serve` serves `build` with `--emit-sir`, and a `build`
  without `--native`, which calls no backend: its output, `built ...`, as
  the process prints it. A `build --native` without `--emit-sir`, which
  runs the backend, is still answered as misused.
- `check --holes` takes a package's directory, or no path for the working
  directory's package (the nearest manifest above it), and reports the
  holes of each of its modules, each named by its module's file as
  `check` names a package's (`src/dimmer.sisu:13:12`), with
  `--format=json` too. A workspace reports each member's. More than one
  path is the usage error it was.

```text
$ printf 'build\t--emit-sir\t--dir\tlamps\n' | sisu1 sir --repl-serve
answer 0 1342 146
# executable /work/lamps/build/debug/lamps
sir 0
...
```

What the driver may rely on from 0.18.0: the request and its standard
output, a line `# executable PATH` (or `# built ...`) before each
program's SIR in the build's order, and its standard error and status,
which are `build --native`'s. The same line before R170's and R189's SIR,
and `check --holes` of a directory and of none, served. The driver
compiles each SIR into the executable the line names, makes its directory
and prints the backend's `built` line, as the build's backend would have.

**Why.** Item 1 of the round; decision 1062, "Serving them is stage 1's to
add".

**In stage 1.** `main` (`build`'s words: `emit_sir` implies `native`;
`build_program`'s `EMITTED_EXECUTABLE` and `EMITTED_BUILT`;
`build_command`'s deferred answer; `repl_serve`'s served `build`;
`holes_command` and `hole_lines`), `sisu1` (its header).

**What it changes.** No output of the corpus. The cases:
`cases/findings23/served.requests` asks `build --emit-sir` of
`lamp.sisu`, with `--release --contracts off` and with `--output`, of
`lamp_library.sisu` (`# built`), a plain `build` and the refused `build
--release` and `build --native`, `build --emit-sir` and `--for-run
--emit-sir --release` (its hole an error) of the package case
`cases/packages/served_build.pkg`, and `check --holes` of the package
without a path, by its directory, with `--format=json`, and of two files.
`lamp.sisu` has its `.out`. Blessed again:
`cases/findings20/query_assist.answers`, whose `build --emit-sir` was a
usage error and is now the SIR, and whose request `check --holes` without
a path, which now checks stage 1's own package, the directory's nearest,
names two files instead; and the line before the SIR in its and
`cases/findings22/roles.answers`'s `--for-run --emit-sir`.

**Decision 1120**, for docs/architecture.md:

1120. `sisu1 build --emit-sir [PATH]` writes on standard output, for each
      program the build makes, a line `# executable PATH` (or `# built
      ...` for a library) and the SIR the native build would hand its
      backend, with the build's diagnostics and status, `--native`
      implied; R170's and R189's answers carry the line too, `sir
      --repl-serve` serves the build and a build without `--native`, and
      `check --holes` takes a package's directory or none. `sisu1
      --version` is 0.18.0. Stage 1 decides it (RULES.md, R195).

## R196. `query values FILE`: a worksheet's inline values from stage 1's evaluator (decision 1121)

**The rule.** Decision 1060 leaves worksheets' inline values to the Rust
front end. `query values FILE` answers them as the driver's
`evaluate_sheet` shows them. The worksheet FILE is checked as written and
made the program `sir --worksheet` writes (R79, R95), each top-level
statement marked and shown. That program is run by stage 1's evaluator
(`interp`, R165), its `System` a value nothing can use, with a budget of
100,000,000 steps. A run keeps what `print` writes (at compile time
printing is the world). The answer is one line per value and per line
printed, by the line the statement ends on in the file as written, R164's
fields:

```text
4	value	[3, 5, 8]
8	output	added 3
11	stopped	the standard error
```

`value` is a value the statement shows (a `let`'s each name, an
expression's value, as `{value:?}` renders it). `output` is a line the
statement printed. Where the evaluator stops, the last line is the line
of the statement it was running. That line is `stopped` with what it does
not run (`the standard error`, any other use of the world, an intrinsic
it lacks such as `list.first`, or its budget), or `panicked` with the
panic's message. Nothing comes after it, though the driver's run would go
on, so an editor shows those values and marks the rest as not evaluated.
A worksheet that does not build answers nothing (`query diagnostics`
has its errors). `sir --repl-serve` serves it as any `query`.

What the driver may rely on from 0.18.0: the query, its kinds `value`,
`output`, `stopped` and `panicked`, their lines and their order, and that
the values and output before a `stopped` line are those `evaluate_sheet`
shows for the same lines. A worksheet whose run reads the world stops at
it, where the driver's interpreter runs it with `System.fake`. So the
driver keeps its own run for the rest, or shows the mark.

**Why.** Item 2 of the round: the last role of the language server on the
Rust front end, beside `sisu.explainBorrow`, which R187 answers.

**In stage 1.** `interp` (`Interp.printed`, `run_sheet`, `print`), `main`
(`instrumented_sheet`, taken out of `worksheet_answer`; `held_input`,
taken out of `held_build`; `sheet_values`, `SHEET_BUDGET`, `sheet_lines`;
`query_command`'s `values`).

**What it changes.** No output of the corpus. The worksheet
`cases/repl/values.ws.sisu`, which the class `ws` renders as `sisu ws`
does, is asked by `cases/findings23/served.requests` with
`cases/repl/sheet.ws.sisu` (every value as `sisu ws` shows it) and
`wrong.ws.sisu` (nothing).

**Decision 1121**, for docs/architecture.md:

1121. `query values FILE` answers a worksheet's values and printed lines
      by the line each statement ends on, from the program `sir
      --worksheet` writes run by stage 1's evaluator, with the line where
      it stops `stopped` or `panicked`, so that the language server's
      inline values need no Rust front end. Stage 1 decides it (RULES.md,
      R196).

## R197. `@fix_inline`'s parentheses only where precedence needs them (decision 1122)

**The rule.** The manual's finding (chapter 28): R177's fix wrote an
argument that is more than a name in parentheses wherever it landed, so
`to_celsius(sauna + 4.0)` of `return celsius_of(f)` became
`celsius_of((sauna + 4.0))`, which `sisu fmt` keeps. Now an argument
stands without parentheses where its parameter's use fills a place whole:
after `(`, `[`, `{`, `,` or a keyword argument's `=`, before `)`, `]`,
`}` or `,`, or the expression itself. A generator keeps its parentheses,
since a call's other arguments would take it apart. The whole expression
is in parentheses only where it is more than a chain and the call does
not stand alone. A call stands alone after `(`, `[`, `{`, `,`, an
assignment's `=`, `return` or the start of its line, and before `)`,
`]`, `}`, `,`, a comment or the end of the line. So `print(warmer(sauna))`
becomes `print(sauna + 4.0)` and `warmer(sauna) * 2.0` becomes `(sauna +
4.0) * 2.0`.

Fixed on the way: an argument with a `-` after its start, `half - 1`, was
taken for a negative number, written as it was and used more than once.
`doubled(half - 1)` of `return n * 2` became `half - 1 * 2`, which means
another thing. A `-` is a sign only as the argument's first token.

**Why.** Item 3 of the round, the manual's finding of chapter 28.

**In stage 1.** `typeck.fix_inline` (`InlineForm.delimited`, `inlined`,
`placed`, `stands_alone`, `opening`, `closing`, `generator`, `plain`),
`typeck` (`inline_call`).

**What it changes.** The case `cases/findings23/fix_inline_parens.sisu`
(its `.out`, `.diag` and `.fixed`, and `fix --dry-run` of it in
`served.requests`). `cases/findings21/fix_inline.fixed` and its `.diag`
are as they were: each of their fixes is a chain, or stands where
precedence needs its parentheses. The manual's transcripts of chapter 28
that show `celsius_of((sauna + 4.0))` are made again by its writers.

The Rust front end has no `@fix_inline` (R177), so its check of the case
has no warning.

- Differs: `compiler/stage1/cases/findings23/fix_inline_parens.sisu`: ids, human

**Decision 1122**, for docs/architecture.md:

1122. `@fix_inline`'s fix writes an argument, and the whole expression, in
      parentheses only where precedence needs them, not where it fills a
      call's argument, an element or the place of a call that stands
      alone. Stage 1 decides it (RULES.md, R197).

## R198. `E0279` and `E0280`: the gap's offsets whole, and fields of numbers to fill it (decision 1123)

**The rule.** The manual's finding (chapter 27): R182's note suggested a
field `_pad1: Array[u8, 3]`. 03 does not make an `Array` of a `Copy`
element `Copy`, and stage 1 does not, so a struct that took the note was
`E0242` and `E0244` under its `@derive(Copy, Plain)`. The gap's offsets
also ended at the first byte after it. Now the offsets are the gap's first
and last byte, "at offsets 1 to 3" for the three bytes 1, 2 and 3. The
note, `resolve.plain-padding-fill`, names fields of numbers that fill
the gap at C's offsets, each the widest unsigned number its offset's
alignment allows and the bytes left hold, named by its offset: "fields
of their own there, `_pad1: u8, _pad2: u16`, make those bytes the
struct's, or the fields ordered from the widest leave none". The struct
that takes it derives `Plain` and `Copy`.

**Why.** Item 4 of the round, the manual's finding of chapter 27.

**In stage 1.** `typeck.instances` (`plain_gaps`, `padding_note`,
`padding_fields`, with a test), the catalog (`resolve.plain-padding-fill`,
whose parameter is `fields`, with a comment).

**What it changes.** `cases/findings21/plain_padding.diag` is blessed
again (the offsets' ends, `4` and `12` now `3` and `11`). The note is not
in the conformance form, so the case
`cases/findings23/plain_filled.sisu` (its `.out`) is the struct with the
note's fields, which derives both. The manual's transcripts of chapter 27
that show `E0279` or `E0280` are made again by its writers.

The case imports `Plain` from `std.mem`, stage 1's (R5), which the Rust
front end answers with `E0220`, as for `plain_padding.sisu` (R182).

- Differs: `compiler/stage1/cases/findings23/plain_filled.sisu`: names, ids, human, doc, doc-output

**Decision 1123**, for docs/architecture.md:

1123. `E0279` and `E0280` name a gap's first and last byte, and their note
      names fields of numbers at C's offsets that fill it, which are
      `Copy` where an `Array[u8, N]` is not. Stage 1 decides it (RULES.md,
      R198).

## R199. `fix`'s status: 1 when errors without a fix are left (decision 1124)

**The rule.** The manual's finding (R16): `fix` ended with status 0 after
applying what it could while errors without a fix were left, so a script
could not tell a program it fixed from one that still does not compile.
Now `fix` checks the text its fixes leave (the last round's check, or
one more when a round stopped at a fix that undoes another or at the
last round). When an error is left, it says so after its lines on
standard error, `D0190`: "`fix_left.sisu` does not check after the
fixes: errors that no fix applies to are left (1), which `sisu check`
shows". It ends with status 1, as `check` does at an error, with
`--dry-run` too, for the text it would write. Warnings left do not count.
Its other statuses are as R176 gives them: 0 when the program checks
after the fixes (or had nothing to fix and checks), 2 for another option,
and the usage error. 10 names no status for `sisu fix`; 1 is the
command line's status for errors (`check`, `build`).

What the driver may rely on from 0.18.0: the status 1 with `D0190` and
the file written as before.

**Why.** Item 5 of the round, the manual's finding.

**In stage 1.** `main` (`fix_command`), the catalog (`D0190`).

**What it changes.** The case `cases/findings23/fix_left.sisu` (its
`.diag` and `.fixed`, which holds `D0190`, and `fix --dry-run` of it in
`served.requests`). No other case of `fix` leaves an error.

**Decision 1124**, for docs/architecture.md:

1124. `sisu1 fix` ends with status 1 and `D0190` when the program still
      has an error after its fixes, 0 when it checks. Stage 1 decides it
      (RULES.md, R199).

## R200. `@fixture`: a test's parameter set up by a call and torn down by its drop; `Expectation` decided against (decision 1125)

**The rule.** 17, "Fixtures": "a `fixture` function with a scope is
requested by parameter name (pytest's model, typed)". A function marked
`@fixture` that takes nothing is a fixture of its module. A test of the
module (or of a suite in it) that names it as a parameter, `test
"rejects unknown fields"(schema: Schema):`, receives what it returns. The
front end carries it whole. The type checker's input has the parameter
out of the test's signature, and the test's body starts with `let
schema: Schema = schema()`, in the parameters' order. So the fixture is
set up before the test and torn down after it, when the local is dropped
at the body's end, also when the test fails. The runner sees a test that
takes nothing, the real `System`, or the fakes of the capabilities it
names, and needs nothing new. A parameter whose type is not the
fixture's is `E0300` at the parameter. A `@fixture` with a parameter, or
a method, is `E0281`, `type.fixture-signature`, at the attribute, and a
test that names it is `E0702`, a parameter the runner cannot give.

`scope="module"` is taken and does not change the lowering: the value is
made for each test that names it, as a test's own would be. A value made
once for a module's tests needs the runner to keep it between tests, which
it runs in parallel and in random order (17, "Isolation and
parallelism"). That is what stage 0 must do for 17's scope (below).

**Decided against**, for `Expectation`: 17's `std.test.Expectation`
"takes a compile-time expression tree (reflection over the argument
expression), so a library can define `assert_approx` or `assert_json_eq`
with the same output". 08's reflection has types, fields, variants,
functions and attributes, and no expression. A tree of the argument
expression needs a reflected expression type in `std.reflect`, its
syntax at a call (the owner's, as compile-time types are, R155), and
the lowering `assert` has for its parts. A struct of an expected value and
a text, which is all `std.test` could hold now, would not be 17's form
and would give a second spelling of `assert x == y`. So the round leaves
it for the round that takes compile-time types.

**What stage 0 must do.** Nothing for the lowering. For `scope="module"`:
the runner would make the fixture's value once per module, before the
first of its tests that names it, and drop it after the last. Stage 1
would then lower such a parameter into a call the runner answers, not the
body's `let`.

**Why.** Item 6 of the round.

**In stage 1.** The new module `typeck.fixtures` (`lower`,
`fixture_names`, `lower_item`, `lower_test`), `typeck` (`check_program`
lowers each module built with its tests), `typeck.collect` (`E0281`),
`diag` and the catalog (`E0281`).

**What it changes.** The cases `cases/findings23/fixtures.sisu` (its
`.out` and `.tests`: two tests, one with a capability and two fixtures;
a copy whose test fails showed `set up scratch`, the test's own line and
`dropped scratch`, in that order, under "what the test printed") and
`fixture_errors.sisu` (its `.diag`). The Rust front end answers a
fixture's parameter with `E0702`.

- Differs: `compiler/stage1/cases/findings23/fixtures.sisu`: ids, human
- Differs: `compiler/stage1/cases/findings23/fixture_errors.sisu`: ids, human, doc, doc-output

**Decision 1125**, for docs/architecture.md:

1125. A test's parameter named after a `@fixture` function of its module
      that takes nothing is lowered into the test's body as a `let` of
      the fixture's call, dropped at its end; another `@fixture` is
      `E0281`, and `scope="module"` waits for the runner. 17's
      `Expectation` waits for reflected expressions. Stage 1 decides it
      (RULES.md, R200).

## R201. The sharing of instances in `--explain-size`: decided against (decision 1126)

**The rule.** 03, "Dispatch": "A per-package instantiation budget,
reported by `sisu build --explain-size`", and the sharing of
instantiations "whose types have the same layout". R108 left it, and the
manual lists it among what is missing beside 03. Stage 1's lowering makes
each instance once per program, by its generic function and its type
arguments (`json.field#Mood`). Every instance is shared by each package
that asks for it, and no two of another layout are ever one. What it does
not keep is who asked. A request's site is known while the instances are
requested (`Request.Instance`'s `site`), but lowering does not carry it
into the SIR, and the executable `--explain-size` reads has symbols, not
packages. Decided against, for the round: the report needs the
requesting packages of each instance kept through lowering and written
in the SIR (a line per instance, which stage 0's reader and the backend's
symbols would carry), a budget in the manifest (09 has none), and 03's
sharing by layout, which neither lowering does. Each is a change to the
SIR, the manifest and the backend beside the front end. With R195 the
driver has the SIR of each build, so a count of instances per generic
function is the driver's to print once the SIR names packages.

**Why.** Item 7 of the round.

**What it changes.** Nothing.

**Decision 1126**, for docs/architecture.md:

1126. `--explain-size`'s instances per package and their sharing wait for
      the SIR to name the packages that request each instance, a budget
      in the manifest, and sharing by layout in the lowerings. Stage 1
      decides it (RULES.md, R201).

## R202. `grammar`: the parser's grammar from its `##!` comments (decision 1127)

**The rule.** The manual's finding (R20): the architecture notes' grammar
and the parser's `##!` comments are two texts that nothing checks, and
both lacked most of the declarations and statements. `sisu1 grammar`
prints the productions in the `##!` comments of the parser's modules, in
one order: `items`, `stmts`, `exprs`, `patterns`, `types`. Each module is
a line `-- TITLE (parser/NAME.sisu)`, the title the comment's first
paragraph, then the lines of its ```` ```text ```` blocks, with a blank
line between modules. It reads the modules at run time from the
compiler's directory (`--home`), as it reads the catalog. Any argument is
the usage error, and `sir --repl-serve` serves it.

The comments now hold what the parser parses that they lacked:
`bench_def`, `suite_def`, `extern_block`, visibility, generic parameters,
parameters (`self`, `*`, `/`, `*xs: T`), `where` clauses, `-> T from a`,
and the statements `return`, `assert` and `debug_assert`, `expect`,
`defer`, `with`, `unsafe:`, `yield`, `select`, `comptime` and the
contracts, each read from its function. The check of the manual's R20
and the architecture notes against the output is theirs.

**Why.** Item 8 of the round.

**In stage 1.** `main` (`grammar`, `GRAMMAR_MODULES`, `grammar_text`,
with a test; `served_command`), the `##!` comments of `parser.items` and
`parser.stmts`.

**What it changes.** No output of the corpus. The case
`cases/findings23/served.requests` holds the grammar (`grammar`) and the
usage error (`grammar extra`).

**Decision 1127**, for docs/architecture.md:

1127. `sisu1 grammar` prints the parser's grammar from its modules' `##!`
      comments in one order, so that the manual and the architecture
      notes can be checked against it. Stage 1 decides it (RULES.md,
      R202).

## R203. The merge's differences: the conformance suite's `holes/typed_holes` (decision 1128)

**The rule.** Nothing a command prints changes. Stage 0's twentieth round
added three conformance cases (decisions 1063 and 1065):

- `holes/typed_holes.sisu` reaches its typed holes in a run, and stage
  1's SIR prints each hole's closure there (R27), which the Rust front
  end's does not, so its SIR differs, as `types/typed_hole`'s does.
- `holes/holes_refused.sisu` has an error, so neither front end writes
  its SIR, and its diagnostics are the same.
- `eval/usage`, of the case kind `NAME.eval`, is run by the suite, not by
  `check.sh`.

The two `.holes` files are the suite's, which its runner compares with
`sisu check --holes`. Stage 1's answer for them is the twenty-first
round's whole report (R181), which decision 1063 says the merge blesses.

**Why.** Item 9 of the round: the merge brought the cases, and `check.sh`
found the one difference.

**What it changes.** Nothing.

- Differs: `compiler/conformance/cases/holes/typed_holes.sisu`: sir

**Decision 1128**, for docs/architecture.md:

1128. The conformance suite's `holes/typed_holes` differs from stage 0's
      front end in its SIR under R27 (a hole's closure, printed where a
      run reaches it); `holes/holes_refused` and `eval/usage` do not
      differ. Stage 1 decides it (RULES.md, R203).

## R204. `@derive(Decode)` through a field type's own impl (decision 1160)

**The rule.** The manual's finding (G7, R17): `E0608`'s note says that an
enum decodes with an `impl Decode` of its own, and such an impl does
decode the enum alone (`json.decode[Mood]`), yet R193 refused a struct
that derives `Decode` with a field of that enum, so the struct's decoder
had to be written by hand too. `json.from_json[T]` calls `T.decode_json`
first wherever `T` implements `Decode`, derived or written (R36), so the
derived decoder of the struct already reaches the enum's own impl. Now
the derive asks for `E0608` only where the enum the field holds (itself,
or as a list's or an option's element) has no `impl Decode` in its
module: `@derive(Decode) struct Day: mood: Mood, later: list[Mood],
maybe: Mood?` decodes each through `impl Decode for Mood`, and a text
that impl refuses is its `Mismatch` at the field's path (`$.mood`).

`@derive(Encode)` had no such gap: `json.encode[T]` calls
`T.encode_json` first wherever `T` implements `Encode`, so a field of an
enum with an `impl Encode` of its own is written in that form, and one
without it in the form `encode` gives every enum.

**Why.** Item 1 of the round, the manual's finding.

**In stage 1.** `typeck.instances` (`declare_derived`: R193's check asks
`has_impl` of the enum).

**What it changes.** The case `cases/findings24/decode_own_impl.sisu`
(its `.out`: a struct of three such fields decoded, encoded again, and a
mood the impl refuses). `cases/findings22/decode_enum_field.sisu`, whose
enum has no impl, keeps its `E0608`. The Rust front end's `std.json`
decodes no enum (R32's copy is stage 1's), so its check of the case
reports the instance's failure, and it has no `Decode` or `Encode` to
import (`E0204`).

- Differs: `compiler/stage1/cases/findings24/decode_own_impl.sisu`: names, ids, human, doc, doc-output

**Decision 1160**, for docs/architecture.md:

1160. `@derive(Decode)` of a struct with a field that holds an enum is
      `E0608` only when the enum has no `impl Decode` of its own; with one,
      the derived decoder calls it, as `json.from_json` does for any type
      that implements `Decode`. `@derive(Encode)` already writes such a
      field with the enum's own `impl Encode`. Stage 1 decides it
      (RULES.md, R204).

## R205. Unicode's own data files: every rule of UAX #29, and D-048's lints from `Scripts.txt` and `confusables.txt` (decision 1161)

**The rule.** R167 left three rules of UAX #29 out of `graphemes()`, and
R136 read a letter's script from its character name and the confusables
from a table of the usual look-alikes, because the generator's one source
was Python's `unicodedata`, which carries none of the properties. The
generator, `tools/unicode_tables.py`, now reads Unicode 15.1.0's own
files as well, fetched once from their pinned URLs
(`https://www.unicode.org/Public/15.1.0/ucd/`, and `confusables.txt`
from `https://www.unicode.org/Public/security/15.1.0/`, where Unicode
publishes the security data of UTS #39) into
`compiler/target/unicode/15.1.0/`, and checked to be the version's by
their headers: `GraphemeBreakProperty.txt`, `DerivedCoreProperties.txt`
(its `InCB` and `XID_Continue`), `emoji-data.txt`
(`Extended_Pictographic`), `Scripts.txt` and `confusables.txt`. The
repository holds the tables, not the files (2.3 MB), and a run of the
script writes the tables byte for byte again (`--check`, which
`check.sh` runs).

- **Graphemes.** `std.string`'s `graphemes()` follows every rule of UAX
  #29's extended grapheme clusters, GB3 to GB999 in their order:
  `Prepend` keeps what follows it (GB9b), a spacing mark stays with its
  base (GB9a), an Indic conjunct joins across its virama (GB9c,
  `InCB=Consonant`, then `Extend` and `Linker`, at least one `Linker`,
  then a `Consonant`), U+200D joins only a pictograph with its extenders
  to the next pictograph (GB11), and regional indicators pair from the
  start of their run, a break between two pairs not resetting the count
  (GB12, GB13). So `"a\u{200d}b"` is two graphemes, `a\u{200d}` and `b`,
  and `"\u{915}\u{94d}\u{937}"` (क्ष) one. The three rules that look back
  past one character keep what they need in a small state
  (`BreakState`: the regional indicators in a row, whether a pictograph
  and then a joiner end the text so far, whether a consonant and then a
  virama do). The properties are one table, `grapheme_table()`, of 733
  records `LLLLLL-HHHHHH-PP` (12 KB), each run of code points with the
  same `Grapheme_Cluster_Break`, `Extended_Pictographic` and
  `Indic_Conjunct_Break`, but for the Hangul syllables, whose `LV` and
  `LVT` alternate and are computed; it replaces R162's table of marks,
  which only the old rules read. Checked against `GraphemeBreakTest.txt`
  whole: **1,187 of its 1,187 lines** split as the file says (a program
  of every line, run from stage 1's SIR, natively and by the
  interpreter).
- **Scripts.** `W0031`'s scripts are `Scripts.txt`'s for every letter
  and mark beyond ASCII (its `Common`, `Inherited` and `Unknown` are no
  script's; Han, Hiragana, Katakana, Hangul and Bopomofo are still one,
  `CJK`), 373 ranges of 156 scripts, where the names gave 639 ranges.
  The names were wrong for 11,924 letters: 6,145 Tangut ideographs and
  170 Latin letters (the modifier letters, `ʰ`, `ʲ`, ...) had no script,
  and a script of two words was its first (`Old Italic` and `Old Turkic`
  were one script `OLD`, `Linear A` and `Linear B` one `LINEAR`, the
  Egyptian hieroglyphs `EGYPTIAN`). So `αʰ` now mixes Greek and Latin.
- **Confusables.** `W0032`'s skeleton is UTS #39's: a name's canonical
  decomposition with each character written as what `confusables.txt`
  maps it to, decomposed again. Every record of the file whose source
  can stand in a name (`XID_Continue`) is in the table, 5,147 of its
  6,311, mapped to 7,133 code points, where R136's table held 863 letters
  that read as one ASCII letter or digit. A mapping may be several
  characters (`m` reads as `rn`) and ASCII is mapped too (`1` and `I`
  read as `l`), so an ASCII name in scope is compared by its skeleton as
  well, still only when one of the two names is beyond ASCII. So `гun`
  with a Cyrillic `г` reads as `run`, and `rnоdern` as `modern`.

The tables are text of fixed-width records, searched as before;
`src/unicode_tables.sisu` is 196 KB, where it was 83 KB (the confusables
and their pool 120 KB of it), and `std/string.sisu`'s generated part
grows by 10 KB.

**Why the table is not a module of its own.** The brief asks a generated
table to stand in a module of its own. `std/string.sisu` keeps the
graphemes' table in the generated part R162 made at its end, below the
line `STD_MARK`, which only the script writes: the Rust front end, which
still reads `std/` and is frozen, knows a fixed list of std modules, so
`std.string` importing a new module would be an error in every program
that imports `std.string`, there and in its tests. The rules themselves
are `graphemes()`'s and `breaks_between`'s, rewritten in place, with the
helpers only they read (`is_control`, `is_regional`, `extends`, `jamo`)
gone.

**Why.** Item 2 of the round, R167's and R136's items left for Unicode's
own files.

**In stage 1.** `tools/unicode_tables.py` (`FILES`, `data`,
`properties`, `grapheme_records`, the scripts and confusables of
`tables`), `unicode_tables` (generated; `confusable_pool`), `text.names`
(`prototype_into`, `skeleton`, `decomposed`, with the test "a name's
scripts and skeleton"), `resolve` (`confusable`), and `std/string.sisu`
(`Graphemes`, `BreakState`, `breaks_between`, `grapheme_property`, the
constants `GCB_*` and `INCB_*`, and the generated `grapheme_table`).

**What it changes.** The cases `cases/findings24/graphemes_rules.sisu`
(its `.out`: the manual's two texts, a family, a pictograph with a mark,
a conjunct across a joiner, a virama before a Latin letter, a prepended
number sign, a spacing mark, five regional indicators and `\r\n` before
a mark) and `cases/findings24/confusables_whole.sisu` (its `.diag` and
`.out`). `cases/findings15/confusable_names.diag` is as it was.
`std.string`'s doc example of `Graphemes` asserts the manual's two
texts, and its doc comment no longer lists rules it leaves out; the
conformance suite's `doc/std_pages.doc` is blessed again for it. The
manual's two findings ("Syntax and names": the lints' tables;
"Types and traits": the three rules of UAX #29) are settled: the
scripts are `Scripts.txt`'s, the confusables `confusables.txt`'s that a
name can hold, and `GraphemeBreakTest.txt` passes whole.

The Rust front end has neither lint, and its formatter removes `from
std.string import Graphemes` as unused, as for R162's cases.

- Differs: `compiler/stage1/cases/findings24/confusables_whole.sisu`: names, ids, human
- Differs: `compiler/stage1/cases/findings24/graphemes_rules.sisu`: fmt, fmt-check

**Decision 1161**, for docs/architecture.md:

1161. `graphemes()` follows every rule of UAX #29 (GB9b's `Prepend`,
      GB9c's conjuncts, GB11's pictographs, GB12 and GB13 by the run),
      and `W0031` and `W0032` read `Scripts.txt` and UTS #39's
      `confusables.txt` and skeleton, from tables `tools/unicode_tables.py`
      writes from Unicode 15.1.0's own files, fetched by pinned URLs;
      `GraphemeBreakTest.txt` passes whole (1,187 lines). Stage 1
      decides it (RULES.md, R205).

## R206. The tz database: each zone's history in `std.time` (decision 1162)

**The rule.** The manual's finding (G9, chapter 21): `Zone.named` knew
its 52 zones by today's rules alone (decision 627), which it gave an
earlier year too, so Helsinki in July 1950, when Finland kept no summer
time, was `EEST`, and New York's 2006 March was daylight time by the
rule of 2007. A new generator, `tools/tzdata.py`, reads the system's
compiled tz database, the TZif files under `/usr/share/zoneinfo` (RFC
8536, version 2 or later, their 64-bit data), whose version `tzdata.zi`
names (`2026c` here), and writes each zone's history at the end of
`std/time.sisu`, below the line `MARK`, as `zone_history()`: a line per
zone, `NAME|ABBR/OFFSET ...|AT/K ...`, its local time types and its
changes, up to the first change from which the zone's POSIX rule gives
every later one. The rule of each zone must be the one its TZif file
ends with, which the script checks for every zone of `ZONES`, and they
are. `Zone.named` reads the zone's line into its history (`Shift`s, the
first the zone's time before its first change), and `offset_at`,
`abbreviation_at`, `local` and `format` take the change in force at a
time before the last one kept, and the rules from then on:

```text
Europe/Helsinki 1900-01-01T12:00:00Z: 1900-01-01T13:39:49+01:39 HMT 5989
Europe/Helsinki 1950-07-01T12:00:00Z: 1950-07-01T14:00:00+02:00 EET 7200
America/New_York 1974-01-15T12:00:00Z: 1974-01-15T08:00:00-04:00 EDT -14400
Europe/Moscow 2012-01-15T12:00:00Z: 2012-01-15T16:00:00+04:00 MSK 14400
```

Each line of the case is what Python's `zoneinfo` gives for the same
time, but the offset of `format`, which RFC 3339 writes to the minute.

**The size, and what is left out.** The 52 zones' history is 50 lines
(`UTC` and `Etc/UTC` have none) of 3,545 changes, 50 KB of `std/time.sisu`,
which every program that imports `std.time` parses. The whole database
is 447 zones of 18,732 changes, 278 KB in this form, more than the
normalisation tables, for zones `Zone.named` does not name; it is left
out, and the zones are `ZONES`'s, which 11's table of zones still
bounds. A change after 2037, which a TZif file leaves to its rule, is
the rule's.

**Decided against**, for the round: `std.x.tzdata` as 11 has it, a
blessed package versioned apart from the compiler that updates the data
`std.time` embeds. `std.x` packages wait for 09's registry, and a
module of its own inside `std/` would be what the Rust front end, frozen
with a fixed list of std modules, cannot import from `std.time` (R205's
reason); so the data is `std.time`'s, embedded as 11's table says ("time
zones (data embedded, updatable via `std.x.tzdata`)"), and written by the
script. `check.sh` runs `tools/tzdata.py --check`, which compares only
on a system whose tz database is the table's version, since another
version's history is another text, and says so.

**Why.** Item 3 of the round, the manual's finding.

**In stage 1.** `tools/tzdata.py`, `std/time.sisu` (`Zone.past`,
`past_at`, `Shift`, `FAR_PAST`, `history`, the generated
`zone_history`; `Zone.named`, `fixed`, `seconds_at` and
`abbreviation_at`), `check.sh` (the stale check).

**What it changes.** The case `cases/findings24/tz_history.sisu` (its
`.out`: dates before a rule changed in eleven zones, and after). `Zone`'s
doc example asserts Helsinki's offset of 1950, and its doc comment says
the history is the database's; the conformance suite's
`doc/std_pages.doc` is blessed again for it. The conformance cases of
`std.time` (`std/time_zones`, `std/zone_debug`, `capabilities/datetime`)
ask for times under today's rules and print what they printed.

**Decision 1162**, for docs/architecture.md:

1162. `Zone.named` gives each of its zones' history from the tz database
      (the system's TZif files, written into `std.time` by
      `tools/tzdata.py` with the database's version), and today's rules
      from the change after which they give every later one; the
      whole database and `std.x.tzdata` are left out. Stage 1 decides it
      (RULES.md, R206).

## R207. `sisu doc --digest`: a program's or the standard library's API as one plain text (decision 1163)

**The rule.** 10, "AI-assisted development": "`sisu doc --digest` writes
one plain-text file per package with each `pub` signature, its inferred
error set, capability parameters, contracts and first doc example ... so
that a model can hold a package's API in its context window" (N-50).
`doc --digest [PATH] [--output DIR]` and `doc --std --digest
[MODULE...]` make the pages `doc` makes and write them as one text,
printed, or with `--output` written as `DIR/NAME.txt` (`NAME` the file's
stem, the package's directory, or `std`):

```text
# digest: the API as `sisu doc --digest` writes it

# module `digest`
##! `sisu doc --digest` (RULES.md, R207; ...): each item's declaration ...

## Checks the heat.
pub def measure(heat: int) -> int ! StoveError  # inferred
    requires heat >= 0

## The log of the stove, written to `fs`.
## Capabilities: `fs` (`Fs`).
pub def save(fs: Fs, s: Stove) -> ! IoError
```

Each module is a line `# module PATH`, the first paragraph of its `##!`
lines and its re-exports as `##!` lines, then each item the page shows:
its declaration as the page writes it (attributes, header with the
inferred error set, contracts, a struct's fields, an enum's variants, the
headers of the type's impls), the fields' and impls' own doc comments
left out, with `##` lines before it as a doc comment is written: the
first paragraph of its doc, its capabilities (`Capabilities: ...`) and
holes (`Unfinished: ...`), and its first example between fences. A
type's methods are indented under it, as they are written. A package's
digest shows what another package may name, as its pages do; the
standard library's shows what is written `pub` (its pages show
everything, decision 147), so that a model sees what a program may call.
A program with an error has the pages' `D0114`. The standard library's
digest is 147 KB, about 37,000 tokens at four bytes a token, inside 10's
"about 50k for `std`"; its pages are 438 KB.

**Decided against**: `--open` (10: "`--open` serves it locally"), which
is a server for the pages and a browser to show them, the host's and the
driver's, not the front end's; stage 1 writes the pages and the digest,
and the driver may serve what `--output` wrote.

What the driver may rely on from 0.19.0: `doc --digest` and `doc --std
--digest` with their output and `--output DIR`'s one file, served.

**Why.** Item 4 of the round.

**In stage 1.** `doc` (`digest`, `digest_page`, `digest_code`,
`unlinked`, with the test "a digest's links are their texts"), `main`
(`DocArgs.digest`, `doc_with`, `digest_name`; `std_pages` and
`doc_program` take `public`).

**What it changes.** No output of the corpus. The case
`cases/findings24/digest.requests` asks the digest of
`cases/findings24/digest.sisu` (a struct with a method and its example,
an inferred set, a contract, a capability, a hole), of `std.time` and of
the package `cases/packages/doc_pages.pkg`'s `shop`, whose re-exports are
plain names. `digest.sisu`'s own pages are stage 0's.

**Decision 1163**, for docs/architecture.md:

1163. `sisu1 doc --digest` and `doc --std --digest` write a program's or
      the standard library's API as one plain text, each item's
      declaration with its inferred set and contracts after `##` lines
      of its first paragraph, capabilities, holes and first example;
      `--open` is the driver's. Stage 1 decides it (RULES.md, R207).

## R208. `sir --tests --mutants`: the mutants of `sisu test --mutate` (decision 1164)

**The rule.** 17, "Coverage and mutation": "`sisu test --mutate` runs
mutation testing on the changed region only, rerunning the tests that
cover each mutated line and suppressing arid nodes such as logging".
This is the front end's half. `sir --tests --mutants [PATH]` prints, for
each mutant of the root package's modules in the order of their places,
a line `# mutant K: FILE:LINE:COL: WHAT`, then the SIR `sir --tests`
prints of the program with that one change; `sir --tests --mutant=K`
prints the `K`th alone. The mutants are:

- a comparison's boundary moved: `<` and `<=`, `>` and `>=` (`the
  boundary changed`);
- an operator flipped: `==` and `!=`, `+` and `-`, `and` and `or`,
  `True` and `False` (`the operator flipped`);
- a negation removed: `not x` is `x` (`the negation removed`);
- a statement of one line that calls a function or assigns removed: it
  is `pass` (`the statement removed`).

Only code a run executes is mutated: the bodies of the program's
functions and methods, not its tests, suites, benchmarks, fixtures,
`comptime` functions, `@cfg` functions or declarations. The arid nodes
are left alone: a call of `print` or `eprint` or of a function of `log`,
and everything inside it, whose mutants no test is meant to kill. A
mutant that does not check (`+` of two texts flipped to `-`) has its
line with ` (it does not check)` after it and no SIR, so that the
numbers stay the places'. The line is a comment of the SIR, which its
reader passes by, so the runner splits the output at those lines or
asks for one mutant. A program with an error has `sir --tests`'s
refusal (R46), and its warnings are printed once, on standard error.

```text
# mutant 6: mutants.sisu:17:9: the statement removed: `sum += x`
sir 0
...
```

Each mutant's SIR runs as `sisu test --from-sir` runs any: of the case's
seven, the tests kill three (the negation, `False` to `True`, the
removed `sum += x`) and four survive (the boundaries of `n > high` and
`degrees >= 60`, `n + 0`, and the `and`), which a report would suggest
as tests to write.

**What stage 0 must do.** The runner's half: `sisu test --mutate` asks
stage 1 for `sir --tests --mutants` (or for each `--mutant=K`), keeps the
mutants whose place is in the changed region (the lines a diff against
the base revision touches), runs against each one the tests that cover
its line (`--cover`'s map) with `sisu test --from-sir`, and reports each
mutant whose tests all passed, by its line, as a suggestion. A mutant
whose tests fail, or stop it by a panic or a timeout, is killed.

What the driver may rely on from 0.19.0: `sir --tests --mutants` and
`--mutant=K`, the line `# mutant K: FILE:LINE:COL: WHAT` before each
mutant's SIR (with ` (it does not check)` and no SIR for a stillborn one),
in the order of the modules and their places, served.

**Why.** Item 5 of the round.

**In stage 1.** The new module `mutation` (`Site`, `sites`, `mutated`,
`place`, `fn_sites`, `is_arid`, `expr_sites`, `stmt_sites`, with the
test "a module's mutants, and the arid calls left alone"), `main`
(`sir`'s `--mutants` and `--mutant=K`, `mutant_sirs`).

**What it changes.** No output of the corpus. The case
`cases/findings24/mutants.sisu` (its `.out`), whose mutants
`cases/findings24/digest.requests` asks for, all and the seventh alone.

**Decision 1164**, for docs/architecture.md:

1164. `sisu1 sir --tests --mutants` prints each mutant of the program's
      own code (a boundary moved, an operator flipped, a negation or a
      statement removed; calls that only report left alone) as a line
      `# mutant K: FILE:LINE:COL: WHAT` and its SIR, and `--mutant=K`
      one; running the tests against each, in the changed region, is
      the runner's. Stage 1 decides it (RULES.md, R208).

## R209. `@fixture(scope="module")`: the per-test lowering kept, and what the runner must do (decision 1165)

**The rule.** R200 lowered every fixture as a `let` at the start of each
test that names it. A module's fixture made once, by the first test that
names it, and dropped after the last, needs a value that outlives one
test and a place where the module's tests end, and the SIR has neither:
its functions keep their values in their locals, with no state of a
module that a run writes, and which tests run, in which order and how
many at once is the runner's (17, "Isolation and parallelism":
parallel, in random order with the seed printed, filtered by name or
module). A cell written by the first test would be shared by tests that
run at once, which 06 rules out without a lock, and only the runner
knows which test of a module is the last it runs. So the lowering is
R200's, with `scope="module"` taken, and the rule's note says so.

**Decided against**: a module-level cell with a drop at the tests' end,
which the SIR cannot carry without the runner's help.

**What stage 0 must do.** For each module whose tests name a fixture
with `scope="module"`, the runner calls the fixture once, before the
first of those tests it runs, keeps the value for the module's tests
(a test receives it as `ref T`, read only, so tests that run at once
share it, which asks `T: Sync`), and drops it after the last of them
ends, failed or not. Stage 1 would then lower such a parameter as a
`ref T` parameter of the test's function, naming the fixture on the
test's line of the SIR, in place of the body's `let`, once the runner
reads that line; until then each test makes its own value, which a test
cannot tell from the shared one but by its cost.

**Why.** Item 6 of the round, R200's open item.

**What it changes.** Nothing.

**Decision 1165**, for docs/architecture.md:

1165. `@fixture(scope="module")` keeps R200's lowering, a value per
      test: the SIR has no module-level state and only the runner knows
      the first and last of a module's tests it runs, so a value shared
      by them is the runner's to keep. Stage 1 decides it (RULES.md,
      R209).

## R210. The merge's differences: none (decision 1166)

**The rule.** Nothing a command prints changes. The worktree is at stage
1's own head, so the merge brought no case of stage 0's this round, and
`check.sh` reports no difference that a rule of the round's items does
not list.

**Why.** Item 7 of the round.

**What it changes.** Nothing.

**Decision 1166**, for docs/architecture.md:

1166. Stage 1's twenty-fourth round brought no difference from the
      merge: every difference `check.sh` reports is a rule's. Stage 1
      decides it (RULES.md, R210).

## R211. The driver's three asks of `--defer-borrow` and `query explain`: the human form for `build`, each loan's ranges, each finding's function (decision 1180)

**The rule.** Decisions 1104 and 1106 say what the driver works around
in R187's and R189's answers, and stage 1 now gives each:

- **`build --for-run --emit-sir --defer-borrow`** writes the errors that
  stop it, those that are not the ownership rules', in the human form, as
  R189 gives it for `build` (which takes no `--format`), and as `build
  --for-run --emit-sir` without the option writes them: the diagnostics
  as `check` prints them, then `D0006`, with status 1. It wrote them in
  the conformance form, which the driver met by building again without
  the option to get the words. `sir --defer-borrow` keeps writing them in
  the form `--format` asks for. The deferred diagnostics stay in the
  conformance form, which the counterexample mode reads.
- **`query explain FILE LINE --ranges`** ends each loan's line with two
  fields more, where the loan starts and where it is last used as ranges
  with their ends, `LINE:COL-LINE:COL`, as `query.range_of` writes a
  range: `main	shared	s	21:21	l	22:21	21:21-21:22	22:21-22:22`.
  Without the option the line is R187's six fields, which the driver of
  0.17.0 to 0.19.0 reads as exactly six; the option is what keeps it
  reading them.
- **Each deferred finding names its function.** In the answer of `sir
  --defer-borrow` and `build --for-run --emit-sir --defer-borrow`, each
  finding of the ownership checker is followed by a line `  function
  NAME`, the SIR function whose statement it is at (`main`,
  `Synth.next_sample`, `main#closure0`), before its labels and fixes:

```text
7:5-7:15 E0405 borrow.mut-while-borrowed name="xs" line="6" end="8"
  function grow
  label 6:17-6:26 borrow.borrowed-by borrower="first"
```

  A diagnostic of the front end, about a declaration, has no such line.
  The line starts with spaces, as a label does, so a reader of 0.19.0's
  answer that passes labels by passes it by too.

What the driver may rely on from 0.20.0: the human form of `build
--for-run --emit-sir --defer-borrow`'s stopping errors, the same bytes
and status as the build without the option; `query explain FILE LINE
--ranges` and its eight fields; and the line `  function NAME` after each
deferred finding of both commands, the SIR function's name as the SIR
writes it; each served.

**Why.** Item 1 of the round: decisions 1104 and 1106, "What stage 1
must do".

**In stage 1.** `main` (`build_command`'s deferred answer,
`ExplainedLoan`'s `start_range` and `end_range`, `loc_range`,
`query_explain`'s `ranges`, `query_command`, `Deferred.functions`,
`deferred_answer`), `borrowck` (`finding_functions`) and `diag`
(`render_ids_in_functions`).

**What it changes.** No output of the corpus. The case
`cases/findings25/defer_functions.sisu`, two findings in two functions, is
asked by `cases/findings25/asks.requests` with both commands, with
`findings22/defer_other.sisu`'s type error and with `query explain`'s
ranges, its old form and a misspelt option.
`cases/findings22/roles.answers` is blessed again: its two deferred
answers have the line `  function main`.

**Decision 1180**, for docs/architecture.md:

1180. `build --for-run --emit-sir --defer-borrow` writes the errors that
      stop it in the human form, `query explain FILE LINE --ranges` gives
      each loan's start and last use as ranges with their ends, and each
      finding `--defer-borrow` defers is followed by a line `  function
      NAME` naming its SIR function. Stage 1 decides it (RULES.md, R211).

## R212. `E0702`'s words: a test's parameter may name a fixture (decision 1181)

**The rule.** The manual's finding (chapter 19, R15, R17): `E0702`, for a
parameter the runner cannot give a test, said that a test takes nothing,
the real `System` or the fakes of capabilities, and named no `@fixture`,
which a test may take since R200. Its message now lists the four forms:

```text
error[E0702]: a test takes nothing, the real `System` as its one parameter (`test "name"(sys: System):`), fresh fakes of the capabilities it names, one of each (`test "name"(fs: Fs, clock: Clock):`), or the values of its module's `@fixture` functions, each named after its fixture (`test "name"(schema: Schema):`)
```

And a parameter that names a `@fixture` that takes something, which is
`E0281` at the fixture, is no longer `E0702` at the test as well: the one
mistake has one error, the fixture's.

**Why.** Item 2 of the round, the manual's finding.

**In stage 1.** The catalog (`E0702`'s text, beside a comment naming the
rule), `typeck` (the test's signature passes by a fixture `E0281`
refuses) and `typeck.collect` (`Program.refused_fixtures`).

**What it changes.** The case `cases/findings25/test_params.sisu` (its
`.diag`, and its words in `asks.answers`): a fixture's value, a fixture
that takes something and a number. `cases/findings23/fixture_errors.diag`
is blessed again without its `E0702`. The Rust front end reads the same
catalog, so its words of `E0702` change too, and it answers a fixture's
parameter with `E0702` as before, and writes the case's pages, where
stage 1 refuses them for its `E0281`.

- Differs: `compiler/stage1/cases/findings25/test_params.sisu`: ids, human, doc, doc-output

**Decision 1181**, for docs/architecture.md:

1181. `E0702` names a module's `@fixture` among what a test's parameter
      may be, and a parameter that names a fixture `E0281` refuses is
      that error alone. Stage 1 decides it (RULES.md, R212).

## R213. A contract's generated test calls the prelude's `check` by a name the program cannot shadow (decision 1182)

**The rule.** Found by the twenty-fourth round: the test `contract of
NAME` that the compiler writes for a function with a contract (07,
"Contracts as properties"; stage 1's `typeck.arbitrary`) calls the
prelude's `check` on the function's cases, and the module it writes is
looked up through the module it serves, so a program's own function
named `check` shadowed the prelude's there, and the test failed with
`E0305` in `<arbitrary>/main.sisu`. The written module now calls
`__prelude_check(...)`, and `within_ulps` of a `@refines` test
`__prelude_within_ulps(...)`, names bound in the written module itself
to the prelude's functions, which its own items come before the served
module's in every lookup; they are not copied into the served module,
so the program sees neither. The program's `check` is then the function
under test, as its contract test calls it.

**Why.** Item 3 of the round.

**In stage 1.** `typeck.arbitrary` (`PRELUDE_CHECK`,
`PRELUDE_WITHIN_ULPS`, `declare_arbitrary`'s written module).

**What it changes.** The case `cases/findings25/check_shadowed.sisu`
(its `.out` and `.tests`): a `check` with `requires` and `ensures`,
whose contract test passes. The Rust front end reports the `E0305`.

- Differs: `compiler/stage1/cases/findings25/check_shadowed.sisu`: ids, human, doc, doc-output

**Decision 1182**, for docs/architecture.md:

1182. The tests the compiler writes for contracts and `@refines` call the
      prelude's `check` and `within_ulps` by names bound in the module
      it writes, which a program's own function of the name does not
      shadow. Stage 1 decides it (RULES.md, R213).

## R214. The printed grammar against the parser: the five places, and a case of their forms (decision 1183)

**The rule.** The manual's narrowed finding (R20): `sisu1 grammar`
printed less than the parser reads in five places, which the manual's R20
page names. The `##!` comments of `parser.items` and `parser.types`, and
so the printed text, now say what the parser's functions read there, each
read from its function:

- **an attribute's dotted name**: `attribute := '@' name ('.' name)*
  ('(' args? ')')? NEWLINE` (`attribute`), the parentheses empty or not;
- **an error set's generic parameters**: `error_def := 'error' name
  generic_params? ':' ...`, as an enum's (`enum_like`);
- **the members of an enum or an error set**: `variant_member :=
  attribute* (variant | '..' NEWLINE | 'pass' NEWLINE | item_body)`
  (`variant_member`), beside a struct's `struct_member`, which holds any
  declaration, `pass` and the `invariant` contract, and the `member` of
  an impl, a trait and an `extern` block, any declaration, `pass` and a
  trait's constant (`impl_member`), where the text had methods alone;
- **the forms of a type**: `path_type` with `Self` and `type`,
  `fn_type`, `tuple_type` with its labelled parts (`type_element`),
  `'(' type ')'`, `dyn` of any primary type, `ptr mut T` and `impl Trait
  for T` (`primary_type`, `paren_types`);
- **a trait's constant**: `trait_const := 'const' name (':' type)? ('='
  expr)? NEWLINE`, with or without its value, and a `const_def` whose
  type `E0219` asks for.

The text also names what it lacked beside them: `item_body`, `import c
"header"` with `with`, a parenthesised `from` import, and `self: T`.

How the two are kept together: each production names the parser's
function that reads it, and is changed with it. The cases
`cases/findings25/grammar_forms.sisu` and `grammar_attribute.sisu` write
each form of the five places once and run, so that `check.sh` parses
them in every class against stage 0's parser too, and
`cases/findings23/served.answers` holds the printed text, so that a
change of a comment is a difference `check.sh` shows until it is blessed.
A case that parses every example of the manual's R20 page is the
manual's to run, since the page's blocks are grammar, not Sisu.

**Why.** Item 4 of the round, the manual's finding.

**In stage 1.** The `##!` comments of `parser.items` and `parser.types`.

**What it changes.** No output of the corpus.
`cases/findings23/served.answers` is blessed again with the new text.
Where the new cases differ from the Rust front end it is under earlier
rules: `grammar_forms.sisu`'s trait constant, which R37 gives a trait,
is in its AST and its page, and `grammar_attribute.sisu`'s `@lint.allow`
is R30's `W0022`.

- Differs: `compiler/stage1/cases/findings25/grammar_forms.sisu`: ast, doc, doc-output
- Differs: `compiler/stage1/cases/findings25/grammar_attribute.sisu`: names, ids, human

**Decision 1183**, for docs/architecture.md:

1183. `sisu1 grammar` prints what the parser reads of an attribute's
      dotted name, an error set's generic parameters, the members of an
      enum, an error set, an impl, a trait and an `extern` block, every
      form of a type and a trait's constant, each production read from
      its function, with a case that writes each form. Stage 1 decides it
      (RULES.md, R214).

## R215. `@no_alloc` checked, through the program's call graph (decision 1184)

**The rule.** The manual's finding (chapter 27): a program says that a
function allocates nothing with `@no_alloc`, which nothing checked
(`W0030`). 04, "Resource attributes", gives the rule: "no heap
allocation, which means no construction or growth of `list`, `dict`,
`String`, `Box` or `Shared` and no allocating callee", checked "over the
call graph", with "panic paths exempt", and D-059: "a `dyn` call or a
call through a non-generic closure parameter is an error because it must
be assumed to allocate". Stage 1 checks it on the program's SIR, after
the ownership checker, as a pass of its own (`borrowck.alloc`): in a
function marked `@no_alloc`, each statement that allocates, or that calls
a function that may, is `E0282`, `type.no-alloc`, at the statement:

```text
error[E0282]: `render` is `@no_alloc`, but `Synth.next_sample` may allocate
  --> no_alloc.sisu:19:18
   |
19 |         out[i] = synth.next_sample()
   |                  ^^^^^^^^^^^^^^^^^^^
  = note: `Synth.next_sample` calls `list.push`, at no_alloc.sisu:13:9
```

What allocates: a `list`, `set` or `dict` made (a literal, a
comprehension), a `dyn` value boxed, text joined by `+`, a `.copy()` of a
value that owns memory (a collection, a text, a `Shared`, a number of any
size, or a struct, enum or tuple that holds one), a closure that holds
what it captures by value or that a task runs, an intrinsic that grows a
collection, a text or the heap (`list.push`, `list.insert`,
`list.extend`, `dict.insert`, `dict.entry`, `set.add`, `String.push`,
`String.push_char`, `Shared.new`, `bytes.concat`), and an intrinsic that
returns a new value that owns memory (`str.to_string`, `fmt.new`, which
an interpolation calls); one that moves a value out of a collection or
hands back one it holds (`list.pop`, `dict.get`, `Shared.get`) does not.
A call of a function of the program or of the standard library is
checked through its body, to any depth, the callee's first allocation
the note's (`` `F` HOW, at FILE:LINE:COL ``); a trait's method is
checked through the impl the call's types name. A call through a
function value, and one of a trait's method on a type only the run knows
(a type parameter, a `dyn` value, a capability's handle), is the error
too, with the note `type.no-alloc-unknown`. The blocks from which every
path ends in a panic (an `assert`'s message, an index's), and the
cleanup that drops on the way out, are exempt. A recursive call is
taken to allocate nothing more than the rest of its body.

`@no_alloc` no longer has `W0030`; `@no_block` and `@stack` keep it.

What the typed program cannot tell, and the rule leaves: 04 checks
"after monomorphisation", with errors at each instantiation, where SIR
keeps a generic function generic, so a `@no_alloc` generic function's
call of a trait's method on its type parameter is the error at the
function rather than at an instantiation that might allocate nothing;
the `list[T, A]` allocators of D-060 do not exist (the manual's other
finding), so no collection is allocation-free by its allocator; and
whether a closure's environment fits in its value is the backend's
(`env_words`), so a closure that captures by value is taken to allocate.

**Why.** Item 5 of the round, the manual's finding.

**In stage 1.** The new module `borrowck.alloc` (`check`, `allocates`,
`allocation`, `grows`, `moves_out`, `owns_memory`, `captures_by_value`,
`panic_blocks`), `borrowck` (`check_with` runs it),
`typeck.collect` (`Program.no_alloc`), `lower` (`sir.Program.no_alloc`,
not written), `resolve` (`unchecked_promise` without `no_alloc`), `diag`
and the catalog (`E0282` and its two notes). `stage2`'s reader fills the
new fields of `sir.Program` with nothing.

**What it changes.** The cases `cases/findings25/no_alloc.sisu` (its
`.diag`, and its words in `asks.answers`: a callee's `list.push`, a list
literal, an interpolation and a function value) and `no_alloc_kept.sisu`
(its `.out`: a function that reads, writes in place, calls a helper and
asserts with a formatted message, which checks). The manual's
`solutions/27-systems/mixer.sisu` checks. `cases/findings13/
unchecked_attributes.diag` is blessed again without the `W0030` of
`@no_alloc`, which the Rust front end still reports.

- Differs: `compiler/stage1/cases/findings13/unchecked_attributes.sisu`: ids, human
- Differs: `compiler/stage1/cases/findings25/no_alloc.sisu`: ids, human

**Decision 1184**, for docs/architecture.md:

1184. `@no_alloc` is checked on the program's SIR through its call graph,
      the standard library's functions included: a statement that makes,
      grows, copies or boxes what owns memory, or calls a function that
      may, or calls through a function value or a type only the run
      knows, is `E0282` at the statement; the paths that only panic are
      exempt, and the attribute no longer has `W0030`. Stage 1 decides it
      (RULES.md, R215).

## R216. `sir --tests --cover`: the program's regions, the front end's half of `sisu test --cover` (decision 1185)

**The rule.** 17, "Coverage and mutation": "`sisu test --cover` reports
uncovered regions from the compiler's own instrumentation". This is the
front end's half. `sir --tests --cover [PATH]` prints the SIR `sir
--tests` prints, then a line `# regions` and a line for each region of
the root package's modules, in the order of the modules and of their
places:

```text
# regions
# region cover.sisu:5:5-10:21 statement grade bb0:0
# region cover.sisu:6:9-6:22 branch grade bb2:0
# region cover.sisu:6:9-6:22 statement grade bb2:0
```

A region is a statement of the program's own functions and methods, or
a branch, the body of an `if`'s branch or its `else`, of a loop, of an
arm of a `match` that stands as a statement, of a `defer`, a `with` or an
`unsafe:`, as `FILE:LINE:COL-LINE:COL` from its first character to its
last (a branch from its first statement), then `statement` or `branch`,
and the statement of SIR that starts it: its function, its block and its
place in the block, `bbB:S`, the block's statements counted from 0 and
its terminator after them. The SIR statement is the first one inside the
region, by its position, or before it in the same block. A region
lowering wrote no statement of is `none`. Tests, benchmarks, fixtures,
compile-time and `@cfg` code are left out, as they are from the mutants
(R208). The lines are comments of the SIR, which its reader passes by.
A program with an error has `sir --tests`'s refusal (R46).

**What stage 0 must do.** The runner's half: `sisu test --cover` asks
stage 1 for `sir --tests --cover`, runs the tests from the SIR with each
statement counted (the counting `test --defer-borrow`'s run does,
decision 1143), marks a region reached when its statement ran in any
test, and reports the regions no test reached, by their `FILE:LINE:COL`,
as a map and never a gate (17), with `none` regions left out.

What the driver may rely on from 0.20.0: `sir --tests --cover`, the line
`# regions` after the SIR and the lines `# region SPAN KIND FUNCTION
bbB:S` or `# region SPAN KIND none`, served.

**Why.** Item 6 of the round.

**In stage 1.** The new module `coverage` (`Region`, `regions`,
`SirPlace`, `sir_places`, `table`), `main` (`sir`'s `--cover`,
`cover_table`).

**What it changes.** No output of the corpus. The case
`cases/findings25/cover.sisu`, an `if` with an `elif` and an `else` and a
`for`, is asked by `asks.requests`.

**Decision 1185**, for docs/architecture.md:

1185. `sisu1 sir --tests --cover` prints after the SIR a table of the
      program's regions, each statement's and branch's span and the SIR
      statement that starts it, so that the runner can report the regions
      no test reached. Stage 1 decides it (RULES.md, R216).

## R217. `sisu doc`'s examples with their values: decided against again (decision 1186)

**The rule.** Nothing changes: a doc example is shown as written.

**Decided against**, for this round, with what it needs: 10 has `sisu
doc` render each example's expression statements with their values. The
evaluator now runs a worksheet whole (R196, `interp.run_sheet`), but an
example is not a worksheet: it is the body of a test of its module
(R185), which imports the module's items, often by the package's path,
and a value needs that test's program built and instrumented as `sir
--worksheet` instruments a script (`__ws_show` after each expression
statement), then run, for each example of each page. The examples of the
standard library print and take capabilities (`Fs.in_memory()`,
`System.fake()`), where the evaluator stops at the world, so most of
them would show no value; and the pages of every program with an
example, and 197 examples of the standard library's pages, would differ
from stage 0's `sisu doc` for the doc classes to list. The round left it
for the round that gives the evaluator the fakes `sisu ws` runs against,
and builds an example's program as the worksheet builder builds a
script's.

**Why.** Item 7 of the round, R139's decided-against item.

**What it changes.** Nothing.

**Decision 1186**, for docs/architecture.md:

1186. `sisu doc`'s rendering of an example's values waits for an
      example to be built and instrumented as a worksheet is and for the
      evaluator to run against the fakes of capabilities, since most
      examples print or take one. Stage 1 decides it (RULES.md, R217).

## R218. The merge's differences: the conformance suite's `unsafe/tuple_pointer` (decision 1187)

**The rule.** Nothing a command prints changes. Stage 0's twenty-first
round added two conformance cases (decision 1100), and its twenty-second
none:

- `unsafe/tuple_pointer.sisu` imports `std.mem` and writes `unsafe` code,
  stage 1's (R4, R5), which the Rust front end answers with `E0220` and
  `E0900`, so its AST, names, SIR, diagnostics and pages differ as those
  of `unsafe/narrow_pointer` do (R184).
- `std/narrow_tuples.sisu` does not differ.

What each does is what its `.out` says, from stage 1's SIR
(`behaviour`), interpreted and natively.

**Why.** Item 8 of the round: the coordinator's run of `check.sh` on the
main tree found `unsafe/tuple_pointer` unlisted in seven classes.

**What it changes.** Nothing.

- Differs: `compiler/conformance/cases/unsafe/tuple_pointer.sisu`: ast, names, sir, ids, human, doc, doc-output

**Decision 1187**, for docs/architecture.md:

1187. The conformance suite's `unsafe/tuple_pointer` differs from stage
      0's front end under R4 and R5, as `unsafe/narrow_pointer` does, and
      `std/narrow_tuples` does not differ. Stage 1 decides it (RULES.md,
      R218).

## R219. Fixtures on the `test` lines of the SIR, and a module's fixture a `ref T` parameter the runner gives (decision 1188)

**The rule.** Decision 1147 and R209: the runner would keep one value
for a module's tests if the SIR named each test's fixtures and the test
took the value as a parameter. `sir --tests --module-fixtures [PATH]`
lowers so:

- Each test's line names its fixtures after its function (and after `in
  "MODULE"` with `--modules`), before its `tags`, in the order of its
  parameters: ` fixture NAME FUNCTION test` for a fixture of the test,
  which its body makes and drops as R200 lowers it, and ` fixture NAME
  FUNCTION module K` for one with `scope="module"`, which the test's
  function takes as its parameter `K` (counted from 0 among the
  parameters it keeps, the capabilities among them), of type `ref T`.
  NAME is the parameter's and the fixture's name, FUNCTION the SIR
  function that makes the value.

```text
test "within the limits" test#0 fixture limits limits module 0
test "a start within them" test#1 fixture start start test fixture limits limits module 0
```

- A module's fixture is read only by its tests, which share it while
  they run at once, so what it returns must be `Sync` (06): `E0283`,
  `type.module-fixture-sync`, at the attribute, with or without the
  option, since `scope="module"` says the value is shared.

Without the option the lowering is R200's, a `let` in each test's body,
and the lines are as before, so the runner of 0.19.0, which gives a
test its capabilities alone, runs the same tests. The option is the
runner's to pass once it makes the value (decision 1147): it calls
FUNCTION once before the first test it runs that names the fixture,
keeps the value, passes a reference to it as parameter K of each such
test, runs those tests in one interpreter, in turn, and drops the value
after the last, failed or not; a test's own fixtures need nothing.

What the driver may rely on from 0.20.0: `sir --tests --module-fixtures`,
with `--modules`, `--format=ids` and the build's options, the groups `
fixture NAME FUNCTION test` and ` fixture NAME FUNCTION module K` on the
test's line before `tags`, the parameter K of type `ref T`, and `E0283`;
served.

**Why.** Item 9 of the round: decision 1147, R209.

**In stage 1.** `typeck.fixtures` (`lower`'s `shared`, `fixture_names`
with the scope, `scoped_to_module`, `lower_test`), `syntax.ast`
(`TestDef.fixtures`, `TestFixture`), `typeck` (`check_program`'s
`module_fixtures`; the test's signature passes by the parameters the
runner gives; `E0283`), `typeck.collect` (`Program.test_fixtures` and
`module_fixtures`, a test's `ref T`), `lower` (`sir.Program.
test_fixtures`, `FixtureUse`), `sir.write` (the groups), `main` (`sir`'s
`--module-fixtures`, `Input.module_fixtures`), and the catalog
(`E0283`).

**What it changes.** The case `cases/findings25/module_fixtures.sisu`
(its `.out` and `.tests`, run without the option), whose SIR with and
without it `asks.requests` asks for, and `fixture_sync.sisu` (its
`.diag`, and its words in `asks.answers`). The Rust front end has no
`@fixture`, and answers both cases' fixtures with `E0702`.

- Differs: `compiler/stage1/cases/findings25/module_fixtures.sisu`: ids, human
- Differs: `compiler/stage1/cases/findings25/fixture_sync.sisu`: ids, human, doc, doc-output

**Decision 1188**, for docs/architecture.md:

1188. `sisu1 sir --tests --module-fixtures` names each test's fixtures on
      its line, `fixture NAME FUNCTION test` or `fixture NAME FUNCTION
      module K`, and lowers a module's fixture as the test's `ref T`
      parameter K, for the runner to make once per module; a module's
      fixture returns a `Sync` value (`E0283`). Stage 1 decides it
      (RULES.md, R219).

## R220. A memo between a server's requests: the importable std modules parsed once, each module's parse kept while it is unchanged, the standard library's bodies kept, and one check for a build (decision 1220)

**The rule.** Nothing a command prints changes; a served request does
less. Decision 1200's "What stage 1 must do": each save of `sisu run
--watch` is a served `build --for-run --emit-sir` of the whole program,
0.132 s where the Rust front end's kept queries took 0.072 s on
`live/swap`, and the editor's `query diagnostics` pays the same for each
change. The server of R158 now keeps, besides the base, what the last
requests made:

- **The std modules a program may import, parsed once.** A base made from
  the cache parsed an imported std module at each import, in each round
  of each check, and the name pass read and parsed it again for its
  names: `live/swap`'s `from std.time import Ticker` parsed `std.time`
  (71 KB) three times a save. The server's base (`typeck.
  kept_for_requests`, `Resolver.parse_importables`) parses each
  importable module once when it is made, for the resolver's imports and
  for the name pass's environment, which then reads no std file.
- **Each module's parse, while its text is unchanged.** The name pass's
  parse of a module is the type checker's too (`resolve.Parsed`, its tree,
  its syntax diagnostics and the AST, which `typeck.InputModule.parsed`
  carries, with the text it parsed, so that a module whose text changed
  since, a mutant's of R208, is parsed again), where the checker parsed
  and built each module again; and the
  server's environment keeps each parse by the text parsed
  (`resolve.ParseMemo`, `Environment.parses`), so a request reads an
  unchanged module's parse from the memo and parses only what changed.
  After each request the parses it did not use are let go
  (`forget_unused`): the memo holds the last request's program, for the
  directory it names, and an editor's unsaved text (`--text`) is one more
  text.
- **The standard library's bodies, from one request to the next.** What
  a batch keeps from one program to the next (`Kept`: the std bodies as
  the type checker typed them, the functions lowering made of them and
  their drops, R125's round and "Where the time goes"), the server keeps
  from one request to the next (`Served.kept`), for `sir`, `check`,
  `build` and `query diagnostics`; it is made again with the base. Each
  is keyed as a batch keys it, by the body and what the program's build
  agrees on (`lower.std_agreement`: the contracts checked, the prelude's
  names the program hides, the instances it asks for), so a request with
  other options lowers what it needs again.
- **One check for a build.** `build` checked a program twice, once for
  its report and once more for its SIR, the same program both times
  (`report_of`, then `sir_of`); its SIR now comes from the check that
  made the report (`report_program`, `sir_text`), as `sir --tests` takes
  its SIR from its one check (R47). And a served `build` no longer copies
  the base it is served (`build_with`).

The answers are byte for byte a fresh request's: what the memo holds is
what the same text makes, a parse a function of the text and the
catalog, which a change of the catalog or of a std text makes again with
the base (R158's key). `cases/findings26/asks.requests` asks for
`memo.sisu`'s diagnostics and its run's SIR, then for its diagnostics
with one line changed (`--text memo_changed.txt`), then for both again,
and the answers are those of a server without the memo (the round's
first build, before it, gave the same bytes); nine requests of
`live/swap`, `rt`'s 18 modules and their changed text gave the same
227,916 bytes from both.

**Measured** (natively, the release profile, the mean of 20 requests
after the first, on four cores at a load of about 6 to 9, two runs each in turn): a save of
`live/swap`, the served `build --for-run --emit-sir`, takes
0.022 to 0.029 s where it took 0.104 to 0.107 s; under callgrind a save
took 1.18 G instructions, 0.23 G of them the base's copy and drop, 0.31
G the second check and lowering, 0.12 G the three parses of `std.time`
and 0.09 G the std bodies typed again. A `query diagnostics` of a module
of `compiler/rt` (18 modules, 7,624 lines), each request with the module
changed by a line from the last (its text and `--text` in turn), takes
1.33 to 1.38 s where it took 1.42 to 1.45 s: its time is the program's
own type check (3.7 G instructions), lowering and ownership check (11 G),
which nothing kept between requests would shorten short of keeping each
function's check while its module is unchanged (below).

**Decided against**, for the round: a memo of what the resolver, the
type checker, lowering and the ownership checker make of the unchanged
modules. Each round of resolution resolves the whole program again from
a copy of the base (`check_program`), the checker's table of types is one
for the program, and a body's types, its SIR and its drops depend on the
declarations of every module it reaches, so a module's results kept
across a change need the dependencies of each body on the others'
declarations, which no phase records; the parse is the part that depends
on the module's text alone.

**Why.** Item 1 of the round: decision 1200, "What stage 1 must do".

**In stage 1.** `main` (`Served.kept`, `remade`, `forget_unused`,
`sir_with` and `sir_kept`, `check_with` and `check_kept`, `build_command`
and `build_with`, `build_program`'s one check, `sir_text`,
`query_answer`'s `held`, `name_diagnostics` and `held_named_modules`
through `Environment.parsed`), `resolve` (`Parsed`, `ParseMemo`,
`Environment.parses`, `Environment.parsed`, `NamedModule.parsed`),
`typeck` (`kept_for_requests`, `InputModule.parsed`, `syntax_of_input`)
and `typeck.collect` (`Resolver.parse_importables`).

**What it changes.** No output of the corpus. The case
`cases/findings26/memo.sisu`, with `memo_changed.txt` and
`asks.requests`.

**Decision 1220**, for docs/architecture.md:

1220. A served request reads the std modules a program may import from
      the server's base, which parses each once, and each module's parse
      from a memo of the server's while its text is unchanged; the
      standard library's bodies as a request typed, lowered and placed
      them are kept for the next, and `build` takes its SIR from the
      check that made its report. A save of `live/swap` takes
      0.022 to 0.029 s served where it took 0.104 to 0.107 s, each answer
      a fresh request's bytes. Stage 1 decides it (RULES.md, R220).

## R221. `sir --tests --std` and `test --std` naming no path: the standard library's doc examples alone (decision 1221)

**The rule.** Decision 1144: the driver writes an empty program aside to
run the standard library's examples alone. `sir --tests --std` that
names no path, run where no package is (in the working directory or
above it), lowers the examples alone, as the tests of an empty program
whose root module is `examples.sisu`, which is not read
(`examples_input`); `test --std` naming none does the same and runs
them. Inside a package each still reads the package (R185), and a path
is read as before. The SIR is the one `sir --tests --std examples.sisu`
prints of an empty file of that name: its first source line is `source 0
"examples.sisu"`, where the driver's file aside gave the path of its
directory. Its 197 tests are R185's; statuses and warnings are `sir
--tests`'s. Served, the request runs in its `--dir`.

What the driver may rely on from 0.21.0: `sir --tests --std` with no
path outside a package, with `--contracts=LEVEL`, `--format=ids`,
`--modules` and the other options of `sir --tests`, and `test --std`;
the source line `source 0 "examples.sisu"`; served. Then the file aside
of decision 1144 retires: the driver sends `sir --tests --std` from the
working directory (or `--dir`) where it found no package.

**Why.** Item 2 of the round: decision 1144's leftover.

**In stage 1.** `main` (`run_command`'s `sir`, `package_above`,
`nearest_package`, `app_input`, `examples_input`, `EXAMPLES_FILE`,
`test_command`).

**What it changes.** No output of the corpus. Checked by hand, outside a
package: `sir --tests --std` gives the bytes of `sir --tests --std
examples.sisu` of an empty file, and `test --std` passes its 197 tests.

**Decision 1221**, for docs/architecture.md:

1221. `sisu1 sir --tests --std` and `test --std` that name no path,
      outside a package, lower and run the standard library's doc
      examples alone, as the tests of an empty program `examples.sisu`
      that is not read, so that the driver's empty program written aside
      retires. `sisu1 --version` is 0.21.0. Stage 1 decides it
      (RULES.md, R221).

## R222. `@no_alloc` at each instance of a generic function (decision 1222)

**The rule.** R215's open item: 04 checks `@no_alloc` "after
monomorphisation", and the SIR keeps a generic function generic, so a
`@no_alloc` generic function's call of a trait's method on its type
parameter was `E0282` at the function, whatever impl its instances
call. Now `borrowck.alloc` checks a generic `@no_alloc` function twice
over:

- **As written**, for what allocates whatever its type arguments are (a
  list literal, a call of a function that pushes): `E0282` as before,
  named by the function alone. What depends on a type parameter, a call
  of a trait's method on it, a `.copy()` of its value, an intrinsic whose
  result holds it, a call whose type arguments name it, waits for the
  instances.
- **At each instance the program makes** (`instances`): the calls with
  type arguments that name no type parameter, in any function, and those
  of each instance's body with its arguments put for its parameters, to
  any depth (at most 4,096 functions and instances walked), as
  monomorphisation would make them. Each statement the function as
  written did not report is checked with the instance's arguments: a
  trait's method on a type parameter is checked through the impl the
  arguments name, a callee through its own instance. What allocates is
  `E0282` at the statement, named by the instance, `total[Big]`, with
  the note `type.no-alloc-instance`, where the program makes it:

```text
error[E0282]: `total[Big]` is `@no_alloc`, but `Big.Weighed.weight` may allocate
  --> no_alloc_instances.sisu:31:16
   |
31 |         sum += xs[i].weight()
   |                ^^^^^^^^^^^^^^
  = note: `Big.Weighed.weight` calls `list.push`, at no_alloc_instances.sisu:24:9
  = note: in this instance, which the program makes at no_alloc_instances.sisu:43:11
```

  An instance that allocates nothing has no error, so a generic function
  whose every instance allocates nothing checks; one the program never
  makes is checked as written only. A non-generic `@no_alloc` function
  that calls a generic one checks the callee's instance, where it
  checked the callee as written (with its trait calls on a type
  parameter as calls on a type only the run knows). A `dyn` value and a
  function value are still known only at run time (D-059).

**Why.** Item 3 of the round: R215's open item.

**In stage 1.** `borrowck.alloc` (`check_body`, `Instance`, `instances`,
`MAX_INSTANCES`, `subst_all`, `has_param`, `has_param_in`;
`allocates`, `called` and `allocation` take the instance's arguments),
`diag` and the catalog (`type.no-alloc-instance`).

**What it changes.** The cases `cases/findings26/no_alloc_instances.sisu`
(its `.diag`, and its words in `asks.answers`: `total[Big]` at its
instance, `pair`'s list literal at the function, `total[Small]` and
`pair[Small]` checked) and `no_alloc_generic_fine.sisu` (its `.out`: a
generic function whose one instance calls a method that allocates
nothing). The Rust front end answers both with `W0030`, which the
classes compare for the first, whose errors stage 1 reports in their
place.

- Differs: `compiler/stage1/cases/findings26/no_alloc_instances.sisu`: ids, human

**Decision 1222**, for docs/architecture.md:

1222. A generic `@no_alloc` function is checked as written for what
      allocates whatever its type arguments are, and at each instance
      the program makes for what depends on them, through the impls the
      arguments name; an instance that allocates is `E0282` named by its
      arguments, `total[Big]`, with a note where the program makes it.
      Stage 1 decides it (RULES.md, R222).

## R223. `sir --tests --transcripts`: the world of transcripts kept on the driver's word (decision 1223)

**The rule.** port-debt.md lists a program's transcripts among the Rust
front end's roles left. Since R120 (0.11.0) `sir --tests` keeps
`transcript_world` and `transcript_system` for a program with a
`.transcript` file in `tests/` beside its root module, and since decision
722 the driver runs the transcripts from that SIR, so the round looked
for what the driver still takes from the Rust front end, by running
`sisu test` of programs with transcripts that only stage 1 accepts (a
`dict[String, V]` looked up by a `str`, R113): a file, a package, with
`--native` and with `--defer-borrow`, each ran its transcripts from stage
1's SIR. What is left is the driver's: `transcript_sir`, the Rust front
end's build, which it takes when the SIR it has lacks the two functions
or the stage 1 it finds is older than 0.11.0 (`keeps_transcripts`), and
for `--front-end=rust`. A program without `main` is the driver's
`D0128` either way, and a program that does not check never reaches its
transcripts, since `sir --tests` refuses it (R46).

So that the driver need not agree with stage 1 on where transcripts are,
nor fall back when they disagree, `sir --tests --transcripts` keeps the
world of transcripts whatever is beside the program (`world_roots`),
after the tests, as R120 roots it; without the option stage 1 looks for
them as before. With `--defer-borrow` it does the same. A program
without `main` keeps them too, and the driver says it cannot run them.

**What stage 0 must do.** From `sisu1 0.21.0`, ask for `sir --tests
--transcripts` when `transcript::find` finds a transcript, run the
transcripts from that SIR, and delete `transcript_sir`, the
`keeps_transcripts` probe and `SISU_STAGE1_TRANSCRIPTS`; `--front-end=rust`
keeps its own build until the Rust front end goes. With that and R221,
what port-debt.md lists of `sisu test` on the Rust front end is `test
--std --defer-borrow`, which `sir --tests --std --defer-borrow` already
lowers (R189, R185), and building stage 1.

What the driver may rely on from 0.21.0: `sir --tests --transcripts` with
every option of `sir --tests`, the SIR holding `transcript_world` and
`transcript_system`; served.

**Why.** Item 4 of the round: port-debt.md, "The Rust front end's roles
still to replace".

**In stage 1.** `main` (`world_roots`, `sir_kept`'s flags).

**What it changes.** No output of the corpus. The case
`cases/findings26/world.sisu`, which has no transcript, asked by
`asks.requests` with the option (its SIR keeps the world) and without it
(none).

**Decision 1223**, for docs/architecture.md:

1223. `sisu1 sir --tests --transcripts` keeps the world of transcripts
      whatever is beside the program, so that the driver, which finds the
      transcripts, runs them from stage 1's SIR without its own build of
      the program. Stage 1 decides it (RULES.md, R223).

## R224. `test --std`'s speed: the examples' modules made once for a check (decision 1224)

**The rule.** Nothing a command prints changes. Decision 1144 measured
`test --std` at 3.1 s on stage 1 against 0.7 s on the Rust front end.
Profiled (callgrind, `sir --tests --std` of no program, 28.6 G
instructions, the base made whole among them): the program's check
7.5 G, lowering 8.9 G (the 815 std bodies the examples reach typed, 3.6
G, and lowered, 3.6 G), the ownership checker 5.5 G. Of the check, 4.2 G
was declaring the examples (`declare_std_doc_tests`) in each of its two
rounds of resolution: each round parsed the prelude and every
documented module again for its items' starts, made the examples' text
and parsed it. Now `check_program` makes each module's examples' module
once (`typeck.collect.std_docs_of`, `StdDoc`) and each round declares
them (`declare_std_doc`): 26.9 G instructions in all, the declaring
0.7 G and the making once 1.8 G, and natively 2.9 to 3.3 s of user time
where it took 3.4 to 3.6 s (three runs each, in turn, at a load of about
10).

What is left is the examples' own: the std bodies they reach, typed,
lowered and checked for ownership (18 G), which a server's second
request takes from `Kept` (R220), where a process makes them again; and
the std modules loaded for them, which the examples import.

**Why.** Item 5 of the round.

**In stage 1.** `typeck.collect` (`StdDoc`, `std_docs_of`,
`Resolver.std_docs`, `declare_std_doc`), `typeck` (`check_program`).

**What it changes.** No output of the corpus; `sir --tests --std` gives
the same bytes before and after.

**Decision 1224**, for docs/architecture.md:

1224. `sir --tests --std` makes the module of each std module's doc
      examples once for a check, where each round of its resolution made
      them again. Stage 1 decides it (RULES.md, R224).

## R225. D-060's allocators, planned; the first step decided against for the round (decision 1225)

**The rule.** Nothing changes; this round records the plan. 04,
"Allocators", 11 and D-060: collections take their allocator as a type
parameter with a default, `list[T, A: Allocator = Global]`, which
ordinary code never writes; `list.with_allocator(a)` builds one on
another allocator; a collection on a borrowed `Region` is a view; and
`try_push` and friends return `! OutOfMemory`. The manual found
`list[T, A]` with `Global` in the prelude missing (`E0208`, `E0201`),
`list.with_allocator` `E0351` and `try_push` `E0304`. Stage 1's own
`std.mem` has the `Allocator` trait, `Global`, `Region` and
`RawBuf[T, A: Allocator = Global]` (R9), and the checker takes a
defaulted type parameter on a program's own type.

The plan, in its order of rounds:

1. **The parameter, the front end's alone.** `Allocator` and `Global`
   reach the prelude (the trait stays `unsafe` to implement), and
   `list`, `dict`, `set` and `String` take a last type parameter `A:
   Allocator = Global`. Every type the checker makes of them without it
   is at `Global`, and the type table interns `list[T, Global]` as
   `list[T]`, so that a program that names none, and one that names
   `Global`, has the SIR it has now, byte for byte, and the SIR names
   only another allocator. The checker's sites that make a collection's
   type by its one argument (19 in the type checker, lowering and the
   ownership checker) keep making it so.
2. **The runtime's allocation through a value.** The runtime in Sisu
   (D-086, R9) allocates a collection's buffer through `RawBuf[T, A]`,
   whose `A` is the allocator value the collection holds; `Global`'s is
   the C allocator, a `Region`'s its bump. Natively a collection on
   `Global` keeps today's layout (its allocator is zero-sized); one on
   another holds the value beside its buffer. The interpreter's checked
   model allocates a `Region`'s blocks from the region's own range.
3. **The library's forms.** `list.with_allocator(a)` (and `dict`'s and
   `set`'s), whose result is `list[T, A]`, and `list[T, ref Region]` a
   view by the existing rule (R21's: a collection that holds a
   reference); `try_push`, `try_insert` and `try_reserve` with `!
   OutOfMemory` when the allocator gives no block.
4. **What `@no_alloc` then checks.** R215 counts a collection's growth as
   an allocation whatever its allocator; with the parameter, growth of a
   `list[T, ref Region]` allocates from the region and not the heap, so
   it passes `@no_alloc` as 04's region pattern means it to (a region
   made before the function, reset after), and `try_push` on it is the
   allocation-free form of a hot path.
5. **The `freestanding` profile** (12), which has no global allocator:
   `Global` refused there (`E1023` today refuses the profile itself).

**Decided against**, for the round: step 1. It is the front end's alone,
but not small: the prelude's four builtin collections change their
arity, and with them every impl of the prelude and of the std modules
over `list[T]`, `dict[K, V]` and `set[T]` (`impl[T] list[T]`, the
derive templates, the `Lookup` impls of R113), which must take or
default the new parameter; `std/prelude.sisu` is shared with stage 0's
agent, who edits it this round, and the Rust front end, which reads the
same prelude for the `doc` classes, would see the changed impls. Step 1
wants a round of its own that changes the prelude with the checker, so
that the shared text changes once. Step 2 needs the runtime of D-086
past R9's first slice; steps 3 and 4 follow them.

**Why.** Item 6 of the round: the manual's findings under "Ownership"
and "Types and traits".

**What it changes.** Nothing.

**Decision 1225**, for docs/architecture.md:

1225. D-060's allocators come in five steps: the defaulted parameter on
      `list`, `dict`, `set` and `String`, interned away at `Global` so
      that today's SIR stays; allocation through the collection's
      allocator value in the runtime in Sisu; `with_allocator` and
      `try_push`; `@no_alloc` counting a region's growth as none; and
      `freestanding`'s explicit allocator. The first waits for a round
      that changes the shared prelude with the checker. Stage 1 decides
      it (RULES.md, R225).

## R226. The merge's differences: none (decision 1226)

**The rule.** Nothing a command prints changes. Stage 0's twenty-third
round added the conformance case `testing/mutants` (decision 1201),
R208's program, which differs from stage 0's front end in no class:
its AST, names, types, SIR, diagnostics and pages are the Rust front
end's, and what it does is its `.out`, interpreted and natively. The
round's run of `check.sh --native --names --types --sir` reported no
difference that no rule lists but the round's own.

**Why.** Item 7 of the round.

**What it changes.** Nothing.

**Decision 1226**, for docs/architecture.md:

1226. Stage 1's twenty-sixth round brought no difference from the merge:
      the conformance suite's `testing/mutants` differs from stage 0's
      front end in nothing. Stage 1 decides it (RULES.md, R226).

## R227. D-060's allocators, the first step: the collections' allocator parameter, interned away at `Global` (decision 1260)

**The rule.** R225's first step. 04, "Allocators", 11 and D-060 write the
collections `list[T, A: Allocator = Global]`, `dict[K, V, A: Allocator =
Global]`, `set[T, A: Allocator = Global]` and `String[A: Allocator =
Global]`. Stage 1 now reads them so:

- **The prelude's headers.** The prelude stage 1 reads is
  `std/prelude.sisu` with each of the four collections' headers given its
  last parameter `A: Allocator = Global`, on the line it was
  (`std_sources.allocator_headers`, `with_allocators`), so that every
  other line, and every position the prelude's diagnostics and pages
  name, is the shared file's. The shared text keeps its headers: the
  frozen Rust front end builds `std/prelude.sisu` into its executable and
  builds stage 1 the first time, and it fills a defaulted parameter into
  every type it resolves while its literals and its builtins make a
  collection's type of one argument, so the changed text would leave it
  unable to build stage 1 (below).
- **`Allocator` and `Global` are the prelude's.** They are `std.mem`'s
  (R4), the one `from std.mem import Global` names. The base declares
  the part of `std.mem` that the allocators' signatures need, the raw
  pointers, `Layout`, `size_of`, `align_of`, `align_up`, `Allocator`,
  `Global` and its impl, and `alloc_raw` (up to `MEM_HEAD_END`), and the
  prelude's names hold `Allocator` and `Global` (the name pass's
  environment too, `allocator_names`); a program that imports `std.mem`
  gets the rest of it, `Region`, `RawBuf` and `Plain` with its impls on
  the numbers, in the same module (`Resolver.std_rest`, `complete_std`),
  so that a program that does not import it sees no method of `Plain` on
  an `f32` (`plain_size`), as before, and the definitions the base made
  take the places in the program's order of definitions that the import
  gives them (`Program.reranked`, `lower.def_rank`, whose index now
  weighs `RANK_STEP`), so that the SIR of a program that imports
  `std.mem`, the runtime in Sisu's seed `compiler/rt/rt.sir` among them,
  declares its types in the order it did. The trait stays `unsafe` to
  implement.
- **Interned away at `Global`.** The table of types records each
  collection's count of parameters and the default of its last
  (`Types.intern_default`), and makes `list[T, Global]` the type
  `list[T]` (`Types.adt`): a program that names no allocator, and one that
  names `Global`, has the types, the SIR and the output it had, byte for
  byte, and the 19 sites of the checker, lowering and the ownership
  checker that make a collection's type of its element keep making it so.
  A written `list` of one argument takes the default (`left_out`, the
  defaulted parameters of D-060 as a program's own type takes them), and
  `list` in a body annotation infers its element alone. Unifying a
  collection at `Global` with one whose allocator is a parameter or not
  yet known unifies the allocator with `Global` (`written_args`), so
  `def count[T, A: Allocator](xs: list[T, A])` takes a `list[int]`. The
  allocator is not a value the collection holds: R21's check of a
  collection of references (`E0438`) passes it by (`is_allocator_arg`),
  and `list[T, ref Region]` is a type, which step 3 makes a view.
- **Arity.** `list[int, Global, Global]` is `E0208`, "expected 2, found
  3"; the SIR names a collection on another allocator,
  `list[int, mem.Region]`.

**The impls.** The prelude's and the std modules' impls over
`list[T]`, `dict[K, V]`, `set[T]` and `String` default the new
parameter, as a written type does, so each is an impl for the collection
at `Global`, and their text does not change: every program that names no
allocator calls them as before. A collection on another allocator has
none of them (`list[int, ref Region]` does not implement `Len`,
`E0347`). Making them generic over the allocator (`impl[T, A: Allocator]
list[T, A]`) waits for step 3, decided against for this round: each of
their methods would take the allocator as a type argument the SIR names
(`list.push[int, Global]`), which the interning cannot leave out, since a
function's type arguments are not a type; and until step 2 no allocator
but `Global` has a runtime to call them on.

**What the second step needs.** The runtime in Sisu (D-086, R9) past its
first slice: `list` and `String` stored on `RawBuf[T, A]`, a collection's
value holding its allocator beside its buffer where the allocator is not
zero-sized (`Global` is, so today's layouts stay), and the runtime's
intrinsics that grow a collection (`list.push`, `dict.insert`, `String.
push`) allocating through `A.acquire` and `A.release`; the backend's
layouts of a collection with such a value; and, in the interpreter's
checked model, a `Region`'s blocks from the region's range. Step 3's
`with_allocator` and `try_push` (with `! OutOfMemory`) then build one
and grow it without a panic; the impls become generic over `A` with them.

**The Rust front end** reads the shared prelude, whose collections have
one parameter, and does not read `std.mem` (R4). For the oracle and the
`doc` classes:

- the pages of `std.core` (`sisu doc --std`) print stage 1's headers,
  `pub struct list[T, A: Allocator = Global]`, where the Rust front end's
  print `pub struct list[T]`;
- a program that names `Global` or `Allocator` without an import is
  `E0201` there, and one that imports them from `std.mem` `E0220`, as
  every program of R4 that imports `std.mem` is;
- `list[int, Global]` is `E0208` there, "expected 1, found 2".

**Why.** Item 1 of the round: R225's first step, which this round takes
since the round owns the prelude and the collection modules.

**In stage 1.** `typeck.std_sources` (`allocated`, `allocator_names`,
`allocator_headers`, `with_allocators`), `main` (`std_texts`,
`environment`), `typeck.collect` (`base_core`'s part of `std.mem`,
`MEM_HEAD_END`, `std_rest`, `complete_std`, the generics' default
interned, the written references' check), `typeck.ty` (`Types.interned`, `adt`,
`intern_default`, `written_args`, `is_allocator_arg`, `left_out`),
`typeck.check` (a type's omitted arguments, the written references' check),
`typeck.infer` (`unify`), `typeck.instances` (a type of no written
arguments), `typeck.program` (`ref_items`, `reranked`, `RANK_PHASE`,
`RANK_STEP`), `lower` (`def_rank`), `typeck` (`std_promotions`, the
eager instances' order, and the derived impls of a module the base declared in
part, which a program declares when it imports it).

**What it changes.** No program of the corpus that names no allocator.
The pages of `std.core` (the four headers). The cases
`cases/findings27/allocators.sisu` (its `.out`: a `list[int, Global]`, a
`dict` and a `String[Global]`, a function generic over the allocator) and
`allocator_errors.sisu` (its `.diag`: `E0208` and `E0347` on a
`list[int, ref Region]`).

- Differs: `--std std.core`: doc, doc-output
- Differs: `compiler/stage1/cases/findings27/allocators.sisu`: names, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/findings27/allocator_errors.sisu`: names, ids, human, doc, doc-output

**Decision 1260**, for docs/architecture.md:

1260. `list`, `dict`, `set` and `String` take their allocator as a last
      type parameter, `A: Allocator = Global` (D-060), which stage 1
      reads into the prelude's headers and the table of types leaves out
      at `Global`, so that a program that names no allocator, or names
      `Global`, has the SIR and the output it had; `Allocator` and
      `Global` are `std.mem`'s and the prelude's names, and the impls over
      the collections are at `Global` until the allocators' third step.
      Stage 1 decides it (RULES.md, R227).

## R228. The ownership checker's results kept by each function's dependencies, and the constants evaluated by one evaluator (decision 1261)

**The rule.** Nothing a command prints changes; a served request after a
change does less. R220 decided against a memo of the later phases for
want of each body's dependencies on the other modules' declarations,
which no phase recorded. This round records them for one phase, the
ownership checker, where the SIR names them, and found the larger part
of a `query diagnostics` elsewhere:

- **Each function's dependencies.** For a function of the program's own,
  the ownership checker reads, besides its SIR, the declared types it
  reaches (`Types.reached_decls`, which `Placed` keyed the standard
  library's functions by since R125), the functions it names (each it
  calls, makes a closure of, or reaches through a `call trait` by the
  impls), of a non-generic one its signature, the function without its
  blocks, and of a generic one, whose instances must-consume checks, the
  whole function and what it names in turn, to must-consume's depth
  (`deps_of`); and, of the program as a whole, every declared type, the
  intrinsics' and trait methods' facts, the impls, and where each `Drop`
  takes its value, which a finding's note names (`Facts`).
- **The memo.** `Placed`, which a batch keeps from one program to the next
  and the server from one request to the next (`Kept.placed`), now keeps
  each function of the program's own as the last check found it, with its
  dependencies, its drops placed, the findings of its moves, and those of
  its loans and of must-consume (`OwnFn`). A function whose SIR (but for
  its source's number), declared types and dependencies are the same as
  the kept one's, in a program whose `Facts` are the same, takes the
  kept results, and its loans are not walked again; any other is checked
  as before and replaces it. A name two functions share is checked each
  time. Purity and `@no_alloc`, which walk the call graph, run whole each
  time.
- **The constants by one evaluator.** `check` evaluates every constant of
  the program at compile time for its errors (R178), and made a new
  evaluator for each, with the table of the program's functions and a
  copy of the catalog (`interp.value_of`): the 244 constants of
  `compiler/rt` made 244 tables and 244 copies of the catalog, 5.4 G of a
  request's 16 G instructions, a third of its time. One evaluator now runs them all
  (`interp.stops_of`), its frames and budget set again for each.

The answers are byte for byte a fresh request's: what is kept is a
function of the inputs it is keyed by, and `cases/findings27/asks.
requests` asks for `memo.sisu`'s diagnostics, then with `scaled`'s body
changed on its line (`memo_changed.txt`), then again, `lent`'s `E0405` in
a function the change leaves as it was coming from the memo; the answers
are those of a server that answers each request alone. Twenty-one
requests of `compiler/rt`'s `src/show.sisu`, its text and a change of
one line in turn, gave the same bytes before and after, as did
twenty-one saves of `live/swap`.

**Measured** (natively, the release profile, served, the mean of 20
requests after the first, at a load of about 2): a `query diagnostics`
of `compiler/rt`'s `src/show.sisu`, each request with the module changed
by a line from the last, takes 0.82 s where it took 1.34 to 1.36 s; a
save of `live/swap` takes 0.011 to 0.014 s where it took 0.008 to 0.016
s, no change a run this short shows. Under callgrind, three requests of
`rt` took 47.8 G instructions, the base made among them: the constants'
evaluators 16.2 G before this rule's second part, the program's check 9.8
G, lowering 4.7 G and the ownership checker 6.3 G with the memo, of
which its dependencies and their comparison 2.0 G. In a batch of
`compiler/rt` three times, the ownership checker and what follows it
takes 0.18 s for each later program where it took 0.36 s.

**Decided against**, for the round: the memo of the type checker's and
lowering's results by the same records. The type checker's table of
types is one for the program, each body's types are indices into it, and
each round of resolution makes it again from a copy of the base, so a
body's types kept from another request name another table's indices;
lowering names each function by the program's instances and numbers, made
for the whole program at once (`lower.index_names`, `StdSlots`). Keeping
either needs the table of types made stable across requests, by a key of
each type rather than its index, and the instances and names numbered
per module; the records `deps_of` makes are the shape both would use,
by declaration rather than by SIR. That, a check of 3.3 G and a lowering
of 1.6 G a request on `rt`, is what stands between `query diagnostics`
and the 100 ms an editor allows, with the base's copy for each round of
resolution (R220).

**Why.** Item 2 of the round.

**In stage 1.** `borrowck` (`Placed.own_fns`, `facts`, `OwnFn`, `Facts`,
`own_reuse`, `facts_of`, `same_facts`, `deps_of`, `deps_into`,
`DEPS_DEPTH`, `check_with`), `sir` (`Eq` on `ImplDecl`, `ImplTarget` and
`IntrinsicDecl`), `interp` (`stops_of`), `main` (`const_failures`).

**What it changes.** No output of the corpus. The case
`cases/findings27/memo.sisu` (its `.diag`), with `memo_changed.txt`,
asked by `asks.requests`.

**Decision 1261**, for docs/architecture.md:

1261. The ownership checker keeps, from one program of a batch or one
      served request to the next, each function of the program's own
      with what its check reads besides its SIR, the declared types it
      reaches and the functions it names, and checks again only a
      function whose SIR or dependencies changed; and `check` evaluates
      the program's constants with one evaluator, where it made one for
      each. A `query diagnostics` of `compiler/rt` after a change takes
      0.82 s where it took 1.34 to 1.36 s, each answer a fresh request's
      bytes; the type checker's and lowering's memo waits for a table of
      types stable across requests. Stage 1 decides it (RULES.md, R228).

## R229. `E0282` names the program's own operation, once, with its fixes (decision 1262)

**The rule.** The manual's finding (its step 38): `E0282` named what
allocates by the intrinsic lowering calls, which the program never
writes ("`fmt.new` may allocate" of an interpolation, "`list.new`" of a
comprehension), reported a comprehension as two errors ("`list.new`" at
it and "`list.push`" at its element), and had no fix. Now:

- **The program's text.** A statement of the `@no_alloc` function that
  allocates is named by what the program wrote there, in backticks: the
  comprehension `[x * 2 for x in xs]`, `s + t`, `xs.push(v)`, the
  interpolation `"n is {n}"`, `s.to_string()`, a list literal, a
  `.copy()`, a closure (`Arg.Code`, the source at the statement's place).
  Where that text spans lines or is longer than 40 bytes the words name
  what it is: "the list it makes", "the set it makes", "the dict it
  makes", "the text it formats", or "a call of `push`", the method the
  intrinsic is, never the intrinsic's own name (`intrinsic_words`). A
  call of a function that allocates is named by the function, as before,
  with its note; what is known only at run time keeps its words.
- **Once.** A statement that allocates within another that does, at a
  place inside the other's, is the other's error: the element a
  comprehension pushes, the list a `push` is given (`xs.push([1, 2])` is
  one error). A call of a function inside one is still its own error.
- **Fixes.** Every error has the fix "remove `@no_alloc`, so that `NAME`
  may allocate" (`type.no-alloc-remove`), whose edit removes the
  attribute's line, offered when the attribute stands alone on it (a
  comment after it, which the edit would remove, leaves the fix out);
  an error at a list the function makes has first "take the list as a
  `mut` parameter, made by the caller where allocating is allowed, and
  set its elements in place without growing it"
  (`type.no-alloc-pass-in`), without an edit, so that `sisu fix` (R176)
  leaves such an error to the programmer. A set or a dict filled in
  place grows, and a text formatted in place does, so neither has the
  second fix.

```text
error[E0282]: `doubled` is `@no_alloc`, but `[x * 2 for x in xs]` may allocate
  --> no_alloc_named.sisu:7:12
  |
7 |     return [x * 2 for x in xs]
  |            ^^^^^^^^^^^^^^^^^^^
  = fix: take the list as a `mut` parameter, made by the caller where allocating is allowed, and set its elements in place without growing it
  = fix: remove `@no_alloc`, so that `doubled` may allocate
```

**Why.** Item 3 of the round: the manual's finding.

**In stage 1.** `borrowck.alloc` (`Allocation.quoted` and `list`,
`Found`, `within`, `Fixes`, `made_list`, `intrinsic_words`,
`method_of`), `borrowck.finding` (`Arg.Code`, `code_text`,
`CODE_WIDTH`), `sir` (`Program.no_alloc` with the attribute's line),
`lower` (`attribute_line`), `diag` and the catalog
(`type.no-alloc-remove`, `type.no-alloc-pass-in`).

**What it changes.** The `E0282` errors of every case: the `.diag` of
`cases/findings25/no_alloc.sisu` and of
`cases/findings26/no_alloc_instances.sisu`, blessed again, and their
words in `findings25/asks.answers` and `findings26/asks.answers`. The new
case `cases/findings27/no_alloc_named.sisu` (its `.diag`). The Rust front
end answers each with `W0030`, as R215 lists.

- Differs: `compiler/stage1/cases/findings27/no_alloc_named.sisu`: ids, human

**Decision 1262**, for docs/architecture.md:

1262. `E0282` names the operation the program wrote that allocates, in
      its own text (`[x * 2 for x in xs]`, `s + t`, `xs.push(v)`) or, where
      that is long, in words that name no intrinsic; a statement within
      another that allocates is the other's error, so a comprehension is
      one; and each error has the fix that removes `@no_alloc`, and one at
      a list the function makes the fix of a list its caller makes and it
      fills in place. Stage 1 decides it (RULES.md, R229).

## R230. `E0283`'s words: no order of the tests (decision 1263)

**The rule.** The manual's finding: `E0283` said a module's fixture is
shared by its tests "while they run at once", where the interpreter's
runner runs the tests that share one in turn, on one thread (decision
1202), and a native run gives each test a value of its own. The
requirement stays: 17 runs tests in parallel by default, and a value its
tests share is reached from whatever thread runs each. The words now say
what is true of both runners:

```text
error[E0283]: `counter` is a fixture of its module, one value that every test naming it shares, and tests may run in parallel (17), so what it returns must be `Sync`, and `Cell[int]` is not
```

**Why.** Item 4 of the round: the manual's finding.

**In stage 1.** The catalog (`type.module-fixture-sync`).

**What it changes.** The words of `cases/findings25/fixture_sync.sisu`'s
error in `findings25/asks.answers`, blessed again; its `.diag`, which
holds the identifiers, does not change.

**Decision 1263**, for docs/architecture.md:

1263. `E0283` says that a module's fixture is one value its tests share
      and that tests may run in parallel, which is true of both runners,
      where it said they run at once. Stage 1 decides it (RULES.md, R230).

## R231. The regions' positions: each region of `--cover` has a statement with a place (decision 1264)

**The rule.** Decision 1241's note: the runner skips a region whose SIR
statement has no place, and stage 1 wrote `none` for a region whose
statements lowered to nothing at a place of their own: a `pass`, a
`break` and a `continue` (a jump and the drops on the way, which have
none), a `defer` (registered for the scope's exits), its body when it is
`pass`, and a `let` without a value, so that a match's arm of `pass`, a
loop's `break` and the like were never in the map. With `--cover`
(`Input.cover`, `lower.lower_covered`, `Ctx.cover`) lowering gives each
of them a unit assignment at its place, `_N = const () @LINE:COL-...`,
which the runner counts when it runs (`FnLower.cover_mark`); a block's
`pass` statements are kept by the type checker beside its statements
(`TBlock.passes`), which no other phase reads. Only `sir --tests
--cover` lowers so, and only the program's own functions, so `sir`,
`sir --tests` and the standard library's bodies are as before, byte for
byte. Every region of a program now names a statement of SIR with a
place; `none` is left for a region lowering writes nothing of at all,
which no case of the corpus has.

**What stage 0 must do.** Nothing: the runner counts the new statements
as it counts any, and `cases/findings27/cover.sisu`, run by `sisu test
--cover`, maps 25 regions where the round's first build left 9 of them
`none`, which the runner skipped. No form the driver relies on is added,
and `sisu1 --version` stays 0.21.0.

**Why.** Item 5 of the round: decision 1241's note.

**In stage 1.** `main` (`Input.cover`, `lower_of`), `lower`
(`lower_covered`), `lower.names` (`Ctx.cover`), `lower.stmts`
(`block_stmts`, `cover_mark`, `break`, `continue`, `defer`, `let`),
`typeck.thir` (`TBlock.passes`), `typeck.check` (`new_block_with`,
`check_block_in_scope`).

**What it changes.** No output of the corpus but `sir --tests --cover`'s.
The case `cases/findings27/cover.sisu`, asked by
`findings27/asks.requests`.

**Decision 1264**, for docs/architecture.md:

1264. `sisu1 sir --tests --cover` gives each statement of the program that
      lowers to nothing at a place of its own (`pass`, `break`,
      `continue`, `defer`, a `let` without a value) a unit assignment at
      its place, so that every region of the map names a statement the
      runner counts. Stage 1 decides it (RULES.md, R231).

## R232. The merge's differences: the conformance suite's `unsafe/option_pointer` (decision 1265)

**The rule.** Nothing a command prints changes. Stage 0's twenty-fourth
round added two conformance cases (decisions 1240 and 1241):

- `unsafe/option_pointer.sisu` imports `std.mem` and writes `unsafe`
  code, stage 1's (R4, R5), which the Rust front end answers with `E0220`,
  `E0201` and `E0900`, so its AST, names, SIR, diagnostics and pages
  differ as those of `unsafe/tuple_pointer` do (R218).
- `testing/cover.sisu` does not differ.

What each does is what its `.out` says, from stage 1's SIR
(`behaviour`), interpreted and natively.

**Why.** Item 6 of the round: the coordinator's run of `check.sh` on the
round's first commit found `unsafe/option_pointer` unlisted in seven
classes.

**What it changes.** Nothing.

- Differs: `compiler/conformance/cases/unsafe/option_pointer.sisu`: ast, names, sir, ids, human, doc, doc-output

**Decision 1265**, for docs/architecture.md:

1265. The conformance suite's `unsafe/option_pointer` differs from stage
      0's front end under R4 and R5, as `unsafe/tuple_pointer` does, and
      `testing/cover` does not differ. Stage 1 decides it (RULES.md,
      R232).
