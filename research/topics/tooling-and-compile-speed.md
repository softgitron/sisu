# Tooling and compile speed

## The question

What ships with the compiler, how fast the edit-compile-test loop is, and
which *language* decisions determine both.

## What the evidence says

1. **Compile time is the top productivity complaint wherever it is
   slow.** Rust's surveys: about a quarter call compile times a big problem
   and 56% want them improved, making it the top limiter ahead of the
   borrow checker (10.6%); 55% wait more than ten seconds and 45% of
   people who left cite it ([../evidence/surveys.md](../evidence/surveys.md)); Scala lost
   the "better Java" niche to Kotlin partly on compiler speed; Swift's type
   checker timeouts; C++ builds; Julia's time-to-first-plot defined its
   image for a decade ([rust.md](../languages/rust.md),
   [scala.md](../languages/scala.md), [swift.md](../languages/swift.md),
   [julia.md](../languages/julia.md)). Go and Zig are praised for the
   opposite.
2. **The causes are language decisions, not compiler bugs.** Rust:
   crate-as-unit plus monomorphisation plus proc-macro derive plus LLVM;
   Swift: the inference triangle; Scala: implicit search and a rich type
   system; C++: textual inclusion and templates; Crystal: global
   inference; Julia: JIT with no AOT story. Go: acyclic imports, explicit
   signatures, no macros, a custom backend. Zig: self-hosted backend for
   debug builds, incremental compilation
   ([rust.md](../languages/rust.md), [go.md](../languages/go.md),
   [zig.md](../languages/zig.md), [crystal.md](../languages/crystal.md)).
3. **The implementation language and architecture matter.** TypeScript's
   compiler ported to Go for a 10x speedup with no algorithmic change
   (native code, threads, value types); Roslyn's compiler-as-a-service
   from the start versus rust-analyzer's years of divergence from rustc;
   Roc's post-mortem of a compiler that could not be fixed incrementally
   ([typescript.md](../languages/typescript.md), [csharp.md](../languages/csharp.md),
   [roc.md](../languages/roc.md)).
4. **One binary with everything is what users rate.** Go, Cargo, Zig,
   mix, Dart, Deno, `uv`; the language server, formatter, test runner,
   documentation and package manager must share the compiler's front end
   or they lag (Kotlin's LSP in year ten; `zls` versus Zig; Clojure's
   error messages as a decade-long top complaint because the language did
   not own them) ([kotlin.md](../languages/kotlin.md),
   [clojure.md](../languages/clojure.md)).
5. **Error messages can be designed, and surveys reward it.** Elm's
   "Compiler Errors for Humans", Rust's diagnostics working group (9% of
   users call messages a big problem, against the borrow checker's 10.6%
   and compile times' roughly 25%), Gleam listing new diagnostics in every
   release note; the controlled error-message studies on novices (Becker
   et al., Denny et al.) are mixed, so the evidence is for satisfaction
   rather than a measured learning gain ([elm-purescript.md](../languages/elm-purescript.md),
   [gleam.md](../languages/gleam.md), [../evidence/studies.md](../evidence/studies.md)).
   The Rust finding is specific: the *explanation* of borrow errors, not
   the rule, is what improved the learning curve.
6. **Tests, docs and property testing belong in the toolchain.** Zig, D
   and Rust inline tests; Python and Rust doctests; Elixir's ExUnit and
   docs reaching >90% use because they are built in; QuickCheck's lineage
   used an order of magnitude more when zero-dependency
   ([erlang-elixir.md](../languages/erlang-elixir.md), [d.md](../languages/d.md)).
7. **Hot reload and live development drive adoption on their own.** Dart
   kept a JIT in the toolchain and cancelled macros for it; Smalltalk's
   fix-and-resume debugger has no modern equal; Clojure's REPL workflow is
   its users' top-cited benefit ([dart.md](../languages/dart.md),
   [smalltalk.md](../languages/smalltalk.md), [clojure.md](../languages/clojure.md)).
   The cost is a debug ABI with reifiable frames and an indirection point
   per definition that release builds remove.
8. **Cross-compilation that requires a second toolchain is
   cross-compilation nobody does.** Zig's and Go's built-in targets are a
   large part of their adoption ([zig.md](../languages/zig.md)).
9. **Measure it or it grows.** Julia's TTFX and Rust's compile times were
   both allowed to grow because nothing in CI stopped them
   ([julia.md](../languages/julia.md)).

## Recommendation for Sisu

One binary (build, run, test, check, fmt, lint, doc, add, publish, audit,
fix, explain, lsp, repl, bench); numeric compile-speed targets in the
proposal; explicit signatures, acyclic packages, no macros, bounded
comptime, trait-checked generics with instantiation sharing, a non-LLVM
debug backend, a query-based incremental front end shared with the LSP,
compile-time benchmarks in CI, error-tolerant compiler architecture;
diagnostics with codes, spans, fix-its and `explain`; a formatter with no
options; tests, doctests, property tests, snapshots and benchmarks in the
standard library; reifiable debug frames as a stated goal. See
[proposal/10](../../proposal/10-tooling.md); decisions D-029, D-030,
D-038, D-041.

## Open questions

- Q-22 live programming; Q-30 interpreter for scripts; Q-32 warnings
  policy.
