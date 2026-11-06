# Grammar

This page gives Sisu's grammar: every form that the parser reads, rule
by rule, from a file down to a pattern. It says what is written, not
what it means; each section links to the page that gives the meaning.
The tokens that the rules are made of, and the layout that makes
`NEWLINE`, `INDENT` and `DEDENT`, are [R1](lexical-structure.md).

**See also:** the proposal's [02, "Grammar
notes"](../../proposal/02-syntax.md#grammar-notes); the compiler's
[architecture notes](../../compiler/docs/architecture.md), "Parsing and
the syntax tree", whose notation this page uses, and the parser of
stage 1 in [compiler/stage1/src/parser/](../../compiler/stage1/src/parser/),
which is the grammar's source; [R3](declarations.md) for declarations,
[R4](expressions.md) for expressions, [R5](statements.md) for
statements, [R6](patterns.md) for patterns and [R2](types.md) for
types; [R17, Diagnostics](diagnostics.md#parsing) for the codes, E0100 to
E0127.

Each section ends with a status, in the words that [R16](sisu-command.md#how-to-read-this-page)
explains. The syntax is decided: blocks by indentation and no
semicolons (D-020, D-027), and a parser that decides layout in the
lexer and never fails ([02](../../proposal/02-syntax.md#grammar-notes)).
Whether this page becomes part of a specification is open (Q-62).

Stage 1's parser keeps a grammar of its own in the `##!` comments of
its modules, each production beside the function that reads it, which
[`sisu grammar`](sisu-command.md#sisu-grammar) prints in one text.
This page agrees with that text, rule for rule, as `sisu grammar`
prints it from `sisu1 0.20.0`, and names some rules apart that the text
writes inline (`cases`, `borrowed`, `bounds`, `module_path`); it was
checked against the text by hand, since a page's rules are notation
that no command runs.

## The notation

- `rule := ...` defines a rule.
- `'def'` is a token written as it stands: a keyword, an operator or a
  piece of punctuation. A quoted word that is not one of the keywords of
  [R1](lexical-structure.md#keywords), such as `'pure'` or `'suite'`, is
  a contextual keyword: a name that is a keyword only where the rule
  puts it.
- `NEWLINE`, `INDENT`, `DEDENT` and `EOF` are the tokens of the layout,
  and `name`, `integer`, `float`, `char`, `string`, `raw_string` and
  `bytes` the tokens of names and literals ([Tokens](#tokens)).
- `a b` is `a` followed by `b`, `a | b` either, `(...)` groups, `x?` is
  an optional `x`, `x*` any number of `x` and `x+` at least one.
- `-- text` is a comment on a rule.

Two conventions keep the rules short. A list in brackets, of arguments,
parameters, elements or fields, may end with a comma: `f(a, b,)` is
`f(a, b)`, and the formatter breaks a list with a trailing comma onto
lines of its own. The rules leave the trailing comma out, except where
it changes the meaning: `(x,)` is a tuple of one. And a `:` before an
indented block is written `':' block` or `':' NEWLINE INDENT ... DEDENT`.

**Status.** In stage 0.

## Tokens

The parser reads the tokens of [R1](lexical-structure.md). The layout
gives three that no character spells: `NEWLINE` ends a logical line,
`INDENT` opens a block where a line is indented more than the line
before it, and `DEDENT` closes one. Inside brackets, and on a line that
starts with a binary operator, `.`, `?.` or `??`, a line break is no
`NEWLINE` ([R1](lexical-structure.md#lines-and-statements)). A string
is a sequence of tokens, so that its interpolations are expressions:

<!-- unchecked: grammar notation, which no command runs; the section's session shows the tokens -->
```text
string        := STRING_START (STRING_TEXT | interpolation)* STRING_END
interpolation := INTERP_START expr '='? (':' FORMAT_SPEC?)? INTERP_END
```

`STRING_START` and `STRING_END` are the quotes, `"` or `"""`, and
`INTERP_START` and `INTERP_END` the braces of an interpolation. A raw
string and a bytes literal are one token each. Comments, doc comments
and spaces are trivia, which the syntax tree keeps and the rules skip.
`sisu tokens` prints the tokens of a file, each with its kind: `IDENT`
for a name, `INT_NUMBER`, `FLOAT_NUMBER`, `CHAR_LIT`, `RAW_STRING` and
`BYTE_STRING` for literals, and a kind of its own for each keyword and
operator, such as `DEF_KW` and `PLUS`:

<!-- file: examples/reference-grammar/tokens.sisu -->
```sisu
# The lexer makes NEWLINE, INDENT and DEDENT from the layout.
def main():
    if True:
        print("hei {1 + 2}")
```

<!-- output: examples/reference-grammar/tokens.console -->
```console
$ sisu tokens tokens.sisu
DEF_KW 62..65 "def"
IDENT 66..70 "main"
L_PAREN 70..71 "("
R_PAREN 71..72 ")"
COLON 72..73 ":"
NEWLINE 73..74 "\n"
INDENT 78..78 ""
IF_KW 78..80 "if"
IDENT 81..85 "True"
COLON 85..86 ":"
NEWLINE 86..87 "\n"
INDENT 95..95 ""
IDENT 95..100 "print"
L_PAREN 100..101 "("
STRING_START 101..102 "\""
STRING_TEXT 102..106 "hei "
INTERP_START 106..107 "{"
INT_NUMBER 107..108 "1"
PLUS 109..110 "+"
INT_NUMBER 111..112 "2"
INTERP_END 112..113 "}"
STRING_END 113..114 "\""
R_PAREN 114..115 ")"
NEWLINE 115..116 "\n"
DEDENT 116..116 ""
DEDENT 116..116 ""
EOF 116..116 ""
```

**Status.** In stage 0 (D-027).

## Files and items

A file is a list of items; a statement at the top level of a file is
E0107 (`parse.top-level-statement`), except in a script
([R13](modules.md#scripts-and-inline-metadata)).

<!-- unchecked: grammar notation, which no command runs -->
```text
file        := item* EOF
item        := attribute* visibility? item_body
item_body   := fn_def | struct_def | enum_def | error_def | trait_def | impl_def
             | import | from_import | test_def | suite_def | bench_def
             | const_def | type_alias | extern_block
attribute   := '@' name ('.' name)* ('(' args? ')')? NEWLINE
visibility  := 'pub' ('(' 'package' ')')?
```

An attribute stands on a line of its own ([R14](attributes.md#writing-an-attribute)):
one on its declaration's line is E0131 (`parse.attribute-line`), and
one before a statement inside a block E0129
(`parse.attribute-on-statement`). Attributes and a visibility before a
line that holds no declaration are E0106 (`parse.expected-declaration`).
A name between the parentheses of `pub(...)` other than `package`,
Rust's `pub(crate)`, is E0130 (`parse.visibility-scope`).

**Status.** In stage 0.

## Functions and parameters

<!-- unchecked: grammar notation, which no command runs -->
```text
fn_def      := ('pure' | 'comptime')? 'unsafe'? 'def' name generic_params? params
               result? where_clause? (':' block | NEWLINE)
result      := '->' type ('from' borrowed (',' borrowed)*)?
borrowed    := name | 'self'
params      := '(' (param (',' param)*)? ')'
param       := ('mut' | 'own')? ( name ':' type ('=' expr)?
                                | 'self' (':' type)?
                                | '*' (name ':' type)?
                                | '/' )
```

- A function without a body ends its line after its signature: a
  method that a trait requires, a function of an `extern` block, or an
  intrinsic of the standard library (E0224 elsewhere).
- `*` alone makes the parameters after it keyword-only, `/` makes those
  before it positional-only, and `*xs: T` collects the remaining
  arguments ([R3](declarations.md#parameters)).
- `from` names the parameters that a returned borrow comes from
  ([R7](ownership.md#returning-a-borrow)).

**Stage 0 differs.** `*xs: T` parses, and is E0900, whose message names
the disagreement of 02 and 14 that it waits for
([R3](declarations.md#parameters)).

**Status.** In stage 0, except variadic parameters.

## Generic parameters and where clauses

<!-- unchecked: grammar notation, which no command runs -->
```text
generic_params := '[' generic_param (',' generic_param)* ']'
generic_param  := 'const'? name (':' bounds)? ('=' type)?
bounds         := type ('+' type)*
where_clause   := 'where' type ':' bounds (',' type ':' bounds)*
generic_args   := '[' generic_arg (',' generic_arg)* ']'
generic_arg    := type | integer | expr
```

A generic argument is an expression, a constant argument such as
`Array[int, W * H]`, when it starts with what starts an expression and
no type (`-`, `~`, `comptime`, a literal other than an integer alone),
or when its leading name, path, integer or parenthesised group is
followed by an operator or by the `(` of a call; otherwise it is a type
([R2](types.md#constant-parameters)).

**Status.** In stage 0 (D-017, D-060).

## Structs, enums and error sets

<!-- unchecked: grammar notation, which no command runs -->
```text
struct_def     := 'view'? 'struct' name generic_params? ':'
                  NEWLINE INDENT struct_member+ DEDENT
struct_member  := attribute* visibility? (field | item_body | 'pass' NEWLINE)
                | contract_stmt                    -- `invariant e`
field          := name ':' type ('=' expr)? NEWLINE
enum_def       := 'enum' name generic_params? ':' NEWLINE INDENT variant_member+ DEDENT
error_def      := 'error' name generic_params? ':' NEWLINE INDENT variant_member+ DEDENT
variant_member := attribute* (variant | '..' NEWLINE | 'pass' NEWLINE | item_body)
variant        := name ('(' variant_field (',' variant_field)* ')')? NEWLINE
variant_field  := (name ':')? type
```

The items in a body are its methods and constants
([R3](declarations.md#structs)); `..` opens an error set to other
packages ([R3](declarations.md#error-sets)). Any other line in a struct's
body is E0126 (`parse.expected-member`), once for each body.

**Stage 0 differs.** [03](../../proposal/03-types-and-generics.md#algebraic-data-types)
lets an enum declare its variants' values, its discriminants, which
stage 0 does not lower yet: the parser reads `Red = 1` and reports E0900
([R2](types.md#structs-and-enums)).

**Status.** In stage 0, except discriminants.

## Traits, impls and foreign functions

<!-- unchecked: grammar notation, which no command runs -->
```text
trait_def     := 'unsafe'? 'trait' name generic_params? supertraits? ':'
                 NEWLINE INDENT member+ DEDENT
supertraits   := '(' type (',' type)* ')'
impl_def      := 'unsafe'? 'impl' generic_params? type ('for' type)? ':'
                 NEWLINE INDENT member+ DEDENT
extern_block  := 'extern' string? ':' NEWLINE INDENT member+ DEDENT
member        := attribute* visibility? ('pass' NEWLINE | item_body | trait_const)
trait_const   := 'const' name (':' type)? ('=' expr)? NEWLINE
```

A trait's parentheses list its supertraits. Its members are methods,
with or without a body, `type` declarations of associated types,
constants with or without a value (without a type, E0219), and a
`comptime def derive`; an impl's are methods and `type NAME = T`
([R3](declarations.md#traits), [R3](declarations.md#impls)). An
`extern` block's string names its ABI, as `"C"`, and its members are
functions without bodies ([R3](declarations.md#foreign-functions)).

**Status.** In stage 0; `extern` blocks are arriving (D-080).

## Imports

<!-- unchecked: grammar notation, which no command runs -->
```text
import       := 'import' (module_path | 'c' string) ('as' name)?
                ('with' name '=' expr (',' name '=' expr)*)? NEWLINE
from_import  := 'from' module_path 'import' (import_names | '(' import_names ')') NEWLINE
import_names := import_name (',' import_name)*
import_name  := name ('as' name)?
module_path  := name ('.' name)*
```

`from m import *` is E0100 (`parse.expected`), since there are no glob
imports ([R13](modules.md#imports)).

**Stage 0 differs.** `import c "zlib.h" as zlib with link="z"`, a C
header's import, parses and is E0900 (`stage0.unsupported`).

**Status.** In stage 0 (D-019), except C header imports.

## Tests, suites and benchmarks

<!-- unchecked: grammar notation, which no command runs -->
```text
test_def   := 'test' string cases? params? ':' block
bench_def  := 'bench' string cases? params? ':' block
cases      := 'for' patterns 'in' expr
suite_def  := 'suite' string ':' NEWLINE INDENT item+ DEDENT
```

`bench` and `suite` are keywords only before a string. A test's
parameters are the capabilities it receives
([R15](testing.md#the-forms-of-a-test)).

**Status.** In stage 0 (D-057).

## Constants and type declarations

<!-- unchecked: grammar notation, which no command runs -->
```text
const_def   := 'const' name (':' type)? '=' expr NEWLINE
type_alias  := 'type' name generic_params? ('=' type)? ('where' expr)? NEWLINE
```

A constant without its type is E0219 (`resolve.const-type-required`).
A `type` with `=` declares an alias, with `where` a refinement type, and
without `=` an associated type in a trait
([R3](declarations.md#type-declarations)).

**Status.** In stage 0.

## Blocks and statements

<!-- unchecked: grammar notation, which no command runs -->
```text
block        := NEWLINE INDENT stmt+ DEDENT
stmt         := let_stmt | return_stmt | if_stmt | while_stmt | for_stmt
              | 'break' NEWLINE | 'continue' NEWLINE | 'pass' NEWLINE
              | assert_stmt | expect_stmt | defer_stmt | with_stmt | select_stmt
              | yield_stmt | unsafe_block | comptime_stmt | contract_stmt
              | item | assign_stmt | expr_stmt
let_stmt     := ('let' | 'var') patterns (':' type)? ('=' exprs let_else?)? NEWLINE
let_else     := 'else' ('as' name)? ':' (block | 'return' exprs? | 'break' | 'continue' | expr)
return_stmt  := 'return' (exprs | impl_def)? NEWLINE
assign_stmt  := exprs assign_op exprs NEWLINE
assign_op    := '=' | '+=' | '-=' | '*=' | '/=' | '//=' | '%=' | '**='
              | '&=' | '|=' | '^=' | '<<=' | '>>='
              | '+%=' | '-%=' | '*%=' | '+|=' | '-|=' | '*|='
expr_stmt    := exprs NEWLINE
exprs        := expr (',' expr)*              -- a tuple without parentheses
```

- A block holds at least one statement; `pass` is the statement that
  does nothing. Nothing may follow the `:` that opens a block on its
  line (E0102, [R17](diagnostics.md#e0102)), and a block that is missing
  is E0101 (`parse.expected-block`).
- `let x: T` without a value declares a binding that is assigned later
  ([R3](declarations.md#a-binding-declared-before-its-value)).
- `return impl Trait for T:` is the body of a derive
  ([R12](compile-time.md#the-derive-protocol)).
- An assignment is a statement, never an expression: `=` in a condition
  is E0114 (`parse.assignment-in-expression`) and `:=` E0123
  (`parse.walrus`) ([R5](statements.md#assignment)).
- An item in a body is a declaration local to it, such as a nested
  `def`, which stage 0 reports as E0900.

**Status.** In stage 0, except declarations inside a function.

## Conditions and loops

<!-- unchecked: grammar notation, which no command runs -->
```text
if_stmt     := 'if' cond ':' block ('elif' cond ':' block)* ('else' ':' block)?
while_stmt  := 'while' cond ':' block
for_stmt    := 'for' patterns 'in' ('mut' | 'own')? expr ':' block
cond        := cond_part (',' cond_part)*
cond_part   := 'let' pattern '=' expr | expr
```

A condition's parts are tested in order, each `let` binding for the
parts after it (D-071, [R5](statements.md#conditions)). A comma at the
top level of a condition always separates its parts, so a tuple there
is written in parentheses: `if let (a, b) = pair, a < b:`. `else` after
a loop is E0127 (`parse.loop-else`).

An `if` at the start of a statement is the statement form when the
first `:` outside brackets ends its line; otherwise it is an `if`
expression, whose branches are expressions
([If and match expressions](#if-and-match-expressions)). So an `if`
whose branches fit on one line is an expression statement, and a
statement on the line of the `:` is E0102:

<!-- file: examples/reference-grammar/if_forms.sisu -->
```sisu
# An `if` whose `:` ends its line is a statement; one that goes on is an
# expression, and an expression has no statements in it.
def sign(n: int) -> str:
    if n > 0: return "plus"
    return "zero or minus"

def main():
    let n = 4
    if n > 3: print("big") else: print("small")
    print(sign(n))
```

<!-- output: examples/reference-grammar/if_forms.console -->
```console
$ sisu run if_forms.sisu
error[E0102]: a block starts on the next line, indented; nothing may follow `:` on the line that opens it
  --> if_forms.sisu:4:15
  |
4 |     if n > 0: return "plus"
  |               ^^^^^^

error[D0006]: the program was not run: errors found (1)
```

**Status.** In stage 0 (D-021, D-071).

## Other statements

<!-- unchecked: grammar notation, which no command runs -->
```text
assert_stmt   := 'assert' expr (',' expr)? NEWLINE
expect_stmt   := 'expect' expr (',' expr)? NEWLINE
debug_assert_stmt := 'debug_assert' expr (',' expr)? NEWLINE
defer_stmt    := 'defer' (':' block | stmt)
with_stmt     := 'with' expr ('as' name)? ':' block
select_stmt   := 'select' ':' NEWLINE INDENT select_arm+ DEDENT
select_arm    := 'case' pattern '=' expr ':' block
yield_stmt    := 'yield' expr? NEWLINE
unsafe_block  := 'unsafe' ':' block
comptime_stmt := 'comptime' stmt
contract_stmt := ('requires' | 'ensures' | 'invariant' | 'decreases') expr NEWLINE
```

- `expect` and `debug_assert` are statements when an expression
  follows them on their line, and names otherwise, as in
  `x.expect("...")`.
- `requires`, `ensures`, `invariant` and `decreases` start a contract
  when an expression follows; followed by `=`, `.`, `,`, `[`, `?.`, an
  augmented assignment or the end of the line, each is a name
  ([R11](contracts.md#where-clauses-stand)).
- `comptime` before a statement runs it at compile time: `comptime for`,
  `comptime if`, `comptime assert` and `comptime e`
  ([R12](compile-time.md#asking-for-compile-time-evaluation)).
- `with` without `as NAME` keeps its value unnamed to the end of the
  block ([R5](statements.md#with-and-defer)), and `select` is
  [R9](concurrency.md#waiting-for-the-first-of-several).

**Status.** In stage 0.

## Expressions and precedence

The rules for expressions go from the loosest binding to the tightest,
so each level's operands are the level after it; the precedence table
of [R4](expressions.md#precedence) says the same from the other end.

<!-- unchecked: grammar notation, which no command runs -->
```text
expr           := closure | coalesce
coalesce       := or_expr ('??' coalesce)?
or_expr        := and_expr ('or' and_expr)*        -- not mixed with `and`
and_expr       := not_expr ('and' not_expr)*
not_expr       := 'not' not_expr | compare
compare        := bitwise (compare_op bitwise)*    -- a chain points one way
compare_op     := '==' | '!=' | '<' | '<=' | '>' | '>=' | 'in' | 'not' 'in'
bitwise        := shift (('&' | '^' | '|') shift)*  -- one operator, no arithmetic
shift          := additive (('<<' | '>>') additive)*  -- no arithmetic operands
additive       := multiplicative (('+' | '-' | '+%' | '-%' | '+|' | '-|') multiplicative)*
multiplicative := unary (('*' | '/' | '//' | '%' | '*%' | '*|') unary)*
unary          := ('-' | '~' | '^' | 'ref' 'mut'? | 'comptime' | 'unsafe') unary | power
power          := postfix ('**' unary)?            -- `-x ** y` needs parentheses
```

`??` and `**` group to the right, and `**` takes a unary operand, so
`x ** -y` parses; comparisons chain ([R4](expressions.md#comparisons)),
and every other binary operator groups to the left. The comments
are the mixing rules, which the grammar would allow and the parser
reports: `and` with `or` (E0108, [R17](diagnostics.md#e0108)), two
bitwise operators (E0109), a bitwise operator with arithmetic or a
shift (E0110), a shift with arithmetic (E0111), `-x ** y` (E0112), a
chain that turns (E0113, [R17](diagnostics.md#e0113)) and an operator
that cannot chain, such as `in` (E0119); [R4](expressions.md#precedence)
shows them. `sisu parse --tree` prints the groups the parser made:

<!-- file: examples/reference-grammar/grouping.sisu -->
```sisu
# How the parser groups the operators of one expression.
def main():
    print(not 1 + 2 * 3 == 6)
```

<!-- output: examples/reference-grammar/grouping.console -->
```console
$ sisu parse --tree grouping.sisu
SOURCE_FILE 0..99
  FN_DEF 57..99
    DEF_KW 57..60 "def"
    IDENT 61..65 "main"
    PARAM_LIST 65..67
      L_PAREN 65..66 "("
      R_PAREN 66..67 ")"
    COLON 67..68 ":"
    BLOCK 68..99
      NEWLINE 68..69 "\n"
      INDENT 73..73 ""
      EXPR_STMT 73..99
        CALL_EXPR 73..98
          NAME_REF 73..78
            IDENT 73..78 "print"
          ARG_LIST 78..98
            L_PAREN 78..79 "("
            ARG 79..97
              PREFIX_EXPR 79..97
                NOT_KW 79..82 "not"
                COMPARE_EXPR 83..97
                  BIN_EXPR 83..92
                    LITERAL 83..84
                      INT_NUMBER 83..84 "1"
                    PLUS 85..86 "+"
                    BIN_EXPR 87..92
                      LITERAL 87..88
                        INT_NUMBER 87..88 "2"
                      STAR 89..90 "*"
                      LITERAL 91..92
                        INT_NUMBER 91..92 "3"
                  EQ_EQ 93..95 "=="
                  LITERAL 96..97
                    INT_NUMBER 96..97 "6"
            R_PAREN 97..98 ")"
        NEWLINE 98..99 "\n"
      DEDENT 99..99 ""
  EOF 99..99 ""
```

`not` holds the comparison, the comparison holds the sum, and the sum
holds the product.

**Status.** In stage 0 (D-022, D-025).

## Calls, indexing and fields

<!-- unchecked: grammar notation, which no command runs -->
```text
postfix     := primary (args | index | field | '?.' name | '?')* (args? do_block)?
args        := '(' (arg (',' arg)*)? ')'
arg         := (name '=')? 'mut'? expr
index       := '[' index_part (',' index_part)* ']'
index_part  := expr | expr? ':' expr?
field       := '.' (name | integer | '[' expr ']')
do_block    := 'do' 'own'? do_params? ':' block
do_params   := name (',' name)* | '(' (closure_param (',' closure_param)*)? ')'
```

- `.0` is a tuple's field, and `x.[f]` the field of `x` that the
  compile-time value `f` names ([R12](compile-time.md#reflection)).
- `x?` passes an error or `None` on, and `x?.f` reads a field of an
  optional value ([R4](expressions.md#optional-values-and-results)).
- A `do` block is a closure passed as a call's last argument; it ends
  the expression, so it comes last on its logical line
  ([R4](expressions.md#closures)).

**Status.** In stage 0 (D-026, D-042).

## Primary expressions

<!-- unchecked: grammar notation, which no command runs -->
```text
primary       := literal | string | name | 'self' | 'Self' | parens | list
               | dict_or_set | if_expr | match_expr     -- in an attribute, a keyword is a name
literal       := integer | float | char | raw_string | bytes
parens        := '(' ')' | '(' expr ')' | '(' expr ',' ')' | '(' expr (',' expr)+ ')'
               | '(' expr comp_clause+ ')'
list          := '[' (expr (',' expr)* | expr comp_clause+)? ']'
dict_or_set   := '{' (entry (',' entry)* | expr (',' expr)*
               | (entry | expr) comp_clause+)? '}'
entry         := expr ':' expr
comp_clause   := 'for' patterns 'in' or_expr | 'if' or_expr
```

`()` is the unit value, `(e)` is `e`, and `(e,)` a tuple of one. `{}` is
an empty `dict`; a `{...}` whose first element has a `:` is a `dict`,
and any other a `set` ([R1](lexical-structure.md#collection-literals)).
A comprehension's clauses take an `or_expr`, so a closure or `??` in one
needs parentheses ([R4](expressions.md#comprehensions)).

<!-- file: examples/reference-grammar/commas.sisu -->
```sisu
# A trailing comma ends a list, and makes a tuple of one.
def total(
    prices: list[int],
    tip: int,
) -> int:
    return sum(prices) + tip

def main():
    let one = (5,)
    let plain = (5)
    let prices = [
        350,
        280,
    ]
    print("{one} {plain} {total(prices, 20,)}")
```

<!-- output: examples/reference-grammar/commas.out -->
```text
(5,) 5 650
```

**Status.** In stage 0 (D-044).

## If and match expressions

<!-- unchecked: grammar notation, which no command runs -->
```text
if_expr     := 'if' expr ':' expr ('elif' expr ':' expr)* 'else' ':' expr
match_expr  := 'match' expr ':' NEWLINE INDENT arm+ DEDENT
arm         := 'case' pattern ('if' expr)? ':' (expr NEWLINE | block)
             | 'comptime' 'for' patterns 'in' expr ':' NEWLINE INDENT arm+ DEDENT
             | 'comptime' 'if' cond ':' NEWLINE INDENT arm+ DEDENT
               ('else' ':' NEWLINE INDENT arm+ DEDENT)?
```

- An `if` expression has its `else` (E0116, [R17](diagnostics.md#e0116)),
  and Python's `a if c else b` is E0115 ([R17](diagnostics.md#e0115)).
- A `match` that starts a statement is a statement, whose arms may hold
  blocks; a `match` inside an expression has one expression in each arm
  ([R4](expressions.md#if-and-match-as-expressions)). A statement on the
  line of a `case` is E0118 (`parse.arm-statement`).
- `comptime for` and `comptime if` among the arms make arms at compile
  time, in a derive's template ([R12](compile-time.md#reflection)).

**Status.** In stage 0 (D-056).

## Closures

<!-- unchecked: grammar notation, which no command runs -->
```text
closure         := 'own'? (name | '(' (closure_param (',' closure_param)*)? ')') '->' expr
closure_param   := name (':' type)?
```

`x -> e` and `(a, b) -> e` are closures: a name followed by `->`, or a
parenthesised list whose `)` is followed by `->`, starts one. Names
without parentheses, as in `do acc, x:`, are a `do` block's parameters
only ([R4](expressions.md#closures)). A parameter that is not a name is
E0122 (`parse.closure-parameter`).

**Status.** In stage 0 (D-026).

## Types

<!-- unchecked: grammar notation, which no command runs -->
```text
type          := '!' error_set? | option ('!' error_set?)?
error_set     := '{' option (',' option)* '}' | option
option        := ref_type '?'*
ref_type      := 'ref' 'mut'? primary_type | primary_type
primary_type  := path_type | fn_type | tuple_type | '(' type ')'
               | 'dyn' primary_type | 'ptr' 'mut' primary_type
               | 'impl' type ('for' type)? | '{' option (',' option)* '}'
path_type     := (name | 'Self' | 'type') ('.' name)* generic_args?
fn_type       := 'pure'? '(' (type_element (',' type_element)*)? ')' '->' type
tuple_type    := '(' ')' | '(' type_element ',' ')' | '(' type_element (',' type_element)+ ')'
type_element  := (name ':')? type
```

- `T ! E` is a result, `! E` one of nothing, and `T !` one whose error
  set is inferred; `{A, B}` is a union of error sets
  ([R2](types.md#results-and-error-sets)).
- `ref T?` is `(ref T)?`, an optional reference.
- `(a: int, b: int)` is a tuple with labels, the result of a function
  that returns two values ([R2](types.md#tuples)).
- `type` is a type in a `comptime def`'s parameter, `T: type`; `impl
  Trait for T` is the result of a derive; `ptr mut T` is a raw pointer
  of `std.mem` ([R7](ownership.md#unsafe-code-and-raw-pointers)).

**Status.** In stage 0 (D-002).

## Patterns

<!-- unchecked: grammar notation, which no command runs -->
```text
patterns         := pattern (',' pattern)*          -- a tuple without parentheses
pattern          := primary_pattern ('|' primary_pattern)*
primary_pattern  := '_' | name | constructor | literal | string | '-' (integer | float)
                  | '(' ')' | '(' pattern ')' | '(' pattern ',' ')'
                  | '(' pattern (',' pattern)+ ')' | '[' (pattern (',' pattern)*)? ']'
                  | '*' name | '..'
constructor      := (name | 'Self') ('.' name)* ('(' pattern_arg (',' pattern_arg)* ')')?
pattern_arg      := name '=' pattern | pattern
```

A pattern's names are decided by their spelling: `_` matches anything,
a name that starts with a capital letter, or that a `.` follows, or
`Self`, names a variant or a constant (a `constructor`), and any other
name binds ([R6](patterns.md#names-that-bind-and-names-that-name)). A
missing pattern is E0105 (`parse.expected-pattern`).

**Stage 0 differs.** Star unpacking in a binding, `let first, *rest =
xs`, parses, and is E0900, whose message names the list pattern of a
`match` ([R3](declarations.md#bindings)).

**Status.** In stage 0, except star unpacking in `let`.

## Diagnostics of this page

| Code | Identifier | Section |
|---|---|---|
| [E0107](diagnostics.md#e0107), [E0106](diagnostics.md#e0106), [E0129](diagnostics.md#e0129), [E0130](diagnostics.md#e0130), [E0131](diagnostics.md#e0131) | `parse.top-level-statement`, `parse.expected-declaration`, `parse.attribute-on-statement`, `parse.visibility-scope`, `parse.attribute-line` | [Files and items](#files-and-items) |
| E0224 | `resolve.missing-body` | [Functions and parameters](#functions-and-parameters) |
| E0126, [E0117](diagnostics.md#e0117) | `parse.expected-member`, `parse.trailing-tokens` | [Structs, enums and error sets](#structs-enums-and-error-sets) |
| [E0100](diagnostics.md#e0100), [E0900](diagnostics.md#e0900) | `parse.expected`, `stage0.unsupported` | [Imports](#imports) |
| E0219 | `resolve.const-type-required` | [Constants and type declarations](#constants-and-type-declarations) |
| [E0102](diagnostics.md#e0102), E0101, E0114, [E0123](diagnostics.md#e0123) | `parse.block-on-same-line`, `parse.expected-block`, `parse.assignment-in-expression`, `parse.walrus` | [Blocks and statements](#blocks-and-statements) |
| [E0127](diagnostics.md#e0127) | `parse.loop-else` | [Conditions and loops](#conditions-and-loops) |
| [E0108](diagnostics.md#e0108), E0109, [E0110](diagnostics.md#e0110), E0111, [E0112](diagnostics.md#e0112), [E0113](diagnostics.md#e0113), E0119 | `parse.mixed-and-or`, `parse.mixed-bitwise`, `parse.bitwise-arithmetic`, `parse.shift-arithmetic`, `parse.negative-power`, `parse.mixed-chain`, `parse.chain-operator` | [Expressions and precedence](#expressions-and-precedence) |
| [E0115](diagnostics.md#e0115), [E0116](diagnostics.md#e0116), E0118 | `parse.python-conditional`, `parse.if-expression-else`, `parse.arm-statement` | [If and match expressions](#if-and-match-expressions) |
| E0122 | `parse.closure-parameter` | [Closures](#closures) |
| [E0105](diagnostics.md#e0105) | `parse.expected-pattern` | [Patterns](#patterns) |
| [E0103](diagnostics.md#e0103), E0104, E0121, [E0124](diagnostics.md#e0124), E0125 | `parse.expected-expression`, `parse.expected-type`, `parse.empty-interpolation`, `parse.keyword-as-name`, `parse.unexpected-indent` | throughout |
