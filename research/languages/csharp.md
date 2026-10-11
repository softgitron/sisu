# C#

## Snapshot
| | |
|---|---|
| First appeared | Announced June 2000; C# 1.0 shipped January 2002 with .NET Framework 1.0 and Visual Studio .NET; standardised as ECMA-334 (2002) and ISO/IEC 23270 (2003) [1] |
| Designed by | Anders Hejlsberg (chief architect; previously Turbo Pascal and Delphi), with Scott Wiltamuth and Peter Golde; Mads Torgersen is lead designer of the language today; Eric Lippert and Neal Gafter were long-time design team members |
| Current version (Oct 2026) | C# 14 (November 2025, with .NET 10 LTS). C# 15 is in public preview with the .NET 11 previews and ships November 2026 [1][2] |
| Governance | Microsoft's C# Language Design Meeting (LDM), with notes and proposals public in `dotnet/csharplang`; compiler in `dotnet/roslyn` (MIT); .NET Foundation (2014) holds some projects; ECMA TC49 standardises the language several versions behind |
| Typing | Static, nominal; reified generics (2005); flow-sensitive nullable reference types as warnings (2019); pattern matching (2017-); records (2020); `dynamic` escape hatch (2010); union types and `closed` hierarchies in C# 15 preview [2][3] |
| Memory | Generational, compacting tracing GC (workstation/server, concurrent/background modes, regions since .NET 7). Value types (`struct`) from 1.0; `Span<T>`/`ref struct` stack-only types (2017); `unsafe` pointers; Native AOT since .NET 7 |
| Compilation | Source to IL; RyuJIT with tiered compilation and dynamic PGO (default since .NET 8); ReadyToRun pre-compilation; Native AOT for closed-world binaries |
| Primary domains | Enterprise and web backends (ASP.NET Core), games (Unity, Godot), Windows desktop (WPF, WinForms), cross-platform apps (MAUI), cloud (Azure) |
| Notable users | Microsoft, Stack Overflow, Unity, Bing, Alaska Airlines, most Windows-centric enterprises |

Usage: 27.8 % of all respondents and 29.9 % of professional developers in the 2025 Stack Overflow survey (n = 31,771), effectively tied with Java [4]. TIOBE named C# its language of the year for 2023.

## Stated design goals

The ECMA-334 standard's own description is "a simple, modern, general-purpose, object-oriented programming language" intended for "developing software components suitable for deployment in distributed environments", with "strong type checking, array bounds checking, detection of attempts to use uninitialized variables, and automatic garbage collection" [1]. In practice the goals that have driven C# are visible in Hejlsberg's choices rather than the standard's prose:

- *Learn from Java, then diverge.* C# 1.0 "looked a lot like Java" (Microsoft's own history page) but rejected checked exceptions, kept value types and `unsafe`, added properties, events, delegates and attributes from Hejlsberg's Delphi work [1][5].
- *Components, not just classes.* Properties, events and attributes exist so that components can be assembled by tools (the Delphi/Visual Basic RAD lineage).
- *Keep evolving.* Hejlsberg's recurring theme is that a language must change or die; fourteen major versions in 24 years is the output. Torgersen's framing of recent releases is a set of themes: "removing ceremony, separating data from algorithms, and improved performance for the .NET Runtime" [1].
- *Pragmatic over pure.* Microsoft's internal feature rule, articulated by Eric Gunnerson in 2004, is that every feature "starts out in the hole by 100 points" and must earn its way in [6]. The kitchen-sink critique (see Weaknesses) is about whether that bar has held.

## Timeline of design-relevant decisions

| Year | Version | Decisions that matter |
|---|---|---|
| 2002 | C# 1.0 | Java-like core; properties, events, delegates, attributes, structs, `unsafe`; no checked exceptions; no generics |
| 2005 | C# 2.0 | Reified generics (Kennedy and Syme, 2001), iterators (`yield`), anonymous methods, nullable value types `T?` [1][7] |
| 2007 | C# 3.0 | LINQ: lambdas, expression trees, extension methods, anonymous types, `var`, object initialisers [1] |
| 2010 | C# 4.0 | `dynamic`, named/optional arguments, generic variance [1] |
| 2012 | C# 5.0 | `async`/`await`, the origin of the modern coloured-function model [1] |
| 2014 | Roslyn | Compiler rewritten in C# and open-sourced (April 2014); compiler-as-a-service; .NET Foundation founded |
| 2015 | C# 6.0 | Boilerplate removal: string interpolation, `?.`, expression-bodied members, `nameof` |
| 2016 | .NET Core 1.0 | Cross-platform, MIT-licensed runtime; start of the "two .NETs" period |
| 2017 | C# 7.0-7.2 | Tuples, pattern matching, local functions, `ref` returns; `Span<T>`, `ref struct`, `in`, `readonly struct` [1] |
| 2019 | C# 8.0 | Nullable reference types (warnings), default interface members (from Java 8), switch expressions, async streams, ranges [1][8] |
| 2020 | C# 9.0 / .NET 5 | Records, `init`, top-level statements, source generators; .NET Framework and Core unified |
| 2021 | C# 10 / .NET 6 | Record structs, global usings, file-scoped namespaces; hot-reload controversy (October) [9] |
| 2022 | C# 11 / .NET 7 | Generic math (static abstract interface members), raw strings, required members, list patterns; Native AOT production-ready [1] |
| 2023 | C# 12 / .NET 8 | Primary constructors, collection expressions, inline arrays; interceptors preview; dynamic PGO default [1] |
| 2024 | C# 13 / .NET 9 | `params` collections, `System.Threading.Lock`, `ref struct` in generics and interfaces [1] |
| 2025 | C# 14 / .NET 10 LTS | Extension members (properties, static members, operators), `field` keyword, null-conditional assignment [1][10] |
| 2026 | C# 15 / .NET 11 (preview) | Union types, `closed` hierarchies, extension indexers, labelled `break`/`continue`, first step of a new memory-safety model for `unsafe` [2] |

## Coverage of the 15 evaluation dimensions

1. **Syntax and readability.** C-family, very regular, with heavy use of contextual keywords to avoid breaking changes. Reads well after a year within one era of the language; across eras, the same program can be written in three or four styles (see Weaknesses). String interpolation, expression-bodied members and pattern matching are among the most readable in mainstream languages.
2. **Type system.** Static, nominal; reified generics with declaration-site variance; flow-sensitive nullability as warnings; records; sealed and `closed` hierarchies; union types arriving in 15; `dynamic` opts out of static typing per expression. Sound apart from `unsafe`, `dynamic` and the nullability warnings being advisory.
3. **Memory management and safety.** Generational compacting GC; value types and `Span<T>` reduce allocation; `unsafe` and `stackalloc` give raw access. Memory-safe by default; the C# 15 preview redefines `unsafe` to tag memory-accessing operations rather than pointer types [2].
4. **Mutability, aliasing and value semantics.** Mutable by default; `readonly` fields, `readonly struct`, `init`-only setters and records provide immutability idioms; structs are copied on assignment (with the classic "mutating a copy" trap). `ref`/`in`/`out` give controlled aliasing; `ref struct` prevents heap escape.
5. **Error handling.** Exceptions only; `Try*` methods by convention; `Result` types from libraries; union types in C# 15 provide the data shape but no propagation operator.
6. **Concurrency and parallelism.** Threads, thread pool, Task Parallel Library, `async`/`await` with `Task`/`ValueTask`, `IAsyncEnumerable`, `Parallel.ForEachAsync`, channels. Coloured functions; no structured concurrency; no static data-race prevention. Green threads were evaluated and shelved by the .NET team in 2023.
7. **Polymorphism.** Single inheritance, interfaces with default members, reified generics with constraints, static abstract interface members (generic math), extension members, operator overloading, `dynamic`.
8. **Metaprogramming.** Attributes and reflection; expression trees (code as data, since 2007); Roslyn source generators (2020) and analyzers; interceptors (preview); no macros. The generator model is the current answer to AOT-compatible code generation (JSON, regex, logging, P/Invoke).
9. **Modules, packages, build.** Assemblies and namespaces; NuGet (2010) as the single package manager with a central registry; SDK-style project files; `dotnet` CLI; lock files optional; package signing and source mapping exist but no transparency log.
10. **Tooling.** Roslyn powers Visual Studio, VS Code (C# Dev Kit, proprietary licence), Rider and OmniSharp with the same semantic model; fast incremental compilation; hot reload; mature debugger and profilers; `dotnet test`, `dotnet format`, BenchmarkDotNet.
11. **Interop and platforms.** P/Invoke with source-generated marshalling, COM, C++/CLI on Windows; Linux, macOS, Windows, iOS, Android, WebAssembly (Blazor), consoles via Unity. Assembly format is the ABI; Native AOT exports C-callable functions.
12. **Performance and compilation model.** Tiered JIT with on-stack replacement and dynamic PGO; near-C performance for tuned code using structs and spans; .NET's own HTTP stack and regex engine are the showcases. Native AOT trades peak throughput for startup and size.
13. **Standard library and ecosystem.** Very large, well-documented base class library; ASP.NET Core, Entity Framework and the Azure SDK are first-party; NuGet ecosystem is large but more Microsoft-centred than npm or Maven Central.
14. **Learnability.** Approachable for Java or TypeScript programmers; beginner traps are async misuse, struct copies, event leaks, `==` on reference types versus records, and choosing among overlapping features. Top-level statements made the first program one line in 2020.
15. **Evolution and governance.** Annual releases tied to .NET; public LDM notes; preview language versions; warning waves to tighten rules without breaking; Microsoft decides, the community advises; trust episodes in 2021 and 2023.

## Strengths

### Deliberate, fast evolution with public design notes
- **Feature.** One major version per year since 2017, each with a stated theme; every design decision recorded in LDM notes and proposals in `dotnet/csharplang`; "preview" language versions let features ship behind a flag before they are final [1].
- **Why it is good.** C# has absorbed functional programming (lambdas, LINQ, pattern matching, records, unions), low-level programming (spans, ref structs, function pointers) and null safety without a fork or a "C# 2" break. Features arrive with analyzers and fixers that migrate code.
- **Evidence.** C# shipped generics (2005) nine years before Java's lambdas and records (C# 9, Nov 2020) in step with Java 16 (Mar 2021); extension members (C# 14) and union types (C# 15) are responses to Kotlin and F# that arrived within a few years of the demand [2][10]. Java's six-month cadence (2018) and Kotlin's public KEEP proposals copy the mechanism.

### `async`/`await`: the origin of modern asynchronous syntax
- **Feature.** C# 5 (2012) turned a method into a state machine with `async` and `await`, built on `Task<T>`; the pattern came from F#'s asynchronous workflows (2007) and C# 2 iterators [1].
- **Why it is good.** Sequential-looking code for asynchronous I/O, with exceptions, loops and `try/finally` working across awaits; the compiler, not the programmer, builds the continuation.
- **Evidence.** Copied by JavaScript (ES2017), Python 3.5 (2015), Dart, Kotlin (`suspend`, 2017), Rust (2019, postfix `.await`), Swift (5.5, 2021), Hack, and C++20 coroutines at a lower level. No feature of C# has been more widely adopted; the costs are under Weaknesses.

### LINQ and expression trees
- **Feature.** C# 3 (2007) added lambdas, extension methods, anonymous types, local inference and query comprehension syntax, plus expression trees that reify a lambda as data so that a provider can translate it (to SQL, for example) [1].
- **Why it is good.** One query vocabulary over in-memory collections, databases, XML and remote services; expression trees made Entity Framework and similar providers possible without a separate query language.
- **Evidence.** Java streams (2014), Kotlin collection functions, Swift's and Dart's functional collection APIs all follow `Select`/`Where`/`Aggregate` shapes; no other mainstream language has copied expression trees, which suggests the comprehension syntax is the portable part and provider-translatable trees are a niche win.

### Reified generics with value-type specialisation
- **Feature.** Generics are represented in IL and the runtime (Kennedy and Syme, PLDI 2001); `List<int>` stores unboxed ints; the JIT specialises code per value-type instantiation and shares it for reference types [7].
- **Why it is good.** No boxing, runtime type information (`typeof(T)`, `is List<string>`), generic constraints that include `struct`, `unmanaged` and, since C# 11, static abstract members for generic math [1].
- **Evidence.** Java's Valhalla is a twelve-year effort to approximate what the CLR had in 2005 (see `java.md`); Kotlin's `reified` inline functions and Swift's specialisation are partial equivalents. The 2001 design is the reference point for "generics done right in a managed runtime".

### Value types, `Span<T>` and `ref struct`: low-level performance in a managed language
- **Feature.** `struct` since 1.0; `Span<T>`/`ReadOnlySpan<T>` (C# 7.2, 2017) as a unified view over arrays, stack memory and native memory; `ref struct` types that the compiler forbids from escaping to the heap; `stackalloc` into spans; `ref` locals and returns; `in`/`ref readonly` parameters; inline arrays (C# 12); function pointers (C# 9) [1].
- **Why it is good.** Hot paths can avoid allocation entirely while staying memory-safe: the compiler's lifetime rules for `ref struct` are a small, decidable borrow checker. The base library was rewritten on spans (parsing, formatting, HTTP, regex).
- **Evidence.** .NET's Kestrel server and the yearly "Performance Improvements in .NET" posts by Stephen Toub document order-of-magnitude gains from spans; TechEmpower benchmarks place ASP.NET Core among the fastest managed frameworks. C# 13 and 14 extended `ref struct` to generics and interfaces and added implicit span conversions [1][10]. Java's `MemorySegment` (FFM API) and Swift's non-escaping buffers are later analogues.

### Properties, events, delegates, indexers: first-class components
- **Feature.** Properties with `get`/`set` (and now `field`-backed accessors, C# 14), events with `+=`/`-=`, delegates as typed function references, indexers, operator overloading [1][10].
- **Why it is good.** Data binding, serialisation and designers work against a uniform surface; Java's getter/setter convention is the counter-example.
- **Evidence.** Swift, Kotlin, Dart and TypeScript all have properties; Kotlin and Swift copied extension methods; Dart added extension methods in 2019.

### Pattern matching, records and (in 15) unions
- **Feature.** Type, property, positional, relational, logical and list patterns (C# 7-11); `switch` expressions; records with value equality, `with` expressions and positional deconstruction (C# 9-10); union types with exhaustive `switch` and `closed` hierarchies (C# 15 preview) [1][2][3].
- **Why it is good.** Data-oriented code without inheritance hierarchies; exhaustiveness checked by the compiler.
- **Evidence.** Java's JEP 441 and Kotlin's `when` converged on the same shape; C# 15's unions are explicitly aimed at "result-or-error returns" and "replacing marker interfaces or abstract base classes" [3].

### Roslyn: the compiler as a library
- **Feature.** The C# compiler is a reusable API (syntax trees, semantic model, incremental analysis) that powers the IDE, analyzers, code fixes, source generators and scripting; open-sourced under MIT in April 2014.
- **Why it is good.** One semantic engine serves compilation and every tool, so IDE features are exact and language features ship with refactorings. Analyzers let the team tighten rules ("warning waves") without breaking the language [1].
- **Evidence.** TypeScript's language service (also Hejlsberg), rust-analyzer, Kotlin's compiler plugins and Swift's SourceKit follow the same "compiler serves the editor" model; Microsoft's own nullable-reference-types rollout was feasible only because the compiler could run as an analyzer over millions of lines.

### One package manager, one CLI, one project format
- **Feature.** NuGet (2010) with nuget.org as the registry; SDK-style project files; the `dotnet` CLI for build, test, run, publish, format and tool installation.
- **Evidence.** Contrast with Java's Maven/Gradle split (`java.md`). The weakness is the absence of a transparency log or mandatory lockfile.

### Cross-platform unification
- **Feature.** .NET Core (2016) and the .NET 5 unification (2020) replaced the Windows-only .NET Framework with an MIT-licensed, cross-platform runtime; Native AOT (2022) produces self-contained binaries; Blazor targets WebAssembly.
- **Evidence.** Linux is now the primary server target for ASP.NET Core deployments on Azure; Unity and Godot bring C# to game consoles and mobile. The cost was a decade of confusion (`.NET Standard`, `.NET Framework 4.8` frozen, Xamarin/MAUI churn), covered below.

## Weaknesses

### `async`/`await`: function colouring and its pathologies
- **Feature.** Asynchronous methods return `Task`/`ValueTask`; calling one from synchronous code requires blocking (`.Result`, `.Wait()`), which deadlocks under a `SynchronizationContext` (classic ASP.NET, UI threads).
- **Failure.** Bob Nystrom's 2015 essay "What Color is Your Function?" names the problem C# created: every API is duplicated (`Read`/`ReadAsync`), asynchrony is viral through call chains, and library authors must choose colours for their users [11]. Specific C# traps:
  - `async void` methods: exceptions escape to the thread pool and crash the process; allowed only because event handlers return `void`;
  - sync-over-async deadlocks when `.Result` or `.Wait()` is called under a `SynchronizationContext`;
  - `ConfigureAwait(false)` boilerplate in libraries (Toub's 2019 FAQ exists because the rule is non-obvious) [12];
  - lost stack context in traces and profilers across continuations;
  - `ValueTask` rules (await once, never concurrently) that are enforced only by documentation;
  - "fire and forget" tasks whose exceptions are observed late or never.
- **Evidence.** The runtime team wrote a multi-page "async guidance" document for ASP.NET Core; Java chose the opposite model in 2023 (JEP 444 argues the async style loses loops, `try/catch`, stack traces and profiler attribution); .NET evaluated green threads in 2023 and shelved them because of the installed base of `async` code. C# is now the main evidence for the cost of coloured functions, having been the main evidence for their benefit.

### Nullable reference types: a warning-level retrofit
- **Feature.** C# 8 (2019) added `string?` annotations and flow analysis; unannotated types in enabled code are non-nullable by default [8].
- **Failure.** By design "all enforcement of null behavior will be in the form of warnings, not errors"; the `!` operator suppresses them; annotations "have no runtime effect" and "produce the same IL as before" [8]. The gaps that follow:
  - `NullReferenceException` remains possible wherever a warning was ignored, suppressed with `!`, or disabled per file;
  - unannotated ("oblivious") libraries silently turn checking off at the boundary, and annotations added later by a library create new warnings in consumers;
  - `default(T)` for a non-nullable `T` is null, and `new string[10]` is an array of nulls typed as non-nullable (the design "favors convenience over strictness" here to avoid a flood of warnings [8]);
  - constructors that do not initialise non-nullable fields needed the `required` modifier (C# 11) to close the gap;
  - deserialisers, Entity Framework, reflection and `Unsafe` bypass the analysis entirely;
  - generic `T?` means different things for structs and classes, and needed C# 9 changes to be expressible for unconstrained `T`;
  - the analysis is intra-procedural, so post-conditions must be stated with attributes (`[NotNullWhen]`, `[MemberNotNull]`), which library authors often omit.
- **Evidence.** Microsoft annotated its own base libraries over .NET 5 and 6, which demonstrates the migration cost; adoption in the ecosystem remains partial. Kotlin (errors, 2016), Swift (errors, 2014) and Dart (sound null safety, 2021, errors) show that errors are achievable; TypeScript's `strictNullChecks` is the model C# followed, and Torgersen cites its experience as the justification [8]. The retrofit is a qualified success: much better than nothing, not a guarantee.

### Exceptions as the sole error mechanism
- **Feature.** All failures are exceptions; the base library signals expected failures with the `Try*` convention (`int.TryParse(s, out v)`), which is itself evidence that exceptions were the wrong tool for expected outcomes.
- **Failure.** Exceptions are invisible in signatures (Hejlsberg's stated preference after rejecting checked exceptions [13]), expensive to throw (improved substantially in .NET 9 but still orders of magnitude above a return), and not composable with LINQ or `async` in a typed way. The ecosystem grew a zoo of `Result` libraries (OneOf, LanguageExt, FluentResults, ErrorOr) with no interoperability.
- **Fixes attempted.** Discriminated unions were championed in `dotnet/csharplang` from 2017; C# 15's union types finally give a closed, exhaustively matched `union Result(Success, Error)` [3]. There is still no propagation operator (`?`), so a `Result` chain is a `switch` per call, and the base library will not change its exception-based APIs. The C# 15 union is also a *type* union (a value is one of several existing types, stored boxed as `object?` in a struct) rather than a tagged union with named cases, a design constrained by the existing record and class model [3].

### The kitchen-sink problem
- **Failure.** Overlapping features accumulate because nothing is removed: delegates, anonymous methods, lambdas and local functions; `class`, `struct`, `record`, `record struct`, `readonly struct`, `ref struct`, primary constructors; properties, auto-properties, `init`, `required`, `field`; LINQ query syntax and method syntax; `var`, target-typed `new`, collection expressions, object and collection initialisers; `dynamic`; three ways to do interface default behaviour. Each is defensible in isolation; together they mean two developers can write the same program in dialects that share little surface.
- **Concretely.** The number of distinct syntaxes for one concept, as of C# 14:

| Concept | Ways to express it |
|---|---|
| A function value | Named method via delegate, anonymous method (`delegate {}`), lambda (`=>`), local function, static lambda, method group conversion |
| A type with a constructor and data | `class`, `struct`, `record`, `record struct`, `readonly record struct`, any of these with a primary constructor, `required` members, `init` setters |
| A property | Full property, auto-property, auto-property with initialiser, expression-bodied, `init`, `required`, `field`-backed (C# 14) |
| A collection literal | `new List<int> { 1, 2 }`, `new() { 1, 2 }`, `[1, 2]` (C# 12), array initialisers, `stackalloc` spans |
| A query | LINQ query syntax, LINQ method syntax, `foreach` |
| Null handling | `if (x != null)`, `x?.`, `??`, `??=`, `is not null`, `is {}`, `!` |
| Interface behaviour | Abstract members, default interface members (C# 8), static abstract members (C# 11), extension members (C# 14) |

None of these has been removed or deprecated; the newest form is recommended by analyzers (`IDE` rules) but all remain valid.
- **Evidence.** The language reference lists over 100 keywords and contextual keywords; "What's new" pages since C# 6 are mostly "syntax you already knew is available in more places" [1]. The management strategy, themes per release, analyzers and `.editorconfig` style rules, warning waves, and the -100-points rule, limits damage but has never removed a feature.
- **Comparison.** Go's refusal to add error syntax (go.md) is the opposite failure mode. Neither language has a mechanism to retire a feature; Rust's editions are the only mainstream answer.

### Events and delegates leak
- **Failure.** A subscriber that never unsubscribes is kept alive by the publisher; long-lived publishers (static events, UI models) are a standard source of memory leaks. WPF's weak-event pattern is the acknowledgement.
- **Evidence.** The problem is documented in Microsoft's own guidance and in every .NET memory-profiling tutorial; it follows from events being strong delegate lists with no ownership model.

### Mutable structs and value-copy traps
- **Failure.** Calling a mutating method on a struct stored in a property or a `readonly` field mutates a copy; `List<T>` indexers return copies; defensive copies silently cost performance. `readonly struct`, `in` parameters and the analyzer warnings mitigate but the default is still wrong.
- **Evidence.** Microsoft's guidelines say to make structs immutable, which is an admission that mutable value types are a trap the language permits.

### Platform churn and the "two .NETs"
- **Failure.** From 2016 to 2020 developers had .NET Framework (Windows, frozen at 4.8), .NET Core, Mono/Xamarin and the `.NET Standard` compatibility matrix; UI stacks churned (Silverlight died, Xamarin.Forms became MAUI, WinUI versions); Visual Studio for Mac was retired in August 2024. Enterprises still run on .NET Framework because WCF and WebForms were never ported.
- **Evidence.** The .NET 5 unification was explicitly marketed as the end of this period; the retained `.NET Framework` support policy is indefinite because migration is incomplete.

### Governance trust episodes
- **Hot reload (October 2021).** Microsoft removed the hot-reload capability from the `dotnet watch` CLI in the .NET 6 release candidate to make it a Visual Studio 2022 feature; after public outcry, Scott Hunter wrote on 23 October 2021 that "in our effort to scope, we inadvertently ended up deleting the source code instead of just not invoking that code path" and that the removal was reversed for the GA SDK [9]. The incident showed that product decisions could override an open-source repository.
- **.NET Foundation (2021).** In the same weeks, a board member resigned publicly and maintainers of a Foundation-hosted project complained that changes had been merged into their repository without consent; the Foundation apologised and revised its processes (widely reported at the time; details not re-verified in this research).
- **C# Dev Kit (2023).** The VS Code C# extension's new "Dev Kit" shipped under the proprietary Visual Studio licence while the underlying language server stayed open, which drew criticism from the OmniSharp community.
- **Structure.** The LDM is Microsoft employees; ECMA standardisation lags years behind; the .NET Foundation does not own C#. Open process, single decision-maker.

### Static analysis gaps and `dynamic`
- `dynamic` (C# 4) silently defers binding to run time and spreads through expressions; it was added for COM and dynamic-language interop and is now mostly avoided.
- Checked arithmetic is off by default (`checked` blocks or a compiler switch); overflow wraps silently as in Java and Go.
- No structured concurrency and no static race detection; `lock` on arbitrary objects (improved by `System.Threading.Lock`, C# 13).

### Problem-to-fix map
Shipped fixes as admissions of the original problem, collected for the proposal's decision log.

| Problem | Fix shipped or in preview |
|---|---|
| Null | C# 8 nullable reference types (warnings); C# 9 `T?` on unconstrained generics; C# 11 `required`; `[NotNullWhen]`-style attributes; base library annotated over .NET 5-6 [8] |
| No generics | C# 2 reified generics (2005) [7] |
| No closed types / unions | C# 15 `union` and `closed` (preview, 2026) [2][3] |
| Exceptions for expected failures | `Try*` convention; no language fix; unions give the data shape only |
| Ceremony | C# 3 `var` and initialisers; C# 6 interpolation, `?.`, expression bodies; C# 9 top-level statements, records; C# 10 file-scoped namespaces, global usings; C# 12 primary constructors, collection expressions; C# 14 `field` [1] |
| Boxing and allocation in hot paths | C# 7.2 `Span<T>`, `ref struct`, `in`; C# 9 function pointers; C# 11 `ref` fields; C# 12 inline arrays; C# 13 `ref struct` in generics; C# 14 implicit span conversions [1][10] |
| `unsafe` attached to the wrong thing | C# 15 memory-safety model: `unsafe` tags memory-accessing operations, `safe` marks externs (preview) [2] |
| Async pitfalls | Analyzers for `async void` and unawaited tasks; `ValueTask`; `IAsyncEnumerable` (C# 8); `Parallel.ForEachAsync`; no change to the colouring model |
| `lock` on arbitrary objects | `System.Threading.Lock` with C# 13 `lock` semantics |
| Reflection blocking AOT | Source generators (C# 9) for JSON, regex, logging, P/Invoke; Native AOT (.NET 7-8) |
| Windows-only runtime | .NET Core (2016), .NET 5 unification (2020), Native AOT (2022) |
| Event leaks | WPF weak-event pattern; no language fix |
| Mutable struct copies | `readonly struct` (7.2), `in` parameters, analyzer warnings; default unchanged |
| Breaking changes from stricter diagnostics | Warning waves (C# 9 onward) gate new warnings behind a version number [1] |

### Three of the traps, concretely
```csharp
// 1. async void: the exception is unobservable and takes down the process.
async void OnClick(object s, EventArgs e) { await File.ReadAllTextAsync("x"); }

// 2. Sync-over-async deadlock under a SynchronizationContext.
string text = ReadAsync().Result;      // UI thread waits for a continuation
                                       // that needs the UI thread

// 3. Nullable annotations are advisory.
string s = null!;                      // compiles with no warning
Console.WriteLine(s.Length);           // NullReferenceException at run time
```

## Designer retrospectives

- **Anders Hejlsberg on checked exceptions (2003) [13].** Rejected them for C# because of versionability ("adding a new exception to a throws clause in a new version breaks client code") and scalability ("people completely circumvent the feature"). C# therefore has exceptions that are invisible in signatures, which is the root of the `Result`-library problem twenty years later; Hejlsberg's position was that the alternative was worse, not that exceptions were ideal.
- **Anders Hejlsberg on null.** He has said in talks and interviews that, starting over, he would make reference types non-nullable by default (wording and venue not re-verified in this research); the C# 8 nullable reference types are Torgersen's implementation of that regret under compatibility constraints [8].
- **Mads Torgersen, "Introducing Nullable Reference Types in C#" (2017) [8].** Frames null as Hoare's "billion-dollar mistake", explains why enforcement is warnings not errors, why `!` exists, and why the design "favors convenience over strictness" where warnings would flood existing code.
- **Stephen Toub, "ConfigureAwait FAQ" (2019) [12].** A designer-level admission that the `async` model needs a multi-page FAQ to be used correctly in libraries.
- **Scott Hunter, hot reload (2021) [9].** "We inadvertently ended up deleting the source code instead of just not invoking that code path"; the reversal and apology are the record.
- **The C# 15 memory-safety proposal [2].** Opens with the admission that `unsafe` has been attached to the wrong thing: pointer types rather than the operations that access unmanaged memory. A redefinition of a 1.0 keyword's meaning after 24 years.
- **Eric Gunnerson, "Minus 100 points" (2004) [6].** The design team's stated bar for features; useful as a retrospective because the language's growth since is the measure of how often the bar was cleared.

## Influence

**Copied from C# (evidence the ideas work):**
- `async`/`await`: JavaScript, Python, Dart, Kotlin, Rust, Swift, Hack, C++20 coroutines (the single most-copied language feature of the 2010s).
- LINQ-style collection pipelines: Java streams, Kotlin, Swift, Dart.
- Properties, extension methods, named/optional arguments: Kotlin, Swift, Dart, TypeScript.
- Attributes (2002): Java annotations (2004).
- `var` with local inference (2007): Java 10 (2018), C++11 `auto` arose in parallel.
- Reified generics for a managed runtime: the target Valhalla is still aiming at.
- Compiler-as-a-service (Roslyn): TypeScript's language service, rust-analyzer's architecture, Kotlin's compiler plugin model.
- Nullable reference types as a flow-typed warning retrofit: the model Java's nullness JEP draft and Python's type checkers follow.
- Source generators as the AOT-friendly replacement for reflection: adopted by Kotlin (KSP) and Dart (build_runner) in spirit.

**Copied by C# from others:** Java (syntax, class model, GC, interfaces, `synchronized`-like `lock`); Delphi and Visual Basic (properties, events, components, `with`-less RAD design); C++ (operator overloading, structs, `unsafe`, `stackalloc`); F# (async workflows, pattern matching, records, discriminated unions, generic math ideas); Haskell via Erik Meijer (LINQ comprehensions as monads); Java 8 (default interface members, copied in C# 8); Kotlin and Swift (nullable types, extension members, primary constructors, string templates); TypeScript (flow-typed null checking); Rust (ref-struct lifetime rules are a restricted borrow checker; the C# 15 `unsafe` redesign cites memory-safety auditing).

## Lessons for Sisu

- **Adopt**
  - *Reified or monomorphised generics with first-class value types.* The 2001 CLR design is the proof that a managed language can have unboxed `List<int>` and runtime type information without an erasure debt.
  - *Stack-only, non-escaping view types* (`Span<T>`, `ref struct`): a small decidable lifetime discipline that yields allocation-free hot paths while staying safe. This is the most transferable piece of C#'s performance story.
  - *Compiler as a library from day one.* Roslyn made analyzers, generators, refactorings and gradual rule-tightening possible; Sisu's compiler should expose syntax trees and a semantic model as a stable API.
  - *A single package manager, registry, CLI and project format*, shipped with 1.0 (add a transparency log, which NuGet lacks).
  - *Public design-meeting notes and proposal repository.*
  - *Switch expressions, property/positional/list patterns, records with `with`.* Converged on by Java and Kotlin as well; the design space is settled.
  - *String interpolation with compile-time handlers* (C# 10) and raw string literals.
  - *The "unsafe tags operations, not types" principle* from the C# 15 memory-safety redesign.
  - *Warning waves*: a versioned way to introduce stricter diagnostics without breaking builds.

- **Adapt**
  - *Asynchrony.* If Sisu has coloured async at all, remove the pathologies at the root: no `async void`, no blocking accessors on futures, no ambient synchronisation context, structured task scopes. The better answer, given Java's and Go's evidence, is runtime-scheduled threads with no colouring, with C#'s state-machine compilation kept only where zero-allocation async is needed (embedded, WebAssembly).
  - *Nullability.* Take the flow-typed analysis but make it an error, not a warning, and make it part of the runtime contract so deserialisers cannot bypass it. C# shows what the warning-level compromise costs.
  - *Unions.* Take C# 15's exhaustiveness and "result-or-error" motivation, but implement tagged unions with named cases and payloads, unboxed, plus a propagation operator. C#'s type-union shape is a compatibility compromise, not a recommendation.
  - *Properties and events.* Keep properties; replace strong-reference event lists with an ownership-aware subscription (scoped or weak by default).
  - *Evolution speed.* Keep the cadence but add a retirement mechanism (editions with automatic migration) so superseded syntax can be removed; C# proves that "add only" produces dialects.
  - *Structs.* Immutable by default, mutation opt-in; the mutable-struct copy trap should be a compile error.

- **Avoid**
  - *Exceptions as the only error channel* and `Try*` conventions as the workaround. The `Result`-library zoo is the measured cost.
  - *A `dynamic`-style escape hatch that spreads through expressions.*
  - *Silent overflow with opt-in `checked`.*
  - *Shipping a feature in several overlapping syntaxes* (query versus method LINQ; five ways to declare a type with a constructor).
  - *Platform splits* (two runtimes, a compatibility "standard", UI framework churn): a new language should have one runtime story before it has a second.
  - *A single-vendor decision process without an institutional check*: the 2021 and 2023 episodes were reversible only because of public pressure.

- **Open questions raised**
  - *Fast evolution versus coherence.* C# evolves faster than any comparable language and is coherent within each era; the cross-era cost is real. Is an editions mechanism enough, or does a language need a deliberate feature budget (Gunnerson's -100 points enforced with a cap)?
  - *Warnings versus errors for retrofits.* C# chose warnings to get adoption; Dart chose a hard migration with tooling and succeeded on a smaller ecosystem. For a new language the question becomes: which rules should be errors from 1.0 so that there is never a retrofit?
  - *Async state machines versus green threads* in a language that wants both C#-level per-operation efficiency and Go-level simplicity. Zero-allocation async is a genuine advantage for embedded and WebAssembly targets; colourless threads are a genuine advantage for everyone else.
  - *Type unions versus tagged unions.* C# chose type unions because records already exist; the trade-off (no named cases, boxing, `null` as a case) is instructive for any language adding unions late.
  - *How much low-level control belongs in a managed language.* `Span<T>`, `ref struct`, `stackalloc`, function pointers and `unsafe` give C# a systems-programming subset; Sisu must decide whether to expose this layer from the start (C#'s evidence says yes, if the safe subset stays the default).

## Sources

1. Microsoft Learn, "The history of C#" (version-by-version feature list, C# 1.0 to 14), updated Nov 2025. https://learn.microsoft.com/en-us/dotnet/csharp/whats-new/csharp-version-history
2. Microsoft Learn, "What's new in C# 15" (collection expression arguments, union types, closed hierarchies, extension indexers, labelled break/continue, memory safety), Sept 2026. https://learn.microsoft.com/en-us/dotnet/csharp/whats-new/csharp-15
3. Microsoft Learn, "Union types - C# reference", Aug 2026. https://learn.microsoft.com/en-us/dotnet/csharp/language-reference/builtin-types/union ; feature specification https://github.com/dotnet/csharplang/blob/main/proposals/csharp-15.0/unions.md
4. Stack Overflow Developer Survey 2025, Technology section. https://survey.stackoverflow.co/2025/technology/ (2025)
5. ECMA-334, C# Language Specification, Ecma International (1st ed. 2001; 7th ed. 2023). https://ecma-international.org/publications-and-standards/standards/ecma-334/
6. Eric Gunnerson, "Minus 100 points", Microsoft blogs, 2004. https://learn.microsoft.com/en-us/archive/blogs/ericgu/minus-100-points
7. Andrew Kennedy and Don Syme, "Design and Implementation of Generics for the .NET Common Language Runtime", PLDI 2001. https://www.microsoft.com/en-us/research/publication/design-and-implementation-of-generics-for-the-net-common-language-runtime/
8. Mads Torgersen, "Introducing Nullable Reference Types in C#", .NET Blog, 15 Nov 2017. https://devblogs.microsoft.com/dotnet/nullable-reference-types-in-csharp/
9. Scott Hunter, "Update on .NET Hot Reload progress and Visual Studio 2022 Highlights" / ".NET Hot Reload support via CLI", .NET Blog, 23 Oct 2021. https://devblogs.microsoft.com/dotnet/net-hot-reload-support-via-cli/
10. Microsoft Learn, "What's new in C# 14" (extension members, `field`), Nov 2025. https://learn.microsoft.com/en-us/dotnet/csharp/whats-new/csharp-14
11. Bob Nystrom, "What Color is Your Function?", 1 Feb 2015. https://journal.stuffwithstuff.com/2015/02/01/what-color-is-your-function/
12. Stephen Toub, "ConfigureAwait FAQ", .NET Blog, Dec 2019. https://devblogs.microsoft.com/dotnet/configureawait-faq/ ; David Fowler, "ASP.NET Core Diagnostic Scenarios: Async Guidance". https://github.com/davidfowl/AspNetCoreDiagnosticScenarios/blob/master/AsyncGuidance.md
13. Bill Venners with Bruce Eckel, "The Trouble with Checked Exceptions: A Conversation with Anders Hejlsberg, Part II", Artima, 18 Aug 2003. https://www.artima.com/articles/the-trouble-with-checked-exceptions
14. Stephen Toub, "All About Span: Exploring a New .NET Mainstay", MSDN Magazine, Jan 2018. https://learn.microsoft.com/en-us/archive/msdn-magazine/2018/january/csharp-all-about-span-exploring-a-new-net-mainstay
15. JEP 444: Virtual Threads (JDK 21), for the contrasting design argument. https://openjdk.org/jeps/444
16. Microsoft, "Introducing C# 14", .NET Blog, 17 Nov 2025. https://devblogs.microsoft.com/dotnet/introducing-csharp-14/ ; "Explore C# 15", .NET Blog, 24 Aug 2026. https://devblogs.microsoft.com/dotnet/explore-csharp-15
17. dotnet/csharplang, Language Design Meeting notes and proposals (public since 2017). https://github.com/dotnet/csharplang
18. .NET Foundation. https://dotnetfoundation.org/ (founded 2014)
