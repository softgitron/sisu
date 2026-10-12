# Esoteric, art and live-coding languages

## Why this family matters for Sisu

Sisu has committed to hot function replacement in debug builds (D-053),
an interpreter-first implementation (D-052) and a teaching profile (Q-41).
Live coders edit programs while they run as the normal way of working,
with time as the output; esolang designers vary notation and model for
their own sake. Together they answer three questions: what a hot swap
must guarantee so that a running loop changes *in time*; whether time
should be a value the program advances rather than a side effect of
`sleep`; and whether any non-linear notation has a real use.

## Systems covered

| System | Years | Status (Oct 2026) | Central idea |
|---|---|---|---|
| Befunge-93, Funge-98 | 1993, 1998 | Frozen specs, hobby implementations | Code on a grid; the instruction pointer moves in space; `t` splits pointers |
| Piet | 2001 | Hobby | Programs are images; commands are colour transitions |
| Brainfuck | 1993 | Hobby; standard teaching vehicle | Eight commands; a 240-byte compiler |
| INTERCAL, C-INTERCAL | 1972, 1990 | C-INTERCAL maintained (Raymond) | Parody; `COME FROM` |
| Whitespace, Folders | 2003, 2015 | Hobby, art | Only invisible characters; a program that is a directory tree |
| Shakespeare, Chef, Rockstar, LOLCODE | 2001, 2002, 2018, 2007 | Rockstar 2.0 shipped Nov 2024; others dormant | Programs that read as plays, recipes, lyrics, memes |
| Malbolge | 1998 | Hobby | Hostile by design; first program found by search |
| Thue, Fractran, Unlambda, Iota, Funciton, Hexagony | 2000, 1987, 1999, 2001, 2011, 2015 | Esolang wiki entries | Rewriting, prime-exponent registers, combinators, 2D dataflow, hexagonal grids |
| Esolang wiki | 2005- | 7,379 languages listed | A catalogue of the design space |
| ChucK | 2003- | Active (1.5.5.8) | "Strongly-timed": `now` is a variable |
| Sonic Pi | 2012- | Active (v5.0.0 listed) | `live_loop`; virtual-time `sleep`; built for schools |
| TidalCycles, Strudel | 2009, 2022 | Active | Patterns as pure functions of rational, cyclic time; mini-notation |
| Impromptu, Extempore | 2005, c. 2011 [unverified] | Extempore active (v0.11) | Temporal recursion; hot-swapped LLVM-compiled code |
| Orca | alpha Dec 2018, 1.0 May 2019 | Active | A grid of letter-operators evaluated every frame |
| Hydra, Shadertoy | 2017, 2013 | Active | One function from pixel coordinate to colour, run on the GPU |
| Processing, p5.js | 2001, 2013 | Active; p5.js 2.0 (2025) | The "sketch": `setup`/`draw` with immediate visual output |
| PICO-8 | 2015 | Active (0.2.7, Aug 2025) | A fantasy console; 8,192-token cartridges |
| TOPLAP, ICLC | 2004, 2015- | 10th ICLC in 2026 | The live-coding community and its conference |

## The ideas

### Time as a value the program advances

ChucK (2003) made time a first-class, typed value: `time` and `dur` are
native types, `now` "holds the current logical time" and "only changes
when it is explicitly modified"; writing a duration to `now` suspends the
process until that logical time [4]:

```chuck
while( true ) { Std.rand2f( 30, 1000 ) => s.freq; 100::ms => now; }
```

Sonic Pi supplies the before-and-after. Version 1's POSIX-style `sleep`
"resulted in badly timed music, especially when multiple musical threads
were executing concurrently": an iteration lasted its compute time plus
its sleeps, so loops started together drifted apart [1]. Version 2 kept
the syntax and changed the meaning: each thread has a *virtual time*
advanced only by `sleep`; `sleep t` blocks until `t` seconds have passed
since the previous `sleep`, subtracting time spent computing; effects
are stamped with virtual time plus a `scheduleAheadTime` (0.5 s in the
paper's figure) and sent early; overruns give timing warnings, and a
thread too far behind is stopped with a "time exception". The authors
formalise this as a *time system* proved sound in a Haskell monad, "to
draw analogy with traditional notions of type system and type safety"
[1]; Extempore likewise schedules "ahead of the scheduled time" [5].

*Evidence.* Designer accounts, not measurements: the temporal model "is
easy to understand by even complete beginners, including children" [1];
Sonic Pi was built for the UK computing curriculum of September 2014 and
"typically 11-12 year-old children" [1], "within classrooms in close
collaboration with teachers" [2]. "Over a million users" appears only in
secondary sources [unverified]; sonic-pi.net gives no count [3].
*Transplants.* General-purpose languages have the mechanism, not the
default: tokio's `Interval` ticks on deadlines with a chosen missed-tick
policy (`Burst`, the default, `Delay` or `Skip`) [32], but the plain
`sleep(d)` of Python, Go, Rust and Sisu's `clock.sleep` is Sonic Pi v1.

### Live loops: replacement at a phase boundary

A Sonic Pi `live_loop` can be redefined "whilst they're still
running"; "you'll hear the change next time the loop goes round"; each
pass emits a cue other loops can `sync` to; loops "each manage their own
time"; a body with no `sleep` is stopped by a "safety system" [2][3].
Extempore gets the same effect from a function that reschedules itself:

```scheme
(define foo (lambda () (play-note (now) synth 60 80 *second*)
                       (callback (+ (now) *second*) 'foo)))
```

"You can re-evaluate a function while it is temporally recursing",
"provided that the signature of the method does not change, i.e. same
arguments and same name" [5], the restriction D-053 adopted
independently; `callback` names the function by symbol, not by closure,
which is why a redefinition reaches the next call. Sorensen calls this
practice "cyber-physical programming" [6]; Extempore, successor to
Impromptu (2005) [7], swaps native code, since xtlang is "a
statically-typed lisp that compiles to LLVM IR at runtime" [5][8], the
closest precedent for D-053's compiled debug builds. A re-evaluated Tidal
pattern is a new pure function of cycle time, in phase with nothing to
migrate [10]. Erlang reached the same design for servers: a process
moves to new code only when it makes "a fully qualified function call"
(`m:loop()`) [31], used mostly in development
([erlang-elixir.md](../languages/erlang-elixir.md)). The convergence:
swap at the top of an iteration, keep state in values, refuse
(Extempore) or restart (D-053) when state would need reshaping. None
solves migration; Sonic Pi avoids mixed versions only because a loop
body is one closure replaced as a unit.

### Liveness as a ladder, and the rule that the music never stops

Tanimoto's levels (four in 1990, six in 2013) are 1 *informative*, 2
*significant* (an executable flowchart), 3 *responsive* ("edit-triggered
updates"), 4 *live* ("stream-driven updates"), 5 *tactically predictive*
(running predicted next edits in sandboxes) and 6 *strategically
predictive* (inferring "gross functionality") [13]. His criticisms stand:
level 4 is pointless for programs that run for milliseconds, edits behind
the point of execution do nothing, and live edits create "semantic
inconsistencies" (deleting a Pac-Man mid-game in Scratch leaves an
inoperable character) [13]. TOPLAP (February 2004) requires
performers' screens to be projected [14]; nine ICLCs ran from Leeds 2015
to Barcelona 2025 [15]; the handbook is *Live Coding: A User's Manual*
(MIT Press, open access, 2022) [16]. D-053 is level 4 for body edits and
level 3 (restart) for signature or layout edits. All these systems share
one rule by construction rather than policy: an evaluation that fails to
compile changes nothing, so the old loop keeps playing [inference].

### Patterns as functions of time, and the mini-notation

TidalCycles (Alex McLean, 2009, a Haskell DSL) represents a pattern as a
query from a time span to the events in it, with `type Time = Rational`
and the cycle as unit; rhythm is a string mini-notation [9][10]:

```haskell
d1 $ every 4 (fast 2) $ s "bd*2 [sn cp] <hh oh> bd(3,8)"
```

`*2` repeats, `[ ]` subdivides, `< >` alternates per cycle, `(3,8)` is
a Euclidean rhythm, `~` rests. The model crossed hosts: Strudel
(JavaScript, 2022, Felix Roos and McLean) is "an official port of
Tidal's pattern engine" [11], and a NIME 2026 paper names the family's
defining features: "pure functions of rational time", "expressive
'mini-notation'", combinator libraries, in-code visualisation [12]. No
adoption numbers exist. The notation never became host syntax: it is a
string a library parses, the form Sisu's comptime string DSLs exist for
(the `sql.query` example).

### One function per pixel

Shadertoy (February 2013, Íñigo Quílez and Pol Jeremias) publishes
programs that are one function from pixel coordinate and time to colour;
Wikipedia reports over 52,000 public shaders by mid-2021 (citation
needed) and a 2018 prize page claims 85,000, so counts are soft [18].
Hydra (Olivia Jack, begun for a workshop at the December 2017 ICLC)
chains `osc(20, 0.1, 0.8).rotate(0.8).out()` and compiles to WebGL; "the
base element is a transformation from coordinates to color", and "the
GPU runs a program simultaneously on each pixel" [17]. It may be the
most used implicitly parallel model among non-experts [judgement], and it
works because each invocation is pure and writes only its own output
(transplants: [parallel-languages.md](parallel-languages.md)).

### The sketch

Processing (Ben Fry and Casey Reas, spring 2001) calls a program "a
sketch" on purpose: planning software fully before coding "prematurely
cuts off possible outcomes". It was built for "visual people (designers,
artists, architects)" and by 2018 was "used more frequently in high
schools, mostly for classes in math and science" [19]. p5.js (Lauren
McCarthy, 2013) carried it to the browser: "almost half a million
visitors" a month to site and editor (March 2025), with 2.0 the editor
default from August 2026 [20]. What transfers is the shape: `setup()`
once, `draw()` every frame, canvas and input already packaged, so the
first program draws something; the ceiling (a big sketch is global
mutable state in one file) is a common complaint [anecdote].

### Constraint as a feature

PICO-8 (Joseph White, Lexaloffle, April 2015) fixes a 128x128 display,
16 colours, 32 KB cartridges stored as PNG images and "a maximum of 8192
tokens" of Lua, limits "carefully chosen to be fun to work with,
encourage small but expressive designs" [21]. What is free matters:
"commas, periods, LOCALs, semi-colons, ENDs, and comments are not
counted", so the budget measures size without punishing comments or
structure [21]. itch.io lists 10,147 games made with PICO-8 (12 Oct
2026); *Celeste Classic*, made in four days for a jam, became the 2018
*Celeste*; "fantasy console" now names a genre (TIC-80, Picotron) [21].
A small fixed machine is learnable completely, so effort moves to
composition.

### Space as program

Befunge-93 (Chris Pressey, 1993) runs on an 80x25 grid; `> < ^ v ?` steer
the instruction pointer, `p` writes instructions into the grid, and the
goal was to be "as difficult to compile as possible" [24]. Funge-98
dropped the size limit, added one and three dimensions and `t`, which
splits a pointer; pointers take one tick each "in a sequential circular
loop" [24]. Piet's commands are hue and lightness changes between colour
blocks [24][25]; Hexagony's source is a hexagon run by six pointers;
Funciton draws dataflow in box-drawing characters [24]. Orca (Hundred
Rabbits) is the one people perform with: "every letter of the alphabet is
an operation, where lowercase letters operate on bang, uppercase letters
operate each frame"; `D` "bangs on modulo of frame", `:` sends MIDI [22].
Its grid is program and state display at once: values appear in cells as
they change, adjacency is wiring, the frame counter is time. The cost is
what Pressey designed for: spatial control flow and self-modification
defeat static analysis. No general-purpose language adopted 2D source;
the 2D medium that works is the spreadsheet
([spreadsheets-and-end-user.md](spreadsheets-and-end-user.md)).

### Minimal models: tarpits as tests, teaching vehicles and a catalogue

Brainfuck (Urban Müller, 1993) exists so that Müller could write "the
smallest possible compiler for the Amiga OS, version 2.0" (240 bytes);
its eight commands reduce to Böhm's P'' (1964), hence Turing-complete
[24]. Smallness makes it a standard implementation exercise:
Bendersky's 2017 series goes from a naive interpreter to one "almost 10x
faster", then an x64 JIT, LLVM and Python, because BF "really boils
programmability down to the essentials" [26]. The Esolang wiki (since
April 2005; 8,222 articles, 7,379 languages on 12 Oct 2026 [23]) also
catalogues models mainstream languages never tried: Thue (2000),
nondeterministic string rewriting until no rule applies; Fractran
(Conway, 1987), fractions applied to one integer whose prime exponents
are the registers; Unlambda (Madore, 1999), combinators with `d` (delay)
and `c` (call/cc) and no variables; Iota (Barker, 2001), a single
combinator `ι x = x S K` [24]: reference points for the sibling files,
not features. Malbolge (Ben Olmstead, 1998) is the limit: its first
program was found two years later by Andrew Cooke's beam search, not
written by a person [29].

### Syntax as skin: what the joke languages show

Shakespeare (Karl Hasselström and Jon Åslund, 2001) uses characters as
variables, values a noun at 1 or -1 with each adjective doubling it, and
uses scenes as `goto` labels ("let us proceed to scene II") [24]. Chef
(David Morgan-Mar, 2002) wants recipes "easy to prepare and delicious"
over a stack machine [24]. Rockstar (Dylan Beattie, 2018; 2.0 on 30
November 2024) satirises "rockstar developer" job ads [24][30]; its
poetic literals take digits from word lengths, so `Tommy was a lean mean
wrecking machine` sets Tommy to 14487 [specification; unverified here].
LOLCODE (Adam Lindsay, 2007) opens with `HAI` and `CAN HAS STDIO?`, a
"corruption" of `#include <stdio.h>` that implementations ignore [29].
Whitespace (Edwin Brady and Chris Morris, Durham, 1 April 2003) gives
meaning only to space, tab and newline; Brady went on to develop Idris
[29]. Folders (Daniel Temkin, 2015) is a directory tree whose largest
program "is still 0 bytes" [24]. Under every skin is a small
conventional machine: syntax is cheap to vary and semantics is not, the
economy D-055 follows. Whitespace is the reductio of significant
whitespace; indentation that agrees with what the reader sees is not
absurd, which is why Sisu forbids tabs (D-055).

### COME FROM: a joke that became tracing and aspects

R. Lawrence Clark's 1973 *Datamation* parody ("We don't know where to
GOTO if we don't know where we've COME FROM") was implemented in
C-INTERCAL (1990, Eric S. Raymond), later with computed `COME FROM`;
Threaded INTERCAL turns several `COME FROM`s on one label into threads
[27]. It became real twice. As observation: a breakpoint is a `COME FROM`
into the debugger, and Richie Hindle's April 2004 `comefrom` for Python
was built on the trace hooks [27]. As semantics: aspect-oriented advice
runs code at join points named elsewhere, which "AOP Considered Harmful"
(Constantinides, Skotiniotis, Störzer, EIWAS 2004) called "closely
analogous to the joke COME FROM statement" [28]. Inverted control flow is
indispensable when it can only observe, harmful when it can change what
the program does.

## Transferable ideas for Sisu

The answer to the central question: yes to a live loop, as boundary-
aligned replacement on top of D-053 plus drift-free time as a library;
notation only as comptime-checked string DSLs; no 2D source.

**1. Version-consistent hot replacement at loop boundaries ("live loops
for free").** D-053 swaps a body at its next call and lets frames in
flight finish on the old one, so a loop calling a top-level function
each iteration is already a live loop. Three refinements make it land on
the beat:

```sisu
def main(sys: System) -> ! AppError:
    var song = Song.new()
    let beat = live.Ticker(sys.clock, every=Duration.millis(500))
    while True:
        beat.wait()?                  # reload boundary: adopt the newest code here
        bar(mut song, beat.count)     # bar and all it calls run one code version

def bar(mut s: Song, n: int):         # edit and save: the next beat runs this
    s.play(Drum.Kick)
    if n % 4 == 3:
        s.play(Drum.Snare)
```

(a) A successful `--watch` check publishes all changed bodies as one
*epoch* of the function table; a task adopts the newest epoch at task
start and at `live.boundary()` (which `Ticker.wait` calls), and resolves
every call in between in its adopted epoch, so no iteration mixes an old
`bar` with a new helper; tasks without boundaries keep D-053's next-call
rule. (b) A save that fails to check publishes nothing; the old epoch
keeps running beside the diagnostic. (c) A top-level function used as a
value (a callback, a route table) holds its slot, not its body, in debug
builds, like Extempore's `callback 'foo`. *Cost:* an epoch word per task
and a versioned table; old epochs are freed by the bodies' existing
reference counts; nothing in release builds. *Conflicts:* refines D-053,
whose next-call rule lets one iteration mix versions; adds (c) to the
debug ABI. *Confidence:* high for the boundary semantics (four systems
converged on it independently), medium for the API.

**2. Drift-free time in `std.time`.** A deadline `Ticker` with an
explicit missed-tick policy (tokio's three) and a `Timeline` with Sonic
Pi v2 semantics, both over the `Clock` capability:

```sisu
var tl = Timeline(clock)              # virtual time starts at clock.now()
for note in melody:
    out.note(note, at=tl.now() + AHEAD)   # stamped in virtual time, sent early
    tl.sleep(Duration.millis(250))    # virtual += 250 ms; waits only the remainder
```

All time flows through `Clock`, so the same code is real-time in
production and deterministic under `Clock.frozen_at` and `clock.advance`
in tests. *Cost:* small (a library over `Clock` and `Instant`).
*Conflicts:* none; `clock.sleep` stays the primitive for timeouts.
*Confidence:* high that it is needed, medium that it belongs in
`std.time` rather than `std.x`.

**3. Notations as comptime-checked libraries, with Tidal's mini-notation
as the test case.**

```sisu
const GROOVE: pattern.Pattern[pattern.Sound] = comptime pattern.parse(
    r"bd*2 [sn cp] <hh oh> bd(3,8)")  # parse errors point into the literal

def events_in(cycle: int) -> list[pattern.Event[pattern.Sound]]:
    return GROOVE.query(pattern.Arc.cycle(cycle))   # a pure function of time
```

A pattern is a pure function of time, so replacing `GROOVE` under
`--watch` is in phase automatically. *Cost:* none to the language; the
package needs a rational time type, which std does not plan.
*Conflicts:* none; it exercises the comptime string-DSL path the
non-goals chapter relies on. *Confidence:* medium as a showcase, low for
std.

**4. Observation-only `COME FROM`: tracepoints in the tools, never in
the language.** `sisu run --watch --trace song.bar --trace
"http.get:args,result"` attaches to a definition's table slot (the D-053
indirection) to log, count or break, never to return a value or alter
control flow. *Cost:* low; the debug backend reuses the slot.
*Conflicts:* none; output goes to the tool. *Confidence:* medium.

**5. A constrained sketch template for the teaching profile, and tarpit
exercises.** `sisu new --template sketch`: `update`/`draw` hot-swapped at
60 frames per second on a small fixed canvas, with a PICO-8-style size
meter that counts tokens but not punctuation or comments (the library
behind it is idea 3 of [game-and-narrative-scripting.md](game-and-narrative-scripting.md)).
A Brainfuck interpreter doubles as a 60-line interpreter benchmark with
Bendersky's ladder as baseline. *Cost:* the template is cheap, the
library is not. *Confidence:* medium-low.

## Dead ends

- **2D, image or self-modifying source** (Befunge, Piet, Hexagony,
  Funciton, Orca, Malbolge): defeats compilation, tooling and replacement
  by signature hash; the 2D medium that works is the spreadsheet.
- **Natural-language skins** (Shakespeare, Chef, Rockstar, LOLCODE): the
  semantics underneath is ordinary ([natural-language-and-literate.md](natural-language-and-literate.md)).
- **`COME FROM`, advice or pointcuts in the language**: the AOP critique.
- **Cyclic time or patterns as core types**: a library (14-non-goals).
- **Killing a late thread** (Sonic Pi's time exception): report lateness
  instead. Image-based live environments stay out of scope (Q-22).

## Sources

1. S. Aaron, D. Orchard, A. F. Blackwell, "Temporal Semantics for a Live Coding Language", FARM 2014, pp. 37-47. https://www.cs.kent.ac.uk/people/staff/dao7/publ/farm14-sonicpi.pdf (2014)
2. S. Aaron, "Live Coding Education", *The MagPi* Educator's Edition. https://sonic-pi.net/files/articles/Live-Coding-Education.pdf (c. 2016)
3. Sonic Pi home page and tutorial. https://sonic-pi.net/ ; https://sonic-pi.net/tutorial.html (Oct 2026); v2.1.0 "Core" release notes (introduces `live_loop`) https://zenodo.org/records/213723
4. ChucK, home page and "Time and Timing". https://chuck.stanford.edu/ ; https://chuck.stanford.edu/doc/language/time.html (Oct 2026)
5. Extempore, "Time in Extempore" and README. https://extemporelang.github.io/docs/overview/time/ ; https://github.com/digego/extempore (Oct 2026)
6. A. Sorensen, H. Gardner, "Programming with Time: Cyber-physical Programming with Impromptu", OOPSLA/Onward! 2010, pp. 822-834. https://doi.org/10.1145/1869459.1869526 (2010)
7. "Impromptu (programming environment)", Wikipedia. https://en.wikipedia.org/wiki/Impromptu_(programming_environment) (Oct 2026)
8. A. Sorensen, *Extempore: The design, implementation and application of a cyber-physical programming language*, PhD thesis, ANU. https://openresearch-repository.anu.edu.au/handle/1885/144603 (2018)
9. A. McLean, "Making Programming Languages to Dance to: Live Coding with Tidal", FARM 2014, pp. 63-70. https://userbase.tidalcycles.org/Academic_publications.html (2014)
10. TidalCycles documentation, "What is a pattern" and "Mini-notation". https://tidalcycles.org/docs/innards/what_is_a_pattern ; https://tidalcycles.org/docs/reference/mini_notation/ (Oct 2026)
11. "TidalCycles", Wikipedia; Strudel, "Mini-notation". https://en.wikipedia.org/wiki/TidalCycles ; https://strudel.cc/learn/mini-notation/ (Oct 2026)
12. A. McLean, F. Roos, M. Kaney et al., "Uzulangs: a Community of Musical Pattern Languages", NIME 2026. https://nime.org/web_archive/2026/proceedings/53.html (2026)
13. S. L. Tanimoto, "A Perspective on the Evolution of Live Programming", LIVE 2013, pp. 31-34 (extends Tanimoto, "VIVA: A visual language for image processing", *JVLC* 1(2), 1990). https://homes.cs.washington.edu/~tanimoto/site/wp-content/uploads/2024/10/LiveProgramming2013.pdf (2013)
14. "Live coding", Wikipedia (TOPLAP, manifesto). https://en.wikipedia.org/wiki/Live_coding (Oct 2026)
15. TOPLAP, "Call for hosts: International Conference on Live Coding 2026". https://blog.toplap.org/?p=8062 (2025)
16. A. Blackwell, E. Cocker, G. Cox, A. McLean, T. Magnusson, *Live Coding: A User's Manual*, MIT Press (open access). https://livecodingbook.toplap.org/ (2022)
17. Hydra documentation and README; P. Kirn, interview with Olivia Jack, CDM, 22 Feb 2019. https://hydra.ojack.xyz/docs/ ; https://github.com/hydra-synth/hydra ; https://cdm.link/2019/02/hydra-olivia-jack/
18. "Shadertoy", Wikipedia; STARTS Prize 2018 nomination page. https://en.wikipedia.org/wiki/Shadertoy ; https://starts-prize.aec.at/en/?p=1581
19. C. Reas, B. Fry, "A Modern Prometheus", Processing Foundation, 29 May 2018; Processing Foundation history. https://processingfoundation.org/blog/a-modern-prometheus/ ; https://processingfoundation.org/about/history
20. Kit (p5.js project lead), "[dev updates] p5.js 2.0: You Are Here", 31 Mar 2025, updated to Jun 2026. https://discourse.processing.org/t/dev-updates-p5-js-2-0-you-are-here/46130
21. Lexaloffle, PICO-8 manual v0.2.7; "PICO-8", Wikipedia; itch.io "made with PICO-8". https://www.lexaloffle.com/dl/docs/pico-8_manual.html ; https://en.wikipedia.org/wiki/PICO-8 ; https://itch.io/games/made-with-pico-8 (Oct 2026)
22. Hundred Rabbits, Orca README and itch.io page. https://github.com/hundredrabbits/Orca ; https://hundredrabbits.itch.io/orca (Oct 2026)
23. Esolang wiki, main page. https://esolangs.org/wiki/Main_Page (12 Oct 2026)
24. Esolang wiki articles Befunge, Funge-98, Piet, Hexagony, Funciton, Thue, Fractran, Unlambda, Iota, Brainfuck, Shakespeare, Chef, Rockstar, Folders: `https://esolangs.org/wiki/<name>` (Oct 2026)
25. D. Morgan-Mar, "Piet". https://www.dangermouse.net/esoteric/piet.html (updated 2018)
26. E. Bendersky, "Adventures in JIT compilation: Part 1 - an interpreter", 20 Mar 2017. https://eli.thegreenplace.net/2017/adventures-in-jit-compilation-part-1-an-interpreter/
27. "COMEFROM" and "INTERCAL", Wikipedia. https://en.wikipedia.org/wiki/COMEFROM ; https://en.wikipedia.org/wiki/INTERCAL (Oct 2026)
28. "Aspect-oriented programming", Wikipedia, citing C. Constantinides, T. Skotiniotis, M. Störzer, "AOP Considered Harmful", EIWAS 2004. https://en.wikipedia.org/wiki/Aspect-oriented_programming (Oct 2026)
29. "Whitespace (programming language)", "LOLCODE", "Malbolge", Wikipedia. https://en.wikipedia.org/wiki/Whitespace_(programming_language) ; https://en.wikipedia.org/wiki/LOLCODE ; https://en.wikipedia.org/wiki/Malbolge (Oct 2026)
30. D. Beattie, Rockstar. https://codewithrockstar.com/ (2018-2026)
31. Erlang/OTP, "Compilation and Code Loading". https://www.erlang.org/doc/system/code_loading.html (Oct 2026)
32. tokio, `MissedTickBehavior`. https://docs.rs/tokio/latest/tokio/time/enum.MissedTickBehavior.html (Oct 2026)
