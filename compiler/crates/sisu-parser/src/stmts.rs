//! Statements and blocks.
//!
//! ```text
//! block     := NEWLINE INDENT stmt+ DEDENT
//! stmt      := let_stmt | return_stmt | 'break' | 'continue' | 'pass' | if_stmt
//!              | while_stmt | for_stmt | assert_stmt | defer_stmt | with_stmt
//!              | item | expr_stmt | assign_stmt
//! let_stmt  := ('let' | 'var') patterns (':' type)? ('=' exprs let_else?)? NEWLINE
//! let_else  := 'else' ('as' name)? ':' (block | expr | return_stmt | 'break' | 'continue')
//! if_stmt   := 'if' cond ':' block ('elif' cond ':' block)* ('else' ':' block)?
//! cond      := cond_part (',' cond_part)*      cond_part := 'let' pattern '=' expr | expr
//! while_stmt:= 'while' cond ':' block
//! for_stmt  := 'for' patterns 'in' ('mut' | 'own')? expr ':' block
//! assign_stmt := exprs ('=' | '+=' | ...) exprs NEWLINE
//! ```

use crate::parser::Parser;
use sisu_diag::{Diagnostic, msg};
use sisu_syntax::SyntaxKind;
use sisu_syntax::SyntaxKind::*;

const AUG_ASSIGN: &[SyntaxKind] = &[
    PLUS_EQ,
    MINUS_EQ,
    STAR_EQ,
    SLASH_EQ,
    SLASH_SLASH_EQ,
    PERCENT_EQ,
    STAR_STAR_EQ,
    AMP_EQ,
    PIPE_EQ,
    CARET_EQ,
    SHL_EQ,
    SHR_EQ,
    PLUS_PERCENT_EQ,
    MINUS_PERCENT_EQ,
    STAR_PERCENT_EQ,
    PLUS_PIPE_EQ,
    MINUS_PIPE_EQ,
    STAR_PIPE_EQ,
];

/// Keywords that start a statement; after `:` on the same line they mean a
/// block was written inline.
const STATEMENT_KEYWORDS: &[SyntaxKind] = &[
    RETURN_KW,
    BREAK_KW,
    CONTINUE_KW,
    PASS_KW,
    LET_KW,
    VAR_KW,
    ASSERT_KW,
    FOR_KW,
    WHILE_KW,
    DEFER_KW,
    WITH_KW,
];

impl Parser<'_> {
    /// A statement block after `:`. `keyword` and `line` name the opener
    /// for "expected an indented block after `if` on line 12".
    pub(crate) fn block(&mut self, keyword: &str, line: u32) {
        self.start(BLOCK);
        if self.at(NEWLINE) {
            self.bump();
            if self.at(INDENT) {
                self.bump();
                while !self.at_any(&[DEDENT, EOF]) {
                    let before = self.pos_marker();
                    self.statement();
                    if self.pos_marker() == before {
                        self.bump_error();
                    }
                }
                self.eat(DEDENT);
                self.ended_with_block = true;
            } else {
                self.expected_block(keyword, line);
                self.start(ERROR);
                self.finish_node();
            }
        } else if self.at_any(&[EOF, DEDENT]) {
            self.expected_block(keyword, line);
            self.start(ERROR);
            self.finish_node();
        } else {
            // `if x: return 1`: report once, keep the statement.
            let span = self.error_span();
            self.report(Diagnostic::new(msg::PARSE_BLOCK_ON_SAME_LINE, span));
            self.statement();
        }
        self.finish_node();
    }

    /// A block whose NEWLINE was already consumed by recovery.
    pub(crate) fn block_after_newline(&mut self, keyword: &str, line: u32) {
        self.start(BLOCK);
        if self.at(INDENT) {
            self.bump();
            while !self.at_any(&[DEDENT, EOF]) {
                let before = self.pos_marker();
                self.statement();
                if self.pos_marker() == before {
                    self.bump_error();
                }
            }
            self.eat(DEDENT);
            self.ended_with_block = true;
        } else {
            self.expected_block(keyword, line);
        }
        self.finish_node();
    }

    pub(crate) fn statement(&mut self) {
        match self.current() {
            LET_KW | VAR_KW => self.let_stmt(),
            RETURN_KW => {
                self.return_stmt();
            }
            BREAK_KW => self.keyword_stmt(BREAK_STMT),
            CONTINUE_KW => self.keyword_stmt(CONTINUE_STMT),
            PASS_KW => self.keyword_stmt(PASS_STMT),
            IF_KW if self.if_is_statement() => self.if_stmt(),
            WHILE_KW => self.while_stmt(),
            FOR_KW => self.for_stmt(),
            ASSERT_KW => self.assert_stmt(),
            DEFER_KW => self.defer_stmt(),
            WITH_KW => self.with_stmt(),
            UNSAFE_KW if self.nth(1) == COLON => {
                self.start(UNSAFE_BLOCK);
                let line = self.current_line();
                self.bump();
                self.bump();
                self.block("unsafe", line);
                self.finish_node();
            }
            INDENT => self.unexpected_indent_block(false),
            YIELD_KW => {
                self.start(YIELD_STMT);
                self.bump();
                if !self.at_line_end() {
                    self.expr();
                }
                self.end_stmt();
                self.finish_node();
            }
            SELECT_KW => self.select_stmt(),
            COMPTIME_KW if self.nth(1) != DEF_KW => {
                // `comptime for`, `comptime if`, `comptime assert`, `comptime e`.
                self.start(COMPTIME_STMT);
                self.bump();
                self.statement();
                self.finish_node();
            }
            IDENT if self.at_contract() => {
                // `requires e`, `ensures e`, `invariant e`, `decreases e`.
                self.start(CONTRACT_STMT);
                self.bump();
                self.expr();
                self.end_stmt();
                self.finish_node();
            }
            IDENT if self.current_text() == "expect" && self.expect_stmt_follows() => {
                self.start(EXPECT_STMT);
                self.bump();
                self.expr();
                self.end_stmt();
                self.finish_node();
            }
            _ if self.at_item_start() => self.item(),
            _ => self.expr_or_assign(),
        }
    }

    pub(crate) fn at_contract(&self) -> bool {
        matches!(self.current_text(), "requires" | "ensures" | "invariant" | "decreases")
            && !matches!(self.nth(1), EQ | DOT | COMMA | NEWLINE | L_BRACKET | QUESTION_DOT | EOF)
            && !AUG_ASSIGN.contains(&self.nth(1))
    }

    /// `expect cond` is a statement when an expression follows on the line.
    fn expect_stmt_follows(&self) -> bool {
        matches!(
            self.nth(1),
            IDENT
                | INT_NUMBER
                | FLOAT_NUMBER
                | STRING_START
                | NOT_KW
                | MINUS
                | SELF_KW
                | L_BRACKET
                | CHAR_LIT
        )
    }

    /// `select:` and arms `case pattern = operation:` with blocks.
    fn select_stmt(&mut self) {
        self.start(SELECT_STMT);
        let line = self.current_line();
        self.bump();
        if self.expect(COLON) {
            self.start(MATCH_ARM_LIST);
            if self.eat(NEWLINE) && self.eat(INDENT) {
                while !self.at_any(&[DEDENT, EOF]) {
                    if !self.at(CASE_KW) {
                        let before = self.pos_marker();
                        self.expect(CASE_KW);
                        self.recover_line();
                        if self.pos_marker() == before {
                            // An indented block where a `case` belongs.
                            self.bump_error();
                        }
                        continue;
                    }
                    self.start(MATCH_ARM);
                    let arm_line = self.current_line();
                    self.bump();
                    self.pattern();
                    self.expect(EQ);
                    self.expr();
                    self.colon_block("case", arm_line);
                    self.finish_node();
                }
                self.eat(DEDENT);
                self.ended_with_block = true;
            } else {
                self.expected_block("select", line);
            }
            self.finish_node();
        }
        self.finish_node();
    }

    fn keyword_stmt(&mut self, kind: SyntaxKind) {
        self.start(kind);
        self.bump();
        self.end_stmt();
        self.finish_node();
    }

    /// `return [exprs]`; the NEWLINE is consumed unless `inline`.
    pub(crate) fn return_stmt(&mut self) {
        self.start(RETURN_STMT);
        self.bump();
        if self.at(IMPL_KW) {
            // `return impl Trait for T:` inside a `comptime def` (08).
            let cp = self.checkpoint();
            self.impl_def(cp);
        } else if !self.at_line_end() {
            self.expr_list();
        }
        self.end_stmt();
        self.finish_node();
    }

    fn let_stmt(&mut self) {
        self.start(LET_STMT);
        self.bump();
        self.pattern_list();
        if self.eat(COLON) {
            self.type_();
        }
        if self.at(WALRUS) {
            self.error(msg::PARSE_WALRUS);
            self.bump();
            self.expr_list();
        } else if self.eat(EQ) {
            self.expr_list();
            if self.at(ELSE_KW) {
                self.let_else();
            }
        }
        self.end_stmt();
        self.finish_node();
    }

    fn let_else(&mut self) {
        self.start(LET_ELSE);
        let line = self.current_line();
        self.bump();
        if self.eat(AS_KW) {
            self.expect_name();
        }
        if self.expect(COLON) {
            if self.at(NEWLINE) {
                self.block("else", line);
            } else {
                match self.current() {
                    RETURN_KW => {
                        self.start(RETURN_STMT);
                        self.bump();
                        if !self.at_line_end() {
                            self.expr_list();
                        }
                        self.finish_node();
                    }
                    BREAK_KW => {
                        self.start(BREAK_STMT);
                        self.bump();
                        self.finish_node();
                    }
                    CONTINUE_KW => {
                        self.start(CONTINUE_STMT);
                        self.bump();
                        self.finish_node();
                    }
                    _ => {
                        self.expr();
                    }
                }
            }
        }
        self.finish_node();
    }

    /// Decides whether `if` at the start of a statement is the statement
    /// form: the first `:` outside brackets ends the line, or a statement
    /// keyword follows it (an inline block, reported by `block`).
    fn if_is_statement(&self) -> bool {
        let mut depth = 0i32;
        let mut i = 1;
        loop {
            let k = self.nth(i);
            match k {
                NEWLINE | EOF | DEDENT | INDENT => return true,
                L_PAREN | L_BRACKET | L_BRACE | INTERP_START => depth += 1,
                R_PAREN | R_BRACKET | R_BRACE | INTERP_END => depth -= 1,
                COLON if depth <= 0 => {
                    let next = self.nth(i + 1);
                    return matches!(next, NEWLINE | EOF | DEDENT) || STATEMENT_KEYWORDS.contains(&next);
                }
                _ => {}
            }
            i += 1;
        }
    }

    fn if_stmt(&mut self) {
        self.start(IF_STMT);
        let line = self.current_line();
        self.bump();
        self.cond();
        self.colon_block("if", line);
        while self.at(ELIF_KW) {
            self.start(ELIF_CLAUSE);
            let line = self.current_line();
            self.bump();
            self.cond();
            self.colon_block("elif", line);
            self.finish_node();
        }
        if self.at(ELSE_KW) {
            self.start(ELSE_CLAUSE);
            let line = self.current_line();
            self.bump();
            self.colon_block("else", line);
            self.finish_node();
        }
        self.finish_node();
    }

    /// A condition: boolean tests and `let` bindings separated by commas.
    pub(crate) fn cond(&mut self) {
        self.start(COND);
        loop {
            if self.at(LET_KW) {
                self.start(LET_COND);
                self.bump();
                self.pattern();
                self.expect(EQ);
                self.expr();
                self.finish_node();
            } else {
                self.expr();
            }
            if self.at(EQ) {
                self.error(msg::PARSE_ASSIGNMENT_IN_EXPRESSION);
                self.bump();
                self.expr();
            } else if self.at(WALRUS) {
                self.error(msg::PARSE_WALRUS);
                self.bump();
                self.expr();
            }
            if !self.eat(COMMA) {
                break;
            }
        }
        self.finish_node();
    }

    fn while_stmt(&mut self) {
        self.start(WHILE_STMT);
        let line = self.current_line();
        self.bump();
        self.cond();
        self.colon_block("while", line);
        self.finish_node();
    }

    fn for_stmt(&mut self) {
        self.start(FOR_STMT);
        let line = self.current_line();
        self.bump();
        self.pattern_list();
        self.expect(IN_KW);
        if self.at(MUT_KW) || self.at(OWN_KW) {
            self.bump();
        }
        self.expr();
        self.colon_block("for", line);
        self.finish_node();
    }

    fn assert_stmt(&mut self) {
        self.start(ASSERT_STMT);
        self.bump();
        self.expr();
        if self.eat(COMMA) {
            self.expr();
        }
        self.end_stmt();
        self.finish_node();
    }

    fn defer_stmt(&mut self) {
        self.start(DEFER_STMT);
        let line = self.current_line();
        self.bump();
        if self.eat(COLON) {
            self.block("defer", line);
        } else {
            self.statement();
        }
        self.finish_node();
    }

    fn with_stmt(&mut self) {
        self.start(WITH_STMT);
        let line = self.current_line();
        self.bump();
        self.expr();
        if self.expect(AS_KW) {
            self.expect_name();
        }
        self.colon_block("with", line);
        self.finish_node();
    }

    fn expr_or_assign(&mut self) {
        let cp = self.checkpoint();
        let before = self.pos_marker();
        self.expr_list();
        if self.pos_marker() == before {
            // Nothing could start an expression here: skip the line.
            self.start_at(cp, ERROR);
            self.recover_line();
            self.finish_node();
            return;
        }
        if self.at(EQ) || self.at_any(AUG_ASSIGN) {
            self.start_at(cp, ASSIGN_STMT);
            self.bump();
            self.expr_list();
            self.end_stmt();
            self.finish_node();
        } else if self.at(WALRUS) {
            self.start_at(cp, ASSIGN_STMT);
            self.error(msg::PARSE_WALRUS);
            self.bump();
            self.expr_list();
            self.end_stmt();
            self.finish_node();
        } else {
            self.start_at(cp, EXPR_STMT);
            self.end_stmt();
            self.finish_node();
        }
    }

    /// One expression, or a bare tuple `a, b` where a statement allows it.
    pub(crate) fn expr_list(&mut self) {
        let cp = self.checkpoint();
        self.expr();
        if self.at(COMMA) {
            self.start_at(cp, TUPLE_EXPR);
            while self.eat(COMMA) {
                if self.at_expr_start() {
                    self.expr();
                } else {
                    break;
                }
            }
            self.finish_node();
        }
    }
}
