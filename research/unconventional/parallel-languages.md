# Structured parallelism and data parallelism

## Why this family matters for Sisu

Sisu's ownership rules already decide at compile time whether two pieces of code can touch the same data.
Every parallel language answers that question in one of three ways: the compiler checks it (occam), the
programmer asserts it (OpenMP, Fortran's `do concurrent`), or a tool hunts for violations at run time
(Cilk's race detector, Go's). This family says whether a checked parallel form deserves syntax in Sisu,
and what occam, X10, Chapel and Cilk teach about its shape.

## Systems covered

| System | Years | Status (Oct 2026) | Central idea |
|---|---|---|---|
| occam | 1983-1995 | Dead with the transputer; occam-pi (Kent) academic | `SEQ`, `PAR`, `ALT` blocks; compiler-checked usage rules for parallel components |
| X10 | 2004-2019 | Inactive since 2.6.2 (January 2019) | `async`/`finish`/`at` over places (APGAS) |
| Chapel | 2009- | 2.10 (September 2026); HPE, hosted by HPSF | `forall` versus `coforall`, `cobegin`, locales and domain maps |
| Fortress | 2006-2012 | Cancelled | Implicitly parallel `for`, mathematical notation |
| Cilk, Cilk Plus, OpenCilk | 1994-; Intel 2009-2017; MIT 2021- | OpenCilk 3.0 (May 2025) | `spawn`/`sync`, work stealing, serial elision |
| OpenMP | 1997- | 6.0 (November 2024); 6.1 draft (July 2026) | Pragmas that leave the sequential program intact |
| NESL; Data Parallel Haskell | 1990s; 2007- | Historical; DPH never left research | Nested data parallelism compiled by flattening |
| Futhark | 2014- | Active (PLDI 2026 paper) | Pure array language for GPUs; incremental flattening |
| Dex | 2019-2024 | Dormant (last commit January 2024) | Typed index sets; `for` builds arrays |
| Fortran `do concurrent` | 2008- | Locality clauses 2018, `reduce` 2023; GPU offload in nvfortran | Asserted, unchecked iteration independence |
| Rayon | 2015- | The standard Rust choice | `par_iter`, `join`; races excluded by `Send`/`Sync` |
| Java parallel streams | 2014- | In the JDK, hedged by its own documentation | `.parallel()` over the common fork-join pool |
| Go | 2009- | No data-parallel construct, by design | Goroutines plus `WaitGroup` or `errgroup` |

## The ideas

### Parallel composition as a block, with checked disjointness (occam)

occam (David May at INMOS, advised by Hoare, 1983) turned CSP's operators into indented blocks: `SEQ` runs
its components in order, `PAR` runs them together and ends when all have ended, `ALT` takes one ready
guarded input; `?` and `!` are channel input and output [1][2].

```occam
PAR
  in ? next
  out ! x * x
```

The "usage in parallel" rules are the part that matters for Sisu. "A variable ... which is assigned to in a
component of a parallel, may not appear in any other component"; a channel may be used "in one component of
the parallel for input, and in one other component of the parallel for output"; array elements may be
assigned in several components only if the compiler can see that the subscripts "select distinct
components", and abbreviations can "decompose an array into non-overlapping disjoint parts". "Programs
violating these rules are deemed illegal as alias and usage checking can usually be performed at compile
time" [1]. That is Rust's and Sisu's exclusivity rule applied to parallel blocks, in 1988, with the
consequence the manual states: "changing the order of the processes combined in a parallel does not change
the effect of that parallel". Nondeterminism enters only through `ALT`: for two ready inputs "the language
does not define which one" is taken [1].

occam died with its hardware, not because of `PAR`. The transputer (announced 1983, T414 in 1985) lost to
cheap microcontrollers and faster RISC chips, the T9000 "consistently failed to reach its own performance
goal", INMOS was sold to SGS-Thomson, and the language and its development environment slowed early
adoption [2]. occam was also small: "Procedures in occam cannot be recursive" [1], and run-time process
creation came only with Kent's occam-pi [2]. Go took CSP's channels and `select`, not the usage rules.

### Structured fork-join (X10, Cilk, Chapel)

X10 (IBM, 2004, part of the PERCS project in DARPA's HPCS program) introduced `async S` (start an activity)
and `finish S` (wait for every activity started inside `S`, transitively), with `at (p) S` to run at a
place, `atomic` and clocks [3]; exceptions from activities propagate to the enclosing `finish`, which
reports them together. That is structured concurrency, failure model included, a decade before the name.
Cilk (MIT, 1994) has `spawn` and `sync` (OpenCilk: `cilk_spawn`, `cilk_sync`, `cilk_scope`), and Chapel
(Cray's Cascade project in the same program) has `begin`, `cobegin` and `sync` blocks [4][8]:

```x10
finish for (i in 0..(n-1)) async { a(i) = f(i); }                       // X10
```
```c
int x = cilk_spawn fib(n - 1); int y = fib(n - 2); cilk_sync; return x + y;   // Cilk
```

Sisu's `task.scope` and `s.spawn` already are `finish` and `async` with Trio's cancellation. What Sisu does
not have is Cilk's cost discipline. The "work-first principle" (Cilk-5, PLDI 1998) puts the overhead of
parallelism on the thief rather than on `spawn`, and randomised work stealing bounds running time by work
over processors plus span (Blumofe and Leiserson) [7]. A Cilk spawn costs a small multiple of a call; a
Sisu task owns a stack (Q-6). Fine-grained recursive parallelism needs a second, lighter mechanism.

X10 stayed a research language; its last release is 2.6.2 (January 2019) [3]. Chapel survived: 2.0 (March
2024) promised that "the core language and library features are designed to be backwards-compatible from
here on", 2.10 shipped in September 2026, the project is hosted by the High Performance Software
Foundation, and its showcase applications are Arkouda (interactive data science at scale), CHAMPS
(multiphysics simulation) and coral-reef image analysis [5]. Chapel's domain maps, which choose how an index
set is distributed without changing the loops over it, are a cousin of Halide's schedules
([`algorithm-schedule-separation.md`](algorithm-schedule-separation.md)).

### "May run in parallel" versus "must run concurrently" (Chapel, Rayon)

Chapel's specification draws a line that mainstream languages blur. A `forall` body "may be executed
concurrently with the others, but this is not guaranteed", so the loop must be correct when serialised; in a
`coforall` "each iteration is guaranteed to run using a distinct task", which is "essential when concurrency
is needed for correctness" [4]. Outer scalars are captured by value in a `forall`, and reductions are
declared on the loop (`with (+ reduce sum)`); but a mutable outer array defaults to `ref`, and the
specification warns that some forms "may introduce undesirable race conditions" [4]. Rayon made the same
"may" choice: `join(a, b)` only permits parallelism ("the decision ... is made dynamically, based on whether
idle cores are available"), and messages between the two closures "will likely deadlock", because `join` is
a hint on a sequential algorithm [16]. Doug Lea's guidance for Java says the same from the failure side:
when subtasks wait, "CPU resources may sit idle" [19]. Sisu's `spawn` is the `coforall` side; Sisu has no
`forall` side.

### Serial elision: annotations that leave the program intact (Cilk, OpenMP, `do concurrent`)

Deleting Cilk's keywords leaves a valid C program, the "serial elision", with the same meaning as the Cilk
program on one processor [8]. OpenMP (1997 for Fortran, 1998 for C and C++; 6.0 in November 2024) [10] built
an industry on the same property: `#pragma omp parallel for reduction(+:sum)` is ignored by a compiler that
does not support it, so parallelism is added loop by loop to a program that stays correct sequentially.
Fortran 2008's `do concurrent` is the in-language version, and nvfortran offloads it to GPUs [15]. This is
the family's strongest evidence about what programmers accept: an annotation on a loop they already have.

It is also the evidence about what unchecked annotations cost. Variables declared outside an OpenMP
parallel region are shared by default, so a forgotten `private` is a data race, and a benchmark suite exists
to measure how well tools find such races (DataRaceBench, SC 2017) [11]. `do concurrent` "tells the compiler
there are no data dependencies between iterations", which the standard neither checks nor requires, so a
wrong assertion is a silent wrong answer [15]. Cilk needed a race detector; OpenCilk advertises "provable
race-detection guarantees that your OpenCilk program has a deterministic result" [9], for the inputs run.
The commercial branch died as a vendor extension: Intel bought Cilk Arts in 2009, shipped Cilk Plus in
2010, deprecated it in September 2017, and GCC 8 removed it ("The Cilk+ extensions to the C and C++
languages have been removed") [8]. MIT's OpenCilk (stable since 2021, 3.0 in May 2025) continues it [8][9].

### Implicit parallelism by default (Fortress)

Fortress (Sun Labs, Guy Steele and others, 2006-2012, also funded by HPCS) made `for` loops parallel by
default, with `for` a replaceable library function, and wrote programs in mathematical notation [6]. It
was not funded for HPCS's third phase (November 2006) and was wound down in July 2012, the stated reason
being the difficulty of implementing its type system on existing virtual machines [6]. Steele's lasting
lesson concerned data, not syntax: his ICFP 2009 keynote ("foldl and foldr considered slightly harmful")
argued that implicit parallelism needs associative combining and divide-and-conquer structures rather than
sequential accumulation [6]. Sisal and Id ([`synchronous-and-dataflow.md`](synchronous-and-dataflow.md))
were the other implicit-parallelism bets; none survived as a default execution model.

### Nested data parallelism and flattening (NESL, Futhark, Dex)

NESL (Blelloch, CMU, 1990s) let parallel operations nest and compiled them by *flattening* nested calls into
flat vector operations, with a work-depth cost model; its project page compares a 10-line NESL quicksort
with a 1,700-line MPI one [12]:

```nesl
function quicksort(S) =
if (#S <= 1) then S
else let a = S[rand(#S)];
         S1 = {e in S | e < a}; S2 = {e in S | e = a}; S3 = {e in S | e > a};
         R = {quicksort(v) : v in [S1, S3]};
     in R[0] ++ S2 ++ R[1];
```

The two recursive calls in `R` are themselves a parallel apply, which is what flattening turns into flat
vector operations.

Flattening never reached a general-purpose compiler: Data Parallel Haskell tried to put it in GHC, and the
Haskell wiki still describes it as work in progress for GHC 7.4 [unverified when it was dropped]. Futhark
(Henriksen, Copenhagen; PLDI 2017) compiles a pure array language to GPUs; because full flattening can cost
memory and overhead for no speed-up, *incremental flattening* (PPoPP 2019) generates several versions of
each nested construct and picks one at run time against thresholds that `futhark autotune` tunes [13]. Dex
(Google Research, 2019-2024) typed arrays by finite index sets (`n=>Float`) and made `for i. e` an array
builder, so a loop's iteration space and the array's shape cannot disagree; its repository has had no
commit since January 2024 [14]. On CPUs, work stealing handles nesting dynamically (a `join` inside a
parallel loop only adds stealable work), which is why the CPU-side languages never needed flattening.

### The library form and what it cannot check (Rayon, Java, Go)

Rayon (2015) is the library form at its best. `v.par_iter().map(f).sum()` is race-free because "Rayon
guarantees you that using Rayon APIs will not introduce data races": closures that share data must be
`Sync`, and two closures cannot both hold `&mut` [16]. What it cannot check: a `Mutex` inside the closure
is race-free but makes the result order-dependent; a parallel floating-point `sum` reassociates; channels
between `join` closures deadlock; and granularity is the user's problem (Matsakis's 1,000-element quicksort
ran at 0.41x before a sequential cutoff) [16].

Java's parallel streams (Java 8, 2014) check nothing. The package documentation shows
`forEach(s -> results.add(s))` into an `ArrayList` and warns that in parallel "the non-thread-safety of
`ArrayList` would cause incorrect results", and that with a stateful lambda "the results for the same input
could vary from run to run" [18]. Lea's guidance asks for `N * Q` (elements times cost per element) of "at
least 10000" before parallelism pays [19]; an ICSE 2019 tool refactors streams to parallel only where that
is safe and profitable [20, results not re-read].

Go has no data-parallel construct, by design: "Concurrency is about dealing with lots of things at once.
Parallelism is about doing lots of things at once." [17]. A parallel loop is goroutines with
`sync.WaitGroup` or `errgroup`, whose `SetLimit` bounds the goroutines but which has no map or for-each
helper [17], over slices the compiler does not check: in Uber's study, 39% of data races involved
implicitly aliased slices ([`go.md`](../languages/go.md)) [21].

## Transferable ideas for Sisu

The central question has two halves. Should the independence check exist? Yes: Sisu's exclusivity and
`Send`/`Sync` rules are occam's usage rules plus Rust's closure rules, and the trust-based alternatives
produce silent wrong answers. Does it need new block syntax? No: `task.scope` with `spawn` already is
`PAR`, `finish`/`async` and `cobegin`. The missing piece is Chapel's `forall`, potential parallelism with
serial-elision semantics. On its shape: occam says keep parallel and sequential forms symmetric (`for x in
xs`, `for x in par xs`); X10 says the statement itself is the join and the place where failures surface;
Chapel says keep "may" apart from "must", make outer variables read-only and declare reductions at the
loop; Cilk says keep serial elision and make the fork cost about a call.

1. **`par` iteration with checked independence.** `par` is a contextual keyword after `in`, in loops and
   comprehensions. It desugars to `std.par`, whose methods take closures that are `pure` and do not mutate
   their captures, so the library form (`xs.par_map(f)`) gets the same check.

   ```sisu
   for p in par mut particles:            # each iteration gets `ref mut` to its own element
       p.vel += gravity * dt               # gravity, dt: shared reads, frozen for the loop
       p.pos += p.vel * dt
   let speeds = [p.vel.norm() for p in par particles]   # same list, same order as without `par`

   var hits = 0
   for line in par lines:
       if word in line:
           hits += 1     # error: `hits` is assigned by every iteration of a `par` loop
                         #   help: let hits = par.count(lines, word in it)
   ```

   The guarantee is serial elision made exact: deleting `par` gives the same result, and a failing loop
   reports the lowest-index error. On `single` and `freestanding` the loop runs serially with the same
   result. Cost: closure kinds (mutating versus non-mutating captures) visible in function types, which the
   proposal does not yet spell; a work-stealing pool in the `full` runtime; diagnostics that name the
   crossing variable, as the `Send`/`Sync` messages do. Conflicts: `pure` must exclude channel, lock and
   atomic operations ("no shared mutation" reads that way). Confidence: medium-high.

2. **`par.join` for divide and conquer (Cilk's `spawn`/`sync`, Rayon's `join`).**

   ```sisu
   def sort[T: Ord + Send](mut xs: Slice[T]):
       if len(xs) <= 4096:
           insertion_sort(mut xs)          # granularity stays the programmer's job
           return
       let mid = partition(mut xs)
       let lo, hi = xs.split_at_mut(mid)   # disjoint views: the checker sees they cannot alias
       par.join(() -> sort(mut lo), () -> sort(mut hi))
   ```

   Cost: one function on the same pool, with the same checks. Conflicts: none. Confidence: medium-high.

3. **Parallelism that cannot block, kept apart from tasks that can.** A `pure` body cannot do I/O, take a
   lock or wait on a channel, so it runs to completion on a worker's stack: no task stack and no parking,
   which keeps fine-grained parallelism out of bet 3's stack budget.

   ```sisu
   for url in par urls:
       let page = http.get(net, url)?   # error: `http.get` uses the capability `net`; a `par` body
                                        #   cannot block. Use task.scope and s.spawn for concurrent I/O.
   ```

   Cost: the runtime either shares OS threads between the scheduler and the pool or keeps two pools (Q-6).
   Conflicts: none. Confidence: high for the distinction, medium for the runtime design.

4. **Reducers with a fixed merge order (Cilk hyperobjects, Chapel `reduce`, OpenMP `reduction`).**
   `par.Sum`, `par.Collect` and user types implementing `par.Reducer` may be updated from a `par` body: each
   worker updates its own view, and views merge in a fixed tree shape, so results are identical across runs
   and core counts. For floats that differs from the sequential left fold, which the documentation must say.
   Cost: a trait and the checker's exception for it. Conflicts: the numerics design (Q-23). Confidence:
   medium.

5. **Granularity tooling.** Adaptive splitting by default (Rayon, Cilk), a `grain=` override, and a
   `sisu bench` warning when a `par` loop is slower than its serial elision (Lea's `N * Q` rule, Rayon's
   0.41x). Cost: small. Conflicts: none. Confidence: medium-low.

## Dead ends

- **Parallel-by-default `for`**: Fortress was cancelled and its designer's lesson was about associative
  data, not loops; Sisal and Id did not survive either.
- **Unchecked parallel annotations**: OpenMP's shared-by-default variables and Fortran's `do concurrent`
  buy silent wrong answers; with ownership the check is available.
- **A `PAR` or `cobegin` keyword**: `task.scope` already is occam's `PAR`, X10's `finish` and Chapel's
  `cobegin`.
- **Flattening in a general-purpose compiler**: Data Parallel Haskell stayed experimental; Futhark needed
  incremental flattening and autotuning, for GPUs; on CPUs work stealing handles nesting.
- **Places, locales and PGAS in the core**: X10's adoption and Chapel's HPC niche;
  [`fortran.md`](../languages/fortran.md) reaches the same conclusion for coarrays.
- **Parallelism as a vendor extension**: Cilk Plus was deprecated by its owner and removed from GCC.

## Sources

1. SGS-Thomson, occam 2.1 Reference Manual, 1995 (first published 1988 as the occam 2 Reference Manual), Appendix E "Usage in parallel" and procedures. http://www.wotug.org/occam/documentation/oc21refman.pdf
2. Wikipedia, "Occam (programming language)" and "Transputer" (dates, T9000, adoption). https://en.wikipedia.org/wiki/Occam_(programming_language) ; https://en.wikipedia.org/wiki/Transputer
3. P. Charles et al., "X10: An Object-Oriented Approach to Non-Uniform Cluster Computing", OOPSLA 2005 (not re-read); Wikipedia, "X10 (programming language)" (PERCS, 2.6.2, January 2019). https://en.wikipedia.org/wiki/X10_(programming_language)
4. Chapel language specification, "Data Parallelism" (forall, coforall, intents). https://chapel-lang.org/docs/language/spec/data-parallelism.html
5. Chapel website (2.10, September 2026; applications; HPSF) and "Announcing Chapel 2.0", 21 March 2024. https://chapel-lang.org/ ; https://chapel-lang.org/blog/posts/announcing-chapel-2.0/
6. Wikipedia, "Fortress (programming language)" (2006-2012, parallel `for`, HPCS funding, July 2012 wind-down). https://en.wikipedia.org/wiki/Fortress_(programming_language) ; G. Steele, "Organizing Functional Code for Parallel Execution; or, foldl and foldr Considered Slightly Harmful", ICFP 2009 keynote (not re-read).
7. M. Frigo, C. Leiserson, K. Randall, "The Implementation of the Cilk-5 Multithreaded Language", PLDI 1998; R. Blumofe, C. Leiserson, "Scheduling Multithreaded Computations by Work Stealing", JACM 46(5), 1999. (Standard citations, not re-read.)
8. Wikipedia, "Cilk" (history, serial elision, hyperobjects, Intel deprecation September 2017). https://en.wikipedia.org/wiki/Cilk ; GCC 8 release notes. https://gcc.gnu.org/gcc-8/changes.html
9. OpenCilk website (OpenCilk 3.0, 13 May 2025). https://www.opencilk.org/
10. OpenMP specifications (1.0 Fortran October 1997 to 6.0 November 2024; 6.1 public comment draft July 2026). https://www.openmp.org/specifications/
11. C. Liao et al., "DataRaceBench: A Benchmark Suite for Systematic Evaluation of Data Race Detection Tools", SC 2017. (Standard citation, not re-read.)
12. NESL project page, CMU (cost model; NESL versus MPI quicksort). https://www.cs.cmu.edu/~scandal/nesl.html ; G. Blelloch, "Programming Parallel Algorithms", CACM 39(3), 1996 (not re-read).
13. Futhark publications (PLDI 2017; "Incremental Flattening for Nested Data Parallelism", PPoPP 2019; size types 2021 and 2023; PLDI 2026). https://futhark-lang.org/publications.html
14. Dex repository (last commit 11 January 2024), https://github.com/google-research/dex-lang ; A. Paszke et al., "Getting to the Point: Index Sets and Parallelism-Preserving Autodiff for Pointful Array Programming", ICFP 2021 (not re-read).
15. Sisu research corpus, Fortran analysis (`do concurrent` semantics; nvfortran GPU results). [`../languages/fortran.md`](../languages/fortran.md)
16. N. Matsakis, "Rayon: data parallelism in Rust", 18 December 2015. https://smallcultfollowing.com/babysteps/blog/2015/12/18/rayon-data-parallelism-in-rust/
17. A. Gerrand, "Concurrency is not parallelism" (Rob Pike's talk), Go blog, 16 January 2013, https://go.dev/blog/waza-talk ; errgroup documentation, https://pkg.go.dev/golang.org/x/sync/errgroup
18. Java SE 21, `java.util.stream` package documentation (side effects, statefulness). https://docs.oracle.com/en/java/javase/21/docs/api/java.base/java/util/stream/package-summary.html
19. D. Lea et al., "When to use parallel streams", draft of 1 September 2014. https://gee.cs.oswego.edu/dl/html/StreamParallelGuidance.html
20. R. Khatchadourian, Y. Tang, M. Bagherzadeh, "Safe Automated Refactoring for Intelligent Parallelization of Java 8 Streams", ICSE 2019. (Not re-read.)
21. Sisu research corpus, Go analysis (Uber data-race study). [`../languages/go.md`](../languages/go.md)
