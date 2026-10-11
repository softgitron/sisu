# Lua

## Snapshot
| | |
|---|---|
| First appeared | 1993, Tecgraf, PUC-Rio, Brazil (born from the DEL and SOL data-description languages for Petrobras) |
| Designed by | Roberto Ierusalimschy, Luiz Henrique de Figueiredo, Waldemar Celes; the same three people still decide every feature |
| Current version (Oct 2026) | 5.5.1 (3 Aug 2026); 5.5.0 (22 Dec 2025); 5.4 branch still maintained (5.4.9, Aug 2026) |
| Governance | "Closed-development, open-source": the three authors accept suggestions but never outside code; a feature is added only when all three agree [1] |
| Typing | Dynamic, strong (no implicit string/number coercion except in arithmetic and concatenation; see regrets); eight types: nil, boolean, number (integer and float since 5.3), string, table, function, userdata, thread |
| Memory | Incremental mark-and-sweep GC with a generational mode (5.4; improved in 5.5); weak tables; finalizers (`__gc`); user-supplied allocator |
| Compilation | Register-based bytecode VM in ANSI C, about 32,000 lines; LuaJIT (separate project, Lua 5.1 semantics) is a tracing JIT |
| Primary domains | Embedded scripting in C/C++ applications: games (Roblox, World of Warcraft, many engines), Neovim, Redis, Nginx/OpenResty, Wireshark, Adobe Lightroom, routers and set-top boxes |
| Notable users | Roblox (Luau dialect), Blizzard, Valve, Neovim, Redis, Cloudflare (OpenResty), Adobe, Wireshark |

Usage: Stack Overflow 2025 reports 9.2% of all respondents (8.6% of
professionals) used Lua, higher than Ruby, largely via game scripting and
Neovim configuration [2].

## Stated design goals

lua.org states them directly: "provide meta-mechanisms for implementing
features" instead of many features; "keep the language small, while allowing
the semantics to be extended in unconventional ways"; "a fast language engine
with a small footprint that you can embed easily into your application" [3].
The CACM 2018 article: "exactly one general mechanism for each major aspect of
programming: tables for data; functions for abstraction; and coroutines for
control" [4]. The HOPL history: "Lua provided mechanisms, not policy -- a
tenet that we have tried to follow closely ever since", and "simplicity is
our most important asset. Most other qualities of Lua -- speed, small size,
and portability -- derive from its simplicity" [1]. Numbers: about 32,000
lines of C; 390 KB tarball; a 293 KB interpreter and 484 KB library on 64-bit
Linux; builds "on all platforms that have a standard C compiler"; MIT [3].

## Strengths

### Minimalism and embeddability as the primary design constraint

The C API is a stack-based protocol (`lua_pushnumber`, `lua_gettable`,
`lua_call`) that solves two mismatches at once, static-versus-dynamic typing
and manual-versus-GC memory, because values on the API stack are never
collected and vanish with the C frame [1]. Lua never touches the C stack, takes
a user-supplied allocator, can be built freestanding, and keeps all state in
`lua_State`. The evidence is adoption exactly where Python and JavaScript
cannot go: consoles, routers, Redis, Nginx request handlers, and games, where
gamedev.net polls placed Lua first as early as 2003 [1].

### Tables as the single data structure

Arrays, records, hash maps, sets (keys as elements), objects, classes,
modules, namespaces and environments are all tables, with a hybrid array/hash
implementation that makes `t[1..n]` as fast as a C array. The authors' own
retrospective: "One thing that has worked really well was the early decision
(made in Lua 1.0) to have tables as the sole data-structuring mechanism",
having "resisted user pressure to include other data structures, mainly
'real' arrays and tuples, first by being stubborn, but also by providing
tables with an efficient implementation" [1]. Metatables (5.0, replacing fallbacks and tag
methods) are themselves ordinary tables, which is how inheritance by
delegation, operator overloading, classes and proxies are implemented in user
code rather than in the VM [1]; the grammar never needed a `class` keyword,
which is why Luau, Teal, Fennel and MoonScript could add types or syntax
without forking it. Lua 5.5 adds `table.create` and compact array storage
reported to cut memory for large arrays by about 60% [5].

### Asymmetric, stackful coroutines

Lua 5.0 (2003) added coroutines as first-class values with `coroutine.create`,
`resume` and `yield`. The HOPL paper explains the choice: full first-class
continuations and symmetric coroutines were implemented experimentally but
could not coexist with the C API, because a captured continuation may include
C frames; asymmetric coroutines capture only the partial continuation up to
the matching `resume`, so "the only restriction is that we cannot yield across
a C call" [1]. Because they are stackful (a yield can occur deep inside nested
Lua calls, not only in the generator body), they implement generators,
cooperative schedulers, symmetric coroutines and one-shot continuations in
library code. De Moura and Ierusalimschy's "Revisiting Coroutines" (TOPLAS
2009) is the standard citation; Ruby's fibers, Squirrel, Janet and Wren adopt
the same stackful asymmetric model, while Kotlin, Python and JavaScript chose
stackless (CPS-transformed) designs, which is the live trade-off. The 5.2
continuation API (`lua_callk`/`lua_yieldk`) was the fix for the
yield-across-C limitation.

### LuaJIT: what a dynamic language can do

Mike Pall's LuaJIT 2 (2010 onward, Lua 5.1 semantics) is a tracing JIT with
an interpreter written in hand-optimised assembly, NaN-tagged values and a
best-in-class FFI (`ffi.cdef` parses C declarations and calls them directly,
with C structs as first-class values). It reaches C-comparable speed on
numeric loops and is why OpenResty can serve Cloudflare-scale traffic in Lua.
Its status is also a lesson: a one-person rolling release with no numbered
stable version since 2017; distributions package dated snapshots
(2.1-20260824) or OpenResty's fork and back-port CVE fixes themselves [6].
The fastest implementation froze the language at 5.1, splitting the ecosystem.

### Lua 5.5's correction of global-by-default

Undeclared variables were global, the top beginner trap for 30 years (a typo
in a `local` silently creates a global). Lua 5.5 adds `global` declarations so
globals can be required to be declared, and makes for-loop variables
read-only [5]. Changing the default after three decades is direct evidence
that "undeclared means global" was a mistake.

## Weaknesses

### No standard package story

The authors' own regret: "we should have provided a precise set of policies
for modules and packages earlier. The lack of a common policy ... prevents
different groups from sharing code and discourages the development of a
community code base" [1]. `require` arrived in 5.1 (2006); LuaRocks (2007) is
third-party, has no lockfile by default, and is split across 5.1/LuaJIT and
5.4. Most Lua is written against a host's own module conventions. This is the
clearest case in this research of "mechanisms only" failing at ecosystem level.

### Version fragmentation

5.1 (2006, LuaJIT), 5.2 (2011), 5.3 (2015, integers), 5.4 (2020) and 5.5
(2025) are each slightly incompatible because the designers "considered
simplicity and elegance more important than compatibility"; Tecgraf itself
"never upgraded from Lua 3.2 to Lua 4.0 because of the big changes in the
API" [1]. Hosts therefore pin a version forever (Roblox, WoW, Redis and Neovim
on 5.1 derivatives) and libraries target the lowest common denominator.

### 1-based indexing and `nil` holes

Indices start at 1, inherited from SOL's lists whose "keys were implicit
natural numbers starting at 1" and an audience of engineers [1]. Every C
boundary and every transcribed algorithm pays an off-by-one tax, and no
general-purpose language designed since copied it (Julia, R and MATLAB are
numerical lineages). Because `nil` in a table means "absent", arrays with
holes have an ill-defined length (`#t` is a "border"), a direct consequence
of tables being the only structure.

### Designer-acknowledged regrets: two false values, `nil, err`, coercion

"We should have introduced booleans from the start ... we now have two false
values: nil and false", and the error convention of returning `nil` plus a
message "would have been better if false had been used" [1]. Error handling
is otherwise `error(value)` plus `pcall`, untyped and without a traceback
unless `xpcall` is used; "return nil and a message" scales only in small
programs. "Automatic coercion of strings to numbers in arithmetic operations,
which we took from Awk, could have been omitted" [1]; `"10" + 1` is `11`, and
5.4 moved the coercion into string metamethods so it can be disabled. The
standard library is deliberately tiny (strings with Lua patterns rather than
regex, tables, math, I/O, OS, coroutines, `utf8`): no sockets, JSON or
threads, so every host (Roblox's Luau with its own gradual type system,
Neovim, OpenResty) grows its own, and the largest Lua population programs in
a dialect the core team does not control.

## Designer retrospectives

The HOPL III "Retrospect" section is the primary source [1]: tables as the
sole structure and portability worked; being "raised by a small committee"
worked ("most successful languages are raised rather than designed"); closed
development with open source worked; aggressive removal of superseded
features is not regretted but the team has "become more conservative";
booleans should have existed from the start; string-to-number coercion should
have been omitted; module and package policies should have come earlier; a
macro facility was endlessly requested and never agreed. Games were never a
target; the fit came from portability, embeddability and simplicity for
non-programmers. The 2018 CACM article restates one-mechanism-per-aspect and
defines a policy as "a way of combining existing mechanisms to build a new
abstraction" [4].

## Influence

Copied by others: the embeddable stack-based C API (Wren, Squirrel, Janet,
AngelScript); metatable-style delegation (Io, Wren, Luau); asymmetric stackful
coroutines (Ruby fibers, Squirrel, Janet, Wren); `<close>` variables (5.4) as
Lua's `with`/`using`; Luau's gradual types borrow from TypeScript. Copied
from others: Scheme (first-class functions, proper tail calls, lexical
scoping), Self and NewtonScript (delegation), Awk (coercion, associative
arrays), Modula/Pascal (syntax, `repeat ... until`), CLU (multiple returns),
SOL and DEL (table constructors `{x=1, y=2}`), Lisp (nil as false).

## Lessons for Sisu

- **Adopt**: a small, stable, versioned embedding API designed on day one
  (host calls language and language calls host), with no hidden global state
  and a pluggable allocator; one general mechanism per aspect (data,
  abstraction, control) with sugar on top; asymmetric stackful coroutines as
  the primitive under generators, async and schedulers, with "cannot yield
  across FFI" made explicit in the types; a small-committee veto on features
  ("much easier to add features later than to remove them").
- **Adapt**: "mechanisms not policies" for the core, but policies from the
  standard toolchain for everything the designers regretted leaving open:
  module layout, package manager with lockfile, error convention, string
  formatting. Tables as a universal structure suit a scripting layer; a typed
  language needs distinct array, map and record types behind one indexing and
  iteration protocol. Version breaks were survivable only because hosts pin
  versions; Sisu needs editions instead.
- **Avoid**: 1-based indexing; undeclared-means-global; two false values and
  `nil`-as-absent in collections; string/number coercion; a standard library
  so small that every host reinvents strings, JSON and sockets; letting the
  fastest implementation freeze an old language version.
- **Open questions raised**: how small a core can be before the ecosystem
  fragments (Lua is the limit case: the core is loved, the ecosystem is split
  per host); whether closed-development triumvirate governance beats RFC
  processes (Lua's consistency says yes; it does not scale past one
  implementation); whether Sisu should offer an embeddable subset for
  configuration and plugins, since every large application eventually embeds
  a scripting language.

## Sources

1. R. Ierusalimschy, L. H. de Figueiredo, W. Celes, "The Evolution of Lua", HOPL III (ACM, 2007). https://www.lua.org/doc/hopl.pdf
2. Stack Overflow Developer Survey 2025, Technology. https://survey.stackoverflow.co/2025/technology (2025)
3. lua.org, "About Lua". https://www.lua.org/about.html (2026)
4. Ierusalimschy, de Figueiredo, Celes, "A Look at the Design of Lua", CACM 61(11). https://www.lua.org/doc/cacm2018.pdf (2018)
5. Lua version history (5.5.0 22 Dec 2025; 5.5.1 3 Aug 2026) and the 5.5 manual. https://www.lua.org/versions.html ; https://www.lua.org/manual/5.5/ ; Phoronix, "Lua 5.5 Released With Declarations For Global Variables" (Dec 2025)
6. LuaJIT. https://luajit.org/ ; Debian package tracker (snapshot 2.1-20260824; OpenResty branch) https://tracker.debian.org/pkg/luajit ; Ubuntu notices for CVE-2024-25176/7/8 (2026)
7. A. L. de Moura, R. Ierusalimschy, "Revisiting Coroutines", ACM TOPLAS 31(2) (2009). https://www.inf.puc-rio.br/~roberto/docs/MCC15-04.pdf
8. R. Ierusalimschy, "Small is Beautiful: the Design of Lua" (2012). https://www.inf.puc-rio.br/~roberto/talks/ppl-2012.pdf
9. Luau https://luau.org/ ; Teal https://github.com/teal-language/tl ; LuaRocks https://luarocks.org/
