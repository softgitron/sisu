//! SIR, the Sisu intermediate representation (proposal/18-next-steps.md,
//! pipeline step 5; "Bootstrapping": SIR has a serialised, documented form
//! that the conformance suite compares).
//!
//! A program is a list of type declarations and functions. A function is
//! a typed control-flow graph: locals (`_0` is the return place, then the
//! parameters), basic blocks of statements, and one terminator per block.
//! Every block may name an `unwind` block, the cleanup that runs when
//! anything in the block panics; cleanup blocks end in `resume`. Drop
//! elaboration is explicit: `drop` statements at scope exits in reverse
//! declaration order, guarded by a drop flag for a local that may have
//! been moved.
//!
//! The text form is written by [`write()`] and read back by [`read()`]; the
//! grammar is in docs/architecture.md. Reading what was written gives the
//! same program, which a test checks for every conformance case.
//!
//! Lowering invariant (checked by [`validate()`]): a statement that
//! initialises a value needing drop, or drops one, is the last statement
//! of its block, so a block's unwind edge is exact for every statement in
//! it.

pub mod read;
pub mod validate;
pub mod write;

pub use read::read;
pub use validate::validate;
pub use write::write;

pub const VERSION: u32 = 0;

#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    pub sources: Vec<String>,
    /// The entry function and whether it takes `System`.
    pub entry: Option<Entry>,
    pub types: Vec<TypeDecl>,
    pub functions: Vec<Function>,
    /// Test name and the function that runs it.
    pub tests: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub function: String,
    pub takes_system: bool,
    pub returns_result: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TypeDecl {
    Struct { name: String, fields: Vec<FieldDecl>, drop: Option<String> },
    Enum { name: String, variants: Vec<VariantDecl>, drop: Option<String>, error: bool, open: bool },
}

impl TypeDecl {
    pub fn name(&self) -> &str {
        match self {
            TypeDecl::Struct { name, .. } | TypeDecl::Enum { name, .. } => name,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldDecl {
    pub name: String,
    pub ty: Type,
    /// The function that computes the default.
    pub default: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VariantDecl {
    pub name: String,
    pub fields: Vec<(Option<String>, Type)>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Type {
    Int,
    Float,
    Bool,
    Char,
    Str,
    Unit,
    Never,
    /// A builtin or declared type, possibly with arguments: `list[int]`,
    /// `Option[str]`, `Point`, `cli.Error`.
    Named(String, Vec<Type>),
    Tuple(Vec<Type>),
    ErrSet(Vec<String>),
    Fn(Vec<Type>, Box<Type>),
    Ref(Box<Type>, bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LocalId(pub u32);

#[derive(Clone, Debug, PartialEq)]
pub struct LocalDecl {
    pub ty: Type,
    /// The source name, for debugging and for the SIR dump.
    pub name: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FnKind {
    Normal,
    /// A closure: parameter 1 is the tuple of captured references.
    Closure,
    /// The `drop` method of a type with `impl Drop`.
    Drop,
    /// A default value or a constant.
    Value,
    Test,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Function {
    pub name: String,
    pub kind: FnKind,
    pub param_count: u32,
    pub locals: Vec<LocalDecl>,
    pub blocks: Vec<BasicBlock>,
    /// Index into `Program::sources` for statement locations.
    pub source: u32,
}

impl Function {
    pub fn ret(&self) -> &Type {
        &self.locals[0].ty
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BasicBlock {
    pub stmts: Vec<Statement>,
    pub term: Terminator,
    pub unwind: Option<BlockId>,
    pub cleanup: bool,
    pub loc: Option<Loc>,
}

/// A 1-based source position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Loc {
    pub line: u32,
    pub col: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Statement {
    pub kind: StatementKind,
    pub loc: Option<Loc>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StatementKind {
    Assign(Place, Rvalue),
    Call {
        dest: Place,
        callee: Callee,
        args: Vec<Operand>,
    },
    /// Runs the value's drop glue, if `flag` (when given) is true; the
    /// place is left uninitialised.
    Drop {
        place: Place,
        flag: Option<LocalId>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Callee {
    Fn(String),
    Intrinsic(String, Vec<Type>),
    Value(Operand),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Terminator {
    Goto(BlockId),
    /// Branches on an integer: a `bool` (0 or 1), an `int`, or the
    /// variant index from `discriminant`.
    Switch {
        value: Operand,
        cases: Vec<(i64, BlockId)>,
        default: BlockId,
    },
    Return,
    /// Ends a cleanup block: unwinding continues in the caller.
    Resume,
    Unreachable,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Place {
    pub local: LocalId,
    pub proj: Vec<Proj>,
}

impl Place {
    pub fn local(l: LocalId) -> Place {
        Place { local: l, proj: Vec::new() }
    }

    pub fn project(&self, p: Proj) -> Place {
        let mut proj = self.proj.clone();
        proj.push(p);
        Place { local: self.local, proj }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Proj {
    /// The referent of a reference.
    Deref,
    Field(u32),
    /// The payload of variant `n` of an enum value.
    Downcast(u32),
    /// A list element at the index held by a local; panics out of bounds.
    Index(LocalId),
    /// A list element counted from the end (`xs[^i]`).
    IndexFromEnd(LocalId),
    /// A dict entry at the key held by a local; panics if absent.
    Key(LocalId),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Operand {
    Copy(Place),
    Move(Place),
    Const(Const),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Const {
    Int(i64),
    Float(f64),
    Bool(bool),
    Char(char),
    Str(String),
    Unit,
    Fn(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    /// String concatenation; the result is a `String`.
    Concat,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    /// `a in b`: membership in a list, set, dict (keys) or string.
    In,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
    BitNot,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Aggregate {
    Tuple,
    Struct(String),
    /// Type name and variant name.
    Variant(String, String),
    List,
    Set,
    /// Keys and values alternate.
    Dict,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Rvalue {
    Use(Operand),
    Ref(Place, bool),
    /// Arithmetic traps on overflow and division by zero (D-009).
    Binary(BinOp, Operand, Operand),
    Unary(UnOp, Operand),
    Aggregate(Aggregate, Vec<Operand>),
    /// The variant index of the enum value at a place.
    Discriminant(Place),
    /// A closure value: its function and its captured references.
    Closure(String, Vec<Operand>),
    /// A deep copy of the value at a place (`.copy()`).
    CopyOf(Place),
    /// A `str` view of the `String` (or `str`) at a place.
    StrView(Place),
}

impl BinOp {
    pub const ALL: &'static [(BinOp, &'static str)] = &[
        (BinOp::Add, "add"),
        (BinOp::Sub, "sub"),
        (BinOp::Mul, "mul"),
        (BinOp::Div, "div"),
        (BinOp::FloorDiv, "floordiv"),
        (BinOp::Mod, "mod"),
        (BinOp::Pow, "pow"),
        (BinOp::WrapAdd, "wadd"),
        (BinOp::WrapSub, "wsub"),
        (BinOp::WrapMul, "wmul"),
        (BinOp::SatAdd, "sadd"),
        (BinOp::SatSub, "ssub"),
        (BinOp::SatMul, "smul"),
        (BinOp::Shl, "shl"),
        (BinOp::Shr, "shr"),
        (BinOp::BitAnd, "bitand"),
        (BinOp::BitOr, "bitor"),
        (BinOp::BitXor, "bitxor"),
        (BinOp::Concat, "concat"),
        (BinOp::Eq, "eq"),
        (BinOp::Ne, "ne"),
        (BinOp::Lt, "lt"),
        (BinOp::Le, "le"),
        (BinOp::Gt, "gt"),
        (BinOp::Ge, "ge"),
        (BinOp::In, "in"),
    ];

    pub fn name(self) -> &'static str {
        BinOp::ALL.iter().find(|(o, _)| *o == self).map(|(_, n)| *n).unwrap_or("?")
    }

    pub fn from_name(s: &str) -> Option<BinOp> {
        BinOp::ALL.iter().find(|(_, n)| *n == s).map(|(o, _)| *o)
    }
}

impl UnOp {
    pub fn name(self) -> &'static str {
        match self {
            UnOp::Neg => "neg",
            UnOp::Not => "not",
            UnOp::BitNot => "bitnot",
        }
    }

    pub fn from_name(s: &str) -> Option<UnOp> {
        match s {
            "neg" => Some(UnOp::Neg),
            "not" => Some(UnOp::Not),
            "bitnot" => Some(UnOp::BitNot),
            _ => None,
        }
    }
}

impl Program {
    pub fn function(&self, name: &str) -> Option<&Function> {
        self.functions.iter().find(|f| f.name == name)
    }

    pub fn type_decl(&self, name: &str) -> Option<&TypeDecl> {
        self.types.iter().find(|t| t.name() == name)
    }
}

#[cfg(test)]
mod tests;
