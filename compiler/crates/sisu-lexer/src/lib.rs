//! The Sisu lexer (proposal/02-syntax.md, "Lexical structure").
//!
//! Two passes, neither of which consults the parser:
//!
//! 1. [`scan`] turns text into raw tokens, every byte included: trivia
//!    (whitespace, comments, line breaks), names, keywords, literals and
//!    punctuation. Strings with interpolation are split into
//!    `STRING_START`, `STRING_TEXT`, `INTERP_START`, the tokens of the
//!    expression, an optional `FORMAT_SPEC` and `INTERP_END`, then
//!    `STRING_END`: interpolation is part of the grammar (PEP 701).
//! 2. [`layout`] decides logical lines and blocks with Python's algorithm:
//!    a stack of indentation widths, one `INDENT` per push and one `DEDENT`
//!    per pop, layout only outside brackets, blank and comment-only lines
//!    ignored, and a line continued after a trailing comma or when the next
//!    line starts with a binary operator, `.`, `?.` or `??`.
//!
//! [`lex`] runs both. Every input produces tokens; problems are reported as
//! diagnostics and the token stream stays well formed.

mod layout;
mod scan;

pub use layout::layout;
pub use scan::scan;

use sisu_diag::{Diagnostic, FileId};
use sisu_syntax::Token;

/// The output of the lexer: tokens covering every byte, plus diagnostics.
#[derive(Clone, Debug)]
pub struct Lexed {
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Lexes `text` completely.
pub fn lex(file: FileId, text: &str) -> Lexed {
    let mut diagnostics = Vec::new();
    let raw = scan(file, text, &mut diagnostics);
    let tokens = layout(file, text, raw, &mut diagnostics);
    Lexed { tokens, diagnostics }
}

#[cfg(test)]
mod tests;
