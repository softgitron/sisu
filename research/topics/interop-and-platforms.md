# Interop and platforms

## The question

How Sisu code calls and is called by other languages, which platforms it
runs on, and whether it should be hosted on an existing runtime.

## What the evidence says

1. **The C ABI is the FFI.** Go, Rust, Zig, Odin, C3, Carbon, Swift,
   Kotlin/Native and Nim all kept it; nothing else will be accepted. The
   successors differ only in how painless the boundary is
   ([c.md](../languages/c.md)).
2. **Direct header import beats binding generators.** Zig's `@cImport`
   (bundling Clang), D's ImportC and Swift's Clang importer let a program
   use a C library with no generated bindings to commit; Rust's `bindgen`
   is workable and a known friction. The cost is bundling a C front end;
   Zig shows it is also the route to building C/C++ dependencies without a
   separately installed toolchain, which is a large part of its adoption
   ([zig.md](../languages/zig.md), [d.md](../languages/d.md),
   [swift.md](../languages/swift.md), [rust.md](../languages/rust.md)).
3. **C++ interop at the template/class level requires designing the
   language around C++.** Carbon exists for this and has slipped years with
   Google funding; Swift's C++ interop took five years for a partial
   result; Circle is one person ([carbon.md](../languages/carbon.md),
   [cpp.md](../languages/cpp.md), [swift.md](../languages/swift.md)).
4. **Hosting on a VM is an adoption engine and a ceiling.** F#, Scala,
   Kotlin and Clojure got libraries and users from the JVM/.NET/JS and
   inherited `null`, exceptions, erasure, boxing and a foreign object
   model; Kotlin/Native's memory model failure and Gleam's untyped BEAM
   mailboxes show the ceiling ([fsharp-sml.md](../languages/fsharp-sml.md),
   [kotlin.md](../languages/kotlin.md), [gleam.md](../languages/gleam.md)).
   Native with FFI is the choice for a language that wants to control its
   own type system.
5. **Interop types should be visible.** Kotlin's platform types (nullable
   from Java, non-denotable) are the right idea for not forcing annotation
   of a foreign world; the research recommends making the boundary type
   grep-able; F#'s `T | null` at the boundary only is the same
   ([kotlin.md](../languages/kotlin.md), [fsharp-sml.md](../languages/fsharp-sml.md)).
6. **Blocking FFI must be designed with the scheduler.** Erlang's NIF
   1 ms rule and VM-crash risk, Go's thread hand-off, and the general
   "cannot yield across FFI" rule for stackful coroutines
   ([erlang-elixir.md](../languages/erlang-elixir.md), [lua.md](../languages/lua.md)).
7. **ABI stability is a choice with a known price.** Swift's library
   evolution cost years and taxes every generic call, and exists because
   Apple ships frameworks in Swift; Rust's unstable ABI leaves layout and
   monomorphisation free; C++'s frozen standard-library ABI blocked fixes
   and produced renamed replacements ([swift.md](../languages/swift.md),
   [cpp.md](../languages/cpp.md)).
8. **Static single binaries and cross-compilation by default** are
   repeatedly cited adoption drivers (Go, Zig, Rust) and PHP's "copy a
   file to the server" deployment is the extreme case of deployment
   simplicity winning ([go.md](../languages/go.md), [perl-php.md](../languages/perl-php.md)).
9. **WebAssembly is a first-class target now**, and single-threaded event-
   loop runtimes are what it needs; stackful coroutines are the obstacle
   (no stack switching in Wasm without the stack-switching proposal)
   ([javascript.md](../languages/javascript.md), [zig.md](../languages/zig.md)).
10. **Embedding** (Lua's small, stable C API; every large application
    eventually embeds a scripting language) is a demand Sisu can serve via
    `@export` without being an embeddable scripting language itself
    ([lua.md](../languages/lua.md)).
11. **Bare-metal with no runtime** is a real constituency (Forth, Rust
    `no_std`, Zig, Ada's Ravenscar profile), and Ada's restriction
    profiles show that a compiler-checked subset works
    ([forth.md](../languages/forth.md), [ada-spark.md](../languages/ada-spark.md)).

## Recommendation for Sisu

Native language with a C ABI boundary in both directions; header import
with a bundled C front end; C sources built by the toolchain; `@repr(c)`
and `@export`; `unsafe` for FFI, in trusted packages only; `@blocking` for
the scheduler; runtime profiles `full`/`single`/`freestanding` with the
capability traits as the seam; WebAssembly tier 1 via the `single` profile;
cross-compilation built in; unstable Sisu ABI; no C++ interop beyond C
wrappers. See [proposal/12](../../proposal/12-interop-and-targets.md);
decision D-039.

## Open questions

- Q-24 C++ interop; Q-31 stable ABI; Q-6 stackful vs stackless under the
  Wasm target.
