//! Types.
//!
//! ```text
//! type      := '!' error_set? | option ('!' error_set?)?
//! error_set := '{' option (',' option)* '}' | option
//! option    := ref_type '?'*
//! ref_type  := 'ref' 'mut'? primary_type | primary_type
//! primary_type := path generic_args? | 'Self' | 'pure'? '(' types? ')' ('->' type)?
//!              | 'dyn' path
//! generic_args := '[' (type | integer) (',' (type | integer))* ']'
//! ```
//!
//! `ref T?` is `(ref T)?`, an optional reference (02, "Types").

use crate::parser::Parser;
use sisu_diag::msg;
use sisu_syntax::SyntaxKind::*;

impl Parser<'_> {
    fn at_type_start(&self) -> bool {
        matches!(
            self.current(),
            IDENT | SELF_TYPE_KW | L_PAREN | REF_KW | DYN_KW | L_BRACE | IMPL_KW | TYPE_KW
        )
    }

    pub(crate) fn type_(&mut self) {
        let cp = self.checkpoint();
        if self.at(BANG) {
            self.start_at(cp, RESULT_TYPE);
            self.bump();
            if self.at_type_start() {
                self.error_set();
            }
            self.finish_node();
            return;
        }
        self.option_type();
        if self.at(BANG) {
            self.start_at(cp, RESULT_TYPE);
            self.bump();
            if self.at_type_start() {
                self.error_set();
            }
            self.finish_node();
        }
    }

    fn error_set(&mut self) {
        if self.at(L_BRACE) {
            self.start(ERROR_SET_TYPE);
            self.bump();
            while !self.at_any(&[R_BRACE, NEWLINE, EOF]) {
                self.option_type();
                if !self.eat(COMMA) {
                    break;
                }
            }
            self.expect(R_BRACE);
            self.finish_node();
        } else {
            self.option_type();
        }
    }

    fn option_type(&mut self) {
        let cp = self.checkpoint();
        self.ref_type();
        while self.at(QUESTION) {
            self.start_at(cp, OPTION_TYPE);
            self.bump();
            self.finish_node();
        }
    }

    fn ref_type(&mut self) {
        if self.at(REF_KW) {
            self.start(REF_TYPE);
            self.bump();
            self.eat(MUT_KW);
            self.primary_type();
            self.finish_node();
        } else {
            self.primary_type();
        }
    }

    fn primary_type(&mut self) {
        match self.current() {
            IDENT if self.current_text() == "pure" && self.nth(1) == L_PAREN => {
                self.start(FN_TYPE);
                self.bump();
                self.paren_types();
                self.expect(ARROW);
                self.type_();
                self.finish_node();
            }
            IDENT if self.current_text() == "ptr" && self.nth(1) == MUT_KW => {
                // `ptr mut T`, a raw pointer (04, "Unsafe").
                self.start(PTR_TYPE);
                self.bump();
                self.bump();
                self.primary_type();
                self.finish_node();
            }
            IMPL_KW => {
                // `impl Trait for T`, the result of a comptime derive.
                self.start(IMPL_TYPE);
                self.bump();
                self.type_();
                if self.eat(FOR_KW) {
                    self.type_();
                }
                self.finish_node();
            }
            IDENT | SELF_TYPE_KW | TYPE_KW => {
                self.start(PATH_TYPE);
                self.bump();
                while self.at(DOT) && self.nth(1) == IDENT {
                    self.bump();
                    self.bump();
                }
                if self.at(L_BRACKET) {
                    self.generic_args();
                }
                self.finish_node();
            }
            L_PAREN => {
                let cp = self.checkpoint();
                let (count, trailing_comma) = self.paren_types();
                if self.at(ARROW) {
                    self.start_at(cp, FN_TYPE);
                    self.bump();
                    self.type_();
                    self.finish_node();
                } else {
                    let kind = if count == 1 && !trailing_comma { PAREN_TYPE } else { TUPLE_TYPE };
                    self.start_at(cp, kind);
                    self.finish_node();
                }
            }
            DYN_KW => {
                self.start(DYN_TYPE);
                self.bump();
                self.primary_type();
                self.finish_node();
            }
            _ => {
                self.error_found(msg::PARSE_EXPECTED_TYPE);
                self.start(ERROR);
                if !self
                    .at_any(&[NEWLINE, COLON, EQ, COMMA, R_PAREN, R_BRACKET, R_BRACE, EOF, DEDENT, INDENT])
                {
                    self.bump();
                }
                self.finish_node();
            }
        }
    }

    /// `(A, b: B)`: the elements of a tuple or the parameters of a function
    /// type. Returns the element count and whether a comma trails.
    fn paren_types(&mut self) -> (usize, bool) {
        self.expect(L_PAREN);
        let mut count = 0;
        let mut trailing = false;
        while !self.at_any(&[R_PAREN, NEWLINE, EOF]) {
            if self.at(IDENT) && self.nth(1) == COLON {
                self.start(TUPLE_TYPE_FIELD);
                self.bump();
                self.bump();
                self.type_();
                self.finish_node();
            } else {
                self.type_();
            }
            count += 1;
            trailing = false;
            if !self.eat(COMMA) {
                break;
            }
            trailing = true;
        }
        self.expect(R_PAREN);
        (count, trailing)
    }

    fn generic_args(&mut self) {
        self.start(GENERIC_ARGS);
        self.bump();
        while !self.at_any(&[R_BRACKET, NEWLINE, EOF]) {
            if self.at(INT_NUMBER) {
                self.start(CONST_ARG);
                self.bump();
                self.finish_node();
            } else {
                self.type_();
            }
            if !self.eat(COMMA) {
                break;
            }
        }
        self.expect(R_BRACKET);
        self.finish_node();
    }
}
