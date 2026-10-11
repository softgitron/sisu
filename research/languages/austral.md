# Austral

## Snapshot
| | |
|---|---|
| First appeared | Design work from 2018 (license says "Copyright 2018–2023"); public introduction December 2022 [1][2]. |
| Designed by | Fernando Borretti, alone, spec-first. |
| Current version (Oct 2026) | No numbered release ever. Bootstrapping compiler (OCaml, emits C) "implements every feature of the spec" but "does not support separate compilation"; 8,953 commits, ~1.6k stars; last commit 28 Jul 2025 (a stdlib bug fix). Effectively dormant [2][3]. |
| Governance | One designer; the specification is the design document; small Discord [2][14]. |
| Typing | Static, nominal; linear types; typeclasses; no subtyping; "no type inference, type information flows in one direction" [4]. |
| Memory | Linear types (use exactly once) plus Rust-style borrowing with explicit regions; no GC, no destructors; `Free` vs `Linear` universes [1][5]. |
| Compilation | AOT: Austral → C → native. |
| Primary domains | Systems programming "for building pyramids": long-lived, security-sensitive code [5]. |
| Notable users | None known. The LambdaClass interview (Dec 2023) describes "a little Discord" and the designer deferring marketing until the stdlib and tooling exist [14]. |

## Stated design goals
The spec lists eight goals: Simplicity ("the language must be simple enough
to fit in a single person's head", simplicity defined by Kolmogorov complexity:
a system is simple when "it can be described briefly"), Correctness ("if the
code compiles, it should work"), Security ("It should not be difficult to write
secure code"), Readability ("We are not typists, we are programmers"),
Maintainability, Modularity (interfaces and bodies separated so modules can be
typechecked before implementation), Strictness ("Austral is decidedly a
language for building pyramids") and Restraint ("Austral is designed to
restrain programmer power and minimize footguns") [5]. The trade-off is stated
up front: "we sacrifice terseness and programmer ergonomics for simplicity"
[5]. The introduction reduces this to two words, simplicity and strictness,
and argues "programming languages are not tools"; they are "a toolbox,
building material, and physical laws combined", so complexity in a language
is more damaging than complexity in a library [1].

## The central bets

### Linear types as the single safety mechanism
Two rules do all the work [1][5]:
- **Linear Universe Rule**: types live in the `Free` or `Linear` universe; a
  type is linear by declaration or by containing a linear type ("linearity is
  viral", so "you can't escape linearity by accident").
- **Use-Once Rule**: "A value of a linear type must be used once and only
  once." Discarding is an error; using twice is an error. In an `if`, an outer
  linear variable must be consumed zero times or exactly once in *every*
  branch; "a variable of a linear type, defined outside a loop, cannot appear
  in the body of the loop"; "`return` statements are illegal where there are
  unconsumed linear values" [5].

```austral
-- Only this shape compiles: the handle is threaded through every call.
let f: File := openFile("log.txt");
let f1: File := writeString(f, "hello");
let f2: File := writeString(f1, "world");
closeFile(f2);
```

This gives memory safety (no use-after-free, no double free, no leak: an
unconsumed allocation is a compile error) and *resource* safety (files,
sockets, DB transactions get the same guarantee) with no runtime, no GC, no
lifetime annotations [1]. Destructors are unnecessary: "Type classes are enough
to implement destructors" via a `Destructable`-style typeclass whose method
consumes the value [5]. The pattern "linear interface, non-linear interior"
marks the trust boundary: a linear API can wrap unsafe internals [5].

### Borrowing, deliberately smaller than Rust's
Passing a linear value transfers ownership, which is "more permission than
many functions need", so Austral adds references "stolen lock, stock, and
barrel from Rust" but "more restricted" [1][5]:
- Read-only `&[T, R]` and read-write `&![T, R]`; both are "bound to a region
  and are acquired by borrowing" [5].
- Named borrow statement: `borrow x as xref in R do ... end;` (and `borrow!`
  for mutable); "A borrowed variable cannot appear in the body of the statement
  that borrows it" [5].
- Anonymous `&x` / `&!x` with a nameless region: "Borrow expressions cannot
  escape the statement where they appear" [5].
- A borrow does not count as a use; paths (`.s`, `->s`, `[i]`) "must end in a
  path in the `Free` universe" and "A path that starts in a reference is a
  reference" [5].

Borretti reports the whole linearity-plus-borrow checker is "about 700 lines
of OCaml" [11]. The cost is "more typing in exchange for clearly marked region
ends and a simpler model" [1].

### Capability-based security against supply-chain attacks
The argument: in a package ecosystem "all code has the same permission level:
do anything"; "Code is permissionless by default" [1][8]. Austral's fix is
that filesystem, network, console and clock access require a *capability*
value, so "a string-padding function that claims to need network access
instantly stands out" in its signature [1]. Capabilities need four properties:
destroyable, surrenderable, non-duplicable, not acquirable from thin air;
linear types give the first three, and the fourth is a convention: "they must
be passed by the client" [5]. The root is built in: `main(root: RootCapability)`;
`getFilesystem(root: &[RootCapability, R])` derives weaker capabilities;
`surrenderRoot` consumes it, after which the program can do nothing
effectful [5][8]. Hierarchy by construction: a `Filesystem` capability yields
the root `Path`; a directory path yields subpaths but "can never go up to its
parent"; a file path yields read/write [1]. Clock access is also a capability
because precise timing enables Spectre-style side channels [1][5]. The FFI is
the admitted hole: "Ultimately, all guarantees are lost at the FFI boundary";
FFI requires an `Unsafe_Module` pragma so a build system can list unsafe
modules across transitive dependencies for audit [5].

## Strengths

### A complete, readable specification written before the compiler
The spec contains the rules *and* the rationale, with comparative sections on
error handling strategies, prior art and rejected alternatives [5]. Every
anti-feature is a decision, not an omission. This is the strongest "no
surprises" artifact among the languages in this group and the compiler
"implements every feature of the spec" [2].

### Resource safety, not only memory safety
Rust's affine types let you forget to close a file (drop is implicit); Austral's
linear types make forgetting a compile error [1][7]. Vale's "Higher RAII" cites
exactly this property (see `vale.md`).

### Capabilities fall out of linearity
"Capability security is therefore simply a consequence of linear types, not a
separate feature" [8]: no global state means a capability cannot be stashed,
linearity means it cannot be copied. Flix independently arrived at
effect-system-based supply-chain defence (OOPSLA 2026, see `flix.md`), which is
adoption-as-evidence that the problem is real and type-level solutions are
plausible.

### Mechanically simple syntax
Full `end` terminators (`end if;`, `end module body.`), statements distinct
from expressions, no operator precedence ("full parenthesization of nested
binary expressions"), records with compiler-chosen layout [1][5]. The
rationale is the dangling-else CVE and the observation that "language
lawyering is a design flaw" [1].

## Weaknesses

### Use-once is too strict in loops and branches
The loop rule means a linear value cannot even be touched inside a loop body
without being moved out and back through a borrow or an explicit threading
pattern. This is the price of not having affine types; the designer's own
essay concedes the Rust-style affine route is what you want "if you need
traditional throw-and-catch exceptions" [7].

### Verbose borrowing and the iterator problem
Named regions and `borrow ... do ... end` blocks are heavier than Rust's
elided lifetimes; Borretti planned "lifetime elision" and admits he wants "a
better answer for iterators and closures before abandoning first-class
references" [11]. He considered switching to Hylo-style second-class
references and found the simplicity gain small because the checker is already
tiny [11].

### No concurrency model
"I'm procrastinating on concurrency models because I don't know enough about
them" [14]. The spec has nothing on threads, async or data races; "no
first-class async" is an anti-feature [4].

### Error handling defers hard cases to termination
"programs should crash at the slightest contract violation, because recovery
efforts can become attack vectors" and "For failures — as opposed to errors —
you should use `Option` and `Result` types" [5]. The companion essay admits
termination "makes testing harder" and that returning `Result` for arithmetic
and indexing is "too limiting" because error cases "leak into client code up
the call stack"; Austral chose fatal-vs-value anyway [7].

### No type inference, by principle
Beyond function signatures (where Rust and Haskell also require them), Austral
declines local inference: "Type inference Was a Mistake" argues inference
"makes code harder to read" and that OCaml-style errors surface "arbitrarily
far away from the actual mistake" [10]. Few users will accept annotating every
`let`; this is a stated ergonomics sacrifice, not an oversight.

### Capability model admitted "good—but not good enough"
Capabilities are irrevocable; global uniqueness (one terminal) relies on an
"inconvenient" `UniqueCapability` wrapper or discipline; unsafe modules and
capabilities are not unified ("the faultline"); the stricter `Unsafe`-capability
design is "much more tedious to use" [8].

### Dormant project, zero adoption
No release, no package manager, no stdlib beyond basics, last commit mid-2025
[2][3]. The design has never been validated by a program larger than the
examples. The designer's post-2024 writing is about other topics [9].

## Designer retrospectives
- "This is one area where I wish I'd kept something like a lab notebook while
  iterating on the language design" [14].
- "the most frustrating thing has been writing the compiler"; a tension
  between a minimal prototype and "production-grade diagnostics" [14].
- Rust: "the only industrial language to have anything like linear types",
  but "the problem with pragmatism is that it never ends"; async with
  lifetimes means "everyone ends up putting all of their shared resources under
  reference-counted pointers" [14].
- From the compiler-lessons essay: "you have to design the language's module
  system and compilation model with separate compilation in mind" (Austral did
  not, and still lacks it); start with "the simplest, dumbest possible MVP
  compiler"; "The language determines how sophisticated the environment has
  to be" [13].
- On references: first-class references "offer a midpoint between raw pointers
  and second-class references"; he plans elision but keeps them [11].

## Influence
- **Took from**: Rust (borrowing "lock, stock, and barrel", ownership),
  Haskell (typeclasses, with Austral's stricter global-uniqueness rule: "You
  are not allowed to define an instance for a foreign typeclass and a foreign
  type"), Ada (trap-on-overflow vs modular arithmetic, `end` syntax,
  readability), E/object-capability literature and Midori/Herbceptions for the
  fatal-vs-recoverable error split [1][4][5][7].
- **Gave to**: Vale's Higher RAII and Ovadia's taxonomy cite Austral as the
  example of "borrow checking (with linear types)" for liveness [12]. No
  mainstream language has copied Austral's capability-in-`main` design, but
  Deno's permission flags and WebAssembly component-model capabilities are
  sibling ideas at the runtime level (not language level).

## Lessons for Sisu
- **Adopt**: The spec-first, rationale-included discipline. Every Sisu
  anti-feature should have a written "why not" with the alternatives
  considered, as Austral's spec does for exceptions.
- **Adopt**: Capability-passing for ambient authority (filesystem, network,
  clock, randomness, process) rooted in `main`, so a dependency's signature
  reveals what it can touch. Combine with an unsafe-module audit list as
  Austral does for FFI. This is cheap if Sisu already has linear or affine
  types and no mutable globals.
- **Adopt**: Compile-time "must be consumed" for resources (linear, not
  merely affine) *as an opt-in per type*, so files, transactions and
  capabilities cannot be forgotten, while ordinary data stays affine.
- **Adapt**: Austral shows that pure use-once is hostile to loops and early
  return. Sisu should default to affine (drop allowed, with destructors) and
  reserve strict linearity for types that declare it; this is the design
  Borretti's own essay points to for languages that want exceptions or panics
  [7].
- **Adapt**: "No operator precedence" is too far; but "no mixing of bitwise,
  Boolean and arithmetic operators without parentheses" is a cheap lint-level
  rule worth keeping.
- **Avoid**: Treating concurrency as a later add-on. Linear types interact
  with threads (a linear value sent to another thread is a move; a borrow across
  threads needs regions); deferring the design left Austral without an answer.
- **Avoid**: Rejecting all local type inference. Signature annotations yes;
  `let x: Int32 := 1` everywhere is a demonstrated adoption killer in the
  languages that tried it.
- **Open questions raised**: (1) Can a linear-capability model be made
  revocable and uniqueness-enforcing without the "tedious" `Unsafe` capability
  on every FFI call? (2) Is second-class (Hylo-style) borrowing enough, or
  does a systems language need first-class references for iterators and
  closures? Borretti studied both and did not decide [11]. (3) Where is the
  line between "bug: terminate" and "failure: value" when arithmetic overflow
  and bounds errors are pervasive?

## Sources
1. F. Borretti, "Introducing Austral: A Systems Language with Linear Types and Capabilities", https://borretti.me/article/introducing-austral (2022).
2. Austral compiler repository (README, status), https://github.com/austral/austral (2026).
3. Austral commit history (last commit 28 Jul 2025), https://github.com/austral/austral/commits/master.
4. Austral "Features and anti-features" page, https://austral-lang.org/features (2023).
5. Austral Language Specification (goals, rationale, linearity, borrowing, capabilities, typeclasses), https://austral-lang.org/spec/spec.html (2023).
6. F. Borretti, "Linear Types and Safety", https://borretti.me/article/linear-types-safety (2019).
7. F. Borretti, "Linear Types and Exceptions", https://borretti.me/article/linear-types-exceptions (2021).
8. F. Borretti, "How Capabilities Work in Austral", https://borretti.me/article/how-capabilities-work-austral (2023).
9. F. Borretti, article index, https://borretti.me/article/ (2026).
10. F. Borretti, "Type Inference Was a Mistake", https://borretti.me/article/type-inference-was-a-mistake (2024).
11. F. Borretti, "Second-Class References", https://borretti.me/article/second-class-references (2023).
12. E. Ovadia, "Borrow checking, RC, GC, and the Eleven (!) Other Memory Safety Approaches", https://verdagon.dev/grimoire/grimoire (2023).
13. F. Borretti, "Lessons from Writing a Compiler", https://borretti.me/article/lessons-writing-compiler (2022).
14. LambdaClass, "Interview with Fernando Borretti about Austral", https://blog.lambdaclass.com/austral/ (Dec 2023).
