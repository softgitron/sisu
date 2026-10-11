# TypeScript

## Snapshot
| | |
|---|---|
| First appeared | 1 October 2012 (TypeScript 0.8, Microsoft); 1.0 in April 2014; 2.0 (control-flow analysis, strict null checks) in September 2016 |
| Designed by | Anders Hejlsberg (lead architect; also Turbo Pascal, Delphi, C#) with the Microsoft TypeScript team (Daniel Rosenwasser, Ryan Cavanaugh and others) |
| Current version (Oct 2026) | 7.0 (8 July 2026), the first release of the native Go port; 6.0 (6 March 2026) was the last JavaScript-based compiler; 5.9 (July 2025) |
| Governance | Microsoft-owned open-source project (Apache 2.0); design meetings and notes public on GitHub; no foundation or external committee; language features follow TC39 |
| Typing | Static, structural, gradual (`any`), deliberately unsound; fully erased at run time |
| Memory | JavaScript's (tracing GC in the host engine); TypeScript adds nothing |
| Compilation | Transpiles to JavaScript (or erases types only: Node, Bun, Deno, esbuild, SWC); the checker is separate from emit |
| Primary domains | Everything JavaScript is used for; dominant in front-end frameworks (Angular, Next.js, SvelteKit generate TS by default) and large Node code bases |
| Notable users | Microsoft (VS Code, Office web), Google (Angular), Airbnb, Slack, Bloomberg, Shopify, most open-source JavaScript libraries |

Adoption: State of JS 2025 reports 40% of respondents write exclusively
TypeScript (34% in 2024, 28% in 2022) against 6% writing only JavaScript [1].
Stack Overflow 2025: 43.6% of all respondents and 48.8% of professionals use
TypeScript [2]. GitHub Octoverse 2025: TypeScript became the most used language
on GitHub by monthly contributors in August 2025 (about 2.64M, 66% year-on-year
growth), passing Python; GitHub attributes this to AI-assisted coding
favouring typed languages and to frameworks defaulting to TypeScript [3]. No
retrofit of a type system onto an existing dynamic language has come close to
this: Python's typing is used by 86% of a self-selected typing survey [4] but
checked by far fewer projects; Ruby's RBS/Sorbet, PHP's Psalm/PHPStan and
Hack, Dart's migration from optional to sound types, all remain minorities or
required a new language.

## Stated design goals

The TypeScript Design Goals wiki page (2014, still current) lists as goals:
"Statically identify constructs that are likely to be errors", "Impose no
runtime overhead on emitted programs", "Emit clean, idiomatic, recognizable
JavaScript code", "Align with current and future ECMAScript proposals",
"Preserve runtime behavior of all JavaScript code", "Avoid adding
expression-level syntax", "Use a consistent, fully erasable, structural type
system", and "Do not cause substantial breaking changes from TypeScript 1.0".
The explicit non-goals include: "Apply a sound or 'provably correct' type
system. Instead, strike a balance between correctness and productivity", "Add
or rely on run-time type information in programs, or emit different code
based on the results of the type system", "Exactly mimic the design of
existing languages", and "Aggressively optimize the runtime performance of
programs" [5].

The handbook states the consequence plainly: "TypeScript's type system allows
certain operations that can't be known at compile-time to be safe. When a type
system has this property, it is said to not be 'sound'. The places where
TypeScript allows unsound behavior were carefully considered" [6]. The
language's niche was defined by its constraint: describe the JavaScript people
already write, at scale, without changing what runs.

## Strengths

### The most successful type-system retrofit in history, and why

The mechanism is a combination of four decisions: (1) structural typing, so
existing untyped object literals and duck-typed APIs are describable without
nominal declarations; (2) `any` and gradual boundaries, so a file or a line can
be typed later; (3) erasure with zero runtime change, so adopting TypeScript
never changes behaviour or performance and can be reverted by deleting
annotations; (4) declaration files (`.d.ts`) so untyped libraries can be used
with types written by third parties. The evidence is the adoption data above
and the fact that TypeScript became the default output of the major
frameworks without any vendor forcing it. The counter-example that proves the
point is Dart 1.x's optional types, which were unsound and ignored by the VM,
and which Google replaced with a sound, mandatory system in Dart 2 (2018) once
it had the freedom of a new runtime; TypeScript never had that freedom and
chose the other side of the trade-off deliberately.

### Control-flow narrowing as the central ergonomic win

Union types and type guards arrived in 1.4 (2015); 2.0 (2016) added
control-flow-based type analysis, `strictNullChecks`, discriminated unions,
`never`, and the non-null assertion. After `if (typeof x === "string")`,
`if ("kind" in x)`, `if (x.kind === "circle")`, `if (x != null)`, a user-
defined `x is T` predicate, `instanceof`, `switch` on a tag, or an early
`return`, the variable's type is narrowed for the rest of the block, with
no cast and no new binding. Later refinements: `unknown` (3.0, 2018) as the
safe top type, `asserts x is T` (3.7), narrowing through destructured
discriminants (4.6), `satisfies` (4.9), inferred type predicates from function
bodies (5.5, 2024). Kotlin's smart casts (announced 2011, 1.0 in 2016) are the
older design, but TypeScript made flow-sensitive typing the expectation in
mainstream languages; Python checkers (mypy, pyright), Dart's type promotion,
PHP's Psalm/PHPStan, Luau and Swift's `if let` family all deliver the same
experience. The evidence that it matters: it is the feature that lets a
nullable type (`T | undefined`) be usable without an `Option` wrapper, which
is how TypeScript achieved null safety in an ecosystem where every API returns
`undefined` somewhere.

### Discriminated unions as a practical ADT

`type Shape = { kind: "circle"; r: number } | { kind: "square"; s: number }`
plus `switch (s.kind)` plus exhaustiveness via `never` is an algebraic data
type that (1) needs no runtime support, (2) serialises to JSON unchanged, and
(3) is what most libraries already did with a `type` field. Redux actions,
React's reducer patterns, protocol message types and compiler ASTs in the
ecosystem are all written this way. The limitation is that exhaustiveness is
opt-in (a `default: const _: never = s` idiom, or the `switch (true)` and
`noFallthroughCasesInSwitch` tricks) and that there is no pattern matching;
the TC39 pattern-matching proposal has sat at stage 1 since 2017.

### Inference strong enough that most code has few annotations

Return types, generic arguments at call sites, literal types in `const`
contexts, contextual typing of callback parameters, `const` type parameters
(5.0), and inference from mapped/conditional types mean idiomatic TypeScript
annotates function signatures and little else. This is what made the `strict`
mode defaults acceptable: `strictNullChecks` was opt-in in 2.0, `strict` became
the recommended `tsc --init` default in 2.3, and TypeScript 7.0 makes `strict`
the default for everyone [7].

### A type system expressive enough to type real JavaScript

Mapped types (2.1), `keyof` and indexed access types, conditional types with
`infer` (2.8, 2018), variadic tuple types (4.0), template literal types (4.1,
2020), recursive conditional types, `const` type parameters, and generics over
function overloads make it possible to type `Object.keys`, `pick`,
`Promise.all`, event-emitter maps, SQL-like builders, routing parameters
parsed from string literals, and ORMs (Prisma, Drizzle, tRPC and Zod are built
on these). This is the reason TypeScript could absorb the existing ecosystem
rather than demand it be rewritten in a typed style. The cost is in the
weaknesses below.

### Erasure made the language portable across every runtime

Because types never affect emit, any tool can strip them: Babel (2018),
esbuild, SWC, Bun and Deno run `.ts` directly, and Node 22.6 added
`--experimental-strip-types`, enabled by default from 23.6 (January 2025) and
22.18 LTS (2025), with the documentation now marking it Stable [8]. This is a
direct consequence of goal 9 ("fully erasable") and non-goal 5 (no emit based
on types), and it is why TypeScript could survive its own compiler being 10x
too slow: checking and running were separable.

### The Go port: the implementation-language lesson

Announced in March 2025 as "A 10x faster TypeScript" and shipped as 7.0 on
8 July 2026, the native compiler is a line-by-line port to Go that reports full
build speed-ups of 7.7x-11.9x (VS Code 125.7 s to 10.6 s; Sentry 139.8 s to
15.7 s; Playwright 12.8 s to 1.47 s) and editor time-to-first-error on VS Code
from 17.5 s to under 1.3 s, with `--checkers 8` giving 16.7x [7]. The stated
sources of the gain are native code and shared-memory multithreading, neither
available to a JavaScript program. Hejlsberg's public explanation of choosing
Go over Rust or C# (paraphrased from the typescript-go repository discussion
in 2025, not re-verified here) was that the existing checker is a graph of
mutually referencing, cyclic, garbage-collected objects with closures
everywhere, so a GC'd language with a similar shape allowed a mechanical port,
while an ownership-based language would have required a redesign. 7.0 ships
without a programmatic API, which breaks typescript-eslint, ts-jest and the
Vue/Svelte/Astro template checkers until a new API arrives in 7.1; Microsoft
published `@typescript/typescript6` with a `tsc6` binary as the bridge [7]. The
lessons: a compiler written in the language it compiles is excellent for
dogfooding and portability and bad for speed once the language is slow; a 10x
gain was available without algorithmic change; and the API surface of a
compiler is a compatibility promise as serious as the language.

## Weaknesses

### Deliberate unsoundness: where it leaks and whether users care

The documented holes [6]: method parameter bivariance (fixed for function-typed
properties by `strictFunctionTypes` in 2.6, 2017, but methods declared with
method syntax remain bivariant so that `Array<Dog>` is assignable to
`Array<Animal>`); covariant mutable arrays and properties; `any`; type
assertions (`x as T`) that are checked only for "comparability"; non-null
assertions `x!`; indexed access on arrays returning `T` instead of
`T | undefined` (opt-in fix: `noUncheckedIndexedAccess`, 4.1); optional
properties versus `undefined`-valued ones (`exactOptionalPropertyTypes`, 4.4);
excess property checks that apply only to fresh literals; `Function` and
`object` types; declaration files that may simply be wrong. Each of the opt-in
flags is an admission that the default was a mistake that could not be
changed under goal 11 (no substantial breaks). Whether users care: the
surveys do not list unsoundness as a top pain point; State of JS's top
language pain point is the *absence* of static typing in JavaScript, not the
quality of TypeScript's [1]. Teams that need guarantees treat `any` as a lint
error (`noImplicitAny`, `@typescript-eslint/no-explicit-any`) and use
`unknown` plus parsing. The honest reading is that unsoundness has been
acceptable because the alternative in TypeScript's position was not a sound
system but no system; a new language is not in that position.

### Type-level programming: power that becomes a maintenance and performance problem

Conditional, mapped, recursive and template-literal types make the type
system Turing complete (demonstrated in 2017 in TypeScript issue #14833).
Libraries use this to produce extraordinary inference (tRPC, Zod, Drizzle,
type-safe routers), and the costs follow: compile times that scale with type
instantiation depth, the error "Type instantiation is excessively deep and
possibly infinite" (TS2589, with hard limits on recursion depth), error
messages hundreds of lines long, and types that only their author can modify.
`--generateTrace` and `--extendedDiagnostics` exist specifically to debug type
checking performance; the TypeScript team's performance wiki is largely
advice on avoiding type-level constructs. The Go port made checking 10x faster
but did not change the complexity class of the problem. The lesson is that a
type system with no escape into an ordinary compile-time language will have
one invented inside it, badly.

### No runtime types: validation libraries as the missing half

Erasure means `JSON.parse` returns `any`, every network boundary is a lie
waiting to happen, and TypeScript cannot check what arrives at run time. The
ecosystem's answer is schema-first libraries that produce both a validator and
a static type: Zod (2020), io-ts, Yup, Valibot (2023), ArkType, TypeBox, Typia,
Effect Schema. Each is a separate language-in-a-library for describing types,
with its own inference quirks, and the pattern "parse, don't validate" is
reinvented in every project. Microsoft has refused to add runtime type
information (non-goal 5), reasonably, because `.d.ts` files describe code that
may not exist, but the consequence is that the single most common source of
type errors in deployed TypeScript, untrusted input, is outside the type
system. The lesson for a language that controls its runtime: types that exist
at run time (or a standard derive/reflect mechanism that generates parsers
from types) remove an entire category of library and bug.

### Non-erasable features: enums, namespaces, parameter properties, decorators

TypeScript's early years (2012-2015, before ES2015 shipped) added features
with runtime semantics: `enum` (numeric enums create bidirectional objects,
`const enum` inlines values and breaks isolated compilation), `namespace`
(pre-ES-module code organisation), constructor parameter properties, and
experimental (legacy) decorators. All conflict with the goal of alignment with
ECMAScript and the goal of erasability. The fixes: `isolatedModules` (1.5),
`verbatimModuleSyntax` (5.0, 2023), and `--erasableSyntaxOnly` (5.8, 2025),
which turns those features into errors so that code can run under Node's type
stripping, which deliberately refuses to transform them (they need the
separate `--experimental-transform-types` flag) [8]. The TC39 "type
annotations" (types-as-comments) proposal, championed by Microsoft in 2022 to
make erasable syntax part of JavaScript itself, remains at stage 1 as of this
research, with the committee having asked for a less TypeScript-specific and
smaller syntax [9]. Enums are now widely discouraged in favour of `as const`
object literals and union-of-literal types. Lesson: a language layered on
another must not add runtime semantics, and features added before the host
language settled became permanent liabilities.

### Declaration files and the DefinitelyTyped burden

DefinitelyTyped (2012, community-run, thousands of `@types/*` packages; exact
counts not verified in this research) supplies types for packages that do not
ship their own. The maintenance problems are structural: types are written by
someone other than the library author, drift from the library's actual
behaviour, lag releases, and have their own versioning that must be matched to
the library's by hand; TypeScript upgrades can break thousands of `.d.ts`
files at once, which is why goal 11 exists. Over time libraries have moved to
shipping their own types (framework templates generate them), and JSR (Deno's
registry, 2024) accepts TypeScript source directly so types are never separate
[10]. The lesson: types that live apart from code rot; a language should make
the interface of a compiled artifact derivable from the artifact itself.

### Build configuration surface, and governance by one company

`tsconfig.json` has over a hundred options, many of which (`module`,
`moduleResolution`, `target`, `esModuleInterop`, `baseUrl`, `paths`) exist to
cope with JavaScript's module mess rather than with types. TypeScript 6.0
deprecated and 7.0 removed `target: es5`, `downlevelIteration`, the `node10`/
`classic` resolvers, AMD/UMD/SystemJS output and `baseUrl`, and changed the
defaults to `strict: true`, `module: esnext`, `types: []` [7]; that cleanup was
possible only with a major-version line in the sand. Microsoft decides all of
this: design notes are public but there is no RFC process or foundation, and
the 7.0 API break was unilateral. In practice this has been benign (Apache
licence, fast releases, a port no community could have funded), and erasure
is the mitigation: because every runtime can strip types, TypeScript the
syntax could survive TypeScript the compiler.

## Designer retrospectives

- The design goals page itself is a standing retrospective: non-goal 3
  (soundness) and goal 11 (no substantial breaks since 1.0) explain every
  opt-in strictness flag listed above [5].
- The handbook's soundness note states that unsound places were "carefully
  considered" and that bivariance is "unsound, but useful and common", with
  the alternative requiring casts everywhere [6]. The team has never claimed
  the trade-off was free; `strictFunctionTypes` is their partial walk-back.
- Hejlsberg (talks and interviews, 2015-2025) has consistently said that
  TypeScript's success came from refusing to be a new language and from
  structural typing; the one regret he has voiced publicly is early features
  that are not erasable (enums, namespaces) and the pre-ES2015 module syntax.
- The 2025 "10x faster TypeScript" announcement is an implicit retrospective
  on implementation language: the team concluded that no amount of
  optimisation of the JavaScript compiler would deliver the required speed and
  that native code with shared-memory parallelism was necessary [7].
- The introduction of `--erasableSyntaxOnly` (5.8) and the TC39 type
  annotations proposal are admissions that TypeScript's non-erasable features
  were mistakes and that the long-term home of the syntax should be
  JavaScript itself [8][9].

## Influence

Copied by others: flow-sensitive narrowing as a mainstream expectation (Python
checkers, PHP analysers, Luau, Dart promotion); `unknown` as a safe top type;
discriminated unions by tag; `Protocol` in Python (structural typing, PEP 544)
and `TypedDict` (PEP 589) are explicitly modelled on TypeScript's structural
objects; template literal types inspired Python's `LiteralString` (PEP 675);
Luau (Roblox) and Sorbet (Ruby) and Hack (PHP) and Elixir's new gradual set-
theoretic types all cite the gradual, erasable model; `satisfies`-style
checking appears in Kotlin and Swift proposals; declaration files as a
pattern (Python's `.pyi` stubs, Ruby's `.rbs`, PHP's stub files); the
schema-to-type pattern (Zod) is being copied by Python's Pydantic v2 and
Kotlin's kotlinx.serialization.

Copied from others: structural typing and type inference from OCaml/Standard
ML via Hejlsberg's earlier work; generics and union-type ideas from C# and
Scala; smart casts from Kotlin (the older design); `readonly`, `Partial`-style
mapped types from the Flow type checker (Facebook, 2014) which TypeScript
competed with and absorbed; decorators from Python via Angular; `async`/`await`
emit from C#.

## Lessons for Sisu

- **Adopt**: flow-sensitive narrowing on unions, `null`/`undefined`-like
  values and tags, with no new binding required; discriminated unions as the
  everyday ADT, but with built-in exhaustiveness checking and pattern matching
  rather than the `never` idiom; a safe top type (`unknown`) distinct from an
  unsafe escape hatch; bidirectional inference good enough that signatures are
  the only required annotations; `strict` semantics as the only mode from
  version 1; `const`-context literal types; a deprecation-then-removal
  cadence with version lines (the 6.0 deprecations to 7.0 errors pattern).
- **Adapt**: structural typing where it buys interop (records, interfaces,
  function types) combined with nominal types where identity matters (units,
  IDs, newtypes), since TypeScript's purely structural system cannot
  distinguish `UserId` from `OrderId` without branding hacks. Gradual typing
  only at explicit boundaries (FFI, deserialisation), not as a pervasive
  `any`. Type-level computation only through an ordinary compile-time
  function language with the same syntax as run-time code (Zig's approach),
  never through a second accidental language of conditional types. Types
  present at run time, or a standard derive that generates validators, so
  that parsing untrusted data is a language feature not a library genre.
- **Avoid**: deliberate unsoundness that cannot later be removed (bivariant
  methods, covariant mutable arrays); features with runtime semantics layered
  on a host (if Sisu ever targets JavaScript, emit only erasable
  constructs); types maintained separately from the code they describe
  (DefinitelyTyped); a hundred-option build configuration; writing the
  compiler in a language that cannot use multiple cores or native code if
  compile speed is a goal; a single-vendor decision process without a public
  proposal mechanism, even if the vendor is benevolent.
- **Open questions raised**: how much soundness users will tolerate paying for
  in annotations and refactoring effort, given that the largest typed-language
  population on earth chose an unsound system and does not list that as a pain
  point; whether structural typing's interop benefits outweigh its loss of
  nominal identity in a language that controls both sides; whether gradual
  typing is valuable at all in a new language or only as a migration device
  (TypeScript's `any` is a migration device that became permanent); whether
  the "10x from a port, no algorithmic change" result means Sisu's own
  compiler must be self-hosted only if Sisu is fast enough to host it.

## Sources

1. State of JavaScript 2025 results (survey Nov 2025), reported by InfoQ (Mar 2026) and DevClass (Feb 2026). https://www.infoq.com/news/2026/03/state-of-js-survey-2025 ; https://2025.stateofjs.com/
2. Stack Overflow Developer Survey 2025, Technology. https://survey.stackoverflow.co/2025/technology (2025)
3. GitHub Octoverse 2025. https://github.blog/news-insights/octoverse/octoverse-a-new-developer-joins-github-every-second-as-ai-leads-typescript-to-1/ (28 Oct 2025)
4. Meta Engineering, "Python Typing Survey 2025". https://engineering.fb.com/2025/12/22/developer-tools/python-typing-survey-2025-code-quality-flexibility-typing-adoption/ (Dec 2025)
5. TypeScript Design Goals (wiki). https://github.com/microsoft/TypeScript/wiki/TypeScript-Design-Goals (2014, current)
6. TypeScript Handbook, "Type Compatibility" including "A Note on Soundness" and "Function Parameter Bivariance". https://www.typescriptlang.org/docs/handbook/type-compatibility.html (current)
7. Daniel Rosenwasser, "Announcing TypeScript 7.0". https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/ (8 July 2026); Anders Hejlsberg, "A 10x Faster TypeScript" (Mar 2025); "Announcing TypeScript 6.0" (6 Mar 2026)
8. Node.js, "Modules: TypeScript" documentation and v22.18.0 release notes (type stripping enabled by default). https://nodejs.org/api/typescript.html ; https://nodejs.org/en/blog/release/v22.18.0 (2025); TypeScript 5.8 release notes, `--erasableSyntaxOnly` (2025)
9. TC39 proposal-type-annotations README (Stage 1). https://github.com/tc39/proposal-type-annotations (2022-2026)
10. Deno 2 announcement (JSR accepts TypeScript source). https://deno.com/blog/v2.0 (Oct 2024)
11. TypeScript 2.0 release notes (control flow analysis, discriminated unions, strictNullChecks). https://www.typescriptlang.org/docs/handbook/release-notes/typescript-2-0.html (2016)
12. TypeScript issue #14833, "Type system is Turing complete". https://github.com/microsoft/TypeScript/issues/14833 (2017)
13. TypeScript wiki, "Performance" (advice on type-level constructs and `--generateTrace`). https://github.com/microsoft/TypeScript/wiki/Performance
14. Alexis King, "Parse, don't validate". https://lexi-lambda.github.io/blog/2019/11/05/parse-don-t-validate/ (2019)
