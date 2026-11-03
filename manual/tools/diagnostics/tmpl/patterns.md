# Patterns

A pattern describes the shape of a value: it tests the value and binds
names to its parts. This page lists every form of pattern, the places a
pattern stands, how a name binds, and the rules that a `match` covers
every value and that every arm can be reached.

**See also:** the tutorial's [chapter 4](../tutorial/04-control-flow.md)
(`match` on values) and [chapter 7](../tutorial/07-structs-and-enums.md)
(destructuring, exhaustiveness); the proposal's [02, "Control
flow"](../../proposal/02-syntax.md) and [03, "Sum
types"](../../proposal/03-types-and-generics.md); the decision D-043;
[R17, Diagnostics](diagnostics.md) for the codes.

Each section ends with a status, in the words that [R16](sisu-command.md#how-to-read-this-page)
explains.

## Where patterns stand

A pattern stands in each of these places:

| Place | Form | When the pattern does not match |
|---|---|---|
| A `match` arm | `case PATTERN:` or `case PATTERN if GUARD:` | the next arm is tried |
| `if let` | `if let PATTERN = VALUE:` | the `else` branch runs |
| `while let` | `while let PATTERN = VALUE:` | the loop ends |
| A chained condition | `if let P = A, let Q = B, TEST:` | the chain fails at that part |
| `let ... else` | `let PATTERN = VALUE else: ...` | the `else` block leaves the scope |
| A plain `let` or `var` | `let (a, b) = pair` | the pattern must match every value (E0329) |
| A `for` loop | `for (i, word) in enumerate(words):` | the pattern must match every element |

@@file other_places.sisu@@

@@output other_places.out@@

The parts of a chained condition run left to right; each binding is in
scope for the rest of the chain and in the branch it guards (leaning,
D-071).

**Status.** In stage 0.

## Literal patterns

An integer, a negative integer, a character, a string or a boolean
literal, and `None`, match a value equal to them. `_` matches any value
and binds nothing. `|` between patterns matches when any one does.

@@file literals.sisu@@

@@output literals.out@@

**Status.** In stage 0.

## Names that bind and names that name

A lowercase name in a pattern binds the value it stands for, and so
matches anything; a capitalised name is a variant or a constant, and
matches only that value. The naming rules make the two cases tell
themselves apart ([R1](lexical-structure.md#naming-rules)), where Python's
`case` cannot.

@@file bindings.sisu@@

@@output bindings.out@@

A capitalised name that the subject's type has no variant of is E0327
([R17](diagnostics.md#e0327)). A variant's own name is enough where it
is not ambiguous; where two enums share it, qualify it as `Light.Red`
(E0211).

**Status.** In stage 0.

## Variant patterns

A variant pattern names a variant and gives a pattern for each of its
fields: `Dot` for a unit variant, `Circle(r)` for a tuple variant, and
`Rect(w, h)` or `Rect(w=w, h=h)` for a record variant, by position or by
keyword. `Some(x)`, `None`, `Ok(v)` and `Err(e)` are variant patterns of
the prelude's enums.

@@file variants.sisu@@

@@output variants.out@@

**Status.** In stage 0.

## Struct patterns

A struct pattern is the struct's name with a pattern for its fields, by
position in declaration order or by keyword, as in Python. `..` at the
end stands for the fields not named; without it, a pattern names every
field, and one that leaves a field out is E0328
([R17](diagnostics.md#e0328)).

@@file structs.sisu@@

@@output structs.out@@

**Status.** In stage 0.

## Tuple and list patterns

A tuple pattern, `(x, y)`, has a pattern per element. A list pattern,
`[first, second]`, matches a list of exactly that length, and `*rest` in
it takes the elements that the other patterns leave, as a list, so
`[first, *rest]` matches any list with at least one element.

@@file sequences.sisu@@

@@output sequences.out@@

**Stage 0 differs.** `let first, *rest = xs`, star unpacking in a plain
`let`, is designed in [02](../../proposal/02-syntax.md) and stage 0 does
not have it; the list pattern `[first, *rest]` works in `match` and in
`let ... else`.

**Status.** In stage 0, except star unpacking in a plain `let`.

## Nested patterns and guards

Every part of a pattern is a pattern, so patterns nest: `Some(Rect(w,
h))`. A guard, `if CONDITION` after the pattern, adds a test that runs
after the pattern matched, with the pattern's names in scope; an arm
whose guard is `False` does not match, and the next arm is tried.

@@file nested.sisu@@

@@output nested.out@@

**Status.** In stage 0.

## Alternatives that bind

The alternatives of `|` may bind names, as long as every alternative
binds the same names with the same types; the arm then runs with
whichever alternative matched.

@@file or_bindings.sisu@@

@@output or_bindings.out@@

An alternative that leaves out a name another binds is E0385:

@@file or_unbound.sisu@@

@@output or_unbound.console@@

**Status.** In stage 0.

## A match covers every value

A `match` covers every value of its subject's type, so that a new
variant shows each `match` that must handle it (D-043). One that does
not is E0314, whose note names a case it misses and suggests an arm for
it or a `case _` arm.

@@file missing_case.sisu@@

@@output missing_case.console@@

**Stage 0 differs.** [03](../../proposal/03-types-and-generics.md) has
the error list the missing patterns; stage 0's E0314 names one at a time.
The proposal's linter also warns about `case _` on an enum of the same
package, since a new variant should then be a compile error; stage 0 has
no such warning.

**Status.** In stage 0 (D-043).

## Every arm can be reached

Arms are tried from the top, and the first that matches runs. An arm
whose values the arms above it all cover is W0003, a warning, and the
program runs:

@@file unreachable.sisu@@

@@output unreachable.console@@

**Status.** In stage 0.

## A plain let matches every value

The pattern of a plain `let`, `var` or `for` must match every value
of its type, as a tuple pattern or a struct pattern does. One that might
not, such as `Some(n)`, is E0329: add `else:` with a block that leaves
the scope, or use `if let` or `match`.

@@file refutable.sisu@@

@@output refutable.console@@

**Status.** In stage 0.

## Diagnostics of this page

| Code | Identifier | Section |
|---|---|---|
| [E0105](diagnostics.md#e0105) | `parse.expected-pattern` | [Literal patterns](#literal-patterns) |
| [E0211](diagnostics.md#e0211) | `resolve.ambiguous-variant` | [Names that bind and names that name](#names-that-bind-and-names-that-name) |
| [E0314](diagnostics.md#e0314) | `type.non-exhaustive` | [A match covers every value](#a-match-covers-every-value) |
| [E0328](diagnostics.md#e0328) | `type.pattern-field-count` | [Struct patterns](#struct-patterns) |
| [E0327](diagnostics.md#e0327) | `type.unknown-variant` | [Names that bind and names that name](#names-that-bind-and-names-that-name) |
| [E0329](diagnostics.md#e0329) | `type.refutable-let` | [A plain let matches every value](#a-plain-let-matches-every-value) |
| [E0385](diagnostics.md#e0385) | `type.bindings-in-or-pattern` | [Alternatives that bind](#alternatives-that-bind) |
| [W0003](diagnostics.md#w0003) | `type.unreachable-arm` | [Every arm can be reached](#every-arm-can-be-reached) |
