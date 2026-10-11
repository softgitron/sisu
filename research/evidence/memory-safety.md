# Memory safety dossier

Compiled October 2026. Memory safety is the largest single fork in Sisu's design: it decides the runtime model (GC,
reference counting, ownership), the learning curve, the FFI story and the regulatory posture. This dossier collects
the industry data, the government and regulatory pressure, the taxonomy of approaches with measured costs, and the
adjacent class of data races. Figures were re-read from primary sources except where marked `[secondary]` or
`[unverified]`.

---

## 1. Industry data: how much of the damage is memory unsafety?

| Organisation | Source and year | Population | Memory-safety share | Dominant sub-class |
|---|---|---|---|---|
| Microsoft | M. Miller, BlueHat IL, Feb 2019; MSRC blog (G. Thomas), 16 Jul 2019 | CVEs Microsoft assigned, 2006-2018 | "~70 % ... each year" | not broken out in the blog; Miller's slides show stack corruption falling and use-after-free, heap corruption, type confusion and uninitialised use rising `[unverified]` |
| Google Chromium | chromium.org memory-safety page, 2020 | 912 high/critical security bugs since 2015 (stable channel) | "around 70 %" | "half of those are use-after-free" |
| Mozilla Firefox | D. Hosfelt, Mozilla Hacks, 28 Feb 2019 | 69 security bugs in the CSS style component since 2002 | 51 (73.9 %) "would have been prevented" by Rust; 32 of 34 critical/high bugs were memory-related | memory, bounds, null/uninitialised, integer overflow; the one exception was a failed-allocation crash |
| Google Project Zero | M. Stone, 19 Apr 2022 | 58 in-the-wild 0-days exploited in 2021 | 39 (67 %) memory corruption | use-after-free 17, OOB read/write 6, buffer overflow 4, integer overflow 4 |
| Android | Google security blog, 6 Apr 2021 | Android high-severity vulnerabilities | "~70 %" | ~50 % of memory bugs are in code less than a year old |
| Android 13 | 1 Dec 2022 | Android security bulletin, critical/high | 223 memory-safety vulns = 76 % (2019) -> 85 = 35-36 % (2022); 86 % of critical and 89 % of remotely exploitable vulns in 2022; 78 % of confirmed in-the-wild exploited | - |
| Android | 25 Sep 2024 | same | 76 % (2019) -> 24 % (2024, 36 extrapolated for the year vs ~70 % "industry norm") | - |
| Android | 13 Nov 2025 | same, ~5 MLOC of Rust in the platform | below 20 % in 2025 for the first time | one potential Rust memory-safety bug (CVE-2025-48530, a linear overflow in CrabbyAVIF, fixed pre-release) |
| Apple | Security Engineering blog, 9 Sep 2025 | mercenary spyware chains | no percentage; "they exploit memory safety vulnerabilities" | MIE (EMTE + typed allocators) shipped on iPhone 17 at "minimal performance impact" |

Three regularities matter for design. **The 70 % number is stable across vendors and a decade** (Microsoft 2006-2018,
Chromium 2015-2020, Android 2019), and the Android series shows it is not a law of nature: it falls when new code
stops being written in C/C++. **Temporal bugs dominate spatial bugs in modern, hardened code**: Chromium's memory bugs
are half use-after-free, and 17 of Project Zero's 39 memory-corruption 0-days were UAF against 10 bounds/overflow
bugs; decades of canaries, ASLR and checked containers have shifted the residual class toward lifetime errors, which
GC, careful RC, ownership or memory tagging address and bounds checks alone do not. **Vulnerabilities concentrate in
new code**: ~50 % of Android memory bugs are under a year old (2021); "vulnerabilities decay exponentially. They have
a half-life", with five-year-old code 3.4x (literature) to 7.4x (Android/Chromium lifetimes) less dense than new code
(2024). This is why writing *new* code in a memory-safe language produced the 76 % -> <20 % decline even though "the
majority of code [is] still unsafe".

### Measured outcomes of switching (Android, 2022-2025)

- 2022: ~21 % of new native code in Rust; ~1.5 MLOC of Rust in AOSP; "zero memory safety vulnerabilities discovered in
  Android's Rust code".
- 2024: "the rollback rate of Rust changes is less than half that of C++"; Chromium's Rust QR-code generator became
  "20 times faster" after its sandbox could be removed.
- 2025: ~5 MLOC Rust; estimated 0.2 memory-safety vulnerabilities per MLOC vs ~1,000 per MLOC for Android's C/C++, a
  "1000x reduction"; rollback rate "~4x lower than C++" for medium and large changes; "about 25 % less time in code
  review"; ~20 % fewer revisions per change; ~4 % of Rust lines inside `unsafe {}`; Rust net code additions surpassed
  C++ in the first three quarters of 2025; "the safer path is now also the faster one".
- Caveats: single vendor, internal metrics, not peer reviewed; Rust code is newer and may be written by more senior
  engineers; review-time and rollback figures are correlational.

---

## 2. Government and regulatory pressure (2022-2026)

| Instrument | Date | What it says |
|---|---|---|
| NSA Cybersecurity Information Sheet, "Software Memory Safety" | Nov 2022 (v1.1 Apr 2023) | Recommends memory-safe languages (lists C#, Go, Java, Ruby, Rust, Swift) and hardening for C/C++ `[list not re-verified; PDF blocked]` |
| CISA, NSA, FBI, ASD ACSC, CCCS, NCSC-UK, NCSC-NZ, CERT-NZ, "The Case for Memory Safe Roadmaps" | 6 Dec 2023 | "Memory safety vulnerabilities are the most prevalent type of disclosed software vulnerability"; cites Microsoft ~70 % (2006-2018 CVEs), Chromium ~70 %, Project Zero 67 % of 2021 0-days; asks manufacturers to publish memory-safe roadmaps; appendix of memory-safe languages: C#, Go, Java, Python, Rust, Swift |
| US ONCD, "Back to the Building Blocks: A Path Toward Secure and Measurable Software" | Feb 2024 | "Using memory safe programming languages can eliminate most memory safety errors"; "the most efficient" lever; names space systems as an edge case where hardware (CHERI) and formal methods are complementary; frames spatial vs temporal safety; calls for better cybersecurity metrics |
| CISA/FBI, "Product Security Bad Practices" | v1.0 Oct 2024; v2.0 17 Jan 2025 | Item 1: developing new product lines for critical infrastructure "in a memory-unsafe language (e.g., C or C++)" is a bad practice; existing products should publish a memory-safety roadmap (v1.0: by 1 Jan 2026; the v2.0 page phrases it as by the end of 2025), exempting products with end-of-support before 1 Jan 2030 |
| CISA/NSA, "Memory Safe Languages: Reducing Vulnerabilities in Modern Software Development" | 24 Jun 2025 | Joint guide on obstacles to MSL adoption and practical solutions: language-level protections, library support, tooling, developer training |
| EU Cyber Resilience Act, Regulation (EU) 2024/2847 | in force 10 Dec 2024; reporting of actively exploited vulnerabilities from 11 Sep 2026; full application 11 Dec 2027 | Mandatory secure-by-design requirements across design, development and maintenance, vulnerability handling over the support period, CE marking, third-party assessment for higher-risk products. It does not name languages, but a manufacturer must justify its vulnerability-handling posture, and the US guidance is the obvious benchmark |

Implication: a systems language launched after 2026 that is not memory safe by default will be excluded from the
"memory safe language" category that procurement guidance now references, regardless of its other merits. Conversely,
"memory safe" is defined operationally (CISA's list includes GC'd Python and Java alongside Rust), so the choice of
*mechanism* is open; only the default matters.

---

## 3. Taxonomy of approaches and their measured costs

Evan Ovadia's catalogue ("Borrow checking, RC, GC, and the Eleven (!) Other Memory Safety Approaches", Vale blog; 17
entries) is the most complete practitioner taxonomy. His summary: garbage collection is "the easiest" and "depending
on one's definitions, the safest too"; reference counting is simple and low-memory but slow and leaks cycles; borrow
checking is "as fast as C" at the price of complexity and of patterns it cannot express (observers, intrusive data
structures, many RAII patterns). The academic measurements:

### Tracing garbage collection

- **Hertz and Berger, "Quantifying the Performance of Garbage Collection vs. Explicit Memory Management", OOPSLA
  2005.** Oracular simulation of Java benchmarks: a generational collector matches explicit management given **5x**
  the memory; with **3x** it runs 17 % slower on average; with **2x** nearly 70 % slower; under paging, an order of
  magnitude slower.
- **Cai, Blackburn, Bond, Maas, "Distilling the Real Cost of Production Garbage Collectors", ISPASS 2022.**
  Lower-bound overhead methodology on OpenJDK 17 with modern benchmarks: at a 2.4x heap, production collectors spend
  "at least 7-82 % more wall-clock time and 6-92 % more CPU cycles" than the intrinsic cost; concurrent collectors
  (Shenandoah, ZGC) cost up to 75-78 % more than G1. The overhead is larger than the community believed. Walter
  Bright's 2019 slides give the practitioner's version: GC "uses 3x the memory of manual management" and is bad for
  interactive programs because of pauses. Pony (per-actor GC) and Verona (regions) avoid global pauses by restricting
  sharing.

### Reference counting

- **Choi, Shull, Torrellas, "Biased Reference Counting", PACT 2018.** Swift's non-deferred RC makes client programs
  "spend 32 % of their execution time performing RC operations", largely because counts must be atomic; biased RC cut
  client execution time by 22.5 % and raised server throughput 7.3 %.
- **Reinking, Xie, de Moura, Leijen, "Perceus: Garbage Free Reference Counting with Reuse", PLDI 2021
  (MSR-TR-2020-42).** Precise, non-atomic RC with compile-time reuse analysis in Koka: competitive with OCaml,
  Haskell, Swift and Java on allocation-heavy benchmarks; on red-black tree insertion "within 10 %" of
  in-place-updating C++ `std::map`. Requires ownership-like static reasoning about uniqueness and forbids cycles.
  Cycle leaks remain RC's structural weakness (Python and Nim pair RC with a tracing backup; Swift relies on `weak`).

### Ownership and borrowing (Rust, Austral)

Zero runtime cost; memory and data-race safety proven for the safe subset (RustBelt). Costs are compile-time and
cognitive: the borrow checker is a "big problem" for 10.6 % of Rust users, async and generics are harder (see
`surveys.md`); Google's top challenges were macros, ownership/borrowing and async. `unsafe` is needed for intrusive
structures, FFI and some patterns: ~4 % of Android's Rust lines (2025); <30 % of crates use it directly but >50 %
depend on it transitively (Evans et al. 2020). Compile times are the top complaint of Rust's users.

### Linear types, mutable value semantics, regions, generational references

- **Linear types** (Austral, Vale's linear-aliasing model): every value used exactly once; enable "higher RAII"
  (compiler-checked resource protocols) at the cost of explicit threading of values.
- **Mutable value semantics** (Swift's value types; Hylo/Val): no first-class references; parts are projected via
  `inout`/subscripts. Racordon, Shabalin, Zheng, Abrahams, Saeta, "Implementation Strategies for Mutable Value
  Semantics", JOT 2022, show it compiles to in-place mutation without a borrow checker for the common cases; graphs
  and shared mutable structures are the expressiveness limit, for which Hylo adds a borrow checker.
- **Regions / arenas** (Cyclone, Verona, Forty2, Pony's `iso`): objects in a region may point to each other freely;
  immutable region views remove checks; "surprisingly flexible and powerful" (Ovadia); no published costs for a
  mainstream language.
- **Generational references** (Vale): a generation number per allocation and a check per dereference turns
  use-after-free into a deterministic fault; one compare per access, removable inside regions; no independent
  benchmarks `[unverified]`.

### Hardware and dynamic approaches

- **CHERI / Morello.** Watson et al., "Early performance results from the prototype Morello microarchitecture",
  UCAM-CL-TR-986, Sep 2023: pure-capability (memory-safe) code on the FPGA prototype showed a 14.97 % geometric-mean
  overhead on SPECint 2006 (train); with two microarchitecture fixes 7.40 % and 5.70 %; the authors' best estimate for
  a tuned production design is **1.8-3.0 %**, the essential cost being 128-bit pointers. Temporal safety needs extra
  mechanisms (Cornucopia, CHERIoT).
- **Memory tagging (ARM MTE / Apple EMTE).** Probabilistic with 4-bit tags in ARM MTE; Apple's MIE runs EMTE
  synchronously with typed allocators and claims "minimal performance impact" without numbers.
- **Sanitizers.** Serebryany et al., "AddressSanitizer", USENIX ATC 2012: average slowdown 73 %, 3.4x memory; a
  testing tool, not a deployment mitigation.
- **C++ profiles / hardened libraries.** Sutter, P3081R0 (Oct 2024) proposes enforced `type_safety`, `bounds_safety`,
  `initialization_safety`, `lifetime_safety` and `arithmetic_safety` profiles; Chromium's hardening of `//base` and
  bounds checks in std are the shipped version. Lifetime safety without annotations remains unsolved in C++.

### Summary table (costs as measured or claimed above)

| Approach | Runtime CPU cost | Memory cost | Temporal safety | Spatial safety | Data-race safety | Main burden |
|---|---|---|---|---|---|---|
| Tracing GC | 7-82 % (ISPASS 2022, 2.4x heap) | 2-5x (Hertz 2005) | yes | yes (with bounds checks) | no | pauses, memory, no deterministic destruction |
| Atomic RC (Swift) | up to ~32 % of time in RC ops (PACT 2018) | low | yes (cycles leak) | yes | no | cycles, `weak`, atomic ops |
| Precise RC + reuse (Perceus/Koka) | within 10 % of C++ on benchmarks | low | yes (no cycles) | yes | partial (purity) | functional style, no cycles |
| Ownership + borrowing (Rust) | ~0 | ~0 | yes (safe subset) | yes | yes (safe subset) | learning curve, compile time, `unsafe` escape |
| Mutable value semantics (Hylo) | ~0 claimed | ~0 | yes | yes | yes (no shared mutation) | expressiveness for graphs |
| Generational refs (Vale) | one compare per deref `[claimed]` | small | yes (fail-fast) | yes | no | unproven at scale |
| CHERI | 1.8-3 % (estimate), 15 % (prototype) | 2x pointer size | partial (needs revocation) | yes | no | hardware availability |
| Sanitizers | 73 % | 3.4x | detects | detects | separate tool | testing only |

---

## 4. Temporal, spatial and type safety: which classes dominate

- **Spatial** (out-of-bounds, buffer overflow): prevented by bounds-checked slices and fat pointers; Walter Bright
  calls C's conflation of arrays and pointers "C's biggest mistake" (2009) and D's slices the fix; bounds-check cost
  is small and often optimised away.
- **Temporal** (use-after-free, double free, dangling stack references): the dominant residual class in hardened C++
  (half of Chromium's memory bugs; 17 of 39 at Project Zero 2021). Prevented by GC, RC, ownership, generational
  references, or hardware revocation.
- **Type confusion**: a rising class in Project Zero's and Microsoft's data; prevented by sound static typing plus
  checked downcasts; Chromium's MiraclePtr and Apple's typed allocators are the C++ mitigations.
- **Uninitialised memory**: 3-5 % of Android vulnerabilities historically (2021); trivially prevented by
  definite-initialisation rules.
- **Integer overflow** feeding spatial bugs: 4 of 39 Project Zero 2021 memory-corruption 0-days; Rust checks in debug
  and wraps in release; Sutter's `arithmetic_safety` profile targets it.

---

## 5. Data races: the adjacent class

A data race on a pointer or length is a memory-safety bug, so the two classes are entangled. Rust: Qin et al. (PLDI
2020) found every non-blocking thread-safety bug in their corpus involved `unsafe` or interior-unsafe code, i.e. the
safe subset held; but all blocking bugs (deadlocks, channel misuse) were in safe code. Go: Tu et al. (ASPLOS 2019): 58
% of blocking bugs came from message passing; Chabbi and Ramanathan (PLDI 2022): ~2,000 data races in Uber's 46 MLOC
monorepo in six months with a dynamic detector. Swift 6 (2024) added compile-time data-race safety via actor isolation
and `Sendable`; the 2025 vision document concedes it produced false positives in single-threaded code, demanded too
much annotation, and confronted beginners "with the concept of data-race safety and actor isolation too early". Pony's
reference capabilities give data-race freedom with per-actor GC; Verona and Hylo pursue the same via regions and value
semantics. Lesson: memory safety without data-race freedom (Go, Java, Swift before 6) still ships thousands of races
at scale; guaranteeing both (Rust, Pony) costs annotation and learning curve and still leaves deadlocks.

---

## 6. Counter-evidence and failure modes to record

- **Cloudflare, 18 November 2025.** Cloudflare's worst outage since 2019 (main impact 11:20-14:30 UTC, fully resolved
  17:06). A database-permission change doubled a Bot Management feature file past a hard-coded limit of 200 features;
  the Rust proxy's check returned an `Err`, the code called `Result::unwrap()`, and the panic ("thread
  fl2_worker_thread panicked: called Result::unwrap() on an Err value") took down core traffic. Memory safety was
  never in question; value-based error handling moves the risk to explicit "this cannot happen" assertions, which need
  the same review discipline as `unsafe`. Remediation: validate internal configuration like user input, more kill
  switches, review failure modes of every core module.
- **Mozilla 2019 and Android 2025.** Rust "is not foolproof": a fix dropped during the Stylo rewrite produced a
  history-leak logic bug (bug 1420001) while the relevant tests were disabled; Android's first potential Rust
  memory-safety vulnerability (CVE-2025-48530) was a linear overflow in an `unsafe` region of an image decoder, caught
  before release. The `unsafe` surface is where the risk concentrates.
- **GC is not free either.** ISPASS 2022's 7-82 % overhead and Hertz and Berger's 2-5x memory are far from the "GC is
  basically free now" folklore; Discord's 2020 move of a hot service from Go to Rust and Cloudflare's Pingora (Rust
  replacing NGINX/C, 2022) were justified by tail latency and memory, not safety `[secondary]`.

---

## Synthesis for Sisu

1. **Memory safety by default is non-negotiable for a language launched after 2026** (high confidence): ~70 % of
   severe vulnerabilities across four vendors and a decade; seven years of Android data showing the share falls below
   20 % when new code is safe; US agency guidance lists memory-unsafe new products as a "bad practice" and the EU CRA
   makes vulnerability handling a legal obligation from 2026-2027.
2. **The mechanism is an open choice; regulators define "memory safe" operationally** (high): CISA's list spans GC'd
   Python and Java to Rust. Sisu may choose GC, RC, ownership or a hybrid on technical merit.
3. **Temporal safety is the class that matters most now** (high): half of Chromium's and 17 of 39 of Project Zero's
   memory bugs are use-after-free; bounds checking alone would be an incomplete answer; lifetimes must be decided
   before anything else.
4. **Every mechanism has a measured, non-trivial cost; "zero-cost" applies only to ownership and only at run time**
   (high): GC 7-82 % CPU and 2-5x memory, atomic RC up to a third of run time, borrow checking paid in learning curve
   and build time, CHERI 2-15 % plus hardware.
5. **Data-race freedom should be designed in with memory safety, not added later** (medium-high): Uber's 2,000 races
   in GC'd Go and Swift 6's painful retrofit show both the need and the cost; deadlocks remain the user's problem
   under every known design.
6. **Keep an explicit, auditable escape hatch and treat "cannot fail" assertions like `unsafe`** (medium): ~4 % unsafe
   lines in Android's Rust is tolerable; Cloudflare's `unwrap()` shows the next outage comes from the explicit
   assertion, not the memory model.
7. **If performance is a goal, the realistic design space is ownership, precise non-atomic RC with reuse (Perceus), or
   mutable value semantics, possibly with regions** (medium): these are the only approaches measured within ~10 % of
   C/C++; tracing GC's measured overheads make it a poor fit for the systems niche the surveys say Rust and Go are
   winning, though it remains "the easiest" for everything else.
8. **Productivity parity with GC'd languages is plausible but not proven** (medium-low): Google's "same team size",
   "2x less effort", 25 % faster review and 4x fewer rollbacks are internal and unreviewed; they are the best data
   that exist and they all point the same way.

## Sources

1. MSRC, "A proactive approach to more secure code", 16 Jul 2019 (M. Miller, BlueHat IL 2019); Chromium, "Memory
   safety" (chromium.org, 2020); D. Hosfelt, Mozilla Hacks, 28 Feb 2019; M. Stone, Project Zero, 19 Apr 2022.
2. Google security blog: "Rust in the Android platform" (6 Apr 2021); "Memory Safe Languages in Android 13" (1 Dec
   2022); "Eliminating Memory Safety Vulnerabilities at the Source" (25 Sep 2024); "Rust in Android: move fast and fix
   things" (13 Nov 2025). Apple SEAR, "Memory Integrity Enforcement", 9 Sep 2025. Cloudflare, "Cloudflare outage on
   November 18, 2025".
3. CISA et al., "The Case for Memory Safe Roadmaps", 6 Dec 2023; CISA, "Product Security Bad Practices" v2.0, 17 Jan
   2025; CISA/NSA, "Memory Safe Languages: Reducing Vulnerabilities in Modern Software Development", 24 Jun 2025;
   ONCD, "Back to the Building Blocks", Feb 2024; European Commission CRA page; Regulation (EU) 2024/2847.
4. E. Ovadia, verdagon.dev/grimoire/grimoire; M. Hertz, E. Berger, OOPSLA 2005; Z. Cai et al., ISPASS 2022; J. Choi et
   al., PACT 2018; A. Reinking et al., PLDI 2021 (MSR-TR-2020-42); D. Racordon et al., JOT 2022.
5. R. Watson et al., UCAM-CL-TR-986, Sep 2023; K. Serebryany et al., USENIX ATC 2012; H. Sutter, P3081R0, Oct 2024; W.
   Bright, "C's Biggest Mistake" (2009) and "D at 20" slides (2019).
6. B. Qin et al., PLDI 2020; A. N. Evans et al., ICSE 2020; T. Tu et al., ASPLOS 2019; M. Chabbi, M. K. Ramanathan,
   PLDI 2022; Swift Evolution vision "Improving the approachability of data-race safety", 2025.
