# Polymorphism and generics

## The question

How code is written once for many types, how behaviour is attached to
types, and how that interacts with error messages, compile times, binary
size and dynamic dispatch.

## Approaches found

| Approach | Languages | Buys | Costs | Evidence |
|---|---|---|---|---|
| Class inheritance | Java, C#, C++, Python, Ruby, Smalltalk, Swift classes | Familiar; frameworks (GUI) | Fragile base class; diamond problem; "composition over inheritance" became consensus; Kotlin and C# made classes final/sealed by default | [java.md](../languages/java.md), [kotlin.md](../languages/kotlin.md), [smalltalk.md](../languages/smalltalk.md) |
| Duck typing (dynamic) | Python, Ruby, JS, Clojure | Flexibility | No contracts; Julia's correctness problems are the static-dispatch version of this | [julia.md](../languages/julia.md) |
| Templates / comptime duck typing | C++, Zig, D (`isInputRange!`) | Maximal power, no declaration overhead | Errors at instantiation, deep inside the library; C++ took 15 years to get concepts; Zig refused interfaces (#1268) and hit the limit at stdlib scale ("Writergate") | [cpp.md](../languages/cpp.md), [zig.md](../languages/zig.md), [d.md](../languages/d.md) |
| Structural interfaces | Go, TypeScript | No declaration needed | Accidental satisfaction; no default methods; no trait-specific docs | [go.md](../languages/go.md), [typescript.md](../languages/typescript.md) |
| Type classes / traits with coherence | Haskell, Rust, Swift protocols | Return-type polymorphism; checked at definition; one impl per type makes `Map[K,V]` behave uniformly; the most-copied Haskell idea | Orphan rule forces newtypes (15.8% of Rust survey respondents call it a problem) | [haskell.md](../languages/haskell.md), [rust.md](../languages/rust.md) |
| Scoped instances | Scala `given`, OCaml modular implicits (never shipped) | No orphan problem | "Which instance is in scope" confusion; Scala 2 implicits were the canonical too-powerful feature | [scala.md](../languages/scala.md), [ocaml.md](../languages/ocaml.md) |
| ML modules and functors | OCaml, SML | Parameters carrying types and values together; strongest module system in use | Nobody copied it fully; no ad-hoc polymorphism (OCaml needed twelve years to ship a reduced "modular explicits") | [ocaml.md](../languages/ocaml.md), [fsharp-sml.md](../languages/fsharp-sml.md) |
| Multiple dispatch | Julia, CLOS | Solves the expression problem cleanly; composes better than single dispatch | No contracts: generic composition produced silent wrong answers at scale (Vishnevsky 2022); type piracy | [julia.md](../languages/julia.md), [lisp-scheme-racket.md](../languages/lisp-scheme-racket.md) |
| Erasure | Java, Scala on JVM, Kotlin | Migration compatibility | Primitives boxed; no reified generics; Valhalla twelve years and counting | [java.md](../languages/java.md) |
| Reification / monomorphisation | C#, Rust, C++, Swift (specialised) | Unboxed `List<int>`; predictable performance | Compile time and binary size (Rust's top problem; Graydon's own objection); no stable ABI | [csharp.md](../languages/csharp.md), [rust.md](../languages/rust.md) |
| Dictionary passing | Haskell, Swift (unspecialised), Go (GC-shape stenciling as a hybrid) | Fast compiles, small binaries, stable ABI | Indirect calls; unpredictable performance; Go chose it for compile speed and pays at runtime | [go.md](../languages/go.md), [swift.md](../languages/swift.md) |
| Higher-kinded types | Haskell, Scala, PureScript | Abstraction over containers | Complexity; Rust lacks them and works around with GATs; Roc and Elm refuse them deliberately | [scala.md](../languages/scala.md), [rust.md](../languages/rust.md), [roc.md](../languages/roc.md) |

## What the evidence says

1. **Generics must exist before the standard library is written.** Go
   (generics in 1.18 after a decade, with permanent library scars), Java
   (erasure for migration), Fortran (templates arriving for a 1957
   language in 2029), Ada (OOP retrofitted in 95): every late addition is
   visibly bolted on ([go.md](../languages/go.md), [java.md](../languages/java.md),
   [fortran.md](../languages/fortran.md)).
2. **Generic code must be checked at its definition.** C++ concepts, Zig's
   stdlib experience, Julia's correctness incidents, D's structural range
   templates and Carbon's explicit "checked generics" decision are five
   independent arrivals at the same rule ([carbon.md](../languages/carbon.md)).
3. **Coherent type classes are the one Haskell idea every successor
   copied.** Rust's reference credits them; Swift's protocols are them;
   OCaml's and F#'s failure to retrofit ad-hoc polymorphism (SRTP hacks,
   twelve years of modular implicits) shows it must exist from day one
   ([haskell.md](../languages/haskell.md), [fsharp-sml.md](../languages/fsharp-sml.md)).
4. **Explicit conformance beats structural.** Go's accidental interface
   satisfaction and inability to attach defaults; Mojo's nominal
   conformance; Rust's and Swift's explicit `impl`/`extension`. The
   convenience of structural satisfaction can be recovered as a compiler
   fix-it ([go.md](../languages/go.md), [mojo.md](../languages/mojo.md)).
5. **Inheritance is not needed.** Rust, Go, Elm and Swift's protocol-
   oriented style ship GUI toolkits, games and large systems without it;
   Smalltalk's own later work (traits) and Kotlin's final-by-default
   agree ([smalltalk.md](../languages/smalltalk.md)).
6. **Monomorphisation vs dictionary passing has no free answer.** Rust and
   Swift took opposite sides and both pay; Go's GC-shape stenciling is the
   documented hybrid. A stated policy with instantiation sharing and a
   budget is the best available compromise ([rust.md](../languages/rust.md),
   [go.md](../languages/go.md)).
7. **Multi-argument dispatch is valuable for mathematics and solvable with
   traits that take type parameters** (`impl Add[Vec] for Matrix`), which
   is static, checked and ambiguity-free at definition time, unlike Julia's
   open method tables ([julia.md](../languages/julia.md)).
8. **Extension methods are cheap and universally liked** when statically
   resolved and import-scoped (Kotlin, Swift, C#, Rust extension traits);
   open classes (Ruby, Smalltalk) are the version to avoid
   ([kotlin.md](../languages/kotlin.md), [ruby.md](../languages/ruby.md)).
9. **UFCS** (D, Nim) delivers the extension property but makes "where is
   this defined" harder and interacts badly with macros and qualified
   names ([nim.md](../languages/nim.md), [d.md](../languages/d.md)).

## Recommendation for Sisu

Traits with explicit `impl` and global coherence; associated types and
constants; default methods; `[T]` generics with bounds checked at the
definition; monomorphisation with instantiation sharing and a budget;
explicit `dyn`; auto traits inferred structurally; no inheritance;
delegation via `@delegate` for newtypes; extension traits instead of UFCS;
const generics from the start; HKT syntax reserved. See
[proposal/03](../../proposal/03-types-and-generics.md); decisions D-012,
D-013, D-017, D-050.

## Open questions

- Q-10 higher-kinded types; Q-11 orphan rule vs scoped instances; Q-12
  UFCS; Q-27 comptime-parameterised types alongside traits; Q-28 whether
  delegation suffices for OOP-heavy domains.
