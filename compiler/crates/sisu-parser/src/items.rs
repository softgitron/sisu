//! Declarations.
//!
//! ```text
//! file      := item* EOF
//! item      := attribute* visibility? (fn_def | struct_def | enum_def | error_def
//!              | impl_def | trait_def | import | from_import | test_def | suite_def
//!              | bench_def | const_def | type_alias | extern_block)
//! attribute := '@' name ('(' args ')')? NEWLINE
//! fn_def    := ('pure' | 'comptime')? 'def' name generic_params? params ('->' type)?
//!              (':' block | NEWLINE)
//! struct_def:= 'view'? 'struct' name generic_params? ':' INDENT (field | fn_def | 'pass')+ DEDENT
//! field     := visibility? name ':' type ('=' expr)? NEWLINE
//! enum_def  := 'enum' name generic_params? ':' INDENT (variant | fn_def)+ DEDENT
//! error_def := 'error' name ':' INDENT (variant | '..' NEWLINE)+ DEDENT
//! variant   := name ('(' (name ':')? type (',' (name ':')? type)* ')')? NEWLINE
//! impl_def  := 'impl' generic_params? type ('for' type)? ':' INDENT fn_def+ DEDENT
//! import    := 'import' path ('as' name)? NEWLINE
//! from_import := 'from' path 'import' name ('as' name)? (',' name ('as' name)?)* NEWLINE
//! test_def  := 'test' string ('for' patterns 'in' expr)? params? ':' block
//! ```

use crate::parser::Parser;
use sisu_diag::{Diagnostic, msg};
use sisu_syntax::Checkpoint;
use sisu_syntax::SyntaxKind::*;

impl Parser<'_> {
    pub(crate) fn source_file(&mut self) {
        self.start_root(SOURCE_FILE);
        while !self.at(EOF) {
            match self.current() {
                NEWLINE | DEDENT => self.bump_error(),
                INDENT => self.unexpected_indent_block(true),
                _ => self.item(),
            }
        }
        self.bump_eof();
        self.finish_node();
    }

    /// Reports an unexpected INDENT and parses the indented lines as a
    /// nested block, keeping their structure.
    ///
    /// After an earlier syntax error, an unexpected block is almost always
    /// a consequence of it (the body of a declaration that failed to
    /// parse), so neither it nor anything inside it is reported.
    pub(crate) fn unexpected_indent_block(&mut self, items: bool) {
        let span = sisu_diag::Span::new(
            self.file,
            self.nth_token(1).map(|t| t.range).unwrap_or(self.current_range()),
        );
        let quiet = self.error_before();
        if quiet {
            self.quiet += 1;
        }
        self.report(Diagnostic::new(msg::PARSE_UNEXPECTED_INDENT, span));
        self.start(BLOCK);
        self.bump();
        while !self.at_any(&[DEDENT, EOF]) {
            let before = self.pos_marker();
            if items && self.at_item_start() {
                self.item();
            } else {
                self.statement();
            }
            if self.pos_marker() == before {
                self.bump_error();
            }
        }
        self.eat(DEDENT);
        self.finish_node();
        if quiet {
            self.quiet -= 1;
        }
    }

    pub(crate) fn at_item_start(&self) -> bool {
        match self.current() {
            AT | PUB_KW | DEF_KW | STRUCT_KW | VIEW_KW | ENUM_KW | ERROR_KW | IMPL_KW | TRAIT_KW
            | IMPORT_KW | FROM_KW | TEST_KW | CONST_KW | TYPE_KW | EXTERN_KW => true,
            COMPTIME_KW => self.nth(1) == DEF_KW,
            IDENT => {
                (self.current_text() == "pure" && self.nth(1) == DEF_KW)
                    || ((self.current_text() == "suite" || self.current_text() == "bench")
                        && self.nth(1) == STRING_START)
            }
            _ => false,
        }
    }

    /// One declaration; anything else is a top-level statement, reported.
    pub(crate) fn item(&mut self) {
        let cp = self.checkpoint();
        let mut prefixed = false;
        while self.at(AT) {
            self.attribute();
            prefixed = true;
        }
        if self.at(PUB_KW) {
            self.visibility();
            prefixed = true;
        }
        if !self.item_body(cp) {
            if prefixed {
                self.error_found(msg::PARSE_EXPECTED_DECLARATION);
                self.start_at(cp, ERROR);
                self.recover_line();
                self.finish_node();
            } else {
                let span = self.error_span();
                self.report(Diagnostic::new(msg::PARSE_TOP_LEVEL_STATEMENT, span));
                self.statement();
            }
        }
    }

    /// Parses the declaration at the current token, wrapping everything
    /// since `cp` (attributes, visibility). Returns false if there is none.
    pub(crate) fn item_body(&mut self, cp: Checkpoint) -> bool {
        match self.current() {
            DEF_KW => self.fn_def(cp),
            COMPTIME_KW if self.nth(1) == DEF_KW => self.fn_def(cp),
            IDENT if self.current_text() == "pure" && self.nth(1) == DEF_KW => self.fn_def(cp),
            STRUCT_KW | VIEW_KW => self.struct_def(cp),
            ENUM_KW => self.enum_like(cp, ENUM_DEF),
            ERROR_KW => self.enum_like(cp, ERROR_DEF),
            IMPL_KW => self.impl_def(cp),
            TRAIT_KW => self.trait_def(cp),
            IMPORT_KW => self.import(cp),
            FROM_KW => self.import_from(cp),
            TEST_KW => self.test_def(cp, TEST_DEF),
            IDENT if self.current_text() == "bench" && self.nth(1) == STRING_START => {
                self.test_def(cp, BENCH_DEF)
            }
            IDENT if self.current_text() == "suite" && self.nth(1) == STRING_START => self.suite_def(cp),
            CONST_KW => self.const_def(cp),
            TYPE_KW => self.type_alias(cp),
            EXTERN_KW => self.extern_block(cp),
            _ => return false,
        }
        true
    }

    fn attribute(&mut self) {
        self.start(ATTRIBUTE);
        self.bump();
        self.expect_name();
        while self.at(DOT) {
            self.bump();
            self.expect_name();
        }
        if self.at(L_PAREN) {
            self.in_attribute = true;
            self.arg_list();
            self.in_attribute = false;
        }
        self.end_stmt();
        self.finish_node();
    }

    fn visibility(&mut self) {
        self.start(VISIBILITY);
        self.bump();
        if self.at(L_PAREN) {
            self.bump();
            self.expect_name();
            self.expect(R_PAREN);
        }
        self.finish_node();
    }

    pub(crate) fn fn_def(&mut self, cp: Checkpoint) {
        self.start_at(cp, FN_DEF);
        let line = self.current_line();
        let (attempts, header_start) = (self.attempts, self.current_range().start);
        if self.at_contextual("pure") || self.at(COMPTIME_KW) {
            self.bump();
        }
        self.expect(DEF_KW);
        self.expect_name();
        if self.at(L_BRACKET) {
            self.generic_params();
        }
        if self.at(L_PAREN) {
            self.param_list();
        } else {
            self.expect(L_PAREN);
        }
        if self.at(ARROW) {
            self.start(RET_TYPE);
            self.bump();
            self.type_();
            if self.eat(FROM_KW) {
                // `-> ref str from a, b`: which parameters a borrow comes from;
                // `self` may be one of them.
                if !self.eat(SELF_KW) {
                    self.expect_name();
                }
                while self.eat(COMMA) {
                    if !self.eat(SELF_KW) {
                        self.expect_name();
                    }
                }
            }
            self.finish_node();
        }
        if self.at(WHERE_KW) {
            self.where_clause();
        }
        // A header with a mistake in it (including a missing `:`) gets an
        // empty ERROR node, so the tree says the signature is unreliable
        // even when recovery left no other trace of the mistake.
        let here = self.current_range().start;
        let colon_missing = !self.at(COLON) && !(self.at(NEWLINE) && self.nth(1) != INDENT);
        if self.attempts > attempts
            || colon_missing
            || self.lex_errors.iter().any(|&e| header_start <= e && e < here)
        {
            self.start(ERROR);
            self.finish_node();
        }
        if self.at(NEWLINE) && self.nth(1) != INDENT {
            // A signature without a body: an intrinsic or a required method.
            self.bump();
        } else {
            self.colon_block("def", line);
        }
        self.finish_node();
    }

    /// `where T: Bound + Other, U: Bound`, before the `:` of the body.
    fn where_clause(&mut self) {
        self.start(WHERE_CLAUSE);
        self.bump();
        loop {
            self.type_();
            self.expect(COLON);
            self.type_();
            while self.eat(PLUS) {
                self.type_();
            }
            if !self.eat(COMMA) {
                break;
            }
        }
        self.finish_node();
    }

    /// `:` and a block. A missing `:` before an indented block is reported
    /// and the block is still parsed, so its statements keep their place.
    pub(crate) fn colon_block(&mut self, keyword: &str, line: u32) {
        if self.eat(COLON) {
            self.block(keyword, line);
        } else if self.at(NEWLINE) && self.nth(1) == INDENT {
            self.expect(COLON);
            self.block(keyword, line);
        } else {
            self.expect(COLON);
            self.recover_line();
            if self.at(INDENT) {
                self.block_after_newline(keyword, line);
            }
        }
    }

    /// The colon form for declaration bodies.
    fn colon_members(&mut self, keyword: &str, line: u32, member: fn(&mut Self)) {
        if self.eat(COLON) {
            self.member_block(keyword, line, member);
        } else if self.at(NEWLINE) && self.nth(1) == INDENT {
            self.expect(COLON);
            self.member_block(keyword, line, member);
        } else {
            self.expect(COLON);
            self.recover_line();
        }
    }

    pub(crate) fn generic_params(&mut self) {
        self.start(GENERIC_PARAMS);
        self.bump();
        while !self.at_any(&[R_BRACKET, NEWLINE, EOF, COLON]) {
            self.start(GENERIC_PARAM);
            self.eat(CONST_KW);
            self.expect_name();
            if self.eat(COLON) {
                self.type_();
                while self.eat(PLUS) {
                    self.type_();
                }
            }
            if self.eat(EQ) {
                self.type_();
            }
            self.finish_node();
            if !self.eat(COMMA) {
                break;
            }
        }
        self.expect(R_BRACKET);
        self.finish_node();
    }

    pub(crate) fn param_list(&mut self) {
        self.start(PARAM_LIST);
        self.bump();
        while !self.at_any(&[R_PAREN, NEWLINE, EOF, COLON, DEDENT]) {
            self.param();
            if !self.eat(COMMA) {
                break;
            }
        }
        self.expect(R_PAREN);
        self.finish_node();
    }

    fn param(&mut self) {
        self.start(PARAM);
        if self.at(MUT_KW) || self.at(OWN_KW) {
            self.bump();
        }
        if self.at(SELF_KW) || self.at(STAR) || self.at(SLASH) {
            self.bump();
            if self.at(IDENT) && self.nth(1) == COLON {
                // `*xs: T` variadics.
                self.bump();
            }
            if self.eat(COLON) {
                self.type_();
            }
        } else {
            self.expect_name();
            if self.expect(COLON) {
                self.type_();
            }
            if self.eat(EQ) {
                self.expr();
            }
        }
        self.finish_node();
    }

    /// A block of declarations: struct, enum, impl and trait bodies.
    fn member_block(&mut self, keyword: &str, line: u32, member: fn(&mut Self)) {
        self.start(BLOCK);
        if self.at(NEWLINE) {
            self.bump();
            if self.at(INDENT) {
                self.bump();
                while !self.at_any(&[DEDENT, EOF]) {
                    let before = self.pos_marker();
                    member(self);
                    if self.pos_marker() == before {
                        self.bump_error();
                    }
                }
                self.eat(DEDENT);
                self.ended_with_block = true;
            } else {
                self.expected_block(keyword, line);
            }
        } else {
            let span = self.error_span();
            self.report(Diagnostic::new(msg::PARSE_BLOCK_ON_SAME_LINE, span));
            self.recover_line();
        }
        self.finish_node();
    }

    pub(crate) fn pos_marker(&self) -> (usize, usize) {
        (self.nth_token(0).map(|t| t.range.start as usize).unwrap_or(usize::MAX), self.nth(0) as usize)
    }

    pub(crate) fn expected_block(&mut self, keyword: &str, line: u32) {
        let span = self.error_span();
        let mut d = Diagnostic::new(msg::PARSE_EXPECTED_BLOCK, span)
            .with("keyword", keyword.to_string())
            .with("line", line.to_string());
        if let Some(at) = self.word_on_line(keyword, line) {
            d = d.label(
                sisu_diag::Span::new(self.file, at),
                msg::PARSE_OPENED_HERE,
                vec![("keyword", keyword.to_string())],
            );
        }
        self.report(d);
    }

    fn struct_def(&mut self, cp: Checkpoint) {
        self.start_at(cp, STRUCT_DEF);
        let line = self.current_line();
        self.eat(VIEW_KW);
        self.expect(STRUCT_KW);
        self.expect_name();
        if self.at(L_BRACKET) {
            self.generic_params();
        }
        // Once per body: a struct declared inside another one has a body
        // of its own, and the outer body's report stands after it.
        let outer_reported = std::mem::replace(&mut self.bad_member_reported, false);
        self.colon_members("struct", line, Self::struct_member);
        self.bad_member_reported = outer_reported;
        self.finish_node();
    }

    fn struct_member(&mut self) {
        let cp = self.checkpoint();
        while self.at(AT) {
            self.attribute();
        }
        if self.at(PUB_KW) {
            self.visibility();
        }
        if self.at(PASS_KW) {
            self.start_at(cp, PASS_STMT);
            self.bump();
            self.end_stmt();
            self.finish_node();
        } else if self.at(IDENT) && self.nth(1) == COLON {
            self.start_at(cp, FIELD_DEF);
            self.bump();
            self.bump();
            self.type_();
            if self.eat(EQ) {
                self.expr();
            }
            self.end_stmt();
            self.finish_node();
        } else if !self.item_body(cp) {
            // `invariant ...` is a contract (Phase 8); any other line is a
            // mistake, reported once per body and kept as a statement.
            if !self.at_contract() && !self.bad_member_reported {
                self.bad_member_reported = true;
                let span = self.error_span();
                self.report(Diagnostic::new(msg::PARSE_EXPECTED_MEMBER, span));
            }
            self.statement();
        }
    }

    fn enum_like(&mut self, cp: Checkpoint, kind: sisu_syntax::SyntaxKind) {
        self.start_at(cp, kind);
        let line = self.current_line();
        let keyword = if kind == ERROR_DEF { "error" } else { "enum" };
        self.bump();
        self.expect_name();
        if self.at(L_BRACKET) {
            self.generic_params();
        }
        self.colon_members(keyword, line, Self::variant_member);
        self.finish_node();
    }

    fn variant_member(&mut self) {
        let cp = self.checkpoint();
        while self.at(AT) {
            self.attribute();
        }
        if self.at(DOT_DOT) {
            self.start_at(cp, OPEN_MARKER);
            self.bump();
            self.end_stmt();
            self.finish_node();
        } else if self.at(PASS_KW) {
            self.start_at(cp, PASS_STMT);
            self.bump();
            self.end_stmt();
            self.finish_node();
        } else if self.at(IDENT) {
            self.start_at(cp, VARIANT_DEF);
            self.bump();
            if self.at(L_PAREN) {
                self.bump();
                while !self.at_any(&[R_PAREN, NEWLINE, EOF]) {
                    self.start(VARIANT_FIELD);
                    if self.at(IDENT) && self.nth(1) == COLON {
                        self.bump();
                        self.bump();
                    }
                    self.type_();
                    self.finish_node();
                    if !self.eat(COMMA) {
                        break;
                    }
                }
                self.expect(R_PAREN);
            }
            self.end_stmt();
            self.finish_node();
        } else if !self.item_body(cp) {
            self.error_found(msg::PARSE_EXPECTED_DECLARATION);
            self.recover_line();
        }
    }

    pub(crate) fn impl_def(&mut self, cp: Checkpoint) {
        self.start_at(cp, IMPL_DEF);
        let line = self.current_line();
        self.bump();
        if self.at(L_BRACKET) {
            self.generic_params();
        }
        self.type_();
        if self.eat(FOR_KW) {
            self.type_();
        }
        self.colon_members("impl", line, Self::impl_member);
        self.finish_node();
    }

    fn impl_member(&mut self) {
        let cp = self.checkpoint();
        while self.at(AT) {
            self.attribute();
        }
        if self.at(PUB_KW) {
            self.visibility();
        }
        if self.at(PASS_KW) {
            self.start_at(cp, PASS_STMT);
            self.bump();
            self.end_stmt();
            self.finish_node();
        } else if !self.item_body(cp) {
            self.error_found(msg::PARSE_EXPECTED_DECLARATION);
            self.recover_line();
        }
    }

    fn trait_def(&mut self, cp: Checkpoint) {
        self.start_at(cp, TRAIT_DEF);
        let line = self.current_line();
        self.bump();
        self.expect_name();
        if self.at(L_BRACKET) {
            self.generic_params();
        }
        if self.at(L_PAREN) {
            self.bump();
            while !self.at_any(&[R_PAREN, NEWLINE, EOF]) {
                self.type_();
                if !self.eat(COMMA) {
                    break;
                }
            }
            self.expect(R_PAREN);
        }
        self.colon_members("trait", line, Self::impl_member);
        self.finish_node();
    }

    fn module_path(&mut self) {
        self.start(MODULE_PATH);
        self.expect_name();
        while self.at(DOT) {
            self.bump();
            self.expect_name();
        }
        self.finish_node();
    }

    fn import(&mut self, cp: Checkpoint) {
        self.start_at(cp, IMPORT_ITEM);
        self.bump();
        if self.at_contextual("c") && self.nth(1) == STRING_START {
            // `import c "zlib.h" as zlib with link="z"` (12).
            self.bump();
            self.string_expr();
        } else {
            self.module_path();
        }
        if self.eat(AS_KW) {
            self.expect_name();
        }
        if self.eat(WITH_KW) {
            loop {
                self.start(ARG);
                self.expect_name();
                if self.expect(EQ) {
                    self.expr();
                }
                self.finish_node();
                if !self.eat(COMMA) {
                    break;
                }
            }
        }
        self.end_stmt();
        self.finish_node();
    }

    fn import_from(&mut self, cp: Checkpoint) {
        self.start_at(cp, FROM_IMPORT);
        self.bump();
        self.module_path();
        self.expect(IMPORT_KW);
        let parens = self.eat(L_PAREN);
        loop {
            self.start(IMPORT_NAME);
            self.expect_name();
            if self.eat(AS_KW) {
                self.expect_name();
            }
            self.finish_node();
            if !self.eat(COMMA) || self.at_any(&[R_PAREN, NEWLINE]) {
                break;
            }
        }
        if parens {
            self.expect(R_PAREN);
        }
        self.end_stmt();
        self.finish_node();
    }

    fn test_def(&mut self, cp: Checkpoint, kind: sisu_syntax::SyntaxKind) {
        self.start_at(cp, kind);
        let line = self.current_line();
        let keyword = if kind == BENCH_DEF { "bench" } else { "test" };
        self.bump();
        if self.at(STRING_START) {
            self.string_expr();
        } else {
            self.expect(STRING_START);
        }
        if self.eat(FOR_KW) {
            self.pattern_list();
            self.expect(IN_KW);
            self.expr();
        }
        if self.at(L_PAREN) {
            self.param_list();
        }
        self.colon_block(keyword, line);
        self.finish_node();
    }

    fn suite_def(&mut self, cp: Checkpoint) {
        self.start_at(cp, SUITE_DEF);
        let line = self.current_line();
        self.bump();
        self.string_expr();
        self.colon_members("suite", line, Self::item);
        self.finish_node();
    }

    pub(crate) fn const_def(&mut self, cp: Checkpoint) {
        self.start_at(cp, CONST_DEF);
        self.bump();
        self.expect_name();
        if self.eat(COLON) {
            self.type_();
        }
        if self.expect(EQ) {
            self.expr();
        }
        self.end_stmt();
        self.finish_node();
    }

    fn type_alias(&mut self, cp: Checkpoint) {
        self.start_at(cp, TYPE_ALIAS);
        self.bump();
        self.expect_name();
        if self.at(L_BRACKET) {
            self.generic_params();
        }
        // An associated type in a trait has no `=`.
        if self.eat(EQ) {
            self.type_();
        }
        if self.eat(WHERE_KW) {
            self.expr();
        }
        self.end_stmt();
        self.finish_node();
    }

    fn extern_block(&mut self, cp: Checkpoint) {
        self.start_at(cp, EXTERN_BLOCK);
        let line = self.current_line();
        self.bump();
        if self.at(STRING_START) {
            self.string_expr();
        }
        self.colon_members("extern", line, Self::impl_member);
        self.finish_node();
    }
}
