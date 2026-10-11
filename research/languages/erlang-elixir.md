# Erlang/OTP and Elixir

## Snapshot
| | |
|---|---|
| First appeared | Erlang: 1986 (Ericsson Computer Science Lab); open-sourced December 1998. Elixir: 2012 (first public release 25 May 2012; 1.0 September 2014). |
| Designed by | Erlang: Joe Armstrong, Robert Virding, Mike Williams (Ericsson). OTP: Armstrong, Magnus Fröberg, Martin Björklund and the OTP team. Elixir: José Valim. |
| Current version (Oct 2026) | Erlang/OTP 29 (29.0 released 13 May 2026; 29.1.x patch line by September 2026). Elixir 1.20 (1.20.0 released 3 June 2026; 1.20.4 patch line by October 2026), requires OTP 27+. |
| Governance | Erlang/OTP: Ericsson's OTP team owns the release; EEP (Erlang Enhancement Proposal) process; Erlang Ecosystem Foundation (EEF, 2019) funds ecosystem work and is a CVE Numbering Authority. Elixir: José Valim (BDFL) with a small core team; Dashbit; sponsors (CNRS/Remote for the type system; Fresha and Tidewave in 2026). |
| Typing | Dynamic, strong. Erlang: optional typespecs checked by Dialyzer (success typings). Elixir: gradual set-theoretic types inferred by the compiler since 1.17-1.20; no user annotations yet. |
| Memory | Per-process generational copying GC; off-heap reference-counted large binaries; message passing copies data. |
| Compilation | Source -> Core Erlang -> BEAM bytecode, run by the BEAM VM with a JIT (BeamAsm, since OTP 24). Elixir compiles to Erlang abstract forms, then BEAM. |
| Primary domains | Telecoms, messaging, soft-real-time servers, web (Phoenix), embedded (Nerves), distributed databases (Riak, CouchDB), message brokers (RabbitMQ). |
| Notable users | Ericsson (AXD301, GPRS/LTE nodes), WhatsApp, Discord, RabbitMQ, ejabberd, Cisco, Klarna, Pinterest, PepsiCo, Nintendo (per Wikipedia), Remote, Fly.io. |

## Stated design goals

Erlang was built to solve one problem: telecom switches that "despite careful
testing will probably contain many errors when the program is put into
service. We assume that such programs do contain errors, and investigate
methods for building reliable systems despite such errors" (Armstrong's 2003
thesis abstract [1]). The thesis derives six requirements for any system that
is to be fault-tolerant: R1 concurrency ("no penalty for creating large
numbers of concurrent processes"), R2 error encapsulation ("errors occurring
in one process must not be able to damage other processes"), R3 fault
detection (locally and remotely), R4 fault identification, R5 code upgrade
("change code as it is executing, and without stopping the system"), R6 stable
storage. Armstrong argues R1-R5 are satisfied by the language and R6 by the
libraries (Mnesia/dets). The programming model is "concurrency-oriented
programming": processes are the unit of modularity, failure and isolation,
modelled on how the physical world works.

Elixir's stated goal was to "enable higher extensibility and productivity in
the Erlang VM while maintaining compatibility with Erlang's ecosystem" [15].
Valim came from Rails; the design bet was that the BEAM's runtime was the
asset and that a modern surface (macros, tooling, a Ruby-flavoured syntax,
first-class documentation) would make it adoptable outside telecoms.

## Strengths

### "Let it crash": isolation plus supervision as the error-handling model
The feature: processes share nothing, communicate only by asynchronous
messages, and are wired together by bidirectional *links* and unidirectional
*monitors*. When a process dies, linked processes receive an exit signal and
die too unless they trap exits; supervisors (OTP `supervisor` behaviour) trap
exits and restart children according to a strategy (`one_for_one`,
`one_for_all`, `rest_for_one`) and a restart intensity (max restarts per
period, after which the supervisor itself dies and escalates). This is the
*opposite* of defensive programming. Armstrong's definition: "exceptions occur
when the run-time system does not know what to do; errors occur when the
programmer doesn't know what to do" and in that case "the program should just
crash" [1, §4.4]. He shows that writing a catch-all clause `asm(X) ->
exit(...)` is pointless because the compiler already generates an equivalent
`function_clause` error, and "the defensive code detracts from the pure case
and confuses the reader - the diagnostic is often no better than the
diagnostic which the compiler supplies automatically". The thesis explicitly
ties this to Jim Gray's fail-fast modules and Candea and Fox's "crash-only
software" [1, §2].

Evidence it works: the AXD301 ATM switch (over 1.1 million lines of Erlang in
the December 2001 snapshot analysed in the thesis, 1.7 million by 2003) was
"thought to be one of the most reliable products ever made by Ericsson" [1].
The model has since been copied outside Erlang (Akka supervision, Pony,
Orleans) and echoed by container orchestrators' restart policies, which is
the strongest form of adoption evidence. The State of Elixir 2025 survey (1,018 respondents) has
57.8% rating "improved system reliability / uptime" as a 5/5 business impact
[12].

What the evidence does *not* support is the "nine nines" number, see
Weaknesses.

### Cheap, isolated, preemptively scheduled processes with per-process GC
A new process costs 327 words of memory, of which 233 are heap and stack
[2]; millions per node are routine. Each process has its own heap collected
by a generational copying collector; a collection stops only that process,
not the VM [3]. Large binaries (over 64 bytes) live off-heap and are
reference counted, so they are shared rather than copied in messages [3].
This is the practical answer to global-GC pause problems: no process can
cause a pause for another, and a process that allocates garbage pays for its
own collection. The scheduler is preemptive by *reductions*: a process is
descheduled after roughly 4,000 reductions (`CONTEXT_REDS`, raised from 2,000
in OTP 20) [4], where a reduction is approximately one function call, so a
runaway loop cannot starve the node. Multi-core support is one scheduler per
core with work stealing (SMP since 2006, R11B). WhatsApp reported 2 million
simultaneous TCP connections on one FreeBSD box in 2012 [6], and Discord ran
roughly 5 million concurrent users on Elixir in 2017 [7].

Preemption is only possible because the compiler controls every loop (there
are no loops, only tail calls, and every call is a preemption point) and
because no shared mutable memory can be left half-updated when a process is
suspended or killed. Messages are matched with *selective receive*: `receive`
takes the first mailbox message matching a pattern and leaves the rest, which
is how request/response (`receive {Ref, Reply} -> Reply end`) and
`gen_server:call` are written in user code.

### Location-transparent distribution built into the runtime
A pid identifies a process on any connected node; `Pid ! Msg`, `link`,
`monitor` and `spawn(Node, ...)` work identically across the cluster [5].
Nodes connect transitively into a full mesh by default; distribution
protocol, node discovery (epmd), cookies and `global` name registration are
in the standard distribution. This is the feature that let WhatsApp run with
a small engineering team and that Phoenix PubSub/Presence build on without
Redis. It is also the origin of the "nodes" abstraction later imitated by
Akka Cluster and Orleans.

### Immutability everywhere, with single-assignment variables
All Erlang terms are immutable; variables are bound once per scope (Elixir
allows rebinding of a name, but the value is still immutable). There is no
`null`; absence is a tagged tuple or atom. Together with share-nothing
processes this is why there are no data races in Erlang programs, by
construction, without a borrow checker. Armstrong's thesis measured that in
the AXD301 code base, by his (deliberately simple) definition, 92% of
functions were side-effect free and fewer than four "dirty" functions per
thousand lines of code [1, §8.3].

### Hot code loading as a first-class runtime capability
Each module can exist in two versions, *current* and *old*; loading a third
purges the old one and kills any process still executing it; a process
switches version only by making a fully-qualified call `m:f(...)` [8]. OTP
layers release handling (`appup`/`relup`, `code_change/3` in behaviours) on
top. This has been used for decades in Ericsson gear and was part of
WhatsApp's operations [6]. It is also the mechanism behind the fast REPL
feedback loop (recompile one module in a running system) even for teams that
never do production hot upgrades.

### Behaviours: interfaces for the hard part, written once
A behaviour (`gen_server`, `gen_statem`, `supervisor`, `gen_event`) is a
module that implements the generic, concurrency-aware part of a pattern
(message loop, synchronous call with timeout and monitor, system messages for
debugging, code upgrade) and dispatches to callbacks (`init/1`,
`handle_call/3`, `handle_cast/2`, `handle_info/2`, `terminate/2`,
`code_change/3`). The thesis derives `gen_server` by stepwise refinement from
a 30-line universal server, then adds fault tolerance and then hot upgrade
[1, §4]. A `-behaviour(gen_server).` attribute makes the compiler check that
the callbacks exist. It is a typed-interface-shaped contract in a dynamic
language, and it is the reason "Erlang programmers all write the same kind of
server". Gleam's `gleam_otp` and Akka's typed actors show that the pattern
transplants to static typing only partially (selective receive and
`handle_info` catch-alls resist typing).



### Elixir's macros and tooling: the adoption lever
Elixir's macros are hygienic by default and operate on a quoted AST
(`quote`/`unquote`, three-element tuples), with explicit escape hatches
(`var!`, `Macro.escape`). The entire surface of Phoenix routing, Ecto queries,
ExUnit assertions (which print the expression and the values on failure) and
`defstruct` is ordinary macros. The tooling stack shipped early and in one
place: `mix` (build, deps, tasks), Hex (package registry, 2014; 26,484
packages and about 4.07 million downloads per day as of October 2026 [16]),
ExDoc (documentation is a first-class language construct via `@doc`/
`@moduledoc` with runnable doctests), `mix format` (1.6, 2018) and ExUnit. In
the 2025 Curiosum survey, `mix format` is used by 94.9% of respondents and
Credo by 69.6% [12]. This is the concrete difference from Erlang: the BEAM
had the same runtime for 25 years and did not grow a mass community until a
language with a package manager, formatter, test framework and docs tool
arrived. Stack Overflow 2025 has Elixir "admired" by 66% (third after Rust
72% and Gleam 70%), Phoenix as the most admired web framework, and Elixir
usage at 2.7% (up from 2.1%) [13].

### The type-retrofit case study: Dialyzer, then set-theoretic gradual types
Dialyzer (2006) uses *success typings* [9]: it infers a type that over-
approximates the set of inputs for which a function can succeed and only
reports a discrepancy when it can prove a call *will* fail. The philosophy is
"never a false positive", at the price of missing many bugs; it needs no
annotations and never broke a program. It remained the only typing story for
nearly two decades; 46.7% of Elixir developers still run it via Dialyxir [12].

Elixir's retrofit (Castagna, Duboc, Valim; announced 2022, paper 2023/2024
[10], moved "from research into development" in June 2023 [11]) follows the
same rule, enforced more rigorously: a **gradual, set-theoretic** system
(unions, intersections, negations, semantic subtyping) with a `dynamic()`
type that "works as a range": a `dynamic()` value is only reported when it is
provably incompatible with *every* accepted type. Soundness is obtained
without inserting run-time checks by accounting for the checks the VM already
performs (pattern matching, guards) [10]. The milestones were explicitly
staged so the initiative could be dropped: "If we are unhappy with the
results, we still have time to reassess our work or drop the initiative
altogether" [11].

Verified state by version: 1.17 (June 2024) inferred types from patterns and
guards for internal checks; 1.18 (December 2024) checked function calls
against inferred definitions, patterns and return types, tuples and lists,
finding "mostly dead code" in existing projects [17]; 1.19 (October 2025)
added inference and checking of anonymous functions, captures and protocol
dispatch, and said the work had to "go beyond existing literature by
extending current theory and developing new techniques" [18]; 1.20 (June
2026) completed "type inference and gradual type checking of every Elixir
program, with no type annotations required", including guard inference,
clause refinement across `case`/`cond`/`with`, non-atom map keys, and
narrowing of `dynamic()` through use, and the release post calls Elixir "now
a gradually typed language" [19]. User-written signatures are still not
available; the roadmap is typed structs next, then `$`-prefixed function
signatures, gated on "efficient recursive types" and "efficient parametric
types" [19, 20]. No release in this sequence broke existing code. Survey
demand is real but not for annotations first: 78.8% expect "an improvement
over Dialyzer", 71.3% better IDE support, 70.2% safety, and only 44.3% want
type annotations [12].

## Weaknesses

### The "nine nines" claim is not what the evidence shows
The 99.9999999% availability figure (31 ms downtime per year) for the AXD301
comes from Armstrong's 2002 MIT Lightweight Languages talk. Ulf Wiger, who
worked on the AXD301, has written that the number was a British Telecom
quotation about one network for a particular period after initial trial, that
the system was designed for "5 nines or better", and that telecom metrics
routinely ignore outages under 30 seconds, so a single 30-second outage would
consume the whole budget [21]. Mats Cronqvist, also on the project, called the
claim "pretty bogus": the system was mostly C by volume, had no restarts or
upgrades in the measured period, the figure was integrated over about five
node-years, and "Joe Armstrong was not part of the AXD 301 team" [22]. Wiger
also notes that maintenance and upgrades are the largest source of telecom
downtime, which is the thing the measurement window excluded. The honest
claim is: a large Erlang system reached carrier-grade (five-nines-class)
availability with a supervision-tree architecture. Sisu's research should
not repeat the nine-nines number.

### Unbounded mailboxes and the overload failure mode
A mailbox has no bound. A process that receives faster than it processes
grows its heap until the node runs out of memory; the symptom is a whole-VM
crash long after the cause. Selective receive makes it worse: a receive
that does not match the first message scans the *entire* queue, which the
efficiency guide calls "very expensive for processes with long message
queues" [2]; a slow process with a selective receive degrades quadratically.
Discord hit exactly this with guild processes of ~30,000 members whose
queues fell behind at peak, forcing them to manually disable features and
then build `manifold` (batching fan-out) and `semaphore` [7]. Fred Hébert's
*Erlang in Anger* devotes a chapter to diagnosing mailbox growth [23]. Fixes
that shipped, which are admissions: `process_flag(max_heap_size, ...)`
(OTP 19) lets a process be killed when it exceeds a heap limit;
`message_queue_data` `off_heap` (OTP 19) reduces GC cost of large queues; the
receive-marker optimisation; and OTP 28's priority messages (EEP 76), which
let a process receive certain messages ahead of a backlog. None of them give
back-pressure by default; it remains a library pattern (`GenStage`, `Broadway`,
`gen_server:call` with timeouts as implicit flow control).

### Performance ceiling: numeric code, large data, NIF escape hatch
The BEAM is a bytecode VM with a JIT since OTP 24, but it is designed for
latency and isolation, not throughput: every message is copied, integers are
arbitrary precision, floats are boxed, there is no shared-memory parallelism
for a large array, and the GC copies live data. Discord's member-list case is
the canonical data point: with 250,000 entries, Erlang `ordsets` insertion
took about 27,000 µs worst case and a custom skip-list-of-ordsets 640 µs;
rewriting the structure in Rust as a NIF via Rustler gave 0.61-3.68 µs [24].
NIFs are the escape hatch, with two documented costs: "A native function that
crashes will crash the whole VM" [25] (Rustler's safe wrappers and
`catch_unwind` mitigate, not eliminate, this), and a NIF must return within
about 1 millisecond or be split or scheduled on a dirty scheduler, because the
scheduler cannot preempt C [25]. For numerics the ecosystem's answer (Nx,
EXLA, 2021-) is to compile numerical graphs to XLA outside the VM, which
concedes the point.

### Dynamic typing for 35 years, and the cost of retrofitting late
Erlang tried to add a type system in 1997 (Marlow and Wadler) for a subset;
it was never integrated [26]; Armstrong's thesis uses a simplified type
notation purely as documentation [1, §3]. Dialyzer's "never wrong" stance
means the common bug class (wrong tuple shape passed between modules) is
found at run time as a `badmatch`/`function_clause`, which "let it crash"
turns into a restart loop. The Elixir retrofit is going well precisely
because it is slow and annotation-free; but after four years (2022-2026)
users still cannot write a signature, and 1.20 lists recursive and
parametric type performance as unsolved prerequisites [19].

### Hot code upgrades are rarely used in practice
Elixir's own release tooling states: "this feature is not supported out of
the box by Elixir releases. The reason we don't provide hot code upgrades is
because they are very complicated to perform in practice", recommends
blue/green, canary and rolling deployments instead, and calls hot upgrades
"rarely a viable option" [27]. In the 2025 survey 36.2% of production
deployments are a single node and 45.6% are 2-5 nodes [12]. The feature
imposes permanent design costs (two code versions per module, the
fully-qualified-call rule, `code_change` callbacks) that most users never
cash in, and a subtle failure: a process that never makes an external call
is killed on the second reload.

### Erlang's syntax and the 1998 ban as adoption history
Ericsson banned Erlang for new products in February 1998, "the main reason
for the ban was that Ericsson wanted to be a consumer of software
technologies rather than a producer" [1, §1]; the team left to found
Bluetail. Erlang's Prolog-derived punctuation (`,` `;` `.` terminators,
records as compiler-expanded tuples, no string type) is widely cited as the
reason the BEAM needed Elixir to grow; OTP 29's experimental *native
records* (EEP-79), 40 years in, admits that tuple-based records were a
problem [28]. Elixir solved syntax but kept Erlang-isms (charlists vs
binaries, `{:ok, v}` tuples with no static guarantee).

### Distribution is naive by default
Full mesh means `n^2` connections; the transitive-connect default and the
lock-based `global` registry do not scale to hundreds of nodes; the protocol
is clear text unless TLS is configured [5]; there is no built-in consensus,
partition handling or inter-node back-pressure. Users layer Partisan,
libcluster or Horde on top. Built-in distribution is excellent for 2-50
nodes and a liability if mistaken for a cluster framework.

### Tooling gaps the community still reports
The top "most needed missing tool" in the 2025 Elixir survey is "better LSP,
IDE support and debuggers" (23.8%); tooling/LSP/IDE is the second largest
"improvement focus" (18.3%) after adoption/jobs (19.9%) [12]. Elixir 1.18
added a compiler lock so editors and IEx stop recompiling under each other
[17]; 1.19's headline was "up to 4x faster compilation" [18].

## Designer retrospectives

- Armstrong, thesis conclusions (2003): "The property of strong isolation is
  not strictly adhered to in any known implementation of Erlang. One process
  could affect another process in the system by allocating vast amounts of
  memory or by sending large numbers of messages to another process. A
  malicious process could destroy the entire system by creating large numbers
  of atoms and overflow the atom table" [1, §10.2]. He wanted more "conceptual
  integrity": "everything is a process", all processes named, a `!!` RPC
  operator, files as processes. None of this shipped; the mailbox problem he
  names in 2003 is the one Discord hit in 2017.
- Armstrong, HOPL III (2007) [26], on the failed 1997 type system and on the
  ban: the language survived because it was open-sourced after being banned.
- Armstrong (2013): "If Java is 'write once, run anywhere', then Erlang is
  'write once, run forever'" [29] - the pitch is reliability, never
  performance.
- Valim, June 2023 [11]: the type system was planned with an explicit exit:
  "If we are unhappy with the results, we still have time to reassess our
  work or drop the initiative altogether." Valim, June 2026 [19]: the
  milestone is reached but signatures wait on performance; the tone is that
  the research outran the literature ("go beyond existing literature") [18].
- The Elixir release docs' refusal to ship hot upgrades [27] is a
  retrospective on Erlang's R5 requirement: valuable in telecoms, "rarely a
  viable option" elsewhere.

## Influence

Copied from Erlang: supervision trees and "let it crash" (Akka, Pony,
Orleans, Elixir, Gleam, Kubernetes-style restart policies); lightweight
preemptive green processes with per-process heaps (Go's goroutines are
cooperative-then-preemptive and share a heap, which is the main divergence);
actor-style location transparency (Akka Cluster, Orleans grains); pattern
matching on messages; bit syntax for binary protocols (copied into Elixir,
Gleam, and admired by Rust's `nom`/Zig users); Hex and Mix influenced by
Bundler/Rubygems and in turn influenced Gleam's tooling. Erlang copied from
Prolog (first implementation, syntax, atoms, pattern matching), Smalltalk
(message passing as the primitive), ML (functions, immutability) and Jim
Gray's fail-fast process pairs. Elixir copied from Ruby (surface syntax,
blocks, Bundler), Clojure (protocols, reducers, macros as functions on data),
F# (`|>`) and Python (doctests). Gleam exists because people wanted the BEAM
with static types.

## Lessons for Sisu

- **Adopt**: *isolation as the unit of failure.* A task/actor that can die
  without corrupting anything else is the only error-handling model with a
  decades-long industrial reliability record. For Sisu this means: no shared
  mutable memory between concurrent units by default, and a runtime (or
  library with language support) that can kill a unit and run a supervisor
  policy. Panics should be unit-local, not process-global.
- **Adopt**: *per-unit heaps or an equivalent pause-isolation guarantee.* If
  Sisu has a GC, the Erlang result is that pause isolation matters more than
  throughput for servers; if Sisu uses ownership/RC, the same property comes
  for free and should be advertised.
- **Adopt**: *preemption points under compiler control.* Fair scheduling of
  many lightweight tasks requires the compiler to know where a task can be
  suspended; a language with loops needs the compiler to insert yield checks
  (as Go did in 1.14 with async preemption) or accept starvation.
- **Adopt**: *the Dialyzer/Elixir "no false positives first" rule for any
  gradual or inferred checking.* Elixir's four-year rollout broke nothing
  because every warning is a guaranteed run-time failure. A new language does
  not need gradual typing, but its lints, nullability or effect checks should
  obey the same rule or users will disable them.
- **Adopt**: *documentation, tests, formatter and package registry as part
  of the language's initial release.* Elixir, not the BEAM, grew the
  community, and the survey shows which tools reach >90% use when they are
  built in.
- **Adapt**: *behaviours as typed interfaces.* The `gen_server` shape
  (generic loop, user callbacks, system messages) should be an interface with
  a typed message protocol. Gleam shows the difficulty: `handle_info`-style
  catch-alls and selective receive do not type cleanly. Sisu should design
  the actor protocol with typed channels per message kind rather than one
  untyped mailbox.
- **Adapt**: *bounded mailboxes and back-pressure as the default.* Erlang's
  unbounded mailbox is its best-documented production failure mode; three
  separate OTP fixes mitigate it. Sisu should make bounded channels the
  default and unbounded an explicit opt-in, and expose queue depth as a
  first-class metric.
- **Adapt**: *hot code loading as a development feature, not a deployment
  promise.* Keep the ability to replace a function in a running debug build
  (REPL-grade feedback); do not design the module system around production
  hot upgrades, which Elixir itself declines to support.

- **Avoid**: *dynamic typing with the intent to add types later.* The Erlang
  to Elixir-1.20 arc shows the cost: 20 years with Dialyzer, then a research
  programme that still has no signatures after four years. Start statically
  typed; let any `dynamic` escape hatch be the exception.
- **Avoid**: *repeating the nine-nines marketing.* Cite five nines with the
  Wiger/Cronqvist caveats; the credibility of the reliability argument depends
  on it.
- **Avoid**: *designing the scheduler and the FFI separately.* NIFs' 1 ms
  rule and VM-crash risk follow from a VM that does not own the native
  stack; for Sisu the equivalent risk is a blocking call inside a task.
- **Open questions raised**: (1) Can "let it crash" coexist with a static
  type system that pushes errors into `Result` values? Gleam says yes (panic
  for bugs, `Result` for expected failures) but it is early. (2) Is
  copy-on-send message passing acceptable in a language that also wants
  zero-copy performance, or does Sisu need ownership transfer of messages
  (Pony/Rust-style)? (3) Should supervision be a runtime concept (BEAM) or a
  structured-concurrency library concept (Trio/Kotlin nurseries)? The BEAM
  evidence favours runtime support for *killing* a task safely; the library
  approach is sufficient for restart policy. (4) Elixir's set-theoretic types
  are the first production-scale test of semantic subtyping in a mainstream
  language; whether its `dynamic()` discipline is a better gradual story than
  TypeScript's `any` is not yet settled.

## Sources

1. Joe Armstrong, *Making reliable distributed systems in the presence of software errors*, PhD thesis, KTH, 2003. https://erlang.org/download/armstrong_thesis_2003.pdf
2. Erlang/OTP Efficiency Guide, "Processes". https://www.erlang.org/doc/system/eff_guide_processes.html (2026)
3. Erlang/OTP ERTS documentation, "Erlang Garbage Collector". https://www.erlang.org/doc/apps/erts/garbagecollection.html (2026)
4. Erik Stenman, *The BEAM Book*, chapter "Scheduling" (CONTEXT_REDS). https://github.com/happi/theBeamBook/blob/master/chapters/scheduling.asciidoc
5. Erlang/OTP Reference Manual, "Distributed Erlang". https://www.erlang.org/doc/system/distributed.html (2026)
6. Rick Reed, "Scaling to Millions of Simultaneous Connections", Erlang Factory SF 2012; "That's 'Billion' with a 'B'", 2014. Wired, "Why WhatsApp Only Needs 50 Engineers for Its 900M Users", 2015.
7. Discord Engineering, "How Discord Scaled Elixir to 5,000,000 Concurrent Users", 2017. https://discord.com/blog/how-discord-scaled-elixir-to-5-000-000-concurrent-users
8. Erlang/OTP Reference Manual, "Compilation and Code Loading". https://www.erlang.org/doc/system/code_loading.html (2026)
9. Tobias Lindahl, Konstantinos Sagonas, "Practical Type Inference Based on Success Typings", PPDP 2006.
10. Giuseppe Castagna, Guillaume Duboc, José Valim, "The Design Principles of the Elixir Type System", *The Art, Science, and Engineering of Programming* 8(2), 2024 (arXiv:2306.06391, 2023). https://arxiv.org/abs/2306.06391
11. José Valim, "Type system updates: moving from research into development", elixir-lang.org, 22 June 2023. https://elixir-lang.org/blog/2023/06/22/type-system-updates-research-dev/
12. Curiosum, *State of Elixir 2025* (1,018 respondents, 45 questions). https://elixir-hub.com/surveys/2025 (2025)
13. Stack Overflow Developer Survey 2025, "Technology" (admired/desired). https://survey.stackoverflow.co/2025/technology/
14. Erlang/OTP 29.0 release announcement, 13 May 2026. https://erlang.org/news/188
15. Wikipedia, "Elixir (programming language)" (goals quote; first release 25 May 2012). https://en.wikipedia.org/wiki/Elixir_(programming_language)
16. hex.pm homepage counters, retrieved October 2026. https://hex.pm/
17. "Elixir v1.18 released", 19 December 2024. https://elixir-lang.org/blog/2024/12/19/elixir-v1-18-0-released/
18. "Elixir v1.19 released", 16 October 2025. https://elixir-lang.org/blog/2025/10/16/elixir-v1-19-0-released/
19. "Elixir v1.20 released", 3 June 2026. https://elixir-lang.org/blog/2026/06/03/elixir-v1-20-0-released
20. Elixir reference, "Gradual set-theoretic types". https://github.com/elixir-lang/elixir/blob/main/lib/elixir/pages/references/gradual-set-theoretic-types.md
21. Ulf Wiger, erlang-questions, "erlang flagship product (was The Beauty of Erlang Syntax)", February 2009. https://erlang.org/pipermail/erlang-questions/2009-February/042020.html
22. Mats Cronqvist, "The nine nines", Erlang Factory SF Bay 2010 (slides). https://www.erlang-factory.com/upload/presentations/243/ErlangFactorySFBay2010-MatsCronqvist.pdf
23. Fred Hébert, *Erlang in Anger*, 2014 (chapter on memory leaks and mailbox growth). https://www.erlang-in-anger.com/
24. Discord Engineering, "Using Rust to Scale Elixir for 11 Million Concurrent Users", 2019. https://discord.com/blog/using-rust-to-scale-elixir-for-11-million-concurrent-users
25. Erlang/OTP ERTS documentation, `erl_nif` (crash warning, 1 ms rule, dirty schedulers). https://www.erlang.org/doc/apps/erts/erl_nif.html
26. Joe Armstrong, "A History of Erlang", HOPL III, 2007. https://dl.acm.org/doi/10.1145/1238844.1238850
27. Elixir `mix release` documentation, section on hot code upgrades. https://mix.hexdocs.pm/Mix.Tasks.Release.html
28. EEP-79, "Native records" (experimental in OTP 29). https://www.erlang.org/eep/eeps/eep-0079.html
29. Joe Armstrong, Rackspace interview, 2013 ("write once, run forever"), as quoted in Wikipedia, "Erlang (programming language)".
30. Wikipedia, "Erlang (programming language)" (1998 ban, open-sourcing, users; infobox 29.1.1, 22 Sept 2026). https://en.wikipedia.org/wiki/Erlang_(programming_language)
