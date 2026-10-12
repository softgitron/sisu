# Live and exploratory environments

## Why this family matters for Sisu

Sisu's plan makes three things true together that no mainstream compiled language has: programs run
first in an interpreter over the typed IR (D-052), every interaction with the world passes through a
capability handed down from `main` (06), and debug builds can replace a function in a running
program (D-053). Live and record/replay tools have spent decades building the boundary that
capabilities provide by construction; this family shows which of their features become cheap for
Sisu, and which have the evidence to be worth building.

## Systems covered

| System | Years | Status (Oct 2026) | Central idea |
|---|---|---|---|
| Bret Victor's essays and talks | 2012- | Essays and talks [1][2] | Show the data; immediacy is a prerequisite, not the goal |
| Light Table | 2012-2022 | Repository archived 21 Oct 2022 [6] | Every expression evaluated inline as you type |
| Smalltalk/Pharo, Glamorous Toolkit | 1980-, 2010s- | Pharo 14 (Jun 2026); GT book v1.1.617 [13] | Live image; per-type "moldable" inspectors |
| Boxer | 1980s- | Boxer Sunrise 3.4.29, 7 Oct 2026 [17] | "Naive realism": everything visible and editable |
| Jupyter | 2011- (Jupyter 2014) | The dominant notebook | Cells over a stateful kernel |
| Observable | 2017- | Notebooks 2.0 technology preview [21] | Reactive dataflow runtime |
| Pluto.jl, marimo, Livebook | 2020s | Active [22][23][24] | Reactive notebooks stored as plain source files |
| Swift Playgrounds, Xcode | 2014-, iPad 2016 | `#Playground` macro in Xcode 26 [4] | Results shown beside the code |
| Python Tutor | 2010- | "Over 25 million users since 2010" [5] | Step-by-step visualisation of program state |
| Quokka.js | 2010s- | "over 1 million" users claimed [7] | Runtime values inline as you type |
| Dark; Val Town, Replit | 2019-2025; 2020s, 2016- | Dark out of money Jun 2025, Classic wound down from Jul 2025 [25]; Val Town and Replit active, marketed for AI agents [27] | Deployless hosting; trace-driven development (Dark) |
| Eve | 2014-c. 2018 [unverified end] | "no longer under active development" [39] | Records, relational logic, literate documents |
| Subtext | 2004-2020 | Retrospective published 2025 [38] | No syntax; schematic tables |
| Unison watch expressions | 2010s- | Active [8] | Evaluate on save, cache by hash, no I/O |
| Elm debugger | 2014 (Reactor), 2016 (0.18) | `--debug` flag | Record messages; import and export sessions |
| rr, Pernosco | 2014-, late 2010s- | rr 5.9.0; Pernosco hosted [28][29] | Record/replay at the system-call boundary; omniscient queries |
| WinDbg Time Travel Debugging | 2017- | Active [30] | Instruction-level recording |
| Tardis, Node-ChakraCore TTD | 2014-2020 | Archived 15 Oct 2020 [31] | Time travel inside a managed runtime |
| Replay.io | 2020s | Now sells "Replay QA" [34] | Recorded browser sessions |
| Whyline | 2004-2008 | Research [33] | Why and why-not questions over a full trace |
| FoundationDB simulation, Antithesis | 2010-, 2018- | FDB open source; Antithesis commercial [35] | Deterministic simulation of whole systems |
| loom, shuttle (Rust); Go `testing/synctest` | 2010s-; 2025- | Active [36][37] | Controlled schedules and virtual time in tests |

## The ideas

### 1. Victor's demands, and which have been met

"Learnable Programming" (September 2012) asks the environment to let a learner "read the
vocabulary", "follow the flow", "see the state", "create by reacting" and "create by abstracting"
[1]. Its strongest lines are about state: "The entire purpose of code is to manipulate data, and we
never see the data", and "All state must be eliminated or shown" [1]. It also corrects the popular
reading of "Inventing on Principle" (CUSEC, January 2012) [2]: "live coding, as a standalone
feature, misses the point", and "Immediate-update is merely a prerequisite for doing anything
interesting"; Khan Academy's environment, which cited Victor, is his example of learners nudging
unlabelled numbers [1].

| Demand | Met where, by October 2026 | Evidence of use |
|---|---|---|
| Read the vocabulary | Hover documentation, completion, block palettes | Universal; palettes measured in `visual-and-structure-editing.md` |
| Follow the flow | Python Tutor's step-through; Quokka's "Time Machine" | Python Tutor's 25 million users [5] |
| See the state | Inline values (Quokka, playgrounds, LSP 3.17 inline values [40]), debugger visualisers | Moderate; small preference studies (idea 2) |
| Create by reacting | Hot reload (Flutter, [`dart.md`](../languages/dart.md)), SwiftUI previews, `#Playground` | Strong adoption |
| Create by abstracting | Extract-variable and extract-function refactorings; nothing keeps the concrete case live while generalising | None |

Lattner: Playgrounds were "heavily influenced by Bret Victor's ideas" and Light Table [3]; Xcode 26's
"#Playground macro enables interactive debugging and code exploration in the preview panel" [4].

### 2. Inline values and always-on evaluation

Light Table (Chris Granger, April 2012) promised "Trying is encouraged - changes produce
instantaneous results" and "Files are not the best representation of code, just a convenient
serialization"; typing an expression showed its value and the values flowing through the functions
it called [6]. It passed its $200,000 Kickstarter goal, joined Y Combinator in May 2012, was
open-sourced in January 2014, and its team moved on to Eve in October 2014 (a $2.3M seed); the
repository was archived on 21 October 2022 [6]. Granger's lesson two years into Eve: "We learned
with Light Table that we can't just slap a UI onto Javascript and expect it to work" [6].

The idea survives in narrower tools. Quokka.js shows "runtime values in your editor as you type"
[7]. Unison evaluates `> square 4` watch lines on every save without re-running unchanged ones
("caching results, keyed by the hash of each expression"), and can do so safely because "Unison's
type system won't let you do I/O inside one of these watch expressions" [8]. Babylonian-style
programming attaches named examples and "probes" to ordinary JavaScript functions, stores them as
comments in the source, and substitutes user input with "replacements", which its authors compare to
mock objects [9]. rust-analyzer offers an on-demand "Interpret a function, static or const" action
and nothing always-on [12].

The evidence is adoption and preference, not outcomes. In the Projection Boxes study (CHI 2020) ten
programmers were "not distracted by the always-on visualization", but preferences varied so widely
that configurability became the main requirement [10]; Rein et al.'s literature study finds
live-programming tools are usually evaluated by user studies of this kind [11]. The failure modes
are cost and side effects: Light Table and Quokka execute arbitrary code on every change, so the
user must keep in mind what each re-evaluation does to the world. Unison's rule removes that
problem by construction.

### 3. Moldable inspectors

Glamorous Toolkit (feenk, on Pharo) generalises the Smalltalk inspector: any class can contribute
views of its instances, and the environment ships "more than 6000 contextual tools" on the premise
that "Code reading is the bottleneck" [13]. A view is a method marked with a pragma:

```smalltalk
gtItemsFor: aView
    <gtView>
    ^ aView list title: 'Items'; items: [ self items ]
```

The claims are the designers' own; there are no outcome studies, and GT's users are a Pharo niche
(see [`smalltalk.md`](../languages/smalltalk.md)). The small version is everywhere: IPython's
`_repr_html_` and `_repr_mimebundle_`, the mechanism behind rich tables and plots in notebooks [14];
.NET's `DebuggerDisplay` [15]; Natvis files, which Rust crates embed with
`#![debugger_visualizer(natvis_file = "...")]` [16]. Their specifications warn about one hazard:
IPython's methods should "have no side effects", by convention [14], and Microsoft notes that
"Evaluating an expression can change the state of the application" [15]. Boxer pushed the idea to
its limit: in diSessa and Abelson's "naive realism" the screen is the whole computational world,
every value a visible, editable box (CACM 1986; paraphrased, not retrieved); it survives as Boxer
Sunrise [17].

### 4. Notebooks and the hidden-state problem

Pimentel et al. (MSR 2019) collected 1,159,166 Jupyter notebooks from 264,023 GitHub repositories
and re-ran them: "out of 863,878 attempted executions of valid notebooks ..., only 24.11% executed
without errors and only 4.03% produced the same results"; 36.36% of notebooks with an unambiguous
execution order had cells run out of order; 29.23% of executions failed on missing modules [18].
Interviews and a survey (n = 20 and 156) catalogue nine pain points [19]; replaying 666 real
sessions, NBSafety flagged 117 with potential stale-state errors [20].

The fix that has shipped is reactivity, so that the code on screen is the state. Observable's
runtime treats variables like spreadsheet cells and recomputes dependents when inputs change,
including through promises and generators [21]. Pluto.jl: "At any instant, the program state is
completely described by the code you see"; notebooks are "saved as pure Julia files" with the
package environment inside [22]. marimo analyses each cell's definitions and references into a
graph, "requires that every global variable be defined by only one cell", stores notebooks "as pure
Python (Git-friendly)", and does not track mutation: "Avoid defining a variable in one cell and
mutating it in another" [23]. Livebook stores Markdown (`.livemd`) and marks stale cells [24];
Observable's Notebooks 2.0 preview moves to "an open file format" [21]. The lesson: reactivity needs
dependencies that are statically known, and mutation through aliases is where it leaks.

### 5. Deployless and trace-driven development

Dark joined editor, language and infrastructure: code was "immediately deployed to production,
ready to be enabled with a feature flag", deploys took about 50 ms, and the editor was structured
[25]. Its original feature was trace-driven development. The documentation's advice is "Send
requests to Darklang _before writing code_"; an unhandled request appears as a 404 from which a
handler can be created; each request is kept as a trace; with a trace selected, every expression
shows its "live value"; expressions with side effects (database writes, external calls) get a play
button so the programmer decides when they run [25].

The costs are documented. Dark kept every request in its main database; trace garbage collection
(keeping "the last 10 requests" and the last week) caused most of the database load; traces keyed by
URL path were lost when a route was renamed; the proposed fix was log-style object storage and
traces tied to handler identity [25]. The company raised a $3.5M seed in July 2019, laid off its
team in June 2020 ("we have been focusing on growth of a product that has not yet reached
product-market fit"), rewrote the backend in F#, went "all-in on AI" in 2023, and had "officially
run out of money" by June 2025; Darklang-Classic, costing about $2,600 a month, was wound down from
July 2025 [25]. No post-mortem blames the traces. Their mainstream descendant is smaller: Ruby's
VCR, "Record your test suite's HTTP interactions and replay them during future test runs for fast,
deterministic, accurate tests" [26]. Val Town ("Instant deployments for you & your agents") and
Replit continue deployless hosting, now aimed at AI agents [27].

### 6. Record and replay: the cost is set by where the boundary is

rr states the method: "We identify a boundary around state and computation, record all sources of
nondeterminism within the boundary and all inputs crossing into the boundary, and reexecute the
computation within the boundary by replaying the nondeterminism and inputs" [28]. With only the
Linux process boundary available, rr intercepts system calls, runs one thread at a time ("timing is
nondeterminism that must be recorded"), counts retired conditional branches with performance
counters, and in 2017 "cannot be implemented on ARM CPUs" (Apple M1 and later are supported now)
[28]. It reaches "slowdowns down to ≤ 1.2x" on Firefox tests and under 2x on important workloads,
is "in daily use by many developers", and Pernosco builds a hosted omniscient debugger on its
recordings [28][29]. Recording instructions costs more: WinDbg's TTD expects "about a 10x-20x
performance hit", trace files of "several gigabytes" within minutes, and "you can't change
history" [30].

Inside a language runtime the boundary is cheaper. Tardis (OOPSLA 2014) built time travel into a
managed runtime at "an average overhead of only 7%", about 0.6 MB/s of logs and 0.68 s worst-case
latency [31]; its production form, Node-ChakraCore's TTD, was supported in VS Code and archived with
the engine on 15 October 2020 [31]. Elm records the inputs of a pure program. Elm Reactor (2014)
snapshotted every 100 events and could "replay these inputs on new code to see a bug fix" [32]. Elm
0.18 (November 2016) kept only import and export of sessions after Czaplicki asked production users
what they wanted: "Import/export is *the* feature", "the features that demo well are not so
important for baseline production usage", and the limit: "You can rewind your Elm program, but your
JavaScript and database are not going to come along!" [32].

A full trace also answers questions a breakpoint cannot. The Whyline "reduced debugging time by
nearly a factor of 8, and helped programmers complete 40% more tasks" in Alice, and in Java "novice
programmers with the Whyline were twice as fast as expert programmers without it"; the Java
version's tracing slowed programs by 1.7x to 15x [33]. Commercial adoption trails the lab:
Replay.io's browser time-travel debugger has become "Replay QA", autonomous testing built on its
recordings [34].

### 7. Deterministic simulation

FoundationDB built a "fully-deterministic event-based network simulation" before the database. It
runs "deterministic simulation of an entire FoundationDB cluster within a single-threaded process",
"tens of thousands of simulations every night", "roughly one trillion CPU-hours" in all, and Will
Wilson, who was on that team, reports "we only ever had one or two bugs reported by a customer"
[35]. Antithesis (founded 2018) sells the property for unmodified software through a hypervisor that
makes the machine deterministic [35]. At language level, loom permutes executions "under the C11
memory model" with state reduction, and shuttle schedules threads randomly (PCT); shuttle "is not
sound ... but it scales to much larger test cases than Loom" and reproduces failures
deterministically [36]. Go's `testing/synctest` gives a test a fake clock that advances "when all
goroutines are blocked" (experimental in Go 1.24) [37]. [`testing.md`](../topics/testing.md) already
uses this evidence for Sisu's virtual test clock, and Phase 6 of
[18-next-steps](../../proposal/18-next-steps.md) plans `--schedule-seed N` and `--explore N` for the
interpreter's single-threaded scheduler; a simulated network with injected faults is not yet
planned.

### 8. Live images, new paradigms and the editor as product

Smalltalk's fix-and-resume debugger is covered in [`smalltalk.md`](../languages/smalltalk.md) and is
the model for D-053 [41]. The more radical systems failed for reasons their authors give. Edwards
calls Subtext (2004-2020) "a series of overambitious failed experiments": he was "trying to invent
too many things at the same time" [38]. Eve went through more than 30 versions in two years;
"generalized tools tend to end up with developers as their first customers", and "we need developers
to like it before end users will" [6]; its repository says it "is no longer under active
development" [39]. Light Table, Eve and Dark each made a new editor the product and none survived as
one; Lamdu is a technology preview after fifteen years.

## Transferable ideas for Sisu

What Sisu's design makes cheap. The boundary rr reconstructs from system calls is Sisu's capability
layer, and the scheduler rr must serialise is Sisu's own runtime. The nondeterminism of a run is (a)
the results of capability calls, (b) scheduling choices at blocking points and compiler-inserted
yield points (which task runs, which `select` arm fires, the order of lock, channel and atomic
operations), and (c) whatever `unsafe` and `extern` code in `[trust]` packages does. The runtime
owns (a) and (b); (c) is already the audited exception (06). In the interpreter, which runs the
`single` profile on a seeded scheduler (18-next-steps, Phase 6), (b) reduces to the seed plus the
order in which I/O completes; for a native M:N runtime, data-race freedom means recording
synchronisation order covers (b) without serialising threads. What has the evidence: deterministic
simulation (strong), record and replay (strong where it is cheap), inspectors (broad adoption in
simple forms), inline values (adoption, little outcome data), time-travel interfaces (strong lab
results, weak adoption).

1. **Deterministic simulation as a test mode.**

   ```sisu
   test "replicas agree after a partition heals"(sim: Sim):
       let net = sim.network(nodes=3, drop=0.01)
       sim.at(Duration.seconds(5), () -> net.partition([0], [1, 2]))
       sim.at(Duration.seconds(20), () -> net.heal())
       sim.run_for(Duration.minutes(2)) do s:
           for i in range(3):
               s.spawn(() -> run_replica(net.node(i), sim.clock))
       assert net.all_agree()
   ```

   Phase 6 already gives the interpreter a seedable scheduler (`--schedule-seed N`, `--explore N`);
   what is missing is the world. `Sim` is the per-test capability set (17-testing) plus a simulated
   `Net` with loss, delay and partitions and fault points in the other fakes (a failing `Fs` write,
   a jumping `Clock`), all driven by the same seed, so `sisu test --explore 10000` explores
   interleavings and failures together and a failing seed replays exactly: FoundationDB's design at
   language level. Cost: low to medium (a network simulator and fault schedule in `std.test` on
   planned machinery). Conflicts: tests holding the real `System`, and `extern` code, cannot be
   simulated; the standard library must keep no hidden entropy (hash seeds come from `Rand`; nothing
   depends on addresses, which 17-testing already treats as non-deterministic). Confidence: high.

2. **Capability-level recording and replay.**
   [`incremental-reversible-and-exotic-paradigms.md`](incremental-reversible-and-exotic-paradigms.md)
   proposes the mechanism (its idea 1: log capability results and scheduling decisions in the
   interpreter, replay without touching the world, snapshots for reverse execution):

   ```
   $ sisu run --record=crash.trace server.sisu   # capability results, I/O completion order, seed
   $ sisu debug --replay=crash.trace             # the same execution; no disk or network touched
   ```

   This file adds the evidence and three details. The evidence: rr's model and adoption, Tardis's 7%
   for a runtime-level boundary, and Elm's finding that importing and exporting a session is "the
   feature" for production users. The details: (a) in the interpreter a trace is the seed, the
   capability results and the I/O completion order, so it is small, and a CI failure replays on a
   laptop (Elm's "perfect bug report" for a systems language); (b) a native `full` runtime logs
   synchronisation order per object instead of serialising threads, and counting loop yield points
   does the job of rr's branch counter; (c) traces carry secrets (TTD's documentation warns about
   personal data [30]), so recording needs per-capability redaction (for example `Env` keys), with
   large payloads in content-addressed blobs. Cost: medium. Conflicts: `extern` calls must be logged
   at the FFI boundary or the trace marked partial. Confidence: high for the interpreter, medium for
   a native runtime.

3. **Trace-to-test.** `sisu test new --from-trace crash.trace --request 412` writes a test whose
   capability fakes are preloaded from the trace (a VCR cassette) and whose outputs become
   `assert_snapshot` values:

   ```sisu
   test "issue 812: checkout with an empty cart":
       let world = Recorded.load("tests/traces/issue-812.trace")
       assert_snapshot(handle(world.net, world.fs, world.clock, world.request(412)))
   ```

   Dark's live values show the appeal; its tracing post-mortem gives the storage rules (append-only
   files outside the application's database, keyed by handler identity, with explicit retention)
   [25]. Cost: low once (2) exists. Conflicts: none beyond the snapshot policy. Confidence: medium.

4. **Always-on evaluation of capability-free examples.**

   ```sisu
   ## ```
   ## assert slug("Hello, World!") == "hello-world"
   ## ```
   pure def slug(title: str) -> str: ...
   ```

   The language server runs doc examples and `test` blocks that need no real capability in the
   comptime interpreter, which already has "no I/O at all" (06), after each edit, off the request
   path and with a step budget, and caches results by the body's hash as Unison does. The `assert`
   intrinsic already records every sub-expression's value (17-testing), so the editor can show
   them through LSP inline values [40] with no new syntax; a Babylonian "probe" is an editor
   command, not syntax. Code that takes capabilities runs only against the default fakes. Cost: low
   to medium. Conflicts: the 100 ms language-server target (10-tooling) means evaluation must be
   asynchronous and cancellable. Confidence: medium (adoption evidence, little outcome evidence).

5. **Inspectors as a `pure` trait.**

   ```sisu
   impl Inspect for Histogram:
       pure def views(self) -> list[View]:
           return [View.table("Buckets", self.buckets, columns=["start", "end", "count"]),
                   View.bars("Shape", [(b.start, b.count) for b in self.buckets])]
   ```

   The REPL, the debugger (through DAP variable views), test failures and snapshot diffs render
   the views, and the default is derived from `Debug`. Requiring `pure` turns IPython's convention
   and Microsoft's warning into a checked guarantee: a paused debugger may call any inspector
   without changing the program. `View` is data (table, tree, chart, text), not a GUI toolkit, so
   it stays inside the non-goals (14). Cost: low to medium. Conflicts: overlaps `Debug`, so it is
   optional. Confidence: medium.

6. **Replay on edit.** After a hot replacement (D-053), re-run the current recording or example
   against the new code and show what changed (return values, capability calls, snapshots), stopping
   at the first divergence from the log ("the new code calls `fs.write` where the recording has
   `net.send`"). Precedents: Elm Reactor's replay on new code, Dark's live values, Hazel's
   fill-and-resume. Cost: medium once (2) exists. Conflicts: none. Confidence: low-medium; Elm
   dropped the feature in 0.18 as unimportant for production use [32].

7. **Time travel in the debugger, then why-questions.** DAP already defines `stepBack` and
   `reverseContinue` [40]; with the snapshots of idea 2 they become re-execution from checkpoints
   (Elm snapshotted every 100 events; rr forks a process per checkpoint, which "typically takes less
   than ten milliseconds" [28]). The Whyline's results argue for why and why-not queries over the
   trace as the later step. Cost: medium-high. Conflicts: the native debug backend would need the
   same machinery. Confidence: low-medium: strong lab evidence, weak adoption (rr's niche,
   Replay.io's pivot, ChakraCore's end).

8. **A reactive notebook stored as `.sisu`.** If Sisu ever serves notebook users (00 says not yet),
   copy marimo and Pluto: a plain source file, cells ordered by static dependencies, no kernel
   state; single-definition `let` and explicit `mut` borrows close marimo's mutation gap. Cost:
   medium. Conflicts: the stated audience. Confidence: low.

## Dead ends

- **A bespoke live editor as the product**, as Light Table, Eve, Dark and Lamdu built. Sisu's live
  features ship through the LSP (inline values) and DAP (step back) into existing editors.
- **An imperative kernel with hidden state.** Jupyter's 36% out-of-order and 4% reproducible.
- **Always-on evaluation of code that holds real capabilities.** Only capability-free code, or code
  given fakes, is evaluated without being asked.
- **Deployless editing of production.** Dark's model bypasses review, `sisu publish` provenance and
  `sisu audit`, and the company that built it did not survive.
- **Instruction-level recording as the default, or traces in the application's database.** TTD's
  10-20x and gigabytes per minute buy nothing the capability boundary does not, and Dark's trace
  garbage collection became most of its database load.
- **A new paradigm and a new environment at once.** Eve and Subtext, by their authors' accounts.
- **Image persistence of live state.** Already out of scope (D-053, `smalltalk.md`).

## Sources

1. B. Victor, "Learnable Programming", September 2012. http://worrydream.com/LearnableProgramming/
2. B. Victor, "Inventing on Principle", CUSEC, January 2012 (video, https://vimeo.com/36579366; not re-watched).
3. C. Lattner, home page (Playgrounds and their influences). http://nondot.org/sabre/
4. Apple, Xcode 26 release notes (`#Playground`). https://developer.apple.com/documentation/xcode-release-notes/xcode-26-release-notes
5. Python Tutor (usage figures, Oct 2026). https://pythontutor.com/
6. C. Granger: "Light Table - a new IDE concept" (12 Apr 2012), "We made it" (12 May 2012), "Light Table is open source" (7 Jan 2014), "Beyond Light Table" (1 Oct 2014), "Two years of Eve" (21 Jul 2016), https://www.chris-granger.com/archive ; https://github.com/LightTable/LightTable (archived 21 Oct 2022).
7. Quokka.js. https://quokkajs.com/
8. Unison documentation, "Tour" (watch expressions, result caching, test cache). https://www.unison-lang.org/docs/tour/
9. D. Rauch, P. Rein, S. Ramson, J. Lincke, R. Hirschfeld, "Babylonian-style Programming", The Art, Science, and Engineering of Programming 3(3), 2019. arXiv:1902.00549
10. S. Lerner, "Projection Boxes: On-the-fly Reconfigurable Visualization for Live Programming", CHI 2020. https://cseweb.ucsd.edu/~lerner/papers/projection-boxes-chi2020.pdf
11. P. Rein, S. Ramson, J. Lincke, R. Hirschfeld, T. Pape, "Exploratory and Live, Programming and Coding: A Literature Study Comparing Perspectives on Liveness", Programming 3(1), 2019. arXiv:1807.08578
12. rust-analyzer manual, "Features". https://rust-analyzer.github.io/book/features.html
13. feenk, Glamorous Toolkit, https://gtoolkit.com/ ; GT book (v1.1.617), https://book.gtoolkit.com/ ; https://moldabledevelopment.com/
14. IPython documentation, "Rich display" (integrating your objects). https://ipython.readthedocs.io/en/stable/config/integrating.html
15. Microsoft, "Display custom info using DebuggerDisplay" (2026). https://learn.microsoft.com/en-us/visualstudio/debugger/using-the-debuggerdisplay-attribute
16. The Rust Reference, "Debugger attributes". https://doc.rust-lang.org/reference/attributes/debugger.html
17. A. diSessa, H. Abelson, "Boxer: A Reconstructible Computational Medium", CACM 29(9), 1986 (not retrieved); Boxer project, https://boxer-project.github.io/ ; https://github.com/boxer-project/boxer-sunrise
18. J. F. Pimentel, L. Murta, V. Braganholo, J. Freire, "A Large-scale Study about Quality and Reproducibility of Jupyter Notebooks", MSR 2019. http://www.ic.uff.br/~leomurta/papers/pimentel2019a.pdf
19. S. Chattopadhyay, I. Prasad, A. Z. Henley, A. Sarma, T. Barik, "What's Wrong with Computational Notebooks? Pain Points, Needs, and Design Opportunities", CHI 2020. https://www.microsoft.com/en-us/research/publication/whats-wrong-with-computational-notebooks-pain-points-needs-and-design-opportunities/
20. S. Macke, H. Gong, D. J.-L. Lee, A. Head, D. Xin, A. Parameswaran, "Fine-Grained Lineage for Safer Notebook Interactions", 2020-2021 (VLDB 2021 [unverified venue]). arXiv:2012.06981
21. Observable runtime README, https://github.com/observablehq/runtime ; Notebook Kit README (Notebooks 2.0 technology preview), https://github.com/observablehq/notebook-kit
22. Pluto.jl, https://plutojl.org/ ; https://github.com/fonsp/Pluto.jl
23. marimo documentation, "Reactivity", https://docs.marimo.io/guides/reactivity/ ; overview, https://docs.marimo.io/
24. Livebook. https://livebook.dev/
25. Darklang: P. Biggar, "What is Dark?" (28 Feb 2019), https://blog.darklang.com/what-is-dark/ ; "Trace-driven development", https://docs.darklang.com/discussion/trace-driven-development ; "Evolving Dark's tracing system" (28 Jul 2020) and "Dark and the long term" (23 Jun 2020), https://blog.darklang.com/ ; blog index (seed round, Jul 2019; "Darklang is going all-in on AI", Mar 2023; "Goodbye Dark Inc. - Hello Darklang Inc.", Jun 2025); "Winding Down Darklang-Classic" (2025), https://blog.darklang.com/winding-down-darklang-classic/
26. VCR (Ruby). https://github.com/vcr/vcr
27. Val Town, https://www.val.town/ (2026); Replit not re-verified.
28. R. O'Callahan, C. Jones, N. Froyd, K. Huey, A. Noll, N. Partush, "Engineering Record And Replay For Deployability", USENIX ATC 2017 (extended report arXiv:1705.05937); rr project, https://rr-project.org/ (5.9.0).
29. Pernosco. https://pernos.co/
30. Microsoft, "Time Travel Debugging overview" (Nov 2025). https://learn.microsoft.com/en-us/windows-hardware/drivers/debuggercmds/time-travel-debugging-overview
31. E. T. Barr, M. Marron, "Tardis: Affordable Time-Travel Debugging in Managed Runtimes", OOPSLA 2014, https://www.microsoft.com/en-us/research/publication/tardis-affordable-time-travel-debugging-in-managed-runtimes/ ; https://github.com/nodejs/node-chakracore (archived 15 Oct 2020).
32. M. James, "Time Travel made Easy" (24 Sep 2014); E. Czaplicki, "The Perfect Bug Report" (14 Nov 2016). Sources of both posts: https://github.com/elm/elm-lang.org/tree/master/pages/news
33. A. J. Ko, B. A. Myers, "Designing the Whyline", CHI 2004, https://faculty.washington.edu/ajko/papers/Ko2004Whyline.pdf ; "Debugging Reinvented: Asking and Answering Why and Why Not Questions about Program Behavior", ICSE 2008, https://faculty.washington.edu/ajko/papers/Ko2008JavaWhyline.pdf
34. Replay.io (Oct 2026). https://www.replay.io/
35. W. Wilson, "Is something bugging you?", Antithesis blog, 13 Feb 2024, https://antithesis.com/blog/is_something_bugging_you/ ; FoundationDB, "Simulation and Testing", https://apple.github.io/foundationdb/testing.html
36. tokio-rs/loom README, https://github.com/tokio-rs/loom ; awslabs/shuttle README, https://github.com/awslabs/shuttle
37. D. Neil, "Testing concurrent code with testing/synctest", The Go Blog, 19 Feb 2025. https://go.dev/blog/synctest
38. J. Edwards, "Subtext Retrospective" (2025). http://www.subtext-lang.org/retrospective.html
39. Eve repository README. https://github.com/witheve/Eve
40. Language Server Protocol 3.17 (Inline Value request, since 3.17), https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/ ; Debug Adapter Protocol (`stepBack`, `reverseContinue`), https://microsoft.github.io/debug-adapter-protocol/specification
41. In this repository: [`smalltalk.md`](../languages/smalltalk.md), [`dart.md`](../languages/dart.md), [`unison.md`](../languages/unison.md), [`elm-purescript.md`](../languages/elm-purescript.md), [`testing.md`](../topics/testing.md).
