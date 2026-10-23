//! The raw scanner.

use sisu_diag::{Diagnostic, FileId, Span, TextRange, msg};
use sisu_syntax::Token;
use sisu_syntax::kinds::SyntaxKind::*;
use sisu_syntax::kinds::{PUNCTUATION, SyntaxKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Code,
    /// Inside `{...}` of a string; `depth` counts brackets opened inside.
    Interp {
        depth: u32,
        triple: bool,
    },
    /// Inside the text of a string that opened at `start`.
    Str {
        triple: bool,
        start: usize,
    },
}

struct Scanner<'a> {
    file: FileId,
    text: &'a str,
    pos: usize,
    tokens: Vec<Token>,
    modes: Vec<Mode>,
    diags: &'a mut Vec<Diagnostic>,
    last_significant: SyntaxKind,
}

/// The known escape letters after `\` in strings and characters.
const ESCAPES: &[char] = &['n', 't', 'r', '0', '\\', '"', '\'', 'u'];

/// Scans `text` into raw tokens. Line breaks are `LINE_BREAK` here; the
/// layout pass decides which of them end logical lines.
pub fn scan(file: FileId, text: &str, diags: &mut Vec<Diagnostic>) -> Vec<Token> {
    let mut s = Scanner {
        file,
        text,
        pos: 0,
        tokens: Vec::new(),
        modes: vec![Mode::Code],
        diags,
        last_significant: NEWLINE,
    };
    while s.pos < s.text.len() {
        let before = s.pos;
        match s.mode() {
            Mode::Str { triple, start } => s.string_text(triple, start),
            Mode::Code | Mode::Interp { .. } => s.code(),
        }
        if s.pos == before {
            // Never stall: consume one character as an error token.
            let c = s.peek().unwrap_or(' ');
            s.pos += c.len_utf8();
            s.push(ERROR_TOKEN, before);
        }
    }
    // Close any string left open at the end of the file.
    while let Some(mode) = s.modes.last().copied() {
        match mode {
            Mode::Code => break,
            Mode::Str { triple, start } => {
                let at = s.pos;
                s.error(
                    if triple { msg::LEX_UNTERMINATED_BLOCK_STRING } else { msg::LEX_UNTERMINATED_STRING },
                    start,
                    at,
                );
                s.push(STRING_END, at);
                s.modes.pop();
            }
            Mode::Interp { .. } => {
                let at = s.pos;
                s.push(INTERP_END, at);
                s.modes.pop();
            }
        }
    }
    s.tokens
}

impl Scanner<'_> {
    fn mode(&self) -> Mode {
        self.modes.last().copied().unwrap_or(Mode::Code)
    }

    fn peek(&self) -> Option<char> {
        self.text[self.pos..].chars().next()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.text.get(self.pos + offset..).and_then(|s| s.chars().next())
    }

    fn rest(&self) -> &str {
        &self.text[self.pos..]
    }

    fn push(&mut self, kind: SyntaxKind, start: usize) {
        self.tokens.push(Token { kind, range: TextRange::new(start as u32, self.pos as u32) });
        if !kind.is_trivia() {
            self.last_significant = kind;
        }
    }

    fn span(&self, start: usize, end: usize) -> Span {
        Span::new(self.file, TextRange::new(start as u32, end as u32))
    }

    fn error(&mut self, m: sisu_diag::MsgId, start: usize, end: usize) {
        let span = self.span(start, end);
        self.diags.push(Diagnostic::new(m, span));
    }

    fn error_with(
        &mut self,
        m: sisu_diag::MsgId,
        start: usize,
        end: usize,
        name: &'static str,
        value: String,
    ) {
        let span = self.span(start, end);
        self.diags.push(Diagnostic::new(m, span).with(name, value));
    }

    fn code(&mut self) {
        let start = self.pos;
        let Some(c) = self.peek() else { return };
        // Inside an interpolation of a one-line string, a line break ends
        // the string: report it, close the interpolation and the string,
        // and scan the line break as code, so that it ends the line.
        if (c == '\n' || c == '\r') && matches!(self.mode(), Mode::Interp { triple: false, .. }) {
            self.error(msg::LEX_NEWLINE_IN_INTERPOLATION, start, start);
            while matches!(self.mode(), Mode::Interp { .. } | Mode::Str { .. }) {
                let m = self.mode();
                self.modes.pop();
                self.push(if matches!(m, Mode::Interp { .. }) { INTERP_END } else { STRING_END }, start);
            }
        }
        match c {
            ' ' | '\t' | '\u{c}' => {
                while matches!(self.peek(), Some(' ' | '\t' | '\u{c}')) {
                    self.pos += 1;
                }
                self.push(WHITESPACE, start);
            }
            '\n' => {
                self.pos += 1;
                self.push(LINE_BREAK, start);
            }
            '\r' => {
                self.pos += 1;
                if self.peek() == Some('\n') {
                    self.pos += 1;
                }
                self.push(LINE_BREAK, start);
            }
            '#' => {
                let kind = if self.rest().starts_with("##!") {
                    MODULE_DOC_COMMENT
                } else if self.rest().starts_with("##") {
                    DOC_COMMENT
                } else {
                    COMMENT
                };
                while let Some(ch) = self.peek() {
                    if ch == '\n' || ch == '\r' {
                        break;
                    }
                    self.pos += ch.len_utf8();
                }
                self.push(kind, start);
            }
            '"' => self.string_start(start),
            '\'' => self.char_literal(start),
            c if c.is_ascii_digit() => self.number(start),
            c if c.is_alphabetic() || c == '_' => self.ident(start),
            _ => self.punctuation(start),
        }
    }

    fn ident(&mut self, start: usize) {
        while let Some(ch) = self.peek() {
            if ch.is_alphanumeric() || ch == '_' {
                self.pos += ch.len_utf8();
            } else {
                break;
            }
        }
        let word = &self.text[start..self.pos];
        if (word == "r" || word == "b") && self.peek() == Some('"') {
            self.prefixed_string(start, word == "r");
            return;
        }
        // After `.` or `?.` a keyword is an ordinary name (02, "Lexical
        // structure"), and so after `def`, which declares such a method
        // (`def or(own self, default: T)`, called as `x.or(d)`).
        let keyword_context = matches!(self.last_significant, DOT | QUESTION_DOT | DEF_KW);
        let kind = if keyword_context { IDENT } else { SyntaxKind::from_keyword(word).unwrap_or(IDENT) };
        self.push(kind, start);
    }

    fn number(&mut self, start: usize) {
        let after_dot = self.last_significant == DOT;
        let mut is_float = false;
        let mut valid = true;
        let radix_prefix = self.rest().get(..2).map(|p| p.to_ascii_lowercase());
        let radix = match radix_prefix.as_deref() {
            Some("0x") => 16,
            Some("0b") => 2,
            Some("0o") => 8,
            _ => 10,
        };
        if radix != 10 {
            self.pos += 2;
            let digits_start = self.pos;
            while let Some(ch) = self.peek() {
                if ch == '_' || ch.is_digit(radix) {
                    self.pos += 1;
                } else {
                    break;
                }
            }
            if self.text[digits_start..self.pos].chars().all(|c| c == '_') {
                valid = false;
            }
        } else {
            self.digits();
            if !after_dot && self.peek() == Some('.') && self.peek_at(1).is_some_and(|c| c.is_ascii_digit()) {
                is_float = true;
                self.pos += 1;
                self.digits();
            }
            if !after_dot && matches!(self.peek(), Some('e' | 'E')) {
                let sign = matches!(self.peek_at(1), Some('+' | '-'));
                let digit_at = if sign { 2 } else { 1 };
                if self.peek_at(digit_at).is_some_and(|c| c.is_ascii_digit()) {
                    is_float = true;
                    self.pos += digit_at;
                    self.digits();
                }
            }
        }
        // A suffix such as `u8` or `f32` is part of the literal.
        let suffix_start = self.pos;
        while let Some(ch) = self.peek() {
            if ch.is_alphanumeric() || ch == '_' {
                self.pos += ch.len_utf8();
            } else {
                break;
            }
        }
        let suffix = &self.text[suffix_start..self.pos];
        if !suffix.is_empty() {
            const SUFFIXES: &[&str] = &[
                "i8", "i16", "i32", "i64", "i128", "u8", "u16", "u32", "u64", "u128", "f32", "f64", "int",
                "uint",
            ];
            if !SUFFIXES.contains(&suffix) || (radix != 10 && suffix.starts_with('f')) {
                valid = false;
            } else if suffix.starts_with('f') {
                is_float = true;
            }
        }
        if !valid {
            let text = self.text[start..self.pos].to_string();
            self.error_with(msg::LEX_INVALID_NUMBER, start, self.pos, "text", text);
        }
        self.push(if is_float { FLOAT_NUMBER } else { INT_NUMBER }, start);
    }

    fn digits(&mut self) {
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() || ch == '_' {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    /// Consumes an escape sequence starting at `\`, reporting unknown ones.
    fn escape(&mut self) {
        let start = self.pos;
        self.pos += 1;
        let Some(c) = self.peek() else { return };
        if c == '\n' || c == '\r' {
            return;
        }
        self.pos += c.len_utf8();
        if !ESCAPES.contains(&c) {
            self.error_with(msg::LEX_INVALID_ESCAPE, start, self.pos, "escape", c.to_string());
            return;
        }
        if c == 'u' {
            // `{`, one to six hex digits naming a Unicode scalar value, and
            // `}`; otherwise the braces stay in the text.
            match sisu_syntax::ast::build::unicode_escape(self.rest()) {
                Some((_, len)) => self.pos += len,
                None => self.error_with(msg::LEX_INVALID_ESCAPE, start, self.pos, "escape", "u".into()),
            }
        }
    }

    fn char_literal(&mut self, start: usize) {
        self.pos += 1;
        let mut count = 0;
        let mut closed = false;
        while let Some(c) = self.peek() {
            match c {
                '\'' => {
                    self.pos += 1;
                    closed = true;
                    break;
                }
                '\n' | '\r' => break,
                '\\' => {
                    self.escape();
                    count += 1;
                }
                _ => {
                    self.pos += c.len_utf8();
                    count += 1;
                }
            }
        }
        if !closed {
            self.error(msg::LEX_UNTERMINATED_CHAR, start, self.pos);
        } else if count != 1 {
            self.error(msg::LEX_BAD_CHAR_LITERAL, start, self.pos);
        }
        self.push(CHAR_LIT, start);
    }

    fn string_start(&mut self, start: usize) {
        let triple = self.rest().starts_with("\"\"\"");
        self.pos += if triple { 3 } else { 1 };
        self.push(STRING_START, start);
        self.modes.push(Mode::Str { triple, start });
    }

    /// Raw `r"..."` and byte `b"..."` strings are single tokens.
    fn prefixed_string(&mut self, start: usize, raw: bool) {
        let triple = self.rest().starts_with("\"\"\"");
        let delim = if triple { "\"\"\"" } else { "\"" };
        self.pos += delim.len();
        let mut closed = false;
        while self.pos < self.text.len() {
            if self.rest().starts_with(delim) {
                self.pos += delim.len();
                closed = true;
                break;
            }
            let Some(c) = self.peek() else { break };
            if !triple && (c == '\n' || c == '\r') {
                break;
            }
            if c == '\\' && !raw {
                // A byte string also takes `\xNN`, any byte.
                let hex = self.text.get(self.pos + 1..self.pos + 4);
                if let Some(h) = hex
                    && h.starts_with('x')
                    && h[1..].chars().all(|d| d.is_ascii_hexdigit())
                {
                    self.pos += 4;
                } else {
                    self.escape();
                }
            } else {
                self.pos += c.len_utf8();
            }
        }
        if !closed {
            let m = if triple { msg::LEX_UNTERMINATED_BLOCK_STRING } else { msg::LEX_UNTERMINATED_STRING };
            self.error(m, start, self.pos);
        }
        self.push(if raw { RAW_STRING } else { BYTE_STRING }, start);
    }

    fn string_text(&mut self, triple: bool, opened: usize) {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if triple && self.rest().starts_with("\"\"\"") || !triple && c == '"' {
                break;
            }
            if !triple && (c == '\n' || c == '\r') {
                break;
            }
            if c == '{' {
                if self.peek_at(1) == Some('{') {
                    self.pos += 2;
                    continue;
                }
                break;
            }
            if c == '}' {
                if self.peek_at(1) == Some('}') {
                    self.pos += 2;
                    continue;
                }
                let at = self.pos;
                self.pos += 1;
                self.error(msg::LEX_LONE_CLOSE_BRACE, at, at + 1);
                continue;
            }
            if c == '\\' {
                self.escape();
                continue;
            }
            self.pos += c.len_utf8();
        }
        if self.pos > start {
            self.push(STRING_TEXT, start);
        }
        let here = self.pos;
        match self.peek() {
            None => {}
            Some('{') => {
                self.pos += 1;
                self.push(INTERP_START, here);
                self.modes.push(Mode::Interp { depth: 0, triple });
            }
            Some('\n' | '\r') => {
                // An unterminated one-line string: close it before the line
                // break, and report it from its opening quote. Brackets
                // left open before the quote and closed at the end of the
                // text were meant to follow it: `print("no end)` closes as
                // `print("no end")`.
                let line = self.text[..opened].rfind('\n').map_or(0, |i| i + 1);
                let mut open = 0i32;
                for b in self.text[line..opened].bytes() {
                    match b {
                        b'(' | b'[' => open += 1,
                        b')' | b']' => open -= 1,
                        _ => {}
                    }
                }
                let mut end = here;
                while open > 0 && end > start && matches!(self.text.as_bytes()[end - 1], b')' | b']') {
                    end -= 1;
                    open -= 1;
                }
                let at = self.span(end, end);
                let d = Diagnostic::new(msg::LEX_UNTERMINATED_STRING, self.span(opened, here)).fix(
                    msg::LEX_CLOSE_STRING,
                    Vec::new(),
                    vec![sisu_diag::Edit { span: at, text: "\"".into() }],
                );
                self.diags.push(d);
                self.push(STRING_END, here);
                self.modes.pop();
            }
            Some(_) => {
                self.pos += if triple { 3 } else { 1 };
                self.push(STRING_END, here);
                self.modes.pop();
            }
        }
    }

    fn punctuation(&mut self, start: usize) {
        if let Mode::Interp { depth, triple } = self.mode() {
            let c = self.peek();
            if depth == 0 && c == Some('}') {
                self.pos += 1;
                self.push(INTERP_END, start);
                self.modes.pop();
                return;
            }
            if depth == 0 && c == Some(':') {
                self.pos += 1;
                self.push(COLON, start);
                // The format spec runs to the closing brace.
                let spec_start = self.pos;
                while let Some(ch) = self.peek() {
                    if ch == '}' || ch == '"' || ch == '\n' || ch == '\r' {
                        break;
                    }
                    self.pos += ch.len_utf8();
                }
                self.push(FORMAT_SPEC, spec_start);
                return;
            }
            if c == Some('"') && depth == 0 {
                // A string ends inside an interpolation: close both.
                self.push(INTERP_END, start);
                self.modes.pop();
                return;
            }
            let delta: i32 = match c {
                Some('(' | '[' | '{') => 1,
                Some(')' | ']' | '}') => -1,
                _ => 0,
            };
            if delta != 0 {
                let new_depth = (depth as i32 + delta).max(0) as u32;
                if let Some(top) = self.modes.last_mut() {
                    *top = Mode::Interp { depth: new_depth, triple };
                }
            }
        }
        for (text, kind) in PUNCTUATION {
            if self.rest().starts_with(text) {
                self.pos += text.len();
                match kind {
                    SEMICOLON => self.error(msg::LEX_SEMICOLON, start, self.pos),
                    BACKSLASH => {
                        let continuation = matches!(self.peek(), Some('\n' | '\r'));
                        if continuation {
                            self.error(msg::LEX_BACKSLASH_CONTINUATION, start, self.pos);
                        } else {
                            self.error_with(msg::LEX_UNKNOWN_CHARACTER, start, self.pos, "char", "\\".into());
                        }
                    }
                    _ => {}
                }
                self.push(*kind, start);
                return;
            }
        }
        let c = self.peek().unwrap_or(' ');
        self.pos += c.len_utf8();
        self.error_with(msg::LEX_UNKNOWN_CHARACTER, start, self.pos, "char", c.to_string());
        self.push(ERROR_TOKEN, start);
    }
}
