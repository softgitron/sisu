# Stage 0: the Rust implementation of Sisu

This is the reference for the stage-0 compiler and interpreter in
`compiler/`: what each crate does, the rules each stage follows, the
serialised forms other implementations must read and write, and the
decisions the proposal left open that the implementation had to make. It
is written so that the stage-1 front end in Sisu
([18](../../proposal/18-next-steps.md), "Bootstrapping") can be a
transcription: every rule here is a rule of the language implementation,
not of Rust.

Scope: [18](../../proposal/18-next-steps.md) Phases 0 and 2, the
non-generic, non-concurrent core. The ownership checker and dynamic
monitor (Phase 3), traits, user generics and the comptime evaluator
(Phase 4), tasks (Phase 6), and contracts and the rest of the
interpreter-era surface (Phase 8) are not here; their syntax
parses, and using them is a diagnostic (`E0900 stage0.unsupported`, or a
specific one such as `E0210` for user generics and `E0214` for trait
impls other than `Drop`).

## Crate map

One crate per pipeline stage (18, "Implementation language and
architecture"); data flows down the table.

| Crate | Lines | Role |
|---|---:|---|
| `sisu-diag` | 817 | Spans, the source map (1-based lines, columns in characters), the message catalog, diagnostics with labels, notes and fixes, the human and `--format=ids` renderers |
| `sisu-syntax` | 3405 | Token and node kinds, the lossless arena tree with stable node identities, and the AST built from the tree as a total function |
| `sisu-lexer` | 1179 | Scanning (every byte a token, interpolation split into tokens) and layout (Python's INDENT/DEDENT algorithm) |
| `sisu-parser` | 3232 | Error-tolerant recursive descent that always returns a tree |
| `sisu-resolve` | 1828 | Loads the Sisu-written std (`compiler/std/`), declares every item, resolves signatures to types, registers methods |
| `sisu-typeck` | 6343 | The bidirectional checker; produces the typed tree (THIR) for every body, broken or not |
| `sisu-sir` | 1725 | SIR: the data model, the text writer and reader, the validator |
| `sisu-lower` | 2713 | THIR to SIR: control flow, Drop elaboration, drop flags, unwind edges, closure conversion |
| `sisu-interp` | 4048 | The trampolined SIR interpreter, values, drop glue, unwinding, host intrinsics |
| `sisu-driver` | 738 | The query layer and the `sisu` command line |
| `sisu-conformance` | 429 | The language-agnostic conformance runner |

Line counts include each crate's unit tests. Besides the crates:
`catalog/messages.catalog` (every message), `std/prelude.sisu`,
`std/cli.sisu` and `std/math.sisu` (the standard library surface, in
Sisu), and `conformance/` (the suite, described in its own
[README](../conformance/README.md)).

The workspace forbids `unsafe` and denies `unwrap`, `expect`, `panic`,
`todo`, `unimplemented` and `unreachable` outside tests (18, Phase 0,
"Panic policy"); an internal error becomes a catalog message, never a
host panic.

## Pipeline

```text
source --lex--> tokens --parse--> lossless tree --ast::build--> AST
       --resolve--> Program (items, signatures, methods)
       --typeck--> THIR (typed bodies) + diagnostics
       --lower--> SIR --validate--> SIR --interp--> output, exit status
```

Every stage is total: it returns its output and its diagnostics for any
input, and a later stage runs on whatever an earlier one produced. Only
lowering requires a program without errors; the driver does not lower a
program that has any.

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
break and is reported from its opening quote.

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
generic_args := '[' (type | integer) (',' (type | integer))* ']'

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
expression, pattern, type or statement.

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
  nor `pass` is reported once per body (`E0126`).
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
parser, `E02xx` names, `E03xx` types, `E05xx` SIR, `E09xx` stage-0 limits
and internal errors, `W0xxx` warnings, `R0xxx` run time, `D0xxx` driver.

Code never contains English: `sisu-diag/build.rs` generates one `MsgId`
constant per entry (`lex.tab-in-indentation` becomes
`msg::LEX_TAB_IN_INDENTATION`), and a `MsgId` cannot be built from a
string outside that crate, so a message that is not in the catalog does
not compile. Severity comes from the catalog. Templates avoid number
agreement ("wrong number of arguments to `f`: expected 2, found 1") so
the catalog needs no plural rules yet.

A diagnostic is an identifier, named parameters, a primary span, labelled
secondary spans, notes and fixes (each fix a message and text edits). The
human renderer prints rustc-style excerpts; `--format=ids` prints the
conformance form, described in the suite's
[README](../conformance/README.md).

## Names and the standard library

The prelude and the std modules are Sisu source (`compiler/std/*.sisu`)
compiled by the same front end as user code. A function without a body
names a host intrinsic with `@intrinsic("name")`, and the `##` comment
above it is the intrinsic's contract, stated in Sisu terms; `@builtin`
marks a type whose values the host represents. Both attributes are
reserved for std (`E0225`). Generic parameters appear only in std until
Phase 4 (`E0210` for user declarations), with the bounds `Display`,
`Debug`, `Eq`, `Ord`, `Hash`, `Len` and `Num`, which the checker decides
from a table:

| Bound | Satisfied by |
|---|---|
| `Display`, `Debug`, `Eq` | every type except function types |
| `Len` | `str`, `String`, `list`, `dict`, `set` |
| `Num` | `int`, `f64` |
| `Ord` | `int`, `char`, `str`, `bool`, `()`, `String`, `Path`, `Ordering`; tuples, `list` and `Option` of `Ord` types (not `f64`, which has NaN) |
| `Hash` | `int`, `char`, `str`, `bool`, `()`, error sets; tuples and other types whose arguments are `Hash`; not `f64`, functions, `list`, `dict`, `set`, `Iter` or `Entry` |

`Iterable[T]`, a parameter type only, accepts a `list`, `set`, `dict` or
`Iter` whose items are `T`; a borrowed collection's items are references,
and an item `ref T` with a `Copy` type `T` decays to `T`.

Resolution (`sisu-resolve`) declares every item of the root module and of
each std module it imports (`import cli`, `import std.math`), resolves
every signature to a `Ty`, registers methods by receiver type and name,
and records every variant by its unqualified name. A name in an
expression is a variant of the expected type first, then a declaration of
that name, then a variant declared in the current module, then any
visible variant, where two candidates are ambiguous (`E0211`); a name in a
pattern is a variant of the scrutinee's type first. `impl Drop
for T` is the only trait impl (`E0214` otherwise), for a struct or enum of
the same module, with exactly `def drop(mut self)`.

## Type checking

`sisu-typeck` checks each body locally and bidirectionally against
explicit signatures, with inference by unification for locals, closure
parameters and the type arguments of std generics. It never fails: every
body yields a THIR body, with ill-typed parts typed `Error`.

- Integer literals are `int` and float literals `f64` unless the expected
  type is another numeric type (03, "Integer literals without context are
  `int`"); `half(3)` with an `f64` parameter is fine, and `a + b` with an
  `int` and an `f64` is `E0373`.
- A parameter is borrowed unless marked `own` or `mut`; as a local it is
  `ref T`, or `T` for a `Copy` type (04, rule 2). `Copy` types: the
  scalars, `str`, `()`, shared references, functions, tuples and `Option`
  of `Copy` types, error sets whose members are `@derive(Copy)`,
  `@derive(Copy)` structs and enums, and the capability handles.
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
- `pure` (D-074): a pure function takes no capability and calls only pure
  functions; a closure passed for a `pure` function type must be pure.
- `match` is exhaustive by Maranget's usefulness algorithm, with a
  witness of a missing case in the message and a warning for unreachable
  arms. Lists are split by length: with `L` one more than the longest
  fixed part of any list pattern in the column, the constructors are
  "exactly 0", ..., "exactly L-1" and "L or more".
- `todo(name)` is a typed hole (D-061): it checks as the type expected
  there and is reported as warning `W0001` with that type.
- Desugaring decided here: `it` closures, `do` blocks bound to the last
  function-typed parameter, `Type(...)` constructors with field defaults,
  parameter defaults, `range(n)` as `range(0, n)`, string literals as
  `String` or `Path` where expected, auto-deref, and implicit borrows of
  arguments, all made explicit in THIR (`Coerce`, `Borrow`, `Deref`,
  `DefaultArg` nodes).

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
Callee     := fn NAME | intrinsic NAME[TYPES] | Operand (a function value)
Terminator := Goto(bb) | Switch { value, cases: [(int, bb)], default: bb }
              | Return | Resume | Unreachable
Place      := local projection*      projection := .* | .N | .as(V) | [_i] | [^_i] | {_k}
Operand    := copy Place | move Place | const Const
Rvalue     := Use(Operand) | ref [mut] Place | BinOp(Operand, Operand) | UnOp(Operand)
              | tuple/struct/variant/list/set/dict(Operand...) | discriminant Place
              | closure NAME(Operand...) | copyof Place | strview Place
```

Every block may name an `unwind` block, the cleanup that runs when
anything in it panics; cleanup blocks are marked, cannot unwind, and end
in `resume` (or jump to another cleanup block). A panic is a call to a
`Never`-returning intrinsic followed by `unreachable`. Compared with the
statement list in 18 (pipeline step 5), Move, Copy and Borrow are
operand and rvalue forms of Assign; `StorageDead`, `Yield` and
`ContractCheck` arrive with the phases that need them (the borrow checker,
tasks, contracts).

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
enum Shape { Circle(r: f64), Dot }
error IoError open { NotFound(path: Path), Interrupted }

fn NAME(PARAM_COUNT) [closure|drop|value|test] source N {
    let _0: TYPE ["source name"]
  bb0:
    STATEMENT [@line:col]
    TERMINATOR [@line:col]
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
_1 = call fn parse_args(copy _2) | call intrinsic dict.new[String, int]() | call copy _3(copy _4)
drop _1 | drop _1 if _5
goto bb2 | switch copy _1 [0: bb3, 1: bb4, else: bb5] | return | resume | unreachable
```

Types: `int f64 bool char str () Never`, a named type with arguments
(`list[int]`, `Option[str]`, `cli.Error`), tuples `(int, str)` and
`(int,)`, error-set unions `{IoError, cli.Error}`, function types
`fn(int) -> bool`, and `ref T`, `ref mut T`. Constants: integers in
decimal, floats always with a `.` or an exponent (or `nan`, `inf`,
`-inf`), `true` and `false`, strings and characters with the escapes
`\n \t \r \0 \\ \" \'` and `\u{hex}`, `()`, and `fn NAME`.

Names: a function is its source name; a method is `Type.method`; a std
module function is `module.function`; a test is `test#N`; a closure is
`OWNER#closureN`; a field default is `Type#field` and a parameter default
`function#param`, each a `value` function of no parameters. Builtin types
(`String list dict set Iter Entry Path DateTime Fs Env Clock System Option
Result`) are not declared; every other type a function mentions is, with
`drop` naming its `Drop` function.

### Invariants

`sisu_sir::validate` checks them, and the driver runs it before every run:
every local, block and function named exists; unwind edges lead to cleanup
blocks; cleanup blocks do not unwind; `resume` only in cleanup; and the
lowering invariant: a statement that initialises a value needing drop, or
drops one, is the last statement of its block, so each block's unwind
edge is exact for every statement in it.

## Lowering

`sisu-lower` turns each THIR body into SIR functions (the body and its
closures). The rules, so that another front end produces the same SIR:

- **Scopes and drops.** Each block and each statement is a scope; a value
  that needs drop (not `Copy`, not a reference) is registered in its scope
  when it is initialised, and dropped when the scope ends, in reverse
  order of registration. `return`, `break`, `continue` and `?` drop every
  scope they leave, innermost first. Temporaries of a statement live until
  the end of the statement.
- **Unwind edges.** A block's unwind edge is computed when the block
  starts, from the values registered at that point: a chain of cleanup
  blocks dropping them innermost first and ending in `resume`. The chain
  for a given live set is built once and shared. Because initialising or
  dropping such a value ends the block, the edge is exact.
- **Drop flags.** A function is lowered twice. The first pass finds the
  locals moved as a whole somewhere; the second gives each of them a
  boolean flag, set when the local is initialised and cleared when it is
  moved, and their drops become `drop _n if _flag`. Other locals have no
  flag. A partly moved value (a payload moved out of a temporary) is
  dropped with the rest of its parts; dropping a place drops only what it
  still holds.
- **`defer`** registers its block as a scope entry; the block is emitted
  at every exit of the scope, on the normal and the cleanup paths, in
  order with the drops.
- **Control flow.** `if`/`elif` chains and conditions become `switch` on
  a `bool`; `while` is a head block with the condition and an exit; `for`
  calls `iter.ref`, `iter.mut` or `iter.own` (or uses an `Iter` directly)
  and then `Iter.next` each round, switching on the option. `and`/`or`
  short-circuit. A comparison chain evaluates each operand once.
- **`match`** tests each arm's pattern in order, binding nothing until it
  has matched, so a failed test leaves the scrutinee as it was; guards are
  tested after binding. A variant pattern on a union of error sets first
  calls `error.is[Decl]` to learn which declaration the value belongs to.
- **`?`** switches on the discriminant; the error path moves the payload
  out, calls `error.trace_push` with the location text `file:line:col`,
  stores `Err(e)` (or `None`) in `_0`, drops every scope and returns.
- **Closures** become a function whose parameter 1 is a tuple of
  references to the captured locals (`ref mut` when the closure assigns
  them; Phase 2 closures do not escape); the closure value is
  `closure NAME(refs...)`.
- **Interpolation** builds a `String` with `fmt.new`, then `fmt.text` and
  `fmt.display` or `fmt.debug` (with the format spec) per part.
- **Intrinsic calls** carry type arguments when the intrinsic needs them at
  run time: when a generic parameter appears in the result and in no
  parameter (`list.new[T]`, `cli.parse[T]`), or when a parameter is
  `Iterable[T]` (whose items are copies when `T` is not a reference).
- Only what the entry, the tests and the root module's functions reach is
  emitted, with the declarations of every type they mention.

### Lowering-only intrinsics

Besides the prelude's intrinsics, lowering introduces these; their
contracts:

| Intrinsic | Contract |
|---|---|
| `iter.ref(ref c)`, `iter.mut(ref mut c)` | an `Iter` over a borrowed `list`, `set` or `dict`: references to the elements, or `(ref K, ref V)` pairs for a dict, in order |
| `iter.own(c)` | an `Iter` that consumes `c`; a dict yields `(K, V)` pairs |
| `fmt.new()` | an empty `String` |
| `fmt.text(ref mut s, t: str)` | appends `t` |
| `fmt.display(ref mut s, v, spec: str)`, `fmt.debug(...)` | appends `v` rendered by `Display` (or `Debug`) and the format spec |
| `assert.fail(text, message)` | panics with `assertion failed: text` and the message |
| `assert.fail_compare(text, message, left, right)` | the same, with the `Debug` renderings of both sides |
| `error.trace_push(ref mut e, location: str)` | appends a location to the error's return trace |
| `error.trace(ref e) -> String` | the trace, one `at location` per line |
| `error.is[D](ref e) -> bool` | whether the error value belongs to declaration `D` |
| `list.slice(ref xs, lo, hi)` | a new list of copies of `xs[lo:hi]`, bounds clamped to the length; negative bounds panic |
| `list.slice_middle(ref xs, n, m)` | copies of the elements between the first `n` and the last `m` (a `*rest` binding) |

## The interpreter

`sisu-interp` runs SIR directly (18, "The interpreter").

**Frames and the run loop.** A frame holds its function, locals, current
block and statement index, its current source location, and, while it
runs cleanup, the panic it is unwinding from. The loop is a trampoline: a
call to a SIR function pushes a frame and the loop continues in it; a
return pops it and stores the result in the caller's destination. Sisu
calls never recurse on the host stack (a test runs 50 000 nested calls on
a small test thread); deeper than 100 000 frames is a stack-overflow
panic. The host recurses only when an intrinsic calls back into Sisu (a
`sort_by` comparator, `Iter.map` and the other adapters) and for drop glue
calling a type's `Drop`; those nest a run loop on the same frame stack.
The run happens on a thread with a 256 MiB stack so that nesting is safe.

**Values** are a tagged enum: `int`, `f64`, `bool`, `char`, `()`, `str`
(an immutable shared text), `String`, `Path`, `DateTime` (seconds since
the Unix epoch, UTC), tuples, structs, enum values (with an error return
trace when the enum is an error set), `list`, `dict` and `set`
(insertion-ordered, as D-074 and 11 require, with a hash index), function
values, closures (function and captured references), iterators, dict
entries, the capability handles, references, and `Moved`.

**References are places**: a frame index and serial number, a local, and
a path of steps (field, payload, list index, dict key or value position,
set element). Reading or writing walks the path from the frame's local; a
reference into a frame that has returned is detected by the serial number
and panics. Resolving a SIR place follows `.*` through references, so a
pointer never contains a dereference.

**Moves** take the value and leave `Moved`. Reading `Moved` panics (the
ownership checker of Phase 3 will reject such programs before they run);
dropping it does nothing, which is how a partly moved value drops only
what remains.

**Drop glue.** `drop` takes the value out of its place, then runs the
type's `Drop` with a reference to it (in a scratch frame), then drops its
parts: fields and tuple elements in reverse order (04, rule 6), collection
elements first to last, a dict entry's value before its key, an
iterator's remaining owned items, an entry's key. Because the place is
`Moved` from the moment its drop begins, a cleanup block reached from that
drop does not drop it again.

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
panic; 134 an abort (a panic while unwinding, or a run past its step
limit). The driver adds 2 for usage errors and 70 for an internal
compiler error.

**Formatting.** `Debug` renders a value the way it is written:
`"text"`, `'c'`, `[1, 2]`, `(1,)`, `{"k": 1}`, `set()`, `Point(x=1.0,
y=2.0)`, `BadDigit(at=1, found='x')`, `Some(3)`, `None`. `Display` is the
plain text of strings, characters and paths, decimal for integers, the
shortest text that reads back as the same value for floats (`3.0`, `0.1`,
`1e100`, `inf`, `NaN`), `True`/`False`, ISO 8601 for a `DateTime`, and
the `Debug` form for every other type until traits let a type define its
own. Format specs follow Python's mini-language,
`[[fill]align][sign][#][0][width][,|_][.precision][type]` with types
`b o x X d e E f F % s ?`; a precision without a type means that many
decimals; `?` asks for `Debug`. A `DateTime` takes a `strftime` pattern
(`%Y %y %m %d %H %M %S %F %T %%`) as its spec.

### The host contract

Everything a program does to the world goes through the `Host` trait:
standard output and error, the arguments after the program's name,
environment variables, the clock, and file reads, writes and existence
checks. There is no other I/O intrinsic. `System` is built only for a
`main(sys: System)`, from three handles, `Fs`, `Env` and `Clock`, whose
methods are intrinsics. `Fs.read_only` returns a handle whose writes fail
with `PermissionDenied`; `Fs.restrict_to(dir)` one whose paths are
relative to `dir` and may not be absolute or contain `..`;
`Fs.in_memory(files)` a fake held in memory and shared by every copy of
the handle (17, "Fixtures").

`StdHost` is the real process. `TestHost` is deterministic: captured
output, a fixed clock, given arguments and variables, and files in memory
over an optional read-only directory. The conformance runner uses the real
`sisu` executable in an empty environment and a fresh directory instead,
so that it can test any implementation.

## The query layer and the driver

`sisu_driver::Db` memoises `parsed`, `checked` and `sir` per file name on
a hash of the file's text, the skeleton of 18's query pipeline. It is an
implementation detail: a test checks that unchanged text reuses every
result and a change recomputes every step after it, and nothing depends on
whether a result was cached. Stage 0 has one user file per program plus
std, so a query's key is the file; per-item keys come with the language
server.

`sisu`:

```text
sisu run FILE [ARGS...]      check, lower and run; `--sir` runs a SIR text file
sisu check FILE              diagnostics; `--format=ids` for the conformance form
sisu sir FILE                the serialised SIR
sisu test FILE               run the `test` blocks: ok, FAILED, or blocked by a hole
sisu tokens FILE             the token stream; `--trivia` includes trivia
sisu parse FILE              `--tree` the syntax tree, `--print` the source again
sisu explain CODE            the catalog entry for a code or identifier
```

`run` prints warnings and runs; with any error it prints the diagnostics
and does not run. An internal compiler error (a host panic, which the
lints are meant to make impossible) is caught and reported as `D0012`
naming the stage, with the input written to the temporary directory, and
exits 70.

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
7. **Running**: in `sisu-interp`; a new intrinsic gets its contract as a
   `##` comment in `compiler/std/*.sisu` (or in the table above if only
   lowering uses it).
8. **Cases**: conformance cases under `conformance/cases/` for the
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
8. Or-patterns cannot bind names yet (`E0385`).
9. `print` and `print_err` take exactly one argument (the prelude signature
   `print(x)`); several values print as a tuple.
10. `f64` is not `Ord` (NaN), so `max(2.5, 1.5)` is an error and floats
    compare with `f64.cmp`, which panics on NaN.
11. The `IoError` variants are `NotFound(path)`, `PermissionDenied(path)`,
    `AlreadyExists(path)`, `InvalidData(path)` (not UTF-8),
    `Interrupted` and `Other(message: String)`.
12. `cli.parse[T]` (the stand-in for `std.x.cli`'s derive): each field
    `f` of type `int`, `f64`, `str`, `String`, `Path` or `bool` is the
    option `--f`, with `-` for `_`, given as `--f value` or `--f=value`; a
    `bool` is a flag; the one `list` field collects the positional
    arguments; `--` ends the options; a field with a default may be
    omitted. Its errors are `UnknownOption`, `MissingValue`,
    `InvalidValue`, `MissingArgument` and `UnexpectedArgument`, naming the
    option as written.
13. `()` is a pattern matching the unit value.
14. Inferred error sets are found by iterating every `T !` function to a
    fixpoint, at most 32 rounds.

**Semantics**

15. `d[k] = v` inserts a missing key; reading a missing key panics.
16. `xs[lo:hi]` copies, with the bounds clamped to the length; negative
    bounds panic (`^` counts from the end, so a negative index is never
    meaningful).
17. A shift amount outside 0..63 panics; bits shifted out of `<<` are lost
    rather than trapping, since shifts are bit operations, not arithmetic.
18. Float `/` by zero follows IEEE 754 (an infinity or NaN); `%` on floats
    has the sign of the divisor, like the integer `%`.
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
24. `Display` of a non-text type is its `Debug` form until traits.
25. Floats display as the shortest text that reads back as the same
    value, always with a `.` or an exponent.
26. In a format spec, a precision without a type is that many decimals
    (`{x:.2}` is `3.14`), as in Rust; Python's general format needs `g`,
    which stage 0 does not have.
27. Float `sum` adds pairwise by halves (the "fixed shape" of D-074).
28. `Option.expect` and `Result.expect` with an empty message panic with
    "called `expect` on `None`" (or "on an error" and the error); with a
    message, the message, the error and its trace.
29. A key function's result that refers to the item passed to it is
    copied before the item is released (`max_by(it)` on references).
30. Exit statuses: 0, 1 (an error returned by `main`, a failed test,
    compile errors), 2 (usage), 70 (internal compiler error), 101
    (panic), 134 (abort).
31. Sisu frames are limited to 100 000; beyond that is a panic, not a
    crash.
32. A typed hole run by `sisu run` panics; in `sisu test` the test is
    reported as blocked, and blocked tests do not fail the run.

**SIR**

33. SIR v0's statements are Assign, Call and Drop, and its terminators
    Goto, Switch, Return, Resume and Unreachable; the mapping to 18's list
    is in the SIR section.
34. Matching a union of error sets tests the declaration with
    `error.is[D]` before the variant index, so an error value needs to
    know its declaration at run time.
35. Error return trace locations are strings computed at lowering and
    embedded in SIR, which makes traces independent of debug information.
