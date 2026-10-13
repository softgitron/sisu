//! Source files and line/column conversion.
//!
//! Lines are 1-based. Columns are 1-based and count Unicode scalar values
//! from the start of the line, so a position means the same thing to a
//! front end written in any language (a decision recorded in
//! docs/architecture.md).

use crate::span::{FileId, Span, TextRange};

/// A 1-based line and column.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LineCol {
    pub line: u32,
    pub col: u32,
}

#[derive(Clone, Debug)]
pub struct SourceFile {
    pub id: FileId,
    /// The name used in messages, normally the path given on the command line.
    pub name: String,
    pub text: String,
    line_starts: Vec<u32>,
}

impl SourceFile {
    pub fn new(id: FileId, name: String, text: String) -> SourceFile {
        let mut line_starts = vec![0u32];
        for (i, b) in text.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(i as u32 + 1);
            }
        }
        SourceFile { id, name, text, line_starts }
    }

    pub fn line_col(&self, offset: u32) -> LineCol {
        let offset = offset.min(self.text.len() as u32);
        let line_index = match self.line_starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i - 1,
        };
        let start = self.line_starts[line_index] as usize;
        let end = (offset as usize).max(start);
        let col = self.text.get(start..end).map(|s| s.chars().count()).unwrap_or(end - start);
        LineCol { line: line_index as u32 + 1, col: col as u32 + 1 }
    }

    /// The text of a 1-based line, without its line break.
    pub fn line_text(&self, line: u32) -> &str {
        let index = (line as usize).saturating_sub(1);
        let Some(&start) = self.line_starts.get(index) else {
            return "";
        };
        let end = self.line_starts.get(index + 1).map(|&e| e as usize).unwrap_or(self.text.len());
        let s = self.text.get(start as usize..end).unwrap_or("");
        s.trim_end_matches(['\n', '\r'])
    }

    pub fn line_count(&self) -> u32 {
        self.line_starts.len() as u32
    }

    pub fn slice(&self, range: TextRange) -> &str {
        self.text.get(range.start as usize..range.end as usize).unwrap_or("")
    }
}

/// All files of one compilation.
#[derive(Clone, Debug, Default)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    pub fn new() -> SourceMap {
        SourceMap { files: Vec::new() }
    }

    pub fn add(&mut self, name: impl Into<String>, text: impl Into<String>) -> FileId {
        let id = FileId(self.files.len() as u32);
        self.files.push(SourceFile::new(id, name.into(), text.into()));
        id
    }

    pub fn get(&self, id: FileId) -> Option<&SourceFile> {
        self.files.get(id.0 as usize)
    }

    pub fn files(&self) -> &[SourceFile] {
        &self.files
    }

    pub fn name(&self, id: FileId) -> &str {
        self.get(id).map(|f| f.name.as_str()).unwrap_or("<unknown>")
    }

    pub fn line_col(&self, span: Span) -> (LineCol, LineCol) {
        match self.get(span.file) {
            Some(f) => (f.line_col(span.range.start), f.line_col(span.range.end)),
            None => (LineCol { line: 0, col: 0 }, LineCol { line: 0, col: 0 }),
        }
    }

    pub fn slice(&self, span: Span) -> &str {
        self.get(span.file).map(|f| f.slice(span.range)).unwrap_or("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_and_column_count_scalars() {
        let f = SourceFile::new(FileId(0), "t".into(), "ab\nä x\n".into());
        assert_eq!(f.line_col(0), LineCol { line: 1, col: 1 });
        assert_eq!(f.line_col(3), LineCol { line: 2, col: 1 });
        // 'ä' is two bytes; the space after it is column 2.
        assert_eq!(f.line_col(5), LineCol { line: 2, col: 2 });
        assert_eq!(f.line_text(2), "ä x");
        assert_eq!(f.line_col(100), LineCol { line: 3, col: 1 });
    }
}
