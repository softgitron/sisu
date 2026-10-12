# Total, hermetic and lattice-typed configuration languages

## Why this family matters for Sisu

Sisu bans build scripts, runs `comptime` in an interpreter with fuel, a
memory limit and no I/O except `embed`, and keeps its manifest in TOML
([08](../../proposal/08-metaprogramming.md), [09](../../proposal/09-modules-packages-and-build.md)).
Configuration languages are the long experiment in what such a sandbox
should guarantee (purity, totality, hermeticity, content-addressed inputs,
constraint validation). Should the manifest or `comptime` be total, should
CUE's unification validate configuration, should imports carry hashes, and
should Sisu offer itself as a configuration language as Starlark does?

## Systems covered

| System | Years | Status (Oct 2026) | Central idea |
|---|---|---|---|
| Nix | 2003- | Nix 2.35; flakes and content-addressed derivations still experimental; forks Lix (2024), Determinate Nix 3 (2025), Snix (2025) | Lazy, pure, dynamically typed package language; store paths hashed from all inputs |
| Guix | 2012- | 1.5.0 (Jan 2026); on Codeberg since May 2025 | Nix's model in Guile Scheme; full-source bootstrap |
| Dhall | 2016- | Standard v23.1.0; a short list of production users | Total, typed, no recursion; imports pinned by the hash of their normal form |
| CUE | 2018-19 [unverified] | v0.17.1 (2026), still 0.x | Types and values on one lattice; unification instead of overrides |
| Jsonnet | 2014- | Maintained | Lazy, pure JSON templating with mixin inheritance |
| Starlark | 2015- (as Skylark) | Spec; Java (Bazel), Go and Rust (Buck2) implementations | Deterministic, hermetic Python subset; no recursion; values frozen after load |
| KCL | 2022- [unverified] | CNCF sandbox since Sep 2023 | Python-like schemas with `check` constraints |
| Pkl | 2024- | 0.32.1 (Jul 2026) | Typed configuration with constraints, amending and code generation |
| Nickel | 2020-; 1.0 2023 [unverified] | Maintained by Tweag | Gradual types, contracts and merging |
| HCL / Terraform | 2014-; HCL2 2019 | Terraform under the BSL since 2023; OpenTofu fork GA Jan 2024; CDKTF archived Dec 2025 | Declarative blocks plus expressions; validations, pre/postconditions, checks |
| Bicep | 2020- | Maintained by Microsoft | Typed DSL compiled to ARM JSON |
| Bazel rules, Buck2, Declarative Gradle | 2015-; 2023-; 2024- [unverified] | Declarative Gradle experimental (third EAP) | Build logic in a restricted language |

## The ideas

### Purity, hermeticity and determinism

**What it is.** Evaluation cannot observe the environment, so the same input
gives the same output anywhere. Starlark: "Executing the same code twice
will give the same results" and "Execution cannot access the file system,
network, system clock. It is safe to execute untrusted code" [18]. Jsonnet:
purity "(together with determinism) gives us the property of hermeticity"
[17]. Dhall touches the outside world only through imports [14]. Nix and
Guix apply the idea to builds: a store path is a hash of everything that
went into the package, so identical inputs give identical names [1, 2, 10].

**What it buys.** Parallelism and caching. Starlark freezes a module's
top-level values after it runs, so "the module may be published to and used
by other threads in a parallel program without the need for locks", which
is how Bazel loads BUILD and `.bzl` files in parallel [18]. Determinism has
to be specified down to iteration order: Starlark dictionaries iterate "in
the order that they were inserted" [18]. Every system keeps an escape hatch
(imports, fetchers, host builtins), and determinism is only as good as the
least-specified builtin, which matters for Sisu below. Starlark is the
transplant of hermeticity into a general-purpose syntax by subsetting
Python; Sisu's `comptime` applies it to a whole language through `pure` and
the absence of capabilities.

### Totality versus budgets

**What it is.** Dhall (2016 [12]) is total: "If an expression type-checks
then evaluating that expression always succeeds in a finite amount of time"
[14]. Starlark gets there by syntax: "Execution is finite. The language does
not allow recursion or unbounded loops", and a recursive call is a dynamic
error [18].

**What it buys, by the designers' own account.** Less than advertised.
Gonzalez calls non-Turing-completeness a "shibboleth" and an "umbrella" for
the real guarantees (no general recursion, no escape hatches, no side
effects) and concedes that "you can craft compact Dhall functions that can
take longer than the age of the universe to evaluate" [13]. The Starlark
specification's own comment: the rule "doesn't stop Starlark programs from
consuming too much time or space. Perhaps it should be a dialect option";
implementations "may permit clients to bound computation directly, by
limiting the number of computational steps" [18]. Meta's starlark-rust,
which Buck2 depends on, lists recursion among its extensions [19]. Jsonnet
decided that "restricting termination would create more problems than it
would solve" [17].

**Budgets have their own failure mode.** Rust capped the statements in a
constant evaluation and removed the cap in 1.72 (August 2023) because
"whether code hit the limit could vary wildly based on libraries invoked by
the user"; it now reports long evaluations and raises a deny-by-default lint,
`const_eval_long_running`, that a crate may `allow` [30]. A budget that a
library's internals can exhaust turns a dependency's refactoring into a
broken build downstream.

### Content-addressed inputs and pinned imports

**What it is.** Nix names a store path by a hash that takes "into account
all dependencies of a package" [2]. Content-addressed derivations (RFC 62,
opened 14 August 2019) derive the path from the output instead, so an
identical rebuild stops the cascade ("substantial build cutoffs"), with
"non-trivial implications (regarding the trust model in particular)" [4].
A Dhall import can be pinned with `sha256:`; the hash is of "the
expression's normal form, not of its source text", so reformatting keeps it
valid while any transitive change is caught, and a frozen import "can never
successfully return a different expression" [14]. Guix's full-source
bootstrap (April 2023) roots a package graph of more than 22,000 nodes "in a
357-byte program", treating the trusting-trust attack as "a symptom of an
incomplete or missing bootstrap story" [11]. Bazel's content-addressable
cache and command history make it a cloud build system [28].

**Where it stalled.** Content-addressed derivations are still experimental
in Nix 2.35, seven years after the RFC, which flags the trust model as the
hard part [3, 4]. Hashes in import statements make every upgrade a source
edit; ecosystems keep them in lock files instead (`go.sum`, `Cargo.lock`).

**For Sisu.** The registry is content-addressed with a transparency log, and
the cache key is the hash of the normalised AST, which is Dhall's
normal-form hashing applied to the compiler
([09](../../proposal/09-modules-packages-and-build.md)). Source-level pins
would add nothing.

### Validation: lattices, contracts and inheritance

**CUE.** "Types are values", ordered in a lattice: `&` is the meet
(unification), `|` the join, and bottom (`_|_`) is the error a conflict
produces, so the result does not depend on order [15]:

```cue
#Service: {
    name:     =~"^[a-z-]+$"
    port:     int & >=1 & <=65535
    replicas: int | *1
}
web: #Service & {name: "web", port: 8080}
web: replicas: 3    // unifies with the line above, in either order
web: port: 9090     // error: conflicting values 9090 and 8080
```

Inheritance "is not commutative and idempotent in the general case"; "as
CUE disallows overrides, deep layerings are naturally prevented"; the
tooling GCL's descendants promised "never materialized, though, as the model
made it intractable". CUE's roots are GCL, "originally designed to configure
Borg", and graph unification from linguistics; it takes "the validation
first stance" [15].

**The inheritance school.** Jsonnet holds that late binding is "ideal for
deriving variants from existing data", with mixins and an unrestricted
`super` [17]. Pkl derives variants by *amending* a base module and puts
constraints in types (`age: Int(isBetween(0, 130))`); "A failing constraint
causes an evaluation error", and schemas generate Swift, Go, Java and Kotlin
types [21].

**The contracts school.** KCL ("a constraint-based record & functional
language", CNCF sandbox) writes Python-like schemas with `check` blocks
(`check: 1 <= port <= 65535, "port out of range"`) [20]. Nickel adds
contracts that "enforce types (or more complex, user-defined assertions) at
runtime" to gradual types [9]. Terraform grew contracts in steps: variable
`validation` blocks (a `condition` plus an `error_message`) in 0.13,
preconditions and postconditions in 1.2, `check` blocks in 1.5 [23]. Bicep
types Azure's ARM JSON and "isn't intended as a general programming
language" [22].

**Adoption.** The languages tied to a host tool have the users: HCL
(Terraform), Starlark (Bazel), Pkl (Apple), KCL (cloud-native tooling).
CUE is still 0.x (v0.17.1 in 2026) [15], and Dagger, its best-known
adopter, ended its CUE SDK on 14 December 2023 after "a steep decline in
usage" once Go, Python and Node.js SDKs existed: "Learning a brand new
language, however powerful, is simply not what they're looking for" [16].
Dhall's production list names about fourteen small companies [14]. Nickel's
rationale sums up the trade-offs from a competitor's side: Dhall's types are
"expressive but complex", CUE "sacrifices flexibility by not supporting not
only general recursion, but even general functions", Jsonnet "does not
feature static types, contracts or metadata" [9].

**For Sisu.** Refinement types, `invariant` and `Decode` (which validates
input against a type, refinements included) already give the
validation-first stance ([07](../../proposal/07-contracts.md),
[11](../../proposal/11-stdlib.md)). What CUE adds is order-independent
merging of partial values with conflicts as errors.

### Laziness and where errors appear

Nix is "a declarative, purely functional, lazily evaluated, dynamically
typed programming language" [2]; Jsonnet made "the whole language" lazy,
arguing that without I/O it hurts neither speed nor stack traces [17].
Nix's community says otherwise: Lix lists "better error messages in many
places" and evaluation "8-20% faster than 2.18" among its reasons to exist
[7], and Tweag built Nickel because "Nix lacks any native typing and
validation capabilities" [9]. Under laziness an error surfaces where a value
is forced, not where it was written (my characterisation, not a
measurement). Sisu's `comptime` is strict and statically typed, the other
side of the trade.

### Build languages as programming languages

Mokhov, Mitchell and Peyton Jones model a build system as a scheduler plus a
rebuilder [28]. Bazel restarts tasks, keeps a cloud cache and "at present,
user-defined build rules cannot have dynamic dependencies"; Nix "uses the
suspending scheduler and is monadic". Static dependencies are what keep
parallel scheduling and remote caching simple. A Bazel rule is a Starlark
function that declares actions rather than running them:

```python
def _impl(ctx):
    out = ctx.actions.declare_file(ctx.label.name + ".txt")
    ctx.actions.write(out, "\n".join(ctx.attr.lines))
    return [DefaultInfo(files = depset([out]))]
```

Starlark spread to Buck2, Tilt, Copybara, Drone, ytt, Chromium's lucicfg,
Kurtosis and Delve, with implementations in Java, Go, Rust, C++ and Python
[19]. The movement runs both ways. Gradle, whose scripts are full Groovy or
Kotlin, is building Declarative Gradle, "a tailored tiny subset of the
Kotlin language", for "excellent IDE integration" (experimental, third
early-access preview) [29]; HashiCorp archived CDK for Terraform on 10
December 2025, recommending "migrating to standard Terraform and HCL" [24];
Dagger went from CUE to general-purpose SDKs [16]. The stable points are a
restricted declarative layer for data that tools read and edit, and a real
language for logic.

### The configuration complexity clock and the convergence

Hadlow's clock (2012) runs from hard-coded values through a configuration
file, a rules engine and a DSL back to 12 o'clock, hard-coding everything
again "except now in a much crappier language"; "at a certain level of
complexity, hard-coding a solution may be the least evil option" [26].
Google's Borg, Omega and Kubernetes retrospective: configuration languages
"(eventually) become Turing complete", which "just moves the computations
from a real programming language to a domain-specific one, which typically
has weaker development tools"; the recommendation is "a simple, data-only
format such as JSON or YAML", changed programmatically "in a real
programming language" [27].

The family converged on five properties: purity (all), totality (Dhall,
Starlark), hermeticity (Starlark, Jsonnet, Dhall, Nix builds),
content-addressed inputs (Nix, Guix, Dhall, Bazel) and constraint
validation (CUE, KCL, Pkl, Nickel, HCL). Each is a step round the clock
towards a programming language; a language that already has types,
contracts, purity and a sandboxed evaluator can offer all five without a
new configuration language.

### Governance: what the Nix split says

After Anduril's NixCon sponsorship caused an uproar (September 2023), an
April 2024 open letter criticised Eelco Dolstra's leadership; "Dolstra
stepping down and an initiative towards community-based governance"
followed [5]. The constitution set up a seven-member elected Steering
Committee with "a hard maximum of two members with the same CoI" [6]. Lix
forked from CppNix at 2.18, "built for a community, not for a corporation",
citing "commercial interests that have long plagued both upstream CppNix and
corporate-authored forks" [7]. Determinate Nix, which Wikipedia calls a
proprietary fork and its maker "a downstream distribution", declared flakes
"production ready today" in version 3.0 (5 March 2025), citing "deep
uncertainty in the ecosystem due to flakes' experimental status in upstream
Nix" [2, 8]; flakes have been experimental since Nix 2.4 (1 November 2021)
and still are in 2.35 [3]. Tvix and its fork Snix reimplement the evaluator
in Rust [2]. Terraform split over a licence: after the move to the BSL
(2023 [unverified]), OpenTofu became a Linux Foundation project, 1.6.0 on
10 January 2024 [25]. An experiment with no end becomes a standard that
vendors stabilise on their own; purity and content addressing prevented
neither split.

### Sisu's own sandbox and manifest

`comptime` has no I/O, `embed("path")` records its file among the build
inputs, fuel and memory limits are "configurable in the manifest", and "a
build that evaluates the same source produces the same output"
([08](../../proposal/08-metaprogramming.md)); `sisu.toml` is data that
`sisu add` edits ([09](../../proposal/09-modules-packages-and-build.md)).
Against this family four gaps appear:

1. **Iteration order.** D-049 randomises `dict` iteration per process, so
   `comptime` code that iterates a `dict` gives different output in
   different builds, against the reproducibility goal; Starlark specifies
   insertion order for this reason [18].
2. **Fuel in the root manifest.** If a dependency's `comptime` work grows in
   a patch release, the downstream build fails until the root raises its
   limit: Rust's 1.72 lesson [30].
3. **Computed `embed` paths** make the input set dynamic, which Bazel
   forbids in user rules [28].
4. **Manifest drift.** Cargo already embeds a `cfg(...)` expression language
   in TOML keys for target-specific dependencies: the clock turning inside a
   manifest.

## Transferable ideas for Sisu

The answer to the central question: no totality by syntax, but a budget
designed after Rust 1.72; no lattice type system, but conflict-checked
merging in a library; no hash pins in source (the lockfile, registry and
normalised-AST keys already pin); and yes to Sisu as its own configuration
language through `sisu eval` of `pure` modules, while `sisu.toml` stays data.

1. **Close `comptime`'s determinism holes.** In `comptime`, `dict` and `set`
   iterate in insertion order with a fixed hash key, and reflection returns
   declarations in source order; `sisu build --check` builds twice with
   different interpreter hash keys and compares artefacts. *Cost*: low.
   *Conflicts*: D-049 needs a `comptime` exception. *Confidence*: high (two
   decided documents contradict each other).
2. **The manifest stays data.** No conditionals, templating or expressions in
   `sisu.toml`; target conditions are structured keys; anything computed
   lives in `comptime` Sisu.
   ```toml
   winapi = { version = "0.3", when = { os = "windows" } }   # not a cfg() string
   ```
   *Cost*: none. *Conflicts*: none. *Confidence*: high (the Borg
   retrospective, Hadlow, Declarative Gradle, CDKTF's end, and `sisu add`,
   which can only edit data reliably [24, 26, 27, 29]).
3. **Budgets owned by the package that spends them.** Count fuel in SIR
   steps; on exhaustion raise a deny-by-default diagnostic naming the
   expression and package, print progress for long evaluations, and let a
   package raise its own budget so that dependents never edit their manifest
   for a dependency's internals.
   ```sisu
   @comptime_budget(steps=50_000_000)   # this package's tables are expensive
   const TABLES: CaseTables = comptime build_tables(embed("data/UnicodeData.txt"))
   ```
   *Cost*: low. *Conflicts*: replaces the manifest knob of
   [08](../../proposal/08-metaprogramming.md). *Confidence*: medium-high
   (Rust 1.72, Starlark's comment, Dhall's admission [13, 18, 30]).
4. **`sisu eval`: Sisu as its own configuration language.** A module of
   `const` items and `pure` functions, evaluated by the `comptime`
   interpreter and rendered as JSON, TOML or YAML through `Encode`, with the
   types, refinements and invariants as the schema: Pkl's model and the Borg
   recommendation, without a second language.
   ```sisu
   ## deploy/prod.sisu, rendered by `sisu eval deploy/prod.sisu --to yaml`
   type Port = int where 1 <= self <= 65535

   @derive(Encode, Decode)
   struct Service:
       name: String
       port: Port
       replicas: int = 1
       invariant self.replicas >= 1

   pub const SERVICES: list[Service] = [Service(name="web", port=8080, replicas=3)]
   ```
   *Cost*: low to medium (a driver around `comptime` and `Encode`).
   *Conflicts*: none. *Confidence*: medium; Dagger's lesson cuts both ways,
   since people configure in the language they already know [16].
5. **Conflict-checked merging for layered configuration.** CUE's rule as a
   library: equal values unify, conflicting scalars are an error
   (`config.merge(base, overlay)?` returning `MergeConflict(path="db.port",
   ...)`), overriding is explicit, and inherited workspace keys follow the
   same rule. *Cost*: low. *Conflicts*:
   [09](../../proposal/09-modules-packages-and-build.md) adopts Cargo's
   workspaces unchanged. *Confidence*: medium-low.
6. **A maximum lifetime for previews.** A `@preview` feature is stabilised or
   removed within four releases (two years). *Cost*: none. *Conflicts*:
   [13](../../proposal/13-evolution-and-governance.md) sets only a minimum.
   *Confidence*: medium (Nix flakes [3, 8]).

## Dead ends

- **Totality by syntax** (Dhall, Starlark): its designers concede it does
  not bound running time, Buck2's Starlark adds recursion back, Jsonnet
  refused it [13, 17, 18, 19]; fuel is the honest bound.
- **A lattice type system** (CUE): types as values clash with nominal types
  and ownership, and CUE is still 0.x after its best-known adopter left
  [15, 16]; refinements and `Decode` already validate.
- **Hash pins in import statements** (Dhall): duplicate the lockfile and
  turn upgrades into source edits [14].
- **Lazy evaluation** (Nix, Jsonnet): errors surface where values are
  forced, and error messages are Nix's standing complaint [7, 9].
- **Inheritance and late binding for variants** (Jsonnet, Pkl, GCL): order
  dependence and untraceable layers [15]; D-012 already rejects inheritance.
- **An executable manifest** (Gradle scripts, Package.swift, build.zig):
  conflicts with D-032 and with tools that edit manifests; Gradle is
  retreating to a declarative subset [29].
- **A separate configuration dialect** (Starlark-style): Starlark subsets
  Python because Python is neither hermetic nor deterministic; `comptime`
  already is both, so a dialect would only double the specification.
- **Dynamic dependencies from `comptime`** (globs, computed `embed` paths):
  they give up the static input set that keeps builds parallel and
  cacheable [28].

## Sources

The web-search limit was reached during this research: unchecked claims are flagged in the text, and entries without a URL are cited from memory.

1. E. Dolstra, M. de Jonge, E. Visser, "Nix: A Safe and Policy-Free System for Software Deployment", LISA 2004; E. Dolstra, "The Purely Functional Software Deployment Model", PhD thesis, Utrecht University, 2006.
2. Wikipedia, "Nix (package manager)" (first release 15 June 2003; Tvix, Snix, Determinate Nix, Lix), retrieved Oct 2026. https://en.wikipedia.org/wiki/Nix_(package_manager)
3. Nix 2.35.2 Reference Manual, "Experimental features", and "Release 2.4 (2021-11-01)". https://nix.dev/manual/nix/latest/development/experimental-features ; https://nix.dev/manual/nix/latest/release-notes/rl-2.4
4. NixOS RFC 62, "Content-addressed paths" (start date 2019-08-14). https://github.com/NixOS/rfcs/blob/master/rfcs/0062-content-addressed-paths.md
5. Wikipedia, "NixOS" (Anduril sponsorship 2023; open letter April 2024; NixOS Foundation board post of 1 May 2024), retrieved Oct 2026. https://en.wikipedia.org/wiki/NixOS
6. NixOS constitution (Steering Committee, conflict-of-interest limits). https://github.com/NixOS/org/blob/main/doc/constitution.md
7. Lix, "About Lix", retrieved Oct 2026. https://lix.systems/about/
8. Determinate Systems, "Determinate Nix 3.0" (5 March 2025). https://determinate.systems/blog/determinate-nix-30/
9. Tweag, Nickel "RATIONALE.md" and nickel-lang.org, retrieved Oct 2026. https://github.com/tweag/nickel/blob/master/RATIONALE.md ; https://nickel-lang.org/
10. Wikipedia, "GNU Guix" (1.5.0, January 2026; Codeberg, 25 May 2025), retrieved Oct 2026. https://en.wikipedia.org/wiki/GNU_Guix
11. GNU Guix blog, "The Full-Source Bootstrap: Building from source all the way down" (26 April 2023). https://guix.gnu.org/en/blog/2023/the-full-source-bootstrap-building-from-source-all-the-way-down/
12. G. Gonzalez, "Dhall - A non-Turing-complete configuration language" (5 December 2016). https://www.haskellforall.com/2016/12/dhall-non-turing-complete-configuration.html
13. G. Gonzalez, "Why Dhall advertises the absence of Turing-completeness" (17 January 2020). https://www.haskellforall.com/2020/01/why-dhall-advertises-absence-of-turing.html
14. Dhall documentation, "Safety guarantees" and "Dhall in production"; dhall-lang releases (v23.1.0). https://docs.dhall-lang.org/discussions/Safety-guarantees.html ; https://docs.dhall-lang.org/discussions/Dhall-in-production.html ; https://github.com/dhall-lang/dhall-lang/releases
15. CUE documentation, "The logic of CUE", "Configuration use case", "Introduction"; cue-lang/cue releases (v0.17.1; v0.18 pre-releases), retrieved Oct 2026. https://cuelang.org/docs/concept/the-logic-of-cue/ ; https://cuelang.org/docs/concept/configuration-use-case/ ; https://cuelang.org/docs/introduction/ ; https://github.com/cue-lang/cue/releases
16. Dagger, "Ending support for the Dagger CUE SDK" (14 December 2023). https://dagger.io/blog/ending-cue-support
17. Jsonnet, "Design rationale". https://jsonnet.org/articles/design.html
18. Starlark specification and README (bazelbuild/starlark), retrieved Oct 2026. https://github.com/bazelbuild/starlark/blob/master/spec.md ; https://github.com/bazelbuild/starlark
19. "Awesome Starlark" (implementations and users); facebook/starlark-rust README. https://github.com/laurentlb/awesome-starlark ; https://github.com/facebook/starlark-rust
20. CNCF, "KCL" project page (sandbox, accepted 20 September 2023); KCL documentation, "Introduction". https://www.cncf.io/projects/kcl/ ; https://www.kcl-lang.io/docs/user_docs/getting-started/intro
21. Pkl Team, "Introducing Pkl" (1 February 2024); apple/pkl releases (0.32.1, 23 July 2026). https://pkl-lang.org/blog/introducing-pkl.html ; https://github.com/apple/pkl/releases
22. Microsoft Learn, "What is Bicep?" (updated January 2026). https://learn.microsoft.com/en-us/azure/azure-resource-manager/bicep/overview
23. HashiCorp, Terraform "Custom conditions" (validation 0.13, pre/postconditions 1.2, check blocks 1.5). https://developer.hashicorp.com/terraform/language/expressions/custom-conditions
24. HashiCorp, terraform-cdk README (sunset and archive on 10 December 2025). https://github.com/hashicorp/terraform-cdk
25. OpenTofu, "OpenTofu is going GA" (10 January 2024). https://opentofu.org/blog/opentofu-is-going-ga/
26. M. Hadlow, "The Configuration Complexity Clock" (7 May 2012). http://mikehadlow.blogspot.com/2012/05/configuration-complexity-clock.html
27. B. Burns, B. Grant, D. Oppenheimer, E. Brewer, J. Wilkes, "Borg, Omega, and Kubernetes", ACM Queue 14(1), 2016. https://static.googleusercontent.com/media/research.google.com/en//pubs/archive/44843.pdf
28. A. Mokhov, N. Mitchell, S. Peyton Jones, "Build Systems à la Carte", PACMPL 2 (ICFP), 2018. https://www.microsoft.com/en-us/research/uploads/prod/2018/03/build-systems.pdf
29. Gradle, "Declarative Gradle" (experimental, third EAP), retrieved Oct 2026. https://declarative.gradle.org/
30. The Rust Release Team, "Announcing Rust 1.72.0" (24 August 2023), const evaluation. https://blog.rust-lang.org/2023/08/24/Rust-1.72.0/
