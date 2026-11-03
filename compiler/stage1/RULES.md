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
