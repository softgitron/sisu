# Synchronous reactive languages and dataflow

## Why this family matters for Sisu

Sisu already has structured tasks, a capability for every outside source of nondeterminism and a seedable
deterministic scheduler in its interpreter. This family is where determinism of concurrent programs became
a language guarantee that was then certified, flown and measured. It answers whether Sisu should add a
logical-time subset for embedded, robotics and simulation, or whether the pieces it has give most of it.

## Systems covered

| System | Years | Status (Oct 2026) | Central idea |
|---|---|---|---|
| Kahn process networks | 1974 | Theory under every stream system | Deterministic processes with blocking FIFO reads form a deterministic network |
| Lucid | 1976-1985 | Historical; lives on in Lustre | Variables are infinite streams; `fby` ("followed by") |
| Esterel | 1982- | Survives in SCADE 6 state machines, HipHop.js (2020), Blech | Perfect synchrony: zero-time reactions, instantaneous broadcast, preemption |
| Lustre, SCADE | 1984-87; SCADE 1990s- | SCADE Suite (Synopsys since 2025); KCG qualified to DO-178B/C Level A | A program is a clocked dataflow network compiled to one step function |
| Signal | 1982- | Research toolset; commercialised as Sildex (TNI, 1993) | Multiclock ("polychronous") relations between signals |
| Sisal; Id and Monsoon | 1983-1996; 1980s-1991 | Historical | Single assignment and implicit parallelism on dataflow hardware |
| SDF, StreamIt | 1987; 2002-2010 | StreamIt historical | Fixed token rates give static schedules and buffer bounds |
| Simulink | 1990s- | Dominant in control engineering | Block diagrams with sample times |
| Faust | 2002- | Active (2.83.1, Dec 2025) | Block-diagram algebra for audio, compiled to C++, C, LLVM, Wasm, Rust |
| Céu | 2011-2018 | Academic | Esterel's control constructs in a C-like language: `par/or`, `par/and`, `await` |
| ReactiveX | 2009- | Mature; Java moving to virtual threads | Observables: asynchronous push streams with operators |
| Elm | 2012- | 0.19.1 (2019) current; FRP removed in 0.17 (2016) | Model-update-view instead of signals |
| Frontend signals | 2018- | Solid, Preact, Angular, Svelte 5; TC39 proposal at Stage 1 since April 2024 | Glitch-free derived state; scheduling left to frameworks |
| Lingua Franca | 2019- | 0.13.0 (Aug 2026); C, C++, Python, TypeScript, Rust targets | Reactors with logical time: deterministic concurrency and distribution |

## The ideas

### Logical instants: perfect synchrony (Esterel)

Esterel (Berry, Sophia-Antipolis, begun 1982) runs a program as a sequence of *reactions*: inputs are
sampled at the start of a logical instant, outputs are computed in zero logical time, and parallel branches
see each other's emitted signals within the instant. Berry's ABRO (emit `O` once `A` and `B` have both
arrived; restart on `R`) grows linearly where the equivalent automaton grows exponentially [1]:

```esterel
loop
  [ await A || await B ];
  emit O
each R
```

The contract is "specify with zero-delay, implement with predictable delay": "WCET = guarantee of
no-overlap" between reactions [1]. **What it buys:** deterministic parallel composition and a clean time
algebra ("`delay 1; delay 2` exactly equals `delay 3`"). The designers' 2003 retrospective says the model
"has remained valid and was never questioned" and that code needing no operating-system scheduler "was
particularly attractive to certification authorities"; at Texas Instruments, Esterel models of DSP control
logic showed existing test suites exercised "only about 30%" of the full state space [2].

**Where it failed.** Instantaneous broadcast makes a reaction a fixpoint equation that may have no solution
or several; the designers call the hunt for a sound causality analysis "like a Mary Higgins Clark novel:
each time the right solution was reported found, a flaw in it was reported soon thereafter" [2]. The
zero-delay hypothesis bars long computations [10]. Esterel lives on in SCADE 6's state machines [3],
HipHop.js (PLDI 2020) and Bosch Research's Blech (site last updated 2022) [4].

### A program is a clocked dataflow network (Lustre, SCADE, Signal, Lucid)

Lustre (Caspi, Halbwachs and others, Grenoble; POPL 1987 [5]) is the dataflow face of the same model: every
variable is an infinite stream, a program is a set of equations, `pre` is the previous instant's value and
`->` gives the initial one. The counter from Berry's SCADE lectures [1]:

```lustre
node Counter(event: bool) returns (count: int);
let
  count = 0 -> (if event then pre(count) + 1 else pre(count));
tel
```

A node compiles to a *step function* over a static memory record, called once per tick by a loop the
integrator writes. Slower activity is a sub-clock (`x when c`, `current`) checked by a clock calculus;
Signal (Le Guernic and Benveniste, Rennes, 1982) generalises this to "multiclock" programs in which each
signal has its own clock [2]; Lucid's `fby` (Ashcroft and Wadge, 1976-77) is the ancestor [6].

**The certification argument.** SCADE descends from two in-house tools built for the A320 and the N4
nuclear plants (SAO at Aerospatiale; SAGA, on Lustre, at Merlin-Gerin); Esterel Technologies bought it in
2001 [2]. Its generator KCG checks, "as a prerequisite to code generation", that every flow is defined
exactly once, that no undefined data is read, that there is no combinational cycle ("=> deterministic
scheduling-independent result") and no recursion ("=> static memory allocation, bounded stack"). The C it
emits has "no recursion, no loop" and "bounded execution time", which eases analysis with Astrée and
AbsInt's tools [1]. KCG was developed under a DO-178B Level A process and qualified as a development tool,
so there is "no need to unit-test the generated C code" [1]; it is qualifiable as a DO-330 TQL-1 tool under
DO-178C [7]. Timing is deterministic for a plain reason: the program is one bounded step function per
period, its worst case is a static-analysis result, and the period is chosen above it.

**Adoption.** Esterel Technologies listed SCADE in the A380's flight control, flight warning, electrical
load management, anti-icing, braking and steering, cockpit display and FADEC software, and in the Paris
Météor metro line (EN 50128, up to SIL 4) [1]; the 2003 survey adds French nuclear protection and the Hong
Kong subway [2]. Ansys bought Esterel Technologies in 2012, Synopsys bought Ansys in 2025 [8]. **Where it
stayed niche:** the product is the qualified toolchain and graphical editor, sold to control engineers who
would otherwise draw Simulink and Stateflow. No textual general-purpose language adopted clocks; Lucid
Synchrone, an ML over streams (version 3, 2006), stayed in research [9].

### Structured preemption: `par/or`, `par/and`, `await` (Céu)

Céu (Sant'Anna, Ierusalimschy and others, PUC-Rio, 2011-2018) puts Esterel's control constructs into a
C-like language for microcontrollers. Trails run until they `await` an event; `par/and` "terminates if all
sides end", `par/or` "aborts if any side ends", and `finalize` runs however a trail ends. The paper's
example toggles an LED on each radio packet and stops, LED off, on a button press [10]:

```ceu
par/or do
    await BUTTON;
with
    finalize with _led(0); end
    loop do
        _led(1); await RADIO_RECV;
        _led(0); await RADIO_RECV;
    end
end
```

Each external event is one reaction; trails run in lexical order; every path through a loop must `await` an
external event (checked), so every reaction terminates. LCTES 2018 proves reactions "memory-bounded,
deterministic, and terminating" [10]. SenSys 2013 measured "around 25%" less code complexity than nesC on
sensor-network protocols, with ROM and RAM "below 10%" higher [11]; a case study rewrote the Pingus game
from C++ [12]. "Immediate orthogonal abortion" "cannot be expressed effectively in traditional
(asynchronous) multi-threaded languages" [10]. It stayed with one research group. Sisu's scope is already
`par/and`, cancellation is the abort and `defer`/`Drop` are `finalize`; a named `par/or` is missing.

### Deterministic composition and the delay rule (Kahn, SDF, StreamIt, Faust, Simulink)

Kahn (1974) showed that deterministic sequential processes joined by FIFO channels, where reads block and
no process may test a channel for emptiness, compute a deterministic function of their inputs under any
schedule [13]. The extension that breaks it is the merge: occam's `ALT` takes one ready input and "the
language does not define which one" [14]; Go's and Sisu's `select` are the same construct. Synchronous
dataflow (Lee and Messerschmitt, 1987 [13]) fixes token rates, so schedules and buffer sizes are computed
at compile time; StreamIt (MIT, CC 2002 [15]) gave it filters with declared `push`, `pop` and `peek` rates.
A study of 65 StreamIt programs (33,600 lines, 22 programmers) by its designers measured the cost [16]:
single-input single-output streams were "useful and tractable" but "occasionally unnatural"; a quarter of
the programs needed stateful filters; dynamic rates were "absolutely necessary" for the 9 that used them
(JPEG and MPEG parsing); and programmers "can accidentally introduce unnecessary sequential bottlenecks
(mutable state)".

Faust (GRAME, 2002) is the static model's success: a block-diagram algebra for audio (`:` sequence, `,`
parallel, `<:` split, `:>` merge, `~` feedback) compiled to C++, C, LLVM, WebAssembly and Rust, with
integrations in Wwise, Godot and HISE [17]. A one-pole smoother is `smooth(c) = *(1 - c) : + ~ *(c);`,
where `~` feeds the output back one sample later.

These systems share one rule: **every cycle contains a delay**. Lustre "solves this trivially by forbidding
delay-free loops" [2], Faust's `~` inserts a sample, StreamIt's feedback loops are primed with initial items
and Lingua Franca needs an `after` or a microstep. The rule buys determinism without Esterel's causality
analysis. Simulink's discrete-time block diagrams are synchronous dataflow in all but name; Sildex imports
them and SCADE 6 positions itself as the alternative to Simulink plus Stateflow [2][9].

### Dataflow as the execution model (Id, Sisal)

Arvind's Id on MIT's Monsoon machine (1988-91) and LLNL's Sisal (1983) bet that single-assignment,
implicitly parallel programs would run on dataflow hardware. Arvind's 2005 ISCA keynote is the designer
retrospective [18]: Id programs "scaled effortlessly", but on one processor Monsoon "took 2 to 3 times as
many cycles as Fortran/C on a modern workstation"; the "invisible software (loaders, simulators, I/O
libraries, ...)" dominated the effort; and "It is difficult enough to cause one revolution but two?". His
lesson for implicit parallelism: "Dataflow model gives you parallelism for free, but requires analysis to
get locality; C (mostly) provides locality for free but one must do analysis to get parallelism." Sisal
matched Fortran on early-1990s shared-memory machines (Cann, CACM 1992), could not follow the move to
distributed memory, and was frozen in 1996 [19]. The ideas outlived the machines: single assignment (SSA,
pure array languages), I-structures (write-once futures), dataflow scheduling inside out-of-order cores.

### Streams and signals in mainstream languages (Rx, Elm, frontend signals)

ReactiveX (Rx.NET, then Netflix's RxJava in 2013) made event streams first-class: an Observable is "the
asynchronous/push 'dual' to the synchronous/pull Iterable" [20]. Rx has no logical time; `merge`
interleaves by arrival. Java's verdict on the style is JEP 444: "stack traces provide no usable context,
debuggers cannot step through request-handling logic", while virtual threads give the same scalability
"transparently" [21]; Sisu has taken JEP 444's side. Elm removed signals in 0.17 (2016) because "all the
toughest concepts in Elm (signals, addresses, and ports) could collapse into simpler concepts", and "They
were not necessary" [22]. The Elm Architecture that replaced them is a degenerate synchronous program: each
message is an instant, `update` the step function, the model the state.

The frontend signals wave kept the dataflow half and dropped the time half. Solid, Preact Signals, Angular
(preview in v16, 2023; `effect`, `linkedSignal` and `toSignal` stable in v20, 2025) and Svelte 5's runes
(22 October 2024) propagate derived state glitch-free [23]. The TC39 proposal (`new Signal.State(0)`, `new
Signal.Computed(() => ...)`), written with input from the maintainers of Angular, Ember, MobX, Preact,
Qwik, RxJS, Solid, Svelte, Vue and Wiz, requires computed signals to be "glitch-free" and "lazy", includes
no `effect`, leaves scheduling to frameworks, and has been at Stage 1 since April 2024 [24]. Glitch-freedom
is the synchronous hypothesis applied to a user interface; it works as a library in every framework that
has it, and standardising it is slow.

### Logical time across threads and machines (Lingua Franca)

Lingua Franca (Lohstroh, Lee and others at Berkeley, TU Dresden and Kiel; reactor model 2019, language paper
2021 [25]) is a coordination language: reactors with ports, timers and state, whose reaction bodies are C,
C++, Python, TypeScript or Rust [26]. Every event carries a tag (logical time plus microstep); "logical time
does not advance while a reaction runs" [26]; reactions of one reactor run in declaration order, dependent
reactions in an acyclic precedence order, and "independent reactions may be executed in parallel without
breaking determinism" [27].

```lf
reactor Sensor {
  output y: int
  timer t(0, 10 ms)
  reaction(t) -> y {= lf_set(y, read_adc()); =}
}
main reactor {
  s = new Sensor(); c = new Controller()
  s.y -> c.x after 1 ms    // c sees the value at logical time t + 1 ms
}
```

Programs are "deterministic by default", and "physical actions provide a means for deliberately introducing
actor-like nondeterminism" (inputs stamped with physical time) [27]; deadlines bound the lag between
logical and physical time, and federated programs keep the semantics across machines [26]. On 22 of the 30
Savina benchmarks the C++ runtime was about 1.85x faster than Akka and 1.42x faster than CAF (geometric
mean, 20 threads); dynamic actor creation was unsupported (8 benchmarks omitted) [27]. A June 2026 paper
runs unmodified ROS 2 applications under LF: on the Autoware reference system, default ROS 2 callback order
differed between runs and LF's was deterministic, with consistent latencies [28]. Version 0.13.0 shipped in
August 2026 [26]; Xronos (2023, Lohstroh and Menard) is the commercial arm [29, secondary]; other use is small.

### Determinism by construction, or by simulation (Lee, FoundationDB)

Lee's "The Problem with Threads" (2006) argued that threads "discard the most essential and appealing
properties of sequential computation: understandability, predictability, and determinism", citing reviewed
and tested Ptolemy II code whose deadlock appeared "on April 26, 2004, four years later" [30]. The
synchronous languages, LF, Deterministic Parallel Java and LVars [32] get determinism by construction;
FoundationDB gets it by simulation: "All database code is deterministic; accordingly multithreaded
concurrency is avoided (instead, one database node is deployed per core)", and "all sources of
nondeterminism and communication are abstracted, including network, disk, time, and pseudo random number
generator", so a cluster under fault injection runs in one process [31]. It is the strongest measured
outcome in this file: CloudKit ran it "for more than 0.5M disk years without a single data corruption
event", and without it ambitious rewrites "would have been deemed too risky or too difficult" [31]. Sisu is
halfway there: nondeterminism enters only through capabilities, test clocks advance "only when every task
in the test is blocked" ([`17-testing.md`](../../proposal/17-testing.md)), and the Phase 6 interpreter
schedules from a seed.

## Transferable ideas for Sisu

The answer to the central question is no language mode (it would bring Esterel's causality problem and the
zero-delay limit) but yes to logical time as a runtime feature and a library, built from parts Sisu has.

1. **Deterministic simulation of whole programs.** A `std.sim` world runs several programs as nodes in one
   process on the `single` scheduler, each with a simulated `System` (`Net`, `Fs`, a discrete-event
   `Clock`, seeded `Rand`), injects faults and replays any failing seed exactly.

   ```sisu
   test "replicas converge under partitions" for seed in range(500):
       let world = sim.World(seed=seed)               # one OS thread, logical time
       let nodes = [world.node("n{i}") for i in range(3)]
       for n in nodes:
           world.start(n, sys -> kv.serve(sys))       # each node gets its own System
       world.inject(sim.Partition(rate=0.02), sim.DiskStall(rate=0.01))
       world.run_for(Duration.minutes(30))?           # seconds of wall time
       assert kv.replicas_agree(nodes)
   ```

   Cost: the Phase 6 scheduler in the native runtime (it is the `single` profile), simulated `Net` and `Fs`
   with fault models, and nothing nondeterministic outside a capability (seeded hash orders). Conflicts:
   none; it scales up the test-clock rule, though `extern` code must sit behind a capability and the M:N
   `full` runtime is not simulated (FoundationDB avoids threads too). Confidence: high.

2. **`task.race`: Céu's `par/or` over a scope.** The first branch to finish wins; the others get
   `Cancelled` at their next blocking point and run their `defer` and `Drop`, which is `finalize`.

   ```sisu
   let reply = task.race do r:
       r.spawn(() -> fetch(net, primary, key))
       r.spawn(() -> fetch(net, replica, key))
   ```

   Cost: a library function over `task.scope` and `s.cancel()`. Conflicts: none; under the deterministic
   scheduler the winner is a function of the seed. Confidence: medium-high (`Promise.race`, Kotlin
   `select`).

3. **`std.x.reactor`: Lingua Franca's logical time as a library.** LF's guarantee needs reactions that
   communicate only through ports, an order within a tag, and one source of nondeterminism. A `pure`
   reaction cannot block, read the clock or touch shared state; the graph supplies the order; capabilities
   are the physical inputs. A reaction is a `pure def react(mut self, x: In) -> Out?` method; on
   `freestanding` it can also be `@bounded` (no recursion, no allocation, bounded loops, as KCG checks).

   ```sisu
   def control(imu: Imu, motor: Motor, clock: Clock) -> ! RobotError:
       var g = reactor.Graph()                         # import std.x.reactor
       let accel = g.physical(imu)                     # stamped on arrival: the only nondeterminism
       let smooth = g.add(Smoother(alpha=0.2), accel)
       let cmd = g.add(Pid(kp=0.8, ki=0.1), smooth, after=Duration.millis(1))
       g.output(cmd, motor, deadline=Duration.millis(2))
       g.check()?                                      # a cycle without a delay fails here
       return g.run(clock)                             # g.replay(trace) in tests
   ```

   Independent reactions at one tag can run on the `par` pool of
   [`parallel-languages.md`](parallel-languages.md). Cost: a library (graph, tag-ordered queue, deadlines,
   replay); `check` runs at start-up unless the graph is built at `comptime`. Conflicts: assumes `pure`
   admits an exclusive `mut self` ("no shared mutation" suggests so). Confidence: medium (semantics and
   benchmarks are solid; demand beyond robotics and simulation is unproven).

4. **A checked deterministic scope (Kahn's conditions).** Tasks that talk only through single-producer
   single-consumer channels with blocking operations, share only immutable data and hold no
   nondeterministic capability give the same result on any schedule, M:N included. Ownership makes this
   checkable: a channel end that is moved and never copied has one user.

   ```sisu
   @deterministic                          # the error names what breaks it: select, Mutex, Clock...
   def transcode(frames: list[Frame]) -> list[Packet] ! PipeError:
       let send, recv = channel.spsc[Frame](capacity=8)    # neither end can be copied
       return task.scope do s:
           s.spawn(own () -> send.send_all(own frames))
           return Ok([encode(f) for f in recv])             # blocking receives only
   ```

   Cost: a second inferred property beside `pure` (no `select`, `try_*`, locks, atomics, copied senders or
   nondeterministic capabilities, transitively); the scope must report the earliest-spawned failure, not
   the first in time. Conflicts: Q-17 keeps `pure` as the one tracked property. Confidence: low-medium.

## Dead ends

- **Instantaneous broadcast with constructive causality in the core**: the designers' own account of the
  causality hunt; Lustre's delay rule gets determinism without it.
- **Clocks (`when`, `current`) in a general-purpose type system**: their value is tied to a qualified
  toolchain, and the textual transplant (Lucid Synchrone) stayed in research.
- **First-class FRP signals**: Elm removed them as unnecessary; frontend signals thrive as libraries, and
  the TC39 proposal has not left Stage 1 in two and a half years.
- **Rx-style streams as the concurrency model**: JEP 444's critique; `merge` is nondeterministic.
- **Dataflow evaluation by default, static token rates in types**: Arvind's two revolutions and locality
  lesson; StreamIt's own programs needed dynamic rates and mutable state.

## Sources

1. G. Berry, Esterel and SCADE lectures, Marktoberdorf 2008 (Esterel Technologies slides). https://asimod.in.tum.de/2008/Berry.pdf
2. A. Benveniste, P. Caspi, S. Edwards, N. Halbwachs, P. Le Guernic, R. de Simone, "The Synchronous Languages 12 Years Later", Proc. IEEE 91(1), 2003. https://www.cs.columbia.edu/~sedwards/papers/benveniste2003synchronous.pdf
3. B. Pagano, "Le langage SCADE 6 pour les systèmes embarqués", Collège de France seminar, 23 April 2013. https://www.college-de-france.fr/fr/agenda/seminaire/le-temps-et-les-evenements-en-informatique/le-langage-scade-6-pour-les-systemes-embarques-de-la-conception-la-compilation
4. G. Berry, M. Serrano, "HipHop.js: (A)Synchronous Reactive Web Programming", PLDI 2020, https://pldi20.sigplan.org/details/pldi-2020-papers/24/HipHop-js-A-Synchronous-Reactive-Web-Programming ; Blech language site (2019-2022), https://www.blech-lang.org/
5. P. Caspi, D. Pilaud, N. Halbwachs, J. Plaice, "LUSTRE: A Declarative Language for Programming Synchronous Systems", POPL 1987. https://archive.air.in.tum.de/pub/Main/TeachingWs2009Echtzeitsysteme/p178-caspi.pdf
6. E. Ashcroft, W. Wadge, "Lucid, a Nonprocedural Language with Iteration", CACM 20(7), 1977. https://cacmb4.acm.org/magazines/1977/7/11332-lucid-a-nonprocedural-language-with-iteration/abstract
7. Ansys SCADE Suite catalogue (KCG qualifiable to DO-178B Level A and DO-330 TQL-1). https://pdf.directindustry.com/pdf/ansys/ansys-scade-suite-170/9123-682958.html
8. Ansys 10-K for 2012 (Esterel acquisition completed 1 August 2012), https://www.sec.gov/Archives/edgar/data/1013462/000101346213000003/anss2012123110k.htm ; Synopsys, "Synopsys Completes Acquisition of Ansys", 17 July 2025, https://news.synopsys.com/2025-07-17-Synopsys-Completes-Acquisition-of-Ansys
9. M. Pouzet, Lucid Synchrone page (version 3.0, 2006), https://www.di.ens.fr/~pouzet/lucid-synchrone/ ; J.-L. Colaço, B. Pagano, M. Pouzet, "A Conservative Extension of Synchronous Data-flow with State Machines", EMSOFT 2005, https://www.di.ens.fr/~pouzet/bib/emsoft05b.pdf
10. R. Santos, G. Lima, F. Sant'Anna, R. Ierusalimschy, E. Haeusler, "A Memory-Bounded, Deterministic and Terminating Semantics for the Synchronous Programming Language Céu", LCTES 2018. https://www.ceu-lang.org/chico/ceu_lctes18_pre.pdf
11. F. Sant'Anna, N. Rodriguez, R. Ierusalimschy, O. Landsiedel, P. Tsigas, "Safe System-level Concurrency on Resource-Constrained Nodes", SenSys 2013. https://www.ceu-lang.org/chico/ceu_sensys13_pre.pdf
12. Céu publications list (TECS 2017; Pingus case study, SBGames 2018). https://www.ceu-lang.org/publications.html
13. G. Kahn, "The Semantics of a Simple Language for Parallel Programming", IFIP Congress 1974; E. A. Lee, D. G. Messerschmitt, "Synchronous Data Flow", Proc. IEEE 75(9), 1987. (Standard citations, not re-read for this file.)
14. SGS-Thomson, occam 2.1 Reference Manual, 1995 (first published 1988), section on alternation. http://www.wotug.org/occam/documentation/oc21refman.pdf
15. W. Thies, M. Karczmarek, S. Amarasinghe, "StreamIt: A Language for Streaming Applications", CC 2002. https://commit.csail.mit.edu/papers/2002/streamit-cc.pdf
16. W. Thies, S. Amarasinghe, "An Empirical Characterization of Stream Programs and its Implications for Language and Compiler Design", PACT 2010. https://commit.csail.mit.edu/papers/2010/thies-pact10.pdf
17. Faust project site (2026) and release history (2.83.1, December 2025). https://faust.grame.fr/ ; https://en.wikipedia.org/wiki/FAUST_(programming_language)
18. Arvind, "Dataflow: Passing the Token", ISCA 2005 keynote. https://csg.csail.mit.edu/Users/arvind/ISCAfinal.pdf
19. D. Cann, "Retire Fortran? A Debate Rekindled", CACM 35(8), 1992, https://www.deepdyve.com/lp/association-for-computing-machinery/retire-fortran-a-debate-rekindled-PKdmbAIIaD ; P. Miller, "Sisal Lives", https://sisal.sourceforge.net
20. ReactiveX, "Introduction". https://reactivex.io/intro.html
21. JEP 444, "Virtual Threads" (Java 21, 2023). https://openjdk.org/jeps/444
22. E. Czaplicki, "A Farewell to FRP", 10 May 2016. https://elm-lang.org/news/farewell-to-frp
23. Angular `linkedSignal` reference (stable since v20), https://angular.dev/api/core/linkedSignal ; LogRocket, "Angular v20 update" (2025), https://blog.logrocket.com/angular-v20-update/ ; Svelte team, "Svelte 5 is alive", 22 October 2024, https://svelte.dev/blog/svelte-5-is-alive
24. TC39 Signals proposal README (Stage 1; April 2024 agenda). https://github.com/tc39/proposal-signals
25. M. Lohstroh et al., "Reactors: A Deterministic Model for Composable Reactive Systems", CyPhy 2019 (not re-read); M. Lohstroh, C. Menard, S. Bateni, E. A. Lee, "Toward a Lingua Franca for Deterministic Concurrent Systems", ACM TECS 20(4), 2021. https://par.nsf.gov/biblio/10602597-toward-lingua-franca-deterministic-concurrent-systems
26. Lingua Franca website, "Time and Timers" documentation and releases (0.13.0, 16 August 2026). https://www.lf-lang.org/ ; https://www.lf-lang.org/docs/writing-reactors/time-and-timers ; https://github.com/lf-lang/lingua-franca/releases
27. C. Menard et al., "High-Performance Deterministic Concurrency Using Lingua Franca", ACM TACO 2023. https://arxiv.org/abs/2301.02444
28. H. Teper, S. Lin, S. Li, E. A. Lee, J.-J. Chen, "Deterministic Execution of ROS 2 Applications via Lingua Franca", arXiv:2606.09203, June 2026. https://arxiv.org/abs/2606.09203
29. Startup directory entry for Xronos (founders, 2023; secondary source). https://startup-seeker.com/company/xronos~com
30. E. A. Lee, "The Problem with Threads", UC Berkeley EECS-2006-1 and IEEE Computer 39(5), 2006. https://www2.eecs.berkeley.edu/Pubs/TechRpts/2006/EECS-2006-1.pdf
31. J. Zhou et al., "FoundationDB: A Distributed Unbundled Transactional Key Value Store", SIGMOD 2021. https://www.foundationdb.org/files/fdb-paper.pdf
32. R. Bocchino et al., "A Type and Effect System for Deterministic Parallel Java", OOPSLA 2009; L. Kuper, R. Newton, "LVars: Lattice-based Data Structures for Deterministic Parallelism", FHPC 2013. (Standard citations, not re-read.)
