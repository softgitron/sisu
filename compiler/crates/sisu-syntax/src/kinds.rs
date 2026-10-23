//! Token and node kinds, and the lexical tables (keywords and operators)
//! that the lexer reads. The tables are data, so a port can transcribe them.

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u16)]
pub enum SyntaxKind {
    // Trivia: kept in the tree, ignored by the parser.
    WHITESPACE,
    COMMENT,
    DOC_COMMENT,
    MODULE_DOC_COMMENT,
    /// A line break that is not a NEWLINE: inside brackets, on a blank or
    /// comment-only line, or before a continuation line.
    LINE_BREAK,

    // Layout.
    NEWLINE,
    INDENT,
    DEDENT,
    EOF,

    // Names and literals.
    IDENT,
    INT_NUMBER,
    FLOAT_NUMBER,
    CHAR_LIT,
    STRING_START,
    STRING_TEXT,
    STRING_END,
    INTERP_START,
    INTERP_END,
    FORMAT_SPEC,
    RAW_STRING,
    BYTE_STRING,
    ERROR_TOKEN,

    // Punctuation.
    L_PAREN,
    R_PAREN,
    L_BRACKET,
    R_BRACKET,
    L_BRACE,
    R_BRACE,
    COMMA,
    COLON,
    DOT,
    DOT_DOT,
    QUESTION,
    QUESTION_DOT,
    QUESTION_QUESTION,
    ARROW,
    AT,
    BANG,
    SEMICOLON,
    BACKSLASH,
    WALRUS,

    // Operators.
    EQ,
    EQ_EQ,
    BANG_EQ,
    LT,
    LT_EQ,
    GT,
    GT_EQ,
    PLUS,
    MINUS,
    STAR,
    SLASH,
    SLASH_SLASH,
    PERCENT,
    STAR_STAR,
    PLUS_PERCENT,
    MINUS_PERCENT,
    STAR_PERCENT,
    PLUS_PIPE,
    MINUS_PIPE,
    STAR_PIPE,
    SHL,
    SHR,
    AMP,
    PIPE,
    CARET,
    TILDE,
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

    // The 47 reserved keywords of proposal/02-syntax.md.
    AND_KW,
    AS_KW,
    ASSERT_KW,
    BREAK_KW,
    CASE_KW,
    COMPTIME_KW,
    CONST_KW,
    CONTINUE_KW,
    DEF_KW,
    DEFER_KW,
    DO_KW,
    DYN_KW,
    ELIF_KW,
    ELSE_KW,
    ENUM_KW,
    ERROR_KW,
    EXTERN_KW,
    FOR_KW,
    FROM_KW,
    IF_KW,
    IMPL_KW,
    IMPORT_KW,
    IN_KW,
    LET_KW,
    MATCH_KW,
    MUT_KW,
    NOT_KW,
    OR_KW,
    OWN_KW,
    PASS_KW,
    PUB_KW,
    REF_KW,
    RETURN_KW,
    SELECT_KW,
    SELF_KW,
    SELF_TYPE_KW,
    STRUCT_KW,
    TEST_KW,
    TRAIT_KW,
    TYPE_KW,
    UNSAFE_KW,
    VAR_KW,
    VIEW_KW,
    WHERE_KW,
    WHILE_KW,
    WITH_KW,
    YIELD_KW,

    // Nodes.
    SOURCE_FILE,
    ERROR,
    ATTRIBUTE,
    VISIBILITY,
    FN_DEF,
    GENERIC_PARAMS,
    GENERIC_PARAM,
    PARAM_LIST,
    PARAM,
    RET_TYPE,
    WHERE_CLAUSE,
    BLOCK,
    STRUCT_DEF,
    FIELD_DEF,
    ENUM_DEF,
    VARIANT_DEF,
    VARIANT_FIELD,
    ERROR_DEF,
    OPEN_MARKER,
    IMPL_DEF,
    TRAIT_DEF,
    IMPORT_ITEM,
    FROM_IMPORT,
    MODULE_PATH,
    IMPORT_NAME,
    TEST_DEF,
    SUITE_DEF,
    BENCH_DEF,
    CONST_DEF,
    TYPE_ALIAS,
    EXTERN_BLOCK,
    LET_STMT,
    LET_ELSE,
    ASSIGN_STMT,
    EXPR_STMT,
    RETURN_STMT,
    BREAK_STMT,
    CONTINUE_STMT,
    PASS_STMT,
    IF_STMT,
    ELIF_CLAUSE,
    ELSE_CLAUSE,
    COND,
    LET_COND,
    WHILE_STMT,
    FOR_STMT,
    ASSERT_STMT,
    EXPECT_STMT,
    DEFER_STMT,
    WITH_STMT,
    UNSAFE_BLOCK,
    SELECT_STMT,
    COMPTIME_STMT,
    CONTRACT_STMT,
    YIELD_STMT,
    LITERAL,
    STRING_EXPR,
    INTERPOLATION,
    NAME_REF,
    PAREN_EXPR,
    TUPLE_EXPR,
    LIST_EXPR,
    DICT_EXPR,
    DICT_ENTRY,
    SET_EXPR,
    COMPREHENSION,
    COMP_FOR,
    COMP_IF,
    BIN_EXPR,
    COMPARE_EXPR,
    PREFIX_EXPR,
    CALL_EXPR,
    ARG_LIST,
    ARG,
    DO_BLOCK,
    FIELD_EXPR,
    OPT_FIELD_EXPR,
    INDEX_EXPR,
    SLICE,
    TRY_EXPR,
    CLOSURE_EXPR,
    CLOSURE_PARAMS,
    CLOSURE_PARAM,
    IF_EXPR,
    MATCH_EXPR,
    MATCH_ARM_LIST,
    MATCH_ARM,
    MATCH_GUARD,
    PATH_TYPE,
    GENERIC_ARGS,
    CONST_ARG,
    OPTION_TYPE,
    RESULT_TYPE,
    ERROR_SET_TYPE,
    TUPLE_TYPE,
    TUPLE_TYPE_FIELD,
    PAREN_TYPE,
    FN_TYPE,
    REF_TYPE,
    DYN_TYPE,
    IMPL_TYPE,
    PTR_TYPE,
    IDENT_PAT,
    WILDCARD_PAT,
    LITERAL_PAT,
    CTOR_PAT,
    PAT_ARGS,
    PAT_ARG,
    REST_PAT,
    TUPLE_PAT,
    PAREN_PAT,
    LIST_PAT,
    STAR_PAT,
    OR_PAT,
}

use SyntaxKind::*;

/// The reserved keywords, in the order of proposal/02-syntax.md.
pub const KEYWORDS: &[(&str, SyntaxKind)] = &[
    ("and", AND_KW),
    ("as", AS_KW),
    ("assert", ASSERT_KW),
    ("break", BREAK_KW),
    ("case", CASE_KW),
    ("comptime", COMPTIME_KW),
    ("const", CONST_KW),
    ("continue", CONTINUE_KW),
    ("def", DEF_KW),
    ("defer", DEFER_KW),
    ("do", DO_KW),
    ("dyn", DYN_KW),
    ("elif", ELIF_KW),
    ("else", ELSE_KW),
    ("enum", ENUM_KW),
    ("error", ERROR_KW),
    ("extern", EXTERN_KW),
    ("for", FOR_KW),
    ("from", FROM_KW),
    ("if", IF_KW),
    ("impl", IMPL_KW),
    ("import", IMPORT_KW),
    ("in", IN_KW),
    ("let", LET_KW),
    ("match", MATCH_KW),
    ("mut", MUT_KW),
    ("not", NOT_KW),
    ("or", OR_KW),
    ("own", OWN_KW),
    ("pass", PASS_KW),
    ("pub", PUB_KW),
    ("ref", REF_KW),
    ("return", RETURN_KW),
    ("select", SELECT_KW),
    ("self", SELF_KW),
    ("Self", SELF_TYPE_KW),
    ("struct", STRUCT_KW),
    ("test", TEST_KW),
    ("trait", TRAIT_KW),
    ("type", TYPE_KW),
    ("unsafe", UNSAFE_KW),
    ("var", VAR_KW),
    ("view", VIEW_KW),
    ("where", WHERE_KW),
    ("while", WHILE_KW),
    ("with", WITH_KW),
    ("yield", YIELD_KW),
];

/// Contextual keywords: identifiers that are keywords only in their position.
pub const CONTEXTUAL_KEYWORDS: &[&str] = &[
    "requires",
    "ensures",
    "invariant",
    "decreases",
    "old",
    "result",
    "it",
    "bench",
    "suite",
    "expect",
    "pure",
];

/// Operators and punctuation, longest first, so the lexer takes the
/// longest match by scanning in order.
pub const PUNCTUATION: &[(&str, SyntaxKind)] = &[
    ("//=", SLASH_SLASH_EQ),
    ("**=", STAR_STAR_EQ),
    ("<<=", SHL_EQ),
    (">>=", SHR_EQ),
    ("+%=", PLUS_PERCENT_EQ),
    ("-%=", MINUS_PERCENT_EQ),
    ("*%=", STAR_PERCENT_EQ),
    ("+|=", PLUS_PIPE_EQ),
    ("-|=", MINUS_PIPE_EQ),
    ("*|=", STAR_PIPE_EQ),
    ("==", EQ_EQ),
    ("!=", BANG_EQ),
    ("<=", LT_EQ),
    (">=", GT_EQ),
    ("//", SLASH_SLASH),
    ("**", STAR_STAR),
    ("+%", PLUS_PERCENT),
    ("-%", MINUS_PERCENT),
    ("*%", STAR_PERCENT),
    ("+|", PLUS_PIPE),
    ("-|", MINUS_PIPE),
    ("*|", STAR_PIPE),
    ("<<", SHL),
    (">>", SHR),
    ("+=", PLUS_EQ),
    ("-=", MINUS_EQ),
    ("*=", STAR_EQ),
    ("/=", SLASH_EQ),
    ("%=", PERCENT_EQ),
    ("&=", AMP_EQ),
    ("|=", PIPE_EQ),
    ("^=", CARET_EQ),
    ("->", ARROW),
    ("..", DOT_DOT),
    ("?.", QUESTION_DOT),
    ("??", QUESTION_QUESTION),
    (":=", WALRUS),
    ("(", L_PAREN),
    (")", R_PAREN),
    ("[", L_BRACKET),
    ("]", R_BRACKET),
    ("{", L_BRACE),
    ("}", R_BRACE),
    (",", COMMA),
    (":", COLON),
    (".", DOT),
    ("?", QUESTION),
    ("@", AT),
    ("!", BANG),
    (";", SEMICOLON),
    ("\\", BACKSLASH),
    ("=", EQ),
    ("<", LT),
    (">", GT),
    ("+", PLUS),
    ("-", MINUS),
    ("*", STAR),
    ("/", SLASH),
    ("%", PERCENT),
    ("&", AMP),
    ("|", PIPE),
    ("^", CARET),
    ("~", TILDE),
];

/// Tokens that, as the first token of a line, continue the previous
/// logical line (02: "a line whose first token is a binary operator, `.`,
/// `?.` or `??`"). `not` continues only as part of `not in`, which the
/// lexer checks separately.
pub const CONTINUATION_STARTERS: &[SyntaxKind] = &[
    DOT,
    QUESTION_DOT,
    QUESTION_QUESTION,
    PLUS,
    MINUS,
    STAR,
    SLASH,
    SLASH_SLASH,
    PERCENT,
    STAR_STAR,
    PLUS_PERCENT,
    MINUS_PERCENT,
    STAR_PERCENT,
    PLUS_PIPE,
    MINUS_PIPE,
    STAR_PIPE,
    SHL,
    SHR,
    AMP,
    PIPE,
    CARET,
    EQ_EQ,
    BANG_EQ,
    LT,
    LT_EQ,
    GT,
    GT_EQ,
    AND_KW,
    OR_KW,
    IN_KW,
];

impl SyntaxKind {
    pub fn is_trivia(self) -> bool {
        matches!(self, WHITESPACE | COMMENT | DOC_COMMENT | MODULE_DOC_COMMENT | LINE_BREAK)
    }

    pub fn is_keyword(self) -> bool {
        (self as u16) >= (AND_KW as u16) && (self as u16) <= (YIELD_KW as u16)
    }

    pub fn is_node(self) -> bool {
        (self as u16) >= (SOURCE_FILE as u16)
    }

    pub fn from_keyword(text: &str) -> Option<SyntaxKind> {
        KEYWORDS.iter().find(|(k, _)| *k == text).map(|&(_, kind)| kind)
    }

    pub fn keyword_text(self) -> Option<&'static str> {
        KEYWORDS.iter().find(|(_, k)| *k == self).map(|&(t, _)| t)
    }

    pub fn punctuation_text(self) -> Option<&'static str> {
        PUNCTUATION.iter().find(|(_, k)| *k == self).map(|&(t, _)| t)
    }

    pub fn is_open_bracket(self) -> bool {
        matches!(self, L_PAREN | L_BRACKET | L_BRACE | INTERP_START)
    }

    pub fn is_close_bracket(self) -> bool {
        matches!(self, R_PAREN | R_BRACKET | R_BRACE | INTERP_END)
    }

    /// How the kind is named in diagnostics ("expected `:`, found ...").
    pub fn describe(self) -> String {
        if let Some(t) = self.keyword_text() {
            return format!("`{t}`");
        }
        if let Some(t) = self.punctuation_text() {
            return format!("`{t}`");
        }
        match self {
            NEWLINE => "the end of the line".into(),
            INDENT => "an indented block".into(),
            DEDENT => "the end of the block".into(),
            EOF => "the end of the file".into(),
            IDENT => "a name".into(),
            INT_NUMBER => "an integer".into(),
            FLOAT_NUMBER => "a float".into(),
            CHAR_LIT => "a character".into(),
            STRING_START | RAW_STRING | BYTE_STRING => "a string".into(),
            ERROR_TOKEN => "an unknown character".into(),
            INTERP_END => "`}`".into(),
            _ => format!("{self:?}"),
        }
    }

    /// The upper-case name used in tree dumps.
    pub fn name(self) -> String {
        format!("{self:?}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forty_seven_keywords() {
        assert_eq!(KEYWORDS.len(), 47);
        for (text, kind) in KEYWORDS {
            assert!(kind.is_keyword(), "{text}");
            assert_eq!(SyntaxKind::from_keyword(text), Some(*kind));
        }
        assert!(!IDENT.is_keyword());
        assert!(SOURCE_FILE.is_node() && !YIELD_KW.is_node());
    }

    #[test]
    fn punctuation_is_longest_first() {
        // A shorter entry must never shadow a longer one that starts with it.
        for (i, (a, _)) in PUNCTUATION.iter().enumerate() {
            for (b, _) in &PUNCTUATION[..i] {
                assert!(!(a.len() > b.len() && a.starts_with(b)), "{a} is shadowed by {b}");
            }
        }
    }
}
