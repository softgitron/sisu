//! Where a space goes between two tokens of a line: the spacing of PEP 8
//! and Black, decided by the tokens' kinds and the nodes they belong to.

use crate::lines::Tok;
use sisu_syntax::SyntaxKind::{self, *};

fn is_open(k: SyntaxKind) -> bool {
    matches!(k, L_PAREN | L_BRACKET | L_BRACE)
}

fn is_close(k: SyntaxKind) -> bool {
    matches!(k, R_PAREN | R_BRACKET | R_BRACE)
}

/// Whether a space goes between `a` and `b`.
pub fn between(a: &Tok, b: &Tok) -> bool {
    let (ak, bk) = (a.kind, b.kind);
    if is_open(ak) || is_close(bk) {
        return false;
    }
    if matches!(bk, COMMA | COLON | SEMICOLON) {
        return false;
    }
    // Field access, module paths and optional chaining.
    if matches!(ak, DOT | QUESTION_DOT) || matches!(bk, DOT | QUESTION_DOT) {
        return false;
    }
    // `f()?`, `T?`.
    if bk == QUESTION {
        return false;
    }
    // `x: int`, `{"a": 1}`: a space after the colon, except in a slice.
    if ak == COLON {
        return a.parent != SLICE;
    }
    // A keyword argument and a keyword pattern argument: `f(x=1)`.
    if (ak == EQ && matches!(a.parent, ARG | PAT_ARG)) || (bk == EQ && matches!(b.parent, ARG | PAT_ARG)) {
        return false;
    }
    // `@derive`, `*rest`, `-x`, `~x`, `^1`: a prefix hugs its operand.
    if ak == AT {
        return false;
    }
    if matches!(ak, MINUS | PLUS | TILDE | CARET | STAR | STAR_STAR)
        && matches!(a.parent, PREFIX_EXPR | STAR_PAT | PARAM | REST_PAT | LITERAL_PAT)
    {
        return false;
    }
    // A call, a definition's parameters, a pattern's arguments, a variant's
    // fields, a trait's supertraits and `pub(` hug what they follow; a tuple
    // or a parenthesised expression does not.
    if bk == L_PAREN {
        return !matches!(b.parent, ARG_LIST | PARAM_LIST | PAT_ARGS | VISIBILITY | VARIANT_DEF | TRAIT_DEF);
    }
    // An index and type arguments or parameters hug what they follow; a
    // list does not.
    if bk == L_BRACKET {
        return !matches!(b.parent, GENERIC_ARGS | GENERIC_PARAMS | INDEX_EXPR);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tok(kind: SyntaxKind, parent: SyntaxKind) -> Tok {
        Tok { kind, text: String::new(), parent, ..Tok::default() }
    }

    #[test]
    fn calls_hug_and_lists_do_not() {
        assert!(!between(&tok(IDENT, NAME_REF), &tok(L_PAREN, ARG_LIST)));
        assert!(between(&tok(EQ, LET_STMT), &tok(L_BRACKET, LIST_EXPR)));
        assert!(!between(&tok(IDENT, PATH_TYPE), &tok(L_BRACKET, GENERIC_ARGS)));
        assert!(!between(&tok(EQ, ARG), &tok(INT_NUMBER, LITERAL)));
        assert!(between(&tok(EQ, PARAM), &tok(INT_NUMBER, LITERAL)));
        assert!(!between(&tok(MINUS, PREFIX_EXPR), &tok(IDENT, NAME_REF)));
        assert!(between(&tok(MINUS, BIN_EXPR), &tok(IDENT, NAME_REF)));
    }
}
