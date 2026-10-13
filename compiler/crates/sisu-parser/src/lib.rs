//! The Sisu parser: hand-written recursive descent that never fails
//! (proposal/18-next-steps.md, Phase 0; 02-syntax.md, "Grammar notes").
//!
//! Every production can yield an `ERROR` node; recovery synchronises at
//! NEWLINE and DEDENT; and the result is always a lossless [`Tree`] whose
//! text is the input byte for byte. The grammar it follows is written out
//! in docs/architecture.md, one rule per function here.
//!
//! Diagnostics are suppressed when a second error would land on the token
//! where the previous one was reported, so one mistake yields one message.

mod exprs;
mod items;
mod parser;
mod patterns;
mod stmts;
mod types;

use sisu_diag::{Diagnostic, FileId, msg};
use sisu_syntax::Tree;

/// The result of parsing one file.
#[derive(Clone, Debug)]
pub struct Parse {
    pub tree: Tree,
    /// Lexer and parser diagnostics, in source order.
    pub diagnostics: Vec<Diagnostic>,
}

/// Parses a whole file.
pub fn parse(file: FileId, text: &str) -> Parse {
    let lexed = sisu_lexer::lex(file, text);
    let mut diagnostics = lexed.diagnostics;
    let mut p = parser::Parser::new(file, text, &lexed.tokens);
    p.lex_errors = diagnostics.iter().filter(|d| d.is_error()).map(|d| d.span.range.start).collect();
    p.source_file();
    let (tree, mut parse_diags) = p.finish();
    diagnostics.append(&mut parse_diags);
    sisu_diag::diagnostic::sort(&mut diagnostics);
    let diagnostics = without_bracket_echoes(text, diagnostics);
    let diagnostics = without_structure_echoes(diagnostics);
    let diagnostics = one_per_line(text, diagnostics);
    Parse { tree, diagnostics }
}

/// When the lexer gives up on a bracket that is never closed, it does so
/// at the start of a line (a statement keyword at the bracket's
/// indentation) or at the end of the file, and the parser then reports
/// "expected `)`" at exactly that token. That second report says the same
/// thing again and is dropped; a missing closer in the middle of a line
/// is a separate mistake and stays.
fn without_bracket_echoes(text: &str, diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
    let Some(first) =
        diagnostics.iter().filter(|d| d.msg == msg::LEX_UNCLOSED_BRACKET).map(|d| d.span.range.start).min()
    else {
        return diagnostics;
    };
    let starts_line = |at: u32| {
        let at = at as usize;
        at >= text.len()
            || text[..at].rsplit('\n').next().is_some_and(|before| before.bytes().all(|b| b == b' '))
    };
    diagnostics
        .into_iter()
        .filter(|d| {
            let closer = matches!(d.param("expected"), Some("`)`" | "`]`" | "`}`"));
            !(d.msg == msg::PARSE_EXPECTED
                && closer
                && d.span.range.start > first
                && starts_line(d.span.range.start))
        })
        .collect()
}

/// A statement at the top level (`E0107`) or an unexpected indented block
/// (`E0125`) after an earlier syntax error is almost always that error's
/// consequence: the block structure the parser sees has already diverged
/// from the one intended. They are reported only when nothing precedes
/// them.
fn without_structure_echoes(diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
    let mut seen_error = false;
    let mut out = Vec::new();
    for d in diagnostics {
        let structural = d.msg == msg::PARSE_TOP_LEVEL_STATEMENT || d.msg == msg::PARSE_UNEXPECTED_INDENT;
        if structural && seen_error {
            continue;
        }
        seen_error |= d.is_error();
        out.push(d);
    }
    out
}

/// How likely a diagnostic is to be the cause rather than a consequence
/// of the others on its line: a lexical error about the characters
/// themselves first, then bracket errors, then grammar errors.
fn rank(d: &Diagnostic) -> u8 {
    let code = d.code();
    match code {
        "E0008" | "E0009" | "E0010" => 1,
        c if c.starts_with("E00") => 0,
        _ => 2,
    }
}

/// Keeps one syntax diagnostic per line: the most likely cause, the
/// first of equals. A second error on the same line almost always follows
/// from the first (an unterminated string, then an unclosed bracket, then
/// a missing operand), so this is the guard against diagnostic storms;
/// errors on different lines are all reported.
fn one_per_line(text: &str, diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
    let mut out: Vec<(usize, Diagnostic)> = Vec::new();
    for d in diagnostics {
        let line = text.as_bytes().iter().take(d.span.range.start as usize).filter(|&&b| b == b'\n').count();
        match out.last_mut() {
            Some((l, prev)) if *l == line => {
                if rank(&d) < rank(prev) {
                    *prev = d;
                }
            }
            _ => out.push((line, d)),
        }
    }
    out.into_iter().map(|(_, d)| d).collect()
}

#[cfg(test)]
mod tests;
