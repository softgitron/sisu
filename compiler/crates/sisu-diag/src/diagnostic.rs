//! The diagnostic value: a message identifier, its parameters, a primary
//! span, labelled secondary spans, notes and fixes.

use crate::catalog::{Catalog, Kind};
use crate::msg::MsgId;
use crate::span::Span;

pub type Params = Vec<(&'static str, String)>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Error,
    Warning,
    Note,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Label {
    pub span: Span,
    pub msg: MsgId,
    pub params: Params,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    pub msg: MsgId,
    pub params: Params,
}

/// A replacement of `span` by `text`; `text` is source code, never prose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edit {
    pub span: Span,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fix {
    pub msg: MsgId,
    pub params: Params,
    pub edits: Vec<Edit>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub msg: MsgId,
    pub params: Params,
    pub span: Span,
    pub labels: Vec<Label>,
    pub notes: Vec<Note>,
    pub fixes: Vec<Fix>,
}

impl Diagnostic {
    pub fn new(msg: MsgId, span: Span) -> Diagnostic {
        Diagnostic { msg, params: Vec::new(), span, labels: Vec::new(), notes: Vec::new(), fixes: Vec::new() }
    }

    /// Adds a parameter.
    pub fn with(mut self, name: &'static str, value: impl Into<String>) -> Diagnostic {
        self.params.push((name, value.into()));
        self
    }

    pub fn label(mut self, span: Span, msg: MsgId, params: Params) -> Diagnostic {
        self.labels.push(Label { span, msg, params });
        self
    }

    pub fn note(mut self, msg: MsgId, params: Params) -> Diagnostic {
        self.notes.push(Note { msg, params });
        self
    }

    pub fn fix(mut self, msg: MsgId, params: Params, edits: Vec<Edit>) -> Diagnostic {
        self.fixes.push(Fix { msg, params, edits });
        self
    }

    pub fn severity(&self) -> Severity {
        match Catalog::builtin().kind(self.msg.id()) {
            Some(Kind::Warning) => Severity::Warning,
            Some(Kind::Note) | Some(Kind::Help) => Severity::Note,
            _ => Severity::Error,
        }
    }

    pub fn is_error(&self) -> bool {
        self.severity() == Severity::Error
    }

    pub fn code(&self) -> &'static str {
        Catalog::builtin().code(self.msg.id()).unwrap_or("E????")
    }

    pub fn message(&self) -> String {
        Catalog::builtin().render(self.msg.id(), &self.params)
    }

    pub fn param(&self, name: &str) -> Option<&str> {
        self.params.iter().find(|(k, _)| *k == name).map(|(_, v)| v.as_str())
    }
}

/// Sorts diagnostics by position, then code, for stable output.
pub fn sort(diags: &mut [Diagnostic]) {
    diags.sort_by(|a, b| {
        (a.span.file, a.span.range.start, a.span.range.end, a.code()).cmp(&(
            b.span.file,
            b.span.range.start,
            b.span.range.end,
            b.code(),
        ))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg;
    use crate::span::{FileId, TextRange};

    #[test]
    fn builds_and_renders() {
        let span = Span::new(FileId(0), TextRange::new(1, 4));
        let d = Diagnostic::new(msg::RESOLVE_UNDEFINED_NAME, span).with("name", "foo");
        assert_eq!(d.code(), "E0200");
        assert_eq!(d.message(), "cannot find `foo` in this scope");
        assert!(d.is_error());
        let w = Diagnostic::new(msg::TYPE_HOLE, span).with("name", "x").with("ty", "int");
        assert_eq!(w.severity(), Severity::Warning);
    }
}
