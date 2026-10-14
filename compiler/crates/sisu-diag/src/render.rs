//! Renderers: the human form with source excerpts, and the conformance form
//! (`--format=ids`), which names messages by code, identifier, position and
//! parameters so that any front end can be compared against expected files.

use crate::catalog::Catalog;
use crate::diagnostic::{Diagnostic, Severity};
use crate::source::SourceMap;
use crate::span::Span;

/// Quotes a parameter value for the conformance form.
pub fn quote(value: &str) -> String {
    let mut s = String::from("\"");
    for c in value.chars() {
        match c {
            '"' => s.push_str("\\\""),
            '\\' => s.push_str("\\\\"),
            '\n' => s.push_str("\\n"),
            '\t' => s.push_str("\\t"),
            '\r' => s.push_str("\\r"),
            c => s.push(c),
        }
    }
    s.push('"');
    s
}

fn position(map: &SourceMap, span: Span) -> String {
    let (a, b) = map.line_col(span);
    format!("{}:{}-{}:{}", a.line, a.col, b.line, b.col)
}

/// One line per diagnostic, then one indented line per label and fix:
///
/// ```text
/// 3:5-3:8 E0200 resolve.undefined-name name="foo"
///   label 1:5-1:8 resolve.first-defined
///   fix type.use-floor-division 2:7-2:8 "//"
/// ```
pub fn render_ids(map: &SourceMap, diags: &[Diagnostic]) -> String {
    let mut out = String::new();
    for d in diags {
        out.push_str(&format!("{} {} {}", position(map, d.span), d.code(), d.msg.id()));
        for (k, v) in &d.params {
            out.push_str(&format!(" {k}={}", quote(v)));
        }
        out.push('\n');
        for l in &d.labels {
            out.push_str(&format!("  label {} {}", position(map, l.span), l.msg.id()));
            for (k, v) in &l.params {
                out.push_str(&format!(" {k}={}", quote(v)));
            }
            out.push('\n');
        }
        for f in &d.fixes {
            out.push_str(&format!("  fix {}", f.msg.id()));
            for e in &f.edits {
                out.push_str(&format!(" {} {}", position(map, e.span), quote(&e.text)));
            }
            out.push('\n');
        }
    }
    out
}

fn severity_word(s: Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Note => "note",
    }
}

/// One source line with a span marked under it. `gutter` is the width of
/// the line-number column, the same for every excerpt of a diagnostic.
fn excerpt(out: &mut String, map: &SourceMap, span: Span, marker: char, text: &str, gutter: usize) {
    let Some(file) = map.get(span.file) else {
        return;
    };
    let (a, b) = map.line_col(span);
    let line = file.line_text(a.line);
    let pad = " ".repeat(gutter);
    out.push_str(&format!("{pad} |\n{:>gutter$} | {line}\n", a.line));
    let start = a.col.saturating_sub(1) as usize;
    // A span over several lines is marked to the end of its first line.
    let width = if b.line == a.line {
        (b.col.saturating_sub(a.col) as usize).max(1)
    } else {
        line.trim_end().chars().count().saturating_sub(start).max(1)
    };
    out.push_str(&format!("{pad} | {}{}", " ".repeat(start), marker.to_string().repeat(width)));
    if !text.is_empty() {
        out.push(' ');
        out.push_str(text);
    }
    out.push('\n');
}

/// The human form, modelled on rustc's multi-span layout.
pub fn render_human(map: &SourceMap, diags: &[Diagnostic]) -> String {
    let catalog = Catalog::builtin();
    let mut out = String::new();
    for d in diags {
        out.push_str(&format!("{}[{}]: {}\n", severity_word(d.severity()), d.code(), d.message()));
        let (a, _) = map.line_col(d.span);
        out.push_str(&format!("  --> {}:{}:{}\n", map.name(d.span.file), a.line, a.col));
        let gutter = std::iter::once(d.span)
            .chain(d.labels.iter().map(|l| l.span))
            .map(|s| map.line_col(s).0.line.to_string().len())
            .max()
            .unwrap_or(1);
        excerpt(&mut out, map, d.span, '^', "", gutter);
        for l in &d.labels {
            excerpt(&mut out, map, l.span, '-', &catalog.render(l.msg.id(), &l.params), gutter);
        }
        for n in &d.notes {
            out.push_str(&format!("  = note: {}\n", catalog.render(n.msg.id(), &n.params)));
        }
        for f in &d.fixes {
            // A deletion has no text to show; the fix's message says what goes.
            let edits: Vec<String> =
                f.edits.iter().filter(|e| !e.text.is_empty()).map(|e| format!("`{}`", e.text)).collect();
            let suffix = if edits.is_empty() { String::new() } else { format!(": {}", edits.join(", ")) };
            out.push_str(&format!("  = fix: {}{}\n", catalog.render(f.msg.id(), &f.params), suffix));
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg;
    use crate::span::TextRange;

    #[test]
    fn ids_form_is_stable() {
        let mut map = SourceMap::new();
        let f = map.add("a.sisu", "def main():\n    foo(1)\n");
        let d = Diagnostic::new(msg::RESOLVE_UNDEFINED_NAME, Span::new(f, TextRange::new(16, 19)))
            .with("name", "foo");
        assert_eq!(
            render_ids(&map, std::slice::from_ref(&d)),
            "2:5-2:8 E0200 resolve.undefined-name name=\"foo\"\n"
        );
        let human = render_human(&map, &[d]);
        assert!(human.contains("error[E0200]: cannot find `foo` in this scope"));
        assert!(human.contains("--> a.sisu:2:5"));
        assert!(human.contains("^^^"));
    }

    #[test]
    fn quoting_escapes() {
        assert_eq!(quote("a\"b\\c\n"), "\"a\\\"b\\\\c\\n\"");
    }
}
