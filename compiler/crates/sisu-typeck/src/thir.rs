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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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
    /// The function's contract (07): what lowering checks on entry and on
    /// every exit.
    pub contracts: Option<Contracts>,
    /// Its `comptime assert`s on its `const` generic parameters, which only
    /// a call with constant arguments can decide: each is checked at every
    /// such call, once the program is typed (`generic_asserts.rs`).
    pub generic_asserts: Vec<GenericAssert>,
    /// What the per-item queries know of the body (docs/architecture.md,
    /// "The query layer").
    pub memo: Memo,
}

/// A body's identity for the per-item queries: `key` is the same for two
/// checks exactly when they give the same typed body up to where its item
/// is, which is `file` from byte `start` (the start of the line its name is
/// on) to `end`. A key of 0 is a body no later query may reuse.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Memo {
    pub key: u64,
    pub file: sisu_diag::FileId,
    pub start: u32,
    pub end: u32,
}

/// A `comptime assert` on `const` generic parameters: its condition, its
/// message, and the condition as written.
#[derive(Clone, Debug)]
pub struct GenericAssert {
    pub cond: Expr,
    pub message: Option<Expr>,
    pub text: String,
}

/// What a check of a contract is for (07), which names it when it fails.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContractKind {
    Requires,
    Ensures,
    Invariant,
    /// A constant written where a refinement type is expected, checked at
    /// compile time.
    Refinement,
}

/// One clause of a contract, checked where lowering puts it.
#[derive(Clone, Debug)]
pub struct ContractCheck {
    pub kind: ContractKind,
    pub cond: Expr,
    /// The clause as written, without its keyword.
    pub text: String,
    /// The clause's expression.
    pub span: Span,
    /// The condition only reads: it calls `pure` functions and changes
    /// nothing, so when it fails its parts can be evaluated again to show
    /// their values (17, "Assertions").
    pub replay: bool,
}

/// A function's contract: `requires` on entry, `ensures` on every exit
/// (with `result` and the values `old(e)` saved on entry), and for a `mut
/// self` method of a type with an invariant, the invariant on every exit.
#[derive(Clone, Debug, Default)]
pub struct Contracts {
    pub requires: Vec<ContractCheck>,
    pub ensures: Vec<ContractCheck>,
    /// `old(e)`: the local that holds `e`'s value from the entry, and `e`.
    pub olds: Vec<(LocalId, Expr)>,
    /// The local `ensures` calls `result`: the returned value, by
    /// reference unless it is `Copy`.
    pub result: Option<LocalId>,
    /// The labels of a labelled result (`-> (q: int, r: int)`), each the
    /// local that names the element at that index.
    pub labels: Vec<(LocalId, u32)>,
    /// The invariant check a `mut self` method runs on every exit, with
    /// its type arguments.
    pub invariant: Option<(DefId, Vec<Ty>)>,
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
    /// `text` is the condition as written, for the panic message; with
    /// `replay`, the condition only reads, so its parts can be evaluated
    /// again to show their values when it fails.
    Assert {
        cond: Expr,
        message: Option<Expr>,
        text: String,
        span: Span,
        replay: bool,
    },
    /// A clause of a contract checked here: an `invariant` line in the
    /// check the compiler writes for a struct, a refinement's predicate
    /// on a constant.
    Contract(ContractCheck),
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
    /// An `f32` as the `f64` that holds it exactly.
    FloatWiden,
    /// A value of a refinement type used as its base (07): the same value.
    Unrefine,
    /// A type error: the expression is kept, its type replaced by the
    /// expected one so nothing downstream reports again.
    Mismatch,
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    /// An integer literal: its value, or for a `u64` its 64 bits.
    Int(i64),
    /// A literal of `i128` or `u128` that `i64` cannot hold: its 128 bits.
    WideInt(u128),
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
    /// The default of field `field` of the struct `def`, evaluated here
    /// (`reflect.default_of`, 08).
    FieldDefault {
        def: DefId,
        field: u32,
    },
    /// `x.copy()`: an explicit deep copy (04, rule 1).
    CopyOf(Box<Expr>),
    /// `e.trace()` on an error: its error return trace as text (05).
    Trace(Box<Expr>),
    /// `X.try_from(v)` for a refinement type `X` (07): `Ok(v)` when `v`
    /// satisfies the predicate, else `Err(Unsatisfied(...))`.
    RefineTry {
        value: Box<Expr>,
        refinement: DefId,
        type_args: Vec<Ty>,
    },
    /// The value of the function's `const` generic parameter at this
    /// index (03, "Const generics"), an `int`.
    ConstParam(u32),
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
    WideInt(u128),
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
            // A refinement's value is its base's: the same place.
            ExprKind::Coerce { expr, kind: Coercion::Unrefine } => expr.is_place(),
            _ => false,
        }
    }

    /// Whether the place is reached through a reference or an index, from
    /// which a value cannot be moved out.
    pub fn is_behind_ref(&self) -> bool {
        match &self.kind {
            ExprKind::Deref(_) | ExprKind::Index { .. } => true,
            ExprKind::Field { base, .. } => base.is_behind_ref(),
            ExprKind::Coerce { expr, kind: Coercion::Unrefine } => expr.is_behind_ref(),
            _ => false,
        }
    }

    /// The local at the root of a place, if any.
    pub fn place_root(&self) -> Option<LocalId> {
        match &self.kind {
            ExprKind::Local(l) => Some(*l),
            ExprKind::Field { base, .. } | ExprKind::Index { base, .. } => base.place_root(),
            ExprKind::Coerce { expr, kind: Coercion::Unrefine } => expr.place_root(),
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
    /// Shared with the per-item memo, which keeps the bodies it reuses.
    pub bodies: Vec<std::sync::Arc<Body>>,
    /// The inferred error set of each `T !` function.
    pub inferred: std::collections::HashMap<DefId, Vec<DefId>>,
    /// The instances of templates the checked bodies call without one yet,
    /// and the generic functions that call one with their own type
    /// parameters (08, "Reflection"): what the next resolution declares.
    pub requests: Vec<sisu_resolve::Request>,
}

impl Typed {
    pub fn body_of(&self, def: DefId, kind: &BodyKind) -> Option<&Body> {
        self.bodies.iter().find(|b| b.def == def && b.kind == *kind).map(|b| &**b)
    }
}
