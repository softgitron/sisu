# Separating algorithm from schedule

## Why this family matters for Sisu

Sisu wants Fortran-class speed from code that reads like Python, and its ownership rules give the compiler
exact aliasing information. This family writes *what* to compute and *how* to compute it (loop order,
fusion, tiling, vectorisation, parallelism, memory layout) as separate programs, with the second unable to
change the first. It answers whether a general-purpose language should have a `schedule` construct, and
which part of the idea pays outside image processing and machine-learning kernels.

## Systems covered

| System | Years | Status (Oct 2026) | Central idea |
|---|---|---|---|
| Halide | 2012- | Active; v21 is the latest release; used for Pixel Visual Core | Pure algorithm plus a schedule: `tile`, `vectorize`, `parallel`, `compute_at` |
| TVM | 2018- | Apache project; criticised by Lattner (2025) | Halide-style schedules for tensor operators, with learned search |
| Taichi | 2019- | 1.7.4; no commit since July 2025 | Python-embedded kernels; data layout declared separately |
| Tiramisu | 2019- | Research | Polyhedral IR with algorithm, schedule, layout and communication layers |
| Exo | 2022- | Active research (Exo 2, ASPLOS 2025) | User scheduling as rewrites proven equivalent by effect analysis |
| Mojo | 2023- | 1.x (2026) | No separate schedule: compile-time parameters and library combinators |
| Futhark | 2014- | Active | Size types; compiler-generated versions chosen by tuned thresholds |
| Dex | 2019-2024 | Dormant | Index-set types make loop and array shapes agree |
| Bluespec | 2000- | Open-source compiler (bsc 2026.07.1) | Guarded atomic rules; the compiler builds the concurrent schedule |
| ISPC | 2011- | Intel open source | SPMD on SIMD: `uniform`/`varying`; vectorisation as a programming model |
| Odin `#soa`, Zig `MultiArrayList` | 2016- | Shipping | Struct-of-arrays chosen by type, field access unchanged |

## The ideas

### The algorithm and the schedule are separate programs (Halide)

Halide (Ragan-Kelley, Adams, Paris, Levoy, Amarasinghe, Durand; SIGGRAPH 2012, PLDI 2013) is embedded in
C++. An algorithm is a set of pure functions over integer coordinates; in the project's words the algorithm
has "no storage or order" and the schedule "defines order, locality; implies storage" [1][2]:

```cpp
// The algorithm - no storage or order
blur_x(x, y) = (input(x-1, y) + input(x, y) + input(x+1, y))/3;
blur_y(x, y) = (blur_x(x, y-1) + blur_x(x, y) + blur_x(x, y+1))/3;
// The schedule - defines order, locality; implies storage
blur_y.tile(x, y, xi, yi, 256, 32).vectorize(xi, 8).parallel(y);
blur_x.compute_at(blur_y, x).vectorize(x, 8);
```

The schedule chooses the three things that dominate image pipelines: parallelism, locality and
recomputation (redundant work in overlapping tiles, traded for cache residency) [2]. Because every stage is
pure, any legal schedule computes the same values. The 2022 formal semantics states the rule ("the result of
a program should always be the same, regardless of how it is scheduled"), notes that recomputation and
computation on uninitialised data make it harder to guarantee than reordering, and reports that the
formalisation "revealed flaws and led to improvements" in the compiler [3].

**What it buys.** PLDI 2013 [2]: Adobe's local Laplacian filter (262 lines of C++ with OpenMP and Intel
IPP, "several months to develop, including 2-3 weeks dedicated to optimization") was rewritten "in one day,
in 52 lines" and ran 1.7x faster, its schedule mixing "different fusion, tiling, vectorization, and
multithreading strategies throughout the expansive 99 stage graph". Across five pipelines Halide was 1.2x
to 4.4x faster than expert x86 code in 2x to 18x fewer lines; with a GPU schedule the bilateral grid beat
the hand-tuned CUDA of the algorithm's own authors by 2.3x, and interpolation ran 5.9x faster than the
expert CPU code. Several of those schedules came from the paper's autotuner, not a person [2]. Google
programmed the Pixel 2's Pixel Visual Core with "Halide for image processing and TensorFlow for machine
learning" [4]; Halide also targets Qualcomm's Hexagon (QuRT) [1], and its release numbers follow LLVM's
[5].

**Where it stayed niche.** Schedules are expert work: "writing schedules that achieve high performance is
hard: it requires expertise in hardware architecture and optimization, and even then, the space of possible
schedules is enormous" [6]. And the algorithm language is pure functions over rectangular domains with
restricted reductions. That restriction is what makes schedules checkable, and why Halide has stayed in
imaging and parts of ML.

### Search instead of experts: autoschedulers (Halide, TVM, Futhark)

Because experts are scarce, the schedule is searched. Halide's autoschedulers are Mullapudi et al.
(SIGGRAPH 2016), Li et al. (SIGGRAPH 2018, with differentiable Halide), Adams et al. (SIGGRAPH 2019: tree
search with a cost model trained on "hundreds of thousands of random" programs) and Anderson et al.
(OOPSLA 2021, GPUs) [7]. Adams 2019 beat the 2016 autoscheduler "by 75% on average with no autotuning" and
was "the first automatic scheduling algorithm to significantly outperform human experts on average" [6].
TVM (Chen et al., OSDI 2018) carried the model to tensor operators, adding `tensorize` for hardware
intrinsics and "a novel, learning-based cost modeling method", and reported results "competitive with
state-of-the-art, hand-tuned libraries" [8]; AutoTVM, Ansor and MetaSchedule then searched larger spaces
[25]. TVM's original tensor-expression interface:

```python
B = te.compute((n,), lambda i: A[i] * 2.0)                 # algorithm
s = te.create_schedule(B.op)                               # schedule
xo, xi = s[B].split(B.op.axis[0], factor=64)
s[B].parallel(xo); s[B].vectorize(xi)
```

Futhark's incremental flattening is the compiler-chosen variant: it generates several versions of each
nested parallel construct and picks one at run time against thresholds that `futhark autotune` sets [9].

**Where it failed.** Search spaces and cost models are tied to one sublanguage and one hardware family.
Lattner's 2025 assessment is that TVM "struggled to deliver peak performance on modern AI hardware", that
"vendors forked the code, made incompatible changes, and struggled to stay in sync", and that it "was
designed for 'TradAI'" [10]; more broadly, "Compilers can't optimize as well as a skilled CUDA developer",
and he has never seen the "sufficiently smart compiler" work out [11].

### Schedules as checked rewrites (Exo, Tiramisu)

Exo (Ikarashi, Bernstein, Reinking, Genc, Ragan-Kelley; PLDI 2022) makes the schedule a sequence of
rewrites that the performance engineer applies, backed by "a set of effect analyses which guarantee program
equivalence and memory safety through these transformations", with SMT queries for the index arithmetic;
custom instructions, memories and accelerator state are defined in user libraries [12]. The price is stated:
"We make the simplifying assumption of affine loops and array indexing" [12]. Matrix multiply and
convolution kernels for the Gemmini accelerator and AVX-512 took "a few dozen lines of code each" [12], and
Exo code is reported "as performant as" MKL, OpenBLAS and BLIS [13]. Exo 2 (ASPLOS 2025) lets users define
scheduling operators ("Cursors"); its libraries cover more than 80 kernels and cut scheduling code "by an
order of magnitude" [14]. Tiramisu (Baghdadi et al., CGO 2019) separates algorithm, schedule, data layout
and communication in a polyhedral IR [25]. Ragan-Kelley's group continues the verified thread (scheduling
rewrites, POPL 2022; a verified tensor compiler, PLDI 2024) [15].

### Data layout separated from computation (Taichi, Halide, Chapel, Odin, Zig)

Taichi (Hu, Li, Anderson, Ragan-Kelley, Durand; SIGGRAPH Asia 2019) embeds kernels in Python and declares
the data structure separately, so that kernels are written "as if they are working with a dense array"
whether the layout is dense, blocked, pointer-sparse or bitmasked [16]:

```python
x = ti.field(ti.f32)
ti.root.pointer(ti.ij, 32).dense(ti.ij, 8).place(x)   # layout: sparse blocks of dense 8x8 cells

@ti.kernel
def scale(k: ti.f32):
    for i, j in x:            # outermost loop is parallel; visits only active cells
        x[i, j] *= k
```

The paper reports "1/10 th as many lines of code" and "4.55x higher performance on average" than
hand-optimised references [16]; the project shipped 1.7.4 and has had no commit since July 2025 [17].
Halide's storage directives and Chapel's domain maps ([`parallel-languages.md`](parallel-languages.md)) are
the same separation. In general-purpose systems languages the version that shipped is struct-of-arrays:
Odin's `#soa[N]Vector3` stores each field as its own array with the "same syntax as AOS" (`v[0].x`, or
`v.x[0]` for the same memory) [18], and Zig's standard library has `MultiArrayList`.

### Shape in the types (Futhark, Dex)

Futhark's size types put extents in signatures (`def dot [n] (xs: [n]f64) (ys: [n]f64): f64`), so a
mismatch is a type error and the compiler knows sizes when it picks versions [9]; Dex typed arrays by index
sets (`n=>m=>Float`), so a loop over `i:n` and an array of `n` cannot disagree [19]. Both make schedules
safer by making bounds static; neither separates the schedule from the algorithm.

### No separate schedule at all (Mojo, ISPC)

Mojo is the deliberate counter-position. Its `algorithm` package offers "High performance data operations
including vectorization, functional map, and tiling" [20] as higher-order functions parameterised at compile
time: the programmer writes the optimised form directly, and nothing checks it against a reference
algorithm. Modular's 2023 launch material also showed an `autotune` facility that no longer exists
[unverified]. Lattner's position (above) that the expert, not the compiler, must decide is shared by Exo;
the two differ on whether the expert's decisions should be checked rewrites of a reference program.

ISPC (Matt Pharr at Intel; open source since 2011) rejects compiler-chosen vectorisation. Its founding
critique, credited to T. Foley, is that "auto-vectorization is not a programming model" [21]: an ISPC
function describes one program instance, values are `uniform` or `varying` across SIMD lanes, and
vectorisation always happens. Embree "made extensive use of ispc" and DreamWorks' MoonRay renderer was
written in it; Pharr regrets that `unmasked` is "more or less a workaround for a hardware limitation that
leaked into the language" and that the SIMD width is fixed per file [22].

### Concurrency schedules derived from declared access (Bluespec, Bevy)

Bluespec (Hoe and Arvind, ICCAD 2000; Bluespec Inc.; now a BSD-licensed compiler) describes hardware as
"Guarded Atomic Actions", and "the compiler then handles scheduling" [23]: it picks which rules fire in the
same clock cycle so that the effect equals some one-rule-at-a-time order. Method calls carry implicit guards
(`fifo.enq` waits for space, `fifo.first` for data):

```bsv
rule produce (count < 10);
    fifo.enq(count);
    count <= count + 1;
endrule
rule consume;
    $display(fifo.first); fifo.deq;
endrule
```

It is a schedule that cannot change the result, derived from each rule's read and write sets. Its
general-purpose cousin is the entity-component-system scheduler: in Bevy (Rust) "non-conflicting systems can
run in parallel", where conflicts come from each system's declared component access [24].

## Transferable ideas for Sisu

Can a general-purpose language have a schedule that cannot change the result? Yes, over a sublanguage whose
dependences the compiler sees: Halide's pure functions over integer domains, Exo's affine loops, Bluespec's
rules with known read and write sets, and in Sisu, loops whose independence ownership proves. Outside such a
sublanguage equivalence of arbitrary rewrites is undecidable, and even inside it floating-point
reassociation changes results. Is it worth having outside image processing and ML kernels? The loop-schedule
half pays where a few dense kernels dominate run time, the same algorithm is retargeted across machines and
experts are available; there is no measured success in general application code. The layout half and the
access-set half have broader evidence. So: no `schedule` block in Sisu, and four fragments worth taking.

1. **Layout-polymorphic collections (Odin `#soa`, Zig `MultiArrayList`, Taichi).** A comptime-generated
   `SoaList[T]` stores each field in its own array; code is written against the logical struct.

   ```sisu
   @derive(Soa)
   struct Particle:
       pos: Vec3
       vel: Vec3
       mass: f32

   var ps = SoaList[Particle]()          # three arrays, one per field
   ps.push(Particle(pos=p, vel=v, mass=1.0))
   for q in par mut ps:                  # q: a view struct of three `ref mut` fields
       q.pos += q.vel * dt                 # same source as for list[Particle]
   ```

   Cost: library code, provided comptime can construct a type from a struct's fields; element access returns
   a `view struct` of field references, which Sisu's views already express. Conflicts: depends on Q-27
   (comptime-parameterised types), whose text names SoA transforms as the motivating case. Confidence:
   medium-high.

2. **A reference algorithm with generated equivalence tests (`@refines`).** The clear version stays in the
   source as the specification; the optimised version names it, and `sisu test` generates property tests
   comparing the two on `arbitrary` inputs, with an explicit tolerance where reassociation is intended. A
   later verifier (Q-16) could prove such pairs for the pure, affine subset, as Exo does.

   ```sisu
   pure def blur_ref(img: Image) -> Image: ...                   # the algorithm

   @refines(blur_ref, tolerance=Ulps(2))                         # sisu test checks blur == blur_ref
   def blur(img: Image) -> Image: ...                            # tiled, vectorised, par
   ```

   Cost: an attribute, a test generator over the existing `check`/`arbitrary` machinery
   ([`17-testing.md`](../../proposal/17-testing.md)), and documentation linking the pair. Conflicts: none.
   Confidence: medium.

3. **Schedule directives that are checked no-ops.** `par` loops (from
   [`parallel-languages.md`](parallel-languages.md)) plus statement attributes that either hold or fail to
   compile: `@vectorize(8)` (ISPC's lesson: vectorisation is a guarantee, not a hope), `@unroll(4)`, and
   `@tile(64, 64)` on a loop nest whose independence is proven.

   ```sisu
   @vectorize(8)
   for i in range(1, len(out)):
       out[i] = out[i - 1] * decay + x[i]   # error: cannot vectorize: iteration i reads out[i - 1],
                                            #   written by iteration i - 1 (loop-carried dependence)
   ```

   Cost: attributes on statements (02-syntax puts them on the line before an item, D-046) and backend work
   per directive. Conflicts: that placement rule; the SIMD design (Q-23). Confidence: medium.

4. **Access-set scheduling as a library (Bluespec, Bevy).** Sisu's parameter conventions already declare
   access: a borrowed parameter is a read, `mut` an exclusive write. A comptime planner can build the
   conflict graph of a set of functions over shared state and run non-conflicting ones in parallel with the
   sequential order's result, without Bevy's access-declaring query types. Cost: a library over comptime
   reflection and the `par` pool. Conflicts: none. Confidence: low-medium (games and simulation have the
   demand; no evidence elsewhere).

Kernel DSLs in the Halide or TVM mould stay libraries: Sisu's part is comptime staging, `std.simd` (Q-23)
and C interop strong enough to host or call one.

## Dead ends

- **A general `schedule` block for arbitrary code**: no evidence of value outside dense loop nests, and no
  equivalence check outside restricted sublanguages.
- **Autoscheduling in a general-purpose compiler**: Adams 2019 needed a fixed search space and a cost model
  trained on random programs from it; Lattner's "sufficiently smart compiler" point stands.
- **Autotuning in the language**: it belongs in tools (`futhark autotune`), not in source.
- **A tensor-expression sublanguage in the core**: TVM's vendor forks and its lag behind new hardware
  (Lattner) show the cost of keeping one current.
- **SPMD as a language mode (ISPC)**: masks leak into the language (`unmasked`); explicit SIMD types (Q-23)
  cover the need.
- **Polyhedral optimisation as the general compiler's job**: Exo and Tiramisu need affine loops and
  indexing.

## Sources

1. Halide project site (blur example; targets including Hexagon/QuRT). https://halide-lang.org/
2. J. Ragan-Kelley et al., "Halide: A Language and Compiler for Optimizing Parallelism, Locality, and Recomputation in Image Processing Pipelines", PLDI 2013, http://people.csail.mit.edu/jrk/halide-pldi13.pdf ; "Decoupling Algorithms from Schedules for Easy Optimization of Image Processing Pipelines", SIGGRAPH 2012, http://people.csail.mit.edu/jrk/halide12
3. A. Reinking, G. Bernstein, J. Ragan-Kelley, "Formal Semantics for the Halide Language", arXiv:2210.15740, 2022. https://arxiv.org/abs/2210.15740
4. O. Shacham, M. Reynders, "Pixel Visual Core: image processing and machine learning on Pixel 2", Google blog, 17 October 2017. https://blog.google/products/pixel/pixel-visual-core-image-processing-and-machine-learning-pixel-2/
5. Halide repository and releases. https://github.com/halide/Halide ; https://github.com/halide/Halide/releases
6. A. Adams et al., "Learning to Optimize Halide with Tree Search and Random Programs", SIGGRAPH 2019 (ACM TOG 38(4)). https://halide-lang.org/papers/autoscheduler2019.html
7. R. Mullapudi et al., "Automatically Scheduling Halide Image Processing Pipelines", SIGGRAPH 2016; T.-M. Li et al., "Differentiable Programming for Image Processing and Deep Learning in Halide", SIGGRAPH 2018; L. Anderson et al., "Efficient Automatic Scheduling of Imaging and Vision Pipelines for the GPU", OOPSLA 2021. Listed at https://people.csail.mit.edu/jrk/
8. T. Chen et al., "TVM: An Automated End-to-End Optimizing Compiler for Deep Learning", OSDI 2018. https://www.usenix.org/conference/osdi18/presentation/chen
9. Futhark publications (incremental flattening, PPoPP 2019; size types, ARRAY 2021 and FHPNC 2023). https://futhark-lang.org/publications.html
10. C. Lattner, "Democratizing AI Compute, Part 6: What about AI compilers (TVM and XLA)?", Modular, 12 March 2025. https://www.modular.com/blog/democratizing-ai-compute-part-6-what-about-ai-compilers
11. C. Lattner, "Democratizing AI Compute, Part 7: What about Triton and Python eDSLs?", Modular, 26 March 2025. https://www.modular.com/blog/democratizing-ai-compute-part-7-what-about-triton-and-python-edsls
12. Y. Ikarashi, G. Bernstein, A. Reinking, H. Genc, J. Ragan-Kelley, "Exocompilation for Productive Programming of Hardware Accelerators", PLDI 2022. https://www.cs.cornell.edu/~yuka/pdf/exo_pldi2022_full.pdf
13. Exo project site. https://exo-lang.dev/
14. Y. Ikarashi, K. Qian, S. Droubi, A. Reinking, G. Bernstein, J. Ragan-Kelley, "Exo 2: Growing a Scheduling Language", ASPLOS 2025. https://arxiv.org/abs/2411.07211
15. J. Ragan-Kelley, publication list (verified scheduling rewrites, POPL 2022; verified functional tensor compiler, PLDI 2024). https://people.csail.mit.edu/jrk/
16. Y. Hu et al., "Taichi: A Language for High-Performance Computation on Spatially Sparse Data Structures", SIGGRAPH Asia 2019. https://yuanming.taichi.graphics/publication/2019-taichi/
17. Taichi releases and commit history (1.7.4; last commit 30 July 2025). https://github.com/taichi-dev/taichi/releases ; https://github.com/taichi-dev/taichi/commits/master
18. Odin overview, `#soa` data types. https://odin-lang.org/docs/overview/
19. Dex repository (no commit since January 2024); ICFP 2021 paper cited in [`parallel-languages.md`](parallel-languages.md). https://github.com/google-research/dex-lang
20. Mojo standard library, `algorithm` package (v1.1). https://mojolang.org/docs/std/algorithm/
21. M. Pharr, "The story of ispc: origins", 18 April 2018. https://pharr.org/matt/blog/2018/04/18/ispc-origins
22. M. Pharr, "The story of ispc: retrospective", 29 April 2018. https://pharr.org/matt/blog/2018/04/29/ispc-retrospective
23. Bluespec compiler (bsc) README (2026.07.1, BSD-3), https://github.com/B-Lang-org/bsc ; J. Hoe, Arvind, "Synthesis of Operation-Centric Hardware Descriptions", ICCAD 2000 (not re-read).
24. Bevy ECS schedule documentation (`MultiThreadedExecutor`, system conflicts). https://docs.rs/bevy_ecs/latest/bevy_ecs/schedule/index.html
25. Standard citations, not re-read: T. Chen et al., "Learning to Optimize Tensor Programs" (AutoTVM), NeurIPS 2018; L. Zheng et al., "Ansor", OSDI 2020; J. Shao et al., "Tensor Program Optimization with Probabilistic Programs" (MetaSchedule), NeurIPS 2022; R. Baghdadi et al., "Tiramisu: A Polyhedral Compiler for Expressing Fast and Portable Code", CGO 2019; M. Pharr, W. Mark, "ispc: A SPMD Compiler for High-Performance CPU Programming", InPar 2012.
