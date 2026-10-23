//! The layout pass: logical lines, INDENT and DEDENT.
//!
//! The algorithm is Python's: a stack of indentation widths starting at 0;
//! at the start of each logical line outside brackets, a wider line pushes
//! its width and emits INDENT, a narrower one pops and emits one DEDENT per
//! popped width. Blank and comment-only lines are trivia. A line break ends
//! a logical line (becomes NEWLINE) unless it is inside brackets, follows a
//! trailing comma, or the next non-blank line starts with a continuation
//! operator.
//!
//! Recovery: a dedent that matches no level is reported once and the
//! width is pushed as a phantom level that emits no tokens, so the rest of
//! the block keeps its structure; a bracket left open when a line starting
//! with a declaration keyword appears at or left of the bracket's line is
//! reported and closed there, so one missing `)` does not swallow the file.

use sisu_diag::{Diagnostic, FileId, Span, TextRange, msg};
use sisu_syntax::Token;
use sisu_syntax::kinds::SyntaxKind::*;
use sisu_syntax::kinds::{CONTINUATION_STARTERS, SyntaxKind};

struct Line {
    tokens: Vec<Token>,
    /// Whether the last token is the line break.
    has_break: bool,
}

#[derive(Clone, Copy)]
struct Level {
    width: u32,
    /// A phantom level was pushed by error recovery and emits no tokens.
    real: bool,
}

#[derive(Clone, Copy)]
struct Open {
    kind: SyntaxKind,
    range: TextRange,
    line_width: u32,
}

fn split_lines(raw: Vec<Token>) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut cur = Vec::new();
    for t in raw {
        let is_break = t.kind == LINE_BREAK;
        cur.push(t);
        if is_break {
            lines.push(Line { tokens: std::mem::take(&mut cur), has_break: true });
        }
    }
    if !cur.is_empty() {
        lines.push(Line { tokens: cur, has_break: false });
    }
    lines
}

fn first_significant(line: &Line) -> Option<(usize, Token)> {
    line.tokens.iter().copied().enumerate().find(|(_, t)| !t.kind.is_trivia())
}

fn closes(open: SyntaxKind, close: SyntaxKind) -> bool {
    matches!(
        (open, close),
        (L_PAREN, R_PAREN) | (L_BRACKET, R_BRACKET) | (L_BRACE, R_BRACE) | (INTERP_START, INTERP_END)
    )
}

/// A bracket never closed, with the fix that closes it at the end of the
/// line it opens on (before the line's comment). On a block header the
/// bracket closes before the `:`, and a parenthesis before the `->` of a
/// return type: `def f(x: int -> int:` becomes `def f(x: int) -> int:`.
fn unclosed(text: &str, file: FileId, open: &Open) -> Diagnostic {
    let from = open.range.end as usize;
    let line_end = text[from.min(text.len())..].find('\n').map(|i| from + i).unwrap_or(text.len());
    let line = &text[from.min(line_end)..line_end];
    // A comment is a `#` outside a string; good enough for where to close.
    let mut code = match line.find(" #") {
        Some(i) if !line[..i].contains('"') => &line[..i],
        _ => line,
    }
    .trim_end();
    if let Some(head) = code.strip_suffix(':') {
        code = head.trim_end();
        if open.kind == L_PAREN
            && let Some(i) = code.find(" ->")
        {
            code = code[..i].trim_end();
        }
    }
    let at = (from + code.len()) as u32;
    let closer = match open.kind {
        L_PAREN => ")",
        L_BRACKET => "]",
        _ => "}",
    };
    Diagnostic::new(msg::LEX_UNCLOSED_BRACKET, Span::new(file, open.range))
        .with("open", bracket_text(open.kind))
        .fix(
            msg::LEX_CLOSE_BRACKET,
            Vec::new(),
            vec![sisu_diag::Edit { span: Span::new(file, TextRange::empty(at)), text: closer.to_string() }],
        )
}

fn bracket_text(kind: SyntaxKind) -> &'static str {
    match kind {
        L_PAREN => "(",
        R_PAREN => ")",
        L_BRACKET => "[",
        R_BRACKET => "]",
        L_BRACE | INTERP_START => "{",
        R_BRACE | INTERP_END => "}",
        _ => "?",
    }
}

/// Keywords that start a declaration or statement and can never appear
/// inside brackets; seeing one at the start of a line while a bracket is
/// open means the bracket was never closed.
const STATEMENT_KEYWORDS: &[SyntaxKind] = &[
    DEF_KW,
    STRUCT_KW,
    ENUM_KW,
    ERROR_KW,
    IMPL_KW,
    TRAIT_KW,
    LET_KW,
    VAR_KW,
    RETURN_KW,
    WHILE_KW,
    IMPORT_KW,
    TEST_KW,
    CONST_KW,
    BREAK_KW,
    CONTINUE_KW,
    PASS_KW,
    ASSERT_KW,
    DEFER_KW,
    WITH_KW,
    CASE_KW,
    ELIF_KW,
];

/// Keywords that start a line ending in `:` and a block.
const BLOCK_KEYWORDS: &[SyntaxKind] = &[
    DEF_KW, IF_KW, ELIF_KW, ELSE_KW, WHILE_KW, FOR_KW, WITH_KW, MATCH_KW, CASE_KW, STRUCT_KW, ENUM_KW,
    ERROR_KW, IMPL_KW, TRAIT_KW, TEST_KW,
];

/// The indentation (in bytes) of the next line with significant tokens.
fn next_indentation(lines: &[Line]) -> Option<u32> {
    lines.iter().find_map(|l| {
        let (_, first) = first_significant(l)?;
        let start = l.tokens.first().map(|t| t.range.start).unwrap_or(first.range.start);
        Some(first.range.start - start)
    })
}

pub fn layout(file: FileId, text: &str, raw: Vec<Token>, diags: &mut Vec<Diagnostic>) -> Vec<Token> {
    let lines = split_lines(raw);
    let mut out: Vec<Token> = Vec::new();
    let mut levels = vec![Level { width: 0, real: true }];
    let mut brackets: Vec<Open> = Vec::new();
    let mut continuing = false;
    // Whether the current logical line has significant tokens.
    let mut line_has_content = false;
    // Index in `out` of the line break that ended the last line with content.
    let mut last_break: Option<usize> = None;
    // Index in `out` of the last NEWLINE.
    let mut last_newline: Option<usize> = None;
    let mut logical_width = 0u32;
    // The first significant token of the current logical line.
    let mut logical_first: Option<SyntaxKind> = None;
    let span = |r: TextRange| Span::new(file, r);

    for (li, line) in lines.iter().enumerate() {
        let Some((first_idx, first)) = first_significant(line) else {
            // Blank or comment-only: all trivia.
            out.extend(line.tokens.iter().copied());
            continue;
        };
        let width = indentation_width(text, line, first_idx, file, diags);

        // Recovery for a bracket that was never closed.
        if continuing && !brackets.is_empty() {
            let outer = brackets[0];
            if STATEMENT_KEYWORDS.contains(&first.kind) && width <= outer.line_width {
                diags.push(unclosed(text, file, &outer));
                brackets.clear();
                continuing = false;
                if let Some(i) = last_break {
                    out[i].kind = NEWLINE;
                    last_newline = Some(i);
                }
                line_has_content = false;
            }
        }

        let mut idx = 0;
        if !continuing {
            // Leading whitespace is trivia before the layout tokens.
            while idx < first_idx {
                out.push(line.tokens[idx]);
                idx += 1;
            }
            let at = TextRange::empty(first.range.start);
            let top = levels.last().map(|l| l.width).unwrap_or(0);
            if width > top {
                levels.push(Level { width, real: true });
                out.push(Token { kind: INDENT, range: at });
            } else if width < top {
                let candidates: Vec<String> = levels
                    .iter()
                    .filter(|l| l.real && l.width < top)
                    .map(|l| (l.width + 1).to_string())
                    .collect();
                // DEDENTs go right after the NEWLINE that ended the block, so
                // blank lines and comments before this line (a doc comment of
                // the next item, say) belong to the enclosing level.
                let (insert_at, dedent_at) = match last_newline {
                    Some(i) => (i + 1, TextRange::empty(out[i].range.end)),
                    None => (out.len(), at),
                };
                let mut count = 0;
                while levels.last().is_some_and(|l| l.width > width) {
                    if let Some(l) = levels.pop()
                        && l.real
                    {
                        count += 1;
                    }
                }
                for _ in 0..count {
                    out.insert(insert_at, Token { kind: DEDENT, range: dedent_at });
                }
                let new_top = levels.last().map(|l| l.width).unwrap_or(0);
                if new_top != width {
                    diags.push(
                        Diagnostic::new(
                            msg::LEX_INCONSISTENT_DEDENT,
                            span(TextRange::new(
                                first.range.start - width.min(first.range.start),
                                first.range.start,
                            )),
                        )
                        .with("columns", candidates.join(" or ")),
                    );
                    levels.push(Level { width, real: false });
                }
            }
            logical_width = width;
            logical_first = Some(first.kind);
        }

        // The content of the line.
        let end = if line.has_break { line.tokens.len() - 1 } else { line.tokens.len() };
        let mut last_sig: Option<Token> = None;
        while idx < end {
            let t = line.tokens[idx];
            idx += 1;
            if !t.kind.is_trivia() {
                last_sig = Some(t);
                line_has_content = true;
            }
            if t.kind.is_open_bracket() {
                brackets.push(Open { kind: t.kind, range: t.range, line_width: logical_width });
            } else if t.kind.is_close_bracket() {
                close_bracket(&mut brackets, t, file, diags);
            }
            out.push(t);
        }

        // A block header (`def f(x: int -> int:`) that ends in `:` with a
        // bracket still open, followed by an indented line: the bracket was
        // never closed, and the indented lines are the block.
        if !brackets.is_empty()
            && last_sig.is_some_and(|t| t.kind == COLON)
            && logical_first.is_some_and(|k| BLOCK_KEYWORDS.contains(&k))
            && next_indentation(&lines[li + 1..]).is_some_and(|w| w > logical_width)
        {
            let outer = brackets[0];
            diags.push(unclosed(text, file, &outer));
            brackets.clear();
        }

        // Does the next logical line continue this one?
        let next_continues = !brackets.is_empty()
            || last_sig.is_some_and(|t| t.kind == COMMA)
            || next_line_continues(&lines[li + 1..]);
        continuing = next_continues;
        if line.has_break {
            let mut brk = line.tokens[line.tokens.len() - 1];
            if !next_continues && line_has_content {
                brk.kind = NEWLINE;
                line_has_content = false;
                last_newline = Some(out.len());
            }
            last_break = Some(out.len());
            out.push(brk);
        } else if !next_continues && line_has_content {
            last_newline = Some(out.len());
            out.push(Token { kind: NEWLINE, range: TextRange::empty(text.len() as u32) });
            line_has_content = false;
        }
    }

    let eof = TextRange::empty(text.len() as u32);
    if let Some(open) = brackets.first() {
        diags.push(unclosed(text, file, open));
    }
    if line_has_content {
        last_newline = Some(out.len());
        out.push(Token { kind: NEWLINE, range: eof });
    }
    let (insert_at, dedent_at) = match last_newline {
        Some(i) => (i + 1, TextRange::empty(out[i].range.end)),
        None => (out.len(), eof),
    };
    while let Some(l) = levels.pop() {
        if l.real && l.width > 0 {
            out.insert(insert_at, Token { kind: DEDENT, range: dedent_at });
        }
    }
    out.push(Token { kind: EOF, range: eof });
    out
}

fn indentation_width(
    text: &str,
    line: &Line,
    first_idx: usize,
    file: FileId,
    diags: &mut Vec<Diagnostic>,
) -> u32 {
    let mut width = 0u32;
    for t in &line.tokens[..first_idx] {
        if t.kind != WHITESPACE {
            continue;
        }
        let s = text.get(t.range.start as usize..t.range.end as usize).unwrap_or("");
        for (off, c) in s.char_indices() {
            width += 1;
            if c == '\t' {
                let at = t.range.start + off as u32;
                diags.push(
                    Diagnostic::new(msg::LEX_TAB_IN_INDENTATION, Span::new(file, TextRange::new(at, at + 1)))
                        .note(msg::LEX_USE_SPACES, Vec::new()),
                );
            } else if c != ' ' {
                let at = t.range.start + off as u32;
                diags.push(
                    Diagnostic::new(
                        msg::LEX_BAD_INDENT_CHAR,
                        Span::new(file, TextRange::new(at, at + c.len_utf8() as u32)),
                    )
                    .with("char", format!("{:?}", c)),
                );
            }
        }
    }
    width
}

fn next_line_continues(rest: &[Line]) -> bool {
    for line in rest {
        let mut sig = line.tokens.iter().filter(|t| !t.kind.is_trivia());
        let Some(first) = sig.next() else { continue };
        if CONTINUATION_STARTERS.contains(&first.kind) {
            return true;
        }
        if first.kind == NOT_KW {
            return sig.next().is_some_and(|t| t.kind == IN_KW);
        }
        return false;
    }
    false
}

fn close_bracket(brackets: &mut Vec<Open>, t: Token, file: FileId, diags: &mut Vec<Diagnostic>) {
    let span = Span::new(file, t.range);
    match brackets.last() {
        None => {
            diags.push(Diagnostic::new(msg::LEX_UNMATCHED_CLOSE, span).with("close", bracket_text(t.kind)))
        }
        Some(top) if closes(top.kind, t.kind) => {
            brackets.pop();
        }
        Some(_) => {
            if let Some(pos) = brackets.iter().rposition(|o| closes(o.kind, t.kind)) {
                let top = brackets[brackets.len() - 1];
                diags.push(
                    Diagnostic::new(msg::LEX_MISMATCHED_BRACKET, span)
                        .with("close", bracket_text(t.kind))
                        .with("open", bracket_text(top.kind))
                        .label(Span::new(file, top.range), msg::LEX_OPENED_HERE, Vec::new()),
                );
                brackets.truncate(pos);
            } else {
                // Nothing deeper matches: report the mismatch and treat the
                // innermost bracket as closed, which keeps it to one error.
                let top = brackets[brackets.len() - 1];
                diags.push(
                    Diagnostic::new(msg::LEX_MISMATCHED_BRACKET, span)
                        .with("close", bracket_text(t.kind))
                        .with("open", bracket_text(top.kind))
                        .label(Span::new(file, top.range), msg::LEX_OPENED_HERE, Vec::new()),
                );
                brackets.pop();
            }
        }
    }
}
