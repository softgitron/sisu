# Testing

## The question

How tests are declared, found and run; what a failing assertion tells you; how
setup, parameters, generated inputs, fuzzing, snapshots and doc examples are
handled; how tests stay isolated and non-flaky; what coverage, mutation and
benchmark tooling buys; and which of these belong in the language and its
toolchain rather than a framework. Givens: indentation syntax, explicit
`return`, errors as values with `?`, panics for bugs, capabilities instead of
ambient authority, compile-time evaluation instead of macros, and a toolchain
that ships tests, doc tests, property tests, snapshots and benchmarks (D-029,
D-041).

## Approaches found

| Approach | Languages | Buys | Costs | Evidence |
|---|---|---|---|---|
| xUnit classes: `setUp`/`tearDown`, `assertEquals`, discovery by reflection | SUnit (1994), JUnit (1997), JUnit 5 (2017), XCTest, unittest, NUnit | Portable idea; IDE integration; the lineage every other row reacts to | Inheritance for fixtures; one assertion method per comparison; naming conventions instead of declarations | [smalltalk.md](../languages/smalltalk.md), [java.md](../languages/java.md) |
| Naming convention, plain `assert`, rewritten assertions | pytest (`def test_*`, `conftest.py` fixtures), Go (`_test.go`, `TestXxx(t *testing.T)`) | Zero ceremony; pytest shows every sub-expression value; `go test` is the one `go` subcommand nobody needs docs for | Convention is invisible to the compiler; Go has no assertion library at all | [python.md](../languages/python.md), [go.md](../languages/go.md) |
| Tests as declarations compiled only under `test` | D `unittest { }` (runs before `main`, `-cov`), Zig `test "name"` (leak-checking allocator), Rust `#[test]` + `tests/` | Tests next to code, no framework; doc examples double as tests (D, Zig, Rust) | Rust has no fixtures and compiles one binary per integration file and (pre-2024) per doctest | [d.md](../languages/d.md), [zig.md](../languages/zig.md), [rust.md](../languages/rust.md) |
| `describe`/`it`/`setup` DSLs | ExUnit (`async: true`, a process per test), Jest/Vitest, RSpec, kotest | Readable grouping; per-process isolation on the BEAM; snapshots and mocks built in (Jest) | Deep nesting; ExUnit forbids nested `describe` on purpose | [erlang-elixir.md](../languages/erlang-elixir.md), [javascript.md](../languages/javascript.md), [ruby.md](../languages/ruby.md) |
| Attribute + expression macro: `@Test`, `#expect`, traits, `arguments:` | Swift Testing (2024; default in Xcode 16; parallel by default) | Each argument reported separately; `#expect` captures sub-values; conditions and time limits as traits | Needs macros; XCTest kept for UI and performance tests | [swift.md](../languages/swift.md) |
| Property-based testing | QuickCheck (2000), Hypothesis, proptest/quickcheck, fast-check, StreamData | Finds the cases nobody wrote; shrinking; Hypothesis replays saved failures first | Generators and judging a property are the reported hard parts; 4 % use in Python versus pytest's 53 % | [haskell.md](../languages/haskell.md), [python.md](../languages/python.md) |
| Fuzzing in the toolchain | Go (`FuzzXxx(*testing.F)`, 1.18), cargo-fuzz/libFuzzer, AFL++, OSS-Fuzz | A fuzz target is a test with a corpus; OSS-Fuzz: 13,000 vulnerabilities, 50,000 bugs, 1,000 projects (May 2025) | Memory and cache growth; needs checked builds to turn crashes into findings | [go.md](../languages/go.md), [rust.md](../languages/rust.md) |

## What the evidence says

1. **Zero-configuration discovery beats frameworks, and built in beats third
   party.** D's designer lists built-in unit tests among the hits ("built-in
   wins over third party tool") and Zig copied `test` blocks directly
   ([d.md](../languages/d.md),
   [../evidence/designer-retrospectives.md](../evidence/designer-retrospectives.md));
   Go's 2025 survey found `go test` the one subcommand for which fewer than
   15 % "often needed to review documentation" [23]; Elixir's built-in `mix
   format` reaches 94.9 % use
   ([erlang-elixir.md](../languages/erlang-elixir.md)). Python moved from the
   standard `unittest` to the third-party pytest (53 % versus 23 % in 2024)
   because `def test_*` plus plain `assert` is less ceremony [24]. The xUnit
   lineage from SUnit to JUnit 5 works everywhere, and every later design
   strips its class scaffolding ([smalltalk.md](../languages/smalltalk.md)).
2. **Rich assertion output is copied by every modern design, and it always
   needs the compiler.** pytest rewrites `assert` to record sub-expression
   values (`assert 3 == 4` / `where 3 = f()`) [11]; Spock's power asserts, written by Peter Niederwieser and adopted into Groovy 1.7 (December
   2009), print every intermediate value [25]; Swift Testing's `#expect` "captures the evaluated
   values so you can quickly understand what went wrong" [8]; ExUnit's `assert`
   prints expression and values
   ([erlang-elixir.md](../languages/erlang-elixir.md)); Rust's `assert_eq!`
   prints both sides (`pretty_assertions` adds the diff); Zig's `expectEqual`
   exists because `expect(a == b)` shows no values [12]. The evidence is
   imitation (class 5), and the mechanism is an AST rewrite, a macro or a
   build-time transform (power-assert-js is a Babel plugin; AssertJ and Truth
   get part of the effect from fluent APIs alone): without macros, `assert`
   must be a compiler intrinsic or compile-time code must see the expression
   tree.
3. **Fixtures: injection by parameter beat inheritance.** pytest fixtures are
   requested by argument name, have scopes (function, module, session) and
   compose ([python.md](../languages/python.md)). Go added `t.Cleanup` (1.14),
   `t.TempDir` (1.15) and `t.Setenv` (1.17) as the minimal mechanism [10];
   Zig's `std.testing.allocator` fails a leaking test [12]; ExUnit runs `setup`
   in the test's own process [13]; Rust has nothing and `rstest` fills the gap
   ([rust.md](../languages/rust.md)). With RAII and capability values a fixture
   is a value whose destructor is the teardown; only "share one expensive value
   across tests" (pytest scopes, `setup_all`) remains.
4. **Parametrised tests want declared arguments.** Go's table-driven tests are
   an idiom with `t.Run` subtests; `pytest.mark.parametrize`, JUnit 5
   `@ParameterizedTest` (5.0.0, September 2017) [26], Swift Testing
   `arguments:` [8] and ExUnit's parameterised tests (1.18) [13] let the runner
   report, filter, rerun and (Swift) parallelise one case at a time, which
   needs declared parameters rather than a loop.
5. **Property-based testing works, is cheap when built in, and is under-used
   when it is not.** QuickCheck (2000) is Haskell's most-copied library
   ([haskell.md](../languages/haskell.md)); Hypothesis (JOSS 2019) integrates
   shrinking and stores failing examples in `.hypothesis/examples`, replaying
   them first on the next run [14][15]. Adoption: Hypothesis 4 % against pytest
   53 % in Python 2024 [24]. The ICSE 2024 interviews with 30 Jane Street
   engineers found PBT strongest for "complex code", used in "a relatively
   small number of high-leverage idioms", and weakest in writing generators and
   judging a property's effectiveness [16].
6. **Fuzzing is property testing with a coverage-guided generator, and Go
   unified the two.** Go 1.18 (15 March 2022) added `FuzzXxx(*testing.F)`: seed
   inputs run as ordinary tests, `-fuzz` explores, failing inputs are written
   to `testdata/fuzz/<Name>` so the regression is kept [9][10]. cargo-fuzz
   wraps libFuzzer; AFL++ (WOOT 2020) is the reference mutational fuzzer [19];
   OSS-Fuzz reports "over 13,000 vulnerabilities and 50,000 bugs across 1,000
   projects" as of May 2025 [18]. In a checked language fuzzing finds panics
   and contract violations, so contracts must stay on in fuzz builds.
7. **Snapshot testing is popular and its misuse is documented by its authors.**
   Jest 14 (July 2016) introduced `toMatchSnapshot` with React [20]; `insta` is
   the Rust equivalent and Verify the .NET one. Jest's guidance: "fight against
   the habit of regenerating snapshots when test suites fail instead of
   examining the root causes", keep them deterministic and short, and they are
   "not to replace existing unit tests" [21].
8. **Doctests are used where they are free and feed the docs.** Rust's doc
   tests, D's documented `unittest` blocks (which become the Examples section),
   Zig's identifier-named tests and Elixir's `doctest` all run under the
   default test command and render into documentation [12][22]; Python's
   `doctest` is a separate module with fragile output matching, at 6 % use
   [24]. The cost shows in Rust, which compiled every doctest as its own binary
   until the 2024 edition merged them, a fix that only worked from 1.85.1
   (March 2025) after a silent fallback bug [27].
9. **The mocking debate resolves to fakes and injected dependencies.** Fowler's
   classical-versus-mockist distinction (2004, revised 2007) states the cost:
   mockist tests are coupled to implementation and fragile under refactoring
   [28]. Google's guidance prefers hand-written fakes and says "don't mock
   types you don't own" [29]; Go's official `gomock` was archived in June 2023
   and survives as an Uber fork [30]; `unittest.mock` is at 11 % in Python
   [24]. Every mocking framework replaces a dependency obtained ambiently; with
   capabilities as parameters the replacement is an ordinary value (Zig's
   `std.testing.io`, Go's `io.Reader`; [zig.md](../languages/zig.md),
   [capabilities-and-effects.md](capabilities-and-effects.md)). What remains is
   writing the fakes, where generating a recording fake from an interface by
   reflection still earns its place.
10. **Parallel by default is right; shared state is the price.** Rust runs
    tests on threads by default and the Book warns that the working directory,
    environment and files are shared [31]; Swift Testing is parallel by default
    [8]; Go's `t.Parallel` is opt-in, and Uber attributes 14 % of its fixed
    data races to parallel tests with shared fixtures
    ([go.md](../languages/go.md)); Go 1.25 makes `AllocsPerRun` panic under
    parallel tests as "inherently flaky" [32]. ExUnit defaults to `async:
    false` and isolates each test in a fresh process [13]; pytest needs
    `xdist`. The Rust/ExUnit difference is whether the language can isolate;
    capabilities give each Sisu test its own file system, clock and environment
    by construction.
11. **Flakiness has three known causes and capabilities address all of them.**
    Luo et al. (FSE 2014; 201 fixing commits, 51 projects): async wait 45 %,
    concurrency 20 %, test-order dependency 12 %; 78 % of flaky tests were
    flaky when first written; 54 % of async-wait flakes were fixed with an
    explicit wait and 74 % of order dependencies by resetting shared state [1].
    A 2025 study of 53 fixed flaky tests in Rust projects found the same order:
    async waiting 33.9 %, concurrency 24.5 % [2]. Google (2016): "about 1.5 %
    of all test runs reporting a flaky result", "almost 16 % of our tests have
    some level of flakiness", "about 84 % of the transitions we observe from
    pass to fail involve a flaky test" [3]. The mechanism that works is a
    virtual clock: Go's `testing/synctest` (1.25) runs a test in a "bubble"
    whose fake clock advances only when every goroutine is blocked [32]. In
    Sisu the clock, randomness, network and file system are parameters, so the
    fake clock is the test default, not a special package. The strongest
    evidence that injected time, randomness and I/O make whole systems
    deterministically testable is deterministic simulation testing:
    FoundationDB's simulator, TigerBeetle's VOPR and Antithesis all depend on
    exactly that injection [42]. Rust projects reach for `cargo-nextest`
    (process per test, retries, JSON output) for the isolation the language
    does not give them [43]. Other flaky-test studies find different mixes
    (Gruber et al. 2021: Python flakiness dominated by order dependency and
    infrastructure), so the shares are indicative, not universal.
12. **Coverage is a map of untested code, not a quality metric.** Inozemtseva
    and Holmes (ICSE 2014; 31,000 generated suites, five Java systems, mutation
    score as ground truth): with suite size controlled the
    coverage-effectiveness correlation is "low to moderate", stronger criteria
    add nothing, and coverage "should not be used as a quality target"; ICSE
    Most Influential Paper 2024 [4]. Google's own report on coverage adoption
    (Ivanković et al., FSE 2019) qualifies rather than contradicts this: coverage
    is used as a map in review, not as a gate [44]. The tooling is settled: Go `-cover` (1.2,
    2013), D `-cov`, llvm-cov, `coverage.py`, `cargo-llvm-cov` [33][22]; 59 %
    of developers with unit tests use coverage metrics (JetBrains 2023) [34].
13. **Mutation testing works at scale only when incremental and shown in
    review.** Google's 2018 report ran diff-based mutation analysis for 6,000
    engineers and 13,000 code authors on about 30 % of diffs with statement
    coverage, suppressing "arid" nodes such as logging [5]; the 2021 TSE paper
    covers 24,000 developers on 1,000 projects and produces "orders of magnitude
    fewer mutants" [6]; the finding that "developers using mutation testing
    write more tests, and actively improve their test suites" is from the same
    authors' ICSE 2021 study [39], and the coupling of mutants to real faults
    is Just et al., FSE 2014 [40]. PIT, Stryker, mutmut and
    cargo-mutants are all third party; the enabler is a compiler that emits a
    mutant and reruns only the tests covering that line.
14. **Benchmarks need the compiler's cooperation.** JMH's samples catalogue
    what a JIT does to a naive loop: dead-code elimination, constant folding,
    loop hoisting, hence `Blackhole` and forking [35]. Go's `testing.B` gained
    `b.Loop` (1.24) so the harness, not the author, controls iteration and
    timing [10]. Rust's `#[bench]` has been unstable since its tracking issue
    opened on 4 November 2015; a 2023 request to stabilise or deprecate it is
    unanswered and `criterion` is the de facto harness [36]. Zig's standard
    library has no benchmark harness [unverified]. An optimiser-respected
    `black_box` and statistics rather than a single mean are the two
    non-negotiables.
15. **Machine-readable output is a day-one requirement.** TAP dates from Perl
    1.0's `t/TEST` (1988) [37]; JUnit XML is the unspecified CI de facto; Go
    added `go test -json` and `test2json` in 1.10 (2018) "to produce a
    machine-readable JSON-formatted description of test execution" and `T.Attr`
    in 1.25 [9][32]. Go 1.10 also cached test results keyed on the files and
    environment a test consulted [9]; with capabilities Sisu knows that set
    exactly.
16. **Contracts turn every test into many tests.** Ada 2012's `Pre`, `Post` and
    type invariants are checked in test builds and proven by SPARK; D's
    `in`/`out`/`invariant` and language `assert` run under `-unittest`
    ([ada-spark.md](../languages/ada-spark.md), [d.md](../languages/d.md),
    [contracts-and-verification.md](contracts-and-verification.md)).
17. **TDD evidence is mixed and does not justify a mandate.** Rafique and
    Mišić's meta-analysis (TSE 2013; 27 primary studies) found that "TDD has
    a small positive effect on quality but little to no discernible effect on
    productivity", with both the quality gain and the productivity drop much
    larger in industrial than in academic studies [7]; Karac and Turhan's 2018
    review reaches the same "we do not really know" conclusion [41]; Fucci et al. (TSE 2017; 82
    observations, 39 professionals) found test-first sequencing had "no
    important influence" while granularity and uniformity of cycles did [17].
    The toolchain's job is to make small cycles cheap.
18. **Test compile time is the hidden cost of compiled tests.** Rust's 2025
    compiler-performance survey (3,700 responses): 55 % wait more than ten
    seconds for a rebuild, about 45 % of lapsed users cite compile times, and
    incremental rebuilds are the top complaint [38]; `cargo test` builds a test
    binary per integration file and per crate, and doctests were one binary
    each until 1.85.1 [27]
    ([tooling-and-compile-speed.md](tooling-and-compile-speed.md)). Go, with a
    fast compiler and a test cache, records no such complaint; Sisu's
    interpreter mode is the lever.
19. **What developers say they want.** JetBrains 2023: 80 % call testing
    integral, 83 % of those with unit tests write them, 58 % of testers
    automate, 84 % of automation targets APIs [34]. Python 2024: pytest 53 %,
    unittest 23 %, mock 11 %, doctest 6 %, Hypothesis 4 %, none 36 % [24]. The
    Rust surveys carry no testing item, weak evidence that `cargo test` is not
    a pain point ([../evidence/surveys.md](../evidence/surveys.md)).

## Recommendation for Sisu

Refines D-041 and D-029 ([proposal/10](../../proposal/10-tooling.md)).

- **Discovery.** `test "name":` blocks as declarations in the module they test,
  compiled only by `sisu test` (D, Zig); a `tests/` directory for black-box
  tests (Rust); no naming convention, no classes; one level of `suite "name":`
  grouping; tags to select and skip.
- **Assertions.** `assert` is a compiler intrinsic that captures sub-expression
  values and prints them pytest-style, with a structural diff for values of the
  same type; `assert_eq` adds a two-sided diff. Capture uses compile-time
  reflection over the expression tree, the one place Sisu needs what others get
  from macros. `expect` is the non-fatal form; a panic fails the test with its
  error return trace.
- **Fixtures.** No fixture framework: a fixture is a value with a destructor or
  a capability fake. A `fixture` function with a scope (`test`, `module`,
  `run`) may be requested by parameter name (pytest, typed). Temporary
  directories, environment and clock come from the test's capability set
  (`fs.temp_dir()`, a scoped `env` fake).
- **Parametrisation.** `test "name" for case in table:` declares the parameter
  list so the runner reports, filters and parallelises each case.
- **Property tests and fuzzing.** `check(any[T])` in the standard library with
  derived generators, integrated shrinking and a failure database replayed
  first (Hypothesis); under `--fuzz` the same `check` is coverage-guided and
  saves failing inputs under `tests/corpus/<name>` (Go). Contracts stay on.
- **Snapshots.** `assert_snapshot(value)` writes `.snap` files next to the
  test, fails with a diff, updates only with `sisu test --update`, and warns
  above a size threshold.
- **Doctests.** Fenced code in `///` comments compiles as one merged test
  binary per module (Rust 2024) and renders as the Examples section (D).
- **Isolation.** Parallel by default; each test gets its own capability set:
  in-memory `Fs`, a `Clock` that advances when all tasks block (Go `synctest`),
  seeded `Random`, no `Net` unless requested. A test that asks for the real
  `System` is `serial`. Order is randomised, seed printed.
- **Coverage and mutation.** `sisu test --cover` from the compiler (Go, D),
  reported as uncovered regions, never a percentage gate; `--mutate` for a
  changed region only, rerunning the tests that cover the mutated line, with
  arid-node suppression.
- **Benchmarks and output.** `bench "name":` with a harness-controlled loop, a
  compiler-respected `black_box` and confidence intervals (criterion), not run
  by `sisu test`; a JSON event stream from day one (Go `-json`), TAP and JUnit
  XML as `--format`; a test cache keyed on the capabilities a test touched.
- **Capabilities replace mocks.** No mocking framework; every standard
  capability ships an in-memory fake with a call log, and `sisu test` can
  derive a recording fake from any interface via compile-time reflection.
  [proposal/16](../../proposal/16-validation-plan.md) should time test-writing
  with fakes against a mocking framework.

## Open questions

- Whether a name-injected `fixture` is needed once capabilities and RAII cover
  setup and teardown; try without it first.
- The compile-time mechanism for `assert` to capture sub-expressions without
  macros, and whether user code can define equally powerful assertions.
- How a test declares the capabilities it needs so the compiler can pick
  parallel versus serial and the cache key without a signature burden (Q-17).
- Whether derived `any[T]` generators should respect contracts (preconditions
  as filters), the generator problem the Jane Street study names as PBT's main
  obstacle.

## Sources

1. Luo, Hariri, Eloussi, Marinov, "An Empirical Analysis of Flaky Tests", FSE 2014. https://huang.isis.vanderbilt.edu/cs8395/paper/flakytest.pdf (2014)
2. "A Preliminary Study of Fixed Flaky Tests in Rust Projects on GitHub", arXiv 2502.02760. https://arxiv.org/abs/2502.02760 (2025)
3. Micco, "Flaky Tests at Google and How We Mitigate Them", Google Testing Blog. https://testing.googleblog.com/2016/05/flaky-tests-at-google-and-how-we.html (27 May 2016)
4. Inozemtseva, Holmes, "Coverage Is Not Strongly Correlated with Test Suite Effectiveness", ICSE 2014. https://cs.ubc.ca/~rtholmes/papers/icse_2014_inozemtseva.pdf (2014); award https://www.cs.ubc.ca/award/2024/04/icse-most-influential-paper-award (2024)
5. Petrović, Ivanković, "State of Mutation Testing at Google", ICSE-SEIP 2018. https://research.google/pubs/state-of-mutation-testing-at-google/ (2018)
6. Petrović, Ivanković, Fraser, Just, "Practical Mutation Testing at Scale: A view from Google", IEEE TSE. https://arxiv.org/abs/2102.11378 (2021)
7. Rafique, Mišić, "The Effects of Test-Driven Development on External Quality and Productivity: A Meta-Analysis", IEEE TSE 39(6). https://doi.org/10.1109/TSE.2012.28 (2013)
8. swiftlang/swift-testing README (Swift 6 toolchain, Xcode 16). https://github.com/swiftlang/swift-testing (2024-2026)
9. Go release notes: 1.18 fuzzing https://go.dev/doc/go1.18 (March 2022); 1.10 `-json` and test caching https://go.dev/doc/go1.10 (February 2018); release dates https://go.dev/doc/devel/release
10. Go `testing` package (`Cleanup` go1.14, `TempDir` go1.15, `Setenv` go1.17, `F` go1.18, `B.Loop` go1.24). https://pkg.go.dev/testing (2026)
11. pytest, "How to write and report assertions in tests". https://docs.pytest.org/en/stable/how-to/assert.html (2026)
12. Zig language reference, "Zig Test". https://ziglang.org/documentation/master/#Zig-Test (2026)
13. ExUnit.Case documentation. https://hexdocs.pm/ex_unit/ExUnit.Case.html (2026)
14. MacIver, Hatfield-Dodds et al., "Hypothesis: A new approach to property-based testing", JOSS 4(43). https://joss.theoj.org/papers/10.21105/joss.01891 (2019)
15. Hypothesis documentation, "The Hypothesis Example Database". https://hypothesis.readthedocs.io/en/latest/database.html (2026)
16. Goldstein, Cutler, Dickstein, Pierce, Head, "Property-Based Testing in Practice", ICSE 2024. https://cis.upenn.edu/~bcpierce/papers/icse24-pbt-in-practice (2024)
17. Fucci et al., "A Dissection of the Test-Driven Development Process: Does It Really Matter to Test-First or to Test-Last?", IEEE TSE 43(7). https://arxiv.org/abs/1611.05994 (2017)
18. google/oss-fuzz README, "Trophies". https://github.com/google/oss-fuzz (May 2025 figures)
19. Fioraldi, Maier, Eißfeldt, Heuse, "AFL++: Combining Incremental Steps of Fuzzing Research", WOOT 2020. https://www.usenix.org/conference/woot20/presentation/fioraldi (2020)
20. "Jest 14.0: React Tree Snapshot Testing". https://jestjs.io/blog/2016/07/27/jest-14 (27 July 2016)
21. Jest documentation, "Snapshot Testing" (Best Practices, FAQ). https://jestjs.io/docs/snapshot-testing (2026)
22. D Language Specification, "Unit Tests". https://dlang.org/spec/unittest.html (2026)
23. "Results from the 2025 Go Developer Survey". https://go.dev/blog/survey2025 (January 2026)
24. PSF/JetBrains Python Developers Survey 2024, "Testing". https://lp.jetbrains.com/python-developers-survey-2024/ (2025)
25. Groovy 1.7 release announcement https://spring.io/blog/2009/12/22/groovy-1-7-released (22 December 2009); `powerassert` package, Groovy 1.7.0 API https://docs.groovy-lang.org/1.7.0/html/gapi/org/codehaus/groovy/transform/powerassert/package-summary.html
26. JUnit 5.0.0 release (10 September; the year 2017 is from the release history, not shown on the page). https://github.com/junit-team/junit-framework/releases/tag/r5.0.0
27. Rust 1.85.1 release post (combined doctests fix) https://blog.rust-lang.org/2025/03/18/Rust-1.85.1/ (2025); Edition Guide, "Rustdoc combined tests" https://doc.rust-lang.org/edition-guide/rust-2024/rustdoc-doctests.html
28. Fowler, "Mocks Aren't Stubs". https://martinfowler.com/articles/mocksArentStubs.html (2004, revised 2007)
29. Google Testing Blog, "Know Your Test Doubles" https://testing.googleblog.com/2013/07/testing-on-toilet-know-your-test-doubles.html (2013); "Don't Mock Types You Don't Own" https://testing.googleblog.com/2020/07/testing-on-toilet-dont-mock-types-you.html (2020)
30. golang/mock, archived 27 June 2023; maintained fork go.uber.org/mock. https://github.com/golang/mock
31. The Rust Programming Language, "Controlling How Tests Are Run". https://doc.rust-lang.org/book/ch11-02-running-tests.html (2026)
32. Go 1.25 release notes (`testing/synctest`, `T.Attr`, `AllocsPerRun`). https://go.dev/doc/go1.25 (August 2025)
33. Go 1.2 release notes, "Test coverage". https://go.dev/doc/go1.2 (December 2013)
34. JetBrains, State of Developer Ecosystem 2023, "Testing". https://www.jetbrains.com/lp/devecosystem-2023/testing/ (2023)
35. OpenJDK JMH samples (`08_DeadCode`, `09_Blackholes`, `10_ConstantFold`, `11_Loops`, `12_Forking`). https://github.com/openjdk/jmh/tree/master/jmh-samples/src/main/java/org/openjdk/jmh/samples
36. rust-lang/rust issue 29553, tracking issue for `#[bench]` (opened 4 November 2015, still open). https://github.com/rust-lang/rust/issues/29553
37. Test Anything Protocol, "History". https://testanything.org/history.html (1988 origin)
38. "Rust compiler performance survey 2025 results". https://blog.rust-lang.org/2025/09/10/rust-compiler-performance-survey-2025-results (September 2025)
39. Petrović, Ivanković, Fraser, Just, "Does mutation testing improve testing practices?", ICSE 2021. https://arxiv.org/abs/2103.07189 (2021)
40. Just, Jalali, Inozemtseva, Ernst, Holmes, Fraser, "Are Mutants a Valid Substitute for Real Faults in Software Testing?", FSE 2014. https://doi.org/10.1145/2635868.2635929 (2014)
41. Karac, Turhan, "What Do We (Really) Know about Test-Driven Development?", IEEE Software 35(4). https://doi.org/10.1109/MS.2018.2801554 (2018)
42. FoundationDB, "Simulation and Testing" (apple.github.io/foundationdb/testing.html); TigerBeetle, "VOPR" (docs.tigerbeetle.com); Antithesis, "How Antithesis works" (antithesis.com/docs) (2026)
43. cargo-nextest documentation. https://nexte.st/ (2026)
44. Ivanković, Petrović, Just, Fraser, "Code Coverage at Google", ESEC/FSE 2019. https://doi.org/10.1145/3338906.3340459 (2019)
