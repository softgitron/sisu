# Zig

## Snapshot
| | |
|---|---|
| First appeared | 2015-2016 (Andrew Kelley); first tagged release 0.1.1, Oct 2017 |
| Designed by | Andrew Kelley, with a core team of about 15 (Matthew Lugg, Jacob Young, Alex Rønne Petersen, Loris Cro, Isaac Freund, and others) |
| Current version (Oct 2026) | 0.17.0 (tagged 1 Oct 2026, announced 2 Oct); 0.16.0 on 13/14 Apr 2026; master is 0.18.0-dev. Release cadence of 5-8 months [17][5] |
| Governance | Zig Software Foundation (501(c)(3), New York; Andrew Kelley president) funds a small paid team from donations; language decisions by Andrew Kelley; design proposals are GitHub/Codeberg issues, not an RFC process; project moved from GitHub to Codeberg in Nov 2025 |
| Typing | Static, strong; types are first-class compile-time values; generics via `comptime` parameters ("compile-time duck typing"); no traits/interfaces |
| Memory | Manual, with explicit allocator parameters everywhere; no GC, no RC, no ownership checking; runtime safety checks in Debug and ReleaseSafe only |
| Compilation | Self-hosted compiler; own x86_64 backend is the default for Debug builds; LLVM for optimised builds; incremental compilation available (`-fincremental --watch`) but still experimental |
| Primary domains | Systems software, embedded, runtimes and databases, game and GUI tooling, and as a C/C++ cross-compiler and build system (`zig cc`) |
| Notable users | Bun (JavaScript runtime), TigerBeetle (financial database; also a ZSF sponsor), Ghostty (terminal), ZML; Mitchell Hashimoto pledged US$300k to ZSF in 2024 (user list from general knowledge; sponsors from the ZSF page) |

## Stated design goals

Zig's "Zen" is printed by `zig zen` and in the language reference: "Communicate intent
precisely. Edge cases matter. Favor reading code over writing code. There is an idiomatic way
to do it. Runtime crashes are better than bugs. Compile errors are better than runtime crashes.
Incremental improvements. Avoid local maximums. Reduce the amount one must remember. Focus on
logic, not style. Resource allocation may fail. Resource deallocation must succeed. Together,
we serve the users!" [1].

The positioning page "Why Zig when there is already C++, D, and Rust?" [2] states the
differentiators in one sentence each: "If Zig code doesn't look like it's jumping away to call a
function, then it isn't" (no hidden control flow: no operator overloading, no exceptions, no
destructors); "Zig has a hands-off approach when it comes to heap allocation" (no hidden
allocations; it singles out that "the main Rust standard library APIs panic on out of memory
conditions" and that "C++ coroutines allocate heap memory in order to call a coroutine");
first-class support for no standard library; "Zig is attempting to become the new portable
language for libraries"; "Zig is a toolchain in addition to a programming language"; and
simplicity: "Zig has no macros yet is still powerful enough to express complex programs in a
clear, non-repetitive way".

The stated niche is therefore C's niche, pursued with modern tooling: a language for people who
want to know "where are the bytes?" [1], not a memory-safe language.

## Strengths

### `comptime` replaces macros, templates and generics with one mechanism
Types are values: "In Zig, types are first-class citizens" that "can be assigned to variables,
passed as parameters to functions, and returned from functions"; "compile-time parameters is how
Zig implements generics. It is compile-time duck typing"; a generic container is "a function
that returns an anonymous struct" [1]. The same mechanism does conditional compilation
(`comptime if`), constant folding, type-level computation and reflection (`@typeInfo`), and
string formatting is implemented in the standard library rather than the compiler (the "Case
Study: print" section of the reference). The practical effect is one concept to learn where C++
has templates, `constexpr`, `if constexpr`, concepts and the preprocessor. Evidence of value:
later languages copied it (Mojo's parametric `comptime`-style metaprogramming; C3's `$if`/`$foreach`
compile-time constructs; Rust's steady expansion of `const fn` is the same pressure), and the
2025 Rust survey shows the demand on the other side ("compile time reflection" and "generic const
expressions" among the most-wanted unimplemented features) [3].

### Error sets, error unions and error return traces
Errors are values of error-set types ("an error set is like an enum", each error gets a nonzero
`u16`) combined with the success type as `E!T`. `try` propagates (`try a` is `a catch |err|
return err`), `catch` handles, `errdefer` runs cleanup "if and only if the function returned with
an error", error sets merge with `||`, and a function can let the compiler infer its error set
[1]. The distinctive feature is error return traces: every point at which an error was returned
is recorded, so a bare `error.FileNotFound` can be traced to its origin without a payload. The
reference states the cost: "practically free in terms of performance" on the success path and
"2 math operations plus some memory reads and writes" per return on the error path; they are on
by default in Debug and off in the release modes [1]. No other mainstream language has this; it
is the strongest argument that payload-free errors can be debuggable.

### Explicit allocators and "resource allocation may fail"
Functions that allocate take a `std.mem.Allocator`. The reference gives a ten-step decision list
("Choosing an Allocator") starting with: for a library, "best to accept an `Allocator` as a
parameter and allow your library's users to decide what allocator to use"; arena for
command-line programs and per-request/per-frame workloads; a testing allocator that detects
leaks; a safe debug allocator and a fast general-purpose one [1]. Out-of-memory is an ordinary
error (`error.OutOfMemory`), and the reference argues against relying on overcommit because not
all systems have it, real-time systems fix memory up front, and OOM handling makes libraries
reusable [1]. This design was adopted almost verbatim by Odin's context allocator, by C3, and by
Jai's context, and is the idiom Rust is still trying to retrofit (the "Allocator trait and better
OOM handling" item in the Rust survey) [3].

### Optionals, `orelse`, and null-pointer optimisation
`?T` is the only way to express absence; `null` has its own type and cannot be used as a `T`;
`a orelse b` unwraps with a default and `a.?` is `a orelse unreachable`. "An optional pointer is
guaranteed to be the same size as a pointer" and "the null of the optional is guaranteed to be
address 0" [1], so C-interop and data structures pay nothing. This is Rust's `Option<&T>` without
the trait machinery.

### `defer` / `errdefer` instead of RAII
Cleanup is written at the acquisition site and runs at scope exit, in reverse order; `errdefer`
runs only on the error path. Compared with destructors this is visible in the source (no hidden
control flow) and composes with the error model; compared with Go's `defer` it is block-scoped.
Odin, C3 and Jai all copied `defer`; only Zig has `errdefer`.

### A C toolchain, not just a C FFI
`zig cc` is a drop-in Clang driver that cross-compiles to any supported target from a single
downloaded archive, bundling libc headers and sources (musl, glibc, MinGW, WASI, macOS/Linux
headers); the build system "can be used even in C or C++ projects"; `translate-c` turns C headers
into Zig; "one of the primary use cases for Zig is exporting a library with the C ABI" [1][2].
Uber's use of `zig cc` to cross-compile its C/C++ (widely reported; not re-verified here) and
Bun's use of Zig for a V8-era JavaScript runtime are the adoption evidence. In 0.16 the C
translator moved off libclang onto Aro and the Zig implementation; in 0.17 `@cImport` was removed
from the language and C translation became an external ZSF package with its own release cadence
(proposal #20630, opened July 2024, closed April 2026) [4][5][6].

### I/O as an interface: colourless async by passing `Io` like an allocator (0.16, April 2026)
The design, described by Loris Cro in July 2025 [7] and shipped in 0.16.0 [4]:
- "Starting with Zig 0.16.0, all input and output functionality requires being passed an `Io`
  instance", where I/O means anything that "potentially blocks control flow or introduces
  nondeterminism" (files, sockets, sleeping, time, entropy, process spawning, mutexes) [4].
- `main` receives `std.process.Init` ("Juicy Main") whose `init.io` is "an appropriate default
  Io implementation based on the target configuration"; tests use `std.testing.io` [4].
- `io.async(fn, args)` returns a `Future(T)`; `future.await(io)` collects; `future.cancel(io)`
  requests cancellation. "It is legal for `Io` implementations to implement `async` calls simply
  by directly calling the function." `io.concurrent` "communicates that the operation must be
  done concurrently for correctness", allocates, and can fail with
  `error.ConcurrencyUnavailable` [4]. Cro's framing: "Asynchrony is not concurrency" [7].
- `Io.Group` gives structured concurrency (await or cancel N tasks together); `select` waits on
  several futures; `Batch` exposes operation-level concurrency (readv/writev-style) [4].
- Cancellation is cooperative: requests "may or may not be acknowledged"; acknowledged I/O
  returns `error.Canceled`; "only the logic that made the cancelation request can soundly ignore
  an `error.Canceled`"; `io.checkCancel` adds cancellation points in CPU-bound loops [4].
- Implementations: `Io.Threaded` (feature-complete, blocking syscalls on threads; with
  `-fsingle-threaded` neither concurrency nor cancellation), and `Io.Evented` ("work-in-progress,
  experimental": userspace stack switching, M:N green threads, io_uring and Grand Central
  Dispatch backends landed Feb 2026, kqueue proof-of-concept, no networking yet) [4][8].
- The claim: the same library runs unchanged under blocking, thread-pool, green-thread or
  (future) stackless implementations, so "Zig has completely defeated function coloring" [7].
  The Zig compiler itself runs on `Io.Evented`; "the `app` function is identical between those
  two snippets" [8]. Cost: `Io` is a vtable; with exactly one implementation calls devirtualise,
  with several they do not, and buffering moved into the `Reader`/`Writer` so only flushes cross
  the vtable [7].
This is the most important recent idea in this file for Sisu: it makes the execution model a
dependency-injected value instead of a property of the function's type.

### Compile speed as a first-class engineering goal, with a plan
Andrew Kelley's stated reason for competing with LLVM is that "Zig can substantially beat LLVM
on compilation speed" [8]. Since June 2025 the self-hosted x86_64 backend is the default for
Debug builds (not yet on Windows, pending COFF linker work) [8]; the new ELF linker took a
one-line-change rebuild from 194 ms to 65 ms in the 0.16 notes [4]; incremental compilation with
`-fincremental --watch` works on most x86_64-linux projects as of 0.17 [5]. The 0.17 roadmap
makes the aarch64 self-hosted backend "the default backend for debug mode" a goal [5].

### A small language with a sharp illegal-behaviour model
"Many operations in Zig trigger what is known as Illegal Behavior (IB)." Most IB is
safety-checked: integer overflow, out-of-bounds, unwrapping null or error, invalid enum/error
casts, misaligned pointers, wrong union field; `debug` and `safe` modes check, `fast` and `small`
do not; "unchecked Illegal Behavior still always results in a compile error if evaluated at
comptime"; `@setRuntimeSafety` toggles per scope; undefined memory is filled with `0xaa` in
checked modes [1]. There is no operator overloading, no macros, no hidden allocation, no
inheritance, no exceptions.

## Weaknesses

### Not memory-safe, and no plan to be
The reference is explicit: "Zig programmers must always be able to answer the question: where
are the bytes?"; "it is the programmer's responsibility to avoid accessing a pointer after the
memory it points to is gone"; dangling stack pointers are "unchecked Illegal Behavior" [1].
Use-after-free is caught only by the debug allocator in Debug/Safe builds; there is no
data-race freedom (sync primitives moved into `std.Io` in 0.16, but nothing prevents sharing a
`*T` across tasks); in `fast`/`small` modes safety checks are off and "anything can happen". No
Zig codebase has published data comparable to Android's for Rust [see rust.md]. The design bet is
that explicit allocators, arenas, slices with bounds checks, optionals and checked modes catch
most bugs at a fraction of Rust's complexity; it is plausible and unproven. "Runtime crashes are
better than bugs" is the honest statement of the trade-off [1].

### No interfaces or traits: duck typing hurts errors, documentation and tooling
Proposal #1268 ("comptime interfaces") was closed "not planned" by Andrew Kelley in May 2023:
"there are no current plans for adding them", users should accept that Zig "may never gain an
interface-like feature" [9]. The costs are catalogued in that thread by users: errors surface
deep inside a library rather than at the call site, no autocompletion for `anytype` parameters,
`anytype` described as a "wild west" for embedded HAL code, and standard-library signatures that
are "cryptic for newcomers" [9]. The standard library uses two incompatible patterns: `anytype`
duck typing (the pre-0.15 `Reader`/`Writer`) and hand-rolled vtables (`Allocator`, `Io`). The
0.15 "Writergate" rewrite replaced the generic duck-typed I/O with non-generic, vtable-based,
buffered `std.Io.Reader`/`Writer` [7]; that reversal is evidence the duck-typed approach did not
scale for the most-used interface in the library.

### Pre-1.0 instability: every release breaks every program
Ten years in, there is no stability promise. 0.16: "although your upgrade diff might be large,
it will be quite simple to understand what needs to be done" (every `file.close()` became
`file.close(io)`; `fs`, `net`, `process`, time and entropy APIs all changed) [4]. 0.17: `@bitCast`
semantics changed in a way that "has the potential to break existing code without triggering a
compile error", `containsAtLeastScalar` had its parameters swapped so "old calls may still
compile but behave incorrectly", `void{}`, `errdefer` captures and `i0` were removed, and the
build-system split "prevents the ZLS project from working with 0.17.0" at release time [5].
Earlier: `usingnamespace` removed (0.15), `async`/`await` removed (0.11). The C3 comparison page
notes Zig's async as "temporarily missing" for years [10]. The roadmap lists "completing and
stabilizing the language" as a requirement "before tagging Zig 1.0", with no date [5].

### The async story was removed in 2023 and its replacement is still experimental
Stage-1 Zig had `async`/`await`/`suspend`/`resume` with stackless coroutines. The self-hosted
compiler (default from 0.10, Oct 2022; the only compiler from 0.11, Aug 2023) did not implement
them; Andrew Kelley, July 2023: after
promising them for nine months, "today I am reneging", because "the project will benefit more
from releasing now than from waiting until async function support is restored" [11]. Issue
#6025 to re-introduce them was closed "not planned" in July 2025: Zig "will not have async/await
keywords"; "stackless coroutines may return to Zig, but if they do, it will be in a different
form" (#23446) [12]. The replacement (`std.Io`, above) shipped in April 2026 with the evented
implementation marked experimental, without networking, with io_uring "not the focus of this
release cycle", kqueue a proof-of-concept, and an "unexpected performance degradation" in the
compiler under `IoMode.evented` still undiagnosed [4][8]. Stackless coroutines, needed for WASM
and for embedded without stack switching, are not back. The design is promising; the evidence
that it works at ecosystem scale does not exist yet.

### Threading `Io` through everything is a real cost
The 0.16 definition of I/O is broad (time, randomness, sleep, mutexes), so almost every
non-trivial function now takes an `Io` next to its `Allocator`. Functions become generic over
the implementation but monomorphic in signature; libraries that need concurrency must use
`io.concurrent` and handle `error.ConcurrencyUnavailable`; code that uses `io.async` for things
that must overlap "can deadlock under blocking I/O", which Cro calls "a programming error" the
type system does not catch [7]. Whether two capability parameters on every call is acceptable
ergonomically is exactly the question Odin's implicit `context` answers differently (see
odin-jai-c3.md).

### Errors carry no payload
An error is a `u16` tag. Anything beyond the name (an errno, a path, a parse position) must be
smuggled through an out-parameter or a diagnostics struct passed in by the caller. Error return
traces mitigate the "where did it come from" half of the problem but not the "what was the
context" half. The C3 comparison lists this implicitly by distinguishing its fault model;
Rust's `thiserror`/`anyhow` ecosystem exists because payloads matter. (Community complaint,
widely documented; no designer statement found.)

### BDFL governance with no written process
Decisions are made by Andrew Kelley; the Contributing and Language Proposals wiki pages now
redirect elsewhere ("Content moved to README.md"/code of conduct) [13]; proposals are issues
that the lead accepts or closes. This produced fast, coherent decisions (removing `async`,
`@cImport`, `usingnamespace`; the Codeberg migration announced "effective immediately" with a
"strict no LLM / no AI policy") [14] and also the nine-month broken promise on async [11]. The
ZSF publishes financials and board minutes and is donation-funded [15]; bus factor and
sustainability are the obvious risks, and there is no mechanism for the community to overrule
a design decision.

### Two code generators to maintain, and the self-hosted one is not yet good
The 0.16 notes say the x86 backend's machine code quality is "inferior" to LLVM's and it passes
fewer behaviour tests; the aarch64 backend "currently crashes when running behavior tests"; the
LLVM backend remains necessary for release builds [4]. The long-term goal is "moving from a
library dependency on LLVM to a process dependency on Clang" [4][5]. Until then users get fast
debug builds and slow release builds with different bugs.

### Ecosystem size and the absence of a registry
Stack Overflow 2025: 2.1% of respondents use Zig (Rust 14.8%) [16]. The package manager (0.11,
2023) fetches by URL and hash; there is no central registry by design, so discovery and
transitive-dependency auditing are manual. Package management moved from the compiler into
the build system in 0.17 [5]; the tooling is still changing.

### Safety only in Debug/ReleaseSafe, and the fast default is tempting
Because `ReleaseFast` removes every check and is what benchmarks use, the practical safety level
of shipped Zig binaries depends on a build flag. There is no equivalent of Rust's "safe subset is
safe at every optimisation level". The reference's own advice is to prefer `ReleaseSafe`; the
documentation table lists `safe` as "medium" runtime performance [1].

## Designer retrospectives

- **On async (2023, 2025):** "Well, today I am reneging" on the promised async functions [11];
  the keywords will not return [12]; the replacement is an interface value, not syntax. This is
  the clearest case in this research of a designer abandoning a shipped language feature.
- **On duck-typed I/O (2025, "Writergate"):** the generic `anytype` reader/writer design was
  replaced by non-generic buffered interfaces because de-virtualised buffering is both faster and
  simpler [7] (Andrew Kelley's own "Don't forget to flush" post was not fetched; the account here
  is Cro's).
- **On `@cImport` (2024-2026):** removed from the language so the compiler drops libclang and so
  "`TranslateC` can gain advanced options ... without changing the language"; Kelley insisted on
  "a migration proposal that is appealing" before removal [6].
- **On LLVM (2025):** worth replacing because "Zig can substantially beat LLVM on compilation
  speed" [8].
- **On the Zen's trade-off:** "Runtime crashes are better than bugs. Compile errors are better
  than runtime crashes" [1] is a ranked statement of priorities, not a claim of safety.

## Influence

From Zig to others: explicit allocator parameters and arena idioms (Odin's `context.allocator`,
C3, Jai's context predates but Zig popularised the explicit form); `comptime` as generics
(Mojo's parameters, C3's compile-time constructs); `try`/error unions (C3's `!` and faults);
`defer` (Odin, C3); `zig cc` as a cross-compiler used by Go and C projects; the single-archive
toolchain distribution; the `Io`-as-value idea is new (2025-2026) and has not yet been copied.

From others to Zig: `defer` from Go; error-as-value and `try` from Rust's `?`; `comptime` is in
the lineage of Jai's `#run` and D's CTFE; the allocator interface resembles C++'s `std::pmr`;
`Io`'s "asynchrony is not concurrency" distinction is close to Kotlin's structured concurrency
and to effect-handler designs (Koka, Effekt) where the handler is chosen by the caller.

## Lessons for Sisu

- **Adopt**
  - Error return traces. They solve the "where did this error come from" problem at near-zero
    success-path cost and make payload-light errors viable.
  - `errdefer` alongside `defer`, or an equivalent, if Sisu has any explicit cleanup.
  - Allocation failure as an ordinary error in library code, and allocator parameterisation
    (explicit or via an implicit context) for anything that allocates.
  - Null-pointer optimisation for optionals and the rule that absence has exactly one spelling.
  - "No hidden control flow" as a readability principle: whatever Sisu's answer to destructors
    and operators, the reader must be able to see where calls happen.
  - A checked-illegal-behaviour catalogue with the rule that anything undefined at runtime is a
    compile error at compile time.
  - A toolchain that is one archive, cross-compiles by default, and can build C/C++ too; this is
    a large part of Zig's adoption.

- **Adapt**
  - `Io`-as-value is the right shape for colourless concurrency, but Sisu should make it a
    tracked capability or effect rather than an ordinary parameter, so that (a) it is not
    boilerplate on every signature, (b) "needs concurrency" versus "may run inline" is a static
    property, and (c) the deadlock case Cro describes becomes a compile error. Odin's implicit
    `context` and effect systems (Koka/Effekt) are the two directions.
  - `comptime` generics plus a way to state constraints. Keep types as values and compile-time
    evaluation, add named constraints (concepts/traits) so that errors land at the call site
    and tooling can complete members. Zig's own refusal (#1268) is the clearest evidence of the
    cost of not doing this.
  - Error values with optional payloads: keep Zig's cheap tagged errors as the default and let
    an error carry data when needed.
  - Make the safe build mode the default for release and require opting out per module, not
    per build.

- **Avoid**
  - Shipping without any memory-safety story for release builds. Zig's bet may pay off for
    expert teams; the Android data say it will not for the industry at large.
  - Duck typing as the only polymorphism mechanism.
  - A decade of pre-1.0 breakage; Sisu should define a stability policy (editions or
    equivalent) before it has users.
  - Removing a shipped feature with no replacement available ("today I am reneging").
  - Governance that depends on one person's time, with no written decision process.

- **Open questions raised**
  - Explicit `Io`/`Allocator` parameters versus implicit context versus effects: which gives
    the readability Zig wants with the ergonomics it lacks?
  - Stackful (green threads, stack switching, no WASM) versus stackless (WASM, no stack
    switching, but a calling-convention rewrite) coroutines behind the `Io` interface: Zig has
    not settled it after three years.
  - Can arenas, bounds-checked slices, optionals and checked builds deliver a measurable
    fraction of Rust's safety benefit without ownership? Nobody has the data.
  - Is "no traits" sustainable once a language has a large standard library, or will every
    such language converge on vtable-plus-duck-typing like Zig did?

## Sources
1. Zig Language Reference (master, Oct 2026): Zen, Errors, Optionals, comptime, Memory, Optimization Mode, Illegal Behavior, C. https://ziglang.org/documentation/master/
2. "Why Zig When There is Already C++, D, and Rust?" https://ziglang.org/learn/why_zig_rust_d_cpp/
3. Rust Blog, 2025 State of Rust Survey results and PDF report, Mar 2026 (for the comparison items). https://blog.rust-lang.org/2026/03/02/2025-State-Of-Rust-Survey-results
4. Zig 0.16.0 release notes, 13-14 Apr 2026 (244 contributors, 1,183 commits): "I/O as an Interface", Compiler, Linker, Roadmap. https://ziglang.org/download/0.16.0/release-notes.html and https://ziglang.org/news/0.16.0-released/
5. Zig 0.17.0 release notes, 1-2 Oct 2026 (206 contributors, 925 commits): language changes, `@cImport` removal, build system, backends, roadmap. https://ziglang.org/download/0.17.0/release-notes.html and https://ziglang.org/news/0.17.0-released/
6. ziglang/zig issue #20630, "move `@cImport` to the build system", Jul 2024 - Apr 2026. https://github.com/ziglang/zig/issues/20630
7. Loris Cro, "Zig's New Async I/O", 13 Jul 2025. https://kristoff.it/blog/zig-new-async-io/
8. Zig devlog 2025 and 2026 (Andrew Kelley, Matthew Lugg): x86 backend default (8 Jun 2025), std.Io DNS example (15 Oct 2025), io_uring/GCD `Io.Evented` (13 Feb 2026), incremental compilation with LLVM (8 Apr 2026), build system rework (26 May 2026). https://ziglang.org/devlog/2025/ and https://ziglang.org/devlog/2026/
9. ziglang/zig issue #1268, "comptime interfaces", closed not planned 31 May 2023. https://github.com/ziglang/zig/issues/1268
10. C3 documentation, "Comparisons with other languages" (Zig entry). https://c3-lang.org/faq/compare-languages/
11. Andrew Kelley, "The Upcoming Release Postponed Two More Weeks and Lacks Async Functions", 19 Jul 2023. https://ziglang.org/news/0.11.0-postponed-again/
12. ziglang/zig issue #6025, "async/await/suspend/resume", closed not planned 15 Jul 2025. https://github.com/ziglang/zig/issues/6025
13. ziglang/zig wiki, "Contributing" and "Language Proposals" (redirect notices, Nov 2025). https://github.com/ziglang/zig/wiki/Contributing
14. Andrew Kelley, "Migrating from GitHub to Codeberg", 26 Nov 2025. https://ziglang.org/news/migrating-from-github-to-codeberg/
15. Zig Software Foundation page (board, sponsors, financial reports). https://ziglang.org/zsf/
16. Stack Overflow Developer Survey 2025, "Most popular technologies" (Rust 14.8%, Zig 2.1%). https://survey.stackoverflow.co/2025/technology
17. Zig download page (release list and dates). https://ziglang.org/download/
