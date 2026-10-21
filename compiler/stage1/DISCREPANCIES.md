# Where stage 1 and stage 0 differ

Stage 1 transcribes the Rust front end (`sisu-lexer`, `sisu-syntax`,
`sisu-parser`) rule for rule. Where the Rust behaviour is an accident
rather than a rule, stage 1 does what the documents say instead, and the
difference is listed here, with the file that shows it, both outputs, and
the documents that decide. The coordinator decides which side changes; until
then `check.sh` accepts exactly the differences listed under **Differs** and
fails on any other.

None of D1 to D3 occurs in the conformance suite, `compiler/std/` or
`proposal/examples/`: every one of those 370 files gives identical tokens,
trees and reprints. D4 is a difference of name resolution, the stage-1
resolver's, and shows in `proposal/examples/testing.sisu`. The other files
below are stage 1's own cases (`compiler/stage1/cases/`).

The **Differs** lines are read by `check.sh`: a path from the repository
root, then the outputs that differ (`tokens` is `sisu tokens --trivia` with
its diagnostics, `tree` is `sisu parse --tree` with its diagnostics,
`trivia-tree` is `sisu parse --tree --trivia`, and `names` is the name
diagnostics of `sisu check --format=ids`, those stage 1's resolver
reports).

## D1. A line break inside a one-line interpolation becomes an error token

- Differs: `compiler/stage1/cases/newline_in_interpolation.sisu`: tokens, tree, trivia-tree

```sisu
    print("a{x
    print("b")
```

The scanner reports `E0018 lex.newline-in-interpolation`, closes the
interpolation and the string with zero-width `INTERP_END` and `STRING_END`,
and returns without consuming the line break. `scan`'s guard against
stalling ("Never stall: consume one character as an error token") then
takes the line break as an `ERROR_TOKEN`, so the layout pass never sees a
line end there and the next line joins the broken one; the parser puts the
`ERROR_TOKEN` in an `ERROR` node. No diagnostic names the error token.

Stage 0 (`sisu tokens --trivia`, line 31 of 70):

```text
ERROR_TOKEN 175..176 "\n"
```

Stage 1:

```text
LINE_BREAK 175..176 "\n"
```

and in the tree, stage 0's `ERROR 175..222` (which holds the
`ERROR_TOKEN`) is stage 1's `ERROR 180..222`. The diagnostics are the
same in this file; when an open bracket is closed by the layout's recovery
on the line after, the two also report different `E0009`s, because they
see different lines. This is the only difference mutation finds: in
`check.sh --mutants 2000 --seed 7`, 25 of the 2000 mutants differ, all
this way, 3 of them in their diagnostics too (with `--mutants 400`, the
default seed, 5 of 400).

**The documents support stage 1.** The scanner's own comment is "Inside an
interpolation of a one-line string, a line break ends the string: report it
and fall back to code", and docs/architecture.md says "An unterminated string
ends at the line break"; an `ERROR_TOKEN` is otherwise the token of an
unknown character, reported as `E0005`. Stage 1 closes the interpolation and
the string and scans the line break as code. The fix in stage 0 is to fall
through to the `match c` in `Scanner::code` (`sisu-lexer/src/scan.rs`)
instead of returning.

## D2. `\u{+41}` is accepted as an escape

- Differs: `compiler/stage1/cases/unicode_escape_sign.sisu`: tokens, tree, trivia-tree

```sisu
    let a = '\u{+41}'
    let b = "x\u{+1F600}y"
```

`Scanner::escape` checks the digits of `\u{...}` with
`u32::from_str_radix(hex, 16)`, which accepts a leading `+`, so stage 0 takes
`'\u{+41}'` as `'A'` (and `sisu run` prints `A`). Stage 1 accepts one to six
hex digits only, so for stage 1 `\u{+41}` is an invalid escape, handled as
stage 0 handles any other invalid `\u{...}`: `E0007` from the backslash,
and the braces stay in the text (in a string, `{` then opens an
interpolation).

Stage 0: no diagnostics. Stage 1 (`sisu parse --tree`):

```text
4:13-4:22 E0016 lex.bad-char-literal
5:15-5:17 E0007 lex.invalid-escape escape="u"
```

**The documents support stage 1.** proposal/02-syntax.md writes the escape
as `'\u{1F600}'` and names no sign; a sign in a code point is a property of
Rust's integer parser, not of Sisu. The fix in stage 0 is to check that
`hex` is all ASCII hex digits before `from_str_radix`, in the lexer and
wherever the literal's value is decoded.

## D3. A nested struct resets "reported once per body"

- Differs: `compiler/stage1/cases/nested_struct_members.sisu`: tree

```sisu
struct A:
    print(1)
    struct B:
        x: int
    print(2)

struct C:
    struct D:
        print(4)
    print(5)
```

Stage 0 keeps one flag, `bad_member_reported`, for the whole parser, and
`struct_def` sets it to `false` at the start of every struct. A struct
declared inside another struct's body therefore resets the outer body's
flag, and the inner body's report sets it for the outer one.

Stage 0 (`sisu parse --tree`, diagnostics):

```text
5:5-5:10 E0126 parse.expected-member
8:5-8:10 E0126 parse.expected-member
13:9-13:14 E0126 parse.expected-member
```

Stage 1:

```text
5:5-5:10 E0126 parse.expected-member
13:9-13:14 E0126 parse.expected-member
14:5-14:10 E0126 parse.expected-member
```

`A`'s body is reported twice by stage 0 and `C`'s not at all.

**The documents support stage 1.** docs/architecture.md, "Error tolerance":
"A struct body line that is neither a field, a method nor `pass` is reported
once per body (`E0126`)", and the field's comment in `parser.rs`, "A struct
body already has a line that is not a member, so the following ones are not
reported again". Stage 1 saves the flag before a struct's body and restores
it after (`struct_def` in `src/parser/items.sisu`); stage 0 can do the same.

## D4. One unknown name in a parametrised test is reported once per case

- Differs: `compiler/stage1/cases/parametrised_test_names.sisu`: names
- Differs: `proposal/examples/testing.sisu`: names

```sisu
test "doubles" for n in [1, 2, 3]:
    assert double(n) == 2 * n
```

Stage 0 makes a test of each case of a parametrised test (17) before it
checks them (`tests_of::cases` in `sisu-resolve/src/collect.rs`), and so
checks the body once per case: `sisu check --format=ids` prints

```text
4:12-4:18 E0200 resolve.undefined-name name="double"
4:12-4:18 E0200 resolve.undefined-name name="double"
4:12-4:18 E0200 resolve.undefined-name name="double"
```

where stage 1's resolver prints the line once. `proposal/examples/testing.sisu`
shows it for its undeclared `parse_int` and `BadDigit` (three cases, each
reported three times); with the repeats removed the two outputs are the same.

**The documents support stage 1.** The parser's own principle is "one
mistake yields one message" (`sisu-parser/src/lib.rs`), docs/architecture.md
calls the error-tolerance rules "the rules that keep one mistake from
producing many messages", and the conformance README compares diagnostics
"by catalog identifier" as a set of mistakes. Stage 0 can check the body of
a parametrised test once and expand the cases after, or drop diagnostics
that repeat one at the same span.

## Not discrepancies, but found on the way

- **Dead code in `Scanner::punctuation`.** The branch "A string ends inside
  an interpolation: close both" (`c == Some('"') && depth == 0`) is never
  reached: `Scanner::code` sends every `"` to `string_start` first, so a
  quote inside an interpolation opens a nested string (as in
  `"{d["key"]}"`). Stage 1 leaves the branch out; the outputs are the same.
- **`without_bracket_echoes` drops more than its comment says.** It drops
  every "expected `)`" (or `]`, `}`) that starts a line after the first
  `E0009` in the file, where the comment and docs/architecture.md speak of
  the token at which the lexer closed the bracket. No file here shows a
  difference between the two readings; stage 1 transcribes the code.
- **The two dumps quote differently.** `sisu tokens` quotes a token's text
  with the renderer's `quote` (only `"`, `\`, `\n`, `\t` and `\r` escaped),
  `sisu parse --tree` with Rust's `Debug` of a string, which also escapes
  `\0`, other control characters, and the characters beyond ASCII that
  Rust's tables call unprintable or grapheme-extending (`\u{301}`,
  `\u{a0}`). Stage 1 reproduces both, with Rust's tables pinned in
  `src/text/unicode.sisu` (Unicode 17.0.0, from Rust 1.97); a written-down
  quoting for the tree dump would remove the dependency.
