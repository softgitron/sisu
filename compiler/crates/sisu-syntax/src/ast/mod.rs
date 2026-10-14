//! The abstract syntax that later stages consume.
//!
//! The AST is built from the lossless tree by [`build::module`], a total
//! function: every tree yields an AST, and wherever the tree has an `ERROR`
//! node or a missing piece the AST has an `Error` variant or `None`. Every
//! AST node records the syntax node it came from (`node`) and its byte
//! range, so diagnostics and the language server can map back.

pub mod build;

use crate::tree::NodeId;
use sisu_diag::TextRange;

#[derive(Clone, Debug, PartialEq)]
pub struct Name {
    pub text: String,
    pub range: TextRange,
}

#[derive(Clone, Debug, Default)]
pub struct Module {
    pub items: Vec<Item>,
    /// `##!` lines.
    pub docs: Vec<String>,
    /// Names written inside top-level code that did not parse as a
    /// declaration (`stru ct Log:`): a later use of one of them most likely
    /// refers to the declaration that failed, so it is not reported as
    /// undefined.
    pub broken_names: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Attr {
    pub name: String,
    pub args: Vec<Arg>,
    pub range: TextRange,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Vis {
    Private,
    Pub,
    PubPackage,
}

#[derive(Clone, Debug)]
pub struct Item {
    pub node: NodeId,
    pub range: TextRange,
    pub attrs: Vec<Attr>,
    pub vis: Vis,
    /// `##` lines before the item.
    pub docs: Vec<String>,
    pub kind: ItemKind,
}

#[derive(Clone, Debug)]
pub enum ItemKind {
    Fn(FnDef),
    Struct(StructDef),
    Enum(EnumDef),
    ErrorSet(EnumDef),
    Impl(ImplDef),
    Trait(TraitDef),
    Import(Import),
    FromImport(FromImport),
    Test(TestDef),
    Suite(SuiteDef),
    Bench(TestDef),
    Const(ConstDef),
    TypeAlias(TypeAlias),
    Extern(Vec<Item>),
    /// A statement at the top level (reported by the parser).
    Stmt(Stmt),
    Error,
}

#[derive(Clone, Debug)]
pub struct FnDef {
    pub node: NodeId,
    pub range: TextRange,
    pub name: Name,
    pub is_pure: bool,
    pub is_comptime: bool,
    pub generics: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub ret: Option<TypeExpr>,
    /// `-> ref T from a, b`.
    pub from: Vec<Name>,
    /// `where T: Display + Hash, C.Item: Display`.
    pub where_: Vec<WherePred>,
    /// `None` for a signature without a body (an intrinsic).
    pub body: Option<Block>,
    pub attrs: Vec<Attr>,
    /// The header (name, parameters, result) has a syntax error, so the
    /// signature is not reliable; checking treats it as unknown.
    pub header_broken: bool,
    /// The body has a syntax error, so a binding it meant to make may be
    /// missing.
    pub body_broken: bool,
}

#[derive(Clone, Debug)]
pub struct GenericParam {
    pub name: Name,
    pub bounds: Vec<TypeExpr>,
    /// `A: Allocator = Global`: the type a use site that omits it gets.
    pub default: Option<TypeExpr>,
    pub is_const: bool,
}

/// One predicate of a `where` clause: a type and the traits it must
/// implement.
#[derive(Clone, Debug)]
pub struct WherePred {
    pub ty: TypeExpr,
    pub bounds: Vec<TypeExpr>,
    pub range: TextRange,
}

/// `type Item` in a trait (an associated type, with an optional default),
/// or `type Item = T` in an impl (its binding).
#[derive(Clone, Debug)]
pub struct AssocTypeDef {
    pub name: Name,
    pub ty: Option<TypeExpr>,
    pub range: TextRange,
}

/// A parameter or argument convention (04, "The decision").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mode {
    Borrow,
    Mut,
    Own,
}

#[derive(Clone, Debug)]
pub struct Param {
    pub node: NodeId,
    pub range: TextRange,
    pub mode: Mode,
    pub kind: ParamKind,
}

#[derive(Clone, Debug)]
pub enum ParamKind {
    SelfParam,
    Named {
        name: Name,
        ty: TypeExpr,
        default: Option<Expr>,
    },
    /// A bare `*`: the parameters after it are keyword-only.
    KeywordOnly,
    /// A bare `/`: the parameters before it are positional-only.
    PositionalOnly,
    Variadic {
        name: Name,
        ty: Option<TypeExpr>,
    },
    Error,
}

#[derive(Clone, Debug)]
pub struct StructDef {
    pub name: Name,
    pub generics: Vec<GenericParam>,
    pub is_view: bool,
    pub fields: Vec<FieldDef>,
    pub methods: Vec<FnDef>,
    /// Other members (an `invariant` line, a nested type), by range and kind.
    pub unsupported: Vec<(TextRange, &'static str)>,
}

#[derive(Clone, Debug)]
pub struct FieldDef {
    pub node: NodeId,
    pub range: TextRange,
    pub name: Name,
    pub ty: TypeExpr,
    pub default: Option<Expr>,
    pub vis: Vis,
}

#[derive(Clone, Debug)]
pub struct EnumDef {
    pub name: Name,
    pub generics: Vec<GenericParam>,
    pub variants: Vec<VariantDef>,
    pub methods: Vec<FnDef>,
    /// A trailing `..` line (05, open error sets).
    pub open: bool,
}

#[derive(Clone, Debug)]
pub struct VariantDef {
    pub node: NodeId,
    pub range: TextRange,
    pub name: Name,
    pub fields: Vec<VariantField>,
}

#[derive(Clone, Debug)]
pub struct VariantField {
    pub name: Option<Name>,
    pub ty: TypeExpr,
    pub range: TextRange,
}

#[derive(Clone, Debug)]
pub struct ImplDef {
    pub generics: Vec<GenericParam>,
    pub trait_ty: Option<TypeExpr>,
    pub self_ty: TypeExpr,
    pub methods: Vec<FnDef>,
    /// `type Item = T` lines.
    pub assoc_types: Vec<AssocTypeDef>,
}

#[derive(Clone, Debug)]
pub struct TraitDef {
    pub name: Name,
    pub generics: Vec<GenericParam>,
    /// `trait Container(Len)`: traits every implementer must implement too.
    pub supertraits: Vec<TypeExpr>,
    /// Required (no body) and provided methods.
    pub methods: Vec<FnDef>,
    pub assoc_types: Vec<AssocTypeDef>,
}

#[derive(Clone, Debug)]
pub struct Import {
    pub path: Vec<Name>,
    pub alias: Option<Name>,
    /// `import c "zlib.h"`.
    pub c_header: Option<String>,
}

#[derive(Clone, Debug)]
pub struct FromImport {
    pub path: Vec<Name>,
    pub names: Vec<(Name, Option<Name>)>,
}

#[derive(Clone, Debug)]
pub struct TestDef {
    pub name: String,
    pub name_range: TextRange,
    pub cases: Option<(Pattern, Expr)>,
    pub params: Vec<Param>,
    pub body: Option<Block>,
}

#[derive(Clone, Debug)]
pub struct SuiteDef {
    pub name: String,
    pub items: Vec<Item>,
}

#[derive(Clone, Debug)]
pub struct ConstDef {
    pub name: Name,
    pub ty: Option<TypeExpr>,
    pub value: Option<Expr>,
}

#[derive(Clone, Debug)]
pub struct TypeAlias {
    pub name: Name,
    pub ty: Option<TypeExpr>,
    pub refinement: bool,
}

#[derive(Clone, Debug)]
pub struct Block {
    pub node: NodeId,
    pub range: TextRange,
    pub stmts: Vec<Stmt>,
}

#[derive(Clone, Debug)]
pub struct Stmt {
    pub node: NodeId,
    pub range: TextRange,
    pub kind: StmtKind,
}

#[derive(Clone, Debug)]
pub enum StmtKind {
    Let {
        mutable: bool,
        pat: Pattern,
        ty: Option<TypeExpr>,
        init: Option<Expr>,
        else_: Option<LetElse>,
    },
    /// `op` is set for augmented assignment.
    Assign {
        target: Expr,
        op: Option<BinOp>,
        value: Expr,
    },
    Expr(Expr),
    Return(Option<Expr>),
    Break,
    Continue,
    Pass,
    If {
        branches: Vec<(Cond, Block)>,
        else_block: Option<Block>,
    },
    While {
        cond: Cond,
        body: Block,
    },
    For {
        pat: Pattern,
        mode: Mode,
        iter: Expr,
        body: Block,
    },
    Assert {
        cond: Expr,
        message: Option<Expr>,
    },
    Expect(Expr),
    Defer(Block),
    With {
        value: Expr,
        name: Name,
        body: Block,
    },
    /// A nested declaration (`def`, `const`, ...).
    Item(Box<Item>),
    /// A construct this stage parses but does not implement.
    Unsupported(&'static str),
    Error,
}

#[derive(Clone, Debug)]
pub struct LetElse {
    pub range: TextRange,
    pub binding: Option<Name>,
    pub body: ElseBody,
}

#[derive(Clone, Debug)]
pub enum ElseBody {
    Expr(Expr),
    /// `else: return x`, `else: break`, `else: continue`.
    Diverge(Box<Stmt>),
    Block(Block),
}

#[derive(Clone, Debug)]
pub struct Cond {
    pub range: TextRange,
    pub parts: Vec<CondPart>,
}

#[derive(Clone, Debug)]
pub enum CondPart {
    Expr(Expr),
    Let { pat: Pattern, value: Expr },
}

#[derive(Clone, Debug)]
pub struct Expr {
    pub node: NodeId,
    pub range: TextRange,
    pub kind: ExprKind,
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Int {
        text: String,
        value: Option<u128>,
        suffix: Option<String>,
    },
    Float {
        text: String,
        value: f64,
        suffix: Option<String>,
    },
    Char(char),
    /// A string literal; plain when it has no interpolation.
    Str(Vec<StrPart>),
    RawStr(String),
    ByteStr(Vec<u8>),
    Name(String),
    SelfValue,
    SelfType,
    Paren(Box<Expr>),
    Tuple(Vec<Expr>),
    List(Vec<Expr>),
    Dict(Vec<(Expr, Expr)>),
    Set(Vec<Expr>),
    Comprehension {
        kind: CompKind,
        elem: Box<Expr>,
        value: Option<Box<Expr>>,
        clauses: Vec<CompClause>,
    },
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    /// A comparison or a chain of them.
    Compare {
        first: Box<Expr>,
        rest: Vec<(CmpOp, Expr)>,
    },
    Unary {
        op: UnOp,
        expr: Box<Expr>,
    },
    Ref {
        mutable: bool,
        expr: Box<Expr>,
    },
    /// `^e` inside an index: counted from the end.
    FromEnd(Box<Expr>),
    Call {
        callee: Box<Expr>,
        args: Vec<Arg>,
        trailing: Option<Box<Closure>>,
    },
    Field {
        base: Box<Expr>,
        name: Name,
    },
    OptField {
        base: Box<Expr>,
        name: Name,
    },
    Index {
        base: Box<Expr>,
        args: Vec<IndexArg>,
    },
    Try(Box<Expr>),
    Closure(Box<Closure>),
    If {
        branches: Vec<(Expr, Expr)>,
        else_: Option<Box<Expr>>,
    },
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<Arm>,
    },
    Unsupported(&'static str),
    Error,
}

#[derive(Clone, Debug)]
pub enum StrPart {
    Text(String),
    Interp {
        expr: Expr,
        /// For `{expr=}`: the expression's source text.
        debug: Option<String>,
        spec: Option<String>,
        range: TextRange,
    },
}

#[derive(Clone, Debug)]
pub enum IndexArg {
    Expr(Expr),
    Slice { lo: Option<Expr>, hi: Option<Expr> },
}

#[derive(Clone, Debug)]
pub struct Arg {
    pub node: NodeId,
    pub range: TextRange,
    pub name: Option<Name>,
    /// `mut` written before the argument.
    pub mutable: bool,
    pub value: Expr,
}

#[derive(Clone, Debug)]
pub struct Closure {
    pub node: NodeId,
    pub range: TextRange,
    pub own: bool,
    pub params: Vec<ClosureParam>,
    /// A `do` block with no parameter list (`do:`), which uses `it`.
    pub implicit_param: bool,
    pub body: ClosureBody,
}

#[derive(Clone, Debug)]
pub struct ClosureParam {
    pub name: Name,
    pub ty: Option<TypeExpr>,
}

#[derive(Clone, Debug)]
pub enum ClosureBody {
    Expr(Expr),
    Block(Block),
}

#[derive(Clone, Debug)]
pub struct Arm {
    pub node: NodeId,
    pub range: TextRange,
    pub pat: Pattern,
    pub guard: Option<Expr>,
    pub body: ArmBody,
}

#[derive(Clone, Debug)]
pub enum ArmBody {
    Expr(Expr),
    Block(Block),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompKind {
    List,
    Dict,
    Set,
    Gen,
}

#[derive(Clone, Debug)]
pub enum CompClause {
    For { pat: Pattern, iter: Expr },
    If(Expr),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    FloorDiv,
    Mod,
    Pow,
    WrapAdd,
    WrapSub,
    WrapMul,
    SatAdd,
    SatSub,
    SatMul,
    Shl,
    Shr,
    BitAnd,
    BitOr,
    BitXor,
    And,
    Or,
    Coalesce,
}

impl BinOp {
    pub fn text(self) -> &'static str {
        match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::FloorDiv => "//",
            BinOp::Mod => "%",
            BinOp::Pow => "**",
            BinOp::WrapAdd => "+%",
            BinOp::WrapSub => "-%",
            BinOp::WrapMul => "*%",
            BinOp::SatAdd => "+|",
            BinOp::SatSub => "-|",
            BinOp::SatMul => "*|",
            BinOp::Shl => "<<",
            BinOp::Shr => ">>",
            BinOp::BitAnd => "&",
            BinOp::BitOr => "|",
            BinOp::BitXor => "^",
            BinOp::And => "and",
            BinOp::Or => "or",
            BinOp::Coalesce => "??",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    In,
    NotIn,
}

impl CmpOp {
    pub fn text(self) -> &'static str {
        match self {
            CmpOp::Eq => "==",
            CmpOp::Ne => "!=",
            CmpOp::Lt => "<",
            CmpOp::Le => "<=",
            CmpOp::Gt => ">",
            CmpOp::Ge => ">=",
            CmpOp::In => "in",
            CmpOp::NotIn => "not in",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UnOp {
    Neg,
    BitNot,
    Not,
}

impl UnOp {
    pub fn text(self) -> &'static str {
        match self {
            UnOp::Neg => "-",
            UnOp::BitNot => "~",
            UnOp::Not => "not",
        }
    }
}

#[derive(Clone, Debug)]
pub struct TypeExpr {
    pub node: NodeId,
    pub range: TextRange,
    pub kind: TypeKind,
}

#[derive(Clone, Debug)]
pub enum TypeKind {
    /// A name with optional arguments: `int`, `list[T]`, `cli.Error`, `Self`.
    Path {
        segments: Vec<Name>,
        args: Vec<TypeExpr>,
    },
    Option(Box<TypeExpr>),
    /// `T ! E`, `T !` (inferred set), `! E` (unit result).
    Result {
        ok: Option<Box<TypeExpr>>,
        err: Option<Box<TypeExpr>>,
    },
    /// `{A, B}` after `!`.
    Union(Vec<TypeExpr>),
    Tuple(Vec<(Option<Name>, TypeExpr)>),
    Fn {
        pure: bool,
        params: Vec<TypeExpr>,
        ret: Box<TypeExpr>,
    },
    Ref {
        mutable: bool,
        inner: Box<TypeExpr>,
    },
    ConstArg(i128),
    /// `dyn Trait`: the inner type is the trait's path.
    Dyn(Box<TypeExpr>),
    Unsupported(&'static str),
    Error,
}

#[derive(Clone, Debug)]
pub struct Pattern {
    pub node: NodeId,
    pub range: TextRange,
    pub kind: PatKind,
}

#[derive(Clone, Debug)]
pub enum PatKind {
    Wildcard,
    Bind(Name),
    /// A literal, possibly negated.
    Literal(Expr),
    /// A variant or constant, with arguments when written with parentheses.
    Ctor {
        path: Vec<Name>,
        args: Option<Vec<PatArg>>,
    },
    Tuple(Vec<Pattern>),
    List(Vec<Pattern>),
    Star(Option<Name>),
    Rest,
    Or(Vec<Pattern>),
    Error,
}

#[derive(Clone, Debug)]
pub enum PatArg {
    Positional(Pattern),
    Keyword(Name, Pattern),
    Rest,
}
