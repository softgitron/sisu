# Ada and SPARK

## Snapshot
| | |
|---|---|
| First appeared | Ada: 1980 (ANSI/MIL-STD 1815), Ada 83 (MIL-STD 1815A, 1983; ISO/IEC 8652:1987). SPARK: 1987-88 (Bernard Carré, University of Southampton / Program Validation Ltd); SPARK 2014 (AdaCore and Altran) rebased the language on Ada 2012 contracts. |
| Designed by | Ada: Jean Ichbiah's "Green" team (CII Honeywell Bull) to the US DoD's Steelman requirements (1978); Ada 95 led by Tucker Taft (Intermetrics); later revisions by the ISO Ada Rapporteur Group (ARG). SPARK: Carré, Trevor Jennings; since 2010 AdaCore (Yannick Moy et al.) with Altran/Capgemini. |
| Current version (Oct 2026) | Ada 2022 (ISO/IEC 8652:2023, approved 2 May 2023). SPARK: "SPARK 2014" language, defined by the SPARK Reference Manual that AdaCore revises with each yearly GNAT Pro / SPARK Pro release (2026 release numbers not verified in this research). Free toolchain: GNAT FSF in GCC 16, GNATprove, Alire package manager. |
| Governance | ISO/IEC JTC1/SC22/WG9 with the ARG doing technical work; in practice AdaCore (GNAT) is the only actively developed general implementation and defines SPARK unilaterally. |
| Typing | Static, strong, nominal; range and derived subtypes; no implicit numeric conversions; enumerations with representation clauses; generics with explicit instantiation; tagged types (OOP) since Ada 95; contracts (`Pre`, `Post`, `Type_Invariant`, predicates) since Ada 2012 |
| Memory | Manual via access types and storage pools, scope-bound stack objects, controlled types (finalisation, RAII-like); runtime range/index/overflow checks on by default. SPARK: no aliasing; ownership-based pointer rules (preview 2019, now standard) borrowed from Rust. |
| Compilation | AOT; GNAT is a GCC front end; strict separate compilation with package specifications and bodies checked for consistency by the compiler |
| Primary domains | Avionics (DO-178B/C), rail signalling, space, defence, air-traffic control, security-critical firmware, medical |
| Notable users | Boeing 777 primary flight controls, Eurofighter Typhoon and Saab Gripen fly-by-wire, F-22, Ariane 4/5, Canadian Automated Air Traffic System, UK NATS iFACTS (SPARK), TGV TVM in-cab signalling, metro systems in Paris, London, Hong Kong and New York; NVIDIA root-of-trust firmware (SPARK). Stack Overflow 2025: 1.4% of respondents; TIOBE Sep 2026: #17 at 0.85%, noted as showing "renewed strength". |

## Stated design goals
The DoD's High Order Language Working Group (formed 1975) published the Steelman requirements
(1978) to replace the hundreds of languages in use in embedded defence systems with one.
Steelman's priorities, in its own ordering, were reliability and maintainability over
efficiency, and readability over writability: programs are read far more often than written, so
the language should be explicit, unambiguous and free of implicit conversions. Four contractors
competed (Red, Green, Blue, Yellow); Green, Ichbiah's design, was chosen in May 1979.

SPARK's goal is narrower and sharper: a subset of Ada with unambiguous semantics, no aliasing
and no erroneous execution, so that a tool (GNATprove today) can *prove* properties of
programs, first and foremost the absence of runtime errors (AoRTE: no overflow, no
out-of-range, no out-of-bounds, no null dereference, no division by zero), then functional
contracts. AdaCore's adoption guidance defines assurance levels (Stone: valid SPARK; Bronze:
initialisation and data-flow; Silver: AoRTE; Gold: key integrity properties; Platinum: full
functional correctness) so that a project can choose how much it buys.

## Strengths

### Range subtypes and derived types make units and bounds part of the type
`type Celsius is digits 6 range -273.15 .. 1.0e6;` and `subtype Index is Integer range 1 ..
100;` are first-class; arrays can be indexed by any discrete type, and `A'First`, `A'Last`,
`A'Range` make off-by-one loops rare. Derived types (`type Metres is new Float;`) are distinct
even though they have the same representation, so mixing metres and feet is a compile error.
Mixed-kind arithmetic without an explicit conversion is a compile error. Evidence that it
works: every modern safety-oriented design re-invents a piece of it (Rust newtypes, refinement
types in Liquid Haskell and F*, Nim's subrange types, Pascal/Modula lineage); and the runtime
checks these types imply are precisely what SPARK then proves away.

### Contracts in the language since 2012
Ada 2012 added `Pre`, `Post`, `Type_Invariant`, `Static_Predicate` and `Dynamic_Predicate` as
aspects in declarations, executable as runtime checks (`-gnata`) and used as proof obligations
by SPARK. Wikipedia's summary is apt: contracts "became part of the language, which helped make
SPARK possible", because SPARK 2014 replaced its 1990s annotation comments with the standard
aspects and the two communities merged. C++ only adopted contracts in C++26, fourteen years
later and "despite disquiet over their value" (see `cpp.md`); Eiffel (1986) is the origin, Ada
is the first mainstream systems language to ship them.

### SPARK: proof of absence of runtime errors, in production
GNATprove discharges verification conditions with SMT solvers (Alt-Ergo, CVC5, Z3) and reports
which checks are proven. Production evidence, in order of strength:
- **NVIDIA** (AdaCore case study, vendor-published): NVIDIA moved security-critical firmware,
  including root-of-trust components, from C/C++ to SPARK, citing the cost of fixing fielded
  firmware. SPARK code "is linkable with C", so adoption was incremental. Principal engineer
  Cameron Buschardt reported that "the assembly generated from SPARK was almost identical to
  that from the C code". NVIDIA uses it to prove AoRTE on its most critical components and
  conformance to specification for root-of-trust applications. (No independent verification of
  the outcomes was found.)
- **NATS iFACTS** (UK en-route air traffic control, 2011): written in SPARK with proof of AoRTE
  (per Wikipedia's list of Ada systems).
- **Rail**: TGV TVM signalling and several metros (Wikipedia; project-level evidence not
  re-verified).
- **Tokeneer** (NSA-commissioned demonstrator, 2008; from memory): a biometric access system
  developed with SPARK "correctness by construction" and released as a public case study.
DevClass's 2022 headline, "SPARK as good as Rust for safer coding, AdaCore cites NVIDIA case
study", shows how AdaCore positions it: equivalent memory safety plus provable functional
properties, without a borrow checker on every line.

### Packages: specification/body separation enforced by the compiler
Ada's `package` (1983) is a true module: a specification (`.ads`) with public and `private`
parts, and a body (`.adb`), both compiled, with the compiler checking that every unit `with`ing
the spec is consistent at link time ("no header drift", unlike C). Private types give
abstraction without inheritance. C++ namespaces and Java packages are weaker; Ada's are closest
to ML signatures. Fortran 90 modules and Modula-2/Oberon modules share the lineage.

### Parameter modes and named association
`procedure Move (From : in Position; To : in out Position; Steps : out Natural)` declares data
flow in the signature, and calls can say `Move (From => A, To => B, Steps => N)`. Fortran's
`intent(in/out/inout)` (1990), Swift's argument labels and `inout`, Rust's explicit `&mut` and
Kotlin/Python named arguments are the same idea. SPARK's flow analysis (initialisation and
information flow) is built directly on modes plus `Global` and `Depends` aspects.

### Concurrency in the language, with certifiable profiles
Ada 83 had tasks and rendezvous; Ada 95 added protected objects (monitors with entries and
barriers); Ada 2005 standardised the **Ravenscar profile** (originally 1997): static tasks,
fixed priorities, library-level protected objects with at most one entry and simple Boolean
barriers, no dynamic task creation, so that schedulability analysis and certification (DO-178C,
EN 50128) are tractable. SPARK supports Ravenscar and proves absence of data races and
deadlocks structurally. Ada 2022 added **Jorvik** (AdaCore's 2016 `GNAT_Extended_Ravenscar`,
standardised): multiple entries per protected object, queued callers, relative delays,
`Pure_Barriers`, CPU affinity for protected objects, keeping "the same fundamental execution
model (static tasks, fixed priorities, and library-level protected objects)" for "hard
real-time/embedded systems that may require schedulability analysis but not the most stringent
analyses". This is the only widely deployed example of a language-level *restriction profile*
(`pragma Profile (Ravenscar)` / `pragma Restrictions (...)`) that actually works, which is
worth contrasting with C++'s six-year failure to agree on "profiles": Ada's restrictions are
syntactic and semantic subsets a compiler can check directly. Ada 2022 also adds `parallel`
blocks and loops and the `'Reduce` attribute (compiler support lagging; AdaCore documents Ada
2022 features behind `-gnat22`).

### Runtime checks on by default, provable off
Index, range, overflow and null checks are on unless the programmer disables them (`pragma
Suppress`, `-gnatp`). This is the inverse of C and is why Ada code fails loudly (the Ariane 5
flight 501 failure was an *unhandled* operand-error exception from a float-to-integer
conversion in reused Ariane 4 code, i.e. the check fired and the system did the wrong thing
with it, not a silent corruption; from memory of the 1996 Lions inquiry report). SPARK's AoRTE
is then the mechanism to remove checks with evidence rather than with a flag.

## Weaknesses

### Verbosity and ceremony
`procedure P (X : in Integer) is begin ... end P;`, duplicated signatures in spec and body,
`with Ada.Text_IO; use Ada.Text_IO;`, explicit `Float (I)` conversions, declarations only in
declarative regions, `package Int_Vectors is new Ada.Containers.Vectors (Natural, Integer);`
before any vector can be used. The admissions are in Ada 2022's feature list, nearly all of
which are ergonomic repairs: `'Image` for every type (previously scalar only), the `@`
target-name symbol (`X := @ + 1;`), declare expressions, delta aggregates (`(Rec with delta F
=> 1)`), container aggregates with `[...]`, user-defined literals, iterated component
associations, and `Big_Numbers`. These are 2022 fixes to 1983 and 2005 problems.

### It did not win general adoption despite being technically ahead
Ada had packages, generics, exceptions, tasking, strong typing and range types in 1983, a
decade or more before mainstream rivals. The documented reasons it stayed niche:
- **Hoare's 1980 Turing Award lecture** ("The Emperor's Old Clothes") attacked the Green
  design's size as "overly complex and hence unreliable" (Wikipedia's paraphrase) and ended
  with an appeal that the language not be used, in its then state, where reliability is
  critical (wording from memory). This framed Ada as bureaucratic and bloated for a generation
  of academics, even though Hoare later softened his view in a textbook foreword.
- **Compilers were late, slow and expensive.** Validation against the ACVC suite was mandatory
  to call a product "Ada"; early compilers "struggled with the large language" and "both
  compile-time and run-time performance were slow with primitive tools" (Wikipedia). A free,
  high-quality compiler (GNAT, NYU, 1995) arrived twelve years after Ada 83, by which point C++
  and Java had taken the mainstream.
- **The mandate.** The DoD required Ada for new software from 1991 (with a 1987 directive
  before it), "though exceptions were often granted", and removed the mandate in 1997 in favour
  of COTS. Mandated use produced resentment and "Ada = government paperwork" associations
  rather than enthusiasm; the lesson is that mandates buy compliance, not ecosystems.
- **Unfamiliar concurrency.** Into the 1990s "Ada's tasking model differed from what most
  real-time programmers were used to" (Wikipedia), i.e. rendezvous rather than
  threads-and-mutexes.
- **No open ecosystem until recently.** Alire (package manager) and the community-maintained
  Ada/SPARK libraries date from the 2020s; AdaCore's free "Community" editions were
  discontinued around 2022 in favour of FSF GNAT via Alire (from memory). Stack Overflow 2025
  puts usage at 1.4%, below Zig.

### Pointers were an afterthought for verification; the fix was imported from Rust
Access types exist since Ada 83 (with accessibility-level rules that are themselves a notorious
complexity), but SPARK excluded them entirely until 2019 because aliasing defeats proof. The
solution, described in AdaCore's "Using Pointers in SPARK" (June 2019) and the paper "Borrowing
Safe Pointers from Rust in SPARK" (Jaloyan, Moy, Paskevich, 2018): "an object designated by a
pointer always has a single owner"; assignment *moves* ownership; a callee can *borrow*
(read-write, exclusive) or *observe* (read-only, shared) for a scope; "Concurrent Read,
Exclusive Write". AdaCore names Rust explicitly as what "made the ownership restrictions look
tractable and perhaps desirable for safety", along with Taft's ParaSail. Restrictions in the
2019 preview: no general access types (to avoid stack accessibility issues), no access types in
tagged components, local borrowers not yet supported by proof, "memory leak checking is
specified but not yet implemented", and moving one array element makes the whole array
unreadable until reassigned. Several have since been lifted (not re-verified per release). The
point for Sisu: the formal-methods community independently concluded that Rust's ownership
discipline is the right model for pointer-manipulating code, and that it can be expressed as
rules on an existing language without lifetime annotations on every reference, because SPARK
forbids the general case rather than typing it.

### Generics without inference, OOP bolted on
Generic units require explicit instantiation (`package P is new Q (T)`), formal packages and
formal subprograms are powerful but heavy, and there is no implicit instantiation or
trait-style constraint resolution. Tagged types (Ada 95) brought dispatching, `'Class`-wide
types and abstract types into a language designed around packages, with object-prefix notation
only in Ada 2005 and interfaces only in Ada 2005; Ichbiah publicly disagreed with the Ada 9X
direction (from memory). The result is capable but idiosyncratic, and SPARK restricts OOP
further (Liskov-style `'Class` contracts).

### Three string types
`String` is a fixed-length array of `Character`; `Bounded_String` and `Unbounded_String` are
library types with their own operations; `Wide_String` and `Wide_Wide_String` add Unicode
variants. Simple text handling is verbose and conversion-heavy; this is a frequent beginner
complaint and a direct consequence of designing strings as arrays in 1983.

### Single-vendor ecosystem
GNAT is effectively the only maintained compiler (GCC-based; others are legacy or
embedded-only), SPARK is defined by AdaCore, and the yearly Pro releases are commercial. The
community (`ada-lang.io`, Alire, AdaCore's `learn.adacore.com`) is active but small; TIOBE's
rank 17 is driven by defence and avionics hiring. Risk for adopters: one company's priorities.

### Learnability
The Ada 2022 Reference Manual runs to roughly 900 pages plus annexes; accessibility rules,
discriminants, tasking, generics and contracts are each a course in themselves. The rules are
*regular* in a way C++'s are not, but Hoare's complexity critique still has force; AdaCore's
`learn.adacore.com` is the main modern on-ramp.

## Designer retrospectives
- Ichbiah (d. 2007) left no retrospective in the HOPL sense; his documented disagreement with
  the Ada 9X process is the main record (from memory, not re-verified).
- Tucker Taft (Ada 95 lead) designed ParaSail (2009-) as an experiment in a language with no
  pointers, no global variables and implicit parallelism; AdaCore cites ParaSail as one of two
  inspirations for SPARK's ownership model. That is as close to "what I would do differently"
  as the Ada line offers: remove aliasing from the language rather than analyse it.
- AdaCore's SPARK team on pointers (2019): pointers were excluded because "aliasing undermines
  its analysis"; Rust's ownership made the restrictions "look tractable and perhaps desirable
  for safety".
- The ARG's Ada 2022 feature set (ergonomics, parallelism, Jorvik) is the committee's own list
  of what was missing.

## Influence
Given to others: generics as the model for C++ templates and exceptions as one model for C++
exceptions (Stroustrup, D&E); packages as the model for C++ namespaces and arguably Fortran 90
modules; parameter modes as Fortran's `intent`; the whole language as the basis of VHDL and
PL/SQL syntax; range/subrange types into Nim and various refinement-type systems; contracts in
a systems language into C++26 and into the design space Rust crates (`contracts`) and
Kani/Creusot occupy; Ravenscar as the reference restricted-tasking model for certifiable
real-time (copied by Ravenscar-like profiles in other RTOS worlds). Rust's own documented
influences list does not include Ada; the shared ideas (strong typing, explicit mutability, no
implicit conversions) are convergent, and the documented borrowing runs the other way: SPARK
took ownership from Rust.

Taken from others: Pascal and ALGOL 68 (syntax, strong typing, subranges), CLU (exceptions,
abstraction), Simula (tasks via a different route), Eiffel (design by contract, Ada 2012), Rust
and ParaSail (ownership, SPARK 2019).

## Lessons for Sisu
- **Adopt**:
  - *Range subtypes, derived types and arrays indexed by discrete types*, with
    `'First`/`'Last`/`'Range`-style attributes. They remove whole classes of off-by-one and
    unit errors and give the prover the bounds it needs.
  - *Contracts as declaration aspects* (pre, post, type invariants, predicates) that are
    executable *and* a proof surface, in the signature from version 1, not a library macro.
  - *Parameter modes and named arguments.* Cheap, readable, and the foundation of flow
    analysis.
  - *Runtime checks on by default*, removable only with evidence (proof) or an explicit, scoped
    opt-out.
  - *Restriction profiles as a language mechanism* (`Restrictions`/`Profile`): a shipped,
    25-year-old precedent that subsetting works when the subset is checkable by the compiler.
    Sisu should have a certifiable concurrency/allocation profile from the start.
  - *Compiler-checked interface consistency across compilation units.* Never header files.
- **Adapt**:
  - *SPARK's assurance ladder (Stone to Platinum) as the tooling model*: make "valid in the
    verifiable subset", "initialisation proven", "AoRTE proven" distinct, cheap-to-reach
    levels, with the prover integrated in the standard toolchain rather than a separate
    product.
  - *Ownership rules in the SPARK style* (single owner, move, borrow, observe) for the safe
    subset, but with inference and lifetime elision so that annotations are rare; SPARK shows
    the discipline can be imposed without per-reference lifetimes if the general case (general
    access types, cycles) is pushed into the unsafe subset.
  - *Generics*: keep Ada's explicit formal interfaces (which make instantiation errors local
    and clear) but add implicit instantiation and trait-style constraints so that `Vectors
    (Natural, Integer)` ceremony disappears.
  - *Readability over writability* as a stated principle, enforced by review of every syntax
    decision, but with Ada 2022's ergonomics (`@`, declare expressions, delta/container
    aggregates, universal `'Image`) present from day one rather than after 39 years.
  - *Concurrency*: protected objects (monitors with declarative barriers) are a better safe
    primitive than mutexes; combine with a Ravenscar/Jorvik-style profile for the certifiable
    subset and a richer model outside it.
- **Avoid**:
  - *Duplicated specification and body files.* Generate the interface from the implementation
    (or a single file with visibility markers); Ada's model gives consistency checking but at a
    verbosity cost nobody else accepted.
  - *Multiple string types.* One UTF-8 string with slices; fixed-length and bounded variants as
    library types.
  - *Explicit-only generic instantiation and OOP added late.* Design polymorphism
    (traits/interfaces, generics) first; Ada 95's retrofit is capable but strange.
  - *Mandate-driven adoption and validation-gated compilers.* The 1983-1995 gap with no free
    compiler is where the language lost. Sisu's reference compiler must be open and good from
    the start.
  - *Single-vendor governance.* AdaCore has been a good steward but the ecosystem's size
    reflects the risk.
  - *Standardising features before implementations exist* (Ada 2022 `parallel`): ship with a
    reference implementation or not at all.
- **Open questions raised**:
  - *How much proof is free?* SPARK's AoRTE needs preconditions on most non-trivial
    subprograms; "Silver" level is reachable at scale (NVIDIA, NATS) but the cost is engineer
    time writing contracts. Whether Sisu should aim for a SPARK-grade verifiable subset in v1
    (which constrains semantics: no aliasing, bounded recursion, no exceptions across proof
    boundaries) or only for runtime checks plus an optional prover later is a real trade-off;
    SPARK suggests the semantics must be prover-friendly from the start or the subset will be
    tiny.
  - *Verbosity vs. adoption.* Ada's record suggests readability-first syntax wins in safety
    engineering and loses among general developers. Sisu wants both; it is unclear whether a
    single syntax can.
  - *Ownership without lifetimes* (SPARK) vs. *ownership with lifetimes* (Rust): SPARK's
    restriction-based approach loses expressiveness (no general access types, limited borrowing
    in data structures) but needs no annotations. Which trade-off suits Sisu's intended domains
    has to be decided with the Rust analysis in hand.

## Sources
1. Wikipedia, "Ada (programming language)" (Steelman, Green/Ichbiah, standards timeline,
   mandate 1991-1997, Hoare critique, notable systems):
   https://en.wikipedia.org/wiki/Ada_(programming_language) (fetched Oct 2026)
2. Ada Information Clearinghouse, Ada 2022 standard page (ISO/IEC 8652:2023):
   https://www.adaic.org/ada-resources/standards/ada22/ ; ISO catalogue entry published
   2023-05-01: https://scc-ccn.ca/standardsdb/standards/8179852
3. AdaCore, "What's New in Ada 2022" course (feature list):
   https://learn.adacore.com/courses/whats-new-in-ada-2022/index.html
4. GNAT Reference Manual, "Implementation of Ada 2022 Features" (`-gnat22`):
https://docs.adacore.com/gnat_rm-docs/html/gnat_rm/gnat_rm/implementation_of_ada_2022_features.html
5. AdaCore, "Introduction to Jorvik" (blog):
   https://www.adacore.com/blog/introduction-to-jorvik ; AdaCore guideline CON02 "Use the
   Jorvik Profile":
https://learn.adacore.com/courses/Guidelines_for_Safe_and_Secure_Ada_SPARK/chapters/guidelines/concurrency/con02_use_the_jorvik_profile.html
   ; GNAT predefined profiles:
   https://docs.adacore.com/gnat_ugx-docs/html/gnat_ugx/gnat_ugx/the_predefined_profiles.html
6. Ada-Europe / AEiC 2026 paper on Jorvik and multicore (OASIcs vol. 143):
https://drops.dagstuhl.de/storage/01oasics/oasics-vol143-aeic2026/OASIcs.AEiC.2026.3/OASIcs.AEiC.2026.3.pdf
7. SPARK User's Guide, "Concurrency and Ravenscar Profile":
   https://docs.adacore.com/spark2014-docs/html/ug/en/source/concurrency.html
8. C. Dross / AdaCore, "Using Pointers in SPARK", Jun 2019:
   https://www.adacore.com/blog/using-pointers-in-spark
9. G.-A. Jaloyan, Y. Moy, A. Paskevich, "Borrowing Safe Pointers from Rust in SPARK", 2018:
   https://arxiv.org/abs/1805.05576 ; "Verification of Programs with Pointers in SPARK" (2020):
   https://hal.ccsd.cnrs.fr/LMF/hal-03094566v1
10. AdaCore, "NVIDIA Adoption of SPARK Ushers in a New Era in Security-Critical Software
    Development" (case study, vendor-published):
https://www.adacore.com/case-studies/nvidia-adoption-of-spark-new-era-in-security-critical-software-development
    ; AdaCore/NVIDIA partnership page: https://www.adacore.com/nvidia
11. DevClass, "SPARK as good as Rust for safer coding, AdaCore cites NVIDIA case study", 8 Nov
    2022:
https://devclass.com/2022/11/08/spark-as-good-as-rust-for-safer-coding-adacore-cites-nvidia-case-study/
12. C. A. R. Hoare, "The Emperor's Old Clothes", Turing Award lecture 1980, CACM 24(2), 1981
    (not retrievable in this session; quotes from memory).
13. AdaCore and Thales, "Implementation Guidance for the Adoption of SPARK" (assurance levels
    Stone-Platinum), 2017-2020 (from memory; verify edition).
14. Stack Overflow Developer Survey 2025: https://survey.stackoverflow.co/2025/technology ;
    TIOBE Index Sep 2026: https://www.tiobe.com/tiobe-index/
15. B. Stroustrup, *The Design and Evolution of C++* (1994), on Ada generics and exceptions as
    influences (from memory).


