//! The typed tree (THIR): what the checker produces for every body, broken
//! or not, and what lowering to SIR consumes. Every expression carries its
//! type; ill-typed subterms have type `Ty::Error` and kind `Error`, so the
//! tree is complete even when the program is not.
//!
//! Compared with the AST, names are resolved (locals, functions, variants),
//! sugar is decided (`it` closures, `Type(...)` constructors, `range(n)`),
//! implicit operations are explicit (auto-deref, borrows of arguments,
//! coercions), and literals have their final types.

use sisu_diag::Span;
use sisu_resolve::{DefId, Ty};
use sisu_syntax::ast::{BinOp, CmpOp, UnOp};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LocalId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ClosureId(pub u32);

#[derive(Clone, Debug)]
pub struct Local {
    pub name: String,
    pub ty: Ty,
    /// `var`, or a `let` declared without a value (assigned later).
    pub mutable: bool,
    /// A `let` declared without a value: it may be assigned once on each
    /// path, which the ownership checker's dataflow decides.
    pub once: bool,
    pub span: Span,
    /// The closure whose parameter or local this is; `None` for the function.
    pub closure: Option<ClosureId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BodyKind {
    Fn,
    /// The default value of a struct field, `(struct, field index)`.
    FieldDefault(DefId, u32),
    /// The default value of a parameter, `(function, parameter index)`.
    ParamDefault(DefId, u32),
    Const,
}

#[derive(Clone, Debug)]
pub struct Body {
    pub def: DefId,
    pub kind: BodyKind,
    /// Parameters in order, `self` first for a method.
    pub params: Vec<LocalId>,
    pub locals: Vec<Local>,
    pub closures: Vec<Closure>,
    pub ret: Ty,
    pub block: Block,
    pub span: Span,
    /// Whether any diagnostic was reported in this body.
    pub has_errors: bool,
    /// A generator's item type: the body has a `yield`, and calling the
    /// function makes an `Iter` of these.
    pub generator: Option<Ty>,
}

#[derive(Clone, Debug)]
pub struct Capture {
    pub local: LocalId,
    /// Captured by `ref mut` because the closure modifies it.
    pub mutable: bool,
    /// Kept by value in the environment rather than borrowed: every
    /// capture of an `own` closure, and a task's `Copy` captures it does
    /// not change (decided once the types are known).
    pub by_value: bool,
}

/// What a closure is for, which lowering and the ownership checker need
/// (06, "Tasks and scopes").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClosureRole {
    Plain,
    /// The body of `task.scope` or `task.race`.
    ScopeBody,
    /// A task, written at its `spawn`.
    Task,
}

#[derive(Clone, Debug)]
pub struct Closure {
    pub params: Vec<LocalId>,
    pub captures: Vec<Capture>,
    /// `own x -> ...`: the closure keeps copies of what it captures rather
    /// than borrowing it.
    pub own: bool,
    pub ret: Ty,
    /// An inline closure `x -> e` is a block holding `return e`.
    pub body: Block,
    pub span: Span,
    pub fn_ty: Ty,
    /// The body of `comptime e` (08): no parameters, no captures, pure,
    /// run by the compiler.
    pub comptime: bool,
    pub role: ClosureRole,
}

#[derive(Clone, Debug)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unwrap {
    Option,
    Result,
}

#[derive(Clone, Debug)]
pub enum LetElse {
    /// `else: v` and `else as e: v`: `v` replaces the missing value.
    Fallback { binding: Option<LocalId>, value: Expr },
    /// A block that must leave the scope.
    Diverge { binding: Option<LocalId>, body: Block },
}

#[derive(Clone, Debug)]
pub enum Stmt {
    /// `unwrap` is set when `init` is a `T?` or `T ! E` whose value the
    /// pattern binds, with `else_` handling the missing case.
    Let {
        pat: Pat,
        init: Option<Expr>,
        unwrap: Option<Unwrap>,
        else_: Option<LetElse>,
        span: Span,
    },
    Assign {
        place: Expr,
        value: Expr,
        span: Span,
    },
    /// `a, b = x, y`: all values are computed, then assigned in order.
    AssignTuple {
        places: Vec<Expr>,
        value: Expr,
        span: Span,
    },
    /// `_ = e`.
    Discard {
        value: Expr,
        span: Span,
    },
    AugAssign {
        place: Expr,
        op: BinOp,
        value: Expr,
        span: Span,
    },
    Expr(Expr),
    Return {
        value: Option<Expr>,
        span: Span,
    },
    Break(Span),
    Continue(Span),
    If {
        branches: Vec<(Cond, Block)>,
        else_: Option<Block>,
        span: Span,
    },
    While {
        cond: Cond,
        body: Block,
        span: Span,
    },
    For {
        pat: Pat,
        iter: Expr,
        mode: ForMode,
        item: Ty,
        body: Block,
        span: Span,
    },
    Match {
        scrutinee: Expr,
        arms: Vec<Arm>,
        span: Span,
    },
    /// `text` is the condition as written, for the panic message.
    Assert {
        cond: Expr,
        message: Option<Expr>,
        text: String,
        span: Span,
    },
    Defer {
        body: Block,
        span: Span,
    },
    /// A nested scope (from `with`).
    Block(Block),
    /// `select:` (06, "Channels and select").
    Select {
        arms: Vec<SelectArm>,
        span: Span,
    },
    /// `yield e` in a generator.
    Yield {
        value: Expr,
        span: Span,
    },
    Error(Span),
}

/// `case pattern = operation:` of a `select`.
#[derive(Clone, Debug)]
pub struct SelectArm {
    pub op: SelectOp,
    pub pat: Pat,
    pub body: Block,
    pub span: Span,
}

/// What a `select` arm waits for. A call's arguments are evaluated before
/// the `select` waits, and the call runs when its arm is taken; an event
/// is evaluated, then waited for, and its arm binds `()`.
#[derive(Clone, Debug)]
pub enum SelectOp {
    /// `rx.recv()`, the call.
    Recv(Expr),
    /// `tx.send(v)`.
    Send(Expr),
    /// `h.join()`.
    Join(Expr),
    /// A `Timer` value (`clock.after(d)`).
    Timer(Expr),
    /// A `Cancellation` value (`s.cancelled()`).
    Cancelled(Expr),
}

/// How a `for` loop or comprehension takes its items.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForMode {
    /// Borrows a collection; items are references.
    Borrow,
    /// Borrows a collection mutably; items are `ref mut`.
    Mut,
    /// Consumes a collection.
    Own,
    /// Drives an `Iter` value.
    Iter,
    /// Drives a value of another type that implements `Iterator`.
    Trait,
}

#[derive(Clone, Debug)]
pub struct Cond {
    pub parts: Vec<CondPart>,
}

#[derive(Clone, Debug)]
pub enum CondPart {
    Bool(Expr),
    Let { pat: Pat, value: Expr },
}

#[derive(Clone, Debug)]
pub struct Arm {
    pub pat: Pat,
    pub guard: Option<Expr>,
    pub body: ArmBody,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum ArmBody {
    Expr(Expr),
    Block(Block),
}

#[derive(Clone, Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub ty: Ty,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum Callee {
    Fn {
        def: DefId,
        type_args: Vec<Ty>,
    },
    /// A function value: a local, a field, any expression of function type.
    Value(Box<Expr>),
}

/// How an argument is passed (04, parameter conventions).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArgMode {
    /// Moved, or copied when the type is `Copy`.
    Value,
    /// A shared borrow of the argument place (or of a temporary).
    Borrow,
    /// An exclusive borrow; the argument is a mutable place.
    BorrowMut,
    /// A shared borrow even of a `Copy` value: the parameter's declared
    /// type is a generic parameter (or an associated type of one), which
    /// every caller passes by reference.
    Ref,
}

#[derive(Clone, Debug)]
pub struct Arg {
    pub expr: Expr,
    pub mode: ArgMode,
}

#[derive(Clone, Debug)]
pub enum FieldInit {
    Value(Expr),
    /// The field's default, evaluated at this construction.
    Default(u32),
}

#[derive(Clone, Debug)]
pub enum InterpPart {
    Text(String),
    /// `debug` holds the source text for `{expr=}`; `spec` is the format
    /// spec after `:`, empty if none; a spec of `?` asks for `Debug`.
    Value {
        expr: Expr,
        debug: Option<String>,
        spec: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompKind {
    List,
    Set,
    Dict,
}

#[derive(Clone, Debug)]
pub enum CompClause {
    For { pat: Pat, iter: Expr, mode: ForMode, item: Ty },
    If(Expr),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TryKind {
    Option,
    /// `from` is the operand's error set, `to` the function's.
    Result {
        from: Vec<DefId>,
        to: Vec<DefId>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndexKind {
    List,
    /// `xs[^i]`.
    ListFromEnd,
    Dict,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Coercion {
    /// `Never` to anything: the expression does not return.
    NeverToAny,
    /// `String` viewed as `str`.
    StrView,
    /// `ref String` viewed as `str`.
    RefStrView,
    /// `ref T` to `T` for a `Copy` type.
    DerefCopy,
    /// `ref mut T` used as `ref T`.
    MutToShared,
    /// An error set widened to a superset, possibly inside `T ! E`.
    Widen,
    /// A value of a type that implements the trait, as a `dyn Trait`.
    ToDyn,
    /// `Box[T]` to `Box[dyn Trait]`.
    BoxToDyn,
    /// `ref T` to `ref dyn Trait`: the reference, carrying `T`.
    RefToDyn,
    /// A value of a type that implements a capability's operations trait,
    /// as the capability's handle (06): the handle takes the value over.
    ToCap,
    /// An integer as a wider integer type that holds all its values.
    IntWiden,
    /// A type error: the expression is kept, its type replaced by the
    /// expected one so nothing downstream reports again.
    Mismatch,
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Int(i64),
    Float(f64),
    Bool(bool),
    Char(char),
    /// A string literal of type `str`, `String` or `Path` (FromLiteral).
    Str(String),
    /// A byte string literal, `b"..."`.
    Bytes(Vec<u8>),
    Unit,
    Local(LocalId),
    Const(DefId),
    FnRef(DefId),
    Call {
        callee: Callee,
        args: Vec<Arg>,
    },
    Variant {
        def: DefId,
        variant: u32,
        args: Vec<Expr>,
    },
    Struct {
        def: DefId,
        fields: Vec<FieldInit>,
    },
    Tuple(Vec<Expr>),
    List(Vec<Expr>),
    Dict(Vec<(Expr, Expr)>),
    Set(Vec<Expr>),
    /// A struct field or tuple element of a place or value.
    Field {
        base: Box<Expr>,
        index: u32,
    },
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
        kind: IndexKind,
    },
    /// `xs[lo:hi]` on a list: a new list of copies.
    Slice {
        base: Box<Expr>,
        lo: Option<Box<Expr>>,
        hi: Option<Box<Expr>>,
    },
    /// The place a reference points to.
    Deref(Box<Expr>),
    Borrow {
        place: Box<Expr>,
        mutable: bool,
    },
    Unary {
        op: UnOp,
        operand: Box<Expr>,
    },
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    /// `and` / `or`, short-circuiting.
    Logic {
        and: bool,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Compare {
        first: Box<Expr>,
        rest: Vec<(CmpOp, Expr)>,
    },
    /// `opt ?? default`; `deref` when the option holds a `ref T` of a
    /// `Copy` type and the result is the `T`.
    Coalesce {
        opt: Box<Expr>,
        default: Box<Expr>,
        deref: bool,
    },
    If {
        cond: Box<Expr>,
        then: Box<Expr>,
        else_: Box<Expr>,
    },
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<Arm>,
    },
    Closure(ClosureId),
    /// `comptime e`: the value of the closure, which takes nothing and
    /// captures nothing, computed at compile time.
    Comptime(ClosureId),
    /// A generator's `Iter`: the closure runs the generator's body, which
    /// owns the function's parameters, as items are asked for.
    Generator(ClosureId),
    Try {
        expr: Box<Expr>,
        kind: TryKind,
    },
    /// `base?.rest`: `binder` holds the value inside `base`.
    OptChain {
        base: Box<Expr>,
        binder: LocalId,
        then: Box<Expr>,
        flatten: bool,
    },
    Interp(Vec<InterpPart>),
    Comprehension {
        kind: CompKind,
        clauses: Vec<CompClause>,
        elem: Box<Expr>,
        value: Option<Box<Expr>>,
    },
    Coerce {
        expr: Box<Expr>,
        kind: Coercion,
    },
    /// The default of parameter `param` of `def`, evaluated at this call.
    DefaultArg {
        def: DefId,
        param: u32,
    },
    /// `x.copy()`: an explicit deep copy (04, rule 1).
    CopyOf(Box<Expr>),
    /// `e.trace()` on an error: its error return trace as text (05).
    Trace(Box<Expr>),
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindMode {
    /// Takes the value: a move, or a copy for a `Copy` type.
    Value,
    /// Binds a shared reference to the matched part.
    Ref,
    /// Binds an exclusive reference.
    RefMut,
}

#[derive(Clone, Debug)]
pub struct Pat {
    pub kind: PatKind,
    pub ty: Ty,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum Lit {
    Int(i64),
    Float(f64),
    Bool(bool),
    Char(char),
    Str(String),
}

#[derive(Clone, Debug)]
pub enum PatKind {
    Wild,
    Bind {
        local: LocalId,
        mode: BindMode,
    },
    Lit(Lit),
    /// A variant with one pattern per field.
    Variant {
        def: DefId,
        variant: u32,
        fields: Vec<Pat>,
    },
    Struct {
        def: DefId,
        fields: Vec<Pat>,
    },
    Tuple(Vec<Pat>),
    /// `[a, b, *rest, z]`: `rest` is `Some(binding)` when a star is present.
    List {
        prefix: Vec<Pat>,
        rest: Option<Option<LocalId>>,
        suffix: Vec<Pat>,
    },
    Or(Vec<Pat>),
    /// Matching goes through a reference to the value (`ref T` scrutinee).
    Deref(Box<Pat>),
    Error,
}

impl Expr {
    pub fn error(span: Span) -> Expr {
        Expr { kind: ExprKind::Error, ty: Ty::Error, span }
    }

    pub fn new(kind: ExprKind, ty: Ty, span: Span) -> Expr {
        Expr { kind, ty, span }
    }

    /// Whether the expression denotes a place (a local, a field or element
    /// of a place, or the target of a reference) rather than a temporary.
    pub fn is_place(&self) -> bool {
        match &self.kind {
            ExprKind::Local(_) | ExprKind::Deref(_) | ExprKind::Index { .. } => true,
            ExprKind::Field { base, .. } => base.is_place(),
            _ => false,
        }
    }

    /// Whether the place is reached through a reference or an index, from
    /// which a value cannot be moved out.
    pub fn is_behind_ref(&self) -> bool {
        match &self.kind {
            ExprKind::Deref(_) | ExprKind::Index { .. } => true,
            ExprKind::Field { base, .. } => base.is_behind_ref(),
            _ => false,
        }
    }

    /// The local at the root of a place, if any.
    pub fn place_root(&self) -> Option<LocalId> {
        match &self.kind {
            ExprKind::Local(l) => Some(*l),
            ExprKind::Field { base, .. } | ExprKind::Index { base, .. } => base.place_root(),
            _ => None,
        }
    }
}

impl Block {
    pub fn empty(span: Span) -> Block {
        Block { stmts: Vec::new(), span }
    }
}

/// The checker's result for a whole program.
#[derive(Clone, Debug, Default)]
pub struct Typed {
    pub bodies: Vec<Body>,
    /// The inferred error set of each `T !` function.
    pub inferred: std::collections::HashMap<DefId, Vec<DefId>>,
}

impl Typed {
    pub fn body_of(&self, def: DefId, kind: &BodyKind) -> Option<&Body> {
        self.bodies.iter().find(|b| b.def == def && b.kind == *kind)
    }
}
