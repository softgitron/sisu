# Stage 0: the Rust implementation of Sisu

This is the reference for the stage-0 compiler and interpreter in
`compiler/`: what each crate does, the rules each stage follows, the
serialised forms other implementations must read and write, and the
decisions the proposal left open that the implementation had to make. It
is written so that the stage-1 front end in Sisu
([18](../../proposal/18-next-steps.md), "Bootstrapping") can be a
transcription: every rule here is a rule of the language implementation,
not of Rust.

Scope: [18](../../proposal/18-next-steps.md) Phases 0 and 2, the core,
with Phase 2's modules and packages (the module tree under `src/`,
`pub` and `pub(package)`, `sisu.toml` with path dependencies, `sisu.lock`,
workspaces and scripts);
Phase 3's ownership checker, dynamic ownership monitor, counterexample
mode and `sisu audit --ownership`, with its library types (`Box`, `Shared`, `Weak`, `Cell`, `Mutex`,
`Arena[T]` with generational `Key[T]`) and `own` closures; Phase 4's
traits and impls, user generics with `const` parameters, `Array[T, N]`,
`dyn Trait`, the standard library's
iterators, formatting, collection algorithms and text methods written in
Sisu (a text is its UTF-8 bytes), drop
glue as SIR functions, the comptime evaluator with derive written as
compile-time Sisu, the sized integer types from `u8` to `u128` and `i8`
to `i128`, `bytes` and `Slice`; and
Phase 6's tasks on a seeded cooperative scheduler (scopes, cancellation,
shields, deadlines, races, channels, `select`, generators), `Send` and
`Sync` checked at spawns, the `Shared` escape analysis, the capabilities
of `std.cap` with their fakes and interceptors, and `std.x.http`, a
minimal HTTP/1.1 client and server written in Sisu. From Phase 8:
contracts (`requires`, `ensures` with `result` and `old`, invariants,
refinement types) checked at run time at a chosen level and run as
properties by `sisu test`; property tests with shrinking, `exhaustive[T]`,
`@refines`, parametrised tests, suites and tags, snapshots, transcript
tests, benchmarks, fakes and interceptors of the program's own traits,
and tests in parallel; the capability log (`sisu run --record`, `sisu
replay`) and replay tokens; PCT and bounded exhaustive scheduling and the
`sim` profile; compile-time reflection in generic functions
(instantiated per type), `embed` and `target`, with `std.x.cli`'s parser
written over it, and conditional compilation (`@cfg`, decision 243); and
the tools of 10: the formatter (`sisu fmt`), hot replacement in epochs
(`sisu run --watch`), the REPL (`sisu repl`, with `:back` and `:why`),
worksheets (`sisu ws`) and the language server (`sisu lsp`). Not here:
from Phase 6, TLS; the checked string DSLs and foreign functions (Phase
8). Their syntax parses, and using them is a diagnostic (`E0900
stage0.unsupported`). Stage 2, a native debug backend (`sisu build
--native`) that compiles every program the conformance suite runs, is
described in its own section, "Stage 2".

## Crate map

One crate per pipeline stage (18, "Implementation language and
architecture"); data flows down the table.

| Crate | Lines | Role |
|---|---:|---|
| `sisu-diag` | 955 | Spans, the source map (1-based lines, columns in characters), the message catalog, diagnostics with labels, notes and fixes, the human and `--format=ids` renderers |
| `sisu-syntax` | 4112 | Token and node kinds, the lossless arena tree with stable node identities, and the AST built from the tree as a total function |
| `sisu-lexer` | 1258 | Scanning (every byte a token, interpolation split into tokens) and layout (Python's INDENT/DEDENT algorithm) |
| `sisu-parser` | 3557 | Error-tolerant recursive descent that always returns a tree |
| `sisu-fmt` | 1868 | The formatter: the lossless tree regrouped into logical lines, laid out by a Wadler-style group printer, with the comments and blank lines placed between them; the shape a formatted file must keep (`sisu parse --shape`) and the unused imports it removes |
| `sisu-resolve` | 12423 | Loads the Sisu-written std (`compiler/std/`), declares every item, resolves signatures to types, registers methods; traits, impls, coherence and the trait solver; derives (unfolding each trait's `comptime def derive`), delegation and the instances of templates; the generators and generated tests of property testing, the interceptors and fakes of the program's traits; the borrow rules about declarations |
| `sisu-typeck` | 11888 | The bidirectional checker, with method resolution through traits and `comptime` expressions; produces the typed tree (THIR) for every body, broken or not |
| `sisu-sir` | 3215 | SIR: the data model, the text writer and reader, the validator, the types of places, and the jump threading of the SIR that runs |
| `sisu-lower` | 5975 | THIR to SIR: control flow, scopes and their drops, unwind edges, closure conversion, trait calls and their static resolution, drop glue, `comptime` functions, the borrow facts of signatures |
| `sisu-borrowck` | 5686 | The ownership checker on SIR: initialisation and moves, drop elaboration, loans and the borrow rules, the task rules, escapes, must-consume, purity, `explain-borrow`, and the `Shared` escape analysis |
| `sisu-interp` | 16409 | The trampolined SIR interpreter, values, drops, dispatch of trait calls, unwinding, host intrinsics, the checked memory model of unsafe code, the task scheduler (random, PCT, bounded exhaustive) and the `sim` profile's faults, capabilities and the host's objects, the capability log, hot replacement in epochs, the parallel test runner, the dynamic ownership monitor, and the compile-time evaluator |
| `sisu-rt` | 13038 | Stage 2's runtime, linked into every native executable: the shared ABI (the value layout, the type table's encoding, the entry points), panics and stack traces, allocation, the structural operations on values, iterators, dicts and sets, the intrinsics, the scheduler of tasks on pooled stacks, the library types, capabilities, and the host's files, network and processes |
| `sisu-codegen` | 10142 | Stage 2's debug backend: SIR to Cranelift IR, one instance per function and list of type arguments, with trait calls resolved at compile time; the release profile's facts about a function's integers, which leave out the checks they prove; the object file and the link |
| `sisu-driver` | 14390 | The query layer and the `sisu` command line, whose front end is stage 1's unless the Rust one is asked for (`front.rs`); manifests (a TOML reader of its own), the package layout and the lockfile; the counterexample mode, `audit --ownership` and `--caps`, `sisu doc`, replay tokens, snapshot files and transcripts, the `sim` build check; `fmt`, the REPL and worksheets, `run --watch`'s rebuilds, and the language server with its JSON; `build --native` and `run --native`; the soundness tests (fault injection, mutation fuzzing), the language server's protocol tests and the native conformance test |
| `sisu-conformance` | 1830 | The language-agnostic conformance runner, and its native mode; either front end of the executable |

Line counts are of each crate's `src`, unit tests included. Besides the crates:
`catalog/messages.catalog` (every message); the standard library, in
Sisu: `std/prelude.sisu` (4150 lines: the builtin types, the core traits
and their derives, iterators, formatting, collections, the text methods,
`Shared` and the other library types, tasks and channels, time, the sized
integers and `bytes`, `Array`, the capabilities and their operations
traits, property testing), `std/cap.sisu` (961: std.cap's attenuations,
caretakers, fakes, interceptor and the `sim` profile's file system and
network), `std/collections.sisu` (441: std.collections), `std/num.sisu`
(865: std.num, with `BigInt` and `Decimal`), `std/regex.sisu` (738: a
Pike VM), `std/hash.sisu` (592: SipHash, FNV, CRC-32, SHA-2, BLAKE3),
`std/json.sisu` (447: `encode` and `decode` over reflection, `parse`),
`std/io.sisu` (319: readers and writers), `std/encoding.sisu` (306),
`std/fs.sisu` (254), `std/sort.sisu` (228), `std/rand.sisu` (225:
xoshiro256** and ChaCha20), `std/par.sisu` (209), `std/search.sisu`
(198), `std/math.sisu` (178), `std/process.sisu` (132), `std/env.sisu`
(125), `std/sim.sisu` (152: nodes, partitions, `choose`, `always`),
`std/cli.sisu` (175: std.x.cli over reflection), `std/http.sisu` (299:
std.x.http), `std/test.sisu` (snapshots, benchmarks), `std/live.sisu`
(hot replacement's boundaries), `std/task.sisu`, `std/log.sisu`,
`std/net.sisu` (`Url`), `std/time.sisu`, and `std/string.sisu`
(`Builder`) and `std/fmt.sisu`, which name the prelude's text and
formatting (decisions 245 and 252); and `conformance/` (the suite,
described in its own [README](../conformance/README.md)).

The workspace forbids `unsafe` and denies `unwrap`, `expect`, `panic`,
`todo`, `unimplemented` and `unreachable` outside tests (18, Phase 0,
"Panic policy"); an internal error becomes a catalog message, never a
host panic. `sisu-rt`, where native code meets Rust, is the one crate
allowed `unsafe` (decision 187).

## Pipeline

```text
source --lex--> tokens --parse--> lossless tree --ast::build--> AST
       --resolve--> Program (items, signatures, methods, impls, derived impls)
       --typeck--> THIR (typed bodies) + diagnostics
       --lower--> SIR (every drop unconditional, drop glue, comptime functions)
       --borrowck--> SIR with its drops placed + ownership findings
       --simplify--> SIR with its jumps threaded
       --comptime--> SIR with every `comptime` value built from literals
       --validate--> SIR --interp [--monitor]--> output, exit status
                         --codegen--> object file --cc, with sisu-rt--> executable
```

Every stage is total: it returns its output and its diagnostics for any
input, and a later stage runs on whatever an earlier one produced. Only
lowering requires a program without errors; the driver does not lower a
program that has any, except that the ownership rules about declarations
(`E04xx` from name resolution) do not stop lowering, so that the
counterexample mode can run such a program. The ownership checker's SIR
is always complete and valid, even when it reports errors. The compiler
runs the `comptime` expressions of a program the ownership checker
accepts (section "Compile-time evaluation"); one that it rejects runs
them at run time, in the counterexample mode, with the same result.

## Lexing

`sisu_lexer::lex` runs two passes, neither of which consults the parser.

**Scanning** turns the text into raw tokens covering every byte: trivia
(spaces, comments, line breaks), names, keywords, literals and
punctuation. A string with interpolation is a sequence of tokens:
`STRING_START`, `STRING_TEXT`, `INTERP_START`, the expression's tokens, an
optional `COLON FORMAT_SPEC` or `EQ` (for `{x=}`), `INTERP_END`, and
`STRING_END`, so interpolation is part of the grammar (PEP 701). After
`def`, a keyword is scanned as a name, so a method may be called `or`.

**Layout** is Python's algorithm. A stack of indentation widths starts at
0; at the start of each logical line outside brackets, a wider line pushes
its width and emits `INDENT`, a narrower one pops and emits one `DEDENT`
per popped width. Blank and comment-only lines are trivia. A line break is
a `NEWLINE` unless it is inside brackets, follows a trailing comma, or the
next non-blank line starts with a continuation operator (`.`, `?.`, `??`,
or a binary operator). Indentation is spaces only.

Recovery: a dedent to a width that matches no level is reported once
(`E0002`, naming the columns that would match) and pushed as a phantom
level that emits no tokens. A bracket still open when a line starts with a
declaration or statement keyword (`def`, `let`, `return`, `case`, ...) at
or left of the bracket line's indentation is reported (`E0009`) and closed
there, so one missing `)` does not swallow the file; so is a bracket left
open by a block header (a line that starts with `def`, `if`, `for`,
`while`, `match`, `case`, `struct` and the like and ends in `:`) when the
next line is indented further. An unterminated string ends at the line
break and is reported from its opening quote. A line break inside the
interpolation of a one-line string is reported (`E0018`), closes the
interpolation and the string, and is then scanned as code, so it ends
the line as any other does. `\u{...}` names a character by one to six
ASCII hex digits (no sign, no space) of a Unicode scalar value; anything
else after `\u` is an invalid escape (`E0007`, reported at the
backslash), whose braces stay in the text, and the decoded value of the
literal keeps it as written.

`sisu tokens FILE` prints the significant tokens, one per line, as
`KIND start..end "text"`; `--trivia` adds the trivia.

## Parsing and the syntax tree

The parser is hand-written recursive descent, one function per rule.
Every production can produce an `ERROR` node; recovery synchronises at
`NEWLINE` and `DEDENT`. The grammar:

```text
file      := item* EOF
item      := attribute* visibility? (fn_def | struct_def | enum_def | error_def
             | impl_def | trait_def | import | from_import | test_def | suite_def
             | bench_def | const_def | type_alias | extern_block)
attribute := '@' name ('(' args ')')? NEWLINE
fn_def    := ('pure' | 'comptime')? 'def' name generic_params? params ('->' type)?
             where_clause? (':' block | NEWLINE)
struct_def:= 'view'? 'struct' name generic_params? ':' INDENT (field | fn_def | 'pass')+ DEDENT
field     := visibility? name ':' type ('=' expr)? NEWLINE
enum_def  := 'enum' name generic_params? ':' INDENT (variant | fn_def)+ DEDENT
error_def := 'error' name ':' INDENT (variant | '..' NEWLINE)+ DEDENT
variant   := name ('(' (name ':')? type (',' (name ':')? type)* ')')? NEWLINE
impl_def  := 'impl' generic_params? type ('for' type)? ':' INDENT fn_def+ DEDENT
import    := 'import' path ('as' name)? NEWLINE
from_import := 'from' path 'import' name ('as' name)? (',' name ('as' name)?)* NEWLINE
test_def  := 'test' string ('for' patterns 'in' expr)? params? ':' block
const_def := 'const' name ':' type '=' expr NEWLINE
type_alias:= 'type' name generic_params? ('=' type ('where' expr)?)? NEWLINE

block     := NEWLINE INDENT stmt+ DEDENT
stmt      := let_stmt | return_stmt | 'break' | 'continue' | 'pass' | if_stmt
             | while_stmt | for_stmt | assert_stmt | defer_stmt | with_stmt
             | item | expr_stmt | assign_stmt
let_stmt  := ('let' | 'var') patterns (':' type)? ('=' exprs let_else?)? NEWLINE
let_else  := 'else' ('as' name)? ':' (block | expr | return_stmt | 'break' | 'continue')
if_stmt   := 'if' cond ':' block ('elif' cond ':' block)* ('else' ':' block)?
cond      := cond_part (',' cond_part)*
cond_part := 'let' pattern '=' expr | expr
while_stmt:= 'while' cond ':' block
for_stmt  := 'for' patterns 'in' ('mut' | 'own')? expr ':' block
assert_stmt := 'assert' expr (',' expr)? NEWLINE
defer_stmt:= 'defer' ':' block
with_stmt := 'with' expr ('as' name)? ':' block
assign_stmt := exprs ('=' | '+=' | '-=' | ... ) exprs NEWLINE

expr      := closure | coalesce
closure   := 'own'? (name | '(' params ')') '->' expr
coalesce  := or ('??' coalesce)?
or        := and ('or' and)*           -- mixing `and` and `or` needs parentheses
and       := not ('and' not)*
not       := 'not' not | compare
compare   := bitwise (cmp_op bitwise)* -- a chain points one way: a < b <= c
bitwise   := shift (('&' | '^' | '|') shift)*     -- one operator, no arithmetic operands
shift     := additive (('<<' | '>>') additive)*
additive  := mult (('+' | '-' | '+%' | '-%' | '+|' | '-|') mult)*
mult      := unary (('*' | '/' | '//' | '%' | '*%' | '*|') unary)*
unary     := ('-' | '~' | 'ref' 'mut'? | '^') unary | power
power     := postfix ('**' unary)?     -- `-x ** y` needs parentheses
postfix   := primary (call | index | '.' name | '?.' name | '?' | do_block)*
do_block  := 'do' ('own'? params)? ':' block      -- only at the end of a logical line
primary   := literal | string | name | '(' ... ')' | '[' ... ']' | '{' ... '}'
             | if_expr | match_expr
if_expr   := 'if' expr ':' expr ('elif' expr ':' expr)* 'else' ':' expr
match_expr:= 'match' expr ':' INDENT ('case' pattern ('if' expr)? ':' (expr NEWLINE | block))+ DEDENT

type      := '!' error_set? | option ('!' error_set?)?
error_set := '{' option (',' option)* '}' | option
option    := ref_type '?'*
ref_type  := 'ref' 'mut'? primary_type | primary_type
primary_type := path generic_args? | 'Self' | 'pure'? '(' types? ')' ('->' type)? | 'dyn' path
generic_args := '[' generic_arg (',' generic_arg)* ']'
generic_arg := const_expr | integer | type  -- an expression where one starts and no type does,
                                            -- or a path, integer or (...) followed by an operator or `(`

patterns  := pattern (',' pattern)*    -- a bare tuple in `let` and `for`
pattern   := primary_pattern ('|' primary_pattern)*
primary_pattern := '_' | lower_name | literal | '-' number
             | Upper_path ('(' pattern_arg (',' pattern_arg)* ')')?
             | '(' patterns ')' | '[' patterns? ']' | '*' name | '..'
pattern_arg := name '=' pattern | pattern | '..'
```

A lowercase name in a pattern binds; a capitalised one names a variant or
a constant, so the parser decides by spelling. The forms of later phases
(`select`, `yield`, contracts, `comptime`, `extern`, `x.[f]`) parse so
that every example in the proposal produces a tree without syntax errors.

**The tree** is lossless: every byte belongs to exactly one token and
every token to exactly one node, so `sisu parse --print` reproduces the
input byte for byte, and the conformance runner checks it for every case.
The tree is an arena of nodes and tokens addressed by index, built by a
builder with checkpoints (left-recursive expressions wrap already-built
children without backtracking). `sisu parse --tree` prints it, one node or
token per line, indented, with byte ranges.

**Stable identities.** `Tree::stable_id(node)` names a node so that the
name survives edits elsewhere in the file: the path of enclosing items as
`KIND:name#ordinal` (the ordinal counts earlier siblings with the same
kind and name), then the node's pre-order index inside the innermost item.
The language server and the query layer key on these.

**The AST** (`sisu_syntax::ast`) is an owned view built from the tree by
`ast::build::module`, a total function: an `ERROR` node becomes an `Error`
expression, pattern, type or statement. `sisu parse --ast` prints it as
Rust's `{:#?}` of it, one field per line, indented: byte ranges and node
indices, no addresses, and every list in source order, so that the text
is the same on every run and stage 1 can compare its AST with it.

### Error tolerance

The rules that keep one mistake from producing many messages:

- A missing `:` before an indented block is reported (`E0100`) and the
  block is parsed as if the colon were there.
- An unexpected indented block is reported once (`E0125`) and its lines
  are parsed as a nested block.
- At most one syntax diagnostic is kept per line: the most likely cause,
  ranked lexical errors about the characters themselves first (an
  unterminated string, a tab), then bracket errors, then grammar errors;
  the first of equals. Diagnostics on different lines are all kept, which
  is what makes "three mistakes, three reports" hold.
- When the lexer closes a bracket at a line start (see Lexing), the
  parser's "expected `)`" at that same token is dropped; a closer missing
  in the middle of a later line is a separate mistake and is kept.
- A statement at the top level (`E0107`) or an unexpected indented block
  (`E0125`) after an earlier syntax error is not reported, and nothing
  inside such a block is: the block structure has already diverged from
  the intended one. A struct body line that is neither a field, a method
  nor `pass` is reported once per body (`E0126`); a struct declared
  inside another's body has a body of its own, and the outer body's
  report stands after it.
- A function whose header has a syntax error (including a missing `:`;
  the parser marks the header with an empty `ERROR` node) has an unknown
  signature: its result type is `Error`, calls to it are checked only for
  their own arguments, and its body does not report names it cannot find
  (most likely parameters lost in the error). A body with a syntax error
  does not report unknown names or a missing `return` either.
- Names written in top-level code that did not parse as a declaration
  (`stru ct Log:`) are not reported as undefined where they are used.
- A pattern naming an unknown variant still declares the names it binds,
  so the arm's body reports nothing about them.
- Name and type diagnostics on a line that has a syntax error are dropped:
  the line has no reliable meaning.
- A declaration whose header did not parse at all (no name) produces no
  name or body diagnostics.
- Names and types are poisoned: an expression that failed to check has
  type `Error`, which unifies with everything and suppresses every
  diagnostic that would mention it, so only the first error of a chain is
  reported.

A test in the driver mutates programs (deleted, inserted and swapped
characters, with a fixed seed) and checks that every stage returns, that
the tree reprints its input, and that a mutated program without errors
lowers to valid SIR.

## Diagnostics and the message catalog

Every message the implementation prints is an entry in
`catalog/messages.catalog` (D-065): compile-time diagnostics, labels,
notes and fixes, run-time panic messages and the driver's own messages.
The format is line based so any implementation can read it:

```text
# a comment
CODE  KIND  IDENTIFIER  TEMPLATE...
```

`CODE` is stable and never reused (`E0101`), or `-` for a message that is
part of another (a label, a note, a fix). `KIND` is one of `error`,
`warning`, `note`, `help`, `label`, `fix`, `runtime`, `cli`. The template
is the rest of the line: `{name}` is a parameter, `{{` and `}}` are
literal braces, `\n` is a line break. Code ranges: `E00xx` lexer, `E01xx`
parser, `E02xx` names, `E03xx` types, `E04xx` ownership, `E05xx` SIR,
`E06xx` compile-time evaluation, `E09xx` stage-0 limits and internal
errors, `W0xxx` warnings, `R0xxx` run
time (the monitor and the counterexample mode from `R0042`), `D0xxx`
driver. Two ownership findings keep the type checker's codes they had
before the checks moved to SIR: `E0366` (use before assignment) and
`E0367` (a second assignment to a `let`); purity on SIR reports `E0333`,
`E0392` and `E0393`.

Code never contains English: `sisu-diag/build.rs` generates one `MsgId`
constant per entry (`lex.tab-in-indentation` becomes
`msg::LEX_TAB_IN_INDENTATION`), and a `MsgId` cannot be built from a
string outside that crate, so a message that is not in the catalog does
not compile. Severity comes from the catalog. Templates avoid number
agreement ("wrong number of arguments to `f`: expected 2, found 1") so
the catalog needs no plural rules yet.

A diagnostic is an identifier, named parameters, a primary span, labelled
secondary spans, notes and fixes (each fix a message and text edits). The
driver sorts a program's diagnostics by position and code and reports a
diagnostic identical to an earlier one (identifier, parameters, spans,
notes and fixes) once (`sisu_diag::diagnostic::sort_unique`): where the
compiler checks one piece of source more than once, as it does the body
the cases of a parametrised test share, one mistake is still one
message. The human renderer prints rustc-style excerpts; `--format=ids` prints the
conformance form, described in the suite's
[README](../conformance/README.md).

## Names and the standard library

The prelude and the std modules are Sisu source (`compiler/std/*.sisu`)
compiled by the same front end as user code. A function without a body
names a host intrinsic with `@intrinsic("name")`, and the `##` comment
above it is the intrinsic's contract, stated in Sisu terms; `@builtin`
marks a type whose values the host represents. Five more attributes are
std's: `@structural("name")` on a function with a body says that the
intrinsic `name` computes the same result whenever the call's type
arguments meet every bound with a structural impl (lowering then calls the
intrinsic: `list.sort` and `join` are written in Sisu and run on the host
for builtin elements); `@keeps(self, x)` on an intrinsic names the `own`
and `mut` arguments whose every value it keeps or returns (the only ones
that may take a must-consume value); `@sealed` on a trait means only std
implements it (`Num`); `@marker` on a trait means it has no methods and is
implemented by the language's rules (`Copy`); `@internal` on a function
means only std may call it (`E0701`). All seven are reserved for std
(`E0225`). A std module may name prelude items too
(`std_sources::reexports`), so `from std.time import DateTime` finds the
prelude's. The prelude declares the core traits (`Display`, `Debug`, `Eq`,
`PartialOrd`, `Ord`, `Hash`, `Default`, `Len`, `Contains`, `Copy`, `Num`,
`Drop`, `Iterator`, the operator traits, `FromLiteral`), their impls for
the builtin types, and the generic code over them; the impls of the leaf
types and of tuples are generated (`<std>/builtin_impls.sisu`, from
`sisu-resolve/src/std_sources.rs`).

`Iterable[T]`, a parameter type only, accepts a `list`, `set`, `dict` or
`Iter` whose items are `T`; a borrowed collection's items are references,
and an item `ref T` with a `Copy` type `T` decays to `T`. A value of any
other type that implements `Iterator` (`xs.iter().map(f)`, a program's
own iterator) is passed as `iter_from(x)`, a prelude generator that takes
its items from it as they are asked for (decision 247); a reference to
one is a mismatch (`E0300`), since nothing could own what it is moved
into.

Resolution (`sisu-resolve`) declares every item of every module of the
program (next section) and of each std module they import (`import cli`,
`import std.math`), resolves every signature to a `Ty`, registers methods
by receiver type and name, and records every variant by its unqualified
name. A name in an
expression is a variant of the expected type first, then a declaration of
that name, then a variant declared in the current module, then any
visible variant, where two candidates are ambiguous (`E0211`); a name in a
pattern is a variant of the scrutinee's type first. Traits, impls and
generics are the next section's.

Resolution also checks the ownership rules that are about declarations
(04, "Views" and "Borrowing rules in detail"), after every signature is
known:

- A type is a *view* by its spelling: a reference, `str`, a `view
  struct`, or a builtin collection, option or tuple with a view argument.
  Only a `view struct` may have a field of view type (`E0411`, with the fix
  of writing `view ` before `struct`); an enum has no view form, so a
  variant may not hold one (`E0412`). `ref str` is `str`: a shared borrow
  of a view is the view, valid as long as the text it views.
- A parameter is *borrowed* when it is `mut`, or passed without `own` and
  either not `Copy` or able to hold a loan (a `str` is `Copy` and can). A
  result that can hold a loan comes from the parameters of its `from`
  clause; without one, from the only borrowed parameter, or from `self`
  when it is borrowed; otherwise the clause is required (`E0413`, with a
  fix naming every borrowed parameter). A `from` name must be a borrowed
  parameter (`E0414`); a result that cannot hold a loan takes no clause
  (`E0415`, with a fix removing it).
- `@must_consume` on a struct or enum is recorded for the ownership
  checker (D-054).

## Modules and packages

[09](../../proposal/09-modules-packages-and-build.md). What a program is
made of is found before anything is parsed (`sisu-driver/src/package.rs`):
a single file; a script (a file starting `#!`, whose top-level statements
are its `main`, with optional inline metadata between `# /// script` and
`# ///` giving its dependencies, `E1014`); a package, the directory of a
`sisu.toml` and its `src/` tree; or a workspace's member. Without a file,
`sisu check`, `run`, `test` and `build` work on the package of the
current directory or the nearest one above it (`D0064`).

**The module tree mirrors `src/`.** The root module is `src/main.sisu`
(an application, which `sisu run` runs) or `src/lib.sisu` (a library), not
both (`E1008`, `E1009`), and is named by the package's name;
`src/parser.sisu` is `PACKAGE.parser`, a directory `src/net/` the module
`PACKAGE.net` whose children are its files and directories, and a file
`src/net.sisu` beside that directory gives the module items of its own.
A module is named in lowercase letters, digits and `_` (`E1012`). A
module's file names it in diagnostics relative to the root package's
directory.

**Imports** name a module by its path: the importing package's own
modules by the package's name, a dependency's by the dependency's name,
the standard library's by `std.NAME` (and stage 0's short names `cli`,
`http`, `json`, ...). `import a.b` binds `b`; `from a.b import x, y as z`
binds items; an import that names nothing is `E0220` (`E0203` past
its first word, `E0223` for an item that is not a module), and one of a
package of the program that this one does not depend on `E0260`.
Items are private to their module unless `pub`; `pub(package)` reaches
the modules of the same package (`E0255`, `E0256`; fields `E0258` and
methods `E0259` likewise). A name an import bound is visible through the
module only when the import is `pub`, a re-export (`pub from a import
x`; `E0257` otherwise). A module is always reachable by its path. std's
items are all public in stage 0 (its `@internal` functions are checked
apart).

**The manifest** (`manifest.rs`, over a TOML reader of its own,
`toml.rs`, since stage 0 takes nothing from a registry) is data (D-063):
`[package]` with `name`, `version` (`MAJOR.MINOR.PATCH`), `edition`,
`description` and `license` (and keys recorded for later stages);
`[dependencies]` and `[dev-dependencies]`, each `NAME = { path = "..." }`
with an optional `when = { os = ..., arch = ..., runtime = ... }` over the
build target's facts (`E1011`); `[trust] unsafe = [...]`, the dependencies
that may contain `unsafe` and `extern` (`E1015`); `[workspace] members`;
`[features]`, with a dependency's `features` and `default-features`,
which `@cfg(feature="NAME")` reads (decision 243);
`[profile.release] contracts`. A mistake is a diagnostic at its place in
the file (`E1000`-`E1017`), an unknown key a warning (`W0005`). A
dependency is a path (`E1010`: there is no registry), named by its own
`[package] name` (`E1006`); the graph has no cycle (`E1007`), and a
package may not take a std module's name (`E1017`).

**The lockfile** `sisu.lock`, beside the root manifest or its
workspace's, records every package the build resolved, sorted: name,
version, `source = "path+DIR"` relative to the lockfile, and every
dependency, active on this target or not, so it is the same on every
target. `sisu check` and the other commands rewrite it when the
manifests imply another; `--locked` refuses to and fails (`D0065`). A
workspace has one lockfile for all its members, each of which must be a
package (`E1013`).

**Resolution of a program of many modules.** The driver gives the
resolver every module of every package with its path and package
(`sisu_resolve::SourceModule`, `SourcePackage`); the query layer's key is
the program (its files and manifests), and diagnostics are filed against
each module's own file. SIR names an item of a module other than the
root by the module's path (`atlas.geo.point.origin`), and the root
module's without one (`main`). `sisu build` checks the package and lowers
it, printing what it built (`D0066`, `D0070`); `sisu new NAME` writes a
manifest and a root module (`D0071`-`D0073`). A dependency the root
package does not trust may not contain `unsafe` or `extern`
(`E0703`).

## Traits and generics

Phase 4 of 18, after [03](../../proposal/03-types-and-generics.md).

**Traits** declare required and provided methods and associated types
(`type Item`, with an optional default), and may name supertraits
(`trait Ord(Eq, PartialOrd)`); in a trait's signatures `Self` is the
first type parameter and the trait's own parameters follow. **Impls**
(`impl[T: Eq] Eq for list[T]:`) define every required method with the
trait's signature (`E0232`, `E0233`, `E0234`) and bind every associated
type without a default (`E0237`). A method of an impl inherits `pure`
from the trait's declaration of it. An impl for a type implements the
trait's supertraits only if the type implements them too (`E0242`).

**Coherence.** A type implements a trait at most once: two impl headers
of one trait overlap when their patterns (the self type and the trait's
arguments) unify, which is an error at the second (`E0235`); impl
selection then matches headers only, and `where` clauses never choose
between impls. The orphan rule: an impl lives in the package of the
trait or of the type, except in the root package, which nothing depends
on (D-013, Q-11; `E0236`). std is one package and may implement
anything; a program's root package may implement std's traits for std's
types, and a dependency only what involves its own trait or type
(`packages/orphan`). A `@sealed` trait has only std's impls (`E0240`).

**Generics.** Functions, structs, enums, traits and impls take type
parameters with bounds (`[T: Display + Hash]`, `where` clauses,
`I.Item: Display`); a type's and a trait's parameters may have defaults
(`A: Allocator = Global`), where a parameter without a default cannot
follow one with a default (`E0245`); an impl's and a function's may not
have defaults (`E0246`). A generic body is checked once, at its definition, against
its bounds: calling a method on a `T` needs a bound that has it, and a
violation is reported in the body, never at a use (18, Phase 4 exit
criterion). A call infers its type arguments by unification; a
definition-site bound of a type (`struct Labelled[T: Display]`) is
required where the type is written.

**Const generics** (03, "Const generics"). A parameter `const N: int`
of a struct, enum, impl or function takes a constant: an integer literal,
another `const` parameter in scope, a constant of the program, or an
expression of them (`Grid[int, 3]`, `Array[T, W * H]`; a type there is
`E0262`, a constant where a type goes `E0263`). An expression argument
is computed where the type is written (`sisu-resolve/src/consts.rs`):
integers, `const` parameters, the program's constants whose values are
written that way, and the operators `+ - * // % ** << >> & | ^` and
unary `-` and `~`, with the run time's rules (`//` floors, `%` takes the
divisor's sign); a call, a local or a `comptime` expression is `E0264`,
a value of another type `E0265`, and an operation without a value
(overflow, a zero divisor) the `E0602` its run would panic with. An
operation on a `const` parameter stays an operation, `Ty::ConstOp`
(`sir::Type::ConstOp`, `mul($1, $2)` in SIR), until a use gives the
parameter a constant: substituting the arguments of `Grid[int, 3, 2]`
folds its field's `Array[int, W * H]` to `Array[int, 6]`, in the checker
and at run time alike. Two operations are equal when they are the same
operation on equal arguments (`W * H` is not `H * W`, `E0300`). The
constant is part of the type (`Ty::Const`, `sir::Type::Const`), so `Array[int, 3]` and
`Array[int, 4]` differ (`E0300`), and unification infers it like a type
argument (`Array.filled(0)` takes its `N` from the type expected). In the
bodies it scopes, `N` is an `int`: lowering reads it from the frame's
type arguments (`generic.value[$i]`), since type arguments are values at
run time (below). A `comptime assert` over `const` parameters in a
generic body (08) cannot be decided at the definition, so it is checked
at each call that gives them constants, once the program is typed
(`sisu-typeck/src/generic_asserts.rs`): its condition and message are
folded with the call's arguments (integer and boolean arithmetic,
comparisons, interpolation), and a false condition is the call's
`E0602`. **`Array[T, N]`** is a builtin type of the prelude: `N` values,
read and written by index with the bounds check of a list (`a[^1]`
counts from the end), iterated, compared, hashed and printed as a list
is, made by `Array.filled(value)` (`N - 1` copies and the value), with
`len` returning `N` and `to_list` the elements; nothing adds or removes
one. At run time it is a list of exactly `N` values.

**Type arguments are values at run time** (18, Phase 4: "polymorphic
interpretation ... no monomorphisation"). Every call of a generic SIR
function passes its type arguments (`call fn total[Sq](...)`), a frame
holds them, and a type in the body (`$0`, `<$0 as Iterator>.Item`) is
made concrete by substituting them and replacing associated types by
what the matching impl binds. Intrinsics receive type arguments only
where lowering's rule says they need them (section "Lowering").

**Methods.** `x.name(...)` is a method of `x`'s type, then a method of a
trait the type implements (or, for a generic parameter, a trait it is
bounded by), looking through references and `Box`es; a name that more
than one such trait has is ambiguous (`E0394`), and `Trait.method(x)`
names one. A call whose `Self` is known when lowering is resolved there
to the impl's method or the trait's provided body; one whose `Self` is a
type parameter, an associated type of one, a reference or a `dyn` is a
`call trait` that the interpreter dispatches: it matches the impls'
patterns against the concrete type arguments (a reference dispatches on
what it points to, a `dyn` on the type it carries), and with no impl
uses the language's own structural one for the core traits. A borrowed
parameter whose declared type is a type parameter is passed by
reference even when the argument is `Copy`, unless the parameter is
bounded by `Copy`; dispatch to an impl that takes a `Copy` type by value
reads the value through the reference.

**`dyn Trait`** is a value of some type that implements the trait,
carrying its type: behind `ref` (a `ref T` coerces to `ref dyn Trait`) or
in a `Box` (`E0395` for a bare one). A trait is object safe when every
method takes `self` or `mut self`, has no type parameters of its own and
mentions `Self` nowhere but its receiver (`E0241`); `dyn Trait`
implements its supertraits. A `dyn` owns its value, so a view cannot
become one (`E0396`).

**The core traits are std's.** `Display`, `Debug`, `Eq`, `PartialOrd`,
`Ord`, `Hash` and `Len` are prelude traits with impls for the builtin
types, collections, options, results and tuples; `Copy` is a marker
computed from fields (`@derive(Copy)` opts a type in when every field is
`Copy`); `Num` (`int`, `f64`) is sealed; every type has `Debug`, its own
or the derived one (11). The operators `+ - * / %` and unary `-` on
non-numbers call the operator traits `Add`, `Sub`, `Mul`, `Div`, `Rem`
and `Neg`; on numbers they are the language's own operations, with their
traps. `FromLiteral[L]` makes a literal of type `L` a value of a type
that implements it where that type is expected (a string literal is a
`String` or a `Path`). `Iterator` has `type Item` and `next`, and its
adapters and consumers are provided methods written in Sisu.

**Structural impls.** The impls std writes for the builtin types, and
every derived impl, compute what the interpreter's structural operations
compute (compare field by field, hash the fields, print the way values
are written). When every bound of a call's type arguments is met by
such an impl, lowering calls the host's operation instead (an
interpolation of a struct, `sorted(xs)`, the `@structural` functions);
otherwise the Sisu code runs. A `dict` key or `set` element is hashed and
compared structurally by the host in stage 0, so a key type with its own
`Eq` or `Hash` impl is rejected there (`E0398`) rather than silently
hashed the other way.

**Derive and delegation.** `@derive(T)` runs `T`'s `comptime def derive`
at compile time (section "Compile-time evaluation"); `Debug`, `Eq`,
`PartialOrd`, `Ord`, `Hash` and `Default` are written that way in the
prelude, and any trait may declare one. The derived impl of a generic
type bounds each parameter that a field uses by the trait (`impl[T: Eq]
Eq for P[T]`), and the fields of a type deriving one of the language's
traits must implement it (`E0244`), for `Default` those without a
default of their own (decision 238). `@delegate(Trait, ..., to="field")` writes impls
that forward to a field (03, "cheap newtypes and delegation"): the
operator traits (when the field is the type's only one, since they
return `Self`), `Eq`, the orders, `Hash`, `Display`, `Debug` and `Len`
(`E0250`-`E0253`).

**`Drop`** is for a struct or enum of the same module, with exactly
`def drop(mut self)` (`E0213`, `E0216`), and, for a generic type, for all
of it: `impl[T] Drop for W[T]` with the type's parameters in order and
no bound the type does not declare (`E0254`), since the drop glue runs it
for every value.

## Compile-time evaluation

[08](../../proposal/08-metaprogramming.md) and 18, Phase 4: "the comptime
evaluator as the same interpreter with fuel, a memory limit and no
`System`; used first for the built-in `@derive`".

**`comptime e`** asks the compiler to compute `e`. It is checked as the
body of a closure of no parameters, which must capture nothing (a local
or a parameter is a run-time value, `E0600`) and whose value cannot
depend on the function's type parameters (`E0600`, checked once types
are known). As an operator, `comptime` takes the operand after it
(`comptime f() + 1` adds at run time); before a condition it makes all
of it compile-time (`if comptime a or b:`), and both branches are
checked. `comptime assert c, "m"` and `comptime e` as statements run at
compile time and leave nothing. A `comptime def` may be called only at
compile time, inside `comptime` or another `comptime def` (`E0605`, also
for taking it as a function value); one a trait declares besides its
`derive` is called `Trait.name(...)` (decision 251). `const X: T =
comptime f()` is a constant computed by the compiler; a constant is
computed even when nothing uses it.

**Evaluation.** Lowering makes each `comptime` expression a SIR function
of no arguments, `OWNER#comptimeN`, of kind `comptime STEPS`, and the use
a call of it. Once the ownership checker accepts the program, the driver
runs each such function in the interpreter, the ones whose values it
uses first (so each spends its own budget), with a budget of SIR steps
and a host through which nothing reaches the world, and replaces its
body by statements that build the value from literals, which makes it a
`value` function: the program then does at run time what it would do for
a literal. The semantics are the run-time semantics: the same
interpreter, the same overflow traps, `Drop` order and iteration order.
The budget is 1 000 000 steps, or `@comptime_budget(steps=N)` on the
function or constant that holds the expression (`E0606` for a malformed
one); the budget belongs to the item that spends it.

Errors, each at the `comptime` expression: a panic (`E0602`, with its
message; a failed `comptime assert` is one), a spent budget (`E0603`,
naming the item and the budget; 08's deny-by-default diagnostic, an
error, since stage 0 has no lint levels), a use of the world (`E0601`:
standard output or error, arguments, the environment, the clock, the
file system), and a value with no literal form (`E0604`: a function, a
reference, a `Shared` or another handle; numbers, text, booleans,
characters and the lists, sets, dicts, tuples, options, results, structs
and enums of them have one). An expression that uses a failed one is not
evaluated: its error is that one's. Purity is not required: a function
that does no I/O can run at compile time whether or not it is declared
`pure`, and one that tries is stopped where it does.

**Derive at compile time.** A trait's derive is `comptime def derive(T:
type) -> impl Trait for T` whose body is `return impl Trait for T:` with
the impl's methods (`E0607` otherwise). `@derive(Trait)` on a type runs it
for that type: the code that runs at compile time is evaluated and the
rest becomes the impl's source, with the compile-time values in it as
literals, declared beside the type and checked like any impl. Compile-time
code in a template is `comptime for x in e:` (unrolled), `comptime if c:`
and `if comptime c:` (decided), the same two among a `match`'s arms
(arms made per item), and any expression made only of compile-time
values: the type `T`; reflection, the compile-time module `reflect`, with
`name_of(T)`, `is_struct(T)`, `is_enum(T)`, `fields_of(T)` and
`variants_of(T)`; a field's `name` (its position for a positional variant
field), `index`, `named` and `default` (whether a struct's field has
one); a variant's `name`, `index` and `fields`;
integers, text and booleans with `enumerate`, `len`, arithmetic,
comparisons, `and`, `or`, `not`, and interpolation into text. `x.[f]` is
the field `f` of `x`; a variant's field is reached inside an arm `case
V(..)` that `comptime for V in reflect.variants_of(T)` made for a `match
x`, where it is the arm's binding (`__x_0`, ...). A template that cannot
unfold for a type is reported at the `@derive`, with a label where it
stopped (`E0608`).

### Templates and instances

A generic free function whose body reflects on its type parameters (a
`reflect` query, `x.[f]`, or a `comptime` expression naming one) is a
**template**: it is not checked as written but made once for each list
of type arguments it is called with, an **instance**, by the same
unfolding (`instances.rs`, `template.rs`). Which instances a program
needs is known only once its calls are typed, so the driver resolves and
checks the program, collects the checker's requests (`Request::Instance`
for a call of a template with concrete type arguments) and resolves it
again with them, until no new one comes: a fixpoint of at most 32
rounds, each resolving the whole program (`E0610` past it, or past 256
instances in one round, for a template whose instances call it with ever
larger types). A generic function that calls a template with its own type
parameters cannot have that call's instance until it is itself called,
so it becomes a template too (`Request::Promote`): monomorphisation by
need, of the part of the call graph that reflects.

An instance is declared in a module of its own, `<NAME[ARGS]>/MODULE.sisu`
(under `<std>/` for a std template, so traces leave it out), whose scope
is the template's module; each type it may name (an argument, a field's
type, an element type) is bound to an alias there (`__T0`, `__T0_f1`,
`__T0_e`), so the unfolded code names it whatever module it comes from.
Its SIR name is the template's with the arguments (`to_json#Point`), and
lowering redirects every call of the template to its instance. An
instance that cannot be made is reported at the call that first needed it
in the program's own code (an instance needed by another inherits its
site), with a label where a program's template stopped (`E0609`); a
template used as a value is `E0611`, since only a call makes an instance
of it (a generic function that is not a template is a value, decision
225).

Reflection in templates adds to a derive's: `is_int`, `is_float`,
`is_bool`, `is_char`, `is_str` (`str` or `String`), `is_list`,
`is_option`, `is_tuple`, `elem_of(T)` (a list's or option's element
type), `attributes_of(T)` (each attribute on the declaration, with `name`
and `args`, the text of each argument), `implements(T, "Trait")`; a
field's `ty` and `default` (whether it has one); `is_trait(T)` and
`methods_of(T)`, a trait's methods or a type's own (decision 251), each
with `name`, `index`, `receiver`, `params` (each with `name`, `index`,
`mode` and `ty`), `ret` and `provided`; `comptime panic(msg)`
(a compile error for the types that reach it). Two queries make code
rather than values: `reflect.default_of(f)` is the field's default,
evaluated where it is written (a call `X.__default_f()`, which the
checker types as the field's default, `ExprKind::FieldDefault`), and
`reflect.construct(T, {name: value, ...})` is the struct built from a
dict whose keys are known at compile time, every field without a default
given (so the struct is built once and its invariant checked on what was
given). A comprehension over compile-time lists is unrolled into the
list, set or dict it makes (`[f.name for f in reflect.fields_of(T)]`), an
`if comptime c: a else: b` expression picks its branch, and compile-time
values may appear inside a `match` expression and a closure.
`std.x.cli`'s `parse[T]` and `help_text[T]` are templates written this
way (`std/cli.sisu`), as are `json.encode[T]` (`std/json.sisu`) and
`std.test`'s `assert_snapshot`.

**`embed("path")`** is the text of a file beside the program, read by the
driver when it checks the program (`Db::checked`): a path inside the
program's directory, as a string literal (`E0613`); one that cannot be
read is `E0612`, with a reason that is the same on every machine.
**`target`** is an ambient module of constants (`os`, `arch` and
`runtime`, which is `interpreter`), so `if comptime target.os ==
"windows":` is decided when the program is compiled and both branches are
checked.

**`@cfg(...)`** leaves out the items whose condition does not hold before
names are resolved (`cfg.rs` in the driver, over each module's syntax
tree; decision 243): `test`, `feature="NAME"`, `not` and a condition, and
`all(...)` and `any(...)` of conditions. `sisu run`, `sisu build` and
`sisu sir` build the program without its tests (`Db::no_tests`), so
there `test` does not hold and the tests, suites, benchmarks and doc
examples are left out as well.

A function or a method whose body has such code is a template too
(decision 244): one without type parameters (a function or a method that
reflects on the program's own types, `main` included) is made once,
before anything calls it, and its instance takes its name, so a call, a
trait's call, `dyn` and a function value all run it; a method's instance
is a method of an inherent impl of its type, whose compile-time code
names the type `Self`; and a method of a generic type is made for each
type argument it is called with. So are the places decision 251 adds:
a test (each case of a parametrised one too), made once; a `comptime
def` that reflects or takes a type (`T: type`, whose argument a call
writes among its arguments), made for each type it is given; a trait's
provided method, made in each impl that does not write it, as that
impl's method; and a method of a trait impl, made once whatever its type
parameters, which stay parameters (known by name only, so asking about
one stops the instance), in an impl with the impl's parameters, so that
a trait's call reaches it through a bound or a `dyn`. A constant's or a
default's value that reflects is unfolded once everything it names is
declared, and compile-time code may be in a closure's block. A
refinement's `where` clause and an `invariant` have no instances, so
`reflect`, `x.[f]` and `comptime for` there are `E0900`, naming the
place, as is `return impl` outside a derive.

### The interceptors and fakes of a program's traits

For a trait the program declares whose methods can all be forwarded (a
`self` or `mut self` receiver, no type parameters of their own, no
borrow or `Self` in a signature; the trait without type parameters,
associated types or supertraits), the resolver writes, as source in a
module whose scope is the trait's (`shims.rs`, `<shims>/main.sisu`):
`impl[__T: Trait, __I: cap.Interceptor] Trait for cap.Wrap[__T, __I]`,
whose methods show each call to the interceptor (arguments and outcome
rendered with `Debug`) and forward it, so `cap.Wrap(inner=x, hook=h)`
implements the trait (D-075); and a fake, `struct __Fake_Trait`, which
`cap.Fake[Trait]` names (in a type or as `cap.Fake[Trait].new()`;
`E0614` for anything else): each method logs its call (`calls()` gives
the log, each call written `method(arguments)`) and returns what
`on_METHOD(result)` queued for it, the last result again once the others
are used, or panics with nothing queued; copies share what it recorded.
A module gets them only when its text names `Wrap` or `Fake`. In Sisu
both are compile-time code over the trait's methods: `reflect.methods_of`
gives them (decision 251), and Q-27 is to say how compile-time code
writes a method with a reflected signature.

## Type checking

`sisu-typeck` checks each body locally and bidirectionally against
explicit signatures, with inference by unification for locals, closure
parameters and the type arguments of std generics. It never fails: every
body yields a THIR body, with ill-typed parts typed `Error`.

- Integer literals are `int` and float literals `f64` unless the expected
  type is another numeric type (03, "Integer literals without context are
  `int`"); `half(3)` with an `f64` parameter is fine, and `a + b` with an
  `int` and an `f64` is `E0373`. The sized integers are "Numbers, bytes
  and slices".
- A parameter is borrowed unless marked `own` or `mut`; as a local it is
  `ref T`, or `T` for a `Copy` type (04, rule 2). `Copy` types: the
  scalars, `str`, `()`, shared references, functions, tuples and `Option`
  of `Copy` types, error sets whose members are `@derive(Copy)`,
  `@derive(Copy)` structs and enums, the sized integers, the capability
  handles, `TestClock`, `Cap` and `Lent`, and the task handles.
- A pattern matched against a place binds references to non-`Copy` parts
  and copies of `Copy` ones; against a temporary it takes the parts by
  value.
- Error sets: `T ! E` is `Result[T, E]`; an error set type is a union of
  declarations, and a value of a subset coerces to a superset. `?`
  coerces the operand's set into the function's (`E0321` with a fix
  otherwise). A function declared `T !` has its set inferred: an absorbing
  variable collects every set that reaches it, and the sets of all such
  functions are iterated to a fixpoint (at most 32 rounds) before the
  final pass.
- A `Result` or `Option` value that is neither used nor discarded with
  `_ = ...` is an error (`E0318`).
- `pure` (D-074): a pure function takes no capability, and a closure
  passed for a `pure` function type must be pure (`E0334`, `E0335`);
  which functions it calls, and which `Drop` its drops run, are checked on
  SIR by the ownership checker (`E0333`, `E0392`, `E0393`). A `comptime`
  expression is a value to the code around it.
- Closures capture by reference; an `own` closure (`own x -> ...`) moves
  what it captures in (copies what is `Copy`), cannot change it
  (`E0424`), and so may be stored and returned. A task's captures are
  "Send, Sync and what a task captures".
- What a `Shared`, `Cell`, `Atomic`, `Mutex`, `RwLock`, `Cap`,
  capability handle or channel owns can be neither a view (`E0421`) nor
  must-consume (`E0422`, which
  looks through fields), checked once the type is known (a channel's
  receiver may keep what was sent longer than what it borrows lives,
  decision 230); a `Shared` value is reached through the owner, and
  nothing may be changed through it except inside a `Cell` or a `Mutex`.
- Coercions where a type is expected, besides those below: a value
  implementing a capability's operations trait to the handle
  (`Coercion::ToCap`), a narrower integer to a wider one
  (`Coercion::IntWiden`), an `f32` to an `f64` (`Coercion::FloatWiden`),
  and a `list[T]` or `bytes` to a `Slice` (a call of `as_slice`).
- Bounds are checked once inference is done: a bound fixes a defaulted
  associated type to its default (`Program::bound_holds`), while the
  `Self: Trait` of a trait method's own call asks only for an impl.
- A format spec is checked where it is written (`E0399`); a spec ending
  in `?` asks for `Debug`.
- `a?.f` through a reference borrows a field that is not `Copy` (as a
  `str` view for a `String`), since nothing can be moved out of a borrowed
  value: `eldest?.name ?? "nobody"` is a `str`.
- `match` is exhaustive by Maranget's usefulness algorithm, with a
  witness of a missing case in the message and a warning for unreachable
  arms. Lists are split by length: with `L` one more than the longest
  fixed part of any list pattern in the column, the constructors are
  "exactly 0", ..., "exactly L-1" and "L or more".
- `todo(name)` is a typed hole (D-061): it checks as the type expected
  there and is reported as warning `W0001` with that type.
- Desugaring decided here: `it` closures, `do` blocks bound to the last
  function-typed parameter, `Type(...)` constructors with field defaults,
  parameter defaults, `range(n)` as `range(0, n)`, literals through
  `FromLiteral` where its implementer is expected, auto-deref through
  references, `Box`, `Shared` and mutex guards, `ref T` to `ref dyn
  Trait`, and implicit borrows of arguments, all made explicit in THIR
  (`Coerce`, `Borrow`, `Deref`, `DefaultArg` nodes).

## SIR

SIR (`sisu-sir`) is the boundary between the front end and any back end:
the interpreter now, the native backend later, and the stage-1 front end
must emit the same form. A program is a list of source names, an entry, a
list of tests, type declarations and functions. A function is a typed
control-flow graph: locals (`_0` is the return place, then the
parameters, for a closure the environment tuple first), and basic blocks
of statements, each ending in one terminator.

```text
Statement  := Assign(Place, Rvalue) | Call { dest, callee, args } | Drop { place, flag? }
              | StorageDead(local)
Callee     := fn NAME[TYPES] | intrinsic NAME[TYPES] | trait TRAIT.METHOD[TYPES]
              | Operand (a function value)
Terminator := Goto(bb) | Switch { value, cases: [(int, bb)], default: bb }
              | Return | Resume | Unreachable
Place      := local projection*      projection := .* | .N | .as(V) | [_i] | [^_i] | {_k}
Operand    := copy Place | move Place | const Const
Rvalue     := Use(Operand) | ref [mut] Place | BinOp(Operand, Operand) | UnOp(Operand)
              | tuple/struct/variant/list/set/dict(Operand...) | dyn TYPE(Operand)
              | discriminant Place | closure NAME[TYPES](Operand...) | copyof Place
              | strview Place
```

A generic function has type parameters, `$0` and on in its types, and
every call passes their arguments; `call trait Trait.method[Self, trait
arguments..., the method's own...]` is dispatched at run time to the impl
whose pattern matches. The program lists the impls it may dispatch to
(`impl[N] Trait[args] for Type { method: function, type Item = T }`, the
impl's parameters `$0` to `$(N-1)` in its patterns; a method it omits is
the trait's provided one, the function `Trait.method`) and declares the
trait methods it calls through `call trait` with their facts (`method
Shape.area pure`), as it declares intrinsics.

Every block may name an `unwind` block, the cleanup that runs when
anything in it panics; cleanup blocks are marked, cannot unwind, and end
in `resume` (or jump to another cleanup block). A panic is a call to a
`Never`-returning intrinsic followed by `unreachable`. Compared with the
statement list in 18 (pipeline step 5), Move, Copy and Borrow are
operand and rvalue forms of Assign; `Yield` and `ContractCheck` arrive
with the phases that need them (tasks, contracts).

`StorageDead(_n)` (`dead _n`) ends a local's storage: a loan of it may not
be live afterwards, and the interpreter forgets its value. Lowering emits
one at the end of the scope of every local that is borrowed somewhere in
its function (others need none). A `drop _n if _f` runs only while the
flag `_f` is `true` and clears it; a `drop` of a variant's payload
(`drop _n.as(V).0`) when the value holds another variant does nothing.

Besides its blocks, a function records what the ownership rules need to
know about it at its call sites, and a program records the same about
every intrinsic it calls:

- `pure`: the function is `pure` (D-074), or a closure whose body is.
- `from`: the arguments (0-based, `self` first, a closure's environment
  first) that the result may borrow from deeply: it may hold the loan of
  the argument itself and every loan the argument holds. These are the
  `from` clause or the elision rule's parameters (resolution's rules
  above), plus every `own` parameter that can hold a loan (a view moved in
  can come back out). A closure's result may borrow from every argument.
- `inner`: borrowed arguments the result borrows from through their values
  only, without the loan of the argument itself: the result cannot point
  into the argument's own storage. That is so when the argument is passed
  by reference and the result can only view text (a `str`, a `view
  struct` of `str` fields) while the argument owns no text (`String` or
  `Path`) outside references: `longest(words: list[str]) -> str` returns
  one of the views the list holds, so the caller may change the list while
  the result lives, but not the text.
- `own`: the arguments taken with `own`. For an intrinsic they are the
  only ones it may keep in what its `mut` arguments point to
  (`list.push(mut self, own x)` keeps `x`); a caller of a function assumes
  the call may keep what they hold in its `mut` arguments.
- For an intrinsic, also `keeps`: the `own` and `mut` arguments whose
  every value it keeps or returns, from its contract's `@keeps`; only
  these may take must-consume values (D-054).

A function records where it is declared (its name, or the closure), for
diagnostics about it as a whole. A local records where its binding is
declared, and whether it is a `let` declared without a value (`once`),
which may be assigned once on every path. A type declaration records
whether it is a `view struct` and whether it is `@must_consume`; a
function type, whether it is `pure`.

Projections: `.*` the referent of a reference; `.N` field or tuple
element `N`; `.as(V)` the payload of variant `V` (then `.N` its field);
`[_i]` the list element at the index in local `_i`, which panics out of
bounds; `[^_i]` counted from the end; `{_k}` the dict entry at the key in
`_k`, which panics if absent when read, inserts when it is the last
projection of an assignment's destination, and is nothing to drop when
dropped.

Binary operators: `add sub mul div floordiv mod pow wadd wsub wmul sadd
ssub smul shl shr bitand bitor bitxor concat eq ne lt le gt ge in`; unary:
`neg not bitnot`. Integer arithmetic traps on overflow and division by
zero (D-009), `floordiv` floors and `mod` has the sign of the divisor
(02, "Operators"); `w*` wrap, `s*` saturate; a shift amount outside 0..63
panics; float arithmetic is IEEE 754. `concat` makes a `String` from two
texts. `eq` and the orderings compare structurally through references;
`in` is membership in a list, set, dict (keys) or string.

### The text form

`sisu sir FILE` writes it and `sisu run --sir FILE` reads and runs it. One
construct per line; the reader accepts `#` comment lines and blank lines.
Reading what was written gives a program that writes the same text, and
the conformance runner checks for every case that running the text gives
the same result as running the source.

```text
sir 0
source 0 "wordcount.sisu"
source 1 "<std>/prelude.sisu"
entry main system result          -- `system`: main takes System; `result`: it returns T ! E
export "sauna_heat" heat          -- `@export("sauna_heat")` on `heat` (decision 683)
test "counts case-insensitively" test#0
test "parse/rejects empty input" test#1 tags "slow" "io"   -- its `@tag`s, when it has any
bench "wordcount" bench#0          -- a benchmark: its name and the function that runs it

struct Options { top: int = Options#top, paths: list[Path] }
struct Res drop Res.drop { id: int }
view struct Token { text: str }
struct Receipt must_consume { amount: int }
enum Shape { Circle(r: f64), Dot }
error IoError open { NotFound(path: Path), Interrupted }

intrinsic print
intrinsic list.push pure own 1 keeps 0 1
intrinsic str.split_whitespace pure from 0
intrinsic Iter.next pure inner 0 keeps 0

method Shape.area pure
impl[1] Eq for list[$0] { eq: list.Eq.eq }
impl Iterator for Count { next: Count.Iterator.next, type Item = int }

fn NAME[T, U](PARAM_COUNT) [closure|drop|glue|value|test|comptime STEPS] [pure] source N [from I...] [inner I...] [own I...] [@L:C-L:C] {
    let _0: TYPE ["source name"] [once] [@L:C-L:C]
  bb0:
    STATEMENT [@L:C | @L:C-L:C]
    TERMINATOR [@L:C | @L:C-L:C]
  bb1 (unwind bb3):
    ...
  bb3 (cleanup):
    ...
}
```

Statements and terminators:

```text
_1 = copy _2.0                          _1 = move _2.as(0).0
_1 = const 3 | const -1.5 | const 2e100 | const nan | const inf | const true
_1 = const "text\n" | const 'c' | const () | const fn helper
_1 = ref _2 | ref mut _2[_3] | ref _4.*.1
_1 = add copy _2, const 1               _1 = not copy _2
_1 = tuple(copy _2, move _3) | struct Point(...) | variant Option::Some(...)
_1 = list(...) | set(...) | dict(k1, v1, k2, v2)
_1 = discriminant _2 | closure main#closure0(copy _3) | copyof _2.* | strview _2
_1 = dyn ref Sq(copy _2)
_1 = call fn parse_args(copy _2) | call fn total[Sq](copy _2) | call intrinsic dict.new[String, int]()
_1 = call trait Shape.area[$0](copy _2) | call copy _3(copy _4)
drop _1 | drop _1 if _5 | dead _1
goto bb2 | switch copy _1 [0: bb3, 1: bb4, else: bb5] | return | resume | unreachable
```

The program `sisu run`, `build` and `sir` see is built without its tests:
it has no `test` and no `bench` lines. `sisu sir --tests` prints the
program `sisu test` and `sisu bench` run, built with them (`@cfg(test)`
holds): a `test` line for each test, in the order the program declares
them, its name the test's own after its suites' (`"suite/test"`), the
function `test#K` (a parametrised test is a test for each case, its name
the case's), and, after the function, `tags` and each `@tag("...")` of the
test as a quoted text, in the order written, when it has any; then a
`bench` line for each benchmark, its name after its suites' too and the
function `bench#K`, which the program's functions include. `sisu test
--from-sir FILE` and `sisu bench --from-sir FILE` run them (decision 285).

Types: `int f64 bool char str () Never`, a named type with arguments
(`list[int]`, `Option[str]`, `cli.Error`), tuples `(int, str)` and
`(int,)`, error-set unions `{IoError, cli.Error}`, function types
`fn(int) -> bool` and `pure fn(int) -> bool`, `ref T` and `ref mut T`, a
type parameter `$0`, `dyn Shape` (a value carrying its type), an
associated type of a parameter, `<$0 as Iterator>.Item`, a constant
argument (`256`) and an integer operation on constant arguments not
known yet (`mul($1, $2)`, `add($0, 2)`, `neg($0)`: the operators' SIR
names), which substitution folds to a constant.
A location is the source range of the construct, `@line:col-line:col`
(the end exclusive), or a point `@line:col`; the ownership checker's
diagnostics are built from these ranges, so they must be exact.
Constants: integers in
decimal, floats always with a `.` or an exponent (or `nan`, `inf`,
`-inf`), `true` and `false`, strings and characters with the escapes
`\n \t \r \0 \\ \" \'` and `\u{hex}`, `()`, and `fn NAME`.

Names: a function is its source name; a method is `Type.method`, and a
trait impl's method `Type.Trait.method`; a trait's provided method is
`Trait.method`; a std module function is `module.function`; a test is
`test#N`; a closure is `OWNER#closureN`; a `comptime` expression is
`OWNER#comptimeN`; a field default is `Type#field` and a parameter
default `function#param`, each a `value` function of no parameters; the
drop glue of a type is `Type#drop`. Builtin types (`String list dict set
Iter Entry Path DateTime Fs Env Clock System Option Result Iterable`) are
not declared; every other type a function mentions is, with its type
parameters, and with `drop` naming its `Drop` function.

**Drop glue** is a function of kind `glue` per declared type whose drop
can do anything (run a `Drop`, change a count, unlock a mutex), made by
lowering: it takes the value with `own` (`_1 "self"`), calls the type's
`Drop` with a reference to the whole value, then drops the fields in
reverse declaration order (of the active variant, through a switch on the
discriminant, for an enum), each in a block of its own, and returns right
after the last. Cleanup blocks beside them form a ladder: if the `Drop`
or a field's drop panics, the fields that remain are still dropped,
where a second panic aborts. A field whose type drops nothing is left
out; a type parameter's field is dropped, since it may stand for
anything. The host-backed types have no glue: their drops are part of
their contracts (section "The interpreter").

**Kinds**: `closure` (parameter 1 is the environment tuple), `drop` (the
method of `impl Drop`), `glue`, `value` (a default or a constant, or a
`comptime` expression once evaluated), `test`, and `comptime STEPS`: a
`comptime` expression not yet evaluated, with its budget of steps. The
driver evaluates and bakes every `comptime` function before it writes or
runs SIR, so written SIR has one only when the ownership checker
rejected the program; running one computes its value at run time.

### Invariants

`sisu_sir::validate` checks them, and the driver runs it before every run:
every local, block and function named exists; unwind edges lead to cleanup
blocks; cleanup blocks do not unwind; `resume` only in cleanup; `from` and
`inner` name existing arguments; `dead` names a local other than the
return place and the parameters; types and type arguments name only the
function's parameters; an impl's methods exist and its patterns name only
its parameters; a type with a `Drop` has its glue, and glue is a declared
type's, taking the value, with the type's parameters; and the lowering
invariant: a statement
that initialises a value needing drop, or drops one, is the last
statement of its block, so each block's unwind edge is exact for every
statement in it. The ownership checker keeps the invariant when it
places drops (a flagged drop, or the drops of a partly moved value's
parts, each end their block).

## Lowering

`sisu-lower` turns each THIR body into SIR functions (the body and its
closures). The rules, so that another front end produces the same SIR:

- **Scopes and drops.** Each block and each statement is a scope; a value
  that needs drop (not `Copy`, not a reference) is registered in its scope
  when it is initialised, and dropped when the scope ends, in reverse
  order of registration. `return`, `break`, `continue` and `?` drop every
  scope they leave, innermost first. Temporaries of a statement live until
  the end of the statement. The operands of one statement (a call's
  arguments, an aggregate's parts) are evaluated in order, and one held in
  a temporary that needs drop is registered from its evaluation until the
  statement takes it, when an operand after it may leave the statement (a
  `?`, a `return`, a panic: anything but literals, locals, functions,
  closures, and the parts, borrows and aggregates of such, a struct with
  an invariant or a field's default excepted); a struct with an invariant
  is registered while the check after its construction runs (decision
  222). Every drop is unconditional, whether or not
  the value was moved: the ownership checker decides which drops stay,
  which go and which need a flag. A scope's drops carry the location of
  its end (the end of its last statement, as a point), and after them
  comes a `dead` for every local of the scope that is borrowed anywhere in
  the function, so that a borrow outliving its local is caught where the
  local goes.
- **Unwind edges.** A block's unwind edge is computed when the block
  starts, from the values registered at that point: a chain of cleanup
  blocks dropping them innermost first and ending in `resume`. The chain
  for a given live set is built once and shared. Because initialising or
  dropping such a value ends the block, the edge is exact.
- **Borrow facts.** Each function gets its `pure`, `from` and `inner`
  lists from its signature, and every intrinsic the program calls gets a
  declaration with the same facts and its `own` arguments, from its
  prelude signature (lowering-only intrinsics have fixed ones:
  `iter.ref`, `iter.mut` and `iter.own` borrow from their argument,
  `list.slice` returns copies of what it holds).
- **`defer`** registers its block as a scope entry; the block is emitted
  at every exit of the scope, on the normal and the cleanup paths, in
  order with the drops.
- **Control flow.** `if`/`elif` chains and conditions become `switch` on
  a `bool`; `while` is a head block with the condition and an exit; `for`
  calls `iter.ref`, `iter.mut` or `iter.own` (or uses an `Iter` directly)
  and then `Iter.next` each round, switching on the option; iterating a
  reference (a borrowed parameter) borrows what it points to, and an
  `Iter` behind a reference is driven through the reference, which the
  loop does not drop (decision 196). The loop's
  own statements are located at its head, `for x in xs`. `and`/`or`
  short-circuit. A comparison chain evaluates each operand once.
- **`match`** tests each arm's pattern in order, binding nothing until it
  has matched, so a failed test leaves the scrutinee as it was; guards are
  tested after binding. A variant pattern on a union of error sets first
  calls `error.is[Decl]` to learn which declaration the value belongs to,
  through a reference of that declaration's type, and tests and binds the
  rest through it. A variant pattern on an error value that moves a field
  out takes the value out whole first, into a local of its declaration's
  type, and moves the fields out of that, which is dropped with the
  bindings: an error value is never partly moved (decision 221).
  The arms at the start that test only which variant the value is (a
  variant whose fields cannot fail, or a choice of such variants), and
  have no guard, share one `switch` on the discriminant with a case per
  variant, an earlier arm taking a variant first; the other arms are
  tested in turn from its default. A `match` over many variants without
  payloads is then two steps whatever the variant, where a test per arm
  and per alternative took two steps each (decision 175). An or-pattern
  that binds names binds from the alternative that matched: the
  alternatives but the last are tested again in order (a test reads and
  changes nothing), each binding its names into the same locals, and the
  bindings are registered once where the alternatives join (decision
  182).
- **`?`** switches on the discriminant; the error path moves the payload
  out, calls `error.trace_push` with the location text `file:line:col`,
  stores `Err(e)` (or `None`) in `_0`, drops every scope and returns.
- **Closures** become a function whose parameter 1 is a tuple of
  references to the captured locals (`ref mut` when the closure assigns
  them); the closure value is `closure NAME[TYPES](refs...)`, with the
  type arguments of the function that makes it, and holds the loans of
  those references, so the ownership checker keeps it from escaping the
  frame of what it captures. An `own` closure's tuple holds copies
  instead, and no loan. A function value is dropped as a value that holds
  a capability handle is, since its environment may hold copies of
  handles (decision 248): lowering drops every value of a type that holds
  a function value, and a callee drops a function value it was given.
- **Functions as values.** A function used as a value is its constant,
  `const fn NAME`, unless it is generic, or borrows a parameter (a
  function value takes its arguments by value): then it is a closure of
  its value wrapper, `closure NAME#value[TYPES]()`, with the type
  arguments inferred where the value is used. The wrapper, made with the
  function's body by `sisu_sir::value_wrapper`, is generic as the function
  is, takes an empty environment and the parameters as the function's
  type has them, lends those the function borrows, calls the function,
  and drops what it lent; a function that returns a borrow of a parameter
  it borrows is not a value (`E0352`, decision 225).
- **Interpolation** builds a `String` with `fmt.new`, then `fmt.text` per
  text part, and per value: without a spec, `fmt.display` or `fmt.debug`
  when the value's impl is structural (the host writes its plain text);
  otherwise a `Formatter` with the spec (`fmt.formatter`), a call of the
  value's `Display.fmt` (or `Debug.fmt_debug`, for `{x:?}`, `{x=}` and a
  spec ending in `?`), and `Formatter.finish`, which pads the text by what
  is left of the spec (section "Formatting").
- **Generic calls** pass every type argument to a SIR function. A trait
  method whose `Self` is known is called as the impl's method (or the
  trait's provided one) directly; otherwise `call trait`. A call whose
  function is `@structural` and whose type arguments meet every bound
  with a structural impl calls the intrinsic instead.
- **Intrinsic calls** carry type arguments when the intrinsic needs them at
  run time: when a generic parameter appears in the result and in no
  parameter (`list.new[T]`, `cli.parse[T]`), or when a parameter is
  `Iterable[T]` (whose items are copies when `T` is not a reference).
- **Drop glue** (section "SIR") is made for every declared type that
  needs it, once the program's types are known: a type with a `Drop`, or
  with a field whose type drops something (a type parameter, a `dyn`, a
  `Shared`, `Weak` or mutex guard, a declared type with glue, or a
  collection, option, tuple, `Cell`, `Mutex` or `Arena` of one).
- **`comptime e`** becomes the function `OWNER#comptimeN` of kind
  `comptime STEPS`, with the item's budget, whose body computes `e`, and
  the expression a call of it.
- Only what the entry, the tests, the root module's functions and its
  constants reach is emitted, with the declarations of every type they
  mention and the impls of every trait they call through `call trait`
  for the types they mention; only those bodies are lowered (a worklist
  from the roots), so the standard library costs a program only what it
  uses. The ownership checker takes std's functions as checked
  (`Options.trust_std`): it elaborates their drops but does not analyse
  their loans again for every program; `sisu test --std` and a test that
  lowers every std body (`lower_every_body`) check them.

### Lowering-only intrinsics

Besides the prelude's intrinsics, lowering introduces these; their
contracts:

| Intrinsic | Contract |
|---|---|
| `iter.ref(ref c)`, `iter.mut(ref mut c)` | an `Iter` over a borrowed `list`, `set` or `dict`: references to the elements, or `(ref K, ref V)` pairs for a dict, in order |
| `iter.own(c)` | an `Iter` that consumes `c`; a dict yields `(K, V)` pairs |
| `fmt.new()` | an empty `String` |
| `fmt.text(ref mut s, t: str)` | appends `t` |
| `fmt.display(ref mut s, v, spec: str)`, `fmt.debug(...)` | appends the plain `Display` (or `Debug`) text of a value whose impl is structural; the spec is empty (a spec is applied in Sisu) |
| `fmt.formatter(spec: str) -> Formatter` | `Formatter(out="", spec=spec, depth=0)` |
| `assert.fail(text, message)` | panics with `assertion failed: text` and the message |
| `assert.fail_compare(text, message, left, right)` | the same, with the `Debug` renderings of both sides |
| `error.trace_push(ref mut e, location: str)` | appends a location to the error's return trace |
| `error.trace(ref e) -> String` | the trace, one `at location` per line |
| `error.is[D](ref e) -> bool` | whether the error value belongs to declaration `D` |
| `list.slice(ref xs, lo, hi)` | a new list of copies of `xs[lo:hi]`, bounds clamped to the length; negative bounds panic |
| `list.slice_middle(ref xs, n, m)` | copies of the elements between the first `n` and the last `m` (a `*rest` binding) |
| `cap.new[T, H](own v)` | a handle of type `H` on a new heap cell holding `v` as a `dyn` of type `T`, never freed (a `Coercion::ToCap`) |
| `u64.from_bits(n)`, `u64.widen(x)` | the `u64` whose 64-bit pattern is `n`; an integer as a `u64` (a literal; a widening) |
| `i128.widen(x)`, `u128.widen(x)`, `i128.from_text(t)`, `u128.from_text(t)` | an integer that the type holds as a value of it (a literal that `int` holds; a widening); the value the decimal text spells (a literal beyond `int`) |
| `generic.value[$i]()` | the value of the function's `const` parameter `$i`: its type argument, an `int` |
| `T.add(a, b)` ... `T.not(a)` for a sized integer type `T` | the operators on `T`: `add`, `sub`, `mul`, `floordiv`, `rem`, `pow` trap at `T`'s bounds (`R0003`), `wrap_*` wrap, `sat_*` saturate, `shl` and `shr` take 0 to the width less one (`R0093`), `and`, `or`, `xor`, `not` are on the bits |
| `bytes.from_hex(text)` | the bytes the hex digits spell (a `b"..."` literal) |
| `select.wait(kind, subject, ...)` | the index of the `select` arm to take (06, "Channels and select"; "Tasks and the scheduler") |
| `gen.new(own body)`, `gen.yield(own item)` | a generator's iterator; giving an item to its consumer |
| `dict.get_str(ref d, k: str)`, `dict.remove_str(ref mut d, k: str)`, `dict.index_str(ref d, k: str)`, `set.contains_str(ref s, k: str)`, `set.remove_str(ref mut s, k: str)` | a lookup of a `dict[String, V]` or a `set[String]` by the text `k`, compared with the keys' bytes without a `String` made: `dict.get`'s, `dict.remove`'s, `d[k]`'s (a reference to the value; a missing key panics, `R0010`), `set.contains`'s and `set.remove`'s results (decision 784); no front end lowers to them yet |

A call through the trait of a capability handle (`cap.forward` in the
prelude) is a `call trait` whose `Self` the dispatcher replaces by the
object in the handle's cell.

## The ownership checker

`sisu-borrowck` checks the memory model of
[04](../../proposal/04-memory-and-ownership.md) on SIR (18, Phase 3, and
pipeline step 6, "Checkers on SIR"). It takes lowering's SIR, in which
every drop is unconditional, and returns the SIR that runs, with its
drops placed, together with its findings. It works on one function at a
time (so it analyses the functions on threads, decision 219) and never
looks at THIR: everything it needs about a callee is in SIR (`pure`,
`from`, `inner`, the intrinsic declarations), so the stage-1 front end
gets the same checker by emitting the same SIR.

The rule it follows throughout is that **the checker is sound before it
is complete**: where it cannot tell, it rejects, and each such case is a
known over-rejection, listed below, because the experiment of 20
measures false positives. The dynamic monitor re-checks every monitored
run (every `sisu test`, and every conformance case that runs), and the
counterexample mode reruns rejections, so a hole or an over-rejection
shows up as data rather than as a belief.

The steps, in order:

### 1. Initialisation and moves

A *move path* is a local followed by field and variant projections
(`_3`, `_3.1`, `_3.as(0).0`); a place through a reference, an index or a
key is tracked as the path before that projection, since nothing can be
moved out of it. Paths form a tree per local, with one child per variant
under every place whose discriminant is read.

A forward dataflow gives each path two bits before every statement, *may
be initialised* and *may be uninitialised*, joined by union. Assigning a
path sets the first and clears the second for it and every path below
it; a move, a drop or a `dead` does the opposite. A switch on a
discriminant clears both bits of the other variants' payloads on each
edge, so a payload that cannot exist there counts as neither. Unwind
edges carry the state at every statement of the block, after its moves.

Findings, at each use of a path (a copy, a move, a borrow, a
discriminant read; a drop is not a use):

| Code | When |
|---|---|
| `E0400` | the path may have been moved (with the move's location; in a loop, the move in an earlier iteration) |
| `E0401` | a part of it may have been moved, and it is used as a whole |
| `E0366` | it may never have been assigned (a `let` declared without a value) |
| `E0367` | a `once` local is assigned where it may already hold a value |
| `E0416` | a move out of a field of a value whose type implements `Drop`, which needs the whole value |
| `E0419` | a move out of a place reached through a reference, an index or a key |

### 2. Drop placement

Each unconditional drop is decided by the initialisation state before it:

- initialised on every path: kept;
- uninitialised on every path (moved, or a payload of a variant that is
  not active there): removed;
- moved as a whole on some paths: guarded by a drop flag, `drop _3 if
  _9`, where `_9` is a new `bool` local set after every statement that
  initialises the path and cleared before every statement that moves or
  drops it (a parameter's flag starts `true`);
- partly moved, or a value whose type has variants of which only some can
  be active: opened into the drops of the parts that may hold something,
  fields in reverse declaration order (04, rule 6), each decided the same
  way. A type that implements `Drop` is never opened (moving out of it is
  `E0416`), so its `Drop` always sees the whole value. A value one of
  whose parts some statement moves or drops on its own is opened wherever
  it may be uninitialised, even where its parts have its own two bits:
  one flag cannot tell a path that moved the whole from one that moved
  the part (decision 220).

A drop outside a cleanup block still ends its block (the lowering
invariant), so placing drops splits blocks; statements the checker adds
at the start of the function go in a new entry block when the old one is
a jump target. This replaces lowering's two-pass drop flags, and covers
partial moves, which lowering could not.

### 3. Loans

A *loan* is made by every borrow statement (`ref P`, `ref mut P`,
`strview P`), by every copy of a `ref mut` used as an operand (a
reborrow of what it points to), and, at entry, by every parameter that
can hold a loan (what the caller lent through it, `Param(i)`).

**Which locals hold which loans** is a forward dataflow (union). A
borrow's destination holds the new loan and the loans of the value it
borrows (a reference is only valid while what it points into is); a copy
or move passes the loans on; an aggregate holds its operands' loans; a
call's result holds the loans of its `from` arguments (the argument's
own and everything it holds) and of its `inner` arguments (only what the
argument's value holds; a reference read out of a value is treated as
pointing into every place that value holds a loan of); an intrinsic
keeps its `own` arguments' loans in what its `mut` arguments point to;
`d{k} = v` keeps the loans of `k` in `d`, since a missing key is
inserted. For a local holding a reference, the dataflow also knows which
of its loans are the reference itself, when it can tell, so that a write
through it reaches only that loan's place.

**Which locals are live** is a backward dataflow: a local is live where
it may be read later before being overwritten. Dropping a value whose
drop glue runs a user `Drop` reads it, since `Drop` may use what it
borrows (the conservative reading of Rust's drop check, without
`may_dangle`). A loan is live where some live local holds it: borrows
end at their last use, not at the end of the scope (04, "Borrowing rules
in detail").

**Accesses.** Every statement makes accesses to places, in order: reads
(a copy, a shared borrow, a discriminant, the index and key locals of a
place), writes (an assignment's destination), moves, `mut` borrows, drops
at the end of a scope, drops of the old value before an assignment, and
`dead`. An access conflicts with a live loan of an overlapping place when
it writes, moves, drops, ends the storage or borrows `mut`, and when it
reads and the loan is `mut`. Places overlap unless they differ in a field
or a variant at the same depth; indices and keys are never told apart.

**Two-phase borrows.** A `mut` borrow into an unnamed temporary (the
receiver or argument of a call) is reserved until the call uses it:
until then it conflicts only as a shared borrow would, so
`xs.push(len(xs))` is accepted, and at its first use it must not
conflict with any other live loan. A `ref mut` bound to a name is active
at once, as in Rust.

Conflicts are reported at the access, with a label at the loan ("borrowed
here", naming the holder when it has a name) and at the next use of the
loan after the access; `{line}` is where the loan was made and `{end}`
the last line where it is live, which for a loop's iterator is the end of
the loop's body. A borrow that outlives its place is reported once
(`E0406`); the conflicts its loan meets afterwards follow from it and are
not reported.

| Code | The access, while a loan of an overlapping place is live |
|---|---|
| `E0402` | a modification (assignment, or the drop before one) |
| `E0403` | a use, while the loan is `mut` |
| `E0404` | a move |
| `E0405` | a `mut` borrow (a call with `mut`, a mutating method, a two-phase borrow's first use) |
| `E0406` | the end of the place's scope (its drop or its `dead`): the borrow does not live long enough |

### 4. Escapes

`ref` is second class (04): it can be returned only as its signature
allows, stored only where a view may go, and never outlives the frame of
what it borrows. At every `return`, the return place may hold only the
loans of `from` and `inner` parameters (`E0408` names the parameter that
is not in the clause, with the fix of adding it) and no loan of a place
of the function itself (`E0407`; for a closure, the note that a stored or
returned closure must own what it captures). A value holding a loan may
not be stored in a plain struct or enum (`E0409`, with the fix of
declaring the struct a `view struct`), and a value holding a
loan other than a `mut` parameter's own may not be stored into what that
parameter points to (`E0410`: the caller could not know its value now
borrows something else), whether by an assignment through it, a key
inserted into it, or an intrinsic keeping an `own` argument there (an
entry of a dict counts as pointing into the dict). A closure capturing a
local holds the loans of its captures, so storing or returning it is
caught by the same rules: it never escapes the frame of what it borrows.
"Never sent across a task" holds by the task rules (section "Send, Sync
and what a task captures") and by the type of what a channel carries,
which is never a view (decision 230).

### 5. Must-consume

A value of a `@must_consume` type, or a collection, option, tuple,
iterator or aggregate holding one (D-064), may not be dropped on a normal
path (D-054):

- `E0417`: a drop at the end of the value's scope;
- `E0418`: a drop on a path that leaves the scope early (`return`, `?`);
- `E0420`: an intrinsic that may drop what it is given receives one with
  `own` or `mut`. Only the arguments its contract names with `@keeps`
  may take one (`list.push`, `pop`, `insert`, `remove`, `extend`,
  `reverse`, `dict.insert`'s value, `dict.remove`, `Iter.next`, the
  `Arena`'s `insert`, `take` and `get_mut`); `list.clear`, `set.add`
  (which drops a duplicate) and the key of `dict.insert` may not.
- `E0397`: a must-consume type argument of a generic function whose body,
  instantiated with the call's type arguments (drops placed for them),
  drops one, or passes one on to a generic function that does. The check
  runs per instantiation, as 18 describes for monomorphised code; a
  recursion deeper than eight calls is assumed to drop.

Exempt are drops in cleanup blocks (unwinding, D-054), drops in the
type's own methods (`def settled(own self)` is where a receipt ends),
drop glue (it drops the parts of a value whose own drop was checked
where it happened), and
the drop of a consuming loop's iterator once `Iter.next` has returned
`None` on every path to it: `for r in own receipts` discharges the list,
and leaving the loop early is the error 04 describes (named after the
collection, with a note that the loop has not visited every element).

### 6. Purity

A `pure` function, and every closure it creates, may call only pure
functions and intrinsics and function values of a `pure` type, and may
drop only values whose `Drop` glue is pure (`E0333`, `E0392`, `E0393`,
each with a label at the `pure` declaration). Unwinding is exempt, as for
must-consume.

### `sisu explain-borrow FILE:LINE`

10's "explain this borrow": for each function with a statement on the
line, every loan live there, with what it borrows, where it starts, the
names holding it on that line, and the last position where it is live.
Loans that only the machinery of one expression holds (a formatting
buffer) are left out.

### Known over-rejections

Each of these rejects programs that are memory safe. They are the
checker's known false positives, and each candidate false positive the
counterexample mode finds should be checked against this list first.

1. **Elements are not told apart.** A borrow of `xs[0]` is a borrow of
   every element for the rules, so assigning `xs[1]` while it is live is
   rejected; so are two keys of one dict. (The monitor tells them apart:
   `ownership/candidate_false_positive`.)
2. **A call borrows each argument whole.** Passing `mut p` conflicts with
   a live borrow of any field of `p`; a signature cannot say which fields
   a function touches.
3. **`from` is a contract, not a flow.** A result is assumed to borrow
   from every `from` argument (deeply, unless `inner` applies), even when
   the body returns only one of them.
4. **Two-phase borrows are only for call temporaries.** `let r = ref mut
   xs; print(len(xs)); r.push(1)` is rejected, as in Rust.
5. **Closures capture whole variables, by reference, for as long as the
   closure lives**, and a closure that assigns a capture borrows it with
   `mut`; a plain closure that captures anything is rejected when stored
   in a struct, returned, or kept past the frame of its captures. An
   `own` closure may be, but it copies its captures, so it can capture
   only `Copy` values until capture by move arrives with tasks.
6. **Stores into a `mut` parameter.** Storing a value that holds any loan
   other than the parameter's own through a `mut` parameter is rejected,
   even when the caller's argument would outlive the value (a string
   literal): the caller's knowledge does not cross the signature.
   (`ownership/store_into_caller` is a candidate false positive in the
   counterexample mode for that reason.)
7. **User `Drop` glue keeps loans alive to the drop.** A value whose type
   (or a part's type) implements `Drop` keeps everything it borrows
   borrowed until it is dropped, since `Drop` may read it.
8. **Must-consume is about types, not contents.** An empty list of
   must-consume values is still must-consume; `xs.clear()` on one is
   rejected even when it is empty; leaving a consuming loop early is
   rejected even when no element is left; an intrinsic argument without
   `@keeps` is rejected for any must-consume value, even one it would
   keep; a generic function that drops a `T` on some path is rejected
   for a must-consume `T` whatever path the call takes.
9. **A reference read out of a value points anywhere that value
   borrows.** `Some(x)` and `(a, b)` lose which part holds which loan
   when a reference is read back out of them.
10. **Purity of drops is decided per drop statement.** A `pure` function
    is rejected for a flagged drop of a value with an impure `Drop` even
    when every run moves the value first; only a drop that the placement
    removed (the value is moved on every path) is not checked.
11. **An inserted key is kept even when it was there.** `d[k] = v` keeps
    the loans of `k` in `d`, although a dict that already has the key
    keeps its old key and drops nothing into itself.
12. **Rules about declarations are not about runs.** A plain struct with
    a view field, an enum payload of view type and a missing `from`
    clause are rejected whatever the program does (`E0411`-`E0415`); the
    counterexample mode reports them as such.
13. **A type parameter may stand for a view.** A value of a generic type
    is assumed to hold the loans its parameters may, so storing a generic
    value where a view may not go is rejected even when every call's
    type argument owns its data.
14. **What a `Shared`, `Cell`, `Mutex` or channel holds is decided by its
    type.** A view or a must-consume value cannot be put in one (`E0421`,
    `E0422`) even when it would be released before what it borrows.
15. **A `mut` parameter read in its own method call is a `mut` reborrow.**
    `def add(mut xs: list[int]): xs.push(len(xs))` is rejected (`E0405`),
    where the same call on a local list is accepted as a two-phase borrow:
    lowering passes the parameter, a `ref mut`, to `len` as a copy, which
    is a `mut` reborrow (decision 40) that ends the receiver's loan, where
    a local is passed as a shared borrow. The monitor agrees with the
    checker on that SIR; lowering a read of a `mut` parameter as a shared
    reborrow (`ref xs.*`) would accept it. The wrong-ownership corpus found
    it (decision 263).

## The interpreter

`sisu-interp` runs SIR directly (18, "The interpreter").

**Frames and the run loop.** A frame holds its function, its type
arguments, locals, current block and statement index, its current source
location, the values it has to drop before its next statement, and,
while it runs cleanup, the panic it is unwinding from. The loop is a
trampoline: a call to a SIR function pushes a frame and the loop
continues in it; a return pops it and stores the result in the caller's
destination. Sisu calls never recurse on the host stack (a test runs
50 000 nested calls on a small test thread); deeper than 100 000 frames
(`MAX_DEPTH`) is a stack overflow, a panic that names the function whose
frame does not fit (`panic.stack-overflow`), with the message and exit
status of a native stack's overflow; the unwinding that follows may push
1 000 frames more (`UNWIND_DEPTH`) for the drops it runs, and a
generator's frames count on top of its consumer's (decision 234). No
intrinsic calls back into Sisu (the functions that take closures are
written in Sisu) except `cli.parse`, which runs a field's default; the
host's structural operations recurse over deeply nested values. The run
happens on a thread with a 256 MiB stack so that both are safe.

**Values** are a tagged enum: `int` (and every sized integer type but
`u64`, `u128` and `i128`, kept in its type's range), `u64`, `u128`,
`i128`, `f64`, `bool`, `char`, `()`, `str` (an immutable shared text,
which a piece of it copies), `String` (its UTF-8 bytes), `Path`, `bytes` (immutable,
shared by its copies), `DateTime` (nanoseconds since the Unix epoch,
UTC), tuples, structs, enum values (with an error return trace when the
enum is an error set), `list`, `dict` and `set` (insertion-ordered, as
D-074 and 11 require, with a hash index), function values, closures
(function, type arguments and captures), `dyn` values (a value and its
type), `Shared` and `Weak` handles on a heap cell, `Cell`, `Mutex` and
its guard, `Arena`, iterators, dict entries, capability handles and
`Cap`s (a heap cell that counts the handle's copies, decision 237),
clocks, host sockets, the task
handles (scopes, task handles, cancellations, channel halves, timers),
references, raw pointers (a block of the checked memory model, an offset
and a loan: "The checked memory model" below), and `Moved`. A `Slice` is a `list` of its values (a copy,
like `str`'s; port-debt.md), and an `Array` a `list` of exactly its
length.

**References are places**: a frame index and serial number, a local, and
a path of steps (field, payload, list index, dict key or value position,
set element), and under the monitor the tag of the loan that made the
reference. Reading or writing walks the path from the frame's local; a
reference into a frame that has returned is detected by the serial number
and panics (`R0034`). Resolving a SIR place follows `.*` through
references, so a pointer never contains a dereference.

**Moves** take the value and leave `Moved`; `dead` leaves it too. Reading
`Moved`, or a part of it, panics (`R0033`); the ownership checker rejects
every program that can, and under the monitor it is a violation.
Dropping `Moved` does nothing.

**Drops** run on the loop. A `drop` takes the value out of its place and
puts it, with the place's type made concrete, on the frame's stack of
values to drop, which the loop empties before the frame's next statement.
A value of a declared type with glue is dropped by pushing the glue's
frame, with the type's arguments from the place's type; the glue's last
drop is handed back to its caller, so a chain of values (a list of
boxes) takes one frame, not one per link. A host-backed value is taken
apart there and its parts go back on the stack, so the first part is
dropped with everything it owns before the second: a `list`, `set` or
`Arena` first to last, a `dict` entry's value before its key, a tuple's
elements last to first, an option's or result's payload, an iterator's
remaining owned items. A `Shared` decrements its count and drops the
value with the last owner; a `Weak` decrements its own. A mutex guard
unlocks; when a panic is unwinding, the value's type's `invariant` is
checked first, as a frame of its own, and the mutex is poisoned if it
fails (D-077). A drop never nests the host stack and a `Drop` runs like
any call. A frame that a panic leaves without a cleanup block still
drops what it was dropping, then lets the panic continue. Because the
place is `Moved` from the moment its drop begins, a cleanup block reached
from that drop does not drop it again. Drop glue frames are left out of
stack traces. A value dropped inside an intrinsic (`list.clear`,
`Cell.set`) has no static type, and the glue of a generic type then runs
without type arguments (port-debt.md).

**Trait calls** (`call trait`) are dispatched by matching the impls'
patterns against the concrete type arguments, as described under
"Traits and generics"; a frame's types are made concrete by its type
arguments and by the impls' associated type bindings.

**Shared ownership and interior mutability.** A `Shared` is an owner of a
heap cell (its value, a strong and a weak count, and whether the count is
atomic, which the escape analysis decides per construction site: "Send,
Sync and what a task captures"); `.copy()` makes another owner, and the
value is dropped with the last one. `Weak.upgrade` gives a `Shared`
while the value lives. A reference to a `Shared`'s value is a pointer
into the heap cell. `Cell` replaces its value through a shared reference
and never gives out a reference into it. `Mutex.lock` gives a guard
through which the value is reached; locking a mutex another task holds
waits, locking one the task itself holds panics (`R0073`), as does
locking a poisoned one (`R0074`). An `RwLock` (std.task) is a mutex that
also counts its readers: `read` gives a `ReadGuard` whose value is only
read, and waits while a task writes or waits to write, so readers that
keep coming cannot keep a writer out (decision 249); `write` gives the
guard a `Mutex` gives, and waits while any task reads or writes. An `Atomic` is a `Cell`
that tasks may share, each of whose operations is one step; `Once` and
`Condvar` are Sisu in std.task over a `Mutex` and channels (decision
239). An `Arena[T]` holds
slots with generations; `insert` gives a `Key[T]` (a `Copy` struct of the
slot and its generation), `remove` frees the slot for reuse under a new
generation, and a stale key gives `None` from `get` and panics when
indexed (`R0072`).

**Panics** unwind: a panicking statement makes the frame continue in its
block's cleanup, which ends in `resume`, which continues in the caller's
cleanup for the block holding the call; a block without an unwind edge
leaves the frame at once. Each frame the panic leaves adds a line to its
stack trace. A panic while running cleanup aborts (05, "Panics"),
reporting both panics. The report is printed when the panic reaches the
top of `main`, after unwinding has run every `Drop`:

```text
panic: index out of bounds: the length is 3 but the index is 10
  at level2 (unwind.sisu:10:12)
  at main (unwind.sisu:20:11)
```

**Exit statuses**: 0 success; 1 when `main` returns an error, printed as
`error:` and its `Debug` rendering followed by its return trace; 101 a
panic; 134 an abort (a panic while unwinding, a run past its step
limit, or an ownership violation under the monitor). The driver adds 2
for usage errors and 70 for an internal compiler error.

**Formatting** is the next section's; the host's part is the plain
text of a value and the digits of numbers.

### Speed

The bootstrap runs the stage-1 front end on this interpreter, so its
speed is the bootstrap's (decisions 175 to 178). What the run loop does
per step is unchanged in kind; the work around it is cut where profiling
of the stage-1 commands showed it:

- **Jump threading** (`sisu_sir::simplify`, after the ownership checker
  and before compile-time evaluation): a jump to a block that only jumps
  on goes where that block goes, a block whose jump leads to a block
  nothing else reaches takes that block's statements and terminator, a
  jump to a block without statements takes its terminator, and blocks no
  longer reached are removed, the rest keeping their order. A drop still
  ends its block, a block keeps one unwinding edge for all its
  statements, and drop glue is left alone. No statement is added, moved
  or removed, so a program computes what it did; it takes fewer steps
  (a quarter fewer for stage 1's lexer), and its yield points are the
  backward jumps of the threaded SIR, of which every loop keeps one, since
  a cycle of blocks cannot only go forward. `sisu sir` and the
  counterexample mode print the threaded SIR, which is what runs.
- **Statements without the general machinery**, outside the monitor
  only: a copy of a number, `bool`, `char` or `()`, the arithmetic and
  comparisons of such values and a `switch` on one are read and computed
  in place; a reborrow `ref r.*` shares the pointer `r` holds; a call's
  arguments go straight into the callee's locals, whose vectors are
  reused from frames that returned; a callee's body is looked up once per
  name in the SIR rather than by name at every call. The intrinsics the
  bootstrap calls most (`str.byte_at`, `str.byte_slice`, `str.byte_len`,
  `str.to_string`, `u8.to_int`, `char.code`, `char.from_code`,
  `char.is_whitespace`, `len` of a list, `range`, `Iter.next` of a
  range, `list.push`, `list.pop`, `list.last`, `list.is_empty`,
  `String.push`, and an interpolation's `fmt.new`, `fmt.text` and
  `fmt.display`) are called directly, without the dispatch on their
  name and without collecting their arguments. Each of these gives the
  general path's value and effects or declines before any effect, and
  the general path then makes the step and reports what goes wrong, so
  panics, messages and step counts are the same.
- **The monitor**: a read looks only at the `mut` loans of the place's
  local (a read ends no shared loan, so the shared ones are kept apart),
  and comparing two paths allocates nothing.
- **The ownership checker** keeps the loans of every local of a state in
  one table, so the dataflow copies a state in one allocation, joins what
  reaches an unwinding block instead of keeping a state per statement,
  and checks a block's statements from two states it reuses.

Measured on one machine (4 cores, shared, so times vary by a tenth),
from `compiler/stage1` (20 500 lines of Sisu, 73 tests), against the
compiler before these changes, on the same sources and inputs, the best
of three:

| command | before | after |
|---|---|---|
| `sisu run -- tokens ../std/prelude.sisu` | 7.5 s | 3.2 s |
| `sisu run -- parse --tree ../std/prelude.sisu` | 11.1 s | 4.7 s |
| `sisu check` | 4.5 s | 2.0 s |
| `sisu test` (under the monitor) | 36.3 s | 19.5 s |

On the smaller stage 1 of round 4 (7 800 lines, 59 tests) the same
commands went from 4.5, 7.8, 1.3 and 33.0 s to 1.75, 3.2, 0.6 and
16.9 s. In instructions (callgrind) on those sources, `parse --tree
../std/http.sisu` took 6.45 billion in the interpreter and 11.6 billion
compiling stage 1 before, and 2.95 and 3.7 billion after; the changes to
the monitor alone took the monitored test `random inputs are lossless`
from 16.0 billion to 4.2. A copy of a `str`'s text, which a view makes
(port-debt.md), is not among the costs: `tokens` of the prelude makes
27 000 views copying 1.6 MB in all.

`cargo run --release -p sisu-driver --example speed` prints the front
end's milliseconds per thousand lines and the interpreter's nanoseconds
per step for std and for the conformance corpus; CI runs it on every
push for the record, and it fails nothing (port-debt.md, Phase 0).

### The host contract

Everything a program does to the world goes through the `Host` trait:
standard output and error, the arguments after the program's name,
environment variables, the clock and real-time sleeps, file reads, writes,
metadata, listings, removals and directories, random bits, running a
program to its end, and TCP (connect, listen, accept, read, write and
close, each of which reports `Wait` instead of blocking), and, under
`sisu run --watch`, the saves of the program's files since the last poll
(`poll_edits`). There is no other I/O intrinsic: the intrinsics `host.*`
behind the root objects of a `System` (see "Capabilities") call these and
nothing else.

`StdHost` is the real process; its sockets are nonblocking, and a task
whose socket would block waits on a one-millisecond timer of the host's
clock and tries again, so the scheduler runs other tasks meanwhile (a
poll: functional, not fast). `TestHost` is deterministic: captured
output, a virtual clock, given arguments and variables, files in memory
over an optional read-only directory, a seeded stream of random bits, no
programs and no network. The conformance runner uses the real `sisu`
executable in an empty environment and a fresh directory instead, so
that it can test any implementation.

### The dynamic ownership monitor

`sisu run --monitor`, and every `sisu test` unless `--no-monitor` is
given, runs under the monitor (18, Phase 3: "use-after-move, aliasing
violations, drop-while-borrowed and a view escaping its frame are
internal soundness errors"; principle 5, "the checker checks itself"). It
checks, as the program runs, the rules the static checker proved, so a
program the checker accepted and the monitor stops is a soundness hole
in the checker.

**Loans at run time.** Every borrow statement makes a run-time loan of
the concrete place it borrows (a frame, a local and a path, so `xs[0]`
and `xs[1]` are different places), and every reference and every `str`
view carries the loan it was made by, its *tag*. A copy of a `ref mut`
is a reborrow: a new loan whose parent is the copied reference's loan,
so loans form a tree per place. Whether a copy is a reborrow is decided
by the type of the place it copies, as the checker decides it: a copy of
a shared `ref` passes its loan on, even when a `mut` borrow made it (a
`mut self` passed to a parameter that only reads; decision 185). Pieces
of a view (`split_whitespace`, `trim`, `lines`, `split_once`) carry the
view's tag.

**Accesses.** Every access goes through the reference it uses, and
checks first that the reference's loan and all its ancestors are still
active; using an ended loan is the violation. Then the access ends the
loans it conflicts with: a write (an assignment, a move, a drop, a `dead`,
the return of the frame, a `mut` borrow) ends every active loan of an
overlapping place other than the one it goes through and that loan's
ancestors; a read ends the active `mut` loans of an overlapping place
likewise. Ending a loan is not yet an error: the conflict is real only if
the reference is used afterwards, which is exactly when the static
checker's non-lexical rule calls the loan live, so the monitor and the
checker agree on what a conflict is. A `mut` borrow into an unnamed
temporary is two-phase, as in the checker: it survives reads until its
first use. The reborrows of a call's operands are made only after all
of the call's operands have been read, so `f(mut xs, ...)` behaves as
the checker's operand order says. A mutex's value changes behind shared
references to the mutex: an access through a guard ends no shared loan
of the mutex or of what holds it. A `Shared`'s value lives in a heap
cell, whose loans end with its last owner.

**Uses that read views.** An intrinsic reads what its arguments point
to, and so does an operator (`first == 1` through a reference), so a
reference passed to either is also checked for every view and reference
held by the value behind it (the first 256 values of it, a bound on the
monitor's cost per use): printing a `list[str]` whose `String` is gone is
caught. Passing a value to a function, or calling a closure, uses every
reference and view the value (or the closure's environment) holds, as
the static rule has it, so an ended loan is reported at the call.

**Other violations.** Reading, borrowing or writing into a place that
holds no value (moved out, or never assigned) is the use-after-move
violation; a reference into a frame that has returned, found by the
frame's serial number, is the dangling one; and a drop on a normal path
of a value holding a must-consume value, outside the methods of that
value's type, is the must-consume one (unwinding and drop glue are
exempt: the value's own drop was checked), as is `list.clear` on a list
holding one.

**The report.** A violation stops the program at once with exit status
134 and a report on standard error: what happened; for an ended loan,
the loan (kind, place, function, location and SIR statement that made
it), what ended it, and where it was used afterwards; the value of the
borrowed place and how many loans of it are not ended yet; the stack
(without drop glue frames); and the SIR of the function where the
violation happened.

```text
ownership violation: a reference was used after its loan had ended
  the loan: a shared borrow of `xs[0]` in `main`, created at push_while_borrowed.sisu:6:17 by `_2 = ref _1[_3]`
  ended by: a `mut` borrow of `xs` at push_while_borrowed.sisu:7:5 (`_4 = call intrinsic list.push(copy _5, const 4)`)
  used at: push_while_borrowed.sisu:8:5 (`_6 = call intrinsic print(copy _2)`)
  value: `xs[0]` = 1
  loans of `xs[0]` not ended yet: 0 shared, 2 with `mut`
  at main (push_while_borrowed.sisu:8:5)
SIR of `main`:
fn main(0) source 0 @4:5-4:9 {
...
```

**The cross-check.** The conformance runner runs every case that runs a
second time under the monitor and fails the suite on any difference
(`conformance/README.md`). `sisu-driver/tests/soundness.rs` holds two
more checks. *Fault injection*: with one of the checker's rules switched
off (`sisu_borrowck::Options::disabled`), a dozen programs that rule
exists for are accepted, and each must stop under the monitor with a
violation, which shows that the cross-check catches a weakened checker.
*Mutation fuzzing*: random reorderings of the statements of accepted
programs' SIR (neighbours swapped, or one statement moved within its
block) before drop placement; every mutant the checker accepts must run
under the monitor with no violation and no internal error. The test runs
a small fixed number from a fixed seed; `SISU_FUZZ_MUTANTS=600` raised it
to 27 000 mutants (8 054 accepted by the checker) without a violation.

**Gaps.** The monitor checks views held inside a value only up to its
bound, and only when the value is passed to an intrinsic by reference;
`set.add` dropping a duplicate must-consume value and the key of
`dict.insert` are not observed; `Path.as_str` gives a view without a
loan. A violation it misses is a hole it does not report, never a false
report.

### The counterexample mode

`sisu run --defer-borrow` (and `sisu test --defer-borrow`) runs a program
the ownership checker rejected, with its ownership errors deferred to
the monitor (18, N-26; 20, A2). It applies to the ownership family: the
checker's findings and the rules about declarations (`E04xx` from name
resolution). Purity and a second assignment to a `let`, which the monitor
does not observe, still stop the program; so does any other error. The
program's output is followed, on standard error, by one of three
outcomes:

- **conflict reached**: the monitor stopped at a concrete conflict, with
  its report;
- **no conflict**, with a line per rejected statement saying how many
  times this run reached it, and the verdict: every rejected statement
  was reached without a conflict, *a candidate false positive*; or some
  rejected statement was never reached, *inconclusive, an untested path*;
  a rule about declarations is listed as one no run can test (and a
  program rejected only by such rules is a candidate false positive).

One run proves nothing about other inputs, which the report says by its
wording; the mode is for the team (20, A2), and the `.defer` files of
`conformance/cases/ownership/` pin its output for every rejected case
that has a `main`.

### The checked memory model

Unsafe code (22; D-080 to D-086), which stage 1 checks and lowers, runs in
the interpreter on a checked memory model (D-085, decision 301), Miri's
design on the interpreter's values (`sisu-interp/src/rawmem.rs`). Every
intrinsic of std.mem's raw pointers, layouts, allocation and plain data
is the model's, and so is every foreign call (`extern.c.NAME`).

**Blocks and pointers.** Raw memory is a set of blocks, each with an
identity, an address, a size, an alignment and whether it is live: what
`Global.acquire` gave out and `Global.release` takes back, what the
emulated `malloc` gave out and `free` takes back, what a foreign function
handed out to be read (`getenv`'s text) and the value `view_plain` made of
bytes, both read-only, and the places of the program a pointer was made
from (`ptr.of`, `ptr.of_mut`, a slice's `as_ptr`), whose reads and writes
go to the place itself, through its frame or heap cell. A raw pointer
(`value::RawPtr`) is a block, an offset in bytes, whether it was made from a
shared borrow, and the monitor's loan it was made through; one with no
block (`ptr.dangling`, `ptr.from_address` of an address no foreign code
gave out) holds its address. Its type is the static type of the operand
that holds it, as native code has it: the interpreter reads the operands'
types for these intrinsics (`exec.rs`), and a raw pointer's value does not
carry one. Addresses are made up deterministically: blocks are laid out
from 1 MiB on, in the order made, aligned and with a gap between them, so
that two pointers to one place have one address and `addr` means the same
in every run; `from_address` gives a pointer the block of foreign memory
its address falls in (`exposed`), and none for any other address.

**What a block holds.** A block of `Global` holds values at offsets, each
with the type it was written as, and bytes of plain data where a write cut
a plain value apart. A read at the type a value was written as gives the
value, moved out when the type needs a drop (the slot then holds none, so
a second read is caught); a read at another type goes through the bytes,
which the model knows for the integers, the floats, `bool` and `char` (a
word each, as a value has them), and the structs, tuples and enums made of
them, and decodes them with the bits checked: a `bool` other than 0 or 1,
a `char` that is no scalar value and an enum's tag that names no variant
are refused. A write through a pointer made from a shared borrow, into
read-only memory, or over a value that needs a drop and was never dropped
(`writable`) is refused, and so is a release of memory that still holds
such a value (22's clause 3; not while a panic unwinds, when a half-done
operation may leave one). `copy_from` copies slot by slot, each holding
afterwards what its source held, a value or none, and a value that needs a
drop is the copy's alone; `drop_in_place` moves the value out and queues
its drop, which runs its glue on the run loop as any drop does;
`as_ref` and `as_mut` give a reference into the block (a pointer whose
frame is `value::RAW`, whose first step is the offset) or into the place,
which reads and writes as any reference does.

**The checks**, in this order for every access, each a panic that names
the obligation of the operation's `Safety:` list in std.mem
(`panic.raw-*`, `R0131` to `R0148`): memory behind the pointer
(`R0141`), live (`R0134` for a released block, `R0135` for a place whose
frame returned), the access inside its block (`R0131`, and `R0132` for
`offset`, `in_bounds`), aligned for its type (`R0133`), what it reads
holding a value of that type (`R0136`, `R0137`) with valid bits
(`R0138`), and a write neither through a pointer from a shared borrow
(`R0139`) nor over a value never dropped (`R0140`); `Global.release` of
memory it did not give out (`R0142`), released already (`R0143`), with
another layout (`R0144`) or holding a value never dropped (`R0145`).
`bit_cast` and `transmute` between types of different sizes are refused
when they run (`R0146`), since the front end does not compare the sizes
yet, and `read_plain` past the end of its data panics as std.mem says
(`R0147`).

**Loans.** Under the monitor (`sisu run --monitor`, `sisu test`), a pointer
made from a place carries the loan of the borrow that made it, so the
monitor tracks it while the pointer is used: every access through the
pointer goes through that loan, as an access through a reference does, and
an access to the place that conflicts with it ends it; a later use of the
pointer is then the violation, reported like an ownership violation. A
reference made from a pointer (`as_ref`, `as_mut`) is a loan under the
pointer's, and a block's references end when the block is released. The
monitor's rules are those it applies to references ("The dynamic
ownership monitor" above), which 22 asks for and Q-56 may refine: a read
of the place ends a pointer's `mut` loan, so a pointer made by `ptr.of_mut`
is used before its place is read directly again. A liveness check comes
before the monitor's, so a pointer to a place that is gone panics the same
way with the monitor and without it. Without the monitor the loans are not
kept, and an aliasing violation goes unseen, as natively; the conformance
cases that show one (`unsafe/stale_pointer`, `unsafe/reference_behind`)
have a `.monitor` expectation of their own (decision 303).

**The layout of a type in raw memory** (`mem.size_of`, `mem.align_of`) is
the native backend's (decision 302), so that a program computes the same
sizes either way: a sized integer narrower than 64 bits and an `f32` take
their own width and alignment, a struct and a tuple their parts in C's
order with their padding (SIR does not say which structs are `@repr(c)`,
and the order a struct declares is as good as any other), an option of a
raw pointer one word, null for `None`, and every other type the words of
its value (decision 188) aligned to a word: `bool` and `char` are a word, a
128-bit integer two, and an enum, a list or a `String` its value's words.
So `Layout.of` a `@derive(Plain)` struct of four `u8`s is 4 bytes aligned
to 1, and `bit_cast` of one to a `u32` keeps its bytes in the target's
order. The model takes a function value's size as one word of environment
more than its code's, where natively it is the program's widest closure
(decision 224).

**Foreign calls** (Q-55): the interpreter makes none, but emulates six
functions of the C library on the model, as 22's examples need them:
`strlen` (the bytes up to the NUL, each read checked), `memcpy` (refused
when its ranges overlap, `R0151`), `malloc` and `free` (blocks of their
own, which `Global.release` does not take), `getenv` (the host's variable,
as read-only foreign memory, or null) and `write` to the standard streams
(file descriptors 1 and 2, `R0152` otherwise). A pointer the declaration
types as an option is null for `None`, and a non-null one a foreign
function returns null for is refused (`R0150`). Any other foreign function
is refused (`R0149`), and `sisu run --native` calls it. **The machine
intrinsics** of std.rt (`rt.switch`, `rt.stack_pointer`, `rt.thread_slot`)
are refused too (`R0153`): the interpreter runs its tasks on a scheduler of
its own.

**Natively** (decisions 543 and 544), a program built in the debug
profile checks the same obligations, with the same panics, for the blocks
of `Global` (`sisu-rt/src/checked.rs`): a shadow of each block holds what
the interpreter's holds, a pointer belongs to the block its address falls
in, an address below 64 KiB has no memory behind it, and every other
address (a place, foreign memory) goes unchecked; a released block keeps
its address for the last 4 096 releases, a foreign call exposes every live
block's bytes, and what live blocks hold is let go at the end of the run.
The places of the program are seen by their addresses (decision 560): a
place `ptr.of` or `ptr.of_mut` made a pointer to is remembered with the
stack it is on and the borrows that made pointers to it, an access below
the caller's stack pointer is gone (`R0135`), and a write into a place
only shared borrows reached is refused (`R0139`); the memory of the C
functions the interpreter emulates is the model's too (decision 561).
What a pointer's provenance would add (a frame of the same depth over a
returned one, two pointers of different borrows to one place) and the
reads of a reference after a drop through the pointer are the
interpreter's alone; the release profile checks nothing.

What the model leaves out: overlapping accesses at different offsets are
not seen as overlapping by the monitor (each offset is a place of its
own); memory still held at the end of a run is not reported; a `Slice` is
a copy in stage 0 (port-debt.md), whose pointer from `as_ptr` views the
list or the `bytes` that `as_slice` made it from, where they are, so that
it is valid while they live (decision 823), as natively a slice of a list
or of `bytes` views it (decisions 940 and 1020);
and compile-time evaluation runs on the Rust front end's comptime, which
does not read std.mem.

## Tasks and the scheduler

Tasks run on the interpreter's own loop, one task at a time, under a
seeded cooperative scheduler (18, Phase 6: "the `single` profile of
[12](../../proposal/12-interop-and-targets.md), deliberately"). The code
is `sisu-interp/src/sched.rs`; the typing rules are in
`sisu-typeck/src/tasks.rs`, the ownership rules in `sisu-borrowck`.

**A task is a frame stack.** The running task's stack is the
interpreter's frame stack; every other task's is in its record, and a
switch swaps the two. A reference names the task whose stack holds its
frame, so a task reads the frames of the task that opened its scope,
which cannot return before the task ends. The root task runs `main` (or
a test).

**Anchors.** The scheduler pushes frames of its own, with no function and
one result slot. The anchor at the bottom of a spawned task receives what
the task's closure returns or the panic that unwound it. A scope's anchor
sits under the body of `task.scope`: when the body returns or panics, it
cancels as the rules below say, waits for every task of the scope, and
gives the scope's result (or lets the panic go on). A shield's anchor
marks the code under `task.shield`. The innermost scope or shield anchor
of the running stack is the cancellation context of the code that runs;
a task's context is its own node, under its scope.

**Scheduling.** The yield points are the back edges of loops (a jump to
a block at or before the current one, in the SIR with its jumps threaded:
"Speed"), calls of Sisu functions, spawns and `task.yield_now` (Q-34). Each task runs a budget of 1 to 8 yield
points drawn from the scheduler's generator (SplitMix64, seeded by
`--schedule-seed N`, 0 by default); when the budget is spent, the next
task is drawn from the ready ones by the same generator. A run is
therefore a function of the program, its input and the seed. `sisu run
--explore N` and `sisu test --explore N` run under the seeds 0 to N-1 and
report the first seed whose run differs (for `run`, in output or status
from seed 0's; for `test`, a test that fails), with the
`--schedule-seed` that reproduces it. `--stats` reports tasks, switches,
the most tasks alive at once, the scheduling latency (statements and
microseconds from ready to running, p50 and p99) and the `Shared` cells
by tag. `--stats stack-depth` reports instead the calls by the depth of
the frame they push, in buckets of powers of two, and the deepest stack
with an estimate of the native stack it would take (16 bytes per SIR
local and 64 per frame), the histogram 20's stack-exhaustion measure
asks for.

**Blocking.** An intrinsic that must wait (`join`, `recv`, `send` on a
full channel, `lock` of a mutex another task holds, a timer, `select`,
a scope's end, a host socket) returns `Unwind::Block` with its
arguments and what it waits for: keys (a task, a scope, a channel, the
lock) and possibly a timer. The statement records the call in the task,
the task waits, and when an event wakes a key it is ready again; its
call is then retried with the same arguments, so it completes exactly
once. When no task is ready the virtual clocks advance together to the
earliest of their timers; the host's real clock is waited for; when
there is no timer either, the program is deadlocked (`R0080`), and the
report names each task, what it waits for and where (`R0081`).

**Cancellation** (06, "Cancellation") is a flag on scopes and tasks.
`s.cancel()` sets its scope's; a scope is cancelled when an ancestor is;
a task when its scope is, or its deadline (`spawn_with(deadline=timer)`)
has passed on the timer's clock. Every blocking operation but `lock`
checks the cancellation of its code's context first and returns
`Err(Cancelled)` when it is cancelled; a shield's context is never
cancelled. Cancelling wakes every waiting task, which retries and sees
it. `task.checkpoint()` is a blocking point that waits for nothing, and
`task.is_cancelled()` asks.

**Scopes.** `task.scope do s:` runs its body on the calling task above
the scope's anchor; `s.spawn(closure)` starts a task on the scope and
returns its `Handle[T]`. At the end of the body the anchor waits for
every task. The result rules:

- A task fails when it panics, or when it returns `Err` and was not
  cancelled into it (an `Err(Cancelled)` of a cancelled task is not a
  failure). In an effectful scope the first failure in time cancels the
  task's siblings. In a scope opened by `pure` code (the scope is marked
  `pure` by its call), a failure cancels only the siblings spawned after
  the failing task, so those before it run to completion, and the
  failure reported is the lowest-index one (06, "Effects").
- A body that panics, or returns `Err`, cancels the tasks still running;
  a panic is the scope's outcome once every task has ended, an `Err` its
  result.
- Otherwise a failure nobody joined becomes the scope's result: a panic
  as `Err(TaskPanicked(message, trace))` when the result type carries
  `TaskError`, an error when the result type carries its set. When the
  result type cannot carry it, the panic goes on, or the error becomes a
  panic (`R0079`): a failure is never lost.
- Results nobody joined are dropped when the scope ends.

The checker types a scope whose body returns `T ! E` as `T ! {E,
TaskError}` widened by the error sets of the tasks spawned on it, and
`h.join()` as `U ! {E, TaskError}` for a task returning `U ! E` (the sets
flattened) and `T ! TaskError` otherwise; joining twice panics
(`R0077`). A body cannot return its scope or a handle (`E0430`).
`task.race do s:` is a scope whose first task to finish wins: its result
is the race's, the others are cancelled (`E0431` asks the tasks for one
type; a race with no task panics, `R0078`). `task.shield do:` runs its
block under a context that is never cancelled. The closures given to
`scope`, `race`, `shield` and `spawn` must be written at the call
(`E0425`), so the checker sees what they capture.

**Channels.** `channel[T](capacity)` gives a `Sender[T]` and a
`Receiver[T]` over a bounded queue of at least one value. `send` waits
while it is full and `recv` while it is empty; closing (`Sender.close`,
or dropping every sender) lets the receiver drain what was sent and then
get `Closed`, and dropping the receiver makes `send` give `Closed` (the
value dropped). `.copy()` of a sender makes another. A `for` loop over a
receiver receives until the channel is closed and drained, or its task is
cancelled. What a channel carries is neither a view nor must-consume.

**`select`** waits for the first ready arm among `recv`, `send`, `join`
calls, `Timer`s and `Cancellation`s (`E0434` for anything else); among
several ready arms the seeded generator chooses. When the code running
is cancelled, a ready cancellation arm is taken, else the first arm whose
operation reports `Cancelled`, which then gives `Err(Cancelled)`.

**Time.** A `Timer` is a moment on a clock (`clock.after(d)`), which
`Timer.wait` and `select` wait for. A run has the host's clock (real
under `StdHost`, virtual under `TestHost`) and the test clock of
`Clock.fake` (virtual, made on first use). Virtual clocks move only when
told to (`TestClock.advance`, `TestClock.set`) or when every task waits:
then all of them advance by the smallest wait among their timers (17,
"Isolation and parallelism", Go's `synctest`). `clock.sleep(d)` is
`clock.after(d).wait()`, written in Sisu.

**Generators.** A function whose body yields returns an `Iter` whose
items come from a task-like closure run on demand: `next` switches to it
until its next `yield`, which hands the item over and switches back. A
generator runs only when its consumer asks, never because a scope's
cancellation woke the tasks that wait. Code that cannot wait for it
(another iterator's adapter, `zip` or `enumerate` over it; an intrinsic
given it as an `Iterable[T]`, `sum` or `sorted`; a generator being
driven so, whose own `next` on another generator) drives it instead:
the generator runs there and then, outside the schedule, to its next
`yield` or its end, with no yield point switching and the budget left as
it was, so a seed's schedule is the same in both implementations; an
operation in it that would wait aborts the program (`panic.driven-wait`,
`R0130`), since nothing could run the code that asked (decision 247). One dropped before its end is
closed, as Python's `close` closes one (decision 236): suspended at a
`yield`, it unwinds from there at once, outside the schedule, running its
frames' drops and `defer`s before the consumer goes on; one that never
started drops what its closure captured. A panic while it cleans up
aborts the program (`panic.during-abandon`), as a panic during unwinding
does, and so does an operation that would wait there
(`panic.abandoned-wait`). A receiver's iterator inside another
iterator's adapter cannot wait (`R0091`).

**`log`** writes records to the standard error with their level and,
for a task, the task's path (`R0085`), which stands in for the trace
context D-075 gives a scope's tasks.

**Ambient.** `task` and `log` are modules every module sees without an
import (Q-6, Q-5), as is `cap` (std.cap, which grants nothing by itself).

## Send, Sync and what a task captures

`Send` and `Sync` are inferred structurally, as Rust infers them
(`sisu-resolve/src/auto.rs`), and checked where a value crosses a
`spawn`, where a value becomes a capability handle, and where a bound
asks for them. The leaves (numbers, text, paths, times, bytes, the
capability handles, scopes) are both. `ref T` is both when `T` is
`Sync`; `ref mut T` is `Send` when `T` is and `Sync` when `T` is.
`Cell[T]` is `Send` when `T` is and never `Sync`; `Mutex[T]`,
`Atomic[T]`, a channel half and a task handle are both when `T` is
`Send`; `RwLock[T]` is `Send` when `T` is and `Sync` when `T` is both,
since its readers share the value; a mutex guard and a read guard are
never `Send` and `Sync` when `T` is; `Shared[T]`, `Weak[T]`, `Cap[T]`,
`Lent[T]` and `Slice[T]` are both when `T` is `Sync`. A function or
closure value is neither (its captures are not in its type); a `dyn
Trait` is what the trait's supertraits require; a type parameter what
its bounds say; everything else what its parts are, coinductively for a
recursive type. A failure names the field in the way and the wrapper
that would help (`E0426`, `E0427`, `E0437`, `E0433` for a task's result;
`E0700` for a capability's object).

**What a task captures.** Once the types are known, a task's `Copy`
captures that it does not change are copied into it, and the rest are
borrowed; an `own` task moves its captures in. A copied or moved capture
must be `Send`, a borrowed one `Sync`, one changed through a reference
`Send`.

**What a task may borrow** (the ownership checker). A task spawned on
the scope of the body it is spawned in (the body's parameter) may borrow
what the body's environment borrows, which outlives the scope: a loan
the environment brought, or one reached through it; not a local of the
body (`E0428`), since the body ends before the scope joins. A task
spawned anywhere else (on a scope passed in, in a helper) borrows
nothing (`E0429`), since the checker cannot see how long its scope
lasts; it may still copy and move. The loans a task holds are pinned
until its scope's body returns. A generator's `yield` gives its item to
the consumer, so the item may borrow only what the generator's
environment does.

**`Shared` atomicity** (Q-2; `sisu-borrowck/src/shared.rs`). After the
ownership checker, an escape analysis tags each construction site of a
`Shared` task-local or may-cross. A program that spawns no task, sends on
no channel and makes no generator has only task-local sites. Otherwise a
site may cross when a local that may hold its value (or a copy, a
reference to it, or something holding one, followed flow-insensitively
through assignments, borrows, aggregates, `.copy()`, closures and call
results) reaches a spawn, a send, a generator, a scope's body, a
capability handle or a `Cap`, a store through a reference, the
function's result, or a call of a function that may itself let what it
is given cross (one that, directly or through the functions it calls,
does one of those or calls through a function value or a trait). A
may-cross site is written `call intrinsic Shared.new_atomic`. The
monitor checks the tags: a cell from a task-local site reached by
another task is a soundness error of the analysis (`R0089`).
`--stats shared` lists every construction site of the program's own code
(the standard library's left out), with its tag, the cells it made and
those another task reached, after the share of sites tagged task-local,
Q-2's number; a site that never ran is listed with no cells.

## Capabilities

A capability is a `Copy` handle (`Fs`, `Net`, `Clock`, `Rand`, `Env`,
`Proc`) on an object that implements the capability's operations trait
(`FsOps`, `NetOps`, `ClockOps`, `RandOps`, `EnvOps`, `ProcOps`; 06, "The
rule"). The object lives in a heap cell that every copy of the handle
reaches; copying the handle grants nothing new, and a fake's state,
behind the handle, is seen by every copy. The cell counts the copies, and
the last one's drop drops the object and empties the cell (see
"Counting copies" below). The handle
implements the trait by forwarding: its impl's methods are the intrinsic
`cap.forward`, which lowering turns into a call through the trait whose
`Self` the dispatcher replaces by the object in the cell, so the object's
own method (Sisu or intrinsic) runs with `self` pointing into the cell.

**Becoming a handle.** Where a handle type is expected, a value of
another type that implements its operations trait becomes one
(`Coercion::ToCap`, lowered to `call intrinsic cap.new[T, Handle]`, which
puts the value in a new cell). The value must own what it holds (a view
is `E0421`), must not be must-consume, since the handle drops it unseen,
with its last copy (`E0422`), and must be `Send` and `Sync`, since a
handle may reach every task (`E0700`). A result reached through a
handle, a `Cap` or a `Lent` (`c.get()`, `env.args()`, `fs.lend()`)
borrows the place the call copied the handle from, since the object goes
with the handle's last copy: a parameter holding a handle counts as
borrowed for the elision rules, and the ownership checker gives the
result a loan of that place (decision 237).

**Counting copies.** A handle is `Copy` (Q-51), and each copy is counted
(decision 237). Lowering drops a value whose type holds a handle or a
`Cap` (`holds_capability`; a `Lent` holds none) as it drops one that is
not `Copy`, and a value whose type is `Copy` only by a bound, whose type
parameter may stand for a handle; a temporary of such a type moves into
what takes it, an argument a callee borrows is a copy of a place (a
temporary's dropped with the statement's others), and a comparison or an
interpolation borrows it. The interpreter adds a count to the cell for
each copy made into a place, an aggregate, a closure's environment, a
function's argument or an argument an intrinsic keeps, and each drop
takes one; the last drop drops the object, running its `Drop`, and
empties the cell, after which a reference into it dangles
(`panic.dangling`), which the checker's loans keep a program from
reaching. A `Lent` counts nothing. A function value is dropped like a
value that holds a handle, since an `own` closure's environment holds
copies of what it captured (decision 248): a copy of the value counts
the handles its environment holds, and its drop lets them go, so a handle
an `own` closure captures goes with the last copy of the function value.
`capabilities/object_drops`, `capabilities/closure_drops` and
`ownership/handle_borrows` pin the three.

**The root objects** are the host's: `HostFs`, `HostNet`, `HostClock`,
`HostRand`, `HostEnv` and `HostProc`, builtin types that no code can
construct, whose operations are the intrinsics `host.*` over the host
contract. The runtime builds a `System` of handles on them for a
`main(sys: System)`. `System.assemble`, which makes one from handles, is
`@internal`: only std may call it (`E0701`).

**Attenuations and the forms of 06** are written in Sisu, in std.cap
(`std/cap.sisu`, the ambient module `cap`), over the operations traits:

- `fs.read_only()` refuses every change as `PermissionDenied`;
  `fs.restrict_to(dir)` puts paths below `dir` and refuses an absolute
  path or one with a `..` part, reporting errors with the caller's path;
  `net.only_hosts(hosts)` and `net.https_only()` (port 443: stage 0 has
  no TLS) refuse other addresses as `NotAllowed`. They compose, and a
  narrowed capability cannot be widened again.
- `fs.lend()` is a `Lent[Fs]`, a view of the handle at no cost: it
  implements the trait (forwarding as the handle does), so a generic
  parameter `[F: FsOps]` accepts it with every other form, but it never
  becomes a handle, which could be stored.
- `fs.revocable()` and `net.revocable()` give a caretaker and its
  `Revoker`, a `Copy` handle on a flag every copy of the caretaker
  checks; after `revoke()`, every operation fails with `Revoked` (and
  nothing exists).
- `cap.Wrap(inner=c, hook=i)` shows every call through the capability
  `c`'s trait to the interceptor `i` (a `cap.Interceptor`) as a
  `cap.Call` before it runs, and its `Debug` outcome after. Its impls
  are written for each standard capability; a `Wrap` of any trait waits
  for reflection (Phase 8).
- `Cap[T]` is the building block of objects written in Sisu: a `Copy`
  handle on a value every copy shares, never dropped, `Sync` when `T`
  is; what it holds can be neither a view nor must-consume, and changing
  it goes through a `Mutex` inside.

**The fakes** (17, "Mocks"), also in Sisu: `Fs.in_memory(files)` (files
and directories in mutexes, writes stamped by the test clock),
`Clock.fake()` and `Clock.frozen_at(t)` (the run's test clock, a
`TestClock` with `advance` and `set`, which becomes a `Clock`),
`Rand.seeded(seed)` (SplitMix64 in Sisu), `Env.fake(args, vars)`,
`Proc.fake(outputs)`, `Net.fake()` (an in-process network: listeners by
address, connections as pairs of bounded channels of bytes, counted),
and `http.FakeNet()` (scripted HTTP answers and a request count).
`System.fake(seed)` assembles a set of them: an empty in-memory `Fs`, a
`Net` that reaches nothing, the test clock, `Rand.seeded(seed)`, and an
empty `Env` and `Proc`. Each test gets fresh fakes by making them (17,
"Isolation and parallelism"; 10, "each test gets fresh capability fakes
(`Fs.in_memory()`, `Clock.frozen_at(...)`)"): every test runs in an
interpreter of its own, so its test clock, its fakes and the module
statics start new. A test that declares `sys: System` receives the real
one, as `main` does (02, "a test that needs a real capability runs
serially"); stage 0 runs tests one after another, so it runs alone, and
it is the one kind of test a seed does not reproduce. Any other test
parameter is `E0702`. The ambient standard streams and `log` stay
ambient (Q-5).

**The network.** A `Conn` is a `Box[dyn Stream]` and the peer's address;
a `Listener` a `Box[dyn Accept]` and its port. A read waits for data and
gives at most what it is asked for, nothing at the end of the stream; a
write sends everything. The host's streams and listeners are builtin
types over the host's nonblocking sockets; the fake network's are Sisu
structs over channels, so every network wait is a blocking point that
cancellation interrupts.

**`std.x.http`** (`std/http.sisu`) is HTTP/1.1 in Sisu over `Net`:
`http.request(net, method, url, body)` and `http.get(net, url)` (a
non-2xx status is `Status(code)`); `http.serve(listener, handler)` runs
until cancelled, answering each connection on a task of its own with a
`Handler`'s response, and drops a connection that fails. One request per
connection, bodies by `Content-Length` or to the end of the stream.
`HttpError` is in the prelude, because the proposal's examples name it
after `import http`. `std.net` has `Url`.

**Purity.** A `pure` function cannot take a capability, a `TestClock`, a
lent capability or a `System` (`E0334`); the capabilities' operations
are not pure, so it could not use one either.

## Contracts and refinements

[07](../../proposal/07-contracts.md). A function's contract is its
`requires` and `ensures` clauses, at the start of its body (`E0800`
elsewhere, `E0807` in a test); a struct's is its `invariant` lines; a
refinement type is `type X = T where pred`, with `self` the base value.

**Checking** (`sisu-typeck/src/contracts.rs`). A clause is a `bool`
expression that only reads: it calls `pure` functions (`E0801`) and
changes nothing (`E0802`), so a build may leave it out and a failed one
can be evaluated again, part by part. `result` in an `ensures` is the
returned value; `old(e)` is `e` as it was on entry (`E0803` outside an
`ensures`, `E0804` for a malformed one; a generator has no `ensures`,
`E0805`). A field of a struct with an invariant may be changed only by
the struct's own methods (`E0806`), which check it again.

**Where checks run** (`sisu-lower/src/contracts.rs`). `requires` on entry;
the values `old` names saved then; on every exit (`return`, `?`, the end
of the body) the `ensures` clauses with `result` bound, and the invariant
of a `mut self` method's type; a struct's invariant once it is built
(the method the compiler writes for its lines, `X.__check_invariant`, and
`X.__invariant`, its `bool` form, for D-077's `Mutex` check). The level of
a build decides which run: `sisu run --contracts=off|preconditions|all`
(`all` by default; `D0028`); `sisu test` checks every one (`D0029`). A
failed check panics with the clause and, for each part of a clause that
only reads, its value (`R0100` precondition, `R0101` postcondition,
`R0102` invariant), as a failed `assert` shows every sub-expression
(`assert.fail_parts`).

**Refinements.** `X` is a subtype of its base: an `X` is used wherever the
base is (it has the base's reading impls, through `unrefine`; an impl of
another trait of its own is `E0813`), and a base value becomes an `X`
only where its predicate is checked: `X.try_from(v)`, which gives
`Err(Unsatisfied(...))` of `RefinementError`, or a constant, checked at
compile time (`E0810`). A base where an `X` is expected is `E0811`, with
the fix. A refinement of a refinement keeps its chain: a value of the
outer one satisfies every predicate down to the root, and `try_from`
checks them all. A change through the base (`x.push(...)` on a refined
list) is `E0812`: change a copy and check it again. The predicate is a
`pure` function the compiler writes (`X#where`, `E0814` unless it is a
`bool`); a run-time refinement check that fails is `R0103`.

**Documented automatically** (07): `sisu doc` (section "Documentation")
shows each signature with its contracts under it, a type's invariants, a
refinement's predicate and the `try_from` that checks it, and the error
set the compiler inferred for a `T !` function (`... ! {A, B}  #
inferred`); `contracts/explain` pins it.

## Property tests

[17](../../proposal/17-testing.md), "Property-based testing"; D-070.
`check(gen, prop)` (or `check(gen) do x:`) runs a property on the cases a
generator makes, in Sisu (`std/prelude.sisu`), on Hypothesis's model: a
case is made from a sequence of choices (`Draws`), so every generator
shrinks the same way. `arbitrary[T]` draws 100 random cases;
`exhaustive[T](max_len=n)` lists every case whose collections hold at
most `n` items and whose integers are within `n` of zero (or in their
refinement's range), depth first. A failing case is shrunk by deleting
and lowering choices (each lowered by binary search; at most 1000
attempts) and reported with its value; each case has a budget of
statements (`Gen.steps`; past it is a failure, `R0104`, unless the
generator is quiet), so a property that loops forever fails instead of
hanging, deterministically. Natively the budget counts the yield points
the case passes, function entries and back edges, in the counter the
seeded scheduler's budget shares (decision 540).

**Generators** are written by the compiler as Sisu source
(`sisu-resolve/src/arbitrary.rs`, `<arbitrary>/main.sisu`), only for the
types a program generates: `__arbitrary_T` for a struct (its fields, then
its invariant: a case that breaks it is refused), an enum (a variant,
then its fields) and a refinement (its base, then its predicate; an
integer refinement whose predicate bounds `self` draws in that range);
`E0820` for a type with none. The `<arbitrary>` module's diagnostics are
dropped when the program has an error of its own, since they repeat it.

**Contracts as properties** (07): for each function with a contract
whose parameters all have generators, `sisu test` adds `contract of
NAME`, which calls it on cases its `requires` accepts; `ensures` and the
invariants are checked by the call. Such a generator is quiet: a case it
cannot make, or one past its budget, is refused rather than failed.

**`@refines(reference, tolerance=Ulps(n))`** (N-21) on a function adds
`NAME refines REFERENCE`: on quiet `arbitrary` cases of the parameters,
the function gives what its reference gives, exactly, or for an `f64`
within `n` units in the last place (`within_ulps`). Both must be free
functions of the module with the same parameters and result (`E0821`
otherwise, with the reason).

## Testing

`sisu test FILE` (17) runs the `test` blocks, under the monitor unless
`--no-monitor`. A test takes nothing, the real `System` (`sys: System`),
or the capabilities it names, one of each (`fs: Fs, clock: Clock`), and
receives fresh fakes of those: an empty in-memory `Fs`, a network of its
own, the run's test clock as a `Clock` or as the `TestClock` that moves
it, `Rand.seeded` with the test's seed, and `Env`, `Proc` and `Stdin`
fakes (`System.fake`'s, and under `--sim` `System.sim`'s; decision 241).
Each test runs in an interpreter of its own; those that do not take the
real `System` run at the same time on up to `--jobs N` threads (8 by
default, `sisu-interp/src/parallel.rs`), and those that take it one at a
time after them, under `--sim` too, which gives them the real `System`
(decision 822). Each is seeded as when they ran one after another,
nothing but the host's clock and output is shared, and each test's
output is held and written in declaration order, so a run prints the
same at any number of threads (the conformance runner checks `--jobs 1`
against every `.test`). A test that passed is not run again while its
code is the same (`PROGRAM.test-cache` beside the program, or
`build/debug/test-cache` in a package; `--no-cache` runs everything;
decision 242): it prints what it printed when it ran. The run exits with 0 when
every test passed, 1 when one failed, and 3 when none failed and one is
blocked by a typed hole (decisions 340 and 380). `sisu test
src/MODULE.sisu` of a module of a package runs the tests that module
declares, the package compiled as `sisu test` compiles it, which stage 1's
`sir --tests --modules` tells (decision 900).

- **Parametrised tests**: `test "name" for x, y in [(1, 2), (3, 4)]:` over
  a list written out is a test per element, named `name (1, 2)`, whose
  body starts by binding the pattern; over a list computed otherwise, one
  test that runs each case in turn (`sisu-resolve/src/tests_of.rs`). Each
  case is checked, lowered and borrow-checked as a test of its own, and a
  mistake in the body they share is reported once.
- **Suites and tags**: a test in `suite "s":` is named `s/name`;
  `@tag("slow")` marks it. `--filter TEXT` (a part of the name),
  `--tag T` and `--skip-tag T` select (`D0053` counts the others).
- **Snapshots**: `std.test.assert_snapshot(value)` compares the value's
  `Debug` rendering (text as it is) with the snapshot stored under the
  test's name (` #2` and on for later ones) in `PROGRAM.snap` beside the
  program (`sisu-driver/src/snap.rs`): a change fails with a line diff
  (`R0107`), a missing one shows what would be stored (`R0108`); `sisu
  test --update` shows each change and writes the file.
- **Transcripts**: every `tests/*.transcript` beside the program
  (`sisu-driver/src/transcript.rs`): a `$` line runs `main` with the
  words after the program's name, the `>` lines after it are its
  standard input, the other lines what it prints (output and errors as
  written but a failure's trace, whose lines move with the program's,
  decision 428; ` (exit N)` on the last line of a run that fails), against a
  fake `System` that persists between the runs of one file: an in-memory
  `Fs` with the files of `tests/NAME/`, a clock frozen at the epoch,
  `Rand.seeded(0)` and no network (`transcript_world` and
  `transcript_system` in the prelude, which stage 1's `sir --tests` of
  0.11.0 and later keeps for a program that has transcripts, so the
  transcripts run on the SIR the tests run, decision 722; for an older
  stage 1 the Rust front end's lowering keeps them for this run only, so
  the transcripts run on its SIR of the program, decision 380). A difference
  fails with a diff (`D0061`); `--update` rewrites the file.
- **Benchmarks**: `bench "name"(b: Bench):` is checked like a test and
  run by `sisu bench FILE [--iterations N] [--samples K]`, which times
  `K` runs of `b.loop()` (10) after one it does not time and reports the
  median time of an iteration, with the 95% interval of the mean of the
  samples that are not outliers (decision 468; the interpreter's times:
  compare benchmarks with each other); `black_box(x)` consumes a value
  the optimiser must not see through.
- **Fakes and interceptors** of the program's traits: `cap.Fake[Trait]`
  and `cap.Wrap`, above ("The interceptors and fakes of a program's
  traits").
- **Replay tokens**: every failing test prints `replay: sisu test
  --replay TOKEN FILE` (below), or says it has none when it takes the
  real `System` (`D0031`, and under `--sim` `D0138`, decision 822).

## Simulation, schedulers and replay

**The capability log** (N-04; `sisu-interp/src/record.rs`). `sisu run
--record FILE` runs a program on a host wrapper that passes every call of
the host contract through and writes it and its result to a versioned
text log: a header (`sisu-log 1`, the toolchain, the program's name and
content hash, its source, the scheduler's seed, the hash key, the
contract level), then one record per call (`call NAME FIELDS -> RESULT`;
written data by its hash), `out`, `err`, `redacted CAP` (for `--redact
fs,net,...`), `edit POLL FILE TEXT` (a save a run under `--watch` saw,
section "Hot replacement") and `exit STATUS`; a program of several
modules adds each file's text and its layout to the header, so it too
replays without its files.
`sisu replay FILE` runs the logged source again on a host that answers
each call from the log; the first call or output that differs is a
divergence (`D0037`), reported with both, and a different exit status is
`D0040`. In the interpreter every source of nondeterminism is a host call
or the seeded scheduler, so a replay is exact; the conformance runner
records and replays every case that runs.

**Replay tokens** (`sisu-driver/src/replay.rs`) are one word,
`1-HASH-TOOLCHAIN-tT-sS-rR[-cCHOICES][-pDEPTH|-xSCHEDULE][-mPPM]`: the
content hash of the file (of a package's files, and of the files the
program's `embed` calls read, decision 421), the toolchain, the test's index, the scheduler's
seed, the case seed, the shrunk case's choices, the scheduler and the
`sim` profile's fault rate. `sisu test --replay TOKEN FILE` runs that test
that way; a token for other code (`D0033`), another toolchain (`D0034`)
or a missing test (`D0035`) is refused.

**Schedulers** (`sisu-interp/src/sched.rs`, `--scheduler`). `random` (the
default): budgets of yield points and the next task drawn from the seeded
generator. `pct --depth N`: PCT, random priorities with N-1 change points
over the first 1000 steps, the highest-priority ready task runs, a task
that yields lowers its priority. `exhaustive --depth N`: depth-first
enumeration of every schedule with at most N preemptions, each schedule a
list of choices (`next_schedule`), capped at 10 000 runs or `--explore N`
(`D0046` to `D0050`). `--explore N` runs a program or each test under N
seeds of the random or PCT scheduler.

**The `sim` profile** (17, "Simulation and replay"; `sisu test --sim
[--faults P]`). A test that names the capabilities it uses gets those of
`System.sim(seed)`, and one that declares a `System` the real one, which
is neither simulated nor replayed (17; decision 822). `System.sim` holds a
crash-consistent file system (`cap.SimFs`: writes are pending until
`sync`, a crash keeps what was synced, a rename is durable once the new
name's directory is synced, decision 420), an in-process network
(`cap.SimNetwork`), the run's test clock, `Rand.seeded`, and fakes of the
rest. `std.sim` has `Sim` (nodes, each with its own `SimFs`, on one
network: `crash`, `restart`, `partition`, `heal`), `choose(n)` (a choice
the exhaustive scheduler enumerates) and `always(what, holds, body)` (an
invariant checked at every switch between tasks, `R0105`). With
`--faults P` (0.01 by default) a call of a simulation object's operations
fails at that rate with an error from its declared set, never one it
cannot return, and never `Revoked`, which only a caretaker returns
(decision 426). A `sim` build fails to compile if reachable code calls an
`extern` with no simulation model (`E0971`; `sisu-driver/src/sim.rs`).

## Numbers, bytes and slices

**The sized integers** `u8`, `u16`, `u32`, `u64`, `u128`, `i8`, `i16`,
`i32` and `i128` are builtin types of the prelude (`i64` is an alias of
`int`, `uint` of `u64`). A literal takes the sized type expected, or its
suffix's (`0u32`), and must fit (`E0374`). Two operands of one sized type
take the operators `int` takes, with the same result type (`/` is
`E0301`, as for `int`); operands of two integer types widen the narrower
when that loses nothing; otherwise the types mix (`E0373`). Where an
integer type is expected, a narrower one that it holds widens
(`Coercion::IntWiden`: `u8` to `u32`, `i32` or `int`; `int` and `u64` to
`i128`; `u64` to `u128`; a signed type never to an unsigned one); every
other conversion is a method: `x.to_int()` (`None` for a `u64`, `u128` or
`i128` outside `int`'s range), `n.to_u8()` (`None` when it does not fit),
`n.truncate_to[T]()` (two's complement wrapping, through the `Integer`
trait), and `x.into()` for each lossless widening (`Into[T]`). Lowering
calls the typed intrinsics (`u8.add`, `u32.wrap_mul`, `i128.floordiv`,
...), which trap, wrap or saturate at the type's bounds; `//` floors and
`%` takes the divisor's sign, as for `int`, the least `i128` over -1
overflows, and a shift by the width or more panics (`R0093`). The
operator traits (`Add` ... `Neg`) and `FromLiteral[int]` are implemented
for generic code. A `u64`, `u128` or `i128` is a value of its own kind
(`sisu_resolve::std_sources::own_kind`), made by an intrinsic: a `u64`
literal from its 64 bits (`u64.from_bits`), a 128-bit one from the `int`
it equals (`i128.widen`) or, beyond `int`, from its decimal text
(`u128.from_text`, as a byte string travels as hex), and its literal
patterns compare against such a value; the other sized integers are
`int` values in range. A format spec applies to them as to `int`: the
host gives the digits of any integer (`fmt.wide_digits`).

**`f32`** is IEEE 754 binary32, a builtin type of the prelude and a value
of its own kind (`Value::F32`), strict as `f64` is (D-074). A float
literal takes `f32` where one is expected or by its suffix (`1.5f32`),
and an integer literal where one is expected; the checker rounds the
literal's text once to `f32` (never through an `f64`), and SIR carries it
as the `f64` that holds it exactly, made an `f32` by `f32.from_literal`. A
literal whose magnitude its type cannot hold is `E0374`, for `f64` too; a
float literal with an integer suffix is `E0332`. Two `f32`s take `+ - * /
% **` and unary `-` (`//` is `E0372`, as for `f64`), with an `f32`
result, as SIR's own operators on `f32` values: the interpreter computes
in `f64` and rounds once to `f32`, which for `+`, `-`, `*`, `/` and the
square root is exactly `f32` arithmetic (`f64` has more than twice the
bits), and defines `%` and `**` as `f64`'s on the two values, rounded
once. Nothing is computed in a wider format and nothing fuses: `a *
b + c` rounds twice, and `a.mul_add(b, c)` (on `f64` too) is the host's
fused multiply-add, one rounding. An `f32` widens to `f64` where one is
expected and beside an `f64` operand (`Coercion::FloatWiden`, the
intrinsic `f32.to_f64`), as 03 and the tour say ("`f32` to `f64` happen
silently"); a comparison does not widen, as for the integers; the other
conversions are methods: `x.to_f32()` of an `f64` or an `int` (to the
nearest, an infinity beyond the range), `to_f64`, `truncate_to_int`,
`to_bits` and `f32.from_bits` (a `u32`). Like `f64` it is `Display`,
`Debug`, `Eq` and `PartialOrd`, not `Ord` or `Hash`; it displays as the
shortest text that reads back as the same `f32` (`0.1`, not the `f64`
digits of the value), and a format spec applies to it as to an `f64`
(`Spec.float32`, `fmt.float32_digits`); `s.parse_f32()` rounds the text
once. `sum` adds `f32`s pairwise in `f32`, and property tests draw them
(`Draws.gen_f32`).

**Text is bytes** (03, "Strings and text"). A `String` owns the bytes of
a valid UTF-8 encoding and a `str` views them; both are host values
(`Value::String`, `Value::Str`), and the host's operations on them are
primitives over the encoding: `byte_len`, `byte_at(i)` (a `u8`), `bytes()`,
`byte_slice(start, end)` (a view, at boundaries of characters, `R0115`
otherwise, and in bounds), `chars()` and `len` (UTF-8 decoding), `cmp`
(code points, which is the order of the bytes), `String.push` and
`push_char` (which append a valid encoding, so the bytes stay valid),
`to_string`, `encode` and `bytes.decode` (which validates). Every other
text method is Sisu over those (`std/prelude.sisu`, "str and String"):
`byte_find` (Crochemore and Perrin's Two-Way search, as Rust's
`str::find`: linear, in constant space), `find` (the index in
characters), `contains`, `starts_with`,
`ends_with`, `strip_prefix`, `strip_suffix`, `split`, `split_once`,
`lines` (`\n` or `\r\n`, no empty last line), `split_whitespace`, `trim`,
`trim_start`, `trim_end`, `replace`, `repeat`, `is_char_boundary`; their
pieces are views of the text, collected into an `Iter`. Of Unicode's
tables only the character classes (`char.is_whitespace` and the others)
and the case mappings (`to_lowercase`, `to_uppercase`, which need
`SpecialCasing`'s context for a final sigma) are the host's.

**`bytes`** is an immutable owned byte sequence (03): the type of
`b"..."` (with `\xNN` for any byte), of a file's contents and of what a
connection reads. `b[i]` is a `u8`, `b[i:j]` a copy, `a + b`
concatenates, `for x in b` iterates; `s.encode()` gives the UTF-8 bytes
of a text, and `b.decode()` the text or `InvalidUtf8(at)`.

**`Slice[T]`** is a read-only view of a run of values: a `list[T]` is
one, and `bytes` a `Slice[u8]`, where one is expected. `s[i]` is a
reference to a value and `s[i:j]` a view of a run. A `Slice` is not
`Copy` in stage 0 (a `str` is), so it is passed by reference as a `list`
is.

**Operators with other result types.** The operator traits have an
associated `Output = Self`, so an impl can bind another
(`DateTime - DateTime` is a `Duration`). A bound fixes a defaulted
associated type to its default (`T: Sub` means `T - T` is a `T`), so
generic code keeps the meaning it had, while a call of a trait's method
on a concrete type asks for the impl only.

## Formatting

`Display` is text for a program's users and `Debug` text for programmers
(03, "Strings"); both write to a `Formatter` (`std/prelude.sisu`).

**Forms.** `Debug` renders a value the way it is written: `"text"`,
`'c'`, `[1, 2]`, `(1,)`, `{"k": 1}`, `set()`, `Point(x=1.0, y=2.0)`,
`BadDigit(at=1, found='x')`, `Some(3)`, `None`; a positional variant
field has no label. `Display` is the plain text of strings, characters
and paths, decimal for integers, the shortest text that reads back as the
same value for floats (`3.0`, `0.1`, `1e100`, `inf`, `NaN`),
`True`/`False`, ISO 8601 for a `DateTime`, a type's own impl when it has
one, and the `Debug` form for collections, options and results. These
are what the structural impls and the derived ones compute, and what the
host's structural text gives.

**The protocol.** A `Formatter` holds the text being built, the format
spec of the value being rendered and a depth. A value's impl writes its
parts at a greater depth (`f.depth += 1`), where the spec does not apply
to them. A leaf (a number, text, a character, a boolean, a path, `()`)
written at depth 0 with a spec applies the whole spec itself and clears
it, so `{c:.1}` with a `Display` that writes `self.degrees.fmt(mut f)`
rounds the degrees; whatever spec is left when the impl returns pads the
whole text, as text, in `Formatter.finish`.

**Specs** follow Python's mini-language,
`[[fill]align][sign][#][0][width][,|_][.precision][type]`, written in
Sisu (`Spec` in the prelude; the checker validates them, `E0399`):

- align `<` (left; texts' default), `>` (right; numbers' default), `^`
  (centred, the extra fill on the right), `=` (a number's fill after its
  sign and prefix); the fill is one character, a space by default;
- sign `+` (always), `-` (negatives only, the default), ` ` (a space for
  positives);
- `#` writes the prefix `0x`, `0o` or `0b`; `0` before the width pads a
  number with zeros after its sign and prefix, when no alignment is
  given;
- `,` or `_` groups an integer's digits, and a float's integer part, by
  three from the right, and by four in bases other than ten;
- a precision is the digits after the point of a number (a float
  without a type is written fixed with that many), and the most
  characters of a text;
- types: `d` (decimal), `x`, `X`, `o`, `b` for integers; `f` and `F`
  (fixed, six digits after the point unless a precision says), `e` and
  `E` (exponent, `1.500000e+00`, two exponent digits at least), `%` (a
  percentage, fixed) for numbers; `s` for the plain text; `?` for the
  `Debug` text, padded as text.

A `DateTime` takes a `strftime` pattern as its spec instead (`%Y %y %m %d
%H %M %S %F %T %%`, UTC). The host gives the digits of an integer in a
base (`fmt.digits`), the text of a non-negative float by type and
precision (`fmt.float_digits`, correctly rounded, Rust's
shortest-round-trip for the plain form) and the `strftime` text
(`fmt.strftime`); signs, prefixes, grouping, precision of texts and
padding are Sisu.

## The formatter

`sisu fmt [--check] [PATH...]` (10, "Formatter"; 02, "Formatting";
`sisu-fmt`) has no options: four spaces, 100 columns. It formats each
file, each `.sisu` file under each directory, or, without a path, the
package's; `--check` writes nothing and prints each file that would
change, as hunks of the change (`D0074`). A file with a syntax error is
not formatted (`D0075`), and the formatter is the only writer of
canonical text: nothing else in the toolchain rewrites source.

**How.** It works on the lossless tree, so it never disagrees with the
parser. The tree's tokens are regrouped into logical lines (what the
lexer ends with `NEWLINE`), each laid out by a Wadler-style printer of
groups (`doc.rs`: a group prints flat when it fits and breaks otherwise,
as Prettier's does), and the comments and blank lines between lines are
placed by rule. The rules are listed in `sisu-fmt/src/lines.rs` so that
another implementation writes the same text; in short: Black's
bracket-first splitting (a bracket's contents on their own lines, then
one element per line with a trailing comma), a trailing comma in the
source keeps the elements one per line (the magic trailing comma), a long
expression breaks before its binary operators (a continuation line, 02),
a comprehension that does not fit breaks before its clauses, a
definition's parameters break before its return type does (the
left-hand split), a comment at the end of a line does not count toward
its width, every comment keeps its text and its line, one blank line
around each top-level definition and at most one elsewhere, `Option[T]`
written `T?` where that reads unambiguously (of `Option[Option[T]]` the
inner one, `Option[T?]`; with spaces or line breaks inside too, not with
a comment, decision 267), `Result[T, E]` written `T !
E` when `E` names an error set (which is not in the syntax: the driver
resolves the file and tells the formatter which written types do,
`sisu-driver/src/format.rs`; `sisu fmt` resolves each file as a program
of its own, the language server with its whole program), and a run of
imports sorted with the unused ones removed (`prune.rs`: an import is
used when the file spells its name outside the imports, or in a doc
comment, whose examples are tests; a re-export is kept).

**The self-check.** The result must parse without error to the input's
*shape*, the tree's nodes and significant tokens up to what the
formatter changes on purpose (trailing commas, `T?` and `Option[T]`, `T
! E` and `Result[T, E]`, unused imports, the order of imports): otherwise it is refused as an
internal error (`D0076`) and the file left alone. `sisu parse --shape`
prints the shape (stage 1's, which refuses what its `fmt` refuses,
decision 622), so the conformance suite checks any implementation's
formatter: every case that parses keeps its shape and formats to a fixed
point (`fmt(fmt(x)) == fmt(x)`), a crash of `sisu parse --shape` counting
as a failure rather than as a syntax error, and every case without a
`NAME.unformatted` must already be canonical. CI runs `sisu fmt --check`
over `compiler/std/` and those cases.

## Documentation

`sisu doc [PATH] [--output DIR]` (09, "Documentation"; 10;
`sisu-driver/src/doc.rs`) writes a program's documentation as Markdown,
a page per module of the root package, and `sisu doc --std [MODULE...]`
the standard library's. It is a pure function of the modules' syntax
trees and what the front end resolved (decision 258), so that the front
end in Sisu can transcribe it (18's stage table puts it among stage 1's
`pure` functions over the syntax tree): nothing is evaluated, so a doc
example is shown as written and `sisu test` runs it.

**A page** is the module's path, its `##!` lines, the modules under it,
the names it re-exports (linked to their sections), a list of its items
with the first paragraph of each one's doc comment, and a section per
item: the declaration as written, with a header over several lines
joined into one, the contracts under it (07: "documented
automatically"), a struct's fields and an enum's variants with their own
`##` lines, and the headers of the module's impls of the type; the
item's `##` lines, which are Markdown, an example's fence labelled
`sisu`; and what the compiler knows that the declaration does not say:
the error set it inferred for a `T !` function (05), the typed holes left
in a function (10: "`sisu doc` marks the functions that contain one"),
and the parameters that hold capabilities, through the fields of the
types they are of (09). A type's methods and a trait's follow it, each a
section of its own; the impls of types the module does not declare close
the page. A package's pages show what another package may name, its
`pub` items and their `pub` members; a single file's and the standard
library's show everything.

**Where it goes.** Without `--output`, `sisu doc` prints the pages, each
after a line `<!-- NAME.md -->`, which a Markdown viewer shows as
nothing, so the whole output is one document; with `--output DIR`, each
page is the file `DIR/NAME.md`. A program with an error is reported as
`sisu check` reports it and nothing is written (`D0114`). The
conformance suite's `doc/` cases pin the output byte for byte
(`NAME.doc`), and check that `--output` writes the pages it prints.

## Hot replacement

`sisu run --watch` (10, "Debugging and profiling"; D-053, D-068, N-35;
`sisu-interp/src/live.rs`) runs the program once and swaps in each save
of its files, in **epochs**. SIR names a callee, a function value and a
closure by the function's name, never by its body, so every call
resolves the name in the function table of the epoch the running task
has adopted; a task adopts the newest epoch when it starts and at each
`live.boundary()` (std `live`, ambient; `Ticker.wait` calls it), so one
iteration of a loop never mixes an old body with a new helper, and a
frame in flight keeps the body it started with. `live.epoch()` is the
adopted epoch, 0 until the first swap.

The interpreter polls its host for saves at each of those points (the
host contract's `poll_edits`); the driver compiles the program again
from the saved files through its queries (`Rebuild`), and the new SIR is
compared with everything the run has, of every epoch:

- a function whose signature changed (parameters, result, conventions,
  purity or kind), a type whose layout changed, or a changed impl
  restarts the program from `main`, with the reason (`R0111`, `R0112`,
  `R0114`; a changed entry is a changed signature); values held then are
  dropped without their `Drop`s, since their code may be gone;
- otherwise every function whose body changed, and every new one, is
  published as one epoch with the types and impls new to the run
  (`R0109`, naming them); a changed `const` or `comptime` value changes
  the bodies that read it, which are swapped with it;
- a save that does not check publishes nothing, and the diagnostics are
  printed (`R0110`); the run goes on in its epoch.

What a save leaves out (the compiler emits only what a program reaches)
stays in the run for the frames and values of earlier epochs. The run
ends when the program does. Under `--record`, each save is an `edit`
record of the capability log at the poll that saw it, so `sisu replay`
applies the same saves at the same points and publishes the same epochs.
`--watch` does not combine with `--sir`, `--defer-borrow`, `--explore` or
the exhaustive scheduler (`D0078`), which run a program more than once
or not from source. The suite's `.watch` cases save their own next versions from
`NAME.files/`.

## The REPL and worksheets

`sisu repl [FILE|DIR]` (10, "REPL and scripts"; `sisu-driver/src/repl.rs`)
is a script that grows: each input the session keeps is one more
top-level statement or item, and an input that is an expression shows
its `Debug` text (nothing for `()`). An input runs the whole script again
from the top with the capability log of the run before it standing in
for the world up to where that run ended (`record::Resume`), so the
earlier inputs print nothing and touch nothing again and only the new
one runs live: the session has no heap to snapshot and no hidden state,
and it is deterministic under its seed. An input that does not check,
panics or returns an error is not kept (`D0083`). A file argument is the
session's first input; a package's directory (or the current one)
loads the package, whose modules the session imports by their paths
(`D0080`); the session sees their `pub` items.

- `:back N` forgets the last N inputs (`D0084`, `D0085`): the inputs
  before them replay against their log.
- `:why NAME` (`D0087`-`D0090`) shows where a value came from: the input
  that bound it with its value now, the session values that input read,
  each explained the same way, the capability results it received, from
  its log (`read_file "app.toml" -> ok ...  (log #2)`), and the inputs
  that changed it since. It follows the session's bindings, not the
  functions they call.
- If an earlier input does not replay as it ran (the world changed under
  a capability it read), the session says so (`D0091`).

`sisu ws FILE [--run]` runs a worksheet (Q-48): the script top to bottom,
writing after each top-level statement its value (`# => 3`, `D0092`) and
what it printed (`# | ys is [3, 6]`, `D0093`) to standard output; the
file is not changed. It runs against a test's fake `System` (files in
memory, no network, the test clock, seeded random numbers) unless
`--run` asks for the real one.

Both build their programs through stage 1 when it serves them (`sisu1
0.8.0` and later, or `SISU_STAGE1_REPL=1`; decisions 563 and 620), one
run for each input: `sir --repl SESSION INPUT --kept OUT` gives the SIR
of the session with the input after it and the session's program that
keeps it, or the diagnostics with the input's places drawn as `<input>`;
`sir --worksheet FILE` gives the worksheet's SIR with each statement
marked by the line it ends on. `:why` reads the inputs' trees from `sisu1
parse --tree` (decision 621). A stage 1 of 0.9.0 and later (or
`SISU_STAGE1_REPL_OPTIONS=1`) is also told where the program stands,
`--dir` (the working directory for a session, the worksheet's own
directory for a worksheet), and, for `sisu repl FILE`, what to call it,
`--name`; and `:why` reads its `parse --bindings` instead of the tree
(decision 640). A stage 1 of 0.10.0 and later serves them from one
process for the session, `sir --repl-serve`, which makes the standard
library's base once (`SISU_STAGE1_REPL_SERVE=0` runs a process for each
command again; decision 720). An older stage 1 and `--front-end=rust`
leave them to the Rust front end, which builds the same programs, as the
language server's worksheets are.

## The language server

`sisu lsp` (10, "Language server"; `sisu-driver/src/lsp.rs`) speaks LSP
3.17 over standard input and output: JSON-RPC 2.0 with `Content-Length`
framing, read and written by `json.rs` (stage 0 has no registry, so the
small part of JSON the protocol needs is written out). Every answer comes
from the queries of `Db`, the ones `sisu check` and `sisu run` use, so
the three never disagree, and since the checker types broken code, hover
and go-to-definition keep working in it. A document is analysed as part
of its program: the package whose `src/` holds it, the open documents'
texts standing in for the files on disk, or the file alone.

It provides diagnostics on every change (each with its catalog code and
identifier, `code` and `data`), hover (an expression's type; a
function's signature with its inferred error set, the capabilities it
takes and its contracts; a type's fields; the documentation),
go-to-definition (into the standard library's source too), document
symbols, selection ranges (the syntax tree's nodes around a position),
formatting (`sisu fmt`, as one edit) and range formatting, and the
features `assist.rs` writes:

- **Completion** (`.` triggers it): after a `.`, the members of the value
  before it (its fields, its type's methods and the methods of every trait
  without parameters of its own that it implements). The value is found by
  checking the document with the `.` and the word being typed removed, so
  that it is an expression of its own, and taking the expression that
  spans the postfix chain before the `.`. Elsewhere, the names in scope:
  the locals of the innermost body declared before the position, the
  module's items, the prelude's public ones, the program's variants and
  the keywords. A callable is a snippet with a hole for each required
  argument (`area(${1:w}, ${2:h})`, N-31); a struct a constructor with a
  hole for each field without a default.
- **Rename** (with `prepareRename`) and **references**: a local (its declaration and every
  use in its body and closures), a definition of the program's own (each
  token of the program's files spelling it that means it where it is
  written, or, for a type, that is written in a type) or a struct's field
  (its declaration, the reads, and the constructions' keywords), as one
  workspace edit over the program's files (references are the same
  occurrences, as locations). The standard library's items
  are not the program's to rename; a new name must be a name and not a
  keyword (`RequestFailed`, -32803, otherwise).
- **Code actions**: the fixes of the diagnostics over the range, each a
  `quickfix` titled as the human renderer writes the fix (its catalog
  message, then the text it writes), the first preferred.
- **Inlay hints**: the type of each local whose declaration does not write
  one (`let n = len(xs)` shows `: int`; patterns and closure parameters
  too), the error set each `T !` function infers (after the `!`), and
  `mut` or `own` before an argument passed to such a parameter (10's
  parameter conventions at call sites).
- **Explain this borrow**, the command `sisu.explainBorrow` (taking a
  document and a position): every loan live on the position's line, each
  with whether it is `mut`, the place it borrows, the ranges where it
  starts and is last used, the locals that hold it, and the line `sisu
  explain-borrow` prints for it: the data N-28's permissions overlay reads,
  when the bet-1 verdict lets it be shown. A program the front end rejects
  has no loans to show (`RequestFailed`).
- **Worksheets** as inline values (`textDocument/inlineValue`, Q-48): for
  a document that is a script, each top-level statement's value and what it
  printed, at the end of its last line, as `sisu ws` writes them; the
  worksheet runs against a test's fake `System`, with a budget of 10
  million statements, so nothing reaches the world and a loop cannot hold
  the server. The result is kept until the document's text changes.

With stage 1 as the front end, built and 0.14.0 or later, the
diagnostics, hover, go-to-definition, references, completion and document
symbols are stage 1's `query` answers instead (its R164; `query.rs`,
decision 983), each document's text written to a file of the server's own
for `--text` and the query served by the driver's server of stage 1
(decision 980); the other features, an older stage 1 and `--front-end=rust`
keep the answers above.

Documents are synchronised whole. Positions are UTF-16 code units unless
the client offers UTF-8. A request it does not know is answered
`MethodNotFound` (-32601).

Latency, on every module of the standard library opened as a document (a
release build, `cargo test --release --test lsp latency -- --nocapture`):
the diagnostics after an edit of one function body (a `pass` line in a
body in the middle of the file) take 11 to 51 ms (`std/cap.sisu` the
slowest, `std/prelude.sisu` 43 ms, `std/json.sisu`, whose reflection needs
instances, 19 ms), against 10's 100 ms, since the queries reuse every item
the edit left alone (next section); opening one, which checks it from
nothing but std's parse, takes 23 to 101 ms; a hover takes under 2 ms. The
tests in `sisu-driver/tests/lsp.rs` drive the server as an editor does,
over its standard streams, one per feature.

## The query layer and the driver

`sisu_driver::Db` memoises `parsed` per file (a program's files parsed
at once, on threads, decision 219) and `checked` and `lowered` (the SIR
through the ownership checker, with its findings as diagnostics) per
program, on hashes of the texts of its files and manifests, the
skeleton of 18's query pipeline; inside a program's query, checking,
lowering and the ownership checker are memoised **per item** (10,
"Compile speed as a requirement": a body edit never invalidates what does
not depend on it). Nothing depends on whether a result was cached:
`tests/incremental.rs` edits a function body of every std module and
conformance case (a line inserted before its first statement, a space
added at the end of that statement's line, in three bodies of each) and
checks that the database that saw the program before gives the
diagnostics and the SIR a fresh one gives; `Counters` show what each query
did (bodies checked and reused, lowered and reused, functions put through
the ownership analyses and taken as checked).

- **The interface.** `items::interface_hash` hashes what name resolution
  reads of a module: its tokens other than spaces, line breaks and plain
  comments (doc comments hold doc tests), with the statements of each
  function body replaced by a marker, except where resolution reads them:
  the body of a generic or `comptime` function (a generic function that
  reflects is a template, 08), a body's leading contract clauses (the
  compiler writes property tests from them, 07), and a body that names
  `arbitrary`, `exhaustive`, `Wrap`, `Fake` or `reflect` (the generators,
  interceptors and fakes the resolver writes for what a program names).
  The program's interface hashes its modules' interfaces with the layout,
  the files `embed` names and the generic functions promoted to templates.
  Resolution is deterministic, so two programs with one interface have the
  same definitions, with the same `DefId`s and signatures; only positions
  and the bodies left out differ.
- **Resolution** runs again for every change of the program's text, which
  costs 5 to 10 ms for a program over the prelude: the text it writes
  (derived impls, instances of templates, generators, shims, doc tests) is
  parsed once per process per text (`collect::parse_generated`), function
  bodies are shared between the syntax tree and the resolver's records
  (`Arc`), and coherence compares an impl only with the earlier impls of
  its trait whose self type can unify with its own (the same outermost
  constructor, or a parameter). The instances a check asks for are
  declared on top of the resolution that was checked
  (`sisu_resolve::Resolution::add_instances`), since an instance is
  declared after everything else and changes nothing declared before it:
  the rounds of the fixpoint resolve once, and only a promotion resolves
  again.
- **Checking** (`sisu_typeck::check_with`, `memo.rs`): a body is keyed by
  the interface, its definition, and the text of its item, from the start
  of the line its name is on to its end (for an instance, also what it
  instantiates, since two instances can unfold to one text); it records
  the inferred error sets it read (of the `T !` functions it calls). A key
  that matches with the same sets gives the typed body and its diagnostics
  again, every span moved by as many bytes as the item moved. A body with a
  span outside its item (a label at another definition) is not kept. The
  rounds of a check share the memo, so the bodies are checked once per
  check, not once per round. The functions' bodies the memo does not
  give are checked on threads, then kept in order (decision 219).
- **Lowering** (`sisu_lower::lower_cached`): a body's SIR functions by its
  checking key, the line its item starts on and the contract level. SIR's
  locations are lines and columns, so a body that moved to another line is
  lowered again and one that moved along its own line is not; an entry is
  kept only when every location in it lies on its item's lines.
- **The ownership checker** (`sisu_borrowck::check_with`): a function the
  checker found nothing in, and whose analysis read no other function's
  body (a generic callee given a must-consume type argument is read), is
  taken as checked while its SIR, the program's interface with its
  instances and the contract level are the same, with the drops it placed;
  purity, the `Shared` escape analysis and the compile-time evaluation run
  over the whole program.

Each memo belongs to one program and keeps what the last query of the
program used. `run --watch`'s rebuilds keep the database the run was
compiled with, so a save checks, lowers and borrow-checks again only the
items it changed; the REPL's inputs and the language server's changes go
through the same queries, with texts that stand in for the files on disk.
The prelude and std modules are parsed once per process
(`collect::std_ast`), since their text is fixed.

**Measured** (a release build, on the machine of the conformance runs):
over the 15 std modules and the 305 single-file conformance cases, each
opened as a program, 1872 edits of a function body re-check (diagnostics
and SIR) in 14 ms at the median and 117 ms at worst (`reflect/promotion`,
whose promotions resolve again), against 27 ms at the median for the same
program from nothing (`SISU_INCREMENTAL_ALL=1 cargo test --release --test
incremental -- --nocapture`): within 10's 300 ms for one edited body. The
language server's diagnostics after a one-body edit of each std module,
opened as a document, take 11 to 51 ms (`std/cap.sisu` the slowest,
`std/prelude.sisu` 43 ms; `cargo test --release --test lsp latency --
--nocapture`), within 10's 100 ms. A `sisu check` of a conformance case from
the command line takes 55 ms at the median (34 cases of `run/`, 42 to 79
ms), the process start and std's parse included; the memo lives as long as
the process (language server, `--watch`, REPL), so the command line checks
from nothing each time (port-debt.md, "The memo is not kept between
commands").

`sisu`:

```text
sisu run [FILE] [ARGS...]    check, lower and run; `--watch` swaps in each save (hot replacement),
                             `--sir` runs a SIR text file, `--from-sir FILE [ARGS...]` too
                             (decision 254),
                             `--monitor` runs under the ownership monitor,
                             `--defer-borrow` runs a rejected program in the counterexample mode,
                             `--contracts=off|preconditions|all` what it checks (07),
                             `--record LOG [--redact fs,net,...]` writes the capability log,
                             `--stats` the scheduler's numbers, `--stats stack-depth` the
                             frame depths, `--stats shared` each `Shared` construction site,
                             `--native [--release]` builds it natively and runs that
sisu replay LOG              run a recorded program again against its log (`--monitor`)
sisu check [FILE]            diagnostics; `--format=ids` for the conformance form,
                             `--diary LOG` appends them to the friction log
sisu doc [PATH]              the documentation, a Markdown page per module, printed;
                             `--output DIR` writes the pages there, `--std [MODULE...]`
                             documents the standard library (decision 258)
sisu build [FILE]            check and lower a package (or a file), and write its lockfile;
                             `--native [--release] [--output PATH]` the executable, a
                             package's in `build/debug/` or `build/release/`
sisu sir [FILE]              the serialised SIR, drops placed and `comptime` values baked
                             (also for a program the ownership checker rejects)
sisu test [FILE]             run the tests (and `tests/*.transcript`) under the monitor: ok,
                             FAILED, or blocked by a hole; `--no-monitor`, `--defer-borrow`,
                             `--no-cache`, `--jobs N`, `--filter TEXT`, `--tag T`, `--skip-tag T`,
                             `--update` (snapshots and transcripts), `--replay TOKEN`,
                             `--sim [--faults P]`, `--std` (the standard library's doc examples),
                             `--native [--release]` (compiled, decision 460); a test's output
                             is shown with its failure (decision 466)
sisu bench [FILE]            run the benchmarks; `--iterations N`, `--samples K`
                             (`run` and `test` also take `--schedule-seed N`, `--explore N`,
                             `--scheduler random|pct|exhaustive` and `--depth N`; `run`,
                             `check`, `build`, `doc`, `test` and `bench` take `--locked`)
sisu fmt [PATH...]           format in place; `--check` reports what would change
sisu repl [FILE|DIR]         a session; a package's directory loads the package
sisu ws FILE                 run a worksheet; `--run` with the real `System`
sisu lsp                     the language server, over standard input and output
sisu new NAME                a package (`--lib` for a library)
sisu tokens FILE             the token stream; `--trivia` includes trivia
sisu parse FILE              `--tree` the syntax tree, `--ast` the AST built from it,
                             `--print` the source again, `--shape` what formatting keeps
sisu explain CODE            the catalog entry for a code or identifier
sisu explain-borrow FILE:LINE  every borrow live on the line
sisu audit --ownership FILE...  per file, the uses of the ownership model's escape hatches
sisu audit --caps FILE...    per program, the capabilities its I/O takes and threads
```

Without `FILE`, a command works on the package of the current
directory or the nearest above it (section "Modules and packages"); in a
workspace, `check`, `build` and `test` work on every member, and `run`,
`sir` and `bench` on the one application among them (`D0067`
otherwise).

**The front end** of `check`, `sir`, `build`, `run`, `test`, `bench`,
`fmt`, `doc`, `tokens` and `parse` is stage 1, the front end written in
Sisu, compiled natively (`compiler/target/stage1/sisu1`, built from
`compiler/stage1` in the release profile with every contract checked when
a source of it is newer, `SISU_STAGE1_PROFILE=debug` for the debug one,
decision 860): they run its commands and
print what it prints, and a program runs from the SIR it serialised, in
the interpreter or natively, as its tests and benchmarks do (decisions 268
and 357). `--front-end=rust`, among a command's options, or
`SISU_FRONT_END=rust` for a whole process, keeps the Rust front end this
document describes, the oracle; the other commands (`repl`, `ws`, `lsp`,
`replay`, `explain-borrow`, `audit`, `test --std`, `test
--defer-borrow`, the tests of a program with transcripts, and `run
--watch`, `--record` and `--defer-borrow`, `check --diary` and `parse
--shape`) run on it whichever is chosen (port-debt.md, "The Rust front
end's roles still to replace"), but for `lsp`'s questions that stage 1's
queries answer from 0.14.0 (decision 983), as `eval` is stage 1's alone
(decision 984). A command that stage 1's `sir --repl-serve` serves (from
0.13.0: `check`, `sir`, `parse`, `fmt`, `lint`, `doc`, `tokens`, and the
`sir --tests` of `test` and `bench`) is asked of the driver's server of
stage 1, which keeps the standard library's base between commands
(`serve.rs`, decision 980; `SISU_STAGE1_SERVE=0` runs a process for each).

`sisu audit --ownership` is the corpus study's count (18, Phase 3; T1.1
and T1.2): for each file, one line of data, the file's name and then
`lines=N Shared=N Weak=N Cell=N Arena=N Key=N copy=N unsafe=N`. The
counts are of tokens: those names where they mean std's types (not after
a `.`, and not in a file that declares the name itself), calls of
`.copy()`, and the keyword `unsafe`; `lines` counts the lines on which a
token other than trivia and layout starts. A file need not check to be
counted.

`sisu audit --caps` is Phase 6's count (18; 20's T2.1, T2.3 and the
threading depth), read from the program's SIR once it checks (decision
240). Each line starts with the file's name: `function NAME caps=N
io=direct|calls depth=D` for each function of the program that does I/O
(it calls an operation of a capability, or a function that does), `N`
its parameters that hold a capability (a handle, a `System`, a `Lent`, a
struct or collection holding one, or a type parameter an operation is
called on) and `D` the fewest calls from `main` (`-` when `main` does not
reach it); `use FUNCTION OPERATION depth=D` for each operation of a
capability a function calls; `struct NAME caps=N TYPES` for each of the
program's structs whose fields hold three or more capability types
besides a whole `System`; and `summary functions=N median=M max=K uses=U
deepest=D structs=S`.

`sisu check --diary LOG` (18, Phase 3: "appends every diagnostic with a
timestamp and code to a per-author JSONL log for the friction
measurement"; 20, I-3; `sisu-driver/src/diary.rs`) appends to `LOG` one
JSON object per line, whatever the check finds:

- a `check` record: the time (RFC 3339, UTC), the tool and the
  toolchain, each of the program's own files with the FNV-1a hash of its
  text, and the number of diagnostics;
- a `diagnostic` record for each diagnostic: the file and its hash, the
  code, the catalog identifier, the severity, the message rendered and its
  parameters, the span (lines and columns, and bytes), the labels, and
  the fixes, each with its message and edits;
- for each diagnostic of a file's previous check that this one no longer
  has, a `resolved` record: the diagnostic as it was, the edit between
  the two texts (their common prefix and suffix taken away: a span of
  the old text and its replacement), and the fix it applied, when making
  that fix's edits to the old text gives the new one exactly (`null`
  otherwise). A diagnostic is still there when this check has one with
  the same identifier and parameters at the bytes the edit moved it to.

The text of every file checked is kept by its hash in `LOG.d/`, so that
the next check can find the edit, and so that the source of an ownership
rejection can be read again (20, A2). A log that cannot be written is
`D0013`.

`run` prints warnings and runs; with any error it prints the diagnostics
and does not run (`D0006`, or `D0021` for ownership errors, which names
`--defer-borrow`). The ownership checker runs only on a program without
front-end errors, so one mistake is not reported twice, and the
compile-time evaluation only on a program the checker accepts; a failed
`comptime` expression is an error (`D0006`) for `run`, `test` and
`check`. `sisu sir` writes the SIR with every `comptime` value baked. An internal compiler error (a host panic, which the
lints are meant to make impossible) is caught and reported as `D0012`
naming the stage, with the input written to the temporary directory, and
exits 70.

## Stage 2: the native backend

Stage 2 of the bootstrap ([18](../../proposal/18-next-steps.md),
"Bootstrapping") compiles SIR to machine code. Its first round is the
debug backend of [10](../../proposal/10-tooling.md) ("A non-LLVM backend
for debug builds"): Cranelift 0.123 at its lowest optimisation level
writes an object file, and the system's `cc` links it with the runtime.
Two crates hold it, and they carry the workspace's only external
dependencies (decisions 186 and 203):

- `sisu-codegen` compiles a `sisu_sir::Program` to an object file that
  defines the C `main`, and links it. It writes each function's Cranelift
  IR on the calling thread and compiles the IR on threads, defining the
  machine code in the order it made the functions (decision 219); each
  thread keeps its Cranelift context and hands every function back,
  cleared, for the next to be written into (decision 228).
- `sisu-rt` is the runtime: a static library (`libsisu_rt.a`) linked into
  every executable, and a Rust library whose `abi` module the code
  generator shares, so that both sides of the boundary are one
  definition. It holds the type table, the panics in flight and their
  stack traces, standard output, allocation, the structural operations on
  values, iterators, dicts and sets, and the intrinsics, each with the
  interpreter's results, messages and exit statuses. It is the only crate
  with `unsafe` code (decision 187).

`sisu build --native [FILE]` checks, lowers and checks ownership as `sisu
run` does, compiles the SIR that `sisu sir` prints (after the ownership
checker, jump threading and compile-time evaluation, so the backend
compiles exactly what the interpreter runs), links it, and prints `built
PATH`, the path relative to the working directory (decision 354). The
executable goes beside a file's source, named by its stem
(`hello.sisu` makes `hello`), or, for a package, to `build/PROFILE/NAME`
in the directory of its lockfile, its workspace's or its own
(`stage1/build/debug/stage1`; decision 231); `--output PATH` puts it
elsewhere. `sisu build` without `--native` checks and lowers, as before,
and writes nothing but the lockfile. The profile (09, "Build") is `debug`,
or `release` with `--release` (decision 232): Cranelift's `speed`
optimisations, the checks of overflow and bounds as in every build, and
the contracts the root package's `[profile.release] contracts` names,
preconditions by default (07); `--contracts=LEVEL` overrides it, in
either profile. A check is one branch the common path does not take, to a
block out of its way, and the release profile leaves out the checks the
facts it knows about a function's integers prove (decision 270). `sisu run --native [FILE] [ARGS...]` builds a package
where its builds go and a file into a directory of its own under the
system's temporary directory, which it removes after, runs the executable
with the arguments, the standard streams and the environment of the
command, and exits with its status (decision 197). It takes
`--schedule-seed N`, `--explore N`, the scheduler's `--stats` and
`--scheduler pct` or `exhaustive` with `--depth` (section "Tasks" below;
decision 541); it does not combine with the options that only the
interpreter has (`--sir`, `--monitor`, `--defer-borrow`, `--stats
stack-depth`, `--stats shared`, `--watch`, `--record`; `D0109`), and `--release` goes with `--native` only (`D0111`). A
construct the backend does not compile yet is `D0103`, naming it and the
function it is in; the program still runs without `--native`. `SISU_NATIVE_SURVEY=1`
lists every such construct of a program with the number of functions
that have it, and `SISU_NATIVE_TIMES=1` prints where a build's time went:
a line for each step of checking and lowering the program (`time: names
resolved 133.0 ms`), of generating its code, and the link.
The SIR is lowered through stage 2, the lowering written in Sisu, to LIR,
which the backend translates one to one (section "LIR" below); a construct
stage 2 does not lower is `D0103`, and only `--lower=rust` (or
`SISU_LOWER=rust`) lowers with the Rust lowering, the oracle (decisions 304
and 305). A debug build's LIR is compiled by the x86-64 backend written in
Sisu (section "The Sisu backend" below), a release build's by Cranelift,
the oracle of the Sisu backend, which `--backend=cranelift` names for a
debug build too (decision 331).
`sisu build --native --from-sir FILE [--output PATH]` and `sisu run
--native --from-sir FILE [ARGS...]` compile a file of serialised SIR
instead, without the front end (section "From serialised SIR" below), and
`sisu run --from-sir FILE [ARGS...]` runs one in the interpreter
(decision 254). `--from-lir FILE` compiles a file of LIR, the low IR that
stage 2, the backend written in Sisu, lowers SIR to (section "LIR" below;
decisions 281 to 284).

### Values

A sized integer narrower than 64 bits and an `f32` are their own width,
read with the sign their type gives and written at that width wherever
they are, and a declared struct lays its fields out at C's offsets, its
size a multiple of its widest field's alignment (decisions 1020 and
1021), so that a list, an array or a slice of `u8`s holds a byte to an
element and a pointer to a `@repr(c)` struct of narrow fields points to
what C reads. Every other value is a whole number of 8-byte words, and
every part of one (an element of a tuple, a payload, a table's key or
value) starts on a word and takes a word at least, a narrower one's bytes
first (decision 188). Each concrete type of the program has a
number and an entry in the type table, whose kinds (`abi::Kind`) say
where the parts are:

| Type | Words | Layout |
|---|---:|---|
| `()`, `Never`, a constant argument | 0 | |
| `u8`, `i8`; `u16`, `i16`; `u32`, `i32` | 1, 2, 4 bytes | the value at its width, extended by its sign when read (decision 1020) |
| `int`, `u64`, `DateTime` | 1 | the value; a sized integer is kept in its range by its operations, a `u64` is unsigned; a `DateTime` is nanoseconds since the epoch, shown as RFC 3339 text in UTC |
| `i128`, `u128` | 2 | the low word first |
| `f64`, `f32` | 1, 4 bytes | the bits |
| `bool`, `char` | 1 | 0 or 1; the code point |
| `str` | 2 | the address of the UTF-8 bytes, their length |
| `String`, `Path`, `bytes` | 3 | a heap buffer, the length and the capacity in bytes (decision 208) |
| `list[T]`, `Array[T, N]` | 3 | a heap buffer of elements, the length and the capacity in elements |
| `Slice[T]` | 3 | the address of the first element it views in place, their number, and 0 (decision 940); a slice of `bytes` views them in place too (decision 1020), and one `mem.slice_from_raw` made holds copies in a buffer of its own, whose capacity is the third word |
| `ref T`, `ref mut T` | 1 | the address of the value |
| `dyn Trait` | 2 | the number of the type of the value it holds, and the address of a heap cell holding the value, which the `dyn` owns |
| `ref dyn Trait` | 2 | the same two words, borrowed: the address is of the value the `dyn` holds |
| `Box[T]` | 1 | the address of a heap cell holding the value |
| `T?` | 1 + T | a tag (0 `Some`, 1 `None`), then the payload |
| `Result[T, E]` | 1 + the larger | a tag (0 `Ok`, 1 `Err`), then the payload |
| a tuple | the sum | the parts in order, each a word at least |
| a struct | C's | the fields at C's offsets, each aligned to its width (a narrow number's, a struct's widest field's, a word for the rest), the size a multiple of the widest (decision 1021) |
| an enum | 1 + the largest variant | the tag (the variant's index), then the variant's fields |
| an `error` enum, a union of error sets | 3 + the largest variant | its own type's number, its return trace, the tag, then the fields |
| a function value | 2 + K | the code's address, then its closure's environment, K words, the largest an `own` closure of the program needs and at least one, or the address of a larger environment of references (decision 224), then the environment's type number plus one when a copy or a drop must reach what it holds, else 0 (decision 248) |
| `dict[K, V]`, `set[T]`, `Iter[T]` | 1 | the address of a runtime object |
| a capability handle | 1 | the address of a cell that holds its object as a `dyn` holds its value: the object's type, its address, then the object (decision 209), after a word counting the handle's copies (decision 237) |
| `Lent[T]` | T | the handle (or test clock) it lends |
| `Cap[T]` | 1 | the address of a heap cell holding the value, after a word counting the `Cap`'s copies (decision 237) |
| `TestClock` | 1 | the number of the run's test clock in the scheduler |
| `HostStream`, `HostListener` | 1 | the socket's number in the runtime's table |
| `System` | 7 | its handles, in decision 101's order |
| `Scope`, `Handle[T]`, `Cancellation` | 1 | the number of the scope or the task in the scheduler |
| `Sender[T]`, `Receiver[T]` | 1 | the number of the channel |
| `Timer` | 2 | the number of its clock, and the moment in nanoseconds on it |
| `Mutex[T]` | 4 + T | whether it is locked, the task holding it, whether it is poisoned, the tasks waiting to write an `RwLock` (decision 249), then the value |
| `MutexGuard[T]` | 1 | the address of the value inside its mutex |
| `RwLock[T]` | 4 + T | a `Mutex[T]` whose lock word is 2 while tasks read, its holder word then their number (decision 239), and whose fourth word counts the tasks waiting to write (decision 249) |
| `ReadGuard[T]` | 1 | the address of the value inside its `RwLock` |
| `Atomic[T]` | T | the value, as a `Cell[T]` |
| `Shared[T]`, `Weak[T]` | 1 | the address of the value in a heap cell, after a header of four words: the owners, the weak handles, the task that made it, its flags (decision 207) |
| `Cell[T]` | T | the value |
| `Arena[T]` | 1 | the address of a runtime object holding the slots, each a generation, whether it holds a value, and the value |
| `ref Iterable[T]` | 2 | the number of the type of what the caller passed, and its address (decision 210) |
| `ptr[T]`, `ptr mut T` | 1 | the address (decision 302); an option of one is a tag and the address, as any option, and a nullable word only in raw memory and at a foreign call |

A union of error sets holds a value of one of its member enums, as that
enum lays it out; the first word says which. A dict or set is a table of
entries in the order they were inserted, each the key's words then the
value's, with a hash index over the keys' structural hash, as the
interpreter's are; an iterator is a runtime object that yields copies of
its items or references to them.

### Functions, calls and unwinding

A compiled function is `fn(ret, arg_1, ..., arg_n) -> u8`: the result and
every argument live in memory, the function gets their addresses, and it
returns 0, or 1 when a panic is leaving it (decision 189). The caller puts
each argument in memory of its own, which the callee owns as the
interpreter's callee owns its locals. Every local has a stack slot except
the result and the parameters; in the release profile, a local of one
word (a number, a `bool`, a `char`, or a reference that is an address)
whose own memory no code needs is a Cranelift variable instead, a
parameter's value read once at the entry (decision 246).

Unwinding is the cleanup blocks of SIR run as ordinary control flow, with
no unwinder. Each statement that can panic (a call, a drop, an overflow or
bounds check, a dict key, `unreachable`) is followed by a test of its
status, which leads to the failure path of its block: in a cleanup block,
the abort of a panic during unwinding (`panic.during-unwind`, status 134,
both messages); in any other, a line of the stack trace (the function,
file, line and column of the statement, left out for drop glue and the
standard library as the interpreter leaves them out), then the block's
`unwind` block, or the return of 1 when it has none. A runtime entry point
that panics records the panic and returns 1 the same way. The panic's
message and trace are the runtime's until `main` returns: a panic that
leaves `main` is printed with its trace and the program exits with 101; a
`main` that returns `T ! E` and returns an error prints it with its return
trace and exits with 1; otherwise 0.

Every function but drop glue counts itself in `SISU_RT_DEPTH` on entry and
out on return; past 100 000 nested calls a call panics with the
interpreter's `panic.stack-overflow`, which names the function, unless a
panic's unwinding has room left within 1 000 more (decision 234). The
program runs on a thread with a 1 GiB stack, so that the limit, and not
the machine's stack, is what a deep recursion meets (decision 190).
Before that count, every function's entry probes its stack (section
"Tasks"), and after it, but in drop glue, passes a yield point. A panic's
trace deeper than a hundred frames shows the fifty innermost and the
fifty outermost, with a line for the rest between them, in the
interpreter as natively, so that what an overflow prints does not depend
on how deep each one's stack lets it go (decision 234).

### Each SIR construct

| SIR | Native code |
|---|---|
| a place `_n` | the local's slot, or the address the caller passed |
| `.*` (`Deref`) | the address the reference holds; a reference to a `dyn` is the `dyn`'s own words, in place |
| `.i` (`Field`) | the field's offset in the type's layout; a `Box`'s field 0 is the boxed value, at the address the box holds |
| `.as(v)` (`Downcast`) | the payload's fields after the tag; for a `drop`, the tag is tested first and another variant drops nothing |
| `[_n]`, `[^_n]` (`Index`, `IndexFromEnd`) | the buffer plus the index times the element's size, after a bounds test that raises the interpreter's index panic |
| `{_n}` (`Key`) | `sisu_rt_dict_slot`: the address of the value a dict holds for the key; reading a missing key panics, an assignment's destination inserts it, a drop of a missing one does nothing |
| `copy p`, `move p` | the words at the place (a move is the same copy: the ownership checker saw that the source is not used or dropped again) |
| `const` | an immediate; a text is the address and length of read-only bytes; a function is the pair of its value thunk and a null environment |
| `ref p`, `ref mut p` | the place's address; for a `dyn`, a copy of its two words |
| arithmetic on `int` | Cranelift's instructions, `+`, `-`, `*` and unary `-` with an overflow test that raises `panic.overflow` with the interpreter's operands; `//`, `%`, `**` and the shifts through `sisu_rt_int_op`; wrapping and saturating operators inline |
| arithmetic on `f64`, `f32` | instructions, `//`, `%` and `**` through `sisu_rt_float_op`; an `f32` operation is computed in `f64` and rounded once (decision 169) |
| comparisons of numbers, `bool`, `char` | `icmp` and `fcmp` |
| `==`, `<`, `in`, `+` of texts on other values | `sisu_rt_binary`: structural equality, order, membership and concatenation by the type table; `==` and `!=` of texts, options and enums other than error sets inline, the runtime comparing what a word does not hold (decision 253) |
| `not`, `~` | `x ^ 1`, `bnot` |
| `tuple(...)`, `Point(...)` | the parts at their offsets in a new slot |
| a variant | the tag (and for an `error` enum its type's number and an empty trace), then the fields |
| `Box(x)` | a heap cell from `sisu_rt_alloc`, holding the value |
| `dyn T(x)`, `dyn ref T(r)` | the number of `T` and a heap cell holding `x`; the number of `T` and the address `r` holds (decision 202) |
| `list(...)` | a heap buffer holding the elements |
| `set(...)`, `dict(...)` | the entries in a slot, made into a table by `sisu_rt_table` |
| `discriminant p` | the tag word |
| `closure f(...)` | the code of `f`'s instance and the environment, the tuple of the captures (decision 195) |
| `copyof p` | `sisu_rt_copy`, a deep copy by the type table |
| `strview p` | the `String`'s first two words |
| `p = rvalue` | the rvalue's words stored at the place's address |
| `call fn f[T](...)` | a direct call of the instance of `f` for the concrete type arguments |
| `call trait Tr.m[S, ...](...)` | resolved when the instance is compiled (decision 191), then a direct call, an intrinsic, or a structural operation; with a `dyn` `S`, a call of the method's dispatcher (decision 202) |
| `call intrinsic name(...)` | `sisu_rt_intrinsic` with the call's descriptor; the sized integers' operations through `sisu_rt_sized` and `sisu_rt_wide` (decision 193); the commonest inline, the runtime called where they panic, grow a buffer or keep state of their own, and a range or a borrowed list's iterator that never leaves its function in a slot of the frame (decision 253) |
| `call _n(...)` | an indirect call of the value's code with its environment first |
| `drop p [if flag]` | when the type needs a drop: the flag's test (clearing it), then `sisu_rt_drop`, which runs the type's drop glue or the structural drop; a text's buffer, or a list's whose elements need no drop, freed inline (decision 253) |
| `dead _n` | nothing |
| `goto`, `switch`, `return`, `resume` | `jump`, Cranelift's `Switch` on the scalar, the return of 0, the return of 1; an edge to a block at or before the current one passes a yield point first |
| `unreachable` | the `unreachable` panic and the failure path |

### Generics, traits, closures and capabilities

Code generation starts at `main` and compiles each function once for
every list of concrete type arguments that a reached call gives it: an
instance. A call through a trait is resolved when its instance is
compiled, by the rules the interpreter's `dispatch.rs` applies at run
time: the impl whose self type and trait arguments match the concrete
ones, with `Self` taken through any references (the first argument is
read through as many); its method is an SIR function, an intrinsic, or,
where a type has no impl of `Display`, `Debug`, `Eq`, `PartialOrd`, `Ord`
or `Hash`, the structural operation the interpreter falls back to
(decision 191). The associated types of impls are looked up the same way.

A `Self` that is a `dyn` is the one dispatch left to run time, since only
the value knows its type. A call of a trait's method on a `dyn` calls a
dispatcher, one function per method and type arguments, with the
address of the `dyn`'s two words: it switches on the number of the type of
the value, and calls what the call runs for that type, resolved as above,
with `self` the value's address (or a copy of the value, for a method that
takes it). The types a dispatcher knows are those of the values the
compiled code puts in `dyn`s of its type; compiling what a dispatcher
calls can compile more `dyn`s, so code generation repeats until no new
type and no new dispatcher appears, then writes the dispatchers (decision
202). The structural operations look through a `dyn` to its value, as the
interpreter's do.

A closure is its function's code and its environment, the tuple of its
captures, held in the function value itself and copied with it: a
function value has as many words of environment as the largest `own`
closure of the program needs, at least one, so that nothing is allocated
and nothing needs freeing, and a last word that names the environment's
type (its number plus one) when what it holds is counted, an `own`
closure's copies of handles, else 0: a copy of the value counts them and
its drop lets them go, through the runtime only when the word is not 0
(decision 248). Which size that is only compiling finds, so a
program with a larger `own` environment is compiled again with wider
function values. A call passes the callee the address of the
environment's words in the value; a closure of references whose
environment is larger than they are cannot outlive the frame it is made
in, so its environment is a slot of that frame, whose address is the
value's first word of environment (decision 224). A task's closure (or a
generator's) keeps a heap cell, which the runtime frees (decision 206). A
named function used as a value is a thunk that takes an environment and
ignores it, so every function value is called as a closure is (decision
195); a generic one, or one that borrows a parameter, is a closure of its
value wrapper (decision 225).

`main(sys: System)` receives seven handles, each on an object of the host
the runtime keeps. A handle is a cell that holds its object as a `dyn`
holds its value, made by `cap.new[T, H]` for a fake, an attenuation, a
caretaker or an interceptor, and by the runtime for the host's objects,
with the count of its copies in the word before it: the compiled code
adds one for each copy it makes (`sisu_rt_retain`, for a by-value `self`
in the dispatcher), each drop takes one, and the last frees the cell
after dropping the object (decision 237); a
`Lent` is its handle, and a test clock (`Clock.fake`) the number of a
virtual clock of the scheduler, which becomes a `Clock` through `cap.new`
as any other object does. A call through a handle (the prelude's
`cap.forward`) compiles to the host object's operation when nothing `main`
can reach makes a handle of that type hold another object (decision 194);
otherwise it calls a dispatcher, as a call on a `dyn` does, whose types are
the host's object and the objects of the `cap.new`s compiled for that
handle type (decision 209). What `main` reaches is found from `main`
itself, through calls, closures, function values and the impls a trait
call may select (through a `Lent` to its handle's), so that a program's
tests and their fakes, which `main` does not run, do not count.

A parameter of type `Iterable[T]` takes whatever iterable the caller has:
the caller passes a reference to it together with the number of its type,
as a reference to a `dyn` is passed, and the runtime iterates by that type
(decision 210). A value the caller moved in is dropped, with whatever the
callee left of it, when the call returns or unwinds (decision 235); an
intrinsic takes such a value whole, so its caller leaves it alone. An
iterator whose items are references (a generator's of `ref T`) decays
for a `T` that is not one, as a borrowed collection's items do (decision
247).

### The runtime

The runtime's entry points (`abi::sym`) are the boundary:

| Entry point | What it does |
|---|---|
| `sisu_rt_start(argc, argv, entry, flags, ret_ty, types, types_len, fns, fns_len, unprobed) -> i32` | decodes the type table, keeps the function table, starts the scheduler (the probe reserve holds `unprobed`, the most a call may write below a probe limit before a probe sees it), runs `main` on its thread (with a `System` when `flags` says so; with no workers for `std.par` under the profile `single`, decision 783) and gives the exit status |
| `sisu_rt_frame(name, len, file, len, line, col)` | adds a line to the stack trace of the panic in flight |
| `sisu_rt_abort_unwind()` | a panic reached a frame running cleanup: abort |
| `sisu_rt_export_panic()` | a panic left an exported function of C's convention: its report, and status 134 (decision 785) |
| `sisu_rt_panic_overflow`, `_index`, `_stack`, `_unreachable` | raise those panics |
| `sisu_rt_int_op(op, x, y, out) -> u8`, `sisu_rt_float_op(op, x, y) -> f64` | the arithmetic that is not one instruction |
| `sisu_rt_alloc(bytes) -> ptr` | heap memory for a list or a box |
| `sisu_rt_task_env(bytes, ty) -> ptr` | the environment of a task's closure (or a generator's), of type `ty`, which the runtime frees when the closure returns, and drops for a generator closed before it started (decision 236) |
| `sisu_rt_drop(value, ty) -> u8`, `sisu_rt_copy(dst, src, ty)` | the drop and `.copy()` of a value by its type, which the runtime in Sisu walks (decision 350) |
| `sisu_rt_retain(value, ty)` | a copy of a value of type `ty` that holds capability handles was made: each handle's cell counts one more (decision 237) |
| `sisu_rt_binary(op, out, a, a_ty, b, b_ty) -> u8` | equality, order, `in` and concatenation of values that are not numbers, the first three by the runtime in Sisu (decisions 350 and 352) |
| `sisu_rt_bytes_eq(ret, a, b, n) -> u8` | whether the `n` bytes at `a` and `b` are the same: `==` of two texts of one length (decision 253); of Sisu's convention, the runtime in Sisu's (decision 336) |
| `sisu_rt_text_push(ret, t, b, n) -> u8`, `sisu_rt_text_push_int(ret, t, n) -> u8`, `sisu_rt_text_from(ret, dst, b, n) -> u8` | the `n` bytes at `b`, or the digits of `n`, appended to the `String` at `t`; a new `String` at `dst` (decision 265); of Sisu's convention, the runtime in Sisu's (decision 351) |
| `sisu_rt_list_reserve(l, size)` | room for one more element of `size` bytes in the list at `l` (decision 265) |
| `sisu_rt_intrinsic(desc, ret, args) -> u8` | a call of an intrinsic |
| `sisu_rt_sized(kind, op, flags, x, y, out) -> u8`, `sisu_rt_wide(kind, op, x, x_ty, y, y_ty, out) -> u8` | an operation of a sized integer type, of one word or of 128 bits |
| `sisu_rt_table(ty, items, n, out) -> u8`, `sisu_rt_dict_slot(dict, key, key_ty, mode, out) -> u8` | a dict or set literal; a dict's value for a key, and `dict.index_str`'s (decision 861) |
| `sisu_rt_table_take(table, key, key_ty, value, out) -> u8` | `dict.remove_str` (the option of the value) and `set.remove_str` (whether it was there) in place, the key the table held dropped, by the runtime in Sisu (decision 861) |
| `sisu_rt_arena_slot(arena, key, mode, out) -> u8` | the address of the value an arena holds for a key; a stale key panics (`R0072`), but for a drop, which does nothing |
| `sisu_rt_shared_touch(value)` | a `Shared`'s cell reached through `*`: counts it for `--stats` when another task than its maker reaches it |
| `sisu_rt_flush()` | writes what the program printed so far, before a foreign call (decision 302) |
| `sisu_rt_thread_slot() -> ptr`, `sisu_rt_switch(save, load)` | the machine intrinsics `rt.thread_slot` and `rt.switch` (decision 302) |
| `sisu_rt_free(ptr)` | frees a box's or a `dyn`'s cell whose value was moved out (decision 216) |
| `sisu_rt_trace_release(error)` | a field was moved out of an error value, which is never dropped whole: it lets go of its return trace (decision 215) |
| `sisu_rt_yield() -> u8` | a yield point found the budget spent: the scheduler may switch tasks, and a property's case that ran out unwinds (decision 540) |
| `SISU_RT_DEPTH` | the counter of nested calls; this and the next three are thread-local, each thread's own (decision 332) |
| `SISU_RT_BUDGET` | the yield points left before the scheduler is asked |
| `SISU_RT_STACK` | the running task's stack limits: the probe's, the real one, the unwinding floor, and the watermark's and the failure's flags; and whether it is a generator being closed (decision 236) |
| `SISU_RT_SITE` | where the running code last called an intrinsic that may wait, and whether its function is `pure` |

What a program allocates is freed when its value is dropped, as the
interpreter's host frees it (decisions 213 to 218 and 220 to 225): the
runtime counts its live allocations of values (heap blocks, and the
runtime objects of iterators, tables, arenas and return traces), and
`SISU_RT_ALLOCS=1` makes a native run report the count at exit (`R0125`),
which the native mode of the conformance runner holds at 0 for every run
that does not abort (decision 226), as a test does for a program that
makes everything the first two rounds leaked; `SISU_RT_ALLOCS=peak` also
reports the most that were live at once (`R0126`), which a test holds to
the same number for 20 and for 500 rounds of a loop that makes and drops
values whose fields hold lists, dicts, sets, texts and boxes (decision
229). What the interpreter's host
lets go of without dropping it, the runtime frees without running Sisu
code (forgetting: no drop glue, no `Drop`, no guard's check, decision
223) when the run ends: the error a `main` returned, and the cells of
capabilities and `Cap`s that a copy of their handle kept to the end, one
a function value's environment holds (decision 237).

The type table is encoded by `abi::encode` into the object's read-only
data and decoded once when the program starts; the function table holds
the address of each type's drop glue, which is an SIR function compiled
like any other. The structural operations (drop, copy, equality, order,
hashing, `Display` and `Debug`) walk a value by its type's entry, in the
order and with the texts of the interpreter's, and a struct or enum with a
`Drop` impl or fields that need dropping runs its glue (decision 192); all
but `Display` and `Debug` are the runtime in Sisu's, over the table laid
out in memory (section "The runtime in Sisu", decision 350). An
intrinsic call passes a descriptor that the code generator writes once per
shape of call (the intrinsic's number, the types of the result, of each
argument and of each type argument) and the addresses of the arguments;
the runtime reads through references, so an intrinsic does not care
whether it got a value or a reference to one (decision 193). The
commonest intrinsics are compiled inline instead, on the layouts the
table above gives, and an iterator's object begins with a header that
the code steps a range or a borrowed list's elements in (decision 253).
Standard
output is buffered and written before anything goes to standard error and
when the program ends, so the two interleave as the interpreter's do.

### The runtime in Sisu

The runtime is being written in Sisu, piece by piece, as D-086 orders
(decisions 335, 336, 350 to 352, 358, 383, 385, 422, 423, 506, 542, 562, 625, 645, 646, 680, 681, 724, 780, 784, 820, 821, 902, 903 and 940). `compiler/rt` is a Sisu package
(`unsafe = "extern"`) whose root module's `export_NAME` functions are the
runtime's symbols `sisu_rt_NAME`, of Sisu's convention (decision 189),
which the generated code and the runtime in Rust call as any function is
called: the result's address, each argument's, the status byte. It is
compiled once into `compiler/target/rt/rt.o`, which every native program
is linked with before the runtime in Rust, from its seed,
`compiler/rt/rt.sir`, the SIR stage 1 prints of the package (a test holds
it to that), by the Rust lowering in the release profile and Cranelift
(`sisu_codegen::compile_library`), since it must exist before stage 1, a
native program itself, can compile it; stage 2 compiles the same seed
with the Sisu backend (`sisu2 build --library`), which the fixed point of
decision 330 covers (decision 353). Its code is the runtime's own: it
counts no frame, passes no yield point and adds no line to a stack trace,
as drop glue does not; it reaches raw memory only through the operations
of std.mem the backend compiles inline (decision 335), and the runtime in
Rust and C's `memmove` and `memcmp` through foreign calls, before which it
writes out no output (decision 350); a construct that needs a type table
of its own is refused when the object is built.

What is in Sisu (port-debt.md, "The runtime in Sisu", has what is not):

| Piece | Module | Called by |
|---|---|---|
| `==` of two texts' bytes (`sisu_rt_bytes_eq`) | `lib.sisu` | the generated code (decision 253) |
| equality, order, the structural hash, `.copy()`, the drop and the count of a copy's handles, by a value's parts (`sisu_rt_value_*`) | `value.sisu` over `types.sisu` | the runtime in Rust (`bridge.rs`), for `sisu_rt_drop`, `sisu_rt_copy`, `sisu_rt_binary`, dicts and sets, sorting and the intrinsics (decision 350) |
| text appended to a `String`, an `int`'s digits appended, a `String` from a text (`sisu_rt_text_push`, `_push_int`, `_from`), and the digits of an integer of up to 128 bits in a base (`sisu_rt_int_digits`) | `text.sisu` | the generated code (decision 265), and the runtime in Rust for `fmt.digits` and `fmt.wide_digits` (decision 351) |
| text search by Two-Way, and a prefix or suffix (`sisu_rt_find`, `sisu_rt_has_edge`) | `search.sisu` | the runtime in Rust, for `in` on texts and `bytes.find`, `starts_with` and `ends_with` (decision 352) |
| a float's text: the shortest that reads back as the same `f64` or `f32`, the fixed form and the form with an exponent (`sisu_rt_float_digits`) | `floats.sisu` | the runtime in Rust, for `Display` and `Debug` of floats, `fmt.float_digits` and `fmt.float32_digits` (decision 358) |
| `Display` and `Debug` of a value by its type (`sisu_rt_value_display`, `_debug`) | `show.sisu` | the runtime in Rust, for `fmt.display`, `fmt.debug`, `print`, `list.join`, a failed `assert`'s sides and the error `main` returns (decision 383) |
| the intrinsics of lists, dicts, sets and slices (a slice of a list a view of it in place, decision 940), the lookups by a `str` among them (decision 784), the thirteen that make an iterator or take every item of one (`range`, `enumerate`, `zip`, `zip_shortest`, `reversed`, `iterable_list`, `sorted`, `any`, `all`, `iter.ref`, `iter.mut`, `iter.own`, `Iter.of`; decision 820), and `sum` (decision 903), their descriptors decoded (`sisu_rt_collection`, the first six words read once, decision 862, an argument's place found without a call, decision 902), `in` as an operator (`sisu_rt_contains`), a table's literal and the inline lookups and removals, the five by a `str` among them (`sisu_rt_table_literal`, `_inline_find`, `_inline_take`; decision 861) | `dispatch.sisu` over `list.sisu`, `table.sisu` and `iter.sisu` | the runtime in Rust, which reads the intrinsic's number alone and hands the call over, and whose entries `sisu_rt_binary`, `sisu_rt_table` and `sisu_rt_table_find` call the others (decisions 625, 646 and 820) |
| the scheduler's queues: the run queue, the waiters under their keys and the timers (`sisu_rt_queue_*`) | `queue.sisu` over `list.sisu` | the runtime in Rust's scheduler (`sched.rs`), which keeps their addresses (decision 645) |
| the channels' buffers: the values sent and not received, in a ring of slots of the value's size (`sisu_rt_channel_*`) | `channel.sisu` | the runtime in Rust's channels (`tasks.rs`, `queue::ChanBuf`), which keep their addresses (decision 680) |
| the scheduler's seeded generator and, under the default strategy, the choice of the next task (`sisu_rt_draw_*`) | `draw.sisu` over `queue.sisu` | the runtime in Rust's scheduler, `select` and the faults of `sim` (`queue::Rng`), which keep the generator's word (decision 724) |
| the tasks' records (a task's state, what it waits for, whether it is a generator), a task parked under its keys, and the tasks a key or a cancellation wakes put in the run queue (`sisu_rt_task_*`) | `tasks.sisu` over `queue.sisu` | the runtime in Rust's scheduler (`queue::Records`), which keeps beside each record, under the task's number, its coroutine, context and outcome (decision 780) |
| PCT's change points and priorities, a yield point and the next task under PCT, and the exhaustive search's choices, its trail and its next task (`sisu_rt_choose_*`) | `choose.sisu` over `queue.sisu` and `draw.sisu` | the runtime in Rust's scheduler and `sim.choose` (`queue::Choices`), which keeps the queues and the task a search's yield point chose (decision 821) |
| a list's growth and moves, a slice's copies, the sort, `in` and `join` (`sisu_rt_list_*`) | `list.sisu` | `dispatch.sisu`, the runtime in Rust for every list it builds, `sorted`, `reversed` and a slice's copies, and the generated code's `sisu_rt_list_reserve` (decision 385) |
| the tables of dicts and sets: a new one, a key looked up, an entry appended and one taken out, and a whole table copied, compared, shown and dropped; `d{k}`'s slot (`sisu_rt_table_slot`) | `table.sisu`, `dispatch.sisu` | `dispatch.sisu`, the runtime in Rust's `sisu_rt_dict_slot`, which the generated code calls for `d{k}` (decision 681), and `value.sisu` and `show.sisu` (decision 422) |
| iterators: the next item, the iterator of an `Iterable[T]` argument, every item left, a new iterator of the five kinds the runtime in Rust still makes (an owned list's items, a text's characters and bytes, a copy of bytes, a generator's items) and what one may wait for (`sisu_rt_iter_next`, `_iterable`, `_collect`, `_items` and its kin, `_waits`) | `iter.sisu` | the runtime in Rust, for `Iter.next` of what does not wait, every iterator it takes items from or makes, `dispatch.sisu` and `value.sisu` (decisions 423, 542 and 820) |

The structural operations walk a value by the program's type table, which
the runtime in Rust lays out in memory when the program starts
(`sisu-rt/src/desc.rs`): a header of eleven words for each type (its
kind, size, flags, the types it holds, its name's number, where its lists
are, the offset of its tag, where its names are) and the lists after them
(a struct's fields, a tuple's elements, an enum's variants, an error set's
members, the parts a drop and a copy reach in place, decisions 262 and
266, and the texts of the names `Debug` writes, decision 383). The
runtime in Rust passes the table's address to each operation. What it
keeps as objects of its own (channels, guards, `Shared` cells, arenas,
the cells of capability handles, return traces) the Sisu code hands back
to it (`sisu_rt_object_drop`, `_copy`, `_retain`, `sisu_rt_trace_retain`);
a generator's items, a receiver's values and a generator closed early it
asks it for (`sisu_rt_gen_item`, `sisu_rt_recv_item`,
`sisu_rt_gen_abandon`), as they run the scheduler; and the program's drop
glue it runs through the function table (`sisu_rt_glue`), or, on the
system stack, moves aside to be dropped on the task's (`sisu_rt_defer`).
On a task's own stack, the runtime in Rust runs a struct's or an enum's
glue where it is and the Sisu walk of what the glue leaves out on the
system stack (`export_value_drop_rest`), so that a task whose stack is
spent still unwinds with its drops: the walk's frames are a few
kilobytes, every argument of a call being in memory of its own. While
runtime code runs on the system stack for a task, the task's probe limit
is lifted, since the Sisu code probes the stack as every function does.

### Tasks

Natively, tasks run as the interpreter runs them ("Tasks and the
scheduler"): one at a time, under its seeded cooperative scheduler, with
its scopes, cancellation, joins, channels, `select`, generators, timers,
`log` and mutexes, its panics, messages and exit statuses, and, under a
seed, its schedule. The code is `sisu-rt/src/sched.rs` (the scheduler),
`tasks.rs` (the intrinsics) and `stack.rs` (the stacks), after the Phase 1
harness (`taskbench`), whose verdict chose how a task's stack is made
(decisions 203 to 206).

**Stacks.** The root task runs `main` on the program's thread. Every
other task, and every generator, is a coroutine (corosensei, the
harness's context switch) on a stack of its own: a slot of a pool carved
from reservations of 1 GiB of address space, committed only where it is
touched, with no guard page, so that tasks cost mappings by the
reservation, not by the task (the harness's strategy D(B)). A slot holds
the usable stack, 1 MiB unless `SISU_TASK_STACK` says otherwise (`512K`,
`8M`, a number of bytes; at least 64 KiB), above the probe reserve. What
keeps a task inside its slot is the probe at the entry of every function:
the stack pointer, below the frame the prologue made, against the task's
limit (`SISU_RT_STACK`). Cranelift allocates a frame before any code of
ours runs, but writes only its top there (the saved registers, and the
spill slots the register allocator may fill at the entry), never the
stack slots below them, and the code generator keeps the parameters'
addresses for the debugger only after the probe: comparing the stack
pointer after the prologue is the Phase 1 report's comparison of the
stack pointer less the frame, for all of the frame but its top. So the
limit leaves the policy's 5 KiB below it, and room for what a call may
write before a probe sees it: the largest top of a probed frame and the
largest frame of a function without a probe (a dispatcher, a thunk),
which the code generator measures (for stage 1, 1.2 KiB rather than its
largest frame's 69 KiB; decision 234). A function whose probe fails
records its name, lowers the limit to an unwinding floor and returns
`UNWINDING` at once, calling nothing; the runtime raises the stack
overflow, which names the function (`panic.stack-overflow`), the next
time it looks at the panics in flight, and the task unwinds with its
drops, which may use the reserve down to the floor. `SISU_TASK_STACK` is
where the reservation is set: 06 and 10 name no option for it. When a
task waits, the pages of its stack below the stack pointer it parked at,
less 2 KiB for the switch, are given back (`madvise`), and a slot is
given back when it returns to the pool but for its top page. The probe
doubles as the decommit watermark: after a decommit the task's limit is
the decommit point, so a probe fires only when the stack grows below it
again, marks the task dirty and lowers the limit, and a task that parks
at the same depth again makes no system call.

**The system stack.** Runtime code called from a task runs on the root
task's stack, as Go's `systemstack` does, so that a task's stack holds
Sisu frames, the thin wrappers around the switch, and nothing whose depth
the probe cannot see. Two things stay on the task's stack: what makes it
wait, and Sisu code. An operation that must wait registers the wait and
returns; its wrapper suspends the task, and tries the operation again when
the task is woken, so it completes exactly once, as the interpreter
retries a blocked call. A drop on the system stack that would run drop
glue or a mutex guard's `invariant` check moves the value aside, and the
wrapper drops it on the task's stack once the runtime code returns, as the
interpreter queues the drops of an intrinsic.

**The loop.** When the root task gives way or waits, it runs the
scheduler's loop on its own stack: the loop picks the next task, resumes
it until it gives way, waits or ends, and picks again, until it picks the
root task, whose call then returns. A task that gives way or waits
suspends itself back to the loop. Each task keeps its count of nested
calls (an anchor of the interpreter's, a scope's or a shield's, counts
as a frame), its stack limits and its panics in flight, which a switch
exchanges.

**The same schedule.** The yield points are the interpreter's: the entry
of every function the interpreter enters with a call statement (the
probe and the count of the frame come first), every edge to a block at
or before the current one, a spawn (which spends one without switching),
and `task.yield_now`. Generated code counts them down in
`SISU_RT_BUDGET` and calls `sisu_rt_yield` when it reaches zero, whose
status unwinds a property's case that ran out of its budget (the counter
holds both budgets, decision 540); the
budgets, the next task and every choice among ready `select` arms are
drawn from the interpreter's generator in its order, and the ready list,
the waits and the timers are its data. A function the interpreter enters
without a call statement (`main`, a task's closure when it first runs, a
generator's body, a guard's `invariant` check) is entered without its
entry counting. A run under a seed is therefore the interpreter's run
under it: `sisu run --native --schedule-seed N` gives the seed to the
executable (`SISU_SCHEDULE_SEED`), `--explore N` runs it under N seeds as
the interpreter's runs do, and `--stats` (`SISU_STATS`) reports the
interpreter's tasks, most at once and switches, the latency in
microseconds (native code counts no statements, `R0124`) and the
`Shared` cells. `cargo test` holds it: the executables print what the
interpreter prints under every seed tried, for programs whose output is
their schedule (`sisu-driver/tests/schedules`) and for the concurrency
cases.

**Closing generators.** A generator whose iterator is dropped before its
end is closed, as the interpreter closes it (decision 236). One suspended
at a `yield` is resumed there and then, outside the schedule, with its
`SISU_RT_STACK` word of abandonment set: its yield returns `UNWINDING`
without a panic, and its frames unwind through their cleanup blocks,
running their drops and `defer`s; meanwhile the running task's state, its
panics in flight and the values it moved aside are kept apart. A panic
there aborts the program (`panic.during-abandon`), and an operation that
would wait raises one (`panic.abandoned-wait`). One that never started
drops what its environment holds, by the type its block carries
(`sisu_rt_task_env`), and an item it yielded that nobody took is dropped
too. A generator runs only when its consumer asks: a scope's
cancellation does not wake one.

**Deadlock.** When nothing is ready and no timer is pending, the program
stops with the interpreter's report (`R0080`, `R0081`) and status 134; a
call that may wait records where it is (`SISU_RT_SITE`) for it.

**Data parallelism.** `std.par.map` runs its bodies on a pool of worker
threads apart from the scheduler, as 06 and D-069 have it (decision 333;
`sisu-rt/src/par.rs`): the backend compiles `map`'s body as a call of the
runtime's parallel map, which hands the elements to the workers (as many
as the machine has processors, or `SISU_PAR_THREADS`) and writes each
result at its element's place, so the list is the one the elision gives.
Each worker has a 1 GiB stack, its own count of nested calls, stack
limits and budget (decision 332), the latter never spent, and a scheduler
of its own for the generators a body drives. The lowest index's panic is
the call's, after the results before it are dropped; a call on a worker
runs its bodies there in order. The interpreter runs `map` as written, on
the asking task, whose budget its bodies spend (port-debt.md). A program
whose SIR says `runtime single` (12's profile, decisions 694 and 723) has
no workers: both lowerings set `ENTRY_SINGLE` in `main`'s flags, and every
call of `map` runs its bodies on the asking thread, one after another, as
a call on a worker does, so the executable starts no thread for it, as
the interpreter starts none (decision 783).

### Raw memory, foreign calls and the machine intrinsics

Unsafe code runs natively without the interpreter's checks in the release
profile (decision 302): the raw memory operations are the unchecked ones
(22). In the debug profile the runtime checks them against a shadow of
`Global`'s blocks, as the checked memory model does (decisions 543 and
544): `main`'s flags carry `ENTRY_CHECKED`, the raw reads, writes and
offsets the release profile compiles inline go through the runtime, and
each intrinsic of std.mem checks what it reaches before it does it. The intrinsics of std.mem
are the runtime's (`sisu-rt/src/rawmem.rs`), called through the
descriptor as the others are (`abi::Intrinsic`, numbered after the older
ones so that theirs stay what they were), and `ptr[T]` and `ptr mut T` are
a word whose type table entry names what they point to (`Kind::Ptr`). The
runtime computes each type's layout in raw memory from the type table when
the program starts, the interpreter's rule ("The checked memory model"):
a value whose layout in memory is its layout as a value (most types) is
read and written as its bytes, a narrow number and a struct of them among
them since decisions 1020 and 1021; another (an option of a raw pointer, a
tuple holding a narrow number, a struct holding either) is converted part
by part, each part at its C offset and width.
`Global.acquire` is `posix_memalign` with the layout's alignment, counted
among the program's allocations, and `Global.release` frees it;
`drop_in_place` converts the value back and drops it by its type, its glue
on the task's stack; `copy_from` is `memmove`; `bit_cast` and `transmute`
convert the value to its bytes and the bytes to the other type, refusing
other sizes with the interpreter's panic; `read_plain` reads from bytes at
any alignment; `view_plain` gives the bytes in place, or, for a type laid
out otherwise in a value, a copy as a value that is freed with the bytes it
views (`views.rs`, decision 214); and `slice_from_raw` copies the values
into a new slice that owns them; a slice of a list or of `bytes` views it
in place (decisions 940 and 1020).

A place of the program holds its value as a value is laid out, and a
reference is read so, so a pointer to a place (`ptr.of`, `ptr.of_mut`, a
slice's `as_ptr`) and a reference made from a pointer (`as_ref`,
`as_mut`) are made only to a type that raw memory holds as a value is laid
out, which since decisions 1020 and 1021 are a narrow number and a struct
of them too (so a `CString`'s `as_ptr()`, `mem.slice.as_ptr` of its bytes'
slice, compiles, and so does `ptr.of` of a `@repr(c)` struct with `u8`
and `i32` fields); another, a tuple with a narrow number among its parts
or an option of a raw pointer, is `D0103` when the program is built,
naming the operation and the type, and the interpreter runs the program. A foreign call
(`extern.LIBRARY.NAME`) is a call of the C function `NAME`, imported from
the C library the executable is linked with, in C's convention: each
argument loaded from its slot and narrowed to its C type (an integer or
float of its width, a raw pointer, an option of one as a nullable
pointer), the result widened back (sign-extended for a signed type) or
made an option, and the runtime's buffered standard output written first,
since the function may write to the same streams. Two calls that give one
function different C signatures are `D0103`. The machine intrinsics are
the backend's: `rt.stack_pointer` reads the stack pointer in place,
`rt.thread_slot` is the address of a thread-local word of the runtime, and
`rt.switch` is a routine of the runtime's (x86-64 and AArch64 Linux) that
pushes the registers C keeps across a call, saves the stack pointer, the
frame pointer and the address to resume at in the `Context` at `save`, and
continues from the one at `load`, whose second half pops the registers and
returns from the switch that saved it. No program reaches them (std.rt is
internal), so `crates/sisu-driver/tests/machine/machine.sir` calls them
from SIR, and a test of the runtime switches to a second stack and back.

Stage 2, the lowering in Sisu, lowers all of it as `sisu-codegen` does
(decision 305): the intrinsics of std.mem in its table of names, at the
numbers `abi::Intrinsic` gives them after `live.epoch` (277 to 305, in its
order); `ptr` and `ptr_mut` laid out as one word with the entry `Ptr`
(kind 46) naming their argument (`layout.sisu`, and the encoding of the
type table in `abi.sisu`); the build-time refusal above, by the rule of
`Types::same_in_memory`; a foreign call as `declare NAME import (C TYPES)
-> (C TYPE)`, made at its first call, a `call sisu_rt_flush()` before it,
the arguments loaded and narrowed (`ireduce`, `bitcast` for the floats, a
`select` of the payload or 0 for an option of a pointer) and the result
widened back (`sextend`, `uextend`, `bitcast`, or an `icmp_imm eq` making
an option's tag), as `call_foreign` and `c_argument` in `func.rs` do
(`call.sisu`); and the machine intrinsics (`get_stack_pointer`, and calls
of `sisu_rt_thread_slot` and `sisu_rt_switch`), as `machine_intrinsic`
does. LIR gained two words for them: the conversion `sextend` and the type
`i16`, a C `short`'s.

An `@export("name")` of a program is the global symbol `name` of C's own
convention over plain values (decision 785): a wrapper, `unprobed`,
declared `export` with the C signature of the function's parameters and
result (an integer or float of its width, a `bool` as a byte, a `char` as
a 32-bit code, a raw pointer as an address; stage 1's `E0991` admits the
rest, a `@repr(c)` struct, which C passes as a pointer to it), that
writes each argument widened to its word in a slot of its frame, calls the
function, compiled under a local symbol of Sisu's convention as any other
and called by `main` directly, and narrows its result back, as a foreign
call's arguments and result are converted the other way; a status other
than 0, a panic that left the function, calls `sisu_rt_export_panic`,
which writes the panic's report and stops the program with status 134,
since no C caller can unwind. Both lowerings write it alike
(`Codegen::export_c`, stage 2's `Cg.export_c`) and give the same object.
A runtime library's exports stay of Sisu's convention.

### Linking

`sisu-codegen` writes the object beside the output (`NAME.o`, removed
afterwards unless `SISU_NATIVE_KEEP=1`), and `cc` links it with the
runtime in Sisu's object (decision 336; `SISU_RT=PATH` names another, used
as it is, decision 353), the runtime and the system
libraries a Rust static library needs, using LLVM's
`lld` when `ld.lld` is on the search path: it links in a third of the time
of the system's `ld` (87 against 224 ms for a small program). The search
path is given to `cc` explicitly, since the conformance runner runs `sisu`
in an empty environment (decision 198). The runtime is the
`libsisu_rt.a` cargo built beside the `sisu` executable, or the newest
`deps/libsisu_rt-*.a` there, which a `cargo test` leaves without copying
it up; without one the build is `D0104` (decision 199). The link leaves
out the sections nothing reaches from `main` (`--gc-sections`), and the
runtime in Sisu's object has one for each function (decision 682). An
executable is mostly the runtime: 6.25 MB for a ten-line program with the
release runtime, 1.74 MB of it stripped, 21 MB with the debug one.

### Debug information

The executable carries DWARF 4 for its Sisu code (decision 227), which
`sisu-codegen/src/debug.rs` writes with gimli into the object beside the
code, as 10 asks of the debug backend ("Debugging and profiling"):

- A line table maps each function's code to the file, line and column of
  the SIR statement it was compiled from. Cranelift carries a statement's
  location to the instructions made for it (a `SourceLoc` of a 20-bit line
  and a 12-bit column), and with no optimisation each statement's code is
  its own, so single steps go statement by statement. The code before the
  first statement (the prologue, and the keeping of the parameters'
  addresses) is the declaration's line, and the first statement's row is
  marked as the prologue's end, so a breakpoint on a function stops with
  its parameters in place. A standard library file is named in the
  directory of the `std` the compiler read, the program's own files
  relative to the directory of the build.
- A subprogram for each function instance names it, with its named
  locals and parameters, each at its stack slot and of its Sisu type. A
  parameter's memory is the caller's, so its address is kept in a slot of
  its own, through which the debugger finds it. Drop elaboration's flags
  are left out, and so is a release build's local in a register (decision
  246).
- `.debug_frame`, from Cranelift's unwind information, lets a debugger
  unwind through Sisu frames into the runtime's.

The unit says its language is Rust, the closest one gdb and lldb know: a
text is described as `&str`, a list's items as a slice, an enum, an option,
a result and an error as a struct with a variant part (an error's type
number and trace are artificial words before it), and an error set of
several errors as a variant part chosen by the number of the error's type,
so gdb prints `r = Err(TooBig{limit: 2})` and `Err(Problem(Missing{what:
...}))`. A lent parameter is a reference, as Rust's is. lldb (18) has no
Rust plugin and prints a variant part as its raw variants. A panic's
message is not the debugger's business: the code keeps the location of
each call as it runs, and the runtime prints the Sisu functions and lines
the interpreter prints. `crates/sisu-driver/tests/native.rs` builds a
program and runs it under `gdb --batch` (else `lldb`, else it reads the
line table with `readelf`): a breakpoint on a function and one on a line,
the Sisu functions and lines of the backtrace, and the locals and
parameters with their values. For stage 1 the debug information is 1.2
MB of the 18 MB executable and adds about 8% to the build (medians 6.6
against 6.1 s on a loaded machine), 0.1 s of it writing the DWARF and the
rest emitting and linking the larger object.

### From serialised SIR

The serialised SIR `sisu sir` prints is a whole program: the program's
functions and the standard library's that it reaches, its types, impls
and intrinsics. `sisu_sir::read` parses exactly what `sisu_sir::write`
prints, so a program read back writes the same bytes again; a test does
that for every `.sir` expectation of the suite, and another for the SIR of
every case of `native.list`. `--from-sir FILE` hands such a file to the
backend: the reader, the validator, then code generation and the link as
for a program from source, the executable named by the file's stem. Code
generation is deterministic, so the executable is byte for byte the one
the program's source gives. Without `--native`, `sisu run --from-sir
FILE` runs such a file in the interpreter, as `sisu run --sir FILE` does,
and `sisu build --from-sir` goes with `--native` (`D0110`; decisions 212
and 254).

This is the path of the self-hosted check of the bootstrap. Stage 1, the
front end written in Sisu, is built natively and prints the SIR of a
program; that text must be byte-identical to what stage 0's front end
prints for the same program (`sisu sir --front-end=rust`), and `sisu build
--native --from-sir` turns it into the executable that building the
program from its source gives. Stage 1 then stands in for stage 0's front
end: the same SIR, the same native code. Since decision 268 it does: the
`sisu` command's front end is stage 1, whose `build --native` hands its
SIR to this backend, and `sisu run` interprets the SIR it serialised. The runner's mode `sisu-conformance --native-from-sir
conformance/native.list` builds every case of the list that way (from the
case's own `.sir` where it has one, else from what `sisu sir` prints for
it) and compares its runs with the interpreter's expectations, and CI runs
it beside the native mode. The mode `--interpreted-from-sir` runs the same
SIR in the interpreter (`sisu run --from-sir`), so that what a front end
serialised is checked against both of stage 0's ways of running it.

### LIR

LIR is the backend-neutral low IR of D-087's first step: the program as
the backend's single pass sees it, which stage 2, the backend written in
Sisu (`compiler/stage2`), lowers SIR to, and which every backend of
D-087 and D-088 starts from (decisions 281 to 284). It holds what
`sisu-codegen` decides between SIR and Cranelift IR: the layouts of the
values and the type table, every SIR local in a slot of its function's
frame (or, in the release profile, a variable, decision 246), the calls
by address that return the status byte (decision 189), the entry probe,
the count of nested calls and the yield points (decisions 190, 203, 204
and 234), the overflow and bounds checks as branches to cold blocks
(decision 270), drop glue, dispatchers and thunks as ordinary functions,
the runtime's entry points by their symbols, the read-only data with the
words that hold functions' addresses, a source location for every
instruction, and what the debug information needs beyond the code (each
function's name, file and line, its named locals with their slots, and a
description of each local's Sisu type). What it leaves to the backend is
what only machine code knows: where each slot lies in the frame, which
register holds a value, the encoding, the line table's addresses and the
call frame information, which the backend makes from the code it writes,
and the probe reserve of a task's stack, which it measures from the
frames it laid out (decision 234).

A function's body is the calls `sisu-codegen` makes of Cranelift's
function builder, one per line, in their order: an instruction defines its
values (numbered in the order defined, the entry's parameters first), a
variable is used and defined as the builder's are (`use`, `def`), a block
is made, marked cold and switched to as the builder's is, and a `switch`
says how many blocks the function had made when it was written, so that
a backend that numbers its blocks as it makes them makes the switch's
own after them. So the translation of LIR into Cranelift IR inside the
Rust backend (`sisu-codegen/src/lir.rs` reads it, `translate.rs` makes
the calls) gives each function the IR the backend gives it from SIR,
value for value and block for block, and the object file and the
executable are the same bytes; the conformance runner holds every case of
`native.list` to that (decision 284). A debug backend that compiles LIR
in one pass reads the same lines as a sequence of typed operations on
virtual registers with explicit control flow; the variables are SSA
construction's, the only state it carries across blocks.

`sisu2 lir FILE.sir [--release] [--output FILE.lir]` (the script
`compiler/stage2/sisu2`, which builds stage 2 natively on first use, as
`sisu1` builds stage 1) prints a program's LIR; `sisu build --native
--from-lir FILE.lir [--output PATH]` and `sisu run --native --from-lir
FILE.lir [ARGS...]` compile it, the executable named by the file's stem
(`--from-lir` goes with `--native`, `D0119`). Stage 2 reads the SIR
itself (`stage2/src/read.sisu`, a transcription of `sisu-sir/src/read.rs`,
since stage 1 writes SIR but has no reader of it) into stage 1's SIR types
(`stage1.sir`, a path dependency), and lowers it rule for rule as
`sisu-codegen` does (`stage2/src/gen.sisu` is `lib.rs`, `func.sisu`,
`rvalue.sisu`, `call.sisu` and `scan.sisu` are `func.rs`, `facts.sisu`
is `facts.rs`, `layout.sisu` is `layout.rs`, `debug.sisu` is the part of
`debug.rs` that describes types, `abi.sisu` is the part of
`sisu-rt/src/abi.rs` the code generator shares), in both profiles. It
lowers the SIR of `examples/wordcount` (5 065 lines) to 39 108 lines of
LIR in 0.07 s, and the SIR of stage 1's own package (493 018 lines, 2 616
function instances) to 4 026 377 lines of LIR (96 MB) in 6.0 s, or in
15 s in the release profile, whose facts it computes; built from that
LIR, stage 1 is the same 23 MB executable its SIR gives, in both profiles
(6.1 s from LIR as from SIR: the translation, Cranelift and the link).

#### The text form

One construct per line; blank lines and lines starting with `#` are
ignored, and leading spaces are not significant. The lowering writes it
deterministically, and the same SIR gives the same bytes.

```text
lir 1
profile debug                                   -- or `release`
source 0 "hello.sisu"                           -- the program's files, as SIR numbers them
source 1 "<std>/prelude.sisu"
data d0 import tls SISU_RT_DEPTH                -- a thread-local data symbol of the runtime
data d4 bytes "main\x00"                        -- read-only bytes
data d7 align 8 u32 0 1 0 0 1                   -- 32-bit words, little-endian
data d40 align 8 zeros 16 func 0 sisu$P#drop$7 func 8 sisu$L#drop$9   -- words holding function addresses
declare sisu$main$1 local sisu 0                -- a function of Sisu's convention with 0 arguments
declare sisu_rt_alloc import (i64) -> (i64)     -- a runtime entry point
declare main export (i32, i64) -> (i32)
function sisu$main$1 probed name "main" source 0 line 2
    slot s0 8                                   -- the frame, in the order the slots were made
    vars 2                                      -- variables x0 and x1, each 64 bits
    local "x" t0 s0 line 3                      -- a named local, for the debugger
block b0
    v1 = iconst.i64 2
    def x0, v1
    v3 = get_stack_pointer
    ...
    brif v6, b1, b2
block b1 cold
    ...
end
types
type t0 base "int" signed 8
stats 1 33 0 0 0 0 0
```

The items between the sources and `types` come in the order the lowering
made them, which is the order of the object file: a symbol is declared
before anything refers to it, and the functions are defined in this
order.

- `data dN import [tls] NAME` declares the runtime's data symbol `NAME`,
  thread-local with `tls` (decision 332); `data
  dN [align A] bytes "..."` defines read-only bytes (printable ASCII as
  itself, `\\`, `\"` and `\xHH` for any other byte); `u32 W...` defines
  32-bit words; `zeros N` defines `N` zero bytes, in which each `func
  OFFSET SYMBOL` is a word holding a function's address. Data is numbered
  in the order it is declared, imports included.
- `declare SYMBOL LINKAGE SIGNATURE` declares a function: `local` (the
  program's), `import` (the runtime's) or `export` (C's `main`). The
  signature `sisu N` is Sisu's convention: `N + 1` 64-bit addresses (the
  result's, then each argument's) and the 8-bit status (decision 189);
  otherwise `(T, ...) -> (T, ...)`.
- `function SYMBOL FRAME name "SHOWN" [source S line L]` defines a
  declared function, up to `end`. `FRAME` says how its frame counts
  toward the probe reserve of a task's stack (decisions 203 and 234):
  `probed` (its entry probes the stack), `unprobed` (a dispatcher or a
  thunk) or `outside` (C's `main`). `SHOWN` is the name the debugger
  shows, `source` and `line` where it is declared. Its first lines are the
  frame: `slot sN SIZE` for each slot in the order made (`SIZE` bytes, at
  least 8, aligned to a word), `vars N`, and `local "NAME" tK sN
  [indirect] [param] [line L]` for each named local (its type's
  description, its slot, `indirect` when the slot holds the value's
  address, as a parameter's does, `param` for a parameter). Then its
  blocks.
- `block bN [cold]` switches to block `N`, made earlier, numbered in the
  order made; `cold` marks a block only a panic, a failed check or
  unwinding reaches, which the backend places after the rest (decision
  270). The first is the entry, whose parameters are the function's.
- `types` starts the descriptions of the debug information's types (below);
  `stats` ends the file with the counts `SISU_NATIVE_TIMES` reports: the
  function instances lowered, the SIR statements and terminators, the
  index checks, those proven, the overflow checks, those proven, and the
  tests decided (decision 270).

Values are `vN`, each defined once; types are `i8`, `i16`, `i32`, `i64`,
`f32` and `f64` (`i16` only in a foreign function's signature, decision
305). An operation that defines values starts with them (`v7 =
...`, `v8, v9 = ...`):

| Operation | Meaning |
|---|---|
| `iconst.T N` | the constant `N` (a float's is its bits) |
| `iadd`, `isub`, `imul`, `sdiv`, `srem`, `ishl`, `sshr`, `band`, `bor`, `bxor`, `band_not`, `fadd`, `fsub`, `fmul`, `fdiv` `a, b` | arithmetic, the integer kinds wrapping |
| `iadd_imm`, `imul_imm`, `bxor_imm`, `bor_imm` `a, N`; `ineg`, `bnot`, `fneg` `a` | the same with a constant; negation and complement |
| `icmp.CC a, b`, `icmp_imm.CC a, N`, `fcmp.CC a, b` | a comparison giving an `i8`, 1 or 0 (`eq ne slt sle sgt sge ult ule ugt uge`; `eq ne lt le gt ge`) |
| `sadd_overflow`, `ssub_overflow`, `smul_overflow` `a, b` | the wrapped result and the overflow flag |
| `select c, a, b` | `a` when `c` is not 0, else `b` |
| `uextend.T`, `sextend.T`, `ireduce.T`, `bitcast.T`, `fpromote.T`, `fdemote.T` `a` | conversions |
| `load.T a, OFF`, `uload8 a, OFF`, `sload8 a, OFF`; `store v, a, OFF` | memory at `a + OFF`, aligned, never faulting |
| `stack_addr sN`, `stack_load sN`, `stack_store v, sN` | a slot's address, its first word |
| `symbol_value dN`, `tls_value dN`, `func_addr SYMBOL`, `get_stack_pointer` | addresses; `tls_value` the running thread's copy of thread-local data |
| `call SYMBOL(a, ...)`, `call_indirect N, f(a, ...)`, `memcpy d, s, n` | calls; `call_indirect` of Sisu's convention with `N` arguments |
| `use xN`, `def xN, v`, `param bN` | a variable's value, a new one; a parameter of a block |
| `loc LINE COL` | the source location of the operations that follow |
| `probe_reserve` | the probe reserve of a task's stack, which the backend computes (decision 234) |
| `jump bN(a, ...)`, `brif c, bA(...), bB(...)`, `switch v, bD, [K bA, ...] made M`, `return a, ...`, `trap N` | the terminators |

#### Debug types

A named local's type is described once, `type tN KIND ...`, in the order
the debug information first needs it: the functions in the order defined,
their locals in order, each type numbered before the types it holds, as
the DWARF writer of decision 227 walks them, so that the translation,
walking the descriptions, makes the same entries in the same order:

```text
type t0 base "int" signed 8              -- a base type, shared by its name
type t1 str                              -- `&str`
type t2 struct "Point" 16 { "x" t0 @0, "y" t0 @8 }
type t3 tuple "(int, str)" 24 { 0 t0 @0, 1 t1 @8 }
type t4 pointer "ref Point" t2           -- a reference or a box
type t5 text "String" 24                 -- `String`, `Path`, `bytes`
type t6 list "list[int]" 24 "&[int]" t0
type t7 enum "Option[int]" 16 @0 { } { "Some" { "__0" t0 @8 }, "None" { } }
type t8 errset "{A, B}" 48 { 17 "A" t9, 18 "B" t10 }
type t11 alias t8                        -- the description of another
type t12 opaque "dyn Shape" 16           -- words shown as words
type t13 word "Scope"                    -- one word of the runtime's
```

### The Sisu backend

Step 2 of D-087: an x86-64 Linux debug backend written in Sisu, in stage 2
(`compiler/stage2`), compiles LIR to machine code in one pass over each
function, after one analysis pass, as TPDE compiles LLVM IR, and writes an
ELF relocatable object that the system linker links with the same runtime
library Cranelift's objects are linked with (decisions 306 to 312). It is
the debug profile's backend (decision 331): `sisu build --native` and
`sisu run --native` (with `--from-sir` and `--from-lir` too) lower the
program through stage 2, compile its LIR so and link the object with `cc`
as decision 199 does, unless `--backend=cranelift` (or
`SISU_BACKEND=cranelift`) asks for Cranelift, the oracle, which a release
build and the Rust lowering use unasked; `--backend=sisu` names it for a
release build, any other name is `D0123`, the Sisu backend with the Rust
lowering is `D0124` (it compiles LIR), a construct it does not compile is
the program's `D0103` (`x86: not compiled: WHAT (in FUNCTION)`), and any
other failure `D0125`. A debug build runs `sisu2 build FILE.sir --output
FILE.o`, which lowers the program and compiles its LIR in one process,
without the LIR's text between two (decision 334); `sisu2 x86 FILE.lir
--output FILE.o` compiles a LIR program on its own. Either compiles the
functions on the workers of `std.par` (decision 333), in batches whose
functions are placed in the file's order, so the object is the one a
single thread gives, byte for byte.
Its modules are `lirread.sisu` (LIR read item by item, as
`sisu-codegen/src/lir.rs` reads it, each value given its type),
`x64.sisu` (the instructions' encodings), `x86.sisu` (the analysis, the
frame, the register allocator and the code of each operation),
`dwarf.sisu` (the debug information), `elf.sisu` (the object file) and
`object.sisu` (the pass over the program: each datum placed and each
function compiled as it is read).

- **The frame** (decision 307): every function has a frame pointer
  (`push rbp; mov rbp, rsp; sub rsp, F`, `F` a multiple of 16), and every
  frame slot a fixed offset below it: 40 bytes for the callee-saved
  registers the allocator uses (each saved by the prologue only if it is
  used), LIR's slots in order, its variables, a home of 8 bytes for each
  value that needs one, and at the bottom the arguments that calls pass on
  the stack. The prologue and the one epilogue every `return` jumps to are
  written after the body, once the frame's size and the registers used
  are known.
- **Calls** follow the System V convention, of which Sisu's (decision 189:
  the result's and the arguments' addresses, the status byte returned) is
  an instance: integers in `rdi`, `rsi`, `rdx`, `rcx`, `r8` and `r9`,
  floats in `xmm0` to `xmm7`, the rest on the stack, the result in `rax`
  or `xmm0`.
- **Registers** (decision 308): the analysis finds the block that defines
  each value, whether another block uses it, and its last use in its own
  block. A value is computed in the scratch registers a call clobbers,
  then written to its home if another block uses it (a block's parameter
  is written by every edge into the block, a function's by the prologue),
  and kept in one of `rbx`, `r12` to `r15`, which a call preserves, while
  its block still uses it; when all five are taken, the value whose last
  use is furthest is spilled to its home. A block starts with every value
  in its home.
- **Checks and the status byte** (decision 309): the overflow and bounds
  checks are LIR's branches to cold blocks that call the runtime's panic
  entries (decision 270), compiled as any branch, an overflow flag from
  `seto`; a status byte is `al`, and a value narrower than 64 bits is
  tested and compared at its width, since its register's upper bits are
  unspecified; `trap` is `ud2`.
- **Probes and yield points** (decision 310) are LIR's own code, compiled
  as written; the probe reads `rsp` after the prologue, and the probe
  reserve C's `main` passes to the runtime is computed from the frames
  this backend lays out.
- **The object** (decision 311): `.text`, `.rodata`, `.data.rel.ro` (the
  data holding functions' addresses), their relocations, the symbol table
  (each function a symbol of its size, local but C's `main`) and
  `.note.GNU-stack`. Calls and addresses of the program's own functions
  are resolved by the backend; a call of the runtime or of C is
  `R_X86_64_PLT32`, the runtime's data is reached through the GOT
  (`R_X86_64_REX_GOTPCRELX`), the object's own through `R_X86_64_PC32`.
- **Debug information** (decision 312): DWARF 4 as decision 227 writes
  it, so far the line table (`dwarf.sisu`) from LIR's `loc` lines, the
  code before a function's first row being its declaration's line and the
  first row the prologue's end, and a unit with a subprogram for each
  function's code, so that gdb stops at `break area` and at
  `shapes.sisu:15` and names the Sisu functions and lines of a backtrace,
  unwinding through the frame pointers; the driver gives the backend the
  build's directory and the standard library's (`--comp-dir`, `--std`), as
  it gives the Cranelift path.

It compiles every operation of LIR but a jump with more than nine
arguments and a return of more than one value, which no lowering makes,
and every case of `native.list` runs from its objects with the
interpreter's output (the runner's `--native` mode; `--native-cranelift`
builds the same cases with Cranelift, so both backends meet the same
expectations). Stage 1, compiled by it from its 4 263 811 lines of LIR in
15 to 19 s on one thread of a loaded machine (about 4 ms for each thousand
lines, against Cranelift's 6 s on four cores), prints on the 400
conformance files tried what stage 1 compiled by Cranelift prints, in 1.5
times the time. port-debt.md ("The backend in Sisu") lists what it leaves.

**On threads** (decisions 332 to 334): the words the generated code
reaches by symbol (the count of nested calls, the budget, the stack
limits and the site) are thread-local, LIR's `data dN import tls NAME` and
`tls_value dN`, so that Sisu code runs on several threads at once; the
native backend compiles std.par's `map` as a call of the runtime's
parallel map, whose bodies run on a pool of workers apart from the task
scheduler, each result written at its element's place; and the backend's
pass hands each batch of functions to that map, each function's lines
read and compiled on a worker. On stage 1 on four cores, the backend
compiles its LIR file in 9.4 s (18.3 s on one worker), and `sisu2 build`
lowers it in 6.7 s and compiles it in 6.0 s, where Cranelift compiles the
LIR in 3.7 s.

**The fixed point** (decision 330; D-087's criterion for step 2): the
backend, built by itself, rebuilds itself to the same bytes.
`compiler/stage2/bootstrap.sh` builds sisu2-C, stage 2 as the Cranelift
path builds it (the seed: stage 1's front end, the Rust lowering,
Cranelift); sisu2-C lowers and compiles the SIR of stage 2's package and
of stage 1's with the Sisu backend into sisu2-A and stage1-A, and the
runtime in Sisu's as a library into rt-A.o, which they are linked with
(decision 353); stage1-A prints the three packages' SIR again, and sisu2-A
lowers and compiles it into rt-B.o, sisu2-B and stage1-B. Each B is its A
byte for byte, and each SIR stage1-A prints is the one `sisu sir` prints,
the runtime's the committed seed:

```text
  rt-A.o          582664 bytes  sha256 b6d236430bb4332b...  built by sisu2-C
  sisu2-A       27391840 bytes  sha256 e3636c7e02b0233f...  built by sisu2-C, with rt-A.o (6.0 s)
  stage1-A      53156784 bytes  sha256 bcf0f8e759ec3a57...  built by sisu2-C, with rt-A.o (21.7 s)
  rt-B.o          582664 bytes  sha256 b6d236430bb4332b...  built by sisu2-A
  sisu2-B       27391840 bytes  sha256 e3636c7e02b0233f...  built by sisu2-A, with rt-B.o (12.4 s)
  stage1-B      53156784 bytes  sha256 bcf0f8e759ec3a57...  built by sisu2-A, with rt-B.o (46.1 s)
```

The script takes six to eight minutes on four cores, most of it stage 1
printing the SIR of the three packages; the driver builds with the stage 2
that `SISU_STAGE2` names, as it is, instead of `target/stage2/sisu2`. CI
runs it on every push.

### Coverage

`conformance/native.list` names the cases the backend builds and runs:
216 of them, every case of the suite that runs a program but the
refusals of the checked memory model, which are the interpreter's (the
others are diagnostics, `sisu test` files, and the tools' cases:
`--watch`, the REPL, worksheets, tokens, SIR and documentation). Among them are every
case of `concurrency/` that runs (tasks, scopes, cancellation, channels,
`select`, races, generators and their closing, `log`, mutexes across
tasks, the deadlock report, a task's stack overflow), all 50 of `run/`
that have a `main` (`run/testing` is a test file without one), the 38 of
`ownership/` that run (with `Mutex`, `Shared`, `Weak`, `Cell` and
`Arena`), and the 11 of `capabilities/` (fakes, attenuations, caretakers,
interceptors, `cap.lend`, the test clock, the host's processes, an
in-process network and the drops of objects with their handles' last
copies), with `replay/network` on the host's real network.
`sisu-conformance --native conformance/native.list` builds each with
`sisu build --native`, runs the executable as the runner runs `sisu` (its
arguments, standard input, an environment holding only `SISU_RT_ALLOCS=1`),
compares its standard output, exit status and standard error with the
files the interpreter meets, and requires the runtime's report to say
that nothing it allocated is live at exit (decision 226); `cargo test`
does the same (`crates/sisu-driver/tests/native.rs`), from source, from
serialised SIR, with the release profile (every contract checked,
decision 232), from the LIR stage 2 lowers the SIR to (each executable
byte for byte the one from the SIR, decision 284), and with Cranelift,
the oracle of the Sisu backend that a debug build uses (`--native-cranelift`,
decision 331), and CI runs them on every push. `--native-survey LIST` tries every
runnable case the list leaves out and says which would pass, or the first
construct that stops each.

The interpreter's modes have no native counterpart: the ownership
monitor, the counterexample mode, the capability log and replay, hot
replacement, and `sisu test`. port-debt.md ("Stage 2") lists these and
what the backend still leaves out.

### Compile time and run time

Measured on one machine (4 cores, shared, so times vary by a third from
one run to the next), with the release `sisu`, the best of runs taken in
turn, medians in brackets. Round 2 put the costliest steps on threads
(decision 219): stage 1 as it was then (46 293 lines of Sisu, with the
6 434 of the standard library) built natively in 4.4 s instead of 13.3 s.
Round 3 took on what still ran on one thread (decision 228). Stage 1 now
(`compiler/stage1`, 52 441 lines) builds in 3.9 s (4.1 s, nine runs),
against 4.3 s (4.7 s) with that round's threads and cuts turned off and
4.0 s (4.4 s) with round 2's compiler, which wrote no debug information.
Name resolution takes 0.13 s against 0.18 s, lowering 0.24 s against
0.59 s, the type checker 0.16 s and the ownership checker 1.3 s, on
threads already, and code generation 1.4 s for 2 101 function instances
and 213 660 SIR statements, of which writing the IR is 0.61 s against
0.93 s; the link takes 0.08 s, and the SIR and the 20 MB executable are
the same bytes either way. Code generation is bounded by Cranelift's compiling,
3.2 s of the 11 s of processor time a build spends: the IR is written
before the threads have compiled what they were given. That is 0.066 s
for each thousand lines, so a clean debug build of 100 kLOC would take
under 7 s, within 10's target of 10 s. A small program builds in 0.12
to 0.15 s, of which code generation is 1 to 2 ms and the link 60 to 85
ms; the rest is the front end, which `sisu run` spends as well. Round 10
measured stage 1 again, with its formatter (64 049 lines): a clean
native build takes 5.9 to 6.4 s on the same machine, 17 to 19 s of
processor time, of which the ownership checker is 2.0 to 2.2 s, on
threads, and code generation about 2 s, the IR written in 0.9 to 1.1 s
while the threads compile and a second more for them to finish
(port-debt.md, "Some steps of a build still run on one thread").

The executables run 10 to 30 times as fast as the interpreter. On stage
1's commands, from `compiler/stage1`, with byte-identical output, as
round 2 measured them on stage 1 as it was then (decisions 232 and 233
have this round's times of some):

| command | interpreter | native |
|---|---:|---:|
| `tokens ../std/prelude.sisu` | 5.3 s | 0.09 s |
| `parse --tree ../std/prelude.sisu` | 7.3 s | 0.17 s |
| `ast ../std/prelude.sisu` | 8.5 s | 0.19 s |
| `types ../std/prelude.sisu` | 30.0 s | 0.93 s |
| `batch` of the 405 conformance files | 21.8 s | 1.19 s |
| `names-batch` of the 379 one-module programs | 19.6 s | 0.62 s |
| `types-batch` of the same | 116.8 s | 4.3 s |

The interpreter's times include the 4 s of checking stage 1 that every
`sisu run` spends. Without them, `batch`'s lexing took 2.7 s against
0.11 s, its parsing 4.2 s against 0.13 s, and `types-batch` about 113 s
against 4.3 s. A conformance case's executable runs in 4 to 6 ms, most of
it starting the process and the program's thread.

### Stage 1, compiled

Stage 1 compiles natively without a change (its sources, built from a
copy so that the executable is not written into the package), and the
executable gives the interpreter's output byte for byte on every command
`check.sh` drives: `batch` over the 455 files of the conformance suite,
the standard library, the proposal's examples and stage 1's own cases
(tokens with trivia, trees with and without trivia, reprints and ASTs, and
their diagnostics), `names-batch` and `types-batch` over the 379 programs
of one module. Its tests (`sisu test`) are not compiled: the backend
builds programs, not test runs.

## How to add a construct

1. **Tokens**: a new keyword or punctuation goes in
   `sisu-syntax/src/kinds.rs` (the keyword table and the punctuation
   table drive the scanner); a continuation operator also goes in
   `CONTINUATION_STARTERS`.
2. **Grammar**: a parser function for the rule, with its line in the
   grammar above and in the module comment, and recovery that returns an
   `ERROR` node rather than giving up. Add a parser test, including the
   construct with a mistake in it.
3. **AST**: the node in `ast/mod.rs` and its builder in `ast/build.rs`,
   total over trees with errors.
4. **Names**: if it declares something, `sisu-resolve/src/collect.rs`.
5. **Types**: the THIR node in `thir.rs`, its checking in `check.rs`,
   `expr.rs`, `call.rs` or `pat.rs`, and its cases in `zonk.rs`, the
   purity walk and exhaustiveness if it has patterns. Every new error is a
   catalog entry first; check that the construct typed with an error
   inside yields no second diagnostic.
6. **Lowering**: in `sisu-lower`, keeping the invariant that initialising
   or dropping a value that needs drop ends its block. If SIR needs a new
   form, change `sisu-sir/src/lib.rs`, `write.rs`, `read.rs` and
   `validate.rs` together, bump `VERSION` if old text no longer reads, and
   document the form here.
7. **Ownership**: a new statement or rvalue needs its operands and
   destination in `sisu-borrowck/src/init.rs` (`operands`, `dest`), its
   accesses and loans in `loans.rs`, and, if it can hold or store a loan,
   its transfer; a new intrinsic that keeps an argument takes it with
   `own` in its prelude signature. Under the monitor, every access to a
   place goes through `mon_access` with the reference it uses. Add an
   accepted case (which the suite also runs under the monitor) and a
   rejected one with its `.defer`.
8. **Running**: library behaviour is Sisu in `compiler/std/` where it can
   be, over intrinsics that are primitives; a new intrinsic is
   implemented in `sisu-interp` and gets its contract as a `##` comment in
   `compiler/std/*.sisu` (or in the table above if only lowering uses
   it), with `@keeps` for the arguments it keeps. The native backend
   needs it too: a name in `sisu-rt/src/abi.rs` (`intrinsics!`) and its
   implementation in `sisu-rt/src/intrinsics.rs`, with the interpreter's
   results and messages; until then a program that calls it is `D0103`
   under `--native` (section "Stage 2").
9. **Cases**: conformance cases under `conformance/cases/` for the
   working construct, its diagnostics, and a broken use; bless them,
   review every expected file, and run the suite.

## Decisions the documents did not make

Where the proposal was silent, stage 0 took the simplest reading. Each is
open to revision; the conformance cases pin the current choice.

**Syntax and diagnostics**

1. Columns in positions count Unicode scalar values from the start of the
   line, 1-based, so a position means the same to any implementation.
2. After `def`, a keyword is a name (`def or(...)` in the prelude).
3. A bracket left open is closed by the lexer at a line that starts with a
   declaration or statement keyword at or left of the bracket line's
   indentation, or at the `:` of a block header followed by an indented
   line; a dedent matching no level becomes a phantom level.
4. One syntax diagnostic per line, ranked: lexical, then bracket, then
   grammar errors; after the first syntax error, misplaced statements and
   unexpected indented blocks are not reported; name and type diagnostics
   on a line with a syntax error are dropped; a function whose header or
   body did not parse checks with an unknown signature and quiet names.
5. A misspelt name suggests the closest name in scope (an edit distance,
   counting a swap of neighbours as one edit, of at most a third of its
   length) as a fix.
6. Diagnostic templates avoid number agreement instead of carrying plural
   rules.

**Names and types**

7. A variant is looked up in the expected type, then among declarations,
   then in the current module, then everything visible; a remaining tie
   is `E0211`.
8. The alternatives of `|` bind the same names, of the same types
   (decision 182; until then they could bind none, `E0385`).
9. `print` and `print_err` take exactly one argument (the prelude signature
   `print(x)`); several values print as a tuple.
10. `f64` is not `Ord` (NaN), so `max(2.5, 1.5)` is an error and floats
    compare with `f64.cmp`, which panics on NaN.
11. The `IoError` variants are `NotFound(path)`, `PermissionDenied(path)`,
    `AlreadyExists(path)`, `InvalidData(path)` (not UTF-8),
    `Interrupted`, `Revoked` and `Other(message: String)`.
12. `cli.parse[T]` (`std.x.cli`, a template over reflection): each field
    `f` of type `int`, `f64`, `String`, `Path` or `bool` is the option
    `--f`, with `-` for `_`, given as `--f value` or `--f=value`; a `bool`
    is a flag (or `--f=true`); the one `list` field collects the
    positional arguments; `--` ends the options; a field with a default
    may be omitted. Its errors are `UnknownOption`, `MissingValue`,
    `InvalidValue`, `MissingArgument` and `UnexpectedArgument`, naming the
    option as written: unknown options and missing values first, as the
    arguments are read, then invalid values and missing options in field
    order. A `str` field is a compile error (use `String`), since the
    parsed struct owns its text. The struct is built once with every
    field, so its invariant is checked on what was parsed.
13. `()` is a pattern matching the unit value.
14. Inferred error sets are found by iterating every `T !` function to a
    fixpoint, at most 32 rounds.

**Semantics**

15. `d[k] = v` inserts a missing key; reading a missing key panics.
16. `xs[lo:hi]` copies, with the bounds clamped to the length; negative
    bounds panic (`^` counts from the end, so a negative index is never
    meaningful).
17. A shift amount outside 0..63 (0 to the width less one, for a sized
    integer) panics; bits shifted out of `<<` are lost rather than
    trapping, since shifts are bit operations, not arithmetic.
18. Float `/` by zero follows IEEE 754 (an infinity or NaN); `%` on floats
    has the sign of the divisor, like the integer `%`: the exact remainder
    (IEEE 754's `fmod`) plus the divisor when their signs differ, the one
    rounding (Python's rule; `1e17 % 3.0` is `1.0`).
19. Collections drop their elements first to last; a dict drops each
    entry's value then its key; tuples drop in reverse like fields.
20. A panic's report is printed after unwinding finishes, so the `Drop`
    output of the unwound frames comes before it.
21. A panic while unwinding prints both messages and exits 134.
22. `main` returning an error prints `error:` with the error's `Debug`
    form and its return trace, one `at file:line:col` per line, oldest
    first, and exits 1.
23. `e.trace()` is those lines joined with line breaks; the location is
    where each `?` the error passed through was written.
24. A struct or enum displays only through its own (or a delegated)
    `Display` impl (`E0347` otherwise); collections, options and results
    display as their `Debug` form.
25. Floats display as the shortest text that reads back as the same
    value, always with a `.` or an exponent.
26. In a format spec, a precision without a type is that many decimals
    (`{x:.2}` is `3.14`), as in Rust; Python's general format needs `g`,
    which stage 0 does not have. A spec ending in `?` asks for `Debug`.
27. Float `sum` adds pairwise by halves (the "fixed shape" of D-074).
28. `Option.expect` and `Result.expect` with an empty message panic with
    "called `expect` on `None`" (or "on an error" and the error); with a
    message, the message, the error and its trace.
29. A key function's result that refers to the item passed to it is
    copied before the item is released (`max_by(it)` on references).
30. Exit statuses: 0, 1 (an error returned by `main`, a failed test,
    compile errors), 2 (usage), 3 (`sisu test`: no test failed and one is
    blocked by a typed hole, decision 380), 70 (internal compiler error),
    101 (panic), 134 (abort, including an ownership violation under the
    monitor).
31. Sisu frames are limited to 100 000; beyond that is a panic, not a
    crash (decision 234: the stack overflow names the function, natively
    as in the interpreter).
32. A typed hole run by `sisu run` panics; in `sisu test` the test is
    reported as blocked, and blocked tests do not fail the run, which
    exits with 3 when none failed (decision 380).

**SIR**

33. SIR v0's statements are Assign, Call, Drop and StorageDead, and its
    terminators Goto, Switch, Return, Resume and Unreachable; the mapping
    to 18's list is in the SIR section.
34. Matching a union of error sets tests the declaration with
    `error.is[D]` before the variant index, so an error value needs to
    know its declaration at run time.
35. Error return trace locations are strings computed at lowering and
    embedded in SIR, which makes traces independent of debug information.
36. SIR records what call sites need to know about a function's borrows
    (`from`, `inner`), its purity, and, for intrinsics, which arguments
    they keep (`own`), so the ownership checker never reads THIR or the
    prelude; locations are ranges, not points.

**Ownership**

37. The ownership checker runs on SIR, function by function, after
    lowering, and only on a program without front-end errors; lowering
    emits every drop unconditionally and the checker places them.
38. Borrows are non-lexical: a loan is live where a local holding it is
    live. Dropping a value whose drop glue runs a user `Drop` counts as a
    use of everything it borrows.
39. Two-phase borrows are only the `mut` borrows into unnamed temporaries
    (a call's receiver and arguments); a `ref mut` bound to a name is
    active at once.
40. A copy of a `ref mut` is a reborrow, whose loan is a child of the
    copied reference's.
41. The static rules do not tell list elements or dict entries apart;
    the monitor does.
42. `ref str` is `str`.
43. A parameter is borrowed when it is `mut`, or passed without `own` and
    either not `Copy` or able to hold a loan. A result that can hold a
    loan comes from its `from` clause, else the only borrowed parameter,
    else a borrowed `self`; otherwise `from` is required. Every `own`
    parameter that can hold a loan is a source too.
44. A result that can only view text, from an argument passed by
    reference that owns no text outside references, borrows only what
    the argument holds (`inner`), not the argument.
45. Closures capture by reference and cannot escape the frame of what
    they capture; an `own` closure moves its captures in (copies the
    `Copy` ones). A closure's result may borrow from any of its
    arguments and its environment.
46. A loan of a `mut` parameter, or of anything reached through one, is
    memory of the caller: only that parameter's own loan may be stored
    there. An intrinsic keeps only the arguments it takes with `own`; a
    dict entry points into its dict.
47. A must-consume value ends in its type's own methods (functions named
    `Type.method`); a consuming loop discharges its collection on every
    path where `Iter.next` returned `None`; intrinsics may take one with
    `own` or `mut` only at the arguments their contract marks `@keeps`; a
    generic function may take one for a type parameter only if its body,
    instantiated, drops none.
48. A drop flag is a `bool` local named `flag`, set after each
    initialisation and cleared before each move; a flagged drop clears its
    own flag. A partly moved value drops its remaining parts in reverse
    field order; a value whose type implements `Drop` is never split.
49. A conflict is reported at the access, labelled at the loan and at the
    next use of the loan; the message's end line is the last line where
    the loan is live. A borrow that outlives its place is reported once,
    and not again for the conflicts its loan meets afterwards.
50. Use before assignment and a second assignment to a `let` keep their
    type-checker codes (`E0366`, `E0367`) now that the dataflow on SIR
    reports them.
51. `a?.f` through a reference borrows a field that is not `Copy`; a
    `String` field becomes a `str` view.
52. A `for` loop's own statements are located at its head (`for x in
    xs`), and iterating a reference borrows its referent.
53. The monitor ends a conflicting loan at the access and reports only a
    later use of it, so it agrees with the non-lexical rule; a violation
    exits 134 with the SIR of the function. `sisu test` runs under the
    monitor unless `--no-monitor` is given; `sisu run` does with
    `--monitor`.
54. The counterexample mode defers the ownership family, including the
    rules about declarations, and not purity or a second assignment to a
    `let`; its outcome is one of conflict reached, no conflict with a
    candidate false positive, or inconclusive, an untested path.

**Traits and generics**

55. Type arguments are run-time values: every generic SIR function
    receives them, and a trait call whose `Self` is not known when
    lowering is dispatched at run time by matching the impls' headers
    against the concrete type arguments; nothing is monomorphised.
56. Impl selection matches headers only; `where` clauses never choose
    between impls (coherence leaves at most one header that matches), and
    are checked once the impl is chosen.
57. For the orphan rule, std is one package and each package of the
    program another; the root package may implement std's traits for
    std's types.
58. A method call looks for the receiver type's own method first, then
    the traits it implements or (for a type parameter) is bounded by;
    a name in more than one of those traits is an error, resolved by
    calling `Trait.method(x)`.
59. A borrowed parameter whose type is a type parameter is passed by
    reference unless the parameter is bounded by `Copy`, so one body
    serves `Copy` and non-`Copy` arguments; dispatch to an impl that takes
    a `Copy` type by value reads it through the reference.
60. An impl's method inherits `pure` from the trait's declaration.
61. Arithmetic operators on numbers are the language's own operations;
    on other types they call `Add`, `Sub`, `Mul`, `Div`, `Rem` and `Neg`,
    whose result is the impl's `Output` (by default `Self`); a bound
    fixes a defaulted associated type to its default.
62. `FromLiteral[L]` converts a literal of type `L` where a type that
    implements it is expected; std's `String` and `Path` do for `str`.
63. The derived and std impls of the language's traits compute the
    host's structural operations, so lowering calls the host when every
    bound is met structurally; `dict` keys and `set` elements are hashed
    structurally, so a key type with its own `Eq` or `Hash` impl is
    rejected (`E0398`) rather than hashed inconsistently.
64. `Iterator.next` and the iterator adapters are not `pure` (03); an
    impl's `next` may be, and a `pure` function may loop over a host
    iterator.
65. `Drop` for a generic type covers all of it, with the type's
    parameters in declaration order and no bound the type does not
    declare (Rust accepts any order of distinct parameters; stage 0
    rejects that, an over-rejection).
66. Every struct and enum without its own `Debug` gets the derived one.

**The library types of 04**

67. `Shared`'s own functions take it as an argument (`Shared.count(s)`,
    `Shared.downgrade(s)`), since the value's fields and methods are
    reached through it; `.copy()` makes another owner. Whether its count
    is atomic is decided per construction site by the escape analysis.
68. Locking a `Mutex` another task holds waits, and cancellation does not
    interrupt the wait (`lock` cannot fail); locking one the task already
    holds panics, since it would wait forever. When unwinding drops a
    guard, the value's type's `invariant` is checked (D-077): if it fails
    the mutex is poisoned, and every later `lock` panics; a type without
    an invariant unlocks.
69. `Key[T]` is a `Copy` struct of a slot and a generation; a stale key
    gives `None` from `get` and panics when indexed.
70. `@keeps` in a contract is the must-consume rule for an intrinsic.

**Drops**

71. Drop glue is a SIR function per declared type that needs one; the
    drops of host-backed types are part of their contracts; a drop
    statement's place gives the glue its type arguments, and a value
    dropped inside an intrinsic is dropped by what it is.
72. A `Drop` that panics still has its value's fields dropped, in reverse
    order, while the panic unwinds; a second panic aborts.
73. The glue's last drop runs in its caller's frame, so a chain of
    values takes constant depth; nothing but the frame limit can tell.

**Compile-time evaluation**

74. `comptime` is a prefix operator on the operand after it; before a
    condition (`if comptime`) it covers the whole condition.
75. Purity is not required: the evaluator refuses the world as it runs
    (any use of the host is an error), so a function that does no I/O can
    run at compile time whether or not it is declared `pure`.
76. The budget is 1 000 000 SIR steps unless `@comptime_budget(steps=N)`
    on the function or constant holding the expression says otherwise;
    exhausting it is an error (stage 0 has no lint levels to make it
    deny-by-default and nothing else).
77. A `comptime` expression that uses another's value runs after it, each
    with its own budget; one that uses a failed one is not reported.
78. A value made at compile time is kept as statements that build it
    from literals; one with no literal form is an error.
79. A `comptime` expression cannot depend on the function's type
    parameters, whose arguments are known only at a call.
80. The root module's constants are computed even when nothing uses
    them.
81. A derive template unfolds to source text: its compile-time parts are
    evaluated, the rest is copied with the compile-time values as
    literals, and the impl is checked as written code. Arms are made with
    `comptime for` and `comptime if` among a `match`'s arms, and `case
    V(..)` for a variant bound at compile time binds its fields as
    `__x_0`, ... for a `match x`. A positional variant field's `name` is
    its position.

**Formatting**

82. The format-spec mini-language is Sisu (`Spec`); the host gives
    digits only. A spec is checked where it is written (`E0399`).
83. A type's `Display` that writes a leaf at depth 0 passes the whole
    spec to it (`{p:>6}` of a `Display` writing `"P"` and then a `7` is
    `P     7`), as the protocol says.

**Tooling**

84. `sisu audit --ownership` counts tokens, not resolved uses, and a name
    the file declares itself is not counted. It counts `Cell` (T1.2's
    density includes it) as well as 18's `Shared`, `Weak`, arena handles,
    `.copy()` and `unsafe`, with `Arena` and `Key` apart, and its `lines`
    are the lines that hold code, the denominator of "per 100 final
    lines".

**Tasks** (06; 18, Phase 6)

85. The yield points are loop back edges, calls of Sisu functions,
    spawns and `task.yield_now`; a task runs 1 to 8 of them, drawn from
    the seeded generator, before the scheduler may switch, and the next
    task is drawn among the ready ones. Seed 0 is the default.
86. Every blocking operation but `lock` checks cancellation before it
    waits, so a cancelled task returns `Err(Cancelled)` at its next one
    even when it would not have waited; `lock` waits uncancellably,
    since it cannot fail.
87. A task's `Err(Cancelled)` after its scope or task was cancelled is
    not a failure: it cancels nothing and is not the scope's result.
88. In a `pure` scope, a failure cancels only the tasks spawned after the
    failing one, and the unjoined failure reported is the lowest-index
    one; the body joining later tasks first can still observe them in
    time order (port-debt.md).
89. A failure nobody joined that the scope's result type cannot carry
    panics (`R0079`), and an unjoined panic re-raises unless the result
    carries `TaskError`; results nobody joined are dropped at the end of
    the scope.
90. The closures of `scope`, `race`, `shield` and `spawn` must be written
    at the call (`E0425`); a task spawned outside its scope's body
    borrows nothing (`E0429`).
91. A task's `Copy` captures that it does not change are copied, so a
    loop variable given to a task is the iteration's value.
92. A channel's capacity is at least one; `select` among ready arms
    chooses by the seeded generator; a cancelled `select` takes a ready
    cancellation arm, else the first arm that reports `Cancelled`.
93. Every virtual clock advances by the smallest wait among the virtual
    timers when every task waits; a real timer is waited for only when
    no virtual one is pending.
94. A function or closure value is neither `Send` nor `Sync`, since its
    captures are not in its type; a server's handler is a trait object
    or a generic parameter instead (`http.Handler`).
95. A generator dropped before its end is abandoned with its iterator;
    its frames are dropped without running to completion (decision 236:
    it is closed, its drops and `defer`s running).
96. `log` records carry the task's path, not a span identifier: the
    trace context of D-075 is that path until Phase 8's tooling.

**Capabilities** (06; 11; 17)

97. Each capability is a builtin `Copy` handle type on an object of a
    type implementing its operations trait (`Fs`/`FsOps`, ...); the
    proposal calls the trait "a trait" without a name. The handle
    forwards by dispatch (`cap.forward`), and a value implementing the
    trait becomes a handle where one is expected (the handle takes it
    over; a `Copy` one is copied).
98. An object must own what it holds, not be must-consume, and be `Send`
    and `Sync`, since a handle can be stored and can reach every task;
    nothing frees it (decision 237: it goes with the handle's last copy).
99. `Lent[T]` implements the capability's trait but never becomes a
    handle: a parameter that accepts the lent form is generic
    (`[F: FsOps]`), not of the handle's type, which could be stored.
100. `Cap[T]` is a `Copy` handle on a shared value, the building block of
     fakes and caretakers written in Sisu; its value is never dropped.
101. `System`'s fields are `fs`, `net`, `clock`, `rand`, `env`, `proc`,
     in the proposal's order, then `stdin`: reading standard input is a
     capability (`Stdin`, `read_line`), so that a capability log records
     and a replay answers it; standard output and error stay ambient
     (Q-5).
102. A test takes nothing or one `System` (`E0702`), and one that takes
     a `System` receives the host's, as the proposal's text says, except
     in the `sim` profile, which gives it `System.sim(seed)`; a test
     that wants a whole fake set makes it with `System.fake(seed)`, so no
     test is handed fakes implicitly outside `--sim`.
103. A run has one test clock (`Clock.fake`), which `Clock.frozen_at(t)`
     sets to `t` and every `TestClock` reads, and which stamps the writes
     of in-memory file systems; it starts at the epoch and is virtual.
104. `restrict_to` reports an error with the path the caller gave, and
     an attenuation of an attenuation narrows again; `https_only` is a
     restriction to port 443, since stage 0 has no TLS.
105. A revoked caretaker answers `exists` with `False` and fails every
     other operation with `Revoked`; `Revoked` joins `IoError`,
     `NetError` and `ProcError`.
106. `Wrap[T, I]` takes the interceptor as a type parameter rather than a
     `Box[dyn Interceptor]`; the interceptor sees each call's arguments
     and outcome rendered with `Debug`.
107. `@internal` marks a std function only std may call (`E0701`);
     `System.assemble` is the one that builds a `System`.
108. `Env.args()` gives `list[str]` (views of the object's text), as
     `wordcount` needs; `Proc.run` runs a program to its end with no
     input; `Rand.bits()` is the one primitive, and `between`, `unit`,
     `coin` and `shuffle` are Sisu over it.
109. `HttpError` is a prelude error set, since the proposal's examples
     name it bare after `import http`; `std.time` and `std.net` re-export
     the prelude's names they document (`from std.time import DateTime`).
110. A host socket that would block makes its task wait a millisecond on
     the host's clock and try again: a poll, not an event loop.

**Numbers and bytes** (03)

111. The sized integers are builtin types; a `u64` is a value of its own
     kind and the others `int` values in range; a literal takes the
     expected sized type or its suffix's, checked at compile time.
112. Two integer operands of different types widen the narrower when
     that loses nothing; comparisons do not widen.
113. `Into[T]` has an impl for each lossless widening, and a method of
     one generic trait implemented for several arguments infers them
     (`b.into()` where a `u32` is expected).
114. `bytes` is immutable and shared by its copies; `b[i:j]` copies;
     `Slice[T]` is not `Copy`.
115. `Iter.fold` is `pure` with a `pure` function, so `pure` code can
     fold (03, "Function types"); `Iterator.fold` keeps any function.
116. `DateTime` holds nanoseconds since the epoch (the years 1678 to
     2261), parses RFC 3339 in Sisu, and displays with the fraction of a
     second only when there is one.

**Contracts** (07)

117. A clause only reads, so a build may leave it out and a failed one can
     be evaluated again part by part to show each part's value; `old(e)`
     is a copy of `e` made on entry, of every `old` of the function,
     whether or not the exit that checks it is reached.
118. A struct's invariant is checked after each construction and when
     each `mut self` method of the type returns (by any exit); a field of
     such a struct is changed only in the type's own methods (`E0806`),
     and in one the invariant may be broken until it returns.
119. The levels are `off`, `preconditions` and `all`; `sisu run` checks
     `all` unless told otherwise, and `sisu test` always does.
120. A refinement of a refinement keeps the chain: `try_from` and a
     constant check every predicate from the root's up, and the error
     names the first that fails; a coercion or a method looks through one
     level at a time.
121. A refinement has its base's reading impls (`Display`, `Debug`, `Eq`,
     `Ord`, `Hash`, `Len`), not the ones that make or change values
     (`Add`, `Iterator`), and no impl of its own (`E0813`).

**Property tests and testing** (17)

122. A case is a sequence of choices; an integer is one choice; shrinking
     deletes runs of choices and lowers each one by binary search, at
     most 1000 attempts; the reported case is the smallest found.
123. A case has a budget of statements (`Gen.steps`: 1 000 000, or
     100 000 for the tests the compiler writes), not a deadline, so a
     run is deterministic; past it the case fails, or, for a quiet
     generator (contract and `@refines` tests), is refused.
124. Generators exist only for the types a program generates; a
     generic function's type parameter is `int` in its contract test,
     when `int` meets every bound, else the function has none.
125. `@refines` compares with `==`, or with `within_ulps` for an `f64`
     result given `tolerance=Ulps(n)`; both functions are free
     functions of the module with the same signature.
126. The tests that take no `System` run on threads, those that take the
     real one after them, one at a time; every test's output is printed,
     held per test and written in declaration order before the results
     (17's "shown on failure" is not adopted yet: stage 0 prints it all).
127. A case of a parametrised test is named by its element's source text
     (`name ("4x", 1)`); over a list not written out, the test is one.
128. A missing snapshot fails like a changed one; only `--update`
     writes `PROGRAM.snap`, keeping the snapshots of tests that did not
     run and dropping those a test that ran no longer takes.
129. A transcript's `$` line drops the program's name; its `>` lines
     are input only before the run's first output line; ` (exit N)`
     ends the last line of a run that fails, or is a line of its own;
     transcripts run only when no test is filtered out.
130. `sisu bench` runs each benchmark once with `--iterations N` (100)
     and reports the mean time of an iteration on the host's clock, with
     no statistics yet.
131. A fake's methods return their queued results in order and the last
     one again; one with nothing queued panics; a method that returns
     nothing needs nothing queued. Its log writes each argument with
     `Debug`.
132. `--filter` (a part of the name), `--tag` and `--skip-tag` may each
     be given more than once: a test runs when one filter and one tag
     match and no skipped tag does.

**Replay and simulation** (17; N-04)

133. The capability log is text, version 1, with the data the program
     wrote logged by its hash; a replay checks the program's output
     against the log too, and stops at the first difference.
134. A token is one word of `-`-separated fields read from the end, so a
     toolchain version may hold a `-`; a test that takes the real
     `System` has none.
135. PCT draws `depth - 1` change points among the first 1000 yield
     points; the exhaustive scheduler's choice 0 continues the running
     task, and its search stops at 10 000 schedules.
136. Faults go only to the simulation's own objects (`cap.SimFs`,
     `cap.SimNetwork`, `sim.SimNet`), never to a program's fakes, and are
     always errors of the operation's declared set other than
     `TaskError`; a fault's `path` is the call's path argument, else
     `(simulated)`.
137. `cap.SimFs` keeps a write pending until `sync(path)`; a crash keeps
     only what was synced, so a file whose name was never synced
     vanishes whole.

**Reflection** (08)

138. A template is a generic free function that reflects; it is made per
     list of type arguments by resolving the program again with the
     checker's requests, to a fixpoint (32 rounds, 256 instances a round);
     a generic function that calls one with its own parameters becomes
     one.
139. `reflect.construct(T, {...})` and `reflect.default_of(f)` build a
     struct from its fields and give a field's default: the proposal
     says reflection produces values for expressions, not how a value is
     built from fields; this is the smallest form that builds each struct
     once (invariant checked) and evaluates a default only when used.
140. A comprehension over compile-time lists is unrolled, and `if
     comptime c: a else: b` picks its branch, in templates.
141. A type of the program with the name of a prelude type is `main.NAME`
     in SIR, since the run time knows the prelude's types by name.
142. `embed` reads a file in the program's directory only, as text, when
     the program is checked; `target.runtime` is `interpreter`.
143. The `?`s in the standard library and in instances of its templates
     are not part of an error's return trace, as its frames are not part
     of a panic's.
144. The interceptor and the fake of a program's trait are written only
     for a module whose text names `Wrap` or `Fake`; the interceptor
     forwards every method, provided ones too, so the inner value's own
     runs.
145. `Option.unwrap`, `Result.unwrap`, `Iterator.nth`, `str.parse_float`
     (decimal syntax only: no `inf` or `nan`) and `within_ulps` join the
     prelude, for the examples that use them.

**Modules and packages** (09)

146. A file `src/net.sisu` and a directory `src/net/` are one module: the
     file's items and the directory's files and directories as its
     children. The root module takes the package's name; a single file's
     module has no path, so nothing imports it.
147. A module is reachable by its path whatever its items' visibility; a
     name an import bound is visible through the module only when the
     import is `pub` (a re-export); `pub(package)` reaches every module
     of the same package, and std's items are all public.
148. A package may not take the name of a std module (`json`, `cli`,
     `time`, ...), so `import json` means one thing.
149. `sisu.lock` lists every package the build resolved, sorted, each
     with every non-dev dependency, active on this target or not, and a
     `path+DIR` source relative to the lockfile, so it is the same on
     every target and machine; it is rewritten without a message when
     the manifests imply another, and `--locked` makes that an error.
     Only the root package's dev-dependencies are built.
150. A dependency's `when` is a structured condition over the target's
     facts `os`, `arch` and `runtime` (data, D-063); an unknown manifest
     key is a warning, so a manifest written for a later toolchain still
     builds.
151. A script's top-level statements are its `main` (`E0261` if it
     declares one too), and its inline metadata, between `# /// script`
     and `# ///`, is a manifest whose paths are relative to the script.

**Tools** (10)

152. The formatter writes `T?` for `Option[T]` (in impl headers too:
     `impl[T] T?`) unless `T` is optional, a result or a function type,
     or the option is behind `ref`, where `?` would read otherwise; it
     writes `T ! E` for `Result[T, E]` when `E` names an error set where
     the file is (the syntax does not show it, so the driver resolves the
     file first) and `T` is not a result or a function type and the
     result is not behind `ref` or optional.
153. An import is unused, and `sisu fmt` removes it, when the file spells
     its name nowhere outside the imports, not even in a doc comment (whose
     examples are tests); that needs no name resolution, since a method is
     found through every trait a type implements, imported or not.
154. A comment at the end of a line does not count toward the line's
     width, so adding a comment never moves code; `fmt --check` prints
     the changed lines with two lines of context.
155. A task adopts a new epoch only when it starts and at
     `live.boundary()`; a changed signature, layout, impl or entry
     restarts `main` and drops the old run's values without their
     `Drop`s; a watched run ends when its program does.
156. A save under `--watch` is an `edit` record of the capability log, at
     the poll that saw it, so a replay recompiles the same texts at the
     same points.
157. The REPL runs the whole session again for each input against the
     log of the run before it, instead of keeping a heap: `:back N` is
     exact, and a session is deterministic under its seed. A panic names
     its line in the session's script (`<repl>:N:M`).
158. A session that loads a package sees the package's `pub` items, as a
     dependent package would; `:why` follows the session's bindings and
     the capability results of the inputs that computed them, not the
     functions they call.
159. A worksheet runs against a test's fake `System` unless `--run`, so
     code that can reach a real capability runs only when asked.
160. The language server analyses a document as part of its package,
     with the open documents' texts standing in for their files; a
     diagnostic carries its catalog code as `code` and its identifier as
     `data.id`; positions are UTF-16 code units unless the client offers
     UTF-8.

**Types** (03)

161. A `const` parameter is an `int`, and its argument an integer literal,
     a `const` parameter in scope, a constant of the program or an integer
     expression of them; `Array.filled(v)` makes `N - 1` copies of `v` and
     keeps `v` as the last element; an `Array` is a list of `N` values at
     run time.
162. A `comptime assert` over `const` parameters is checked at each call
     that gives them constants, by folding integer and boolean
     arithmetic, comparisons and interpolation; one that calls a function
     is checked nowhere (port-debt.md).
163. `i128` holds every `int` and `u64`, and `u128` every `u64`, so those
     widen implicitly; a 128-bit literal beyond `int` travels in SIR as
     its decimal text.
164. `str.lines` breaks at `\n` and drops a `\r` only before one;
     `split` and `replace` take the leftmost occurrences, which never
     overlap; `find` gives an index in characters, `byte_find` one in
     bytes. A text method's misuse (an empty separator, a negative
     count) is a panic whose message names the method
     (`str.split: the separator must not be empty`).

**Stage 1's findings** (`stage1/DISCREPANCIES.md`, D1 to D4: where the
Rust code was an accident, the documents decided and stage 0 changed)

165. A line break inside the interpolation of a one-line string is
     reported (`E0018`), closes the interpolation and the string with
     zero-width `INTERP_END` and `STRING_END`, and is then scanned as code:
     a line end like any other, not an error token.
166. `\u{...}` takes one to six ASCII hex digits naming a Unicode scalar
     value, in the lexer and in a literal's decoded value alike; anything
     else after `\u`, a sign included, is an invalid escape (`E0007`, at
     the backslash) whose braces stay in the text.
167. A stray line in a struct body is reported once per body (`E0126`):
     the flag is saved before a struct's body and restored after it, so a
     struct declared inside another neither resets nor sets the outer
     body's.
168. A diagnostic identical to an earlier one (identifier, parameters,
     spans, notes and fixes) is reported once. The cases of a parametrised
     test stay tests of their own, each checked, lowered and run, and a
     mistake in the body they share is one message.

**Floats** (03, "Numbers"; D-074)

169. `f32` is a builtin type whose values are a kind of their own. Its
     operators are SIR's own on `f32` values, computed in `f64` and
     rounded once to `f32`, which is exactly `f32` arithmetic for `+`,
     `-`, `*`, `/` and the square root; `%` and `**` are `f64`'s on the
     two values, rounded once. A literal is rounded once from its text
     and travels in SIR as the `f64` that holds it (`f32.from_literal`).
170. An `f32` widens implicitly to `f64` where one is expected and beside
     an `f64` operand, as the integers widen (03 and the tour list `f32`
     to `f64` among the silent widenings); a comparison does not widen;
     `to_f32()` rounds an `f64` or an `int` to the nearest `f32`, an
     infinity beyond its range.
171. A float literal whose value rounds to an infinity in its type is
     `E0374`, for `f64` as for `f32`; a float literal with an integer
     suffix is `E0332`.
172. `mul_add` is the host's fused multiply-add on both float types, the
     one place a product and a sum round once; nothing else fuses or
     evaluates in a wider format.
173. A type's constant argument written as an expression is computed
     where the type is written, by folding (`consts.rs`), not by the
     compile-time evaluator, which runs on SIR after every type is
     known: integers, `const` parameters, the program's constants written
     that way, and the integer operators with the run time's rules. A call
     or a `comptime` expression there is `E0264`.
174. An operation on `const` parameters is part of the type until
     substitution folds it; two are equal only when they are the same
     operation on equal arguments (`W * H` is not `H * W`), and one whose
     value overflows once its arguments are known stays as it is written,
     so the mismatch it causes names it.

**Speed of the bootstrap** (18, "Bootstrapping")

175. The leading arms of a `match` that only test the variant share one
     `switch` on the discriminant, and the SIR that runs has its jumps
     threaded after the ownership checker (which still sees lowering's
     blocks, so its findings are unchanged). Both change the number of
     steps a program takes and where its yield points are, not what it
     computes: a seeded schedule may interleave differently, and the
     bounded exhaustive search of `sim/lost_update` explores 283
     schedules where it explored 307, finding the same failure.
176. The interpreter's quick paths (a scalar statement or `switch`, a
     reborrow, a call's arguments, the commonest intrinsics) run only
     outside the monitor and either give exactly the general path's
     value and effects or decline before any effect; under the monitor
     every step takes the general path, whose hooks they would skip.
177. The monitor's active loans of a local are kept as shared and `mut`
     apart, since a read can end only a `mut` one; the order in which an
     access ends loans was never observable.
178. The ownership checker's dataflow state is one table per state (a
     row of loans per local), and what flows to an unwinding block is the
     join of the states after each statement, computed as it goes; the
     results are those of the state per statement it replaces.

**Stage 1's findings, continued** (`stage1/DISCREPANCIES.md`, D5, and
what the stage-1 work found in stage 0)

179. A call with too many arguments to a function with keyword-only
     parameters (after a bare `*`, 02) counts its positional parameters
     against the arguments without a name, as PEP 3102 does:
     `scale(2, 3)` for `def scale(x: int, /, *, by: int)` is `E0305`
     with expected 1, found 2. A function without keyword-only parameters
     counts every parameter against every argument, as before.
180. The ownership checker's questions about a type (must it be consumed,
     does dropping it run a `Drop`, which `Drop`s) look once at each type
     the type reaches, so a recursive type (an enum holding itself through
     lists, any number of times) takes time in proportion to its types,
     not exponential in them. The answers are those the bound on depth
     gave: a type met again inside itself adds nothing to must-consume,
     and counts as running a `Drop` (which may read the references it
     holds).
181. A `list` or `Array` is a `dict` key or `set` element when its
     elements are, as the prelude's `Hash` impls for them declare: the
     host keys it by its elements in order, as it keys a tuple, so equal
     lists are one key, alone or inside a tuple, an option or a struct.

**Stage 1's findings, continued** (the stage-1 README, items 18 to 20:
what transcribing the Rust type checker found in stage 0)

182. The alternatives of `|` bind the same names, of the same types:
     D-055 adopts Python's or-patterns, and PEP 634 requires every
     alternative to bind the same set of names, as Rust does. The first
     alternative declares the names; each later one binds the same
     locals, its types unified with the first's (`E0300` when they
     differ); a name that some alternative does not bind is `E0385`,
     naming it. Exhaustiveness is unchanged (a binding matches anything).
     Lowering tests again which alternative matched and binds from it
     (section "Lowering"), so the interpreter and the ownership checker
     see ordinary SIR.
183. `let x = e else:` decays its target as a plain `let` does: a `Copy`
     value behind a reference (an element that `points.first()` gives
     for a `list[Point]` of `Copy` structs, or a tuple of numbers from
     `pairs.get(i)`) is bound as a copy, read through the reference
     (`copy _2.as(0).0.*`). The binding used to keep the reference under
     the value's type, and the first read of a field stopped the
     interpreter.
184. A `let` with a pattern and `else` (`let (name, i) = e else:`) makes
     its bindings on the matching path, and on the fallback's, without
     registering them, and registers them once where the paths join, in
     the block around the statement, as a single name's binding is. They
     used to be registered in the statement's own scope, dropped at its
     end, so the first use after it was a use after a move.
185. The monitor decides whether a copied reference is a reborrow by the
     type of the place it copies, as the checker does: a copy of a `ref
     mut` is a `mut` reborrow; a copy of a shared `ref` passes its loan
     on, whatever borrow made the reference. It used to look at the loan
     the value carried, so a `mut self` passed to a parameter that only
     reads (`c: Cx`), and passed on from there, ended the shared loans of
     what had been read through it (`case Kind.Items(fields)` on
     `c.nodes[i].kind`), and a later read of `fields` was a violation the
     checker had rightly not reported.

**Stage 2, round 1** (the native debug backend; section "Stage 2")

186. The front end, SIR and the interpreter stay free of external
     dependencies, as stage 0 has been from its first commit; the backend
     crates carry them. `sisu-codegen` depends on Cranelift 0.123
     (`cranelift-codegen`, `-frontend`, `-module`, `-object` and
     `-native`), and `sisu-rt` only on `sisu-diag`, for the catalog's
     messages. The driver depends on `sisu-codegen`, so `cargo build`
     builds the runtime with `sisu`, and a build without the backend would
     drop one dependency of one crate.
187. `unsafe` is allowed in `sisu-rt` and nowhere else: the runtime's entry
     points take the raw addresses native code passes, and its operations
     read values in the native layout. Its own lints deny `unsafe` inside
     an `unsafe fn` without a block of its own, and the workspace's lints
     against `unwrap`, `expect`, `panic` and the rest; every `unsafe` block
     says why it is sound. The workspace keeps `unsafe_code = "forbid"`.
188. Every value is a whole number of 8-byte words, with every part on a
     word: a sized integer, a `bool` or a `char` takes a word, an option's
     tag takes a word before its payload. Values are larger than they need
     be, and in exchange generated code moves every value as words, the
     runtime finds every part at an offset the type table gives, and one
     table describes both sides. The layouts are section "Stage 2"'s
     table; `i128` and `u128` are two words, the low one first.
189. A function takes the addresses of its result and arguments and
     returns a status, 0 or 1 when a panic is leaving it, and unwinding is
     SIR's cleanup blocks run as ordinary control flow: each statement
     that can panic is followed by a test of its status. There is no
     unwinder, no landing pad and no table to keep, the order of drops and
     the messages are the interpreter's because the same SIR runs, and a
     panic during unwinding is found where SIR says the frame is running
     cleanup. The cost is a compare and a branch after every call, which
     a debug build can afford.
190. The interpreter's limit of 100 000 nested calls holds natively: a
     counter in the runtime, incremented by every function but drop glue,
     raises `panic.stack-overflow` past it, and the program runs on a
     thread with a 1 GiB stack, so that the limit and not the machine's
     stack is what a deep recursion meets, with the interpreter's message
     and exit status.
191. Generic functions are monomorphised: each is compiled once for every
     list of concrete type arguments a reached call gives it, from `main`.
     A call through a trait is resolved when the instance that makes it is
     compiled, by the interpreter's dispatch rules (`dispatch.rs`): the
     matching impl's method, with `Self` taken through references, or the
     structural operation the interpreter falls back to. The interpreter
     resolves at run time what native code resolves once; nothing about
     the values decides it in either. 10's instantiation sharing and
     budget are not round 1's.
192. The structural operations (drop, `.copy()`, equality, order, hashing,
     `Display`, `Debug`) are the runtime's, walking a value by its type's
     entry in the type table, which the program carries encoded and
     decodes when it starts; drop glue stays an SIR function, compiled
     like any other, which the runtime calls through the function table.
     Generated code calls one entry point for each, with the type's
     number, rather than a function per type, so code generation is
     proportional to the program and not to its types.
193. An intrinsic call passes a descriptor (the intrinsic, the types of
     its result, arguments and type arguments), written once per shape
     of call into the read-only data, and the addresses of its arguments;
     the runtime reads through references, so an intrinsic takes a value
     and a reference to one alike, as the interpreter's do. The sized
     integers' operations have entry points of their own, one for the
     types a word holds and one for `i128` and `u128`, with the
     interpreter's arithmetic and messages.
194. A call through a capability handle compiles to the host object's
     operation only when nothing `main` reaches can make a handle of that
     type hold another object (`cap.new` of it, `Clock.fake`, `cap.lend`);
     otherwise it is `D0103`. Round 1 has no dispatch on the object a
     handle holds; finding what `main` reaches, rather than looking at the
     whole program, keeps a program's tests and their fakes from making
     its `main` unsupported.
195. A closure is two words, its code and the address of its environment.
     The environment of a closure whose captures are all references is a
     slot of the frame that makes it, which the ownership checker's rules
     keep the closure from outliving; one that holds values gets a heap
     cell. A named function used as a value is a thunk that takes an
     environment and ignores it, so that a call of a function value is one
     indirect call whatever made the value.
196. A `for` loop over an `Iter` behind a reference (a parameter `it: ref
     Iter[T]` iterated with `for x in it:`) drives the iterator through
     the reference: the loop's local holds the reference, and the loop
     registers no drop of it, since the caller owns the iterator. Lowering
     used to give the local the type `Iter[T]` while it held the
     reference, which the interpreter, whose values carry their kind,
     never noticed, and the native layout could not survive.
197. `sisu build --native` writes the executable beside a file's source,
     named by its stem, or into a package's directory, named by the
     package; `--output PATH` names another place. `sisu run --native`
     builds into `sisu-native-PID` under the system's temporary directory,
     named as `build` would name it, runs it with the program's arguments,
     standard streams and environment, removes the directory, and exits
     with the program's status (128 and the signal's number when a signal
     ended it). `--native` refuses the options that only the interpreter
     honours (`D0109`) rather than ignoring them.
198. The link is the system's `cc` with the object, the runtime and the
     system libraries a Rust static library needs, with LLVM's `lld` when
     `ld.lld` is on the search path, which links in a third of the time of
     the system's `ld`. `cc` gets the search path explicitly (the usual
     directories when the environment has none), since the conformance
     runner gives `sisu` an empty environment.
199. The runtime is found beside the running `sisu`: `libsisu_rt.a`, or the
     newest `deps/libsisu_rt-*.a`, which `cargo test` builds without
     copying it up; the newest of them wins, so a stale copy does not.
     Without one, the build is `D0104`, which says to run `cargo build`.
200. Round 1 frees what the interpreter frees, except four things it
     leaks (port-debt.md, "Stage 2"): the heap environment of a closure
     that holds values, the list of copies a `*rest` pattern borrows
     from, the nodes of error return traces, which copies of an error
     share, and the cell of a `Box` or a `dyn` whose value was moved
     out. Each is bounded by what the program makes, and none changes
     what a program prints.
201. `conformance/native.list` is the backend's oracle: the cases it names
     must build natively and give the interpreter's standard output, exit
     status and standard error (less the build's warnings), in `cargo
     test` and in CI. The expectations are the interpreter's files, never
     the backend's own; a case joins the list when `--native-survey` says
     it passes.
202. A `dyn` is two words: the number of the type of its value and the
     value's address, a heap cell the `dyn` owns. A reference to a `dyn`
     is the same two words, borrowed, so that `dyn ref T(r)`, the
     coercion of a `ref T`, is the number of `T` and the address `r`
     holds and allocates nothing; it is the one reference of two words. A
     call of a trait's method on a `dyn` goes to a dispatcher, one per
     method and type arguments, which switches on the number among the
     types the program's `dyn`s of that type hold, and calls what the
     interpreter's dispatch would choose for that type (`dispatch.rs`,
     which dispatches a `dyn` on the type it carries). The types are known
     only once everything is compiled, so the dispatchers are written
     last, after compiling until nothing new appears.

**Stage 2, round 2** (tasks; section "Stage 2", "Tasks")

203. Tasks run natively on the stacks the Phase 1 verdict chose (the
     harness's strategy D(B), its report's Q-6 recommendation and the
     owner's policies 2, 5 and 6): coroutines on slots of a pool carved
     from reservations of address space, without guard pages, with a probe
     at every function's entry and decommit on park, the probe doubling as
     the watermark. The runtime takes two dependencies for it, corosensei
     0.3 (the harness's context switch, policy 2) and libc (`mmap`,
     `madvise`). A task's usable stack is `SISU_TASK_STACK`, 1 MiB by
     default, since the interpreter has no option for it. The report asks
     the probe to compare the stack pointer less the frame before the
     frame is allocated; Cranelift allocates it in the prologue, before any
     code of ours, so the probe compares after, and the reserve holds the
     program's largest frame (which the code generator measures) beyond
     the policy's 5 KiB, which keeps every frame inside its slot just as
     well. A failed probe returns at once without calling anything, and
     the runtime raises `panic.stack-exhausted` when it next looks at the
     panics in flight; the unwinding may use the reserve down to a floor.
     The root task stays on the program's thread (decision 190).
204. A seeded schedule is the interpreter's: the compiled code passes the
     interpreter's yield points (a function's entry, after its probe and
     its count, but in drop glue; an edge to a block at or before the
     current one), counted down in `SISU_RT_BUDGET`, and the runtime makes
     the interpreter's draws in its order on its data. When the budget is
     spent and nothing is ready, the counter is set out of reach until a
     task becomes ready, which keeps the interpreter's decisions without
     calling the scheduler at every yield point. A function the
     interpreter enters without a call statement is entered with its
     entry's count given back. `sisu run --native` takes `--schedule-seed`
     and `--explore` (`SISU_SCHEDULE_SEED`) and the scheduler's `--stats`
     (`SISU_STATS`), whose latency is in microseconds only (`R0124`):
     native code counts no statements.
205. Runtime code called from a task runs on the system stack (the root
     task's, through corosensei's `on_parent_stack`), as policy 5 says, so
     a task's stack holds what the probe can see. What must stay on the
     task's stack is split off: an operation that waits registers the wait
     and returns, and its wrapper suspends the task and tries again when
     it is woken; a drop that would run Sisu code (drop glue, a guard's
     `invariant`) moves the value aside, to be dropped in order on the
     task's stack when the runtime code returns. A switch exchanges each
     task's count of nested calls, stack limits and panics in flight, and
     the scheduler's state is never dropped, since a suspended coroutine's
     frames have no unwinder.
206. The environment of a task's closure (or a generator's) is a heap cell
     even when it holds only references, since the task runs after the
     statement that made it and the next iteration of a loop would reuse a
     slot of the frame; the runtime frees the cell when the closure
     returns, or when the generator is abandoned (closed, decision 236).

**Stage 2, round 2, continued** (the rest of the suite; section "Stage 2")

207. The library types the interpreter implements as values of its own
     are runtime objects in the native layout, with its results, messages
     and order of drops. A `Shared` is the address of its value in a heap
     cell whose header counts the owners and the weak handles; the value
     is dropped with the last owner, as the interpreter drops it, and the
     cell is freed with the last handle of either kind, where the
     interpreter's heap keeps every cell, which nothing a program prints
     can tell apart. A dereference reports the cell to
     `sisu_rt_shared_touch`, so `--stats` counts what the interpreter
     counts (`R0088`). A `Cell[T]` is its value; an `Arena[T]` is an object
     of slots whose keys carry the slot's generation, read through
     `sisu_rt_arena_slot`, and a key whose slot was reused is `R0072`. A
     `MutexGuard[T]` dropped by unwinding checks `T`'s invariant (D-077)
     through the function table, as drop glue is run.
208. `bytes` is laid out as a `String`, a buffer of the bytes themselves
     rather than a word per byte, with a kind of its own in the type
     table, so that it shows as `b"..."`, compares by its bytes and hashes
     apart from text. `bytes.iter` owns a copy of the bytes, so the
     iterator borrows nothing. `DateTime` keeps its one word but has a
     kind of its own, so that it shows as RFC 3339 text as the
     interpreter's does; `fmt.strftime` is the interpreter's subset of the
     directives. `bytes.as_slice` is left out: a `Slice[u8]` is a view of
     words, and no runnable case asks for one.
209. A capability handle of a type the program makes other objects for
     (a `cap.new[T, H]` that `main` reaches) is the address of a cell
     holding its object as a `dyn` holds its value, and a call through it
     calls a dispatcher (decision 202) whose types are the objects of the
     `cap.new`s compiled for that handle type and the host's object, which
     the handles of `main`'s `System` hold (the runtime finds the host
     objects' numbers in the type table by name). A handle of any other
     type keeps decision 194's direct call of the host's operation, and of
     the calls decision 194 counted only `cap.new` remains: a test clock
     becomes a `Clock` through `cap.new`, and `cap.lend` makes no object. A
     `Lent` is its handle and calls as its handle does; a `Cap[T]` is the
     address of a cell holding the `T`. Neither cell is freed, since a
     `Copy` handle has no last copy (the interpreter never frees them
     either). The test clock is a virtual clock of the scheduler, made on
     first use and moved to the next timer when every task waits, as the
     interpreter's is. The host's sockets are numbers in a table of
     non-blocking sockets, and a socket that is not ready makes its task
     wait a millisecond on the host's clock and try again, the
     interpreter's poll. Since the runtime puts them in the `dyn Stream`
     and `dyn Accept` of a `Conn` and a `Listener`, the code generator
     adds the socket types to those `dyn`s wherever it compiles a host
     network operation.
210. A parameter of type `Iterable[T]`, which stands for any iterable
     argument, is passed as a reference with the number of the
     argument's type, the two words of a reference to a `dyn`: the caller
     makes them from its argument (a reference to a list, set, dict,
     iterator or receiver, or a value moved in, which the caller drops
     when the call returns), and the runtime iterates by the type. An
     instance per argument type, the other way, would compile the
     function again for every kind of iterable its callers pass.
211. A field of a variant of a union of error sets is read at the offset
     the members agree on; a `String` in one member and a `Path` in
     another agree, since the two are the same words and drop alike, and
     the field reads as the first such member's type. A `Contextual`
     error shows as `message: cause`, as the interpreter shows it.
212. The backend compiles serialised SIR as well as the program the front
     end hands it: `sisu build --native --from-sir FILE` and `sisu run
     --native --from-sir FILE` read and validate the form `sisu sir`
     prints, which stage 1 emits byte for byte, so stage 1's output is
     compiled without stage 0's front end. The reader parses exactly what
     the writer prints (a program read back writes the same bytes), which
     the tests check on the suite's `.sir` expectations and on the whole
     SIR of every case of `native.list`; code generation is deterministic,
     so the executable is the one the source gives, byte for byte (a test
     compares the two). `--from-sir` names its file, SIR
     whatever its extension, and goes with `--native` only (`D0110`): the
     interpreter keeps `sisu run --sir` (decision 254 lets `sisu run
     --from-sir` interpret it too). The runner's mode
     `--native-from-sir` builds each case of the list from its own `.sir`
     where it has one and from `sisu sir`'s output otherwise, and CI runs
     it beside the native mode, so the path from serialised SIR to native
     code is checked on every push.

**Stage 2, round 2: what the first round leaked** (section "Stage 2",
"The runtime")

213. An environment of one word (a closure that captures one `int`, one
     handle, one reference) is the word itself, in the function value's
     second word, rather than the address of a cell: the call passes it as
     it passes an address, and the closure's code stores it in a slot of
     its frame, so the body reads its captures as before. A function value
     is `Copy` and SIR never dropped one (decision 248 drops one for the
     handles its environment holds), so a heap environment could never
     be freed; the suite's and stage 1's `own` closures all have
     environments of one word. A task's closure keeps its cell, which the
     runtime frees when the task ends (decision 206).
214. A `*rest` pattern binds a view of the elements in place rather than a
     reference to a list of copies: a header of a list's three words (a
     capacity of 0 marking a view) and the address of the buffer it views,
     on the heap, freed when that buffer is (the ownership checker keeps
     the binding from outliving the list). A view of a view views the same
     buffer; an empty view is one header kept for the run.
215. A node of an error's return trace counts what holds it: the errors
     whose trace it is (a copy of an error holds it once more) and the
     node pushed after it. Dropping an error lets go of its trace, and the
     nodes nothing holds are freed. So every `error` enum needs a drop,
     with or without glue. Moving a field out of an error value, which
     then is never dropped whole, lets go of its trace there
     (`sisu_rt_trace_release`).
216. Moving the value out of a `Box` frees the box's cell and leaves the
     box holding none (an assignment to the box's value makes a new cell,
     and a box without one drops nothing); a `dyn` moved into a method that
     takes `own self` has its cell freed after the call, the method having
     taken the value (`sisu_rt_free`). This covers lowering's coercion of a
     `Box[T]` to a `Box[dyn Trait]`, which moves the value out of the first
     box.
217. The runtime counts its live allocations of values, and
     `SISU_RT_ALLOCS=1` makes a native run report the count at exit
     (`R0125`): heap blocks of `sisu_rt::mem`, and the runtime objects of
     iterators, tables, arenas and trace nodes, but not the cells kept for
     the run (capabilities, `Cap`s, the empty view). An environment
     variable rather than an option, since `sisu run --native` compares
     what it prints with the interpreter's.
218. What the interpreter leaves to its host to free, which no program can
     observe, the runtime frees: SIR's drop glue leaves out a field whose
     drop nothing observes (a `String`), and a type whose fields are all
     such has no glue, so the type table marks each field the glue drops
     (`abi::Field::glue`, computed by SIR's `drops_something`, now in
     `sisu_sir::types` for lowering and the code generator alike) and the
     runtime drops the others after the glue, last to first. A value moved
     into a parameter of an intrinsic that neither owns it nor lets its
     result hold it (`print` of a text a format made) is dropped after the
     call, also when the call panicked, when its drop is unobservable; one
     whose drop a program would observe is left alone, as the interpreter's
     host leaves it.
219. The compiler's costliest steps run on threads where their parts are
     independent, and take the results in the program's order, so that
     what it prints and writes is the same as on one thread, byte for
     byte (10, "Compile speed as a requirement": bodies are checked
     independently and in parallel). The query layer parses a program's
     files at once; the type checker checks the bodies its memo does not
     hold; the ownership checker analyses each function on its own (moves
     and drop placement, then loans, escapes and must-consume), the
     largest first; and the code generator hands each function's
     Cranelift IR to threads that compile it while it writes the next, and
     defines the machine code in the order it made the functions, so the
     object file does not depend on which thread finished first. As many
     threads as the machine runs at once (`available_parallelism`), at
     most 8; on one, nothing changes. A panic on a thread is raised again
     on the calling one, so an internal error is reported as before; an
     error Cranelift returns is reported once every function is made,
     after a construct the backend does not compile. Besides the threads:
     the ownership checker keeps its answer about a type's drop glue,
     finds functions by name through an index, and gives a loans state a
     row only for the locals whose type can hold a loan (nothing is stored
     in another's); lowering does not collect again the types it has
     already met; and a command frees its query database on a thread of
     its own, since the process exits without waiting for it.

**Stage 2, round 3: what the second round left** (section "Stage 2";
"Lowering"; "The ownership checker")

220. Drop elaboration opens a value one of whose parts some statement
     moves or drops on its own ("split") wherever the value may be
     uninitialised, instead of guarding the whole by one flag when every
     part has the whole's two bits: a path that moved the part and a path
     that moved the whole give the same bits, and the flag, which a move
     of the part leaves set, dropped the whole after the part had gone
     (in `Iterator.collect`'s cleanup, and in a struct one branch takes a
     field of and another moves), which the interpreter's moved markers
     hid and a native run freed twice. A part with a path of its own is
     decided by its own state and flag; one without is in its closest
     path's. Only drops whose value is split and may be uninitialised
     change, and stage 1's elaboration follows.
221. A variant pattern on an error value that moves a field out first
     moves the whole value into a local of its declaration's type, moves
     the fields out of that, and drops it with the bindings; a variant
     pattern on a union of error sets tests the variant, the fields and
     binds through a reference of the declaration's type, once
     `error.is[D]` has chosen it. An error value is then never partly
     moved, so elaboration never opens one: a union's parts are its
     member's (elaboration gave a union no parts at all, which leaked the
     value an arm took nothing from, `examples/errors`), and the return
     trace, which no SIR place names, is let go of with the value or at
     the move out of the member's local (decision 215). The native
     backend reads every field of a union's variant at its member's
     offset, so members that lay a field out differently compile. Stage
     1's lowering follows.
222. The operands of one statement (a call's arguments, an aggregate's
     parts) are evaluated in order, and one held in a temporary that needs
     drop is registered for drop from its evaluation until the statement
     takes it, when an operand after it may leave the statement (a `?`, a
     `return`, a panic); a struct with an invariant is registered while
     its check runs. A value made for a statement that a later operand
     abandons is therefore dropped, as 04 asks of every value: the
     interpreter did not drop it, so a `Drop` that should have run did
     not (`Pair(a=Loud(...), b=f()?)`), and natively it leaked
     (`reflect/cli`). "May leave" is anything but literals, locals,
     functions, closures, and the fields, borrows, tuples, lists, variants
     and structs of such (a struct with an invariant, or with a field's
     default, may); the operand that is last, or before only such
     operands, is not registered, so most statements lower as before.
     Stage 1's lowering follows.
223. What the interpreter's host lets go of without dropping it, the
     runtime frees by forgetting: a drop that frees memory and runs no
     Sisu code (no drop glue, no `Drop`, no guard's check or unlock). The
     error a `main` returned and the objects of capabilities and values
     of `Cap`s (an arena per run) are forgotten when the run ends. A
     generator abandoned at a `yield` (decision 95) is resumed there,
     outside the schedule, its yield returning `UNWINDING` without a
     panic, and its frames unwind through their cleanup blocks with their
     drops forgetting; one that never started forgets its environment by
     the type its block carries (`sisu_rt_task_env`). A generator whose
     cleanup runs code of its own (a `defer`) cannot unwind without
     running it, which the interpreter does not, so it says so when it
     starts and is left as it was (port-debt.md). Decision 236 closes an
     abandoned generator instead, running its drops and `defer`s in both,
     and decision 237 frees a capability's cell with its handle's last
     copy.
224. A function value holds its closure's environment itself: one word
     of code and K words of environment, K being the largest environment
     an `own` closure of the program needs, at least one. Only compiling
     finds K (an environment's size depends on the instance), so a
     program whose `own` closure needs more is compiled again with wider
     function values, which no case of the suite or stage 1 needs. A
     call passes the address of the value's environment words; a closure
     of references whose environment is larger cannot outlive its frame,
     and keeps it in a slot of the frame, the value holding its address;
     a task's closure keeps its heap cell. Nothing is allocated for a
     closure, so nothing needs freeing, where an `own` environment of more
     than one word (decision 213) had a heap cell nothing could free.
225. A function used as a value is its constant when that will do;
     otherwise it is a closure of its value wrapper, `NAME#value`
     (`sisu_sir::value_wrapper`), made with the function's body: for a
     generic function, whose type arguments the type checker infers where
     the value goes and whose bounds it requires of them, as for a call
     (it was `E0350`); and for one that borrows a parameter, since a
     function value takes its arguments by value (a call through one
     passed an owned value where the function expected a reference, and
     the interpreter stopped with an internal error). The wrapper is
     generic as the function is, takes an empty environment and the
     parameters as the function's type has them, lends those the function
     borrows and drops them after the call; a function returning a borrow
     of such a parameter cannot be a value (`E0352`), and a generic
     intrinsic has no body to wrap (`E0350`). `bytes.as_slice` compiles
     natively as the interpreter runs it: a `Slice[u8]` of copies of the
     bytes, each a word.
226. The native mode of the conformance runner runs every executable
     with `SISU_RT_ALLOCS=1` and requires its report to say that nothing
     the runtime allocated is live at exit, leaving the report out of the
     standard error it compares; only a program that aborts (status 134)
     ends without one. Every case of `native.list`, both from source and
     from serialised SIR, is then a leak test, in `cargo test` and in CI.
227. The native executable's debug information is DWARF 4 that the code
     generator writes with gimli (a dependency of `sisu-codegen` only): a
     line table from SIR's locations, which Cranelift carries to the
     machine code; a subprogram for each function instance with its named
     locals at their stack slots and of their Sisu types, each parameter's
     address kept in a slot of its own; and `.debug_frame` from Cranelift's
     unwind information. The unit's language is Rust, the closest one gdb
     and lldb know, whose conventions for texts, slices and variant parts
     make them print a Sisu value as Sisu shows it. The code before a
     function's first statement is its declaration's line and the
     prologue's end is marked, so a breakpoint on the function stops after
     the parameters' addresses are kept. A drop flag is a local named
     `flag` with no place in the source, so a variable a program names
     `flag` is described (and a `defer` that assigns it is code of its
     own, decision 223). A panic's message needs none of this: it names
     the Sisu functions and lines as the interpreter's does, from the
     locations the code keeps as it runs.
228. The steps of a build that ran on one thread do less on it, and what
     the compiler writes is the same byte for byte (stage 1's SIR and
     executable, every case's). Resolution plans the derived impls of
     every module before it declares any, runs their templates on
     threads (a template is a pure function of the type's shape) and
     parses the texts they write on threads into the cache of generated
     texts, then declares them in the modules' order. Lowering hands a
     body to threads when the worklist first names a function of it, and
     the program's own bodies at once (not the standard library's, of
     which a program reaches few), lowering one itself while it waits;
     the worklist takes each body's functions when it comes to them, in
     its own order, so the program is the one it makes alone. The writing
     of IR stays on one thread, since the numbers it gives types,
     functions and data as it meets them are in the object file, and
     threads would have to number them again in that order and rewrite
     the IR; it does less instead: it finds functions by name through an
     index (SIR's `function` searched them all), hashes its maps with
     FxHash's function rather than SipHash, does not rebuild a type that
     has nothing to put in, keeps whether a type needs a drop, and the
     threads that compile keep their Cranelift contexts and hand each
     function back cleared, to be written into again on the thread that
     allocated it. Stage 1 builds in 3.9 s instead of 4.3 s (section
     "Compile time and run time"); `SISU_NATIVE_TIMES=1` reports each
     step.
229. A native run under `SISU_RT_ALLOCS=peak` also reports the most of
     the runtime's allocations that were live at once (`R0126`), before
     the count at exit. A program whose memory is bounded holds as many at
     once however many rounds its loops run, which a test checks of
     structs, enums and tuples whose fields hold lists, dicts, sets, texts
     and boxes, made, copied, replaced and dropped in a loop
     (`tests/leaks/aggregates.sisu`, for 20 and for 500 rounds). Dropping
     such a value frees every part, with drop glue or without, since
     decision 218: the leak stage 1 found (a struct with a `list[bool]`
     field kept the list), which made its lowering of its own package
     peak at 3.4 GB natively, was a backend from before that decision;
     that run now peaks at 530 MB (the interpreter's at 2.5 GB). A count
     of allocations rather than the process's resident memory, since the
     count is the same on every machine and every run.
230. What a channel carries is checked as what a `Shared`, `Cell` or
     `Mutex` owns is: it is neither a view (`E0421`) nor must-consume
     (`E0422`), as the prelude's documentation of `channel` says. A
     channel's receiver may keep what was sent after what it borrows is
     gone: the checker accepted `tx.send("a{i}")` on a `channel[str]`,
     whose view of a temporary text the interpreter printed (its `str`
     owns its text) and a native run read after the text was freed. The
     two programs of the suite that sent string literals through a
     `channel[str]` send `String`s instead, with the same output, and
     `concurrency/channel_owner_errors` holds the diagnostics. As for a
     `Shared`, a type parameter and a function value count as views
     (known over-rejections 13 and 14), so a generic function cannot make
     a `channel[T]`, nor a program a channel of closures. Stage 1's type
     checker follows.
231. The native build of a package writes its executable to
     `build/PROFILE/NAME` in the directory of its lockfile, its
     workspace's or its own. 09 names the profiles (`debug`, `release`,
     `release-size`) but not where a build's artefacts go; a directory per
     profile keeps a release build from replacing a debug one, as Cargo's
     `target/PROFILE` and SwiftPM's `.build/PROFILE` do, under the name of
     the command that makes it. A single file's executable still goes
     beside it, named by its stem. `sisu run --native` builds a package's
     there and keeps it, the executable a debugger is then pointed at, and
     a file's aside as before. `sisu build` without `--native` keeps its
     meaning: it checks and lowers, and writes nothing but the lockfile.
232. `--release` is the native backend's release profile as far as it
     goes without LLVM (09: "LLVM, optimised, overflow and bounds checks
     on, contracts per manifest"): Cranelift's `speed` level rather than
     `none`; the checks of overflow and bounds of every build, since no
     profile turns them off; and the contracts of the root package's
     `[profile.release] contracts`, preconditions by default (07), which
     `--contracts=LEVEL`, now accepted with `--native`, overrides. It goes
     with `--native` only (`D0111`). The runner's `--native-release` mode
     builds every case of the list so with every contract checked, and
     `cargo test` and CI hold it to the interpreter's expectations. It
     buys little yet. Stage 1 builds in 4.5 s instead of 4.0 s (code
     generation 2.0 s instead of 1.5 s) and runs as fast: its batch of the
     409 conformance files 2.05 s against 2.22 s, its type check of the
     prelude 1.66 s against 1.67 s, its lowering of its own package 51.4 s
     against 51.5 s, with the same output. Loops of arithmetic and indexing
     gain 8 to 20% (`fib(32)` 0.04 s against 0.05 s, a sieve of 3 million
     0.26 s against 0.31 s). Stage 1's time is mostly in the runtime's
     intrinsics, which a release build calls as a debug one does (decision
     233 made them cheaper for both), and the generated code keeps every
     SIR local in a stack slot, which Cranelift's optimisations do not
     keep in registers (port-debt.md, "Stage 2").
233. A call of an intrinsic does less in the runtime. The runtime's own
     table of intrinsics is matched first and the modules with tables of
     their own (cells, bytes, capabilities, the host's network) only for
     the rest, where each of the four was asked first, and the drops the
     runtime moved aside (decision 205) are drained only when there are
     some, which a count kept beside them says in one load. Stage 1's
     native type check of the prelude runs 12.1 rather than 15.1 billion
     instructions (1.22 s against 1.63 s), its batch of the conformance
     files takes 1.6 s against 2.2 s, and its lowering of its own package
     46.6 s against 48.8 s, with the same output.

**Stage 2, round 4: the runtime** (section "Stage 2"; "The interpreter")

234. A stack overflow is one panic, `panic.stack-overflow` (`R0032`), whose
     message names the function whose frame did not fit (the intrinsic,
     for the anchor of a `task.scope`, `race` or `shield`), with the
     same text and exit status in the interpreter and natively, as the
     Phase 1 verdict means it: a clean Sisu panic, never a crash. Where it
     happens is each implementation's: the interpreter's 100 000 frames
     (`MAX_DEPTH`), a native task's stack (1 MiB, `SISU_TASK_STACK`,
     since neither 06 nor 10 names an option for it) at the probe of a
     function's entry, and the root task's 100 000 calls (decision 190)
     on its 1 GiB. So nothing a run prints may depend on the depth: a
     panic's trace deeper than a hundred frames shows the fifty innermost
     and the fifty outermost, as Go's tracebacks do, with one line between
     them that gives no count (`run.trace-elided`), in a panic's report
     and in `TaskPanicked`'s trace alike, and `run/stack_overflow` and
     `concurrency/stack_overflow` hold the two to the same output. The
     unwinding that follows an overflow runs drops, and a `Drop` is a
     call: the interpreter lets it push 1 000 frames beyond the limit
     (`UNWIND_DEPTH`), native code counts 1 000 calls beyond it, and a
     native stack keeps its floor below the probe limit, so that an
     overflow with a `Drop` in each frame unwinds instead of aborting as a
     panic during unwinding. A generator's frames count on top of its
     consumer's (the consumer's depth when it last asked for an item, in
     both), so a recursion through generators overflows as one through
     calls does, where each generator's own stack let it grow until the
     memory ran out. The probe compares the stack pointer after
     Cranelift's prologue, which writes only the top of the frame (the
     saved registers and the spill slots, above the stack slots); the
     parameters' addresses kept for the debugger are stored after the
     probe; so the probe is the Phase 1 report's comparison of the stack
     pointer less the frame for all of the frame but its top, and the
     reserve below the limit holds the policy's 5 KiB and the most a call
     may write before a probe sees it (the largest top of a probed frame
     and the largest frame without a probe, a dispatcher's or a thunk's),
     not the program's largest frame: 1.2 KiB rather than 69 KiB for
     stage 1. `R0123` (`panic.stack-exhausted`) is retired. The monitor
     finds a reference's frame by the index the reference carries rather
     than by walking the stack, which made a deep recursion's run under
     it quadratic (33 s rather than 0.7 s for `run/stack_overflow`).
235. A value moved into an `Iterable[T]` parameter (`f(reversed(xs))`, a
     receiver) is dropped, with whatever the callee left of it, when the
     call returns and when it unwinds, in the interpreter as natively: the
     parameter is a reference (decision 210), so the caller owns what it
     moved in, and 04 drops every value once. The interpreter let the
     value go once the callee began, so the rest of an iterator the callee
     did not finish was never dropped and a `Drop` that should have run
     did not; native code dropped it after a return but not after a panic.
     Now the interpreter keeps the value in a local past the callee's own
     (the parameter refers to it, as native code passes the caller's
     memory) and drops it when the frame goes, and native code drops it on
     the panic path too. An intrinsic given such a parameter
     (`iterable_list`, `zip`, `sum`, `sorted`, ...) takes what is left of
     the iterator and leaves it ended, as native code does, so a second
     pass sees nothing; one that a panic stops drops what it took and what
     it left (`zip` of different lengths). `run/iterable_rest` holds the
     two to the same output.
236. A generator dropped before its end is closed, as Python's `close`
     closes one, in the interpreter as natively: suspended at a `yield`,
     it is resumed there at once, outside the schedule, with a panic that
     nothing reports, and its frames unwind running their drops and
     `defer`s before the consumer goes on; one that never started drops
     what its closure captured, and an item it yielded that nobody took is
     dropped. 04 drops every value and 02's `defer` runs at the end of its
     block, and a generator left at a `yield` leaves its blocks; the
     two implementations let its frames go without running a `Drop` or a
     `defer` (decisions 95 and 223), and the native runtime kept the frames
     of one whose cleanup ran a `defer`. The cleanup runs where the
     consumer dropped the generator, which cannot take a panic: a panic
     there aborts the program with status 134 (`panic.during-abandon`,
     `R0128`), as a panic during unwinding does, and so does an operation
     that would wait (`panic.abandoned-wait`, `R0129`), since the consumer
     is not a task the scheduler could switch away from. A generator runs
     only when its consumer asks: a scope's cancellation, which wakes the
     tasks that wait, no longer wakes a suspended generator, which then ran
     its body on by itself. `concurrency/generator_close` and
     `concurrency/generator_close_panic` hold the two to the same output.
237. A capability's object, and a `Cap`'s value, goes with the last copy of
     its handle, in the interpreter as natively: each copy of a handle is
     counted, and the drop of the last runs the object's `Drop` and frees
     its cell, where the cell lived as long as the run and the `Drop` never
     ran (decision 98), so a program that made fakes or attenuations in a
     loop grew until it ended. Handles stay `Copy` (Q-51); a copy is counted
     where it is made (a variable, a field, an aggregate, a closure's
     environment, a function's argument, an argument an intrinsic keeps,
     `sisu_rt_retain` natively) and a move is not; a `Lent` counts nothing.
     This changes rules of the front end, which stage 1 transcribes.
     Lowering's `needs_drop` holds for a value whose type holds a handle or
     a `Cap` (`holds_capability`) and for one whose type is `Copy` only by a
     bound (a generic function's `T: Copy` may be a handle; the drop of an
     `int` does nothing); a temporary of such a type is moved into what
     takes it, an argument a callee borrows (not `own`) is a copy of a
     place, a temporary's being dropped with the statement's others, and a
     comparison or an interpolation borrows it. SIR's `drops_something`
     holds for the handles, `System` and `Cap`, so drop glue drops them. A
     parameter holding a handle counts as borrowed for the elision rules,
     only a `TestClock` lends nothing of its own (`is_run_handle`), and the
     ownership checker gives a call's result that may point into a handle's
     object (its `from`) a loan of the place the call copied the handle
     from, which a `return` may carry for a parameter in its `from`: `let v
     = Cap(x).get()` is `E0406` and replacing a handle while a borrow of its
     object is used is `E0402` (`ownership/handle_borrows`), where both ran
     with the object kept alive. `System.assemble` takes its handles `own`,
     and the interpreter's transcript runs give each run's `System` its own
     counts of the world's handles. A function value's environment is never
     dropped (decision 224), so a handle an `own` closure captures keeps its
     object until the run ends, when the runtime forgets the cells left
     (decision 223; decision 248 drops a function value), a registry of live
     cells by address that a release finds in logarithmic time.
     `capabilities/object_drops` holds the two implementations to the same
     drops.

**Stage 2, round 4, continued: the language** (section "Traits and
generics"; "Compile-time evaluation")

238. `@derive(Default)` is the prelude's trait `Default` (11's core
     traits; D-051 makes it a trait and no language rule) with a derive
     written in Sisu: `T.default()` is the struct with each field's own
     default where it has one and its type's `default()` otherwise,
     through `reflect.construct`, which leaves out the fields with
     defaults. A derive template's `reflect.fields_of(T)` now says which
     struct fields have defaults (`default`, which was always false
     there), and the derive's field check (`E0244`) asks `Default` of the
     fields without one. An enum's default is one of its variants, which
     only an impl can name (as Rust's `#[default]` marks), so deriving it
     for an enum is `E0608` with the template's reason. The prelude
     implements it for the numbers (zero), `bool` (`False`), `str` and
     `String` (empty), `list`, `dict` and `set` (empty), options (`None`)
     and tuples of up to four elements; a capability, a channel or a
     `Mutex` has no natural first value and no impl.
     `comptime/derive_default` and `comptime/derive_default_errors` pin it.
239. The sync types of 11 beside `Mutex`: `Atomic[T]` in the prelude
     (01's `Shared(Atomic(0))`), and `RwLock[T]`, `ReadGuard[T]`, `Once`
     and `Condvar` in std.task. A generic library type cannot own a
     `Mutex[T]` of its parameter (known over-rejection 14), so `Atomic`
     and `RwLock` are builtin types of the runtime, and `Once` and
     `Condvar`, which are not generic, are Sisu. `Atomic[T]` (for a
     `Copy` `T`) is a `Cell` that tasks may share, `Send` and `Sync` when
     `T` is `Send`: `load`, `store` and `swap` are the cell's intrinsics,
     `compare_exchange` one of its own that compares as values compare
     (C++ compares bytes), and `Atomic[int]`'s `fetch_add` and
     `fetch_sub` are Sisu with no yield point between the read and the
     write. The runtime runs one task at a time, so each operation is one
     step of the program; the C++11 memory orders of 06 are accepted
     (`MemoryOrder`, every operation's last argument) and all act as
     `SeqCst`. `RwLock[T]` is a mutex that counts its readers: `read`
     gives a `ReadGuard[T]`, a view whose `value` nobody may change
     (`E0341`, with a note naming `write`), and waits while a task
     writes; `write` gives a `MutexGuard[T]` and waits while any task
     reads or writes; neither wait is a point of cancellation, a task
     that writes and asks again panics (`R0073`), and a write guard
     dropped by unwinding poisons the lock as a mutex's does (D-077).
     Readers that keep coming keep a writer waiting (decision 249: no
     longer). It is `Send` when
     `T` is, and `Sync` when `T` is also `Sync`. `Once.call_once` runs
     its function under a `Mutex[bool]`, so a caller that comes while it
     runs waits; a call that panics leaves it not done. `Condvar.wait`
     takes the mutex and its guard, registers a channel of its own, lets
     the guard go, waits for a notify on the channel and locks the mutex
     again; `wait_while` loops on a condition; `notify_one` passes over a
     waiter that has gone. The panics of a held or poisoned lock now say
     "lock" rather than "mutex" (`R0073`, `R0074`, `R0090`), since an
     `RwLock` raises them too. The front end learns the new builtins
     where it knows `Mutex`, `MutexGuard` and `Cell` (their auto traits,
     a read guard as a view and a smart pointer whose value is read-only,
     the owner checks, `Atomic`'s own `Debug`), which stage 1 transcribes.
     `concurrency/sync_types` (under thirty schedules) and
     `concurrency/sync_errors` pin them.
240. `sisu audit --caps FILE...` is 18's Phase 6 audit, read from SIR as
     18 says, with the choices it leaves open made so: a function does
     I/O when it calls an operation of a capability (a method of an
     operations trait, or any other call whose first argument is a
     handle, a `System`, a `TestClock` or a `Lent`, an attenuation
     included, but not `lend`, which passes it on), or calls a function
     of the program that does, a closure counting as called where it is
     made and a call through a trait as a call of each of the program's
     impls of the method; a capability parameter is one whose type holds a
     capability, through references, fields of the program's structs and
     enums, tuples, options and collections, or a type parameter that an
     operation is called on (`[F: FsOps]`, the form that takes a `Lent`);
     the threading depth of a use is the fewest calls from `main` to its
     function (a breadth-first walk of the same call graph), `-` when
     `main` does not reach it (a test's code); and a bundle is a struct of
     the program whose fields hold three or more distinct capability
     types, a `System` among them not counting, since bundling them all is
     the language's composition root. The output is data, a line per
     function, use and bundle and a summary with the median and the
     largest number of capability parameters, so that 20's thresholds
     (a median above 2, a maximum above 4, any bundle of four) read off
     it. `sisu-driver`'s tests hold it to a program of each kind.
241. A test declares the capabilities it needs by naming them as
     parameters, one of each of `Fs`, `Net`, `Clock`, `TestClock`, `Rand`,
     `Env`, `Proc` and `Stdin` (17's open question, "How a test declares
     the capabilities it needs so that parallel-versus-serial and the
     cache key are known without a signature burden"), and receives a
     fresh fake of each: `System.fake`'s in-memory `Fs`, the run's test
     clock, `Rand.seeded` with the test's seed, and `Env`, `Proc` and
     `Stdin` fakes, a network of its own (`Net.fake`, where `System.fake`
     has one that reaches nothing: 17 gives "no `Net` unless requested"),
     and the test clock as a `TestClock` for a test that moves time; under
     `--sim` the simulated system's objects. A test that takes `sys:
     System` still receives the real one and runs alone; a test may not
     mix the two, name a capability twice, or take anything else
     (`E0702`). A test that names capabilities touches nothing outside
     itself, so it runs in parallel and may be cached (decision 242).
     This changes a rule of the type checker (the test signature) and of
     lowering (`System.fake` and `Net.fake` are kept for such a run), which
     stage 1 transcribes. `testing/named_capabilities` and
     `capabilities/test_signature` pin it.
242. `sisu test` keeps a cache of the tests that passed (17, "Output and
     caching"), in `PROGRAM.test-cache` beside a file, as its native
     executable goes beside it (decision 231), or `build/debug/test-cache`
     in a package, beside its debug executable. A test's key hashes the
     toolchain, the options that decide a run (the seed, the monitor, the
     fuel, the stored snapshots), its name, the program's declarations
     (types, intrinsics, trait methods, impls), what runs without a call
     (drop glue, `Drop`s, invariants, the fakes a test receives), and the
     SIR of every function the test reaches (calls, closures, functions used
     as values, each impl of a trait method it calls), without source
     positions and with the test's own number left out: a comment, a
     reordering, a new test or an edit of code the test does not reach
     reruns nothing, and an edit of what it reaches reruns it. A test that
     takes the real `System` touches the world and is never cached, and
     neither is one that failed. The cache is used only when the results are
     the code's and the seed's: not under `--explore`, `--sim`, another
     scheduler, `--update`, `--defer-borrow` or `--replay`, and not with
     `--no-cache`. A cached test prints what it printed when it ran, so
     nothing printed depends on the cache (Go 1.10 marks a cached result;
     here a run is the same either way, which the conformance runner's
     repeated run, with `--no-cache`, still checks). The keys are FNV-1a
     over 64 bits, as the replay tokens' hashes are: a collision would reuse
     a result, which a cache of passing results accepts.
243. Conditional compilation (08, "`@cfg` on items and `if comptime` in
     bodies"): an item, an item of a suite or an `extern` block, or a method
     of an impl or a trait whose `@cfg` does not hold is left out before
     names are resolved, as if it were not written; the driver filters each
     module's syntax tree (`cfg.rs`), and a tool that shows the file shows
     it whole. A condition is `test`, `feature="NAME"`, `not` and a
     condition, or `all(...)` or `any(...)` of conditions, and several in
     one `@cfg` must all hold. Sisu's `not` is the operator, so `@cfg(not
     test)` is the canonical form, and a feature that is off is `not
     any(feature="tls")`: `feature="tls"` is an argument, not an expression,
     so `not(feature="tls")` does not parse (decision 250 parses it).
     Anything else is `E0266`, and its item is kept. `test` holds when the
     program is built with its tests: `sisu test`, `sisu bench`, `sisu
     check` and the tools that check (the language server, the REPL, `sisu
     audit`). `sisu run`, `sisu build` and `sisu sir` build it without them,
     and leave out its `test`, `suite` and `bench` items and its doc
     examples too, as Rust's `cfg(test)` leaves out `#[test]` functions: a
     test may use an `@cfg(test)` helper, and a run does not stop at a
     mistake in a test. A feature belongs to a package (09): the root
     package's `default` is on, and each dependency's unless every entry
     that reaches it says `default-features = false`; an entry's `features`
     are on; and what a feature that is on lists is on in turn, a feature of
     its own package or (`DEP/FEATURE`) of a dependency. These are Cargo's
     rules, without optional dependencies, which arrive with a registry. A
     name a feature list gives that is no feature is `E1018`, and a
     `feature="NAME"` whose package declares no such feature is `W0006` (a
     single file declares none), the mistake `rustc`'s `unexpected_cfgs`
     lint exists for. There is no command-line switch for features yet
     (Cargo's `--features`; decision 250 adds it). The features are part of
     the program's layout, so a recorded run replays with them. This changes
     rules stage 1 transcribes: the filter and its diagnostics, a build
     without tests (the driver's filter, and the resolver's `Sources::tests`
     for doc examples), the manifest's `features` and `default-features` and
     the check of feature names. `comptime/cfg`, `comptime/cfg_errors`,
     `packages/features` and `packages/feature_errors` pin it.
244. Reflection beyond generic functions (08, "Reflection"; 18's Phase 8): a
     function or a method whose body runs code at compile time by unfolding
     (a `reflect` query, `x.[f]`, a `comptime for` or `comptime if` among
     statements or arms, or a `comptime` expression over a type parameter)
     is a template, as a generic function that reflects already was: a free
     function, generic or not, a method declared in a type or in an inherent
     impl, or a method of a trait impl without type parameters. Its
     instances are made the way a generic function's are (`template.rs`,
     `instances.rs`), with three additions. A template without type
     parameters is asked for once by the type checker, whether anything
     calls it or not, and its instance's SIR function takes the template's
     name, so whatever names the template (a call, a trait's dispatch to the
     impl's method, a vtable, a function value, `main` as the entry) runs
     the instance, and nothing else changes. A method's instance is a method
     of an inherent impl of its type (the type's alias in the instance's
     module), with `Self` bound to the type's shape, so
     `reflect.fields_of(Self)` and `self.[f]` work in it. And the types the
     compile-time code names that are not type parameters
     (`reflect.fields_of(Point)`, a non-generic struct, enum, error set or
     alias in the function's scope) are known to the unfolding without being
     replaced in the code's text. A generic method that calls a template
     with its own type parameters is promoted as a generic function is. An
     instance is never a template itself (its module is an instance's), so
     code an instance could not unfold is an error of the instance, not
     another instance. A trait's own methods and the methods of a generic
     trait impl would need an instance for each implementing type that a
     trait's call resolves to, at run time behind `dyn`; a `comptime def`, a
     constant and a test are evaluated or checked directly; all of them keep
     `E0900`, which now names the place (decision 251 makes them all). An
     error in an instance's code is reported in the instance's text
     (`<NAME>/FILE`), as a generic template's always was; the reflection
     over a trait's methods and a `comptime def` in a trait other than its
     derive are not here (port-debt.md). This changes rules of the resolver
     (which functions are templates, `Self` and named types in the
     unfolding, the instance's form) and of the type checker (the eager
     request, a template without type parameters as a value, the `E0900`
     texts) that stage 1 transcribes. `reflect/methods` and
     `reflect/methods_rejected` pin it.
245. `std.core`, `std.string` and `std.fmt` are modules an import names,
     as 11's table of `std` lists them: `std.core` is the prelude itself
     (`from std.core import Option`, or `import std.core` and
     `core.Option`), and `std.string` (`String`) and `std.fmt`
     (`Display`, `Debug`, `Formatter`, `Spec`) name the prelude's items, as
     `std.collections`, `std.num`, `std.time`, `std.net`, `std.cap` and
     `std.task` already named theirs. Their code stays in the prelude: 11
     makes the prelude `std.core`, and the prelude's types and traits are
     what every program sees without an import, so moving them into
     modules would only add an import to every program. `str` and `char`
     are built-in names, not items, so no module names them. The
     resolver's table of std's modules (`std_sources.rs`) is the rule
     that changes, which stage 1 transcribes; `std/modules` pins it.
246. The release profile keeps scalars in registers and computes integer
     division inline, the first two of what port-debt.md's "Stage 2"
     gave decision 232 to pay. A local of one word (a number, a `bool`, a
     `char`, a drop flag, or a reference that is an address rather than a
     `dyn`'s two words) is a Cranelift variable rather than a stack slot
     when no code needs its own memory: no statement borrows it, projects
     it other than through its reference, or names a key with it; a
     parameter's value is read from the caller's memory once, at the
     entry, and the result stays in the caller's memory. Code that takes
     such a local's address anyway (a call's result, a value the runtime
     reads) gets a slot the variable is copied to, and a call's result is
     read back from it after the call. A reference in a variable reaches
     what it refers to at the address it holds. The debug profile keeps
     every local in its slot, which its debug information describes; a
     release build's variables have none. In both profiles, `//`, `%`,
     `<<` and `>>` on `int` are instructions (a truncating division
     adjusted to round down, a remainder adjusted to the divisor's sign,
     a shift), and the runtime is called only on a cold path for the
     operands it raises a panic for (a divisor of zero, `MIN // -1`, a
     shift by less than 0 or more than 63; `MIN % -1`, which is 0, too),
     so every message and status is the runtime's as before. Built by a
     release-built `sisu` and run natively in release, best of five:
     the longest Collatz chain under 10^6 takes 0.73 s against 2.09 s,
     trial division of the numbers under 2*10^6 0.64 s against 1.47 s,
     Euclid over 12 million pairs 0.68 s against 1.02 s, a product of
     400x400 matrices 0.30 s against 0.47 s, a sieve of 5*10^6 0.22 s
     against 0.29 s, and `fib(37)` 0.36 s against 0.43 s; in the debug
     profile, the inline division alone takes Collatz to 1.86 s from 2.35
     s and trial division to 1.19 s from 1.67 s. Stage 1, built in
     release from a copy of its package, gains less, since its time is
     the runtime's intrinsics' (half of its instructions are in
     `sisu_rt_intrinsic`, a third of them in decoding and dispatching the
     call): its check of its own package takes 56.1 s against 59.1 s, its
     SIR of `examples/wordcount` 0.82 s against 0.85 s, its tokens and
     tree of the prelude 0.12 s and 0.19 s as before, each output byte
     for byte the same. `run/integer_division` pins every sign of `//`
     and `%`, the ends of `int` and the shifts, natively in both
     profiles. This changes no front-end rule.

**Round 9: what round 8 left** (sections "Names and the standard
library", "Tasks and the scheduler", "Capabilities", "Stage 2")

247. An iterator written in Sisu given to an `Iterable[T]` parameter
     (`sum(xs.iter().map(f))`, a program's own type that implements
     `Iterator`) is passed as `iter_from(x)`, an `@internal` generic
     generator of the prelude that takes its items from `x` as they are
     asked for, since the functions that take an `Iterable[T]` iterate
     collections and `Iter`s and never call a program's impls: both
     implementations stopped there with an internal error. The checker
     writes the call where it accepts the argument (`Iterable[T]`'s
     coercion, in `call.rs` and `expr.rs`); a reference to such an
     iterator is `E0300`, since what it refers to cannot be moved into the
     generator. A generator that code which cannot wait takes items from
     (another iterator's adapter, `enumerate` or `zip` over it; an
     intrinsic given it as an `Iterable[T]`; a generator being driven so,
     whose own `Iter.next` on another generator) runs there and then to
     its next `yield` or its end, outside the schedule, as decision 236's
     closing runs one: no yield point switches and the budget and a
     `task.yield_now` are left as they were (natively, a task the
     generator made ready ends a starvation it found, which is what the
     interpreter's budget then does), so a seed's schedule is the same in
     both; its calls count on top of the asking code's (decision 234);
     an operation in it that would wait aborts the program
     (`panic.driven-wait`, `R0130`), since nothing could run the code
     that asked meanwhile. It used to panic with `R0091` (a generator
     inside an adapter gave only what it had yielded already), so
     `sum(evens(xs))` of a generator function stopped at its first item.
     Python's iterators are lazy where they are taken, and so these stay:
     `any` stops at the first `True` and closes the generator, which
     closes its program's iterator. Two native mistakes the cases found
     go with it: an intrinsic takes an owned argument of an `Iterable[T]`
     parameter whole, and its caller dropped it again after the call
     when the intrinsic's result could not hold it (`any(reversed(xs))`
     freed its iterator twice), and an iterator of references (a
     generator's of `ref T`) given for a `T` that is not one was summed
     as addresses rather than decayed to its items. This changes a rule
     of the type checker (`iter_from` and the mismatch) and the prelude,
     which stage 1 transcribes. `run/iterable_iterator`,
     `concurrency/generator_driven` (under twenty schedules) and
     `concurrency/generator_driven_wait` pin it.
248. A function value goes with what its environment holds: an `own`
     closure copies what it captures, and a copy of a capability handle is
     counted (decision 237), so a function value is dropped as a value
     that holds a handle is, in both implementations, rather than never
     (decision 224), which kept a captured handle's object to the end of
     the run (port-debt.md). Lowering's `needs_drop` holds for a type that
     holds a function value (`holds_capability` holds for function types)
     and SIR's `drops_something` for a function type, so drop glue drops
     such a field, a callee drops a function value it was given, and a
     copy of one is counted where a copy of a handle is. The interpreter
     counts and drops what a closure's environment holds, by the types of
     its function's environment parameter. Natively, a function value has
     one more word, its last: the type number of its environment plus one
     when what the environment holds is counted (an `own` closure of
     handles, `Cap`s or function values), else 0, so a copy or a drop
     calls the runtime only when the word is not 0; a closure of
     references, a task's closure and a named function have 0. This
     changes rules of lowering (the drops and counted copies of function
     values) and of SIR (`drops_something`), which stage 1 transcribes;
     every function that takes or keeps a function value drops it
     (`sir/closure`). `capabilities/closure_drops` pins it.
249. `RwLock` prefers its writers, the usual rule (POSIX's
     `PTHREAD_RWLOCK_PREFER_WRITER_NONRECURSIVE_NP`, Rust's futex lock),
     since neither 06 nor 11 names one: a task that waits to write is
     among the lock's waiting writers until it takes the lock, and `read`
     waits while a task writes or waits to write, so readers that keep
     coming cannot keep a writer out (decision 239 let them). A reader
     that asks again while a writer waits waits behind it, and a task that
     holds a read guard and asks for another then deadlocks, which the
     scheduler reports, as Rust documents for its lock. The interpreter
     keeps the waiting writers in the lock's value, the native runtime a
     count in a fourth word of the lock's header, which each writer adds
     to once while it waits. This changes no front-end rule.
     `concurrency/rwlock_writers` pins it under twenty schedules.
250. `sisu run`, `check`, `build`, `test` and `bench` take `--features
     NAMES` (09: features are Cargo's additive booleans, and Cargo's
     switch): the root package's features to turn on besides `default`,
     separated by commas or spaces, `DEP/FEATURE` for a dependency's, any
     number of times; and `--no-default-features`, which leaves the root
     package's `default` off. A name that is not a feature of the package
     it names is a usage error (`D0112`, status 2), as is any name for a
     single file, which declares none. In an attribute's arguments,
     `not(feature="tls")` is a call of `not` with a keyword argument, as
     `all(...)` and `any(...)` are, since a parenthesised expression
     cannot hold a keyword argument: the parser reads `not` followed by
     `(`, a name and `=` there as a name called, and `@cfg` takes
     `not(...)` of exactly one condition (`E0266` otherwise). Decision
     243's `not any(feature="tls")` stays a condition. This changes a rule
     of the parser (`not_expr` in an attribute) and of the driver's filter
     (`cfg.rs`), which stage 1 transcribes. `comptime/cfg_errors`,
     `packages/features` and the driver's tests pin it.
251. Reflection reaches every place 08 lets it be but a refinement's `where`
     clause and an `invariant` (still `E0900`, naming the place), by
     templates and unfolding as decision 244's are. A test whose body
     reflects is a template made once, like a function without type
     parameters, and so is each case of a parametrised test: the case's
     instance drops the header's `for PAT in LIST` and begins with `let PAT
     = CASE`. A constant's value and a field's or a parameter's default that
     reflects are unfolded once every declaration is resolved, from a text
     of their own (`<NAME>/MODULE.sisu`, where their errors are reported,
     `E0609` when one cannot be made), with the types they name written as
     their module names them, and then checked and evaluated as written
     ones. A `comptime def` that reflects is a template; its `T: type`
     parameters (08's `field_names(T: type)`) are generic parameters whose
     arguments a call writes among its arguments, by position or by name, so
     the function is made for each type it is given (`E0308` for one not
     given). A trait may declare `comptime def`s besides its `derive`,
     functions of the trait's module called `Trait.name(...)` at compile
     time (08 gives them no other form; `E0605` elsewhere). A trait's
     provided method that reflects is made in each impl that does not write
     it, as that impl's method, its code resolved in the trait's module; a
     trait's own method is never made for a call. A method of a trait impl
     is made once whatever its type parameters (the impl's, its own), which
     stay parameters, since a trait's call through a bound or a `dyn`
     reaches one function for every type argument: its instance is a method
     of `impl[A, ...] __S` (the impl's names, or `__G0`, ... for a method
     the trait wrote, whose code cannot name them), keeps the template's
     bounds and `where` clauses, and an alias in its module may stand for a
     type parameter. Reflection knows a type parameter by its name only
     (`reflect.name_of(f.ty)` of a field of type `A` is `A`), so another
     query about one stops the instance (`E0609`, "`A` is a type parameter,
     which is not known at compile time"), as does an impl for every type
     (`impl[T: B] Trait for T`), whose `Self` is one. A generic inherent
     method stays made for each type argument (decision 244), where its type
     parameters are known. Reflection over methods (08: reflection exposes
     "methods") is `reflect.methods_of` of a trait (its methods, in order)
     or of a type (its own, declared in it or in an inherent impl), each
     with `name`, `index`, `receiver` (`self`, `mut self`, `own self`, or
     empty for an associated function), `params` (each with `name`, `index`,
     `mode`, empty, `mut` or `own`, and `ty`), `ret` and `provided` (whether
     it has a body), and `reflect.is_trait`; a signature's types are known
     by name. A closure's block in a template may have `comptime for` and
     `comptime if` among its statements, unfolded where the block is, and a
     trailing `do` block is read for compile-time values as any closure is;
     `T.default()` on a type parameter is the type's associated function,
     not a compile-time value. The unreachable-code warning (`W0004`) is not
     reported in an instance, where a `comptime if` that holds may end what
     follows. This changes rules of the resolver (which functions, tests and
     values are templates or unfolded, the instances of trait impl methods
     and provided methods, `T: type` parameters, a trait's `comptime def`s,
     `methods_of`, `is_trait`, closures in the unfolding) and of the type
     checker (calls with `T: type` arguments and of a trait's `comptime
     def`, the eager requests, `W0004` in instances, the `E0900` places),
     which stage 1 transcribes. `reflect/trait_methods`,
     `reflect/methods_of`, `reflect/tests` (with its `.test`),
     `reflect/closures`, `comptime/comptime_defs`,
     `comptime/reflecting_values` and `reflect/methods_rejected` pin it,
     natively too but the last.
252. The modules of 11's table that compiler/std lacked are written in Sisu,
     in the table's order, each with its doc examples as tests (`sisu test
     --std`) and a case under `std/`: `std.par` (`join`, the chunks a
     reduction splits a list into, halves down to a grain, fixed by index,
     and `reduce` over a `Reducer` whose views merge up that tree, `Sum` in
     the prelude's pairwise shape, so a parallel sum is the sequential one
     bit for bit; stage 0 runs the chunks in order on the asking task, 06's
     serial elision); `std.io` (`Reader`, `Writer`, `Seek`, `BufReader`,
     `BufWriter`, readers and writers over bytes and standard input, `Conn`
     as both); `std.fs` (a path's parts from its text, walking a tree,
     copying, appending, lines); `std.rand` (xoshiro256** and ChaCha20, RFC
     8439, each a `Rand` every copy of the handle shares, and choosing and
     sampling); `std.encoding` (hexadecimal, base64 in both alphabets,
     LEB128 and zigzag varints, UTF-8 and UTF-16); `std.hash` (SipHash-2-4,
     FNV-1a, CRC-32 with its table computed at compile time, SHA-256,
     SHA-512, BLAKE3 with its chunk tree); `std.sort` (a stable merge sort
     of positions applied by swaps, so any type sorts in place, and
     introsort); `std.search` (`bisect_left`, `bisect_right`,
     `binary_search`, `partition_point`, `select_nth`, and D-075's
     `interleave`, `interleave_all`, `deepening` and `within` over any
     iterator); `std.process` (`Command`, a shell's word splitting,
     `checked`); `std.env` (variables as numbers, flags and lists,
     `require`, `expand`); and `std.regex`, RE2's syntax compiled by
     Thompson's construction and run by a Pike VM, linear in the text,
     leftmost-first with Perl's preferences and RE2's treatment of empty
     matches, a star whose body can match nothing compiled as Go does
     (`(x+)?`), and checked against Go's `regexp` on 2400 generated patterns
     and texts. The modules that existed grow: `std.num` has `BigInt` (limbs
     of nine digits, Knuth's division) and `Decimal` (a `BigInt` scaled by a
     power of ten, rounding only by `round` and `div` with a named
     `Rounding`, a float literal read from its shortest digits), checked
     against Python's `int` and `decimal`, and `checked_sub` and
     `checked_pow`; `std.math` the functions IEEE 754 fixes exactly and the
     constants; `std.json` `parse`, a `Json` value and `decode[T]` over
     reflection, the type as the schema; `std.string` a `Builder`;
     `std.collections` `SortedSet`. Host intrinsics are added for none of
     them. `x in xs` is `xs.contains(x)` through the prelude's new
     `Contains[T]` trait (02) on a type that is not a builtin collection or
     text: the checker writes the call where it checks a comparison of one
     `in` (`not in`, its negation), and the builtin collections and the std
     ones implement it, for generic code. Two rules change with it, both
     found by the modules: the expected type does not drive a call whose
     result type holds an associated type of a type its arguments have not
     told yet (`Iter[I.Item]` with `I` unknown), which bound the outer
     call's type parameter to the projection before it could be normalised
     (`within(deepening(f, 2), ...)` was `E0300`); and lowering counts the
     types a call names as type arguments among those its function mentions,
     since the callee may make one (`R.empty()` in a function generic over
     `R`), whose impl's methods were pruned (an internal error at the call).
     This changes rules of the type checker (`in` through `Contains`, the
     expected type of a call) and of lowering (the types a function
     mentions), and the prelude, which stage 1 transcribes. What 11 names
     and std still lacks: `std.mem` (D-060's allocator parameter),
     `std.reflect` as a module, `From` and `Range` in `std.core`, `SoaList`
     and the `persistent` collections, grapheme iteration and the
     normalisation forms (Unicode's tables), the elementary functions of
     `std.math` (Q-23), pipes and signals and the working directory (the
     host contract), time zones, decoding an enum from JSON (reflection
     makes no variant), and a `BigInt` literal beyond `int` (port-debt.md).
     `std/num_exact`, `std/par`, `std/io`, `std/fs`, `std/rand`,
     `std/encoding`, `std/hash`, `std/sort`, `std/search`,
     `std/process_env`, `std/json_decode`, `std/regex`, `std/text_math`,
     `traits/contains` and `std/doc_examples` pin it, natively too but the
     last.
253. The commonest intrinsics run inline in native code (port-debt.md's
     "Stage 2": stage 1's time was the runtime's, half of its instructions
     in `sisu_rt_intrinsic`), each doing what the runtime's does on the
     values' layouts and calling the runtime only where it raises a panic,
     grows a buffer or keeps state of its own: `len` of a list or `bytes`,
     `list.is_empty`, `first`, `last`, `pop` and, with room, `push`;
     `str.byte_len`, `byte_at` within the text and `byte_slice` at
     characters' edges; `char.code`, `char.from_code`, and `is_whitespace`
     and `is_alphabetic` of an ASCII character; `is_some` and `is_none`;
     `max` and `min` of two `int`s; `Iter.next` of a range or of a borrowed
     list's elements, whose state the iterator's object keeps in a header
     the code reads (`abi::ITER_KIND`: a range that would step past `int`
     ends, its end becoming its current value); and the sized integers'
     `&`, `|`, `^`, a signed `~`, `cmp`, widening and `to_int` but of a
     `u64`. `==` and `!=` of two texts compare their lengths, then their
     bytes (`sisu_rt_bytes_eq`), and of two values of an option or of a
     declared enum other than an error set their tags, then the fields a
     word holds, the runtime comparing any other variant's. Dropping a
     text, a `bytes` or a list whose elements hold nothing to drop frees
     its buffer inline. An iterator that a function makes once, by `range`
     or over a borrowed list's elements, and only steps, through
     references made for that, lives in a slot of the frame: it never
     leaves the function, the runtime never sees it, and its drop frees
     nothing. The runtime does less in what it still does: a dict's index
     maps a key's hash to the key's entries without hashing the hash
     again, a single entry without an allocation of its own, and a key is
     hashed a word at a time and mixed (nothing a program sees depends on
     the hash, since iteration is in insertion order: port-debt.md,
     "Hashing"); a value that owns nothing is copied as its bytes; and a
     drop skips the fields that need none. Built with a release `sisu`
     and run natively in release, best of three in turn, stage 1 from a
     copy of its package prints its SIR of its own package in 21.2 s
     against 39.7 s, of `examples/wordcount` in 0.34 s against 0.70 s,
     and its tokens and tree of the prelude in 0.063 s and 0.100 s against
     0.103 s and 0.157 s, each output byte for byte the same. This changes
     no front-end rule. `run/inline_operations` pins each operation at its
     edges, natively in the three modes.
254. `sisu run --from-sir FILE [ARGS...]` runs a file of serialised SIR in
     the interpreter, so that what stage 1 serialises is run as well as
     compiled: the reader and the validator of decision 212, then the run
     `sisu run --sir FILE` gives, with the schedule's options, what follows
     the file being the program's arguments, as what follows a source file
     is. With `--native` it builds and runs the program, as before; `sisu
     build --from-sir` still goes with `--native` (`D0110`), a build being
     native code; and `--from-sir` does not combine with `--watch`
     (`D0078`) or `--record` (`D0038`). The runner's mode
     `--interpreted-from-sir LIST` runs every case of `native.list` so,
     from the case's own `.sir` where it has one and from `sisu sir`'s
     output otherwise, and compares the run with the interpreter's
     expectations, the front end's warnings that a run from source prints
     first left out; `cargo test -p sisu-driver --test native` and CI run
     it beside the three native modes. This changes no front-end rule.
255. A parameter declared without a convention whose type is a reference
     (`t: ref mut Tally`; 04's rule 3 lets references be passed as
     arguments) is that reference, as a `mut` parameter's local is: the
     call passes the reference, and the body reaches the value through it.
     It used to be a borrow of the reference, unless the reference was
     `Copy`, so a `ref mut` parameter was a reference to a reference, and
     writing through it stopped both implementations with an internal
     error. In an `ensures`, `result` of a function that returns a
     reference (and a named part of a returned tuple that is one) is that
     reference, copied, rather than a reference to it, which a `ref mut`
     result was, with the same failure. This changes rules of the type
     checker (`param_local_ty`) and of lowering (`result` in a contract),
     which stage 1 transcribes. `ownership/reference_params` and
     `contracts/result_references` pin them, natively too.
256. The structural hash of a value, which `dict` and `set` use for keys
     and `hash` computes for any value a derived or prelude `Hash` impl
     would hash, mixes the value's parts into the hasher's state as
     `Hasher.write_int` mixes an `int`, `(state ^ n) *% 1099511628211`:
     an integer of up to 64 bits, a `char`'s code, a `bool` as 0 or 1 and
     a `DateTime`'s nanoseconds as one `int`; a 128-bit integer as its
     low word, then its high one; a text or `bytes` as its length in
     bytes, then each byte; a tuple's and a struct's fields in order; an
     enum's variant index, then its fields; a list's length, then its
     elements; a float as its bits; `()` and a handle as nothing. The
     interpreter (`Interp::structural_hash`) and the native runtime
     (`dict::structural_hash`) compute the same number, so a key hashes
     alike in both, and a structural hash agrees with a written impl that
     hashes the same parts in the same order. The hasher's state is its
     one field.
257. A type parameter bounded by `Escapable` holds no loan
     (`Program::can_borrow_in`): every type argument it can stand for must
     not, so a value of that type can be kept in an owner, sent to a task
     or stored in a `Shared` without the ownership checker's view rules,
     which 04's `Escapable` auto trait already requires of the arguments
     (a type is `Escapable` when nothing in it is a view). A parameter
     without the bound may hold a loan, as before.

**Round 10: `sisu doc`, and what round 9 left** (sections
"Documentation", "Contracts and refinements", "The ownership checker",
"The formatter", "Stage 2")

258. `sisu doc [PATH] [--output DIR]` and `sisu doc --std [MODULE...]`
     write the documentation (09, "Documentation"; 10, which leaves the
     form open) as Markdown, a page per module, made from the modules'
     syntax trees and what the front end resolved, with nothing evaluated,
     so that the front end in Sisu transcribes it (18's stage 1: "`sisu
     doc` as `pure` functions over the syntax tree"). The inputs are each
     module's tree as the build has it, the program's resolved names, and
     the error sets the checker inferred. The program is the one `sisu
     check PATH` takes (a file, a package, each member of a workspace, with
     `--locked`, `--features` and `--no-default-features` as there), built
     without its tests (decision 243); one with an error is reported as
     `sisu check` reports it, then `D0114`, and nothing is written (status
     1). The pages are the root package's modules' (a dependency documents
     itself); with `--std`, the standard library's, `std.core` (the
     prelude) and the modules of 11 (not `target`, whose facts are the
     host's), or those an import path names (`D0113` for another path).
     A page is `PATH.md`, PATH being the module's import path (`geo.shapes`,
     `std.x.http`) or a single file's stem, and a program's pages come in
     the order of their paths. Without `--output`, `sisu doc` prints each
     page after a line `<!-- PATH.md -->`; with it, it writes each into DIR
     (made when missing) and prints nothing. A package's pages show what
     another package may name, its `pub` items and in them the `pub`
     fields and methods (hidden fields leave a line `# private fields` after
     the others); a single file's and the standard library's show every
     item. A page is these blocks, each only when it has something, one
     blank line between two, a line break at the end:
     - ``# `PATH` ``;
     - the module's `##!` lines as doc text (below);
     - `## Modules` and a line per module with a page here whose path is
       the page's and one more name, in the order of paths: ``- [`CHILD`](CHILD.md)``,
       then `: ` and its SUMMARY when it has one;
     - `## Re-exports` and a line per name the module re-exports: each name
       a package module's `pub` imports bind, in order, or the prelude's
       names a std module names (decision 245's table): ``- `NAME`: [`TARGET`](LINK)``,
       TARGET being a module's path or `MODULE.ITEM`, and LINK its page, or
       its page and `#ITEM` for an item that is not a method, when that page
       is here (else ``- `NAME`: `TARGET` ``);
     - `## Contents` and a line per item section, ``- [`NAME`](#NAME)``,
       with `: ` and the SUMMARY of its doc comment;
     - a section per function, struct, enum, error set, trait, constant and
       type alias, in source order (imports, tests, suites, benchmarks,
       `extern` blocks and statements have none);
     - `## Implementations`: in a block fenced by "```sisu" and "```", the
       header of each impl of a type the module does not declare, after its
       doc comment, an inherent one's constants under it; then a section
       per method of the inherent ones, as a type's.
     A section is `<a id="NAME"></a>` and ``## `NAME` `` on the next line
     (`###` and `TYPE.NAME` for a type's or a trait's method), then the
     declaration in a block fenced by "```sisu" and "```", the doc text,
     and for a function `Unfinished: ` and its typed holes (each call of
     `todo` in its body where `todo` is the prelude's, as written, once,
     as inline code, joined by `, `, then `.`) and `Capabilities: ` and its
     parameters that hold a capability (``fs` (`Fs`)``, `self` first, its
     capability types in the order met, joined by `, `, then `.`): decision
     240's rule read from the resolved signature, a handle, `System`,
     `TestClock` or `Lent` reached through references, tuples, type
     arguments and the fields of structs and enums, or a type parameter
     bounded by an operations trait. Then a type's methods (those it
     declares, then its inherent impls', each kept or left as the page
     shows members) and a trait's, each a section. These parts are joined
     by blank lines. A declaration is its attributes, one a line; its
     header, the tokens up to the first line break outside brackets joined
     with one space where the source had a gap and none where a line break
     follows an opening bracket or comes before a closing one, a trailing
     comma before such a closing bracket and a final `:` left out, and for
     a `T !` function ` SET  # inferred` after it (`{}`, a member, or
     `{A, B}` in the checker's order, a member of another module written
     after its module's path); then, four spaces in: a function's
     `requires` and `ensures` as written, later lines keeping their
     indentation relative to the first, and for a `mut self` method of a
     struct with an invariant ``# re-checks the invariant of `T` on
     return``; a struct's fields as written, defaults too, then its
     invariants and constants (`const NAME: TYPE`); an enum's variants and
     `..` for an open one; a trait's associated types; a refinement's `def
     try_from(value: BASE) -> NAME[PARAMS] ! RefinementError`; then a
     type's inherent impls' constants; a field, a variant, an associated
     type and a constant each after its `##` lines, written `## TEXT` (`##`
     for an empty one); then, for a type with impls in the module, a blank
     line and the header of each, inherent or trait, in order, after its
     `##` lines. An impl is of a
     type by its name: a path's names joined by dots, `Option` for `T?`,
     `Result` for `T ! E`, a reference's type's name. Doc text is the `##`
     (`##!`) lines without the marker and the one space after it, the blank
     lines at either end left out, and a line "```" that opens a fenced
     block (an example, which `sisu test` runs) written "```sisu"; a
     SUMMARY is its first paragraph, the lines up to a blank one or a
     fence, each trimmed, joined by spaces. 10's rendering of an example
     with its values needs evaluation and is not here, nor `--open` nor
     `--digest` (port-debt.md). `sisu check --explain`, a stand-in that
     printed a file's signatures "as `sisu doc` renders them", is retired,
     and `NAME.doc` in the conformance suite is now `sisu doc`'s output
     (`# doc ARGS` gives other arguments). This changes no rule of the
     front end; stage 1 transcribes `sisu doc` with the driver's commands.
     `doc/items`, `doc/package`, `doc/std_pages`, `contracts/explain` and
     `contracts/proposal_contracts` pin it.
259. A test's instance stays a test once it is resolved. The resolver
     records an instance before it resolves it, and resolving it made the
     function's `FnInfo` again, as a free function's, keeping only the
     template it was made from, so the instance of a test whose body
     reflects (decision 251) was checked as a function: a `requires` in it
     was taken for a precondition rather than reported (`E0807`), a value
     it returned was `E0316`, naming `__instance_N`, rather than `E0381`,
     and its SIR function was not marked `test`. An instance whose template
     is a test now takes the test's kind when it is resolved. This changes
     a rule of the resolver, which stage 1 transcribes (its
     DISCREPANCIES.md noted the difference). `reflect/test_instances` pins
     it.
260. Two more of what stage 1 calls most run without the runtime's
     dispatch in native code, with the interpreter's results and schedule.
     `Shared.get` is the address its handle holds, after the report to
     `sisu_rt_shared_touch` that a dereference of a `Shared` makes
     (`--stats`); it was decoded and dispatched like any intrinsic. A read
     of a constant whose function only returns a literal (`const LIMIT: int
     = 5`, the shape lowering gives every constant of a number, a `bool`, a
     `char` or a `str`) is the literal, where it was a call: the read keeps
     what the call did that a run shows, its entry's yield point (decision
     204) and its count of nested calls, so a seed's schedule is the same
     and a recursion that reads a constant at the limit overflows there, in
     the constant's name (decision 234). Stage 1 built in release and run
     natively prints the SIR of `examples/wordcount` in 3.431 G
     instructions against 3.465 G (valgrind's count), and of its own
     package, with decision 262, in 128.0 G against 130.8 G, its output
     the same byte for byte. This changes no front-end rule.
     `run/constant_reads` pins both behaviours in the three native modes.
261. A statement's operands are used in order (decision 222), so what one
     of them moves is gone for the operands after it: `pair(s, s)` with two
     `own` parameters, `take(p.a, p)`, and the tuple `(t, t)` moved the
     same value twice, which the ownership checker accepted, since it
     checked every operand of a statement against the state before the
     statement; the interpreter stopped at the second use (`R0033`), the
     monitor called it a violation, and natively the value would have been
     freed twice. Now an operand that uses a value an earlier operand of
     its statement moved, or a part of it, is `E0400` (a value or what it is
     part of) or `E0401` (the whole after a part), the move labelled at the
     statement. Moving two different fields of one value in one statement
     (`both(q.a, q.b)`) stays accepted. The wrong-ownership corpus found it
     (`wrong/moved_twice_in_call`). This changes a rule of the ownership
     checker (`init.rs`), which stage 1 transcribes.
262. The native runtime copies a value that owns something (`.copy()`, and
     the copies the generated code asks for) by a plan made once per type
     when the type table is decoded: a struct or a tuple is its bytes, then
     each part that owns something copied again over them, the structs and
     tuples it holds in place opened into their own parts; an enum other
     than an error set the same, by the parts of the variant it holds; and a
     list of such values copies the elements' bytes at once and then each
     element's parts, where it looked at the element's type for every
     element. A list of texts or of options does the same by its element's
     kind. A struct or tuple holding a lock in place keeps the rule of its
     kind, since a copied lock starts unlocked. What a copy makes is the
     same, part for part, and nothing a program sees depends on the order in
     which the parts are copied (a copy runs no Sisu code; it counts handles
     and return traces, which nothing observes in order). It gains little:
     stage 1's copies are mostly of its bitsets' words (a `list[int]`,
     copied as its bytes either way) and of texts, whose cost is the
     allocation and the bytes; of `examples/wordcount`, 2.913 G
     instructions against 2.926 G without it, of its own package within a
     tenth of a percent either way. Copying the parts a word at a time
     rather than by `memmove` counted more instructions and is not kept.
     This changes no front-end rule.
263. The corpora of 18's Phase 3 that need no people. The wrong-ownership
     corpus is the programs of `wrong/` whose one error is a rule of the
     ownership model: the ownership checker's codes (`E04xx`) and the type
     checker's about ownership (`E0338`, `E0339`, `E0340`, `E0341`,
     `E0352`, `E0366`, `E0367`, `E0396`, `E0397`); fifty-two programs join
     the ten that were there, and `tests/wrong.rs` counts them (sixty-two,
     at least sixty) and holds each to one error that names its fix and to
     a program without errors once a fix with edits is applied. Naming a
     fix took a note on every diagnostic the corpus reached without one,
     each a catalog entry: `E0401` (`borrow.copy-before-move`), `E0366`
     (`type.assign-every-path`), `E0367` (the fix `type.declare-var`, as
     `E0312` has it), `E0341` through a borrow that only reads
     (`type.borrow-with-mut`), `E0352` (`type.call-instead`), `E0396`
     (`type.dyn-owned`), `E0397` (`borrow.consume-before-call`), `E0410`
     (`borrow.store-owned`), `E0412` (`borrow.owned-field`), `E0414`
     (`borrow.from-only-parameters`), `E0416` (`borrow.copy-part`), `E0420`
     (`borrow.consume-each`), `E0421` (`borrow.own-instead`), `E0422`
     (`borrow.consume-first`), `E0423` (`type.capture-copy`), `E0424`
     (`type.plain-closure`), `E0425` (`task.write-closure`), `E0426` and
     `E0437` (`task.make-inside`, when no other note applies), `E0430`
     (`task.join-inside`) and `E0433` (`task.return-send`). The translated
     slice of rustc's borrow-checker UI tests is `ownership/rustc/`: twelve
     of their shapes, rewritten in Sisu from memory of them, each pinning
     its code and its counterexample run, and the two-phase borrow they
     accept. Writing them found a hole and an over-rejection: two operands
     of one statement moving one value (decision 261), and a `mut`
     parameter read in its own method call (known over-rejection 15). The
     notes change what the front end reports in the human form (the
     identifiers' form does not show notes), which stage 1 transcribes.
264. `max` and `min` of two `int`s run inline, as decision 253 meant them
     to: its code took two `int`s, but lowering passes every argument of
     `min` and `max` by reference (std declares them generic, and a
     borrowed parameter whose type is a type parameter is passed by
     reference, decision 59), so no call matched and each went through the
     runtime's dispatch. The inline code now reads its two arguments
     through any references when both are `int`s and so is the result. A
     loop taking the `max` and the `min` of 20 million values runs in
     0.32 s against 2.43 s; stage 1 makes 3.3 million such calls printing
     the SIR of its own package. This changes no front-end rule.
     `run/inline_operations` pins both at their edges in the three native
     modes.
265. More of what stage 1 calls most runs without the intrinsic's
     descriptor (port-debt.md, "Stage 2"), whose decoding, finding the
     arguments through their references by the type table, and dispatch
     cost about two hundred instructions a call: the generated code calls
     an entry of the runtime that takes the values' words. Appending text to
     a `String` (`fmt.text`, `fmt.display` of a text, `String.push`) is
     `sisu_rt_text_push` with the text's address and length, `fmt.display`
     of an `int` is `sisu_rt_text_push_int`, which writes the digits
     without the allocation `to_string` made, a new `String` from a text
     (`str.to_string`, `String.new`) is `sisu_rt_text_from`, and `fmt.new`
     writes an empty `String` inline; `dict.get`, `dict.contains` and
     `set.contains` are `sisu_rt_table_find`, which gives the address of
     the key's value or whether the key is there; and a push compiled
     inline that finds no room calls `sisu_rt_list_reserve`, which grows
     the buffer as the runtime's push does, and then stores the element as
     with room. Each does what the runtime's arm did, with the same
     allocations, so `SISU_RT_ALLOCS` counts the same; an argument moved
     into the call, whose drop after it decision 218 gives the call, still
     goes through the descriptor. In the runtime, `==` and the order of two
     values look at each type once where they looked three times, and the
     `Display` text of an `int` inside any structural text is written
     without an allocation. This changes no front-end rule; the cases of
     `conformance/native.list` that build texts (`run/interpolation`,
     `run/format_specs`), look keys up (`ownership/dict_entries`) and push
     pin it in the three native modes.
266. The native runtime drops a struct without drop glue, a tuple, and an
     enum without glue that is not an error set by a list of the parts
     that need a drop, made once per type when the type table is decoded,
     where it looked at every field's type on every drop: a struct's
     fields and a tuple's elements last to first, the structs without glue
     and the tuples it holds in place opened into their own parts at their
     offsets, an enum's those of the variant it holds. A struct without
     glue drops its fields last to first (lowering gives glue to every
     declared type whose drop runs code or changes a count, so such a
     struct's drop only frees memory), as a tuple does, so the order of
     every drop is the one its kind's rule gives, and a part that panics
     leaves the others to be dropped as cleanup, a second panic aborting,
     as before. A list looks at its element's way of dropping once, and
     frees the texts of a list of texts without a call for each. This
     changes no front-end rule. `run/drop_parts` pins the order through
     tuples within tuples and a panic among their parts, and drops of
     values that only free memory, in the three native modes.

     Decisions 260, 262 and 264 to 266, measured on stage 1 as the round
     began (61 211 lines), built in release by a release `sisu` from a copy
     of its package and run natively, each output byte for byte the same:
     printing the SIR of its own package takes 104.6 G instructions
     (valgrind's count) against 130.8 G with round 9's compiler, and 19.6 s
     against 24.0 s (best of five in turn, medians 20.1 s and 24.7 s); of
     `examples/wordcount`, 2.91 G against 3.46 G. Stage 1 with its
     formatter merged (64 049 lines) prints its own SIR in 20.9 s against
     25.2 s. The arithmetic loops of decision 246 compile to the same
     instructions, but that a push without room calls
     `sisu_rt_list_reserve`, and their times move with where the link
     places them, since functions are not aligned: best of five against
     round 9's compiler, Collatz 1.21 s against 1.02 s and `fib(37)` 0.56 s
     against 0.43 s with the very same code, trial division 2.57 s against
     2.53 s, Euclid 1.56 s against 1.54 s, the sieve 0.27 s against 0.27 s,
     the product of matrices 0.63 s against 0.73 s; aligning functions to
     16, 32 or 64 bytes did not bring the slower placements back
     (port-debt.md, "Stage 2").
267. Two accidents of the formatter that stage 1's transcription found
     (its DISCREPANCIES.md, D6 and D7), decided for the written rule of
     decision 152 (D-079). Whether an option inside another is written
     `T?` depends on whether the outer one is, and the two questions asked
     each other, so `sisu fmt` overflowed its stack on `Option[Option[T]]`
     (status 134); now the inner option asks the outer one with itself
     taken as written `T?`, so the outer keeps its brackets:
     `Option[T?]`, and three deep `Option[T?]?`. And any space or line
     break inside `Option[ T ]` kept its brackets, after which the
     formatter's own check found that the result, `Option[T]`, read as
     `T?` while the input had not, and refused the file (`D0076`, status
     70); now only a comment inside keeps them, as for `Result[T, E]`.
     This changes a rule of the formatter (`sisu-fmt/src/lines.rs`,
     `option_sugar`), which stage 1 transcribes. `syntax/option_nested`
     and `syntax/option_spaced`, spelled out (`.unformatted`), pin them
     through the suite's formatting checks, which now count a crash of
     `sisu parse --shape` as a failure rather than as a syntax error, and
     `syntax/option_sugar`, their canonical spelling, through the
     canonical-form check and CI's loop.

**Round 11: stage 1 as the front end, and the release profile's checks**
(sections "The query layer and the driver", "Stage 2")

268. The front end of the `sisu` command is stage 1, the front end
     written in Sisu, compiled natively: it gives the Rust front end's SIR,
     diagnostics, pages and formatting on the whole corpus and reaches the
     fixed point (18's stage 3), the owner retires the Rust implementation
     as early as each of its roles has a replacement, and new language
     rules are written in stage 1 only, so the Rust front end becomes the
     oracle, which falls behind. `sisu check`, `sir`, `build`, `fmt`,
     `doc`, `tokens` and `parse` run stage 1's command of the same name
     with the options as given (`--format=ids`, `--locked`, `--features`,
     `--no-default-features`, `--contracts`, `--release`, `--output`) and
     print what it prints, its exit status theirs. `sisu build --native`
     runs stage 1's `build --native`, which checks, lowers and checks
     ownership as `sisu build` does and hands the SIR to the backend (`sisu
     build --native --from-sir`, decision 212): stage 1 calls
     `{home}/target/release/sisu`, so the driver gives it a home of links
     whose `target/release/sisu` is the running `sisu`, and whose `catalog/`,
     `std/` and `stage1/std/` (the modules only stage 1 reads) are the
     compiler's, and the executable links the runtime
     beside it, byte for byte the one the Rust front end's build makes.
     `sisu run` runs the same build with `SISU_KEEP_SIR` in stage 1's
     environment, which makes that backend keep the SIR it is given rather
     than compile it, and runs the SIR in the interpreter with the run's
     options (`--monitor`, `--stats`, `--schedule-seed`, `--explore`,
     `--scheduler`), as `run --from-sir` would (decision 254); `sisu run
     --native` builds with stage 1 and runs the executable (decision 197),
     a package's kept where stage 1 put it, which the backend's `built` line
     names. The stage-1 executable is `compiler/target/stage1/sisu1`, where
     `stage1/sisu1` builds it; the first command after a `.sisu` file under
     `stage1/src` became newer builds it, as the script does, with the Rust
     front end (`sisu build --native` in `compiler/stage1`), under a lock so
     that commands started at once build it once, into a file renamed over
     the old one. `--front-end=rust` among a command's options, or
     `SISU_FRONT_END=rust` for a whole process, keeps the Rust front end
     (`--front-end=stage1` names the default; any other name is `D0115`),
     and CI's `stage1/check.sh` runs under `SISU_FRONT_END=rust`, its
     oracle. The commands stage 1 does not serve run on the Rust front end
     whichever is chosen: `test` and `bench`, since stage 1 prints no SIR
     of a program built with its tests; `lsp`, `repl`, `ws`, `run --watch`,
     `run --record` and `replay`; `run --defer-borrow`, `check --diary`,
     `explain-borrow`, `audit` and `parse --shape` (port-debt.md, "The Rust
     front end's roles still to replace"). The driver does what stage 1's
     commands leave to their caller: it reports a missing file and a
     directory without a package before stage 1 runs, with the Rust front
     end's messages and statuses (`D0001`, `D0064`), runs a workspace's one
     application (`D0067`) and reports a program without `main` (`D0005`,
     `D0069`), takes the `error: Failed` and `error: Exit(status=N)` that
     stage 1's `main` returns off its standard error (N being the status),
     sets `PWD`, by which stage 1 knows the working directory, keeps the
     native runtime's variables (`SISU_RT_ALLOCS`, `SISU_SCHEDULE_SEED`,
     `SISU_STATS`, `SISU_TASK_STACK`) from stage 1, which is a native
     program, and reports the errors of a program whose stage-1 `sir`
     printed nothing with stage 1's plain `build`, as `sisu sir` reports
     them. A stage 1 that panics or aborts is an internal error (`D0118`,
     status 70); one missing or failing to build is `D0116` or `D0117`.
     Stage 1's `sir` prints no warnings, so the runner's native modes from
     SIR know that a run starts with the front end's warnings from the
     case's `.diag`. The stage-1 path uses none of the Rust front end's
     crates: it parses the command line, finds what a command works on
     (`package.rs`), runs stage 1, and hands SIR to the interpreter or the
     backend (`front.rs`, and the `stage1_*` functions of `cli.rs`), so that
     deleting the Rust front end deletes `FrontEnd::Rust`, the flag and the
     branches of `cli.rs` that read a program (`inputs_for`, `command`),
     once the roles above have replacements; building stage 1 is one of
     them, its seed then the previous stage-1 executable. Through stage 1
     the conformance suite passes whole, 505 cases and 5 075 checks, in 4
     min 20 s against 1 min 23 s through the Rust front end, which passes
     it whole too, on this machine's 4 cores with the debug `sisu`: stage 1 resolves the standard library
     at every command, so a small program's `check` takes 0.55 s, its
     `tokens` 0.02 s. This changes no rule of the language.
     `front::tests` and the driver's own tests hold the choice and the
     statuses, and every native and conformance test of `cargo test` runs
     through stage 1. CI follows (`.github/workflows`): the main job,
     `compiler` in `ci.yml`, builds stage 1 with its first command and runs
     `cargo test`, whose conformance suite and native modes go through
     stage 1 (the expectations, `.diag`, `.sir`, `.doc`, `.out`, `.err`,
     `.status` and the rest, against stage 1's output, and the programs run
     from stage 1's SIR in the interpreter and natively), and its
     canonical-form check is one run of stage 1's formatter over the
     standard library and the canonical cases; the `native` job's modes go
     through stage 1 too. The `stage1` workflow is the oracle's: the suite
     through the Rust front end (the runner's `--front-end=rust`, which puts
     `SISU_FRONT_END` in the environment it gives every command) and
     `check.sh` under `SISU_FRONT_END=rust`, both `continue-on-error:
     true`, since the Rust front end falls behind as stage 1 moves; stage
     1's own tests and its canonical form stay required there. All of it
     runs on pushes only, never on a schedule.
269. What a channel carries is `Send`, as the bound asks it, so a generic
     function makes a channel of its type parameter and a program a
     channel of closures, which decision 230 refused (a type parameter
     and a function value counted as views). A channel takes what one
     task sends to another, which may keep it after the sender's frame is
     gone: `channel[T]` and a send (`send`, `try_send`, a `select` arm's)
     of a value of a type parameter ask `T: Send`, and `Send` as a bound
     asks the type to own what it holds besides being `Send` by its parts
     (06, "Send, Sync, and shared state"; 04, "Views"), since a bound
     cannot see how long what a value borrows lives: with `Send` alone a
     generic function instantiated at `str` would hand its caller a
     channel of views, decision 230's bug. So a type parameter bounded by
     `Send` holds no loan, as one bounded by `Escapable` (decision 257),
     and `def pipeline[T: Send](own xs: list[T])` makes a `channel[T]` and
     keeps its values in a `Mutex`; a view is no `Send` type argument
     (`E0347`, with the note `borrow.own-instead`); what a task captures
     is still checked by its parts alone, since its scope sees how long a
     view lives. A missing bound is `E0347` with the fix `type.add-bound`
     naming `Send` (the first parameter in the way, for a type that holds
     some and would be `Send` with them bounded), once a body for each
     parameter; a concrete view or must-consume type is still `E0421` or
     `E0422` at the channel, and another type that is not `Send` is
     `E0347`. A function type is
     `Send` when its closure is, which the type does not say (Q-52), so
     each closure is checked where it is sent, as a task's where it is
     spawned: a send on a channel of a function type takes a closure
     written at the call (`E0425`, as `spawn`), which owns what it
     captures (a plain closure that captures anything borrows it:
     `E0421`, with the note `borrow.use-own-closure`), each capture owning
     what it holds (`E0421`, naming it) and `Send` (`E0426`); the halves
     of such a channel are `Send` and `Sync` and own what they hold, and
     a function type is still no `Send` type argument, since a generic
     body could send any closure of it. Lowering does not change: the SIR
     is what stage 0 writes for `[T: Send + Escapable]` and for a closure
     passed by value, and the interpreter and the native backend run it.
     Known over-rejections 13 and 14 narrow accordingly. Stage 1 decides
     it (its RULES.md, R1, with `cases/channels/`); the frozen Rust front
     end keeps decision 230's rule.
270. Release builds keep the checks of overflow, bounds and contracts (09:
     no profile turns them off; 07: the contracts the manifest names) and
     pay as little as the backend can for each, as the owner asked. In
     both profiles, a check is one branch the common path does not take:
     an overflow of `+`, `-` and `*` is tested on the processor's flag
     (Cranelift's `sadd_overflow`, `ssub_overflow` and `smul_overflow`,
     the branch on their flag result), an index by one unsigned comparison
     with the length, and the block that raises the panic, its constants
     made there, is written after the SIR block's terminator, out of the
     way of the code after the check; the SIR blocks every path from which
     ends in a panic or in unwinding (a failed contract's report and its
     `contract.fail`, a `panic`, the cleanup blocks) and each block's
     failure path are cold, placed after the function's other code. In the
     release profile the code generator also knows facts about each
     function's integers (`sisu-codegen/src/facts.rs`), a forward analysis
     over its SIR whose states are joined where paths meet (the hull of
     the bounds, the facts both paths have) and widened at a block that
     keeps growing, then narrowed again by the comparisons that still
     hold: the bounds of every integer and `bool` kept in a variable
     (decision 246, so no code changes one behind its back) and of the
     result, which only whole assignments change; the comparisons a
     `switch` on one establishes on each edge; which local holds `len` of
     a list that cannot change meanwhile, through a shared reference (04:
     nothing changes what one refers to while it lives) or in a local
     whose mutable borrows go only straight into a call whose result holds
     no reference, each such call counting as a change of it (and a write,
     a move, a drop or a new value of it, or of a part it lies in, as one
     too); and what a range iterator of the frame (decision 253) yields,
     from its start to below its end. With them it leaves out an index's
     test that they prove (`0 <= i` and `i` below that list's length, or
     below another's that a test found equal), the overflow test of an
     operation whose operands' bounds rule it out (the plain instruction),
     and the branch of a `switch` they decide, the test a dominating one
     implies or the type of what it compares decides (a jump), which is
     how a postcondition a precondition implies costs nothing; every
     answer is a proof, and where they know nothing the check stays. The
     facts are each function's own: a callee checks its preconditions
     whatever its caller knows, a refinement is its base in SIR, and the
     lists behind a `ref mut` are not followed (port-debt.md, "What the
     release profile's facts cannot see"). `SISU_NATIVE_TIMES=1` reports
     the index and overflow checks a release build compiled and those it
     left out, and the tests it decided: of stage 1, 565 of its 3 192
     index checks and 599 of its 1 405 overflow checks are left out, and
     255 tests decided. Built by a release `sisu` and run in the release
     profile, against the compiler before this decision, best of nine in
     turn, valgrind's count of instructions after each, every output byte
     for byte the same: Collatz's chains take 0.91 s against 1.14 s (7.82 G
     instructions against 8.13 G), `fib(37)` 0.44 s against 0.53 s (4.77 G,
     5.36 G), the product of 400x400 matrices 0.43 s against 0.61 s (5.17
     G, 6.39 G), the loop of `max` and `min` 0.24 s against 0.30 s (2.92 G,
     3.24 G), the sieve 0.27 s against 0.29 s (0.63 G, 0.66 G), a loop of
     `while k < len(xs)` over a list 0.058 s against 0.080 s (0.54 G, 0.74
     G), `dot` of two lists through a function with preconditions 0.112 s
     against 0.133 s (1.38 G, 1.46 G), and Euclid 1.52 s against 1.55 s
     (the same 4.44 G: nothing in its loop is a check the facts prove or
     one that fails into the code after it). Two run fewer instructions
     and take longer: a loop of `for i in range(len(xs))` over a list
     behind a reference, whose index is no longer tested, 1.14 G against
     1.20 G but 0.115 s against 0.090 s, and trial division 9.16 G against
     9.70 G but 2.60 s against 2.45 s, because Cranelift's register
     allocator, weighing the failure blocks where they now are, keeps the
     loop's counter on the stack where it kept it in a register
     (port-debt.md, "A check's flag is materialised before its branch").
     Stage 1, built so, prints the SIR of its own package in 20.9 s as
     before (medians 21.3 s and 21.5 s), and of `examples/wordcount` in
     2.863 G instructions against 2.874 G, its output the same: its time
     is the runtime's intrinsics' (decision 253), not its own checks'. A
     release build of stage 1 spends 3.1 to 3.3 s generating code, against
     2.7 to 2.9 s. `run/proven_checks` pins what the facts prove and the
     check they keep, natively in the three modes, and the driver's tests
     hold the counts of its release build. This changes no front-end rule.

**Unsafe code in stage 1** (22; stage 1's RULES.md, R2 to R9)

271. Unsafe code (22; D-080) is implemented in stage 1, the front end:
     an unsafe operation, which is a call of an `unsafe def` (a function,
     a method or a trait's method), a call of a foreign function, or an
     `unsafe impl`, is permitted only inside an `unsafe:` block or an
     `unsafe e` expression. A call outside one is `E0981` (`E0982` for a
     foreign function), labelled with the obligations of the callee's
     `Safety:` list; neither kind of function may be a function value
     (`E0983`). `Send`, `Sync` and the traits declared `unsafe trait` are
     implemented with `unsafe impl` (`E0984`, with the fix that writes
     it); `unsafe impl` of another trait or of none is `E0985`, of `Copy`
     or `Escapable` `E0986`; an impl's method is `unsafe def` exactly when
     the trait's is (`E0234`). An `unsafe def`'s body is no unsafe
     context. An `extern` block declares foreign functions, whose calls
     lower to the intrinsic `extern.LIBRARY.NAME` (`extern.c.NAME` for
     `extern "C"`), which marks them in SIR as `sim.rs` reads them; the
     blocks and expressions lower as the blocks and expressions they mark,
     so SIR is unchanged by `unsafe` and the interpreter and the backend
     run unsafe code whose callees' bodies they know. `@intrinsic` and the
     other attributes only the standard library writes are `E0225` on a
     program's methods as on its items, since a function with an
     intrinsic body would be an unsafe operation that needs no `unsafe`.
     Stage 1 decides it (its RULES.md, R2, with `cases/unsafe/`); the
     frozen Rust front end parses `unsafe:` and `unsafe e` and reports
     them as `E0900`.
272. The `# safety:` comment of unsafe code (22, D-083) is the run of
     comment lines immediately above the line where the statement holding
     an `unsafe:` block or an `unsafe e` expression starts, or the
     `unsafe impl` or the `extern` block (its attributes included), from
     its line that starts with `# safety:`; its parts name the obligations
     they discharge, names and a `:` at its start or after a `;` or a
     sentence's `.`. An `unsafe def`'s and an `unsafe trait`'s obligations
     are the ``- `name`:`` items of the `Safety:` paragraph of their `##`
     comment, and a trait impl's `unsafe def` without one has the trait
     method's. Stage 1 reports, as warnings of the program's own modules: a
     block or expression with no unsafe operation (`W0009`, with the fix
     that removes an expression's `unsafe`); a block, an expression, an
     `unsafe impl` or an `extern` block without its comment (`W0007`); an
     `unsafe def` or `unsafe trait` without its list (`W0008`); a comment
     naming no obligation of a callee (`W0010`); an `unsafe impl Send` or
     `Sync` whose parameter lacks the bound (`W0013`, with `type.add-bound`);
     and a public safe function, not `pure`, that calls a foreign function
     and takes no capability (`W0014`). Stage 1 decides it (RULES.md, R3).
273. Raw pointers are `std.mem`'s (22, D-081): `ptr[T]` and `ptr mut T`,
     builtins `ptr` and `ptr_mut` that a module imports (`from std.mem
     import ptr`; `ptr mut T` is the mutable form of the `ptr` in scope,
     `E0987` when that is another type), `Copy`, `Escapable` when `T` is,
     never `Send` or `Sync`, printed without their address. An `unsafe
     impl Send` or `Sync` asserts the trait for the arguments its bounds
     admit. `std.mem` declares the pointers' `unsafe def`s (intrinsics
     `mem.*`), `Layout`, `size_of`, `align_of`, `align_up`, `Allocator`,
     `Global`, `Region`, `RawBuf[T, A]` (whose `grow` is safe, and whose
     making, growing and dropping are `pure`, as `Region`'s are) and
     `Plain`, whose derive asks `@repr(c)` (or one field) of a struct and
     `Plain` of each field; `std.ffi` the C types, `CString` and `CStr`.
     The modules whose syntax only stage 1 reads
     (`std.mem`, `std.ffi`, `std.rt`) are in `compiler/stage1/std/` until
     the frozen Rust front end, which reads and formats `compiler/std/`,
     is retired; a derive template may come from one, and a derive's
     target shows reflection its attributes. Stage 1 decides it (RULES.md,
     R4).
274. An `unsafe` block does not make a function impure (22, D-085):
     inside a `pure` function, a call of a function that is not `pure`
     made in an `unsafe` block or expression is its author's assertion
     that the function stays observationally pure, which the block's
     comment explains under a `pure:` part (`W0011` otherwise); the
     ranges of such blocks go from the typed body (`TBody.asserted`)
     through lowering into `sir.Program.asserted`, which SIR's text does
     not write, and the purity check reports no call inside one. A
     foreign call and a machine intrinsic stop compile-time evaluation
     (`E0601`); code that compile-time evaluation writes holds no
     `unsafe` (`E0615`); a contract holds none (`E0988`). Stage 1 decides
     it (RULES.md, R5).
275. A package declares its unsafe code in its manifest, `unsafe =
     "memory"` or `"extern"` (22, D-082), which stage 1 compares with the
     package's code (its `unsafe` and `extern` tokens and C header
     imports): code without a declaration is `E1019`, foreign code under
     `memory` `E1020`, a declaration above the code `W0012`; a dependency
     that declares a level needs the root's `[trust] unsafe` (`E1021`). The
     lockfile records each package's level, and a rewrite that would
     raise one stops with `E1022` unless the command has
     `--accept-unsafe`. Stage 1 decides it (RULES.md, R6).
276. `sisu audit --unsafe` (22, D-083) is stage 1's: per package, its
     declared level and the program's consent; per module with unsafe
     code, its counts and the FNV-1a hash of its text; per block,
     expression, `unsafe def`, `unsafe trait`, `unsafe impl` and `extern`
     block, its line, its function and its comment or `Safety:` list; the
     package's foreign functions with what their callers hold; and the
     lints, as `check --format=ids` prints them. `--std` lists the
     standard library's. Stage 1 decides it (RULES.md, R7).
277. `std.rt` (22, D-086), internal to `std` (`E0701` for a program's
     import), declares the machine intrinsics `rt.switch`,
     `rt.stack_pointer` and `rt.thread_slot` as `unsafe def`s with their
     `Safety:` lists, and `Context`; compile-time evaluation stops at each.
     Stage 1 decides it (RULES.md, R8).
278. The first slice of the runtime in Sisu (22, D-086) is `std.mem`'s
     structural operations, generic code that compile-time reflection
     takes apart per type in place of `sisu-rt`'s walk over a type table:
     `structural_hash` mixes a value's parts in the order of decision 256
     (a float's bits worked out from its value, `f64_bits`),
     `structural_eq` compares them, `structural_copy` copies them anew,
     and `drop` drops a value at once through the compiler's drop glue,
     which already is the structural drop; tuples and arrays wait for
     reflection to take them apart. They are in `std.mem` rather than in
     the internal `std.rt`, since none reaches the machine. Stage 1's case
     `cases/runtime/structural.sisu` checks them against the interpreter's
     and the native runtime's answers. Stage 1 decides it (RULES.md, R9).
**Stage 2 in Sisu, step 1: LIR** (section "Stage 2", "LIR"; D-087)

281. LIR, the low IR of D-087, is the code generator's own calls of
     Cranelift's function builder, one per line and in their order, over
     values, blocks, slots and variables numbered as the builder numbers
     them, with the module's declarations and data in the order of the
     object file, and with what the debug information needs beyond the
     code said outright: each function's name, file and line, its named
     locals with their slots, and a description of each local's type, in
     the order the DWARF writer of decision 227 walks them. The other
     shapes were an SSA IR after the variables are resolved, which a
     lowering in Sisu would have had to build exactly as
     cranelift-frontend's SSA builder does (its block parameters and its
     value numbers) to give the same machine code, and a typed IR of its
     own, whose translation could not be checked byte for byte. Holding
     the builder's calls, LIR keeps every decision of `sisu-codegen` (the
     layouts and the type table, the frames, calls by address with the
     status byte, probes and yield points, checks as branches to cold
     blocks, glue, dispatchers and thunks), and a one-pass backend reads
     the same lines as typed operations on virtual registers in explicit
     blocks, the variables being SSA construction's. What LIR leaves out
     is what only machine code knows (where slots lie in the frame,
     registers, encodings, the line table's addresses, call frame
     information) and the probe reserve, which the backend measures from
     the frames it lays out. A `switch` says how many blocks were made
     before it, so that a backend numbering its blocks as it makes them
     puts the switch's own after them, without LIR describing how
     Cranelift's `Switch` splits its cases. The form is versioned (`lir
     1`).
282. Stage 2 (`compiler/stage2`) is a Sisu package with stage 1 as a path
     dependency for the SIR types, and lowers serialised SIR to LIR as a
     transcription of `sisu-codegen`, rule for rule: its modules mirror the
     Rust ones, and the program's state and one function's are one struct
     whose methods span the modules, as `lib.rs` and `func.rs` share
     theirs. It reads the SIR itself (`read.sisu`, a transcription of
     `sisu-sir/src/read.rs`), since stage 1 writes the form but has no
     reader of it. Both profiles are lowered, the release one with the
     facts of decision 270 (`facts.sisu`), whose states keep their facts in
     the order Rust derives for them, each fact coded as a list of integers
     ordered as that, since a lookup takes the first fact that fits. A
     type is known by its text (stage 1's `sir.write.ty`)
     where the Rust keys its maps by `Type`, which stage 1's SIR types
     cannot hash, and a float constant's bits are found by scaling by
     powers of two, which is exact, since Sisu has no `f64.to_bits`.
     `compiler/stage2/sisu2 lir FILE.sir [--release] [--output FILE.lir]`
     builds stage 2 natively on first use, through stage 1, in about 21 s,
     and lowers the SIR of stage 1's own package (493 018 lines) in 6.0 s,
     15 s in the release profile, to the LIR from which stage 1 is the same
     executable its SIR gives. A construct it does not lower stops it with
     `lir: not lowered: WHAT (in FUNCTION)` and status 1, as `D0103`
     stops the Rust backend.
283. The translation of LIR into Cranelift IR (`sisu-codegen/src/lir.rs`
     reads it, `translate.rs` replays it) makes each function's builder
     calls in the order given, declares a function's references to other
     functions and data at their first use, which `sisu-codegen` now does
     everywhere (its dispatchers and C's `main` declared some earlier, which
     changed only their numbering in the IR), compiles the functions on
     threads and places them in order as decision 219 does, C's `main`,
     which reads the probe reserve, once the others are placed, and writes
     the debug information from LIR's descriptions with the writer that
     describes SIR's types, walking them as it walks those. So the IR, the
     object file and the executable are the ones the program's SIR gives,
     byte for byte; `SISU_NATIVE_CLIF=DIR` writes each function's IR as
     it is compiled, to compare the two paths where they differ.
284. `sisu build --native --from-lir FILE` and `sisu run --native
     --from-lir FILE` compile LIR (without `--native`, `D0119`), and the
     conformance runner's mode `--native-from-lir LIST` lowers every case
     of `native.list` with stage 2 (`--sisu2 PATH`, else built with the
     `sisu` under test) and requires each run to meet the interpreter's
     expectations and each executable to be byte for byte the one
     `--from-sir` builds from the same SIR, and each case's release LIR
     (`sisu2 lir --release`) to the executable `--release --from-sir`
     builds; `cargo test` and CI's `native` job run it. All 200 cases pass,
     their executables identical in both profiles. This changes no
     front-end rule.
285. SIR carries what `sisu test` and `sisu bench` need, so that they run
     from a front end's SIR as `sisu run` does (decision 254) and stage 1
     can serve them: `sisu sir --tests` prints the program built with its
     tests (`@cfg(test)` holding, as `sisu test` builds it), each test's
     `@tag`s after its function (`test "edges/zero" test#1 tags "slow"`)
     and a line for each benchmark (`bench "doubling" bench#0`), whose
     functions lowering keeps as roots after the rest, where a program of
     one module's already reached them as its root module's functions, so
     no function moves (section "SIR", "The text form"). A suite is the
     prefix of its tests' names, `suite/test`, which the resolver already
     writes. `sisu test --from-sir FILE` selects the tests by name and tag
     (`--filter`, `--tag`, `--skip-tag`) and runs them as `sisu test` runs
     a program's, under the monitor unless `--no-monitor`, with the seed,
     `--explore`, `--sim`, `--jobs` and the snapshots beside the file
     (`--update`); what only the source gives is left out (the ownership
     checker's deferral, `--std`, transcripts, the cache, replay tokens).
     `sisu bench --from-sir FILE` runs the benchmarks. A program built
     without its tests prints as before, so no `.sir` expectation changes.
     `sisu sir --tests` runs on the Rust front end until stage 1's `sir`
     takes `--tests` and writes these lines, and stage 2 reads them and
     leaves them out, since stage 1's SIR types do not hold them yet. This
     changes no rule of the language.

**The unsafe round of stage 0** (sections "The checked memory model" and
"Raw memory, foreign calls and the machine intrinsics"; D-085, D-086)

301. The interpreter runs unsafe code on a checked memory model (D-085):
     raw memory is blocks with an identity, an address, a size, an
     alignment and whether they are live (`Global`'s, the emulated
     `malloc`'s, read-only foreign memory, and the program's places a
     pointer was made from), a raw pointer is a block, an offset, whether
     it came from a shared borrow and the monitor's loan it came through,
     and its type is the static type of the operand holding it. A block of
     `Global` holds values at offsets, each with its type, read back whole
     at that type (moved out when the type needs a drop) or through their
     bytes at another, for the types whose bits the model knows (the
     numbers, `bool`, `char`, and structs, tuples and enums of them), the
     bits checked for the type. Every access checks, in order, memory
     behind the pointer, liveness, bounds, alignment, initialisation,
     valid bits, and that a write goes neither through a pointer made from
     a shared borrow nor over a value never dropped; each failure is a
     panic naming the obligation of std.mem's `Safety:` list it broke
     (`panic.raw-*`, `R0131` to `R0148`); a release checks that the block
     is `Global`'s, live, of the layout given and holds no value never
     dropped, but while a panic unwinds. Under the monitor a pointer made
     from a place carries the loan of the borrow that made it and every
     access through it goes through that loan, a reference made from a
     pointer is a loan under it, and the monitor's rules for references
     apply (22: "applies the loan rules the monitor already applies to
     references"). The layout of a type in raw memory, which `size_of` and
     `align_of` give, is the native backend's (decision 302). Foreign calls
     run only for six functions of the C library the interpreter emulates
     on the model (`strlen`, `memcpy`, `malloc`, `free`, `getenv`, `write`
     to the standard streams; Q-55), and the machine intrinsics not at all
     (`R0149` to `R0153`). The intrinsics need their type arguments as
     stage 1 writes them, but `mem.size_of` and `mem.align_of`, whose `T`
     appears in no parameter and not in the result, which stage 1's rule
     for an intrinsic's type arguments leaves out today (port-debt.md,
     "Shortcuts in the interpreter"); a call without one is an internal
     error naming it. This changes no rule of the language: it implements
     22's and D-085's.
     `conformance/cases/unsafe/` runs it: a vector on raw memory, pointers
     to places, plain data, foreign calls, and a refusal for each check.
302. Native code runs the same unsafe code without the checks (22: the
     raw memory operations are the unchecked ones): std.mem's intrinsics
     are the runtime's, called through the descriptor and numbered after
     the older intrinsics, so that every program without unsafe code
     compiles to the same bytes; `ptr[T]` and `ptr mut T` are a word whose
     type table entry is `Ptr`. A type lies in raw memory as the
     interpreter's model lays it out: a sized integer narrower than 64 bits
     and an `f32` at their own width, structs and tuples in C's order, an
     option of a raw pointer as a nullable word, everything else as its
     value's words; that layout is C's for the types a foreign function
     takes, which a value's (decision 188, a word for every part) is not,
     so values whose two layouts differ are converted part by part at
     every read and write. A pointer to a place and a reference made from
     a pointer are therefore only made to a type whose two layouts are one
     (`D0103` otherwise, at the build), since a place and a reference hold
     a value as a value is laid out; `view_plain` of another type gives a
     copy freed with the bytes it views. The other choices were each part
     at its own width in values too, which changes every layout of
     decision 188 and stage 2's transcription with it, and raw memory laid
     out as values, which C cannot read (a `c_char` would take a word, and
     `strlen` would see one byte). A foreign call is a C call of the
     function the declaration names, with C types from the operands' and
     the result's types, after the runtime's standard output is written
     out; the machine intrinsics are the stack pointer read in place, a
     thread-local word of the runtime, and a switch routine of the
     runtime's that saves and loads `std.rt`'s `Context` (stack pointer,
     frame pointer, resume address), for x86-64 and AArch64 Linux, where
     corosensei's switch keeps its own frames and takes no such context.
     Stage 2 lowers none of it yet: what it must add is listed in the
     section above, and the unsafe cases of `native.list` are marked `not
     from LIR:` with the reason, which `--native-from-lir` leaves out
     (`native::read_list_from_lir`). This changes no front-end rule.
303. `sisu run --record LOG` runs on stage 1, as `sisu run` does: the log
     holds the SIR that ran (a `sir` line in its header, the SIR as its
     source and the front end's warnings in a `warnings` line), and `sisu
     replay LOG` runs that SIR again with no front end, printing the
     warnings first; a log of the Rust front end's still compiles its
     source again. So a program only stage 1 reads (unsafe code) is
     recorded and replayed, as the conformance runner does with every case
     that runs, and one more role of the Rust front end is stage 1's
     (port-debt.md, "The Rust front end's roles"). `sisu run --from-sir`
     takes `--monitor` too, which it read and ignored. A case whose run
     under the monitor stops where the plain run does not, because the
     monitor's checked memory model sees an aliasing violation of unsafe
     code that no static checker checks, has a `NAME.monitor` expectation
     (its output, `--- stderr`, its standard error, `--- status N`), which
     the runner compares instead of requiring the two runs to agree. This
     changes no rule of the language.
304. `sisu build --native` lowers SIR through stage 2 by default, as
     D-087's first step intends: the backend writes the SIR it is given (by
     stage 1's `build --native`, by `--from-sir`, or by the Rust front end)
     to a file, `compiler/target/stage2/sisu2 lir` lowers it (with
     `--release` in the release profile), and the Rust backend translates
     the LIR one to one (decision 283), which gives the executable the Rust
     lowering gives, byte for byte (decision 284); `sisu run --native`
     builds so too. `--lower=rust` among a command's options, or
     `SISU_LOWER=rust` for a whole process, keeps the Rust lowering, the
     oracle, which the runner's `--native-from-lir` mode builds the
     executable it compares with (`--lower=stage2` names the default; any
     other name is `D0120`); the driver gives stage 1 the choice in
     `SISU_LOWER`, so that the backend stage 1 calls lowers as the command
     chose. The driver builds stage 2 as it builds stage 1 (decision 268):
     when `sisu2` is missing or older than a source of stage 2 or of stage
     1's `sir` module, under a lock, with `sisu build --native` in
     `compiler/stage2` through stage 1 and the Rust lowering, into a file
     renamed over the old one (`D0121` when it does not build); stage 1 is
     built with the Rust lowering too, since stage 2 is built with stage 1.
     A program with a construct stage 2 does not lower yet (it stops with
     `lir: not lowered: ...`) is lowered by the Rust lowering instead, so
     that a construct `sisu-codegen` has and stage 2 has not transcribed
     (today the unsafe code of decision 302) still builds; any other failure
     of stage 2 is `D0122`, an internal error. So stage 2 is what `sisu`
     builds with, and the Rust lowering is the oracle and that fallback
     (port-debt.md, "The backend in Sisu"). `SISU_NATIVE_TIMES` reports the
     lowering's time, and `SISU_NATIVE_SURVEY` the Rust lowering's survey.
     This changes no rule of the language.

**Stage 1's diagnostics round** (stage 1's RULES.md, R10 to R19; the manual's findings)

291. A non-exhaustive `match` (`E0314`) names every pattern no arm covers
     (03), up to eight, in the order of the constructors their types
     declare, with the note `type.more-missing` past eight: where some arm
     names a constructor of a part of the value, each constructor no arm
     names is a pattern of its own with `_` for its fields; where none
     does, the part is `_`. Stage 1 decides it (RULES.md, R10); the frozen
     Rust front end names the first missing pattern only.
292. `sisu lint` (10) is stage 1's `lint`: the program checked as `sisu
     check` checks it, and its lints printed in either form, the warnings
     the compiler's own do not include; a program with errors has them
     reported and fails. Its first lint is D-043's (03): `W0015`, a
     reachable `case _` over an enum declared in the same package as the
     `match`, naming the variants it matches, with the fix that lists them
     in its place. The checker reports a lint with its diagnostics, and
     `Checked.lints` takes it out of them. Stage 1 decides it (RULES.md,
     R11).
293. Naming is enforced (02): the name pass reports a declaration whose
     name is not in its kind's case, with the name written so as the fix:
     `E0267` for a type, a trait, an associated type, a variant or a type
     parameter not in `CamelCase` (a `_` in it, or a lowercase first
     letter), `E0268` for a function, a method, a field, a parameter or a
     variable not in `snake_case` (an uppercase letter in it), `E0269` for
     a constant or a `const` generic parameter not in `SCREAMING_CASE` (a
     lowercase letter in it). A capitalised name in a pattern is a
     constant and not checked; an `extern` block's names are its
     library's. Stage 1 decides it (RULES.md, R12).
294. A trait is not a value: in an expression, a name that names a trait
     and a visible variant is the variant (the module's own first), called
     or bare, so `Add(left, right)` makes a variant named like the
     prelude's `Add` where no type is expected; with no such variant the
     name is the trait, `E0309` called and `E0350` bare. Stage 1 decides
     it (RULES.md, R13).
295. An interpolation reads its values: an optional chain of fields
     written in one (`{u.team?.name}`) reads what it reaches through a
     place, each link's field in place and the last one's value read
     when it is not `Copy` (a `String` as a `str`, an optional field as an
     optional reference), as `x?.f` through a reference borrows `f`; a
     chain over a value, or outside an interpolation, moves as before.
     Stage 1 decides it (RULES.md, R14).
296. An error set named as one of the prelude's hides it in its module
     and warns (`W0016`, D-024), since the two are different sets with one
     name. Stage 1 decides it (RULES.md, R15).
297. The diagnostics the manual's writers found without a fix get one
     (10): `E0411` on a field that borrows by `str` owns a `String`;
     `E0341` through a method's `self` takes `mut self`; `E0347` and
     `E0368` for a missing `Eq`, `PartialOrd`, `Ord`, `Hash` or `Copy` of
     the module's struct or enum derive it with the supertraits it lacks;
     `E0300` for a borrowed collection of non-`Copy` items given as an
     `Iterable[T]` notes why and copies it, and for text where a `Path` is
     expected calls `Path.new`; `E0320` in a function that returns nothing
     returns `! E` and `Ok(())`; `E0325` over text iterates its characters
     or bytes; `E0369` for a character in text writes it as text; `E0305`
     for `print(a, b)` interpolates; `E0200` for `open` names `Fs`, and
     for an assignment that declares a name writes `let` (or `var`), the
     name declared from then on. Python's `else` on a loop is `E0127`, an
     assignment on a `case` line `E0118` with the block it needs, and a
     hole whose value nothing uses is of type `()`. Stage 1 decides it
     (RULES.md, R16).
298. An intrinsic's call writes its type arguments in SIR also when one
     of its own generic parameters appears in no parameter, in its result
     or nowhere (`mem.size_of[int]()`, `mem.align_of[(int, bool)]()`),
     besides when one appears in its result and in no parameter or a
     parameter is an `Iterable[T]`; type argument `k` is the `k`th
     generic parameter, the impl's first. Stage 1 decides it (RULES.md,
     R17).
299. An associated function takes the type arguments of its own generic
     parameters after its name, `Type.function[Args](...)`, as a free
     function does: the type gives its impl's parameters, written or
     inferred, and the brackets the function's own (`E0208`, `E0349` for
     a function without them); a trait's function called on a type takes
     them the same way. A body of the standard library that does not
     type-check stops the compiler with an internal error naming the
     module and the diagnostic, where it lowered to `unreachable`. Stage 1
     decides it (RULES.md, R18).
300. An `@export`ed function's parameters and result hold no reference,
     view, `Shared`, `Weak`, closure or capability (22, "Foreign calls";
     `E0989` at each that does), since foreign code that calls it cannot
     keep them alive; `@blocking` marks a foreign function only (`E0990`
     elsewhere). Stage 1 decides it (RULES.md, R19); what the backend makes
     of either is stage 2's.

**Stage 2 in Sisu: the unsafe code, and stage 2 alone** (sections "Raw
memory, foreign calls and the machine intrinsics" and "LIR"; D-087)

305. Stage 2 lowers everything the Rust lowering lowers: the intrinsics of
     std.mem, `ptr` and `ptr_mut` with their `Ptr` entry in the type table,
     the build-time refusal of a pointer to a place or a reference from a
     pointer whose two layouts differ, foreign calls and the three machine
     intrinsics, each as `sisu-codegen` lowers it (decision 302), so the
     five unsafe cases of `native.list` build from LIR to the executables
     their SIR gives, in both profiles, and the list's `not from LIR:`
     marks and the runner's reading of them are gone. LIR gains the
     conversion `sextend` (a C function's `int`, `short` or `signed char`
     result widened back) and the type `i16` (a C `short` in a foreign
     function's signature); the alternative, a sign extension spelled as
     shifts, would not have been the Rust lowering's instructions and so
     not its executable. A foreign function is declared at its first call,
     with the C signature that call gives, where `sisu-codegen` declares
     it. With nothing left that only the Rust lowering lowers, a native
     build lowers through stage 2 alone: decision 304's fallback, which
     lowered a program stage 2 stopped on with the Rust lowering, is
     removed, and stage 2's `lir: not lowered: WHAT (in FUNCTION)` is the
     program's `D0103`, naming what and where as the Rust backend names
     it. The Rust lowering lowers only when `--lower=rust` (or
     `SISU_LOWER=rust`) asks for it: the oracle the runner's
     `--native-from-lir` mode compares executables with, and the seed that
     builds stage 1 and stage 2 themselves, since stage 2 cannot lower
     itself before it is built. `SISU_NATIVE_SURVEY` still lists a
     program's missing constructs through the Rust lowering's survey, which
     lowers what stage 2 lowers, and the build then goes on through stage
     2. The runner's modes that build with the default lowering (`--native`,
     `--native-from-sir`, `--native-release`) run each build with
     `SISU_NATIVE_TIMES=1`, take its reports off the standard error they
     compare, and fail a case whose build does not say `native: lowered by
     stage 2 in T ms`; their summary counts those cases and the time stage
     2 took, and `cargo test` requires every case of the list to have been
     lowered so. All 205 cases are, and all 205 build from LIR to
     identical executables in both profiles. This changes no rule of the
     language.

**Stage 2 in Sisu, step 2 begun: the x86-64 debug backend** (section
"Stage 2", "The Sisu backend"; D-087)

306. The x86-64 Linux debug backend written in Sisu is part of stage 2
     (`lirread.sisu`, `x64.sisu`, `x86.sisu`, `elf.sisu`, `object.sisu`):
     `sisu2 x86 FILE.lir --output FILE.o` compiles a LIR program to an
     ELF relocatable object, and `sisu build --native --backend=sisu`
     (`SISU_BACKEND=sisu`, which the driver gives stage 1 as it gives
     `SISU_LOWER`) lowers the program through stage 2, compiles the LIR so
     and links the object with `cc` against the runtime library
     Cranelift's objects are linked with, so that both backends' programs
     run on one runtime and their behaviour is compared on it. The default
     stays Cranelift (`--backend=cranelift`); another name is `D0123`, the
     Sisu backend with `--lower=rust` `D0124` (it compiles LIR, which the
     Rust lowering does not write), a construct it does not compile is the
     program's `D0103`, and any other failure of it `D0125`. The round's
     scope was the operations the `run/` cases of integers, booleans,
     control flow, calls, structs and runtime calls need; LIR's thirty-two
     operations are few enough that the backend compiles all of them but
     a jump with more than nine arguments and a return of more than one
     value, which no lowering makes, so its coverage list,
     `conformance/native-sisu.list`, holds every case of `native.list`.
     The runner's `--native-sisu LIST` builds each case so and compares
     its runs with the interpreter's expectations, and requires the build
     to say that stage 2 lowered the program and the Sisu backend compiled
     it (`native: compiled by the Sisu backend in T ms`); `cargo test`
     runs it. Stage 1 compiled by it (4 263 811 lines of LIR in 15 to 19 s
     on one thread) prints, on the 400 conformance files tried, what stage
     1 compiled by Cranelift prints, in 1.5 times the time. The other ways
     were an assembly text for the system's assembler (a process and two
     passes over text where the encoder is a few hundred lines, and the
     assembler's choices of encoding rather than the backend's), and a
     pass without the analysis, every value in its home (twice the memory
     traffic for the same simplicity). What the backend leaves is in
     port-debt.md ("The backend in Sisu"). This changes no rule of the
     language.
307. The Sisu backend's frame has a frame pointer, and every slot of it a
     fixed offset below the pointer, so that the body is written in one
     pass: 40 bytes for the callee-saved registers the allocator uses
     (`rbx`, `r12` to `r15`, each saved by the prologue only when used),
     then LIR's slots in order (each a multiple of 8 bytes, aligned to 8),
     the variables, the homes of the values that need one (8 bytes each,
     made as they are needed), and at the bottom the arguments calls pass
     on the stack; `rsp` is the frame pointer less a multiple of 16, so it
     is aligned at every call. The prologue (`push rbp; mov rbp, rsp; sub
     rsp, F`, the registers saved, the parameters that come in registers
     written to their homes) and the one epilogue every `return` jumps to
     are written after the body, which goes between them; a parameter
     passed on the stack has its home in the caller's frame. Calls follow
     the System V convention, of which Sisu's (decision 189) is the
     instance with `N + 1` addresses and a status byte: integers in `rdi`,
     `rsi`, `rdx`, `rcx`, `r8`, `r9`, floats in `xmm0` to `xmm7`, the rest
     on the stack, results in `rax` or `xmm0`. A value narrower than 64
     bits leaves its register's upper bits unspecified, as Cranelift's do,
     so a comparison, a shift, an extension or a test of a condition reads
     it at its width; a float is kept as its bits in a general register
     and moved to `xmm0` and `xmm1` for an operation. This changes no rule
     of the language.
308. The Sisu backend allocates registers block by block, after one
     analysis pass over the function that finds the block defining each
     value, whether another block uses it, the last use in its own block,
     and each block's parameters. A value is computed in the scratch
     registers (`rax`, `rcx`, `rdx` and the argument registers, which no
     value outlives an instruction in); written to its home when another
     block uses it, a block parameter's home by every edge into its block
     (all the edge's arguments read before any is written, a parallel
     copy) and an entry parameter's by the prologue; and kept in one of
     `rbx`, `r12` to `r15` while its block still uses it, a register that a
     call preserves, so that nothing is spilled around a call. When the
     five hold live values, the one whose last use is furthest is spilled
     to its home (Belady's rule, as cheap here as TPDE's round robin). A
     block starts with every value in its home, so nothing is reconciled
     at an edge. This changes no rule of the language.
309. The Sisu backend compiles LIR's checks as any other code: the
     overflow and bounds checks are branches to cold blocks that call the
     runtime's `sisu_rt_panic_overflow` and `sisu_rt_panic_index` and take
     the failure path (decision 270), `sadd_overflow`, `ssub_overflow` and
     `smul_overflow` are `add`, `sub` and `imul` with `seto` making the
     flag the branch tests, and `trap` is `ud2`. The status byte a function
     returns is `al`, put there by `return`; a caller tests it as an `i8`,
     at its width (`test al, al`), since the runtime's Rust functions do
     not define the rest of `rax`. Cold blocks stay where LIR has them,
     where Cranelift moves them after the others (decision 270): placement
     is a matter of speed, and this backend's is the debug profile's. This
     changes no rule of the language.
310. The Sisu backend emits the stack probe at a function's entry, the
     count of nested calls and the yield points as LIR writes them
     (decisions 203, 204 and 234): `get_stack_pointer` reads `rsp` after
     the prologue, so the probe compares the bottom of the whole frame
     with the task's limit, and the budget's decrement and the call of
     `sisu_rt_yield` are ordinary operations. The probe reserve that C's
     `main` hands the runtime (`probe_reserve`) is the largest frame of an
     unprobed function plus the largest top of a probed one, what it
     writes before its probe (the return address, the saved frame pointer
     and the 40 bytes of saved registers), plus twice the 256 bytes the
     Rust backend allows a call, so that the runtime keeps below a task's
     probe limit what this backend's frames can write there. This changes
     no rule of the language.
311. The Sisu backend's object is ELF64 for x86-64: `.text` (each
     function aligned to 16 bytes, the gaps filled with `int3`),
     `.rodata` (LIR's read-only bytes), `.data.rel.ro` (the data holding
     functions' addresses, with `R_X86_64_64` relocations, writable only
     while the loader relocates it), `.note.GNU-stack`, the relocations of
     the first and the third, and a symbol table holding the sections, each
     function as a symbol of its size (local, but C's `main`), and the
     runtime's and C's symbols the code names. The backend resolves the
     calls and addresses of the program's own functions itself; a call of
     the runtime or of C is `R_X86_64_PLT32`, the runtime's data is loaded
     through the GOT (`R_X86_64_REX_GOTPCRELX`, which the linker relaxes
     to an address), and the object's own data is reached with `lea` and
     `R_X86_64_PC32`, so the object links into a position-independent
     executable as Cranelift's (`is_pic`) does. This changes no rule of
     the language.
312. The Sisu backend writes DWARF 4 (`dwarf.sisu`), as the Cranelift path
     does (decision 227), beginning with what a debugger needs to stop at
     Sisu code and name it: a line table (`.debug_line`) with a sequence for
     each function that has a source, its first row the declaration's line
     at the function's start and each later row the `loc` before the code it
     starts (repeats of a line and column left out, the first marked as the
     prologue's end, so a breakpoint on a function stops past its prologue
     and probe), and a unit (`.debug_info`, `.debug_abbrev`) holding a
     subprogram for each function, with its name, its declaration's file and
     line, its code's range and the frame pointer as its frame base. The
     files are LIR's sources, the program's relative to the build's
     directory and the standard library's in its directory, which the driver
     passes as it passes them to the Cranelift path (`sisu2 x86 ...
     --comp-dir DIR --std DIR`). Without `.debug_frame` a debugger unwinds
     through the frame pointers every function has (decision 307). The
     parameters, locals and their types, which LIR describes (`local`,
     `type`), are not written yet (port-debt.md, "The backend in Sisu").
     `cargo test` runs gdb on a program built with `--backend=sisu` (a
     breakpoint on a function and on a line, and the backtrace's Sisu
     functions and lines), else lldb, else the line table as `readelf`
     decodes it. This changes no rule of the language.
313. The `Debug` impls of `f64` and `f32` that the front end writes apply a
     format spec as their `Display` impls do (`Spec.float`, `Spec.float32`),
     since the two texts are the same: `{x=:.2f}` rounds (02), where the
     precision was dropped and the `Debug` text padded; a spec of type `?`
     still pads the `Debug` text. Stage 1 decides it (RULES.md, R20).


**The Sisu backend's fixed point and its default** (section "Stage 2",
"The Sisu backend"; D-087, step 2)

330. The Sisu backend's fixed point, D-087's criterion for step 2, holds,
     and `compiler/stage2/bootstrap.sh` checks it: sisu2-C, stage 2 as the
     Cranelift path builds it (stage 1's front end, the Rust lowering and
     Cranelift, the seed of decision 305), lowers and compiles the SIR of
     stage 2's package and of stage 1's with the Sisu backend into sisu2-A
     and stage1-A; stage1-A, the front end compiled by the Sisu backend,
     prints both packages' SIR again, and sisu2-A, the backend compiled by
     itself, lowers and compiles that SIR into sisu2-B and stage1-B. Each B
     is its A byte for byte (stage 2 14 979 976 bytes, stage 1 39 824 256,
     with the pipeline of decisions 334 to 336),
     and stage1-A prints the SIR `sisu sir` prints, so building once more
     changes nothing: the lowering and the backend are deterministic
     (Sisu's dicts and sets iterate in the order of insertion, and nothing
     they write reads an address, the time or the environment), and
     nothing the Sisu backend miscompiles changes what stage 2 and stage 1
     write. The script prints each file's size and SHA-256, as stage 1's
     `bootstrap.sh` does, and exits non-zero when a pair differs; CI's
     `bootstrap` job runs it on every push. Native builds take
     `SISU_STAGE2=PATH`, a stage-2 executable they lower and compile with
     as it is (neither built nor checked against the sources), which stage
     1 passes on to the backend it calls: the script builds sisu2-B with
     sisu2-A so. The other ways were the comparison of LIR and objects
     alone, which leaves out the link and the front end compiled by the
     backend, and a fixed point of stage 2 only, which leaves out the
     larger program. This changes no rule of the language.
331. A debug build's backend is the one written in Sisu: `sisu build
     --native` and `sisu run --native`, from source, `--from-sir` or
     `--from-lir` of a debug LIR, compile stage 2's LIR with the x86-64
     backend in Sisu, and `--backend=cranelift` (or `SISU_BACKEND=cranelift`)
     asks for Cranelift, which stays as its oracle. A release build's
     backend stays Cranelift, whose `speed` optimisations stand in for the
     LLVM release backend of D-088, since D-087 gives release builds to
     that backend and not to the debug one, whose code is unoptimised
     (`--backend=sisu --release` compiles a release LIR with it); the Rust
     lowering, the oracle, which writes no LIR, keeps Cranelift too, so it
     needs no backend named. A command that names no backend leaves the
     choice to each build's profile, and the driver gives stage 1
     `SISU_BACKEND` only when the command named one. The runner's
     `--native` and `--native-from-sir` modes build with the Sisu backend
     and require each build to say so (`native: compiled by the Sisu
     backend in T ms`); `--native-cranelift` builds every case of
     `native.list` with Cranelift and compares its runs with the same
     expectations, so that each case behaves the interpreter's way under
     both backends; the mode from LIR names Cranelift, whose executables it
     compares with the Rust lowering's (decision 284); and `--native-sisu`
     and `native-sisu.list`, which held the cases of `native.list`, are
     gone. What a debug build gives up for now: the Sisu backend's debug
     information is the line table and a subprogram for each function
     (decision 312), so a debugger stops at Sisu lines and names the
     functions of a backtrace but shows no parameters, locals or types,
     which `--backend=cranelift` still describes (the test of decision
     227's DWARF builds so), and its code is naive (port-debt.md, "The
     backend in Sisu"). Stage 1 and stage 2 themselves are still built
     with the Rust lowering and Cranelift, the seed. `D0123` names the two
     defaults. This changes no rule of the language.

**The pipeline in one process and on threads** (section "Stage 2", "The
Sisu backend" and "Tasks"; D-069, D-087)

332. The words the generated code reaches by symbol, the count of nested
     calls (`SISU_RT_DEPTH`), the budget (`SISU_RT_BUDGET`), the running
     task's stack limits (`SISU_RT_STACK`) and the site of the last call
     that may wait (`SISU_RT_SITE`), are thread-local, each thread's own,
     so that Sisu code runs on several threads at once (decision 333). The
     runtime defines them in the executable's thread-local data (`.tbss`)
     in assembly, since stable Rust gives no thread-local variable a symbol,
     and reaches them as the generated code does, through the initial-exec
     model: the thread pointer plus the variable's offset, which the GOT
     holds and the linker relaxes to a constant in an executable (x86-64
     and AArch64 Linux; elsewhere they stay ordinary data). LIR says so:
     `data dN import tls NAME` declares one, and `vN = tls_value dN` is its
     address in the running thread, which the translation makes
     Cranelift's `tls_value` (with `tls_model=elf_gd`, whose sequence the
     linker relaxes too) and the Sisu backend `mov rax, fs:[0]; add rax,
     [rip + NAME@GOTTPOFF]` (`R_X86_64_GOTTPOFF`, the import an `STT_TLS`
     symbol). Both lowerings write `tls_value` where they reached those
     words with `symbol_value`, so every executable changed and LIR still
     gives the Rust lowering's executable byte for byte. The runtime's
     other state that Sisu code on two threads could reach is made safe
     for it: a drop's forgetting (decision 223) is per thread, and the
     table of views (decision 214) is shared under a lock. The other ways
     were the words left global with Sisu code on one thread only, which
     is no parallelism, and the words reached through a call of the
     runtime, which would put a call at every function's entry. This
     changes no rule of the language.
333. Natively, `std.par.map` runs its bodies on a pool of worker threads
     apart from the task scheduler, as 06 and D-069 describe std.par: the
     native backend compiles the body of std's `par.map` (`<std>/par.sisu`)
     as a call of the runtime's parallel map (`par.map`, an intrinsic
     numbered after the others that no SIR names) and the drop of the
     function `map` owns, the same body in both lowerings
     (`sisu-codegen/src/bodies.rs`, stage 2's `gen.native_bodies`), while
     the interpreter runs the SIR as written, the serial elision. The
     runtime starts its workers at the first call, as many as the machine
     has processors (`SISU_PAR_THREADS=N` sets another number), each on a
     1 GiB stack with its own words (decision 332), the asking task's count
     of nested calls, a budget that never runs out and a scheduler of its
     own for the generators a body drives; the workers take the elements
     by index and write each result at its element's place, so the list is
     the elision's whatever the number of workers. When bodies panic, the
     lowest index's panic is the call's, as the elision raises the first:
     the results before it are dropped while it is in flight, as the
     elision's unwinding drops them, those after it freed without running
     Sisu code, and the other panics forgotten. A call made on a worker
     runs its bodies there, one after another. A body spends no yield
     point of the asking task's budget, so a program whose tasks run while
     one of them maps may switch at other points natively than in the
     interpreter, which runs the bodies on the asking task (port-debt.md).
     `conformance/cases/std/par_map` holds the elision's results and panic
     natively and from LIR. The other ways were an intrinsic that std.par
     calls and the interpreter implements, which would change the shared
     standard library and the interpreter for what only native code needs,
     and a pool shared with the scheduler, which Q-6 leaves open. This
     changes no rule of the language: it implements D-069's pool.
334. `sisu2 build FILE.sir --output FILE.o [--release] [--times]` lowers a
     program and compiles its LIR in one process: the LIR's text stays in
     memory with where each function is in it (`gen.lower_spanned`: the
     start of its head's line, the end of its `end` line and its lines), so
     the backend's pass skips a function's lines without reading them. The
     backend compiles the functions in batches of 256 through
     `std.par.map` (decision 333), each function's lines read
     (`lirread.read_function`) and compiled on a worker, and places each
     batch's functions in the file's order, C's `main` once every function
     before it is placed, since their frames give the probe reserve it
     reads (decision 310); the object's bytes are made `bytes` on the
     workers too. The object is byte for byte the one `sisu2 lir` and
     `sisu2 x86` give, and the same on one worker as on four. A debug
     build of the driver runs `sisu2 build`, whose `--times` report gives
     `SISU_NATIVE_TIMES` the lowering's and the backend's times, and
     `sisu2 x86` compiles a LIR file (`--from-lir`) on the workers as well,
     finding the functions' ends by their lines. On stage 1 (4 263 811
     lines of LIR, four cores, a quiet machine): the backend took 15.5 s
     on one thread; it now takes 9.4 s from the LIR file (18.3 s on one
     worker, the functions' lines read twice), and `sisu2 build` lowers in
     6.7 s and compiles in 6.0 s, 13.9 s in all where `sisu2 lir` and
     `sisu2 x86` took 7.9 and 15.5; Cranelift compiles the same LIR in
     3.7 s. A whole native build of stage 1 takes 82.5 s with the Sisu
     backend and 77.6 s with Cranelift, its front end most of both. Under
     callgrind, `sisu2 x86` over stage 2's LIR runs 21 % of its
     instructions on the main thread, most of them finding the functions'
     ends, which `sisu2 build` does not, and the rest on the workers. The
     other ways were the
     lowering handing the backend operations rather than text (a larger
     change, for the reading that is now on the workers), and placement
     overlapping the compiling of the next batch, which `std.par`'s
     fork-join does not give. The `sisu2` script builds stage 2 with the
     Rust lowering and Cranelift, the seed, as the driver does. This
     changes no rule of the language.

**The runtime in Sisu, begun** (section "Stage 2", "The runtime in Sisu";
D-086, D-087 step 3)

335. Both lowerings compile the raw memory of a word or a byte inline,
     without the runtime: `mem.ptr.addr`, `mem.ptr.cast`,
     `mem.ptr.cast_mut`, `mem.ptr.as_const` and `mem.ptr.from_address` are
     the pointer's word, and `mem.ptr.offset`, `mem.ptr.read` and
     `mem.ptr.write` of a pointer to a type whose raw memory is its value's
     word (`int`, `f64`, `bool`, `char`, `u64`, a raw pointer) or to a `u8`
     or an `i8` are the address plus the offset times 8 or 1, a load (of the
     byte, extended with its sign for an `i8`) and a store (of the byte,
     `ireduce.i8`), as `sisu-rt/src/rawmem.rs` reads and writes them; the
     other types still go through `sisu_rt_intrinsic`, which converts them
     part by part (decision 302). So code over raw memory needs no type
     table and no call, which the runtime in Sisu needs (decision 336), and
     the unsafe cases build from LIR to the Rust lowering's executables in
     both profiles. This changes no rule of the language.
336. The runtime in Sisu is a package, `compiler/rt`, compiled once into
     an object (`compiler/target/rt/rt.o`) that every native program is
     linked with, whichever backend compiles it, before the runtime in
     Rust. Each `export_NAME` function of its root module (no type
     parameters) is the global symbol `sisu_rt_NAME`, of Sisu's
     convention, and what it calls is compiled with it under local
     symbols; the generated code calls it as it calls any function. Its
     code is the runtime's own, as drop glue is: no count of nested calls,
     no yield point, no line of a stack trace, and no value walked by a
     type table, which the object has none of (a construct that needs one
     is `Unsupported` when it is built), so it reaches raw memory through
     the operations decision 335 compiles inline. The object is built from
     the package's seed, `compiler/rt/rt.sir`, the SIR stage 1 prints of
     it (a test holds the seed to that), by the Rust lowering in its
     library mode (`sisu_codegen::compile_library`) and Cranelift, when it
     is missing or older than the seed (`D0126` when that fails): the
     runtime must exist before stage 1, which a native program is too and
     which alone reads std.mem, can compile it, as the Rust lowering seeds
     stage 1 and stage 2. The first piece is `sisu_rt_bytes_eq(ret, a, b,
     n)`, the comparison of two texts' bytes that `==` of two texts of one
     length calls (decision 253), now `export_bytes_eq` in Sisu, a byte at
     a time; its Rust counterpart is deleted, and both lowerings call it
     with the result's and the arguments' addresses and test its status,
     so LIR still gives the Rust lowering's executables byte for byte. The
     other ways were the runtime in Sisu compiled with each program, as
     its own functions (which is D-086's end for the structural operations,
     per type, but compiles them again for every program and changes every
     program's SIR), and a C convention for the exports, which Sisu code
     cannot define. What is left (the structural operations by their
     parts, formatting, text search, lists over `RawBuf`, stage 2's own
     library mode) is in port-debt.md ("The runtime in Sisu"). This
     changes no rule of the language.

337. The Sisu backend's objects carry a file symbol, `sisu_program`
     (`STT_FILE`, the first symbol after the null one), the name
     Cranelift's objects carry. Without one, GNU ld gives an object's
     local symbols a file symbol made from the object's file name, which
     `sisu build` takes from the output's (`NAME.o` beside it), so an
     executable's bytes depended on its name: CI run 41622372606, on a
     runner whose `cc` links with GNU ld, found the fixed point of
     decision 330 failing by one byte of `.strtab` (`sisu2-A.o` against
     `sisu2-B.o`), and the driver's tests of one executable built from
     source and from SIR, and on one and three workers, failing the same
     way, where lld, which the development machine's `cc` uses, adds no
     such symbol. The byte-for-byte comparisons now say where two ELF
     files differ, by section (`sisu-conformance --elf-diff A B`, used by
     the tests and by stage2/bootstrap.sh), and CI keeps a failed fixed
     point's files as an artifact. This changes no rule of the language.

**Stage 1's soundness round** (stage 1's RULES.md, R21 to R26; the manual's later findings)

320. A collection's items are values it owns or views, never references
     (04, rule 3): a `ref T` as the item type of `list`, `set`, `dict`,
     `Array` or a collection of `std.collections` is `E0438`, where a
     type writes it (with the fix that takes the `ref` away) and where a
     body's local or expression has it (with the fix that collects
     copies, `.map(it.copy())`, or the note naming copies and views),
     once a body for each collection type; a shared borrow of a `str` is
     the view. Generic bodies and the code the compiler writes are
     checked with their type parameters opaque, which may stand for a
     reference. Stage 1 decides it (RULES.md, R21); the frozen Rust front
     end treats such a collection as a view.
321. The checker refuses what it let through: an impl's method whose
     parameter or result types are not its trait's (`E0234`, as stage 0's
     resolver has it, where stage 1 compared the shape only); `impl Copy`
     written by hand (`E0270`, with the fix that derives it, since `Copy`
     is opt-in through `@derive(Copy)`, 03); a `dyn` of a trait that
     cannot be one in a body's types (`E0241`, as in a signature); a call
     of a trait's method whose impl nothing decides (`E0324`, the call
     named as the expression to annotate). An `assert` whose condition
     moves a value shows its comparison's sides, computed once, rather
     than its parts, which it would evaluate again. Stage 1 decides it
     (RULES.md, R22).
322. A sized integer's compound assignment (`+=`, `+%=`, `+|=` and the
     others) lowers to its operator's intrinsic at its width
     (`u8.wrap_add`), as the binary operator does, where it lowered to
     the `int` operation; and a borrowed parameter whose type holds the
     function's type parameters is taken by value exactly when their
     bounds make it `Copy`, every call passing it so: a call with a
     `Copy` instance of a type the function takes by reference (a tuple,
     an option, a type that derives `Copy`) borrows its argument. Stage 1
     decides it (RULES.md, R23).
323. The fixes and notes the manual's later chapters found missing: `E0304`
     writes Sisu's spelling of Python's and Rust's methods (`names.join(",
     ")`, `push`, `copy`, `trim`), iterates a collection before an
     iterator's method, and bounds a type parameter by the trait whose
     method it lacks; `E0300` converts text with `.to_string()`; `E0340`
     at a call whose function changes the parameter offers the
     function's `mut`, as `E0341` does, where the two undid each other;
     `E0413` and `E0407` on text name a `String` result; `E0400`'s note
     names what each remedy costs and the borrowed parameter; `E0243`
     lists `Default`; and `from pkg import x` binds a name `pkg`
     re-exports, the import waiting for the module's own. Stage 1 decides
     it (RULES.md, R24).
324. `with e:` without `as` keeps the value without a name to the end of
     the block (02); `ref` of a value that is no place borrows a temporary
     to the end of its statement; a `Box` has its value's `Len` and a
     `Slice` a `Display`; a derived `Hash` or `Eq` is structural when the
     fields are, whatever the type arguments no field holds (`Key[T]` as a
     key); a trait method's type arguments are its own, and one that takes
     none is `E0349` (`collect[set[int]]()`, which made a list). Python's
     `sorted(xs, key=len)` names `sort_by`, `max_by` and `min_by`, a
     closure kept in a `let` without its parameters' types is `E0324`,
     and `import *` is `E0128`. Stage 1 decides it (RULES.md, R25).
325. A generic struct's construction takes its type arguments from the
     type expected, when that type is the struct's and holds no `dyn` and
     nothing untold, before its fields are checked (`Reading(21.5, "°C")`
     as a `Reading[f64, String]`); and a field's default in a generic
     struct is a value function generic as the struct is, which each
     construction calls with the struct's type arguments, where lowering
     named the struct's parameter in the caller (`E0501`). Stage 1 decides
     it (RULES.md, R26).

**Stage 1's third round** (stage 1's RULES.md, R27 to R36; typed holes, trait contracts, attributes, exact numbers and JSON derives)

340. Typed holes (D-061): a release build rejects each with `E0445`, which
     names the hole and its type and the `panic("...")` that may ship,
     where `check` and a debug build warn (`W0001`); a run that reaches a
     hole prints its closure first, `in scope at the typed hole
     `todo("name")`:` and a line `NAME = VALUE` (`Debug`) for each binding
     that holds its whole value there and holds no loan or is a parameter,
     whose `Debug` the host writes and which holds no capability, a
     closure's captures after its own; and `sisu test` exits with 3 when
     no test failed and one is blocked, 1 when one failed, 0 when all
     passed. The closure is written into the SIR after the ownership
     checker and compile-time evaluation, which it changes in nothing.
     Stage 1 decides it (RULES.md, R27).

341. Every method of an `Atomic` but its construction is impure in `pure`
     code (`E0333`, named as the program calls it, with a note that it
     observes the schedule), as 06 lists `Atomic` among the operations
     that observe the schedule; and the body of a `par` loop (`for x in
     par.iter(xs):`) is `pure`: it calls only `pure` functions (`E0446`)
     and changes nothing declared before the loop (`E0447`). Stage 1
     decides it (RULES.md, R28).

342. A trait's method's `requires` and `ensures` bind every impl (07):
     an impl's method checks the trait's preconditions on entry before its
     own and the trait's postconditions on exit after its own, with the
     trait's names of the parameters, and a failure names the trait's
     method; a trait's method whose body is its contract alone is
     required, not a default. An impl in another module than the trait's
     is warned that stage 1 does not check it yet (`W0021`). Stage 1
     decides it (RULES.md, R29).

343. A call of a function or method marked `@deprecated("...")`, or a
     use of one as a value, warns with the attribute's text (`W0023`, or
     `W0025` without one), but in its own body (13); an attribute whose
     name Sisu does not read warns: with the fix to the attribute it
     misspells (`W0024`), or, when no compile-time code of the program
     reads attributes, because nothing can read it (`W0022`). Stage 1
     decides it (RULES.md, R30).

344. `embed("path")` in a module of a package resolves the path from the
     package's root, the directory of its `sisu.toml` (08), whatever
     directory the module is in; a single file's from its directory.
     Stage 1 decides it (RULES.md, R31).

345. `json.decode[T]` validates the text at the boundary (11): a
     refinement's predicate and a struct's invariant that the text breaks
     are a `Mismatch` naming the path and the type, not a type error in
     the decoder or a panic (05: malformed input is an expected failure).
     Reflection gains `is_refinement`, `base_of` and `has_invariant`.
     Stage 1 decides it, in its own copy of `std.json` (RULES.md, R32).

346. A float literal reaches a `Decimal` from its text, its places kept
     (`3.50` stays `3.50`, 03); `/` between `Decimal`s names `div` and
     `to_ratio` in a note (D-076); `@float(fast)` is accepted, strict
     floating point satisfying the permission it gives, and `@float` with
     any other argument is `E0448`; `@diagnostic_name` is `E0900` until
     compile-time type construction (Q-27); and a `list` of an `Ord` type
     has `is_sorted`. Stage 1 decides it (RULES.md, R33).

347. `E0609` for a plain `for` over a compile-time list offers the fix
     that writes `comptime for` (08). Stage 1 decides it (RULES.md, R34).

348. `max(xs)` and `min(xs)` with one argument are the largest and the
     smallest item of a collection, the first of equals, and panic for
     none, as Python's raise (02): the prelude's `max_of` and `min_of`.
     Stage 1 decides it (RULES.md, R35).

349. `std.json` declares `Encode` and `Decode`, which `encode` and
     `decode` use for a type that implements them, inside other values
     too; `@derive(Encode, Decode)` gives a type the form of its shape and
     an impl written by hand a form of its own (11). Stage 1 decides it
     (RULES.md, R36).

**The runtime in Sisu, continued** (section "Stage 2", "The runtime in
Sisu"; D-086, D-087 step 3)

350. The structural operations on values by their parts are the runtime in
     Sisu's (D-086): `compiler/rt/src/value.sisu` holds equality, order,
     the structural hash of decision 256, `.copy()`, the drop and the count
     of a copy's capability handles (decision 237), each a walk of a value
     by its type in the program's table, which the runtime in Rust lays out
     in memory when the program starts (`sisu-rt/src/desc.rs`: ten words
     for each type, its kind as `abi` encodes it, size, flags, the types it
     holds, its name's number and where its lists are, then the lists of
     fields, variants, members and the parts a drop and a copy reach in
     place, decisions 262 and 266) and passes to each export
     (`export_value_eq`, `_cmp`, `_hash`, `_copy`, `_drop`, `_drop_rest`,
     `_retain`). The runtime in Rust calls them wherever it compared,
     hashed, copied or dropped a value (`bridge.rs`), and they call it back
     by foreign calls (`host.sisu`, `extern "C"`, so the package declares
     `unsafe = "extern"`) for what it keeps as objects of its own, for its
     allocation and for the program's drop glue, which on the system stack
     they move aside to be dropped on the task's; a foreign call in the
     runtime's own code writes out no output first, as a program's does
     (`library` in both lowerings), and C's `memcmp` and `memmove` compare
     and move bytes, `sisu_rt_bytes_eq` among them. A dict's and a set's
     index hashes a key by its structural hash with its bits spread by
     MurmurHash3's last step, where the runtime in Rust had a second hash
     of its own, so equality and the index agree by one definition. The
     drop of a value whose glue runs on a task's own stack stays the
     runtime in Rust's there: the glue runs where it is and the walk of
     what it leaves out on the system stack, since the walk's frames are 1
     to 3 KB (every argument of a call in memory) and a task whose stack
     overflowed unwinds within 3 KB of the stack it has left; while runtime
     code runs on the system stack for a task, the task's probe limit is 0,
     since the runtime's Sisu code probes the stack as any function does
     and that stack is not the task's. The object is compiled in the
     release profile, and the walks read the table's words and the values'
     in place rather than through helper functions: what Sisu code pays is
     its calls, each argument in memory and each operation's result written
     to memory and read back (decision 246), where rustc inlined. Stage 2
     lowering its own SIR (4.1 MB) takes 2.3 to 2.7 s against 1.6 to 1.8 s
     with the walks in Rust, 1.43 times the instructions on
     `examples/wordcount`'s SIR (callgrind), most of the difference in text
     building, which interpolation calls most. What the runtime in Rust no longer has: the walks of
     `value.rs` (`drop_value`, `copy_value`, `retain_value`, `equal`,
     `compare` and their helpers) and of `dict.rs` (`structural_hash` and
     the key hasher), which are calls of the exports now. The other ways
     were the operations generated per type in each program's SIR (22's
     end, and R9's generic versions in `std.mem`), which compiles them
     again for every program and changes every program's SIR, and the
     Rust walk kept beside a Sisu one, which is two definitions. This
     changes no rule of the language.
351. Text is built by the runtime in Sisu (`compiler/rt/src/text.sisu`):
     `sisu_rt_text_push`, `sisu_rt_text_push_int` and `sisu_rt_text_from`,
     the entries that interpolation, `String.push`, `fmt.text`,
     `fmt.display` of a text or an `int`, `str.to_string` and `String.new`
     call (decision 265), are exports of Sisu's convention, which both
     lowerings call as they call `sisu_rt_bytes_eq`, the arguments in
     memory and the status tested (`rt_sisu` in `func.rs` and in stage 2's
     `func.sisu`), so LIR still gives the Rust lowering's executables byte
     for byte; a buffer grows by the runtime in Rust's `sisu_rt_grow`, to
     twice its size and at least 16 bytes as before, and bytes of the text
     itself pushed onto it are read where the growth moved them. An
     `int`'s digits are worked out on its negative, so that the most
     negative needs no positive counterpart, and `fmt.digits` and
     `fmt.wide_digits` take the digits of a magnitude of up to 128 bits in
     base 2, 8, 10 or 16 from four parts of 32 bits divided in turn
     (`sisu_rt_int_digits`). The runtime in Rust's three entries and its
     `wide_digits` are deleted. Floats' digits stay Rust's (port-debt.md,
     "Float to text"). This changes no rule of the language.
352. Text search is the runtime in Sisu's (`compiler/rt/src/search.sisu`):
     `in` on texts (and of a `char` in a text) and `bytes.find` find a run
     of bytes by Crochemore and Perrin's Two-Way, transcribed from the
     prelude's `str.byte_find`, linear in the lengths and in constant
     space, where the runtime in Rust compared every window of the text
     (quadratic at worst); `bytes.starts_with` and `ends_with` compare the
     edge (`sisu_rt_find`, `sisu_rt_has_edge`). An empty part occurs at 0.
     This changes no rule of the language.
353. Stage 2 has a library mode, so that the backend written in Sisu
     compiles the runtime in Sisu too and the fixed point of decision 330
     covers it: `sisu2 lir FILE.sir --library` and `sisu2 build FILE.sir
     --library --output FILE.o` lower the SIR as `compile_library` does
     (`gen.lower_library_spanned`): each `export_NAME` function of the
     root module is declared `export` under `sisu_rt_NAME` and what it
     calls `local`, the code is the runtime's own (`library`: glue's frame,
     no yield point, a type number refused, no output written before a
     foreign call), and there is no C `main` and no type table; the Sisu
     backend makes every function declared `export` a global symbol, which
     was C's `main` alone. The seed is still compiled by the Rust lowering
     and Cranelift for every build (`compiler/target/rt/rt.o`, decision
     336), since stage 2 links with it; `SISU_RT=PATH` makes a native build
     link with another object as it is. `stage2/bootstrap.sh` prints the
     runtime's SIR with `sisu sir` (A.rt.sir, which must be the committed
     seed) and with stage1-A (B.rt.sir), compiles it as a library with
     sisu2-C (rt-A.o) and with sisu2-A (rt-B.o), links sisu2-A and
     stage1-A with rt-A.o and sisu2-B and stage1-B with rt-B.o, and
     requires each pair to be the same bytes, so the runtime in Sisu,
     compiled by the backend in Sisu and run by every executable the script
     compares, is in the fixed point. A test holds the library's LIR to the
     Rust lowering (`stage_2_lowers_the_runtime_in_sisu_as_the_rust_lowering_does`):
     `sisu2 lir --library --release` of the seed, translated for Cranelift,
     is the object `compile_library` makes of it, byte for byte, as a
     program's LIR gives its SIR's executable (decision 284). Along the way
     both lowerings keep a raw pointer, one word, in a variable in the
     release profile as they keep a number (decision 246), and compile a
     sized integer's `wrapping_from` inline, a mask or a sign extension,
     where it called the runtime; the runtime's package has no `unsafe def`
     (its functions say what they require in a `Requires:` list, and their
     `unsafe` blocks the obligations they discharge) and its exports are
     private, so that the frozen Rust front end's formatter reads it and
     `W0014` does not apply. The other way was a second seed compiled by
     stage 2 and committed, which the fixed point would not have rebuilt.
     This changes no rule of the language.

**The driver and the test runner** (section "The query layer and the
driver"; 17, 10)

354. A `--` right after a run's file, `sisu run FILE -- ARGS` (or after
     `--from-sir FILE`), ends the command's words as it does before a
     package's arguments (`sisu run -- ARGS`), and is not the program's
     first argument, where it was; a second `--` is the program's. `sisu
     build --native` prints the executable's path relative to the working
     directory (`build/debug/app`, or `../build/debug/app` from
     `src/`), where a package's build printed it absolute, so that a session
     can show it; a path with nothing but the root in common with the
     working directory stays absolute. `sisu run --native` finds the
     executable by the same line, from the same directory. This changes no
     rule of the language.
355. The interpreter's and the runtime's correctness items the manual
     found (manual/README.md, "What the plan found in stage 0"):
     `Shared.same(a, b)`, 02's identity of shared values, is an intrinsic
     of the prelude's `Shared` (`Shared.same`, numbered after the others in
     `sisu_rt::abi` and stage 2's table), true when the two own one cell,
     where `==` compares the values (`cases/ownership/shared_same`). A
     call through a trait whose `Self` is a reference reads each argument
     through the references it has beyond what the impl's parameter takes,
     `other: Self` as `self` already was, so `sorted` of a list of a type
     with a hand-written `Ord` no longer stops the interpreter at `Field(1)
     does not apply to Ref` (a program stage 1 now refuses with `E0438`,
     since `sorted` of a borrowed list holds references, and the Rust front
     end still runs). A scope whose body returns `Cancelled` because a
     task's failure that nobody joined cancelled it returns that failure,
     the first in time (06: "the scope returns the first error, after
     joining all"), where the body's `Cancelled` hid it; an error of the
     body's own is still the result, and a `Cancelled` with no failure
     behind it stays one; the interpreter and the native runtime decide it
     alike (`cases/concurrency/first_failure`). This changes no rule of the
     language: it implements 02's and 06's.
356. `sisu run --explore N`, in the interpreter and natively, reports the
     first seed whose run prints other standard output than the first
     seed's, though it exits with the same status (`D0127`, status 1), as
     this document's "Tasks and the scheduler" says a run is compared in
     output or status, where only a failing status stopped it; its
     standard error, which holds the scheduler's numbers under `--stats`,
     is not compared. This changes no rule of the language.
357. `sisu test` and `sisu bench` run a program's tests and benchmarks
     from the SIR stage 1 prints of it built with them (`sisu1 sir
     --tests`, decision 285), as `sisu run` runs a program from stage 1's
     SIR (decision 268): for a file, a package and each member of a
     workspace, the driver asks stage 1 for the SIR, runs stage 1's `check`
     to report the program's errors when it prints none, and runs the tests
     as `sisu test --from-sir` does, with the program's own files for what
     names it again: a replay token's hash and the file its line names, the
     snapshots beside it, the test cache (decision 242) and the check that a
     `sim` build escapes nothing (`E0971`); `sisu sir --tests` is stage 1's
     too. So the tests of unsafe code, of `std.mem`, `std.ffi` and `extern`
     blocks run, on the checked memory model, which 22 makes the place where
     unsafe code is checked. The Rust front end keeps what only it gives:
     `--std`, the standard library's examples; `--defer-borrow`; and a
     program's transcripts, which run against a world of transcripts
     (`transcript_world`) that only it writes into the SIR, so a program
     with transcripts is tested by it whole (decision 380 runs only the
     transcripts on it). Every expectation of the suite that runs `sisu
     test` holds unchanged. This changes no rule of the language.

**The runtime in Sisu: a float's text** (section "Stage 2", "The runtime
in Sisu"; D-086)

358. A float's text is the runtime in Sisu's
     (`compiler/rt/src/floats.sisu`, `sisu_rt_float_digits`), where the
     runtime in Rust formatted it with Rust's: `Display` and `Debug` of an
     `f64` and an `f32`, which is the shortest decimal that reads back as
     the same number, the closest to it of those, with an exponent from
     10^16 up and below 10^-4 (`1e16`, `1.5e-7`) and a point otherwise
     (`100.0`); and the format specs' fixed form and form with an exponent
     (`fmt.float_digits`, `fmt.float32_digits`), the exact value's digits
     rounded half to even (`{0.125:.2}` is `0.12`, `{2.5:.0}` is `2`). The
     shortest digits are Steele and White's free format as Burger and
     Dybvig give it, its rounding boundaries in it when the significand is
     even (as reading back rounds half to even) and the gap below a power
     of two half the gap above; the exact ones come from the value as a
     fraction `r / s`, one digit at a time. The fraction's numbers are
     plain integers while they stay below 2 × 10^17, as they do for values
     from about 0.03 to 10^17, big numbers of up to 40 limbs of 32 bits
     otherwise, and an integer above 2^57 has its digits by division by
     10^9. The text is made in the result's own buffer. The interpreter
     keeps Rust's (`sisu-interp/src/format.rs`), which the native text
     matches on every case of the suite (`cases/run/float_text` among them)
     and on about 30,000 floats a test drew from every range (both widths,
     every exponent, the precisions from 0 to 25, and the subnormals, the
     powers of two and the halfway cases). `{x}` of an `f64` costs 1.3 µs
     where Rust's cost 0.46 µs, and 5 to 50 µs on big numbers, below 0.03
     and above 10^17 (port-debt.md, "Float to text"). What the runtime in
     Rust no longer has: `float_repr`'s and `exponent_form`'s formatting
     and the fixed forms of `float_digits`, which are calls of the export
     now. The other way was Ryu, whose tables of 128-bit powers the runtime
     in Sisu would compute or carry, and whose 128-bit products the
     backends compile as calls. This changes no rule of the language.

**The test runner: a hole in a task** (section "Testing"; 17)

359. A typed hole a task reaches blocks the test, as one the test's own
     code reaches does (17: the interpreter stops at a hole, and the test
     is blocked by it; D-061): the interpreter keeps the first hole a task
     other than the root reached, and the runner reports the test blocked
     by it whatever the test did after, where a test that went on to wait
     for what the task would have sent failed with the deadlock report
     (manual, chapter 22: a handler's hole inside `http.serve`'s connection
     task). A hole in a task nothing joins was blocked already, through the
     scope's end (`cases/testing/hole_in_task`). This changes no rule of
     the language: it implements 17's.

**The native backend: a closure that keeps a function value** (section
"Stage 2"; decision 224)

360. An `own` closure that keeps a function value, directly or in a tuple,
     an option, a result, a struct or an enum it captures, is refused
     natively (`D0103`, "the native backend does not compile an `own`
     closure that keeps a function value yet"), by stage 2's lowering and
     by the Rust one alike, where each compiled the program again and
     again: a function value holds as many words of environment as the
     widest `own` closure's (decision 224), and an environment that keeps a
     function value is wider than one by two words whatever that width is,
     so no width holds it (`own () -> f() + 1`, with `f` an `own` closure
     of its own). The interpreter runs such a program, and a closure of
     references or a task's closure that keeps one is unchanged
     (`an_own_closure_that_keeps_a_function_value_is_refused_natively`).
     The other way was an environment on the heap for such a closure,
     copied and freed with the value, which is port-debt.md's pay, and
     which decision 382 takes. This changes no rule of the language.

**Stage 1's fourth round** (stage 1's RULES.md, R37 to R46; associated constants, `expect`, contracts across modules, attributes, `Arbitrary`, generator expressions)

361. A trait declares associated constants (03): `const NAME: T`, which
     every impl gives, or with a value an impl may replace; each is read
     through a type, `T.NAME` under a bound, `Self.NAME`, or `Type.NAME`,
     and is the trait's `pure` function without `self` that gives it, made
     for the type at the read. An impl gives each constant the trait
     leaves without a value, no other, with the trait's type and as a
     constant (`E0232`, `E0271` to `E0273`); a read through the trait is
     `E0449`, and a trait with a constant is no `dyn` type. Stage 1
     decides it (RULES.md, R37).

362. `expect cond` in a program built with its tests (`sisu test`) lowers
     as `assert cond` does but for the failure, which calls
     `expect.fail`, `expect.fail_compare` or `expect.fail_parts` with
     `assert`'s arguments: the run records the failure in the running
     test's report and goes on, and the runner fails the test at its end
     with every failure recorded (17). `assert` stops the test as before;
     outside a build with its tests, and at compile time, `expect` stops
     as `assert` does. Stage 1 decides it (RULES.md, R38); the interpreter
     and the runner implement the three intrinsics.

363. A trait's contract binds an impl of any module (07): the impl's
     method checks the trait's clauses as R29's rule says, checked in the
     syntax of the trait's module with the impl's parameters under the
     trait's names, its failure naming the trait's method and showing the
     clause's parts, its statements standing at the impl's method's name.
     `W0021` remains only for a trait whose module's syntax the check
     does not have. Stage 1 decides it (RULES.md, R39).

364. A type, a constant or a variant marked `@deprecated("...")` warns
     where it is used, as a deprecated function does (`W0023`, `W0025`):
     a type named or built, a constant read, a variant built or matched,
     but in the type's own declaration and impls (13). An attribute Sisu
     reads that stands on a declaration it means nothing on warns
     (`W0026`, naming where it means something); and a suite's `@tag`s
     tag each of its tests. Stage 1 decides it (RULES.md, R40).

365. The prelude's `Arbitrary` (`def arbitrary(d: Draws) -> Self`) draws a
     type for property tests (17): an impl written by hand draws it its
     own way, which `arbitrary[T]` and the generators of the types that
     hold one use; `@derive(Arbitrary)` gives a struct or an enum without
     type parameters the impl that draws its shape, the generator
     `arbitrary[T]` writes for any type, and is `E0820` for one with a
     part that has no generator. Stage 1 decides it (RULES.md, R41).

366. A comprehension in parentheses, `(f(x) for x in xs if c(x))`, is a
     lazy `Iter[T]` (02): a generator whose body runs the clauses as loops
     and tests around a `yield` of the element, task-like as a generator
     function's body is, borrowing what it reads of the function's own
     while it lives and copying what is `Copy`; its clauses run when it is
     first advanced. Stage 1 decides it (RULES.md, R42).

367. A list's slice takes a bound counted from the end (`xs[1:^1]`, 02);
     an enum's body declares constants as a struct's does (`Light.COUNT`);
     a `const` parameter's value has the methods of its number
     (`N.to_f32()`); two sized integers compare as they add, the narrower
     widened when that loses nothing (D-016); a number or text on an
     operator's left reaches an impl of the operator for it when the right
     is a type of the program's (`2.0 * v`); a supertrait cycle through
     other traits is `E0247`; and a field or a variant declared twice is
     `E0205` or `E0206`, as in stage 0, the second variant left out of the
     derived impls. Stage 1 decides it (RULES.md, R43).

368. `@cfg` on a field, a variant or a method in a struct's or an enum's
     own body leaves it out of a build its condition does not hold in, as
     it leaves out an item and a method of an impl (08). Stage 1 decides
     it (RULES.md, R44).

369. The fixes and notes the manual's fourth round found missing: double
     quotes for text in single quotes (`E0016`); `E0327` with the closest
     variant for `Type.Name` an enum lacks; a note on Python's conversion
     functions (`E0200`); `Some(...)` for a value where its option is
     expected and `.to_f64()` for an `int` where a float is (`E0300`);
     `repeat` for text times a number and the operator's trait for a
     type's own (`E0310`); the closest parameter for a misspelt keyword
     (`E0306`); keyword-only parameters (`E0305`); `.or(...)` for `??` on a
     result (`E0371`); `?.` as one token after a result (`E0370`); a copy
     of a parameter the body changes (`E0312`); `remove` for an item moved
     out of a list (`E0338`); `Cell` or `Mutex` for a `Shared`'s field
     (`E0341`); and a plain closure for an `own` closure's capture
     (`E0423`). Stage 1 decides it (RULES.md, R45).

370. A program built with its tests (`sir --tests`, from which `sisu
     test` and `sisu bench` run, decision 357) has no SIR when it has an
     error of any phase: the front end's, the ownership checker's, or a
     failed `comptime` expression's. `sir --tests` prints the diagnostics
     and `D0006`, or `D0021` at the ownership checker's errors, as `sisu
     test` prints them, and fails. `sir`, the program without its tests,
     keeps printing the SIR of a program the ownership checker rejects,
     as stage 0's `sisu sir` does. Stage 1 decides it (RULES.md, R46).

**The test runner's status, and what native code leaked or refused**
(sections "Testing", "The query layer and the driver" and "Stage 2"; 17,
10; decisions 224, 340 and 360)

380. `sisu test` exits with 3 when no test failed and one is blocked by
     a typed hole, 1 when a test failed (whatever else is blocked) or the
     program has an error, and 0 when every test passed, as decision 340
     has it of stage 1's `test` and 17 and 10 now say, where blocked tests
     left the status 0 (decision 32); a workspace's members combine by the
     worst status, a 3 yielding to any other failure. The conformance
     runner holds every run of `sisu test` it compares (`.test`,
     `.explore`, `.sim`, `.schedule`, `.select`, a package's `.test`) to
     the status its report gives, summed over its summary lines. Three
     regressions of decision 357's routing, which the manual's round found,
     are mended: a program the front end rejects no longer has its tests
     run (the exercises `winner`, `E0408`, and `journeys`, `E0603`, ran
     under the monitor, which stopped them with 60 to 230 lines of SIR),
     since the driver shows what stage 1's `sir --tests` prints when it
     fails, its diagnostics and the line `sisu run` ends with (`D0021`
     after the ownership checker's errors, `D0006` after the others), and
     takes its status, 1 (stage 1's R46, decision 370, where it printed the
     SIR of such a program), and asks its `check` of the program with its
     tests after a `sir --tests` that printed SIR, for the warnings, and
     for the errors a stage 1 without R46 leaves to it, which it reports
     the same way (`sisu bench` alike); the program's warnings, `W0001`
     among them, come before the report, as `sisu run` prints them; and
     the driver no longer writes the lockfile of what stage 1 tests,
     leaving it to stage 1, which records each package's `unsafe` level
     (decision 275), where the driver's lockfile without it made stage 1
     refuse it with `E1022` and broke `sisu run` until it was deleted
     (chapter 27's `levels.pkg`, guide G19's `cenv`); `--locked` and a
     package's features go to stage 1's `check`, which takes them, and not
     to its `sir`, which refused them as a usage error, and a run that asks
     for features stays the Rust front end's, since stage 1's `sir` builds
     without them. The cases `testing/refused_program`, `refused_comptime`
     and `unsafe_level` hold them. A program's transcripts no longer keep its tests on the Rust
     front end (decision 357): the tests run from stage 1's `sir --tests`,
     and only
     the transcripts from the Rust front end's build of the program, with
     the world of transcripts that only it writes (`transcript_world`,
     `transcript_system`), so a program with transcripts and tests of
     unsafe code runs both; a transcript of a program the Rust front end
     rejects fails with its errors (`D0128`). `sisu test --std` outside a
     package and without a file runs the standard library's examples
     alone, on an empty program that a replay line names by the flag
     (`--std`), where it was `D0064`; and a run with them takes nothing
     from the test cache, since the program's hash leaves their code out.
     What stays on the Rust front end is what stage 1's `sir` cannot give:
     the standard library's examples as tests, the world of transcripts,
     `--defer-borrow`'s deferral and a package's features (port-debt.md). This changes no rule of
     the language: it implements decision 340 in the runner.
381. A text an interpolation is building is freed natively when a panic or
     a `?` in one of its values leaves the function before the statement
     that builds it ends: lowering registers the `String` that `fmt.new()`
     makes for its scope only once that statement is done (section
     "Lowering"), so no drop of the SIR reaches it on those paths, which
     the interpreter's host frees and native code leaked (`"{f()}"` with
     `f` panicking left the buffer live at exit, which stage 1's round
     found; a `?` in a value did the same without a panic). Both lowerings
     work out, for each block, the locals a `fmt.new()` made into that
     nothing has moved, dropped, written over or ended since on every path
     to it and that its cleanup does not drop (`building_texts`); each
     failure path of a call frees those its statement holds, after the
     trace's line and before the block's cleanup or the return of the
     failure, and a `return` frees each but the result. A block has a
     failure path for each set of such texts, so a function that has none
     where it may fail compiles as before, and LIR still gives the Rust
     lowering's executable byte for byte. `tests/leaks/leaks.sisu` holds
     both shapes, a `?` in an interpolation's value and a panic in one in
     a task. The other way was the front ends registering the buffer while
     its values are evaluated, as an operand's temporary is, which is
     stage 1's lowering (and the frozen Rust front end's) and changes
     every program's SIR. This changes no rule of the language.
382. An `own` closure that keeps a function value, directly or in a tuple,
     an option, a result, a struct or an enum it captures, compiles
     natively, where decision 360 refused it: its environment, wider than
     any function value holds in place (decision 224), goes in a heap cell
     (`sisu_rt_alloc`) whose address is the value's first word of
     environment, which the closure's code reads as it reads a larger
     environment of references, and the value's last word names a `Box`
     of the environment's type (decision 248), so that a drop of the value
     drops what the cell holds and frees it (the runtime's drop of a
     `Box`) and a copy, the value's words copied and what they hold
     counted (`sisu_rt_retain`), gives the copy a cell of its own, a copy
     of the one it shared: the runtime in Sisu's `retain` of a `Box`, which
     only a function value's environment reaches, since a `Box` is not
     `Copy`. Both lowerings write the same code, and LIR gives the Rust
     lowering's executable byte for byte. The driver's test
     (`an_own_closure_that_keeps_a_function_value_runs_natively`) builds
     such a program with stage 2 and the Sisu backend, with the Rust
     lowering and Cranelift, and in the release profile, copying such
     values through a `let`, a tuple and a list, and each run prints the
     interpreter's output and frees what it allocated. The other way was a
     count of the copies sharing one cell, which needs a header the
     runtime's `Box` has not. This changes no rule of the language.

**The runtime in Sisu: text of values and lists** (section "Stage 2", "The
runtime in Sisu"; D-086, D-087 step 3)

383. `Display` and `Debug` of a value whose impl is structural are the
     runtime in Sisu's (`compiler/rt/src/show.sisu`, `sisu_rt_value_display`
     and `sisu_rt_value_debug`), a walk of the value by its type in the
     program's table as the interpreter writes it (`sisu-interp/src/
     format.rs`): `fmt.display` and `fmt.debug`, `print` of a value that is
     not text, `list.join`, the sides of a failed `assert` and the error
     `main` returns, which the runtime in Rust asks for through `bridge.rs`
     with a `String` the walk appends to. The table gains what `Debug`
     writes (`desc.rs`, eleven words a header): the texts of a struct's
     short name and its fields' names, of an enum's variants and their
     fields', and of an opaque type as it is shown, each the address and
     the length of the decoded table's bytes, which live for the run, and
     flags for std's `Contextual` (`message: cause`) and a struct shown by
     its name alone (`System`). A float's digits are `floats.sisu`'s and an
     integer's `text.sisu`'s, which now appends them too (`push_digits`,
     `push_int`, `put`); the tables of dicts and sets, a task's path, a
     `Shared` cell's liveness, an arena and a time are read out or shown by
     calls of the runtime in Rust (`sisu_rt_table_len`, `_key`, `_value`,
     `sisu_rt_shared_alive`, `sisu_rt_show_task`, `_arena`, `_datetime`).
     The words `Debug` writes are packed eight bytes to a constant, the
     runtime's code having no text of its own to point to. What the runtime
     in Rust no longer has: `value.rs`'s `display`, `debug`, `fields_debug`,
     `quote_char`, `quote_bytes`, `int_digits` and `push_char`, which are
     calls of the exports now. `run/debug_forms` holds every kind natively
     as the interpreter writes it. The other way was the program's own
     `Debug` impls generated for every structural type (22's end), which
     changes every program's SIR. This changes no rule of the language.
384. A `Shared` displays as its value does, as std's `impl[T: Display]
     Display for Shared[T]` says: both front ends take that impl as the
     host's (std's impls of the host-backed types render as the host does),
     and the host showed the value's `Debug` form, `"Aino"` with its
     quotes (the manual's maintenance round); the interpreter's
     `display_into` and the runtime in Sisu's `display` follow a live
     `Shared` to its value (`run/shared_display`). This changes no rule of
     the language: it implements std's impl.
385. Lists are the runtime in Sisu's over raw memory
     (`compiler/rt/src/list.sisu`): a list is its buffer's address, its
     length and its capacity, its elements in the buffer each its type's
     size, moved as bits as std.mem's `RawBuf` moves slots, and the buffer
     grows to twice its capacity, four elements at least
     (`sisu_rt_list_grow`, which the runtime in Rust's `List::reserve` and
     the generated code's `sisu_rt_list_reserve` call). Here are the moves
     of `insert`, `remove` and `pop`, `extend`, `reverse`, `swap` and
     `reversed`, the copies a slice makes (`list[a:b]`, `Slice` cuts), the
     stable sort of `sort` and `sorted` (a merge sort of the indices by
     `value.order`, refusing two elements without an order), `in` and
     `join`; the runtime in Rust decodes the call and checks the indices,
     whose panics its messages name. What it no longer has: `copies`,
     `reverse`, `sort_list`, `merge_sort` and the growth of `List::reserve`.
     `run/list_moves` holds them natively as the interpreter runs them.
     With decision 383, the runtime's seed grows from 308 916 to 428 692
     bytes of SIR and its object, as the fixed point builds it, from
     582 664 to 799 152 bytes, which the fixed point still holds (rt-A.o
     and rt-B.o the same bytes, as sisu2-A and sisu2-B, 27 657 704, and
     stage1-A and stage1-B, 54 718 840). The other way was the list methods
     written in Sisu over `RawBuf` in the prelude and compiled with every
     program, which needs per-type code (22's end) and changes every
     program's SIR. This changes no rule of the language.

**The driver: what the manual found** (section "The query layer and the
driver"; 10, 17; the manual's findings R13, R16, R17 and G14)

386. `sisu --version` and `sisu version` print the toolchain's version
     (`sisu 0.0.1`, `D0131`), the one replay tokens and capability logs
     record; `sisu explain` without a code says it needs one (`D0129`) and
     `sisu new` without a name that it needs the package's (`D0130`), where
     both said a file was needed (`D0004`); `sisu explain CODE` shows the
     message as a reader sees it, one brace where the catalog doubles it
     and its line breaks, a parameter still named in its braces; `check`,
     `run`, `build`, `sir`, `test` and `bench` take `--accept-unsafe`,
     which `E1022` names, and pass it to stage 1, which lets a package's
     `unsafe` level rise in its lockfile (22); `--stats` followed by a word
     that names nothing there, before the file, takes it as the kind asked
     for, so `sisu run --stats bogus FILE` is `D0094` where it read `bogus`
     as the file; and under `--explore N` (or the exhaustive scheduler) a
     test that fails under one of the schedules shows what that run
     printed, where the first seed's lines were shown with the failure of
     another (`testing/explore_output`). The test cache is
     `build/debug/test-cache` in a package, as section "Testing" now says.
     This changes no rule of the language.
387. A float literal's `Decimal` is the same natively as in the interpreter:
     stage 1 lowers it to `Decimal.of_literal` of its text, its places kept
     (decision 346), whose Sisu code in std.num both run
     (`std/decimal_literals`, in `native.list`: `3.50`, `0.10` and
     `12345678901234567890.123456789` as written); the frozen Rust front end
     still reads the literal from its `f64`'s shortest digits. This changes
     no rule of the language.

**The runner: `expect` goes on** (section "Testing"; 17; stage 1's R38,
decision 362)

388. `expect.fail(text, message)`, `expect.fail_compare(text, message,
     left, right)` and `expect.fail_parts(text, message, parts)`, which
     stage 1 lowers a failed `expect` to in a program built with its tests
     (its R38), take what `assert`'s three take: the interpreter renders the
     failure as `assert`'s panic would be reported, with the frames it
     would leave (the statement's and its callers' in the running task,
     but drop glue's and the standard library's), records it in the running
     test and returns, and the runner fails the test at its end with every
     failure it recorded, in order, before the report of the panic that
     ended it if one did, whatever else it did, blocked by a hole too;
     outside a test, a plain run and compile time, they are `assert`'s
     panic. Natively, where no test runs, they are `assert`'s (appended to
     `sisu_rt::abi`'s intrinsics and stage 2's table, so the others keep
     their numbers). `tests/expect/expect.sir`, what stage 1's `sir --tests`
     prints of its case `testing/expect`, holds it
     (`expect_records_its_failures_and_the_test_goes_on`). This changes no
     rule of the language: it implements 17's `expect`.

**Stage 1's fifth round** (stage 1's RULES.md, R47 to R55; warnings from `sir --tests`, slices from the end, generator expressions, `Arbitrary`, spelling fixes)

400. `sir --tests` prints, beside the SIR of a program without an error,
     the program's warnings on standard error as `check` prints them (the
     human form, or `--format=ids`), from one check of the program, so
     that the driver runs stage 1 once for each program `sisu test` and
     `sisu bench` run; `sir` takes `--features`, `--no-default-features`
     and `--locked` as `check` does (decision 250). Stage 1 decides it
     (RULES.md, R47).

401. A bound counted from the end reads a `Slice` and `bytes` as it reads
     a list (`xs[^1]`, `xs[1:^1]`, 02), the base evaluated once; a type of
     a module written with its arguments before `.`,
     `collections.Deque[int].new()`, has its scalar arguments read as
     types; and `E0413` offers the `from` clause first when the body
     returns a borrowed parameter or a part of one, the `String` result
     first when it returns text it makes. Stage 1 decides it (RULES.md,
     R48).

402. A generator expression's first iterable is evaluated where the
     expression is (02, as Python's): a value that is not a place is
     computed there and kept by the generator, moved into its closure, a
     `?` in it passing its error on in the enclosing function; a place is
     borrowed as before. The other clauses run as the generator is
     advanced. Stage 1 decides it (RULES.md, R49).

403. A constant of a type of the program, given by an impl or by the
     type's body, is a type's constant argument where folding computes its
     value (`Array[int, Square.SIDES]`, 03); a trait's constant through a
     type parameter is a value of the instance and no constant argument,
     `E0239` noting the `const` parameter that carries it. Stage 1 decides
     it (RULES.md, R50).

404. A trait or a field marked `@deprecated("...")` warns where it is used
     (13), as a deprecated type does (decision 364): a trait in a bound, an
     impl's header, a supertraits list or a `dyn` type, but in its own
     declaration; a field read, changed, given a value or matched, but in
     its struct's methods and impls. Stage 1 decides it (RULES.md, R51).

405. `@derive(Arbitrary)` on a generic type gives the impl that draws its
     shape with each type parameter drawn through its own `Arbitrary`
     (`impl[T: Arbitrary] Arbitrary for Boxed[T]`), and an impl of
     `Arbitrary` written in another module than its type's draws the type
     wherever it is drawn, a site in a module that imports the type
     drawing it through the impl. Stage 1 decides it (RULES.md, R52).

406. An attribute before a statement is `E0129`, naming the attribute,
     and the statement is parsed as one (02: attributes stand before
     declarations); `@tile`, `@unroll` and `@vectorize` there note that
     they are 19's planned directives (N-22), not in the language yet.
     Stage 1 decides it (RULES.md, R53).

407. A parse error at another language's spelling gets the fix that
     writes Sisu's (`and`, `not`, `ref`, `i += 1`, `var`, `.to_f64()`,
     `print`, `range(0, 3)`, `list[int]`, `match`, `.is_none()`, `:`, `#`,
     `None`); an attribute on its declaration's line is `E0131` and the
     declaration is parsed; `pub(crate)` is `E0130` with `pub(package)`; a
     one-line `if` without `else` is `E0102`; a keyword as a field's name
     is `E0124`. Stage 1 decides it (RULES.md, R54).

408. The type checker's notes and fixes the manual's writers found
     missing: a `var` that holds a borrow it was given (`E0341`), a
     borrowed parameter given to an `own` one, with the fix that takes it
     with `own` (`E0300`), the narrowing conversion of an `int` (`E0300`),
     a capitalised name a `let` binds (`E0327`), and `E0417`, `E0806` and
     `E0810` without an article. Stage 1 decides it (RULES.md, R55).

**Capabilities: a file takes a new name** (section "Capabilities"; 06,
09, 17; N-04)

420. `Fs.rename(source, target)` is an operation of the file system
     capability (`FsOps.rename` in the prelude): a file takes its new name
     in place of a file already there, on the host atomically (POSIX's
     `rename`), so that a reader finds the old file or the new one, never a
     part of either, and a directory takes it with what is below it. A
     failure names `source` when it is not there, a refusal the path
     refused (`source` for a read-only file system), and any other
     `target`. The host's is the intrinsic `host.fs.rename`, in the
     interpreter (`Host::rename_file`, over `std::fs::rename`; a host
     asked after a `NotFound` whether the source is there) and natively
     (appended to `sisu_rt::abi`'s intrinsics and stage 2's table, so the
     others keep their numbers); the capability log records it as
     `call rename_file "SOURCE" "TARGET" -> ok` and replays it from the
     log as it replays the other file operations (09). The attenuations
     and caretakers of std.cap forward or refuse it as they do any change
     (`ReadOnly` refuses it, `Restricted` refuses a path that leaves its
     root and names the path given back, a revoked `Caretaker` is
     `Revoked`), the interceptor shows it to its hook, and the fakes rename
     in memory: `MemFs` at once; `SimFs` at once for every reader and,
     as 17 asks of the `sim` profile's model of rename and directory
     durability, durably once the new name's directory is synced: until
     then a crash puts the old name back with what it had made durable,
     after it the new name holds what the file had made durable, and a
     file whose name was synced but never its contents comes back empty
     after a crash, where the model made such a name a directory.
     `capabilities/rename` (host, fakes, attenuations, interceptor; in
     `native.list`, so the host's rename runs natively too, recorded and
     replayed by the suite) and `sim/rename` (the atomic save that
     survives a crash, and the one that does not) hold it. Stage 1's base
     cache can be written aside and renamed over the old one. This
     changes no rule of the language: it adds an operation to std's file
     system capability.

**The driver: a replay token and what a program embeds** (section
"Simulation, schedulers and replay"; 08, 17; the manual's finding from
guide G30)

421. A failing test's replay token names the content hash of the program
     with the files its `embed("path")` calls read, each path and its text
     (or why it could not be read) after the sources' own hash, so that a
     token made before an embedded file changed is refused after it
     (`D0033`), as a token made before the source changed is, where the old
     token ran the new program; the files are those stage 1 reads (a
     package's module names them from its package's root, a single file
     from its directory, the first module to name a path deciding; its
     R31), found by the Rust parser in the program's own texts. A program
     that embeds nothing keeps the hash of its sources, so every token of
     the suite is what it was. The capability log's content hash is
     unchanged: a log of stage 1's run holds the SIR, whose texts hold what
     was embedded (`a_replay_token_covers_the_files_the_program_embeds`).
     This changes no rule of the language: it implements 08's "the same
     inputs give the same program" for 17's tokens.

**The runtime in Sisu: the tables of dicts and sets, and iterators**
(section "Stage 2", "The runtime in Sisu"; D-086, D-087 step 3)

422. The tables of dicts and sets are the runtime in Sisu's
     (`compiler/rt/src/table.sisu`): a `dict`'s or a `set`'s value is the
     address of a header of nine words (the key's type, the value's, -1
     for a set, the bytes of a key and of a value, the entries' block,
     their number and room, the index's block and mask) and the blocks it
     names, its entries in insertion order, each its key's hash, the key
     and the value, and its index a power of two of slots at least twice
     the room for entries, each 0 or one more than an entry's position,
     probed linearly. A key is hashed by its structural hash (decision 256)
     with its bits spread by MurmurHash3's last step, and compared by
     structural equality but that a `Path` is never the same key as
     another text, as the interpreter keeps them, whether it is a `Path`
     asked once a lookup; an entry keeps its hash, so a growth or a copy
     makes the index again without hashing a key again, and a removal
     moves the entries after it down a place and makes the index again, so
     no slot is a tombstone (the runtime in Rust walked its whole index on
     a removal as well). The runtime in Rust decodes each intrinsic's
     call, writes the value where an entry's goes and drops what a call
     drops, through four exports (`sisu_rt_table_new`, `_lookup`, `_push`,
     `_remove`), reading the header as `table.sisu` lays it out
     (`dict::Table`); `value.sisu` copies, compares and drops a whole table
     and `show.sisu` shows one itself, where they called back for them.
     What the runtime in Rust no longer has: `dict.rs`'s `Table` with its
     `HashMap` index of positions and its hasher, `copy_table`,
     `drop_table` and `key_equal`, and the callbacks `sisu_rt_object_eq`,
     `sisu_rt_table_len`, `_key` and `_value`. The entries the generated
     code calls (`sisu_rt_table_find`, `sisu_rt_table`,
     `sisu_rt_dict_slot`, decision 265) are as they were, so both
     lowerings write the same code. The other way was the tables in Sisu
     over `Hash` and `Eq` in std, compiled per type with each program,
     which needs per-type code (22's end) and lifts `E0398`
     (port-debt.md, "Hashing"). This changes no rule of the language.
423. Iterators are the runtime in Sisu's (`compiler/rt/src/iter.sisu`): an
     `Iter[T]`'s value is the address of an object of eleven words, the
     header compiled code reads to step a range or a borrowed list's
     elements itself (decision 253, unchanged), the state, the bytes of an
     item and five words of the state's own (a list's three words and the
     next index, a text's bytes and position, the inner iterators of
     `enumerate` and `zip`, a table and its next entry, a generator's task,
     a receiver's channel). Its next item, the iterator of an
     `Iterable[T]` argument (an owned or borrowed list, dict or set, a
     receiver, an iterator, behind a reference or a `dyn`, the items of a
     borrowed collection decaying to copies), every item left (decision
     235) and its drop are four exports (`sisu_rt_iter_next`, `_iterable`,
     `_collect`, `_drop`); `enumerate` writes the inner item where its
     pair's goes, `zip` the second item after the first, over the first's
     last word kept aside, and the decay the reference in the object's own
     free words, so that no step allocates, where the runtime in Rust
     allocated a buffer at each; a text's characters are decoded from
     UTF-8 in Sisu, a sequence that is not one ending them as Rust's did.
     A generator's items, a receiver's values and a generator closed
     before its end (decision 236) it asks the runtime in Rust for
     (`sisu_rt_gen_item`, `sisu_rt_recv_item`, `sisu_rt_gen_abandon`),
     since they run the scheduler, which decides there whether
     `Iter.next` waits by the state's word as before, and the panic of
     `zip`'s lengths (`sisu_rt_zip_lengths`); the runtime in Rust makes
     the iterators of its intrinsics as the object is laid out
     (`iter::IterObj`). What the runtime in Rust no longer has:
     `iter.rs`'s `State` with `next`, `iterable`, `collect`,
     `collect_all`'s buffers, `drop_state`, `first_char` and the owned
     table's drop. Every case of `native.list` runs as before, and stage 2
     lowering its own SIR (4.2 MB) gives the same LIR in 2.48 to 2.71 s
     with both pieces in Sisu, against 2.31 to 2.66 s before them, the
     executables built with the Rust lowering and Cranelift as the driver
     builds `sisu2`; on `examples/wordcount`'s SIR it executes 507.0
     million instructions against 495.3 million (callgrind, 1.024 times),
     most of the difference the structural walks a lookup calls
     (`value.target`, `equal`, `hash`) where the runtime in Rust's index
     compared a hash first. The runtime's seed grows from 428 692 to
     552 134 bytes of SIR and its object (`target/rt/rt.o`) from 418 832 to
     564 704 bytes, and as the fixed point builds it with the Sisu backend
     from 799 152 to 1 059 600 bytes, which the fixed point still holds
     (rt-A.o and rt-B.o the same bytes, as sisu2-A and sisu2-B, 27 603 784,
     and stage1-A and stage1-B, 55 255 760). This changes no rule of the
     language.

**The driver: one run of stage 1 for a test** (section "The query layer
and the driver"; decision 380)

424. `sisu test` and `sisu bench` run stage 1 once for each program, `sir
     --tests` with `--locked`, `--accept-unsafe`, the features and the
     path, when its `sir --tests` reports the program's diagnostics itself,
     its warnings among them, and takes `--features`,
     `--no-default-features` and `--locked` (the stage 1 round after R46):
     what it prints on standard error comes before the report, and a run
     that fails or prints no SIR takes its status. Else the driver runs
     its `check` after `sir --tests` for the warnings, as decision 380
     has it. Which stage 1 it is, `Stage1::sir_reports` finds by asking
     it, once for each executable (its size and time kept beside it in
     `target/stage1/sir-reports`), for `sir --tests --locked` of an empty
     directory: a stage 1 whose `sir` takes no `--locked` stops at its
     usage in about 50 ms before it reads anything, and one that takes it
     finds no package there; `SISU_STAGE1_SIR_REPORTS=1` or `0` decides
     instead. With such a stage 1 a run that asks for a package's features
     is stage 1's too, where it stayed the Rust front end's. The stage 1
     of this round's start is the first kind: its tests still run stage 1
     twice. This changes no rule of the language.
425. `sisu lint` and `sisu audit --unsafe [--std] [PATH]` are stage 1's
     commands, which the driver passes on with their arguments, output and
     status as they are (`stage1_as_is`, as `fmt` and `doc`): the lints
     beyond the compiler's warnings, with their fixes (its R11), and the
     audit of a program's unsafe code by module, with each block's reason
     and each package's level (its R7), where `lint` was `D0003` and
     `audit --unsafe` `D0009`; `sisu help` lists both. On the Rust front
     end they are refused as before
     (`lint_and_the_audit_of_unsafe_code_are_stage_1s`). This changes no
     rule of the language.

**What the manual found: faults, stopped runs, transcripts and the audit**
(sections "Simulation, schedulers and replay", "Testing" and "The query
layer and the driver"; 17, Q-51; the manual's findings from X12, chapters 1
and 9, and G32)

426. A fault of the `sim` profile is never `Revoked`: the simulation's own
     file system and network, whose calls `--faults` fails, are never
     revocable, and `Revoked`, which every I/O error set holds (Q-51), is
     a caretaker's alone, so the variant is drawn from the others of the
     declared set, where a test that matched every error a file system
     can give saw it injected into one never made revocable. A caretaker
     of the simulated file system is revoked as any is
     (`sim/no_revoked`, every call failing, each with another variant);
     `sim/http_faults`'s failing schedule now sees `Other` where it saw
     `NotAllowed`, the draw over four variants rather than five. This
     changes no rule of the language: it implements 17's "an error it
     could return".
427. The conformance runner records a case that runs, and replays its log,
     only when the run wrote one or exited with 0: a program its errors
     stop is not run, and writes no capability log, so a case can pin what
     `sisu run` prints for it, the errors as a person reads them and the
     line `D0006` (`run/stopped_by_errors`, with its `.err` and
     `.status`), where the replay of a log never written failed the case;
     the recorded run is still compared with the plain one. This changes
     no rule of the language.
428. A transcript compares a run's output without a failure's trace: the
     frames after an `error:` or a `panic:` line (`  at FILE:LINE:COL`, a
     return trace's, and `  at FUNCTION (FILE:LINE:COL)`, a panic's) are
     left out of what it printed, and of what `--update` writes, as 17's
     own transcript shows an error (`error: no pears in stock (exit 1)`),
     so that a transcript does not change when the program's lines move,
     where it broke at every move; a line a program prints that looks like
     a frame stays when no failure came before it. `testing/shop`'s
     transcript of a failing run is the error and its status. This changes
     no rule of the language: it implements 17's transcripts.
429. `sisu audit --caps` without a file audits the program of the working
     directory's package, each member of a workspace's in turn, every
     module of it, its lines named by the package's directory as a failing
     test's replay line names it (`.`), where it needed a file and read it
     alone, so that a module's import of its package was `E0220` and a
     package's capabilities could not be audited; a file it names is still
     a program of its own (`the_capability_audit_reads_a_package`). This
     changes no rule of the language.

**Stage 1's sixth round** (stage 1's RULES.md, R56 to R64; a version, D0132, `debug_assert`, the findings' fixes)

440. `sisu1 --version` (and `sisu1 version`) prints `sisu1 VERSION`, stage
     1's version, one line, before it reads anything; its minor number is
     the round of stage 1's rules that last changed what the driver may
     rely on, 6 for the stage 1 whose `sir --tests` reports the program's
     diagnostics, so that the driver may ask it rather than probe `sir
     --tests --locked` (decision 424). Stage 1 decides it (RULES.md, R56).

441. A build of a program with an error, `sisu build` (with or without
     `--native`) and `sisu sir` of a program it has no SIR of, ends with
     `D0132`, "the program was not compiled: errors found (N)", where it
     ended with `D0006`, "the program was not run"; `sisu run` builds
     with stage 1's `build --for-run`, which ends as before with `D0006`,
     or `D0021` at the ownership checker's errors, and `sir --tests`
     ends as `sisu test` does. Stage 1 decides it (RULES.md, R57).

442. A typed hole's closure shows each `old(...)` value of its
     function's contract once, by its text, however many clauses write
     it, where it showed a line for each place. Stage 1 decides it
     (RULES.md, R58).

443. `debug_assert cond[, message]` is a statement, `assert`'s form that
     a build checking fewer than all contracts strips (02, 07): checked
     with every part shown where the build checks all of them (a debug
     build, `sisu test`), not evaluated where it checks `preconditions`
     or none (the release profile's default). `debug_assert` is a name,
     the statement only before an expression. Stage 1 decides it
     (RULES.md, R59).

444. `import a.b.Item`, whose path but its last name is a module that has
     no module `Item`, is `E0223` ("is not a module") with the fix that
     writes `from a.b import Item`, and `E0204` where a std module
     declares no such item, where it was `E0203` or `E0220`, which say
     there is no module. Stage 1 decides it (RULES.md, R60).

445. A `Slice` and `bytes` given to an `Iterable[T]` parameter are
     iterated through their iterators, as a `for` loop iterates them; an
     interpolation's `place ?? default` over an optional `String` reads
     the place as an optional `str` (decision 295's reading); a scalar
     type's name given as an argument is a type a `comptime def` taking
     `T: type` takes, and `E0200` elsewhere. Stage 1 decides it (RULES.md,
     R61).

446. `@derive(Decode)` on an enum is `E0608` at the derive, since R36's
     derive gives a struct's form only, where it was accepted and the
     program's run stopped at an invalid SIR. Stage 1 decides it
     (RULES.md, R62).

447. The fixes and notes the manual's writers found missing: a closure
     that passes a value, `x -> f(x)`, for a function given where its
     argument is a reference; `enumerate`'s count from 0; `1.0` and `0.5`
     for Python's `1.` and `.5`; `E0115` and `E0123` inside an
     expression; `ref dyn T` and a generic parameter for a trait written
     as a parameter's type; `pure def` for a contract's impure callee; no
     `E1002` for a key written with a bad value. Stage 1 decides it
     (RULES.md, R63).

448. A variadic parameter is `E0900`, naming the disagreement between 02,
     which gives it, and 14, which rules variadic functions out, where the
     checker stopped without a code and `check` passed; star unpacking in
     a `let` is one `E0900` that names the list pattern of a `match`,
     where it was three errors of its pattern. Stage 1 decides it
     (RULES.md, R64).

**Stage 2: a program's tests run natively** (sections "Testing" and
"Stage 2"; 17; D-089, port-debt.md's plan for a native `sisu run`)

460. `sisu test --native [--release]` compiles a program's tests, the SIR
     stage 1's `sir --tests` prints, into one executable and runs each test
     natively as the interpreter runs it (`sisu-driver/src/native_tests.rs`).
     The driver adds a dispatcher, `tests#main`, as the program's entry: it
     switches on the intrinsic `test.entry` (the runtime's
     `SISU_TEST_ENTRY`) and makes what the test takes as the interpreter's
     `run_test` makes it, the real `System`, `System.sim(seed)` under
     `--sim`, or fresh fakes of the capabilities it names (`System.fake`'s
     fields moved out and the rest dropped, `Net.fake()`, `Clock.fake()` for
     a `TestClock`, decision 241), then calls it; the runtime leaves the
     dispatcher's frame out of a trace. Each test is a process of its own,
     those that take no real `System` in parallel (`--jobs`), the others
     after them one at a time with the terminal's standard input, as
     `parallel.rs` runs them; how it ended goes to the file
     `SISU_TEST_REPORT` names, apart from what it printed: a panic's report,
     or the first hole any task reached (written when it is reached, since
     a run the hole leaves waiting may end in a deadlock's abort), which
     blocks the test whatever else it did. The report, replay tokens,
     `--explore N` (each seed a run of the executable), `--replay` and the
     summary are the interpreter's code, which takes the results; the test
     cache is not used. What it refuses with `D0134` is the interpreter's
     alone: `--scheduler pct` and `exhaustive`, `--faults` (the
     interpreter injects them), and a test parameter no fake stands for;
     `expect` is `assert`'s panic natively (decision 388), and `sim.always`
     and `sim.choose` are not compiled yet (`D0103`). The conformance
     runner's native modes run, for a case of `native.list` that pins `sisu
     test` (`.test`, and the runs `# sim`, `# schedule`, `# select` and `#
     explore N` with `.explore` ask for), `sisu test --native` with the
     mode's flags (`--release`, a test checking every contract,
     `--backend=cranelift`, and `--lower=rust --backend=cranelift` under
     `--native-from-lir`; the
     tests are built from SIR whatever the mode, and
     `--interpreted-from-sir` runs them in the interpreter), compare its
     report with the expectation, hold it to the status its report gives,
     and replay each failure natively from its token; `--native-survey`
     tries the cases with such a run too. `native.list` grows from 217
     cases to 248: 25 by their tests (`testing/`, `contracts/`, `sim/crash`,
     `nodes` and `rename`, `replay/tokens`, `http/server`, a package's
     failing test, `std/doc_examples`), 24 of them with nothing else to
     run, and 6 by their runs (`run/stopped_by_errors`, decision 463, two
     `unsafe/` cases whose runs the checked model does not stop, and the
     cases of decisions 462, 464 and 465); 14 of the 15 cases already in it
     that pin `sisu test` run their tests natively as well, 39 in all
     (`contracts/checks`'s wait for a property's budget, decision 461, and
     its source says so, `# native tests: WHY`, which the runner reads). Of the
     suite's 270 runnable cases 22 are left out: 15 `unsafe/` cases whose
     expectation is the checked memory model's report, and 7 that wait for
     the interpreter's test modes (port-debt.md, D-089). This changes no
     rule of the language.
461. A property test, a snapshot and a contract's generated test run
     natively: std's `check` calls `test.catch(f, x, steps)`, which the
     runtime runs as `f(x)` on the task's stack and gives `Ok`, or
     `Err(Panicked(message))` once a panic has unwound it (its frames'
     drops run, the panic taken), a typed hole going on unwinding; native
     code counts no statements, so `steps` bounds nothing and `TooLong` is
     the interpreter's (`contracts/properties`'s test of a case past its
     budget passes natively, and the case stays out of the list).
     `test.case_seed` and `test.replay_case` read what the driver gives the
     test (`SISU_TEST_CASE_SEED`; `SISU_TEST_REPLAY_CASE`, the token's
     choices, given once as the interpreter gives them), `test.record_case`
     writes the failing case's choices beside the report for the token,
     and `test.snapshot` compares the test's `k`-th snapshot with the one
     stored under its key (the driver writes the test's snapshots to a file
     `SISU_TEST_SNAPSHOTS` names, `SISU_TEST_NAME` naming the test) and
     records what it took, so `--update` stores them as the interpreter's
     run does; the line diff of a changed snapshot moves from the
     interpreter to `sisu_diag::diff`, which both use. The five intrinsics
     are appended to `sisu_rt::abi`'s and stage 2's tables, after
     `test.entry`, so the others keep their numbers. This changes no rule
     of the language.
462. A copy of a value that holds counted handles, put in a `Box` or in a
     `dyn`'s cell, counts them natively, as a copy put in a field or an
     element does (decision 237): both lowerings retain the cell's value
     after writing it when the operand is a copy, where they wrote the
     words alone, so the copy's drop and the original's released the
     handles' cells twice (`notify_all(Box(fake), ...)` with a
     `cap.Fake[Mailer]` freed its logs' mutex while the fake still held
     it, and the next lock deadlocked or corrupted the heap; the
     interpreter copies values, so it never showed). `run/counted_copies`
     holds it, and `testing/fakes`'s third test, which found it. This
     changes no rule of the language.
463. The native modes take a case whose program its errors stop
     (`run/stopped_by_errors`, decision 427) as `sisu run --native` runs
     it: the build, or `sisu sir` in the modes that build from SIR, fails
     and prints the errors and `D0006` its `.err` holds, with the status
     its `.status` gives, and nothing is run. This changes no rule of the
     language.

**`Shared` equality** (section "Running"; 02, 11; decision 384)

464. Two `Shared` values compare their values, as std's `impl[T: Eq] Eq for
     Shared[T]` says, and a `Shared` hashes as its value, where `a == b`
     lowered to the structural `eq` compared nothing the interpreter had a
     rule for (always false) and natively fell through to false as well:
     the interpreter's `equal`, `key_of` and structural hash, and the
     runtime in Sisu's `value.equal` and `value.hash`, follow a live
     `Shared` to its value, as decision 384 does for `Display`; a cell
     that is gone is equal only to itself and hashes as nothing. std gains
     `impl[T: Hash] Hash for Shared[T]`, so a `Shared` is a dict's key or a
     set's element by what it holds (`run/shared_equality`, in
     `native.list`). This changes no rule of the language: it implements
     std's impls.

**What the manual found: channels, tests, benchmarks and the
counterexample mode** (sections "Concurrency", "Testing" and "Ownership";
06, 17; the manual's findings from R9, chapters 10 and 19, G13 and G27)

465. `unbounded_channel[T]()`, 06's separate constructor, is a channel whose
     `send` never waits for room: std's `channel` with the largest
     capacity, so a value sent and not yet received is held however many
     there are, in the interpreter and natively, and `recv` waits while it
     is empty (`concurrency/unbounded_channel`, in `native.list`). Its
     `T` is `Send + Escapable`, as a channel's values are. This changes no
     rule of the language: it adds 06's constructor to std.
466. A test's output is captured and shown with its failure, as 17 says
     ("output is captured per test and shown on failure"): what a test
     prints to standard output and error is held in its report, a passing
     or blocked test's is dropped, and a failing test's block shows it
     after its header, under `what the test printed:` (`D0135`), each line
     indented, before how it failed, where every test's output was printed
     before the report. Under `--explore N` and the exhaustive search the
     run shown is the one that failed (decision 386), and a replay token's
     block holds the output its run printed, natively as in the
     interpreter. `testing/parallel`, `testing/explore_output` and
     `replay/stdin` show it. This changes no rule of the language: it
     implements 17's capture.
467. A counterexample run of `--defer-borrow` that reaches a conflict
     prints, after its account of the conflict, the blocks of the function
     it points into that the conflict passes through, the loan's, its
     end's and the use's (`sir::write::excerpt`: the function's first line,
     those blocks in order, `...` for the ones left out), where it printed
     the function's whole SIR, 40 lines for a loop of three; `R0051` says
     so. Every `.defer` expectation is the shorter form. This changes no
     rule of the language.
468. `sisu bench` times each benchmark `--samples K` times (10), after one
     run it does not time, and reports the median time of an iteration
     with the 95% interval of the mean of the samples that are not
     outliers (further from the median than three scaled median absolute
     deviations, 1.4826 MAD), by Student's t, and how many were outliers
     (`D0057`), where it reported the mean of one run: three runs of the
     manual's `parse_int` benchmark now report 17.3 to 19.5 µs, where one
     run gave 153 µs and another 369 µs. 17's JSON report is not begun.
     This changes no rule of the language.

**The runtime in Sisu: a time's text and a quoted text** (section "Stage
2", "The runtime in Sisu"; D-086, D-087 step 3)

469. A time's text is the runtime in Sisu's (`compiler/rt/src/time.sisu`):
     the RFC 3339 text of a `DateTime` that `Debug` and `Display` write,
     which `show.sisu` appends itself where it called back for it, and
     `fmt.strftime`'s directives, both over Hinnant's `civil_from_days` as
     the interpreter's `format.rs` has them; the runtime in Rust asks for
     either through `sisu_rt_time_text`, and for a hole's name quoted as
     `Debug` quotes a text through `sisu_rt_quote` (`show.quote_str`). What
     the runtime in Rust no longer has: `value.rs`'s `quote_str`, `civil`,
     `strftime`'s body and `datetime_text`, and the callback
     `sisu_rt_show_datetime`. With decision 464, the runtime's seed grows
     from 552 134 to 570 450 bytes of SIR and its object from 564 704 to
     584 912 bytes (`target/rt/rt.o`), and as the fixed point builds it
     with the Sisu backend from 1 059 600 to 1 099 920 bytes, which the
     fixed point still holds (rt-A.o and rt-B.o the same bytes, as sisu2-A
     and sisu2-B, 28 244 928, and stage1-A and stage1-B, 56 485 624). This
     changes no rule of the language.

**The driver: stage 1 says its version** (section "The query layer and
the driver"; decision 424)

470. `Stage1::sir_reports` asks stage 1 its version first: a stage 1 that
     prints `sisu1 X.Y.Z` for `--version` (stage 1's sixth round prints
     `sisu1 0.6.0`) reports the program's diagnostics from `sir --tests`
     and takes `--locked` and the features from 0.6.0 on, and is trusted
     by its version; one whose `--version` fails is older, and the probe
     of decision 424 (`sir --tests --locked` of an empty directory) asks
     it as before. The answer is kept beside the executable as it was, and
     `SISU_STAGE1_SIR_REPORTS` still decides first. This changes no rule
     of the language.

**Stage 1's seventh round** (stage 1's RULES.md, R65 to R72; black-box tests, the evaluator's `Shared`, derive diagnostics at the derive, more habits)

480. A file directly in a package's `tests/` directory is a program of
     one module whose dependencies are the package, by its name, and the
     package's dev-dependencies (17's black-box tests): it sees the
     package's `pub` items only, the package is built without its own
     tests, the file has the package's features, and its files are named
     relative to the package's directory. `sir --tests PKG/tests/NAME.sisu`
     prints its SIR with its own tests' lines, which `sisu test` runs after
     the package's own, a program each; `sisu1 test` does so, and stage
     1's version is 0.7.0. Stage 1 decides it (RULES.md, R65).

481. Stage 1's compile-time evaluator holds a `Shared` as the value it
     shares, so `==`, `hash`, a set and a dict follow it to the value as
     decision 464 has the run time do; its identity (`same`, `count`, a
     `Weak`) is beyond the evaluator, and a `Shared` kept for run time is
     `E0604`. Stage 1 decides it (RULES.md, R66).

482. R54's spelling fix is given also where the parse error expects a
     bracket or a colon (`print(n as f64)`, `for i in 0..3:`), and `#` for
     a `//` comment at the end of a line, also where it is a division by
     a name (`E0200`); a fix's message no longer names the replacement its
     edit shows. Stage 1 decides it (RULES.md, R67).

483. `E0418`, `E0420`, `E0812`, `R0069` and `R0103` write the type without
     an article ("a value of `T`", "of type `T`"); a trait's constant read
     through a type parameter where a type goes is `E0274`, which calls it
     a constant, where `E0239` called it an associated type. Stage 1
     decides it (RULES.md, R68).

484. `sisu lint` answers an option it does not take with `D0009` and
     status 2; the audit heads a program of one file "no level"; a file
     whose only parse diagnostic is `E0130` is formatted, its tree being
     whole. Stage 1 decides it (RULES.md, R69).

485. A diagnostic in an impl a derive writes is placed at the derive's
     argument that asked for it, with a note giving the impl's line and
     without the labels and fixes that point into the generated text;
     `E0243` for a std module's derivable trait names its import; the fix
     "derive it" goes to the type argument that lacks the trait when the
     type derives it, and is offered for `Arbitrary`. Stage 1 decides it
     (RULES.md, R70).

486. An import of a dependency whose `when` excludes the target is
     `E0275`, which says so, where `E0260` asked to add it to
     `[dependencies]`; a call of a function with a variadic parameter is
     not checked against its parameters, so R64's `E0900` is its one
     error. Stage 1 decides it (RULES.md, R71).

487. A parse error at Python's `lambda`, `f"..."` or `class` gets the fix
     that writes Sisu's spelling; at `try:`, `except:`, `finally:` and Go's
     `go f()` a note that says what Sisu has instead; at a keyword where a
     binding's name goes a note that it is a keyword. Stage 1 decides it
     (RULES.md, R72).

**Stage 2: the interpreter's test modes natively** (sections "Testing",
"Simulation, schedulers and replay" and "Stage 2"; 17; D-089, port-debt.md's
plan for a native `sisu run`)

500. A failed `expect` in a native test records its failure and the test
     goes on, as in the interpreter (decision 388), where natively it was
     `assert`'s panic: `expect.fail` and its two forms, in a process whose
     `SISU_TEST_REPORT` names a report, fork the process; the copy raises
     the panic `assert` would and unwinds it as a panic unwinds, each frame
     adding its line, and when the unwinding ends (the panic taken by the
     task's root, an abort, or a call of a host intrinsic, which a copy
     makes none of) it writes the trace to a pipe and exits, writing nothing
     else (its standard output and error, a hole and a `test.catch` are the
     original's); the original reads the trace, keeps the failure and
     returns. The frames are therefore those a panic there leaves, by the
     code that leaves them, with no table of return addresses. At the
     test's end the runtime writes every recorded failure, in order, before
     the panic that ended the test if one did, as the report's failure,
     whatever else the test did. Outside a test it is `assert`'s panic, as
     the interpreter's is. A copy's drops run as an unwinding's do, so a
     `Drop` of the program that writes a file through a fake writes it in
     the copy only; one that reaches the host ends the copy's walk there.
     `testing/expect` holds it (a failure in a helper, two frames, and one
     in a task). This changes no rule of the language.
501. `sisu test --native` runs PCT and the bounded exhaustive search, where
     it refused them with `D0134`: the runtime's scheduler takes
     `SISU_SCHEDULER` (`pct:DEPTH`, or `exhaustive:PREFIX` with the prefix
     dotted) and keeps the interpreter's `Strategy` in `sched.rs`: PCT's
     priorities drawn from the run's generator when a task is first asked
     about, its `depth - 1` change points drawn after the first budget from
     the first 1000 yield points, a task that gives way dropping below every
     other, and the highest-priority ready task picked; the search's choice
     at each yield point (go on, or preempt for a ready task in creation
     order) and at each switch, from the prefix and then the first option,
     each recorded with how many options it had and whether it was at a
     yield point. Under either, every yield point reaches the scheduler
     (the generated code's budget stays at 1), and the budgets the seeded
     scheduler draws are still drawn, so a seed draws what the
     interpreter's does. The search's choices go beside the report
     (`.trail`, `taken/options/y` dotted) and the driver's search, whose
     loop now runs each schedule through the native or the interpreted
     runs alike, takes the next schedule from them (`next_schedule`).
     `sim.choose` is compiled (`sim.choose`, a choice of the search or a
     draw of the generator). `sisu test --native` takes `--scheduler`;
     `sisu run --native` still does not (`D0109`). A native test build that
     fails now prints why, where its message went to the run's captured
     output and the command ended with status 1 and nothing said.
     `sim/pct`, `sim/lost_update` and the third test of
     `sim/choose_always` hold it. This changes no rule of the language.
502. The `sim` profile's faults are injected natively (`--sim --faults R`
     under `--native`, where it was `D0134`): the driver rewrites the SIR of
     the test executable it builds in the `sim` profile, so that each call
     through `FsOps` or `NetOps` whose handle's type is `Fs` or `Net` (the
     calls the interpreter's `sim_fault` looks at) first asks
     `test.sim_fault[R](handle, path)`, `R` the call's result and `path`
     its first `Path` argument, and on `true` gives its destination
     `test.sim_fault_error()` and drops what the call would have taken,
     in place of the call; the new blocks follow the call's own and the
     later blocks are renumbered after them, so a jump that was forward
     stays forward and the yield points, the back edges, are the
     interpreter's. The runtime decides as `sim_fault` does
     (`sisu-rt/src/faults.rs`): the object behind the handle must be
     `cap.SimFs`, `cap.SimNetwork` or `sim.SimNet`, the rate is
     `SISU_SIM_FAULTS` per million, the draws are from a generator seeded
     with the schedule's seed `^ 0x5157FA17` in the interpreter's order,
     the error is of the call's declared set but `TaskError`, never
     `Revoked`, its path field the call's path, its text field "a fault
     the simulation made". No lowering knows of it: both compile the
     rewritten SIR. `sim/faults`, `sim/http_faults` and `sim/no_revoked`
     hold it, replay tokens included. This changes no rule of the
     language.
503. `sim.always(what, holds, body)` is compiled: the runtime pushes the
     predicate, checks it, runs the body on the task's stack (its entry no
     yield point, as the interpreter pushes its frame), checks again when
     it returns, and pops it; meanwhile every switch checks the predicates
     on the stack of the task switched to, a task's first run included,
     with the budget set aside so that no yield point of a predicate
     reaches the scheduler, as the interpreter's `check_always` runs them
     outside it. A false predicate ends the run with `sim.always`'s
     message, as the interpreter's abort does. A predicate that locks a
     mutex another task holds (or an `RwLock` a task writes or waits to
     write) skips the check until the next switch, in the interpreter and
     natively, as one that locks the mutex its own task holds did, where
     the interpreter waited for a critical section that cannot end while
     the predicate runs, and `sisu test` hung with its memory growing (the
     manual's finding from chapter 22). `sim/choose_always` and
     `sim/always_held` hold it. This changes no rule of the language.

**Stack traces: a trait's method by its trait and type** (section "The
interpreter"; the manual's finding from R11 and R17)

504. A frame of a method of a trait's impl is named `Trait.method for
     Type`, where it was its SIR name, `Type.Trait.method` with both paths
     in it (`kiuas.meter.Dial.kiuas.meter.Meter.read` is `Meter.read for
     kiuas.meter.Dial`, `Bomb.Drop.drop` is `Drop.drop for Bomb`), in the
     interpreter and natively, and a closure in it `closure in Meter.read
     for Dial`. The SIR name stays as the front ends write it; the name a
     trace shows is `sisu_sir::display_fn_name`, which the interpreter and
     the Rust lowering share and `stage2/src/gen.sisu` repeats: a name
     whose prefix up to a dot is a type the program declares, followed by
     a path whose last name is capitalised and then the method. A stack
     overflow's message names the function the same way. `run/impl_trace`
     holds it, and two expectations of `run/` show the new name. This
     changes no rule of the language.

**`sisu audit --caps` names a test** (section "The query layer and the
driver"; the manual's finding from G32)

505. A test among the functions `sisu audit --caps` lists, and among the
     uses, is named as a trace names it, `test "loads a file"`, where its
     SIR name numbered it (`test#1`); every other function keeps its SIR
     name. This changes no rule of the language.

**The runtime in Sisu: a table's header read through its exports**
(section "Stage 2", "The runtime in Sisu"; D-086)

506. The runtime in Rust reads a dict's or set's table only through the
     runtime in Sisu's exports, `table_key_type`, `table_len`,
     `table_key_bytes`, `table_value_bytes` and `table_value_at`, beside
     the lookup, the push and the removal it called already, where
     `sisu-rt/src/dict.rs` mirrored `table.sisu`'s nine-word header as a
     `#[repr(C)]` struct: `dict::Table` is opaque, and the header's layout
     is written once. A value's address costs a call where it was a field
     read. The runtime's seed grows from 570 450 to 572 380 bytes of SIR
     and its object from 584 912 to 591 240 bytes (`target/rt/rt.o`), and
     as the fixed point builds it with the Sisu backend from 1 099 920 to
     1 106 688 bytes; the fixed point holds (rt-A.o and rt-B.o the same
     bytes, as sisu2-A and sisu2-B, 29 170 464, and stage1-A and stage1-B,
     57 638 096, which carry decisions 500 to 504 too). This changes no
     rule of the language.

**Two regressions of decision 466** (section "Testing"; 17; decisions 340
and 466; the manual's round)

507. A blocked test's block shows what it printed, where the capture of
     decision 466 dropped it with a passing test's: a typed hole's closure
     (`R0160`, "in scope at the typed hole ..."), which stage 1 makes the
     program print where the hole is reached (R27), is the test's output,
     so `sisu test` showed a blocked test with no closure, where 17 says it
     stops with it and decision 340 printed it. After the summary's lines,
     a blocked test that printed anything has a block of its own, its
     header (`... blocked by todo("name")`) and what it printed under
     `what the test printed:` (`D0135`), as a failed test's block shows
     it, in the interpreter and natively; a passing test's output stays
     hidden. And a diagnostic the test runner issues itself, a native
     build's failure or `D0134`, goes to standard error rather than into
     the capture, where `sisu test --native --sim` printed nothing and
     exited 1. `testing/blocked_closure` holds the first, in
     `native.list`, and `run/testing`'s expectation shows its closure
     again. `sisu --version` prints Cargo's version of the driver, `sisu
     0.0.1`, which is the toolchain's version replay tokens record; it is
     left as it is. This changes no rule of the language.

**The driver: a package's black-box tests** (section "Testing" and "The
query layer and the driver"; 17; stage 1's seventh round, R65, decision
480)

508. `sisu test` of a package (each member of a workspace's) runs, after
     the package's own program, each file of its `tests/` directory,
     `*.sisu` sorted by name, as a program of its own: stage 1's `sir
     --tests PKG/tests/NAME.sisu` with the options it is given for the
     package (`--locked`, `--features`, `--no-default-features`,
     `--accept-unsafe`, `--contracts=`), whose SIR is run (natively under
     `--native`) and reported as any program's, its lines, failures and
     summary, a replay token naming the file (so `sisu test --replay
     TOKEN tests/NAME.sisu` runs it alone, and a replay of the package
     runs none); the status is the worst of the programs', a blocked
     one's yielding to a failure (decision 380). The driver does so for a
     stage 1 that says it is `sisu1 0.7.0` or later
     (`Stage1::black_box_tests`; `SISU_STAGE1_BLACK_BOX` decides first),
     where a file of `tests/` was never run. This changes no rule of the
     language.

**Stage 1's eighth round** (stage 1's RULES.md, R73 to R78; fixed test names, modules behind imports, dev-dependencies under `@cfg(test)`, diagnostics at the instance's call, literals beside `BigInt` and `Decimal`)

520. The name of a test, a suite or a benchmark is fixed text: an
     interpolation in it is `E0132`, with the fix that doubles its braces,
     and what the braces hold is not parsed. Stage 1 decides it (RULES.md,
     R73).

521. The name a child module has in its parent gives way to an import of
     that name there, and the module is still reached by its path; `E0304` on a local named like an imported module says the
     local hides it. Stage 1 decides it (RULES.md, R74).

522. A package's code imports its dev-dependency only by an import marked
     `@cfg(test)`; any other import of it is `E0276`, in every build, and
     the black-box tests of `tests/` import it freely. Stage 1 decides it
     (RULES.md, R75).

523. `else` after a statement that is no `let` notes that it belongs to a
     `let` and fixes `else as e` to `if let Err(e) = ...`; `d.get(k,
     default)` is fixed to `d.get(k) ?? default`; a positional-only
     parameter passed by name is noted, fixed and taken as passed; a std
     module's error set written alone is named through its module; and
     `E0981` writes `ptr mut T` as the program does. Stage 1 decides it
     (RULES.md, R76).

524. A diagnostic in a template's instance is placed at the call that made
     it, with a note giving the instance's line and without the labels and
     fixes that point into the generated text; a typed hole of a derive's
     template is reported once; an untyped binding's interpolated literal
     of compile-time values stays a `String` in the instance; and
     `E0342`, `E0369`, `E0438` (of `sorted`) and a conflict inside a loop
     the borrow lasts through say what the program writes instead. Stage 1
     decides it (RULES.md, R77).

525. A literal operand beside a non-numeric type that implements
     `FromLiteral` for it becomes that type when the operator has no impl
     for the literal as it is; an explicit discriminant is `E0900` until
     the layouts of `@repr` enums are lowered. Stage 1 decides it
     (RULES.md, R78).

**Stage 2: a property's budget natively** (sections "Property tests" and
"Stage 2", "Tasks"; 17; decision 461, port-debt.md's plan for a native
`sisu run`)

540. A property's case runs natively within its budget: `test.catch(f, x,
     steps)` gives `Err(TooLong(steps))` when the call passes more than
     `steps` yield points (function entries and back edges, over every
     task, as the interpreter counts its statements over every task), where
     native code bounded nothing and a case that ran for minutes was not
     stopped. A yield check can unwind: `sisu_rt_yield` returns a status
     (`() -> (i8)`), and its slow path branches on it, to the failure path
     of the block it is in, or at a function's entry to a block of its own
     that gives the frame's count back and returns `UNWINDING`
     (`yield_check(entry)`), the same blocks in the same order in the Rust
     lowering and stage 2's, so that `--native-from-lir` finds the
     executables the same bytes. The runtime keeps both budgets in the one
     counter the generated code counts down (`sisu-rt/src/budget.rs`):
     while a case is open the counter holds the smaller of the case's
     budget left and the seeded scheduler's, the rest of the scheduler's
     kept aside, and every read and write of it but the generated code's
     goes through `budget::load`, `store` and `spend`, so the scheduler's
     budget means what it did and a seed names the same schedule (the
     cases of `sim/` and `concurrency/` show it); a yield point that finds
     the counter spent asks the case first: one that ran out raises
     `run.case-too-long` and unwinds (not while a panic is already
     unwinding, and the unwinding's drops run without a budget, as the
     interpreter's), one whose budget only was spent goes on without the
     scheduler. A case's count is of yield points, not statements, so the
     budget's number means more work natively than in the interpreter;
     only a case that loops past it shows the difference, and
     `contracts/properties`' case of 100 000 iterations past 2 000 fails
     both ways. `contracts/properties` joins `native.list` and
     `contracts/checks` runs its tests natively; the runner's `# native
     tests: WHY`, which no case needs now, is gone. This changes no rule
     of the language.

**`sisu run --native --scheduler`** (section "Simulation, schedulers and
replay"; decision 501)

541. `sisu run --native` takes `--scheduler pct` and `--scheduler
     exhaustive` with `--depth`, where it refused them with `D0109`: the
     runtime's schedulers of decision 501, asked for through
     `SISU_SCHEDULER` (`native_tests::scheduler_env`, which the tests
     share); PCT runs once or under `--explore N` seeds, reported with the
     interpreter's `CLI_PCT_FAILED` and `CLI_PCT_OK`, and the bounded
     exhaustive search runs the executable under each schedule as
     `run_main_search` runs `main`, the runtime writing each run's choices
     to the file `SISU_TRAIL` names, from which `next_schedule` takes the
     next prefix, up to `--explore N` schedules or 10 000, with the
     interpreter's summary or failing schedule. `driver/tests/native.rs`
     holds four runs of a lost update to the interpreter's, byte for byte.
     This changes no rule of the language.

**The runtime in Sisu: iterators made by their exports** (section "Stage
2", "The runtime in Sisu"; D-086; decision 423)

542. The runtime in Rust makes every iterator through the runtime in
     Sisu's exports, one for each of the eleven kinds (`sisu_rt_iter_range`,
     `_list_ref`, `_items`, `_chars`, `_bytes`, `_owned_bytes`,
     `_enumerate`, `_zip`, `_table_ref`, `_generator`, `_receiver`), and
     asks it what an iterator may wait for (`sisu_rt_iter_waits`), where
     `sisu-rt/src/iter.rs` mirrored `iter.sisu`'s eleven words as a
     `#[repr(C)]` struct and wrote a new object's words itself:
     `iter::IterObj` is opaque, and the object's layout is written once,
     in `iter.sisu`, but for the four words of its header that compiled
     code reads (decision 253). `sum` takes its items' size from its
     result's type. The seed grows from 572 380 to 586 692 bytes of SIR
     and the runtime's object from 591 240 to 617 856 bytes
     (`target/rt/rt.o`), and as the fixed point builds it with the Sisu
     backend from 1 106 688 to 1 147 096 bytes; the fixed point holds
     (rt-A.o and rt-B.o the same bytes, as sisu2-A and sisu2-B,
     31 464 040, and stage1-A and stage1-B, 61 475 288, which carry
     decisions 540 and 543's lowering too). This changes no rule of the
     language.

**The native checked memory model, begun** (sections "The checked memory
model" and "Raw memory, foreign calls and the machine intrinsics"; 22;
D-085; decisions 301 and 302)

543. A program built in the debug profile checks its raw memory as the
     interpreter's model does, with the interpreter's panics, for the
     memory the runtime can name; the release profile keeps the unchecked
     operations of decision 302. Both lowerings set `ENTRY_CHECKED` among
     `main`'s flags in the debug profile, and there compile no raw read,
     write or offset inline (decision 335): each is a call of the runtime,
     which checks it before it does it (`sisu-rt/src/checked.rs`); the
     runtime's own code, the library, still reads and writes inline and is
     not checked. The model is a shadow of the blocks `Global.acquire`
     gives out, each with its size, its alignment, whether it is live, and
     what it holds as the interpreter's block does: the values written
     into it at their offsets with their types, and the bytes left of a
     plain value a write cut apart. Native pointers are addresses, so a
     pointer belongs to the block on whose bytes or end its address falls;
     an address below 64 KiB (Linux's `mmap_min_addr`) has no memory
     behind it, so a dangling pointer is seen; any other address is a
     place of the program or memory foreign code gave out, and is not
     checked. The checks are the interpreter's, in its order: memory,
     liveness, bounds and alignment on every access; a read finding a
     value of its type (taken when the type needs a drop), or bytes that
     values of plain types give, whose bits are then checked for the type
     on the program's memory, or nothing; a write refused over a value that
     needs a drop; `offset` leaving its block; `Global.release` of memory
     it did not give out or not at its start, released already, of another
     layout, or holding a value never dropped (not while a panic unwinds);
     `copy_from` slot by slot; `drop_in_place`, `as_ref`, `as_mut` and
     `slice_from_raw` on what they reach; and the bits `transmute` and
     `bit_cast` give. A released block keeps its address and its memory
     for a while, the last 4 096 released, so that a pointer into it is
     seen to point into released memory, as AddressSanitizer's quarantine
     does; beyond that its memory is freed and its record forgotten. A
     foreign call (the `sisu_rt_flush` every one makes first) marks every
     live block exposed, whose bytes that hold no value then read as plain
     bytes, since C may have written them. At the end of a run the values
     left in live blocks are forgotten and the blocks freed, as the
     interpreter drops its model with the run, so that only the runtime's
     own allocations are counted at exit. The other choices were tagged
     pointers, which would carry a block and the borrow a pointer came
     from as the interpreter's do but change every pointer's word and the
     C calls; shadow memory for every address, which needs the stack's
     and the heap's places registered as they come and go; and the
     interpreter for every program that declares `unsafe` (Q-77's second
     option), which keeps the checks but not the native run. A raw read
     or write in the debug profile costs a call of the runtime, its
     descriptor and a lookup under a lock: two million of them take 2.3 s
     natively, 7.1 s in the interpreter and 0.03 s in the release profile.
     What the model leaves out natively: the provenance of a pointer
     (whether it came from a shared borrow, `R0139`), the liveness of a
     place of the program (`R0135`), which the monitor's loans and the
     interpreter's frames give, a reference's reads after its value was
     dropped through the pointer, memory foreign code gave out (`malloc`'s
     blocks, `getenv`'s text), and the read-only memory and emulated C
     functions of the interpreter (`R0148` to `R0152`), whose calls are
     real ones natively. This changes no rule of the language: it
     implements 22's checked model in a debug build.
544. The first slice runs 12 of the 16 `unsafe/` cases the native list
     left out with the interpreter's panics: `invalid_bits`, `leaked`,
     `misaligned`, `mistyped`, `no_memory`, `out_of_bounds`, `overwrite`,
     `release_layout`, `release_other`, `released`, `released_twice` and
     `uninitialised`. Each is marked `checked` in `native.list`, which
     `--native-release` reads (`native::read_checked`): it builds the case
     in the release profile, whose raw memory is unchecked, and does not
     run it; the other five modes run it, `--native-from-lir`'s
     executables the same bytes in both profiles. A refused write drops
     the value it was given, and a refused overwrite leaves the value it
     would have lost in place for the end of the run. Left out:
     `shared_write` (a pointer's provenance), `gone` (a frame's place),
     `reference_behind` (a reference read after a drop through the
     pointer) and `foreign_unknown` (a C function the interpreter does not
     emulate, which native code calls, so that one stays the
     interpreter's). `native.list` grows from 258 cases to 271, 50 of them
     with their tests natively. This changes no rule of the language.

**What the manual found: `expect`, `sisu bench`, `std.regex` and
`std.num`** (sections "Testing" and "The query layer and the driver"; 11,
17; the manual's findings from chapters 19 and 24, R15, G8 and G27)

545. A failure `expect` records is reported in the words of a failed
     expectation, `expectation failed: double(3) == 6` (`R0154`, and
     `R0155` with a message), with the frames a panic there would leave and
     its sides as `assert`'s, where it was `panic: assertion failed: ...`
     though nothing panicked and the test went on; the panic that ends a
     test, an `assert`'s among them, keeps its header. Outside a test
     `expect` is still `assert`'s panic. The interpreter and the runtime
     write the same block (`sisu_interp::expect_report`, and
     `rt::expect_failed`); `testing/expect` shows it. This changes no rule
     of the language.
546. `sisu bench --samples K` takes 2 or more (`D0136`), where 0 and 1
     timed one sample without a word and reported "the median of 1
     samples" with an interval that was the one time; the mean of one
     sample has no interval. This changes no rule of the language.
547. `std.regex` names what its syntax leaves out where it meets it: a
     back-reference (`\1`), "which a match in linear time leaves out", a
     flag (`(?i)`) and a named group (`(?P<h>...)`, `(?<h>...)`), which
     stage 0 leaves out, `(?:...)` being the one group flag, and a Unicode
     class (`\p{L}`); its module's comment says so. `std.num`'s `Rounding`
     says that Python's `decimal` has an eighth mode, `ROUND_05UP`, which
     the enum leaves out. This changes no rule of the language.

**Stage 1's ninth round** (stage 1's RULES.md, R79 to R94; the REPL's and worksheets' programs, sized shifts, `expect` with a message, `lint` for unused imports, the findings' notes)

580. Stage 1 builds the programs of `sisu repl` and `sisu ws`: `sir --repl
     SESSION INPUT --kept OUT [--load]` gives the SIR of the session's
     program with the input after it (an expression alone shown), or the
     diagnostics of the phase that stopped it, those in the input drawn in
     its text; `sir --worksheet FILE` gives the worksheet's SIR with each
     top-level statement marked by the line it ends on in the file as
     written and its values shown, or its diagnostics in the file as
     written. `sisu1 --version` is 0.8.0. Stage 1 decides it (RULES.md,
     R79).

581. A sized integer shifted by an amount of any integer type keeps its
     own type, and an amount outside its width traps; a literal on a
     shift's left keeps `int`. This reverses the sixth round's reading of
     `u32 << int` as widening to `int`. Stage 1 decides it (RULES.md,
     R80).

582. `expect` takes a message as `assert` does, `expect cond, message`,
     which its recorded failure shows. Stage 1 decides it (RULES.md, R81).

583. An assertion whose condition is a call that takes a call evaluates
     that argument once and shows its value and the parts that only read
     when it fails. Stage 1 decides it (RULES.md, R82).

584. The test `@refines` adds stands at the attribute, in the file of the
     function it checks, with the closure it checks the cases with. Stage
     1 decides it (RULES.md, R83).

585. A `comptime` expression that reaches a typed hole is left to run time,
     where the hole blocks the test or the run that reaches it; the build
     warns of the hole. Stage 1 decides it (RULES.md, R84).

586. `E1022` is followed by `D0006` or `D0132`, as the manifests' other
     errors are. Stage 1 decides it (RULES.md, R85).

587. A manifest's `edition` is one the compiler knows, `"2027"` so far;
     another year is `E1003`. Stage 1 decides it (RULES.md, R86).

588. `sisu fmt` keeps an unused import; `sisu lint` reports it (`W0027`)
     with the fix that removes it. Stage 1 decides it (RULES.md, R87).

589. `E0402` for tasks a loop spawns that change a captured value names
     the value and notes that the tasks run at once and a `Mutex` keeps
     it; `E0428`'s note offers making the value before the scope. Stage 1
     decides it (RULES.md, R88).

590. A closure argument of a `T !` function keeps the inferred error set
     open, so a `?` in a scope's body adds to it. Stage 1 decides it
     (RULES.md, R89).

591. `E0335` names the operation that makes the function passed impure,
     followed through the program's functions, with a note giving the way.
     Stage 1 decides it (RULES.md, R90).

592. `E0204` for an item a `@cfg` left out notes the condition that did.
     Stage 1 decides it (RULES.md, R91).

593. A derive's template sees a struct field's `ty` as its declaration
     writes it, with the kinds of the prelude's types. Stage 1 decides it
     (RULES.md, R92).

594. A function cannot be named `type`: `E0124`. Stage 1 decides it
     (RULES.md, R93).

595. `parse --shape` refuses what `fmt` refuses, `E0132` among it, and a
     template instance's note at its call writes the type arguments as the
     source does. Stage 1 decides it (RULES.md, R94).

**The native checked memory model: the places of the program** (sections
"The checked memory model" and "Raw memory, foreign calls and the machine
intrinsics"; 22; D-085; decisions 543 and 544)

560. A program built in the debug profile refuses an access through a
     pointer to a place whose frame has returned (`R0135`) and a write
     through a pointer that only shared borrows made (`R0139`), with the
     interpreter's panics, where native code saw neither. The runtime
     keeps the stack pointer of the generated code's last call of an
     intrinsic: `sisu_rt_intrinsic` is an entry of four instructions
     (`#[unsafe(naked)]`, x86-64 and AArch64 Linux) that writes it to the
     thread's word `SISU_RT_CALLER_SP` and jumps to the call, so no
     lowering changes and every frame of the running task that is still
     live lies at or above it. `ptr.of(x)` and `ptr.of_mut(x)` record the
     place their pointer reaches when it is on the stack of the code that
     asks (at or above that stack pointer, below the top of its stack: a
     task's slot, or the root task's stack from where the scheduler
     started), with the top that names the stack and whether a shared or
     a mutable borrow made a pointer to it (`checked::place`, at most
     4 096 places, the oldest forgotten first). An access through an
     address that no block holds and a remembered place of the same stack
     covers is refused as gone when the address is below the caller's
     stack pointer: below every live frame. A write, `write_volatile`,
     `copy_from` into it, `drop_in_place` and `as_mut` are refused when
     every pointer made to the place came from a shared borrow, and so is
     a write into the text `getenv` gave (decision 561), whose pointer the
     interpreter makes shared. Both checks are of addresses, not of the
     pointers' provenance, so they say less than the interpreter's: a
     frame that returned and whose stack another call of the same depth
     now covers is taken as live when the access comes from that call or
     a deeper one, and a write through a pointer made shared is allowed
     when a mutable borrow also made a pointer to an overlapping place
     (both are panics in the interpreter); a pointer from
     `ptr.from_address` of a place's address is refused a write where the
     interpreter allows it. Neither check refuses an access the
     interpreter allows otherwise. The tagged pointers of port-debt.md,
     the pointer's high 16 bits naming its borrow, are the sound form and
     change every pointer's word and the C calls; the generated code
     passing its frame's base to `ptr.of`, the other way, changes both
     lowerings. Two million reads and writes through a place take 0.31 s
     in the debug profile, 0.011 s in the release one and 3.8 s in the
     interpreter. `unsafe/gone` and `unsafe/shared_write` join
     `native.list` as `checked`. This changes no rule of the language.

561. The debug profile checks the memory C gives out: a call of one of the
     C functions the interpreter emulates (`malloc`, `free`, `getenv`,
     `strlen`, `memcpy`), with words for its arguments and result as their
     declarations in C have them, is a call of the runtime's
     `sisu_rt_c_NAME` of the same signature, followed by a status from
     `sisu_rt_c_checked` that leads to the failure path
     (`abi::checked_c`, and `checked_c` in stage 2's `abi.sisu`; both
     lowerings, the same bytes). The runtime calls C's function and keeps
     what the model needs: `malloc`'s block is one of the model's,
     uninitialised, aligned to 16, which `free` alone takes back (refused
     as the interpreter refuses it: not at a block's start or another
     allocator's, freed already, holding a value never dropped) and whose
     memory is kept in the quarantine of decision 543, then given to C's
     `free`; `getenv`'s text is a block to read only; `strlen` and
     `getenv` read their argument up to its NUL inside its block, the read
     past the end refused; `memcpy` reads plain bytes from its source,
     refuses ranges that overlap and leaves bytes at its destination. A
     `Global.release` of `malloc`'s memory and a `free` of `Global`'s are
     `R0142`. `write` and every other C function are called as before:
     their memory is the program's or foreign. `unsafe/malloc_bounds`,
     `unsafe/free_twice` and `unsafe/unterminated` hold it, in
     `native.list` as `checked`. This changes no rule of the language.

**The runtime in Sisu: a list's indices** (section "Stage 2", "The runtime
in Sisu"; D-086)

562. The runtime in Sisu checks the index of a list's `insert`, `remove`
     and `swap` (`export_list_insert`, `_remove`, `_swap`, which give a
     status now), and for one outside the list asks the runtime in Rust to
     raise the panic its catalog names (`sisu_rt_list_bounds`), where the
     runtime in Rust checked it before calling them; an element `insert`
     refused is dropped by the runtime in Rust after the panic is raised.
     The intrinsics' decoding stays the runtime in Rust's: a descriptor's
     read is four words and two slices, and moves with the dispatch it
     feeds, which is the next piece (D-086's order: the dispatch of a
     list's and a table's intrinsics by Sisu code, their panics raised
     through entries of the runtime in Rust, since a message is the
     catalog's and the runtime's code has no text of its own; about three
     days). The seed grows from 586 692 to 589 082 bytes of SIR and the
     runtime's object from 617 856 to 619 984 bytes (`target/rt/rt.o`),
     and as the fixed point builds it with the Sisu backend from 1 147 096
     to 1 151 544 bytes; the fixed point holds (rt-A.o and rt-B.o the same
     bytes, as sisu2-A and sisu2-B, 32 221 112, and stage1-A and stage1-B,
     62 490 080, which carry decision 561's lowering and the standard
     library's changes of decisions 564 to 567 too). This changes no rule
     of the language.

**The driver: the REPL and worksheets on stage 1** (section "The REPL and
worksheets"; 10; port-debt.md, "The Rust front end's roles still to
replace")

563. `sisu repl` and `sisu ws` check and lower their programs through
     stage 1 when it serves them, `sisu1 0.8.0` and later
     (`Stage1::serves_repl`, asked once; `SISU_STAGE1_REPL=1` or `0`
     decides whatever the version says), and through the Rust front end
     for an older stage 1 or `--front-end=rust`; the language server's
     worksheets stay the Rust front end's. A session's program, and a
     worksheet's, is written to a file of a directory of its own
     (`repl::Checker`), named as the worksheet or `session.sisu`; stage
     1's `sir` gives its SIR, the file's path replaced by the program's
     name (`<repl>`, or the worksheet's), which the interpreter runs as
     before, against the log of the run before it; and a program that
     does not check is reported by stage 1's `check`, whose places in the
     input are moved there (`relocate`: each `--> <repl>:LINE:COL` and the
     numbers of its gutter, the gutter as wide as they then are), as the
     Rust front end's diagnostics are. A worksheet's program names the
     run's fake `System` from a function of its own (`__ws_fake`), since
     stage 1 keeps what the program's functions reach. An expression is
     shown by a call on lines of its own (`__repl_show(` before it), so the
     input keeps its lines and columns, and a panic in a session names the
     program's line one later for each expression before it
     (`live/session` says `<repl>:21:1`, where it said 19). The suite's
     sessions and worksheet give their expectations through stage 1 too
     (`repl_and_worksheets_run_on_stage_1`), with `sisu1 0.7.0` asked for
     it: `max(scores)`, `xs[1:^1]` and a generator expression work in a
     worksheet. A worksheet without its `#!` line names its line as
     written in stage 1's report, where the Rust front end's is one later.
     The rest of what stage 1's round offers the REPL is relayed to the
     driver when it lands. This changes no rule of the language.

**What the manual found: iterators, `DateTime`, `FakeNet` and `std.log`**
(sections "Formatting", "Testing", "Capabilities"; 11, 17; the manual's
findings from chapter 18 and guides G9, G10 and G28)

564. `Iter[T]`, the iterator of the standard library's methods, whose
     `next` is `pure`, has `pure` consumers of its own that take no
     function: `count`, `nth`, `collect`, `sum` and `join`, as it has
     `fold` with a `pure` function, so a `pure` function counts, collects,
     sums or joins the items of `split_whitespace()`, `iter()` or
     `range(n)` (E0333 before). An adapter's (`map`, `filter`, `take`,
     `skip`, `enumerate`) stay `Iterator`'s, which are not `pure`, since
     their `next` calls a function that may be any (03, "Function types");
     a `pure` chain through `map` needs purity over the monomorphised
     call graph, which the checkers do not compute (stage 1's, if wanted).
     The frozen Rust front end resolves the trait's method before the
     type's and still refuses them. `std/pure_iterators` holds it. This
     changes no rule of the language.

565. A `DateTime` has its parts in UTC, as methods: `year`, `month`,
     `day`, `hour`, `minute`, `second`, `nanosecond`, `weekday` (ISO 8601's
     numbers, 1 for Monday) and `day_of_year`, by Howard Hinnant's
     `civil_from_days` (the prelude's, beside `days_from_civil`), on any
     time of the years 1678 to 2261 (`std/time_parts`). `Instant` and
     time zones, which 11 lists, are still missing: the zones need the
     embedded data of `std.x.tzdata`. This changes no rule of the
     language.

566. `http.FakeNet` scripts any answer: `respond(method, url, response)`
     answers a method of a URL with any `Response` (a `503` among them),
     `respond_after(..., delay)` does so once `delay` has passed on the
     run's test clock (`Clock.fake()`), which the client waits for at its
     first read, and `respond_to(url, body)` is `GET` with `200 OK` as
     before; a route's URL is written as `Url` writes it, so a default
     port matches. `http/scripted` holds it, natively too.
     `HttpError.Network` still holds the `NetError` as text. This changes
     no rule of the language.

567. `std.log` has levels and fields: `Level` (`Error`, `Warn`, `Info`,
     `Debug`), `log.at(level, message)`, and a `Logger` that writes the
     records at its level and the more severe ones and carries fields
     after the message (`with(key, value)`, written `key=value`, the value
     quoted as `Debug` writes it when it holds a space, a quote or `=`),
     with `at_level`, `enabled` and `record`; the module's own functions
     still write every level, since the facade is ambient and a setting of
     the process would be state no program declares. `std/log_levels`
     holds it. This changes no rule of the language.

**Stage 1's tenth round** (stage 1's RULES.md, R95 to R103; the REPL's `--dir`, `--name` and `parse --bindings`, variants' `ty` in derives, similar-name fixes, invisible characters, the findings' fixes, adapters' purity, worksheets' options and results)

600. Stage 1's `sir --repl` and `sir --worksheet` take `--dir DIR`, where
     the program stands (`embed` and the inline metadata's paths), and
     `--name NAME`, what its diagnostics and SIR call it; `parse --bindings
     INPUT` gives each top-level statement of an input with what it binds,
     its initialiser's bytes, the names that reads and the names it
     changes, which `:why NAME` reads. `sisu1 --version` is 0.9.0. Stage 1
     decides it (RULES.md, R95).

601. A derive's template sees the `ty` of a variant's fields as written,
     and the kinds of the structs and enums the module names. Stage 1
     decides it (RULES.md, R96).

602. `E0203` offers the path with a module of a similar name, and `E0271`
     the trait's constant of a similar name. Stage 1 decides it (RULES.md,
     R97).

603. `E0310` of two sized integers that an `int` both holds offers making
     the left one an `int`. Stage 1 decides it (RULES.md, R98).

604. `E0812` notes that the copy to change is one typed as the base. Stage
     1 decides it (RULES.md, R99).

605. `E0005` writes a character that shows as a space or as nothing as its
     escape, names it, and offers a space or its removal. Stage 1 decides
     it (RULES.md, R100).

606. Stage 1 gives the fixes and notes the manual found missing: `ref`
     before a value lent to a `ref dyn` parameter, `E0811` for a `ref int`
     where a refinement goes, `BigInt` for a literal beyond `int`, the impl
     a number on an operator's left needs, the type a derive's fix is for,
     one `E0200` for `Deque[int]()`, the spellings of `f"..."` after `=`
     and of a generator as a call's only argument, `case not`, a method
     named in capitals, `@cfg` on the target; and five fixes no longer
     name their replacement. Stage 1 decides it (RULES.md, R101).

607. Purity is not inferred over the monomorphised call graph: an
     iterator's adapter takes any function and is not `pure`, and `E0333`
     at one notes `fold` with a `pure` function. Stage 1 decides it
     (RULES.md, R102).

608. `sisu lint` reports an import the resolver refused by its error
     alone, and a worksheet shows an expression statement's option or
     result. Stage 1 decides it (RULES.md, R103).

**The driver: the REPL and worksheets on stage 1's own programs** (section
"The REPL and worksheets", "The query layer and the driver"; 10; stage 1's
RULES.md, R79 and R94; port-debt.md, "The Rust front end's roles still to
replace")

620. `sisu repl` and `sisu ws` build their programs with stage 1's `sir
     --repl SESSION INPUT --kept OUT [--load]` and `sir --worksheet FILE`
     (decision 580), one run of stage 1 for each input, where decision 563
     ran `sir` and then `check` on a file the driver had wrapped or
     instrumented: the session's program is the OUT of the last input
     kept (or, for a session that loads a package, the `#!` line and the
     package's inline metadata), each input is written as typed and its
     diagnostics come on stage 1's standard error with the input's places
     drawn in it as `<input>` and the session's as `<repl>`, which the
     driver shows as they are (`relocate` is gone); a file loaded into a
     session (`sisu repl FILE`) is given with `--load`, its diagnostics
     drawn in the whole file under the name given. A worksheet's SIR marks
     each statement by the line it ends on in the file as written
     (`__ws_at`, `__ws_show`) and keeps `System.fake`, so the driver runs
     it as it is and no longer needs the statements' spans nor its own
     instrumenting. An expression is shown by `__repl_show(INPUT\n)`, its
     `)` on a line of its own, so a panic in a session names the
     program's line one later for each expression before it, as before
     decision 563 (`live/session` says `<repl>:19:1` again); the Rust
     front end, the fallback for a stage 1 older than 0.8.0 and under
     `--front-end=rust`, wraps an expression the same way, so both give
     the same transcript. A session's input costs one run of stage 1
     (about 0.35 s for a small one), and a worksheet one (0.8 s for
     `live/worksheet`). This changes no rule of the language.

621. `:why NAME` reads the inputs it follows from the tree `sisu1 parse
     --tree` prints of each (the input after a `#!` line, read once and
     kept with the input), through a reader of that printed form
     (`repl::Syntax`: each node's kind, range and tokens), where it parsed
     each input with the Rust parser at each question; under the Rust
     front end the tree is its parser's `dump`, the same text, so the
     bindings an input makes, what an initialiser reads and which later
     input changes a value are found by one code. A value `:why` shows is
     still a run of the session's program with the name as the input,
     now one run of stage 1 each. This changes no rule of the language.

622. `sisu parse --shape` is stage 1's (`stage1_serves`), as `parse
     --tree` is: it refuses what `sisu fmt` refuses, a test's name that
     interpolates (`E0132`) among it, so the conformance runner's format
     check, which asks `parse --shape` whether a file parses and then `sisu
     fmt` to format it, sees the two agree; the Rust front end's, which
     prunes an unused import from the shape where stage 1's keeps it
     (decision 588), answers under `--front-end=rust`. `sisu help` names
     `parse --shape`, `repl` and `ws` among the commands stage 1 serves.
     This changes no rule of the language.

**What the manual found: the bits of the integers, `HttpError` and time
zones** (sections "Numbers, bytes and slices", "Capabilities"; 03, 11; the
manual's findings from chapter 24 and guides G9 and G10)

623. The sized integers and `int` have bit methods in the prelude, written
     in Sisu: `count_ones`, `count_zeros`, `leading_zeros` and
     `trailing_zeros` (an `int`; the width for 0), `rotate_left(n)` and
     `rotate_right(n)` by any `int` amount (taken modulo the width, a
     negative one rotating the other way), `swap_bytes`, `to_be` (the bytes
     swapped, every target of stage 0 being little-endian) and `to_le`;
     and a signed type's `to_bits` and `from_bits`, its two's complement as
     the unsigned type of its width (`(-1i8).to_bits() == 255u8`,
     `int.from_bits(u64)`), through which its bit methods are its
     two's complement's. The names are Rust's, since 03 names none. The
     bodies shift by amounts of the integer's own type (`x >> 1u32`,
     `k.truncate_to[u32]()`), which both front ends type the same: stage 1
     keeps a sized type under a shift by an `int` (decision 581), the
     frozen Rust front end, which checks std, widens `u64 << int` to
     `int` and refuses it (`E0310`), so no method needed a form only one
     of them takes. An `i128`'s bits are built from parts of 32 bits,
     since no conversion between `i128` and `u128` exists. They are loops
     over the bits, not the machine's `popcnt` or `bswap`, which an
     intrinsic in the interpreter and both backends would give. The block
     stands at the end of the prelude, so the lines of the functions
     before it, which a program's SIR records, do not move. Doc examples
     (11 of them, `sisu test --std`) and `std/bit_methods` hold it,
     natively too. This changes no rule of the language.

624. `HttpError.Network` holds the network's error, `Network(cause:
     NetError)`, where it held its text, so a client tells a refused
     connection (`Refused(address)`) from one closed early (`Closed`); a
     wait cancelled or a task that panicked, the `TaskError` half of what
     `Net`'s operations return, is the new variant `Interrupted(cause:
     TaskError)`. `std.x.http`'s `over_net` lifts a result of `{NetError,
     TaskError}` into `HttpError` (a union of error sets is a type only
     after `!`, so the lift matches a result). `http/server` and
     `sim/http_faults` show it. This changes no rule of the language.

**The runtime in Sisu: a list's and a table's intrinsics** (section
"Stage 2", "The runtime in Sisu"; D-086; decision 562)

625. The intrinsics of lists, dicts and sets that the generated code calls
     through `sisu_rt_intrinsic` (35 of them: `list.new` to `list.sort`,
     `dict.new` to `dict.items`, `Entry.or_insert`, `set.new` to
     `set.is_empty`) are decoded and done by the runtime in Sisu
     (`rt/src/dispatch.sisu`, the export `sisu_rt_collection`): the runtime
     in Rust reads the descriptor's first word, the intrinsic's number,
     and hands the descriptor and the addresses to it on the system stack;
     it reads the types of the result and the arguments from the
     descriptor (a byte at a time, the runtime's code reading no `u32` of
     its own), finds the values behind references and `dyn`s, checks the
     indices and does the call by `list.sisu` and `table.sisu`, and asks
     the runtime in Rust for what is that runtime's own through four
     entries: a slice's bounds below 0 and a sort without an order raise
     their panics (`sisu_rt_slice_bounds`, `sisu_rt_unordered`, beside
     decision 562's `sisu_rt_list_bounds`), a drop, which may run a
     program's glue on the task's stack (`sisu_rt_drop_value`), and a
     `*rest` binding's view, an object it keeps (`sisu_rt_list_view`).
     The numbers are the runtime in Rust's (`abi::Intrinsic`), which a test
     of `sisu-rt` holds `dispatch.sisu`'s constants to. Deleted from the
     runtime in Rust: the 27 arms of its dispatch that did them, the
     wrappers of the exports they called (`list_pop`, `list_insert`,
     `list_remove`, `list_extend`, `list_swap`, `list_join`,
     `table_remove`, a table's key type) and those exports themselves in
     the runtime in Sisu. Still the runtime in Rust's: the iterators of a
     list and a set (`list.iter`, `set.iter`, which share their arm with a
     receiver's), `in` as an operator (`ValueOp::In`), a slice's
     intrinsics, a literal's table and the inline lookups' entry
     (`sisu_rt_table_find`). The seed grows from 589 082 to 649 669 bytes
     of SIR and the runtime's object from 619 984 to 686 800 bytes
     (`target/rt/rt.o`), and as the fixed point builds it with the Sisu
     backend from 1 151 544 to 1 269 416 bytes; the fixed point holds
     (rt-A.o and rt-B.o the same bytes, as sisu2-A and sisu2-B, 32 323 848,
     and stage1-A and stage1-B, 63 243 656, which carry the standard
     library's changes of decisions 623, 624 and 627 too). This changes no
     rule of the language.

**The native checked memory model: faster checks** (sections "The checked
memory model"; 22; D-085; decision 543)

626. A raw read or write in the debug profile takes half the time it took:
     the model remembers the block it found last (its address and size,
     forgotten when a block's record goes or comes), so an access inside
     it needs no lookup in the ordered map of blocks, and a write of a
     plain value over one of its own type at the same offset, a loop's
     commonest write, leaves the block's record as it was rather than
     clearing the range and recording the value again. Two million reads
     and writes through a block of `Global`'s take 0.52 s in the debug
     profile, where they took 1.15 s on the same machine (2.3 s in the
     last round's measurement), and 0.010 s in the release profile;
     callgrind counts 2 100 instructions for each, where it counted 4 300.
     The lock, the call of the runtime and its descriptor stay; the
     hoisting of a loop's checks by the release facts (decision 270) and a
     pointer's provenance (tagged pointers, the sound form of `R0135` and
     `R0139`) are not done. This changes no rule of the language.

627. `std.time` has `Instant` and time zones (11). An `Instant` is a
     reading of a clock (`Instant.now(clock)`, `elapsed(clock)`,
     `since(earlier)`, `+` and `-` a `Duration`), ordered and shown as
     its nanoseconds; its readings are the clock's `now`, so the test clock
     makes them deterministic and a recorded run replays them, and on the
     host's clock they are the system's time, which a change of the
     system's clock moves: a monotonic reading would be an operation of
     `ClockOps` with its log and fakes. A `Zone` is a standard offset and
     abbreviation and, when it has daylight time, a second pair with the
     rules of its start and end; `Zone.named` gives the 52 zones of a
     table embedded in the module by their tz names, each as POSIX writes
     a `TZ` variable (`EET-2EEST,M3.5.0/3,M10.5.0/4`), as their rules
     stand since 2007, a time before a zone's rules came into force taking
     today's; `Zone.utc()` and `Zone.fixed(seconds)` make one offset.
     `offset_at(t)`, `abbreviation_at(t)`, `local(t)`, a `DateTime` whose
     parts are the zone's wall clock's, and `format(t)`, RFC 3339 with
     the offset (`2026-07-01T15:00:00+03:00`). The tz database itself,
     with each zone's history, is `std.x.tzdata`'s, which is not written.
     `std/time_zones` holds it, natively too. This changes no rule of the
     language.

**The driver: stage 1's options for the REPL and worksheets, and what an
input costs** (section "The REPL and worksheets"; 10; stage 1's round
that gives `sisu1 0.9.0`; decisions 620 and 621)

640. The driver gives a stage 1 of 0.9.0 and later (`Stage1::version`,
     asked once a process; `serves_repl_options`, or
     `SISU_STAGE1_REPL_OPTIONS=1` or `0` whatever the version says) where
     a program stands and what to call it: `sir --repl` gets `--dir`, the
     working directory, so that an input's `embed` and the inline
     metadata's paths resolve where the session runs and not in the
     directory the driver writes the inputs to, and, for `sisu repl FILE`,
     `--name FILE`, so that the diagnostics and the SIR call the program
     by the file; `sir --worksheet` gets `--dir`, the worksheet's own
     directory. `:why` reads stage 1's `parse --bindings` of an input
     (`repl::Outline`: each top-level node's kind among `let`, `assign`,
     `expr`, `item` and `other`, its range, the names it binds, its
     initialiser's range, the names that reads and the names it may
     change), where it read the whole tree and found these itself; for a
     stage 1 of 0.8.0, and under the Rust front end, the outline is read
     off the tree as decision 621 found them (`Outline::of`), so `:why`
     has one code, and a test holds the two readings to the same outline.
     A stage 1 older than 0.9.0 is given none of the options, as decisions
     620 and 621 run it. This changes no rule of the language.

641. A REPL input costs what decision 620 measured, 0.41 s natively of
     the driver's time (a session of 31 inputs 13.0 s, of 11 4.7 s, of
     one 0.77 s with the banner; under the Rust front end 31 inputs take
     2.4 s), and the stage 1 run is nearly all of it: `sir --repl` of an
     input takes 0.35 to 0.49 s whatever the session's size, the SIR it
     prints is 0.4 KB for one input and 2.7 KB for thirty, the
     interpreter runs it in under 10 ms, and `sisu1 --version`, asked
     once, 9 ms. Callgrind counts 5.06 billion instructions in one `sir`
     of a two-line script, 4.57 billion (90 %) of them in `base_of`, the
     standard library's base made again from its cache: the prelude and
     std parsed (1.95 billion, the lexer 1.15) and their items collected
     (4.10 billion, inclusive), of which the drops of the values the
     collection builds and throws away are 2.68 billion and their copies
     1.36 billion. Nothing the driver does between inputs costs more than
     a few milliseconds, so it keeps what it does: one run of stage 1 for
     each input, its `--version` asked once, an input's outline read once
     when `:why` first asks. What would take the cost away is stage 1's:
     to keep its base between inputs, a session served by one process of
     stage 1 that reads the inputs from its standard input (a `sir
     --repl-serve` that answers each with its SIR and the session it
     keeps, or its diagnostics), which leaves an input the check and the
     lowering of the input itself, a few milliseconds by the numbers
     above; failing that, a base cache that is read as the collected
     items rather than collected again. This changes no rule of the
     language.

**What the manual found: the bounds of the integers, `truncate_to`, a
monotonic `Instant` and the programs of `sisu test`** (sections "Numbers,
bytes and slices", "Capabilities", "Testing"; 03, 06, 11, 17; the
manual's findings from chapter 24, guide G9 and chapter 19 and R15)

642. Each integer type has its least and greatest value as constants of
     the type, `int.MIN` (03 names it), `int.MAX`, `u8.MAX`, `i128.MIN`
     and the rest, and each sized type has `truncate_to[T: Integer]()`,
     which `int` has of the front ends: the value reduced to `T`'s width
     as two's complement wrapping does (`200u8.truncate_to[i8]()` is
     `-56`, `u64.MAX.truncate_to[u32]()` is `u32.MAX`). They are Sisu, in
     the prelude's last block, so that no line of the functions before it
     moves; the types up to 32 bits reduce through `int`, a `u64`, `u128`
     or `i128` through its low 64 bits as an `int`'s (`int.from_bits`).
     `Integer` makes a value from an `int` alone, so a value outside
     `int`'s range taken to a type of 128 bits, where nothing is cut,
     gives its low 64 bits' value: widening (`into`, or implicitly) is the
     conversion to a wider type, and `to_bits` and `from_bits` between
     `i128` and `u128`. The exact form tells the width of `T` by
     `reflect.name_of(T)`, which stage 1 cannot do yet in a generic
     function of the prelude: it stops with an index out of bounds in
     `instances.named_types`, the template's body missing from the base it
     read from its cache (port-debt.md, for stage 1). Both front ends,
     the interpreter and native code give the same values;
     `std/int_bounds` and two doc examples hold it. This changes no rule
     of the language.

643. `ClockOps` has a third operation, `monotonic() -> Duration`: the time
     since an origin of the clock's own, which only moves forward. The
     host's clock reads the process's monotonic clock from the run's
     first reading (`Host::monotonic_nanos`, `Instant` in Rust), which a
     change of the system's time does not move, and a recorded run logs
     each reading (`monotonic_nanos -> N`) and replays it, as it does
     `now_nanos`; the test clock's reading is its time, so a test sees it
     move only by `advance` and the timers; a clock of the program's own
     gets the provided body, its `now` from the epoch, unless it gives
     one; a `cap.Wrap` shows it to its interceptor as `Clock.monotonic`;
     `Clock` and `Lent` forward it. The intrinsic is `clock.monotonic`,
     the interpreter's and the runtime in Rust's (`Intrinsic::
     ClockMonotonic`, the last number, and stage 2's table). `Instant.now`
     reads it, where decision 627 read `now`, so an instant on the host's
     clock is not moved by the system's time, as 11's `Instant` means;
     its text is still its nanoseconds. 06 lists a capability's
     operations as its trait's and leaves their number open, and 11 asks
     for `Instant`, so the operation needs no owner's word. A program's
     own `ClockOps` impl is not changed by it. `std/instant_monotonic`
     holds it, natively too. This changes no rule of the language.

644. `sisu test` names each program before its list when it runs more than
     one, the members of a workspace by their packages and a package's
     black-box test files (`tests/*.sisu`, decision 508) by their paths:
     `tests of `NAME`:` (`D0137`) on a line of its own, where the lists
     followed each other unnamed; a run of one program prints as before.
     `packages/workspace` shows it. This changes no rule of the language.

**The runtime in Sisu: the scheduler's queues, the iterators of lists
and sets, `in`, a slice's intrinsics and a table's literal** (section
"Stage 2", "The runtime in Sisu"; D-086; decision 625)

645. The scheduler's queues are the runtime in Sisu's
     (`rt/src/queue.sisu`, exports `sisu_rt_queue_*`): the run queue of
     ready tasks, the waiters registered under what wakes them (a task, a
     scope, a channel, the locks; the key a word, its kind above its
     number) and the timers (the clock, the time as a word, a time beyond
     a word's range never due, the task), each a queue of records of one
     to three words in the order they came, over `list.sisu`'s growth.
     The runtime in Rust's scheduler (`sched.rs`) keeps their addresses
     (`queue.rs`) where it kept a `Vec`, a `HashMap` of `Vec`s and a
     `Vec` of triples, and chooses among the ready tasks by its seeded
     generator as before; a record taken out keeps the others' order
     where the interpreter's scheduler keeps it (a waiter woken, a timer
     due) and the last moves into its slot where its run queue does so,
     so a seed names the schedule it named (the cases of `concurrency/`,
     `sim/` and `time/` give the same output natively under every
     scheduler). A key wakes its waiters in the order they waited, which
     the runtime in Sisu writes into a queue of the scheduler's own
     (`take_key`). The blocks are the runtime in Rust's, given to `init`,
     kept for the run and not counted among the program's allocations, as
     the collections they replace were not, so `SISU_RT_ALLOCS` says what
     it said. The parking and waking of a task (its state, the wait it
     reports in a deadlock) and the choice of the next one stay the
     runtime in Rust's, as do the channels' buffers, whose values have a
     size of their own: the next pieces of D-086's order. The scheduler's switches cost
     more: 20 000 values sent through a channel of capacity 1 and received
     take 196.7 million instructions (callgrind, the release profile, the
     runtime's object as the driver builds it) where they took 144.6
     million, 36 % more, about 2 600 instructions a value of its two
     switches: a call of Sisu's convention costs a hundred instructions
     and more where a `Vec`'s push cost ten, and each call lifts the
     running task's stack probe (`queue::unprobed`), since the scheduler
     calls from the root task's stack while another task's limits are
     loaded, which a run under valgrind, whose stacks lie elsewhere, showed
     failing. This changes no rule of the language.

646. The intrinsics the generated code calls for a list's and a set's
     iterator (`list.iter`, `set.iter`), a slice (`list.as_slice`,
     `Slice.to_list`, `at`, `get`, `is_empty`, `slice`, `cut`, `iter`),
     `in` as an operator (`ValueOp::In` of `sisu_rt_binary`), a table's
     literal (`sisu_rt_table`) and the inline lookups of `dict.get` and
     `contains` (`sisu_rt_table_find`) are done by the runtime in Sisu:
     the iterators and the slices' intrinsics by `dispatch.sisu`'s
     `sisu_rt_collection`, numbered beside a list's (a `Slice` is a list's
     layout of copies, as the interpreter's is; its index and its bounds
     panic through `sisu_rt_list_bounds`), `in` by `sisu_rt_contains`,
     which `list.contains` shares, and the literal and the lookups by
     `sisu_rt_table_literal` and `sisu_rt_table_inline_find`, which the
     runtime in Rust's entries of the generated code call, so that no
     lowering changes. Deleted from the runtime in Rust: the arms of
     those intrinsics, its `contains`, the table literal's loop, the
     lookup, `Table::new`, a value's size of a table, and the wrappers of
     `sisu_rt_list_copies`, `_list_contains`, `_table_new` and
     `_table_value_bytes`, whose exports are deleted from the runtime in
     Sisu. Still the runtime in Rust's: an iterator of a borrowed value
     reached by `iter` (`IterRef`, `IterMut`), the decoding of the other
     intrinsics, and `d{k}`'s slot (`sisu_rt_dict_slot`). A descriptor's word is read as half
     of the aligned word of eight bytes it is in (both backends align a
     descriptor to eight), where it was read a byte at a time, which
     every intrinsic of `sisu_rt_collection` pays. They cost more than the
     runtime in Rust's arms: 30 000 rounds of `in` on a list, a set's
     iterator, a dict's literal and a slice's `as_slice`, `slice` and
     `to_list` take 976.5 million instructions where they took 762.7
     million (28 % more), the descriptor read and the walk to a value
     behind its references for each argument most of it. The seed grows
     from 649 669 to 685 014 bytes of SIR with decision 645's queues and
     the runtime's object from 686 800 to 727 128 bytes
     (`target/rt/rt.o`), and as the fixed point builds it with the Sisu
     backend from 1 269 416 to 1 335 224 bytes; the fixed point holds
     (rt-A.o and rt-B.o the same bytes, as sisu2-A and sisu2-B,
     32 351 432, and stage1-A and stage1-B, 63 271 168, which carry the
     standard library's changes of decisions 642 and 643 and stage 2's
     table of decision 643's intrinsic too). This changes no rule of the
     language.

**Stage 1's eleventh round** (stage 1's RULES.md, R104 to R110; the findings' fixes and notes, coherence by a trait's type argument, imports that test the target, `arbitrary[T]` through its bound, a float's total order and the methods of 03, `E0235` at a derive, `reflect` in a std template)

660. Stage 1 gives the fixes and notes the manual found missing in its
     eleventh round: a local list's item for `E0338`, a lent capability
     for `E0421`, the parameter narrowed to the capability a test gives,
     the methods `E0806` points to, `E1010` without `E0220`, `E0237`
     without its `E0347`s, the float division for `E0301`, a binding a
     loop makes anew for `E0402`, the associated function for `E0258`,
     a variant reached through its type for `E0204`, `E1021` without the
     `E0703`s, and `+=` named by `E0310`. Stage 1 decides it (RULES.md,
     R104).

661. An impl of a foreign trait for a foreign type is the package's own
     when one of the trait's type arguments is a type of the package and
     no type parameter of the impl stands bare before it, `impl
     Mul[Vector] for f64` beside `Vector`; any other is `E0236`, whose
     note says which type would make it the package's. Stage 1 decides
     it (RULES.md, R105).

662. An import's `@cfg` may test the target, `@cfg(target.os ==
     "windows")`, and binds its names only where the condition holds; a
     use of one is not reported in an `if comptime` branch on the target
     the build leaves out, and is `E0200` naming the `@cfg` elsewhere.
     An import in a function's body is `E0277`, with the fix that moves
     it to the top of the module. 08 asks both branches of an `if
     comptime` to type-check; the names a target-gated import leaves
     out are the one exception. Stage 1 decides it (RULES.md, R106).

663. `arbitrary[T]` of a type parameter bounded by `Arbitrary` is the
     generator its bound gives; without the bound `E0820` names it.
     Stage 1 decides it (RULES.md, R107).

664. `F[_]` is `E0100` with a note that 03 reserves it; the standard
     library has checked arithmetic as methods, `round_to`,
     `f64.total_order()` and `max_by` and `min_by` over a partially
     ordered key, which `E0347` for a float names; `@derive(Plain)` asks
     for `@repr(c)` itself; `@repr(u8)` without discriminants is kept.
     Stage 1 decides it (RULES.md, R108).

665. `E0235` at a derive labels the impl written beside it. Stage 1
     decides it (RULES.md, R109).

666. A generic method of the standard library may ask `reflect` of its
     type parameters; `json.decode`'s `Mismatch` names the expected type
     without an article. Stage 1 decides it (RULES.md, R110).

**The runtime in Sisu: the channels' buffers and `d{k}`'s slot; the
executable's dead code, `@export`, and what the manual found** (sections
"Stage 2", "The runtime in Sisu", "Linking"; "The REPL and worksheets";
D-086; 06, 10, 12, 17; the manual's findings under "Tooling and the
driver", "Metaprogramming", "Ownership", "Testing", "Errors" and "The
standard library")

680. The channels' buffers are the runtime in Sisu's
     (`rt/src/channel.sisu`, exports `sisu_rt_channel_*`): the values a
     channel holds, sent and not received, in a ring of slots of the
     value's size rounded up to a word, whose header is five words (the
     slots' address, the first slot, the number of values, the slots, a
     slot's bytes), grown to twice its slots when a value finds it full,
     the values that wrapped round moved after the others. A value moves
     in and out as its bytes: `send` and `try_send` move it from the
     argument into the buffer and `recv`, `try_recv`, a receiver's
     iterator and `sisu_rt_recv_item` straight out to their result, where
     the runtime in Rust boxed each value in a `VecDeque`; a receiver
     dropped takes out what is left and drops it in the order it was
     sent, as before. The capacity stays a bound the scheduler checks, so
     a channel of a large capacity starts with eight slots. The blocks
     are the runtime in Rust's (`queue::ChanBuf`), kept for the run and
     not counted, as decision 645's queues are, so `SISU_RT_ALLOCS` says
     what it said, and the order of values is the interpreter's, so a
     seed names the schedule it named (the cases of `concurrency/`,
     `sim/` and `time/` give their output natively). Deleted: the
     `VecDeque` of boxed words and the size a receiver's iterator kept
     for its values. A task's record, its parking and waking and the
     choice of the next task stay the runtime in Rust's: they hold the
     coroutine, its context and the seeded generator's draws, the next
     piece of D-086's order. 20 000 values sent through a channel of
     capacity 1 take 173.9 million instructions where they took 164.3
     million (callgrind, the release profile, built from the same SIR by
     the Rust lowering), 5.8 % more, three calls of Sisu's convention a
     value where a push and a pop of a `VecDeque` were inline. This
     changes no rule of the language.

681. `d{k}`'s slot is found or made by the runtime in Sisu
     (`dispatch.slot`, export `sisu_rt_table_slot`): the value's address
     under a key the dict holds; for an assignment, an entry appended
     whose key is a copy made in its place (`table.push_copy`), where the
     runtime in Rust copied it into a vector and moved it in; 0 for a
     drop; -1 for a read, whose panic, naming the key in its `Debug`
     form, the runtime in Rust's entry `sisu_rt_dict_slot` raises, which
     the generated code still calls, so no lowering changes. Deleted:
     `Table`'s position, push, key size and value address, `copy_key`,
     and the exports `sisu_rt_table_lookup`, `_push`, `_key_bytes` and
     `_value_at`, which nothing calls now. 30 000 assignments and 30 000
     reads of `d{k}` take 127.8 million instructions where they took
     125.0 million (2.2 % more). The seed grows from 685 014 to 699 352
     bytes with 680. Still the runtime in Rust's: an iterator of a
     borrowed value reached by `iter` (`IterRef`, `IterMut`) and the
     decoding of the intrinsics other than the collections'. This
     changes no rule of the language.

682. A native executable leaves out what nothing reaches from `main`:
     the link passes `--gc-sections` (`-dead_strip` on macOS), and the
     runtime in Sisu's object, which the driver compiles with Cranelift,
     has a section for each function (`per_function_section`, in the
     library mode only), so that the runtime in Rust's functions a
     program does not call and the runtime in Sisu's are dropped. The
     program's own code was already only what `main` reaches: both
     lowerings compile instances from a queue that `main`'s instance
     starts (and an `@export`'s, 683), the dispatchers the `dyn` values
     made need, and the glue the type table names, and the front ends
     lower only what the entry, the tests and the root module reach, so
     a ten-line program's object is 10.6 KB, its `main` and the function
     it calls. That program's executable is 6 251 048 bytes where it was
     7 325 600 (the debug profile; 6 249 448 where 7 323 992 in the
     release profile; 21 083 192 where 22 464 232 when the driver is a
     debug build, whose runtime in Rust is unoptimised), 1.74 MB
     stripped. What is left is the runtime's: 0.45 MB of the runtime in
     Sisu's code, nearly all of which the runtime in Rust's bridge
     reaches, 0.50 MB of the runtime in Rust's and 0.37 MB of Rust's
     standard library; 3.8 MB of debug information, the runtime's and
     the standard library's, which a debugger and the line table of
     decision 227 need; 0.4 MB of symbols; and the C library and
     `libgcc_s`, linked dynamically (`libm` is linked only as needed).
     The manual's 17.5 MB was a debug build of the driver. Static
     linking and 12's `release-size` profile (stripped, the debug
     information of the runtime left out, the runtime's code per program
     rather than whole) are not this round's. The runtime in Sisu's
     object grows from 727 264 to 776 576 bytes with its sections; the
     fixed point's, which stage 2's backend writes as one section, from
     1 335 224 to 1 361 088 with 680 and 681. The fixed point holds
     (rt-A.o and rt-B.o the same bytes, as sisu2-A and sisu2-B, 31 093 256
     where they were 32 351 432, and stage1-A and stage1-B, 62 792 672
     where they were 63 271 168, both smaller for the sections left out).
     This changes no rule of the language.

683. `@export("name")` makes the function the global symbol `name` in a
     native build, in both lowerings (`Codegen::export_instance`,
     stage 2's `Cg.export_instance`, as a runtime library's exports are,
     decision 336), compiled before `main`'s instance so that `main`'s
     calls of it call that symbol, and a root of what is compiled
     whether or not `main` calls it; it was a local symbol named by the
     lowering (`sisu$heat$2`), or left out when nothing called it. The
     symbol has Sisu's convention, which is the C calling convention
     over addresses: C declares `uint8_t name(R *result, const P1 *a1,
     ...)`, each argument and the result in memory of their layout (an
     `i32` a word, as every sized integer), and reads the status, 0, or
     1 when a panic left the function, after which the caller must not
     call Sisu code again; a function that takes and gives plain C
     values, whose wrapper a C caller would not write, is the next step,
     with the `@repr(c)` check at the boundary that 12 asks for, which is
     stage 1's. The SIR names it in a line after the entry, `export
     "name" FUNCTION` (`Program::exports`), which the Rust front end
     writes, `sisu-sir` and stage 2 read (stage 2 beside stage 1's
     `sir.Program`, which has no field for it, `read.exports`, and writes
     back in place), and stage 1 does not write yet. The SIR path and the
     LIR path give the same executable. The driver's test
     `an_exported_function_is_a_global_symbol_that_c_calls` reads the
     symbol table of both profiles' executables and links a C caller
     that `main` calls through an `extern` block. This changes no rule
     of the language.

684. A `Zone`'s `Debug` form is the code that makes it:
     `Zone.named("Europe/Helsinki")`, `Zone.utc()`, `Zone.fixed(19800)`
     for a zone of one offset, where the derived form wrote its private
     fields, its rules among them, which a worksheet shows for a `let`.
     `std/zone_debug` holds it, natively too. The Rust front end, which
     gives a public type its derived form first, still writes the
     fields. This changes no rule of the language.

685. `sisu repl --echo` writes each input after its prompt, a line of
     continuation after its `...`, so that a session read from a pipe is
     captured as a terminal shows it. The default stays: a terminal
     echoes what is typed itself, and the suite's `.repl` captures, which
     read their inputs from a pipe, are what the default prints. The
     test `the_repl_echoes_its_inputs_when_asked` holds it. This changes
     no rule of the language.

686. Tagged pointers in the debug profile (port-debt.md) are not this
     round's either: the change is one to every inline use of a
     pointer's word in both lowerings and the C calls at once, and the
     round spent its time on 680 to 685. Measured for it: a pointer's
     word is read inline by `addr`, `with_addr`, `==`, order, hash and
     `Debug` of pointers and by every foreign call of a pointer type,
     each of which would strip the tag, and the runtime in Rust's
     `rawmem.rs` and `checked.rs` read the descriptors' pointer
     arguments; the C caller of 683 shows that a pointer handed out of
     Sisu code must be stripped too. This changes no rule of the
     language.

687. A program's transcripts stay on the Rust front end's build of it
     (decision 380): they run against the world of transcripts
     (`transcript_world`, `transcript_system`), prelude functions that
     only the program's tests or transcripts reach, which the Rust front
     end keeps in the SIR it lowers for them (`sir_with`) and stage 1's
     `sir --tests` does not, having no option to keep a function nothing
     reaches. Once stage 1's `sir --tests` keeps them for a program that
     has transcripts, the driver runs the transcripts from the SIR the
     tests run from. This changes no rule of the language.

688. A `CString`'s `as_ptr()` passed to C is refused by the interpreter
     because `bytes.as_slice()` makes a `Slice` of copies of the bytes
     (decision 646 keeps the interpreter's layout), a temporary that dies
     when `as_ptr` returns, so the pointer is to a value that no longer
     exists; natively `mem.slice.as_ptr` of a `u8` is refused at
     lowering. The fix is a slice that views its list's elements where
     they are, in the interpreter and in both lowerings, and is not this
     round's. This changes no rule of the language.

689. A run-time message keeps its form, `panic: ...`, for this round:
     its code, as `panic [R0154]: ...`, would change 81 of the suite's
     expectations, the runtime in Rust's and the interpreter's printing
     of every panic, stage 1's tests that read them, and 193 of the
     manual's sessions at once, a round of its own with the manual's.
     This changes no rule of the language.

690. A panic in `sisu repl FILE` is named by the session's program,
     whose lines are stage 1's: its `sir --repl` writes one source for
     the session, the loaded file's lines one below their own and a
     typed input's after them. The line of the file's own is stage 1's
     to give, with a source of its own for each input typed after the
     load, named `<input>` as its diagnostics are. This changes no rule
     of the language.

691. Under `--sim`, a task that fails with an error nobody joins in a
     scope whose body returns no result stops the test with `R0079`, as
     06 has it: a failure nobody joined becomes the scope's result, and
     a scope whose result cannot carry it panics. The runtime is right
     and the program of G14 is not: a test that looks at the state the
     tasks left joins them, or gives the scope's body a result type
     whose error set holds the tasks' errors. This changes no rule of
     the language.

692. A test that takes the real `System` still gets the simulated one
     under `--sim` (`System.sim`) and prints no token when it fails
     outside it, where 17 says such a test is neither simulated nor
     replayed and its token says so: the change is the test runner's
     choice of a `System` and a token that says why it cannot be
     replayed, a message of the catalog's, with the cases that show
     both, and is left for the next round. This changes no rule of the
     language.

693. `@no_alloc`, `@no_block` and `@stack(...)`, which compile without a
     word and check nothing, are to warn that they are accepted and
     checked by nothing yet, in the manner of decision 343; the warning
     is the front end's, so stage 1's. This changes no rule of the
     language.

694. A package chooses its runtime profile with `runtime` in
     `[package]`, one of 12's `"full"`, `"single"` and `"freestanding"`,
     `"full"` when it is left out; stage 0's scheduler, one thread with
     its tasks as coroutines, is `single`'s, so a native build of either
     of the first two is that runtime, and `"freestanding"` is refused
     until there is a runtime without the scheduler. Reading the key, and
     `target.runtime` naming the profile, are stage 1's manifest and
     compile-time facts. It names the key 12 leaves unnamed.

**Stage 1's twelfth round** (stage 1's RULES.md, R111 to R119; a prelude type's constants' names, `sir --repl-serve`, lookups by `str`, `E0343` for a `Shared` field, `@preview`, an exact `truncate_to`, `--target-runtime`, a field's `##` in reflection, the `@export` boundary, a module beside a file)

700. A prelude type's associated constant is named in the SIR by the
     type, `u8.MAX`, as its methods are. Stage 1 decides it (RULES.md,
     R111).

701. `sisu1 sir --repl-serve` serves `sir --repl`, `sir --worksheet` and
     `parse --bindings` from one process that makes the standard
     library's base once: each line of standard input is a request, the
     command's arguments separated by tabs, and each answer, on standard
     error, is `answer STATUS OUT ERR` and then the bytes the command alone
     prints on standard output and on standard error; an empty line or the
     end of the input ends it. An input costs 0.04 s served where it cost
     0.8 s as a process. `sisu1 --version` is 0.10.0. Stage 1 decides it
     (RULES.md, R112).

702. A lookup in a `dict` or a `set` (`get`, `contains`, `remove`, an
     index) takes a value of a type that implements the prelude's
     `Lookup[K]`, a `str` for `String` keys, and borrows the key it gives;
     `E0300` for an optional reference compared with an optional value has
     the fix `.copied()`. Stage 1 decides it (RULES.md, R113).

703. A change of what a `Shared` holds is `E0343`, which says so whether
     the binding is a `var` or not. Stage 1 decides it (RULES.md, R114).

704. A use of an item marked `@preview` warns (`W0028`, `W0029`), as a
     use of a deprecated one does; `truncate_to` of a value outside
     `int`'s range to a type of 128 bits is exact. Stage 1 decides it
     (RULES.md, R115).

705. Stage 1 takes `--target-runtime native|interpreter`, which
     `target.runtime` gives compile-time code; the driver passes `native`
     for a native build. Stage 1 decides it (RULES.md, R116).

706. A field has `doc`, its `##` comment, to compile-time code, and
     `cli.help_text` prints it under each option. Stage 1 decides it
     (RULES.md, R117).

707. An exported function's signature holds numbers, `bool`, `char`, raw
     pointers and `@repr(c)` types; any other is `E0991`. Stage 1 decides
     it (RULES.md, R118).

708. `E0220` for a file's import of a module beside it notes that only a
     package makes them one program; a directory without a manifest is no
     package. Stage 1 decides it (RULES.md, R119).

**Stage 0's twelfth round: the driver on stage 1's 0.10.0 and 0.11.0, the
scheduler's generator in Sisu, the runtime profile's line, and what was
left** (sections "The REPL and worksheets", "Testing", "SIR", "Stage 2",
"The runtime in Sisu"; D-086; 06, 10, 12, 17; stage 1's R112, R113, R116,
R120 and R122)

720. `sisu repl` and `sisu ws` keep one process of stage 1 for the
     session, `sir --repl-serve` (stage 1's R112), when `sisu1 --version`
     is 0.10.0 or later (`Stage1::serves_repl_server`, or
     `SISU_STAGE1_REPL_SERVE=1` or `0` whatever the version says): it is
     started at the session's first command (`front::Server`), each
     `sir --repl`, `sir --worksheet` and `parse --bindings` is a line of
     its arguments separated by tabs, and the answer, `answer STATUS OUT
     ERR` and the bytes on its standard error, is read as a run of its
     own is (`settle`), so nothing after it changes; an empty line ends
     it when the session ends. A server that goes away or answers out of
     its frame is given up for the session, and that command and the
     ones after it run as a process each, which reports what went wrong;
     a stage 1 older than 0.10.0, one without 0.9.0's options, and
     `--front-end=rust` keep decision 640's path. The suite's session
     (`live/session.sisu` loaded, `live/session.stdin`'s fourteen lines,
     two of them `:why`) takes 0.91 to 1.08 s served where it takes 6.38
     to 6.75 s with a process for each command, measured one after the
     other on the same machine at a load of 3 to 4 on four cores, about
     0.06 s an input where it was 0.43 s (decision 641 measured 0.41 s);
     the outputs are byte for byte the same, which
     `repl_and_worksheets_run_on_stage_1` holds both ways for the suite's
     sessions and worksheet. A worksheet is one command either way. This
     changes no rule of the language.

721. A native build is built for the native runtime: the driver passes
     stage 1 `--target-runtime native` (its R116) for `sisu run --native`,
     `sisu build --native`, `sisu test --native` and `sisu bench
     --native`, when stage 1 is 0.10.0 or later
     (`Stage1::takes_target_runtime`), so `target.runtime` is `native` to
     their compile-time code, where it was `interpreter`; `sisu run`,
     `sisu sir` and `sisu check` pass nothing, and stage 1's default stays
     `interpreter`. A program built from `sisu sir`'s SIR was compiled for
     the interpreter, so the native runner compares the standard output
     of a case built from its source with `NAME.native-out` where the
     case has one, and `NAME.out` otherwise and for the modes built from
     SIR: `reflect/target.native-out` holds `native`. This changes no rule
     of the language.

722. A program's transcripts run from the SIR its tests run from when
     stage 1 keeps the world of transcripts there (`sir --tests` of
     `sisu1 0.11.0` and later, its R120; `Stage1::keeps_transcripts`, or
     `SISU_STAGE1_TRANSCRIPTS=1` or `0`), so a transcript runs the program
     stage 1 built, and a form only stage 1 has no longer fails it: a
     program that looks a `dict[String, V]` up by a `str` (R113), which
     the Rust front end refuses, passes its transcript. An older stage 1,
     an SIR without `transcript_world` and `transcript_system`, and
     `--front-end=rust` keep the Rust front end's build of decision 687.
     `SISU_STAGE1=PATH` runs the stage 1 at `PATH` as it is, neither built
     nor checked against its sources, as `SISU_STAGE2` gives stage 2; the
     round checked the change against stage 1's thirteenth round's
     `sisu1 0.11.0` so. The test
     `transcripts_run_from_stage_1s_sir_when_it_keeps_their_world` holds
     both readings, by the version of the stage 1 it finds. This changes
     no rule of the language.

723. The SIR's line `runtime NAME`, after the entry and before the
     `export` lines, which stage 1 writes for a package whose manifest
     chooses `single` (its R122; `full`, the default, writes none), is
     read: `sisu-sir` reads and writes it (`Program::runtime`, `full` or
     `single`, any other name refused), so the interpreter and the Rust
     lowering read it, and stage 2 reads it beside stage 1's
     `sir.Program` (`read.runtime`) and writes it back with the `export`
     lines (`read.export_lines`), as decision 683 reads those. Both
     profiles run alike: stage 0's scheduler is one thread with its tasks
     as coroutines, `single`'s (decision 694). What `single` would change
     is `std.par.map`'s pool of worker threads, which a program of that
     profile should not have; telling the executable which profile it was
     built for is a symbol of the program's, in both lowerings and
     byte-identical, and is not this round's. The Rust front end, which
     reads no `runtime` key, writes no line. Stage 1's
     `packages/runtime_single.pkg` reads and runs in the interpreter,
     natively and through `sisu2 sir`'s round trip, which gives the same
     text. This changes no rule of the language.

724. The scheduler's seeded generator is the runtime in Sisu's
     (`rt/src/draw.sisu`, exports `sisu_rt_draw_*`): the interpreter's
     SplitMix64 with its seeding (`seeded`, `next`), a number below a
     bound by the unsigned remainder (`below`), and, under the default
     strategy, the choice of the next task among the ready ones, taken
     out of the run queue as decision 645's run queue takes it (`pick`),
     so a seed names the schedule it named (`sisu-driver/tests/schedules`
     and the concurrency cases give the interpreter's output natively).
     The generator's state is a word the runtime in Rust keeps
     (`queue::Rng`), for the scheduler, `select`'s ready arms, PCT's
     priorities and change points and the faults of `sim`. Deleted from
     the runtime in Rust: the generator's arithmetic and seeding, its
     `below`, and the default strategy's draw and removal in `pick`. A
     task's record, its parking and waking, PCT's and the exhaustive
     search's choices stay the runtime in Rust's: the records hold the
     coroutine and its context. 20 000 values sent through a channel of
     capacity 1 take 176.1 million instructions where they took 173.9
     million (callgrind, the release profile), 1.3 % more, a call of
     Sisu's convention for each draw. The seed grows from 699 352 to
     703 530 bytes of SIR and the runtime's object to 785 920 bytes
     (`target/rt/rt.o`), and as the fixed point builds it with the Sisu
     backend from 1 361 088 to 1 372 016; the fixed point holds (rt-A.o
     and rt-B.o the same bytes, as sisu2-A and sisu2-B, 31 167 928, and
     stage1-A and stage1-B, 63 105 968, which carry stage 1's twelfth
     round too). This changes no rule of the language.

725. The rest of D-086's order for the scheduler is not this round's: a
     task's record (its state, the wait a deadlock reports, its scope,
     outcome and stack limits) holds the coroutine, its context and the
     values moved aside, which are Rust objects that Sisu code cannot
     hold, so the record in Sisu comes with the switch's own record of
     the coroutine (a handle the runtime in Rust keeps beside it); then
     `IterRef` and `IterMut`, whose iterators borrow a value the runtime
     in Rust reaches through `iter`; then the decoding of the other
     intrinsics, family by family. The round spent its time on 720 to
     724. This changes no rule of the language.

726. Tagged-pointer provenance in the debug profile is not this round's
     either: it is decision 686's change to every inline use of a
     pointer's word in both lowerings and the Cranelift path at once, a
     week by port-debt.md's measure, which leaves no half state only
     when it is done whole; the cases of chapter 27's finding still pass
     natively where the interpreter stops them. This changes no rule of
     the language.

727. `@export` with C's own convention over plain values and a
     `CString.as_ptr()` that stays valid across a foreign call are not
     this round's: the first is a wrapper of each exported function in
     both lowerings, byte-identical (its arguments and result in
     registers as C passes them, the status turned into an abort), for
     the types stage 1's `E0991` admits (its R118); the second is a slice
     that views its list in place (decision 688), in the interpreter and
     both lowerings, with `mem.slice.as_ptr` of a `u8` compiled
     natively. Decision 683's convention, the C calling convention over
     addresses, stays what a C caller writes to. This changes no rule of
     the language.

728. A test that takes the real `System` still gets the simulated one
     under `--sim`, as decision 692 left it: changing it takes the cases
     of `sim/` that declare a `System` to get a simulated one
     (`sim/faults`, `sim/no_revoked`) to name their capabilities instead,
     whose fakes `--sim` makes, and a message that says why the token
     cannot replay such a test; it is the next round's with those
     cases. This changes no rule of the language.

729. A lookup by a `str` in a `dict[String, V]` or a `set[String]` still
     allocates the `String` that `str.Lookup.lookup_key` makes (stage 1's
     R113): the SIR calls it and passes the `String` to `dict.get`,
     `dict.remove` and the index, so the runtime is given
     a key it cannot tell from any other. Looking the text up in place is
     the front end's lowering first: the SIR passes the `str` to the
     lookup, which the interpreter and the runtime in Sisu then compare
     with the `String` keys by their bytes, as `in` and `dict.contains`
     already do. This changes no rule of the language.

**Stage 1's thirteenth round** (stage 1's RULES.md, R120 to R126; the SIR's `export` and `runtime` lines, transcripts kept in `sir --tests`, `W0030` for unchecked attributes, the manifest's `runtime` key, the REPL's `<input>` sources, over-rejection 15 and the bootstrap's report, the base cache's second form, `E0258` alone)

740. Stage 1's `sir` writes `export "SYMBOL" FUNCTION` after the entry
     for each `@export`ed function and lowers it as a root; `sir --tests`
     of a program with transcripts keeps `transcript_world` and
     `transcript_system`, so that the driver runs its transcripts from that
     SIR. Stage 1 decides it (RULES.md, R120).

741. `@no_alloc`, `@no_block` and `@stack(...)` on a function warn
     (`W0030`) that they are accepted and checked by nothing yet. Stage 1
     decides it (RULES.md, R121).

742. `[package] runtime` is `full`, `single` or `freestanding`; the last
     is `E1023` until there is a runtime without the scheduler; `single`
     is written to the SIR as `runtime single` after the entry, and
     `target.runtime_profile` names the profile. `sisu1 --version` is
     0.11.0. Stage 1 decides it (RULES.md, R122).

743. `sir --repl` names a loaded file's functions by the file at its own
     lines and a typed input's by `<input>` at the input's, `main` by the
     last input. Stage 1 decides it (RULES.md, R123).

744. A read of a `mut` reference through a shared one is lowered as a
     shared reborrow, `ref _1.*`, so a loop over a `mut` parameter may
     read it (known over-rejection 15 is gone from stage 1); the
     bootstrap's comparison of stage 1's SIR with stage 0's is a report,
     the fixed point `B.sir` equal to `A.sir`. Stage 1 decides it
     (RULES.md, R124).

745. The base cache keeps which importable std modules hold templates and
     `unsafe` code, and a base made from it finds a std text's lines when
     lowering first reaches it: a command's start is 10 % shorter; the
     parse of the base modules, half of it, stays. Stage 1 decides it
     (RULES.md, R125).

746. A struct of another module built without a private field is
     `E0258` alone; `TotalF64` stays no key, since the runtime hashes keys
     structurally. Stage 1 decides it (RULES.md, R126).

**Stage 0's thirteenth round: the tasks' records in Sisu, the profile
`single`, stage 2's runtime line, lookups by `str`, the C boundary, and
what was left** (sections "Stage 2", "The runtime in Sisu", "Tasks",
"Raw memory, foreign calls and the machine intrinsics", "Lowering-only
intrinsics"; D-086; 06, 12, 17; stage 1's R113, R118, R120 and R122)

780. A task's record, its parking and its waking are the runtime in
     Sisu's (`rt/src/tasks.sisu`, exports `sisu_rt_task_*`): the records
     are a queue of three words a record (decision 645's `queue.sisu`),
     the record `t` the task `t`'s, holding its state (ready, running,
     waiting, done), what it waits for (a word of the runtime in Rust's,
     the kind of the wait above its subject, which the deadlock report
     reads back) and its flags (a generator); a task parks under the keys
     of what wakes it, registered in the waiters' queue in order, and a
     key wakes the tasks waiting under it in the order they parked, a
     cancellation every waiting task but generators in the order of
     their numbers, each put at the end of the run queue as the
     interpreter's `wake` puts it, so that a seed names the schedule it
     named (`sisu-driver/tests/schedules` and the cases of
     `concurrency/`, `sim/` and `time/` give the interpreter's output
     natively). What the record does not hold, the coroutine, its
     context, the values moved aside, the outcome, the anchors and the
     trace's path, stays beside it in the runtime in Rust's `Task`, under
     the same number: the handle 725 asked for is the number itself.
     Deleted from the runtime in Rust: `Task`'s state and generator flag,
     the scheduler's `wake` and `wake_key` loops, `Queue::take_key` and
     the exports `sisu_rt_queue_clear` and `_take_key`, which only it
     called. 20 000 values sent through a channel of capacity 1 take
     194.4 million instructions where they took 176.1 million
     (callgrind, the release profile, built from the same SIR by the Rust
     lowering), 10.4 % more: each value parks a task and wakes one, and a
     state read or set from the runtime in Rust is a call of Sisu's
     convention where it was a field; a first version, whose functions
     called `queue.sisu`'s for each word, took 271.3 million, so each
     function reads the records' block in place. The seed grows from
     703 530 to 720 405 bytes with 780 and 784, the runtime's object from
     785 920 to 809 912 bytes (`target/rt/rt.o`), and as the fixed point
     builds it with the Sisu backend from 1 372 016 to 1 404 672; the
     fixed point holds (rt-A.o and rt-B.o the same bytes, as sisu2-A and
     sisu2-B, 31 374 768, and stage1-A and stage1-B, 63 522 752, which
     carry stage 1's thirteenth round too). This changes no rule of the
     language.

781. The rest of D-086's order for the scheduler is not this round's: an
     iterator of a borrowed value reached by `iter` (`IterRef`,
     `IterMut`), whose object borrows a value the runtime in Rust reaches
     through a `Pointer` of its own, and the decoding of the intrinsics
     other than the collections', family by family, each with its
     descriptor read by Sisu code as decision 625 reads a collection's;
     PCT's and the exhaustive search's choices stay the runtime in
     Rust's. The round spent its time on 780 and 782 to 785. This changes
     no rule of the language.

782. Stage 2 reads the SIR's `runtime NAME` line into stage 1's
     `sir.Program.runtime` (`read.read`, where it set `None`), and stage
     1's writer writes it back, so `sisu2 sir` gives back the text it read
     with the line where it was; `read.runtime`, which read it beside the
     program, is deleted, and `read.export_lines`, which still writes the
     `export` lines that `read.exports` reads beside it, takes no profile
     and puts them after the runtime line. Stage 2's lowering reads the
     profile from the program (783). The fixed point and the round trip
     of `packages/runtime_single` hold byte for byte. This changes no rule
     of the language.

783. A program of the runtime profile `single` (12; decisions 694 and
     723) starts no worker thread: both lowerings set `ENTRY_SINGLE` (8)
     in the flags `main` gives `sisu_rt_start` when the SIR says `runtime
     single`, byte-identically, and the runtime's entry records it, after
     which every call of `std.par.map` runs its bodies on the asking
     thread, one after another, as a call on a worker does and as the
     interpreter runs them; its scheduler was one thread already. The
     interpreter, which has no workers, runs both profiles alike, and
     `target.runtime_profile` names `single` to compile-time code (stage
     1's R122). The case `packages/runtime_single` counts the threads of
     the process (Linux's `/proc/self/status`) before and after a call of
     `par.map` and prints 0 for the threads it started, interpreted and
     natively, where the same program of the profile `full` starts one a
     processor natively (4 on the round's machine). This changes no rule
     of the language.

784. The lookups by a `str` of a `dict[String, V]`'s or a `set[String]`'s
     keys are intrinsics of their own, for stage 1 to lower
     `str.Lookup`'s calls to (decision 729): `dict.get_str`,
     `dict.remove_str`, `dict.index_str` (a reference to the value; a
     missing key panics, `R0010`, naming it), `set.contains_str` and
     `set.remove_str`, each taking the dict or set and the `str` itself,
     whose bytes are hashed and compared with the keys' as a `String`'s
     are and as `in` compares them, so no `String` is made. The interpreter looks
     the text up as it looks up a `String`, whose key is the same; the
     runtime in Sisu decodes them with the collections' (`dispatch.sisu`,
     numbers 323 to 327 of `abi::Intrinsic`, after the others so that
     theirs stay what they were), each the lookup of its `String` form
     but for the key given, and `dict.index_str`'s panic is the runtime in
     Rust's (`sisu_rt_missing_key`, which `d{k}`'s slot now calls too);
     stage 2 names them in its table. No front end lowers to them yet, so
     the SIR of `sisu-driver/tests/lookups` calls them, and the test
     `lookups_by_a_str_run_interpreted_and_natively` holds the
     interpreter's output, that of both lowerings and both profiles, and
     no allocation left. This changes no rule of the language.

785. `@export("name")` gives a program's function the global symbol
     `name` of C's own convention over plain values (12, "Interop"),
     replacing decision 683's convention over addresses: a wrapper of the
     function, declared `export` with the C signature of its parameters
     and result (the integers and floats at their width, a `bool` a
     byte, a `char` a 32-bit code, a raw pointer an address, `()` no
     result), writes each argument widened to its word, calls the
     function, compiled as any other under a local symbol that `main`'s
     calls call, and gives back its result narrowed as C reads it; a panic
     that leaves the function calls `sisu_rt_export_panic`, which writes
     the panic's report and stops the program with status 134, since no C
     caller can unwind. Both lowerings write the wrapper alike
     (`Codegen::export_c`, stage 2's `Cg.export_c`), so the two give the
     same object, which the driver's test
     `an_exported_function_is_a_global_symbol_that_c_calls` checks with a
     C caller that calls three exported functions the C way (an `i32`; an
     `f64`, a `bool`, a `char`, a `u8` and an `f32` to an `f64`; one that
     panics), stage 1's SIR naming them (its R120). A `@repr(c)` struct,
     which `E0991` admits, is passed by a pointer to it; by value its C
     classification is not done, and it is `D0103`. A runtime library's
     exports stay of Sisu's convention. `CString.as_ptr()` valid across
     a foreign call, the other half of decision 727, is not this round's:
     it is a slice that views its list in place in the interpreter's
     checked memory model and both lowerings, with `mem.slice.as_ptr` of
     a `u8` compiled natively. This changes no rule of the language.

786. A test that takes the real `System` still gets the simulated one
     under `--sim`, as decisions 692 and 728 left it: the change is the
     test runner's choice of a `System`, a catalog message for the token
     of a test that can be neither simulated nor replayed, and
     `sim/faults` and `sim/no_revoked` rewritten to name the capabilities
     whose fakes `--sim` makes, which the round did not reach after 780
     to 785. This changes no rule of the language.

787. Tagged-pointer provenance in the debug profile is not this round's
     either (decisions 686 and 726): it is a change to every inline use
     of a pointer's word in both lowerings and the Cranelift path at once,
     to which 785's wrapper adds that a pointer handed to C must be
     stripped, and it leaves no half state only when done whole. This
     changes no rule of the language.

**Stage 1's fourteenth round** (stage 1's RULES.md, R127 to R133; the lexer and the AST builder by their bytes, imports and dependencies under the target runtime, `reflect.doc_of` and a variant's fields' `doc`, checks before `return impl`, the template line's label, the transcripts class, Unicode names decided against)

760. Stage 1's lexer dispatches on a punctuation's and a keyword's bytes
     and reads each byte once, and its AST builder walks a tree without
     errors once and finds an item among its siblings by a binary search:
     a command's start is 17.5 % shorter (4.15 G instructions to 3.42 G
     for a two-line `check`, the lexer 1.03 G to 0.53 G), with the same
     tokens and trees. Stage 1 decides it (RULES.md, R127).

761. An import's `@cfg(target.runtime == ...)` and a dependency's `when =
     { runtime = ... }` read the runtime that `--target-runtime` names, so
     a native build reaches a dependency for the native runtime. Stage 1
     decides it (RULES.md, R128).

762. `reflect.doc_of(T)` is the `##` comment of `T`'s declaration, a
     variant's field on a line of its own has `f.doc`, and `cli.about[T]()`
     gives a struct's own comment. Stage 1 decides it (RULES.md, R129).

763. A derive's template may begin with compile-time checks of the type,
     `comptime assert`, `comptime panic` and `comptime if` of them, before
     its `return impl`; a failing check is `E0608` at the `@derive`, and
     `E0607` names what may come first. Stage 1 decides it (RULES.md,
     R130).

764. An error of a template's instance placed at its call labels the
     statement of the program's template that wrote the instance's line.
     Stage 1 decides it (RULES.md, R131).

765. Stage 1's `check.sh` compares the SIR of each program with
     transcripts, built with its tests, with stage 0's and runs its
     transcripts through the driver from stage 1's SIR, against the
     case's `.test`. Stage 1 decides it (RULES.md, R132).

766. NFC names and the confusable and mixed-script lints of D-048 wait
     for generated Unicode tables (1,251 compositions; the confusables of
     the identifier scripts and their script ranges): not this round.
     Stage 1 decides it (RULES.md, R133).

**Stage 0's fourteenth round: the iterators' intrinsics and the
scheduler's other strategies in Sisu, the real `System` under `--sim`, a
`CString` across a foreign call, a package's replay, and what was left**
(sections "The runtime in Sisu", "Testing", "Simulation, schedulers and
replay", "The checked memory model"; D-085, D-086; 06, 12, 17; decisions
688, 692, 726, 728, 781, 785 to 787)

820. The thirteen intrinsics that make an iterator or take every item of
     one are the runtime in Sisu's (`rt/src/dispatch.sisu`, decision
     781): `range`, `enumerate`, `zip`, `zip_shortest`, `reversed`,
     `iterable_list`, `sorted`, `any`, `all`, `iter.ref`, `iter.mut`,
     `iter.own` and `Iter.of`, their descriptors read by Sisu code as
     decision 625 reads a collection's, the runtime in Rust reading the
     number alone and handing the call over. An iterator of a borrowed
     value (`iter.ref`, `iter.mut`) is made where the value is: a list's
     elements by their addresses, a dict's pairs and a set's keys by
     references into the table, a receiver's values from its channel, as
     the runtime in Rust made them, and each of the others takes its
     argument's items, moves them and drops what it leaves in the
     interpreter's order (`sorted` of two unordered items panics,
     `R0081`, and drops what it took; `range` of step 0 panics through
     `sisu_rt_range_step`). `export_collection` takes the mode of a drop
     (decision 423) and reads the descriptor's length once. Deleted from
     the runtime in Rust: the thirteen arms of `intrinsics.rs`, the
     constructors `iter::range`, `list_ref`, `table_ref`, `enumerate`,
     `zip`, `receiver` and `drop_iter`, `bridge::list_reverse`,
     `list_sort` and `iter_drop`, and the exports that only they called
     (`sisu_rt_iter_range`, `_list_ref`, `_enumerate`, `_zip`,
     `_table_ref`, `_receiver`, `_drop`, `sisu_rt_list_reverse`,
     `_list_sort`). 50 000 calls of a function that loops over a borrowed
     list, dict and set (150 000 calls of `iter.ref`) take 814.7 million
     instructions where they took 671.6 million (callgrind, the release
     profile, built from the same SIR by the Rust lowering), 21.3 % more,
     about 950 instructions a call, half reading the descriptor's words
     and half following the argument's references; a first version, which
     read the descriptor's length at each word, took 918.1 million. 20 000
     values through a channel of capacity 1 take 191.8 million, where
     they took 194.4 million. This changes no rule of the language.

821. PCT's and the exhaustive search's choices are the runtime in Sisu's
     (`rt/src/choose.sisu`, exports `sisu_rt_choose_*`; decisions 501,
     724 and 781): PCT's change points drawn when the run starts, its
     priorities by task drawn when first asked, a yield point at which a
     change point drops the running task below every other and a task
     that gives way drops below every other, and the next task, the first
     ready one of the highest priority; the search's next choice, its
     prefix's or 0, recorded in its trail with whether it is at a yield
     point, a yield point's preemption for the ready task it chose, and
     the next task among the ready ones in creation order. Each draws
     from the generator of decision 724 in the interpreter's order over
     queues of `queue.sisu` that the runtime in Rust keeps
     (`queue::Choices`), so a depth and a seed, or a prefix, name the
     schedule they named (`sisu-driver/tests/schedules`, `sim/pct`,
     `sim/lost_update` and the cases of `--explore`). Deleted from the
     runtime in Rust: `Sched`'s priorities, change points, low mark,
     yield count and trail, `priority`, `ready_sorted`, `ready_list`, the
     two strategies' arms of `pick` and of a yield point, and
     `Queue::swap_remove` with its export. 20 000 values through a channel
     of capacity 1 take 267.6 million instructions under `pct:3` where they
     took 240.9 million, 11.1 % more, a call of Sisu's convention for each
     priority PCT compares at a yield point where it was a field, and
     325.3 million under the search where they took 327.2 million. With 820 the Rust of the runtime is 15 367 lines
     (575 416 bytes) where it was 15 506 (581 489): the wrappers of
     `queue::Choices` are most of what it gained; the runtime in Sisu is
     7 290 lines where it was 6 930, its seed 755 222 bytes where it was
     720 405, and its object 851 480 (`target/rt/rt.o`) where it was
     809 912, and as the fixed point builds it with the Sisu backend
     1 478 176 where it was 1 404 672; the fixed point holds (rt-A.o and
     rt-B.o the same bytes, as sisu2-A and sisu2-B, 31 065 632, and
     stage1-A and stage1-B, 63 418 696). What the runtime in Rust still holds, by size: the
     decoding of 265 of the 328 intrinsics (`intrinsics.rs`, `tasks.rs`,
     `host.rs`, `bytes.rs`, `cells.rs`, `caps.rs`, `rawmem.rs`, 5 047
     lines), the scheduler's tasks, switch and clocks (`sched.rs`,
     1 609, and `stack.rs`), the checked memory model (`checked.rs`,
     1 181), the type table's encoding and layout (`abi.rs`, `desc.rs`,
     1 853), the bridge to the runtime in Sisu (`bridge.rs`, 1 144), the
     entry (`entry.rs`, 753) and the rest, panics, wide and sized
     integers, `std.par`. This changes no rule of the language.

822. A test that takes the real `System` gets the real one under `sisu
     test --sim` too, which neither simulates nor replays it (17,
     "Simulation and replay"; decisions 692, 728 and 786): it runs alone,
     after the others, as outside `--sim`, no fault reaches it, since
     faults are the simulation's objects' (decision 502), and its failure
     prints `D0138` (`cli.replay-no-token-sim`) where a token would be,
     which says why and that a test that names the capabilities it uses
     gets simulated ones. The interpreter's runner, the native runner's
     dispatcher and the driver's report choose so; `System.sim` is made
     for a test that names capabilities. `sim/faults` and `sim/no_revoked`
     name the file system they use (`fs: Fs`), and `sim/real_system`
     shows both, in the interpreter and natively. This changes no rule of
     the language.

823. A `CString`'s `as_ptr()` stays valid across a foreign call in the
     interpreter (decisions 688 and 785): a slice that `list.as_slice` or
     `bytes.as_slice` makes is recorded with the place it is stored in,
     and `mem.slice.as_ptr` of it points to the list or the `bytes` it
     was made from, where they are, under no loan of the slice's, while
     they hold as many values; a byte of a `bytes` read through it is the
     `bytes`'s own, so the pointer is valid while the `CString` lives and
     goes with it. The slice's values stay copies (decision 646), so
     `Slice.at`'s references are the copy's. Natively `mem.slice.as_ptr`
     of a `u8` stays refused: a native `Slice[u8]` and `list[u8]` hold a
     word for each byte (the layout of a place, port-debt.md), where a
     `bytes` holds bytes, so no slice can view a `bytes` in place until
     the layouts are per type (D-086); `unsafe/cstring`, which passes a
     `CString` to `strlen` and reads it back through a `CStr`, runs in
     the interpreter and under the monitor and is not in `native.list`.
     This changes no rule of the language.

824. A `@repr(c)` struct by value at the export boundary stays `D0103`
     (decision 785): 12 asks for "the C ABI", which passes a small struct
     in registers by the System V classification of its eightbytes
     (`INTEGER` or `SSE`, or memory above sixteen bytes) and AArch64's by
     its own, in the wrapper of both lowerings byte-identically and in
     the foreign calls that would take one the same way; the round spent
     its time on 820 to 823, and a pointer to the struct is what a C
     caller writes to meanwhile. This changes no rule of the language.

825. Tagged-pointer provenance in the debug profile is not this round's
     either (decisions 686, 726 and 787): done whole or not at all, it is
     the round of its own that port-debt.md now describes, the runtime's
     `checked.rs` first, then each inline use of a pointer's word in both
     lowerings the same day behind a switch that stays off until
     `--native-from-lir` gives the same bytes. This changes no rule of the
     language.

826. The trace context of 06 ("Interceptors and trace context": a scope's
     trace and span identifiers, inherited by its children and attached by
     `log`, and `trace.wrap` with a `Tracer`) is not this round's: it
     changes the text of every `log` record, the interpreter's and the
     native runtime's scopes and std's `cap`, and the identifiers' form is
     not settled by 06; the path of spawn indices stays the context (the
     manual's finding G28). This changes no rule of the language.

827. The conformance runner replays a package case's failing test with
     the file its `replay:` line names (`.` for a module's test,
     `tests/NAME.sisu` for a black-box one), in the interpreter and
     natively, where it named none and the replay of a black-box test ran
     the package's own tests (the manual's finding R15);
     `packages/failing_test` has a black-box test that fails and replays.
     This changes no rule of the language.

828. `sisu test FILE` of a module of the package whose directory it stands
     in (a file under its `src/`) is to run that package's tests of the
     module, its imports resolved as the package's, rather than the file
     alone (the manual's finding from chapter 17); a black-box file under
     `tests/` already compiles against its package. It is not this
     round's: it needs stage 1's `sir --tests` to name each test's module
     so that the driver selects them, which a round of stage 1's gives.
     This changes no rule of the language.

**Stage 1's fifteenth round** (stage 1's RULES.md, R134 to R141; lookups by `str` to the intrinsics, the parser's cost, Unicode names with generated tables and the lints `W0031` and `W0032`, the library's planned helpers, names despite a parse error, `sisu doc` values decided against, every type's `Debug`, the `runtime_single` package's differences)

800. Stage 1 lowers `get`, `remove` and the read of an index of a
     `dict[String, V]`, and `contains` and `remove` of a `set[String]`,
     given a `str`, to decision 784's intrinsics that take the `str`, so
     that no `String` is made for the key; a missing key's panic stands at
     the index. On the interpreter a loop of 30,000 lookups takes 30 % fewer
     instructions; natively it allocates nothing but runs through the
     runtime's dispatch, 27 % more, until the backend lowers the five in
     place as it does `dict.get`. Stage 1 decides it (RULES.md, R134).

801. Stage 1's parser finds the starts of lines from its tokens and calls
     less per token: a parse of the prelude takes 24 % fewer instructions
     and a two-line `check` 6 % fewer (3.43 G to 3.23 G, the parser 0.91 G
     to 0.70 G), with the same trees. Stage 1 decides it (RULES.md, R135).

802. A name may hold combining marks after its letters and is read in NFC;
     `W0031` warns of a name whose letters are of several scripts and
     `W0032` of a name that reads as another in scope, by generated tables
     (`tools/unicode_tables.py`, Unicode 15.1) that `check.sh` keeps
     fresh. Stage 1 decides it (RULES.md, R136; D-048).

803. The sized integers have `checked_add`, `checked_sub`, `checked_mul`
     and `checked_pow`, and `int` has `checked_pow`; `Result.or_else`,
     `Iter.try_map` and `list[Result[T, E]].try_collect` are the prelude's.
     Stage 1 decides it (RULES.md, R137).

804. A body with a syntax error has its names resolved but for the
     statements whose shape the error broke: none of their names is
     reported, and none of the names they write is reported later. Stage 1
     decides it (RULES.md, R138).

805. `sisu doc`'s rendering of an example's values waits for the example
     to be built and run as a worksheet is, which the compile-time
     evaluator cannot do for examples that print or take capabilities.
     Stage 1 decides it (RULES.md, R139).

806. Every type has a `Debug` form without asking, a public one too, and a
     type that hides its fields writes its own impl; 10's opt-in for
     public types is withdrawn. Stage 1 decides it (RULES.md, R140).

807. The conformance suite's `packages/runtime_single.pkg` differs from
     stage 0 as stage 1's own copy of it does (R122), and `check.sh` lowers
     a package of a runtime profile other than `full` by the single command,
     whose base is of its profile, where a batch's base is `full`. Stage 1
     decides it (RULES.md, R141).

**Stage 0's fifteenth round: stage 1 in the release profile for the
driver, the lookups by `str` in place, the descriptors read once, the
export boundary's `D0103`, and what was left** (sections "The query layer
and the driver", "The runtime in Sisu", "Raw memory, foreign calls and the
machine intrinsics"; D-086; 10, 12, 17; decisions 232, 265, 331, 784, 785,
820 and 823 to 828; stage 1's R134 and R142)

860. The stage 1 that the driver builds and every `sisu` command runs
     (`compiler/target/stage1/sisu1`) is built in the release profile with
     every contract checked (`sisu build --native --release
     --contracts=all`, decision 232), Cranelift's `speed` level where the
     debug profile's Cranelift build was `none`;
     `SISU_STAGE1_PROFILE=debug` builds the debug one, and the driver
     writes the profile, with the executable's size and time, to
     `target/stage1/profile`, so that an executable built otherwise (in
     the other profile, or by `stage1/sisu1`, whose build is the debug
     profile's with the Sisu backend) is built again at the next command.
     The fixed points build their own stage 1 as they did
     (`stage1/bootstrap.sh`, `stage2/bootstrap.sh`, `check.sh`'s `self`
     and `fixed`), and the SIR stage 1 prints does not depend on its
     profile (the runtime's seed printed by both is the same file). On the
     round's machine (four cores, at a load of 7 to 17 from the other
     work): the build takes 43.4 s where the debug one took 39.2 s (code
     generation 21.7 s where it was 15.8 s), so CI's warm-up step, `sisu
     tokens std/prelude.sisu`, pays about 4 s more; a parse of the prelude
     (`sisu1 parse --print`) takes 356.7 million instructions where it
     took 450.9 million (callgrind), 20.9 % fewer, and a `sisu check` of a
     two-line program, the driver and stage 1 together, 3.706 G where it
     took 4.191 G, 11.6 % fewer, `--contracts=all` costing nothing
     measurable against the release profile's default preconditions;
     the conformance suite (`sisu-conformance conformance/cases`, 570
     cases, the release driver) takes 507 s where it took 513 s, within
     the load's noise, since most of its time is the programs' runs in the
     interpreter and the runner's own. This changes no rule of the
     language.

861. The five lookups by a `str` of decision 784 run inline, as
     `dict.get`, `dict.contains` and `set.contains` do (decision 265), in
     both lowerings byte for byte: `dict.get_str` and `set.contains_str`
     through `sisu_rt_table_find`, the text's type given as the key's;
     `dict.index_str` through `d{k}`'s slot read (`sisu_rt_dict_slot`,
     `SLOT_READ`), whose missing key panics naming it at the index; and
     `dict.remove_str` and `set.remove_str` through a new entry,
     `sisu_rt_table_take(table, key, key_ty, value, out)`, which the
     runtime in Sisu does (`dispatch.take`, exported as
     `sisu_rt_table_inline_take`): the key taken out of the table as
     `remove` takes it, the option of its value or whether it was there
     written at the result, the key the table held dropped. A loop of
     28 333 lookups by a `str` (`get`, `contains` and an index, stage 1
     lowering them, R134) takes 66.6 million instructions natively where it
     took 121.6 million through the runtime's dispatch, 45 % fewer, and the
     same loop that makes a `String` for each key 92.8 million (callgrind,
     the release profile, the Rust lowering). The test
     `lookups_by_a_str_run_interpreted_and_natively` holds all five to the
     interpreter's output in both lowerings and both profiles, with no
     allocation left. This changes no rule of the language.

862. The runtime in Sisu reads a descriptor's first six words once, when
     a call is made (`dispatch.Call`: the intrinsic's number, its numbers
     of arguments and type arguments, its result's type and the first two
     argument types), two to a word of eight bytes, and both lowerings give
     a descriptor of an odd number of words a word of 0 after them, so
     that no word is read a byte at a time (`Codegen::desc`, stage 2's
     `Cg.desc`), byte for byte alike; the runtime in Rust reads a
     descriptor by its counts and ignores the word. The 150 000 calls of
     decision 820 (50 000 loops over a borrowed list, dict and set) take
     770.0 million instructions where they took 812.2 million, 5.2 %
     fewer, `sisu_rt_collection` 234.8 million where it took 277.0 million
     for its 100 000 calls (the list's iterator is made inline), about 420
     instructions less a call, of the 950 decision 820 measured. What is
     left of the difference is the following of the argument's references
     (`Call.place`, 543 instructions a call, half of it in calls of Sisu's
     convention) and the iterator's allocation, which the runtime in Rust
     gives. The seed grows from 755 222 to 759 359 bytes with 861 and 862,
     the runtime's object from 851 480 to 866 320 bytes (`target/rt/rt.o`),
     and as the fixed point builds it with the Sisu backend from 1 478 176
     to 1 508 224; the fixed point holds (rt-A.o and rt-B.o the same bytes,
     as sisu2-A and sisu2-B, 31 174 752, and stage1-A and stage1-B,
     63 849 072).
     This changes no rule of the language.

863. The rest of the intrinsics' decoding (decision 820's 265 of 328,
     `sum`, the formatting, the characters', the floats', the texts' and
     the bytes' families first) is not this round's: each family moves
     with its panics' messages, which the runtime in Rust raises with
     their parameters (`sum`'s overflow names its two sides), and the
     round spent its time on 860 to 862 and 864; 862 makes each move cost
     about 420 instructions a call less than 820's did. This changes no
     rule of the language.

864. `D0103` has its heading, `error[D0103]: the native backend does not
     compile ...`, as the driver's other errors of a build do, and its
     text names a `@repr(c)` struct that an exported function takes or
     gives by value as the program wrote it, `a `Reading` by value (a
     parameter of the exported function `sauna_load`)`, where it named the
     reference the SIR passes it by (`a `ref Reading``), and the function
     it is in, `load`, where it left it empty; both lowerings give the
     same text (the manual's finding from R14). A raw pointer to a struct
     of words (`int`, `f64`, raw pointers, and structs of them) compiles
     natively as the address of its place, as `ptr.of` of an `int` does,
     since a place holds such a struct as raw memory does, so a program
     hands its own export the `ptr[Reading]` that C passes
     (`unsafe/export_by_pointer`, in `native.list`). A struct with a
     narrower field (an `i32`, a `u8`) stays `D0103` under `ptr.of`,
     `ptr.of_mut` and `as_ref`: a place holds each of its fields in a word
     (the layout of a place, port-debt.md) where raw memory holds it at its
     C offset and width, so the address of the place is not a pointer to
     the struct C lays out, and a copy converted for the call would not
     see a write through `ptr.of_mut`; it waits for the layouts to be per
     type (D-086), as `mem.slice.as_ptr` of a `u8` does (decision 823).
     This changes no rule of the language.

865. `sisu test src/MODULE.sisu` of a package's module (decision 828) is
     not this round's in the driver: stage 1's sixteenth round gives
     `sir --tests --modules`, which writes ` in "FILE"` on each `test` and
     `bench` line (its R142, `sisu1 0.12.0`), but in its own tree, and the
     stage 1 of this one is 0.11.0, whose `sir` takes no `--modules`; the
     driver, when `sisu1 --version` says 0.12.0, is to run the package's
     `sir --tests --modules` for a file under its `src/`, keep the `test`
     lines whose FILE is that module's path from the package's root, and
     take the words out before stage 0's SIR reader reads the text. This
     changes no rule of the language.

866. A native `CString.as_ptr()` (decision 823) and tagged-pointer
     provenance (decision 825) are not this round's either: the first
     waits for the per-type layouts of `list[u8]` and `Slice[u8]` (D-086),
     the second is the round of its own that port-debt.md describes. This
     changes no rule of the language.

**Stage 1's sixteenth round** (stage 1's RULES.md, R142 to R149; a test's module in `sir --tests --modules`, reflection on builtin types and one `E0609`, `E0607` alone, `reflect.functions_of`, `try_collect` after `map`, the trait check's cost, Hangul names, the `cstring` case's differences)

840. Stage 1's `sir --tests --modules` writes ` in "FILE"` on each `test`
     and `bench` line after its function, FILE the source of the module
     that declares it, so that the driver can run one module's tests
     (`sisu test src/MODULE.sisu`, decision 828); `sir --tests` alone is
     unchanged, since stage 0's SIR reader drops the tags after a word it
     does not know. `sisu1 --version` says 0.12.0. Stage 1 decides it
     (RULES.md, R142).

841. A `reflect` query may name the prelude's scalar types (`int`, `f64`,
     `bool`, `char`, `str`) in any function, and a query given what
     compile time knows no type by is one `E0609` at its argument, saying
     what reflection takes, where it was `E0900` with an `E0200` in the
     instance's own module. Stage 1 decides it (RULES.md, R143).

842. A derive template that `E0607` refuses is the one error: a `@derive`
     of its trait is not `E0243`, and the functions and methods the derive
     would have written are not `E0351` or `E0304` where a type that
     derives it uses them. Stage 1 decides it (RULES.md, R144).

843. `reflect.functions_of(m)` gives the functions of a module the code
     imports, each with its name, parameters, result type, attributes and
     comment, and stands in run-time code for the function itself, so a
     framework collects its routed functions into a table at compile time
     (08). Stage 1 decides it (RULES.md, R145).

844. `Map[I, Result[T, E]]` has `try_collect`, appended to the prelude,
     and a `dict`'s index by a `str` that is only borrowed (a method's
     receiver, a `str` view) reads through `dict.index_str` with no
     `String` made. Stage 1 decides it (RULES.md, R146).

845. Stage 1's trait check finds a derived impl among its trait's impls
     and the resolver keeps each trait's impls in place: a two-line
     `check` takes 3.34 G instructions where it took 3.39 G, the trait
     check 0.10 G where it took 0.15 G, resolving 0.38 G as before, its
     cost spread over the base's signatures. Stage 1 decides it
     (RULES.md, R147).

846. A name's NFC composes Hangul jamo into syllables by UAX 15's rule;
     `@derive(Into, From)` waits for the owner, since the prelude has no
     `From`, `Into` promises a lossless conversion, and a derive's
     template takes no trait argument. Stage 1 decides it (RULES.md,
     R148).

847. The conformance suite's `unsafe/cstring` differs from stage 0's front
     end as the other cases of `unsafe/` do (R4). Stage 1 decides it
     (RULES.md, R149).

**Stage 0's sixteenth round: one module's tests, an argument's place
without a call, `sum` in Sisu, and what was left** (sections "Testing",
"The runtime in Sisu", "Raw memory, foreign calls and the machine
intrinsics"; D-085, D-086; 12, 17, 22; decisions 686, 726, 787, 820, 823,
825, 828, 840, 862 to 866; stage 1's R142)

900. `sisu test src/MODULE.sisu` of a module of a package (a file under
     the `src/` of the nearest package's manifest above it) runs the tests
     that module declares, the package compiled as `sisu test` compiles it,
     its imports resolved as the package's, where it compiled the file
     alone, and an import of a module beside it was `E0220` (decision 828,
     the manual's finding from chapter 17): when `sisu1 --version` says
     0.12.0 or later, the driver asks stage 1 for the package's `sir
     --tests --modules` (its R142, decision 840) and keeps the `test` lines
     whose ` in "FILE"` is the module's path from the package's root, with
     their tags; an older stage 1, the Rust front end, a black-box file
     under `tests/` and any other file keep what they did. Stage 0's SIR
     reader reads the words before `tags` and drops them, and
     `sisu_sir::read::test_files` gives each test's file. A test's frame is
     named by the test whose function it is, in `display_fn_name` and stage
     2's alike, where it was named by its function's number, which a
     program of one module's tests no longer keeps; nothing else prints
     differently. The run's replay line names the module's file, and its
     tokens hash the module's path with the package's code, so that a token
     of the whole package's run is refused for the module's and the other
     way round (`D0033`), the numbers of their tests differing; a run with
     `--filter` or `--tag` selects among the module's tests, and its `not
     selected` counts only those. `sisu test` of the package is unchanged.
     `packages/test_modules` has three modules, a suite with a tag, and a test
     that calls a sibling module, its `.test` the package's
     five tests and its `.select` the three of `src/heat.sisu`, which the
     conformance runner gives a package case as `sisu test` with the lines
     of `NAME.select-args`; both run in the interpreter and natively. This
     changes no rule of the language.

901. The per-type layouts of `list[u8]` and `Slice[u8]` (decisions 823,
     864 and 866) are not this round's: a byte to an element would not
     alone give a native `CString.as_ptr()` its pointer, since a native
     `Slice` is a copy in a list's three words (decision 225, `bytes.as_slice`
     copying each byte into a word), so `as_ptr` of
     `self.data.as_slice()` in stage 1's std.ffi would point to a temporary
     dropped before the foreign call; the slice must also view what it was
     made from in place, as the interpreter's does since decision 823, with
     a drop that frees nothing, in both lowerings, the runtime in Rust's
     slice constructors and the runtime in Sisu's `Slice` intrinsics.
     The width of an element is a word for every type wherever a list's
     element is reached: the generated code's inline index and iteration
     and the descriptors' element sizes in both lowerings, and in the
     runtime in Sisu each read of a type's size that steps through a
     list, an iterator, a table or a slice, and the walks of `value.sisu`
     and `show.sisu`, which read a `u8` as a word, each to be held byte for
     byte between the SIR and LIR paths and the interpreter's checked
     model. Done whole it is a round of its own, which port-debt.md
     describes; a pointer to a struct with a narrower field stays `D0103`
     with it (decision 864), and `unsafe/cstring` stays out of
     `native.list`. This changes no rule of the language.

902. The runtime in Sisu finds the place an intrinsic's argument is, or
     refers to, without a call (`dispatch.Call.place`): the argument's type
     from the descriptor's words read when the call was made, and a value
     that is neither a reference nor a `dyn`, or a reference to one, from
     the type table's words in place, `value.target` called only for the
     others; `Call.ty` reads the first two types without `after_head`
     (decision 862). The loop of decision 820 (50 000 calls over a borrowed
     list, dict and set, of which the dict's and the set's 100 000
     iterators are made through the dispatch) takes 573.2 million
     instructions where it took 606.9 million, 5.6 % fewer,
     `sisu_rt_collection` 201.1 million where it took 234.8 million, and
     `Call.place` 202 instructions a call where it took 543 (callgrind, the
     release profile, the Rust lowering). What is left of a call's cost is
     the iterator's allocation (`iter.table_part`, 1 145 instructions a
     call) and the calls of Sisu's convention, each about a hundred. The
     safety comments of `dispatch.take` (decision 861) name the obligations
     stage 1's `check` asks for, so the package checks with no warning.
     This changes no rule of the language.

903. `sum` is the runtime in Sisu's (`dispatch.sum_call`, decision 863's
     first family): every item taken, first to last, by `iter.iterable` and
     `iter.collect` as the runtime in Rust took them through its bridge,
     then the integers added with their overflow checked, whose panic
     (`sisu_rt_sum_overflow`) names the sum so far and the item as before,
     the `f64`s added pairwise in the fixed shape of D-074, and the `f32`s
     pairwise too, each addition of two by the runtime in Rust
     (`sisu_rt_f32_add`), since raw memory holds no `f32` that the
     runtime's code reads inline (the backend's raw access is of words and
     bytes, decision 335); the sum of no float is 0.0, whose bits are 0. Deleted from the
     runtime in Rust: `intrinsics::sum`, `pairwise` and `pairwise32`, the
     bridge's `iter_iterable` and `iter_collect` with their imports,
     `iter::iterable`, `iter::collect_all` and `Call::item`, which only it
     used, and the runtime in Sisu's exports that only they called
     (`sisu_rt_iter_iterable`, `sisu_rt_iter_collect`): the Rust of the
     runtime is 15 294 lines (573 078 bytes) where it was 15 421 (577 573),
     264 of the 328 intrinsics decoded there. `std/sum` holds every kind
     to the interpreter's output (integers, an overflow, `f64`s whose
     pairwise sum differs from a running one, `f32`s, none, a range and
     an iterator that prints as it goes), in `native.list`. 20 000 sums of
     a list of three `int`s and of three `f64`s natively take 583.5
     million instructions, 96 % of them the iterator made of the argument
     (4 950 a call) and its items collected (6 990), which were the
     runtime in Sisu's before too, through the bridge, so a call costs what
     it did but for the decoding. The runtime in Sisu is 7 483 lines where
     it was 7 379, its seed 772 518 bytes where it was 759 359, and its
     object 879 872 where it was 866 320 (`target/rt/rt.o`), and as the
     fixed point builds it with the Sisu backend 1 535 712 where it was
     1 508 224; the fixed point holds (rt-A.o and rt-B.o the same bytes,
     as sisu2-A and sisu2-B, 31 109 272, and stage1-A and stage1-B,
     63 942 768). The
     formatting, the characters', the floats', the texts' and the bytes'
     families are not this round's (decision 863). This changes no rule of
     the language.

904. Tagged-pointer provenance (decisions 686, 726, 787, 825 and 866) is
     not this round's either: port-debt.md puts it at a week, the runtime's
     `checked.rs` first and then each inline use of a pointer's word in
     both lowerings behind a switch, and this round spent its time on 900,
     902 and 903; a `checked.rs` that keeps a tag's borrow and frame with
     no lowering writing a tag yet would be code that nothing runs, so the
     round leaves it to the round that starts the lowerings with it. This
     changes no rule of the language.

**Stage 1's seventeenth round** (stage 1's RULES.md, R150 to R157; stage 1's own builds on the driver's release build, compile-time function values and attribute arguments as values, a missing key's panic at the index, `ptrsize`, `uptrsize`, `Sized`, the `Array` literal and `Url` literals, `try_collect` after other adapters and `Decimal`'s `/` decided against, compile-time types planned, the `export_by_pointer` case's differences)

880. `stage1/sisu1` runs the stage 1 the driver builds (decision 860, the
     release profile with every contract checked, the stamp
     `target/stage1/profile`) and asks the driver to build it, so the two
     no longer build over each other's executable; `check.sh --native`
     runs it for its batches, and the classes `self` and `fixed` a debug
     stage1-A of their own. Stage 1 decides it (RULES.md, R150).

881. A `comptime` expression's function values are kept as the constants
     that name them, so a table of routed functions is evaluated at
     compile time, and an attribute's arguments are values (a string
     literal's text, an integer, a boolean, a name's text) with their
     keywords in `keys`, where they were their source text. Stage 1
     decides it (RULES.md, R151).

882. A `dict`'s entry read inside an expression is read at the index, so
     a missing key's panic names the index whether the key is a literal,
     a `str` or a `String`. Stage 1 decides it (RULES.md, R152).

883. The prelude has `ptrsize` and `uptrsize` (`i64` and `u64` on every
     target now) and the marker trait `Sized`, which every type but a
     `dyn` one implements; a list literal of `N` elements where an
     `Array[T, N]` is expected is that array; and `std.net` has
     `FromLiteral[str] for Url`. Stage 1 decides it (RULES.md, R153).

884. `try_collect` after an adapter whose item is its inner iterator's is
     decided against until the type system has a bound on an associated
     type's value or associated types of associated types; the chain ends
     in its `map`, or in `map(r -> r)`. Stage 1 decides it (RULES.md,
     R154).

885. Compile-time code that makes types starts with a `comptime def ... ->
     type` that the template engine runs and a type position written
     `comptime f(args)`, whose syntax is the owner's; `@diagnostic_name`
     follows it, and `E0900` stays until then. Stage 1 decides it
     (RULES.md, R155).

886. `Decimal`'s `/` giving a `Ratio` waits for a `Ratio` of `BigInt`s,
     since today's `Ratio` of `int`s cannot hold every quotient 03
     promises exactly. Stage 1 decides it (RULES.md, R156).

887. The conformance suite's `unsafe/export_by_pointer` differs from stage
     0's front end as the other cases of `unsafe/` do (R4). Stage 1
     decides it (RULES.md, R157).

**Stage 0's seventeenth round: slices that view in place, one module's
benchmarks, a native executable's sizes explained, and what was left**
(sections "Stage 2: the native backend", "Values", "The runtime in Sisu",
"The checked memory model"; D-085, D-086; 10, 12, 17, 22; decisions 214,
225, 682, 823, 864, 900, 901, 903 and 904; stage 1's R142 and R158)

940. A native `Slice[T]` views the list it was made from in place: three
     words, the address of the first element it views, their number, and
     a capacity of 0, which marks a view and whose drop frees nothing and
     drops nothing (as decision 214's views of `*rest` mark theirs), where
     it was a copy of every element in a list of its own. Its type has a
     kind of its own in the type table (`abi::Kind::Slice`, code 47), laid
     out and encoded alike by both lowerings (`sisu-codegen`'s
     `layout.rs`, stage 2's `layout.sisu` and `abi.sisu`), so the SIR and
     LIR paths give the same bytes; equality, order, the hash, `Debug`,
     `in`, `len`, an iterator's references and `as_ptr` read it as a
     list, in the runtime in Rust and the runtime in Sisu
     (`value.sisu`, `show.sisu`, `iter.sisu`, `dispatch.sisu`), and its
     copy copies its words. `list.as_slice`, `Slice.slice` and
     `Slice.cut` make views (`dispatch.view_of`), and `Slice.to_list` a
     list of copies of what the slice views. A slice of `bytes`
     (`bytes.as_slice`, `mem.slice_from_raw`) is still a copy, a word a
     byte, in a buffer it owns, whose capacity is not 0, since a view of
     packed bytes needs decision 941's layout. In the interpreter, whose
     slice stays a list of copies with its pointer from `as_ptr` viewing
     the source (decision 823), dropping a slice drops none of the copies
     (`drops.rs`), as SIR's `drops_something` has said of a `Slice` all
     along, where a slice of values with a `Drop` ran it for each copy:
     the checker keeps a view from outliving its list, so nothing a slice
     holds is the slice's to drop. `std/slices` holds the views to the
     interpreter's output (two slices of one list have one address, a
     slice of a slice, a slice handed to a function and back, `iter`,
     `contains`, `is_sorted`, a slice of `String`s and of values with a
     `Drop` made and dropped in a loop, a slice of `bytes`), in
     `native.list`, with no allocation left. 2 000 slices of a list of
     1 000 `String`s, each cut and read at its ends, take 13.96 million
     instructions natively where they took 3 572.1 million (callgrind,
     the release profile), the copies of the `String`s gone. The runtime
     in Sisu is 7 513 lines where it was 7 483, its seed 777 695 bytes
     where it was 772 518, and its object 884 584 where it was 879 872
     (`target/rt/rt.o`), and as the fixed point builds it with the Sisu
     backend 1 546 336 where it was 1 535 712; the fixed point holds (rt-A.o
     and rt-B.o the same bytes, as sisu2-A and sisu2-B, 31 127 928, and
     stage1-A and stage1-B, 64 081 192). This changes no rule of the language.

941. The packed layout of `list[u8]`, `Slice[u8]` and `bytes` viewed as a
     slice is not this round's, so a native `CString.as_ptr()` stays
     refused (`unsafe/cstring` out of `native.list`) and so does a pointer
     to a struct with an `i32` or a `u8` field (`D0103`, decision 864):
     with 940 a slice views a list in place, but `bytes` are a byte each
     and a `Slice[u8]` a word each, so `bytes.as_slice()`, which
     `CString.as_ptr()` calls, still copies. A byte an element is not the
     element's size alone: `s[i]` and `Slice.at` give a `ref u8` into the
     buffer, which every read of a `u8` through a reference reads as a
     word, so a packed element is a `u8` laid out a byte wherever it is
     reached in memory, the reads and writes through references in both
     lowerings, the walks of `value.sisu` and `show.sisu` and the
     runtime in Rust's conversions, which is D-086's per-type layout of a
     `u8` and of the narrow fields together, the round of its own that
     port-debt.md describes. This changes no rule of the language.

942. The driver keeps no server of stage 1 for the ordinary commands this
     round: stage 1's eighteenth round gives `sir --repl-serve` every
     `sir`, `check`, `parse`, `fmt`, `lint`, `doc` and `tokens` request
     with its `--dir` (its R158, `sisu1 0.13.0`, decision 920) in its own
     tree, and the stage 1 of this one is 0.12.0. A server that outlives
     the command that started it also needs what the driver does not
     have: the server answers on its standard streams, which only the
     process that started it holds, so the driver is to start, per
     working directory, a process of its own that holds the server and
     answers the commands over a socket under `target/stage1/` (a lock
     beside it so that one starts), ends after some minutes idle and when
     `sisu1`'s stamp changes (R158's `stale`, after which the command is
     sent again), and is left alone under `SISU_STAGE1_SERVE=0`; R158
     measures a two-line `check` at 0.009 s served where it is 0.22 s as
     a process. This changes no rule of the language.

943. `sisu bench src/MODULE.sisu` of a module of a package runs the
     benchmarks that module declares, the package compiled as `sisu bench`
     compiles it, as `sisu test` of one runs its tests (decision 900),
     where it compiled the file alone, as `sisu test` of one did before
     900: the driver asks stage 1 for the package's `sir --tests
     --modules` and keeps the `bench` lines whose ` in "FILE"` is the
     module's path from the package's root (`keep_module_benches`, by
     `sisu_sir::read::bench_files`), and stage 0's SIR reader reads the
     words on a `bench` line as on a `test` line. The times differ from
     run to run, so the driver's test
     `one_modules_benchmarks_run_with_its_package` holds the names and
     the count rather than a case's output. This changes no rule of the
     language.

944. `sisu build --native --explain-size` prints, after the `built` line,
     the executable's size (`D0139`) and its parts (`D0140` to `D0146`):
     the program's own code (the functions the backend named, the
     standard library's instances among them, and the C `main`), the
     runtime in Sisu's (the functions its object `target/rt/rt.o`
     defines), the runtime in Rust's (its crate's), Rust's standard
     library's and the rest of the code, the debug information (the
     `.debug_*` sections), the symbols (the symbol table and its names),
     and the data, the unwinding tables and the headers, which add up to
     the file's size, each with its share; the driver reads them from the
     linked file and the object (`size.rs`, a reader of 64-bit ELF), as
     decision 682 measured them. The option goes with `--native`
     (`D0148`), and a file that is not such an executable is `D0147`; it
     was `D0009` (10, "Binary size"; the manual's "Tooling" finding). A
     two-line program's executable, the release driver, is 6 319 272
     bytes: 1 106 of its own code, 526 944 of the runtime in Sisu's,
     469 303 of the runtime in Rust's, 365 734 of the rest, 4 089 520 of
     debug information, 435 634 of symbols and 431 031 of the rest. The
     driver's test `explain_size_gives_the_parts_of_the_executable` holds
     the form in both profiles (the lines, their order, the parts adding
     up to the size), not the numbers, which change with the runtime.
     This changes no rule of the language.

945. The formatting, characters', floats', texts' and bytes' families of
     the intrinsics (decision 863) stay the runtime in Rust's this round,
     264 of the 328 decoded there: the round spent its time on 940, 943
     and 944, and each family moves with its panics' messages, as `sum`
     did (decision 903). This changes no rule of the language.

**Stage 1's eighteenth round** (stage 1's RULES.md, R158 to R163; a server for every command at 0.13.0, a template's syntax error once and `E0616` for a plain function's compile-time failure, closures kept in compile-time results, the attributes of fields, variants and methods, `Normalize` and `Graphemes` with generated tables, `BigRatio` and `Decimal`'s `/`)

920. `sisu1 sir --repl-serve` serves any `sir`, `check`, `parse`, `fmt`,
     `lint`, `doc` or `tokens` request, run in the working directory its
     `--dir DIR` names, with R112's frame and the status the command gives
     alone; the base is made again when a std text, the catalog or the
     request's runtime changed, and a server whose executable was built
     again answers `stale` and ends. A `check` of a two-line program takes
     0.009 s served where it takes 0.22 s as a process. `sisu1 --version`
     is 0.13.0. Stage 1 decides it (RULES.md, R158).

921. A syntax error in an instance of the program's template is reported
     once, in the template's file, and compile-time code of a function
     without type parameters that stops is `E0616`, in a plain function's
     words, where it was `E0609`'s instance words. Stage 1 decides it
     (RULES.md, R159).

922. A `comptime` result keeps a closure that captures nothing or only
     constants, as the closure of its function; one that captures more is
     `E0604` with a note on what a result keeps of functions. Stage 1
     decides it (RULES.md, R160).

923. Reflection gives a field's and a variant's attributes as
     `f.attributes`, a method's as `reflect.attributes_of(m)`, and `x.[m]`
     names a method, so a template routes by a method's attribute. Stage 1
     decides it (RULES.md, R161).

924. `std.string` has the normalisation forms `nfc`, `nfd`, `nfkc` and
     `nfkd` (`Normalize`) and `graphemes()` (`Graphemes`) for `str`, from
     generated Unicode tables; the graphemes keep a base's marks, `\r\n`,
     Hangul syllables, flags and joined sequences, short of UAX #29's
     `Prepend`, conjunct and pictograph rules. Stage 1 decides it
     (RULES.md, R162).

925. `std.num` has `BigRatio`, an exact fraction of `BigInt`s, and a
     `Decimal`'s `/` gives one, exactly; `Ratio` keeps its `int` parts.
     Stage 1 decides it (RULES.md, R163).

**Stage 0's eighteenth round: the driver's server of stage 1 for every
command, the language server and `sisu eval` on stage 1, a module's
replay token, and what was left** (sections "The query layer and the
driver", "The language server", "The runtime in Sisu"; D-085, D-086; 08,
10, 17, 22; decisions 720, 823, 864, 900, 904, 941, 942 and 945; stage 1's
R158, R164 and R165)

980. The driver keeps a server of stage 1 for every command it serves
     (decision 942's design, stage 1's R158): a process of the driver,
     `sisu __stage1-serve SOCKET SISU1` (`serve.rs`), that holds processes
     of `sisu1 sir --repl-serve` and answers over a Unix socket,
     `target/stage1/serve/HASH.sock`, its name hashed from the driver's
     and stage 1's paths, so that a debug and a release driver, or a stage
     1 given by `SISU_STAGE1`, each have their own. The first command that
     finds none starts it, under a lock beside the socket so that one
     starts, in its own process group and with none of the command's
     standard streams, so that nothing that reads what the command prints
     waits for it. A request is R158's line, the command's words and
     `--dir` with the command's working directory, and the answer R158's
     frame, which the driver reads as it reads a process's output
     (`settle`); since a request names its directory, one server answers
     every directory, with up to one process of stage 1 for each core,
     each making the standard library's base once, started when the others
     are busy. A process of stage 1 that answers `stale` (`sisu1` was
     built again) is replaced and the request sent again; the server
     itself ends after 300 seconds without a request
     (`SISU_STAGE1_SERVE_IDLE` sets the seconds) and when the driver's
     executable was built again, answering `stale` to a request that finds
     it so, after which the command starts a new one and sends the request
     again. Whatever goes wrong, the command runs stage 1 as a process of
     its own, as every command does under `SISU_STAGE1_SERVE=0` or with a
     stage 1 older than 0.13.0. Served: `check`, `sir`, `parse`, `fmt`,
     `lint`, `doc` and `tokens`, `test`'s and `bench`'s `sir --tests` and
     `check`, and from 0.14.0 `query` and `eval` (983, 984); `sisu1
     --version` is asked once for each executable, its answer kept beside
     it (`target/stage1/version`), so that a served command starts no
     process of stage 1. A two-line `sisu check` takes 0.014 s served where
     it takes 0.244 s as a process (the release driver, the median of 20,
     a load of 6 to 9 on four cores), and the conformance suite 138.6 s
     where it took 224.4 s, 0 failed both ways; the runner passes its own
     `SISU_STAGE1_SERVE` into the empty environment of each command, so
     that the suite runs both ways. A process of stage 1 holds 55 to 90 MB.
     CI sets nothing: a job's server has no stream of its steps and ends
     300 seconds after its last command, and the runner ends what a job
     leaves running. The driver's test
     `the_server_of_stage_1_answers_as_a_process_does` holds twelve
     commands (errors, a package, `fmt --check`, `doc`, a missing file, a
     test run) byte for byte both ways. This changes no rule of the
     language.

981. `sisu run`'s SIR is not served: the driver asks stage 1 for it with
     `build --native --for-run`, whose backend, the driver run with
     `SISU_KEEP_SIR`, writes the SIR it is handed (decision 268), and R158
     answers a `build` as misused, the variable being the server's
     environment and not the request's. Stage 1's `sir` is not the same
     command: `build --for-run` stops at the front end's, a `comptime`
     expression's and the ownership checker's errors with `D0006` and
     `D0021`, and checks a release build's holes, which a `sir` followed
     by its `check` would give in another order. A served `sisu run`
     needs a request that answers what the build hands its backend, on
     standard output, with the build's diagnostics, which is stage 1's to
     add. This changes no rule of the language.

982. A byte laid out as a byte (decision 941; D-086's per-type layout of
     `u8` and the narrow integers) is not this round's: it changes how
     every read and write through a reference, every element step of a
     list, a slice, an iterator and a table, and every walk of
     `value.sisu` and `show.sisu` reads a `u8`, in both lowerings, the
     runtime in Rust's conversions and the checked model, held byte for
     byte between the SIR and LIR paths, and this round spent its time on
     980, 983 and 984; half of it would leave the lowerings disagreeing
     about a `ref u8`. The round that does it starts with the layout
     tables (`sisu-codegen`'s `layout.rs` and stage 2's `layout.sisu` and
     `abi.sisu`, a size and an alignment for each scalar, a struct's
     fields at their offsets), then the loads and stores through a
     reference by the referent's size in both lowerings, then a list's
     element size from the descriptor in the runtime in Sisu
     (`list.sisu`, `iter.sisu`, `value.sisu`, `show.sisu`) and in Rust,
     with `std/bytes` and a struct with narrow fields as cases in every
     native mode before `bytes.as_slice` views in place; `unsafe/cstring`
     stays out of `native.list` and a pointer to a struct with a narrow
     field `D0103` (decision 864) until then. This changes no rule of the
     language.

983. `sisu lsp` answers the diagnostics, hover, go-to-definition,
     references, completion and document symbols from stage 1's `query`
     (its R164) when stage 1 is the front end, built and 0.14.0 or later
     (`query.rs`): each document's text written to a file of the server's
     own, which `--text` names, the query served by 980's server, its
     lines of tab-separated fields made the protocol's JSON, a position's
     column counted in characters turned into the editor's UTF-16 or UTF-8
     units. A query stage 1 cannot answer, an older stage 1, a stage 1 not
     built (the language server does not build it under an editor) and
     `--front-end=rust` keep the Rust front end's answers, which also
     answer selection ranges, formatting, rename, code actions, inlay
     hints, worksheets' values and `sisu.explainBorrow`. The tests of each
     feature (`sisu-driver/tests/lsp.rs`) run on the Rust front end's
     answers; `stage_1s_queries_answer_as_the_rust_front_end` drives both
     with a document's diagnostics, a definition, a hover, its symbols, a
     function's references and the members after a `.`, which agree. The
     main tree's stage 1 is 0.13.0, so the queries were checked with stage
     1's nineteenth round's executable given by `SISU_STAGE1`. This
     changes no rule of the language.

984. `sisu eval FILE [--to json|toml|yaml]` (08, "Configuration files")
     runs stage 1's `eval` (its R165) when it is the front end and 0.14.0
     or later, its output and status as they are, served by 980's server;
     an older stage 1 and `--front-end=rust` keep `D0003`, an unknown
     command. The option is R165's `--to`, as 08 spells it. The driver's
     test `eval_writes_a_configurations_constants_with_stage_1` holds JSON
     and TOML with a stage 1 that has `eval` and `D0003` with one that has
     not; the usage (`D0002`) names the command when the main tree's stage
     1 has it. This changes no rule of the language.

985. A replay token of one module's run given to its package's run (or to
     another module's) is `D0149`, "the token is for the run of the tests
     of `src/stats.sisu`; `sisu test --replay TOKEN src/stats.sisu`
     replays it", and one of the package's run given to a module's
     `D0150`, which names the package's directory to replay it in, where
     both were `D0033`, "the token is for other code", with the root
     module's hash (the manual's finding on `D0033`, decision 900): the
     driver hashes the program again for the package's run and for each
     of its modules' (`token_of_other_run`), and a token none of them
     made stays `D0033`. The driver's test
     `a_token_of_another_run_of_the_package_names_its_run` holds both on
     `packages/failing_test`. This changes no rule of the language.

986. The formatting, characters', floats', texts' and bytes' families of
     the intrinsics (decision 945) and tagged-pointer provenance (decision
     904) are not this round's either: the round spent its time on 980,
     983, 984 and 985. This changes no rule of the language.

**Stage 1's nineteenth round** (stage 1's RULES.md, R164 to R168; the language server's queries served by stage 1 at 0.14.0, `eval` for configuration modules, the template evaluator's builtins, UAX #29's remaining rules decided against, the `std/slices` case's differences)

960. `sisu1 query KIND FILE [LINE COL] [--text PATH]` answers the
     language server's questions from stage 1's check of the program that
     holds FILE: `diagnostics`, `symbols`, `hover`, `definition`,
     `references` and `completions`, one line per item of tab-separated
     fields, positions as `check --format=ids` writes them; `sir
     --repl-serve` serves it, a hover in 0.009 s. `sisu1 --version` is
     0.14.0. Stage 1 decides it (RULES.md, R164).

961. `sisu1 eval FILE [--to json|toml|yaml]` evaluates a module's `const`
     items with the compile-time evaluator and writes them as JSON (the
     default), TOML or YAML; a constant whose evaluation stops is that
     error at its name, and one the form cannot write is `E0617`. Stage 1
     decides it (RULES.md, R165).

962. A template's compile-time code may call `range`, `zip`, `reversed`,
     `sorted`, `min`, `max` and `sum` of lists and numbers and the methods
     `to_lowercase`, `to_uppercase`, `to_string` and `byte_len` of text,
     besides `reflect`, `enumerate` and `len`; `E0609`'s reason lists them.
     Stage 1 decides it (RULES.md, R166).

963. `graphemes()` keeps R162's rules: UAX #29's `Prepend`, conjunct
     (GB9c) and pictograph (GB11) rules need Unicode's
     `GraphemeBreakProperty.txt`, `DerivedCoreProperties.txt` and
     `emoji-data.txt`, which Python's `unicodedata` does not carry; they
     wait for those files. Stage 1 decides it (RULES.md, R167).

964. The conformance suite's `std/slices` differs from stage 0's front
     end, which has no `std.mem`, as `unsafe/cstring` does (R149). Stage 1
     decides it (RULES.md, R168).

**Stage 0's nineteenth round: a byte laid out as a byte, narrow fields at
C's offsets, `sisu eval`'s usage and cases, and what was left** (sections
"Stage 2: the native backend", "Values", "Raw memory, foreign calls and
the machine intrinsics", "The checked memory model", "The language
server"; D-085, D-086; 08, 10, 12, 22; decisions 188, 823, 864, 940, 941,
980 to 986; stage 1's R164, R165, R169 and R170)

1020. A sized integer narrower than 64 bits and an `f32` are their own
      width natively: a `u8` and an `i8` one byte, a `u16` and an `i16`
      two, a `u32`, an `i32` and an `f32` four (decision 982's order, done
      whole). Both lowerings' layout tables give each its size and
      alignment (`layout.rs`, `layout.sisu`), and a part of a tuple, an
      option, a result, an enum, a mutex, a cell or a closure's
      environment takes its size rounded up to a word (`room`), so every
      other type keeps its size and its parts' offsets; a list, an array
      and a slice step by the element's own size, so a `list[u8]`, a
      `Slice[u8]` and `bytes` hold a byte to an element. Every read of a
      narrow number, from a place, through a reference, from a buffer,
      loads its width and extends it by its type's sign (`load_bits`,
      `read_value`, `load_scalar`), every write stores its width
      (`store_bits`), a copy of a value that is not a whole number of words
      copies four, two and one bytes after its words, an enum's inline
      `==` compares a narrow field at its width, and a narrow local is not
      one of the release profile's variables (decision 246), whose slot a
      call writes at its width; both lowerings write the same LIR, so the
      SIR and LIR paths give the same bytes. The runtime in Rust reads an
      intrinsic's narrow argument and writes its narrow result at its width
      (`value::scalar`, `set_scalar`), and the runtime in Sisu reads one in
      its walks of equality, order, the hash and `Debug` (`value.int_at`,
      by the sign the table's header gives a sized integer), its sums and
      its `f32` items; a table's key and value, and a pair's parts in
      `zip` and `enumerate`, each take a word at least, a set's owned
      items and a text's bytes their own size. `bytes.as_slice` views the
      bytes in place, a capacity of 0 as decision 940's views of a list,
      `bytes.to_list` copies them, and `mem.slice.as_ptr` of a `u8`
      compiles, so `CString.as_ptr()` is valid across a foreign call
      natively (`unsafe/cstring` in `native.list`, decision 823). The debug
      information describes each at its width. The checked memory model of
      the interpreter is unchanged: its raw layouts were these already
      (decision 302), and the debug profile's checks of raw memory take
      any bits of a narrow number's width as one of its values. `std/bytes` (a `list[u8]` of a
      twenty thousand bytes, its pushes, pops, inserts, slices, sort, sets and
      dicts, a `Slice[u8]` of a list and of `bytes`, boxes, options, enums
      and closures of `u8`s) and `std/narrow` (lists, structs, payloads,
      keys and tuples of `i8`, `u16`, `i32` and `f32`, their signs and
      overflows) hold the interpreter's output in every native mode. A
      `list[u8]` of fifty million bytes runs in 51 856 kB at its peak where
      a list of a word an element (a `list[int]` of as many, the layout a
      `list[u8]` had) takes 393 784 kB (the release profile; a million's
      peak, 11 380 kB, is the process's own); the runtime in Sisu's seed
      is 782 919 bytes where it was 777 695, its object as the fixed point
      builds it 1 559 880 where it was 1 546 336, and the fixed point
      holds (sisu2-A and sisu2-B the same
      31 144 408 bytes, where they were 31 129 464, stage1-A and stage1-B
      65 449 088, where they were 65 538 936). This changes no rule of the
      language.

1021. A declared struct lays its fields out at C's offsets natively, each
      at the next offset its alignment allows (a narrow number's width, a
      struct's widest field's, a word for any other type, 1 for a type of
      no bytes), its size a multiple of its widest field's, in both
      lowerings and the type table, where each field took a word at least;
      a struct of words keeps its layout. A struct is then laid out in a
      value as raw memory lays it out (`rawmem.rs` finds each part at its
      value's offset), so `ptr.of`, `ptr.of_mut` and `as_ref` of a
      `@repr(c)` struct with `u8` or `i32` fields compile, where they were
      `D0103` (decision 864): `unsafe/narrow_pointer` hands its own export
      a pointer to such a struct, writes through `ptr.of_mut` and reads
      `Layout.of`'s 12 bytes aligned to 4, in `native.list`. A tuple keeps
      a word for each part, since the runtime's iterators lay their pairs
      and an entry's key and value out so, and a pointer to a tuple with a
      narrow number among its parts, or to an option of a raw pointer,
      stays `D0103`. This changes no rule of the language.

1022. The usage (`D0002`) names `sisu eval FILE [--to toml|yaml]`, and its
      paragraph on the front ends names `eval` among stage 1's commands
      (decision 980's note), now that the main tree's stage 1 is 0.14.0
      (stage 1's R165). The conformance runner has a case kind for it:
      `NAME.eval` holds, for each line `# eval ARGS` of the case (one run
      without arguments when there is none), a line `--- sisu eval NAME.sisu
      ARGS`, the run's standard output, `--- stderr`, its standard error
      and `--- status N`; `eval/config` writes a module of a struct, a list,
      a dict, an empty list and a `None` as JSON, TOML (which leaves the
      `None` out) and YAML, and `eval/stops` a constant whose evaluation
      panics, `E0602` at its name and status 1 in two forms. The Rust front
      end, the oracle, has no `eval`, and its run of the suite fails the two,
      as it is allowed to (decision 268). This changes no rule of the
      language.

1023. The language server's answers from stage 1's queries (decision 983)
      are held served and as a process by a driver test rather than by a
      case of the suite: the runner drives `sisu`'s commands, and the
      queries are reached through `sisu lsp`'s protocol, which a case's
      files do not hold. `stage_1s_queries_answer_alike_served_and_as_a_process`
      opens a document with an error, asks for a definition, a hover, the
      symbols, a function's references and the members after a `.`, once
      with `SISU_STAGE1_SERVE=1` and once with `0`, and holds the JSON
      messages the same, the hover naming the function; with the main
      tree's stage 1 at 0.14.0, `stage_1s_queries_answer_as_the_rust_front_end`
      now runs stage 1's queries without a `SISU_STAGE1`. This changes no
      rule of the language.

1024. `sisu lsp`'s selection ranges, rename, code actions, inlay hints and
      formatting stay the Rust front end's this round: stage 1's twentieth
      round gives `query rename`, `actions`, `inlay`, `selection`, `format`
      and `holes` at 0.15.0 (its R169, decision 1000), but in its own
      worktree, where no 0.15.0 executable was built that this round could
      hold the driver's answers to, and the main tree's stage 1 is 0.14.0.
      The round that has it turns each request into its query from 0.15.0
      on, as decision 983 did (`query.rs`): a rename's lines into one
      workspace edit and `refused` into `LSP_NOT_A_NAME` and
      `LSP_CANNOT_RENAME`, the actions' `fix` and `edit` lines into code
      actions at the start of the protocol's range, the inlay hints kept in
      the editor's range, `format`'s runs of lines into edits (`refused`
      into the messages of `D0075` and `D0076`), and `prepareRename` from
      `references`; then only the worksheets' values and
      `sisu.explainBorrow` stay on the Rust front end. This changes no rule
      of the language.

1025. `sisu run`'s SIR stays asked for as a process (decision 981): stage
      1's twentieth round serves `build --for-run --emit-sir` at 0.15.0
      (its R170, decision 1001), which writes the SIR the build hands its
      backend with the build's diagnostics in their order, but in its own
      worktree, as decision 1024 says. The round that has it sends that
      request through decision 980's server for `run` and `build
      --native` from 0.15.0 on, reads the SIR from the frame's standard
      output where it read the backend's file, answers an empty one as a
      SIR without an entry, and measures a two-line `sisu run` served and
      as a process. This changes no rule of the language.

1026. The formatting, characters', floats', texts' and bytes' families of
      the intrinsics (decisions 945 and 986) and tagged-pointer provenance
      (decision 904) are not this round's either: the round spent its time
      on 1020 and 1021, whose widths each family that moves to the runtime
      in Sisu now reads with `value.int_at`. This changes no rule of the
      language.

**Stage 1's twentieth round** (stage 1's RULES.md, R169 to R175; the language server's remaining queries and `check --holes` at 0.15.0, `build --for-run --emit-sir` served, `W0022` for member attributes, a slice of a slice, the evaluator's prelude intrinsics, the prelude's visible names and `E0278`)

1000. `sisu1 query` also answers `rename FILE LINE COL NEW` (an edit per
      reference, or `refused` with why), `actions` (each fix of the
      diagnostics at the position with its edits), `inlay` (the type of a
      local declared without one, an inferred error set, `mut` and `own`
      at arguments), `selection` (the syntax ranges around a position,
      innermost first), `format` (`fmt`'s change as runs of lines) and
      `holes`, and a name in an import line is a reference of what it
      imports, so that `sisu lsp` can answer every request from stage 1.
      `sisu1 --version` is 0.15.0. Stage 1 decides it (RULES.md, R169).

1001. `sisu1 build --for-run --emit-sir [PATH]` writes on standard output
      the SIR `build --native --for-run` would hand its backend, with that
      build's diagnostics, order and status, and `sir --repl-serve` serves
      it, so that the driver can serve `sisu run` (decision 981). Stage 1
      decides it (RULES.md, R170).

1002. A member's `attributes` (`f.attributes`, `v.attributes`) is code
      that reads attributes, as a call of `reflect.attributes_of` is, so
      `W0022` does not warn of the attributes of a program that reads them
      that way. Stage 1 decides it (RULES.md, R171).

1003. What a call's result borrows through a shared reference to a
      `Slice` is what the slice borrows, since a slice views its list in
      place (decision 940): a slice of a slice made in one expression
      borrows the list, not the temporary first slice, and is not
      `E0406` when used on a later line. Stage 1 decides it (RULES.md,
      R172).

1004. The compile-time evaluator runs the prelude's `min`, `max`, `abs`,
      `any`, `all`, `sorted`, `enumerate`, `zip`, `zip_shortest` and
      `reversed` (and `max_of`, `min_of` through a derived `Ord`), as the
      interpreter does, so a configuration's constants may call them.
      Stage 1 decides it (RULES.md, R173).

1005. `sisu1 check --holes FILE` lists each typed hole of the program
      with its position, the type its place needs, its function, the
      bindings in scope before it and those of its type (`D0170` to
      `D0174`), and `query holes` answers the same; 10's types,
      contracts, tests and fits to depth two wait. Stage 1 decides it
      (RULES.md, R174).

1006. The prelude marks `pub` the names 09's list and 11 give it, and a
      program's module sees only those: a name of the prelude's own
      helpers (`days_in_month`, `DrawState`, `max_of`, ...) is
      `E0278`, with the import of a std module that has it as the fix
      where one does, and completions offer the `pub` names. `Formatter`, `Spec` and `Hasher` stay visible: the traits
      take them. Stage 1 decides it (RULES.md, R175).
