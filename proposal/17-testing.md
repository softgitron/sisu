# 17 - Testing

Testing is part of the language and the toolchain, not a framework. The
evidence is in [`testing.md`](../research/topics/testing.md); the summary
is that every modern design strips xUnit's class scaffolding, that rich
assertion output always needs the compiler, that fixtures by injection beat
inheritance, and that flakiness has three known causes which capabilities
address directly. Decision D-057.

## Discovery and organisation

```sisu
test "parse_int accepts digits":
    assert parse_int("42") == Ok(42)

suite "parse_int":
    @tag("slow")
    test "rejects empty input":
        assert parse_int("") == Err(UnexpectedEnd)

    test "rejects bad digits" for text, at in [("4x", 1), ("x4", 0)]:
        assert parse_int(text) == Err(BadDigit(at=at, found=text.chars().nth(at).unwrap()))
```

- A `test "name":` block is a declaration in the module it tests, compiled
  only by `sisu test` (D, Zig, Rust). It has access to the module's private
  items. A `tests/` directory holds black-box tests that see only the
  public API (Rust).
- No naming convention, no classes, no registration: the compiler knows
  every test. `suite` groups one level deep (ExUnit forbids deeper nesting
  on purpose). `@tag` selects and skips (`sisu test --tag slow`,
  `--skip-tag slow`).
- `test "name" for pattern in iterable:` declares a parametrised test; each
  case is reported, filtered, rerun and parallelised separately (pytest
  `parametrize`, JUnit 5, Swift Testing), which a loop inside a test cannot
  provide.
- Doc examples in `##` comments are compiled as one merged test binary per
  module and rendered as the Examples section (Rust 2024, D).

## Assertions

`assert expr` is a compiler intrinsic, not a function. When it fails it
prints the expression with the value of every sub-expression (pytest's
assertion rewriting, Groovy's power assert, Swift's `#expect`), and a
structural diff when two values of the same type are compared:

```
assertion failed at parser.sisu:42
    assert parse_int(text).ok() == Some(expected)
           │         │     │        │
           │         "4x"  None     Some(42)
           Err(BadDigit(at=1, found='x'))
```

`assert cond, "message"` adds a message with interpolation. `expect expr`
is the non-fatal form: the test continues and fails at the end. A panic
inside a test fails it with the error return trace. Contract checks are on
in tests, so every call inside a test also checks the preconditions,
postconditions and invariants of everything it calls.

The capture is the one place Sisu needs what other languages get from
macros: the compiler lowers `assert` to code that records each
sub-expression's value before combining them. User code gets the same
power through `std.test.Expectation`, which takes a compile-time
expression tree (reflection over the argument expression), so a library
can define `assert_approx` or `assert_json_eq` with the same output.

## Fixtures

There is no fixture framework. A fixture is a value: its constructor is the
setup, its `Drop` is the teardown, and it is passed to what needs it.
Capability fakes are the common case:

```sisu
test "config loads from a file":
    let fs = Fs.in_memory({"app.toml": "port = 8080"})
    assert load_config(fs, "app.toml").expect("").port == 8080
```

For an expensive value shared across tests, a `fixture` function with a
scope is requested by parameter name (pytest's model, typed):

```sisu
@fixture(scope="module")
def schema() -> Schema:
    return Schema.parse(SCHEMA_TEXT).expect("valid")

test "rejects unknown fields"(schema: Schema):
    assert schema.validate({"nope": 1}).is_err()
```

Temporary directories, environment and clock come from the test's
capability set (`fs.temp_dir()`, a scoped `Env` fake, the test clock),
which is why Go needed `t.TempDir`, `t.Setenv` and `t.Cleanup` and Sisu
does not.

## Isolation and parallelism

Tests run in parallel by default, in random order with the seed printed,
and each test receives its own capability set: an in-memory `Fs`, an `Env`
fake, a seeded `Rand`, no `Net` unless requested, and a `Clock` that
advances only when every task in the test is blocked. The last is Go's
`testing/synctest` idea made the default: time-dependent code runs
deterministically and instantly, and the "async wait" class that Luo et
al. found to be 45% of flaky tests cannot occur. A test that takes the
real `System` as a parameter runs alone.

Order dependency, the third flakiness cause, is impossible by construction:
no test can see another's file system, environment or clock, and the
module's own mutable statics are behind locks.

## Property-based testing and fuzzing

```sisu
test "sort is idempotent":
    check(arbitrary[list[int]]) do xs:
        var once = xs.copy()
        once.sort()
        var twice = once.copy()
        twice.sort()
        return once == twice
```

`check` is in `std.test`: generators are derived for every type (`arbitrary[T]`; with
`@derive(Arbitrary)` for custom shapes, and refinement predicates honoured
as filters), shrinking is integrated, and failing examples are saved to a
database and replayed first on the next run (Hypothesis). Under
`sisu test --fuzz` the same `check` becomes coverage-guided, runs until
stopped, and writes failing inputs to `tests/corpus/<test name>` where they
become ordinary regression cases (Go 1.18's unification of fuzz and test).
Contracts stay on during fuzzing, so a violated precondition in a callee is
a finding, not noise.

Property tests are in the standard library because the research finds them
used an order of magnitude more when they need no dependency, and
under-used (4% of Python developers) when they do.

## Snapshots

`assert_snapshot(value)` renders the value with `Debug` (or a named
renderer), stores it in a `.snap` file beside the test, and fails with a
diff when it changes. Updates require `sisu test --update` and show the
diff first; the runner warns above a size threshold and refuses
non-deterministic content (addresses, times, random values) that the
capability fakes have not pinned. Jest's documented misuse, regenerating
snapshots instead of reading the diff, is the thing the `--update`
ceremony exists to slow down.

## Coverage and mutation

`sisu test --cover` reports uncovered regions from the compiler's own
instrumentation (Go, D). It is a map of untested code and never a gate:
Inozemtseva and Holmes found coverage only weakly correlated with suite
effectiveness once suite size is controlled. `sisu test --mutate` runs
mutation testing on the changed region only, rerunning the tests that
cover each mutated line and suppressing arid nodes such as logging, which
is the form Google found usable at scale; it reports surviving mutants as
suggestions, in review, where developers were measured to act on them.

## Benchmarks

```sisu
bench "parse_int on a 9-digit number"(b: Bench):
    for _ in b.loop():
        black_box(parse_int("123456789"))
```

Benchmarks live beside tests but run only under `sisu bench`. The harness
controls iteration count and timing (Go 1.24's `b.Loop`), `black_box` is
respected by the optimiser, and results are reported as confidence
intervals with outlier detection (criterion), never a single mean. The
report format is stable JSON so that CI can track regressions.

## Output and caching

`sisu test` emits a JSON event stream from day one (Go's `-json`), with
TAP and JUnit XML as `--format` options for CI systems. Test results are
cached keyed on the test's code and the capabilities it touched (Go 1.10's
test cache, made exact because the compiler knows which capabilities each
test received); `sisu test` after an unrelated edit reruns nothing.

## Mocks

There is no mocking framework and no need for one. Every mocking framework
exists to replace a dependency that code obtained ambiently; in Sisu every
such dependency is a parameter, so the replacement is an ordinary value.
Every standard capability ships an in-memory fake with a call log
(`Fs.in_memory`, `Clock.frozen_at`, `http.FakeNet().respond_to(...)`,
`Env.fake`, `Rand.seeded`). For user-defined traits, `sisu test` can derive
a recording fake by compile-time reflection (`Fake[MyTrait]`), which
records calls and returns configured values. Google's "don't mock types you
don't own" and Fowler's classical-versus-mockist analysis are the evidence
that hand-written fakes over injected dependencies age better than
expectation-based mocks; the validation plan times test-writing with fakes
against a mocking framework ([18-next-steps.md](18-next-steps.md), bet 2).

## What is deliberately absent

xUnit lifecycle methods (`setUp`/`tearDown`), test classes, inheritance for
sharing fixtures, a global test registry, mocking frameworks, expectation
DSLs, nested `describe` beyond one level, a `--no-capture` dance (output is
captured per test and shown on failure), and any option that makes
coverage a gate.

## Open questions

- Whether `@fixture` is needed once capabilities and `Drop` cover setup
  and teardown; the plan tries without it first.
- How a test declares the capabilities it needs so that parallel-versus-
  serial and the cache key are known without a signature burden
  (`test "name"(sys: System)` is the current spelling).
- Whether derived generators should respect preconditions as filters by
  default, which can make generation slow, or only on request.

## Additions proposed in October 2026

[19-novel-features.md](19-novel-features.md) extends `std.test` (D-070):
`exhaustive[T](max_len=n)` beside `arbitrary[T]`, so `check` can run every
history up to a small size against a reference model; transcript tests
(`tests/*.transcript`, a `$` line runs `main` against a fake `System`,
`>` lines are standard input, other lines are expected output, `--update`
re-blesses after showing the diff); `@refines(reference, tolerance=...)`,
which generates property tests that an optimised function matches its
reference. Two prototypes are proposed for the experiment (Q-49): `sisu
test --sim`, a runtime profile in which `System` is entirely simulated,
faults are drawn only from each method's declared error set, the scheduler
is seeded and pluggable (random, PCT, bounded exhaustive) and a build fails
to compile if reachable code could escape the simulation; and a replay
token on every failing test (`sisu test --replay <token>`) holding the
content hash, toolchain version, seeds and shrunk case. A hole reached by a
test is a third outcome, "blocked" (D-061). Tests that take `System`
cannot be simulated or replayed, and the token says so.
