# Python

## Snapshot
| | |
|---|---|
| First appeared | 1991 (0.9.0, Guido van Rossum, CWI Amsterdam); 2.0 in 2000; 3.0 on 3 Dec 2008 |
| Designed by | Guido van Rossum (BDFL until 12 July 2018); since 2019 a five-person Steering Council |
| Current version (Oct 2026) | 3.15.0 (9 Oct 2026); 3.14.8 and 3.13.16 maintenance releases (30 Sep 2026) |
| Governance | Python Software Foundation (legal/financial) + elected Steering Council (PEP 13); changes via PEPs |
| Typing | Dynamic, strong; optional gradual static typing via annotations (PEP 484, 2014) checked by external tools |
| Memory | Reference counting plus a cycle collector; free-threaded build uses biased reference counting + mimalloc |
| Compilation | Bytecode interpreter (CPython); specialising adaptive interpreter since 3.11; experimental opt-in JIT since 3.13 |
| Primary domains | Data science and ML/AI, web back ends, scripting/automation, education, scientific computing |
| Notable users | Google, Meta (Instagram), Dropbox, Netflix, NASA, CERN, essentially the whole ML stack (PyTorch, NumPy, pandas) |

Usage figures: Stack Overflow 2025 (31,771 respondents) puts Python at 57.9% of all
respondents and 54.8% of professionals, the second most used language after
JavaScript [1]. GitHub's Octoverse 2025 reports TypeScript overtook Python in
monthly contributors in August 2025 (about 2.64M vs 2.6M), a gap of roughly
42,000 contributors, driven by AI-assisted coding and TypeScript-by-default
frameworks [2]. The PSF/JetBrains 2024 developer survey (October-November 2024,
over 30,000 respondents) found the main uses are data analysis (49%), web
development (48%) and machine learning (42%) [3].

## Stated design goals

Van Rossum designed Python as a successor to ABC, a teaching language developed
at CWI, with the explicit aims of readability, a small core, and extensibility
through C. The community's later codification is PEP 20, "The Zen of Python"
(Tim Peters, 2004), whose most-cited lines are "Readability counts", "Explicit
is better than implicit", "Simple is better than complex" and "There should be
one-- and preferably only one --obvious way to do it" [4]. The last line is a
deliberate counter-position to Perl's "There's more than one way to do it".

PEP 3099, "Things that will Not Change in Python 3000" (2006), is an unusually
explicit statement of what the designers considered settled: no braces, no
static typing of the core language, `self` stays explicit, `lambda` stays
single-expression, no programmable syntax (macros) [5]. Each of those decisions
has been argued against for twenty years; none has been reversed.

## Strengths

### Readability as the main driver of dominance in teaching and data science

The concrete features are significant indentation, English keywords (`and`,
`not`, `in`), few sigils, slicing, comprehensions, keyword arguments and
docstrings. The evidence that this works is adoption in exactly the populations
that read more code than they write: in 2014 Philip Guo found Python was the
most common introductory language at the top 39 US computer-science
departments (data from that year; not re-verified for 2026) [6], and the 2024
PSF survey shows half of Python developers have two years or less experience
with the language, i.e. it keeps absorbing newcomers [3]. The "one obvious way"
philosophy means idiomatic code written by strangers looks alike, which is the
property that made Python the glue language for scientists who are not
professional programmers.

### A consistent object protocol: the data model

Every operator, built-in function and syntactic form dispatches to a named
special method: `__len__`, `__iter__`, `__getitem__`, `__enter__`/`__exit__`,
`__add__`, `__eq__`, `__hash__`, `__call__`, and so on (the "data model"
chapter of the language reference). Because it is one uniform mechanism, user
types can participate in every language construct (iteration, `with`, slicing,
arithmetic, truthiness) without the language knowing about them, and NumPy's
arrays, pandas' frames and PyTorch's tensors all feel built in. The evidence
that it works is that the entire scientific stack is built on it and that other
languages copied the shape (Ruby's operator methods predate it; Rust's operator
traits, Swift's protocols for literals and sequences are the typed analogue).
Dunder names are ugly but reserve a namespace for the protocol without
keywords, which has let Python add protocols (`__fspath__`, `__class_getitem__`,
`__buffer__` in 3.12) for 30 years without syntax changes.

### Batteries included, and the discipline to remove dead ones

The standard library (json, re, sqlite3, http, argparse, pathlib, dataclasses,
unittest, asyncio, zipfile, csv, datetime, decimal, fractions, itertools,
functools, collections, logging, subprocess, typing) is large enough that a
script rarely needs a dependency. The counter-evidence to "batteries rot" is
PEP 594 (2022), which removed 19 dead modules (asynchat, cgi, imp, telnetlib
and so on) in 3.13, and PEP 632 which removed distutils in 3.12 [7]. Python
3.15 continues this with removals of `sre_compile`/`sre_parse` and the
deprecation of the `profile` module for 3.17, replaced by the new
`profiling` package and Tachyon sampling profiler (PEP 799) [8].

### The PEP process as a model of written, archived decision making

Every language change since 2000 is a numbered document with motivation,
rationale, rejected alternatives and a discussion link. Researchers can trace
why `print` became a function (PEP 3105), why `:=` exists (PEP 572), why
annotations are now lazily evaluated (PEP 649/749). This record is the reason
Python's mistakes can be studied at all; it is the strongest form of evidence in
this research project's own hierarchy. The process survived the loss of its
BDFL: PEP 8016 (December 2018) created the Steering Council and the first
council was elected in January 2019 with no interruption to releases [9].

### Iterators, generators and comprehensions

`yield` (PEP 255, 2001), generator expressions (PEP 289, 2004) and the
iterator protocol made lazy pipelines idiomatic a decade before Java streams or
C# LINQ became common. `yield from` (PEP 380) and later `async for` build on the
same mechanism. The evidence of success is imitation: JavaScript generators
(ES2015), Kotlin sequences, C# iterators and Rust's `Iterator` adaptors all
cover the same ground, and comprehensions were copied by Julia, Nim, Elixir
(`for` comprehensions), Haskell's list comprehensions having been the original.
PEP 798 (3.15) is still extending the syntax 25 years later with `*`/`**`
unpacking inside comprehensions [8].

### Context managers and `with`

PEP 343 (2005) tied resource cleanup to lexical scope through `__enter__` and
`__exit__`, with `contextlib` making it trivial to write one as a generator.
This is now the standard answer in GC'd languages to "how do you release
non-memory resources": Java's try-with-resources (2011), C#'s `using`
declarations, Kotlin's `use` and JavaScript's `using` (TC39, 2026) cover the
same ground. Decorators (PEP 318, 2004) are the same kind of win: plain
higher-order functions with sugar, copied by JS/TS decorators and used for
routing, caching and dataclasses. Error messages with "Did you mean" hints
(3.10 onward) are a deliberate learnability investment of the same kind.

## Weaknesses

### The GIL, and what 27 years of attempts to remove it teach

The Global Interpreter Lock serialises bytecode execution, so threads give no
CPU parallelism. The history of fixes: Greg Stein's free-threading patch for
Python 1.4 (1996-1999) cost roughly half of single-threaded speed and was
rejected; Larry Hastings' "Gilectomy" (2016-2017) was abandoned for the same
reason; `multiprocessing` (2008), subinterpreters (PEP 554/684/734, 2017-2025)
and `concurrent.futures` were workarounds. Sam Gross's nogil work became PEP 703
(created January 2023; accepted by the Steering Council on 24 October 2023 with
the proviso that "the rollout be gradual and break as little as possible" and
the explicit right to revert all of it) [10]. The accepted plan: 3.13 ships an
experimental `--disable-gil` build with a separate ABI; after 2-3 releases the
GIL becomes a runtime switch; after another 2-3 releases it is off by default.
PEP 779 (accepted 16 June 2025) promoted the build to "officially supported but
optional" in 3.14; the single-threaded penalty is "roughly 5-10%, depending on
the platform and C compiler" [11]. 3.15 adds a stable ABI for free-threaded
extensions (`abi3t`, PEP 803), makes mimalloc the default raw allocator, and the
official macOS installers now include free-threading by default; whether it
becomes the default build is still undecided [8].

The lessons are specific. (1) The GIL was never a language design decision; it
was an implementation shortcut that became load-bearing because every C
extension's thread safety silently depended on it. (2) Removing it required
re-engineering reference counting (biased refcounting, immortal objects, PEP
683), the allocator, every container's locking, and the C API, and the agreed
budget for acceptable single-thread regression was about 10%. (3) Even with a
funded, accepted plan, the transition is scheduled across roughly 2023-2030.
(4) The successful attempt came from outside the core team with a working
prototype and benchmark numbers, not from a design document. A new language
should decide its parallelism story and its FFI's thread-safety contract on day
one, because the cost of retrofitting them scales with the ecosystem.

### Packaging: twenty years of fragmentation, then `uv`

The build and install story accreted layer by layer: distutils (1998, removed
in 3.12), setuptools and `easy_install` (2004), virtualenv (2007), pip (2008),
`venv` in the standard library (PEP 405, 2012), wheels (PEP 427, 2012),
pyproject.toml build isolation (PEP 517/518, 2016-2017), Pipenv (2017), Poetry
(2018), standardised project metadata (PEP 621, 2020), and only in March 2025 a
standard lock file format (PEP 751, `pylock.toml`) [12]. The 2024 PSF survey
shows the consequences: pip 74%, Poetry 20%, Conda 18%, pip-tools 9%, Pipenv 8%;
requirements.txt still the dominant dependency format at 59% versus 36% for
pyproject.toml; and only 12% use PyPI Trusted Publishers while 51% have never
heard of it [3]. Three partially overlapping tools (pip, conda, Poetry) and two
dependency formats is the "no one obvious way" outcome in a language whose
motto is the opposite.

`uv` (Astral, February 2024, written in Rust) bundled resolver, installer,
virtualenv creation, Python version management, lock file, script runner and
publishing into one fast binary with a Cargo-like command set. In the survey
collected eight months after its first release it already had 11-12% usage
(the first year it appeared) [3]; JetBrains' Django-focused survey published in
August 2026 reports 43% using uv for environment management, behind only `venv`
at 63% [13]. Stack Overflow 2025 ranked uv as the most admired tool tag [14].
No standard changed; the adoption came from one tool being fast, complete and
opinionated. That is the central packaging lesson from Python: standards
(PEP 517/518/621/751) are necessary so tools can interoperate, but adoption
follows a single integrated, fast, default tool, and the language that ships
one from day one avoids a decade of fragmentation.

### The 2-to-3 transition (2008-2020)

Python 3.0 (3 December 2008) broke string semantics (bytes vs text), `print`,
integer division, dict views and removed many APIs. The planned five-year
overlap became twelve: Python 2.7's end of life was extended from 2015 to
1 January 2020 (PEP 373), and 4% of respondents in the 2024 survey still use
Python 2 [3]. What went wrong, as the core team and van Rossum have said in
retrospectives, is not that the changes were wrong (bytes/text was the right
fix) but that (a) there was no migration path in which one codebase could run
on both versions for several years (`2to3` was one-way; `six` and `__future__`
imports were community patches), (b) the libraries that everyone depended on
could not move until their dependencies moved, so the ecosystem deadlocked,
and (c) 3.0-3.2 offered users nothing in return for the pain; adoption only
started once 3.3-3.5 added features (`yield from`, `asyncio`, `async/await`,
type hints) that 2.7 lacked. The current policy is the opposite extreme:
PEP 387 requires a two-release deprecation period, there is no planned
"Python 4", and the free-threading and JIT work are explicitly designed as
opt-in transitions.

### Gradual typing: retrofit success, growing complexity

PEP 484 (2014, Python 3.5) added annotations that the interpreter ignores and
external checkers (mypy, pyright, pytype, Pyre; since 2025 the Rust-based ty,
Pyrefly and Zuban) verify. Adoption among people who care enough to answer a
typing survey is high: 88% "always" or "often" use type hints in the 2024
JetBrains/Meta/Microsoft survey (over 1,000 respondents) and 86% in 2025
(1,241 respondents; mypy 58%, down from 61%; Rust-based checkers over 20%
combined) [15][16]. The pain points listed in 2025 are the ones predicted by
the retrofit: missing or poor annotations in third-party libraries (NumPy,
pandas, Django), advanced features that are hard to use (generics, variance,
callables/decorators), fragmentation and inconsistency between checkers,
"optional and not enforced at runtime", verbosity, and "less expressive than
TypeScript, with frequent syntax changes" [16].

The type system has grown a feature almost every release to describe idioms
the dynamic language already allowed: `Protocol` (PEP 544, structural typing),
`TypedDict` (PEP 589), `Literal` (PEP 586), `Final` (PEP 591), `X | Y` (PEP
604), `ParamSpec` (PEP 612), `TypeGuard`/`TypeIs` (PEP 647/742), variadic
generics (PEP 646), `Self` (PEP 673), `dataclass_transform` (PEP 681), the new
generic syntax `def f[T](x: T)` (PEP 695, 3.12), type parameter defaults (PEP
696, 3.13), `ReadOnly` TypedDict items (PEP 705), deferred annotation
evaluation (PEP 649/749, 3.14), and in 3.15 `TypedDict` extra items (PEP 728),
`TypeForm` (PEP 747) and disjoint bases (PEP 800) [8][17]. There is no single
specification of the type system beyond the typing council's spec started in
2023, and checkers disagree on inference. The lesson is not that gradual typing
fails; it is that a type system designed after the fact must describe every
dynamic idiom users already rely on, and so ends up larger and less uniform
than one designed with the language.

### Performance and the cost of dynamism

CPython's baseline is 10-100x slower than C on scalar code because every
attribute access is a dictionary lookup, every integer a heap object, and every
call builds a frame. The Faster CPython project (Microsoft-funded, 2021-2025)
delivered the specialising adaptive interpreter (PEP 659) and a 1.25x average
speed-up in 3.11 (10-60% on individual benchmarks) [18], then a copy-and-patch
JIT (PEP 744) shipped experimentally in 3.13. In May 2025 Microsoft laid off
most of that team, including Mark Shannon, Eric Snow and Irit Katriel [19].
The JIT nonetheless continued: 3.15's upgraded JIT reports a 7-8% geometric
mean improvement over the optimised interpreter on x86-64 Linux and 11-12% over
the tail-calling interpreter on AArch64 macOS, with individual benchmarks
ranging from a 15% slowdown to over 100% speed-up, and it remains experimental
and opt-in at build time [8]. 3.14's tail-calling interpreter gave 3-5% but
only with Clang 19+ [11]. Five years of expert work bought roughly 1.5x; the
semantics (monkey-patchable everything, `__getattr__`, `locals()`, frame
introspection) cap what any JIT can assume. PyPy demonstrated 4-5x on pure
Python a decade earlier but never became the default because C extensions
depend on CPython's object layout; the C API is both Python's greatest asset
and the reason its implementation cannot be replaced.

### Mojo as evidence of the limits

Modular's Mojo (2023) started as "a superset of Python" with static types,
ownership and SIMD for AI kernels. Mojo 1.0 shipped on 11 August 2026 and the
compiler was open-sourced on 18 August 2026 under Apache 2.0 with LLVM
exceptions [20]; the roadmap now states "Mojo may or may not evolve into a full
superset of Python, and it's okay if it doesn't" and that Python values require
explicit `PythonObject` annotations [21]. The existence of Mojo, Cython, Numba
and the entire pattern of "write the kernel in C++/CUDA, call it from Python"
says that Python won AI as a coordination language, not a computation
language, and that making a Python-compatible language fast enough for kernels
proved incompatible with being a superset.

### Significant indentation: the evidence on both sides

For: the single most-cited reason beginners find Python readable is that the
block structure they see is the block structure the compiler sees, and there is
no brace-style argument. Against: tabs-versus-spaces errors (3.x forbids
mixing, which fixed the worst case), code cannot be pasted into a REPL or a
shell string without care, lines cannot be freely reflowed by formatters
(Black solves formatting but cannot move statements), multi-line lambdas are
impossible (PEP 3099 keeps `lambda` to one expression), and generated or
templated code must track indentation level. Of the mainstream languages designed
after 2010, none copied indentation-sensitive statements (Kotlin, Swift, Rust,
Go, Zig all use braces plus a formatter), which is weak evidence that designers
see the trade-off as negative once a formatter is standard. Python's own
designers have never proposed changing it.

### Mutable default arguments and late-binding closures

`def f(x, acc=[])` evaluates the default once at definition time, so the list is
shared across calls; this is the top entry in the official FAQ ("Why are
default values shared between objects?") and in every list of Python gotchas
[22]. Closures capture variables, not values, so `[lambda: i for i in
range(3)]` all return 2. Both follow logically from the language's "everything
is evaluated at run time" model, both are 30 years old, both are documented
rather than fixed, and linters (ruff B006/B008) exist to catch them. The
lesson is that evaluation-time rules must be the ones a reader expects, and
that defaults evaluated per call (as in Ruby and Kotlin) cost nothing.

### `asyncio` and coloured functions

`asyncio` (3.4, 2014, provisional) and `async`/`await` (PEP 492, 3.5) added a
second, incompatible calling convention. Synchronous libraries (requests,
psycopg2, SQLAlchemy 1.x) cannot be called from a coroutine without blocking
the loop, so the ecosystem duplicated itself (httpx, asyncpg, aiofiles,
SQLAlchemy 2.0's async layer). Mixing is painful: `asyncio.run()` cannot be
nested, `run_in_executor` is the escape hatch, Jupyter needs `nest_asyncio`.
Trio's structured concurrency (2017) was later partially adopted as
`asyncio.TaskGroup` and exception groups (PEP 654, 3.11). Usage data: 23% of
2024 survey respondents use asyncio and 38% FastAPI, so the model is now
mainstream for web services but still a minority idiom overall [3]. The
free-threaded build and subinterpreters (`concurrent.interpreters`, PEP 734 in
3.14) are alternative answers that avoid colouring, which is itself evidence
that the core team does not regard `asyncio` as the whole answer.

### EAFP, exceptions as control flow

The glossary defines EAFP ("easier to ask for forgiveness than permission")
as the recommended style: try the operation and catch `KeyError`,
`AttributeError`, `StopIteration`. It is fast in CPython (zero-cost `try` since
3.11), it is atomic where LBYL is racy, and `StopIteration` is literally how
iteration terminates. The cost is that a function's failure modes are
invisible in its signature; there is no checked-exception or `Result` type, so
callers learn what can be raised by reading the implementation, and bare
`except:` blocks hide bugs. PEP 765 (3.14) had to add a `SyntaxWarning` for
`return`/`break` inside `finally` because the interaction was silently
swallowing exceptions [11]. Type checkers do not model exceptions at all.
Python shows that exceptions-as-control-flow is ergonomic for scripts and
unmanageable for understanding large systems' failure surfaces.

### Metaprogramming power that only experts can use safely

Decorators, metaclasses (`__init_subclass__` was added in 3.6 specifically to
avoid needing a metaclass), descriptors (`property`, `classmethod`), `__getattr__`,
`exec`, and monkey patching make frameworks like Django's ORM, pytest's fixture
injection, SQLAlchemy and dataclasses possible with no language support. The
cost is that static tools cannot see through them: `dataclass_transform` (PEP
681) exists because type checkers had to be taught about libraries that
generate `__init__` at run time, and every ORM ships a mypy plugin. The lesson
is that a language should offer a small, statically analysable form of
metaprogramming (derive-like, compile-time) for the common cases, so run-time
reflection is the exception.

### Governance shock: the walrus operator

PEP 572 (assignment expressions, `:=`) was accepted in July 2018 after an
unusually hostile debate. On 12 July 2018 van Rossum wrote to python-committers:
"Now that PEP 572 is done, I don't ever want to have to fight so hard for a
PEP and find that so many people despise my decisions", and that he was taking
"a permanent vacation from being BDFL", declining to name a successor and
asking the committers to choose a model ("Create a democracy? Anarchy? A
dictatorship? A federation?") [23]. Several governance PEPs (8010-8016) were
written and voted on, and PEP 8016's Steering Council was chosen in December
2018. The lesson is that a single-person veto does not scale with community
size, and that the handover worked only because the PEP process already existed
to carry decisions.

## Designer retrospectives

- Van Rossum on the GIL (many talks, e.g. PyCon 2010 keynote and later blog
  posts): he would accept removal only if single-threaded performance did not
  regress; this became the operating constraint for PEP 703 and PEP 779.
- Van Rossum's 2018 resignation email (above) is the clearest statement that
  BDFL governance had stopped working for him [23].
- Van Rossum and others have repeatedly said that `bytes`/`str` separation was
  right but the lack of a dual-version migration path in 3.0-3.2 was the
  mistake of the 2-to-3 transition; the current no-"Python 4" policy and PEP
  387's deprecation rules follow from this.
- PEP 3099 records the designers' deliberate non-changes; its persistence is a
  retrospective in itself [5].
- The Steering Council's acceptance notice for PEP 703 reserves the right to
  revert the whole project if "too disruptive", an explicit admission that
  retrofitting parallelism might fail [10].

## Influence

Copied from Python by others: significant indentation (CoffeeScript, Nim, YAML,
Haskell's layout rule predates it); comprehensions (Julia, Nim, Elixir); `with`
(C# `using` declarations, Java try-with-resources, JS `using`); decorators
(JS/TS decorators); keyword arguments and `*args`/`**kwargs` (Ruby 2.0 keyword
args, Julia, Nim, Swift labels); generators (ES2015, C#); the iterator protocol;
gradual typing via annotations (Ruby's RBS is a reaction; PHP and TypeScript
share the "erased annotations" model); `dataclasses` (Kotlin data classes
predate; Java records postdate); pattern matching syntax (PEP 634) borrowed
from Rust and Scala. Mojo, Cython, Codon and Nim's syntax exist because of
Python's syntax. The `uv` command set is Cargo's, which is itself Bundler's.

Copied by Python from others: the data model from Smalltalk and CLOS-style
generic operations; `lambda`, `map`, `filter` from Lisp; exceptions from
Modula-3; list slicing and `print` from ABC; `async`/`await` from C#; `match`
from Rust/Scala; `Protocol` from Go's structural interfaces and TypeScript;
`TypedDict` from TypeScript; `Result`-like `ExceptionGroup` from Trio; the
`:=` operator from C via Go; f-strings from C#/Swift interpolation; t-strings
(PEP 750, 3.14) from JavaScript tagged templates.

## Lessons for Sisu

- **Adopt**: readability as a stated, measured goal with one idiomatic form
  per task; a uniform object protocol (every operator and construct maps to a
  nameable method/trait) so user types are first-class in all syntax; a
  numbered, archived proposal process with required "rejected alternatives"
  sections from day one; context-manager-style scoped resources; keyword and
  keyword-only arguments; generators/iterators as the default lazy pipeline;
  quality error messages with suggestions as a release-level priority; a
  single integrated project tool (the `uv`/Cargo shape: resolver, lock file,
  environments, toolchain versions, scripts, publishing) shipped with the
  language.
- **Adapt**: significant indentation only if the language also has a
  canonical formatter and the grammar forbids tab/space mixing; otherwise use
  braces with a mandatory formatter, which gets most of the readability without
  the paste/reflow/lambda costs. Decorators as plain higher-order functions,
  but with a compile-time (derive-like) form for the cases that generate
  members, so tools can see the result. Batteries included, but with a
  scheduled removal policy (PEP 594) written into the governance before the
  first release. Exceptions, if present, must be visible in signatures or
  replaced by result values for recoverable errors; keep "EAFP" ergonomics
  (e.g. `?` style propagation) without the invisible failure surface.
- **Avoid**: an implementation-level global lock that the FFI contract silently
  depends on; a C API that exposes object layout, which locks the
  implementation forever (PyPy) and makes removing the lock a decade-long
  project; retrofitting a type system after idioms have formed (design typing
  in, even if gradual); definition-time evaluation of defaults and capture-by-
  variable closures that surprise readers; a second-colour async API without a
  plan for calling sync code from async and vice versa; a breaking "version 3"
  without a multi-year dual-run path and immediate user-visible benefits; a
  single-person veto as the only decision mechanism.
- **Open questions raised**: whether a language can be as approachable as
  Python for non-programmers while being statically typed by default (the 2025
  typing survey says newcomers use hints at 83%, so annotations themselves are
  not the barrier; inference quality and error messages are). Whether
  significant indentation is net positive once a formatter is mandatory.
  Whether function colouring is acceptable if the standard library is
  async-first from the start (Python's problem was the duplicate ecosystem, not
  the keyword). How much dynamism (reflection, monkey patching) to permit; it
  built Python's ecosystem and capped its performance at roughly 1.5x gains
  from five years of expert JIT work.

## Sources

1. Stack Overflow Developer Survey 2025, Technology section. https://survey.stackoverflow.co/2025/technology (2025)
2. GitHub Octoverse 2025, "A new developer joins GitHub every second as AI leads TypeScript to #1". https://github.blog/news-insights/octoverse/octoverse-a-new-developer-joins-github-every-second-as-ai-leads-typescript-to-1/ (28 Oct 2025)
3. PSF / JetBrains, Python Developers Survey 2024 results (collected Oct-Nov 2024). https://lp.jetbrains.com/python-developers-survey-2024/ (2025)
4. PEP 20, The Zen of Python. https://peps.python.org/pep-0020/ (2004)
5. PEP 3099, Things that will Not Change in Python 3000. https://peps.python.org/pep-3099/ (2006)
6. Philip Guo, "Python is Now the Most Popular Introductory Teaching Language at Top U.S. Universities", CACM blog. https://cacm.acm.org/blogcacm/python-is-now-the-most-popular-introductory-teaching-language-at-top-u-s-universities/ (2014)
7. PEP 594, Removing dead batteries from the standard library. https://peps.python.org/pep-0594/ (2022); PEP 632, Deprecate distutils. https://peps.python.org/pep-0632/ (2020)
8. What's New in Python 3.15. https://docs.python.org/3.15/whatsnew/3.15.html (Oct 2026); release list at https://www.python.org/downloads/
9. PEP 13, Python Language Governance, and PEP 8016. https://peps.python.org/pep-0013/ https://peps.python.org/pep-8016/ (2018)
10. PEP 703, Making the Global Interpreter Lock Optional in CPython (status Final; SC acceptance 24 Oct 2023). https://peps.python.org/pep-0703/ (2023)
11. What's New in Python 3.14 (PEP 779 supported status; 5-10% single-thread penalty; tail-calling interpreter; incremental GC revert in 3.14.5). https://docs.python.org/3.14/whatsnew/3.14.html (2025-2026); PEP 779 discussion https://discuss.python.org/t/pep-779-criteria-for-supported-status-for-free-threaded-python/84319
12. PEP 751, A file format to record Python dependencies for installation reproducibility. https://peps.python.org/pep-0751/ (accepted March 2025); uv 0.6.15 release notes (preliminary pylock.toml support, 2025)
13. JetBrains, "The State of Django 2026". https://blog.jetbrains.com/pycharm/2026/08/the-state-of-django-2026-boring-is-so-back/ (Aug 2026)
14. Real Python news, September 2025 (uv most admired tool in SO 2025). https://realpython.com/python-news-september-2025/ (2025)
15. Meta Engineering, "Typed Python in 2024: Well adopted, yet usability challenges persist". https://engineering.fb.com/2024/12/09/developer-tools/typed-python-2024-survey-meta/ (Dec 2024)
16. Meta Engineering, "Python Typing Survey 2025". https://engineering.fb.com/2025/12/22/developer-tools/python-typing-survey-2025-code-quality-flexibility-typing-adoption/ (Dec 2025)
17. PEP 484, 544, 589, 604, 612, 646, 649, 695, 696, 749 at https://peps.python.org/ (2014-2024)
18. What's New in Python 3.11, "Faster CPython" (10-60% faster, 1.25x average). https://docs.python.org/3/whatsnew/3.11.html (2022)
19. The Register, "Microsoft winnows: Layoffs hit software engineers hard" and Mike Droettboom's LinkedIn post on the cancellation of Faster CPython funding. https://www.theregister.com/2025/05/16/microsofts_axe_software_developers (May 2025)
20. Modular, "Mojo is now open source" (compiler under Apache 2.0 with LLVM exceptions). https://modular.com/blog/mojo-open-source (18 Aug 2026)
21. Mojo roadmap, "may or may not evolve into a full superset of Python". https://mojolang.org/docs/roadmap (2026)
22. Python FAQ, "Why are default values shared between objects?". https://docs.python.org/3/faq/programming.html (ongoing)
23. Guido van Rossum, "[python-committers] Transfer of power". https://mail.python.org/pipermail/python-committers/2018-July/005664.html (12 July 2018)
24. PEP 492 (async/await), PEP 654 (exception groups), PEP 734 (multiple interpreters), PEP 765 (control flow in finally), PEP 750 (t-strings). https://peps.python.org/ (2015-2025)
25. Bob Nystrom, "What Color is Your Function?". https://journal.stuffwithstuff.com/2015/02/01/what-color-is-your-function/ (2015)
