# Smalltalk (and Pharo)

## Snapshot
| | |
|---|---|
| First appeared | Smalltalk-72 (Xerox PARC, 1972); Smalltalk-76; Smalltalk-80 (public release 1980-1983). Squeak 1996 (Apple); Pharo 2008 (fork of Squeak). |
| Designed by | Alan Kay, Dan Ingalls, Adele Goldberg and the Learning Research Group at Xerox PARC. Pharo: INRIA (Stéphane Ducasse, Marcus Denker, Esteban Lorenzano) and the Pharo consortium. |
| Current version (Oct 2026) | Pharo 13.1 (26 June 2025); Pharo 14 delayed (the team's 29 June 2026 note says it is "still not quite at the point where we want to release it"; no release by October 2026). Squeak 6.0 (2022). Cuis, VisualWorks (Cincom, yearly), VA Smalltalk (Instantiations), GemStone/S also active. |
| Governance | Pharo: consortium of institutions and companies plus an association; open, GitHub-based. Squeak: Squeak Oversight Board. Commercial dialects are vendor-controlled. ANSI Smalltalk standard (1998) is largely ignored. |
| Typing | Dynamic, strong; everything is an object, every operation is a message send; duck typing. Pharo has optional pragma-based type annotations for tools only. |
| Memory | Tracing GC in the VM (OpenSmalltalk/Cog VM: generational, with JIT since 2008; Pharo VM is a fork of it). |
| Compilation | Source compiled to bytecode inside the image; JIT to machine code at run time; the whole system (compiler, IDE, debugger) lives in the image. |
| Primary domains | Historically: GUIs, CASE tools, finance (JPMorgan Kapital), manufacturing. Today: research, teaching, tooling research (Moose, Glamorous Toolkit), some enterprise (GemStone, VA Smalltalk legacy), Pharo-based web (Seaside, Teapot). |
| Notable users | Historically JPMorgan (Kapital), Texas Instruments, Adventa; today GemTalk customers, Pharo consortium members (e.g. Thales), Moose/Glamorous Toolkit users. |

## Stated design goals

Kay's goal was a personal dynamic medium for children and non-experts: "if
it was a medium, it had to extend into the world of childhood" [1]. The
technical bet was that a single uniform metaphor (objects exchanging
messages, like biological cells) could replace data structures plus
procedures, and that the whole system should be understandable and
modifiable by its user. Ingalls's 1981 principles [2] are the clearest
statement: *Personal Mastery* ("If a system is to serve the creative spirit,
it must be entirely comprehensible to a single individual"); *Uniform
Metaphor* ("A language should be designed around a powerful metaphor that
can be uniformly applied in all areas"); *Reactive Principle* ("Every
component accessible to the user should be able to present itself in a
meaningful way for observation and manipulation"); and, on the operating
system: "An operating system is a collection of things that don't fit into
a language. There shouldn't be one." Kay in 1998: "The big idea is
'messaging'"; Smalltalk is "not even about classes"; he is "sorry that I long
ago coined the term 'objects'" because it leads people "to focus on the
lesser idea" [3].

## Coverage of the 15 evaluation dimensions

1. **Syntax and readability.** Unary, binary and keyword messages, blocks, cascades (`;`), six reserved words; fits on a postcard; keyword messages read as sentences; precedence (unary before binary before keyword) is the one trap.
2. **Type system.** Dynamic, strong; everything is an object; no static checking; Strongtalk proved optional types feasible but was abandoned; Pharo has pragma-based type hints for tools only.
3. **Memory management and safety.** Generational GC in the VM; memory-safe except through FFI and VM plugins; `become:` (swap object identities) is a powerful and dangerous primitive.
4. **Mutability, aliasing and value semantics.** Mutable objects with shared references by default; literals (numbers, symbols, some strings) are value-like; no immutability guarantees (Pharo added an optional read-only object bit in the VM).
5. **Error handling.** Resumable exceptions (`on:do:`, `retry`, `resume:`, `return:`, `pass`, `outer`) with the debugger as default handler; closer to CL's condition system than to Java's exceptions.
6. **Concurrency and parallelism.** Green threads (`Process`, `Semaphore`, `Mutex`) scheduled by the VM on one OS thread; no parallelism in the classic VM; no data-race prevention.
7. **Polymorphism.** Single inheritance, late-bound message sends, protocols (duck typing); traits (Pharo, Squeak) for reuse without inheritance; `doesNotUnderstand:` for proxies.
8. **Metaprogramming.** Total reflection (classes, methods, contexts are objects), `perform:`, `thisContext`, compiling source at run time, method wrappers/proxies; the compiler and tools are in the image.
9. **Modules, packages, build.** Packages/categories in the image; Monticello then Iceberg (Git with Tonel text files) in Pharo; Metacello for dependency loading; no compile step; builds are "load into a clean image" scripts.
10. **Tooling.** The original IDE: class browser, inspector, debugger with fix-and-resume, refactoring browser, SUnit, code critics, Calypso browser and Spec UI in Pharo; everything is live and scriptable.
11. **Interop and platforms.** Unified FFI (Pharo), VM plugins in Slang (a Smalltalk subset compiled to C); VM runs on Linux/macOS/Windows/ARM; deployment is image plus VM.
12. **Performance and compilation model.** Bytecode plus Cog JIT with inline caches; dynamic dispatch everywhere; roughly comparable to other dynamic JIT languages; no AOT.
13. **Standard library and ecosystem.** Large class library in the image (collections, streams, GUI, networking); ecosystem small; Seaside, Roassal (visualisation), Moose, Glamorous Toolkit are the notable projects.
14. **Learnability.** Syntax learned in an hour; the environment and the "where is my code?" image model take longer; historically the language used to teach OOP; Pharo MOOC exists.
15. **Evolution and governance.** Pharo: open consortium, yearly releases (13 in 2025; 14 delayed in 2026); Squeak: oversight board; commercial dialects vendor-driven; ANSI standard (1998) effectively dormant; dialects diverge.

## Strengths

### A uniform object/message model with a syntax that fits on a postcard
Everything is an object (integers, classes, methods, stack frames, the
compiler), and the only operations are message sends with three forms:
unary (`x size`), binary (`a + b`), keyword (`dict at: k put: v`). Control
flow is messages to booleans and blocks (`cond ifTrue: [...] ifFalse:
[...]`, `1 to: 10 do: [:i | ...]`); classes are created by sending
`subclass:` to a class. Ralph Johnson's postcard snippet exercises the
entire grammar in a dozen lines [4]. The evidence this works: Smalltalk
"influenced virtually all of the object-oriented languages that came after"
(Objective-C is Smalltalk messaging on C; Ruby's blocks and everything-is-
an-object; Python; Java's class library shape) [4]; the keyword-message
syntax survives in Objective-C and Swift's argument labels; blocks became
closures in every mainstream language. The uniformity has a design cost
visible in Kay's own retrospective (metaclasses, below).

### The image-based live environment and the fix-and-resume debugger
The image is a snapshot of all objects, including the development tools; the
developer edits a method in the browser and the change is live immediately.
When an exception reaches the debugger, the stack is *not* unwound: the
developer inspects frames, edits the failing method in place, and chooses
*Proceed* or *Restart* on a frame, continuing the computation with the fixed
code. Unimplemented methods can be created from the debugger at the point
of the failing send. This is the same capability as Lisp's restarts but
integrated with the IDE; it is what "live programming" means in the original
sense. Modern echoes: Jupyter kernels (state persists across cell edits),
Swift Playgrounds, Flutter/React hot reload, Elixir's IEx `recompile`, Edit
and Continue in Visual Studio, Lisp/SLIME. All are partial: none allows
editing a frame's method and resuming the frame, because their stacks are
native and their runtimes do not reify activation records as objects.

### The origin of the IDE, refactoring, unit testing and MVC
Smalltalk-80 shipped the class browser, inspector, workspace and debugger as
one environment, running on a bitmapped display with overlapping windows: the
IDE as a category comes from it [4]. The Refactoring Browser (Roberts,
Brant, Johnson, 1997) was the first tool to perform behaviour-preserving
rename/extract/move with semantic knowledge of the program; IntelliJ and
Eclipse refactoring descend from it, and Martin Fowler's *Refactoring*
(1999) is based on Smalltalk practice. SUnit (Kent Beck, 1989, published
1994 as "Simple Smalltalk Testing: With Patterns") became JUnit (Beck and
Gamma, 1997) and the xUnit family [5]. Model-View-Controller originated in
Smalltalk-76/80 (Reenskaug, 1979). This is the strongest "adoption and
imitation" record of any language in this group: the tools were copied even
though the language was not.

### Blocks, collections and the protocol style of API design
Closures (`[:x | x * 2]`) with non-local return, and a collection protocol
(`collect:`, `select:`, `inject:into:`, `detect:ifNone:`) that treats
iteration as a message to the collection. Ruby copied the protocol almost
verbatim; Java 8 streams, C# LINQ and Kotlin's stdlib are typed descendants.
The *protocol* idea (a set of messages an object understands, documented as
such, independent of class) is the origin of duck typing and of
Objective-C/Swift protocols and Go interfaces as structural contracts.

### Reflection and metacircularity used in practice
Classes, methods, bytecode and stack contexts are ordinary objects; the
compiler is written in Smalltalk; `thisContext` reifies the current
activation. Pharo uses this for MethodProxies (message-send instrumentation,
2026), Glamorous Toolkit's "moldable" inspectors and Moose. The cost is
unpredictable performance and no whole-program optimisation.

### The VM research lineage
Deutsch and Schiffman's 1984 JIT for Smalltalk-80 introduced dynamic
translation and inline caches; Self (Ungar, Chambers, Hölzle) added
polymorphic inline caches and adaptive optimisation; the Self/Strongtalk
team built Java HotSpot, and V8 descends from the same lineage. Generational
scavenging (Ungar, 1984) is from Smalltalk too. The techniques that make
JavaScript and Java fast were invented to make Smalltalk fast.

## Weaknesses

### Why it lost to C++ and Java
Documented factors [4, 6]: (1) *Price and licensing*: Xerox released
Smalltalk-80 v2 in 1983 "under a restrictive license"; ParcPlace's high
price limited penetration to "mid-sized and large organizations"; Java was
free. (2) *Performance and memory*: both vendors struggled with "substantial
memory needs, limited run-time performance" on 1990s hardware. (3) *Database
connectivity*: "initially poor connectivity to SQL databases". (4) *Platform
politics*: ParcPlace-Digitalk "never managed to find an effective response
to Java", and IBM, which had the largest Smalltalk business (VisualAge),
backed Java instead. (5) *The image vs files*: source control, code review,
diff-based collaboration, continuous integration and deployment all assume
text files; a Smalltalk program is an image with changes recorded in a
change log, which did not fit the version-control workflow that became
universal in the 1990s and 2000s. Pharo's answer (Iceberg, 2017, Git
integration with Tonel per-class text files) came decades late and is still a
bridge rather than native. (6) *Vendor fragmentation*: ParcPlace, Digitalk,
IBM, Dolphin, each with incompatible class libraries; the ANSI standard
(1998) arrived after the market had moved. Alan Kay's own view is that
Smalltalk-80 was never meant to be final and that "a way of doing things
finally gets canonized into an inflexible belief structure" [1].

### The uniform model's own warts, admitted by its designers
Kay quotes Peter Deutsch (1989): metaclasses "have proven confusing to many
users, and perhaps in the balance more confusing than valuable" and "this
proliferation of different kinds of instantiations and scoping was probably
a bad idea" [1]. On inheritance: "inheritance - though an incredibly
powerful technique - has turned out to be very difficult for novices", and
"we needed a better theory about inheritance entirely (and still do)" [1].
Single inheritance with an open class hierarchy means any class can be
monkey-patched from anywhere in the image (`Object>>#foo`), which Ruby copied
and then had to tame with refinements. Pharo added traits (Schärli, Ducasse,
Nierstrasz, Black, 2003) as a fix for the limits of single inheritance;
traits were adopted by Scala, PHP 5.4 and Rust (name and partial semantics).

### No static types and no modules
There are no static types, no interfaces, no module system beyond class
categories/packages, no visibility beyond "all methods public, all instance
variables private", and no compile-time check that a message is understood.
Refactoring works because the tools are excellent and the image contains
all senders, not because the compiler verifies anything. Strongtalk (ParcPlace
research 1993, then Animorphic Systems 1994-96, acquired by Sun in 1997)
added an optional static type system and showed it was feasible; it was
abandoned when Sun put the team on Java HotSpot. Pharo has no plan for static
types.

### Deployment and reproducibility
Shipping a Smalltalk application means shipping an image (tens of MB) plus a
VM; stripping an image for deployment is a specialised skill; the image can
accumulate state that is not in any source file; reproducing a build from
source alone requires discipline (Pharo's Metacello/Iceberg "load from
scratch" CI practice). This is the same weakness as Lisp images, with less
mitigation, and it is the single most cited reason enterprises avoided
Smalltalk once Java offered jar files and classpaths.

### Small, aging community and fragile funding
Pharo's main release slipped a full year in 2026 while the team rewrote the
VM (v12), moved to SDL3 and replaced the graphics stack (Bloc/Toplo/
Alexandria) [7]; this is a few-dozen-person effort carrying an entire OS-like
platform. Commercial dialects survive on legacy accounts. The ANSI standard
is ignored; code portability between dialects is poor.

## Designer retrospectives

- Kay (1993) [1]: on Smalltalk-72's syntax, "I never liked this syntax (too
  many parentheses and nestings)"; by 1976 he "wanted to stop, dynamite
  everything and start from scratch again" because "I did not see how OOP by
  itself was going to solve our end-user problems"; "Strong paradigms like
  LISP and Smalltalk are so compelling that they eat their young"; and "it
  came to me as a shock that no child had programmed in any Smalltalk since
  Smalltalk-76 made its debut." Of Smalltalk-76 he applies Hoare's quip: it
  "was a great improvement on its successors."
- Kay (1998) [3]: the essence is messaging and late binding, not classes;
  the industry's OOP (C++, Java) missed it.
- Ingalls (1981) [2]: "Natural Selection: Languages and systems that are of
  sound design will persist, to be supplanted only by better ones" - which,
  read against history, concedes that design soundness alone did not decide.
- Pharo team (June 2026) [7]: a candid note that the release was delayed by
  the scope of VM and UI rewrites, which illustrates the cost of owning the
  whole stack.

## Influence

Copied from Smalltalk: the IDE, class browser and inspector; the integrated
debugger with resume; refactoring tools; xUnit; MVC; blocks/closures with
collection protocols (Ruby, then everyone); keyword/labelled arguments
(Objective-C, Swift); everything-is-an-object (Ruby, Python's unification
in 2.2, Scala, Kotlin); duck typing and protocols (Objective-C, Swift, Go
interfaces, Python ABCs); traits (Scala, PHP, Rust); generational GC and
JIT techniques (Self, HotSpot, V8); live programming (Jupyter, Playgrounds,
hot reload, Lively Kernel, Glamorous Toolkit); Morphic direct-manipulation UI
(Squeak, Etoys, Scratch - Scratch 1.x was written in Squeak). Smalltalk took
from: Simula (classes, objects), Lisp (dynamic typing, GC, interactive
development, the metacircular compiler), Sketchpad and Logo (direct
manipulation, children as users), Flex and the Dynabook vision.

## Lessons for Sisu

- **Adopt**: *the debugger as a first-class language concern*: reified
  stack frames in debug builds, the ability to replace a function and resume
  from a chosen frame, and inspectors that any value can customise
  (Ingalls's Reactive Principle). This is the part of "live programming"
  with no modern equal, and it is a runtime/compiler design decision that
  cannot be retrofitted.
- **Adopt**: *keyword/labelled message syntax for multi-argument calls*
  (`at:put:`), which survived into Swift and Objective-C because it makes
  call sites self-documenting.
- **Adopt**: *collection protocols as the iteration model* (`select:`,
  `collect:`, `inject:into:`) in typed form; every mainstream language
  converged on this.
- **Adopt**: *xUnit-style tests, refactoring and a browser-grade IDE as part
  of the language project*, since the tools, not the language, are what
  Smalltalk's successors copied.
- **Adapt**: *the uniform metaphor*. "Everything is an object" delivered
  simplicity but also metaclass confusion and unpredictable performance.
  Sisu should have a small number of uniform concepts (values, functions,
  records, interfaces) rather than one; uniformity of *interfaces* (every
  value can be inspected, printed, compared, serialised) is the part worth
  keeping.
- **Adapt**: *traits* as the mechanism for code reuse without inheritance;
  Smalltalk's own experience (and Rust's) shows traits plus composition beat
  class hierarchies.
- **Avoid**: *the image as the unit of source truth*. Text files in a
  version-controlled tree, reproducible builds, and a deployable artefact
  that does not contain the IDE. Liveness must be a *development-mode*
  property layered over files, as Pharo's Iceberg eventually conceded.
- **Avoid**: *open classes that any module can extend globally*; extension
  methods should be lexically scoped and imported (Kotlin, Swift, Rust
  trait impls) to keep the Smalltalk convenience without the global
  mutation.
- **Avoid**: *licensing and pricing that gate the runtime*; free, open
  tooling was decisive for Java and against Smalltalk.
- **Open questions raised**: (1) How much of fix-and-resume debugging can a
  native, AOT-compiled language offer? Edit-and-continue exists for C#/C++
  in Visual Studio with restrictions; a Sisu debug-build ABI that keeps
  frames reifiable is a research-grade commitment. (2) Is a "moldable"
  (per-type custom) inspector worth a runtime reflection surface in release
  builds, or only in debug builds? (3) Did Smalltalk lose on the image or on
  price? Both are cited; the counterfactual (free, file-based Smalltalk in
  1995) is Java with a worse type story, which suggests Java's static types
  and familiar syntax also mattered and the evidence does not isolate them.

## Sources

1. Alan C. Kay, "The Early History of Smalltalk", HOPL-II, 1993. https://worrydream.com/EarlyHistoryOfSmalltalk/
2. Daniel H. H. Ingalls, "Design Principles Behind Smalltalk", BYTE, August 1981. https://www.cs.virginia.edu/~evans/cs655/readings/smalltalk.html
3. Alan Kay, squeak-dev mailing list, "prototypes vs classes was: Re: Sun's HotSpot", 10 October 1998. http://lists.squeakfoundation.org/pipermail/squeak-dev/1998-October/017019.html
4. Wikipedia, "Smalltalk" (history, licensing, influence, postcard). https://en.wikipedia.org/wiki/Smalltalk
5. Kent Beck, "Simple Smalltalk Testing: With Patterns", 1994 (SUnit); Beck and Gamma, JUnit, 1997. Don Roberts, John Brant, Ralph Johnson, "A Refactoring Tool for Smalltalk", TAPOS 3(4), 1997.
6. Adele Goldberg, David Robson, *Smalltalk-80: The Language and its Implementation*, 1983; and the ParcPlace/Digitalk/ObjectShare history as summarised in [4].
7. Pharo team, "A note on the Pharo 14 release", 29 June 2026; Pharo news page, retrieved October 2026. https://pharo.org/news/2026-06-29-Pharo14.html, https://pharo.org/news
8. Pharo 13 and 13.1 release announcements, 21 May and 26 June 2025. https://pharo.org/news/2025-05-21-pharo13-released.html
9. L. Peter Deutsch, Allan M. Schiffman, "Efficient Implementation of the Smalltalk-80 System", POPL 1984. David Ungar, "Generation Scavenging", 1984.
10. Nathanael Schärli, Stéphane Ducasse, Oscar Nierstrasz, Andrew Black, "Traits: Composable Units of Behaviour", ECOOP 2003.
11. Gilad Bracha, David Griswold, "Strongtalk: Typechecking Smalltalk in a Production Environment", OOPSLA 1993.
