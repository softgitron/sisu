# Capabilities and the supply chain

In Sisu a function can reach files, the network, the clock, random
numbers, environment variables and other programs only through a value
it was given, a capability. The runtime hands every root capability to
`main` in a `System`, and each function gets what its caller passes on.
This page explains why the language gives up the global `open` that
every mainstream language has, what that buys for security, testing and
portability, what it costs, and what evidence would change it.

**See also:** the tutorial's [chapter 9](../tutorial/09-word-counter.md)
(the first capability) and [chapter 18](../tutorial/18-capabilities.md)
(capabilities); the reference's [R10,
Capabilities](../README.md#the-reference) and [R13, Modules, packages
and the manifest](../reference/modules.md) for `[trust]`; the proposal's
[06, Concurrency and
capabilities](../../proposal/06-concurrency-and-capabilities.md#capabilities)
and decisions D-015, D-032, D-066 and D-084 in its [decision
log](../../proposal/15-decision-log.md); the research in
[capabilities-and-effects.md](../../research/topics/capabilities-and-effects.md).

## The problem: every dependency can do anything

In Python, JavaScript, Go and Rust, any function in any package can open
a file, connect a socket or read an environment variable, because the
functions that do so are global. This is called ambient authority: the
right to act comes from being code in the process, not from anything the
code was given. A program with two hundred dependencies runs two hundred
authors' code with the full authority of whoever runs it.

The supply-chain attacks in the research all have that shape
([javascript.md](../../research/languages/javascript.md)). In 2018 the
new maintainer of `event-stream`, a package with about two million
weekly downloads, added a dependency whose encrypted payload targeted a
bitcoin wallet. In September 2025 a phished maintainer's packages,
`chalk` and `debug` among them, with over two billion weekly downloads
between them, shipped code that swapped cryptocurrency addresses for
about two hours, and the same month the Shai-Hulud worm used install
scripts to steal publishing tokens and republish itself into more
packages. In 2026 a compromise published through a project's own release
pipeline carried valid-looking provenance. In each case a library with
no business touching the network or the file system did so
([capabilities-and-effects.md](../../research/topics/capabilities-and-effects.md),
finding 1).

Every defence in use is a policy applied from outside the program:
lockfiles, provenance attestations, audits, registries that scan
uploads, and Deno's permission flags, which grant the whole process a
right on the command line rather than granting it to one dependency.
None of them is checked when the code is compiled. The question for a
language is whether it can make "this library cannot reach the network"
a fact about the library's types.

## What Sisu does: authority is a value

A Sisu program has no global function that opens a file. Code that tries
to write Python's `open` gets an undefined name, and the diagnostic says
where files come from instead:

<!-- file: examples/explanation-capabilities/no_open.sisu -->
```sisu
# A function that was handed no capability cannot reach a file: there
# is no global function to call.
def shout(text: str) -> String:
    let key = open("/home/aino/.ssh/id_ed25519")
    return text.to_uppercase()

def main():
    print(shout("sisu"))
```

<!-- output: examples/explanation-capabilities/no_open.console -->
```console
$ sisu run no_open.sisu
error[E0200]: cannot find `open` in this scope
  --> no_open.sisu:4:15
  |
4 |     let key = open("/home/aino/.ssh/id_ed25519")
  |               ^^^^
  = note: a program reaches files through the capability `Fs`, which `def main(sys: System)` holds as `sys.fs`: `sys.fs.read_to_string("data.txt")`

error[D0006]: the program was not run: errors found (1)
```

`shout` takes a `str` and nothing else, so nothing it calls can reach a
file, and the same holds for every function it calls in turn. A function
that reads files takes an `Fs`; one that connects to a server takes a
`Net`; one that reads the time takes a `Clock`. The runtime makes the
one `System` and hands it to `main`, and no safe code can make another
that reaches the world (D-015). So the signature of every function in
every dependency says what it can touch, and a library that is passed no
`Net` cannot phone home, whatever its author wrote.

A capability can also be narrowed before it is passed on:
`sys.fs.restrict_to("./data")` reaches one directory, `fs.read_only()`
cannot write, and `net.only_hosts(...)` reaches the hosts named.
Capabilities are `Copy`, since copying one grants nothing new, so a
callee can keep what it is handed. For the cases where it should not,
`std.cap` has two more forms (Q-51): a lent capability, `Lent[Fs]`, is a
view, so the callee cannot store it and its authority ends with the
call, and a revocable capability fails with `Err(Revoked)` in every kept
copy once its caretaker revokes it. E's caretakers and JavaScript's
`Proxy.revocable` are the precedents
([capabilities-distribution-and-resources.md](../../research/unconventional/capabilities-distribution-and-resources.md),
idea 3).

## Closing the ways around it

A capability is a guarantee only if there is no way around it, and the
research names three
([capabilities-and-effects.md](../../research/topics/capabilities-and-effects.md),
finding 6):

- **Unsafe code and foreign functions** can make raw system calls. Sisu
  permits them only in packages that declare their level in their own
  manifest and that the root package's `[trust]` consents to; a
  dependency cannot grant itself the permission (D-032, D-082). Trusted
  code that reaches the world through a foreign call takes the matching
  capability, as a SQLite binding takes an `Fs`, and only the runtime
  makes root capabilities (D-084). `unsafe` is not itself a capability:
  Austral's author found an `Unsafe` capability "much more tedious to
  use", and raw memory is not authority over the world, so a capability
  for it would travel from `main` to every caller of `list.push`.
- **Build scripts** run arbitrary code on every machine that builds the
  package. Rust's `build.rs` and npm's `postinstall` are the vectors
  that attacks used, so Sisu has none; code generation runs in
  `comptime` or as a separate tool.
- **Compile-time code** could read files while the compiler runs, as
  F#'s type providers do. Sisu's `comptime` evaluator has no I/O at all.

The registry builds on the same facts. Each package's capability summary
says which capabilities its API takes, a package whose API takes none
and that declares no `unsafe` is labelled pure, and when a new version
would reach more than the old one, the authority delta is shown before
the version is admitted (D-066). Fewer than 2% of dependency updates add
a capability in the data the research cites, so the signal is rare
enough to be read.

## Two more reasons: tests and portability

The capability rule was chosen for security, and it pays twice more.
Code that takes a `Clock` and an `Fs` is tested by passing it a clock
that stands still and a file system in memory, with no mocking
framework, no global test hooks and no container for dependency
injection. Go's `io.Reader`, Zig's `Io` and every style guide that says
"pass the dependencies in" reach the same pattern by convention; in Sisu
it is the only way, so a library can be tested whether or not its author
planned for it ([chapter 19](../tutorial/19-testing.md) shows the fakes,
and [Testing without mocks](testing.md) explains what they make
possible).

The same parameter makes code portable. A capability is an interface
with more than one implementation, so the same function runs on the full
runtime, in a single-threaded WebAssembly event loop, on bare metal, or
in the test simulator, and only the capability's implementation differs.
Roc's platforms and Zig's `Io` show the pattern working. And because
every effect passes through a capability, the capability is also where
effects are watched: `Wrap[T]` intercepts every call through a
capability for tracing, auditing, faking or recording, with the
interception written in `main` rather than woven in elsewhere, which is
where aspect-oriented programming's pointcuts went wrong (D-075).

## The alternatives other languages chose

Most languages keep ambient authority and defend from outside, as "The
problem: every dependency can do anything" describes. Three other
designs were weighed.

Effect systems, in Koka, Effekt, Flix and Unison, track in a function's
type what it does, and let a caller handle each effect. They are more
powerful than capabilities, and the research finds them not yet
ergonomic for application code: signatures grow noisy, PureScript
shipped effect rows and removed them, OCaml 5 shipped its effects
untyped in order to ship at all, and Scala's capture checking is still
experimental (finding 4). Sisu tracks one property in function types,
`pure`, and has no effect system (Q-17).

Implicit context, in Odin's `context`, Scala's `using` and Kotlin's
context parameters, passes values that a function does not name in its
arguments. It works in production, and it is the thing capabilities must
beat on ergonomics (finding 5). It is also ambient authority through the
back door: a reader cannot tell from a call whether the callee needs the
file system. Sisu keeps implicit context only for the current task scope
and its trace context, which carry cancellation and observation, never
authority.

Austral removes all ambient authority, printing included, so even
"hello, world" takes a parameter. Sisu keeps a short list ambient on
purpose: `print`, `print_err` and the `log` facade write to the standard
streams without a capability (Q-5), and `task.scope` needs none (Q-6).
Printing is convenient and nearly harmless, the reason Q-5 gives for
keeping it, and the thin end of a wedge, the reason it records against.

## What it costs

Every function that does I/O takes one more parameter, and that
parameter is threaded through every caller up to `main`. That is the
main objection in the research on Zig's `Io`, which shipped as a value
in Zig 0.16 in April 2026, and on Austral. Sisu's answers are partial.
Most functions do no I/O, and those that do should say so, as a function
that can fail says so in its type. A struct that needs an `Fs` and a
`Clock` takes them once, when it is built, and keeps them in fields. But
a function deep in a call graph that starts reading a file changes the
signature of every function between it and `main`.

The guarantee also has edges that the types do not cover:

- A capability limits what a dependency can reach, not what it does with
  what it is given. An HTTP client handed `Net` can connect to any host
  that `Net` allows, which is why `main` narrows it with `only_hosts`
  before passing it on.
- Ambient printing means that any function can write to standard output
  and standard error. Q-5 records the dissent.
- A package that the root trusts with `unsafe` can do anything, since
  raw memory can forge any capability. Trust is a decision made once, by
  whoever builds the program, and `sisu audit` lists what it covers.
- Capabilities do nothing about a package that disappears or changes
  under the same version, the `left-pad` kind of incident; that is the
  registry's job, which keeps every published version addressed by its
  content and recorded in a transparency log (D-031).

## What would change it

Capabilities as parameters are the second of the three bets that the
experiment tests ([16, Validation
plan](../../proposal/16-validation-plan.md), bet 2), with the thresholds
fixed in advance
([20](../../proposal/20-pre-registration.md#23-bet-2-capability-parameters-at-scale-d-015-q-5)).
A real Go service is ported twice, and with the corpus programs the
ports are measured by `sisu audit --caps`. The bet holds if the median
function that does I/O carries at most two capability parameters, none
carries more than four, and no struct outside `main` bundles four or
more kinds of capability, which would be ambient authority under another
name. If it fails, Sisu implements the clause that is reserved for this,
`def f() with Fs, Clock`: the capabilities named in the signature, as
now, but passed by the compiler instead of at each call. The same study
times ten tests written with fakes against the same tests written with
mocks in Go; more than 25% slower means the fakes of `std.cap` are
redesigned. And a planted dependency that tries to reach the network
must fail to compile, reproducibly.

Two smaller questions ride on the same study: how often a callee keeps a
capability it should not have, which could make lending the default
rather than an option (Q-51), and whether spawning a task should need a
capability too, which stays open with the runtime (Q-6).
