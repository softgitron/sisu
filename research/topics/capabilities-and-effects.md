# Capabilities and effects

## The question

How a program's interaction with the outside world (files, network, clock,
randomness, environment, processes) is controlled, tested, and made
visible, and whether the language should track what a function *does*
(effects) or what it *can do* (capabilities), or neither.

## Approaches found

| Approach | Languages | Buys | Costs | Evidence |
|---|---|---|---|---|
| Ambient authority | Every mainstream language | Nothing to pass; `print` works anywhere | Any dependency can do anything; supply-chain attacks are the direct result; testing needs mocking frameworks | [javascript.md](../languages/javascript.md), [python.md](../languages/python.md) |
| Runtime permission flags | Deno | Coarse control without language changes | Process-wide, not per dependency; policy, not type | [javascript.md](../languages/javascript.md) |
| Capabilities as values from `main` | Austral, Zig (`Io`), Roc (platforms), Pony (ambient authority removed), E, Wasm component model | A dependency's signature shows what it can touch; tests pass fakes; portability across runtimes | One more parameter on every I/O function; threaded up to `main` (Zig's main objection) | [austral.md](../languages/austral.md), [zig.md](../languages/zig.md), [roc.md](../languages/roc.md) |
| Implicit context | Odin (`context`), Jai, Scala `using`, Kotlin context parameters | Interception of allocator/logger in code you do not own; no boilerplate | Ambient authority through the back door; "does this need the allocator?" answered by convention; Scala's implicits were the canonical too-powerful feature | [odin-jai-c3.md](../languages/odin-jai-c3.md), [scala.md](../languages/scala.md) |
| Purity in types (monadic IO) | Haskell | Composability; STM; production evidence at Meta | Monad transformer wars (six competing effect libraries); the "monad tutorial" learning cost | [haskell.md](../languages/haskell.md) |
| Typed algebraic effects | Koka, Effekt, Flix, Unison, (PureScript rows, deleted) | One mechanism for exceptions, async, generators, state; effect-polymorphic `map` inferred | Signature noise; handler performance; only Unison Cloud is a verified production user (closed world); PureScript deleted effect rows; OCaml 5 shipped effects *untyped* to ship at all | [koka-effekt.md](../languages/koka-effekt.md), [flix.md](../languages/flix.md), [unison.md](../languages/unison.md), [ocaml.md](../languages/ocaml.md), [elm-purescript.md](../languages/elm-purescript.md) |
| Capture checking | Scala 3 (Caprese) | Effects as capabilities tracked by the type system with inference | Four years in, still experimental, "not yet" on annotation burden | [scala.md](../languages/scala.md) |
| Declared purity levels | Fortran (`pure`/`elemental`), D (`pure`), Nim effect tags | Optimiser and reader can rely on them | D: attributes become noise unless they are defaults | [fortran.md](../languages/fortran.md), [d.md](../languages/d.md), [nim.md](../languages/nim.md) |

## What the evidence says

1. **Supply-chain attacks are the same shape every time.** left-pad
   (2016), event-stream (2018), the 2025 Shai-Hulud worm and roughly eight
   major npm incidents in 2026 including one with valid provenance; PyPI
   and crates.io variants: a dependency that has no business touching the
   network or file system does so, usually at install time or on first
   import ([javascript.md](../languages/javascript.md),
   [../evidence/studies.md](../evidence/studies.md)). Every current defence
   (lockfiles, provenance, audits, Deno flags) is policy applied from
   outside. Capabilities are the only defence enforced by the compiler.
2. **Capabilities are proven usable at small scale and are being tested at
   large scale now.** Austral's spec makes the full argument; Zig 0.16
   (April 2026) shipped `Io` as a value and the ergonomic objection is
   exactly the viral parameter ([zig.md](../languages/zig.md)); Roc's
   platforms and Unison's `Cloud` ability show the "platform is an
   interface with real and fake implementations" pattern working
   ([roc.md](../languages/roc.md), [unison.md](../languages/unison.md)).
3. **Testability follows automatically.** Go's `io.Reader`, every "inject
   your dependencies" style guide, and Elixir's and Erlang's behaviour-
   based fakes are the ad-hoc version; capabilities make it the only way.
4. **Typed effect systems are not yet ergonomic enough for application
   programmers.** The designers of Koka, Effekt and Flix each document the
   signature-noise problem; PureScript shipped effect rows and deleted
   them; OCaml chose untyped effects deliberately; Scala's capture checking
   is the most promising path and is explicitly "not yet"
   ([koka-effekt.md](../languages/koka-effekt.md), [flix.md](../languages/flix.md),
   [ocaml.md](../languages/ocaml.md), [scala.md](../languages/scala.md)).
   If a language has effects, the research is specific about the shape:
   inferred, set-based (Flix) not ordered rows (Koka), lexically scoped
   handlers (Effekt), tail-resumptive fast path, a designed debug escape
   hatch (Flix's `Debug`), a visible marker (Roc's `!`).
5. **Implicit context works in production and is the thing capabilities
   must beat on ergonomics.** Odin's `context` is used to intercept
   allocators and loggers in third-party code across dynamic-library
   boundaries; the research recommends carrying it as a *tracked*
   capability so the type system answers "does this need it"
   ([odin-jai-c3.md](../languages/odin-jai-c3.md)).
6. **Enforcement has three holes: `unsafe`, FFI and compile-time code.**
   Austral's `Unsafe` capability on every FFI call is "tedious" by its own
   author's account; Rust's `build.rs` and npm's `postinstall` are the
   exploited vectors; F#'s type providers (I/O during type checking) are
   the compile-time version ([austral.md](../languages/austral.md),
   [rust.md](../languages/rust.md), [fsharp-sml.md](../languages/fsharp-sml.md)).
   A package-level permission for `unsafe`/FFI, no build scripts, and a
   sandboxed compile-time evaluator close them.
7. **Purity as a derived property is free.** A function with no capability
   parameters and no `unsafe` cannot perform I/O; the compiler knows this
   without an effect system and can use it for compile-time evaluation,
   memoisation and `do concurrent`-style parallelism
   ([fortran.md](../languages/fortran.md)).

## Recommendation for Sisu

Capabilities as values: `System` handed to `main`, attenuable, no global
I/O functions except standard streams; `unsafe` and `extern` permitted only
in packages listed in the root manifest's `[trust]`; no build scripts;
compile-time evaluation without I/O. No effect system in version 1; `pure`
as the one tracked property; a `with Fs, Clock` signature clause reserved as
the tracked-capability form if the parameter burden proves too high. See
[proposal/06](../../proposal/06-concurrency-and-capabilities.md); decisions
D-015, D-032.

## Open questions

- Q-5 ambient stdio and implicit/tracked context; Q-6 whether task
  spawning should require a capability; Q-17 effect system.
