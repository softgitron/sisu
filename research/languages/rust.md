# Rust

## Snapshot
| | |
|---|---|
| First appeared | Personal project of Graydon Hoare from 2006; Mozilla sponsorship from 2009; public 2010; Rust 1.0 on 15 May 2015 |
| Designed by | Graydon Hoare (until 2013), then the Mozilla Rust team (Brian Anderson, Patrick Walton, Niko Matsakis, Alex Crichton, Aaron Turon, Saoirse "boats" Shepherd, and others) and the open RFC process |
| Current version (Oct 2026) | Rust 1.99.0 (1 Oct 2026) [11]; six-week release train; current edition is Rust 2024 (stabilised in 1.85, 20 Feb 2025) [18] |
| Governance | Rust Project: Leadership Council (RFC 3392, June 2023) of representatives of nine top-level teams; design via RFCs; Rust Foundation (Feb 2021; AWS, Google, Huawei, Microsoft, Mozilla founding members) owns trademarks and funds infrastructure |
| Typing | Static, strong, nominal; local Hindley-Milner-style inference; algebraic data types; traits with monomorphised generics and `dyn` dispatch; no higher-kinded types |
| Memory | Ownership and borrowing checked at compile time; RAII (`Drop`); no GC; `unsafe` escape hatch |
| Compilation | AOT via LLVM (Cranelift backend experimental); crate is the compilation unit; monomorphisation |
| Primary domains | Systems software, CLI tools, network services, embedded, WebAssembly, OS components, browser engines |
| Notable users | Android platform (about 5 MLOC of Rust by late 2025), Linux kernel (promoted from experimental to core in Dec 2025), Microsoft Windows components, AWS (Firecracker), Cloudflare, Mozilla (Servo, Firefox), Meta, Dropbox, Discord, Figma, Chromium (Rust QR decoder) |

## Stated design goals

The language was built to replace C++ in Mozilla's browser engine work (Servo) with something
that was memory-safe without a garbage collector. The pitch on the official site has been
"performance, reliability, productivity"; the slogans "fearless concurrency" and "zero-cost
abstractions" summarise the two central bets: that the compiler can prove freedom from data
races and memory errors, and that safety need not cost runtime performance [1].

Graydon Hoare's own goals were different from what shipped. In 2023 he wrote that he "would have
traded performance and expressivity away for simplicity", and that the project's eventual
commitment to compete with C++ on performance was "broadly opposed to where a lot of people
wanted it to go" relative to his preferences [12]. Both goals matter for Sisu: the Rust that
exists is the result of the second set winning.

## Strengths

### Ownership and borrowing remove whole bug classes, and the industry data is now strong
The mechanism: every value has one owner; references are either one `&mut` or any number of
`&`; the compiler rejects programs that violate this ("aliasing XOR mutability"); lifetimes tie
references to the scopes of the data they point at; `Send`/`Sync` marker traits extend the same
rule to threads, giving data-race freedom in safe code. The result is that use-after-free,
double-free, dangling pointers, iterator invalidation and data races are compile errors in safe
Rust.

Evidence (Google Android, the best longitudinal dataset available):
- Memory-safety issues were 76% of Android vulnerabilities in 2019 and 24% in 2024, "well below
  the 70% industry norm", while new code shifted to memory-safe languages. The post argues that
  "vulnerabilities decay exponentially" with code age and that "the problem is overwhelmingly
  with new code", so writing new code in a safe language is enough to bend the curve [2].
- In 2025 memory-safety vulnerabilities fell "below 20% of total vulnerabilities for the first
  time". With roughly 5 million lines of Rust in the platform and one potential memory-safety
  vulnerability found and fixed pre-release (CVE-2025-48530 in CrabbyAVIF), Google's estimated
  density is 0.2 vulnerabilities per MLOC against about 1,000 per MLOC for historical C/C++, a
  "1000x reduction". About 4% of the Rust is inside `unsafe` blocks [3].
- Android 13 (Dec 2022): "about 21% of all new native code (C/C++/Rust) is in Rust", and "to
  date, there have been zero memory safety vulnerabilities discovered in Android's Rust code" [4].
- The Linux kernel accepted Rust in 6.1 (2022) and declared it no longer experimental in
  December 2025 [31].

Caveat: the Android numbers are correlational and Google says so ("while correlation doesn't
necessarily mean causation") [4]. They cannot separate ownership from the other things Rust
does (bounds checks, no UB in safe code, sum types), which matters for Sisu's design choices.

### Productivity parity, not just safety
Google's Lars Bergstrom (Rust Nation UK, March 2024) reported that C++-to-Rust rewrites needed
"a decrease by more than 2x in the amount of effort required", Go-to-Rust rewrites took "about
the same size team about the same amount of time", about a third of developers felt as
productive in Rust after two months and half after four, and 85% believed their Rust code was
more likely to be correct [5]. The November 2025 Android post adds measured data: Rust changes
have a "~4x lower" rollback rate for medium and large changes than C++, spend "25% less time in
code review", and need about 20% fewer revisions; "the safer path is now also the faster one"
[3]. Earlier the team had reported a rollback rate "less than half that of C++" [2]. Caveat: the
survey figures are self-reported and the methodology is undisclosed.

### `Result`, `?` and exhaustive matching make errors values without the Go boilerplate
RFC 243 (Sept 2014) introduced the postfix `?` operator as "syntactic sugar for `Result`" [16];
`?` was stabilised in 1.13 (2016), while the `catch`/`try` block half of the RFC remains unstable
a decade later (the 2025 survey still lists "try blocks" as a wanted feature, with 39% saying it
would improve their code [7]). RFC 3058 (2020) redesigned the `Try` trait so `Result`, `Option`
and `ControlFlow` cannot be mixed accidentally [17]. Errors are ordinary enums; `match` must be
exhaustive; `#[must_use]` on `Result` makes ignoring an error a warning. The idea has been copied
widely (Zig's `try`, Swift's `try`, C3's `!`, Gleam's `use`), which is the strongest adoption
evidence in this document.

### Cargo and crates.io are the reference implementation of package management
Cargo ships with the toolchain and gives: a lockfile (`Cargo.lock`) for reproducible builds,
semver-based resolution with caret ranges by default, additive feature flags, workspaces,
`cargo test`/`bench`/`doc` as first-class subcommands, and since the 2024 edition a resolver that
respects each dependency's declared `rust-version` [18]. crates.io is immutable (versions cannot
be deleted, only yanked). Nearly every language designed after 2015 has copied the "one
toolchain-bundled build tool plus lockfile" shape. Known costs: feature unification across a
dependency graph is a recurrent source of confusion, and the crate-as-compilation-unit model
interacts badly with compile times (see Weaknesses).

### Editions: the best known answer to evolving a language without a Python-3 split
RFC 2052 (June 2017) created editions: each crate declares its edition in `Cargo.toml`; crates
on different editions "must seamlessly interoperate"; the compiler supports all editions; the
core guarantee is that "warning-free code on edition N must compile on edition N+1 and have the
same behavior" [19]. Four editions have shipped (2015, 2018, 2021, 2024). Each is accompanied by
`cargo fix --edition` migrations. The constraint that keeps this working is that edition changes
are "skin deep": everything lowers to the same internal representation, so an edition can change
syntax, keywords, defaults and lints but not the type system or ABI [20]. The 2024 edition's
changes (RPIT lifetime-capture rules, `unsafe extern`, never-type fallback, rust-version-aware
resolver) are exactly that kind [18].

### Traits give one abstraction mechanism for static and dynamic dispatch
Traits serve as type classes (`impl Trait for Type`, blanket impls, associated types and
constants), as the generic bound mechanism (monomorphised, zero-cost), and as the dynamic
dispatch mechanism (`dyn Trait` fat pointers). `derive` macros generate the common impls
(`Debug`, `Clone`, `PartialEq`, `Hash`, `serde::Serialize`). Coherence (RFC 1023, 2015) guarantees
at most one impl of a trait for a type anywhere in the program, which is what lets a library add
an impl later without breaking downstream code [21]. Swift protocols, Carbon interfaces and Mojo
traits all adopted this shape.

### Tooling and diagnostics are a competitive advantage, not an afterthought
`rustfmt`, `clippy`, `rust-analyzer`, `cargo doc`/docs.rs and `cargo test` are maintained inside
the project. The compiler's error messages are widely praised, and the 2025 survey bears it out:
only 7.4% of respondents call "opaque/unclear compiler error messages" a big problem and 85%
say compiler bugs are "not an issue for me at all" [7]. The 2025 compiler-performance survey
found 87% of respondents rely on editor inline annotations [8].

### `unsafe` is a small, auditable boundary that the ecosystem actually respects
Unsafe operations are syntactically marked; the soundness contract is that safe code cannot
cause UB regardless of inputs. Google's 4% unsafe figure in Android [3] and the kernel's
acceptance suggest the boundary works in practice. Miri (an interpreter checking UB under the
Stacked/Tree Borrows aliasing models) and `cargo-geiger` exist because the boundary is explicit.

### A release process that has not broken users in eleven years
Six-week trains, nightly/beta/stable channels, feature gates, and "crater" runs that compile
the entire crates.io ecosystem before a release are why there has never been a Rust 2.0 and why
the Linux kernel could depend on it. This is a process lesson as much as a language one.

## Weaknesses

### The learning curve is real, measured, and does not flatten the way advocates claim
The Rust Vision Doc write-up (March 2026, Jack Huey) is titled "Rust's challenges don't
disappear with experience: they evolve": novices "struggle most with ownership and the borrow
checker"; experts stop citing the borrow checker but hit async complexity, ecosystem navigation
and domain constraints; async is described by interviewees as a "chasm of sadness"; a quoted
developer: "I actually did not understand the borrow checker until I spent a lot of time writing
Rust" [9]. JetBrains' 2025 ecosystem survey commentary: "Rust's reputation for being too difficult
has become quite sticky" [10]. In the 2025 State of Rust survey, "borrow checker not allowing
valid code" is a big problem for 9.5% and "could be improved" for a further 42.8% [7]. Google's
own figure is two to four months to feel productive [5].

The fixes that shipped are admissions of the problem: non-lexical lifetimes (RFC 2094, 2017,
shipped 2018) because the lexical borrow checker "rejected valid programs" [22]; disjoint
closure capture (RFC 2229, 2021 edition); the ongoing Polonius rewrite, which the GAT
stabilisation post names as the fix for a "known limitation in the current borrow checker" [23].
Lifetimes in struct definitions (`struct Parser<'a>`) and the viral `'a` annotations they
produce remain the canonical "wall"; Graydon's retrospective says he opposed explicit lifetime
syntax and believed lifetimes could almost always be inferred (per a secondary summary of his
post, [13]).

### Async Rust: function colouring, `Pin`, and no executor in the standard library
The history, from boats' "Why async Rust" (Oct 2023) [14]: Rust had green threads but removed
them in late 2014 (RFC 230) because they were not zero-cost, hurt FFI, and Rust has no GC to
support stack copying; futures were made poll-based so an `async fn` compiles to a single state
machine without allocation; because those state machines are self-referential and Rust has no
`Move` trait, `Pin` (RFC 2349, 2018) was introduced [24]; async/await (RFC 2394, 2018) stabilised
in 1.39 (Nov 2019) with no executor in `std` [25].

Documented consequences:
- The wg-async vision document collected 35 "status quo" stories; recurring themes are `Pin`
  ("very hard but often unavoidable"), cancellation at any `await`, synchronous `Drop`, `Send`
  bounds across `await`, executor lock-in ("libraries are tied to specific runtimes, which can
  panic when combined"), missing async traits/closures, future size and stack trouble, blocking
  calls that look like ordinary calls, and opaque debugging [15].
- `async fn` in traits took until 1.75 (Dec 2023) and still cannot express `Send`-ness
  generically ("a type must always decide if it implements the Send or non-Send version"), is
  discouraged in public traits, and is not object-safe [26].
- Survey 2025: "writing executor-agnostic async code" is a big problem for 17.6%, "achieving
  structured concurrency with async code" for 13.7% [7]; async struggles are called "an all-time
  favourite" of the open answers in the 2024 report [6].
- boats concedes the trait-object exposure of `Pin` was "an unforced error" that "would now be a
  breaking change to fix", and that the ecosystem has been "more stagnant since then than I think
  anyone involved would like" [14]. In "Let futures be futures" (Feb 2024) he still defends the
  model but lists blocking calls that look identical to async ones and `Mutex` held across
  `await` as unresolved hazards, and notes that Rust's C-runtime compatibility "forced stackless
  coroutines, which causes the function-coloring problem" [27].
- Graydon wanted green threads and would have accepted the runtime cost (paraphrase of [12];
  the primary post could not be re-fetched for this research, see Sources).

### Compile times are the number-one reported productivity problem
Survey 2025: "slow compilation" is a big problem for 27.3% and could be improved for 54.7%; it
is the top item, with "high disk space usage" (22.2%) second [7]. The dedicated 2025 compiler
performance survey (3,700+ responses): 55% wait more than ten seconds for the rebuild of their
most painful project; "waiting too long for an incremental rebuild" was "by far the most common
complaint"; about 45% of respondents who had stopped using Rust cited compile times as a reason
[8]. Causes named in the perf book and the survey post: LLVM optimisation time on large IR;
monomorphisation ("generic functions are often the most important ones"); proc macros that emit
large code; crates as serial compilation units; linking from scratch every build; default
debuginfo; and `cargo check` not sharing its cache with `cargo build` [8][28]. Mitigations in
flight: LLD as default linker on x86_64 Linux, Cranelift for debug builds, the parallel
front-end, "relink, don't rebuild" [8]. Graydon ties the problem to a design decision: "I wanted
crates to allow inlining inside but present stable entrypoints to the outside", which would also
have given a stable ABI; "I resisted this at the time and have objected to the choice ever
since" [12].

### No stable ABI and monomorphisation-only generics
Because generics are monomorphised and layout is unspecified, Rust binaries cannot expose Rust
APIs across a dynamic-library boundary without `extern "C"` or third-party crates
(`abi_stable`, `stabby`). Survey 2025: "implementing dynamic library plugins" is a big problem
for 13.0%, and "stable ABI" is the unimplemented feature with the highest "would unblock my
use-case" score at 13.9% [7]. Swift chose the opposite trade-off (resilient ABI); Graydon notes
Swift "wound up close to" what he wanted and that it is "a huge technical headache" [12].

### Complexity is the community's own top worry about the future
The 2024 and 2025 survey reports both say that the worry that Rust "becomes too complex" is one
of the top answers to "What are your biggest worries for the future of Rust?" [6][7] (the exact
percentages are in the PDF charts and could not be recovered as text for this research). Rust
in 2026 has two concurrency sub-languages (sync and async), two standard-library worlds (`std`
and `no_std`), GATs, const generics, RPITIT, async closures, `impl Trait` capture rules, and a
trait solver rewrite in progress. Graydon's judgement that modern Rust is "bureaucratic and
overly protective" and that he preferred a decidable, simpler type system (secondary summary,
[13]) is the sharpest designer-level statement of this concern.

### No specification for ten years; the gap was filled from outside
Rust 1.0 shipped without a language specification; the Reference is explicitly non-normative.
Ferrous Systems wrote the Ferrocene Language Specification to qualify a Rust toolchain for
ISO 26262 (ASIL D), IEC 61508 (SIL 3) and IEC 62304 (Class C) [29], and the Rust Project adopted
the FLS only in March 2025 ("this adoption is being made possible by Ferrous Systems"), with the
Reference unchanged and a stated hope to "bring the FLS and the Reference closer together" [30].
The Safety-Critical Rust Consortium was founded in June 2024 [32]. Interviewed safety-critical
teams still say "we don't have the same tools we have to measure its safety criticality as we
do in C++" [9].

### No higher-kinded types; GATs are the workaround and have sharp edges
Rust cannot abstract over type constructors, so there is no `Functor`/`Monad`, `Iterator` and
`Stream` are separate traits with duplicated adaptors, and `async` could not reuse `Iterator`.
Generic associated types (RFC 1598) took six years to stabilise (1.65, Nov 2022) and shipped
with four acknowledged limitations: the implied-`'static` problem with higher-ranked bounds,
non-object-safety of traits with GATs, borrow-checker gaps awaiting Polonius, and non-local
`where Self: 'a` clauses [23]. The lending-iterator pattern GATs enable is still not in `std`.
Survey 2025: "implementing logic for tuples of various sizes" (the absence of variadics) is a
big problem for 11.9% [7].

### Orphan rule and coherence push boilerplate onto users
RFC 1023 restricts foreign-trait-for-foreign-type impls and admits the newtype workaround "can
be annoying" [21]. Survey 2025: "splitting code across crates (e.g. orphan rule)" is a big
problem for 15.8% and could be improved for 43.8% [7]. The rule is what makes coherence
decidable, so no fix has been found that keeps both properties; the cost falls on every
`serde`-style integration between two third-party crates.

### Error-handling ecosystem churn shows what the standard library left out
`std::error::Error` shipped minimal: no context chaining, no derive, and no backtrace support
in `std` until the `Backtrace` type stabilised in 1.65 (2022), still with no stable way to attach
one through the trait. The ecosystem cycled through `error-chain` (2016), `failure` (2017;
now "deprecated", archived Aug 2021, README pointing to `anyhow` as "a good replacement for
`failure::Error`" and `thiserror` as a "near drop-in replacement for `#[derive(Fail)]`") [33],
then `anyhow`/`thiserror` (2019), `eyre`, `snafu`, `miette`. The split between "application"
errors (`anyhow`, opaque, with context) and "library" errors (`thiserror`, enumerated) is now
conventional but remains outside `std`, so every crate makes a choice and every user learns
two idioms.

### Governance crises, and the fixes
- 22 Nov 2021: the entire moderation team resigned "in protest of the Core Team placing
  themselves unaccountable to anyone but themselves" [34]. An interim "leadership chat" ran the
  project.
- May 2023: RustConf keynote speaker JeanHeyd Meneide was downgraded to a regular talk by a
  decision inside leadership chat; the project's statement: "That decision was not right ...
  We failed you JeanHeyd", blaming "decision-making and communication processes" of a body
  that "lacked clear rules" [35].
- April 2023: the Rust Foundation's draft trademark policy drew "widespread negative reactions"
  [36] (final outcome not verified here).
- Fix: RFC 3392 "Leadership Council" (opened Aug 2022, merged 20 June 2023) replaced the core
  team with one representative per top-level team, motivated by the core team "identifying and
  doing work" in the same body, which "has not scaled and has led to burnout" [37][38].
- Rust for Linux saw maintainer resignations and public disputes in 2024-2025 (not verified in
  primary sources for this research); the kernel nonetheless made Rust core in Dec 2025 [31].

### Strings: correct but a known trap
`String`/`&str` (owned/borrowed UTF-8), `OsString`/`OsStr`, `PathBuf`/`Path`, `CString`/`CStr`,
`Cow<str>`, no integer indexing, byte vs char vs grapheme iteration. The UTF-8 guarantee is right
and has been copied (Swift, Go); the type zoo is the price of not having a GC and of exposing
ownership in every signature. It is one of the first things beginners hit.

### `unsafe` is harder than C because the aliasing rules are stricter and still not finalised
Survey 2025: "writing correct unsafe code" is a big problem for 11.0% [7]. The aliasing model
(Stacked Borrows, then Tree Borrows) is still research-grade; `&mut` noalias guarantees make
patterns that are legal C into UB in Rust. The 2024 edition's `unsafe extern` and the
`unsafe_op_in_unsafe_fn` lint are incremental fixes.

## Designer retrospectives

**Graydon Hoare, "The Rust I Wanted Had No Future" (June 2023)** [12]. The original is behind a
Dreamwidth captcha and could not be fetched; quotations below come from pages that reproduce
it [12][13]. Verified quotes: "I would have traded performance and expressivity away for
simplicity"; on cross-crate inlining and ABI: "I wanted crates to allow inlining inside but
present stable entrypoints to the outside" ... "I resisted this at the time and have objected to
the choice ever since"; on integer overflow (he wanted promotion to bignums): "I wanted it to,
but I lost. Integers overflow and either trap or wrap. Great."; on tail calls: "I actually wanted
them! I think they're great."; conclusion: "the Rust We Got is many, many miles away from The
Rust I Wanted" and "The Rust I Wanted probably had no future, or at least not one anywhere near
as good as The Rust We Got." Points reported second-hand (treat as paraphrase): he disliked
"zero-cost abstractions" as a framing and would accept a constant-factor slowdown; wanted a
simpler grammar (square-bracket generics, no turbofish), inferred rather than written lifetimes,
structural typing, compiler-emitted type descriptors for reflection, built-in decimal floats,
green threads over async/await, and dictionary-passing/boxed generics over monomorphisation; and
he says he would not have been a good BDFL ("I don't like attention or public pressure").

**boats on async** [14][27]: the poll model and `Pin` were the right call given no runtime, but
`Pin` leaking into user code via trait objects was "an unforced error"; `Iterator` and `Drop`
should have supported immovable types; a language with stackful coroutines "could avoid
lifetimes entirely" in async code.

**The Vision Doc group (2025-2026)** [9]: the project's own interview study concluding that
difficulty is domain-specific and persistent rather than a one-time hump.

**RFCs as retrospectives**: NLL (2094), disjoint captures (2229), Try v2 (3058), Leadership
Council (3392), editions (2052) each exist because the previous design was found wanting.

## Influence

Copied from Rust by later languages: `?`-style propagation and `Result`/`Option` as the error
model (Zig, Swift, Kotlin's `Result`, Gleam, C3, Hare); ownership and borrow checking (Mojo,
Carbon's design, Swift's `~Copyable` and `borrowing`/`consuming` parameters, the C++ "Safe C++"
proposal P3390, Vale, Austral's linear types); traits-as-single-abstraction (Carbon interfaces,
Mojo traits); the bundled build tool with lockfile (Zig's build system and package manager,
Swift PM, Deno, uv's lockfile defaults); editions (no language has copied them exactly, which is
mild evidence that they depend on Rust's "skin deep" constraint); `unsafe` blocks as a marked
boundary (Zig does not copy this; Swift and C# have variants).

Reactions against Rust are also evidence: Hylo's mutable value semantics, Zig's and Odin's
rejection of the borrow checker, and Graydon's own post all target the same three costs:
learning curve, compile times, and the async/`Pin` complex.

Rust took from: C++ (RAII, monomorphised templates, zero-cost framing), ML and Haskell (ADTs,
pattern matching, type classes as traits, HM inference), Cyclone (region-based lifetimes),
Erlang and Newsqueak (the original task model, since removed), C# and JavaScript (async/await
syntax), Scheme (hygienic macros).

## Lessons for Sisu

- **Adopt**
  - Errors as values with sum types, exhaustive matching and a `?`-style propagation operator.
    This is Rust's most-copied feature and the surveys show it is not a pain point.
  - Automatic, deterministic resource cleanup (`Drop`/RAII) tied to scope, without a GC.
  - A thread-safety story checked by the compiler (`Send`/`Sync`-like capabilities). The
    Android data shows data-race freedom is part of what makes Rust code cheap to review.
  - Editions, with Rust's exact guarantee ("warning-free code on edition N must compile on
    N+1 with the same behavior") and the "skin deep" constraint that makes it hold.
  - One bundled build tool with an immutable registry, lockfile, semver and feature flags.
  - A marked `unsafe` boundary with a published soundness contract.
  - Formatter, linter, LSP and doc generator maintained in-tree from the first release, and
    diagnostics treated as a product feature.
  - A specification from the first release. Rust's ten-year gap was filled by a vendor, and
    safety-critical users still feel it.

- **Adapt**
  - Ownership without the full borrow checker. Keep single ownership and move semantics (they
    are cheap to learn and give RAII and `Send`); reconsider first-class references with named
    lifetimes, which is where the learning curve and the `struct Foo<'a>` wall come from.
    Candidates: second-class references or mutable value semantics (Hylo), region/arena-first
    allocation with generational handles (the Odin/Jai position), or inferred-only lifetimes
    (Graydon's preference). This is the single biggest design decision Sisu faces.
  - Generics: Rust's monomorphisation-only model is implicated in compile times, binary size
    and the missing ABI. Consider dictionary-passing or boxed generics as the default with
    monomorphisation as an opt-in (Swift's and Graydon's direction), and measure.
  - Concurrency: do not ship stackless async with `Pin` and no runtime interface in the
    standard library. Either (a) own a runtime and use stackful coroutines/green threads, or
    (b) make the I/O implementation an explicit value or effect so code is colourless (Zig's
    `std.Io`, see zig.md). Whichever is chosen, define cancellation semantics and a
    structured-concurrency primitive before 1.0.
  - Coherence: keep the "at most one impl" guarantee (it is what makes libraries composable)
    but give an explicit, language-level escape for the orphan problem rather than newtypes.
  - Error types: put context-chaining, backtraces and an "application error" type in the
    standard library from the start so the `failure`/`anyhow` churn does not repeat.
  - Strings: keep the UTF-8 guarantee and the owned/borrowed split, but reduce the number of
    string types users meet in the first week.
  - Governance: an RFC process plus a council with written decision rules and a separate
    foundation for trademarks is the current best practice; Rust had to learn the "written
    decision rules" part through two public failures.

- **Avoid**
  - Shipping concurrency primitives whose implementation detail (`Pin`) becomes user-facing.
  - Leaving the choice of executor, HTTP library and async traits to the ecosystem; it created
    executor lock-in that the project's own surveys still flag years later.
  - Crate-as-compilation-unit combined with monomorphisation and heavy proc-macro derive; the
    compile-time cost is now the top reported productivity problem.
  - Growing the language faster than its users can hold it; "too complex" is the community's
    own top worry.
  - A governance body that is accountable only to itself.
  - Deferring the specification.

- **Open questions raised**
  - How much of Rust's measured safety benefit comes from ownership/borrowing versus bounds
    checks, no-UB-in-safe-code and sum types? The Android data cannot separate them; this
    determines whether Sisu can drop the borrow checker and keep most of the benefit.
  - Is a two-to-four-month productivity ramp acceptable for Sisu's target users, or must the
    memory model be learnable in days?
  - Stackful coroutines (runtime cost, FFI friction, no embedded) versus stackless (colouring,
    `Pin`) versus explicit I/O capabilities (viral parameters): no language has all three
    properties (zero-cost, colourless, no runtime) and Rust's choice shows the cost of picking
    zero-cost.
  - Monomorphisation versus stable ABI: Rust and Swift took opposite sides and both pay.
  - Can higher-kinded abstraction be provided without Haskell-level complexity, or are GATs the
    right ceiling?
  - Is an orphan-rule-free coherence story possible?

## Sources
1. Rust website and "Rust 1.0" announcement, 2015. https://www.rust-lang.org/ and https://blog.rust-lang.org/2015/05/15/Rust-1.0.html
2. Google Security Blog, "Eliminating Memory Safety Vulnerabilities at the Source", 25 Sept 2024 (Jeff Vander Stoep, Alex Rebert). https://security.googleblog.com/2024/09/eliminating-memory-safety-vulnerabilities-Android.html
3. Google, "Rust in Android: move fast and fix things", 13 Nov 2025 (Jeff Vander Stoep). https://blog.google/security/rust-in-android-move-fast-fix-things/
4. Google Security Blog, "Memory Safe Languages in Android 13", 1 Dec 2022. https://security.googleblog.com/2022/12/memory-safe-languages-in-android-13.html
5. The Register, "Rust developers at Google twice as productive as C++ teams" (report of Lars Bergstrom's Rust Nation UK talk), 31 Mar 2024. https://www.theregister.com/2024/03/31/rust_google_c/
6. Rust Blog, "2024 State of Rust Survey Results", 13 Feb 2025 (7,310 responses). https://blog.rust-lang.org/2025/02/13/2024-State-Of-Rust-Survey-results/
7. Rust Blog, "2025 State of Rust Survey Results", 2 Mar 2026 (7,156 responses) and the PDF report (question "Which of the following aspects of Rust present non-trivial problems to your programming productivity?", n=6,653; "Which unimplemented features ...", n=6,514). https://blog.rust-lang.org/2026/03/02/2025-State-Of-Rust-Survey-results and https://raw.githubusercontent.com/rust-lang/surveys/main/surveys/2025/annual-survey/report/annual-survey-2025-report.pdf
8. Rust Blog, "Rust compiler performance survey 2025 results", 10 Sept 2025 (Jakub Beránek). https://blog.rust-lang.org/2025/09/10/rust-compiler-performance-survey-2025-results/
9. Rust Vision Doc, "Rust's challenges don't disappear with experience: they evolve", 20 Mar 2026 (Jack Huey). https://hackmd.io/@rust-vision-doc/rJKYOjIrWx
10. JetBrains, "State of Rust 2025" (Developer Ecosystem survey), 11 Feb 2026. https://blog.jetbrains.com/rust/2026/02/11/state-of-rust-2025/
11. Rust Blog, releases index (1.99.0, 1 Oct 2026). https://blog.rust-lang.org/releases
12. Graydon Hoare, "The Rust I Wanted Had No Future", June 2023 (primary: https://graydon2.dreamwidth.org/307291.html, not fetchable here); quotations verified via Michael Tsai's excerpt page https://mjtsai.com/blog/2023/06/08/the-rust-i-wanted-had-no-future/ and Bill Mill's link blog.
13. 36kr, Chinese-language summary of [12], 2023 (secondary; used only for paraphrased points). https://www.36kr.com/p/2428616311677697
14. Saoirse "boats" Shepherd, "Why async Rust", 15 Oct 2023. https://without.boats/blog/why-async-rust/
15. wg-async, "Async Rust vision document", status quo stories (2021-). https://rust-lang.github.io/wg-async/vision.html and .../vision/submitted_stories/status_quo.html
16. RFC 243, "Trait-based exception handling" (the `?` operator), 2014. https://rust-lang.github.io/rfcs/0243-trait-based-exception-handling.html
17. RFC 3058, "try_trait_v2", 2020. https://rust-lang.github.io/rfcs/3058-try-trait-v2.html
18. Rust Blog, "Announcing Rust 1.85.0 and Rust 2024", 20 Feb 2025. https://blog.rust-lang.org/2025/02/20/Rust-1.85.0/
19. RFC 2052, "Epochs" (editions), 2017. https://rust-lang.github.io/rfcs/2052-epochs.html
20. The Rust Edition Guide, "What are editions?". https://doc.rust-lang.org/edition-guide/editions/index.html
21. RFC 1023, "Rebalancing coherence" (orphan rules), 2015. https://rust-lang.github.io/rfcs/1023-rebalancing-coherence.html
22. RFC 2094, "Non-lexical lifetimes", 2017. https://rust-lang.github.io/rfcs/2094-nll.html
23. Rust Blog, "Generic associated types to be stable in Rust 1.65", 28 Oct 2022. https://blog.rust-lang.org/2022/10/28/gats-stabilization/
24. RFC 2349, "Pin", 2018. https://rust-lang.github.io/rfcs/2349-pin.html
25. RFC 2394, "async/await", 2018. https://rust-lang.github.io/rfcs/2394-async_await.html
26. Rust Blog, "Announcing `async fn` and return-position `impl Trait` in traits", 21 Dec 2023. https://blog.rust-lang.org/2023/12/21/async-fn-rpit-in-traits/
27. boats, "Let futures be futures", 3 Feb 2024. https://without.boats/blog/let-futures-be-futures/
28. Nicholas Nethercote, The Rust Performance Book, "Compile Times". https://nnethercote.github.io/perf-book/compile-times.html
29. Ferrocene (Ferrous Systems), qualified Rust toolchain. https://ferrocene.dev/
30. Rust Blog, "Adopting the FLS", 26 Mar 2025. https://blog.rust-lang.org/2025/03/26/adopting-the-fls/
31. Wikipedia, "Rust for Linux" (Linux 6.1 merge, Dec 2025 promotion). https://en.wikipedia.org/wiki/Rust_for_Linux
32. Rust Foundation, Safety-Critical Rust Consortium, founded June 2024. https://rustfoundation.org/safety-critical-rust-consortium/
33. `failure` crate README (deprecated; archived Aug 2021). https://github.com/rust-lang-deprecated/failure
34. rust-lang/team PR #671, moderation team resignation, 22 Nov 2021. https://github.com/rust-lang/team/pull/671
35. Rust Blog, "On the RustConf keynote", 29 May 2023. https://blog.rust-lang.org/2023/05/29/RustConf/
36. Wikipedia, "Rust Foundation" (founding Feb 2021; trademark draft April 2023). https://en.wikipedia.org/wiki/Rust_Foundation
37. RFC 3392, "Leadership Council", 2022-2023. https://rust-lang.github.io/rfcs/3392-leadership-council.html
38. Rust Blog, "Introducing the Rust Leadership Council", 20 June 2023. https://blog.rust-lang.org/2023/06/20/introducing-leadership-council/
