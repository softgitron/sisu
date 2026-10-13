//! Patterns.
//!
//! ```text
//! patterns  := pattern (',' pattern)*          -- a bare tuple in let and for
//! pattern   := primary_pattern ('|' primary_pattern)*
//! primary_pattern := '_' | lower_name | literal | '-' number
//!              | Upper_path ('(' pattern_arg (',' pattern_arg)* ')')?
//!              | '(' patterns ')' | '[' patterns? ']' | '*' name | '..'
//! pattern_arg := name '=' pattern | pattern | '..'
//! ```
//!
//! A lowercase name binds and a capitalised one names a variant or
//! constant (02, "Lexical structure"), so the parser decides by spelling.

use crate::parser::Parser;
use sisu_diag::msg;
use sisu_syntax::SyntaxKind::*;

impl Parser<'_> {
    pub(crate) fn pattern_list(&mut self) {
        let cp = self.checkpoint();
        self.pattern();
        if self.at(COMMA) {
            self.start_at(cp, TUPLE_PAT);
            while self.eat(COMMA) {
                if self.at_pattern_start() {
                    self.pattern();
                } else {
                    break;
                }
            }
            self.finish_node();
        }
    }

    fn at_pattern_start(&self) -> bool {
        matches!(
            self.current(),
            IDENT
                | INT_NUMBER
                | FLOAT_NUMBER
                | CHAR_LIT
                | STRING_START
                | RAW_STRING
                | MINUS
                | L_PAREN
                | L_BRACKET
                | STAR
                | DOT_DOT
                | SELF_TYPE_KW
        )
    }

    pub(crate) fn pattern(&mut self) {
        let cp = self.checkpoint();
        self.primary_pattern();
        if self.at(PIPE) {
            self.start_at(cp, OR_PAT);
            while self.eat(PIPE) {
                self.primary_pattern();
            }
            self.finish_node();
        }
    }

    fn primary_pattern(&mut self) {
        match self.current() {
            IDENT if self.current_text() == "_" => {
                self.start(WILDCARD_PAT);
                self.bump();
                self.finish_node();
            }
            IDENT | SELF_TYPE_KW => {
                let text = self.current_text();
                let capital = text.chars().next().is_some_and(|c| c.is_uppercase());
                if capital || self.nth(1) == DOT || self.at(SELF_TYPE_KW) {
                    self.start(CTOR_PAT);
                    self.bump();
                    while self.at(DOT) {
                        self.bump();
                        self.expect_name();
                    }
                    if self.at(L_PAREN) {
                        self.pattern_args();
                    }
                    self.finish_node();
                } else {
                    self.start(IDENT_PAT);
                    self.bump();
                    self.finish_node();
                }
            }
            INT_NUMBER | FLOAT_NUMBER | CHAR_LIT | RAW_STRING => {
                self.start(LITERAL_PAT);
                self.bump();
                self.finish_node();
            }
            STRING_START => {
                self.start(LITERAL_PAT);
                self.string_expr();
                self.finish_node();
            }
            MINUS if matches!(self.nth(1), INT_NUMBER | FLOAT_NUMBER) => {
                self.start(LITERAL_PAT);
                self.bump();
                self.bump();
                self.finish_node();
            }
            L_PAREN => {
                let cp = self.checkpoint();
                self.bump();
                let mut count = 0;
                let mut trailing = false;
                while !self.at_any(&[R_PAREN, NEWLINE, EOF]) {
                    self.pattern();
                    count += 1;
                    trailing = false;
                    if !self.eat(COMMA) {
                        break;
                    }
                    trailing = true;
                }
                let kind = if count == 1 && !trailing { PAREN_PAT } else { TUPLE_PAT };
                self.start_at(cp, kind);
                self.expect(R_PAREN);
                self.finish_node();
            }
            L_BRACKET => {
                self.start(LIST_PAT);
                self.bump();
                while !self.at_any(&[R_BRACKET, NEWLINE, EOF]) {
                    self.pattern();
                    if !self.eat(COMMA) {
                        break;
                    }
                }
                self.expect(R_BRACKET);
                self.finish_node();
            }
            STAR => {
                self.start(STAR_PAT);
                self.bump();
                self.expect_name();
                self.finish_node();
            }
            DOT_DOT => {
                self.start(REST_PAT);
                self.bump();
                self.finish_node();
            }
            _ => {
                self.error_found(msg::PARSE_EXPECTED_PATTERN);
                self.start(ERROR);
                if !self.at_any(&[NEWLINE, COLON, EQ, COMMA, R_PAREN, R_BRACKET, EOF, DEDENT, IN_KW, IF_KW]) {
                    self.bump();
                }
                self.finish_node();
            }
        }
    }

    fn pattern_args(&mut self) {
        self.start(PAT_ARGS);
        self.bump();
        while !self.at_any(&[R_PAREN, NEWLINE, EOF]) {
            self.start(PAT_ARG);
            if self.at(IDENT) && self.nth(1) == EQ {
                self.bump();
                self.bump();
                self.pattern();
            } else {
                self.pattern();
            }
            self.finish_node();
            if !self.eat(COMMA) {
                break;
            }
        }
        self.expect(R_PAREN);
        self.finish_node();
    }
}
