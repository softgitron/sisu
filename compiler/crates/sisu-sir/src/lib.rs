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
//! Lowering invariant (checked by [`validate()`]): a `drop` outside a
//! cleanup block is the last statement of its block, so a block's unwind
//! edge is exact for every statement in it.
//!
//! Lowering emits every `drop` unconditionally; the ownership checker
//! (`sisu-borrowck`) runs the initialisation dataflow, removes the drops of
//! values that were moved, guards the drops of values that may have been
//! moved with a drop flag, and splits the drop of a partly moved value
//! into drops of the parts that remain.

pub mod read;
pub mod simplify;
pub mod types;
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
    /// The host intrinsics the functions call, with the facts the SIR
    /// checkers need about each (purity, which arguments the result
    /// borrows from).
    pub intrinsics: Vec<IntrinsicDecl>,
    pub functions: Vec<Function>,
    /// Test name and the function that runs it.
    pub tests: Vec<(String, String)>,
    /// Each test's `@tag`s, in the order written, by the test's place in
    /// `tests` (a test past its end has none; decision 285).
    pub test_tags: Vec<Vec<String>>,
    /// Benchmark name and the function that runs it (decision 285).
    pub benches: Vec<(String, String)>,
    /// The trait methods called through `call trait`, with the facts the
    /// checkers need about them (as for intrinsics).
    pub trait_methods: Vec<IntrinsicDecl>,
    /// The impls a `call trait` may dispatch to at run time.
    pub impls: Vec<ImplDecl>,
}

/// `impl[N] Trait[args] for Type { method: target, ... }`: what a call of
/// one of the trait's methods runs for a type matching the pattern. The
/// impl's parameters are `$0` to `$(N-1)` in the patterns; a method the
/// impl does not list is the trait's provided one, the function named
/// `Trait.method`.
#[derive(Clone, Debug, PartialEq)]
pub struct ImplDecl {
    pub params: u32,
    pub trait_name: String,
    pub trait_args: Vec<Type>,
    pub self_ty: Type,
    pub methods: Vec<(String, ImplTarget)>,
    /// What the impl binds the trait's associated types to (`type Item =
    /// $0`), for `<T as Trait>.Name` at run time.
    pub assoc: Vec<(String, Type)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ImplTarget {
    Fn(String),
    Intrinsic(String),
}

/// What the checkers know about a host intrinsic: its contract in the
/// prelude, reduced to the facts that matter on SIR.
#[derive(Clone, Debug, PartialEq)]
pub struct IntrinsicDecl {
    pub name: String,
    pub pure: bool,
    /// Arguments (0-based) the result may borrow from, deeply: the result
    /// may hold the argument's own loan and every loan it reaches.
    pub from: Vec<u32>,
    /// Arguments whose referent's loans the result may hold, but not the
    /// loan of the reference itself (`Iter.next(mut self) -> T?` yields
    /// what the iterator borrows, not the iterator).
    pub inner: Vec<u32>,
    /// Arguments taken with `own`: the only ones the intrinsic may keep in
    /// what its `mut` arguments point to (`list.push(mut self, own x)`).
    pub own: Vec<u32>,
    /// Arguments taken with `own` or `mut` whose every value the intrinsic
    /// keeps or returns (its contract's `@keeps`): only these may take
    /// must-consume values (D-054); any other may drop what it is given.
    pub keeps: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub function: String,
    pub takes_system: bool,
    pub returns_result: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TypeDecl {
    Struct {
        name: String,
        /// Generic parameters, `$0` and on in the fields' types.
        params: Vec<String>,
        fields: Vec<FieldDecl>,
        drop: Option<String>,
        /// A `view struct`: it may hold references (04, "Views").
        view: bool,
        /// `@must_consume` (D-054): a value must be passed to an `own`
        /// parameter, never dropped on a normal path.
        must_consume: bool,
    },
    Enum {
        name: String,
        params: Vec<String>,
        variants: Vec<VariantDecl>,
        drop: Option<String>,
        error: bool,
        open: bool,
        must_consume: bool,
    },
}

impl TypeDecl {
    pub fn name(&self) -> &str {
        match self {
            TypeDecl::Struct { name, .. } | TypeDecl::Enum { name, .. } => name,
        }
    }

    pub fn params(&self) -> &[String] {
        match self {
            TypeDecl::Struct { params, .. } | TypeDecl::Enum { params, .. } => params,
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
    /// Parameters, result, and whether the function is `pure`.
    Fn(Vec<Type>, Box<Type>, bool),
    Ref(Box<Type>, bool),
    /// Type parameter `N` of the function (or of the type declaration).
    Param(u32),
    /// `dyn Trait[args]`: a value of a type implementing the trait, which
    /// carries its type.
    Dyn(String, Vec<Type>),
    /// `<T as Trait[args]>.Name`: an associated type of a type parameter.
    Assoc(Box<Type>, String, Vec<Type>, String),
    /// The argument of a `const` generic parameter: the `256` of
    /// `Array[u32, 256]`, written as the number.
    Const(i128),
    /// An integer operation on constant arguments not known yet, `W * H`
    /// in `Array[T, W * H]` (03, "Const generics"), written `mul($1, $2)`:
    /// substituting constants for its parameters folds it to a `Const`.
    ConstOp(ConstOp, Vec<Type>),
}

/// The operations a type's constant argument may compute, on `int`
/// values with the run time's rules: `//` floors, `%` takes the
/// divisor's sign, and overflow, a zero divisor, a negative exponent and a
/// shift outside 0 to 63 are faults.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ConstOp {
    Add,
    Sub,
    Mul,
    FloorDiv,
    Mod,
    Pow,
    Shl,
    Shr,
    BitAnd,
    BitOr,
    BitXor,
    Neg,
    BitNot,
}

/// Why a constant operation has no value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConstFault {
    Overflow,
    DivideByZero,
    NegativeExponent,
    ShiftOutOfRange,
}

impl ConstOp {
    pub const ALL: [ConstOp; 13] = [
        ConstOp::Add,
        ConstOp::Sub,
        ConstOp::Mul,
        ConstOp::FloorDiv,
        ConstOp::Mod,
        ConstOp::Pow,
        ConstOp::Shl,
        ConstOp::Shr,
        ConstOp::BitAnd,
        ConstOp::BitOr,
        ConstOp::BitXor,
        ConstOp::Neg,
        ConstOp::BitNot,
    ];

    /// Its name in SIR, the operator's (`mul`, `neg`).
    pub fn name(self) -> &'static str {
        match self {
            ConstOp::Add => "add",
            ConstOp::Sub => "sub",
            ConstOp::Mul => "mul",
            ConstOp::FloorDiv => "floordiv",
            ConstOp::Mod => "mod",
            ConstOp::Pow => "pow",
            ConstOp::Shl => "shl",
            ConstOp::Shr => "shr",
            ConstOp::BitAnd => "bitand",
            ConstOp::BitOr => "bitor",
            ConstOp::BitXor => "bitxor",
            ConstOp::Neg => "neg",
            ConstOp::BitNot => "bitnot",
        }
    }

    pub fn from_name(n: &str) -> Option<ConstOp> {
        ConstOp::ALL.iter().copied().find(|o| o.name() == n)
    }

    /// Its spelling in Sisu (`*`, unary `-`).
    pub fn symbol(self) -> &'static str {
        match self {
            ConstOp::Add => "+",
            ConstOp::Sub | ConstOp::Neg => "-",
            ConstOp::Mul => "*",
            ConstOp::FloorDiv => "//",
            ConstOp::Mod => "%",
            ConstOp::Pow => "**",
            ConstOp::Shl => "<<",
            ConstOp::Shr => ">>",
            ConstOp::BitAnd => "&",
            ConstOp::BitOr => "|",
            ConstOp::BitXor => "^",
            ConstOp::BitNot => "~",
        }
    }

    pub fn is_unary(self) -> bool {
        matches!(self, ConstOp::Neg | ConstOp::BitNot)
    }

    /// The value on `int` operands, as the run time computes it.
    pub fn fold(self, args: &[i128]) -> Result<i128, ConstFault> {
        let int = |i: usize| -> Result<i64, ConstFault> {
            args.get(i).and_then(|&v| i64::try_from(v).ok()).ok_or(ConstFault::Overflow)
        };
        let x = int(0)?;
        if self.is_unary() {
            return match self {
                ConstOp::Neg => x.checked_neg().map(i128::from).ok_or(ConstFault::Overflow),
                _ => Ok(i128::from(!x)),
            };
        }
        let y = int(1)?;
        let v = match self {
            ConstOp::Add => x.checked_add(y).ok_or(ConstFault::Overflow)?,
            ConstOp::Sub => x.checked_sub(y).ok_or(ConstFault::Overflow)?,
            ConstOp::Mul => x.checked_mul(y).ok_or(ConstFault::Overflow)?,
            ConstOp::FloorDiv | ConstOp::Mod if y == 0 => return Err(ConstFault::DivideByZero),
            ConstOp::FloorDiv => {
                let q = x.checked_div(y).ok_or(ConstFault::Overflow)?;
                if x % y != 0 && ((x < 0) != (y < 0)) { q - 1 } else { q }
            }
            ConstOp::Mod => {
                let r = x.checked_rem(y).unwrap_or(0);
                if r != 0 && ((r < 0) != (y < 0)) { r + y } else { r }
            }
            ConstOp::Pow if y < 0 => return Err(ConstFault::NegativeExponent),
            ConstOp::Pow => match u32::try_from(y) {
                Ok(e) => x.checked_pow(e).ok_or(ConstFault::Overflow)?,
                Err(_) => match x {
                    0 | 1 => x,
                    -1 => {
                        if y % 2 == 0 {
                            1
                        } else {
                            -1
                        }
                    }
                    _ => return Err(ConstFault::Overflow),
                },
            },
            ConstOp::Shl | ConstOp::Shr if !(0..64).contains(&y) => return Err(ConstFault::ShiftOutOfRange),
            ConstOp::Shl => x << y,
            ConstOp::Shr => x >> y,
            ConstOp::BitAnd => x & y,
            ConstOp::BitOr => x | y,
            ConstOp::BitXor => x ^ y,
            ConstOp::Neg | ConstOp::BitNot => x,
        };
        Ok(i128::from(v))
    }
}

impl Type {
    /// Replaces `$i` by `args[i]` (and leaves a parameter without an
    /// argument as it is).
    pub fn subst(&self, args: &[Type]) -> Type {
        match self {
            Type::Param(i) => args.get(*i as usize).cloned().unwrap_or(Type::Param(*i)),
            Type::Named(n, a) => Type::Named(n.clone(), a.iter().map(|t| t.subst(args)).collect()),
            Type::Tuple(e) => Type::Tuple(e.iter().map(|t| t.subst(args)).collect()),
            Type::Fn(p, r, pure) => {
                Type::Fn(p.iter().map(|t| t.subst(args)).collect(), Box::new(r.subst(args)), *pure)
            }
            Type::Ref(t, m) => Type::Ref(Box::new(t.subst(args)), *m),
            Type::Dyn(n, a) => Type::Dyn(n.clone(), a.iter().map(|t| t.subst(args)).collect()),
            Type::Assoc(t, tr, a, n) => Type::Assoc(
                Box::new(t.subst(args)),
                tr.clone(),
                a.iter().map(|x| x.subst(args)).collect(),
                n.clone(),
            ),
            Type::ConstOp(op, a) => Type::const_op(*op, a.iter().map(|t| t.subst(args)).collect()),
            other => other.clone(),
        }
    }

    /// `op` on `args`, folded to a `Const` when every argument is one and
    /// the operation has a value.
    pub fn const_op(op: ConstOp, args: Vec<Type>) -> Type {
        let values: Option<Vec<i128>> =
            args.iter().map(|a| if let Type::Const(n) = a { Some(*n) } else { None }).collect();
        match values.map(|v| op.fold(&v)) {
            Some(Ok(n)) => Type::Const(n),
            _ => Type::ConstOp(op, args),
        }
    }

    /// Whether the type mentions a type parameter.
    pub fn has_params(&self) -> bool {
        match self {
            Type::Param(_) | Type::Assoc(..) => true,
            Type::Named(_, a) | Type::Tuple(a) | Type::Dyn(_, a) | Type::ConstOp(_, a) => {
                a.iter().any(Type::has_params)
            }
            Type::Fn(p, r, _) => p.iter().any(Type::has_params) || r.has_params(),
            Type::Ref(t, _) => t.has_params(),
            _ => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LocalId(pub u32);

#[derive(Clone, Debug, PartialEq)]
pub struct LocalDecl {
    pub ty: Type,
    /// The source name, for debugging, diagnostics and the SIR dump.
    pub name: Option<String>,
    /// Where the binding is declared, for diagnostics.
    pub loc: Option<Loc>,
    /// A `let` declared without a value: it may be assigned once on each
    /// path (the checker reports a second assignment).
    pub once: bool,
}

impl LocalDecl {
    pub fn new(ty: Type, name: Option<String>) -> LocalDecl {
        LocalDecl { ty, name, loc: None, once: false }
    }
}

/// What a closure is for, which the ownership checker's task rules need
/// (06, "Tasks and scopes").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClosureRole {
    /// Any other closure.
    Plain,
    /// The body of `task.scope` or `task.race`: parameter 2 is the scope,
    /// and the scope ends when the body has returned and its tasks are
    /// joined, so what the environment borrows outlives every task.
    ScopeBody,
    /// A task, written at its `spawn`: called once, it owns its
    /// environment (captured values are moved or copied into it) and drops
    /// it when it returns.
    Task,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FnKind {
    Normal,
    /// A closure: parameter 1 is the tuple of captured references (or,
    /// for a task, of the captured values and references).
    Closure(ClosureRole),
    /// The `drop` method of a type with `impl Drop`.
    Drop,
    /// The drop glue of a declared type, `T#drop` (see [`types::glue_name`]):
    /// it takes the value and runs the type's `Drop`, then drops the
    /// fields (of the active variant) in reverse order.
    Glue,
    /// A default value or a constant.
    Value,
    Test,
    /// A `comptime` expression (08): a function of no arguments that the
    /// compiler runs with this budget of steps, replacing its body by the
    /// value it returns (which makes it a `value` function).
    Comptime(u64),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Function {
    pub name: String,
    /// Generic parameters: `$0` and on in the function's types, whose
    /// arguments every call passes at run time (type arguments are values
    /// in stage 0; nothing is monomorphised).
    pub type_params: Vec<String>,
    pub kind: FnKind,
    pub param_count: u32,
    /// Declared `pure` (D-074), or a closure whose body is pure.
    pub pure: bool,
    /// The arguments (0-based; a closure's environment is argument 0) the
    /// result may borrow from: the `from` clause or the elision rules
    /// (04, "Borrowing rules in detail"), plus every owned parameter that
    /// can hold a loan. Empty when the result cannot hold a loan.
    pub from: Vec<u32>,
    /// Borrowed arguments whose values' loans the result may hold, but not
    /// the loan of the argument itself: the result cannot point into the
    /// argument's own storage (a `str` taken from a `list[str]` views the
    /// text the list's elements view, not the list).
    pub inner: Vec<u32>,
    /// The arguments taken with `own`: a caller assumes the call may keep
    /// what they hold in its `mut` arguments.
    pub own: Vec<u32>,
    pub locals: Vec<LocalDecl>,
    pub blocks: Vec<BasicBlock>,
    /// Where the function is declared (its name), for diagnostics about it
    /// as a whole (a `pure` function calling an impure one).
    pub loc: Option<Loc>,
    /// Index into `Program::sources` for statement locations.
    pub source: u32,
}

impl Function {
    pub fn ret(&self) -> &Type {
        &self.locals[0].ty
    }
}

/// The end of the name of a function's value wrapper (`first#value`).
pub const VALUE_WRAPPER: &str = "#value";

/// The value wrapper of `f`, generic as `f` is: what `f` is as a value
/// when its constant would not do (a generic function, whose constant
/// cannot name type arguments; one that borrows a parameter, which a
/// function value takes). It is a closure that takes an empty environment
/// and `takes` (the parameters as `f`'s function type has them), and calls
/// `f`'s instance for its own type arguments: a parameter `f` borrows is
/// lent, and dropped once the call returns or panics; any other is moved
/// in.
pub fn value_wrapper(f: &Function, takes: &[Type]) -> Function {
    let n = f.param_count as usize;
    let mut locals =
        vec![LocalDecl::new(f.ret().clone(), None), LocalDecl::new(Type::Unit, Some("env".to_string()))];
    // How each parameter goes to `f`: lent (with the mutability of `f`'s
    // reference) or moved.
    let mut lent: Vec<Option<bool>> = Vec::new();
    for (i, l) in f.locals.iter().skip(1).take(n).enumerate() {
        let mut decl = l.clone();
        let how = match (&l.ty, takes.get(i)) {
            (Type::Ref(inner, m), Some(t)) if **inner == *t && !matches!(t, Type::Ref(..)) => {
                decl.ty = t.clone();
                Some(*m)
            }
            _ => None,
        };
        lent.push(how);
        locals.push(decl);
    }
    let mut stmts = Vec::new();
    let mut args = Vec::new();
    for (i, how) in lent.iter().enumerate() {
        let param = Place::local(LocalId(i as u32 + 2));
        match how {
            Some(m) => {
                let r = LocalId(locals.len() as u32);
                locals.push(LocalDecl::new(Type::Ref(Box::new(locals[i + 2].ty.clone()), *m), None));
                stmts.push(Statement {
                    kind: StatementKind::Assign(Place::local(r), Rvalue::Ref(param, *m)),
                    loc: f.loc,
                });
                args.push(Operand::Copy(Place::local(r)));
            }
            None => args.push(Operand::Move(param)),
        }
    }
    let tys = (0..f.type_params.len() as u32).map(Type::Param).collect();
    stmts.push(Statement {
        kind: StatementKind::Call {
            dest: Place::local(LocalId(0)),
            callee: Callee::Fn(f.name.clone(), tys),
            args,
        },
        loc: f.loc,
    });
    // The lent parameters are dropped after the call, last first, each in
    // a block of its own, and by a cleanup block when the call panics.
    let dropped: Vec<Place> = lent
        .iter()
        .enumerate()
        .rev()
        .filter(|(_, how)| how.is_some())
        .map(|(i, _)| Place::local(LocalId(i as u32 + 2)))
        .collect();
    let block = |stmts: Vec<Statement>, term: Terminator, unwind: Option<BlockId>, cleanup: bool| {
        BasicBlock { stmts, term, unwind, cleanup, loc: None }
    };
    let drop =
        |p: &Place| Statement { kind: StatementKind::Drop { place: p.clone(), flag: None }, loc: f.loc };
    let mut blocks = Vec::new();
    if dropped.is_empty() {
        blocks.push(block(stmts, Terminator::Return, None, false));
    } else {
        let cleanup = BlockId(dropped.len() as u32 + 2);
        blocks.push(block(stmts, Terminator::Goto(BlockId(1)), Some(cleanup), false));
        for (k, p) in dropped.iter().enumerate() {
            blocks.push(block(vec![drop(p)], Terminator::Goto(BlockId(k as u32 + 2)), None, false));
        }
        blocks.push(block(Vec::new(), Terminator::Return, None, false));
        blocks.push(block(dropped.iter().map(drop).collect(), Terminator::Resume, None, true));
    }
    // What the result may borrow from, a place further on (the
    // environment is argument 0): a lent parameter whose values' loans it
    // may hold is a parameter of the wrapper that holds them.
    let is_lent = |i: &u32| lent.get(*i as usize).is_some_and(Option::is_some);
    let mut from: Vec<u32> = f.from.iter().filter(|i| !is_lent(i)).map(|i| i + 1).collect();
    from.extend(f.inner.iter().filter(|i| is_lent(i)).map(|i| i + 1));
    from.sort_unstable();
    from.dedup();
    let inner = f.inner.iter().filter(|i| !is_lent(i)).map(|i| i + 1).collect();
    Function {
        name: format!("{}{VALUE_WRAPPER}", f.name),
        type_params: f.type_params.clone(),
        kind: FnKind::Closure(ClosureRole::Plain),
        param_count: f.param_count + 1,
        pure: f.pure,
        from,
        inner,
        own: Vec::new(),
        locals,
        blocks,
        loc: f.loc,
        source: f.source,
    }
}

/// The source-level name of a SIR function, for stack traces and the
/// messages that name a frame, in the interpreter and natively alike
/// (stage 2's `gen.sisu` has the same): a test by its name, an invariant's
/// check by its type, a closure by the function it is in, and a method of
/// a trait's impl as `Trait.method for Type` (decision 504), where its SIR
/// name is `Type.Trait.method`, the type's path and the trait's in it.
pub fn display_fn_name(name: &str, prog: &Program) -> String {
    if let Some(i) = name.strip_prefix("test#").and_then(|n| n.parse::<usize>().ok())
        && let Some((t, _)) = prog.tests.get(i)
    {
        return format!("test {}", write::quote(t));
    }
    if let Some(owner) = name.strip_suffix(".__check_invariant") {
        return format!("the invariant of {owner}");
    }
    if let Some((owner, _)) = name.split_once("#closure") {
        return format!("closure in {}", display_fn_name(owner, prog));
    }
    // `Type.Trait.method`: a prefix that names a declared type, then a
    // trait's path whose last name is capitalised, then the method.
    let segs: Vec<&str> = name.split('.').collect();
    let n = segs.len();
    if n >= 3 && segs[n - 2].starts_with(|c: char| c.is_ascii_uppercase()) {
        let mut at = 0;
        for k in 1..n - 1 {
            at += segs[k - 1].len() + usize::from(k > 1);
            let ty = &name[..at];
            if prog.types.iter().any(|t| t.name() == ty) {
                let method = segs[n - 1];
                return format!("{}.{method} for {ty}", segs[n - 2]);
            }
        }
    }
    name.to_string()
}

/// Whether a test function takes the real `System` (17): one parameter,
/// a `System`. A test that names capabilities instead receives fakes of
/// them (decision 241).
pub fn takes_real_system(f: &Function) -> bool {
    f.param_count == 1 && matches!(f.locals.get(1).map(|l| &l.ty), Some(Type::Named(n, _)) if n == "System")
}

#[derive(Clone, Debug, PartialEq)]
pub struct BasicBlock {
    pub stmts: Vec<Statement>,
    pub term: Terminator,
    pub unwind: Option<BlockId>,
    pub cleanup: bool,
    pub loc: Option<Loc>,
}

/// A 1-based source range: the start, and the end when it is known
/// (`end_line == 0` means a position only).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Loc {
    pub line: u32,
    pub col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

impl Loc {
    pub fn point(line: u32, col: u32) -> Loc {
        Loc { line, col, end_line: 0, end_col: 0 }
    }

    pub fn has_end(&self) -> bool {
        self.end_line != 0
    }
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
    /// place is left uninitialised. Dropping the payload of a variant
    /// (`drop _3.as(0).0`) when the value holds another variant drops
    /// nothing.
    Drop {
        place: Place,
        flag: Option<LocalId>,
    },
    /// The local's storage ends (`dead _5`): it holds no value from here,
    /// and a reference to it must not be used again. Lowering emits it at
    /// the end of the scope of every local that is borrowed somewhere.
    StorageDead(LocalId),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Callee {
    /// A function, with its type arguments.
    Fn(String, Vec<Type>),
    Intrinsic(String, Vec<Type>),
    /// `Trait.method` for the type arguments `[Self, trait arguments...,
    /// the method's own...]`, dispatched at run time to the impl for
    /// `Self`.
    Trait(String, Vec<Type>),
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
    /// A `dyn` value holding the operand, whose type is given.
    Dyn(Type),
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
    /// A closure value: its function, the type arguments of the function
    /// that makes it, and its captured references.
    Closure(String, Vec<Type>, Vec<Operand>),
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

    pub fn intrinsic(&self, name: &str) -> Option<&IntrinsicDecl> {
        self.intrinsics.iter().find(|i| i.name == name)
    }

    pub fn trait_method(&self, name: &str) -> Option<&IntrinsicDecl> {
        self.trait_methods.iter().find(|i| i.name == name)
    }
}

#[cfg(test)]
mod tests;
