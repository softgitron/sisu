# The stackless fallback, costed before it is needed

Bet 3's fallback ([16](../../proposal/16-validation-plan.md) section 3; the
bet-3 decision points of [18](../../proposal/18-next-steps.md)): if memory per idle task
exceeds 4x Go's, or stack exhaustion is common in ordinary code, task bodies
are compiled as C#-style state machines behind the same `scope`/`spawn` API.
This note says what that costs, what it does to `ref`-into-frame semantics
and to hot replacement, and what is still unknown. Strategy E of the harness
(Rust `async` on the same scheduler) and tokio are its measured stand-ins.

## What stackless means for a colourless language

Sisu has no `async`, so the compiler, not the programmer, decides which
functions can suspend. A function *may suspend* if it reaches a blocking
point: a capability call, `join`, a channel operation, a lock, or a
compiler-inserted loop yield. That is an inferred call-graph property over SIR,
which amounts to colouring without the annotations. `dyn` calls and function values
must be assumed to suspend unless their type says `@no_block` (D-059). Only
may-suspend functions become state machines. Everything else compiles as
today and runs on the worker thread's stack.

Two lowering choices decide the rest:

* **Frame placement.** Each may-suspend call needs a frame holding the locals
  live across its suspension points plus a resume state. Rust embeds a callee's
  frame inside its caller's, which needs statically known sizes and boxing for
  recursion. C# heap-allocates each frame. Sisu's choice should be *runtime-owned
  frames that never move*: allocated by the task from a per-task LIFO frame
  arena, referenced only by pointer, never values the program can copy, move
  or drop (N-07, D-058).
* **Resumption.** With nested `poll` (Rust), resuming costs O(depth) and
  recurses on the worker stack. The harness measured `naive-async` failing at
  about 86,600 levels on an 8 MiB worker stack, for both E and tokio. With trampolined
  resumption (C#, and Sisu's own interpreter), the scheduler resumes the
  innermost frame directly and a return continues the parent. Resumption is
  then O(1) and depth is bounded by memory.

## What the harness measured (smoke scale, development VM, 4 vCPU)

Per task, resident memory plus page tables; KiB = 1024 bytes
(`results/vm/2026-10-12/` and, for 1M tasks, `results/vm/2026-10-12-large/`).

| | E (stackless) | B (pooled, 1 MiB) | Go 1.24 |
|---|---|---|---|
| memory per idle task, shallow, 10k tasks | 0.65 KiB | 6.5 KiB | 2.7 KiB |
| after a 32 KiB excursion | 0.69 KiB | 34.5 KiB (6.5 KiB with D) | 34.8 KiB (5.1 KiB after GC) |
| memory per idle task, shallow, 1M tasks | 0.63 KiB | 6.5 KiB | 2.7 KiB |
| spawn+join median | 460 ns | 527 ns | 431 ns |
| 1M-deep recursion | worker-stack probe at 173k (sync) / 87k (async) levels | probe at the reservation (21.8k levels at 1 MiB) | grows |

The memory win is real: frames instead of pages, about 10x below B. Spawn cost
barely differs. Recursion does not go away: non-suspending code still runs
on the worker's stack, which every task on that worker shares, so E needs
probes on worker stacks too. Exhaustion moves; it does not vanish.

## What it does to `ref`-into-frame semantics (bet 1)

Stackful: a `ref` to a local of a blocked frame points into a stack that never
moves, so borrowing across a blocking call needs no rule. Scoped tasks
borrow from their parent's frame because the scope joins first.

Stackless with **runtime-owned, never-moving frames**: the same holds. A
suspended frame is a heap object at a fixed address, and a `ref` into it
(including from the same frame across a suspension, from a `view struct`, or
from a child task in a scope) stays valid until the frame returns. The ownership
checker needs **no change**; the self-referential-frame problem (Rust's `Pin`)
does not arise because frames are never values.

Stackless with **frames as values** (the Rust model, needed if the language ever
exposes a suspended computation as a movable object): a `ref` into a frame held
across a yield is a self-reference, and the checker must treat each yield point
as a frame boundary for escape purposes, or `Pin`-like rules appear in the
language. The bet-3 fail branch of [18](../../proposal/18-next-steps.md) asks
Phase 3 to design that rule before the corpus study. This estimate recommends
avoiding it: keep frames runtime-owned, which is also what D-058 requires (no
droppable future values, or completion I/O into borrowed buffers becomes
unsound, the tokio-uring problem `cancel-uring` reproduces as a negative
control).

## What it does to hot replacement (D-053, D-068)

Body swap at the next call works as in the stackful design: a suspended
frame keeps executing its old state machine, so old bodies are reference-counted
until no frame uses them (the interpreter already does this). The new cost:
*may-suspend becomes part of the calling convention.* An edit that makes a
function suspend where it did not (adding a capability call, a lock, a
`join`) changes every caller's code. Under "replacement only with unchanged
signatures", the may-suspend bit must enter the signature hash, so such edits
stop hot-swapping. Mitigation: debug builds compile every function as
may-suspend, a uniform convention that is slower and allocates per call, so
replacement keeps today's rules. Frame-level fix-and-resume stays out of scope
in both designs. The interpreter era is unaffected: its frames are already heap
frames (18, "The interpreter").

## Engineering estimate (one compiler engineer, native backend era)

| Work item | Weeks |
|---|---|
| may-suspend analysis over SIR (call graph, `dyn` and closures conservative, generics per instantiation) | 2 |
| state-machine lowering: liveness across suspension points, frame layout, state numbering, resume dispatch (Cranelift) | 4-6 |
| per-task LIFO frame arena with a spare chunk (avoids the hot-split thrash of segmented stacks); inline frames for known non-recursive callees | 2 |
| trampolined scheduler integration; unwinding through heap frames (drop flags per frame, Drop on unwind edges) | 3 |
| FFI: C code that calls back into Sisu and blocks needs a real stack (a stackful escape path or a dedicated thread) | 1-2 |
| backtraces, debugger and profiler support for heap frame chains (DWARF cannot see them) | 2-3 |
| hot replacement: may-suspend bit in the signature hash, uniform debug convention | 1 |
| rerun this harness on Sisu frames; corpus performance comparison | 2 |
| **total, runtime-owned frames** | **17-21** |
| extra if frames become values (checker escape rule at yield points, diagnostics; Phase 3) | +3 |

For comparison, the stackful native runtime needs an existing context-switch
crate plus strategy B or D and its probes: about 4-6 weeks on top of the
harness, most of it already written here.

## Open risks

1. **Allocation per may-suspend call.** Call-heavy code that also does I/O
   (parsers over streams, interpreters, recursive tree services) pays an arena
   bump and frame initialisation per call, where a stack push is nearly free.
   Unmeasured until the corpus runs natively.
2. **Analysis precision.** If `dyn` and capability calls make most functions
   may-suspend, the frame count approaches the call count and the memory win
   shrinks. It still beats 4 KiB pages.
3. **Worker-stack exhaustion.** Deep non-suspending recursion exhausts a stack
   that other tasks share, so it still needs probes and a clean failure.
4. **Observability.** Stack traces, debuggers and profilers were part of the
   case for colourless tasks (JEP 444). Heap frame chains need custom tooling.
5. **Leaks and D-058.** Any API that exposes a suspended frame as a value
   (a `select` over "futures", say) brings back Rust's leak problem for borrowed
   I/O buffers. The fallback is sound only if frames stay owned by their scope.
6. **Two codegen paths.** Native code would carry both conventions, plus a
   stackful path for FFI callbacks, which roughly doubles the runtime surface
   that bet 3 was meant to keep small.
