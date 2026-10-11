# Developer surveys: what practitioners say hurts and what they admire

Compiled October 2026 for the Sisu language research. Every number was re-checked against the primary publication
unless marked `[secondary]` (taken from press coverage) or `[unverified]` (could not be re-checked). Where a publisher
renders a chart only as an image, the figure is quoted from the publisher's own prose or marked.

Two caveats apply to every survey here. **Self-selection**: almost all are opt-in surveys promoted through the
language's own channels, so respondents are more engaged and more favourable than the user population; the Go team is
the only one that adds a randomly sampled arm (prompts inside VS Code and GoLand), and that arm is slightly *less*
satisfied than the self-selected arm (92 % vs 93 % in 2024 H1). **Wording changes**: Stack Overflow renamed
"loved/wanted" to "admired/desired" in 2023, the Rust survey shortened its fielding window in 2024, Julia merged two
answer options in 2023. Year-on-year comparisons are only valid where the publisher itself makes them.

---

## 1. Stack Overflow Developer Survey (2023-2026)

**Metric definitions.** Since 2023 the survey reports *admired* (share of respondents who used a technology in the
past year *and* want to keep using it) and *desired* (share of all respondents who want to work with it next year),
both derived from "Which ... have you done extensive development work in over the past year, and which do you want to
work in over the next year?" Admired is therefore a retention rate among current users, not a popularity measure, and
it is noisy for small languages.

| Edition | Fielding | Respondents | Base for language question |
|---|---|---|---|
| 2023 | May 2023 | "over 90,000" | 87,585 |
| 2024 | May 2024 | ~65,000 `[secondary]` | 60,171 |
| 2025 | mid-2025 | "49,000+" | 31,771 (64.8 % of respondents) |
| 2026 | 2026 | 30,903 from 169 countries | ~14,100 (8,742 = 62.0 % for JavaScript) |

Response counts collapsed from 90k to 31k in three years. The 2026 report also restructured its sections ("Build vs
AI", "Frontier models") and no language-level admired/desired table could be located in its text, so the Rust streak
below ends at 2025.

### Most used ("have done extensive development work in, past year")

| Language | 2023 | 2024 | 2025 | 2026 |
|---|---|---|---|---|
| JavaScript | 63.6 % | 62.3 % | 66.0 % | 62.0 % |
| HTML/CSS | 53.0 % | 52.9 % | 61.9 % | 58.2 % |
| Python | 49.3 % | 51.0 % | 57.9 % | 58.0 % |
| SQL | 48.7 % | 51.0 % | 58.6 % | 58.4 % |
| TypeScript | 38.9 % | 38.5 % | 43.6 % | 43.8 % |
| Bash/Shell | 32.4 % | 33.9 % | 48.7 % | 51.3 % |
| Java | 30.6 % | 30.3 % | 29.4 % | 27.6 % |
| C# | 27.6 % | 27.1 % | 27.8 % | 27.1 % |
| C++ | 22.4 % | 23.0 % | 23.5 % | 24.3 % |
| C | 19.3 % | 20.3 % | - | - |
| Go | - | - | 16.4 % | - |
| Rust | 13.1 % `[secondary]` | 12.6 % | 14.8 % | - |

The 2025 jump in Python/SQL/Bash coincides with the smaller, more professional base; Stack Overflow's 2025 write-up
says Python rose 7 points and Rust and Go 2 points.

### Most admired

- **2023.** The report says Rust is the most admired language and "more than 80 %" of its users want to use it again;
  the chart value widely reported is 84.66 % `[unverified]`. MATLAB was least admired (<20 %).
- **2024.** Rust 83 % (Stack Overflow blog; Rust had topped the predecessor "most loved" question every year since
  2016). Python overtook JavaScript as the most *desired* language.
- **2025.** Rust 72 %, Gleam 70 %, Elixir 66 %, Zig 64 % (survey page). Among tools, Cargo 70.8 % and uv 74.2 %
  `[secondary, Weekly Rust]`. Press coverage calls this Rust's tenth consecutive year on top.
- The 11-point fall from 83 % to 72 % happened in the year the sample fell by a quarter; the survey does not explain
  it. Do not read it as a measured change in sentiment.

### Pain-point questions

Stack Overflow asks no per-language pain points; its AI items are the closest recurring frustration measure. 2024: 76
% use or plan to use AI tools, 62 % of professionals use them (44 % in 2023), 43 % trust their accuracy, 31 % are
sceptical, 45 % say the tools are bad at complex tasks. 2025: 80 % use AI tools; trust in accuracy fell from ~40 % to
29 %; 45 % name "AI solutions that are almost right, but not quite" as their top frustration.

---

## 2. JetBrains State of Developer Ecosystem (2023-2025)

| Edition | Respondents | Fielding |
|---|---|---|
| 2023 | 26,348 | not stated on the results page |
| 2024 | 23,262 | May-June 2024 |
| 2025 | 24,534, 194 countries | April-June 2025 |

JetBrains publishes most language figures only as charts; the prose yields:

- **Usage trends.** TypeScript rose from 12 % (2017) to 35 % (2024). Python is used by more than half of respondents,
  up from 32 % in 2017. Rust was "the only commonly used language to set a new popularity record" in both 2023 and
  2024. Objective-C "lost two-thirds of its users in the past six years" (2023); fewer than 2 % used it at all in
  2024.
- **Adoption intentions.** Wording (2024): "Do you plan to adopt / migrate to other languages in the next 12 months?
  If so, to which ones?" 2023 and 2024: Go and Rust are the most planned adoptions; "every sixth Go user is thinking
  about adopting Rust". 2025: Go 11 %, Rust 10 %, Python 7 %, Kotlin 6 %, TypeScript 6 %.
- **Abandonment.** 2023: Rust and Scala are "among the languages that the fewest programmers want to migrate from".
  2025: PHP, Ruby and Objective-C "declining steadily"; JavaScript, PHP and SQL "reached a maturity plateau"; the
  "Language Promise Index" ranks TypeScript, Rust and Go highest. Scala leads the top-paid bracket (38 %) while only 2
  % use it as a primary language.
- **AI.** 2024: 69 % had tried ChatGPT for development, 49 % use it regularly; Copilot 40 % / 26 %. 2025: 85 % use AI
  tools regularly; the top-ranked concern is "the inconsistent quality of AI-generated code".

---

## 3. Rust annual survey (2023, 2024, 2025)

Run by the Rust Survey Team; ~92 % of respondents already use Rust, so it measures the experience of users, not
adoption in the industry.

| Edition | Fielding | Started / completed | Daily users | "I feel productive" |
|---|---|---|---|---|
| 2023 | 18 Dec 2023 - 15 Jan 2024 | 11,950 / 9,710 | 49 % | 47 % (42 % in 2022) |
| 2024 | 5-23 Dec 2024 | 9,450 / 7,310 | 53 % | 53 % |
| 2025 | 17 Nov - 17 Dec 2025 | 9,389 / 7,156 | 55 % | 57 % |

### Productivity limiters (2024 report PDF, n = 6,862)

"Which of the following aspects of Rust present non-trivial problems to your programming productivity?" Columns: big
problem / could be improved but does not limit me / not an issue.

| Aspect | Big problem | Could improve | Not an issue |
|---|---|---|---|
| Slow compilation | 25.2 % | 56.1 % | 18.7 % |
| Subpar debugging experience | 21.2 % | 42.9 % | 35.9 % |
| Writing executor-agnostic async code | 19.5 % | 39.7 % | 40.8 % |
| High disk usage (target folder) | 17.7 % | 47.9 % | 34.4 % |
| Interoperating with C/C++ | 15.3 % | 39.0 % | 45.8 % |
| Splitting code across crates (orphan rule) | 14.9 % | 42.6 % | 42.4 % |
| Structured concurrency with async | 14.9 % | 39.1 % | 46.0 % |
| Not enough possible in `const fn` | 14.7 % | 42.9 % | 42.5 % |
| Subpar IDE support | 14.4 % | 39.2 % | 46.3 % |
| Implementing dynamic library plugins | 13.0 % | 29.5 % | 57.5 % |
| Writing correct unsafe code | 12.6 % | 37.0 % | 50.4 % |
| Large binary size | 12.5 % | 45.0 % | 42.5 % |
| Logic for tuples of various sizes | 11.8 % | 43.2 % | 45.1 % |
| Borrow checker rejecting valid code | 10.6 % | 44.0 % | 45.5 % |
| Opaque/unclear compiler error messages | 9.1 % | 38.5 % | 52.4 % |
| Implementing `Iterator` manually | 8.0 % | 42.5 % | 49.5 % |
| Lacking documentation | 4.8 % | 18.2 % | 77.0 % |
| Compiler bugs | 3.6 % | 13.0 % | 83.5 % |
| Slow runtime performance | 3.2 % | 11.3 % | 85.5 % |

Two readings matter. The famous borrow checker is a "big problem" for one user in ten and compiler messages for fewer
still; the real friction is build time, debugging and async. The 2025 post says resource usage (compile time and
storage) "remains a top problem" and debugging fell from 2nd to 4th; the 2023 prose names the hardest areas as
"asynchronous Rust, the traits and generics system, and the borrow checker" (no percentages).

### Why people do not use Rust, or stopped

- 2023 non-users: 67 % "have not had the chance to prioritize learning it"; 31 % perceived difficulty. Former users:
  46 % factors outside their control, 31 % prefer another language (up 9 points), 24 % difficulty.
- 2024 non-users (n = 327, multi-select): 67 % "haven't got around to it", 31 % too difficult, 21 % prefer another
  language, 15 % outside their control. Former users (n = 348): 64 % plan to return, 45 % outside their control, 31 %
  prefer another language, 24 % too difficult; ~5 % each cite missing libraries, features or tools; ~6 % an
  off-putting community.
- 2025 (n = 358): about a third prefer another language, almost as many lack the opportunity, about a quarter find it
  "too difficult to learn", just under a tenth blame the community; 60 % plan to use it when they can.

### Rust at work (organisational adoption)

| Statement (agree) | 2022 | 2023 | 2024 | 2025 |
|---|---|---|---|---|
| Using Rust has helped our company achieve its goals | 72 % | 79 % | 82 % | 85 % |
| Adopting Rust has been challenging for the organisation | 38-39 % | 34 % | 34.5 % | 31 % |
| Adopting Rust has slowed the team | - | - | 7 % | 6 % |
| Worth the cost of adoption | 60 % | 64 % | - | 65 % |
| Likely to use Rust again | - | 77 % | 78 % | 78 % |

Reasons employers invested in Rust (2023): "building relatively correct and bug-free software" 86 %, performance 83 %;
2024 and 2025 repeat the ranking without figures. Employer makes non-trivial use of Rust: 39 % (2023) -> 45 % (2024).
Rust for the majority of their coding: 34 % -> 38 % -> 40 %.

### Worries about Rust's future

| Concern | 2022 | 2023 | 2024 | 2025 |
|---|---|---|---|---|
| Rust becomes too complex | 38 % | 43 % | 45.2 % | 41.6 % `[secondary]` |
| Not enough industry usage | - | 42 % | 45.5 % | top concern (no figure) |
| Developers/maintainers not properly supported | 26 % | 32 % | down ~6 pts | "rose slightly" |
| Not worried at all | 30 % | 18 % | 18.6 % | - |

Only 7 % (2024) want Rust to slow down or stop adding features; more than a quarter want faster stabilisation.
Maintainer priorities (2023): fixing compiler bugs 68 %, runtime performance 57 %, compile times 45 %.

### Sentiment and jobs (2025, new items)

93 % agree Rust provides a real benefit over other languages; 84 % agree it is fun; 35 % agree it is "significantly
more complicated to program in than other languages". Only 3 % agree it is easy for junior programmers to find Rust
jobs (54 % disagree); 19 % for seniors. Community participation fell ~3 points; respondents increasingly ask LLMs
instead of forums `[secondary, The New Stack]`. Many find `rustc --explain` useful, which the team says contradicted
its assumption.

---

## 4. Go developer survey (2023 H2, 2024 H1, 2024 H2, 2025)

Run by the Go team at Google; mixes self-selected respondents with randomly prompted VS Code and GoLand users;
reported with 95 % confidence intervals.

| Edition | Fielding | Responses | Satisfied (somewhat+very) |
|---|---|---|---|
| 2023 H2 | Aug 2023 | 4,005 | 90 % (52 % very) |
| 2024 H1 | Jan-Feb 2024 | 6,224 (64 % self-selected) | 93 % |
| 2024 H2 | 9-23 Sep 2024 | 4,156 (57 % self-selected) | 93 % |
| 2025 | 9-30 Sep 2025 | 7,070 total, 5,379 after cleaning | 91 % (62 % very) |

### Biggest challenges

- **2023 H2.** Starting new projects: choosing a project structure 54 %, learning to write idiomatic Go 47 %. 43 %
  agreed that "Go requires a lot of tedious, boilerplate code to check for errors". Open-ended closing question:
  expressivity 12 % and error handling 12 % tied as the most requested improvements, type safety/reliability 9 %.
  Wanted from error messages: help understanding what led to the error 36 %, guidance on the fix 21 %.
- **2024 H1** (closed-form "biggest challenge when using Go"): learning how to write Go effectively 15 %, verbosity of
  error handling 13 %, no challenges 13 %, other 8 %. Open text: learning Go / best practices / docs 11 %, error
  handling 11 %. Missing features: the type system was the most common theme, "especially requests for enums, option
  types, and sum types".
- **2024 H2** (team-level, "would benefit most if magically solved"): consistent coding standards 58 %, identifying
  performance issues in a running program 58 %, resource inefficiencies 57 %.
- **2025** ("What are your three most frustrating things about working with Go?"): ensuring code follows best
  practices / Go idioms 33 %; a feature valued from another language is not part of Go 28 %; finding trustworthy Go
  modules and packages 26 %. Error handling, enums/sum types, nil-pointer safety and verbosity appear as open-text
  themes without figures. Asked about their favourite *other* language, respondents valued inheritance 71 %, type-safe
  enums 65 %, exceptions 60 %, static typing 51 %. 15-25 % often re-read documentation for each `go` subcommand.
- **AI (2025).** 53 % use AI tools daily, 29 % rarely or never; 55 % satisfied (42 % somewhat, 13 % very); main
  problems are non-functional code (53 %) and working-but-poor-quality code (30 %).

In June 2025 the Go team formally stopped pursuing syntax changes for error handling, noting that "complaints about
error handling have topped our annual user surveys for years" (see `designer-retrospectives.md`).

---

## 5. Python Developers Survey (2024) and the Typed Python surveys

**PSF / JetBrains Python Developers Survey 2024.** October-November 2024; "more than 30,000" participants from ~200
countries, >25,000 after cleaning.

- Versions: 3.12 35 %, 3.11 21 %, 3.13 15 %, 3.10 15 %, 3.9 6 %; still on Python 2: 4 %. Why not upgrade: "meets all
  my needs" 53 %, projects not compatible 27 %, no time 25 %, stability concerns 17 %.
- Dependency tools ("Which tools do you use to manage dependencies?", 2023 -> 2024): pip 77 % -> 74 %, Poetry 19 % ->
  20 %, Conda 19 % -> 18 %, uv (new) 12 %, pip-tools 9 %, Pipenv 9 % -> 8 %. Environment isolation: venv 55 % -> 62 %,
  virtualenv 28 % -> 25 %, Conda 19 %, Poetry 18 %, uv 11 %.
- Metadata: requirements.txt 59 % (63 % in 2023), pyproject.toml 36 % (32 %), setup.py 16 %. 51 % had never heard of
  PyPI Trusted Publishers. No tool other than pip exceeds a fifth of users, and uv went from zero to 12 % in a year
  (43 % of Django developers in a 2026 JetBrains survey `[secondary]`).

**Typed Python survey 2024** (Meta, Microsoft, JetBrains; n = 1,083; 29 Jul - 8 Oct 2024). 88 % use type hints
"always" or "often" (66 % among hobbyists, 78 % among professionals-only). Checkers: mypy 67 %, Pyright 38 %, both 24
%; Pydantic 62 %. Most valued: better IDE support 59 %, preventing bugs 49.8 %, documentation 49.2 %. Pain points
(free-text counts): expressing dynamic features 29, slow checkers 22, inconsistencies across checkers 21, unclear docs
for advanced constructs 10. The authors say the sample is not representative and likely attracted typing enthusiasts.
The 2025 edition reports 86 % adoption and growth of Rust-based checkers `[secondary]`.

---

## 6. State of JS (2024, 2025)

Balance question: "what proportion of your code is TypeScript vs JavaScript" (9-point scale).

| | 2024 (n = 11,435) | 2025 (n = 10,934) |
|---|---|---|
| 100 % TypeScript | 3,936 (34 %) | 4,367 (39.9 %) |
| 0 % TypeScript | 915 (8 %) | 661 (6.0 %) |
| Mean TypeScript share | 74 % | 77 % |

2024 prose: 67 % write more TypeScript than JavaScript; "We're now firmly in the TypeScript era." Static typing was
the top answer to what is missing from JavaScript (5,403 selections); 8,362 want native types "as similar to
TypeScript as possible" (2025: 5,380 votes for TypeScript-like annotations vs 3,524 for runtime types).

"What aspects of JavaScript do you struggle with the most?" (multi-select):

| Pain point | 2024 (n = 10,599) | 2025 (n = 10,171) |
|---|---|---|
| Code architecture | 3,719 | 3,809 |
| State management | 3,290 | 3,490 |
| Managing dependencies | 3,053 | 3,203 |
| Build tools | 2,925 | 2,303 |
| Performance | 2,538 | 2,578 |
| Debugging | 2,355 | 2,177 |
| Date management | 2,338 | 2,681 |
| Typing | 2,338 | 1,916 |
| Async code | 1,151 | 1,110 |
| Finding packages | 1,102 | 1,303 |

JavaScript happiness: 3.8 / 5 (2025, n = 10,820), "constant once again". Async code is the *least* cited struggle in a
language with pervasive async/await; dependency management and build tools are cited roughly three times as often.

---

## 7. Smaller community surveys

### Haskell (2022, Fausak; 2025, Haskell Foundation)

- 2022: 1,038 responses (1-15 Nov 2022). Use at work: most of the time 33 %, some 16 %. Build tools: Cabal 67 %, Stack
  49 %, Nix 33 %. Agreement (strongly agree / agree / neutral / disagree / strongly disagree): "Once my Haskell
  program compiles, it generally does what I intended" 28 / 48 / 11 / 4 / 1; "software written in Haskell is easy to
  maintain" 34 / 35 / 12 / 3 / 1; "Haskell libraries are easy to use" 4 / 32 / 34 / 15 / 4; "I can easily reason about
  the performance of my Haskell code" 3 / 15 / 27 / 30 / 12 (42 % disagree); "satisfied with build tools" 11 / 40 / 22
  / 13 / 5. "If you could change one thing" answers were collected but not analysed.
- 2025: 1,417 responses. 72.3 % use Haskell, 16.3 % used to; of those who stopped, 42 % had used it for more than a
  year. 27.1 % had a compiler upgrade break their code in the last year (mostly incompatible dependencies). Cabal 84
  %, Stack 40 %, Nix 39 %. Wanted writing: best practices 63.6 %, design patterns 51.0 %, performance analysis 46.4 %,
  debugging how-tos 33.6 %.

The Haskell data are the clearest quantitative support for "if it compiles it works" (76 % agree) *and* for its cost:
performance is unpredictable (only 18 % can reason about it) and the ecosystem fractured over build tools.

### OCaml (OCSF 2022; 2023)

The 2022 survey had far fewer respondents than 2020 (~279 answered Q6). Free-text pain points repeatedly named
documentation, the debugger and opam/dune. The 2023 survey (summarised by Tarides and a community HackMD) again
flagged debugging of native executables `[secondary]`. No 2024 report was located.

### Clojure (State of Clojure 2024, 2025)

2024: 73 % use Clojure for work; 58 % already on 1.12; Babashka used by 93 % of those answering. 2025: 71 % work use;
"How likely is it that you would recommend Clojure?" 70 % very likely, 8 % would not; 70 % have used AI tools. The
new-user challenges question is published only as an image; the 2023 open responses (Q7, "biggest challenges in using
Clojure now") are published as PDFs. A 2026 Clojurists Together member survey ranks developer-experience tooling as
the top area needing improvement (45.7 %) `[secondary]`.

### Julia (annual User & Developer Survey, 2024 and 2025)

2024: 682 interviews, 4-26 June 2024, +/- 3.75 points. 2025: 721 interviews, 18 June - 8 July 2025, +/- 3.65 points;
recruited via Slack, Discourse, X, LinkedIn and JuliaLang.org.

"What are the TECHNICAL aspects or features you like LEAST about Julia?" (the PDF's year columns cannot be separated
reliably; ranges cover 2019-2025): slow compile times / too long to generate the first plot 41 % in 2025, 41-50 % in
every year since 2019 (options merged in 2023); cannot generate self-contained binaries 46 % in 2025, 29-52 %;
confusing or overly verbose error messages (added 2023) 22-36 %; debugger too slow or incomplete 17-21 %; poor linter
/ syntax checking vs statically typed languages 13-27 %; missing packages 13-30 %; no built-in interfaces or traits
(added 2023) 17-23 %. Liked most: speed/performance 83-86 % every year; ease of use 67-84 %; package manager 53-72 %.
Top non-technical complaint: "not enough Julia users in my field or industry" 45 % in 2025 (36-64 % over the years).

### Gleam (Developer Survey 2024)

841 responses (end of 2024); 632 current users, 52 in production. Other languages used: TypeScript 385, JavaScript
338, Python 314, Rust 243, Go 230, Elixir 158; users mostly come from statically typed ecosystems. Free-text requests,
categorised by the author: the dynamic JSON decoder API (since redesigned), the minimal OTP library, missing rename in
the language server (since added), documentation on building and deploying, metaprogramming ("requested often, but
rarely with detailed proposals"), difficult Erlang/rebar3 installation, Windows support. Declined: a larger standard
library, a Rails-style framework, single-binary compilation.

### Elixir and Zig

No official Elixir foundation survey exists; Curiosum's "State of Elixir 2025" had 1,018 participants and 45
questions, but its findings could not be retrieved. Stack Overflow 2025: 2.7 % use Elixir (2.1 % in 2024), admired 66
%. No Zig community survey was found; Stack Overflow 2025 admired: 64 %.

---

## 8. Synthesis: what recurs across languages, regardless of paradigm

1. **Build, dependency and packaging tooling** is the most universal complaint. JavaScript: managing dependencies and
   build tools are among the top four struggles. Go: "finding trustworthy modules" 26 %. Python: six competing
   dependency tools, none above 20 % except pip, and a new one (uv) at 12 % within a year. Haskell: Cabal/Stack/Nix
   split, 27 % broken by compiler upgrades. OCaml: opam/dune. Gleam: installing Erlang. The ecosystems that score well
   (Rust: Cargo 70.8 % admired; Go: 91-93 % satisfaction with an integrated `go` command) ship one blessed tool from
   day one.
2. **Compile times and feedback latency.** Rust's top limiter in every survey (25 % big problem, 56 % want it
   improved). Julia's top technical complaint for seven straight years (41-50 %). TypeScript's own team rewrote the
   compiler in Go for a 10x speedup. Nobody complains about *runtime* performance in Rust (3 %).
3. **Debugging and error messages.** Rust: debugging is the 2nd-4th limiter. Julia: confusing error messages 22-36 %.
   Go: the most-wanted error-message improvement is explaining the cause (36 %). Haskell: debugging how-tos wanted by
   34 %. Only 9 % of Rust users call compiler error messages a big problem: investment in diagnostics is visible in
   surveys.
4. **Learning curve of safety features.** Rust: 31 % of non-users cite perceived difficulty, 24 % of ex-users say it
   was too hard, 35 % of users say it is significantly more complicated than other languages; the hardest areas are
   async, traits/generics and the borrow checker. Yet only 10.6 % say the borrow checker rejecting valid code is a big
   problem. The cost is concentrated in learning, not in steady-state use.
5. **Async.** Rust's third-ranked limiter (executor-agnostic async 19.5 %; structured concurrency 14.9 %). In
   JavaScript, where async is pervasive and single-coloured, "async code" is the *least* cited struggle. Function
   colouring plus a pluggable runtime, not async itself, is what surveys flag.
6. **Error handling and missing types.** Go's top language complaint for years (13 % closed-form; 43 % agree it is
   tedious), with enums, option and sum types the most requested features; Go users of other languages prize type-safe
   enums (65 %) and exceptions (60 %). JavaScript's top missing feature is static typing; 88 % of typed-Python
   respondents use hints. Dynamic languages are converging on types; typed languages on sum types.
7. **Complexity anxiety is a leading indicator.** 41-45 % of Rust users fear the language will become too complex,
   though only 7 % want it to slow down; Stroustrup's "Vasa" warning and Go's minimalism are the same fear from the
   other side.

**What satisfaction correlates with** (descriptively): a single integrated toolchain (Go, Rust/Cargo); confidence that
compiled code is correct (Haskell 76 %, Rust 85 % at Google); performance users do not have to think about (Julia's
most-liked feature every year); fast iteration; a welcoming community (only ~6 % of Rust leavers blame the community).
Satisfaction does *not* track simplicity alone: Go (very simple) and Rust (very complex) both sit above 90 % while
their users complain about opposite things.

## Synthesis for Sisu

1. **Ship one blessed build/package/test tool with the compiler** (high confidence): tooling fragmentation is the most
   consistent pain across every ecosystem surveyed; the two best-liked ecosystems both avoided it.
2. **Treat compile latency as a first-class design constraint** (high): top limiter for Rust and Julia and the reason
   for TypeScript's rewrite; decisions that force whole-program monomorphisation or heavy type-level computation must
   be budgeted against it.
3. **Budget diagnostics and debugging as core features, not polish** (high): Rust's diagnostics investment shows up as
   a non-problem in its own survey; Julia and Go show the opposite.
4. **Do not treat async as "solved" by adding async/await** (medium-high): Rust users' pain is executor fragmentation
   and colouring; JavaScript users' pain is elsewhere; the design must decide where the runtime lives.
5. **Provide sum types, option/result and exhaustive matching from day one** (high): the single most requested
   type-system feature in Go, and what Go users admire elsewhere.
6. **Expect the learning-curve cost of safety to be paid mainly once** (medium): Rust's difficulty is concentrated in
   non-users and ex-users; steady-state users call the borrow checker a minor issue; progressive disclosure matters
   more than removing checks.
7. **Keep a complexity budget and say no publicly** (medium): complexity is the top fear of the most-admired
   language's own users; surveys reward restraint (Go) and coherence (Rust) more than feature count.
8. **Discount all of the above for self-selection** (high): none of these surveys sample non-users well; the Go random
   arm and the Rust non-user items are the only windows onto the people a new language must win, and they say "I
   haven't had a reason to try it" far more often than "it is too hard".

## Sources

1. Stack Overflow Developer Survey 2023 (survey.stackoverflow.co/2023/), 2024 (/2024/technology; blog "Developers want
   more, more, more", 24 Jul 2024), 2025 (/2025/technology/; blog "Developers remain willing but reluctant to use AI",
   29 Jul 2025), 2026 (/2026/).
2. JetBrains State of Developer Ecosystem 2023, 2024 (jetbrains.com/lp/devecosystem-20xx/) and 2025
   (blog.jetbrains.com/research/2025/10/).
3. Rust Survey Team: "2023 Annual Rust Survey Results" (19 Feb 2024); "2024 State of Rust Survey Results" (13 Feb
   2025) and report PDF (github.com/rust-lang/surveys); "2025 State of Rust Survey Results" (2 Mar 2026); devclass 18
   Feb 2025 and i-programmer 2026 coverage `[secondary]`.
4. Go team, go.dev/blog: survey2023-h2-results (5 Dec 2023), survey2024-h1-results (9 Apr 2024), survey2024-h2-results
   (20 Dec 2024), survey2025 (21 Jan 2026).
5. PSF/JetBrains, Python Developers Survey 2024 (lp.jetbrains.com/python-developers-survey-2024/); Meta Engineering,
   "Typed Python in 2024", 9 Dec 2024.
6. State of JS 2024 and 2025, usage pages (2024.stateofjs.com, 2025.stateofjs.com).
7. T. Fausak, "2022 State of Haskell Survey results", 18 Nov 2022; Haskell Foundation, "State of Haskell 2025
   results", 2 Mar 2026; OCSF, "Results of the OCaml User Survey 2022", Dec 2022; clojure.org, "State of Clojure 2024"
   (2 Dec 2024) and "2025" (18 Feb 2026).
8. A. Claster, Julia User & Developer Survey 2024 and 2025 (julialang.org/assets/); L. Pilfold, "Gleam Developer
   Survey 2024 Results", 6 Feb 2025; Curiosum, "State of Elixir 2025" announcement (elixirforum.com).
