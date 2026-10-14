# The conformance suite

Language-agnostic tests for any Sisu implementation
([18](../../proposal/18-next-steps.md), "Bootstrapping": "input files,
expected SIR, expected diagnostics by catalog identifier"). The runner
drives an implementation only through the `sisu` command line, so the
stage-1 compiler written in Sisu is checked by the same files as the Rust
stage 0.

Run it with `cargo test -p sisu-driver --test conformance` (the stage-0
`sisu` is built first), or against any executable:

```text
cargo run -p sisu-conformance -- --sisu path/to/sisu conformance/cases
```

`--bless` (or `SISU_BLESS=1` for the cargo test) rewrites the expected
files from what the implementation printed instead of failing. Review the
diff before committing a blessed file: blessing records behaviour, it does
not check it.

## A case

A case is a file `NAME.sisu` anywhere under `cases/`. Files beside it with
the same stem state what an implementation must print. Every expectation
is compared byte for byte.

| File | Compared with |
|---|---|
| `NAME.diag` | `sisu check --format=ids NAME.sisu`; absent means no diagnostics |
| `NAME.out` | standard output of `sisu run NAME.sisu ARGS...` |
| `NAME.err` | standard error of that run; absent means empty |
| `NAME.status` | exit status of that run; absent means `0` |
| `NAME.args` | not compared: the run's arguments, one per line |
| `NAME.files/` | not compared: copied into the run's working directory |
| `NAME.sir` | `sisu sir NAME.sisu`, the serialised SIR |
| `NAME.test` | standard output of `sisu test NAME.sisu` |
| `NAME.tokens` | `sisu tokens NAME.sisu` |
| `NAME.tree` | `sisu parse --tree NAME.sisu` |
| `NAME.defer` | `sisu run --defer-borrow NAME.sisu ARGS...`: standard output, then `--- stderr`, standard error, and `--- status N` |
| `NAME.explain` | `sisu explain-borrow NAME.sisu:N`, where `N` is given by a line `# explain-borrow N` in the case |
| `NAME.explore` | `sisu test --explore N NAME.sisu`, where `N` is given by a line `# explore N` in the case |

A line `# explore N` also runs a case that runs again as `sisu run
--explore N NAME.sisu ARGS...`: the program under the schedule seeds 0 to
N-1, each of which must give the expected output and status.

A case is run when any of `.out`, `.err` or `.status` exists.

Every command runs in a fresh temporary directory holding only
`NAME.sisu` and the contents of `NAME.files/`, with an empty environment,
and names the file by its bare name, so paths in messages are the same on
every machine. A program that writes files writes them there.

Three checks apply to every case without an expectation file:

- `sisu parse --print NAME.sisu` reproduces the file byte for byte (the
  syntax tree is lossless).
- For a case that runs, the SIR is a complete program: `sisu sir` written
  to a file and run with `sisu run --sir` gives the same standard output,
  standard error and exit status as running the source (minus any
  compile-time warnings that precede the program's own output).
- For a case that runs, `sisu run --monitor` (the dynamic ownership
  monitor) prints exactly what `sisu run` printed and exits the same way.
  A case that runs was accepted by the ownership checker, so any
  difference is the monitor stopping a program the checker accepted: a
  soundness hole in the checker, which fails the suite (18, principle 5,
  "the checker checks itself").

The summary line counts the cases, the checks, the runs repeated under
the monitor, the counterexample runs (`.defer`), and the cases explored
under several schedules with the number of schedules.

## The diagnostic form

`--format=ids` prints one line per diagnostic, then one indented line per
label and fix, naming messages by code and catalog identifier, never by
English text:

```text
3:11-3:16 E0200 resolve.undefined-name name="cuont"
  fix resolve.similar-name 3:11-3:16 "count"
```

Positions are `line:column-line:column`, 1-based, columns counted in
characters. Parameters follow in the order the implementation gives them;
the catalog (`compiler/catalog/messages.catalog`) defines which
parameters each identifier takes.

## Groups

- `examples/` holds copies of `proposal/examples/hello.sisu` and
  `wordcount.sisu`, kept identical to the originals by a test, and of
  `traits.sisu`, `errors.sisu`, `capabilities.sisu` and
  `concurrency.sisu` with what they leave undeclared and a `main` (or a
  test) appended, kept starting with the originals.
- `syntax/` is lexing and parsing, with the error-tolerance cases: one
  mistake gives one diagnostic, and the rest of the file is still parsed.
- `names/` is name resolution; `types/` is the checker, including cases
  where broken code still gets a typed tree and only the first error in a
  chain is reported.
- `run/` is the semantics: arithmetic and its traps, strings and
  formatting (`format_specs`: the spec mini-language, written in Sisu),
  collections, `match`, loops, closures, errors and traces, options,
  `Drop` order on normal and unwinding paths (`drop_panic_fields`: a
  `Drop` that panics; `drop_chain`: a long chain dropped at constant
  depth), panics and their exit statuses, capabilities, and tests.
- `traits/` is traits and impls (18, Phase 4): provided methods,
  associated types, supertraits, coherence and the orphan rule, sealed
  traits, method resolution and ambiguity, `dyn Trait` and object safety,
  operator traits, `FromLiteral`, derives and delegation, and their
  diagnostics.
- `generics/` is user generics: functions and types with bounds checked
  at the definition, defaults, type arguments at run time, views through
  generic parameters, and `Drop` for a generic type.
- `comptime/` is compile-time evaluation: constants computed by the
  compiler (`baked` pins the SIR built from literals), `comptime assert`,
  `comptime def`, budgets, the evaluation errors, and derives written as
  compile-time Sisu, with `proposal_derive`, the user-defined derive of
  `proposal/examples/comptime.sisu`, kept identical by a test.
- `sir/` pins the serialised SIR of small programs.
- `concurrency/` is tasks (18, Phase 6), most cases under several
  schedules: joins and their error sets, failures and the scope's result,
  panics in tasks, cancellation at the blocking points, shields and
  deadlines, channels, `select`, races, mutexes across tasks with guards
  dropped by unwinding, the lowest-index rule of `pure` scopes, deadlock
  reports, generators, `log`, and the diagnostics of `Send`, `Sync` and
  what a task may borrow.
- `capabilities/` is std.cap (06): attenuations, lent capabilities and
  their diagnostics, caretakers, interceptors, the fakes, the test clock,
  the fresh fakes of each test and the real `System` a test may declare
  (and the signatures it may not), the network over `Net.fake`, the
  host's objects, `DateTime`, and what the capability rules reject.
- `http/` is std.x.http over the in-process network, under several
  schedules (a test of the driver runs it under a thousand).
- `ownership/` is the ownership checker (18, Phase 3). Accepted programs
  run (and so run again under the monitor): moves and partial moves, drop
  placement on every path (`drop_torture` is the drop-order torture
  program), the parameter conventions, non-lexical and two-phase borrows,
  returned borrows and `from`, views, closures, the three iteration modes,
  `@must_consume`, the library types (`Box`, `Shared` and `Weak`,
  `Cell`, a `Mutex` with D-077's poisoning, `Arena` with generational
  keys), `own` closures, and `proposal_core`, `proposal_shared` and
  `proposal_graph`, the sections of `proposal/examples/ownership.sisu`,
  kept identical to the original by tests. Rejected programs pin their diagnostics by code
  and span, and most also pin their counterexample run (`.defer`): the
  monitor's concrete conflict, or "no conflict" with the account of why
  the checker still rejects them (`candidate_false_positive` and
  `untested_path` show the two other outcomes). `explain_borrow` pins
  `sisu explain-borrow`.
