# Capabilities

This page states the rule that a program reaches the world only through
values it is given: the root capabilities that `main` receives, what
each one can do, how a capability is narrowed, lent, revoked and
observed, the fakes that tests hand to code, what needs no capability,
and how the rule is enforced.

**See also:** the tutorial's [chapter 9](../tutorial/09-word-counter.md)
(a first capability) and [chapter 18](../tutorial/18-capabilities.md)
(capabilities); the proposal's [06,
"Capabilities"](../../proposal/06-concurrency-and-capabilities.md#capabilities);
the compiler's [architecture notes](../../compiler/docs/architecture.md),
"Capabilities"; [R16](sisu-command.md#sisu-audit) for `sisu audit
--caps`; [R17, Diagnostics](diagnostics.md#codes-and-their-ranges) for
the codes (E07xx). Each capability's methods are in the standard
library's reference.

Each section ends with a status, in the words that [R16](sisu-command.md#how-to-read-this-page)
explains. The rule is decided (D-015); lent and revocable capabilities
and interceptors lean (Q-51, D-075).

## The rule

A function can affect the world outside the process only through a value
it was given. There is no function that opens a file, connects a socket,
reads the clock, draws a random number, reads the environment or starts
a process without such a value (D-015). The name that another language
uses for one is E0200, whose note names the capability:

<!-- file: examples/reference-capabilities/no_global.sisu -->
```sisu
# There is no function that opens a file without a capability.
def main():
    print(open("menu.txt"))
```

<!-- output: examples/reference-capabilities/no_global.console -->
```console
$ sisu run no_global.sisu
error[E0200]: cannot find `open` in this scope
  --> no_global.sisu:3:11
  |
3 |     print(open("menu.txt"))
  |           ^^^^
  = note: a program reaches files through the capability `Fs`, which `def main(sys: System)` holds as `sys.fs`: `sys.fs.read_to_string("data.txt")`

error[D0006]: the program was not run: errors found (1)
```

So a function's parameters say what it can do: one that takes no
capability can only compute, whatever it calls.

**Status.** In stage 0 (D-015).

## The root capabilities

`main(sys: System)` receives the program's root capabilities, the fields
of `System`, which the runtime makes and nothing else can:

| Field | Type | Reaches |
|---|---|---|
| `sys.fs` | `Fs` | the file system |
| `sys.net` | `Net` | the network |
| `sys.clock` | `Clock` | the time, timers and sleeping |
| `sys.rand` | `Rand` | random numbers from the operating system |
| `sys.env` | `Env` | the program's arguments and environment variables |
| `sys.proc` | `Proc` | other processes |
| `sys.stdin` | `Stdin` | the standard input |

`main` takes nothing or one `System` (E0378). A test that declares `sys:
System` receives the real one and runs alone; any other test takes fresh
fakes ([Fakes](#fakes)). `System.assemble`, which makes one from
handles, is internal to the standard library (E0701).

<!-- file: examples/reference-capabilities/system.sisu -->
```sisu
# `main` receives the root capabilities in its `System`.
def price(fs: Fs, item: str) -> int ! IoError:
    let text = fs.read_to_string("menu.txt")?
    for line in text.lines():
        let name, cents = line.split_once(" ") else:
            continue
        if name == item:
            return Ok(cents.parse_int() ?? 0)
    return Ok(0)

def main(sys: System) -> ! IoError:
    for item in sys.env.args():
        print("{item}: {price(sys.fs, item)?} cents")
    print("the sauna: {sys.env.get("SAUNA_NAME") ?? "no name"}")
    return Ok(())
```

With `coffee` and `bun` as its arguments and a `menu.txt` beside it,
the program prints:

<!-- output: examples/reference-capabilities/system.out -->
```text
coffee: 350 cents
bun: 280 cents
the sauna: no name
```

**Stage 0 differs.** [06](../../proposal/06-concurrency-and-capabilities.md#the-rule)
lists `stdout` and `stderr` among the root capabilities; stage 0's
`System` has neither, and the standard streams are ambient
([What needs no capability](#what-needs-no-capability)).

**Status.** In stage 0 (D-015).

## What each capability does

Each capability is a handle whose operations are a trait, so that every
form of it, the root one, an attenuated one, a lent one, a fake and an
interceptor, implements the same trait:

| Capability | Trait | Operations |
|---|---|---|
| `Fs` | `FsOps` | `read`, `write`, `read_to_string`, `write_text`, `exists`, `metadata`, `remove`, `rename`, `list_dir`, `create_dir`, `sync`; failures are `IoError` |
| `Net` | `NetOps` | `connect(host, port)`, `listen(port)`, giving a `Conn` or a `Listener`; failures are `NetError` |
| `Clock` | `ClockOps` | `now()`, `monotonic()`, the time since the clock's own origin, which only moves forward, `after(duration)`, a `Timer`, and `sleep(duration)` |
| `Rand` | `RandOps` | `bits()`, and `between(lo, hi)`, `unit()`, `coin()` and `shuffle(xs)` built on it |
| `Env` | `EnvOps` | `args()`, `get(name)` |
| `Proc` | `ProcOps` | `run(program, args)`, giving a `ProcOutput`; failures are `ProcError` |
| `Stdin` | `StdinOps` | `read_line()` |

A capability is `Copy`: copying a handle grants nothing new, and every
copy reaches the same object, so a fake's state is seen by all of them
(Q-51). A function that needs a capability takes it as a parameter, and
a struct that needs one keeps it in a field.

What the library builds on a capability reads it through these
operations alone. `std.x.http`'s client and server work over a `Net`,
and their failures are `HttpError`: `Network(cause)` holds the
`NetError` of the connection, `Interrupted(cause)` the `TaskError` of a
wait that was cancelled, `Malformed(detail)` an answer that is not
HTTP and `Status(code)` a status other than `2xx`
([G10](../guides/network.md)). `std.time`'s `Instant.now(clock)` is the
`monotonic()` of the `Clock` it is given, so a change of the system's
time does not move it on the real clock, a recorded run replays it, and
a test clock, whose monotonic time is its own time, makes it
deterministic ([chapter 21](../tutorial/21-standard-library.md#time)).

**Status.** In stage 0 (Q-51).

## Attenuation

A capability can be narrowed into a weaker one, which is passed on in
its place:

| Call | Gives |
|---|---|
| `fs.restrict_to(dir)` | an `Fs` that reaches only the paths under `dir`, and refuses an absolute path or one with `..` |
| `fs.read_only()` | an `Fs` that refuses every change |
| `net.only_hosts(hosts)` | a `Net` that connects only to the hosts named |
| `net.https_only()` | a `Net` that connects only to port 443 |

A refused operation fails with `PermissionDenied` or `NotAllowed`, and
reports the caller's path. Attenuations compose, and a narrowed
capability cannot be widened again.

<!-- file: examples/reference-capabilities/narrow.sisu -->
```sisu
# An attenuated capability refuses what the wider one allowed.
def main(sys: System):
    let data = sys.fs.restrict_to("data")
    print(data.read_to_string("notes.txt"))
    print(data.read_to_string("../secret.txt"))
    let notes = data.read_only()
    print(notes.write_text("notes.txt", "Forget the whisks."))
    let net = sys.net.only_hosts(["weather.example.com"])
    match net.connect("tracker.example.org", 443):
        case Ok(_):
            print("connected")
        case Err(e):
            print("refused: {e}")
```

<!-- output: examples/reference-capabilities/narrow.out -->
```text
Ok("Buy birch whisks.\n")
Err(PermissionDenied(path="../secret.txt"))
Err(PermissionDenied(path="notes.txt"))
refused: NotAllowed(address="tracker.example.org:443")
```

**Status.** In stage 0 (D-015).

## Capabilities in generic code

A parameter typed by a capability, `fs: Fs`, accepts every form of it: a
value of another type that implements `FsOps` becomes an `Fs` handle
where one is expected. A generic parameter bounded by the trait, `[F:
FsOps]`, accepts the lent form too, which can never become a handle.
What becomes a handle must own what it holds (E0421), must not be
must-consume (E0422), and must be `Send` and `Sync`, since a handle may
reach every task (E0700).

<!-- file: examples/reference-capabilities/lend.sisu -->
```sisu
# Any form of a capability works where its operations trait is asked.
def total_words[F: FsOps](fs: F, paths: list[Path]) -> int:
    var total = 0
    for path in paths:
        let text = fs.read_to_string(path) else: String()
        total += text.split_whitespace().count()
    return total

def main(sys: System):
    print(total_words(sys.fs.lend(), ["a.txt", "b.txt"]))
    print(total_words(sys.fs.read_only(), ["a.txt"]))
    print(total_words(sys.fs, ["missing.txt"]))
```

<!-- output: examples/reference-capabilities/lend.out -->
```text
3
2
0
```

**Status.** In stage 0.

## Lending

`fs.lend()` is a `Lent[Fs]`, a view of the capability: the callee can
use it for the call and cannot keep it, since a view cannot be stored
where it could outlive the call ([R7](ownership.md#views)). Lending
costs nothing at run time. Keeping a lent capability in a field is
E0421:

<!-- file: examples/reference-capabilities/keep_lent.sisu -->
```sisu
# A lent capability is a view, so nothing can keep it.
struct Report:
    fs: Fs

def make(fs: Lent[Fs]) -> Report:
    return Report(fs)

def main(sys: System):
    let report = make(sys.fs.lend())
    print(report.fs.exists("a.txt"))
```

<!-- output: examples/reference-capabilities/keep_lent.console -->
```console
$ sisu run keep_lent.sisu
error[E0421]: `Lent[Fs]` borrows, so it cannot be put in `Fs`, which can outlive what it borrows (04, rule 3)
  --> keep_lent.sisu:6:19
  |
6 |     return Report(fs)
  |                   ^^
  = note: put an owned value in it instead, such as a `String` made with `.to_string()`

error[D0006]: the program was not run: errors found (1)
```

The note's advice is written for text; for a lent capability the fixes
are a parameter of type `Fs`, or doing the work inside the call.

**Status.** In stage 0 (leaning, Q-51).

## Revocation

`fs.revocable()` and `net.revocable()` give a caretaker, a capability
that forwards to the one it was made from, and its `Revoker`. Once
`revoker.revoke()` runs, every copy of the caretaker fails each
operation with `Revoked`, while the capability it was made from goes on
working. This bounds what a plugin keeps after its work is done.

<!-- file: examples/reference-capabilities/revoke.sisu -->
```sisu
# A caretaker's capability fails everywhere once it is revoked.
struct Plugin:
    fs: Fs

    def settings(self) -> String ! IoError:
        return self.fs.read_to_string("plugin.toml")

def main(sys: System):
    let dir = sys.fs.restrict_to("plugins")
    let plugin_fs, revoker = dir.revocable()
    let plugin = Plugin(fs=plugin_fs)
    print(plugin.settings())
    revoker.revoke()
    print(plugin.settings())
    print(dir.read_to_string("plugin.toml"))
```

<!-- output: examples/reference-capabilities/revoke.out -->
```text
Ok("colour = \"green\"\n")
Err(Revoked)
Ok("colour = \"green\"\n")
```

**Status.** In stage 0 (leaning, Q-51).

## Interceptors

`cap.Wrap(inner=c, hook=i)` is a capability that shows every call
through `c` to the interceptor `i`, a value whose type implements
`cap.Interceptor`: `before` receives the call, a `cap.Call` with its
capability, method and arguments, and `after` its outcome. Tracing,
auditing and recording for replay all go through this one point, which
is written in `main`, where a reader sees it (D-075).

<!-- file: examples/reference-capabilities/intercept.sisu -->
```sisu
# An interceptor sees every call through the capability it wraps.
struct Printer:
    pass

impl cap.Interceptor for Printer:
    def before(self, call: cap.Call):
        print("> {call.capability}.{call.method}({call.args})")

    def after(self, call: cap.Call, outcome: str):
        print("< {outcome}")

def main(sys: System):
    let fs: Fs = cap.Wrap(inner=sys.fs, hook=Printer())
    print(fs.exists("settings.txt"))
```

<!-- output: examples/reference-capabilities/intercept.out -->
```text
> Fs.exists("settings.txt")
< True
True
```

**Stage 0 differs.** `cap.Wrap` has impls for the standard capabilities
only; a `Wrap` of a program's own capability trait waits for
reflection.

**Status.** In stage 0 for the standard capabilities (leaning, D-075).

## Fakes

Every capability has fakes for tests, made by associated functions:

| Fake | Holds |
|---|---|
| `Fs.in_memory(files)` | a file system in memory, from a `dict` of paths and texts |
| `Clock.fake()`, `Clock.frozen_at(t)` | the run's test clock, a `TestClock` that moves only with `advance` and `set`, or when every task waits |
| `Rand.seeded(seed)` | a generator that gives the same numbers for the same seed |
| `Env.fake(args, vars)` | the arguments and variables given |
| `Proc.fake(outputs)` | processes that answer from a `dict` |
| `Stdin.fake(lines)` | the lines given |
| `Net.fake()` | a network inside the process, with listeners by address |
| `System.fake(seed)` | a whole set: an empty `Fs`, a `Net` that reaches nothing, the test clock and the others |

A test that names capabilities as its parameters, `test "name"(clock:
TestClock):`, receives fresh fakes of them; every test runs in an
interpreter of its own, so its fakes start new
([R15](testing.md#isolation-and-the-test-clock)).

<!-- file: examples/reference-capabilities/fakes.sisu -->
```sisu
# A test hands its code fakes of the capabilities the code takes.
def greeting(env: Env, rand: Rand) -> String:
    let name = env.get("SAUNA_NAME") ?? "the sauna"
    return "{name}, bench {rand.bits() % 3 + 1}"

def first_line(fs: Fs) -> String ! IoError:
    let text = fs.read_to_string("menu.txt")?
    let line = text.lines().next() ?? ""
    return Ok(line.to_string())

test "a fake environment and seeded randomness":
    let env = Env.fake(vars={"SAUNA_NAME": "Kotiharju"})
    let a = greeting(env, Rand.seeded(7))
    assert a == greeting(env, Rand.seeded(7))
    assert a.starts_with("Kotiharju, bench ")

test "a file system in memory":
    let fs = Fs.in_memory({"menu.txt": "coffee\nbun\n"})
    assert first_line(fs) == Ok("coffee".to_string())
    assert first_line(Fs.in_memory()).is_err()

test "a clock that moves when told"(clock: TestClock):
    let start = clock.now()
    clock.advance(Duration.minutes(90))
    assert clock.now() - start == Duration.minutes(90)
```

<!-- output: examples/reference-capabilities/fakes.test -->
```text
test "a fake environment and seeded randomness" ... ok
test "a file system in memory" ... ok
test "a clock that moves when told" ... ok
3 passed, 0 failed, 0 blocked
```

**Status.** In stage 0.

## What needs no capability

A little authority is ambient, on purpose (Q-5): `print` and
`print_err` write to the standard streams, `log` writes records to the
standard error, and `task`, `log` and `cap` are modules every module
sees. Starting a task needs no capability (Q-6).

<!-- file: examples/reference-capabilities/ambient.sisu -->
```sisu
# What needs no capability: the standard streams, `log`, and `task`.
def main():
    print("to standard output")
    print_err("to standard error")
    log.info("a record, with its level")
    let n = task.scope do s:
        let answer = s.spawn(() -> 6 * 7).join() else: 0
        return answer
    print(n)
```

<!-- output: examples/reference-capabilities/ambient.console -->
```console
$ sisu run ambient.sisu
to standard output
to standard error
info: a record, with its level
42
```

A `with Fs, Clock` clause on a signature, a capability that the
compiler passes, is reserved syntax that may come after the parameter
burden is measured (Q-5).

**Status.** In stage 0 (Q-5, Q-6).

## Pure functions take no capability

A `pure` function cannot take a capability, a `TestClock`, a lent
capability or a `System` (E0334), and the capabilities' operations are
not pure, so it cannot reach one any other way
([R2](types.md#pure-function-types)):

<!-- file: examples/reference-capabilities/pure_cap.sisu -->
```sisu
# A `pure` function cannot take a capability.
pure def note_count(fs: Fs) -> int:
    let notes = fs.list_dir("notes") else: []
    return len(notes)

def main(sys: System):
    print(note_count(sys.fs))
```

<!-- output: examples/reference-capabilities/pure_cap.console -->
```console
$ sisu run pure_cap.sisu
error[E0334]: the pure function `note_count` cannot use the capability `Fs`
  --> pure_cap.sisu:2:21
  |
2 | pure def note_count(fs: Fs) -> int:
  |                     ^^^^^^
  = fix: remove `pure`

error[D0006]: the program was not run: errors found (1)
```

A package whose public API takes no capability types and that is not in
`[trust]` can only compute, and the registry labels it pure (leaning,
D-066).

**Status.** In stage 0 (D-074).

## How the rule is enforced

Capabilities are a guarantee only while nothing goes around them, and
three things could:

- `unsafe` and `extern` code: a dependency may hold it only when the
  root package's `[trust] unsafe` lists it (E0703;
  [R13](modules.md#trust)), and a trusted package that reaches the world
  through foreign calls exposes that only to holders of the matching
  capability (D-084);
- compile-time code: it runs with no I/O at all (E0601,
  [R12](compile-time.md#what-compile-time-code-may-do));
- build scripts: there are none (D-032).

The standard library's own internals are closed to programs (E0701), and
a `sim` build fails to compile when reachable code could escape the
simulation through an `extern` call without a model (E0971).
`sisu audit --caps` lists the capabilities each function of a program
can reach ([R16](sisu-command.md#sisu-audit)).

**Status.** In stage 0 (D-032, D-084).

## Objects of your own

`Cap[T]` is the building block of a capability object written in Sisu:
a `Copy` handle on a value that every copy shares, `Sync` when `T` is,
whose value can be neither a view nor must-consume and changes through
a `Mutex` inside it. A type of your own that implements a capability's
trait over a `Cap` becomes that capability where it is expected, as the
fakes do.

**Status.** In stage 0.

## Diagnostics of this page

| Code | Identifier | Section |
|---|---|---|
| [E0200](diagnostics.md#e0200) | `resolve.undefined-name` | [The rule](#the-rule) |
| E0378, E0701 | `type.main-signature`, `cap.internal` | [The root capabilities](#the-root-capabilities) |
| [E0421](diagnostics.md#e0421), E0422, E0700 | `borrow.view-in-owner`, `borrow.must-consume-in-owner`, `cap.not-shareable` | [Capabilities in generic code](#capabilities-in-generic-code) |
| [E0334](diagnostics.md#e0334) | `type.impure-capability` | [Pure functions take no capability](#pure-functions-take-no-capability) |
| [E0703](diagnostics.md#e0703), [E0601](diagnostics.md#e0601), E0971 | `cap.untrusted`, `comptime.io`, `sim.unmodelled-extern` | [How the rule is enforced](#how-the-rule-is-enforced) |
