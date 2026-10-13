//! The parser state and its token-level helpers.

use sisu_diag::{Diagnostic, FileId, MsgId, Span, TextRange, msg};
use sisu_syntax::SyntaxKind::*;
use sisu_syntax::{Checkpoint, SyntaxKind, Token, Tree, TreeBuilder};

pub(crate) struct Parser<'a> {
    pub(crate) file: FileId,
    text: &'a str,
    tokens: &'a [Token],
    /// Indices of the significant (non-trivia) tokens.
    sig: Vec<usize>,
    /// Position in `sig`.
    pos: usize,
    builder: TreeBuilder,
    diags: Vec<Diagnostic>,
    last_error_at: Option<usize>,
    /// Set when the last construct consumed a block, so the statement
    /// needs no NEWLINE of its own.
    pub(crate) ended_with_block: bool,
    /// Inside `@attr(...)`, keywords are plain names (`@cfg(test)`).
    pub(crate) in_attribute: bool,
    /// A struct body already has a line that is not a member, so the
    /// following ones are not reported again.
    pub(crate) bad_member_reported: bool,
    /// Byte offsets of the lexer's diagnostics.
    pub(crate) lex_errors: Vec<u32>,
    /// While positive, diagnostics are not reported (inside code that is
    /// already known to be misplaced).
    pub(crate) quiet: u32,
    /// Every diagnostic the parser tried to report, kept or not.
    pub(crate) attempts: u32,
    line_starts: Vec<u32>,
}

impl<'a> Parser<'a> {
    pub(crate) fn new(file: FileId, text: &'a str, tokens: &'a [Token]) -> Parser<'a> {
        let sig = tokens.iter().enumerate().filter(|(_, t)| !t.kind.is_trivia()).map(|(i, _)| i).collect();
        let mut line_starts = vec![0u32];
        for (i, b) in text.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(i as u32 + 1);
            }
        }
        Parser {
            file,
            text,
            tokens,
            sig,
            pos: 0,
            builder: TreeBuilder::new(text.to_string(), tokens),
            diags: Vec::new(),
            last_error_at: None,
            ended_with_block: false,
            in_attribute: false,
            bad_member_reported: false,
            lex_errors: Vec::new(),
            quiet: 0,
            attempts: 0,
            line_starts,
        }
    }

    pub(crate) fn finish(self) -> (Tree, Vec<Diagnostic>) {
        (self.builder.finish(), self.diags)
    }

    // ------------------------------------------------------------ looking

    pub(crate) fn nth_token(&self, n: usize) -> Option<Token> {
        self.sig.get(self.pos + n).map(|&i| self.tokens[i])
    }

    pub(crate) fn nth(&self, n: usize) -> SyntaxKind {
        self.nth_token(n).map(|t| t.kind).unwrap_or(EOF)
    }

    pub(crate) fn current(&self) -> SyntaxKind {
        self.nth(0)
    }

    pub(crate) fn at(&self, kind: SyntaxKind) -> bool {
        self.current() == kind
    }

    pub(crate) fn at_any(&self, kinds: &[SyntaxKind]) -> bool {
        kinds.contains(&self.current())
    }

    pub(crate) fn nth_text(&self, n: usize) -> &'a str {
        match self.nth_token(n) {
            Some(t) => self.text.get(t.range.start as usize..t.range.end as usize).unwrap_or(""),
            None => "",
        }
    }

    pub(crate) fn current_text(&self) -> &'a str {
        self.nth_text(0)
    }

    /// An identifier with the given text (a contextual keyword).
    pub(crate) fn at_contextual(&self, word: &str) -> bool {
        self.at(IDENT) && self.current_text() == word
    }

    pub(crate) fn current_range(&self) -> TextRange {
        self.nth_token(0).map(|t| t.range).unwrap_or(TextRange::empty(self.text.len() as u32))
    }

    /// The 1-based line of the current token.
    pub(crate) fn current_line(&self) -> u32 {
        self.line_of(self.current_range().start)
    }

    pub(crate) fn line_of(&self, offset: u32) -> u32 {
        match self.line_starts.binary_search(&offset) {
            Ok(i) => i as u32 + 1,
            Err(i) => i as u32,
        }
    }

    /// Whether the current token ends the logical line.
    pub(crate) fn at_line_end(&self) -> bool {
        self.at_any(&[NEWLINE, DEDENT, INDENT, EOF])
    }

    // ------------------------------------------------------------ building

    fn flush_trivia(&mut self) {
        let target = self.sig.get(self.pos).copied().unwrap_or(self.tokens.len());
        while self.builder.tokens_attached() < target {
            self.builder.token();
        }
    }

    pub(crate) fn start(&mut self, kind: SyntaxKind) {
        self.flush_trivia();
        self.builder.start_node(kind);
    }

    /// Opens the root node without flushing trivia, so that a file's leading
    /// comments belong to it.
    pub(crate) fn start_root(&mut self, kind: SyntaxKind) {
        self.builder.start_node(kind);
    }

    pub(crate) fn checkpoint(&mut self) -> Checkpoint {
        self.flush_trivia();
        self.builder.checkpoint()
    }

    pub(crate) fn start_at(&mut self, cp: Checkpoint, kind: SyntaxKind) {
        self.builder.start_node_at(cp, kind);
    }

    pub(crate) fn finish_node(&mut self) {
        self.builder.finish_node();
    }

    /// Attaches the current token (with the trivia before it) and advances.
    pub(crate) fn bump(&mut self) {
        if self.pos >= self.sig.len() {
            return;
        }
        let kind = self.current();
        if kind == EOF {
            // EOF is attached once, by `source_file`.
            return;
        }
        self.flush_trivia();
        self.builder.token();
        self.pos += 1;
        self.ended_with_block = false;
    }

    /// Attaches the EOF token and everything before it.
    pub(crate) fn bump_eof(&mut self) {
        while self.builder.tokens_attached() < self.tokens.len() {
            self.builder.token();
        }
        self.pos = self.sig.len();
    }

    pub(crate) fn eat(&mut self, kind: SyntaxKind) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    // ------------------------------------------------------------ errors

    /// The span for an error at the current token: the token itself, or for
    /// layout tokens the end of the previous significant token.
    pub(crate) fn error_span(&self) -> Span {
        let kind = self.current();
        if matches!(kind, NEWLINE | DEDENT | INDENT | EOF)
            && self.pos > 0
            && let Some(&prev) = self.sig.get(self.pos - 1)
        {
            return Span::new(self.file, TextRange::empty(self.tokens[prev].range.end));
        }
        Span::new(self.file, self.current_range())
    }

    pub(crate) fn report(&mut self, diag: Diagnostic) {
        self.attempts += 1;
        if self.last_error_at == Some(self.pos) || self.quiet > 0 {
            return;
        }
        self.last_error_at = Some(self.pos);
        self.diags.push(diag);
    }

    /// Whether a syntax error was found before the current token.
    pub(crate) fn error_before(&self) -> bool {
        let here = self.current_range().start;
        !self.diags.is_empty() || self.lex_errors.iter().any(|&e| e < here)
    }

    pub(crate) fn error(&mut self, m: MsgId) {
        let span = self.error_span();
        self.report(Diagnostic::new(m, span));
    }

    pub(crate) fn error_found(&mut self, m: MsgId) {
        let span = self.error_span();
        let found = self.found();
        self.report(Diagnostic::new(m, span).with("found", found));
    }

    pub(crate) fn found(&self) -> String {
        let kind = self.current();
        match kind {
            IDENT => format!("`{}`", self.current_text()),
            INT_NUMBER | FLOAT_NUMBER | CHAR_LIT => format!("`{}`", self.current_text()),
            _ => kind.describe(),
        }
    }

    /// Consumes `kind` or reports "expected `kind`, found ...".
    pub(crate) fn expect(&mut self, kind: SyntaxKind) -> bool {
        if self.eat(kind) {
            return true;
        }
        let span = self.error_span();
        let found = self.found();
        self.report(
            Diagnostic::new(msg::PARSE_EXPECTED, span).with("expected", kind.describe()).with("found", found),
        );
        false
    }

    /// Consumes a name, accepting a keyword with an error so the tree keeps
    /// its shape.
    pub(crate) fn expect_name(&mut self) -> bool {
        if self.eat(IDENT) {
            return true;
        }
        if self.current().is_keyword() {
            let kw = self.current().keyword_text().unwrap_or("");
            let span = self.error_span();
            self.report(Diagnostic::new(msg::PARSE_KEYWORD_AS_NAME, span).with("keyword", kw));
            self.bump();
            return true;
        }
        let span = self.error_span();
        let found = self.found();
        self.report(
            Diagnostic::new(msg::PARSE_EXPECTED, span).with("expected", "a name").with("found", found),
        );
        false
    }

    /// Wraps the current token in an ERROR node.
    pub(crate) fn bump_error(&mut self) {
        self.start(ERROR);
        self.bump();
        self.finish_node();
    }

    /// Skips to the end of the logical line: the tokens go into an ERROR
    /// node and the NEWLINE is consumed. An indented block that follows is
    /// kept, parsed as statements, so its structure is not lost.
    pub(crate) fn recover_line(&mut self) {
        if self.at_any(&[NEWLINE, DEDENT, EOF]) {
            self.eat(NEWLINE);
            return;
        }
        self.start(ERROR);
        while !self.at_any(&[NEWLINE, DEDENT, EOF, INDENT]) {
            self.bump();
        }
        self.finish_node();
        self.eat(NEWLINE);
    }

    /// Ends a simple statement: the NEWLINE, unless a block already ended it.
    pub(crate) fn end_stmt(&mut self) {
        if self.ended_with_block {
            return;
        }
        if self.eat(NEWLINE) || self.at_any(&[DEDENT, EOF]) {
            return;
        }
        let span = self.error_span();
        let found = self.found();
        self.report(Diagnostic::new(msg::PARSE_TRAILING_TOKENS, span).with("found", found));
        self.recover_line();
    }
}
