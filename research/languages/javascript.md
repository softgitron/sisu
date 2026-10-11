# JavaScript

## Snapshot
| | |
|---|---|
| First appeared | 1995 (Netscape Navigator 2.0 beta as LiveScript, Sept 1995; renamed JavaScript, Dec 1995) |
| Designed by | Brendan Eich (Netscape); standardised by Ecma TC39 since 1997 (ECMA-262) |
| Current version (Oct 2026) | ECMAScript 2026, 17th edition, approved by the Ecma General Assembly on 30 June 2026 |
| Governance | Ecma TC39 (member companies: browser vendors, Microsoft, Google, Apple, Mozilla, Bloomberg, Igalia, etc.); staged proposal process; consensus of attending delegates |
| Typing | Dynamic, weak (implicit coercion); no static types (TypeScript layers them on) |
| Memory | Tracing garbage collection (generational, incremental, concurrent in V8/SpiderMonkey/JavaScriptCore); WeakRef/FinalizationRegistry since ES2021 |
| Compilation | JIT in all major engines (V8 Ignition/Sparkplug/Maglev/TurboFan tiers); AOT not standard; WebAssembly as a sibling target |
| Primary domains | Browser front ends (the only language natively in every browser), servers (Node, Deno, Bun), CLI tools, desktop (Electron), mobile (React Native) |
| Notable users | Every website; Google, Meta, Microsoft, Netflix, PayPal, Shopify; Anthropic ships Claude Code as a Bun executable |

Usage: JavaScript is the most used language in every Stack Overflow survey
since the question was asked; 2025: 66% of all respondents and 68.8% of
professionals [1]. GitHub Octoverse 2025: about 2.15M monthly contributors,
third behind TypeScript and Python [2]. npm hosts "over 3.1 million packages"
(figure from 2024) [3]. The State of JS 2025 survey (November 2025, roughly
12,000-13,000 respondents) found 40% now write exclusively TypeScript, 6% write
exclusively plain JavaScript [4].

## Stated design goals

Eich was hired by Netscape to "put Scheme in the browser"; management required
Java-like syntax, and by his own account the first prototype ("Mocha") was
written in ten days in May 1995 [5]. The brief was a scripting language for
non-professional web authors, glueing Java applets and HTML forms, with
Scheme's first-class functions and Self's prototype-based objects under
C/Java syntax. There was no design document of goals in the modern sense; the
goals were "ship in Navigator 2.0" and "look like Java".

TC39's governing principle since the ES4 collapse (2008) is backward
compatibility: new editions must not change the behaviour of existing web
content ("don't break the web"). The process document's stage 4 criteria
require "two compatible implementations which pass the Test262 acceptance
tests" and "significant in-the-field experience with shipping
implementations" [6]; web compatibility is a formal entrance criterion, not a
preference.

## Strengths

### The event loop with promises and `async`/`await`: a working I/O concurrency model

A single-threaded run-to-completion event loop (no data races in user code),
promises (ES2015), `async`/`await` (ES2017), `for await` and async iterators
(ES2018), and top-level `await` in modules (ES2022). Because every JavaScript
API was asynchronous from the start (XMLHttpRequest, timers, DOM events) and
Node adopted non-blocking I/O as its founding premise (2009), there was never
a large synchronous ecosystem to collide with. The colouring problem (Nystrom's
"What Color is Your Function?" is literally about JavaScript [7]) exists, but
in practice the colour is nearly universal in library code, `await` composes
through ordinary expressions, and the cost is low enough that the model was
copied by C# (which originated `async`/`await`), Python, Rust, Swift, Kotlin
and Dart. State of JS 2025 does not list async as a pain point; it lists lack
of static typing first and date handling second [4]. Evidence that it is
"colourless enough": the biggest remaining complaint is blocking the loop with
CPU work, answered by Web Workers and `worker_threads`, not by rewriting APIs.

### "Never break the web" as an extreme compatibility promise

Code from 1997 still runs in a 2026 browser. The mechanism is TC39's rule
that web-visible behaviour cannot change, enforced by shipping implementations
reporting breakage. Two concrete cases show it in action: `Array.prototype.contains`
was renamed `includes` (2015) and `Array.prototype.flatten` was renamed `flat`
(2018, "SmooshGate") because MooTools, deployed on millions of pages, defined
incompatible methods of the same names [8]. The benefit is total: no 2-to-3
migration, no abandoned codebases, the largest installed base of any language.
The cost is listed under weaknesses: every mistake is permanent, and fixes can
only be additive (`===` beside `==`, `let` beside `var`, `class` beside
prototypes, strict mode beside sloppy mode).

### TC39's staged process as a governance model

Stage 0 (strawperson), 1 (problem statement, champion), 2 (draft spec text),
2.7 (design complete, "approved in principle, under validation", awaiting
Test262 tests), 3 (candidate, implementations ship behind flags or unflagged),
4 (two shipping implementations, in-field experience, spec PR merged) [6].
Stage 2.7 was inserted (2023-2024) because proposals were reaching stage 3
without tests and then changing. Editions are yearly snapshots: ES2026 adds
`Array.fromAsync`, `Error.isError`, `Math.sumPrecise`, `Uint8Array` base64/hex
methods, iterator sequencing, `RegExp.escape` and `JSON.parse` source-text
access [9]. Temporal reached stage 4 in 2026 with expected publication in
ES2027 after roughly nine years of work [9], an example of the process
refusing to ship a large API until it had shipped in engines. The model's
strengths: every proposal has a public repository, a champion, meeting notes
and a test suite before standardisation; no single vendor can push a feature
through without a second implementation. Its weakness: speed. Decorators have
been at stage 3 since 2022; the pipeline operator has been at stage 2 since
2021; Temporal took nine years.

### JSON as a data-interchange win

Douglas Crockford extracted JSON from JavaScript object literals (2001;
RFC 4627 in 2006, ECMA-404 in 2013, RFC 8259 in 2017). It displaced XML for
APIs because it maps directly to the host language's literals, has six types
and a grammar that fits on a business card, and because `JSON.parse` was the
fastest deserialiser in the browser. Every language now ships a JSON module,
and ES2026's `JSON.parse` source-text access and the ES2025 JSON modules
(`import data from "./x.json" with { type: "json" }`) continue to invest in
it. The lesson: a language's literal syntax for records and arrays becomes
its wire format whether or not the designer intends it, so make the literal
syntax unambiguous and machine-friendly.

### The browser as a platform, plus WebAssembly

JavaScript's monopoly on the browser gave it a deployment story no other
language has: zero install, sandboxed, cross-platform, with a permissions model
(same-origin policy, CSP) that Deno later copied for the server. WebAssembly
(1.0 in 2017; Wasm 3.0 published 17 September 2025 with garbage collection,
typed references, exception handling, tail calls, 64-bit memories and multiple
memories, "already shipping in most major web browsers") [10] turned the
browser into a target for other languages and simultaneously relieved
JavaScript of the need to be a systems language. Wasm GC in particular lets
managed languages (Kotlin, Dart, Java, OCaml, Scheme) compile without shipping
their own collector, which matters for Sisu's choice of targets.

### The largest package ecosystem and the fastest-moving tooling

npm (2010) made publishing trivial (`npm publish`, no review) and dependency
resolution nested (each package gets its own versions of its dependencies),
which eliminated diamond-dependency conflicts at the cost of enormous
`node_modules` trees. The result is more than 3.1 million packages [3] and a
culture of tiny modules. On tooling, the ecosystem has converged after a
decade of churn: State of JS 2025 shows Webpack at 87% usage with 26%
satisfaction and Vite at 84% usage with 98% satisfaction [4]; Vite (2020),
esbuild (Go, 2020), SWC and Oxc/Rolldown (Rust) replaced Babel/Webpack
pipelines, and the runtimes Deno 2 (October 2024) and Bun (1.0 in 2023,
acquired by Anthropic on 2 December 2025, 1.4 in August 2026) bundle package
manager, test runner, bundler and TypeScript execution into one binary [11]
[12]. The lesson is the same as Python's `uv`: integrated, fast, single-binary
tooling wins adoption, regardless of which language it is written in.

### Prototypes, closures and first-class functions

The Scheme and Self heritage (closures, higher-order functions, objects as
dictionaries, delegation via prototype chains) is what made JavaScript
survive its syntax. Every major framework (jQuery, React, Vue) is built on
closures and object literals rather than classes, and React's hooks are a
pure closure-based design. ES2015 `class` is sugar over prototypes, ES2022
added private fields (`#x`), and the language still lets any object be
extended at run time, which is why monkey patching ("polyfills") works and why
"don't break the web" is so hard (MooTools above).

## Weaknesses

### The ten-day legacy: coercion, `==`, `this`, `var`, ASI

The concrete items, each with its attempted fix:
- Implicit coercion and `==`: `[] == ![]` is true, `"1" == 1`, `null == undefined`
  but `null == 0` is false; `typeof null === "object"`. Fix: `===` (ES3) and
  linters; `==` cannot be removed.
- `this` depends on the call site (method call, plain call, `new`, `call`/
  `apply`, event handlers). Fixes: `Function.prototype.bind` (ES5), arrow
  functions with lexical `this` (ES2015), class fields.
- `var` is function-scoped and hoisted; closures in loops captured the same
  variable. Fix: `let`/`const` with block scope and the temporal dead zone
  (ES2015), which made the loop-closure bug disappear in new code.
- Automatic semicolon insertion: `return` followed by a newline returns
  `undefined`; a line starting with `(` or `[` continues the previous one.
  Fix: none possible; formatters (Prettier) and style rules.
- Sloppy-mode silent errors: assignment to undeclared variables creates
  globals; `arguments` aliasing; `with`. Fix: `"use strict"` (ES5, 2009),
  on by default inside modules and classes (ES2015).
- One number type (IEEE 754 double) until `BigInt` (ES2020); `0.1 + 0.2`.
- `Date` copied from Java 1.0's `java.util.Date`, including month indexing
  from 0 and mutability; the fix (Temporal) took from 2017 to stage 4 in 2026 [9].
  State of JS 2025 lists date handling among the top pain points [4].

Every one of these was fixed by adding a new construct beside the old one.
The lesson for a new language is that the first release's defaults are
permanent; JavaScript spent 1997-2015 adding "the right default" as an
opt-in. ES2015 (classes, modules, `let`/`const`, arrow functions, promises,
template literals, destructuring, iterators, generators) is the proof that a
language can be repaired by addition if the additions are good enough that
new code stops using the old forms.

### The cost of "never break the web"

Deprecated features cannot be removed, so the language grows monotonically;
the specification is over 800 pages. Every engine carries sloppy mode, `with`,
`arguments`, `__proto__`, HTML comment syntax, Annex B "web legacy" semantics
and `document.all` (an object that is falsy and `typeof` "undefined", specified
to keep old browser-sniffing code working). Learners must be taught which half
of the language to ignore. Feature naming is constrained by what was ever
deployed (SmooshGate). The practical mitigation is social: linters, `strict`,
TypeScript and formatters define the subset people actually write.

### npm as the canonical supply-chain-attack case

Concrete incidents, in order:
- left-pad (22 March 2016): a maintainer unpublished an 11-line package after
  a naming dispute; Babel, Webpack and thousands of builds broke within hours.
  npm restored it and changed the unpublish policy [3].
- event-stream (November 2018): a maintainer handed the package to a stranger,
  who added `flatmap-stream` with an encrypted payload targeting the Copay
  bitcoin wallet; it shipped to roughly 2 million weekly downloads [3].
- colors / faker (January 2022): the maintainer deliberately sabotaged his own
  packages (infinite loop, wiped repository) [3].
- chalk / debug / ansi-styles (8 September 2025): the maintainer "qix" was
  phished for 2FA credentials; 18 packages with over 2 billion combined weekly
  downloads shipped a browser crypto-address swapper for about two hours [13].
- Shai-Hulud (14-16 September 2025): a self-propagating worm starting from
  `@ctrl/tinycolor` injected post-install scripts that stole npm and GitHub
  tokens and used them to republish further packages; 187+ packages within two
  days, over 500 removed by GitHub [13][14].
- Shai-Hulud 2.0 (24 November 2025): Zapier, ENS, AsyncAPI, PostHog and
  Postman packages; over 25,000 GitHub repositories exposed secrets [13].
- November 2025: AWS researchers found over 150,000 packages created for a
  "Tea" token-farming scheme [3].
- 2026: TanStack and Mistral SDK packages (11 May, 170+ npm and 2 PyPI
  packages); "Mini Shai-Hulud" on 323 `@antv` packages/639 versions (May); a
  `binding.gyp` worm publishing 286 malicious versions of 57 packages in about
  two hours (3 June); 144 Mastra packages via maintainer phishing (17 June);
  Jscrambler (11 July) and AsyncAPI release pipelines (14 July); and in early
  August the Keyv/flat-cache/file-entry-cache compromise, 434 packages and
  1,381 versions, published through GitHub Actions so they carried
  valid-looking provenance attestations [15]. Counts differ between reports;
  these are the figures given by the cited aggregators.

The recurring mechanisms are (1) install-time scripts (`postinstall`) that run
arbitrary code on every developer machine and CI runner, (2) long-lived
publish tokens stolen from those machines, (3) the transitive-dependency
explosion that makes one compromised package reach millions of installs, and
(4) worm behaviour, which exists only because install scripts plus stolen
tokens close the loop. Responses: GitHub's 22 September 2025 plan deprecates
classic tokens, limits granular publishing tokens to seven days, requires 2FA
(moving from TOTP to FIDO) for local publishing, and pushes trusted publishing
via OIDC [14]; lockfiles (`package-lock.json` since npm 5, 2017) and
`npm ci`; provenance attestations (2023); pnpm 10 (2025) stopped running
dependency lifecycle scripts by default (recalled from release notes, not
re-verified here); Deno's permission model denies file, network and
environment access unless granted on the command line [11]. The August 2026
incident shows that provenance alone is defeated when the CI pipeline itself
is compromised. The design lessons: no code execution at install time;
capabilities for packages (a dependency that needs no network should not get
it); short-lived, scoped publishing credentials by default; and a standard
library large enough that `left-pad`-sized dependencies are not needed.

### Module system fragmentation

CommonJS (`require`, Node 2009) and ES modules (ES2015, in browsers 2017, in
Node unflagged 2019-2020) coexisted for a decade with different resolution,
different `this`, synchronous versus asynchronous loading, and the
"dual-package hazard" where one package loads twice. `package.json` `exports`
maps, `.cjs`/`.mjs` extensions, and `"type": "module"` were the patches; Node
22's unflagged `require(esm)` (late 2024, from memory) finally lets CommonJS
load ESM synchronously. The ecosystem is still only mostly converted. The
lesson: ship one module system before the ecosystem forms, and specify
loading semantics (sync or async, resolution algorithm) in the language, not
per runtime.

### No integers, no standard library

Until `BigInt` every number was a double, so 64-bit IDs from APIs silently lost
precision above 2^53, and bit operations truncate to 32 bits. The standard
library shipped with the browser is small (no file system, no module for
dates that works, no `Result`), which is the root cause of the micro-package
culture (`is-odd`, `left-pad`). Node, Deno and Bun each add their own
standard library (`node:fs`, Deno `std`, Bun APIs), creating three partially
compatible server platforms, with WinterCG/WinterTC trying to standardise the
common subset.

### Performance depends on hidden heuristics

V8 and its peers make JavaScript fast through inline caches and hidden
classes, so performance depends on monomorphism, shape stability and avoiding
deoptimisation triggers (`arguments`, `delete`, mixing integer and double
arrays). None of this is visible in the language; a one-line change can cause
a 10x regression. asm.js and then WebAssembly exist because predictable
performance could not be obtained from JavaScript semantics. TypeScript's
compiler is the most prominent recent example: the port to Go gave 8-12x on
full builds (VS Code's type check from 125.7 s to 10.6 s) [16], with the stated
reasons being native code and shared-memory multithreading, which JavaScript
cannot express (workers share nothing but `SharedArrayBuffer`). The same
pattern appears in esbuild (Go), SWC/Oxc/Rolldown (Rust) and Bun (Zig, with a
July 2026 post titled "Rewriting Bun in Rust" that this research could not
read). A language whose own flagship tools must be rewritten in another
language to be fast enough is evidence against using it for compilers.

### Tooling fragmentation before consolidation

Between 2014 and 2022 a front-end project needed Babel (transpiler), Webpack
or Rollup (bundler), ESLint, Prettier, Jest, ts-node, and a dozen plugins each
with its own configuration file; "JavaScript fatigue" was a recognised term.
Consolidation came from Vite (dev server plus Rollup/Rolldown), Biome
(linter+formatter in Rust), Vitest, and the all-in-one runtimes. The lesson is
that the language shipped no tooling at all, so the community built it four
times; a language that ships formatter, test runner, package manager and
bundler in one tool avoids this entirely (Go, Rust, Deno and Bun all did).

## Designer retrospectives

- Brendan Eich has said repeatedly (e.g. his 2008 "Popularity" post and later
  talks) that the ten-day schedule forced mistakes he regrets: implicit
  coercion with `==`, `typeof null`, `this` binding rules, `with`, and that he
  wished he had made the language more Scheme-like; he also argues the Scheme
  and Self parts (closures, prototypes) are why it survived [5].
- Ryan Dahl, Node's creator, listed "10 Things I Regret About Node.js" (JSConf
  EU, 2018): not sticking with promises, no security sandbox, the build system
  (GYP), `package.json` as a centralised registry dependency, `node_modules`
  resolution, `require` without extensions, `index.js`. Deno (2018, 1.0 in
  2020, 2.0 in October 2024) is the retrospective turned into a product:
  permissions by default, URL/JSR imports, TypeScript built in, one binary
  with formatter, linter and test runner [11].
- TC39's addition of stage 2.7 (requiring complete spec text and tests before
  stage 3) is an admission that the four-stage process let under-specified
  proposals advance [6].
- Douglas Crockford's "JavaScript: The Good Parts" (2008) is a de facto
  retrospective by a language-committee member: it recommends a subset and
  calls the rest "bad parts" and "awful parts"; JSLint/ESLint institutionalise
  it.

## Influence

Copied by others: `async`/`await` style event-loop programming (Python asyncio,
Dart, Rust futures run on executors modelled on libuv); promises (Java
`CompletableFuture`, Swift, Kotlin); arrow functions' lexical capture (Java
lambdas, C#); JSON everywhere; the TC39 staged process (Python's PEP and Rust's
RFC process are older, but Swift Evolution and the WebAssembly CG process copy
TC39's stage structure); npm's lockfile and `package.json` shape (Cargo,
Bundler, Go modules all have lockfiles; Cargo is credited to Bundler, which
predates npm); template literals (Python 3.14 t-strings copy tagged templates
[17]); optional chaining `?.` and nullish coalescing `??` (ES2020, copied from
and to C#, Kotlin, Swift, PHP 8).

Copied from others: Scheme (closures, first-class functions), Self (prototypes),
Java (syntax, `Date`, the name), Perl (regex literals), Python (generators,
iterators, `for...of`), C# (`async`/`await`, LINQ-style array methods),
CoffeeScript (arrow functions, classes, destructuring, template strings were
all prototyped there before ES2015), Rust/Haskell (iterator helpers,
`Result`-like `Promise.allSettled`), Java's `java.util.Date` (regretted).

## Lessons for Sisu

- **Adopt**: a single-threaded-per-isolate event loop with `async`/`await` as
  the standard I/O model, with the entire standard library async-first so no
  second ecosystem forms; a staged, test-first proposal process where nothing
  is standardised before two independent implementations and a conformance
  suite exist; record/array literal syntax that is also the serialisation
  format (JSON-compatible by construction); integrated single-binary tooling
  (package manager, formatter, linter, test runner, bundler) from release 1.0;
  WebAssembly as a first-class compile target; capability-style permissions
  for packages and programs (Deno's model, extended to dependencies).
- **Adapt**: "never break the web" as a stable-channel promise with an
  edition mechanism (Rust editions) so defaults can change per edition without
  breaking old code; strict mode semantics as the only mode; `===` semantics as
  the only equality; lexical `this`/receiver only; block scoping only; a native
  integer type beside floats; a `Temporal`-style date/time API in the standard
  library from day one rather than a `Date` copied from Java 1.0.
- **Avoid**: implicit coercion between unrelated types; call-site-dependent
  receiver binding; automatic semicolon insertion or any grammar where a
  newline silently changes meaning; shipping without a standard library
  (the micro-package culture and its attack surface followed directly); code
  execution at package install time; long-lived publishing tokens;
  nested-resolution dependency trees without a lockfile; two module systems;
  leaving performance to hidden JIT heuristics with no way to express shapes
  or integers in the language; a specification that can only grow (plan the
  removal mechanism before the first release).
- **Open questions raised**: whether a language can keep JavaScript's
  "anyone can publish" ecosystem growth while rejecting install-time code and
  granting capabilities per dependency (Deno has the model but far smaller
  adoption); whether the yearly-edition cadence and stage process are too
  slow for a young language (nine years for Temporal) or exactly right once
  the installed base is large; whether an event-loop model is sufficient for
  CPU parallelism or whether Sisu needs structured threads from day one (the
  TypeScript-to-Go port says single-threaded JavaScript could not make its
  own compiler fast); whether dynamic prototype extension is worth keeping for
  polyfills and frameworks given that it makes compatibility and performance
  harder.

## Sources

1. Stack Overflow Developer Survey 2025, Technology. https://survey.stackoverflow.co/2025/technology (2025)
2. GitHub Octoverse 2025. https://github.blog/news-insights/octoverse/octoverse-a-new-developer-joins-github-every-second-as-ai-leads-typescript-to-1/ (Oct 2025)
3. Wikipedia, "npm" (registry size, left-pad, event-stream, colors/faker, 2025-2026 incidents, with their primary citations). https://en.wikipedia.org/wiki/Npm (retrieved Oct 2026)
4. State of JavaScript 2025 (survey Nov 2025, results Feb-Mar 2026), as reported by InfoQ and DevClass. https://www.infoq.com/news/2026/03/state-of-js-survey-2025 ; https://www.devclass.com/development/2026/02/10/javascript-survey-reveals-gripes-against-date-handling-webpack-and-nextjs-and-that-typescript-has-won/4090262 (2026); official results at https://2025.stateofjs.com/
5. Brendan Eich, "Popularity" (origin of JavaScript, the ten days). https://brendaneich.com/2008/04/popularity/ (2008); Wikipedia, "JavaScript", history section. https://en.wikipedia.org/wiki/JavaScript
6. TC39 Process Document (stages 0-4 including 2.7). https://tc39.es/process-document/ (current)
7. Bob Nystrom, "What Color is Your Function?". https://journal.stuffwithstuff.com/2015/02/01/what-color-is-your-function/ (2015)
8. Mathias Bynens, "#SmooshGate FAQ". https://developer.chrome.com/blog/smooshgate (2018); TC39 proposal-flatMap README
9. TC39 finished proposals table (ES2025, ES2026, Temporal for 2027). https://github.com/tc39/proposals/blob/main/finished-proposals.md (retrieved Oct 2026); InfoWorld, "ECMAScript 2026 specification approved" (June 2026)
10. WebAssembly.org, "Wasm 3.0 Completed". https://webassembly.org/news/2025-09-17-wasm-3.0/ (17 Sep 2025)
11. Deno 2 announcement. https://deno.com/blog/v2.0 (9 Oct 2024); Ryan Dahl, "10 Things I Regret About Node.js", JSConf EU (2018)
12. Anthropic, "Anthropic acquires Bun as Claude Code reaches $1B milestone". https://anthropic.com/news/anthropic-acquires-bun-as-claude-code-reaches-usd1b-milestone (2 Dec 2025); Bun blog index (Bun 1.4, Aug 2026; "Rewriting Bun in Rust", July 2026). https://bun.com/blog
13. FortiGuard, "npm (Shai-Hulud) Supply Chain Attack" threat signal; AWS Security Blog, "Defending against supply chain attacks like chalk/debug and the Shai-Hulud worm". https://www.fortiguard.com/threat-signal-report/6201/npm-supply-chain-attack ; https://aws.amazon.com/blogs/security/defending-against-supply-chain-attacks-like-chalk-debug-and-the-shai-hulud-worm (2025)
14. GitHub, "Our plan for a more secure npm supply chain". https://github.blog/security/supply-chain-security/our-plan-for-a-more-secure-npm-supply-chain/ (22 Sep 2025)
15. 2026 incident reports: Gigazine on the Keyv/flat-cache compromise citing Aikido Security (Aug 2026) https://www.gigazine.net/gsc_news/en/20260805-npm-supply-chain-attack ; Unit 42, "Monitoring npm supply chain attacks" (AsyncAPI, July 2026) https://unit42.paloaltonetworks.com/monitoring-npm-supply-chain-attacks ; Expert Insights on the @antv compromise (May 2026); RH-ISAC on Mastra (June 2026); Rescana on Jscrambler (July 2026)
16. Microsoft, "Announcing TypeScript 7.0" (Go port benchmarks). https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/ (8 July 2026)
17. PEP 750, Template Strings. https://peps.python.org/pep-0750/ (2025)
18. Douglas Crockford, "JavaScript: The Good Parts", O'Reilly (2008); ECMA-404 The JSON Data Interchange Syntax (2013); RFC 8259 (2017)
