# Capabilities, distribution and resources

## Why this family matters for Sisu

Sisu already has this family's core: capabilities handed down from `main`,
`unsafe` gated by `[trust]`, `@must_consume` types and `Shared[T]`
([proposal/06](../../proposal/06-concurrency-and-capabilities.md),
[proposal/04](../../proposal/04-memory-and-ownership.md)). These systems
answer the next questions: how authority is narrowed, revoked and loaded;
whether the discipline survives a process boundary; and what resource types
look like where adversaries attack them for money. Austral, Pony, Unison,
Koka/Effekt and Idris are in [`../languages/`](../languages/).

## Systems covered

| System | Years | Status (Oct 2026) | Central idea |
|---|---|---|---|
| E | 1997- | Dormant; ideas live on in JS, Cap'n Proto, Agoric | Object capabilities, vats, eventual sends, promise pipelining |
| Monte | 2015- | Small, community-maintained | E semantics with Python-like syntax; auditors (`DeepFrozen`) |
| Joe-E; Caja | 2008-2010; 2008-2021 | Research artefact; archived January 2021 | Capability-safe subsets of Java (verifier) and JavaScript (rewriting, then freezing ES5) |
| SES / Hardened JS (Endo) | 2009- | Shim used by Agoric, MetaMask; TC39 proposals at Stage 1 | `lockdown()`, `harden()`, Compartments with endowments |
| Wyvern | 2013- | CMU research language | Pure modules imported freely; resource modules passed in |
| Cap'n Proto, Workers RPC, Cap'n Web | 2013-; 2024; 2025 | Production at Cloudflare | E's CapTP: capability references, promise pipelining |
| Spritely Goblins, OCapN | 2019-; 2023- | Active; OCapN pre-standard | CapTP for Scheme; transactional vats; a shared protocol |
| Cadence (Flow) | 2019- | 1.0 released October 2024 | Resource types `@T`, `<-` moves; entitlements |
| Move (Diem, Aptos, Sui) | 2019- | Two forks; original repository archived May 2024 | Abilities `copy`/`drop`/`store`/`key`; Move Prover |
| Solidity | 2014- | Dominant smart-contract language | Counter-example: synchronous calls into untrusted code |
| Clean | 1987- | Small, still released | Uniqueness types for I/O and in-place update |
| Cyclone | 2001-2006 | Unsupported | Safe C with regions, never-null and fat pointers |
| ATS | 2006- | ATS2 last released 2020; ATS3 in development | Dependent plus linear types with proofs |
| Hume | 2000-2008 | Inactive | Boxes and wires with bounded time and space |
| Newsqueak, Alef, Limbo | 1989-1997 | Alef dropped 2000; Inferno kept by enthusiasts | Typed channels; typed modules loaded at run time |
| Oz/Mozart; Erlang | 1991-; 1986- | Mozart 2 alpha; Erlang in production | Network transparency (Erlang: [erlang-elixir.md](../languages/erlang-elixir.md)) |

## The ideas

### Attenuation, revocation and membranes

In an object-capability (ocap) language a reference *is* the permission,
and the security patterns are plain objects: a *facet* exposes part of an
interface; a *caretaker* forwards calls until revoked; a *membrane* is a
transitive caretaker that wraps every reference crossing it, so a whole
subsystem is cut off at once; a *sealer/unsealer* pair passes a box only
its unsealer opens. ES2015 standardised the caretaker as
`const { proxy, revoke } = Proxy.revocable(api, {})`, part of the Proxy API
designed by Van Cutsem and Miller [10]; after `revoke()`, every use of
`proxy` throws.

*Evidence.* Revocation is what Austral's author misses when he calls his
design "good—but not good enough": a linear capability can be surrendered,
never taken back ([austral.md](../languages/austral.md)). Joe-E's checked
immutability let Waterken (13K lines of Joe-E, 4K of Java) stay correct "so
that we don't need to review the client code" [6]. *Failure.* Cadence shows
the cost of attenuating by *type* rather than by *object*: 1.0 replaced
restricted interface types with entitlements (`auth(Withdraw) &Vault`)
because "It was too easy to accidentally give out a `Capability` with a
more permissible type than intended" [18]. Membranes remain a
JavaScript-sandbox technique, where dynamic typing leaves interposition as
the only tool [inference]. *Sisu today* attenuates (`restrict_to`,
`read_only`, `only_hosts`) but cannot revoke, and capabilities are `Copy`:
a library given `Fs` may keep a copy, bounded by the caller's object graph
(there are no globals) but never withdrawn.

### Capability-safe subsets and the cost of taming

Joe-E (Mettler, Wagner, Close; NDSS 2010) made Java an ocap language with a
verifier and no new syntax: no native methods, no mutable throwables, no
`finalize()`, no catching `Error`, no `finally`, static fields final and
immutable, plus *taming*, a whitelist of library members that confer no
ambient authority [6]. Caja did the same for JavaScript, first by rewriting
(ES5/3, a translator to ES3), then by freezing an ES5 realm (SES) [7].

*What it buys: determinism.* Joe-E code cannot reach ambient
non-determinism, so Waterken computes ETags from "the hash of the inputs to
the response text generator" and aborts any GET that mutates, making cache
hits coherent by construction [6]: the earliest deployment of Sisu's
argument that no capabilities and no `unsafe` imply purity, and purity
makes caching sound ([incremental-reversible-and-exotic-paradigms.md](incremental-reversible-and-exotic-paradigms.md)).

*Where it failed: taming.* Joe-E's authors call it "a time-consuming and
difficult task, and a place where a mistake could violate soundness"; the
DarpaBrowser review found methods "inadvertently allowed" in E's own taming
database; `File.getParentFile()` walks to the root and `new File(null,
path)` makes a relative path absolute [6]. Caja broke with browser
releases: a 2015 advisory records SES mode failing on Chrome 40 and Opera 27
over a non-configurable `%Generator%.constructor` [7]. Google archived Caja
on 31 January 2021, citing "several security vulnerabilities" [7]. The
SES shim "starts where the Caja project left off" and sandboxes plugins for
Agoric and MetaMask [8]; in October 2026 TC39's Compartments, SES and
Stabilize proposals are at Stage 1 and ShadowRealm at 2.7 [9]. Monte, E's
Python-syntax heir, adds *auditors*: `DeepFrozen` "proves that objects are
immutable and that the objects they refer to are also `DeepFrozen`" [5], a
run-time cousin of Sisu's checked `pure`. *For Sisu:* starting
capability-safe removes taming except for `extern` bindings in `[trust]`
packages, its taming database; Joe-E's advice holds: keep it "small, but
sufficient" [6].

### Authority-aware modules and loading

Wyvern (Melicher, Shi, Potanin, Aldrich; ECOOP 2017) splits modules in
two. A *resource module* encapsulates a system resource, uses another
resource module, or holds mutable state, and is a functor receiving its
capabilities as arguments; a *pure module* has none of these, and "any
module can import any pure module" [11]:

```wyvern
module def logger(io : FileIO) : Logger
    def appendToLog(entry : String) : Boolean
        io.open("~/log.txt").append(entry)
```

SES Compartments do this at run time: each has its own `globalThis` filled
from a `globals` endowment, imports routed through `resolveHook` and
`importHook` [8]. Limbo loaded typed modules at run time (`sys = load Sys
Sys->PATH;`) [32], but any module could load `Sys`; authority came from
Inferno's per-process namespaces [unverified]. *Evidence:* Wyvern proves
authority safety, and pure modules avoid Newspeak's
every-dependency-is-a-parameter boilerplate [11]. *Limits:* a trusted
module can re-export a resource through a wrapper (`logger.write(fileName,
text)` calling `io.write`), and "this would not be exposed in the logger's
type or interface" [11]. *Sisu today* draws the pure/resource line per
function, not per package, and cannot safely run code that arrives after
compilation (Q-31).

### Vats, turns and reentrancy

Miller's thesis, *Robust Composition* (Johns Hopkins, 2006), argues that
access control and concurrency control are one problem [1]. Every E object
lives in a *vat* (one thread, one queue); immediate calls (`.`) reach only
the same vat; eventual sends (`<-`) reach anything and return a promise;
each message is a *turn* that runs to completion [2]. A turn cannot be
re-entered mid-update, the hazard the thesis calls *plan interference* [1]:

```e
def balance := bank <- balanceOf(alice)    # eventual send: a promise, at once
when (balance) -> { println(`alice has $balance`) }
```

*The counter-example.* On 17 June 2016 recursive calls drained 3.6 million
Ether (a third of the 11.5 million committed, about $50 million) from The
DAO; Ethereum hard-forked on 20 July 2016 [24]. Solidity's external call
"hands over control to that contract (B)", which can "call back into A
before this interaction is completed" [25]; the fixes are conventions
(Checks-Effects-Interactions, the 2300-gas stipend) [25], and Agoric offers
eventual send as its answer [17]. *Move* had no dynamic dispatch, so
reentrancy needed no rule, until Aptos Move 2.2 added function values
("Which concrete function is called is not known to the caller") and a
runtime rule with them: "The Move VM dynamically detects reentrancy of a
module and locks all resources declared in that module" [19].

*Sisu today.* Ownership is a static reentrancy guard in-process: while
`withdraw(mut self, ...)` runs, `self` is exclusively borrowed, so no
callback reaches the vault; the DAO interleaving needs `Shared[Mutex[Vault]]`,
and then re-entry blocks on the lock instead of reading a stale balance:

```sisu
def withdraw[P: Payee](mut self, who: Addr, mut payee: P) -> ! VaultError:
    let share = self.shares.get(who) ?? 0   # the DAO's order: pay first, zero after
    payee.receive(share)?      # payee cannot call self.withdraw: self is borrowed
    self.shares.insert(who, 0)
    return Ok(())
```

Between tasks interleaving returns, and there Sisu's actor pattern
(proposal/06) should copy E's rule: one message per turn, replies as new
messages.

### Promise pipelining and capability RPC

If a remote call returns a promise at once, a call on that promise can be
sent before the first returns: `bar(foo())` costs one round trip, not two.
Liskov and Shrira invented this in 1988 as Argus *call-streams* (never in a
public release); Miller, Tribble and Jellinghaus reinvented it for Xanadu
around 1989; E and Joule made promises first-class [3]. Cap'n Proto RPC "is
based heavily on CapTP", E's protocol: "Cap'n Proto interface references
are capabilities" [12].

*Evidence.* Cloudflare's Workers RPC (5 April 2024) is "built on Cap'n
Proto", "based on the object-capability model", and speculative calls "will
be sent to the server immediately" [13]. Cap'n Web (22 September 2025)
brings the model to browsers, schema-less and under 10 kB, and turns the
security argument into API design: `authenticate()` returns a session
object, so "It is impossible for the client to "forge" a session object"
and nothing needing a session can be called first [15]. Spritely Goblins and
OCapN are standardising an interoperable CapTP, still pre-standard [16].
E's ideas shipped and the language did not: Q brought promises "proven
and vetted in languages like E" to JavaScript and was archived in 2025 [4].

*Costs.* Distributed garbage: each Workers isolate's collector "cannot see
into other isolates", so callers must dispose stubs explicitly
(`using`/`Symbol.dispose`), and every stub is disposed when the handler
finishes [14]; JavaScript needed Explicit Resource Management (ES2027 [9])
for this. *Syntax did not travel:* TC39's distributed pipelining proposal
and its `p~.name(...args)` ("wavy dot") sit at Stage 1 [9], while Workers
RPC and Cap'n Web pipeline through ordinary method calls on promise
proxies. *The opposite bet,* network transparency (Oz/Mozart [33],
Erlang's local-looking pids), is what Waldo et al. argued against in 1994:
latency, partial failure and concurrency make local and remote objects
irreconcilable [34]. E agrees by construction: only `<-` crosses vats.

### Resources as linear types for assets

Money is the one domain where linear types run at scale under attack.
Cadence: "Resources are types that can only exist in one location at a
time and must be used exactly once", written `@Vault`, moved with `<-`,
ended with `destroy`; "A resource object cannot go out of scope and be
dynamically lost" [18]. Move generalises this into four *abilities*:
`copy`, `drop`, `store` (may live in global storage) and `key` (may be a
storage key), as in `struct Pair has copy, drop, store { ... }`; on generic
types they are conditional, so `Cup<T>` has `copy` "only if `T` has `copy`"
[19]. A struct with no abilities must reach a function that destructures it
before the transaction ends, which is how flash loans force repayment:

```move
public struct Receipt { amount: u64 }   // no abilities: no copy, drop or store
public fun repay(p: &mut Pool, c: Coin<SUI>, r: Receipt) { let Receipt { amount } = r; ... }
```

*Evidence.* Conservation of assets becomes a type rule, where every
Solidity token re-implements it with integers in maps. The Move Prover
verified the Diem framework "in a few minutes", and changes "must be
successfully verified before being integrated" [22]. *Failures.* Types
protect the coins, not the arithmetic deciding how many are owed: in May
2025 about $223 million left the Cetus exchange on Sui because "the
implementation of checked_shlw was flawed", an overflow check on a 64-bit
shift [23]. The original repository was archived on 4 May 2024 in favour
of Aptos and Sui forks [21]; Sui's 2024 edition added methods, enums,
macros and `let mut` [20], and Aptos Move 2.0-2.4 enums, function values
and signed integers [19]: both converged on what general-purpose languages
start with. *For Sisu:* `@must_consume` is Move's absent `drop`, with one
difference: a Move error aborts and rolls back the transaction, so
linearity never meets partial cleanup, while Sisu's `?` returns normally,
where Austral found linearity hostile ([austral.md](../languages/austral.md)).

### Precursors: uniqueness, regions, proofs, bounds and channels

*Clean* (Nijmegen, from 1987) made uniqueness types its I/O model, the
practical form of Wadler's "Linear types can change the world!" (1990)
[26][27]: a `*World` or `*File` has one reference, so it is updated in
place and the program stays pure. Haskell chose monads; the idea returned
as Linear Haskell, Idris 2 and Rust.

```clean
Start :: *World -> *World
Start world
    # (console, world) = stdio world
    # console          = fwrites "Hei\n" console
    # (_, world)       = fclose console world
    = world
```

*Cyclone* (AT&T Labs and Cornell, 2001-2006) gave C regions and never-null
and bounds-checked pointers. Porting changed "Usually fewer than 10% of the
lines" (1,452 of 18,627); I/O-bound programs ran at about 1.0x and
compute-bound ones at up to 2.85x; region analysis was intraprocedural,
with annotations across calls [28]. The Rust Reference credits "ML Kit,
Cyclone" for region-based memory management [29]; Sisu's views continue
the line without annotations. *ATS* (Hongwei Xi, from 2006) adds proofs and
linear views (`T @ L`) to C-like speed, yet is "used mostly for research",
with under 200 GitHub repositories in 2024 against 16,000 for OCaml [30]:
the case for opt-in `@must_consume` and contracts over proofs. *Hume* (St
Andrews and Heriot-Watt, 2000-2008) wires pattern-matching boxes into a
fixed network with static bounds, such as a box `within 500KB (400B)` of
heap and stack [31]; nobody adopted it. *Newsqueak, Alef and Limbo* gave Go
its channels (the Rust Reference credits them for "channels, concurrency"
[29]). Plan 9 dropped Alef because "it proved too difficult to maintain a
variant language across multiple architectures", and Pike blamed its lack
of garbage collection [32]; Limbo added a collector and typed run-time
`load` [32]. Ownership is the answer to Alef's problem.

## Transferable ideas for Sisu

1. **Conditional abilities for `@must_consume` (Move)**, settling Q-37's
   open half: a container of must-consume values is must-consume (as
   `Cup<T>` has `drop` only if `T` does), discharged by consuming
   iteration; `?` out of such a loop is an error unless it returns the rest.
   ```sisu
   @must_consume
   struct Receipt:                 # flash-loan receipt: no Copy, no implicit drop
       amount: int

   def settle(mut pool: Pool, own receipts: list[Receipt]) -> ! RepayError:
       for r in own receipts:      # list[Receipt] is must-consume because Receipt is
           repay(mut pool, r)?     # error: `?` would drop the receipts not yet visited
       return Ok(())
   ```
   *Cost:* small in the checker, real in `std` (consuming container APIs).
   *Conflicts:* D-054 exempts only panics; `?` cannot abort as Move errors
   do. *Confidence:* high for propagation (Move ships it); medium for `?`.

2. **Eventual references and promise pipelining as a library (E, Cap'n
   Proto, Workers RPC).** A `.later` view returns `Promise[T]` values that
   can be called before they resolve, for local actors (with E's turn rule)
   and remote objects; blocking and eventual calls differ in spelling.
   ```sisu
   def count_sessions(net: Net, addr: Addr) -> int ! {RpcError, NotFound}:
       let users = rpc.connect[Users](net, addr)?   # Remote[Users]; Drop releases it
       let user = users.later.lookup("roni")         # Promise[UserRef ! NotFound]
       let n = user.later.sessions()                 # pipelined: same round trip
       return n.join()                               # the one blocking point (task only)
   ```
   `Remote[T]` and `.later` come from the reflection that
   [proposal/17](../../proposal/17-testing.md) plans for `Fake[MyTrait]`;
   `Drop` sends the release Workers needs `using` for. *Cost:* a `std.x`
   library and a wire format (Cap'n Proto or OCapN). *Conflicts:* none
   with "no `async`" (`.later` is an ordinary method); `Remote` cannot be
   `Copy`; cross-machine cycles leak, as with `Shared`. *Confidence:*
   medium (proven model, untested fit with colourless tasks).

3. **Revocable and lend-only capabilities in `std.cap`.** A caretaker gives
   dynamic revocation; a *lent* capability is a `view`, so the callee cannot
   keep it and its authority ends with the call, at zero run-time cost.
   ```sisu
   let plugin_fs, revoker = out.revocable()   # caretaker
   run_plugin(plugin_fs)?
   revoker.revoke()                           # retained copies now fail with Err(Revoked)
   render(out.lend(), report)                 # Lent[Fs] is a view: render cannot store it
   ```
   *Cost:* small; `Lent[T]` reuses the view rules of proposal/04.
   *Conflicts:* capabilities are `Copy` (proposal/06), so APIs must accept
   both forms; `Revoked` joins every I/O error set. *Confidence:* medium
   for revocation (`Proxy.revocable`, ES2015); low-medium for lending.

4. **Pure and resource packages; capability-gated plugins (Wyvern, SES,
   Limbo).** `sisu audit --authority` labels a package *pure* when its API
   takes no capability types and it is not in `[trust]`; the registry
   shows it, and pure packages need no capability review. Because Sisu
   source needs no taming, the interpreter could run an untrusted plugin,
   checked with an empty trust set, holding only what it is handed:
   ```sisu
   let fmt = plugins.load[Formatter]("plugins/md.sisu", fuel=10_000_000)?
   let f = fmt.make(out.lend())     # its whole authority: one lent Fs, plus stdio
   ```
   *Cost:* the interpreter as an embeddable library with comptime's fuel
   and memory limits. *Conflicts:* the "no `eval`" non-goal
   ([proposal/14](../../proposal/14-non-goals.md)); ambient stdio; Q-6 (a
   plugin could spawn without bound: the first concrete case for a `Tasks`
   capability). *Confidence:* medium for the label, low for the loader.

5. **Checked resource bounds for `freestanding` code (Hume).** Expose the
   stack bounds the compiler already computes for tasks (proposal/06):
   ```sisu
   @stack_bound(2048) @no_alloc      # checked over the call graph; recursion is an error
   def on_interrupt(mut uart: Uart, byte: u8): ...
   ```
   *Cost:* low. *Conflicts:* attributes are data
   ([proposal/08](../../proposal/08-metaprogramming.md)); these would be
   checked like `@must_consume`. *Confidence:* low-medium.

## Dead ends

- **Capability-safe subsets of existing languages (Joe-E, Caja):** taming
  is error-prone and tracks every host release; Caja died with holes [6][7].
- **Network transparency (Oz/Mozart, Erlang defaults):** partial failure
  cannot be hidden [34]; copy E's near/eventual split instead.
- **An eventual-send operator (`<-`, `~.`):** still at TC39 Stage 1 in 2026
  while libraries ship pipelining with method syntax [9][13][15].
- **General membranes:** a dynamic-language tool; Sisu's types mark the
  boundary, and caretakers plus scope cancellation revoke.
- **Orthogonal persistence of vats (E, Waterken):** unadopted elsewhere [inference].
- **Pervasive dependent and linear types for systems code (ATS)** [30].
- **Move's `store`/`key` and bytecode verification:** global-storage rules
  for a chain that runs untrusted bytecode; Sisu compiles checked source.

## Sources

1. Mark S. Miller, *Robust Composition: Towards a Unified Approach to Access Control and Concurrency Control*, PhD thesis, Johns Hopkins University, 2006. https://papers.agoric.com/papers/robust-composition/abstract
2. "E (programming language)", Wikipedia (2026). https://en.wikipedia.org/wiki/E_(programming_language)
3. "Futures and promises", section on promise pipelining, Wikipedia (2026). https://en.wikipedia.org/wiki/Futures_and_promises
4. Kris Kowal, Q README (archived 22 January 2025). https://github.com/kriskowal/q
5. "Monte", esolangs wiki (2026), https://esolangs.org/wiki/Monte ; Monte docs, "Auditors". https://monte.readthedocs.io/en/latest/auditors.html
6. A. Mettler, D. Wagner, T. Close, "Joe-E: A Security-Oriented Subset of Java", NDSS 2010. https://www.ndss-symposium.org/wp-content/uploads/2017/09/met.pdf
7. Google Caja README (archive notice, 2021), https://github.com/googlearchive/caja ; security advisory 2015-03-13. https://github.com/googlearchive/caja/wiki/SecurityAdvisory20150313
8. Endo SES README (2026), https://github.com/endojs/endo/blob/HEAD/packages/ses/README.md ; Endo README. https://github.com/endojs/endo
9. TC39 proposals (October 2026): https://github.com/tc39/proposals ; https://github.com/tc39/proposals/blob/main/stage-1-proposals.md ; https://github.com/tc39/proposals/blob/main/finished-proposals.md ; https://github.com/tc39/proposal-wavy-dot
10. T. Van Cutsem, M. S. Miller, "Proxies: Design Principles for Robust Object-oriented Intercession APIs", DLS 2010. https://research.google/pubs/proxies-design-principles-for-robust-object-oriented-intercession-apis/
11. D. Melicher, Y. Shi, A. Potanin, J. Aldrich, "A Capability-Based Module System for Authority Control", ECOOP 2017. https://drops.dagstuhl.de/entities/document/10.4230/LIPIcs.ECOOP.2017.20
12. Cap'n Proto, "RPC Protocol" (2026). https://capnproto.org/rpc.html
13. K. Varda, "We've added JavaScript-native RPC to Cloudflare Workers", 5 April 2024. https://blog.cloudflare.com/javascript-native-rpc/
14. Cloudflare Workers docs, "RPC: Lifecycle" (2026). https://developers.cloudflare.com/workers/runtime-apis/rpc/lifecycle/
15. K. Varda, "Cap'n Web: a new RPC system for browsers and web servers", 22 September 2025. https://blog.cloudflare.com/capnweb-javascript-rpc-library/
16. Spritely Institute, "Introducing OCapN" (2023), https://www.spritely.institute/news/introducing-ocapn-interoperable-capabilities-over-the-network.html ; https://github.com/ocapn/ocapn ; FOSDEM 2026 OCapN talk. https://fosdem.org/2026/
17. Agoric docs, "Eventual Send" (2026). https://docs.agoric.com/guides/js-programming/eventual-send
18. Cadence docs, "Resources", https://cadence-lang.org/docs/language/resources ; "Cadence 1.0 improvements" (2024). https://cadence-lang.org/docs/cadence-migration-guide/improvements
19. Aptos Move Book (2026): https://aptos-labs.github.io/move-book/generics-and-abilities.html ; https://aptos-labs.github.io/move-book/move-versions.html ; https://aptos-labs.github.io/move-book/functions.html
20. The Move Book (Sui), "Move 2024 migration guide". https://raw.githubusercontent.com/MystenLabs/move-book/main/book/guides/2024-migration-guide.md
21. move-language/move README (archived 4 May 2024). https://github.com/move-language/move
22. D. Dill et al., "Fast and Reliable Formal Verification of Smart Contracts with the Move Prover", TACAS 2022, arXiv:2110.08362. https://arxiv.org/abs/2110.08362
23. Halborn, "Explained: The Cetus Hack (May 2025)", 27 May 2025. https://www.halborn.com/blog/post/explained-the-cetus-hack-may-2025
24. "The DAO", Wikipedia (2026). https://en.wikipedia.org/wiki/The_DAO
25. Solidity docs, "Security Considerations" (2026). https://docs.soliditylang.org/en/latest/security-considerations.html
26. P. Wadler, "Linear types can change the world!", IFIP TC 2 Conference on Programming Concepts and Methods, 1990 (not re-fetched). https://homepages.inf.ed.ac.uk/wadler/topics/linear-logic.html
27. "Uniqueness type"; "Clean (programming language)", Wikipedia (2026). https://en.wikipedia.org/wiki/Uniqueness_type ; https://en.wikipedia.org/wiki/Clean_(programming_language)
28. T. Jim et al., "Cyclone: A safe dialect of C", USENIX ATC 2002. https://www.usenix.org/legacy/publications/library/proceedings/usenix02/full_papers/jim/jim_html/index.html
29. The Rust Reference, "Influences" (2026). https://doc.rust-lang.org/reference/influences.html
30. "ATS (programming language)", Wikipedia (2026). https://en.wikipedia.org/wiki/ATS_(programming_language)
31. "Hume (programming language)", Wikipedia (2026). https://en.wikipedia.org/wiki/Hume_(programming_language)
32. "Limbo (programming language)"; "Alef (programming language)", Wikipedia (2026). https://en.wikipedia.org/wiki/Limbo_(programming_language) ; https://en.wikipedia.org/wiki/Alef_(programming_language)
33. "Oz (programming language)", Wikipedia (2026). https://en.wikipedia.org/wiki/Oz_(programming_language)
34. J. Waldo, G. Wyant, A. Wollrath, S. Kendall, "A Note on Distributed Computing", Sun Microsystems Laboratories TR SMLI TR-94-29, 1994 (not re-fetched).
