# C

## Snapshot
| | |
|---|---|
| First appeared | 1972 (Bell Labs, PDP-11 Unix); K&R book 1978; ANSI C 1989; ISO C90 |
| Designed by | Dennis Ritchie, with Ken Thompson (B) and later Brian Kernighan; standardised by ANSI X3J11 / ISO WG14 |
| Current version (Oct 2026) | C23, published as ISO/IEC 9899:2024 on 31 Oct 2024. Next revision "C2Y" (expected to be called C29, target late 2029); working drafts N3854 (Mar 2026) / N3886 (May 2026, sources conflict). GCC 15 defaults to C23; Clang support partial. |
| Governance | ISO/IEC JTC1/SC22/WG14; national-body voting; three-ish year drafts, roughly 6-12 year standards. Compilers (GCC, Clang, MSVC) and their extensions are de facto co-governors. |
| Typing | Static, weak (implicit integer/pointer conversions, `void*` escapes), nominal structs, no generics (only `_Generic` selection since C11) |
| Memory | Manual (`malloc`/`free`), raw pointers, no bounds or lifetime checking |
| Compilation | AOT to native via separate compilation of translation units, textual `#include`, link-time symbol resolution |
| Primary domains | Operating systems, embedded/firmware, language runtimes and interpreters, libraries exported to every other language, HPC glue, networking |
| Notable users | Linux, FreeBSD, Windows kernel parts, SQLite, PostgreSQL, CPython, Git, curl, OpenSSL, glibc/musl, nearly every device firmware. Stack Overflow 2025: 22% of all respondents used C in the past year; TIOBE Sep 2026: #2 at 10.28%. |

## Stated design goals
Ritchie's HOPL-II retrospective "The Development of the C Language" (1993) describes C as the
result of wanting a language "efficient enough to displace assembly language, yet sufficiently
abstract and fluent to describe algorithms and interactions in a wide variety of environments"
and summarises the whole thing as "C is quirky, flawed, and an enormous success" (quoted from
memory; the paper is behind a redirect that could not be fetched during this research, so
wording should be checked against the HOPL-II text before citing it in the proposal).

The goals were never written down as a manifesto; they are recoverable from the choices:

- Be a *system implementation language* for Unix on a 24 KB PDP-11: every construct must map to
  a few machine instructions, and the compiler must be small enough to be ported by one or two
  people.
- Do not hide costs. No garbage collector, no runtime type information, no bounds checks, no
  hidden allocations. The standard library is a library, not part of the language (I/O is
  `stdio`, not syntax).
- Portability through *weak specification*: leave integer widths, evaluation order, char
  signedness and many behaviours implementation-defined or undefined so that one language could
  target machines with 36-bit words, one's complement arithmetic and segmented memory.
- "Trust the programmer": the ANSI C Rationale (1989) lists among the committee's principles
  "Trust the programmer", "Don't prevent the programmer from doing what needs to be done",
  "Keep the language small and simple", "Provide only one way to do an operation", and "Make it
  fast, even if it is not guaranteed to be portable".

The niche was: the language that the operating system is written in, and therefore the language
that every other language must talk to.

## Strengths

### Simplicity bought portability, and portability bought everything else
The K&R-era language fits in a short book; a C compiler can be written by a small team (the
first Unix port to the Interdata 8/32 in 1977-78 was done by Ritchie and Steve Johnson with the
Portable C Compiler). Because a compiler was cheap, every CPU vendor shipped one, and because
every CPU had a compiler, every operating system, runtime and library ended up written in C.
Evidence: C remains #2 on TIOBE 54 years after its creation and is used by roughly a fifth of
all developers (Stack Overflow 2025: 22%), despite having gained almost no new language
features that users ask for. Go's designers explicitly cite this: Pike's "Go at Google" (2012)
says Go had to be "familiar, roughly C-like" because Google's programmers were "most familiar
with procedural languages, particularly from the C family". Zig's "Why Zig" page calls C "the
most versatile and portable language" and sets the goal of being equally portable for
libraries.

### C is the ABI lingua franca
C is the only language with a stable, documented calling convention on every platform (System V
AMD64 psABI, AAPCS64, Windows x64, the Itanium C++ ABI's C subset). Every language FFI is
therefore "call C": Python's `ctypes`/CPython API, Java JNI, Go `cgo`, Rust `extern "C"`,
Swift's C importer, Fortran's `iso_c_binding` (standardised in Fortran 2003). This happened
because the language has almost no runtime: no exceptions, no destructors, no hidden `this`, no
closures, no generics. A struct is a layout; a function is a symbol. The consequence for any
new language is that "FFI" means "produce and consume C ABI", and that the subset of a language
that can cross an ABI boundary is the C-shaped subset. Zig, Odin and C3 all advertise "C ABI
compatibility" as a headline feature (C3: "full C ABI compatibility out of the box"); Carbon's
entire reason to exist is interop with existing C/C++.

### No hidden control flow or allocation
What you see is what executes: a function call is a `call`, `a + b` is an add, assignment is a
copy of the bytes. This property is what embedded, kernel and real-time programmers value, and
it is why both Zig ("If Zig code doesn't look like it's jumping away to call a function, then
it isn't"; "no hidden allocations") and Odin copy it exactly and advertise it as a feature. The
lesson is that predictability of cost is a *readability* property, not just a performance one.

### Separate compilation and a flat, cheap object model
C's compilation model (translation units, object files, a linker) predates and outlives all
build systems. It allows incremental builds, binary distribution of libraries, dynamic linking,
and mixing compilers. Pike's 2012 paper gives the cost side (below) but the model itself was
adopted by nearly every AOT language.

### Small core, long-lived code
Code written in 1989 still compiles in 2026 with warnings, not errors. The standard removes
things very slowly (K&R function definitions were finally removed only in C23, 34 years after
prototypes became standard; trigraphs were removed in C23). For a language used in
30-year-lifetime infrastructure this stability is a feature, and WG14's charter explicitly
prioritises existing code.

### Low-level control that higher-level languages still reach for
Bit-level layout control (`struct` packing, unions, pointer casting, `volatile`, inline asm as
an extension) means C can express memory-mapped I/O, custom allocators and lock-free data
structures without leaving the language. Languages that lack this (Java, Go before `unsafe`
tricks) end up calling C for it.

## Weaknesses

### Undefined behaviour as a design choice, and compiler exploitation of it
C has roughly 200 explicitly undefined behaviours (Annex J of C11 listed 203). UB was
originally a portability device: "the compiler need not define this because machines differ".
From the late 1990s, optimisers started treating UB as "cannot happen" and deleting code around
it. Concrete consequences:

- **CVE-2009-1897 (Linux `tun_chr_poll`)**: a pointer was dereferenced before its null check;
  GCC inferred the pointer could not be null and removed the check, turning a benign bug into
  an exploitable one. The kernel normally maps nothing at page zero, so the vulnerability
  existed only because of the optimisation.
- **Wang, Zeldovich, Kaashoek, Solar-Lezama, "Towards Optimization-Safe Systems" (SOSP 2013)**:
  their STACK checker found ~160 instances (the paper and thesis variously say 158, 160, 161)
  of "optimization-unstable code" in real systems, where a compiler is permitted to discard
  checks because of UB elsewhere.
- **Linux opts out of the language**: the kernel `Makefile` compiles with
  `-fno-strict-aliasing`, `-fno-delete-null-pointer-checks`, `-fno-strict-overflow` ("disable
  invalid 'can't wrap' optimizations for signed / pointers"), and
  `-fno-allow-store-data-races`. The most important C program in the world is not written in
  standard C.
- **"Friendly C" (Cuoq, Flatt, Regehr, Aug 2014)** proposed 14 changes turning UB into
  unspecified values (signed overflow wraps, dangling pointers keep their value, strict
  aliasing removed, `memcpy` = `memmove`, reads of uninitialised memory yield unspecified
  values, out-of-bounds pointer arithmetic allowed). The stated price: "trades some
  optimization capability for ease of reasoning". Regehr's follow-up "The Problem with Friendly
  C" (Dec 2015) gave up: "there are just too many variations, each with its own set of
  performance tradeoffs"; even `memcpy`-as-`memmove` was contested, and for "shift a 32-bit
  integer by 32" ARM code wants 0 while x86 code wants the original value. His conclusion was
  that an influential group (he suggested Android) should just define a dialect and use it,
  rather than seeking consensus.
- **C23 moved a little**: two's complement is now the only permitted representation (N2412), so
  the *representation* is defined though signed overflow remains UB;
  `ckd_add`/`ckd_sub`/`ckd_mul` (N2867) give checked arithmetic as library macros;
  `[[unsequenced]]` and `[[reproducible]]` let the programmer *grant* optimisation permissions
  explicitly rather than the compiler inferring them from UB. WG21 has gone further for C++29
  by adopting annexes that catalogue every undefined and ill-formed-no-diagnostic case
  (P3596R3, June 2026); WG14 has not yet done the equivalent for C2Y as far as this research
  found.

The lesson is not "UB is evil" but that *leaving semantics undefined to permit optimisation
without a mechanism for the programmer to say which behaviour they want* produces a language
where the fastest compiler is the most dangerous one.

### Memory unsafety: the dominant security cost of the last 30 years
Industry data from three vendors agrees:
- Microsoft (Matt Miller, BlueHat IL, Feb 2019): ~70% of CVEs patched by Microsoft over the
  preceding 12 years were memory-safety issues.
- Chromium (2020): of 912 high/critical security bugs on the stable branch since 2015, ~70%
  were memory unsafety, half of those use-after-free.
- Android (Google, Nov 2025): memory-safety vulnerabilities fell below 20% of Android's total
  for the first time, from the historical ~70%, after shifting new code to Rust; Google reports
  a 1000x lower memory-safety vulnerability density in Rust vs C/C++ code, a 4x lower rollback
  rate and 25% less review time for Rust changes.

The policy response: the US ONCD report "Back to the Building Blocks" (26 Feb 2024) and the
CISA/FBI/NSA "Case for Memory Safe Roadmaps" asked manufacturers to publish memory-safety
roadmaps by 1 Jan 2026 (non-binding). C was the proximate target.

Mitigations proposed *within* C, in order of maturity:
1. **Hardening and sanitizers** (ASan, UBSan, `_FORTIFY_SOURCE`, stack protectors, CFI): find
   bugs in testing, do not prevent them in production.
2. **Apple/Clang `-fbounds-safety`** (announced 2023, upstreaming into Clang through 2025):
   annotations `__counted_by(N)`, `__sized_by(N)`, `__ended_by(P)`, `__single`, `__indexable`,
   `__bidi_indexable`; unannotated ABI-visible pointers default to `__single` (no arithmetic),
   locals default to wide pointers; out-of-bounds becomes a deterministic trap. The design doc
   claims adoption "on millions of lines of production C code" in "a consumer operating system"
   (Apple's; not independently verified). It is C-only, not C++/ObjC, and the upstream docs
   still say "not available for users yet" as of this research. This is the most credible path
   to spatial safety for existing C.
3. **CHERI**: hardware capabilities (128-bit pointers carrying bounds and permissions). MSRC's
   2020 analysis estimated CHERI "in its current state, and combined with other mitigations"
   would deterministically mitigate "at least two thirds" of the memory-safety vulnerabilities
   Microsoft received in 2019, with temporal safety still open. Status Oct 2026: Arm Morello
   (Jan 2022) remains a research platform; CHERIoT 1.0 spec published Nov 2025 with first
   silicon (SCI ICENI, CHERIoT-Ibex) heading to production; RISC-V International is
   standardising a CHERI base ISA. The cost is a new ISA plus recompilation plus fixing
   pointer-provenance bugs, so it is a 10-year path, not a compiler flag.
4. **MISRA C / CERT C / static analysis**: subsetting rules; effective in safety-certified
   domains but not adopted broadly.

### No modules or namespaces: the preprocessor as module system
`#include` is textual. Pike (2012) measured a 2007 Google build where "the C++ input grew to a
blow-up of 2000 bytes for every C++ source byte"; already in 1984 `<sys/stat.h>` was included
37 times while compiling `ps.c`. There are no namespaces, so every library prefixes identifiers
by hand (`sqlite3_`, `SDL_`, `gtk_`), and symbol collisions at link time are a routine failure
mode. Workarounds (include guards, `#pragma once`, precompiled headers, unity builds,
"single-header libraries" like stb) are all conventions, not language. C23's `#embed` (N3017)
and `__has_include` (N2799) extend the preprocessor rather than replace it, and C2Y shows no
module proposal; WG14 has effectively decided this will not be fixed. Every C successor fixes
it first: Go packages, Zig's `@import`, Odin packages, C3's "simple and straightforward module
system".

### The preprocessor itself
`#define` is untyped textual substitution with no scoping, no hygiene and no knowledge of the
language. It is used for constants, inline functions, generics (`#define LIST(T) ...`),
conditional compilation, and configuration, because the language offers nothing else. Ritchie's
retrospective credits it to Alan Snyder and BCPL/PL/I file inclusion rather than to any design
intent. Consequences: debuggers see expanded code, tooling (IDEs, refactoring) must run the
preprocessor to understand anything, and portable code becomes a thicket of `#ifdef`. C99
`inline`, C11 `_Generic` and C23 `constexpr` objects (N3018), `typeof` (N2899), `auto` (N3007)
and `[[attributes]]` each replace one use of macros; none replaces the preprocessor. Zig ("Zig
has no macros"), Odin and C3 ("semantic macros ... miles beyond C's preprocessor") all remove
it and provide compile-time evaluation instead.

### No real strings, no slices
A string is a `char*` to a NUL-terminated buffer; an array parameter decays to a pointer and
loses its length. This single representation choice is the direct cause of the
`strcpy`/`sprintf`/`gets` family of buffer overflows and of `strlen` being O(n). The standard
library's attempts to patch it (`strncpy` with its non-terminating edge case, Annex K `_s`
functions that only Microsoft implemented, C23 `strdup`/`strndup`/`memccpy`) are additive.
Every successor introduces a (pointer, length) pair as a primitive: Go slices and strings, Zig
`[]T`, Odin slices, Rust `&[T]`/`&str`, C3 slices, and `-fbounds-safety` re-introduces it to C
as "wide pointers". Lesson: a fat pointer is the minimum viable array type.

### No generics, no sum types, no error type
Generic code is done with `void*` plus function pointers (`qsort`), with macros, or by textual
copy. There are no tagged unions (unions are untagged; enum + union + discipline), so errors
are returned as `-1`/`NULL`/`errno`, which is ignorable by default and conflates "no value"
with "error". C11 `_Generic` provides overloading on type, not parametric code. This is the
largest expressiveness gap and the reason "C with generics" (C3, Odin, Zig's `comptime` types)
is a recurring project.

### Integer semantics
Implicit promotions and the "usual arithmetic conversions" (signed/unsigned comparison, `char`
promoted to `int`, `int*int` overflow being UB even when both operands came from `uint16_t`)
are a documented defect class (CERT INT30-INT36). C23 added `_BitInt(N)` (N2763) and checked
arithmetic macros but did not change the conversion rules; a new language should not copy them.

### Declaration syntax
"Declaration reflects use" (`int (*f[3])(void)`) is the one part of C's syntax that later
C-family languages (Go, Rust, Zig, Odin, C3, Carbon) all abandoned in favour of left-to-right
types. Ritchie himself acknowledged the syntax "contributed to the perceived complexity of the
language" while still defending the idea (HOPL-II; from memory, see caveat above).

### Build and packaging are not the language's concern
There is no package manager and no standard build description; Autotools, CMake, Meson,
pkg-config and vendoring coexist. Each is a workaround for the absence of modules and of a
standard way to express dependencies. This is a weakness shared with and amplified in C++ (see
`cpp.md`).

## Designer retrospectives
- Ritchie, HOPL-II (1993): C "is quirky, flawed, and an enormous success"; the two ideas "most
  characteristic of C among languages of its class" are "the relationship between arrays and
  pointers, and the way in which declaration syntax mimics expression syntax". He regarded the
  preprocessor as an add-on rather than a design, and attributed C's spread primarily to Unix
  and the PDP-11. (All from memory; the canonical copy was not retrievable in this session.)
- Ritchie and Thompson did not predict that undefined behaviour would be *exploited* by
  compilers; the ANSI Rationale frames UB as licence for implementations to not diagnose, not
  as an optimisation contract. The shift was made by compiler writers, not language designers,
  and no designer retrospective endorses it.
- WG14's own direction, visible in C23 and C2Y, is to adopt C++ and successor features
  piecemeal (`nullptr`, `constexpr`, `auto`, `typeof`, `[[attributes]]`, `defer` in C2Y,
  `_Countof`, labelled loops, `case` ranges) while refusing to add modules, generics or a
  string type. That is an implicit admission of which gaps are painful and which the committee
  considers un-fixable within C.

## Influence
What C gave to others (evidence the ideas work): block syntax and operators (C++, Java, C#,
JavaScript, Go, Rust, Zig, Odin, C3, Swift, Kotlin, D); the ABI; separate compilation and the
linker model; the idea that a systems language has no runtime; `printf`-style formatting; the
`main`/exit-code/`argv` program model. Notably *not* copied by any later design: `errno`,
NUL-terminated strings, the preprocessor, and declaration-reflects-use syntax.

What later systems languages kept and dropped (compiled from each project's documentation;
individual cells not re-verified for this research):

| | Go | Zig | Odin | C3 |
|---|---|---|---|---|
| C ABI as the FFI | kept (cgo) | kept, plus `zig cc` | kept | kept ("full C ABI compatibility") |
| Pointer arithmetic | dropped | restricted (`[*]T` many-pointers) | restricted | kept |
| Preprocessor / textual macros | dropped | dropped (`comptime`) | dropped | dropped (semantic macros) |
| Header files | dropped (packages) | dropped (`@import`) | dropped (packages) | dropped (modules) |
| Implicit numeric conversions | dropped | dropped (explicit `@intCast`) | dropped | mostly dropped |
| NUL-terminated strings as *the* string | dropped (slices) | dropped (slices, sentinel-terminated as option) | dropped (slices) | dropped (slices) |
| Manual memory | GC | kept, allocators explicit | kept, context allocators | kept |
| UB on overflow | defined wrap | safety-checked in Debug/ReleaseSafe, UB in ReleaseFast | defined | defined/checked |
| Declaration syntax | left-to-right | left-to-right | left-to-right | C-like |
| No hidden control flow | partly (defer, panics, goroutines) | kept as a slogan | kept | mostly (optionals) |

What C took from others: BCPL and B (typeless word machine, `{}` blocks via B, `++`/`--`),
ALGOL 68 (structs, the type system idea), PL/I and BCPL (file inclusion), Fortran (nothing
syntactic, but the "compile to efficient code or die" attitude; see `fortran.md`).

## Lessons for Sisu
- **Adopt**:
  - *The C ABI as the FFI boundary, with a first-class, cheap `extern "C"` story in both
    directions.* Nothing else will be accepted; all four modern C alternatives chose this.
    Sisu's exported functions and `repr(C)` structs should need no wrapper generation.
  - *No hidden control flow or allocation in the core language.* Make allocation explicit
    (allocator parameters or a context, as in Zig/Odin), make calls look like calls, and make
    the cost model readable from the source. This is the property embedded and kernel users
    will refuse to give up.
  - *Separate compilation with stable object-file and linker semantics* so Sisu libraries can
    be distributed as binaries and linked from C.
  - *Low-level escape hatches* (unions with defined punning rules, explicit layout, raw
    pointers, inline asm) inside a clearly marked unsafe subset. C's success in device drivers
    depends on them.
  - *Stability discipline*: a 10-year-old program should still compile. Deprecate loudly,
    remove rarely.
- **Adapt**:
  - *Undefined behaviour must become a budget, not a default.* Keep a small, enumerated list of
    UB in the unsafe subset only (like the C++29 UB annex effort, but short), make the safe
    subset UB-free, and give the programmer explicit permissions (`[[unsequenced]]`-style
    purity, `restrict`-style non-aliasing, wrapping vs. checked vs. saturating arithmetic
    operators) rather than letting the optimiser infer licence from the absence of a trap.
    Regehr's finding that no single "friendly" dialect gets consensus is itself the lesson:
    *the programmer, not the dialect, chooses per operation.*
  - *Fat pointers / slices as the only array-parameter type*, with NUL-terminated views as an
    explicit conversion for C interop. `-fbounds-safety` shows even C is converging on this.
  - *The preprocessor's jobs need real features*: conditional compilation via compile-time
    `if`, constants via `const`, generics via parametric types, embedding via `#embed`-like
    intrinsics, configuration via build-system-provided compile-time values.
  - *Integer rules*: no implicit narrowing or sign-changing conversions, fixed-width types as
    the default spelling, explicit overflow semantics per operation.
- **Avoid**:
  - *Textual include and macro-based modules.* The 2000x expansion measurement and the
    universal switch to packages in successors settles this.
  - *Untagged unions and in-band error values* (`-1`, `NULL`, `errno`). Provide sum types and a
    result/error type from day one.
  - *"Declaration reflects use" syntax.* No successor kept it.
  - *Implementation-defined integer widths for the common types.* `int` being 16/32/64 bits was
    portability in 1978 and a bug source since.
  - *Annex K style "safe" library variants bolted on later.* The fix has to be in the types,
    not in a parallel API.
- **Open questions raised**:
  - *How much of C's performance actually comes from UB-driven optimisation?* Regehr's group
    and the Friendly C debate never produced a number; the Linux kernel runs fine with three
    major UB optimisations disabled. Sisu should measure this on its own backend before
    deciding how much UB to keep in the unsafe subset.
  - *Can a new language be "C-ABI-shaped" at its boundary without becoming C-shaped inside?*
    Rust's answer (an `extern "C"` subset plus `bindgen`) is workable but the friction is a
    known adoption cost. Swift's and Zig's direct C-header importing is the stronger model but
    ties the compiler to a C front end (Zig literally ships Clang). Which to choose depends on
    whether Sisu is willing to bundle a C parser.
  - *Hardware memory safety (CHERI) vs. language memory safety*: if capability hardware becomes
    mainstream, a language with raw pointers becomes much safer "for free", and the
    cost-benefit of a borrow checker shifts. It is too early to bet on this (Morello is still
    research; CHERIoT is embedded-only), but Sisu's pointer model should not be incompatible
    with 128-bit capabilities (pointer-integer round-tripping must be an explicit, unsafe
    operation).

## Sources
1. D. M. Ritchie, "The Development of the C Language", HOPL-II, 1993.
   https://www.nokia.com/bell-labs/about/dennis-m-ritchie/chist.html (not retrievable during
   this research; quotes from memory, flagged above)
2. ISO/IEC 9899:2024 (C23) feature summary and publication date:
   https://en.wikipedia.org/wiki/C23_(C_standard_revision) (2024-2026)
3. C2Y accepted features and draft history:
   https://en.wikipedia.org/wiki/C2Y_(C_standard_revision) (2026)
4. Clang C status page:
   https://llvm.googlesource.com/llvm-project/clang/+/refs/heads/main/www/c_status.html (2026)
5. P. Cuoq, M. Flatt, J. Regehr, "Proposal for a Friendly Dialect of C", Aug 2014.
   https://blog.regehr.org/archives/1180
6. J. Regehr, "The Problem with Friendly C", Dec 2015. https://blog.regehr.org/archives/1287
7. X. Wang, N. Zeldovich, M. F. Kaashoek, A. Solar-Lezama, "Towards Optimization-Safe Systems:
   Analyzing the Impact of Undefined Behavior", SOSP 2013.
   https://pdos.csail.mit.edu/papers/stack:sosp13.pdf
8. CVE-2009-1897 (Linux `tun_chr_poll`): https://www.opencve.io/cve/CVE-2009-1897
9. Linux kernel top-level `Makefile` (UB-related flags):
   https://raw.githubusercontent.com/torvalds/linux/master/Makefile (fetched Oct 2026)
10. Clang `-fbounds-safety` design document: https://clang.llvm.org/docs/BoundsSafety.html
    (2023-2026); EuroLLVM 2025 tutorial slides:
    https://www.llvm.org/devmtg/2025-04/slides/tutorial/olsonn-stefaniski_fbounds.pdf
11. MSRC, "Security analysis of CHERI ISA", Oct 2020.
    https://www.microsoft.com/en-us/msrc/blog/2020/10/security-analysis-of-cheri-isa/
12. CHERIoT 1.0 specification release, Nov 2025:
    https://cheriot.org/sail/specification/release/2025/11/03/cheriot-1.0.html ; CHERI Alliance
    updates and CHERI Blossoms 2026: https://cheri-alliance.org/?p=677 ; LWN on CHERI status:
    https://lwn.net/Articles/1044229/
13. M. Miller (MSRC), BlueHat IL 2019 memory-safety figure, as reported:
https://www.zdnet.com/article/microsoft-70-percent-of-all-security-bugs-are-memory-safety-issues/
14. Chromium Security, "Memory safety":
    https://www.chromium.org/Home/chromium-security/memory-safety/ (2020, still current)
15. Google, "Rust in Android: move fast and fix things", Nov 2025.
    https://blog.google/security/rust-in-android-move-fast-fix-things/
16. ONCD, "Back to the Building Blocks: A Path Toward Secure and Measurable Software", 26 Feb
    2024.
https://bidenwhitehouse.archives.gov/oncd/briefing-room/2024/02/26/press-release-technical-report
    ; CISA/FBI roadmap deadline coverage:
    https://www.techrepublic.com/article/cisa-fbi-memory-safety-recommendations/
17. R. Pike, "Go at Google: Language Design in the Service of Software Engineering", SPLASH
    2012. https://go.dev/talks/2012/splash.article
18. Zig, "Why Zig When There is Already C++, D, and Rust?":
    https://ziglang.org/learn/why_zig_rust_d_cpp/
19. Odin language homepage: https://odin-lang.org/ ; C3 language homepage: https://c3-lang.org/
20. Stack Overflow Developer Survey 2025, technology section:
    https://survey.stackoverflow.co/2025/technology
21. TIOBE Index, September 2026: https://www.tiobe.com/tiobe-index/
22. ANSI C Rationale (1989) principles ("Trust the programmer" etc.): cited from memory of the
    X3J11 Rationale, section 1 "Scope"; verify against https://www.lysator.liu.se/c/rat/a.html
    before citing in the proposal.


