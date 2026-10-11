# Fortran

## Snapshot
| | |
|---|---|
| First appeared | 1957 (FORTRAN I for the IBM 704; project proposed by Backus in late 1953, compiler delivered April 1957) |
| Designed by | John Backus and the IBM team (Harlan Herrick, Robert Nelson, Irving Ziller, Roy Nutt, Sheldon Best, Lois Haibt, David Sayre et al.); standardised since 1966 by ANSI X3J3, now INCITS/Fortran (J3) with ISO WG5 |
| Current version (Oct 2026) | Fortran 2023 (ISO/IEC 1539-1:2023, published late 2023: OLCF says October, Lionel says 17 November). Fortran 202Y in progress: WG5 work list N2261 (June 2026 meeting, Warwick) includes generic subprograms (JP01) and templates (US17); the standard preprocessor item (US10) was removed. No publication date set. |
| Governance | ISO/IEC JTC1/SC22/WG5 sets direction; INCITS/Fortran (J3) does the technical work. Stated principles (Budiardja, J3 chair, 2024): "performance and numerical centric", "work hard such that feature specifications does not hamper optimization", "avoid chasing passing fads", "previous standards is proper subset of current standards". |
| Typing | Static, strong for user code; implicit typing by first letter unless `implicit none` (only becoming obsolescent in 202Y); `kind` parameters for precision; derived types; OOP since 2003; no generics until 202Y |
| Memory | `allocatable` arrays with automatic deallocation at scope exit; `pointer`/`target` with explicit aliasing; no GC; bounds checks optional (compiler flag) |
| Compilation | AOT. Compilers: GFortran (GCC 16), Intel ifx, NVIDIA nvfortran, HPE/Cray, IBM XL; LLVM Flang (in the LLVM monorepo, aiming to be "a peer to Clang"; OpenMP support beyond 2.5 still limited per a 2025 BSC report); LFortran (alpha; "will reach beta when it can reliably compile 10 third-party production codes", at 9/10 in 2026). |
| Primary domains | HPC: climate and weather (ECMWF IFS, WRF, E3SM), CFD, computational chemistry and materials (VASP, Gaussian, Quantum ESPRESSO), astrophysics, nuclear, reference numerical libraries (BLAS/LAPACK) |
| Notable users | DOE and EU leadership computing facilities (OLCF: Fortran makes up "a large share of applications that are running in supercomputing facilities"); SPEC CFP2006/CFP2017 floating-point components are written in Fortran. Stack Overflow 2025: 1.4% of respondents; TIOBE Sep 2026: #11 at 1.24%. |

## Stated design goals
Backus's HOPL paper (1978) is unusually candid. The motive was economic: in 1954 "the cost of
programmers associated with a computer center was usually at least as great as the cost of the
computer itself" and "programming and debugging accounted for as much as three-quarters of the
cost of operating a computer". The design bet was efficiency over language: "we were alone in
seeking to produce really efficient object programs", and "To this day I believe that our
emphasis on object program efficiency rather than on language design was basically correct. I
believe that had we failed to produce efficient programs, the widespread use of languages like
FORTRAN would have been seriously delayed." On the language itself: "We did not regard language
design as a difficult problem, merely a simple prelude to the real problem: designing a
compiler which could produce efficient programs."

The modern committee's goals (above) are a direct continuation: numerical performance first,
backward compatibility second, everything else third.

## Strengths

### Arrays are first-class, and the compiler knows their shape
Fortran 90 made whole-array expressions (`C = A + B * 2.0`), array sections (`A(2:5, :)`,
strides), intrinsic reductions (`sum`, `maxval`, `matmul`), assumed-shape dummy arguments,
arbitrary lower bounds and `allocatable` arrays with automatic deallocation part of the
language. Fortran 2023 added rank-agnostic code: `rank(n)` declarations, integer-array
subscripts `A(@[3,5])`, and bounds given as arrays, so a routine can be written once for any
rank. Because rank, shape and contiguity are in the type system, the compiler can vectorise,
fuse and parallelise without alias analysis of pointer arithmetic. Evidence by imitation:
NumPy's n-dimensional array with slicing and `order='F'`, Julia's arrays (the Julia manual:
column-major "is common in many languages like Fortran, Matlab, and R"), MATLAB (originally a
front end to LINPACK/EISPACK), Chapel and HPF.

### The no-aliasing rule on dummy arguments
The standard forbids modifying a dummy argument through one name while it is accessible through
another (another dummy, a global, or a pointer); doing so is non-conforming (Fortran 2018
clause 15.5.2.13, "Restrictions on entities associated with dummy arguments"; clause number
from memory). The compiler may therefore assume every argument is `restrict` in C terms. This
single rule is a large part of why Fortran compilers vectorise loops that C compilers cannot
without `restrict`, and it is the same guarantee that Rust derives from `&mut` exclusivity and
passes to LLVM as `noalias`. The difference is enforcement: Fortran trusts the programmer, Rust
checks.

### Declared purity: `pure`, `elemental`, `simple`
Fortran 95 introduced `pure` (no side effects beyond `intent(out)` arguments) and `elemental`
(a scalar function automatically applied element-wise over conforming arrays). Fortran 2023
adds `simple` (pure plus "an entirely local calculation", so safe for threads; all intrinsics
are `simple`). `intent(in/out/inout)` modes (1990, from Ada) make data flow explicit. NumPy's
ufuncs and Julia's broadcasting are `elemental` by another name.

### Parallelism in the language
- **Coarrays** (Fortran 2008, from Numrich and Reid's Co-Array Fortran on Cray, 1998): SPMD
  "images" with a cosubscript `A(:)[p]` for remote access, replacing an MPI send/receive pair
  with one assignment (OLCF's example: `if (p==p1) array2(:)[p2] = array1(:)`); teams, events
  and collectives added in 2018. Supported on Frontier with the HPE/Cray compiler.
- **`do concurrent`** (2008): asserts no cross-iteration dependencies; locality specifiers
  (`local`, `local_init`, `shared`, `default(none)`) in 2018; `reduce` in 2023. NVIDIA's
  nvfortran compiles standard `do concurrent` to GPUs with `-stdpar` (CUDA Unified Memory for
  data movement): a Jacobi solver ran "nearly 13x" faster on an A100 than on 40 CPU cores and
  "nearly the same as the OpenACC version", with the 2021 limitations of no procedure calls
  inside the loop and no reductions. OLCF's summary of the attraction: "everything is done in
  the language, potentially simpler for developer and better optimization opportunities".

### Numerics as a first-class concern
`kind` parameters with `selected_real_kind(p, r)`, the IEEE 754 intrinsic modules
(`ieee_arithmetic`, `ieee_exceptions`, 2003), built-in `complex` and an exponentiation operator
show a community that treats floating point as a specification problem, not a library detail.

### Backward compatibility measured in decades
1966 code compiles today. Deletions are rare and announced a decade ahead (Fortran 95 removed
`PAUSE` and assigned `GO TO`; Fortran 2018 deleted the arithmetic `IF`; `COMMON`,
`EQUIVALENCE`, labelled `DO` and `FORALL` are obsolescent, not gone). This is why the HPC
installed base (millions of lines, validated against physics) has never been forced to move.

### Standard C interoperability
`iso_c_binding` (2003) and C descriptors for allocatable and pointer arrays
(`ISO_Fortran_binding.h`, 2018) are what let Fortran codes call MPI, OpenMP runtimes, CUDA/HIP
and vice versa; OLCF calls it "probably one of the most useful feature in 'heterogeneous'
world". The lesson is that a standardised *descriptor* for arrays (base, rank, bounds, strides,
element size) is what makes a numerical language's FFI useful, not just a calling convention.

## Weaknesses

### Legacy defaults that were never removed
Implicit typing (an undeclared `i` is `integer`, an undeclared `x` is `real`) is the canonical
Fortran bug source; every modern file starts with `implicit none`, and the committee is only
making the default obsolescent in 202Y (US01, ref 23-177), roughly 65 years after it became a
known problem. Fixed-form source, `COMMON` blocks, `EQUIVALENCE`, computed `GO TO` and `D`
format descriptors survive as obsolescent. The compatibility principle ("previous standards is
proper subset of current standards") is the cause: nothing is ever actually deleted, so nothing
is ever actually fixed.

### No generics for 67 years
Generic programming was deferred from Fortran 2023 after five years of design (Lionel: "the
model looks quite useful"); Fortran 2023 shipped only the enabling pieces (`typeof`,
`classof`). The 202Y design (J3 generics subgroup: `template`, `requirement`, `instantiate`,
with `deferred` types and explicit instantiation, as previewed at OLCF in 2024) resembles Ada
generics and early C++0x concepts. Until it ships (2029 at the earliest), libraries use
preprocessor tricks, `include` files and per-kind copies (`_s`, `_d`, `_c`, `_z` BLAS variants)
for generic code.

### No standard preprocessor, no namespaces
The C preprocessor is used informally (`.F90` files); the 202Y proposal to standardise a
Fortran preprocessor (US10, "FPP") was removed from the work list in June 2026, so the status
quo continues. Module entities are imported into a flat scope (`use mod, only: ...` to control
it); the "scoped access to enumeration values" item (US14) and the dropped "namespace-like
module access" item show the committee knows.

### Compiler-specific module files
Modules (1990) compile to `.mod` files whose format is compiler-specific and version-specific,
so Fortran libraries cannot be distributed as binaries plus interfaces across compilers, and
build tools must order compilations by module dependencies (CMake's Fortran support and the
community `fpm` both exist largely to do this). The same problem reappeared 30 years later as
C++ BMIs.

### Strings and records are second-class
`character(len=n)` is fixed length; deferred-length allocatable strings (2003) help but every
operation trims. Derived types gained OOP in 2003 (`type, extends`, `class(*)`, `select type`)
and it took "by now" (2024) for "all major compilers [to have] implemented most if not all of
Fortran 2003". Fortran 2023's enumerations are already criticised in the community for
enumerators that are not namespaced and `select case` that need not be exhaustive.

### Trust-based parallel semantics
`do concurrent` "tells the compiler there are no data dependencies between iterations" but the
standard neither requires parallel execution nor checks the assertion; a wrong assertion is
silent wrong answers. Coarray synchronisation (`sync all`, `sync images`) is manual. The
no-aliasing rule is likewise unchecked. In HPC this has been acceptable for sixty years; it
would not be in a security-sensitive setting.

### Ecosystem and workforce
Vendor compilers dominate; GFortran lags on 2018/2023 features; Flang is OpenMP-incomplete
(2025); LFortran is alpha. The community `fortran-lang` project (fpm, stdlib, 2020-) is small.
OLCF's closing slides ask "How do we make sure we continue to have strong compilers?" and say
"Strong workforce development is needed".

## Designer retrospectives
- Backus (1978) on language design: "As far as we were aware, we simply made up the language as
  we went along"; "our entire attitude about language design had always been a very casual
  one"; "we were hopelessly optimistic in 1954 about the problems of debugging FORTRAN programs
  ... and hence syntactic error checking facilities in the first distribution of FORTRAN I were
  weak."
- Backus (1978) on what mattered: the FORTRAN I optimiser ("section 2") did index-register
  allocation by Monte Carlo analysis of branch frequencies from `DO` and `FREQUENCY`
  statements; "The degree of optimization performed by section 2 in its treatment of indexing,
  array references, and loops was not equalled again until optimizing compilers began to appear
  in the middle and late 1960s."
- Backus's Turing lecture (1977), "Can Programming Be Liberated from the von Neumann Style?",
  repudiated the assignment-statement, word-at-a-time model he had made mainstream in favour of
  functional array programs (from memory; the lecture is well known).
- Lionel (2023) and Budiardja (2024) on generics: five years of design and still not shipped;
  "the model looks quite useful".

## Influence
Given: the compiled-language idea itself and optimising-compiler research (Backus's section 2;
later Allen and Cocke's work was for Fortran); array syntax and column-major semantics to
MATLAB, NumPy (plus `f2py`), Julia, R and Chapel; `elemental`/ufunc-style element-wise
functions; `intent` modes in later numerical DSLs; coarrays/PGAS to Chapel, X10 and UPC; `do
concurrent` as the model for "standard language parallelism" (C++'s parallel algorithms and
`stdpar` play the same role). Mojo's documentation cites Python, C++, Rust and Swift rather
than Fortran; its array lineage is via NumPy (no direct Fortran influence found).

Taken: Ada (`intent` modes, modules in the package spirit, 1990), C (`iso_c_binding`, 2003),
Simula/C++ (OOP, 2003), Co-Array Fortran (Cray, 2008), HPF and OpenMP experience (`do
concurrent`, locality), C++/Ada generics (202Y templates).

## Lessons for Sisu
- **Adopt**:
  - *Multi-dimensional arrays with rank, shape, bounds and strides in the type*, with sections
    and element-wise expressions, so that the optimiser does not have to recover structure from
    pointer arithmetic. This is the one thing Fortran has that no mainstream systems language
    has matched.
  - *Declared purity levels* (`pure`/`elemental`/`simple`) as effect annotations the compiler
    can exploit for vectorisation and parallelism.
  - *A standard array descriptor at the FFI boundary* (`ISO_Fortran_binding.h` style), so
    numerical libraries interoperate without hand-written marshalling.
  - *Numeric precision as a type parameter* (`kind`), with IEEE semantics specified rather than
    implied.
- **Adapt**:
  - *The no-aliasing guarantee should be checked, not trusted.* Rust's exclusivity gives the
    same `noalias` information to the backend with enforcement; Sisu should get Fortran's
    vectorisation from Rust-style rules, with an explicit unsafe escape for overlapping views.
  - *Parallel loops as a verified contract*: `do concurrent`'s "no cross-iteration dependency"
    assertion should be something the compiler can check in the safe subset (purity plus
    disjoint index sets) rather than an unchecked promise. GPU offload of standard-language
    loops (nvfortran `-stdpar`) shows the payoff when the language is the programming model.
  - *PGAS/coarray-style remote access* belongs in a library or extension with the same
    syntax-level visibility (an operation that may hit the network must look different from a
    local assignment), not in the core.
  - *Layout*: pick one default (row- or column-major) and provide layout-polymorphic views (C++
    `mdspan` style) rather than making layout a language-wide constant that fights every C
    library.
- **Avoid**:
  - *Any implicit declaration or implicit typing.* Sixty-five years to deprecate is the cost of
    a convenience default.
  - *Never deleting anything.* Have a deprecation-and-removal policy with migration tooling;
    "proper subset forever" freezes mistakes.
  - *Compiler-specific module artifacts* without a documented, stable interface format.
  - *Shipping generics and OOP decades after the language*: design generics first; Fortran's
    202Y templates arriving in 2029 for a 1957 language is the extreme case.
  - *Leaving the preprocessor and build ordering to convention.*
- **Open questions raised**:
  - *Trusted vs. enforced non-aliasing*: HPC has lived with Fortran's trust model for sixty
    years with acceptable results; enforcement costs expressiveness (Rust's difficulty with
    mutable sub-array views). Where Sisu draws this line decides whether numerical kernels are
    pleasant to write.
  - *Should the language own the parallel programming model?* Fortran (coarrays, `do
    concurrent`) says yes and gets vendor GPU offload; the C/C++ world says no and uses
    MPI/OpenMP/CUDA. Both work; the language-owned path depends on vendor compiler buy-in Sisu
    will not have at first.
  - *Array programming in a general-purpose language*: APL/BQN-style whole-array thinking and
    Fortran-style explicit loops coexist uneasily; NumPy and Julia show both are needed.
    Whether Sisu's arrays should be a core type or a library over a good slice type is open.

## Sources
1. J. Backus, "The History of FORTRAN I, II, and III", ACM SIGPLAN History of Programming
   Languages Conference, 1978.
   https://softwarepreservation.computerhistory.org/FORTRAN/paper/p25-backus.pdf
2. R. D. Budiardja (OLCF, INCITS/Fortran chair), "An Overview of The Fortran Standard: Fortran
   2023 and Beyond", OLCF User Call, April 2024.
   https://www.olcf.ornl.gov/wp-content/uploads/2024-04_OLCFUserCall_FortranStandard.pdf
3. S. Lionel, "Doctor Fortran" posts on Fortran 2023 (publication 17 Nov 2023, feature summary,
   generics deferred): https://stevelionel.com/drfortran/tag/fortran-2023/
4. J. Reid, "The new features of Fortran 2023", WG5 N2212:
   https://wg5-fortran.org/N2201-N2250/N2212.pdf
5. WG5 N2261, Fortran 202Y work items (June 2026 meeting):
   https://wg5-fortran.org/N2251-N2300/N2261.txt
6. NVIDIA, "Accelerating Fortran DO CONCURRENT with GPUs and the NVIDIA HPC SDK", 2021:
https://developer.nvidia.com/blog/accelerating-fortran-do-concurrent-with-gpus-and-the-nvidia-hpc-sdk/
7. Julia manual, Performance Tips (column-major, Fortran lineage):
   https://docs.julialang.org/en/v1/manual/performance-tips/
8. LFortran project page (alpha status, beta criterion): https://lfortran.org/
9. J3 liaison report on LLVM Flang, June 2026 (26-175):
   https://j3-fortran.org/doc/year/26/26-175.txt ; BSC report on Flang maturity, April 2025:
   https://zenodo.org/records/17483543
10. Wikipedia, "Fortran" (standards timeline, Fortran 90 features, removals, SPEC/TOP500
    claims): https://en.wikipedia.org/wiki/Fortran
11. fortran-lang Discourse, "Which five features should Fortran 202Y implement?" (community
    priorities; enumeration critique):
https://fortran-lang.discourse.group/t/which-five-features-should-fortran-202y-implement/3977
12. Intel Fortran Compiler 2026 developer guide, Fortran 2023 conformance:
https://www.intel.com/content/www/us/en/docs/fortran-compiler/developer-guide-reference/2026-1/conformance-compatibility-and-fortran-features.html
13. Stack Overflow Developer Survey 2025: https://survey.stackoverflow.co/2025/technology ;
    TIOBE Index Sep 2026: https://www.tiobe.com/tiobe-index/
14. ISO/IEC 1539-1:2018 clause 15.5.2.13 (dummy argument aliasing restrictions); clause number
    from memory, verify against the J3 interpretation document
    https://j3-fortran.org/doc/year/24/24-007.pdf


