# Unison

## Snapshot
| | |
|---|---|
| First appeared | Unison Computing founded February 2018 (Boston, public benefit corporation); first alpha August 2019; Strange Loop talk 2019 [1]. |
| Designed by | Paul Chiusano, Rúnar Bjarnason, Arya Irani and the Unison Computing team [2]. |
| Current version (Oct 2026) | Unison 1.0 released 25 Nov 2025 ("the language, distributed runtime, and developer workflow have stabilized"); 1.0.2 on 6 Jan 2026; 1.1.0 on 29 Jan 2026 (new MCP tools, VS Code features, Argon2id builtins) [1][3]. |
| Governance | Company-led open source (MIT); Unison Cloud itself open-sourced 8 Oct 2026; a consulting group was announced Feb 2026 [3][4]. |
| Typing | Static, HM-style inference with *abilities* (algebraic effects) in function types; structural types and abilities. |
| Memory | Garbage-collected runtime (interpreter; runtime optimisations landed June 2025) [1]. |
| Compilation | Code is typechecked once and stored as hashed syntax trees in a SQLite codebase; executed by an interpreter; no builds in the conventional sense [1][5]. |
| Primary domains | Distributed systems and cloud services (Unison Cloud, Volturno stream processing) [1]. |
| Notable users | Unison Computing's own cloud; stats on the 1.0 page: 6.2k GitHub stars, 152,459 library downloads, 139,811+ published definitions, 1,300+ project authors [1]. |

## Stated design goals
"Let's identify a definition by its actual contents", so that "we never
compile the same code twice" [1]. The 1.0 page's reason for a new language
rather than a library: "You don't build a rocket ship out of old cars, you
start fresh" [1]. The big idea page: "Each Unison definition is identified by
a hash of its syntax tree", with 512-bit SHA3; "names are just separately
stored metadata that don't affect the function's hash" [5].

## The central bet: content-addressed code

Before hashing, named arguments become positional references and
dependencies are swapped for their hashes, so "the hash of `increment`
uniquely identifies its exact implementation and pins down all its
dependencies" [5]. Everything else follows from immutability of definitions:

- **No builds, perfect incremental compilation.** Definitions "never change",
  so they are parsed and typechecked once; the cache "is part of the Unison
  codebase format"; tests of pure code are cached by hash: "There's no need to
  rerun a deterministic test if none of its dependencies have changed!" [5].
- **No dependency conflicts.** "dependency conflicts and the diamond
  dependency problem are just not a thing"; two versions of a type "exist as
  different types, with different hashes" and coexist [5].
- **Renames are free and never break anything.** "instant non-breaking
  renames"; "A Unison codebase is never in a broken state, even midway through
  a refactoring", because the old hash still exists; changing a type produces
  a "tidy todo list" of dependents [5]. Name collisions are shown with hash
  suffixes (`foo#hash1`) and still compile [6].
- **Distributed execution by hash.** "arbitrary computations can just be moved
  from one location to another, with missing dependencies deployed on the
  fly": the sender ships bytecode and the recipient requests any hashes it
  lacks [5]. Durable storage of closures: "any value at all (including
  functions or values that contain functions) can be persisted" [5].

### Abilities (algebraic effects)
"Abilities are Unison's implementation of algebraic effects, as they're known
in the literature" [7]. Effects appear in arrows: `Text ->{IO} Nat`,
`'{IO, Exception} ()`; a plain `A -> B` is shorthand for `A ->{e} B` [8].

```unison
structural ability Store v where
  get : v
  put : v -> ()

modifyStore : (v -> v) ->{Store v} ()
modifyStore f =
  v = get
  put (f v)

storeHandler : v -> Request (Store v) a -> a
storeHandler storedValue = cases
  {Store.get -> k}   -> handle k storedValue with storeHandler storedValue
  {Store.put v -> k} -> handle k () with storeHandler v
  {a}                -> a
```

Handlers pattern-match on requests `{A.op args -> k}` and `handle e with h`
installs them [8]. Abilities "record in the type system what a piece of code
needs to run"; Unison Cloud is built on a `Cloud` ability whose mock handlers
"can test a full multi-service deployment locally" [6]. LWN notes that
sandboxing untrusted code with abilities has "lurking holes" [6].

### The cost: a codebase manager instead of text files
"Unison code is not saved as text-based file content"; the codebase is an
append-only log in SQLite [1][9]. The workflow: you edit *scratch files*
(`.u`) in any editor; UCM watches them ("I'm watching for changes to .u files
under ~/unisonCode"), typechecks on save, runs `>` watch expressions and
`test>` tests inline, and `update` "Adds everything in the most recently
typechecked file to the namespace, replacing existing definitions having the
same name" and "attempts to update all the existing dependents accordingly";
`edit foo` "prepends the pretty-printed definition" back into the scratch file
with the tool's own formatting and precise `use` statements; a `---` line is a
fold the tool ignores [9]. Consequently git diffs, grep, blame and ordinary
code review do not apply to the codebase: "Unison can push code to a Git
repository, but the stored format isn't human-readable"; "Because existing
tools can't read Unison's database, Share had to be built from scratch" [6].
The company therefore built the whole stack: Unison Share (registry, forge,
code browser, tickets, pull requests), native version control (projects,
branches, clone/push/pull/merge, "no Git required"), UCM Desktop, an LSP
(2022), a `diff.update` command (Jan 2026), "git-style diff integration" (Oct
2025), "branch history comments" (Nov 2025), type-based search across the
ecosystem and an MCP server for AI agents (Aug 2025) [1][3].

## Strengths

### The build/dependency problem genuinely disappears
No other language in this survey can claim zero builds, zero version
conflicts and free renames; Unison achieves all three from one decision [5].
This is strong evidence that hash-identity is the right identity for code.

### Typed effects in production
Abilities are the only typed algebraic-effect system with a commercial product
on top (see `koka-effekt.md`). "Deploy to the cloud with a function call",
"Your access to storage is statically typed and checked by Unison's
typechecker" [10]. Cloud BYOC (Oct 2025) runs on any container infrastructure;
Unison Cloud's worker (Nimbus) is itself written in Unison [4].

### Caching of tests and incremental everything
Hash-keyed caching of typechecking *and* test results is a real productivity
win that conventional build systems approximate badly [5][9].

### A disciplined, long-running project that shipped 1.0
Timeline from 2018 to 1.0 in 2025 with a "no-breaking-upgrades policy" noted
by LWN [1][6].

## Weaknesses

### Breaks every existing tool and workflow
grep, git diff/blame/bisect, GitHub review, editors' project views, CI
caches, and every "find usages" tool in existence must be replaced by UCM
equivalents; the company spent seven years building them [1][6]. Adoption is
gated on accepting a bespoke toolchain, which LWN flags together with language
isolation: Unison "has no stable foreign-function interface, so existing
functionality often must be reimplemented" (a C FFI is on the 1.0 roadmap)
[1][6].

### Upgrades still require the dependents to be present
"The automatic update process assumes affected code is available locally";
the partial answers are small libraries, ability-based interfaces, and
publishing patches [6]. Content addressing removes the *diamond* problem but
not the *propagation* problem.

### Performance and runtime maturity
LWN cited "performance problems, occasional runtime bugs, and unstable
interfaces" pre-1.0 [6]; the runtime is an interpreter with optimisation work
in 2025 and no published benchmarks [1]. This is not a systems language.

### Hash identity ignores names, which humans need
Two people writing the same function get the same hash, which is a feature,
but code review, documentation and error messages must all re-attach names
after the fact; the tooling handles it, text does not [6].

## Designer retrospectives
- The 1.0 announcement frames the first seven years as needed to reach the
  point where "the language, distributed runtime, and developer workflow have
  stabilized"; the roadmap still lists "C FFI, improved record types, improved
  library management, faster codebase sync" [1].
- Open-sourcing Unison Cloud (Oct 2026): the team believes the technology
  "will be more useful to the world as an open source technology" and now
  sells professional support and consulting [3][4]. Read together with the
  consulting-group announcement, this is a signal that the product-led path
  did not scale on its own; it is not a statement of failure.
- Successive UCM releases added `diff.update` and git-style diff "so you can
  check if you really want to run that update command before committing to
  it" [3], an admission that the original no-diff workflow lost information
  developers relied on.

## Influence
- **Took from**: Haskell (syntax, purity, type inference), Frank/Koka
  (abilities are algebraic effects with handlers), Nix and Git (content
  addressing of artefacts), Smalltalk images (code as a database rather than
  files), Erlang (ship code to nodes).
- **Gave to**: No language has copied the codebase-manager model. The
  content-hash idea has been copied at the *tooling* level by everyone
  (Bazel/Nix/Turborepo remote caches, Deno and Go module checksums); Unison's
  contribution is pushing it down to the definition level. Scrapscript and
  Dark are smaller experiments in the same space (not verified in detail).

## Lessons for Sisu
- **Adopt**: Hash of the *normalised* AST (names and formatting stripped,
  dependencies replaced by their hashes) as the compilation-cache key and the
  identity used for incremental compilation and test caching. Text files
  remain the source; the hash is derived. This buys Unison's "never compile
  the same code twice" and cached tests without a codebase manager.
- **Adopt**: Hash-based dependency identity in the package system, so two
  versions of a library can coexist in one program as distinct hashed
  definitions and the diamond problem disappears. Names map to hashes in a
  lockfile.
- **Adapt**: Abilities show typed effects can carry a cloud platform; Sisu
  should study the `Cloud` ability as the model for "the platform is an effect
  interface with real and mock handlers" (compare Roc's platform split in
  `roc.md`).
- **Adapt**: "Never broken mid-refactor": Sisu's compiler can keep the last
  good hashed definition of a dependent while a type change propagates, giving
  a todo list instead of a wall of errors, without abandoning files.
- **Avoid**: Replacing text files. Unison proves the cost is seven years of
  rebuilding the entire tool ecosystem and a permanent adoption barrier; the
  benefits are available from derived hashes.
- **Avoid**: Shipping without an FFI. Language isolation plus no FFI forces
  rewriting the world; Unison is only now adding a C FFI.
- **Open questions raised**: (1) How much of Unison's "free rename" and
  "coexisting versions" survives when source files are the truth and hashes
  are derived (renames still touch text; coexisting versions need namespacing
  in error messages)? (2) Can hash identity be extended to typed effects and
  platform handlers to make remote execution safe, as Unison Cloud does, in a
  language with an FFI and unsafe code?

## Sources
1. Unison 1.0 announcement and timeline, https://www.unison-lang.org/unison-1-0/ (Nov 2025).
2. LWN, "Unison: a new language with content-addressed code" (designers named), https://lwn.net/Articles/978955/ (2024).
3. Unison blog index (1.0.2, 1.1.0, consulting group, Cloud open source), https://www.unison-lang.org/blog/ (2026).
4. P. Chiusano, "Unison Cloud is now open source", https://www.unison-lang.org/blog/unison-cloud-open-source/ (8 Oct 2026).
5. Unison docs, "The big idea", https://www.unison-lang.org/docs/the-big-idea/ (2026).
6. LWN article as in [2]: UCM, git interaction, abilities, criticisms.
7. Unison docs, "Abilities" (fundamentals), https://www.unison-lang.org/docs/fundamentals/abilities/ (2026).
8. Unison language reference, "Abilities and ability handlers", https://www.unison-lang.org/docs/language-reference/abilities-and-ability-handlers/ (2026).
9. Unison docs, "Tour" (scratch files, watch expressions, update/edit), https://www.unison-lang.org/docs/tour/ (2026).
10. Unison Cloud product page, https://www.unison.cloud/ (2026).
