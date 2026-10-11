# Concurrency

## The question

How a program does several things at once, how it waits for I/O without
wasting threads, how tasks are started, stopped and supervised, and how
data races are prevented.

## Approaches found

| Approach | Languages | Buys | Costs | Evidence |
|---|---|---|---|---|
| OS threads + locks | C, C++, Java pre-21, C#, Rust std | Simple model, native performance | Expensive threads; races only found by tools; no cancellation model | [cpp.md](../languages/cpp.md), [java.md](../languages/java.md) |
| `async`/`await` (stackless, coloured) | C#, JS, Python, Rust, Swift, Kotlin (`suspend`), Dart | Zero-allocation state machines; works without a runtime (Rust) | Function colouring splits the ecosystem (Python's duplicate libraries, Rust's executor lock-in); Rust's `Pin`; "async Rust is hard" | [csharp.md](../languages/csharp.md), [rust.md](../languages/rust.md), [python.md](../languages/python.md) |
| Green threads / stackful coroutines (colourless) | Go, Erlang, Java 21+ (Loom), Lua, Ruby fibers, OCaml 5 (effects) | No colouring; ordinary blocking code; Java copied Go after a decade of reactive frameworks | Needs a runtime and a scheduler; stack management; FFI that blocks | [go.md](../languages/go.md), [java.md](../languages/java.md), [ocaml.md](../languages/ocaml.md) |
| Explicit `Io` value | Zig 0.16+ | Colourless without a mandatory scheduler; the implementation is swappable | `Io` is a viral parameter; stackful-vs-stackless still unsettled after three years | [zig.md](../languages/zig.md) |
| Actors with isolated heaps | Erlang/Elixir, Pony, Dart isolates | Race freedom by construction; pause isolation; supervision | Copy on send; shared data needs a second mechanism (Dart is adding one); typed mailboxes are hard (Gleam) | [erlang-elixir.md](../languages/erlang-elixir.md), [pony.md](../languages/pony.md), [dart.md](../languages/dart.md), [gleam.md](../languages/gleam.md) |
| Static data-race freedom via types | Rust (`Send`/`Sync`), Swift 6 (`Sendable`), Pony (6 capabilities) | Races are compile errors; Android data: part of what makes Rust cheap to review | Pony: too much ceremony; Swift 6.0: migration revolt, 6.2 made sequential the default; Rust: accepted because present from day one | [rust.md](../languages/rust.md), [swift.md](../languages/swift.md), [pony.md](../languages/pony.md) |
| Structured concurrency | Trio, Kotlin, Swift, Java (JEP 505/533), Rust `thread::scope` | No leaked tasks; cancellation by scope; borrowing from the parent frame is safe | Long-lived work must live in a long-lived scope | [kotlin.md](../languages/kotlin.md), [swift.md](../languages/swift.md), [java.md](../languages/java.md) |
| Cancellation retrofits | Go `context`, C# `CancellationToken` | Works | Threaded through every signature; universally regretted | [go.md](../languages/go.md), [csharp.md](../languages/csharp.md) |
| Effect handlers as the runtime primitive | OCaml 5, Koka, Effekt | Colourless, fast (OCaml: <1% overhead, faster than monadic schedulers), debuggers keep working | OCaml shipped them untyped to ship at all | [ocaml.md](../languages/ocaml.md), [koka-effekt.md](../languages/koka-effekt.md) |

## What the evidence says

1. **Colourless wins when the runtime can afford it.** Java adopted
   virtual threads citing Go; Kotlin's `suspend` and Loom now coexist on
   one VM as the nearest thing to a controlled experiment; Rust's async is
   the most-regretted part of Rust by its own designers ("unforced error");
   Zig removed `async`/`await` and redesigned around `Io`; Python's problem
   was the duplicate ecosystem, not the keyword ([java.md](../languages/java.md),
   [kotlin.md](../languages/kotlin.md), [rust.md](../languages/rust.md),
   [zig.md](../languages/zig.md), [python.md](../languages/python.md)).
   The remaining case for stackless async is zero-allocation, no-runtime
   targets (embedded, WebAssembly without stack switching).
2. **Structured concurrency is the settled shape for task lifetime.** Five
   independent designs (Trio, Kotlin, Swift, Java, Rust scoped threads)
   converged; the languages without it (Go, C#) show the leak and shutdown
   bug classes and the `ctx` retrofit.
3. **Static race freedom is accepted only with a sequential default.**
   Pony (every reference) was too much; Swift 6.0 (every actor crossing,
   concurrent default) was rejected and 6.2 made sequential the default
   with explicit `@concurrent`; Rust's `Send`/`Sync` (checked only at
   thread boundaries) is accepted ([pony.md](../languages/pony.md),
   [swift.md](../languages/swift.md)). The ladder Pony → Swift 6 → Verona
   regions → Rust → Go race detector is the design space, and the evidence
   says to sit at Rust's rung with Swift 6.2's default.
4. **Runtime-enforced sharing rules fail.** Kotlin/Native's frozen-object
   model (2017-2022) was replaced wholesale ([kotlin.md](../languages/kotlin.md));
   Go's race detector finds only exercised paths; Uber's Go study found
   46% of races were missing locks around shared heap data and 39% involved
   implicitly aliased slices, only 2.5% channels ([go.md](../languages/go.md)).
   The checks belong in the type system.
5. **Share-nothing gets relaxed later.** Dart isolates are gaining shared
   memory; Erlang's copy-on-send is its best-known performance ceiling;
   Pony's `iso`/`val`/`tag` are the minimal vocabulary for moving data
   between domains without copying ([dart.md](../languages/dart.md),
   [erlang-elixir.md](../languages/erlang-elixir.md), [pony.md](../languages/pony.md)).
   A typed shareable tier must exist from the start.
6. **Supervision is the only error-handling model with a decades-long
   reliability record**, and it needs isolation as the unit of failure and
   the ability to kill a unit safely ([erlang-elixir.md](../languages/erlang-elixir.md)).
   Structured scopes give the restart policy for free; killing a task
   safely is a runtime property.
7. **Bounded channels by default.** Erlang's unbounded mailbox is its
   best-documented production failure (three separate OTP mitigations);
   Go's unbounded-by-accident channels are a leak source.
8. **Fair scheduling needs compiler-inserted preemption points.** Erlang
   counts reductions; Go added asynchronous preemption in 1.14 because
   cooperative scheduling starved the GC; without a GC, loop yield points
   suffice ([erlang-elixir.md](../languages/erlang-elixir.md), [go.md](../languages/go.md)).
9. **Shipping a coroutine mechanism without a scheduler, task type and
   cancellation model is a known mistake** (C++20 coroutines, Rust async)
   ([cpp.md](../languages/cpp.md), [rust.md](../languages/rust.md)).

## Recommendation for Sisu

Colourless tasks on a runtime scheduler, with runtime profiles so the
scheduler can be absent; `task.scope`/`spawn` as the only way to start a
task; cancellation and deadlines structural; panics unwind to the scope,
which is the supervisor; `Send`/`Sync` inferred and checked only at `spawn`
boundaries (Rust's rung, Swift 6.2's default); bounded typed channels with
`select`; `Mutex[T]` owning its data; actors as a library pattern; C++11
memory model for atomics. See
[proposal/06](../../proposal/06-concurrency-and-capabilities.md); decisions
D-014, D-040.

## Open questions

- Q-6 stack strategy without a moving GC, and whether spawning should need
  a capability; Q-34 cooperative vs asynchronous preemption; Q-17 whether
  effect handlers should be the runtime primitive (OCaml's evidence) rather
  than stackful tasks.
