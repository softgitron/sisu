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
written over it; and the tools of 10: the formatter (`sisu fmt`), hot
replacement in epochs (`sisu run --watch`), the REPL (`sisu repl`, with
`:back` and `:why`), worksheets (`sisu ws`) and the language server
(`sisu lsp`). Not here: from Phase 6, `RwLock`, `Atomic`,
`Condvar`, TLS and `sisu audit --caps`; the checked string DSLs, `@cfg`
and foreign functions (Phase 8). Their syntax parses, and using them is a
diagnostic (`E0900 stage0.unsupported`). Stage 2, a native
debug backend (`sisu build --native`) that compiles every program the
conformance suite runs, is described in its own section, "Stage 2".

## Crate map

One crate per pipeline stage (18, "Implementation language and
architecture"); data flows down the table.

| Crate | Lines | Role |
|---|---:|---|
| `sisu-diag` | 903 | Spans, the source map (1-based lines, columns in characters), the message catalog, diagnostics with labels, notes and fixes, the human and `--format=ids` renderers |
| `sisu-syntax` | 4112 | Token and node kinds, the lossless arena tree with stable node identities, and the AST built from the tree as a total function |
| `sisu-lexer` | 1258 | Scanning (every byte a token, interpolation split into tokens) and layout (Python's INDENT/DEDENT algorithm) |
| `sisu-parser` | 3552 | Error-tolerant recursive descent that always returns a tree |
| `sisu-fmt` | 1868 | The formatter: the lossless tree regrouped into logical lines, laid out by a Wadler-style group printer, with the comments and blank lines placed between them; the shape a formatted file must keep (`sisu parse --shape`) and the unused imports it removes |
| `sisu-resolve` | 11125 | Loads the Sisu-written std (`compiler/std/`), declares every item, resolves signatures to types, registers methods; traits, impls, coherence and the trait solver; derives (unfolding each trait's `comptime def derive`), delegation and the instances of templates; the generators and generated tests of property testing, the interceptors and fakes of the program's traits; the borrow rules about declarations |
| `sisu-typeck` | 11569 | The bidirectional checker, with method resolution through traits and `comptime` expressions; produces the typed tree (THIR) for every body, broken or not |
| `sisu-sir` | 3093 | SIR: the data model, the text writer and reader, the validator, the types of places, and the jump threading of the SIR that runs |
| `sisu-lower` | 5427 | THIR to SIR: control flow, scopes and their drops, unwind edges, closure conversion, trait calls and their static resolution, drop glue, `comptime` functions, the borrow facts of signatures |
| `sisu-borrowck` | 5476 | The ownership checker on SIR: initialisation and moves, drop elaboration, loans and the borrow rules, the task rules, escapes, must-consume, purity, `explain-borrow`, and the `Shared` escape analysis |
| `sisu-interp` | 13342 | The trampolined SIR interpreter, values, drops, dispatch of trait calls, unwinding, host intrinsics, the task scheduler (random, PCT, bounded exhaustive) and the `sim` profile's faults, capabilities and the host's objects, the capability log, hot replacement in epochs, the parallel test runner, the dynamic ownership monitor, and the compile-time evaluator |
| `sisu-rt` | 10668 | Stage 2's runtime, linked into every native executable: the shared ABI (the value layout, the type table's encoding, the entry points), panics and stack traces, allocation, the structural operations on values, iterators, dicts and sets, the intrinsics, the scheduler of tasks on pooled stacks, the library types, capabilities, and the host's files, network and processes |
| `sisu-codegen` | 3965 | Stage 2's debug backend: SIR to Cranelift IR, one instance per function and list of type arguments, with trait calls resolved at compile time; the object file and the link |
| `sisu-driver` | 10635 | The query layer and the `sisu` command line; manifests (a TOML reader of its own), the package layout and the lockfile; the counterexample mode, `audit --ownership`, `check --explain`, replay tokens, snapshot files and transcripts, the `sim` build check; `fmt`, the REPL and worksheets, `run --watch`'s rebuilds, and the language server with its JSON; `build --native` and `run --native`; the soundness tests (fault injection, mutation fuzzing), the language server's protocol tests and the native conformance test |
| `sisu-conformance` | 1337 | The language-agnostic conformance runner, and its native mode |

Line counts are of each crate's `src`, unit tests included. Besides the crates:
`catalog/messages.catalog` (every message); the standard library, in
Sisu: `std/prelude.sisu` (3919 lines: the builtin types, the core traits
and their derives, iterators, formatting, collections, the text methods,
`Shared` and the other library types, tasks and channels, time, the sized
integers and `bytes`, `Array`, the capabilities and their operations
traits, property testing), `std/cap.sisu` (961: std.cap's attenuations,
caretakers, fakes, interceptor and the `sim` profile's file system and
network), `std/collections.sisu` (308: std.collections), `std/num.sisu`
(211: std.num), `std/sim.sisu` (152: nodes, partitions, `choose`,
`always`), `std/cli.sisu` (175: std.x.cli over reflection),
`std/http.sisu` (299: std.x.http), `std/test.sisu` (snapshots,
benchmarks), `std/json.sisu` (`encode` over reflection), `std/live.sisu`
(hot replacement's boundaries), `std/task.sisu`, `std/log.sisu`,
`std/net.sisu` (`Url`), `std/time.sisu` and `std/math.sisu`; and
`conformance/` (the
suite, described in its own [README](../conformance/README.md)).

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
arguments meet every bound with a structural impl (lowering then calls
the intrinsic: `list.sort` and `join` are written in Sisu and run on the
host for builtin elements); `@keeps(self, x)` on an intrinsic names the
`own` and `mut` arguments whose every value it keeps or returns (the only
ones that may take a must-consume value); `@sealed` on a trait means only
std implements it (`Num`); `@marker` on a trait means it has no methods
and is implemented by the language's rules (`Copy`); `@internal` on a
function means only std may call it (`E0701`). All seven are reserved
for std (`E0225`). A std module may name prelude items too
(`std_sources::reexports`), so `from std.time import DateTime` finds the
prelude's. The prelude declares the core traits (`Display`,
`Debug`, `Eq`, `PartialOrd`, `Ord`, `Hash`, `Len`, `Copy`, `Num`, `Drop`,
`Iterator`, the operator traits, `FromLiteral`), their impls for the
builtin types, and the generic code over them; the impls of the leaf
types and of tuples are generated (`<std>/builtin_impls.sisu`, from
`sisu-resolve/src/std_sources.rs`).

`Iterable[T]`, a parameter type only, accepts a `list`, `set`, `dict` or
`Iter` whose items are `T`; a borrowed collection's items are references,
and an item `ref T` with a `Copy` type `T` decays to `T`.

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
`[features]`;
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
`PartialOrd`, `Ord` and `Hash` are written that way in the prelude, and
any trait may declare one. The derived impl of a generic type bounds each
parameter that a field uses by the trait (`impl[T: Eq] Eq for P[T]`), and
the fields of a type deriving one of the language's traits must
implement it (`E0244`). `@delegate(Trait, ..., to="field")` writes impls
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
for taking it as a function value). `const X: T = comptime f()` is a
constant computed by the compiler; a constant is computed even when
nothing uses it.

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
field), `index` and `named`; a variant's `name`, `index` and `fields`;
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
field's `ty` and `default` (whether it has one); `comptime panic(msg)`
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

Outside derives and templates (a method, a non-generic function),
`reflect`, `x.[f]`, `comptime for` and `return impl` are `E0900`.

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
both are compile-time code over the trait's methods (`reflect.methods_of`,
which waits for Q-27 to say how compile-time code writes a signature).

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
- What a `Shared`, `Cell`, `Mutex`, `Cap`, capability handle or channel
  owns can be neither a view (`E0421`) nor must-consume (`E0422`, which
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
test "counts case-insensitively" test#0

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
  instead, and no loan.
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
references, and `Moved`. A `Slice` is a `list` of its values (a copy,
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
locking a poisoned one (`R0074`). An `Arena[T]` holds
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
cancellation woke the tasks that wait. One dropped before its end is
closed, as Python's `close` closes one (decision 236): suspended at a
`yield`, it unwinds from there at once, outside the schedule, running its
frames' drops and `defer`s before the consumer goes on; one that never
started drops what its closure captured. A panic while it cleans up
aborts the program (`panic.during-abandon`), as a panic during unwinding
does, and so does an operation that would wait there
(`panic.abandoned-wait`). A receiver's or generator's iterator inside
another iterator's adapter cannot wait (`R0091`).

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
`Cell[T]` is `Send` when `T` is and never `Sync`; `Mutex[T]`, a channel
half and a task handle are both when `T` is `Send`; a mutex guard is
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
reaching. A `Lent` counts nothing. A function value's environment is
never dropped (decision 224), so a handle an `own` closure captures keeps
its object until the run ends. `capabilities/object_drops` and
`ownership/handle_borrows` pin the two halves.

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

**`sisu check --explain FILE`** (`sisu-driver/src/doc.rs`) prints the
file's declarations as `sisu doc` renders them: each signature joined
onto one line, with its contracts under it, a type's invariants, a
refinement's predicate and the `try_from` that checks it, and the error
set the compiler inferred for a `T !` function (`... ! {A, B}  #
inferred`). The conformance suite compares it (`NAME.doc`).

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
hanging, deterministically.

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
`--no-monitor`. Each test runs in an interpreter of its own; those that
take no `System` run at the same time on up to `--jobs N` threads (8 by
default, `sisu-interp/src/parallel.rs`), and those that take the real
`System` one at a time after them. Each is seeded as when they ran one
after another, nothing but the host's clock and output is shared, and
each test's output is held and written in declaration order, so a run
prints the same at any number of threads (the conformance runner checks
`--jobs 1` against every `.test`).

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
  written, ` (exit N)` on the last line of a run that fails), against a
  fake `System` that persists between the runs of one file: an in-memory
  `Fs` with the files of `tests/NAME/`, a clock frozen at the epoch,
  `Rand.seeded(0)` and no network (`transcript_world` and
  `transcript_system` in the prelude, which lowering keeps for this run
  only). A difference fails with a diff (`D0061`); `--update` rewrites
  the file.
- **Benchmarks**: `bench "name"(b: Bench):` is checked like a test and
  run by `sisu bench FILE [--iterations N]`, which reports the time of an
  iteration of `b.loop()` (the interpreter's: compare benchmarks with
  each other); `black_box(x)` consumes a value the optimiser must not see
  through.
- **Fakes and interceptors** of the program's traits: `cap.Fake[Trait]`
  and `cap.Wrap`, above ("The interceptors and fakes of a program's
  traits").
- **Replay tokens**: every failing test prints `replay: sisu test
  --replay TOKEN FILE` (below), or says it has none when it takes the
  real `System` (`D0031`).

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
content hash of the file, the toolchain, the test's index, the scheduler's
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
[--faults P]`). A test that declares a `System` gets `System.sim(seed)`: a
crash-consistent file system (`cap.SimFs`: writes are pending until
`sync`, a crash keeps what was synced), an in-process network
(`cap.SimNetwork`), the run's test clock, `Rand.seeded`, and fakes of the
rest. `std.sim` has `Sim` (nodes, each with its own `SimFs`, on one
network: `crash`, `restart`, `partition`, `heal`), `choose(n)` (a choice
the exhaustive scheduler enumerates) and `always(what, holds, body)` (an
invariant checked at every switch between tasks, `R0105`). With
`--faults P` (0.01 by default) a call of a simulation object's operations
fails at that rate with an error from its declared set, never one it
cannot return. A `sim` build fails to compile if reachable code calls an
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
written `T?` where that reads unambiguously, `Result[T, E]` written `T !
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
prints the shape, so the conformance suite checks any implementation's
formatter: every case that parses keeps its shape and formats to a fixed
point (`fmt(fmt(x)) == fmt(x)`), and every case without a
`NAME.unformatted` must already be canonical. CI runs `sisu fmt --check`
over `compiler/std/` and those cases.

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
                             `--sir` runs a SIR text file,
                             `--monitor` runs under the ownership monitor,
                             `--defer-borrow` runs a rejected program in the counterexample mode,
                             `--contracts=off|preconditions|all` what it checks (07),
                             `--record LOG [--redact fs,net,...]` writes the capability log,
                             `--stats` the scheduler's numbers, `--stats stack-depth` the
                             frame depths, `--stats shared` each `Shared` construction site,
                             `--native [--release]` builds it natively and runs that
sisu replay LOG              run a recorded program again against its log (`--monitor`)
sisu check [FILE]            diagnostics; `--format=ids` for the conformance form,
                             `--explain` the signatures with their contracts,
                             `--diary LOG` appends them to the friction log
sisu build [FILE]            check and lower a package (or a file), and write its lockfile;
                             `--native [--release] [--output PATH]` the executable, a
                             package's in `build/debug/` or `build/release/`
sisu sir [FILE]              the serialised SIR, drops placed and `comptime` values baked
                             (also for a program the ownership checker rejects)
sisu test [FILE]             run the tests (and `tests/*.transcript`) under the monitor: ok,
                             FAILED, or blocked by a hole; `--no-monitor`, `--defer-borrow`,
                             `--jobs N`, `--filter TEXT`, `--tag T`, `--skip-tag T`,
                             `--update` (snapshots and transcripts), `--replay TOKEN`,
                             `--sim [--faults P]`, `--std` (the standard library's doc examples)
sisu bench [FILE]            run the benchmarks; `--iterations N`
                             (`run` and `test` also take `--schedule-seed N`, `--explore N`,
                             `--scheduler random|pct|exhaustive` and `--depth N`; `run`,
                             `check`, `build`, `test` and `bench` take `--locked`)
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
```

Without `FILE`, a command works on the package of the current
directory or the nearest above it (section "Modules and packages"); in a
workspace, `check`, `build` and `test` work on every member, and `run`,
`sir` and `bench` on the one application among them (`D0067`
otherwise).

`sisu audit --ownership` is the corpus study's count (18, Phase 3; T1.1
and T1.2): for each file, one line of data, the file's name and then
`lines=N Shared=N Weak=N Cell=N Arena=N Key=N copy=N unsafe=N`. The
counts are of tokens: those names where they mean std's types (not after
a `.`, and not in a file that declares the name itself), calls of
`.copy()`, and the keyword `unsafe`; `lines` counts the lines on which a
token other than trivia and layout starts. A file need not check to be
counted.

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
PATH`. The executable goes beside a file's source, named by its stem
(`hello.sisu` makes `hello`), or, for a package, to `build/PROFILE/NAME`
in the directory of its lockfile, its workspace's or its own
(`stage1/build/debug/stage1`; decision 231); `--output PATH` puts it
elsewhere. `sisu build` without `--native` checks and lowers, as before,
and writes nothing but the lockfile. The profile (09, "Build") is `debug`,
or `release` with `--release` (decision 232): Cranelift's `speed`
optimisations, the checks of overflow and bounds as in every build, and
the contracts the root package's `[profile.release] contracts` names,
preconditions by default (07); `--contracts=LEVEL` overrides it, in
either profile. `sisu run --native [FILE] [ARGS...]` builds a package
where its builds go and a file into a directory of its own under the
system's temporary directory, which it removes after, runs the executable
with the arguments, the standard streams and the environment of the
command, and exits with its status (decision 197). It takes
`--schedule-seed N`, `--explore N` and the scheduler's `--stats` (section
"Tasks" below); it does not combine with the options that only the
interpreter has (`--sir`, `--monitor`, `--defer-borrow`, `--stats
stack-depth`, `--stats shared`, `--watch`, `--record`, `--scheduler`;
`D0109`), and `--release` goes with `--native` only (`D0111`). A
construct the backend does not compile yet is `D0103`, naming it and the
function it is in; the program still runs without `--native`. `SISU_NATIVE_SURVEY=1`
lists every such construct of a program with the number of functions
that have it, and `SISU_NATIVE_TIMES=1` prints where a build's time went:
a line for each step of checking and lowering the program (`time: names
resolved 133.0 ms`), of generating its code, and the link.
`sisu build --native --from-sir FILE [--output PATH]` and `sisu run
--native --from-sir FILE [ARGS...]` compile a file of serialised SIR
instead, without the front end (section "From serialised SIR" below).

### Values

Every value is a whole number of 8-byte words, and every part of a value
starts on a word (decision 188). Each concrete type of the program has a
number and an entry in the type table, whose kinds (`abi::Kind`) say
where the parts are:

| Type | Words | Layout |
|---|---:|---|
| `()`, `Never`, a constant argument | 0 | |
| `int`, `u8` to `u64`, `i8` to `i32`, `DateTime` | 1 | the value; a sized integer is kept in its range by its operations, a `u64` is unsigned; a `DateTime` is nanoseconds since the epoch, shown as RFC 3339 text in UTC |
| `i128`, `u128` | 2 | the low word first |
| `f64`, `f32` | 1 | the bits; an `f32`'s in the low half |
| `bool`, `char` | 1 | 0 or 1; the code point |
| `str` | 2 | the address of the UTF-8 bytes, their length |
| `String`, `Path`, `bytes` | 3 | a heap buffer, the length and the capacity in bytes (decision 208) |
| `list[T]`, `Array[T, N]`, `Slice[T]` | 3 | a heap buffer of elements, the length and the capacity in elements |
| `ref T`, `ref mut T` | 1 | the address of the value |
| `dyn Trait` | 2 | the number of the type of the value it holds, and the address of a heap cell holding the value, which the `dyn` owns |
| `ref dyn Trait` | 2 | the same two words, borrowed: the address is of the value the `dyn` holds |
| `Box[T]` | 1 | the address of a heap cell holding the value |
| `T?` | 1 + T | a tag (0 `Some`, 1 `None`), then the payload |
| `Result[T, E]` | 1 + the larger | a tag (0 `Ok`, 1 `Err`), then the payload |
| a tuple or a struct | the sum | the parts in order |
| an enum | 1 + the largest variant | the tag (the variant's index), then the variant's fields |
| an `error` enum, a union of error sets | 3 + the largest variant | its own type's number, its return trace, the tag, then the fields |
| a function value | 1 + K | the code's address, then its closure's environment, K words, the largest an `own` closure of the program needs and at least one, or the address of a larger environment of references (decision 224) |
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
| `Mutex[T]` | 3 + T | whether it is locked, the task holding it, whether it is poisoned, then the value |
| `MutexGuard[T]` | 1 | the address of the value inside its mutex |
| `Shared[T]`, `Weak[T]` | 1 | the address of the value in a heap cell, after a header of four words: the owners, the weak handles, the task that made it, its flags (decision 207) |
| `Cell[T]` | T | the value |
| `Arena[T]` | 1 | the address of a runtime object holding the slots, each a generation, whether it holds a value, and the value |
| `ref Iterable[T]` | 2 | the number of the type of what the caller passed, and its address (decision 210) |

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
the result and the parameters.

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
| `==`, `<`, `in`, `+` of texts on other values | `sisu_rt_binary`: structural equality, order, membership and concatenation by the type table |
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
| `call intrinsic name(...)` | `sisu_rt_intrinsic` with the call's descriptor; the sized integers' operations through `sisu_rt_sized` and `sisu_rt_wide` (decision 193) |
| `call _n(...)` | an indirect call of the value's code with its environment first |
| `drop p [if flag]` | when the type needs a drop: the flag's test (clearing it), then `sisu_rt_drop`, which runs the type's drop glue or the structural drop |
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
and nothing needs freeing (a function value is `Copy` and SIR never drops
one, so a handle an `own` closure captures is counted until the run
ends, decision 237). Which size that is only compiling finds, so a
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
callee left of it, when the call returns or unwinds (decision 235).

### The runtime

The runtime's entry points (`abi::sym`) are the boundary:

| Entry point | What it does |
|---|---|
| `sisu_rt_start(argc, argv, entry, flags, ret_ty, types, types_len, fns, fns_len, unprobed) -> i32` | decodes the type table, keeps the function table, starts the scheduler (the probe reserve holds `unprobed`, the most a call may write below a probe limit before a probe sees it), runs `main` on its thread (with a `System` when `flags` says so) and gives the exit status |
| `sisu_rt_frame(name, len, file, len, line, col)` | adds a line to the stack trace of the panic in flight |
| `sisu_rt_abort_unwind()` | a panic reached a frame running cleanup: abort |
| `sisu_rt_panic_overflow`, `_index`, `_stack`, `_unreachable` | raise those panics |
| `sisu_rt_int_op(op, x, y, out) -> u8`, `sisu_rt_float_op(op, x, y) -> f64` | the arithmetic that is not one instruction |
| `sisu_rt_alloc(bytes) -> ptr` | heap memory for a list or a box |
| `sisu_rt_task_env(bytes, ty) -> ptr` | the environment of a task's closure (or a generator's), of type `ty`, which the runtime frees when the closure returns, and drops for a generator closed before it started (decision 236) |
| `sisu_rt_drop(value, ty) -> u8`, `sisu_rt_copy(dst, src, ty)` | the drop and `.copy()` of a value by its type |
| `sisu_rt_retain(value, ty)` | a copy of a value of type `ty` that holds capability handles was made: each handle's cell counts one more (decision 237) |
| `sisu_rt_binary(op, out, a, a_ty, b, b_ty) -> u8` | equality, order, `in` and concatenation of values that are not numbers |
| `sisu_rt_intrinsic(desc, ret, args) -> u8` | a call of an intrinsic |
| `sisu_rt_sized(kind, op, flags, x, y, out) -> u8`, `sisu_rt_wide(kind, op, x, x_ty, y, y_ty, out) -> u8` | an operation of a sized integer type, of one word or of 128 bits |
| `sisu_rt_table(ty, items, n, out) -> u8`, `sisu_rt_dict_slot(dict, key, key_ty, mode, out) -> u8` | a dict or set literal; a dict's value for a key |
| `sisu_rt_arena_slot(arena, key, mode, out) -> u8` | the address of the value an arena holds for a key; a stale key panics (`R0072`), but for a drop, which does nothing |
| `sisu_rt_shared_touch(value)` | a `Shared`'s cell reached through `*`: counts it for `--stats` when another task than its maker reaches it |
| `sisu_rt_free(ptr)` | frees a box's or a `dyn`'s cell whose value was moved out (decision 216) |
| `sisu_rt_trace_release(error)` | a field was moved out of an error value, which is never dropped whole: it lets go of its return trace (decision 215) |
| `sisu_rt_yield()` | a yield point found the budget spent: the scheduler may switch tasks |
| `SISU_RT_DEPTH` | the counter of nested calls |
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
`Drop` impl or fields that need dropping runs its glue (decision 192). An
intrinsic call passes a descriptor that the code generator writes once per
shape of call (the intrinsic's number, the types of the result, of each
argument and of each type argument) and the addresses of the arguments;
the runtime reads through references, so an intrinsic does not care
whether it got a value or a reference to one (decision 193). Standard
output is buffered and written before anything goes to standard error and
when the program ends, so the two interleave as the interpreter's do.

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
`SISU_RT_BUDGET` and calls `sisu_rt_yield` when it reaches zero; the
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

### Linking

`sisu-codegen` writes the object beside the output (`NAME.o`, removed
afterwards unless `SISU_NATIVE_KEEP=1`), and `cc` links it with the
runtime and the system libraries a Rust static library needs, using LLVM's
`lld` when `ld.lld` is on the search path: it links in a third of the time
of the system's `ld` (87 against 224 ms for a small program). The search
path is given to `cc` explicitly, since the conformance runner runs `sisu`
in an empty environment (decision 198). The runtime is the
`libsisu_rt.a` cargo built beside the `sisu` executable, or the newest
`deps/libsisu_rt-*.a` there, which a `cargo test` leaves without copying
it up; without one the build is `D0104` (decision 199). An executable is
mostly the runtime: 6 MB for a small program with the release runtime, 9
MB with the debug one.

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
  are left out.
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
the program's source gives. `--from-sir` goes with `--native` only
(`D0110`); the interpreter reads a SIR file with `sisu run --sir`
(decision 212).

This is the path of the self-hosted check of the bootstrap. Stage 1, the
front end written in Sisu, is built natively and prints the SIR of a
program; that text must be byte-identical to what stage 0's front end
prints for the same program (`sisu sir`), and `sisu build --native
--from-sir` turns it into the executable that building the program from
its source gives. Stage 1 then stands in for stage 0's front end: the same
SIR, the same native code. The runner's mode `sisu-conformance --native-from-sir
conformance/native.list` builds every case of the list that way (from the
case's own `.sir` where it has one, else from what `sisu sir` prints for
it) and compares its runs with the interpreter's expectations, and CI runs
it beside the native mode.

### Coverage

`conformance/native.list` names the cases the backend builds and runs:
159 of them, every case of the suite that runs a program (the others are
diagnostics, `sisu test` files, and the tools' cases: `--watch`, the
REPL, worksheets, tokens, SIR and documentation). Among them are every
case of `concurrency/` that runs (tasks, scopes, cancellation, channels,
`select`, races, generators and their closing, `log`, mutexes across
tasks, the deadlock report, a task's stack overflow), all 42 of `run/`
that have a `main` (`run/testing` is a test file without one), the 35 of
`ownership/` that run (with `Mutex`, `Shared`, `Weak`, `Cell` and
`Arena`), and the 10 of `capabilities/` (fakes, attenuations, caretakers,
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
serialised SIR and with the release profile (every contract checked,
decision 232), and CI runs all three on every push. `--native-survey LIST` tries every
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
ms; the rest is the front end, which `sisu run` spends as well.

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
    compile errors), 2 (usage), 70 (internal compiler error), 101
    (panic), 134 (abort, including an ownership violation under the
    monitor).
31. Sisu frames are limited to 100 000; beyond that is a panic, not a
    crash (decision 234: the stack overflow names the function, natively
    as in the interpreter).
32. A typed hole run by `sisu run` panics; in `sisu test` the test is
    reported as blocked, and blocked tests do not fail the run.

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
     interpreter keeps `sisu run --sir`. The runner's mode
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
     is `Copy` and SIR never drops one, so a heap environment could never
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
237. A capability's object, and a `Cap`'s value, goes with the last copy
     of its handle, in the interpreter as natively: each copy of a handle
     is counted, and the drop of the last runs the object's `Drop` and
     frees its cell, where the cell lived as long as the run and the
     `Drop` never ran (decision 98), so a program that made fakes or
     attenuations in a loop grew until it ended. Handles stay `Copy`
     (Q-51); a copy is counted where it is made (a variable, a field, an
     aggregate, a closure's environment, a function's argument, an
     argument an intrinsic keeps, `sisu_rt_retain` natively) and a move
     is not; a `Lent` counts nothing. This changes rules of the front end,
     which stage 1 transcribes. Lowering's `needs_drop` holds for a value
     whose type holds a handle or a `Cap` (`holds_capability`) and for one
     whose type is `Copy` only by a bound (a generic function's `T: Copy`
     may be a handle; the drop of an `int` does nothing); a temporary of
     such a type is moved into what takes it, an argument a callee borrows
     (not `own`) is a copy of a place, a temporary's being dropped with
     the statement's others, and a comparison or an interpolation borrows
     it. SIR's `drops_something` holds for the handles, `System` and
     `Cap`, so drop glue drops them. A parameter holding a handle counts as
     borrowed for the elision rules, only a `TestClock` lends nothing of
     its own (`is_run_handle`), and the ownership checker gives a call's
     result that may point into a handle's object (its `from`) a loan of
     the place the call copied the handle from, which a `return` may carry
     for a parameter in its `from`: `let v = Cap(x).get()` is `E0406` and
     replacing a handle while a borrow of its object is used is `E0402`
     (`ownership/handle_borrows`), where both ran with the object kept
     alive. `System.assemble` takes its handles `own`, and the
     interpreter's transcript runs give each run's `System` its own counts
     of the world's handles. A function value's environment is never
     dropped (decision 224), so a handle an `own` closure captures keeps
     its object until the run ends, when the runtime forgets the cells
     left (decision 223), a registry of live cells by address that a
     release finds in logarithmic time. `capabilities/object_drops` holds
     the two implementations to the same drops.
