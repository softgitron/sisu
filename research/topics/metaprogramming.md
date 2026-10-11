# Metaprogramming

## The question

How code generates code: macros, compile-time evaluation, reflection,
templates; what each buys, what it costs the tooling, and where the
ecosystem fragments.

## Approaches found

| Approach | Languages | Buys | Costs | Evidence |
|---|---|---|---|---|
| Textual preprocessor | C, C++ | Conditional compilation, constants | Universally regretted; 2000x expansion from includes; both languages keep trying to replace it | [c.md](../languages/c.md), [cpp.md](../languages/cpp.md) |
| Templates (duck-typed, Turing-complete by accident) | C++ | Everything | Errors at instantiation; 15 years to get concepts; compile times | [cpp.md](../languages/cpp.md) |
| CTFE + string mixins | D | Compile-time evaluation of ordinary functions (a decade before Zig) | String mixins are untyped text generation; designers' regret | [d.md](../languages/d.md) |
| `comptime` with types as values | Zig, Jai | One language for compile time and run time; generics, tables, derive, conditional compilation without macros | Generic errors at instantiation (Zig uses comptime *for* generics); unbounded comptime becomes build time (Blow's "temperance") | [zig.md](../languages/zig.md), [odin-jai-c3.md](../languages/odin-jai-c3.md) |
| Pattern macros | Rust `macro_rules!` | Hygienic, declarative | A second language with its own syntax and error messages | [rust.md](../languages/rust.md) |
| Procedural / token macros | Rust proc macros, Swift macros | Ecosystem-defining power (`serde`, `tokio`) | Opaque to tooling; the main contributor to compile-time complaints (separate crate, `syn` parse per invocation); Swift reproduced the build cost; Scala 2 macros blocked the Scala 3 migration for years | [rust.md](../languages/rust.md), [swift.md](../languages/swift.md), [scala.md](../languages/scala.md) |
| Type-aware in-language macros | Dart (cancelled 2025), Nim typed AST macros | See types | Dart: could not be made fast enough for hot reload after three years; Nim: powerful, hurts tooling | [dart.md](../languages/dart.md), [nim.md](../languages/nim.md) |
| Hygienic AST macros | Scheme/Racket, Clojure, Elixir, Julia, Lean 4 | Most expressive tool in programming; DSLs; Elixir's whole ecosystem is built on them | The Lisp curse (cheap extension plus weak curation yields incompatible duplicates); readability of macro-heavy code; IDE support needs expansion exposed | [lisp-scheme-racket.md](../languages/lisp-scheme-racket.md), [erlang-elixir.md](../languages/erlang-elixir.md), [julia.md](../languages/julia.md), [idris-agda-lean.md](../languages/idris-agda-lean.md) |
| Runtime reflection | Java, C#, Python, Ruby, Smalltalk | Frameworks, serialisation, monkey patching | Breaks dead-code elimination, AOT and capability models; Python's dynamism capped its performance | [python.md](../languages/python.md), [ruby.md](../languages/ruby.md) |
| Compile-time reflection | Zig (`@typeInfo`), C++26 (reflection), Nim | Derive/serialisation without macros | Must not create declarations or it becomes a macro system | [zig.md](../languages/zig.md) |
| Type providers | F# | Typed access to external schemas | I/O during type checking inside the compiler; irreproducible builds | [fsharp-sml.md](../languages/fsharp-sml.md) |
| Code generation tools | Go, Dart, Java | Works | A parallel toolchain; Dart budgeted for it after cancelling macros | [go.md](../languages/go.md), [dart.md](../languages/dart.md) |

## What the evidence says

1. **Compile-time evaluation of ordinary code is the one metaprogramming
   feature with no regrets.** D, Zig, Jai, C++ `constexpr`/`consteval`,
   Nim's VM and Lean all converged on it; the regrets are about what was
   built *on top* (string mixins, unconstrained generics)
   ([d.md](../languages/d.md), [zig.md](../languages/zig.md)).
2. **Macros that see types are expensive at every edit; macros that do
   not are weak and slow to build.** Dart's cancellation and Swift's forum
   threads are two independent warnings; decide the cost model before
   promising the feature ([dart.md](../languages/dart.md),
   [swift.md](../languages/swift.md)).
3. **Reflection plus comptime covers what proc macros are used for.**
   Zig's standard library JSON encoder is the existence proof that
   serialisation, derive and formatting need no macros; Rust's heaviest
   macro users are exactly these cases ([zig.md](../languages/zig.md),
   [rust.md](../languages/rust.md)).
4. **Compile-time code must be sandboxed and bounded.** F#'s type
   providers, Rust's `build.rs`, npm's install scripts, and Jai's "anything
   at compile time" each show the cost of unbounded or I/O-capable build-
   time code: irreproducibility, supply-chain exposure, build time
   ([fsharp-sml.md](../languages/fsharp-sml.md), [rust.md](../languages/rust.md)).
5. **AST macros are the most powerful tool and the one most correlated
   with fragmentation.** Elixir shows they can be the foundation of a
   healthy ecosystem *when the core library is strongly curated*; Lisp
   shows the alternative ([erlang-elixir.md](../languages/erlang-elixir.md),
   [lisp-scheme-racket.md](../languages/lisp-scheme-racket.md)). If
   adopted later: hygienic, declarative by default, expansion visible in
   the IDE, procedural as a separate marked tier.
6. **Checked string DSLs cover most "new syntax" demand.** Regex, SQL,
   format strings, routes: a comptime function that parses a literal and
   reports errors into it gives the checking without new statement forms
   (Perl's syntax-checked regex literals, C#'s interpolated string
   handlers) ([perl-php.md](../languages/perl-php.md), [csharp.md](../languages/csharp.md)).
7. **Conditional compilation should type-check both branches.** C's
   `#ifdef` soup versus Zig's `comptime if` and D's `static if`
   ([c.md](../languages/c.md), [zig.md](../languages/zig.md)).

## Recommendation for Sisu

`comptime` evaluation of ordinary code over the typed IR, with no I/O and
a fuel limit; compile-time reflection that can compute values and `impl`s
but cannot inject declarations; `derive` as a comptime function any trait
can provide; `comptime if`/`for` and `@cfg`; checked string DSLs via
comptime parsing; no macros in version 1; generics through traits, not
comptime. See [proposal/08](../../proposal/08-metaprogramming.md);
decision D-018.

## Open questions

- Q-15 hygienic AST macros later; Q-27 comptime-constructed types
  alongside trait generics.
