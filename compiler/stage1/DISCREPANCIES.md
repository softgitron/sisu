# Where stage 1 and stage 0 differ

Stage 1 transcribes the Rust front end (`sisu-lexer`, `sisu-syntax`,
`sisu-parser`, and in part `sisu-resolve` and `sisu-typeck`) and the
formatter (`sisu-fmt`) rule for rule. Where the Rust behaviour is an
accident rather than a rule, stage 1 does what the documents say instead,
and the difference is listed here, with the file that shows it, both
outputs, and the documents that decide. The coordinator decides which
side changes (D-079); until then `check.sh` accepts exactly the
differences listed under **Differs** and fails on any other.

The **Differs** lines are read by `check.sh`: a path from the repository
root, then the outputs that differ (`tokens` is `sisu tokens --trivia` with
its diagnostics, `tree` is `sisu parse --tree` with its diagnostics,
`trivia-tree` is `sisu parse --tree --trivia`, `names` is the name
diagnostics of `sisu check --format=ids`, those stage 1's resolver
reports, `types` the type checker's diagnostics of the same command, `fmt`
the text `sisu fmt` leaves of the file, and `fmt-check` what `sisu fmt
--check` prints of it, with its exit status).

Two are listed now, both of the formatter, which stage 1 transcribed this
round: D6 and D7, each shown by a case of its own in `cases/fmt/`, and
neither met by any other file of the corpus, the standard library or
stage 1's own sources, nor by their mutants (`check.sh --mutants 2000
--seed 7`); in the classes of the files laid out otherwise, only those
two cases' own copies show them, which `check.sh` knows by their marks
(stage 0 overflowing its stack, or refusing its own output for a file
with an `Option[`). The first round's five (D1 to D5: a line break inside
the interpolation of a one-line string, `\u{+41}` as an escape, a nested
struct resetting "reported once per body", one unknown name in a
parametrised test reported once per case, and the count `E0305` reports
past a bare `*`) were decided for the documents, and stage 0 now behaves
as stage 1 does (docs/architecture.md, decisions 165 to 168 and 179). The
stage-1 cases that showed them stay in `cases/` and give identical outputs,
and the conformance suite has a case for each (`syntax/newline_in_interpolation`,
`syntax/unicode_escape_sign`, `syntax/nested_struct_members`,
`testing/parametrised_once`, `types/keyword_only_count`).

## D6. `Option[Option[T]]` overflows the formatter's stack

- Differs: `compiler/stage1/cases/fmt/option_of_option.sisu`: fmt, fmt-check

```sisu
def nested(x: Option[Option[int]]) -> int:
    return 0
```

`option_sugar` (`sisu-fmt/src/lines.rs`) decides whether an `Option[T]`
is written `T?`. For the inner option of `Option[Option[T]]` it asks
whether the outer one is written so (`option_sugar(tree, outer) ==
Some(n)`, since "`Option[Option[T]]` written `Option[T]?` would be
`T??`"), and to answer, the outer one asks whether its argument, the inner
one, is (`option_sugar(tree, arg)`, to know whether its `T` is itself
optional): the two ask each other without end. `sisu fmt` overflows its
stack and aborts on any file that writes `Option[Option[T]]` with a `T`
that `?` may follow (a name, a tuple, `ref`, `dyn` or a parenthesised
type), and so does `sisu parse --shape`, which asks the same.

Stage 0 (`sisu fmt --check`, exit status 134; `sisu fmt` leaves the file
as it is):

```text
thread 'main' has overflowed its stack
fatal runtime error: stack overflow, aborting
```

Stage 1 (exit status 1; `sisu fmt` writes the new line):

```text
- def nested(x: Option[Option[int]]) -> int:
+ def nested(x: Option[int?]) -> int:
```

**The documents support stage 1.** Decision 152: the formatter writes
`T?` for `Option[T]` "unless `T` is optional, a result or a function type,
or the option is behind `ref`"; the outer option's `T` is optional, so it
stays `Option[...]`, and the inner one's is `int`, so it is `int?` (10,
"canonical spelling of sugar"; docs/architecture.md, "`Option[T]` written
`T?` where that reads unambiguously"). Stage 1 breaks the cycle where it
starts (`option_sugar_assuming`): the inner option, asking about the outer
one, has it take its argument for written `T?`, so the outer one, whose
`T` is then optional, is not, and the inner one is. The answer is the
same for every other input, where neither asks about the other. The fix
in stage 0 is the same: pass the asking option down to the outer one's
question.

## D7. Spaces inside `Option[ T ]` make the formatter refuse the file

- Differs: `compiler/stage1/cases/fmt/option_with_spaces.sisu`: fmt, fmt-check

```sisu
def spaced(x: Option[ int ]) -> int:
    return 0
```

`option_sugar` leaves an `Option[T]` as written when its node or its type
arguments hold any trivia ("Nothing in it but the type: a comment would
have nowhere to go"), spaces and line breaks among them. The formatter
then writes the option's tokens without the spaces, `Option[int]`, whose
shape is an option's (`shape.rs` asks `option_sugar` too), where the
input's was a path's: the self-check refuses the formatter's own output,
and the file is left as it is. A file that breaks a long `Option[...]`
over lines is refused the same way.

Stage 0 (`sisu fmt --check`, exit status 70):

```text
error[D0076]: internal error: the formatted `option_with_spaces.sisu` would not parse to the same program (from byte 132); it was left as it is
```

Stage 1 (exit status 1):

```text
- def spaced(x: Option[ int ]) -> int:
+ def spaced(x: int?) -> int:
```

**The documents support stage 1.** Spaces are not among decision 152's
exceptions, `D0076` is "a bug of the formatter, reported rather than
written" (`sisu-fmt/src/lib.rs`), and the code's own reason is the
comment, which is all `result_parts` looks for in a `Result[T, E]`
(`has_comment`), whose spaces never stop `T ! E`. Stage 1 leaves the
option as written only when a comment is among its own tokens or its
type arguments'. The fix in stage 0 is the same: `has_comment` in place of
the test for trivia in `option_sugar`.

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
- **The overflow of a sized integer's negation and of its literal.** Stage
  0's `sized_op` (`sisu-interp/src/sized.rs`) writes the expression of a
  `T`'s overflow as `{x} {sym} {y} as T`, and its negation and literal
  pass `-` and `0`, and `as` and `0`: negating an `i8` that holds -128 at
  compile time is `integer overflow: -128 - 0 as i8`, where an `int`'s
  negation is `integer overflow: -(-9223372036854775808)`. The catalog's
  `panic.overflow` gives only `{expr}`, so no document decides; stage 1
  transcribes the code (`interp.sized_op`), and `-(-128) as i8` would read
  as the `int` form does.
- **The `Escapable` rule's decision was unwritten for a round.**
  `sisu-resolve/src/borrows.rs` (`can_borrow_in`) says a type parameter
  bounded by `Escapable` holds no loan, since every type argument it stands
  for must not, and the solver's `Escapable` and the owner checks of
  `sisu-typeck` use it; the code cited a decision 256 that
  docs/architecture.md did not carry. Stage 1 transcribes the rule
  (`Program.can_borrow_in`), which no document contradicts (03 calls
  `Escapable` structural); the coordinator has since written it as
  decision 257 (256 is the structural hash).
- **A test's instance is not a test once it is resolved.** Stage 0's
  `record_instance` gives the instance of a test that reflects the test's
  `FnKind::Test` (decision 251), but a function's instance is recorded
  before it is resolved, and resolving it makes its `FnInfo` again, as a
  free function's; only `instance_of` is carried over. Nothing `check`,
  `sir` or `build` prints shows it (the instance is named `test#N` by its
  template, which `prog.tests` lists); stage 1 does the same in the same
  order.
