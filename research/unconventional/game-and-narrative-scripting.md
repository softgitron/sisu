# Game-engine scripting and narrative languages

## Why this family matters for Sisu

Games are where visual scripting, Python-like scripting, gradual typing,
event-driven rules and hot reload were tried on the largest
non-professional populations: Roblox creators writing their first
program in Luau, students in GDevelop and Godot, designers in Blueprints,
writers in Ink. The record says which models people kept, which vendors
removed and why, and what Sisu's teaching profile (Q-41), `--watch` hot
replacement (D-053) and LSP should copy.

## Systems covered

| System | Years | Status (Oct 2026) | Central idea |
|---|---|---|---|
| Unreal Blueprints | 2014 (UE4; Kismet before) | Active in UE5; Epic reportedly plans Verse to replace it | Node graph: execution wires order effects, data wires carry values |
| UnrealScript, Verse | 1998-2014, 2023- | UnrealScript removed in UE4; Verse active (UEFN) | Engine-specific OO script; Epic's text language for creators |
| Unity Visual Scripting (Bolt), UnityScript, Boo | built in 2021; deprecated 2017; removed in Unity 5 | Visual Scripting active; the others gone | Graphs over the C# API; the scripting languages Unity dropped |
| GDScript, Godot VisualScript | 2014-; typed since 3.1 (2019) | GDScript active (4.6, 2026); VisualScript removed in 4.0 | Python-like, gradually typed, the engine's own object model; graphs over the same API |
| Roblox Luau | 2019-; open source Nov 2021 | Active | Gradual types for millions of novices; "no false positives" non-strict mode |
| Lua in games | 1993- | Active | A small embeddable VM ([lua.md](../languages/lua.md)) |
| Papyrus | 2011- | Active (Starfield) | Event-driven scripts whose state lives in save files |
| GameMaker GML, GML Visual | 1999- | Active | Dynamic C-like script; a visual mode that is a view of code |
| Construct, GDevelop, Kodu | 2007, 2008, 2009 | Active; Kodu dormant [unverified] | Event sheets and WHEN/DO tiles: conditions pick instances, actions apply |
| Zachtronics games; Human Resource Machine, 7 Billion Humans | 2011-2022; 2015, 2018 | Shipped; Zachtronics closed 2022 | Puzzles scored by histograms; assembly as office work; one program, many workers |
| Baba Is You | 2017 jam, 2019 | Shipped | The rules are objects in the level |
| Factorio circuits, Minecraft redstone | 2015-, 2010- [unverified] | Active | Signals on wires, tick-synchronous, programmed by players |
| Ink | 2014-; 1.0 in 2021 | Active, MIT | Text-first narrative markup: knots, diverts, weave |
| Yarn Spinner, Twine, ChoiceScript | c. 2015 [unverified], 2009, 2009 | Active | Dialogue nodes; hypertext with a map; indentation-structured choices |

## The ideas

### Node graphs at scale, and why Godot removed its own

Blueprints shipped with Unreal Engine 4 (19 March 2014), replacing
Kismet [8]. White *execution* wires order the nodes that have effects,
coloured *data* wires carry values, and "pure" nodes without execution
pins are re-evaluated on every pull [11, unverified]:

```
[Event BeginPlay] =exec=> [Set Health] =exec=> [Print String]
                               ^ data                ^ data
                     (Get Max Health)       (Health to Text)      <- pure nodes
```

Epic's guidance (its documentation blocked every fetch from here) keeps
core systems, heavy loops and anything that must be diffed or merged in
C++, and Blueprints for designer-facing, level and prototype logic on
components C++ exposes; nativization to C++ at cook time, the
performance fix, did not survive into UE5 [11, unverified]. The failure
mode has a gallery, "UE4 Blueprints From Hell" (2014-2019) [10]. Epic is
moving off graphs: UEFN has used the text language Verse since 23 March
2023, and at State of Unreal 2026 Epic reportedly said Blueprints and
Actors "would eventually be replaced by Verse and Scene Graph" [9]
(Wikipedia citing the keynote; not verified against it).

Godot's post-mortem is the clearest designer retrospective here.
VisualScript left 4.0 because "it never gained traction and the path to
improve it was never clear"; "only 0.5% of the user base has used
VisualScript as their main engine language" (a poll of over 5,000); its
requesters were mostly not Godot users, and for many "Godot ended up
being a tool to learn programming instead" [1]. The diagnosis transfers:
"Godot lacked high level components to make use of it. Engines like
Unreal, Game Maker or Construct offer high level game features packaged
together with the visual scripting solution. This is what makes it
useful" [1]. GameMaker's GML Visual, whose blocks "translate directly to
code" [19], is a view of the text language; Unity made Bolt its built-in
Visual Scripting in 2021 [7] and has published no usage figures.

### Event sheets: rules that run every tick over picked instances

Construct (Scirra: 2007, 2011, Construct 3 in December 2017) is "aimed
primarily at non-programmers" [20]; GDevelop (Florian Rival, 2008; MIT)
reports "over 10,000 students using it in schools and universities
worldwide by 2025" [21]. Events "run in the order they are listed from
top to bottom" every frame; "by default, actions and conditions will
apply to all instances of the object", and conditions "filter (or
'pick')" the instances the actions then touch; sub-events refine the
parent's pick [21]:

```
Bullet is in collision with Enemy      ->  Enemy: subtract 1 from Health
                                           Bullet: delete
    sub-event: Enemy.Health <= 0       ->  Enemy: delete;  Score: add 10
```

Each event is an implicit `for` over the instances its conditions
select, inside an implicit frame loop, with no variable naming the
selection: a relational query, a per-row effect and a synchronous tick.
Kodu (Microsoft, 2009) gave children the same model as tiles, "WHEN see
apple DO move toward" [22] (Scratch is in
[visual-and-structure-editing.md](visual-and-structure-editing.md)). The
ceiling shows: Construct 3 added JavaScript in July 2019 [20]. No textual
language took the model as syntax; its relatives are entity-component
queries and production rules [judgement].

### GDScript: a Python-like language built for one engine

Godot began with Lua and tried Python, then wrote its own language over
"poor threading support in most script VMs", "poor class-extending
support", "horrible interfaces for binding to C++", "no native vector
types", garbage-collector "stalls", and "difficulty integrating with the
code editor for providing code completion, live editing, etc." [2]. It
reads as Python, but its object model is the engine's: a script extends
a node class, exported variables appear in the inspector, and callbacks
are engine hooks:

```gdscript
extends CharacterBody2D
@export var speed: float = 200.0

func _physics_process(delta: float) -> void:
    velocity = Input.get_vector("ui_left", "ui_right", "ui_up", "ui_down") * speed
    move_and_slide()
```

Types arrived additively: 3.1 (March 2019) [6]; a rewritten GDScript with
typed arrays in 4.0 (1 March 2023); typed dictionaries in 4.4 (2025);
abstract classes, variadic functions and release-build backtraces in 4.5
(2025); a Step Out button and better LSP docstrings in 4.6 (2026) [3][4].
The documentation sells types by their payoff: errors "without even
running the code", better completion, and "optimized opcodes when
operand/argument types are known at compile time"; the typing warnings
are "disabled by default", and a type-checked line shows its number in
green [3]. Hot reload is a menu option, "Synchronize Script Changes" [5],
and 4.4 embeds the running game in the editor [4]. After Unity's
September 2023 runtime-fee announcement, Unity's Global Game Jam share
fell from 61% (2023) to 36% (2024), and Godot overtook Unity at the GMTK
Game Jam in 2026, "ending nine years of Unity dominance" [7]: licensing
was the cause, but the language did not hold the newcomers back.

### Luau: gradual typing for the largest novice population

Roblox's figures: in July 2021 "approximately 20 million experiences ...
created by 8 million developers", with "over 200 Roblox kids' coding
camps in 65 countries" [12]; in 2022 "hundreds of thousands" wrote Luau
monthly, for many "the first programming language they learn" [13];
85.3 million daily users were reported in February 2025 [17].
Luau, derived from Lua 5.1 in 2019, shipped type checking in November
2020 and was open-sourced in November 2021 [13][16]; each file picks
`--!nocheck`, `--!nonstrict` (the default) or `--!strict` [14].

The design starts from the population: learners arrive with a concrete
goal, "a common workflow for getting started is to Google the task, then
cut-and-paste the resulting code", and "for most goal-driven developers,
the type system should help or get out of the way" [12]. Hence three
goals [12]. *Infallible types*: every program, even an ill-typed or
unparsable one, gets types, with bad subterms flagged, because
completion and documentation are type-driven and code is usually broken
while it is written. *Strict mode*: "no false negatives". *Non-strict
mode*: "no false positives", so "any flagged code is guaranteed to
produce a runtime error when executed". The paper's examples:

```lua
--!strict
local i, j = string.find(x, y)
if i then print(j - i) end      -- flagged: j is number? even when i is set
--!nonstrict
print(math.abs("hi"))           -- flagged: certain to fail at run time
```

The first non-strict mode inferred `any` for locals; it was replaced
because "having two different type inference engines is unnecessarily
confusing", and now "the only difference between the modes will be in
which errors are reported" [15]. The engine was rewritten (the "new type
solver", "nearing completion" in July 2024, still being fixed in 2026)
[16]; it computes types with type functions users write "in ordinary
Luau code" [15], a type-level `comptime` (compare
[typescript.md](../languages/typescript.md)).

### What killed engine languages, and why Lua survived

Lua won game scripting as a small VM with a stack-based C API an engine
can embed anywhere; gamedev.net polls put it first by 2003
([lua.md](../languages/lua.md)). Engine-specific languages died of the
two-language tax. Epic announced at GDC 2012 that UnrealScript would
leave UE4 "in favor of C++", and Sweeney later recalled asking "why
don't we just kill UnrealScript and move to pure C++?" for "maximum
performance and maximum debuggability" [8][18]; Unity removed Boo with
Unity 5 and deprecated UnityScript in August 2017 [7]. Bethesda's
Papyrus (Skyrim, 2011, to Starfield) stores script instances, variables
and suspended calls in save files, so an updated or removed mod meets its
old state in old saves, and save-cleaning tools exist for that reason
[35, unverified]. The survivors are a tiny embeddable VM, the engine's
own object model or a mainstream language. Sisu's bet is the opposite of
a split: one language from interpreter to native code.

### Hot reload in engines keeps state and swaps code

Godot reloads saved scripts into the running game [5]; Unreal's C++ live
coding uses Live++, which "directly patches the machine code of the
running executable" [34]; Flutter treats "global variables and static
fields ... as state", so their initialisers do not re-run, while
"changes to `const` field values are always hot reloaded" [33]. Trouble
comes where state's shape depends on code (Papyrus is the extreme [35]),
and the frame boundary is the natural swap point.

### Programming as play

Zachtronics (Zach Barth) made programming the game, from SpaceChem
(2011) through TIS-100 (2015), Shenzhen I/O (2016), Opus Magnum (2017)
and Exapunks (2018) [23]. TIS-100 is twelve nodes in a grid, each with an
`ACC` and a backup register and its own assembly program; Opus Magnum
scores "speed in cycles, cost in G, and area in tiles" and exports
solutions as GIFs; and solved puzzles show statistics "with histograms
from other players" [23], a distribution rather than a rank, which keeps
optimising open-ended (the games were free to schools from 2019; the
studio closed in 2022) [23]. Human Resource Machine (Tomorrow
Corporation, 2015) maps assembly onto an office (the held box is the
register, floor tiles are memory) and grades instruction count and
average steps; 7 Billion Humans (2018) runs "the same program ... to
control all humans simultaneously", SPMD as a puzzle [24]. Baba Is You
(Arvi Teikari, 2019; IGF 2018 Best Student Game) puts the rules in the
level as word tiles, so pushing `WALL IS STOP` apart removes a law [25].
Players with no programming background build machines too: Factorio (3.5
million copies by December 2022) carries typed signals with 32-bit counts
on red and green wires, same signals "are additive", and combinators
compute on them [26]; Minecraft players build "calculators, digital
clocks, and even basic computers" from redstone [27]. Common to all: a
small fixed machine, visible state, a specification given as tests,
several metrics, and a tick.

### Narrative languages: text first, code inserted

Ink (inkle; every inkle game since *80 Days*, 2014; 1.0 in 2021; MIT) is
"markup, not programming: text comes first, code and logic are inserted
within", "conceived as middleware" for game engines [28][29]. Knots
(`=== paris ===`) and stitches are its units, `->` diverts, `*` choices
are used once and `+` choices stay; the *weave* branches without naming
every branch, since "options separate the paths of the story, and gathers
bring them back together"; and each knot is "an integer variable"
counting visits, so the commonest state needs no declaration [28]:

```ink
=== paris ===
*   "Nothing, Monsieur!"[] I replied.
    "Very good, then."
*   "The Louvre, please."
-   With that Monsieur Fogg left the room.
*   {not london} [Take the train.] -> london
```

Yarn Spinner (Night in the Woods, A Short Hike, DREDGE) writes dialogue as
nodes of lines, `->` options and `<<jump>>` commands [30]. Twine (Chris
Klimas, 2009) "emphasizes the visual structure of hypertext", a map of
passages with swappable story formats; *Howling Dogs* (2012) and
*Depression Quest* (2013) were made in it [31]. ChoiceScript (Choice of
Games, 2009; over a hundred own titles and about ninety hosted) is for
"authors with no programming experience": `*choice` opens a block of
`#option` lines, each with an indented body of `*set`, `*goto` or
`*finish`, and "indentation in ChoiceScript is mandatory" [32]. Writers
adopt languages where prose is the default and code the escape, the
inverse of a string literal; automatic state removes the commonest
declarations; an indentation-structured choice tree works for
non-programmers. An Ink-like parser would also make a good test of Sisu's
comptime string DSLs and `embed`. Inform 7 is in
[natural-language-and-literate.md](natural-language-and-literate.md).

## Transferable ideas for Sisu

The answer to the central question: text over graphs, types that pay off
in tooling before errors, packaged domain components over notation, and
swaps at tick boundaries. Worth taking: Luau's infallible typing (as
checker architecture, not unsoundness) and tick-boundary reload rules;
not worth taking: event sheets and exec pins.

**1. Infallible type checking, and running past errors in the teaching
profile.** The checker returns a typed tree for every input, flagging
ill-typed subterms instead of failing, so hover, completion and
go-to-definition work in broken code (Luau's first goal). Under `profile
= "teaching"` and `sisu run --watch` the interpreter may run a module
with type errors: a flagged expression becomes a panic carrying its
diagnostic, raised only if reached; `sisu check`, `test` and `build`
still reject the module.

```sisu
def area(r: f64) -> f64:
    return PI * r * "2"       # flagged; the file stays typed; a teaching run panics here
```

*Cost:* medium; it must be designed into the Phase 1 checker, as error
recovery is hard to retrofit [judgement]. *Conflicts:* soundness holds
because a flagged term never runs as typed code, but "a program with
errors runs" is a new mode, limited to teaching and watch
([holes-synthesis-and-ai-era.md](holes-synthesis-and-ai-era.md)).
*Confidence:* high for the tolerant checker (the LSP shares `sisu
check`'s queries), medium for running past errors.

**2. Hot replacement rules from engines**, on top of the boundary-aligned
swap of [esoteric-and-live-coding.md](esoteric-and-live-coding.md) (idea
1): a changed `const` or comptime result re-swaps every body that uses
it, through the query graph (Flutter's `const` rule); a frame loop's
iteration is the default boundary for the libraries below; and nothing
the runtime persists for the user may contain frames, code addresses or
epochs, only values with type versions (Papyrus). *Cost:* low; the first
rule falls out of the incremental queries. *Conflicts:* none with D-053.
*Confidence:* high.

**3. A packaged first-program library for the teaching profile.**
Godot's diagnosis ("high level components"), Construct's behaviours,
PICO-8's console, Processing's canvas and Roblox Studio's physics agree:
beginners need the domain packaged, whatever the notation.

```sisu
from sketch import Canvas, Input, Key, run     # first-party package

def update(mut x: int, input: Input):
    if input.held(Key.Left):
        x -= 2

def draw(x: int, mut c: Canvas):
    c.sprite("ship", x=x, y=110)

def main(sys: System):
    run(sys, start=64, update=update, draw=draw)   # 60 fps; both hot-swap on save
```

*Cost:* high (rendering, input, sound, assets). *Conflicts:* graphics is
a library under 14-non-goals, so this is a first-party package outside
std; the profile disables generics, so the API is monomorphic;
WebAssembly is deferred in [18-next-steps.md](../../proposal/18-next-steps.md).
*Confidence:* medium.

**4. A lockstep tick model as the teaching profile's concurrency.** 7
Billion Humans, event sheets, Factorio and Orca teach "many things at
once" without races: every agent reads the last tick and writes the next.

```sisu
def step(me: Ant, world: World) -> Ant:      # reads last tick, returns this tick
    let food = world.nearest_food(me.pos) else:
        return me.wander()
    return me.toward(food)
```

`tick.run(sys, World.random(ants=200), step)` steps every ant once per
tick, deterministically, and the same `step` later runs under
`task.scope`. *Cost:* a library. *Conflicts:* none; the profile disables
`scope` and `spawn`. *Confidence:* low-medium.

**5. Deterministic cost metrics and class histograms.** `sisu test
--cost` reports interpreter steps (the comptime fuel counter, so it is
reproducible), peak allocation and a PICO-8-style token count per test,
as JSON a course server can turn into Zachtronics-style histograms.
*Cost:* low. *Conflicts:* none, if documented as a teaching measure.
*Confidence:* medium-low.

## Dead ends

- **Node-graph visual scripting**: 0.5% use and removal in Godot, Epic
  moving to Verse, graphs that do not merge, pure nodes that re-run on
  every pull (Sisu's `let` keeps evaluation count visible).
- **Event sheets or WHEN/DO rules as syntax**: an implicit loop over a
  picked set inside an implicit tick; `for` and a tick library express
  it, and the value was in the packaged behaviours.
- **An optional or unsound typing mode**: Luau's non-strict mode serves
  millions of lines of untyped Lua; Sisu has no untyped legacy.
- **A scripting dialect of Sisu**: UnrealScript, UnityScript and Boo died
  of the two-language tax; embed the interpreter for the same language.
- **Rules as manipulable objects** (Baba Is You): as semantics, action at
  a distance, the `COME FROM` verdict again.

## Sources

1. J. Linietsky, "Godot 4.0 will discontinue visual scripting", 23 Aug 2022. https://godotengine.org/article/godot-4-will-discontinue-visual-scripting/
2. Godot documentation, FAQ ("What were the motivations behind creating GDScript?"). https://docs.godotengine.org/en/stable/about/faq.html (Oct 2026)
3. Godot documentation, "Static typing in GDScript". https://docs.godotengine.org/en/stable/tutorials/scripting/gdscript/static_typing.html (Oct 2026)
4. Godot release notes 4.4, 4.5, 4.6. https://godotengine.org/releases/4.4/ ; https://godotengine.org/releases/4.5/ ; https://godotengine.org/releases/4.6/ (2025-2026)
5. Godot documentation, "Overview of debugging tools". https://docs.godotengine.org/en/stable/tutorials/scripting/debug/overview_of_debugging_tools.html (Oct 2026)
6. "Godot (game engine)", Wikipedia. https://en.wikipedia.org/wiki/Godot_(game_engine) (Oct 2026)
7. "Unity (game engine)", Wikipedia. https://en.wikipedia.org/wiki/Unity_(game_engine) (Oct 2026)
8. "Unreal Engine" and "Unreal Engine 4", Wikipedia. https://en.wikipedia.org/wiki/Unreal_Engine ; https://en.wikipedia.org/wiki/Unreal_Engine_4 (Oct 2026)
9. "Verse (programming language)", Wikipedia, citing Epic Games, "State of Unreal 2026", 17 Jun 2026. https://en.wikipedia.org/wiki/Verse_(programming_language) (Oct 2026)
10. "UE4 Blueprints From Hell". https://blueprintsfromhell.tumblr.com/ (2014-2019)
11. Epic Games, "Balancing Blueprint and C++" and "Nativizing Blueprints", Unreal Engine documentation (not retrievable from this environment; claims marked unverified). https://dev.epicgames.com/documentation/en-us/unreal-engine/balancing-blueprint-and-cplusplus-in-unreal-engine
12. L. Brown, A. Friesen, A. Jeffrey, "Position Paper: Goals of the Luau Type System", HATRA 2021. https://arxiv.org/abs/2109.11397 (2021)
13. Luau, "Luau origins and evolution", 4 Nov 2022. https://luau.org/news/2022-11-04-luau-origins-and-evolution
14. Luau, "Type checking"; Roblox Creator documentation, "Type checking". https://luau.org/types ; https://create.roblox.com/docs/luau/type-checking (Oct 2026)
15. Luau RFCs, "New non-strict mode" and "User-defined type functions". https://rfcs.luau.org/new-nonstrict.html ; https://rfcs.luau.org/user-defined-type-functions.html (Oct 2026)
16. Luau news index, "Recap: July 2024", and release notes. https://luau.org/news ; https://luau.org/news/2024-07-23-luau-recap-july-2024 ; https://github.com/luau-lang/luau/releases (2020-2026)
17. "Roblox", Wikipedia. https://en.wikipedia.org/wiki/Roblox (Oct 2026)
18. "UnrealScript", Wikipedia. https://en.wikipedia.org/wiki/UnrealScript (Oct 2026)
19. "GameMaker", Wikipedia; YoYo Games, "GameMaker Studio 2.3: New GML Features", 18 Aug 2020. https://en.wikipedia.org/wiki/GameMaker ; https://gamemaker.io/en/blog/gamemaker-studio-2-dot-3-new-gml-features
20. "Construct (game engine)", Wikipedia. https://en.wikipedia.org/wiki/Construct_(game_engine) (Oct 2026)
21. GDevelop wiki, "Events"; "GDevelop", Wikipedia. https://wiki.gdevelop.io/gdevelop5/events/ ; https://en.wikipedia.org/wiki/GDevelop (Oct 2026)
22. "Kodu Game Lab", Wikipedia. https://en.wikipedia.org/wiki/Kodu_Game_Lab (Oct 2026)
23. "Zachtronics", "TIS-100", "Opus Magnum (video game)", Wikipedia. https://en.wikipedia.org/wiki/Zachtronics ; https://en.wikipedia.org/wiki/TIS-100 ; https://en.wikipedia.org/wiki/Opus_Magnum_(video_game) (Oct 2026)
24. "Human Resource Machine", "7 Billion Humans", Wikipedia. https://en.wikipedia.org/wiki/Human_Resource_Machine ; https://en.wikipedia.org/wiki/7_Billion_Humans (Oct 2026)
25. "Baba Is You", Wikipedia. https://en.wikipedia.org/wiki/Baba_Is_You (Oct 2026)
26. Factorio wiki, "Circuit network" and "Arithmetic combinator"; "Factorio", Wikipedia. https://wiki.factorio.com/Circuit_network ; https://wiki.factorio.com/Arithmetic_combinator ; https://en.wikipedia.org/wiki/Factorio (Oct 2026)
27. Minecraft Wiki, "Redstone circuits". https://minecraft.wiki/w/Redstone_circuits (Oct 2026)
28. inkle, "ink"; ink README; "Writing with Ink". https://www.inklestudios.com/ink/ ; https://github.com/inkle/ink ; https://github.com/inkle/ink/blob/master/Documentation/WritingWithInk.md (Oct 2026)
29. "Inkle (company)", Wikipedia. https://en.wikipedia.org/wiki/Inkle_(company) (Oct 2026)
30. Yarn Spinner. https://www.yarnspinner.dev/ ; https://docs.yarnspinner.dev/ (Oct 2026)
31. "Twine (software)", Wikipedia. https://en.wikipedia.org/wiki/Twine_(software) (Oct 2026)
32. Choice of Games, "Introduction to ChoiceScript"; "Choice of Games", Wikipedia. https://www.choiceofgames.com/make-your-own-games/choicescript-intro/ ; https://en.wikipedia.org/wiki/Choice_of_Games (Oct 2026)
33. Flutter documentation, "Hot reload". https://docs.flutter.dev/tools/hot-reload (Oct 2026)
34. Live++. https://liveplusplus.tech/ (Oct 2026)
35. Creation Kit wiki, "Save File Notes (Papyrus)" (returned HTTP 403 to this environment; claims marked unverified). https://ck.uesp.net/wiki/Save_File_Notes_(Papyrus)
