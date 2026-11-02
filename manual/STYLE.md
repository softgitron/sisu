# Style guide

The rules for writing the manual's pages. [README.md](README.md) has the
rules for pages and files; where this guide is silent, Google's
developer documentation style guide decides. The model is
[tutorial/01-hello.md](tutorial/01-hello.md).

## Sentences

- Second person, present tense, active voice: "Run the tests", not "The
  tests can be run".
- Short sentences with one idea each, and plain words. A term is used
  only after the page has introduced it or linked to where it is.
- British spelling, as in the proposal: behaviour, colour, initialise.
- No em dashes and no en dashes. Use a colon, a comma, parentheses or
  two sentences; write ranges as "1 to 3".
- Headings in sentence case with no full stop, each saying what the
  section teaches ("Naming a value", not "`let`").
- No directional words such as "above", "below" or "on the left": name
  the section or the line. Link text makes sense on its own; never
  "here".
- Leave out "simply", "just", "easy", "obviously", "please", exclamation
  marks, and jokes or cultural references that do not travel. A Finnish
  word in an example gets its meaning the first time.
- No names of AI models or AI tools anywhere, in pages or in code.

## Terms

Use the proposal's names, as in this table, and the spoken forms in the
last rows when reading code aloud in prose.

| Write | Not | Notes |
|---|---|---|
| binding (`let`, `var`) | assignment that creates a name | "name" is fine in chapter 1 |
| function, method, associated function | static method | `Point.origin()` is an associated function |
| parameter convention: borrow, `mut`, `own` | pass by reference, pass by value | a plain parameter is a read-only borrow |
| move, copy, `.copy()`, `Copy` type | clone, deep copy | |
| drop, `Drop` | destructor, finaliser | |
| reference (`ref T`), borrow, view, `view struct` | pointer, lifetime | references are second-class |
| `Shared`, `Weak`, `Cell`, `Arena[T]`, `Key[T]` | `Rc`, garbage collection | |
| error set, error, panic | exception, throw, catch | panics are bugs, errors are values |
| `T?`, an optional value, `Some`, `None` | null, nil, nullable | |
| `T ! E`, a result | exception specification | |
| typed hole, `todo("...")` | stub, placeholder | |
| capability, root capability, attenuation, fake, interceptor | permission, mock, injection | |
| task, scope, handle, cancellation | thread, goroutine, coroutine, `async` | |
| trait, `impl`, derive | interface, class, inheritance | |
| compile-time evaluation, `comptime`, reflection | macro, template | |
| contract, `requires`, `ensures`, `invariant`, refinement type | assertion library, precondition comment | |
| diagnostic, code (E0200), identifier (`resolve.undefined-name`), fix | compiler error message | the code is the identity |
| package, module, manifest (`sisu.toml`), lockfile | crate, library file | |
| worksheet, script, REPL session | notebook | Sisu has no notebooks |
| runtime profile (`full`, `single`, `freestanding`, `sim`), build profile (`debug`, `release`), teaching profile | mode | three different things |
| standard output, standard error | console | |
| `T ! E` is read "T or error E"; `x?` "x, or return its error"; `a ?? b` "a, or else b"; `x?.f` "f of x, if x is present"; `xs[^1]` "the last element of xs" | | spoken forms |

## Code

- Sisu code is what `sisu fmt` writes and follows
  [proposal/02](../proposal/02-syntax.md): `snake_case` for values,
  functions, fields and modules, `CamelCase` for types, traits and
  variants, `SCREAMING_CASE` for constants. The checker enforces the
  formatting.
- Prefer the Python-like spelling where Sisu has one: `len(xs)` and
  `sorted(xs)` at the start of an expression, methods at the end of a
  chain, `print("{a} {b}")`, a comprehension where a Python programmer
  would write one, `T?` and `T ! E` rather than `Option[T]` and
  `Result[T, E]`.
- An example is a whole file with a `main`, tests, or a worksheet's
  first line, never a fragment. It starts with a comment that says what
  it shows, is at most 40 lines, and keeps lines to 72 columns (80 is the
  hard limit).
- One concept per example. Names come from the problem (`greeting`,
  `word_counts`), never `foo` or `bar`; the proposal's people (Aino,
  Eero, Roni) are welcome.
- Code that touches files, the clock or the network takes capabilities
  and is shown with fakes. No example reaches the real network.
- No `unsafe`, `expect`, `unwrap` or `panic` except in the sections that
  teach them; use `?` and `else`.
- Say what an example does in the sentence before it: it prints, it
  does not compile, it fails a test. Show what Sisu prints from the
  example's expectation file or session, never retyped.
- Fences start at column 0, with `sisu` for source, `console` for a
  session and `text` for output, and a marker on the line before each
  ([README.md](README.md#examples)).

## Diagnostics

- Show a diagnostic whole, as Sisu prints it, from a session
  (`NAME.console`). Do not trim it, colour it or retype it.
- Name it by code and catalog identifier the first time a page shows it,
  "E0200 (`resolve.undefined-name`)", and by code after that. The
  English text can be reworded or translated; the code and the
  identifier cannot, so prose never relies on the exact wording.
- Read the `fix` line for the reader: what it changes and what it costs.
  Where a diagnostic offers several fixes, say when each is right.
- Explain the parts of a diagnostic (code, message, place, fix) once, in
  chapter 1; later pages point there.
- Every diagnostic a page shows has an entry in the reference's
  diagnostics page with a wrong program and its fix; add it there when
  it is missing.
- When a diagnostic is unhelpful (no fix, a misleading message), do not
  hide it with prose. Show it, say what it means, and add it to the list
  of findings in [README.md](README.md#what-the-plan-found-in-stage-0)
  for the compiler's maintainers.
- A replay line (`replay: sisu test --replay ...`) is shown but never
  explained as meaningful text; say what it is for.

## Features and their status

- The tutorial teaches what the current compiler runs. A feature that is
  designed but not implemented gets one sentence at most, saying that it
  is coming and where it is designed; its section waits for it.
- The reference marks each feature: in stage 0, designed, leaning (with
  its D-xxx), or a prototype of the experiment.
- Refer to decisions and questions by their IDs (D-056, Q-41), which the
  documentation checker verifies, and to the proposal by its documents.

## What not to do

- Do not show a block that nothing checks. An `unchecked:` marker needs
  a reason a reviewer accepts, such as an installation command.
- Do not edit an expectation or a session by hand; bless it and read the
  diff.
- Do not use a concept before the page that teaches it, beyond the one
  sentence an exercise's signature needs.
- Do not compare Sisu with another language to mock it. Say what Sisu
  does and, in a **From Python.** note, what the habit would do.
- Do not write an exercise that special-cased inputs could pass: test at
  least two cases with different values, and keep the same tests in the
  solution.
- Do not put the answer's code in a hint. The last hint names the idea.
- Do not carry meaning by colour, bold, position or emoji, and do not
  show images of code or of terminal output.
- Do not paste the proposal's rationale into the tutorial; link to the
  explanation that discusses it.
