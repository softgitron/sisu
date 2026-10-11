# Forth (and concatenative languages: Factor, Joy)

## Snapshot
| | |
|---|---|
| First appeared | Forth 1968-1970 (Chuck Moore, NRAO); FORTH, Inc. 1973; FORTH-79, FORTH-83, ANS Forth 1994, Forth 2012. Joy 2001 (von Thun); Factor 2003 (Slava Pestov). |
| Designed by | Charles H. Moore; standards by the FORTH-79/83 teams and ANSI X3J14 (Elizabeth Rather chair), then the Forth 200x committee. Factor: Slava Pestov and contributors. |
| Current version (Oct 2026) | Forth 2012 standard (Forth 200x process continues). Gforth 0.7.3 (2014, last stable; development snapshots since); SwiftForth (FORTH, Inc.), VFX Forth (MPE) commercial. Factor 0.101 (December 2025). |
| Governance | Forth 200x committee for the standard; each implementation independent; many Forths are single-person or single-company. |
| Typing | Untyped: the stack holds cells (machine words); Factor adds dynamic typing with object system and static stack-effect checking. |
| Memory | Manual (dictionary allotment, `ALLOT`, `ALLOCATE`); Factor has a generational GC. |
| Compilation | Threaded code (indirect/direct/subroutine threaded) or native code; the compiler is itself Forth words running interactively; Factor has an optimising native compiler. |
| Primary domains | Boot firmware (Open Firmware, IEEE 1275: Sun, Apple PowerPC, OLPC), embedded and instrument control, spaceflight (satellites, Space Shuttle experiments, Philae lander), historically handheld devices and some games. |
| Notable users | Sun and Apple (Open Firmware), FreeBSD's boot loader until 12.0 (2018, replaced by Lua), NASA/ESA payloads, FedEx package tracking (1980s), GreenArrays (Moore's GA144 chips). |

## Stated design goals

Moore built Forth as a personal programming system to get work done on
minicomputers with kilobytes of memory and no OS: a single tool that is
editor, compiler, interpreter and operating system, extensible by the user
in itself, and small enough to understand completely. The HOPL-II paper
describes the result: "the programmer's attention is never broken by the
procedural overhead of opening and closing files", and Mitch Bradley cited
"extensibility, allowing easy support of new hardware requirements and
features" as the reason Open Boot used Forth [1]. Moore's later work
(colorForth, GreenArrays) pushes further toward minimalism: the language
should be smaller, not larger, and the programmer should write only the
code the problem needs. Factor's goal is to keep the concatenative model
and make it a practical modern language (GC, objects, libraries, a native
compiler).

## Coverage of the 15 evaluation dimensions

1. **Syntax and readability.** Whitespace-separated words in RPN; no grammar beyond word definitions; stack comments `( a b -- c )` are the only signature; readable only in very short words.
2. **Type system.** None: cells are machine words; Factor adds dynamic types, classes and a static stack-effect checker.
3. **Memory management and safety.** Manual (dictionary `ALLOT`, `ALLOCATE`/`FREE`), raw addresses, no bounds or stack-depth checks; Factor has a GC and safe objects.
4. **Mutability, aliasing and value semantics.** Global mutable dictionary and variables; the stack is the only "value" discipline; no immutability.
5. **Error handling.** `ABORT"`, `CATCH`/`THROW` (ANS) as non-local exits with integer codes; Factor has typed exceptions and restarts (borrowed from CL).
6. **Concurrency and parallelism.** Implementation-specific multitasking (cooperative round-robin in polyForth and many embedded Forths); no standard; Factor has green threads.
7. **Polymorphism.** None in traditional Forth (words are monomorphic over cells); `DEFER`/`IS` for late binding; Factor has generic words with multiple dispatch.
8. **Metaprogramming.** Central: `IMMEDIATE` words run at compile time, `CREATE ... DOES>` defines defining words, parsing words read the input stream; the compiler is user-extensible.
9. **Modules, packages, build.** Wordlists/vocabularies; `INCLUDE` of source files; no package ecosystem; Factor has vocabularies with a central library and dependency metadata.
10. **Tooling.** The interactive interpreter is the tool; `SEE` decompiles words; Factor ships a listener, debugger, profiler, help system and GUI framework.
11. **Interop and platforms.** Assembler words for the target CPU; `C` interop varies by implementation (Gforth has a C interface, Factor a full FFI); runs on anything from 8-bit MCUs to x86-64 and in boot ROMs.
12. **Performance and compilation model.** Threaded code or native code; small and fast enough for firmware; Factor has an optimising compiler with inlining and escape analysis.
13. **Standard library and ecosystem.** ANS/Forth-2012 core and optional word sets (floating, strings, locals, files); otherwise per-implementation; Factor's library is broad for its size.
14. **Learnability.** Simple to start, hard to read others' code; the stack discipline and postfix notation are the barriers; little modern teaching material.
15. **Evolution and governance.** Forth 200x committee updates the standard slowly; Moore rejects standardisation; implementations diverge by design; Factor is maintained by a small group with irregular releases (0.100 in 2024, 0.101 in 2025).

## Strengths

### The stack as implicit argument passing, and concatenative composition
Words take arguments from the data stack and leave results on it; a
program is a sequence of words, and juxtaposition is function composition.
There are no named parameters, no call syntax and no expression grammar:
`: SQUARE DUP * ;` defines a word. Because every word is a function from
stack to stack, factoring a program into tiny words is free and refactoring
is cut-and-paste. Joy formalised this: a concatenative program is a
composition of functions on a stack, with algebraic laws, which is the
theoretical core later used by Factor (and, loosely, by PostScript and the
JVM/WebAssembly instruction sets, which are stack machines).

### Extreme minimalism and self-definition
A Forth kernel is a few kilobytes; most of the language (control flow
`IF ... THEN`, loops, the compiler's `:` and `;`, even `CREATE`/`DOES>` for
defining defining-words) is written in Forth. Control structures are
ordinary immediate words that compile branches, so users can add new ones.
The language can be brought up on new hardware in days with a dozen
primitives in assembly; this is why it dominated boot firmware and
instrument control for 30 years [1, 2].

### Interactive, bottom-up development on bare metal
Forth was interactive from 1970: type a word, it runs; define a word, test
it immediately, build the next layer on it. On an embedded board this works
over a serial line with no OS, debugger or host toolchain, which no other
language offered until MicroPython. Open Firmware exposes this to end
users: an `ok` prompt on a Sun or PowerPC Mac where device-tree nodes are
Forth words. The Space Shuttle experiments and satellite controllers cited
in [1] were developed and patched this way.

### Metaprogramming for free
`IMMEDIATE` words run at compile time and can generate code, parse ahead in
the input, or define new syntax; Factor turns this into parsing words and
macros. As in Lisp, a compiler available at run time plus almost no syntax
makes extension trivial.

## Weaknesses

### Readability: stack shuffling and the absence of names
Without named locals, intermediate values must be managed with `DUP`,
`SWAP`, `OVER`, `ROT`, `-ROT`, `2DUP`, `NIP`, `TUCK`; a word's meaning
depends on the stack picture, documented only by convention in a comment
`( a b -- c )`. Any reader must simulate the stack to follow the code.
Moore's response is that words should be so short (one line) that this is
not a problem, and Factor's data shows that discipline is needed: Factor
added lexical variables (`::`), `cleave`/`spread` combinators and a static
*stack-effect checker* that rejects words whose stack effect is not
inferable, which is the concatenative world's admission that unconstrained
stack juggling does not scale [3]. ANS Forth added `LOCALS|` as an optional
word set for the same reason.

### No types, no safety
A cell is a cell; address arithmetic is the memory model; a wrong `@` or
`!` corrupts the dictionary or crashes the machine. There are no bounds
checks, no stack-depth checks in traditional Forths (underflow is
undefined), no modules beyond vocabularies/wordlists. These are acceptable
in the firmware niche and disqualifying elsewhere.

### Fragmentation and the standard's weakness
Every Forth is different: FORTH-79, FORTH-83 (incompatible division
semantics), ANS 1994 (which removed most restrictions on implementations
rather than choosing [1]), Forth 2012; plus colorForth and dozens of
personal Forths. Moore himself regards standards as a mistake for a
language meant to be rebuilt per application. The consequence is no
shared library ecosystem: Forth code is reused by copying.

### Loss of the niche
Apple dropped Open Firmware with the Intel transition (2006); FreeBSD
replaced its Forth (FICL) loader scripting with Lua in 12.0 (2018) [4]; UEFI
is C; MicroPython and Rust took bare-metal interactivity and safety. Forth
remains in legacy firmware, some space and instrument programs, and hobbyist
chips.

## Designer retrospectives

- Rather, Colburn and Moore, "The Evolution of Forth" (HOPL-II, 1993) [1]:
  the language's strength was its productivity for small teams on small
  machines; the authors note that standardisation repeatedly traded away
  Moore's minimalism for compatibility and that Moore stopped participating.
- Moore, in talks and on colorForth (1990s-2000s): regards ANS Forth as too
  large and most Forth programs as over-engineered; his own practice is to
  rewrite the system per project rather than reuse code. This is a
  retrospective against *reuse* itself, and it is why Forth has no
  ecosystem.
- Pestov (Factor, 2003-2010 blog posts): stack-effect inference and lexical
  locals were added because real programs could not be maintained
  otherwise; Factor's design is a record of what the pure model lacked.

## Influence

Forth gave: the stack-machine execution model used by PostScript (a Forth
descendant, Adobe 1984), the JVM, CLR and WebAssembly bytecodes (as
compilation targets, not source languages); Open Firmware and device trees
(the Linux/ARM device tree format descends from Open Firmware's); RPN
calculators (HP); the concatenative family (Joy, Factor, Cat, Kitten,
Porth); threaded-code interpreters; and the idea of a REPL on bare metal
(MicroPython, Lua on embedded). It took from: RPN and the Burroughs stack
machines, Moore's personal tools, and (for Factor) Lisp and Smalltalk.

## Lessons for Sisu

- **Adopt**: *trivial bring-up on bare metal* as a goal: a minimal runtime
  that needs no OS, so that embedded and firmware use is possible; Forth's
  record shows the demand exists, and Rust's `no_std` is the modern form.
- **Adopt**: *function composition as the normal way to build pipelines*
  (`|>` or method chains), which is the readable half of concatenative
  programming: left-to-right data flow without temporaries, *with* names.
- **Adapt**: *compile-time execution with the full language available*
  (Forth's `IMMEDIATE`, Lisp's macros, Zig's `comptime`) but under static
  checking; Factor's stack-effect checker is the precedent for making
  metaprogramming output verifiable.
- **Avoid**: *implicit argument passing*: Factor's and ANS Forth's additions
  of locals and effect checking are the evidence that unnamed intermediate
  values do not survive maintenance.
- **Avoid**: *rebuilding the language per project* as a culture; Sisu needs
  a stable core and a library ecosystem, which Forth deliberately never had.
- **Open questions raised**: (1) How small can a language's runtime be while
  keeping safety? Forth's few kilobytes are the lower bound; Rust `no_std`
  and Zig are the current answers, and Sisu should state its target. (2) Is
  interactive bare-metal development (a REPL over serial) worth designing
  for in an AOT language, as MicroPython and Forth show demand for?

## Sources

1. Elizabeth D. Rather, Donald R. Colburn, Charles H. Moore, "The Evolution of Forth", HOPL-II, 1993. https://www.forth.com/resources/forth-programming-language/
2. Forth 2012 Standard (Forth 200x committee). https://forth-standard.org/
3. Factor documentation: stack effect checking, lexical variables, cleave/spread combinators; Factor 0.101 release, December 2025. https://www.factorcode.org/
4. FreeBSD 12.0 release notes: "The default libstand(3) interpreter has been changed to Lua." https://www.freebsd.org/releases/12.0R/relnotes/
5. Wikipedia, "Forth (programming language)" (Open Firmware, spaceflight uses, implementations, Forth-2012). https://en.wikipedia.org/wiki/Forth_(programming_language)
6. Manfred von Thun, "Joy: Forth's Functional Cousin", EuroForth 2001.
7. Charles H. Moore, "Programming a Problem-Oriented Language", 1970 (unpublished manuscript, widely circulated).
