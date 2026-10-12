# The AI era, collaboration, evolution, verification and supply chain: candidate features

The lens: code written and reviewed with language models, by teams across time zones and years, under supply-chain
attack, with correctness demands. Under P10 ([00](../../proposal/00-vision-and-principles.md)) a novel *language*
feature gets at most a prototype, while new *tooling* over decided features is low-risk; the experiment
([18](../../proposal/18-next-steps.md)) pre-registers what it does not build, so each "prototype in the experiment" is
a cheap, named amendment; nothing model-facing may cost human readability (D-001). "I found no ..." means my searches
found none. The web-search budget ran out partway: † marks sources checked in this session.

## Summary

| # | Candidate | Nearest existing form | Verdict | Confidence |
|---|---|---|---|---|
| A1 | `todo("name")` as the typed hole; fills checked by the tests and contracts that reach it | Agda, Idris, GHC, Lean, Hazel holes; Hazel and GHC model fills | hole report v1; fill engine tooling | high / medium |
| A2 | Specification-weakening report and gate | none found; Eiffel's redefinition rule | tooling, v1 | medium-high |
| A3 | Authority delta at review and at dependency admission | Capslock and capcheck (Go); pnpm and uv cooldowns | tooling, v1 | high |
| A4 | Review ledger keyed by hashes of the capability slice | cargo-vet, crev (per version) | prototype with the registry | medium-low |
| A5 | Behavioural semver: the previous version's black-box tests at publish | NoRegrets, Dependabot (consumer side) | tooling, with the registry | medium |
| A6 | Library migrations as inlinable functions | Go `//go:fix inline`, Dart data-driven fixes | tooling, v1 | high |
| A7 | Schema lock and required migrations for persisted types | Buf, schema registries (outside languages) | `std.x` plus tooling | medium |
| A8 | Replay token for every failure; re-executable release evidence | FoundationDB, Antithesis (systems, not test runners) | replay: experiment; evidence: tooling | medium-high / medium-low |
| A9 | `sisu why`: queries over the compiler's derivations | Argus, Whyline, rustc notes (piecemeal) | tooling, v1 | medium-high |
| A10 | Left-to-right binding types; a completion-engine API | MoonBit's sampler; Mündler et al. | measure in the experiment | medium-low |
| A11 | Loud divergence from the training-data majority | none found as a principle | design rule; measure | medium-high |
| A12 | Context-window digest of language and APIs; a model arm in the experiment | llms.txt (practice) | prototype in the experiment | medium |
| A13 | Verification re-timed for model-written proofs | vericoding benchmarks | post-1.0 tooling; study after the verdict | medium |

Rejected (R1-R7): intent annotations, provenance markers, a tokeniser-tuned vocabulary, a form beyond `sisu fmt`,
model calls at build time, a machine syntax, a home-grown merge tool.

## Candidates

### A1. The typed hole is `todo("name")`, and its fills are checked by what reaches it

The prelude's `todo` ([02](../../proposal/02-syntax.md)), typed `todo[T](name: str = "") -> T`, is a hole: the body
type-checks around it and the interpreter runs until a test reaches it. `sisu check --holes` reports the expected
type, the bindings in scope with their conventions and capabilities, the contracts that constrain the hole, and the
tests that reach it, with the values seen at each hit (Hazel's hole closures). `sisu fill` asks a provider (a model,
or GHC-style enumeration) for candidates, type-checks and tests them with contracts on, and shows the survivors.

```sisu
def merge_ranges(rs: list[Range]) -> list[Range]:
    ensures all(result[i].end < result[i + 1].start for i in range(len(result) - 1))
    return todo("coalesce")    # check: expected list[Range]; must meet the ensures; 2 tests reach it
```

- **Precedents.** Holes: Agda, Idris (`?name`), GHC's valid hole fits [S3], Lean's `sorry` and `exact?`, Hazel [S4].
  Model fills: Hazel's ChatLSP gives the model the hole's type and context, also for TypeScript [S1]; GHC has
  `ollama-holes-plugin` and `ecta-plugin` [S2]; Smyth fills from assertions [S4]. I found no shipped toolchain that
  filters fills by the tests and contracts reaching the hole.
- **Buys.** Hole context improves completions [S1], type constraints halve compile errors
  ([studies §11](../evidence/studies.md)), and contracts and property tests catch "almost right" fills.
- **Costs and risks.** No new syntax: `?` already serves propagation, `T?`, `?.` and `??`, and P1 wants one hole.
  Fills run on fakes, a free sandbox. A hole no test reaches is flagged unconstrained; `sisu test --mutate` on the
  fill ([17](../../proposal/17-testing.md)) guards against weak tests; `sisu publish` warns on reachable `todo`.
- **Verdict.** Typed `todo` and the hole report in v1 (hover fits Phase 8's LSP); the fill engine is later tooling.
  High / medium. **Settled by** the model arm (A12): fill acceptance and later defects, with and without filtering.

### A2. A specification-weakening report, and a gate for automated changes

`sisu review base..head` lists, apart from the code diff, each change that weakens a promise: a removed or edited
`ensures`, `invariant` or refinement; an added or edited `requires`; a deleted test, changed expected value, new skip,
narrowed generator or smaller case budget; an updated snapshot; a dropped doc example, `pure` or `@must_consume`.
Direction is syntactic where possible and "unknown" for an edited clause until a verifier proves implication; the
normalised hash recognises moves and renames. Under `[review] spec-weakening = "require-trailer"`, CI fails unless the
commit has a human-written `Spec-Weakened-By:` trailer, which agent harnesses can be set never to write.

```
ensures removed   parse.sisu:14  parse_int: result.map(it >= 0) ?? True
expected changed  parse.sisu:88  "rejects bad digits": Err(BadDigit(at=1, found='x')) -> Ok(4)
```

- **Precedents.** Eiffel's `require else`/`ensure then` let a redefinition only weaken its precondition and strengthen
  its postcondition; Sisu applies that to trait impls and, via the API diff, to public contracts
  ([07](../../proposal/07-contracts.md)). CODEOWNERS on `tests/` is a path rule. I found no toolchain that classifies
  weakened contracts and assertions in a diff.
- **Buys.** Models game tests: on ImpossibleBench, whose tests contradict their specifications, frontier models cheat
  frequently, stronger ones more, from editing tests to overloading operators [S5]; METR saw o3 reward-hack in 30.4%
  of 128 RE-Bench runs against about 0.7% on HCAST, partly because the scorer was visible [S6]. Tests and contracts
  are syntax in Sisu, so the report is exact where other languages need heuristics.
- **Costs and risks.** An AST diff, no language change. Bulk rewrites trip it, trailers get rubber-stamped, and
  vacuous new tests evade it (mutation testing and Hypothesis-style health checks partly cover that); the fixed
  operator set closes the overloading route.
- **Verdict.** Tooling for v1, right after 0.1-interp; no registry needed. Medium-high. **Settled by**
  ImpossibleBench-style tasks in Sisu: cheats caught, and false alarms per hundred legitimate commits.

### A3. Authority delta, at review and at dependency admission

`sisu review --authority`, `sisu add` and `sisu update` report, per public function and package, newly reachable
capabilities (via parameters, fields, closures, `dyn` values), new `unsafe` and `extern`, `[trust]` changes, new
dependencies and new panic sites. With no ambient authority beyond standard streams and `task.scope`
([06](../../proposal/06-concurrency-and-capabilities.md)), reach follows from types, so the report is exact up to
`unsafe` in trusted packages. Admission adds a seven-day cooldown (advisory fixes exempt; applied before resolution,
since pnpm's late check broke Dependabot runs [S10]), refuses packages younger than that, or without dependents,
unless given `--accept-new-package`, and in CI or agent runs never adds a package or widens `[trust]` against policy.

- **Precedents.** Google's Capslock follows Go calls to privileged standard-library functions, and deps.dev found that
  fewer than 2% of version updates add a capability requirement [S7]; `capcheck` fails CI on drift from a committed
  baseline, warning that function granularity makes "a much larger and more volatile lock file" [S7]. Cooldowns:
  pnpm's `minimumReleaseAge` (September 2025), uv's relative `exclude-newer` (December 2025) [S10].
- **Buys.** A rare signal gets read (the 2%). Models invent packages: in 576,000 samples from 16 models, average rates
  were at least 5.2% (commercial) and 21.7% (open), and a 2026 replication on five newer models found 4.6-6.1%, with
  127 names all five invent, 53 still registrable [S9]. Eight of ten recent attacks Nesbitt examined had windows under
  a week [S10]. The s1ngularity compromise of Nx (August 2025) had its `postinstall` drive the victim's own AI
  command-line tools, with permission-bypass flags, to find secrets [S11]; Sisu has no install scripts and confines a
  library to what it is passed, so a payload needs the `Net`, `Fs` or `Proc` lines the delta shows.
- **Costs and risks.** None in the language: reach is already computed for the registry's capability summary
  ([09](../../proposal/09-modules-packages-and-build.md)) and for `pure`. Capabilities held in fields make reach
  coarse but correct (each method of a `Client` holding `Net` reaches it; the report names the field); both
  `if comptime` branches type-check, so reach covers every target. Namespaces narrow squatting without stopping it.
- **Verdict.** Tooling for v1; admission ships with the registry. High. **Settled by** the share of updates with a
  non-empty delta, and the registry's time from publish to yank for malicious versions.

### A4. A review ledger keyed by hashes of the capability slice

An audit record (`package`, `scope`, `root`, `criteria`, `who` in `supply-chain/reviews.toml`) holds a Merkle root
over the normalised-AST hashes of its scope; the default scope is the *capability slice*, the functions that can reach
a capability, `unsafe` or `extern` (A3). The hash strips names and formatting and includes dependencies' hashes
([09](../../proposal/09-modules-packages-and-build.md)), so an unchanged root means unchanged slice code and the next
version inherits the review; otherwise only functions whose hashes changed are listed.

- **Precedents.** cargo-vet records audits of a version and deltas between versions, under named criteria, importable
  across organisations, with nothing scoped below a version [S8]; crev's signed proofs cover releases or parts of
  code, with a web of trust [S8]. I found no review record keyed to function-level content hashes.
- **Buys.** Review effort in proportion to change in the code that can do damage. A pure parser has an empty slice:
  its worst case is wrong output, not exfiltration.
- **Costs and risks.** The hash becomes a specified artefact: the build cache may renormalise, a ledger may not, so it
  needs a review hash versioned with editions. A trait call on a type parameter does not hash the consumer's impl, so
  reviews cover the package's own instantiations; capability-touching `derive` output is in the slice.
- **Verdict.** Prototype with the registry. Medium-low: slice size and churn are unmeasured. **Settled by** the
  slice's share of code in ported packages, and the share of releases with an unchanged root.

### A5. Behavioural semver: the previous version's black-box tests at publish

`sisu publish` diffs signatures and contracts; it would also run the previous version's `tests/` (black-box tests that
see only the public API, [17](../../proposal/17-testing.md)) and doc examples against the candidate. A failure is a
behavioural break that needs a major version or `--supersede "<test>" --reason "..."`, shown to consumers in the
changelog: `FAILED "accepts leading zeros": parse_int("007") was Ok(7), now Err(LeadingZero)`.

- **Precedents.** Elm, cargo-semver-checks, japicmp and Go's apidiff compare signatures; Lam, Dietrich and Pearce
  argue for contract-fed "semantic version calculators" [S12]. NoRegrets uses clients' test suites to find type
  regressions in npm libraries, classifying at least 90% of updates correctly and finding 26 breaking minor or patch
  releases [S13]; Dependabot's crowd-sourced score is missing for 83% of updates [S14]; Debian gates migrations on
  reverse-dependency tests. I found no package manager that replays a package's own previous tests at publish.
- **Buys.** It catches what [09](../../proposal/09-modules-packages-and-build.md) leaves to the author, "semantic
  changes behind an unchanged signature", with no crowd data and no flakiness (deterministic fakes).
- **Costs and risks.** Publish time is bounded by the suite. Hyrum's law: a fix breaks a test that encoded the bug,
  and the supersede record is the point. Fewer black-box tests would dodge it; the registry shows their count.
- **Verdict.** Tooling, with the registry. Medium. **Settled by** a study runnable now on crates.io: version N's
  integration tests against N+1 for patch releases, counting breaks semver-checks misses and how many are bug fixes.

### A6. Library migrations shipped as inlinable functions

A library deprecates an item by rewriting its body in terms of the new API and marking it `@fix_inline`;
`sisu fix --deps` replaces dependents' calls with that body, so behaviour is preserved by construction, with no
template language and no macros (attributes are data, D-046). A major release's API diff marks removals that had such
a form as migratable: the library counterpart of `sisu fix --edition`.

```sisu
@deprecated("use Client.get")
@fix_inline
pub def get(net: Net, url: Url) -> Response ! HttpError:
    return Client(net).get(Request.get(url))
```

- **Precedents.** Go's `//go:fix inline`, applied by `go fix -inline` [S15]; Dart's `dart fix`, which applies
  data-driven fixes shipped by dependencies, verified against goldens [S15]; Kotlin's `ReplaceWith`; OpenRewrite.
- **Buys.** Evolution over years with many dependents ([10](../../proposal/10-tooling.md) takes the tool-driven Dart
  migration as its target); agents apply migrations mechanically and reviewably.
- **Costs and risks.** An ownership-aware inliner: inlining moves borrows, `own` moves and `defer` into the caller's
  scope, harder than in Go, whose inliner already needs binding declarations to keep evaluation order.
- **Verdict.** Tooling for v1, in Go's form. High. **Settled by** a major-version change of a ported library: the
  share of corpus call sites migrated without a hand edit.

### A7. A schema lock and required migrations for persisted types

A stored or transmitted type says so; reflection gives a shape hash that includes field names (they matter on the
wire), recorded in a committed `schemas.lock`. A shape change needs a new version, and a new version a compatible
change (a field with a default) or a migration from the frozen old shape, which `sisu schema freeze` writes as source
because `comptime` cannot declare types ([08](../../proposal/08-metaprogramming.md)). `sisu test` derives the property
that decoding version-2 bytes equals migrating the version-2 value.

```sisu
@derive(Encode, Decode)
@persisted("config", version=3)
struct Config:
    port: Port
    hosts: list[String] = []                          # new in version 3, with a default

@migration(Config, from_version=2)
def config_from_v2(own old: ConfigV2) -> Config:      # ConfigV2 frozen in schemas/config_v2.sisu
    return Config(port=old.port, hosts=[old.host])
```

- **Precedents.** Buf's breaking-change categories (FILE, PACKAGE, WIRE_JSON, WIRE), blind to changes of meaning
  [S16]; schema-registry compatibility modes; Django migrations; OTP's `code_change`; Firecracker's `versionize`
  (archived 2026) [S16]. All live outside languages, and Java's unchecked `serialVersionUID` is the warning. I found
  no language that enforces migrations for its own serialised types.
- **Buys.** Stored data outlives the code and the team; a format break stays silent until old data is read.
- **Costs and risks.** No language change: attributes, a `std.x` library, a lock file, a tool. Rules differ per
  encoding, and a new enum variant breaks old readers; migrations must meet the new refinements, which `Decode`
  validates ([11](../../proposal/11-stdlib.md)); a persisted `pub` type's shape change is breaking in the API diff.
- **Verdict.** `std.x` plus tooling after the experiment, whose std has no JSON encoder. Medium. **Settled by**
  persisted types per program in the bet-2 port, and silent format breaks in comparable services' incidents.

### A8. A replay token for every failure, and re-executable release evidence

Every failing test prints a token holding its content hash, toolchain version, `Rand` seed, scheduler seed and shrunk
case; `sisu test --replay 7Q2M-K4XD-91FE` reproduces it exactly anywhere. In tests the per-process SipHash key
(D-049), and so `dict` order, comes from the seed. `sisu publish --evidence` attaches an in-toto test-result statement
[S17], the fuzz corpus and the tokens; runs being deterministic, the registry re-runs a random sample instead of
trusting a signature, and an agent's "the tests pass" becomes checkable.

- **Precedents.** Deterministic simulation in FoundationDB and TigerBeetle; Antithesis's deterministic hypervisor; rr;
  AWS Shuttle and Loom's seeded schedules; Hypothesis's example database and Go's `synctest` (both in
  [17](../../proposal/17-testing.md)); Bazel's test results shared by action hash; the in-toto predicate [S17]. I
  found no language test runner that makes whole-run replay, schedule included, the default.
- **Buys.** Triage of flaky failures across time zones, and reproducible reports from users and agents:
  [17](../../proposal/17-testing.md) already removes the order and time causes of flakiness, leaving the schedule.
- **Costs and risks.** Cheap in the deterministic `single` interpreter profile; the native runtime needs a
  deterministic test scheduler. Tests that take the real `System` cannot replay, and the token says so. Bundles invite
  Goodhart's law, agents first, so the registry publishes facts (tests run, inputs used), not scores.
- **Verdict.** Replay tokens: prototype in the experiment (about one person-week in Phase 8); bundles: tooling with
  the registry. Medium-high / medium-low. **Settled by** the share of corpus failures reproduced from their token on a
  second machine (target: all without `System`), and whether sampled re-runs ever catch a false claim.

### A9. `sisu why`: queries over the compiler's derivations

One command, in human and JSON form, explains an inferred fact; v1 covers error sets, `Send`/`Sync`, `pure`,
capability reach (A3), impl selection and comptime fuel. Sisu infers much by design (P2) and owes the explanation.

```
$ sisu why send cache.sisu:31
Job is not Send: Job.cache: Shared[Cache] (line 8) -> Cache.hits: Cell[int] (line 3), and Cell is not Sync
  fixes: hits: Atomic[int], or cache: Shared[Mutex[Cache]]
```

- **Precedents.** rustc's "required because it appears within the type" notes; Argus, a debugger for Rust trait
  inference, with which 25 participants localised 2.2 times as many faults, 3.3 times faster [S18]; the Whyline, with
  which novices debugged one task twice as fast as experts without it [S18]; Scala's `-explain`; Sisu's "explain this
  borrow" ([10](../../proposal/10-tooling.md)). I found no language with one query over all its inferred facts.
- **Buys.** It repays inference's explicability cost, and gives agents structured repair input.
- **Costs and risks.** Tracing runs only in explain mode, by re-running the query, so compiles pay nothing; the JSON
  schema is an API to keep stable; trees need Argus-style curation.
- **Verdict.** Tooling for v1; the `Send` and error-set explanations fit Phase 8's LSP. Medium-high. **Settled by**
  within-subject timing on `Send` and error-set surprises, with and without `why`.

### A10. Left-to-right binding types, and a completion-engine API

A rule: a binding's type is fixed by the end of its declaring statement (annotation, initialiser, expected type), and
an inferred error set (`-> T !`) stays an open row while its body is checked. The compiler library then exposes
`complete(prefix)`: whether a prefix can still become well typed, and which next tokens keep it so, for constrained
decoding in any model runtime. The tour's `var results = list.with_capacity(len(handles))`, whose element type comes
from a later `push`, would become `var results: list[bytes] = ...`.

- **Precedents.** To constrain TypeScript, Mündler et al. had to *enforce* parameter and return annotations,
  initialised-or-annotated variables and annotated callbacks, left use before declaration and return-type inference
  unsupported, and suggest writing compilers "as an incremental completion engine" [S19]. MoonBit ships a
  semantics-based sampler, and its team counts mandatory top-level signatures as AI-friendly [S19]. Swift, Kotlin and
  Go fix a binding's type at declaration; Rust infers from later uses, as the tour does.
- **Buys.** Sisu meets most of Mündler's conditions already (explicit signatures; `let x: T` without a value needs its
  type); the rule closes the gap, and readers see a binding's type where it is declared.
- **Costs and risks.** Annotations on empty collections, and a narrower D-003. The engine is a second consumer of the
  checker, feasible on the error-tolerant front end but not cheap. Without the rule, constrained decoding still works
  with open inference variables, less precisely.
- **Verdict.** Measure in the experiment by counting bindings typed by later statements (cheap instrumentation); adopt
  the rule if under about 3% and authors report no friction; the API is post-experiment tooling. Medium-low.
  **Settled by** that count, and a constrained-decoding pilot with and without the rule.

### A11. Loud divergence from the training-data majority

A design rule: where Sisu spells something as Python does (or as Rust does, for syntax shared with Rust), it means the
same or fails to compile; a run-time failure is acceptable only where no static check exists, and a silent difference
is a design bug. The specification keeps a false-friend table, a check per row.

| Habit | Sisu today | Loud? | Proposal |
|---|---|---|---|
| `xs[-1]` | traps at run time | late | compile error for a negative constant index, fix-it `xs[^1]` |
| relying on `dict` insertion order | randomised ([11](../../proposal/11-stdlib.md)) | silent, flaky | insertion-ordered `dict`, or rename the type |
| `7 / 2`, `if xs:`, `a if c else b` | type or parse errors | yes | none |
| `-7 % 3` in code from Rust (-1) | 2, as in Python | silent for Rust-trained code | none; flag it in the Rust-to-Sisu corpus |

- **Precedents.** Python 3's silent `/` change (PEP 238) is the cautionary tale; Rust's editions ship migration lints
  for changed meaning. I found no design document that states loudness relative to models' priors.
- **Buys.** Models default to Python and drift between similar languages; explicit language markers help most ([S20],
  10 models); agents in unfamiliar languages write code that does not compile and prototype in Python first [S20].
  Sisu looks like Python and has no training data, so silent false friends would be its commonest generated bug.
- **Costs and risks.** Small for the checks. The `dict` row challenges a decided point: Python pairs insertion order
  with keyed SipHash (PEP 456), so D-049's hash-flooding evidence does not require random iteration, leaving
  [11](../../proposal/11-stdlib.md)'s argument that a guaranteed order invites silent dependence; ordered maps cost an
  index array and tombstones (Python 3.6's compact dict was still 20-25% smaller than before). The rule means
  departing loudly, never copying Python's mistakes ([00](../../proposal/00-vision-and-principles.md)); post-1.0 rows
  go through editions.
- **Verdict.** Adopt as a design rule with the table in the specification, measured per row in the model arm (A12);
  the `dict` change goes to the owner as an open challenge. Medium-high (rule), low-medium (`dict`). **Settled by**
  the arm's per-row rate of compiling programs that fail hidden tests through the Python or Rust meaning.

### A12. A context-window digest, and a model arm in the experiment

`sisu doc --digest` writes one plain-text file per package: each `pub` signature with its inferred error set,
capability parameters, contracts and first doc example, all of which Sisu signatures carry. The toolchain ships a
pocket language reference on a token budget (about 25k tokens; std about 50k). The experiment gains a model arm:
several frontier models solve the corpus tasks with nothing, the reference, or reference plus digest, each with and
without compiler feedback, measuring compile and hidden-test pass rates, false-friend bugs (A11) and tokens to a pass.

- **Precedents.** llms.txt and agent documentation servers, as practice; MultiPL-T
  ([10](../../proposal/10-tooling.md)) helps once vendors train on it. Learning a natural language from one grammar
  book in context (MTOB, Gemini 1.5) suggests the route works; I found no programming-language measurement.
- **Buys.** It attacks the new-language penalty ([studies §11](../evidence/studies.md)) on day one, not after a
  training cycle; agents struggle in unfamiliar languages, and token use varies sharply by language [S20]. Sisu
  signatures say more per token than most (errors, authority, contracts).
- **Costs and risks.** Small tooling, model-API money and a PI's time to pre-register; no outsiders and no confound
  with the human results. The risk is tuning the language to this year's models, so the arm measures and the design
  does not chase it. It runs in the playground's sandbox (Phase 9).
- **Verdict.** Prototype in the experiment as one amendment that also carries A10's count and A11's rows; the digest
  is v1 tooling. Medium. **Settled by** the arm: if reference and digest bring rates near those for the same tasks in
  Python, the route works; if not, the MultiPL-T corpus is necessary.

### A13. Verification re-timed for model-written proofs

Keep Q-16 as decided (verifier after 1.0; `ghost`, `decreases` and loop `invariant` reserved) but change the case for
its priority: proof burden, which kept SPARK and Dafny niche, is the cost models cut fastest, and the checker stays
the trusted base. Concretely: right after the experiment's verdict, a feasibility study translating the `pure` subset
and its contracts to an auto-active backend (Boogie or Why3 style) with model-proposed invariants, `decreases` and
lemmas, the lemmas being ordinary `pure` functions with `ensures` (no proof language); and `sisu verify --suggest`,
which proposes `ensures` from doc prose for property tests to vet before a human accepts them.

- **Precedents.** Vericoding (12,504 specifications): off-the-shelf models solved 82% in Dafny, 44% in Verus, 27% in
  Lean, and pure Dafny verification rose from 68% to 96% in a year [S22]; AlgoVeri, 77 algorithms in all three,
  reports 40.3% (Dafny), 24.7% (Verus) and 7.8% (Lean), a gap put down to automation [S22]. Clover's
  code-docstring-annotation consistency check accepted up to 87% of correct programs and no adversarial wrong ones;
  nl2postcond's GPT-4 postconditions were correct for up to 96% of EvalPlus problems, catching 64-70 Defects4J bugs
  [S22].
- **Buys.** Evidence that the auto-active style, [07](../../proposal/07-contracts.md)'s SPARK and Dafny shape, is the
  one models succeed with, and against drifting toward interactive proofs.
- **Costs and risks.** Large engineering, no language change. Model-written specifications are the weak link, which
  consistency checks and A2's gate address; ownership-heavy code needs a memory model in the verification conditions,
  as Verus and Prusti have for Rust.
- **Verdict.** Post-1.0 tooling as decided, with the study moved to right after the verdict. Medium. **Settled by**
  the share of the corpus's `pure` functions discharged automatically with model-proposed invariants.

## Rejected

- **R1. Intent annotations (`## intent:`).** A fourth specification channel beside prose, contracts and tests, checked
  by no tool, against P1. Spec-driven tools (GitHub's Spec Kit, Kiro) keep intent in Markdown outside the language;
  Clover shows the value lies in checking existing channels against each other (A2, A13).
- **R2. Per-function provenance markers.** Self-reported, unverifiable, stale after the next edit. Attribution belongs
  in version control: Agent Trace (a January 2026 draft) or the Linux kernel's `Assisted-by:` trailer, under which an
  AI may not add `Signed-off-by` [S23]. Behaviour is what can be verified, and A5 and A8 record it.
- **R3. A vocabulary tuned to tokenisers.** SimPy's AI-oriented grammar cut tokens 10.4-13.5% at readability's cost
  [S21]; rankings move with each tokeniser, and the RosettaCode comparison's author calls it "not a scientific study"
  [S21]. Kept: no novel symbols (APL's terseness does not help models [S21]); A12 only counts tokens.
- **R4. A canonical form beyond `sisu fmt`.** Already decided: D-030, canonical sugar, normalised hashes.
- **R5. Model calls at build time.** Non-deterministic builds, a breach of `comptime`'s no-I/O rule, and a
  supply-chain channel by construction.
- **R6. A second, machine-oriented syntax** (SimPy's dual grammar). Two spellings of every program is P1's worst case
  and doubles what editions migrate.
- **R7. A home-grown semantic diff and merge.** Mergiraf merges on syntax trees and adds languages declaratively [S24]
  (also Spork, GumTree, difftastic; Unison merges by hash), and `sisu fmt` removes formatting noise. Contribute a Sisu
  definition to Mergiraf, and put only the hash's rename and move classification in `sisu review`.

## Sources

† checked in this session; unmarked items are from prior knowledge.

- [S1] † Blinn, Li, Kim, Omar, "Statically Contextualizing LLMs with Typed Holes", OOPSLA 2024, arXiv 2409.00921.
- [S2] † Gissurarson, `ollama-holes-plugin`, `ecta-plugin`, https://hackage.haskell.org/package/ollama-holes-plugin
- [S3] Gissurarson, "Suggesting Valid Hole Fits for Typed-Holes", Haskell Symposium 2018.
- [S4] Omar et al., "Live Functional Programming with Typed Holes", POPL 2019; Lubin et al. (Smyth), ICFP 2020.
- [S5] † Zhong, Raghunathan, Carlini, "ImpossibleBench", arXiv 2510.20270, 2025 (ICLR 2026).
- [S6] † METR, "Recent Frontier Models Are Reward Hacking", June 2025,
  https://metr.org/blog/2025-06-05-recent-reward-hacking
- [S7] † deps.dev on Capslock, July 2024, https://blog.deps.dev/capslock/ ; † capcheck,
  https://pkg.go.dev/github.com/git-pkgs/capcheck
- [S8] † https://mozilla.github.io/cargo-vet/audit-entries.html ; † https://github.com/crev-dev/crev
- [S9] † Spracklen et al., USENIX Security 2025; † 2026 replication, alphaxiv 2605.17062.
- [S10] † Nesbitt, "Package managers need to cool down", nesbitt.io, 2026; pnpm 10.16; uv 0.9.17; classmethod.jp.
- [S11] † Wiz on s1ngularity, August 2025, https://www.wiz.io/blog/s1ngularity-supply-chain-attack
- [S12] † Lam, Dietrich, Pearce, "Putting the Semantics into Semantic Versioning", Onward! 2020, arXiv 2008.07069.
- [S13] † Mezzetti, Møller, Torp (NoRegrets), ECOOP 2018.
- [S14] † Rombaut, Cogo, Hassan, on Dependabot compatibility scores, arXiv 2403.09012, 2024.
- [S15] † https://pkg.go.dev/golang.org/x/tools/go/analysis/passes/inline ; † https://dart.dev/tools/dart-fix
- [S16] † https://buf.build/docs/breaking/rules/ ; † https://github.com/firecracker-microvm/versionize
- [S17] † https://github.com/in-toto/attestation/blob/main/spec/predicates/test-result.md
- [S18] † Gray, Crichton, Krishnamurthi, PLDI 2025, arXiv 2504.18704; † Ko, Myers, "Debugging Reinvented", ICSE 2008.
- [S19] † Mündler et al., arXiv 2504.09246 (§4, §6, app. B); † MoonBit, LLM4Code 2024; moonbitlang.com/blog/moonbit-ai
- [S20] † Moumoula et al., arXiv 2503.13620; † Wu et al., arXiv 2607.22807; † Pan et al., ICSE 2024.
- [S21] † Sun et al. (SimPy), ISSTA 2024, arXiv 2404.16333; † Alderson, martinalderson.com, January 2026.
- [S22] † vericoding, arXiv 2509.22908; † AlgoVeri, 2602.09464; † Clover, 2310.17807; † nl2postcond, FSE 2024.
- [S23] † https://agent-trace.dev/ ; † Linux `Assisted-by:` policy,
  https://itsfoss.com/news/linux-ai-coding-assistants-policy/
- [S24] † https://mergiraf.org/
- Prior knowledge: Eiffel; Debian autopkgtest; crater; cargo-semver-checks; FoundationDB; TigerBeetle; Antithesis; rr;
  Shuttle; Django; OTP; Kotlin; OpenRewrite; llms.txt; MTOB; Spec Kit; Kiro; PEPs 238, 456; Spork; Verus.
