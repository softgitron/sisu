//! `sisu fmt` (10, "Formatter"; 02, "Formatting"): the formatter with no
//! options. Four spaces, 100 columns, Black's bracket-first line splitting
//! with breaks before binary operators, a magic trailing comma, trailing
//! commas in multi-line constructs, sorted imports with the unused ones
//! removed, one blank line between items, `T?` for `Option[T]`, and every
//! comment kept, word for word. `Result[T, E]` stays as written: `T ! E`
//! needs `E` to be an error set, which the syntax does not show.
//!
//! It works on the lossless tree, so it never disagrees with the parser:
//! the tree's tokens are regrouped into logical lines (the lexer's
//! `NEWLINE`s), each laid out by the printer of `doc.rs`, and the blank
//! lines and comments between them placed by the rules below. It never
//! changes which block a statement belongs to. A file with a syntax error
//! is not formatted, and the result is checked before it is returned: it
//! must parse to the same tree as the input, up to what the formatter
//! changes on purpose ([`shape`]), or the formatter refuses.

mod doc;
mod lines;
mod prune;
mod shape;
mod space;

pub use shape::shape;

/// The widest line the formatter writes (when a line can be broken).
pub const WIDTH: usize = 100;
/// One level of indentation.
pub const INDENT: usize = 4;

/// Why a file was not formatted.
#[derive(Clone, Debug, PartialEq)]
pub enum Refusal {
    /// The file has syntax errors: it is left as it is.
    Syntax(Vec<sisu_diag::Diagnostic>),
    /// The formatted text would not parse to the same tree: a bug of the
    /// formatter, reported rather than written.
    Changed { at: usize },
}

/// The canonical text of `text`.
pub fn format(text: &str) -> Result<String, Refusal> {
    let parse = sisu_parser::parse(sisu_diag::FileId(0), text);
    let errors: Vec<sisu_diag::Diagnostic> = parse.diagnostics.into_iter().filter(|d| d.is_error()).collect();
    if !errors.is_empty() {
        return Err(Refusal::Syntax(errors));
    }
    let out = lines::format_tree(&parse.tree);
    // The self-check (02: "an AST-equality self-check").
    let again = sisu_parser::parse(sisu_diag::FileId(0), &out);
    if again.diagnostics.iter().any(|d| d.is_error()) {
        let at =
            again.diagnostics.iter().find(|d| d.is_error()).map(|d| d.span.range.start as usize).unwrap_or(0);
        return Err(Refusal::Changed { at });
    }
    let (a, b) = (shape::shape_of(&parse.tree), shape::shape_of(&again.tree));
    if a != b {
        let at = a.bytes().zip(b.bytes()).position(|(x, y)| x != y).unwrap_or(0);
        return Err(Refusal::Changed { at });
    }
    Ok(out)
}

#[cfg(test)]
mod tests;
