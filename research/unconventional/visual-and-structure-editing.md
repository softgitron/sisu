# Visual, block and structure editing

## Why this family matters for Sisu

Block, node and projectional editors are the only programming tools whose benefits for novices
were measured in classrooms rather than asserted, and the only alternative to text with forty years
of commercial history (LabVIEW, Simulink, Max). They answer a concrete question: can a textual,
indentation-based language get the measured benefits of blocks (no syntax errors, a discoverable
vocabulary, faster early learning) through tooling, with text as the only source of truth?

## Systems covered

| System | Years | Status (Oct 2026) | Central idea |
|---|---|---|---|
| Scratch | 2007-; 3.0 on 2 Jan 2019 | 135.1 M registered users, 164.3 M shared projects in the last published statistics (14 Jul 2024) [2] | Blocks shaped so only grammatical programs can be built; click a stack to run it |
| Snap! (formerly BYOB) | BYOB c. 2008 [unverified], Snap! 2011- | v12.2.3 (Oct 2026); Berkeley's BJC course [3] | Scratch plus first-class procedures, lists and continuations; user-defined blocks |
| Blockly | 2011- | Moved from Google to the Raspberry Pi Foundation on 10 Nov 2025 [4] | A library: a definition and one code generator per target language for each block |
| App Inventor | 2010 (Google), 2012- (MIT) | "over 24 million learners", "over 100 million apps" (MIT's claim) [5] | Blocks for Android apps; the palette grows with the components used |
| Microsoft MakeCode | 2016- | micro:bit, Arcade and other editors [6] | Blocks, Static TypeScript and Python round-trip through one hub language |
| Pencil Code / Droplet | 2013- | Site returned HTTP 503 when checked (Oct 2026) | Dual-mode editor: text is primary, blocks are a projection |
| Stride (Greenfoot, BlueJ) | 2015-, 2017- | Shipped in both IDEs [15] | Frames for statements, typing for expressions |
| LabVIEW (G) | 1986- | LabVIEW 2026 Q1; NI part of Emerson since Oct 2023; NXG ended 2021 [17] | Dataflow wires behind an instrument "front panel" |
| Prograph | 1982-late 1990s | Withdrawn; the Marten IDE keeps the language [21] | Object-oriented dataflow |
| Max/MSP, Pure Data | 1988-, 1996- | Max 9; Pd 0.57 [19] | Patching boxes for real-time audio; deterministic logical time |
| Simulink | 1990s- | Active (MathWorks) [21] | Block diagrams for control systems, with C code generation |
| Node-RED | 2013- | OpenJS Foundation; "over 5000" community nodes [20] | Flows stored as JSON; JavaScript inside function nodes |
| TouchDesigner, vvvv | 2000s-, 1998- (gamma 2020) | Active [21] | Real-time media graphs; Python (TD) or C#/F# (vvvv) inside nodes |
| Grasshopper | 2007- | Bundled with Rhino since 6; Grasshopper 2 in alpha [21] | Parametric geometry graphs |
| Unreal Blueprints | 2014- | Active; see `game-and-narrative-scripting.md` | Visual scripting over a C++ engine |
| Godot VisualScript | 2018-2023 | Removed in Godot 4.0 [22] | A node-graph alternative to GDScript |
| JetBrains MPS, mbeddr | 2003-, 2012- | MPS 2026.1; mbeddr's README targets MPS 2024.1 [24] | Projectional editing and language composition |
| Enso (formerly Luna) | mid-2010s- | Sold as "Enso Analytics", a data-prep product; nightlies in 2026 [25] | One program shown as a node graph and as text |
| Lamdu | 2011- | 0.8.1 "technology preview" [26] | Projectional, typed and live; no text files |
| Isomorf | late 2010s [unverified] | Site is a title page only [32] | Structure editor over a code graph [unverified] |
| Hazel | 2016- | Research and teaching (POPL 2017-2025) [27][28] | Every edit state is well-typed and runs |
| Sketch-n-Sketch | 2015-2020 | Research prototype [29] | Edit the program by dragging its SVG output |

## The ideas

### 1. Shapes that make syntax errors unrepresentable

Scratch's designers state the mechanism: "Scratch blocks are shaped to fit together only in ways
that make syntactic sense. Control structures (like forever and repeat) are C-shaped ... Blocks
that output values are shaped according to the types of values they return: ovals for numbers and
hexagons for Booleans" [1]. Weintrop and Wilensky describe blocks as structured editors whose unit
is an AST node, "providing constraints to ensure a node can only be added to the program's AST in a
valid way" [10]. Scratch also reports no errors at run time [unverified wording], and a 2023 paper
notes that block environments for children "often provide no support for debugging at all" [34].

What it buys, measured: in Price et al.'s ICER 2016 study, middle-school students using Stride's
frame editor "progressed through the activity more quickly and completed more objectives", and
spent less time on purely syntactic edits and with non-compilable code, than Java-editor users [15].

Where it fails: viscosity. Bau et al.'s example is rewriting `(a/2 + b/2)` as `(a+b)/2`, which in
blocks needs "more gestures and more forethought than making the edits in text"; blocks are
also low-density, hard to search and navigate, and hard to version-control [8]. The 1980s
syntax-directed editors failed on the same point. The Cornell Program Synthesizer's authors held
that "Programs are not text; they are hierarchical compositions of computational structures"
(Teitelbaum and Reps 1981, quoted in [8]), and a paper on the Synthesizer Generator conceded that
"Program editing will be considerably slower than normal keyboard entry" (quoted in [24]).

Transplants into text that worked keep text editing for expressions. Stride "maintains the look of a
program as essentially a textual, if coloured, document" [15]. Hazel's tylr does keyboard-driven
structure editing in which missing pieces become tracked "obligations" rather than forbidden states
(TyDe 2022, VL/HCC 2023) [28]. Paredit-style editing of ordinary text by s-expression is used by 81%
of experienced Clojure developers (2025 survey, [`clojure.md`](../languages/clojure.md)) [35]. Dark
launched in 2019 with a structured editor in which syntax errors cannot be entered; its 2025
successor is being built as a CLI with a VS Code extension and a language server [30].

### 2. Recognition over recall: the palette

Bau et al. attribute much of blocks' learnability to vocabulary. A small language "typically
involves a vocabulary of about 100-200 words ... Scratch is similar, with 130 blocks", and palettes
"differ from autocomplete menus ... because they persist instead of disappearing and they organize
concepts by topic instead of by name"; App Inventor's palette grows with the components a program
uses [8].

The cleanest measurement comes from a dual-mode editor where students could choose. In Holbert and
Weintrop's Pencil Code logs, high-school and graduate students used blocks 92% and 91% of the
time. Of 217 switches from text to blocks, 65.4% were made to add a command, 67.1% of those added a
block type not yet used in the program, and when a control block (`repeat`, `if`) was added after
a switch it was its first use 86.7% of the time [13]. Students went to the blocks to find vocabulary
and to avoid syntax they had not typed before, not to read.

Text tools found the same need. mbeddr's authors added "a palette ... similar to diagram editors or
the formula editor in MS Word" to MPS because constructs entered differently from how they display
are "hard to find out" [24]. MakeCode derives its palette from code: "Each top-level TypeScript
namespace is used to populate a category in the Block Editor toolbox", and comment attributes on an
ordinary typed API define the block [7]:

```ts
//% color=190 weight=100 block="Basic"
namespace basic {
    //% blockId=device_show_number block="show|number $v"
    export function showNumber(v: number, interval: number = 150): void { }
}
```

The block shows one of the two parameters; the return type picks its shape (`void`: a statement) [6].

### 3. What the classroom studies measure

The best-controlled comparisons are Weintrop and Wilensky's, using isomorphic block and text
versions of one environment, with the same teacher and curriculum:

- ICER 2015 (a "commutative" assessment, each question in both forms): students did better in the
  block form on iterative logic (d = 1.57), conditionals (d = 0.41) and functions (d = 0.41), not
  significantly on variables, and identically on comprehension questions (d = 0.01) [9].
- TOCE 2017 (60 students, 5 weeks): both groups improved, blocks from 54.3% to 66.6% and text from
  51.6% to 58.8% (d = 0.96 against 0.50), and the blocks group scored higher even on the *text*
  questions. Text students rated their experience as closer to what professionals do; confidence
  and enjoyment did not differ [10].
- Computers & Education 2019 (the same students after 10 weeks of Java): "the difference in
  conceptual learning that emerged after five weeks ... fades after 10 weeks in Java. No
  differences in programming practices were found" [11]. In a 2020 seminar Weintrop reported that
  a third, hybrid condition did best overall, and he recommends dual-modality tools: "why not
  both?" [12] (the hybrid result is from the seminar summary).

Other studies point the same way. Price and Barnes (ICER 2015) found a blocks group spent less time
off task and completed more goals in less time, with no difference in attitude [14]. Bau et al.
report a CMU Java course that began with Alice, which generated Java from blocks, scoring 10% or
more higher on every section of the same Java exam, and note "potential challenges" when courses
switch to text without a mediated transfer [8]. MakeCode's designers, with several million
micro:bits distributed, write: "More research is required to evaluate if such a multi-mode editor
approach is useful" [6]. The defensible summary: blocks buy novices an early, measurable
gain, concentrated in control structure and vocabulary, that fades within a term after the move to
text, at a cost in perceived authenticity.

### 4. Dual-mode editing with text as the source of truth

Bau et al. describe the architecture of the dual-mode editors that work (Pencil Code, Code.org's App
Lab, BlockEditor, Tiled Grace): "the text code is the primary representation of the program, and
blocks are a projected user interface view derived by parsing". The price is that the editor "must
allow syntax errors that would not have been possible in blocks", and "complex errors can prevent
mode switching" [8].

MakeCode is the largest instance. Static TypeScript is the hub: "Both Blocks and Static Python are
converted to Static TypeScript before being compiled" [7]. Blocks are produced from STS by a
decompiler (`pxtcompiler/emitter/decompiler.ts`), Python by a converter and a Python decompiler
(`pxtpy/converter.ts`, `pydecompiler.ts`) [7]. Round-tripping works only for code in the shape each
block generates; anything else becomes a grey block that preserves the text and converts "right
back into the original JavaScript" [7]. The documented cases are ordinary code:

```ts
for (let x = 2; x <= 5; x++) { }    // grey: the loop variable must start at 0
for (let x = 0; x > -1; x++) { }    // grey: the condition must use < or <=
```

Block identity is part of the saved program: `blockId` "is serialized in block code", so changing
it, or renaming the function it defaults to, breaks existing projects [7]; and some STS constructs
(classes) are not editable as blocks at all [6]. The lesson for a language: round-tripping is a
property of a canonical pretty-printer plus a decompiler that recognises its output, and everything
outside that subset needs a lossless fallback view.

### 5. Wires as the program: forty years of dataflow notations

Where it wins. LabVIEW has sold continuously since 1986, its diagram mirrored by a front panel of
instrument controls; Simulink is "widely used in automatic control and digital signal processing";
Max and Pd have been computer music's patching languages since 1988 and 1996; Grasshopper is
"endemic in the architectural design world" [21]. Puckette explains Max's fit: "The parallelism so
visually apparent in a Max patch is intended to allow the user to make computer programs that follow
the user's choices", so that patches "could work as musical instruments"; and "Logical time marches
forward in a purely deterministic way ... This determinism is crucial for making Max patches
debuggable" [18]. Green and Petre grant the notation one real strength: dependencies a textual
program hides (`x = 1` here, `y = x + 3` pages later) are visible wires [16].

Where it fails. Green and Petre timed an experienced user making one pre-planned change (adding air
resistance to a rocket simulation) in three notations: LabVIEW 508.3 s, Prograph 193.6 s, Basic
63.3 s, "an astonishing ratio of 8:1", because in LabVIEW "all the boxes had to be jiggled about and
many of the wires had to be rebuilt" [16]. A respondent to Whitley and Blackwell's survey of LabVIEW
programmers hired "a real nitpicker to clean up my diagrams" [16]. L. Peter Deutsch's remark that
"you can't have more than 50 visual primitives on the screen at the same time" names the scaling
problem [31]; version control is the other, since a non-textual program defeats ordinary diff and
merge tools [17][8]. Max's designer is blunt: "Musicians have often used Max as a programming
environment, at which Max succeeds only very awkwardly ... The whole notion of control flow, with
loops, conditionals, and subroutines, is easy to express in text languages, but thus far graphical
programming languages haven't found the same fluency"; and because Cycling '74's Max orders fan-out
by screen position, "merely moving boxes on the screen (to tidy them up for example) can change a
patch's behavior" [18].

The survivors share a design: the graph carries topology and text carries computation. Max boxes
hold typed text because "once you have more than ten or twenty distinct icons it becomes difficult
to remember which icon corresponds to which functionality" and text conveys "more information per
square inch" [18], and Gen "lets you combine procedural commands with visual patching" [19].
Node-RED's Function node "allows JavaScript code to be run against the messages" [20];
TouchDesigner's DATs hold Python, Grasshopper has Python and C# Script components, and vvvv's
custom nodes can be written in C# or F# [21].

NI's own retreat shows the cost of a notation only one tool can read. LabVIEW NXG, a new IDE first
released in 2017, ended after users "shared with us some of your concerns around migrating to an
entirely new LabVIEW product": "We will not release new versions of LabVIEW NXG starting in 2022",
and its strengths were folded back into classic LabVIEW [17]. Godot removed VisualScript in Godot
4.0 (March 2023) because it "never gained traction": 0.5% of more than 5,000 poll respondents used
it as their main language, GDScript was easy enough, Godot lacked the high-level game components
that make visual scripting useful in other engines, its examples could not go into the
documentation, and the team concluded that the approach itself was wrong [22]. Prograph's company
went into receivership in 1995 [21].

### 6. Projectional editing and its adoption problem

The promise, in mbeddr's authors' words: projectional editors "support notations that cannot easily
be parsed, such as tables, diagrams or mathematical formulas" and "various methods of language
composition" [24]. mbeddr is the largest test: 70 to 81 languages depending on the count, about 35
of them C extensions (state machines, physical units, components), "roughly 88,000 lines of code,
and around 10 person years", and the basis of Siemens' commercial Embedded Software Designer
[23][24].

The adoption problem is the editor. The FSE 2016 controlled experiment (19 students and industrial
developers) found basic edits become efficient quickly, but "advanced editing (e.g., larger code
modifications or refactorings) requires significantly more experience and understanding of ... the
AST structure", with "a tradeoff between fewer typing mistakes and an increased complexity of code
editing"; such editors "are niche tools, mainly due to their usability problems" [23]. The mbeddr
retrospective names the two primary drawbacks, "the editor feels similar, but not identical, to a
text editor" and "programs cannot be edited, or diff/merged outside the tool", says the effort of
building usable editors "is still too high", and records that diagram merges needed JetBrains to
add `merge-ours`/`merge-theirs` annotations so that moved boxes would not conflict [24]. In 2026
mbeddr's README still targets MPS 2024.1 while MPS is at 2026.1 [24].

Hybrids that keep text pay a different price. Enso's files are text plus a trailing block that maps
byte ranges to node identities [25]:

```
main =
    x = 6
    y = x.foo 5
#### METADATA ####
[[{"index": {"value": 98}, "size": {"value": 5}}, "5fc0c11d-bd83-4ca3-..."], ...]
```

Its documentation warns: "Notice extra newline in the beginning of the `Main.enso` file, it is
important for the precalculated metadata indexes" [25]. The language now ships as a data-prep
product with "No-code or full-code ... two equally first-class interfaces" [25]. Lamdu stores no
text and is a 0.8.1 "technology preview" fifteen years after it began [26].

### 7. Typed holes: every edit state means something

Hazel moves the "no invalid states" guarantee from the editor's shapes into the type system.
Hazelnut's edit actions "operate over statically meaningful incomplete terms", and a term whose type
is inconsistent with its context is placed "inside a hole", which "safely defers the type
consistency check until the term inside the hole is finished" [27]. The POPL 2019 follow-up makes
incomplete programs run: "evaluation proceeds around holes", with "a fill-and-resume operation that
avoids the need to restart evaluation" [28]. Later work adds keyboard-driven gradual structure
editing (tylr), total type error localisation (POPL 2024), collaborative editing (Grove, POPL 2025)
and hole types as context for language models (OOPSLA 2024) [28]. Hazel has no production users,
but half of its idea transfers to a textual language: holes give every edit state a type in the
checker, without constraining how text is typed. Evaluating past a hole does not transfer to code
with effects; stopping at it does (`holes-synthesis-and-ai-era.md`, which also covers synthesis).

### 8. Editing the output

Sketch-n-Sketch "infers a program update in order to match the changes to the output" by
"trace-based program synthesis that takes program execution history into account" ("live
synchronization"); it works for SVG, where output maps closely onto literals [29]. Its sibling
Deuce (ICSE 2018), structural selection and refactoring menus on an ordinary text editor, is the
transferable part [29]. Nobody has shipped output-directed editing for a general-purpose language.

## Transferable ideas for Sisu

The answer to the central question is yes for two of the three measured benefits and partly for
the third. Vocabulary discoverability transfers fully, because it is a property of the palette, not
of blocks. Freedom from syntax errors transfers to whoever uses a structure mode, and its semantic
form (every edit state type-checks) transfers to everyone through holes. The early learning
gain is real but short-lived and has only been obtained with block or frame editors; a teaching
tool can reproduce it, a text editor cannot. None of this needs a source format other than
formatted text. What MakeCode and Enso show is required is a canonical printer plus stable node
identity, which Sisu's formatter and parser decisions nearly provide already.

1. **Text is the only source of truth, and projection-friendly by construction.** The no-options
   formatter (D-030) is the printer for every view, so view-to-text is `fmt(tree)` and
   text-to-view is the error-tolerant parse; whatever the parser cannot place becomes a lossless
   text island (MakeCode's grey block), never a refusal to show the view. The incremental front end
   assigns node identities that survive edits and exposes them through the language server;
   comments attach to nodes; no layout or identity is written into `.sisu` files (Enso's metadata
   block), and per-user diagram layout lives in the editor's cache. Cost: low, mostly decided
   (D-027, D-030, the error-tolerant parser in 10-tooling). Conflicts: none. Confidence: high.

2. **Structural editing commands in the language server.** Expand and shrink selection by node (LSP
   `selectionRange`, since 3.15 [33]), move a statement or block past its siblings with correct
   re-indentation, wrap a selection in `if`, `for`, `with` or `task.scope do s:`, unwrap, turn an
   `if`/`elif` chain into `match`, and extract a function that takes the capability parameters the
   selection uses. In an indentation language "move this block into the loop" is the paredit
   operation, and doing it on the tree removes indentation mistakes from editing. Evidence:
   paredit's 81% among experienced Clojure users, Deuce, Stride. Cost: low to medium (code actions
   over the existing tree). Conflicts: none. Confidence: medium-high.

3. **A palette that knows types and capabilities.** A language-server request returns the
   vocabulary callable at the cursor, grouped by module and trait (MakeCode's namespace rule), each
   entry a snippet with typed holes, shown as a persistent panel. Because authority is a parameter,
   the palette can also show what is *not* available and why:

   ```sisu
   def save(fs: Fs, doc: Doc) -> ! IoError:
       # palette:  Files    fs.write_text(path, text)   fs.read_only()   fs.restrict_to(dir) ...
       #           Network  (needs a `Net`; save() was not given one)
   ```

   This is recognition over recall for the standard library and for attenuations, the benefit
   students demonstrably went to the blocks for (65.4% of switches) [13]. Cost: medium (an editor
   extension; the data is in the checker). Conflicts: none. Confidence: medium; the evidence is
   from novices, and professionals already have completion.

4. **Holes as legal edit states.** The error-tolerant parser turns each gap in unfinished text (an
   empty block after `:`, a missing operand) into an anonymous hole, and the checker gives every
   hole its expected type, so a half-written function still type-checks, completion and the palette
   can use the expected type, and nothing downstream of the gap is reported (the existing rule in
   10-tooling). Under the debugger, reaching a hole pauses the task instead of unwinding it, so
   after the hole is filled, hot replacement (D-053) restarts the frame: Hazel's fill-and-resume
   without evaluating past the hole, which code with effects must not do. The hole syntax (`?name`),
   its semantics and its reports are designed in `holes-synthesis-and-ai-era.md`. Cost: medium
   (parser recovery and checker). Conflicts: none beyond that file's. Confidence: medium.

5. **A frame mode for the teaching profile.** Sisu's syntax is frame-shaped already: a logical line
   is a frame and a line ending in `:` opens a container, so a Stride-style editor (typing `for`
   creates a frame with slots for target and iterable; expressions stay text) can be generated from
   the grammar and save canonical text. It pairs with the planned teaching profile (Q-41), whose
   subset (no generics, `ref`, `unsafe`, `comptime` or concurrency) is small enough for every
   statement form to get a frame. Evidence: the Stride study [15] and Weintrop's dual-modality
   recommendation [12]; a frame editor for Python exists (Strype, King's College London)
   [unverified]. Cost: medium-high (a web editor that must follow the grammar). Conflicts: none.
   Confidence: medium-low.

6. **Block views for teaching libraries, generated from attributes.** Attributes are compile-time
   data readable through `reflect.attributes_of` (02-syntax, 08), so MakeCode's design needs no
   compiler change:

   ```sisu
   @block("move $steps steps", category="Turtle")
   pub def forward(mut t: Turtle, steps: int): ...
   ```

   A Blockly front end shows annotated functions as blocks, decompiles only canonical formatter
   output, and shows anything else as text blocks. Cost: high (a maintained web tool; call-site
   conventions such as `mut t` need block decorations). Conflicts: ownership annotations, generics
   and closures fit blocks poorly, which is why it belongs to the teaching profile only.
   Confidence: low-medium; worth doing only with a classroom partner, since MakeCode's designers
   call the benefit unmeasured [6].

7. **Read-only diagrams generated from code.** `sisu doc` can draw what text hides and wires show:
   a module's `task.scope` tree and channel wiring, the capability flow from `main` (which
   functions received `Net`, extending `sisu audit`), and the transition graph of an `enum`-driven
   state machine. Generated and read-only, they keep the advantage Green and Petre grant dataflow
   (visible dependencies) without its viscosity, layout commitment or merge problem. Cost: low to
   medium. Conflicts: none. Confidence: low (no evidence of use either way).

## Dead ends

- **A source format other than plain formatted text**: MPS models, LabVIEW's VIs, Lamdu's store,
  Enso's metadata block, Unison's database. Each forfeits diff, merge, grep, review and blame, and
  Sisu's plan for AI assistance depends on a text corpus (10-tooling).
- **Visual dataflow as a general-purpose notation.** The 8:1 viscosity measurement, the Deutsch
  limit, Puckette's verdict on control flow and Godot's 0.5% point the same way; the node tools
  that survive are coordination layers over text.
- **Editors that forbid invalid intermediate text.** The 1980s syntax-directed editors and the
  mbeddr lessons show programmers reject them; Hazel itself moved to gradual structure editing.
- **Layout that carries meaning beyond indentation.** Max's fan-out ordering by screen position is
  the cautionary case. Sisu's indentation is decided in the lexer and never changed by the
  formatter (D-027); no other layout should be semantic.
- **Output-directed editing of general programs.** Sketch-n-Sketch needs a literal-heavy domain.
- **Blocks as the default first experience for Sisu's audience.** The gains fade within a term and
  text students judge text more authentic [10][11]; blocks are a teaching tool, not a mode of the
  language.

## Sources

1. M. Resnick et al., "Scratch: Programming for All", CACM 52(11):60-67, 2009. https://web.media.mit.edu/~mres/papers/Scratch-CACM-final.pdf
2. Scratch statistics data (last generated 14 Jul 2024, retrieved Oct 2026), https://scratch.mit.edu/statistics/data/daily/ ; release dates from Wikipedia, "Scratch (programming language)". https://en.wikipedia.org/wiki/Scratch_(programming_language)
3. Snap! "About", https://snap.berkeley.edu/about ; releases (v12.2.3, Oct 2026), https://github.com/jmoenig/Snap/releases ; Wikipedia, "Snap! (programming language)".
4. R. Fenichel, "Building the future with Blockly at Raspberry Pi Foundation", Google Open Source Blog, 28 Oct 2025, https://opensource.googleblog.com/2025/10/blockly-graduates-from-google.html ; Blockly, "Code generation overview", https://docs.blockly.com/guides/create-custom-blocks/code-generation/overview ; Wikipedia, "Blockly".
5. MIT App Inventor, "About us" (2026), https://appinventor.mit.edu/about-us ; Wikipedia, "App Inventor for Android".
6. T. Ball, A. Chatra, P. de Halleux, S. Hodges, M. Moskal, J. Russell, "Microsoft MakeCode: Embedded Programming for Education, in Blocks and TypeScript", SPLASH-E 2019. https://www.microsoft.com/en-us/research/uploads/prod/2019/10/splashws19splashemain-p8-p-41a6cf2-42682-final.pdf ; M. Moskal, T. Ball, P. de Halleux, "Static TypeScript", MPLR 2019.
7. MakeCode documentation: "JavaScript blocks", https://makecode.microbit.org/blocks/javascript-blocks ; "Defining blocks", https://makecode.com/defining-blocks ; "MakeCode Languages", https://makecode.com/language ; PXT source tree, https://github.com/microsoft/pxt (retrieved Oct 2026).
8. D. Bau, J. Gray, C. Kelleher, J. Sheldon, F. Turbak, "Learnable Programming: Blocks and Beyond", CACM 60(6):72-80, 2017. arXiv:1705.09413
9. D. Weintrop, U. Wilensky, "Using Commutative Assessments to Compare Conceptual Understanding in Blocks-based and Text-based Programs", ICER 2015. https://terpconnect.umd.edu/~weintrop/papers/Weintrop_Wilensky_ICER_2015.pdf
10. D. Weintrop, U. Wilensky, "Comparing Block-Based and Text-Based Programming in High School Computer Science Classrooms", ACM TOCE 18(1), 2017. https://ccl.northwestern.edu/2017/a3_weintrop_wilensky.pdf
11. D. Weintrop, U. Wilensky, "Transitioning from introductory block-based and text-based environments to professional programming languages in high school computer science classrooms", Computers & Education 142, 2019. https://www.terpconnect.umd.edu/~weintrop/papers/Weintrop&Wilensky_2019_C&E.pdf
12. D. Weintrop, "The Role of Block-Based Programming in Computer Science Education", Raspberry Pi Foundation seminar proceedings, 2021, https://terpconnect.umd.edu/~weintrop/papers/Weintrop_RaspPi_2021.pdf ; seminar summary, 11 Dec 2020, https://www.raspberrypi.org/blog/block-based-programming-does-it-help-students-learn-research-seminar/
13. N. Holbert, D. Weintrop, "Exploring why novice programmers switch between text and blocks in a dual-modality coding environment", 2017 (venue not verified), https://ccl.northwestern.edu/2017/HolbertWeintrop.pdf ; companion paper: D. Weintrop, N. Holbert, "From Blocks to Text and Back", SIGCSE 2017.
14. T. W. Price, T. Barnes, "Comparing Textual and Block Interfaces in a Novice Programming Environment", ICER 2015 (abstract level). doi:10.1145/2787622.2787712
15. T. W. Price, N. C. C. Brown, D. Lipovac, T. Barnes, M. Kölling, "Evaluation of a Frame-based Programming Editor", ICER 2016 (abstract level), https://isnap.csc.ncsu.edu/home/public/publication/price-2016-icer ; Stride, https://stride-lang.net/
16. A. F. Blackwell, T. R. G. Green, "Notational Systems: the Cognitive Dimensions of Notations framework", in J. M. Carroll (ed.), *HCI Models, Theories, and Frameworks*, 2003 (reports Green and Petre, JVLC 7(2), 1996, and Whitley and Blackwell 2001). http://www.cl.cam.ac.uk/~afb21/publications/BlackwellGreen-CDsChapter.pdf
17. O. Sojoodi, S. Breyer, "Our Commitment to LabVIEW as we Expand our Software Portfolio", NI forums, 23 Nov 2020, https://forums.ni.com/t5/LabVIEW/Our-Commitment-to-LabVIEW-as-we-Expand-our-Software-Portfolio/td-p/4101878 ; Wikipedia, "LabVIEW" and "National Instruments" (retrieved Oct 2026).
18. M. Puckette, "Max at Seventeen", Computer Music Journal 26(4), 2002. https://msp.ucsd.edu/Publications/dartmouth-reprint.pdf
19. Cycling '74, Max product page (2026), https://cycling74.com/products/max ; Pd software page (0.57-0), https://msp.ucsd.edu/software.html
20. Node-RED, "About", https://nodered.org/about/ ; "Writing Functions", https://nodered.org/docs/user-guide/writing-functions
21. Wikipedia, "Simulink", "Grasshopper 3D", "TouchDesigner", "Vvvv", "Prograph" (retrieved Oct 2026); vvvv, https://vvvv.org/ ; McNeel, Rhino and Grasshopper scripting guides, https://developer.rhino3d.com/guides/scripting/
22. J. Linietsky, "Godot 4 will discontinue VisualScript", 23 Aug 2022, https://godotengine.org/article/godot-4-will-discontinue-visual-scripting/ ; "Godot 3.0 released", 29 Jan 2018, https://godotengine.org/article/godot-3-0-released/ ; "Godot 4.0 sets sail", 1 Mar 2023.
23. T. Berger, M. Völter, H. P. Jensen, T. Dangprasert, J. Siegmund, "Efficiency of Projectional Editing: A Controlled Experiment", FSE 2016. http://voelter.de/data/pub/fse2016-projEditing.pdf
24. M. Voelter, B. Kolb, T. Szabó, D. Ratiu, A. van Deursen, "Lessons Learned from Developing mbeddr: A Case Study in Language Engineering with MPS", SoSyM 2019 (preprint 2017), http://voelter.de/data/pub/voelterEtAl2017-buildingMbeddr.pdf ; mbeddr.core README, https://github.com/mbeddr/mbeddr.core ; JetBrains, "What's new in MPS" (2026.1), https://www.jetbrains.com/mps/whatsnew/
25. Enso repository and language-server protocol document (the `#### METADATA ####` section), https://github.com/enso-org/enso ; https://raw.githubusercontent.com/enso-org/enso/develop/docs/language-server/protocol-language-server.md ; Enso Analytics, https://www.ensoanalytics.com/ (retrieved Oct 2026).
26. Lamdu, https://www.lamdu.org/ ; https://github.com/lamdu/lamdu
27. C. Omar, I. Voysey, M. Hilton, J. Aldrich, M. A. Hammer, "Hazelnut: A Bidirectionally Typed Structure Editor Calculus", POPL 2017. arXiv:1607.04180
28. C. Omar, I. Voysey, R. Chugh, M. A. Hammer, "Live Functional Programming with Typed Holes", POPL 2019, arXiv:1805.00155 ; Hazel project and publication list (tylr, POPL 2024, Grove, OOPSLA 2024/2025), https://hazel.org/
29. R. Chugh, B. Hempel, M. Spradlin, J. Albers, "Programmatic and Direct Manipulation, Together at Last", PLDI 2016, arXiv:1507.02988 ; Sketch-n-Sketch project page (Deuce, ICSE 2018; UIST 2019), https://ravichugh.github.io/sketch-n-sketch/
30. P. Biggar, "What is Dark?", 28 Feb 2019, https://blog.darklang.com/what-is-dark/ ; https://darklang.com/ (2025 snapshot).
31. Wikipedia, "Deutsch limit". https://en.wikipedia.org/wiki/Deutsch_limit
32. Isomorf, https://isomorf.io/ (title page only, Oct 2026).
33. Language Server Protocol 3.17 specification (Selection Range, since 3.15). https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/
34. A. Deiner, G. Fraser, "NuzzleBug: Debugging Block-Based Programs in Scratch", ICSE 2024. arXiv:2309.14465
35. In this repository: [`clojure.md`](../languages/clojure.md) (2025 State of Clojure survey on structural editing), [`learnability.md`](../topics/learnability.md), [`unison.md`](../languages/unison.md).
