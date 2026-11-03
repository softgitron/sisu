# How Sisu evolves

From version 1.0, Sisu promises that a program that compiles keeps
compiling, with the same meaning, on every later 1.x release.
Incompatible changes come only through editions, every three years,
which a package opts into and `sisu fix` migrates; libraries ship the
migrations of their own changes; new features are previewed under a
deadline; a written specification decides disputes; and the language is
governed by written rules. This page explains why the design spends so
much on changing safely, what each mechanism costs, and what is still
open. Most of it is designed rather than built: stage 0 knows one
edition, and `sisu fix` and previews wait.

**See also:** the reference's [R13, Modules, packages and the
manifest](../reference/modules.md#editions), [R16, The sisu
command](../reference/sisu-command.md#sisu-fix) and [R19, False
friends](../reference/false-friends.md); the tutorial's chapter 28, in
the [plan](../README.md#part-v-further), and the guide on moving to a
new edition, in the [list of guides](../README.md#the-guides); the
proposal's [13, Evolution and
governance](../../proposal/13-evolution-and-governance.md),
[10](../../proposal/10-tooling.md#migration) and
[09](../../proposal/09-modules-packages-and-build.md), and decisions
D-036, D-037, D-067 and D-072 in its [decision
log](../../proposal/15-decision-log.md); the research in
[evolution-and-governance.md](../../research/topics/evolution-and-governance.md)
and
[designer-retrospectives.md](../../research/evidence/designer-retrospectives.md).

## The problem: mistakes outlive their makers

A language's largest decisions are made after it ships, by whoever is
allowed to make them, under whatever rules exist for breaking things.
The designers' retrospectives in the research hold more regret about
this than about any choice of syntax
([13](../../proposal/13-evolution-and-governance.md)).

Breaking things has a measured price. Python 3 fixed a real mistake, the
confusion of bytes and text, and still needed twelve years in which both
versions ran: its conversion tool worked in one direction only, the
libraries everyone needed could not move until their own dependencies
moved, and the first releases offered users nothing in return for the
work. Perl 6 took fifteen years and Perl's momentum with it; Swift's
first three versions left three years of broken packages; Scala 3's
migration was held up for years by Scala 2's macros
([evolution-and-governance.md](../../research/topics/evolution-and-governance.md),
finding 1). Never breaking has a price too. JavaScript's rule never to
break the web makes every mistake permanent, Java's `Date` and `Vector`
live forever, and Fortran took sixty-five years to deprecate implicit
typing (finding 2). Russ Cox calls Go's compatibility promise "the most
important design decision we made for Go 1", and Rich Hickey argues that
"breaking changes are broken" and that a change should add a new name
instead
([designer-retrospectives.md](../../research/evidence/designer-retrospectives.md)).
A language has to keep its promise and still be able to fix its
mistakes.

## A promise, and editions for what it forbids

The promise is Go's, in spirit (D-036). From 1.0, a program that
compiles on 1.x compiles on every later 1.y with the same behaviour,
except for security fixes, changes to behaviour the specification leaves
unspecified, and compiler bugs whose fix is announced a release in
advance. The research on Go finds the promise to be the reason that
companies trust it.

Editions are how mistakes still get fixed. Every three years a new
edition may change syntax, defaults, lints and the prelude in ways that
are not compatible. A package names its edition in its manifest, and
packages on different editions link together, because the compiler
understands every edition and they share one type system and one ABI.
That is Rust's mechanism, which the research calls the only known way to
fix early mistakes without a split like Python's, and it rests on a
constraint that Rust states as editions being "skin deep": an edition
may not change the meaning of a type or a trait that other editions see
(finding 3). Go fixed its loop variables in 1.22 through the version in
each module's file in the same way, and Dart gave each package its own
language version and an interactive migration tool, after which 98% of
its top packages moved within a year. GHC's per-file language flags are
the warning on the other side, because they produce dialects rather than
versions.

`sisu fix --edition` rewrites a package for the next edition with the
compiler's own understanding of the code, and its target is Dart's year,
not Python's twelve ([10](../../proposal/10-tooling.md#migration)).
Editions also carry the rule of [Loud divergence](loud-divergence.md)
after 1.0: a false friend found later gets its check through an edition,
because a new check can turn a program that compiled into one that does
not. One more rule follows from the same evidence: library code has one
meaning, whatever the flags of the build. Nim's `--panics` and
memory-manager switches, D's preview flags and Zig's release modes each
gave a library several meanings, which destroys trust in it (finding 6).
In Sisu no build mode drops the overflow and bounds checks that safe
code relies on (D-009), though how many contracts a build checks is
still a choice of the build.

## Migrations that travel with libraries

A library has the same problem as a language: a major version breaks its
dependents, and each has to find and rewrite every call by hand. Sisu's
registry already handles the first half of the problem, finding the
break. `sisu publish` compares a package's public API with the previous
version's, its types, signatures, impls, error sets and contracts, and
refuses a breaking change without a major version, as Elm's package
manager has done since 2014 without one accidental breaking release; and
it runs the previous version's black-box tests against the new one,
which catches some changes in behaviour behind an unchanged signature
(D-031, and N-44 in [19](../../proposal/19-novel-features.md)).

For the second half, the rewriting, the design has libraries ship their
migrations as code (D-067). A library rewrites the body of the function
it deprecates in terms of its new API and marks it `@fix_inline`:

<!-- file: examples/explanation-evolution/migrate.sisu -->
```sisu
# A library's old function, rewritten in terms of its new one: the
# body is the migration that a tool can inline into each caller.
struct Temperature:
    celsius: f64

    def fahrenheit(self) -> f64:
        return self.celsius * 9.0 / 5.0 + 32.0

@deprecated("use Temperature(celsius).fahrenheit()")
@fix_inline
def to_fahrenheit(celsius: f64) -> f64:
    return Temperature(celsius).fahrenheit()

def main():
    print(to_fahrenheit(80.0))
    print(Temperature(80.0).fahrenheit())
```

<!-- output: examples/explanation-evolution/migrate.out -->
```text
176.0
176.0
```

The old function still works, and its body is exactly what each caller
should now write. `sisu fix --deps` is designed to replace every call to
it in the dependents with that body, so the rewrite keeps the program's
behaviour by construction, with no template language and no macros. Go's
`//go:fix inline`, Dart's fixes that libraries ship as data, checked
against expected outputs, and Kotlin's `ReplaceWith` are the precedents.
Sisu's version is harder than Go's, because the inliner must respect
ownership: borrows, `own` moves and `defer` move into the caller's scope
with the body. A major release's API diff then marks the removals that
came with such a function as ones that migrate themselves. In stage 0
the attributes are accepted as data and nothing reads them yet, so the
call in `main` gets no warning.

The standard library holds itself to a stricter rule. Nothing in `std`
is ever removed: a deprecated item warns with its replacement named, and
after two editions it moves to `std.deprecated`, where it still
compiles. Java's permanent `Date` is the honest model, and Python's
removal of old modules in 3.13, which broke code that had worked for
twenty years, is the warning. Libraries that should be free to break
live in `std.x`, which is versioned on its own, and a module moves from
`std.x` to `std` only after two years without a major version (D-033).

## Previews with a deadline, and a specification

A feature that is not ready yet ships behind `@preview` for at least one
release before it becomes stable, and every preview is stabilised or
removed within four releases, two years (D-072). Java's previews have a
good record: nine rounds of structured concurrency produced a better
API. Previews without a deadline do not. Kotlin's context receivers were
experimental for three years, were used in production code all the same,
and were then withdrawn; Nix's flakes have been experimental since
November 2021 and are at the centre of a split in its ecosystem (finding
4). Releases come every six months, each supported for a year, with
every fourth a release supported for three, the cadence that the
research on Java credits with reviving the language.

A written specification exists before 1.0, changes with every change to
the language, and decides when the compiler and the document disagree.
Rust went a decade without one, until a vendor wrote one for
safety-certified users, and Go's and C's specifications are assets of
their languages (finding 5). The specification is prose with a reference
test suite, and a mechanised semantics of the core is a goal rather than
a requirement (Q-38). This repository already lives by the rule on a
small scale: when the front end written in Sisu disagreed with the one
written in Rust, the written rule decided, and the code that
contradicted it changed (D-079).

## Who decides

Until 1.0, a small design team with a lead makes the decisions, under a
written process (D-037). Every coherent design in the research came from
one mind or a very small group, Python, Go, Rust before 2015, Clojure,
Elm, Zig and Elixir among them, and the languages designed by committee
from the start show the alternative (finding 8). A single lead also
stalls, leaves or becomes the bottleneck: Elm went more than six years
between two releases, Python's lead stepped down after the dispute over
PEP 572, and D's two-person approval cost it a co-designer and a fork
(finding 9). So every decision is written down with its alternatives and
evidence, as this repository's decision log does; a public proposal is
accepted, rejected or deferred within 90 days, with reasons, since
Rust's open process also brought burnout and stalls of years; and a
foundation owns the trademark, the registry and the infrastructure from
the first day, so that the project survives any one person.

After 1.0, the foundation holds the assets and a language team of five
to nine owns the specification. The board cannot overrule a design
decision, the team cannot spend the foundation's money, and no single
sponsor may hold a majority of either. Rust's governance failures of
2021 and 2023 were about authority that nobody had written down, which
is why the boundaries are written now (finding 10), and the research
finds single-vendor control good for speed and bad for trust: Go, Swift,
Kotlin, C# and Dart are excellent languages whose roadmaps are visibly
their sponsors' (finding 11).

## What it costs

- **The compiler carries every edition.** Understanding all editions at
  once means an implementation and a specification that grow with each
  one, as Rust's do.
- **Editions are skin deep.** A mistake in the meaning of a type or a
  trait that every edition sees cannot be fixed by an edition at all,
  which raises the stakes of getting those right before 1.0.
- **The promise freezes accidents.** Once programs depend on a behaviour
  the specification never promised, changing it breaks them anyway, as
  Go found when a faster sort reordered equal elements; the
  specification's list of what is unspecified is the only defence.
- **Deadlines force decisions.** A preview must be decided within two
  years whether or not the evidence is complete.
- **A lead can stall.** The written process and the 90-day limit reduce
  that risk and add work for every proposal.

## What would change it

The governance is leaning, and its details, who the lead is, how the
language team is chosen and how the foundation is funded, are left to
whoever starts the project (Q-21). The migration design is tested on a
real case: a major version of a library ported in the experiment,
measured by the share of its users' call sites that migrate without an
edit by hand
([ai-era-and-evolution.md](../../research/ideation/ai-era-and-evolution.md),
A6). D-067 and D-072 are leaning until the owner reviews them. And one
gap is named in the research itself: the policy for breaking changes
before 1.0 is not stated. Sisu breaks freely today, as when D-074
reversed the order of `dict` iteration or D-089 froze the Rust front
end, and the decision log records each break with its reason; when
version 0.1 starts to have users, that policy has to be written as well.
