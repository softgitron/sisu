//! Byte ranges in source text.

/// Identifies one source file in a [`crate::SourceMap`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct FileId(pub u32);

/// A half-open byte range `start..end` inside one file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct TextRange {
    pub start: u32,
    pub end: u32,
}

impl TextRange {
    pub fn new(start: u32, end: u32) -> TextRange {
        if end < start { TextRange { start: end, end: start } } else { TextRange { start, end } }
    }

    pub fn empty(at: u32) -> TextRange {
        TextRange { start: at, end: at }
    }

    pub fn len(self) -> u32 {
        self.end - self.start
    }

    pub fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// The smallest range covering both.
    pub fn cover(self, other: TextRange) -> TextRange {
        TextRange { start: self.start.min(other.start), end: self.end.max(other.end) }
    }

    pub fn contains(self, offset: u32) -> bool {
        self.start <= offset && offset < self.end
    }
}

/// A range in a specific file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Span {
    pub file: FileId,
    pub range: TextRange,
}

impl Span {
    pub fn new(file: FileId, range: TextRange) -> Span {
        Span { file, range }
    }

    pub fn cover(self, other: Span) -> Span {
        if self.file != other.file {
            return self;
        }
        Span { file: self.file, range: self.range.cover(other.range) }
    }

    /// The empty span at the start of this one.
    pub fn start(self) -> Span {
        Span { file: self.file, range: TextRange::empty(self.range.start) }
    }

    /// The empty span at the end of this one.
    pub fn end(self) -> Span {
        Span { file: self.file, range: TextRange::empty(self.range.end) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cover_and_order() {
        let a = TextRange::new(5, 2);
        assert_eq!(a, TextRange { start: 2, end: 5 });
        assert_eq!(a.cover(TextRange::new(7, 9)), TextRange::new(2, 9));
        assert!(a.contains(2) && !a.contains(5));
        assert!(TextRange::empty(3).is_empty());
    }
}
