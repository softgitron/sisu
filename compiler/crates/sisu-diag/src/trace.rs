//! The stack trace of a panic as a run prints it, the same in the
//! interpreter and in a native executable (docs/architecture.md, decision
//! 234).
//!
//! A trace names one frame per line, innermost first. One deeper than
//! [`INNER`] and [`OUTER`] frames together shows its innermost [`INNER`]
//! and its outermost [`OUTER`], with one line between them for the rest
//! (`run.trace-elided`), which gives no count: how deep a stack overflow
//! goes is the implementation's (the interpreter's limit of frames, a
//! native stack's bytes), so what a run prints must not depend on it.

use crate::catalog::Catalog;
use crate::msg;

/// The innermost frames a long trace shows.
pub const INNER: usize = 50;
/// The outermost frames a long trace shows.
pub const OUTER: usize = 50;

/// The lines of the trace to print, innermost first: every line of a
/// trace of at most [`INNER`] + [`OUTER`] frames, else the innermost and
/// the outermost with the line that stands for the rest between them.
pub fn shown(lines: &[String]) -> Vec<String> {
    if lines.len() <= INNER + OUTER {
        return lines.to_vec();
    }
    let mut out = Vec::with_capacity(INNER + OUTER + 1);
    out.extend_from_slice(&lines[..INNER]);
    out.push(Catalog::builtin().render(msg::RUN_TRACE_ELIDED.id(), &[]));
    out.extend_from_slice(&lines[lines.len() - OUTER..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_long_trace_keeps_its_ends() {
        let short: Vec<String> = (0..INNER + OUTER).map(|i| format!("at f{i}")).collect();
        assert_eq!(shown(&short), short);
        let long: Vec<String> = (0..1000).map(|i| format!("at f{i}")).collect();
        let s = shown(&long);
        assert_eq!(s.len(), INNER + OUTER + 1);
        assert_eq!(s[0], "at f0");
        assert_eq!(s[INNER - 1], format!("at f{}", INNER - 1));
        assert_eq!(s[INNER], "... (frames left out)");
        assert_eq!(s[INNER + 1], format!("at f{}", 1000 - OUTER));
        assert_eq!(s.last().map(String::as_str), Some("at f999"));
    }
}
