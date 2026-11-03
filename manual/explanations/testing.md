# Testing without mocks

Sisu has no mocking framework, and its tests do not need one. Code
reaches the world only through capabilities, so a test hands it fakes: a
file system in memory, a clock that moves when the test's tasks wait, a
seeded source of random numbers, a simulated network. And because every
source of chance in a test is one of those values or the seeded
scheduler, a failing test prints a token that reproduces the failure on
any machine. This page explains why testing takes that shape, what other
languages do instead, what it costs, and how its claims are tested.

**See also:** the tutorial's [chapter 19](../tutorial/19-testing.md)
(testing in depth), [chapter
20](../tutorial/20-concurrency.md#testing-under-many-schedules) (tests
under many schedules) and [chapter 22](../tutorial/22-web-service.md)
(simulation and replay); the reference's [R15,
Testing](../README.md#the-reference); the
[guides](../README.md#the-guides) on testing with the clock and the
network, finding a concurrency bug and reproducing a failure; the
proposal's [17, Testing](../../proposal/17-testing.md) and decisions
D-057 and D-070 in its [decision
log](../../proposal/15-decision-log.md); the research in
[testing.md](../../research/topics/testing.md).

## The problem: tests of code that touches the world

A function that reads the clock, a file or the network is hard to test
in most languages, because it reaches those things through globals:
`time.time()`, `open()`, a socket library. A test can let it touch the
real world, which makes the test slow, dependent on the machine and on
the order of the tests, and flaky. Or the test can replace the
dependency, and since the code did not receive it, replacing it means
patching it at run time with a mocking framework, configuring a
container for dependency injection, or adding a hook that exists only
for tests.

Both roads have documented costs. Martin Fowler's analysis of mocking
found that tests written with expectation-based mocks are coupled to the
implementation and break under refactoring; Google's guidance prefers
hand-written fakes and says "don't mock types you don't own"; Go's
official `gomock` was archived in June 2023
([testing.md](../../research/topics/testing.md), finding 9). And the
real world makes tests flaky in known ways. Luo and colleagues studied
201 commits that fixed flaky tests in 51 projects: 45% of the flakes
came from waiting on asynchronous work, 20% from concurrency and 12%
from tests that depended on each other's order. Google reported that
almost 16% of its tests showed some flakiness, and that about 84% of the
transitions it saw from pass to fail involved a flaky test (finding 11).

## Fakes are ordinary values

Every mocking framework exists to replace a dependency that code
obtained ambiently. In Sisu no code obtains one that way: a function
that reads files takes an `Fs`, one that reads the time takes a `Clock`
([Capabilities and the supply chain](capabilities.md)). So the
replacement is an ordinary value, passed where the real one would be
(D-057). Every standard capability comes with a fake: `Fs.in_memory`,
`Clock.frozen_at`, `Env.fake`, a seeded `Rand`, and `http.FakeNet` for a
network that answers as the test says. For a program's own traits,
`cap.Fake[T]` builds a fake that records its calls and returns what the
test configures, by compile-time reflection, which is the one job left
for which generating code earns its place. A fixture is a value too: its
constructor is the setup and its drop is the teardown, which is pytest's
injection by parameter, typed, rather than xUnit's setup methods
inherited from a base class (finding 3).

Each test also receives its own set of capabilities: a file system in
memory, a fake environment, a seeded source of random numbers, no
network unless it asks for one, and a clock that advances only when
every task in the test is waiting. That last idea is Go's
`testing/synctest`, made the default rather than a package to opt into.
Together they remove the known causes of flakiness by construction. A
test that waits for a timer finishes at once, with the clock where the
timer said, so the 45% of flakes that came from waiting cannot occur. No
test can see another's files, environment or clock, so order cannot
matter. And concurrency is handled by the scheduler, as the next section
shows.

## Schedules, simulation and replay

In the interpreter, the scheduler owns every point where a task can give
way, and it is seeded like everything else in a test.
`sisu test --explore N` runs each test under N schedules,
`--schedule-seed` reruns one of them, and other schedulers search more
cleverly: PCT, whose random priorities find a bug of a given depth with
a known minimum probability per run, and a bounded exhaustive search. A
race that a test meets once in a thousand runs on a real machine becomes
a seed that fails every time ([chapter
20](../tutorial/20-concurrency.md#testing-under-many-schedules) shows
one).

`sisu test --sim` takes the idea from one test to a whole system. It
runs the test in a fourth runtime profile, `sim`, where the entire
`System` is simulated: the test clock, a network inside the process with
latency, loss and partitions, a file system whose unsynced writes vanish
at a simulated crash, and seeded randomness; several nodes, each with
its own `System`, can run in one process. Faults come from types. At
each capability call the simulator may return any error in that method's
declared error set and nothing else, so it never injects a failure the
code could not have handled, which is the main source of noise in fault
injection elsewhere. This test passes on the plain test fakes, fails
under the simulator, and fails again from its token:

<!-- file: examples/explanation-testing/save.sisu -->
```sisu
# A note saved through the test's own file system, which the simulator
# may fail in any way that the error sets of its methods allow.
def save(fs: Fs, note: str) -> int:
    var tries = 1
    while fs.write_text("note.txt", note).is_err():
        tries += 1
    return tries

test "a saved note can be read back"(fs: Fs):
    assert save(fs, "sisu") >= 1
    assert fs.read_to_string("note.txt") == Ok("sisu")
```

<!-- output: examples/explanation-testing/save.console -->
```console
$ sisu test save.sisu
test "a saved note can be read back" ... ok
1 passed, 0 failed, 0 blocked
$ sisu test --sim --faults 0.7 save.sisu
test "a saved note can be read back" ... FAILED

test "a saved note can be read back" ... FAILED
panic: assertion failed: fs.read_to_string("note.txt") == Ok("sisu")
  left:  Err(Revoked)
  right: Ok("sisu")
  at test "a saved note can be read back" (save.sisu:11:5)
replay: sisu test --replay 1-fb87add2ea595989-0.0.1-t0-s0-r0-m700000 save.sisu

0 passed, 1 failed, 0 blocked
$ sisu test --replay 1-fb87add2ea595989-0.0.1-t0-s0-r0-m700000 save.sisu
test "a saved note can be read back" ... FAILED

test "a saved note can be read back" ... FAILED
panic: assertion failed: fs.read_to_string("note.txt") == Ok("sisu")
  left:  Err(Revoked)
  right: Ok("sisu")
  at test "a saved note can be read back" (save.sisu:11:5)
replay: sisu test --replay 1-fb87add2ea595989-0.0.1-t0-s0-r0-m700000 save.sisu

0 passed, 1 failed, 0 blocked
```

`save` retries a write until it succeeds, and the test assumed that the
read afterwards could not fail. Under the simulator the read failed with
`Revoked`, one of the variants of `IoError`, which the signature of
`read_to_string` declares, so the test met a failure that real code can
meet and that the test had not considered. The line that starts
`replay:` is for reproducing the failure: running it gives the same
failure on any machine with the same code and toolchain, as the
session's last command shows. It works because inside a test every
source of chance is a fake or the seeded scheduler. A test that takes
the real `System` runs alone and has no token; under `--sim`, the
`System` it receives is a simulated one. Outside tests,
`sisu run --record` writes a log of every capability call, and
`sisu replay` runs the program again from it.

The simulation comes with a guarantee that no other language gives: a
`sim` build fails to compile if reachable code could escape the
simulation, which only a foreign call or `unsafe` code in a trusted
package with no simulation model can do (D-070). FoundationDB's team
wrote that it was "unlikely that we would have been able to build
FoundationDB without" its simulator, and TigerBeetle's VOPR is in the
same tradition, but both keep their simulations by discipline: any code
that calls the clock or the network directly escapes them. Libraries
such as Shuttle and Coyote, and Go's `synctest`, cover what goes through
them, and Antithesis and Hermit make programs deterministic from outside
the process
([systems-and-determinism.md](../../research/ideation/systems-and-determinism.md),
candidate 2). In Sisu the escape is a compile error, because the only
way to the world is a capability.

## What it costs

- **The capability parameter.** Everything on this page is bought by the
  parameter that every function doing I/O carries, which [Capabilities
  and the supply chain](capabilities.md) weighs.
- **Fakes must be faithful.** A file system model kinder than Linux's
  gives false confidence, so failed `fsync`, rename and directory
  durability are modelled. Simulated time hides performance bugs that
  only real time shows. And the weak memory ordering of `Atomic` on real
  hardware needs a checking mode of its own, which the simulator does
  not give.
- **Faults are as broad as the error sets.** The simulator injects any
  declared error, so a set that declares more than can happen in
  practice yields faults that cannot happen. The `Revoked` in the
  session can occur only on a file system that someone revoked; the set
  is shared, and so the simulator offers it everywhere.
- **The real world still needs its own tests.** A test that takes the
  real `System` runs alone, against the real world, with no token that
  replays it.
- **A token belongs to its code.** A token names the code it was made
  from, so after any change it reproduces nothing; the failure has to be
  found again, by the seed or the search that found it first.
- **Most of this is a prototype.** The `sim` profile and replay tokens
  run in stage 0 as prototypes for the experiment, not as promises of
  version 1.

## What would change it

The claims on this page are measured in the experiment, with the
thresholds fixed in advance. The port of a Go service for the second bet
times ten pre-specified tests written with `std.cap`'s fakes against the
same tests written with the Go original's mocks; if the fakes are more
than 25% slower to write, they are redesigned
([20](../../proposal/20-pre-registration.md#23-bet-2-capability-parameters-at-scale-d-015-q-5)).
The simulator is judged by a seeded-bug study: twenty known concurrency
and crash-consistency bugs from the ALICE and Jepsen catalogues, planted
in the corpus's HTTP server and in a small replicated log, of which
`--sim` must find at least fifteen within a fixed CPU budget, and more
than random schedules alone find; otherwise the profile stays a
prototype and does not enter version 1 (D-070, Q-49). And replay is held
to a target: every failure in the corpus that does not hold the real
`System` reproduces from its token alone.

Two questions of the testing design stay open in
[17](../../proposal/17-testing.md#open-questions): whether a `@fixture`
form is needed at all once capabilities and drops cover setup and
teardown, and how a test declares the capabilities it needs without a
signature for each. Both are settled by writing the corpus's tests
without them first.
