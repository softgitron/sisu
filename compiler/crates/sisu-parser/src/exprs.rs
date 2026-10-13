//! Expressions, by precedence from lowest to highest (02, "Operator
//! precedence"):
//!
//! ```text
//! expr      := closure | coalesce
//! closure   := 'own'? (name | '(' params ')') '->' expr
//! coalesce  := or ('??' coalesce)?
//! or        := and ('or' and)*            -- mixing with `and` needs parentheses
//! and       := not ('and' not)*
//! not       := 'not' not | compare
//! compare   := bitwise (cmp_op bitwise)*  -- chains point one way
//! bitwise   := shift (('&' | '^' | '|') shift)*   -- one operator, no arithmetic
//! shift     := additive (('<<' | '>>') additive)*  -- no arithmetic operands
//! additive  := mult (('+' | '-' | '+%' | '-%' | '+|' | '-|') mult)*
//! mult      := unary (('*' | '/' | '//' | '%' | '*%' | '*|') unary)*
//! unary     := ('-' | '~' | 'ref' 'mut'? | '^') unary | power
//! power     := postfix ('**' unary)?      -- `-x ** y` needs parentheses
//! postfix   := primary (call | index | '.' name | '?.' name | '?' | do_block)*
//! primary   := literal | string | name | '(' ... ')' | '[' ... ']' | '{' ... '}'
//!              | if_expr | match_expr
//! ```

use crate::parser::Parser;
use sisu_diag::{Diagnostic, msg};
use sisu_syntax::SyntaxKind;
use sisu_syntax::SyntaxKind::*;

/// The outermost operator of an unparenthesised expression, for the
/// mixing rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Class {
    Atom,
    Closure,
    Coalesce,
    Or,
    And,
    Not,
    Compare,
    Bitwise,
    Shift,
    Additive,
    Multiplicative,
    Unary,
    Power,
}

const ADDITIVE: &[SyntaxKind] = &[PLUS, MINUS, PLUS_PERCENT, MINUS_PERCENT, PLUS_PIPE, MINUS_PIPE];
const MULTIPLICATIVE: &[SyntaxKind] = &[STAR, SLASH, SLASH_SLASH, PERCENT, STAR_PERCENT, STAR_PIPE];
const BITWISE: &[SyntaxKind] = &[AMP, PIPE, CARET];

/// Tokens that never start an expression and where a missing expression
/// should not consume anything.
const EXPR_SYNC: &[SyntaxKind] = &[
    NEWLINE, DEDENT, INDENT, EOF, COLON, R_PAREN, R_BRACKET, R_BRACE, COMMA, EQ, INTERP_END, ELSE_KW,
    ELIF_KW, CASE_KW, STRING_END, ARROW,
];

impl Parser<'_> {
    pub(crate) fn at_expr_start(&self) -> bool {
        matches!(
            self.current(),
            IDENT
                | SELF_KW
                | SELF_TYPE_KW
                | INT_NUMBER
                | FLOAT_NUMBER
                | CHAR_LIT
                | STRING_START
                | RAW_STRING
                | BYTE_STRING
                | L_PAREN
                | L_BRACKET
                | L_BRACE
                | MINUS
                | TILDE
                | NOT_KW
                | REF_KW
                | IF_KW
                | MATCH_KW
                | OWN_KW
                | CARET
                | COMPTIME_KW
        )
    }

    pub(crate) fn expr(&mut self) -> Class {
        if self.at_closure_start() {
            self.closure();
            return Class::Closure;
        }
        self.coalesce()
    }

    fn at_closure_start(&self) -> bool {
        match self.current() {
            OWN_KW => true,
            IDENT => self.nth(1) == ARROW,
            L_PAREN => {
                let mut depth = 0;
                let mut i = 0;
                loop {
                    match self.nth(i) {
                        L_PAREN | L_BRACKET | L_BRACE => depth += 1,
                        R_PAREN | R_BRACKET | R_BRACE => {
                            depth -= 1;
                            if depth == 0 {
                                return self.nth(i + 1) == ARROW;
                            }
                        }
                        NEWLINE | EOF | DEDENT | INDENT => return false,
                        _ => {}
                    }
                    i += 1;
                }
            }
            _ => false,
        }
    }

    fn closure(&mut self) {
        self.start(CLOSURE_EXPR);
        self.eat(OWN_KW);
        self.closure_params();
        self.expect(ARROW);
        self.expr();
        self.finish_node();
    }

    /// `x`, `(a, b)` or `(a: int, b: int)`.
    pub(crate) fn closure_params(&mut self) {
        self.start(CLOSURE_PARAMS);
        if self.at(IDENT) {
            self.start(CLOSURE_PARAM);
            self.bump();
            self.finish_node();
            // `do acc, x:` lists parameters without parentheses.
            while self.at(COMMA) && self.nth(1) == IDENT {
                self.bump();
                self.start(CLOSURE_PARAM);
                self.bump();
                self.finish_node();
            }
        } else if self.eat(L_PAREN) {
            while !self.at_any(&[R_PAREN, NEWLINE, EOF]) {
                self.start(CLOSURE_PARAM);
                if !self.eat(IDENT) {
                    self.error_found(msg::PARSE_CLOSURE_PARAMETER);
                    if !self.at_any(EXPR_SYNC) {
                        self.bump();
                    }
                }
                if self.eat(COLON) {
                    self.type_();
                }
                self.finish_node();
                if !self.eat(COMMA) {
                    break;
                }
            }
            self.expect(R_PAREN);
        } else {
            self.error_found(msg::PARSE_CLOSURE_PARAMETER);
        }
        self.finish_node();
    }

    fn coalesce(&mut self) -> Class {
        let cp = self.checkpoint();
        let c = self.or_expr();
        if self.at(QUESTION_QUESTION) {
            self.start_at(cp, BIN_EXPR);
            self.bump();
            self.coalesce();
            self.finish_node();
            return Class::Coalesce;
        }
        c
    }

    fn or_expr(&mut self) -> Class {
        let cp = self.checkpoint();
        let mut c = self.and_expr();
        while self.at(OR_KW) {
            if c == Class::And {
                self.error(msg::PARSE_MIXED_AND_OR);
            }
            self.start_at(cp, BIN_EXPR);
            self.bump();
            if self.and_expr() == Class::And {
                self.error(msg::PARSE_MIXED_AND_OR);
            }
            self.finish_node();
            c = Class::Or;
        }
        c
    }

    fn and_expr(&mut self) -> Class {
        let cp = self.checkpoint();
        let mut c = self.not_expr();
        while self.at(AND_KW) {
            self.start_at(cp, BIN_EXPR);
            self.bump();
            self.not_expr();
            self.finish_node();
            c = Class::And;
        }
        c
    }

    fn not_expr(&mut self) -> Class {
        if self.at(NOT_KW) && self.nth(1) != IN_KW {
            self.start(PREFIX_EXPR);
            self.bump();
            self.not_expr();
            self.finish_node();
            return Class::Not;
        }
        self.compare()
    }

    /// The comparison operator at the current token, and its token count.
    fn at_compare_op(&self) -> Option<(SyntaxKind, usize)> {
        match self.current() {
            EQ_EQ | BANG_EQ | LT | LT_EQ | GT | GT_EQ | IN_KW => Some((self.current(), 1)),
            NOT_KW if self.nth(1) == IN_KW => Some((NOT_KW, 2)),
            _ => None,
        }
    }

    fn compare(&mut self) -> Class {
        let cp = self.checkpoint();
        let c = self.bitwise();
        let Some(_) = self.at_compare_op() else { return c };
        self.start_at(cp, COMPARE_EXPR);
        let mut ops: Vec<SyntaxKind> = Vec::new();
        while let Some((op, n)) = self.at_compare_op() {
            if let Some(&prev) = ops.last() {
                self.check_chain(prev, op);
            }
            ops.push(op);
            for _ in 0..n {
                self.bump();
            }
            self.bitwise();
        }
        self.finish_node();
        Class::Compare
    }

    fn check_chain(&mut self, prev: SyntaxKind, op: SyntaxKind) {
        let direction = |k: SyntaxKind| match k {
            LT | LT_EQ => Some(1),
            GT | GT_EQ => Some(-1),
            EQ_EQ => Some(0),
            _ => None,
        };
        let text = |k: SyntaxKind| match k {
            NOT_KW => "not in".to_string(),
            k => k.punctuation_text().or(k.keyword_text()).unwrap_or("?").to_string(),
        };
        match (direction(prev), direction(op)) {
            (Some(a), Some(b)) if a == b => {}
            (Some(_), Some(_)) => {
                let span = self.error_span();
                self.report(
                    Diagnostic::new(msg::PARSE_MIXED_CHAIN, span)
                        .with("first", text(prev))
                        .with("second", text(op)),
                );
            }
            _ => {
                let bad = if direction(prev).is_none() { prev } else { op };
                let span = self.error_span();
                self.report(Diagnostic::new(msg::PARSE_CHAIN_OPERATOR, span).with("op", text(bad)));
            }
        }
    }

    fn bitwise(&mut self) -> Class {
        let cp = self.checkpoint();
        let mut c = self.shift();
        let mut first: Option<SyntaxKind> = None;
        while self.at_any(BITWISE) {
            let op = self.current();
            let op_text = op.punctuation_text().unwrap_or("?");
            if let Some(f) = first
                && f != op
            {
                let span = self.error_span();
                self.report(
                    Diagnostic::new(msg::PARSE_MIXED_BITWISE, span)
                        .with("first", f.punctuation_text().unwrap_or("?"))
                        .with("second", op_text),
                );
            }
            first = Some(op);
            if is_arithmetic(c) {
                let span = self.error_span();
                self.report(Diagnostic::new(msg::PARSE_BITWISE_ARITHMETIC, span).with("op", op_text));
            }
            self.start_at(cp, BIN_EXPR);
            self.bump();
            if is_arithmetic(self.shift()) {
                let span = self.error_span();
                self.report(Diagnostic::new(msg::PARSE_BITWISE_ARITHMETIC, span).with("op", op_text));
            }
            self.finish_node();
            c = Class::Bitwise;
        }
        c
    }

    fn shift(&mut self) -> Class {
        let cp = self.checkpoint();
        let mut c = self.additive();
        while self.at_any(&[SHL, SHR]) {
            if matches!(c, Class::Additive | Class::Multiplicative) {
                self.error(msg::PARSE_SHIFT_ARITHMETIC);
            }
            self.start_at(cp, BIN_EXPR);
            self.bump();
            if matches!(self.additive(), Class::Additive | Class::Multiplicative) {
                self.error(msg::PARSE_SHIFT_ARITHMETIC);
            }
            self.finish_node();
            c = Class::Shift;
        }
        c
    }

    fn additive(&mut self) -> Class {
        let cp = self.checkpoint();
        let mut c = self.multiplicative();
        while self.at_any(ADDITIVE) {
            self.start_at(cp, BIN_EXPR);
            self.bump();
            self.multiplicative();
            self.finish_node();
            c = Class::Additive;
        }
        c
    }

    fn multiplicative(&mut self) -> Class {
        let cp = self.checkpoint();
        let mut c = self.unary();
        while self.at_any(MULTIPLICATIVE) {
            self.start_at(cp, BIN_EXPR);
            self.bump();
            self.unary();
            self.finish_node();
            c = Class::Multiplicative;
        }
        c
    }

    fn unary(&mut self) -> Class {
        match self.current() {
            MINUS | TILDE | CARET => {
                let op = self.current();
                self.start(PREFIX_EXPR);
                self.bump();
                let inner = self.unary();
                if op == MINUS && inner == Class::Power {
                    self.report_negative_power();
                }
                self.finish_node();
                Class::Unary
            }
            REF_KW => {
                self.start(PREFIX_EXPR);
                self.bump();
                self.eat(MUT_KW);
                self.unary();
                self.finish_node();
                Class::Unary
            }
            COMPTIME_KW | UNSAFE_KW => {
                // `comptime e` and `unsafe e` mark one expression.
                self.start(PREFIX_EXPR);
                self.bump();
                self.unary();
                self.finish_node();
                Class::Unary
            }
            _ => self.power(),
        }
    }

    fn report_negative_power(&mut self) {
        let span = self.error_span();
        self.report(Diagnostic::new(msg::PARSE_NEGATIVE_POWER, span));
    }

    fn power(&mut self) -> Class {
        let cp = self.checkpoint();
        let c = self.postfix();
        if self.at(STAR_STAR) {
            self.start_at(cp, BIN_EXPR);
            self.bump();
            self.unary();
            self.finish_node();
            return Class::Power;
        }
        c
    }

    fn postfix(&mut self) -> Class {
        let cp = self.checkpoint();
        let c = self.primary();
        let mut c = c;
        loop {
            match self.current() {
                L_PAREN => {
                    self.start_at(cp, CALL_EXPR);
                    self.arg_list();
                    let ended = self.at(DO_KW);
                    if ended {
                        self.do_block();
                    }
                    self.finish_node();
                    if ended {
                        return Class::Atom;
                    }
                }
                L_BRACKET => {
                    self.start_at(cp, INDEX_EXPR);
                    self.index_args();
                    self.finish_node();
                }
                DOT => {
                    self.start_at(cp, FIELD_EXPR);
                    self.bump();
                    if self.eat(L_BRACKET) {
                        // `x.[f]`: a field named by a compile-time value.
                        self.expr();
                        self.expect(R_BRACKET);
                    } else if !self.eat(INT_NUMBER) {
                        self.expect_name();
                    }
                    self.finish_node();
                }
                QUESTION_DOT => {
                    self.start_at(cp, OPT_FIELD_EXPR);
                    self.bump();
                    self.expect_name();
                    self.finish_node();
                }
                QUESTION => {
                    self.start_at(cp, TRY_EXPR);
                    self.bump();
                    self.finish_node();
                }
                DO_KW => {
                    self.start_at(cp, CALL_EXPR);
                    self.do_block();
                    self.finish_node();
                    return Class::Atom;
                }
                _ => break,
            }
            c = Class::Atom;
        }
        c
    }

    pub(crate) fn arg_list(&mut self) {
        self.start(ARG_LIST);
        self.bump();
        while !self.at_any(&[R_PAREN, EOF, NEWLINE, DEDENT]) {
            self.start(ARG);
            if self.at(IDENT) && self.nth(1) == EQ {
                self.bump();
                self.bump();
            }
            self.eat(MUT_KW);
            self.expr();
            self.finish_node();
            if !self.eat(COMMA) {
                break;
            }
        }
        self.expect(R_PAREN);
        self.finish_node();
    }

    fn index_args(&mut self) {
        self.bump();
        while !self.at_any(&[R_BRACKET, EOF, NEWLINE]) {
            let cp = self.checkpoint();
            if self.at(COLON) {
                self.start_at(cp, SLICE);
                self.bump();
                if !self.at_any(&[R_BRACKET, COMMA]) {
                    self.expr();
                }
                self.finish_node();
            } else {
                self.expr();
                if self.at(COLON) {
                    self.start_at(cp, SLICE);
                    self.bump();
                    if !self.at_any(&[R_BRACKET, COMMA]) {
                        self.expr();
                    }
                    self.finish_node();
                }
            }
            if !self.eat(COMMA) {
                break;
            }
        }
        self.expect(R_BRACKET);
    }

    /// `do [own] [params]:` and an indented body: a closure bound to the
    /// call's last function-typed parameter.
    fn do_block(&mut self) {
        self.start(DO_BLOCK);
        let line = self.current_line();
        self.bump();
        self.eat(OWN_KW);
        if !self.at(COLON) {
            self.closure_params();
        }
        if self.expect(COLON) {
            self.block("do", line);
        }
        self.finish_node();
    }

    fn primary(&mut self) -> Class {
        match self.current() {
            INT_NUMBER | FLOAT_NUMBER | CHAR_LIT | RAW_STRING | BYTE_STRING => {
                self.start(LITERAL);
                self.bump();
                self.finish_node();
            }
            STRING_START => self.string_expr(),
            IDENT | SELF_KW | SELF_TYPE_KW => {
                self.start(NAME_REF);
                self.bump();
                self.finish_node();
            }
            L_PAREN => self.paren_or_tuple(),
            L_BRACKET => self.list_or_comprehension(),
            L_BRACE => self.dict_or_set(),
            IF_KW => self.if_expr(),
            MATCH_KW => self.match_expr(),
            k if self.in_attribute && k.is_keyword() => {
                self.start(NAME_REF);
                self.bump();
                self.finish_node();
            }
            _ => {
                self.error_found(msg::PARSE_EXPECTED_EXPRESSION);
                self.start(ERROR);
                if !self.at_any(EXPR_SYNC) && !self.current().is_keyword() {
                    self.bump();
                }
                self.finish_node();
            }
        }
        Class::Atom
    }

    pub(crate) fn string_expr(&mut self) {
        self.start(STRING_EXPR);
        self.bump();
        loop {
            match self.current() {
                STRING_TEXT => self.bump(),
                INTERP_START => self.interpolation(),
                STRING_END => {
                    self.bump();
                    break;
                }
                _ => break,
            }
        }
        self.finish_node();
    }

    fn interpolation(&mut self) {
        self.start(INTERPOLATION);
        self.bump();
        if self.at(INTERP_END) || self.at(COLON) {
            self.error(msg::PARSE_EMPTY_INTERPOLATION);
        } else {
            self.expr();
        }
        self.eat(EQ);
        if self.eat(COLON) {
            self.eat(FORMAT_SPEC);
        }
        if !self.at(INTERP_END) {
            self.error_found(msg::PARSE_EXPECTED_EXPRESSION);
            self.start(ERROR);
            while !self.at_any(&[INTERP_END, STRING_END, EOF, NEWLINE]) {
                self.bump();
            }
            self.finish_node();
        }
        self.eat(INTERP_END);
        self.finish_node();
    }

    fn paren_or_tuple(&mut self) {
        let cp = self.checkpoint();
        self.bump();
        if self.at(R_PAREN) {
            self.start_at(cp, TUPLE_EXPR);
            self.bump();
            self.finish_node();
            return;
        }
        self.expr();
        let kind = if self.at(FOR_KW) {
            self.comp_clauses();
            COMPREHENSION
        } else if self.at(COMMA) {
            while self.eat(COMMA) {
                if self.at(R_PAREN) {
                    break;
                }
                self.expr();
            }
            TUPLE_EXPR
        } else {
            PAREN_EXPR
        };
        self.start_at(cp, kind);
        self.expect(R_PAREN);
        self.finish_node();
    }

    fn list_or_comprehension(&mut self) {
        let cp = self.checkpoint();
        self.bump();
        let mut kind = LIST_EXPR;
        if !self.at(R_BRACKET) {
            self.expr();
            if self.at(FOR_KW) {
                self.comp_clauses();
                kind = COMPREHENSION;
            } else {
                while self.eat(COMMA) {
                    if self.at(R_BRACKET) {
                        break;
                    }
                    self.expr();
                }
            }
        }
        self.start_at(cp, kind);
        self.expect(R_BRACKET);
        self.finish_node();
    }

    fn dict_or_set(&mut self) {
        let cp = self.checkpoint();
        self.bump();
        let mut kind = DICT_EXPR;
        if !self.at(R_BRACE) {
            let is_dict = self.dict_or_set_element(None);
            kind = if is_dict { DICT_EXPR } else { SET_EXPR };
            if self.at(FOR_KW) {
                self.comp_clauses();
                kind = COMPREHENSION;
            } else {
                while self.eat(COMMA) {
                    if self.at(R_BRACE) {
                        break;
                    }
                    self.dict_or_set_element(Some(is_dict));
                }
            }
        }
        self.start_at(cp, kind);
        self.expect(R_BRACE);
        self.finish_node();
    }

    /// One element of a `{...}` literal; returns whether it was `k: v`.
    fn dict_or_set_element(&mut self, dict: Option<bool>) -> bool {
        let cp = self.checkpoint();
        self.expr();
        if self.at(COLON) {
            self.start_at(cp, DICT_ENTRY);
            self.bump();
            self.expr();
            self.finish_node();
            if dict == Some(false) {
                self.error_found(msg::PARSE_EXPECTED_EXPRESSION);
            }
            true
        } else {
            if dict == Some(true) {
                self.expect(COLON);
            }
            false
        }
    }

    fn comp_clauses(&mut self) {
        while self.at(FOR_KW) || self.at(IF_KW) {
            if self.at(FOR_KW) {
                self.start(COMP_FOR);
                self.bump();
                self.pattern_list();
                self.expect(IN_KW);
                self.or_expr();
                self.finish_node();
            } else {
                self.start(COMP_IF);
                self.bump();
                self.or_expr();
                self.finish_node();
            }
        }
    }

    /// `if c: a elif d: b else: e` on one line.
    fn if_expr(&mut self) {
        self.start(IF_EXPR);
        self.bump();
        self.expr();
        self.expect(COLON);
        self.expr();
        while self.at(ELIF_KW) {
            self.bump();
            self.expr();
            self.expect(COLON);
            self.expr();
        }
        if self.eat(ELSE_KW) {
            self.expect(COLON);
            self.expr();
        } else {
            self.error(msg::PARSE_IF_EXPRESSION_ELSE);
        }
        self.finish_node();
    }

    /// `match e:` and an indented list of `case` arms. In statement
    /// position the arms may hold blocks; in an expression each arm is one
    /// expression (checked later).
    pub(crate) fn match_expr(&mut self) {
        self.start(MATCH_EXPR);
        let line = self.current_line();
        self.bump();
        self.expr();
        if self.expect(COLON) {
            self.start(MATCH_ARM_LIST);
            if self.at(NEWLINE) {
                self.bump();
                if self.at(INDENT) {
                    self.bump();
                    while !self.at_any(&[DEDENT, EOF]) {
                        let before = self.pos_marker();
                        self.match_arm();
                        if self.pos_marker() == before {
                            self.bump_error();
                        }
                    }
                    self.eat(DEDENT);
                    self.ended_with_block = true;
                } else {
                    self.expected_block("match", line);
                }
            } else {
                let span = self.error_span();
                self.report(Diagnostic::new(msg::PARSE_BLOCK_ON_SAME_LINE, span));
            }
            self.finish_node();
        }
        self.finish_node();
    }

    fn match_arm(&mut self) {
        if !self.at(CASE_KW) {
            self.start(ERROR);
            self.expect(CASE_KW);
            self.recover_line();
            self.finish_node();
            return;
        }
        self.start(MATCH_ARM);
        let line = self.current_line();
        self.bump();
        self.pattern();
        if self.at(IF_KW) {
            self.start(MATCH_GUARD);
            self.bump();
            self.expr();
            self.finish_node();
        }
        if self.expect(COLON) {
            if self.at(NEWLINE) {
                self.block("case", line);
            } else if self.at_any(&[RETURN_KW, BREAK_KW, CONTINUE_KW, PASS_KW, LET_KW, VAR_KW]) {
                self.error(msg::PARSE_ARM_STATEMENT);
                self.start(BLOCK);
                self.statement();
                self.finish_node();
            } else {
                self.expr();
                self.end_stmt();
            }
        } else {
            self.recover_line();
        }
        self.finish_node();
    }
}

fn is_arithmetic(c: Class) -> bool {
    matches!(c, Class::Additive | Class::Multiplicative | Class::Shift | Class::Power)
}
