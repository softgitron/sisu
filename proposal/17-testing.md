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

A test can also end a third way. When it reaches a hole, `todo("name")`,
the interpreter stops with the hole's closure (the `Debug` rendering of
every binding in scope), and the runner reports the test as blocked by
that hole, neither passed nor failed (D-061). A suite can therefore be
written before the code it tests, and its report says which hole each test
is waiting on; `sisu check --holes` adds the values each test reached the
hole with ([10](10-tooling.md)). Release builds and `sisu publish` reject
holes, so blocked is never a shipped state.

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

## Simulation and replay

Schedules are controlled like time. In the interpreter the scheduler owns
every yield point and is seeded, so `sisu test --explore N` reruns a test
under N schedules and `--schedule-seed` reruns one of them
([18](18-next-steps.md), Phase 6). `sisu test --sim` extends this from one
test to a whole system. It runs the test in `sim`, a fourth runtime
profile beside the three of [12](12-interop-and-targets.md), in which
`System` is entirely simulated: the test clock above, an in-process `Net`
with latency, loss and partitions, an `Fs` with a pessimistic durability
model in which unsynced writes vanish at a simulated crash, and a seeded
`Rand`. Several nodes, each with its own `System` and its own module
statics, run in one process, and each node's standard streams, ambient
everywhere else, are captured. The scheduler is seeded and pluggable:
random; PCT, whose random priorities find a bug of a given depth with a
known minimum probability per run; and bounded exhaustive, with state
hashing and partial-order reduction.

```sisu
test "replicas converge under partitions, torn writes and crashes":
    var sim = Sim(scheduler=Pct(depth=3), faults=Faults.storage() + Faults.network(drop=0.02))
    for i in range(3):
        sim.node("r{i}", sys -> replica.main(sys))                        # the production main
    sim.every(Duration.minutes(5), own s -> s.crash(s.pick(s.nodes())))   # unsynced writes are lost
    sim.always(own s -> len(lease_holders(s.nodes())) <= 1)               # checked after every step
    sim.run_for(Duration.hours(6))                                        # seconds of real time
    assert linearizable(sim.history())
```

Faults come from types. At each capability call the simulator may return
any error in that method's declared error set and nothing else, so it
never injects a failure the code could not have handled, which is the main
source of noise in fault injection. `sim.always` states an invariant
checked after every step, and `sim.choose(range(3))` makes a value a
choice point that the exhaustive mode enumerates; with both, the explorer
is a P-style model checker whose model is the program itself
([`logic-constraint-and-rewriting.md`](../research/unconventional/logic-constraint-and-rewriting.md),
idea 1). The language-level part is a guarantee: a `sim` build fails to
compile if reachable code could escape the simulation, which only an
`extern` call or `unsafe` code in a `[trust]`ed package with no simulation
model can do.

```
error[E0971]: this `sim` build reaches `sqlite.exec`, an `extern` call with no simulation model
```

FoundationDB ("unlikely that we would have been able to build
FoundationDB without this technology") and TigerBeetle's VOPR keep their
simulations by discipline; Shuttle, Coyote and Go's `synctest` are
libraries; Antithesis and Hermit determinise programs from outside the
process; P and S3's ShardStore (16 issues kept out of production) are the
model-checking precedents
([`systems-and-determinism.md`](../research/ideation/systems-and-determinism.md),
candidate 2). No compiler guarantees that a program cannot escape its
simulation. The risks are known. An `Fs` model kinder than Linux would
give false confidence, so failed `fsync`, rename and directory durability
are modelled; simulated time hides real-time performance bugs; and the
weak-memory behaviour of `Atomic` on the `full` profile needs a
loom-style mode of its own. The profile is prototyped in the experiment
across Phases 6 and 8 (Q-49; about four person-weeks), where it adds
faults and crashes to the 1000-schedule run that Phase 6 already
requires. A seeded-bug study judges it: of twenty known concurrency and
crash-consistency bugs from the ALICE and Jepsen catalogues, planted in
the corpus HTTP server and a small replicated log, `--sim` should find at
least fifteen within a fixed CPU budget, more than random schedules find
alone.

Every failing test prints a replay token holding the content hash of the
code, the toolchain version, the `Rand` and scheduler seeds and, for a
property test, the shrunk case. `sisu test --replay <token>` reproduces
the failure on any machine, because inside a test every source of
nondeterminism is a capability fake or the seeded scheduler. A test that
takes the real `System` can be neither simulated nor replayed, and its
token says so. Outside tests the same reproduction comes from the
capability log that `sisu run --record` writes and `sisu replay`
re-executes ([10](10-tooling.md)). The tokens and the log are prototyped
in Phase 8 (Q-49; about one person-week), where corpus triage needs them;
the target is that every corpus failure that does not hold the real
`System` reproduces from its token alone
([`ai-era-and-evolution.md`](../research/ideation/ai-era-and-evolution.md),
A8).

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

Beside `arbitrary[T]`, `exhaustive[T](max_len=n)` (leaning, D-070)
enumerates every value of `T` up to a small size, with refinement types
bounding the domains, so `check` can run every history of up to `n`
operations against a reference model:

```sisu
type Key = int where 0 <= self < 3          # the refinement bounds the domain

enum Op:
    Put(k: Key, v: bool)
    Delete(k: Key)

test "the store agrees with a dict on every short history":
    check(exhaustive[list[Op]](max_len=4)) do ops:      # 7,381 histories
        return apply_all(Store(), ops) == apply_all(DictModel(), ops)
```

This is S3's ShardStore method and Alloy's small-scope hypothesis, that
most bugs show up in a small scope, which SmallCheck brought to property
testing
([`logic-constraint-and-rewriting.md`](../research/unconventional/logic-constraint-and-rewriting.md),
idea 2).

`@refines(reference, tolerance=...)` (D-070) keeps a clear algorithm in
the source as the specification of an optimised one. `sisu test`
generates property tests that the optimised function matches its
reference on `arbitrary` inputs. The tolerance is needed only where
reassociation is intended, since floating point is otherwise strict
([03](03-types-and-generics.md)). It is Exo's reference equivalence
turned into a test
([`algorithm-schedule-separation.md`](../research/unconventional/algorithm-schedule-separation.md),
idea 2), and a later verifier could prove such pairs for the pure, affine
subset.

```sisu
pure def blur_ref(img: Image) -> Image: ...

@refines(blur_ref, tolerance=Ulps(2))      # sisu test checks blur against blur_ref
def blur(img: Image) -> Image: ...          # tiled, with its sums reordered for speed
```

Property tests are in the standard library because the research finds them
used an order of magnitude more when they need no dependency, and
under-used (4% of Python developers) when they do.

## Snapshots and transcripts

`assert_snapshot(value)` renders the value with `Debug` (or a named
renderer), stores it in a `.snap` file beside the test, and fails with a
diff when it changes.

A transcript test (D-070) snapshots a whole program. `sisu test` runs
every `tests/*.transcript` file: a `$` line runs the package's `main` with
those arguments against a fake `System` (an in-memory `Fs` loaded from a
fixture directory, a frozen `Clock`, a seeded `Rand` and no `Net`), `>`
lines are its standard input, and every other line is expected output. A
mismatch fails with a diff.

```
$ shop price
> apples
apples: 0.40
$ shop buy apples 2
ok: 1 left
$ shop buy pears 1
error: no pears in stock (exit 1)
```

With `sisu>` prompts the same format tests REPL sessions, so the sessions
in tutorials cannot silently go stale. Inform's "Test me with", doctest
sessions and Go's `Example` output comments are the precedents, and the
capability fakes remove their fragility
([`natural-language-and-literate.md`](../research/unconventional/natural-language-and-literate.md),
idea 2).

Snapshots and transcripts update the same way. `sisu test --update`
re-blesses them only after showing the diff; the runner warns above a size
threshold and refuses non-deterministic content (addresses, times, random
values) that the capability fakes have not pinned. A `dict` needs no
pinning, because it iterates in insertion order ([11](11-stdlib.md)).
Jest's documented misuse, regenerating snapshots instead of reading the
diff, is the thing the `--update` ceremony exists to slow down.

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
TAP and JUnit XML as `--format` options for CI systems. Every test that
runs ends passed, failed or blocked by a hole, and every failure carries
its replay token. Test results are cached keyed on the test's code and the
capabilities it touched (Go 1.10's test cache, made exact because the
compiler knows which capabilities each test received); `sisu test` after
an unrelated edit reruns nothing.

## Mocks

There is no mocking framework and no need for one. Every mocking framework
exists to replace a dependency that code obtained ambiently; in Sisu every
such dependency is a parameter, so the replacement is an ordinary value.
Every standard capability ships an in-memory fake with a call log
(`Fs.in_memory`, `Clock.frozen_at`, `http.FakeNet().respond_to(...)`,
`Env.fake`, `Rand.seeded`). For user-defined traits, `sisu test` can derive
a recording fake by compile-time reflection (`Fake[MyTrait]`), which
records calls and returns configured values. The same reflection builds
`Wrap[T]` in `std.cap` (leaning, D-075; [11](11-stdlib.md)), which
intercepts the calls to a real capability instead of replacing it, so a
test can watch real traffic through the interception point that tracing
and recording use. Google's "don't mock types you don't own" and Fowler's
classical-versus-mockist analysis are the evidence that hand-written fakes
over injected dependencies age better than expectation-based mocks; the
validation plan times test-writing with fakes against a mocking framework
([18-next-steps.md](18-next-steps.md), bet 2).

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
