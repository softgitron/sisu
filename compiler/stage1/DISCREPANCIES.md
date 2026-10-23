# Where stage 1 and stage 0 differ

Stage 1 transcribes the Rust front end (`sisu-lexer`, `sisu-syntax`,
`sisu-parser`, and in part `sisu-resolve` and `sisu-typeck`) rule for
rule. Where the Rust behaviour is an accident rather than a rule, stage 1
does what the documents say instead, and the difference is listed here,
with the file that shows it, both outputs, and the documents that decide.
The coordinator decides which side changes (D-079); until then `check.sh`
accepts exactly the differences listed under **Differs** and fails on any
other.

The **Differs** lines are read by `check.sh`: a path from the repository
root, then the outputs that differ (`tokens` is `sisu tokens --trivia` with
its diagnostics, `tree` is `sisu parse --tree` with its diagnostics,
`trivia-tree` is `sisu parse --tree --trivia`, `names` is the name
diagnostics of `sisu check --format=ids`, those stage 1's resolver
reports, and `types` the type checker's diagnostics of the same command).

None is listed now. The first round's five (D1 to D5: a line break inside
the interpolation of a one-line string, `\u{+41}` as an escape, a nested
struct resetting "reported once per body", one unknown name in a
parametrised test reported once per case, and the count `E0305` reports
past a bare `*`) were decided for the documents, and stage 0 now behaves
as stage 1 does (docs/architecture.md, decisions 165 to 168 and 179). The
stage-1 cases that showed them stay in `cases/` and give identical outputs,
and the conformance suite has a case for each (`syntax/newline_in_interpolation`,
`syntax/unicode_escape_sign`, `syntax/nested_struct_members`,
`testing/parametrised_once`, `types/keyword_only_count`). A new
difference is listed from D6 on.

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
