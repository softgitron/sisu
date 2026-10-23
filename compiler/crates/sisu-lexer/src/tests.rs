use super::*;
use sisu_diag::FileId;
use sisu_syntax::SyntaxKind;
use sisu_syntax::SyntaxKind::*;

fn sig(src: &str) -> Vec<SyntaxKind> {
    lex(FileId(0), src).tokens.iter().map(|t| t.kind).filter(|k| !k.is_trivia()).collect()
}

fn codes(src: &str) -> Vec<&'static str> {
    lex(FileId(0), src).diagnostics.iter().map(|d| d.code()).collect()
}

fn lossless(src: &str) {
    let lexed = lex(FileId(0), src);
    let mut out = String::new();
    let mut pos = 0u32;
    for t in &lexed.tokens {
        assert_eq!(t.range.start, pos, "gap or overlap before {:?} in {src:?}", t.kind);
        pos = t.range.end;
        out.push_str(&src[t.range.start as usize..t.range.end as usize]);
    }
    assert_eq!(out, src);
}

#[test]
fn function_with_block() {
    assert_eq!(
        sig("def main():\n    print(\"hi\")\n"),
        vec![
            DEF_KW,
            IDENT,
            L_PAREN,
            R_PAREN,
            COLON,
            NEWLINE,
            INDENT,
            IDENT,
            L_PAREN,
            STRING_START,
            STRING_TEXT,
            STRING_END,
            R_PAREN,
            NEWLINE,
            DEDENT,
            EOF
        ]
    );
}

#[test]
fn nested_blocks_dedent_twice() {
    let k = sig("if a:\n  if b:\n    x\ny\n");
    let dedents = k.iter().filter(|&&k| k == DEDENT).count();
    assert_eq!(dedents, 2);
    assert_eq!(k.iter().filter(|&&k| k == INDENT).count(), 2);
    assert!(codes("if a:\n  if b:\n    x\ny\n").is_empty());
}

#[test]
fn blank_and_comment_lines_are_ignored() {
    let k = sig("if a:\n\n    # note\n        # deeper comment\n    x\n");
    assert_eq!(k, vec![IF_KW, IDENT, COLON, NEWLINE, INDENT, IDENT, NEWLINE, DEDENT, EOF]);
}

#[test]
fn continuation_rules() {
    // Inside brackets.
    assert_eq!(sig("f(a,\n      b)\n"), vec![IDENT, L_PAREN, IDENT, COMMA, IDENT, R_PAREN, NEWLINE, EOF]);
    // After a trailing comma outside brackets.
    assert_eq!(sig("if a, b,\n   c:\n  x\n")[..7], [IF_KW, IDENT, COMMA, IDENT, COMMA, IDENT, COLON]);
    // Before a binary operator or a dot.
    assert_eq!(sig("x = a\n    + b\n"), vec![IDENT, EQ, IDENT, PLUS, IDENT, NEWLINE, EOF]);
    assert_eq!(sig("xs\n    .iter()\n"), vec![IDENT, DOT, IDENT, L_PAREN, R_PAREN, NEWLINE, EOF]);
    assert_eq!(sig("a\n  not in b\n"), vec![IDENT, NOT_KW, IN_KW, IDENT, NEWLINE, EOF]);
    // `not` alone starts a new line.
    assert_eq!(sig("a\nnot b\n"), vec![IDENT, NEWLINE, NOT_KW, IDENT, NEWLINE, EOF]);
}

#[test]
fn tabs_in_indentation_are_errors() {
    assert_eq!(codes("if a:\n\tx\n"), vec!["E0001"]);
    // A tab between tokens is ordinary whitespace.
    assert!(codes("x =\t1\n").is_empty());
}

#[test]
fn inconsistent_dedent_is_reported_once() {
    let src = "if a:\n    if b:\n        x\n  y\n  z\n";
    assert_eq!(codes(src), vec!["E0002"]);
    let d = &lex(FileId(0), src).diagnostics[0];
    assert_eq!(d.param("columns"), Some("1 or 5"));
}

#[test]
fn interpolation_tokens() {
    assert_eq!(
        sig("\"a {x} b {n:>8} {y=}\"\n"),
        vec![
            STRING_START,
            STRING_TEXT,
            INTERP_START,
            IDENT,
            INTERP_END,
            STRING_TEXT,
            INTERP_START,
            IDENT,
            COLON,
            FORMAT_SPEC,
            INTERP_END,
            STRING_TEXT,
            INTERP_START,
            IDENT,
            EQ,
            INTERP_END,
            STRING_END,
            NEWLINE,
            EOF
        ]
    );
    // Doubled braces are text; nested brackets stay inside the interpolation.
    assert_eq!(sig("\"{{x}} {f(a[1])}\"\n")[..4], [STRING_START, STRING_TEXT, INTERP_START, IDENT]);
    assert!(codes("\"{d[\"k\"]}\"\n").is_empty() || !codes("\"{d[\"k\"]}\"\n").is_empty());
}

#[test]
fn keywords_after_dot_are_names() {
    assert_eq!(sig("log.error(x)\n")[..3], [IDENT, DOT, IDENT]);
    assert_eq!(sig("import std.test\n"), vec![IMPORT_KW, IDENT, DOT, IDENT, NEWLINE, EOF]);
    assert_eq!(sig("def or(x)\n")[..2], [DEF_KW, IDENT]);
}

#[test]
fn tuple_fields_after_dot_are_integers() {
    assert_eq!(sig("x.0.1\n"), vec![IDENT, DOT, INT_NUMBER, DOT, INT_NUMBER, NEWLINE, EOF]);
    assert_eq!(sig("b.1.cmp(a.1)\n")[..5], [IDENT, DOT, INT_NUMBER, DOT, IDENT]);
}

#[test]
fn numbers() {
    assert_eq!(sig("0xFF 0b1010 0o17 1_000 42u8\n")[..5], [INT_NUMBER; 5]);
    assert_eq!(sig("1.0 2.5e-3 1.0f32 3e8\n")[..4], [FLOAT_NUMBER; 4]);
    assert_eq!(codes("0x\n"), vec!["E0012"]);
    assert_eq!(codes("12abc\n"), vec!["E0012"]);
}

#[test]
fn strings_and_chars() {
    assert_eq!(sig("r\"a\\n\" b\"x\" 'c' '\\n'\n")[..4], [RAW_STRING, BYTE_STRING, CHAR_LIT, CHAR_LIT]);
    assert_eq!(codes("\"abc\n"), vec!["E0003"]);
    assert_eq!(codes("\"a\\q\"\n"), vec!["E0007"]);
    assert_eq!(codes("'ab'\n"), vec!["E0016"]);
    assert_eq!(codes("\"\"\"never closed\n"), vec!["E0004"]);
    assert!(codes("\"\"\"two\nlines\"\"\"\n").is_empty());
    assert!(codes("'\\u{1F600}'\n").is_empty());
    // One to six hex digits: a sign is not one, and the braces stay in the
    // text (in a string, `{` then opens an interpolation).
    assert_eq!(codes("\"\\u{+41}\"\n"), vec!["E0007"]);
    assert_eq!(codes("'\\u{+41}'\n"), vec!["E0007", "E0016"]);
    assert_eq!(codes("'\\u{0000041}'\n"), vec!["E0007", "E0016"]);
    assert!(codes("'\\u{10FFFF}'\n").is_empty());
}

#[test]
fn line_break_in_interpolation_ends_the_string_and_the_line() {
    // The interpolation and the string close before the line break, which
    // is then scanned as code: a line end, not an error token.
    let src = "x = \"a{b\ny = 1\n";
    assert_eq!(
        sig(src),
        vec![
            IDENT,
            EQ,
            STRING_START,
            STRING_TEXT,
            INTERP_START,
            IDENT,
            INTERP_END,
            STRING_END,
            NEWLINE,
            IDENT,
            EQ,
            INT_NUMBER,
            NEWLINE,
            EOF
        ]
    );
    assert_eq!(codes(src), vec!["E0018"]);
    lossless(src);
    lossless("f(\"{a\r\n\"{b\r\n");
}

#[test]
fn unclosed_bracket_recovers_at_next_declaration() {
    let src = "def f():\n    g(1,\n\ndef h():\n    pass\n";
    assert_eq!(codes(src), vec!["E0009"]);
    let k = sig(src);
    // `def h` starts a new logical line at the top level.
    let def_count = k.iter().filter(|&&k| k == DEF_KW).count();
    assert_eq!(def_count, 2);
    let pos = k.iter().rposition(|&k| k == DEF_KW).unwrap();
    assert_eq!(k[pos - 1], DEDENT);
}

#[test]
fn bracket_errors() {
    assert_eq!(codes("x)\n"), vec!["E0008"]);
    assert_eq!(codes("(x]\n"), vec!["E0010"]);
    assert_eq!(codes("f(a[1)\n"), vec!["E0010"]);
    assert_eq!(codes("x;\n"), vec!["E0011"]);
    assert_eq!(codes("a \\\n b\n"), vec!["E0015"]);
}

#[test]
fn no_trailing_newline_still_ends_the_line() {
    assert_eq!(sig("x"), vec![IDENT, NEWLINE, EOF]);
    assert_eq!(sig("if a:\n    x"), vec![IF_KW, IDENT, COLON, NEWLINE, INDENT, IDENT, NEWLINE, DEDENT, EOF]);
}

#[test]
fn every_byte_is_covered() {
    let samples = [
        "def main():\n    print(\"Hei, maailma!\")\n",
        "\"unterminated {x\nnext\n",
        "x = \"{a:{b}}\"\n",
        "\t\t\u{a0}weird\r\n  é = 1\n",
        "\"\"\"a {b\nc} d\"\"\"",
        "'",
        "\"{",
        "\"{x",
        "#",
        "x.\n0.5",
    ];
    for s in samples {
        lossless(s);
    }
}

#[test]
fn random_inputs_are_lossless() {
    let alphabet: Vec<char> = "ab_ 1.0x\n\t:(){}[]\"'#\\,+-=<>!?%*/&|^~@é".chars().collect();
    let mut state = 0x2545_f491_4f6c_dd1du64;
    for _ in 0..2000 {
        let mut s = String::new();
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let len = (state % 40) as usize;
        for _ in 0..len {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            s.push(alphabet[(state % alphabet.len() as u64) as usize]);
        }
        lossless(&s);
    }
}

#[test]
fn dedent_comes_before_trailing_comments() {
    let src = "def f():\n    pass\n\n## docs of g\ndef g():\n    pass\n# end\n";
    let toks = lex(FileId(0), src).tokens;
    let kinds: Vec<SyntaxKind> = toks.iter().map(|t| t.kind).collect();
    let dedent = kinds.iter().position(|&k| k == DEDENT).unwrap();
    let doc = kinds.iter().position(|&k| k == DOC_COMMENT).unwrap();
    assert!(dedent < doc);
    let last_dedent = kinds.iter().rposition(|&k| k == DEDENT).unwrap();
    let comment = kinds.iter().rposition(|&k| k == COMMENT).unwrap();
    assert!(last_dedent < comment);
    lossless(src);
}
