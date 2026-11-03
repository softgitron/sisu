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
and `doc-output`. Any other difference fails the check. What a program
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
- Differs: `compiler/conformance/cases/unsafe/gone.sisu`: ast, names, sir, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/unsafe/invalid_bits.sisu`: ast, names, sir, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/unsafe/leaked.sisu`: ast, names, sir, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/unsafe/misaligned.sisu`: ast, names, sir, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/unsafe/mistyped.sisu`: ast, names, sir, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/unsafe/no_memory.sisu`: ast, names, sir, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/unsafe/out_of_bounds.sisu`: ast, names, sir, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/unsafe/overwrite.sisu`: ast, names, sir, ids, human, doc, doc-output
- Differs: `compiler/conformance/cases/unsafe/plain_casts.sisu`: ast, names, types, sir, ids, human, doc, doc-output
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
records `memory` and a manifest that declares `extern`).

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

## R20. A float's `Debug` takes the whole format spec (decision 305)

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
and the SIR of each program that reaches a float's `Debug` impl (42 of
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
- Differs: `compiler/conformance/cases/run/format_specs.sisu`: sir
- Differs: `compiler/conformance/cases/run/proven_checks.sisu`: sir
- Differs: `compiler/conformance/cases/run/sized_integers.sisu`: sir
- Differs: `compiler/conformance/cases/std/json_decode.sisu`: sir
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

**Decision 305**, for docs/architecture.md:

305. The `Debug` impls of `f64` and `f32` that the front end writes apply a
     format spec as their `Display` impls do (`Spec.float`, `Spec.float32`),
     since the two texts are the same: `{x=:.2f}` rounds (02), where the
     precision was dropped and the `Debug` text padded; a spec of type `?`
     still pads the `Debug` text. Stage 1 decides it (RULES.md, R20).
