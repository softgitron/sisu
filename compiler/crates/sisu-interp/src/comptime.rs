//! Compile-time evaluation (08, "Compile-time evaluation"; 18, Phase 4: "the
//! comptime evaluator as the same interpreter with fuel, a memory limit and
//! no `System`").
//!
//! A `comptime` expression is lowered to a function of nothing, of kind
//! `comptime N`. The compiler runs each one, in program order, in this
//! interpreter with a budget of `N` steps and a host through which nothing
//! reaches the world, then replaces its body by statements that build the
//! value it returned, which makes it a `value` function: at run time the
//! program does only what it would do for a literal. A panic, a spent
//! budget, a use of the world (printing, files, the environment, the
//! clock) or a value with no literal form (a function, a reference, a
//! `Shared`) is an error of the program at the `comptime` expression.
//!
//! The memory limit of 08 is not enforced yet (port-debt.md).

use crate::host::{Host, IoErrorKind};
use crate::value::Value;
use crate::{Interp, Options, Unwind, on_big_stack};
use sisu_sir as sir;
use sisu_sir::{
    Aggregate, BasicBlock, Callee, Const, LocalDecl, LocalId, Operand, Place, Rvalue, Statement,
    StatementKind, Terminator, Type, TypeDecl,
};
use std::cell::Cell;
use std::collections::HashMap;

/// Why a `comptime` function has no value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    /// It panicked, with this message.
    Panicked(String),
    /// A refinement's predicate was false for a constant (07): the type,
    /// the value and the predicate.
    Refinement { ty: String, value: String, predicate: String },
    /// It ran out of its budget of steps.
    Budget(u64),
    /// It used the world: what it used.
    World(&'static str),
    /// Its value has no form that can be kept for run time: what it is.
    NotKept(String),
    /// The interpreter stopped on an internal error.
    Internal(String),
}

/// A `comptime` function that failed, by name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub function: String,
    pub failure: Failure,
}

/// The host at compile time: nothing reaches the world. The first use of
/// it is recorded, which makes the evaluation an error.
#[derive(Default)]
struct ComptimeHost {
    used: Cell<Option<&'static str>>,
}

impl ComptimeHost {
    fn touch(&self, what: &'static str) {
        if self.used.get().is_none() {
            self.used.set(Some(what));
        }
    }
}

impl Host for ComptimeHost {
    fn stdout(&mut self, _: &str) {
        self.touch("the standard output");
    }

    fn stderr(&mut self, _: &str) {
        self.touch("the standard error");
    }

    fn args(&self) -> Vec<String> {
        self.touch("the program's arguments");
        Vec::new()
    }

    fn env_var(&self, _: &str) -> Option<String> {
        self.touch("the environment");
        None
    }

    fn now(&self) -> i64 {
        self.touch("the clock");
        0
    }

    fn read_file(&mut self, _: &str) -> Result<String, IoErrorKind> {
        self.touch("the file system");
        Err(IoErrorKind::NotFound)
    }

    fn write_file(&mut self, _: &str, _: &str) -> Result<(), IoErrorKind> {
        self.touch("the file system");
        Err(IoErrorKind::PermissionDenied)
    }

    fn exists(&self, _: &str) -> bool {
        self.touch("the file system");
        false
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Running,
    Done,
    Failed,
}

/// Runs every `comptime` function of the program and bakes its value into
/// it. Returns the functions that failed (which keep their bodies).
///
/// A `comptime` expression that uses another one's value (a constant
/// computed at compile time) is run after it, so each spends its own
/// budget; one that uses a failed one is not run, its error being that
/// one's.
pub fn evaluate(prog: &mut sir::Program) -> Vec<Error> {
    let mut errors = Vec::new();
    let mut state: HashMap<String, State> = HashMap::new();
    let names: Vec<String> = prog
        .functions
        .iter()
        .filter(|f| matches!(f.kind, sir::FnKind::Comptime(_)))
        .map(|f| f.name.clone())
        .collect();
    for n in names {
        visit(prog, &n, &mut state, &mut errors);
    }
    errors
}

fn visit(prog: &mut sir::Program, name: &str, state: &mut HashMap<String, State>, errors: &mut Vec<Error>) {
    if state.contains_key(name) {
        return;
    }
    state.insert(name.to_string(), State::Running);
    for dep in comptime_deps(prog, name) {
        visit(prog, &dep, state, errors);
    }
    if comptime_deps(prog, name).iter().any(|d| state.get(d) == Some(&State::Failed)) {
        state.insert(name.to_string(), State::Failed);
        return;
    }
    let Some(i) = prog.functions.iter().position(|f| f.name == name) else { return };
    let sir::FnKind::Comptime(budget) = prog.functions[i].kind else { return };
    match run(prog, name, budget) {
        Ok((locals, stmts, intrinsics)) => {
            let f = &mut prog.functions[i];
            f.locals = locals;
            f.blocks = vec![BasicBlock {
                stmts,
                term: Terminator::Return,
                unwind: None,
                cleanup: false,
                loc: f.loc,
            }];
            f.kind = sir::FnKind::Value;
            for n in intrinsics {
                declare_intrinsic(prog, n);
            }
            state.insert(name.to_string(), State::Done);
        }
        Err(failure) => {
            errors.push(Error { function: name.to_string(), failure });
            state.insert(name.to_string(), State::Failed);
        }
    }
}

/// The `comptime` functions that the function `name` calls, directly or
/// through functions that are not themselves `comptime`.
fn comptime_deps(prog: &sir::Program, name: &str) -> Vec<String> {
    let mut seen: Vec<&str> = vec![name];
    let mut work: Vec<&str> = vec![name];
    let mut deps = Vec::new();
    while let Some(n) = work.pop() {
        let Some(f) = prog.function(n) else { continue };
        let mut callees: Vec<&str> = Vec::new();
        for b in &f.blocks {
            for s in &b.stmts {
                match &s.kind {
                    StatementKind::Call { callee: Callee::Fn(c, _), args, .. } => {
                        callees.push(c);
                        callees.extend(fn_consts(args));
                    }
                    StatementKind::Call { args, .. } => callees.extend(fn_consts(args)),
                    StatementKind::Assign(_, Rvalue::Closure(c, _, ops)) => {
                        callees.push(c);
                        callees.extend(fn_consts(ops));
                    }
                    StatementKind::Assign(_, Rvalue::Use(o)) => {
                        callees.extend(fn_consts(std::slice::from_ref(o)))
                    }
                    StatementKind::Assign(_, Rvalue::Aggregate(_, ops)) => callees.extend(fn_consts(ops)),
                    _ => {}
                }
            }
        }
        for c in callees {
            if seen.contains(&c) {
                continue;
            }
            seen.push(c);
            match prog.function(c).map(|g| g.kind) {
                Some(sir::FnKind::Comptime(_)) => deps.push(c.to_string()),
                Some(_) => work.push(c),
                None => {}
            }
        }
    }
    deps
}

fn fn_consts(ops: &[Operand]) -> impl Iterator<Item = &str> {
    ops.iter().filter_map(|o| match o {
        Operand::Const(Const::Fn(n)) => Some(n.as_str()),
        _ => None,
    })
}

/// The baked body: locals and statements, and the intrinsics they call.
type Baked = (Vec<LocalDecl>, Vec<Statement>, Vec<&'static str>);

fn run(prog: &sir::Program, name: &str, budget: u64) -> Result<Baked, Failure> {
    let mut host = ComptimeHost::default();
    let result = on_big_stack(&mut host, |host| {
        let options = Options { fuel: Some(budget), ..Options::default() };
        let mut it = Interp::new(prog, host, &options);
        let r = it.call_by_name(name, Vec::new());
        let spent = it.fuel == Some(0);
        match r {
            Ok(v) => {
                let ret = prog.function(name).map(|f| f.ret().clone()).unwrap_or(Type::Unit);
                let mut b = Baker {
                    types: &it.types,
                    locals: vec![LocalDecl::new(ret.clone(), None)],
                    stmts: Vec::new(),
                    intrinsics: Vec::new(),
                };
                b.build(&v, &ret, Place::local(LocalId(0)))
                    .map(|()| (b.locals, b.stmts, b.intrinsics))
                    .map_err(Failure::NotKept)
            }
            Err(Unwind::Panic(p)) => match p.refinement {
                Some((ty, value, predicate)) => Err(Failure::Refinement { ty, value, predicate }),
                None => Err(Failure::Panicked(p.message)),
            },
            Err(Unwind::Abort(_)) if spent => Err(Failure::Budget(budget)),
            Err(Unwind::Abort(m)) => Err(Failure::Internal(m)),
            Err(Unwind::Block(_) | Unwind::Switch) => {
                Err(Failure::Internal("a task switch escaped the run loop".into()))
            }
        }
    });
    if let Some(what) = host.used.get() {
        return Err(Failure::World(what));
    }
    result.unwrap_or_else(|| Err(Failure::Internal("the evaluation thread did not finish".into())))
}

fn declare_intrinsic(prog: &mut sir::Program, name: &str) {
    if prog.intrinsic(name).is_some() {
        return;
    }
    let at = prog.intrinsics.partition_point(|d| d.name.as_str() < name);
    prog.intrinsics.insert(
        at,
        sir::IntrinsicDecl {
            name: name.to_string(),
            pure: true,
            from: Vec::new(),
            inner: Vec::new(),
            own: Vec::new(),
            keeps: Vec::new(),
        },
    );
}

/// Builds the statements that make a value.
struct Baker<'a> {
    types: &'a crate::fx::FxMap<&'a str, &'a TypeDecl>,
    locals: Vec<LocalDecl>,
    stmts: Vec<Statement>,
    intrinsics: Vec<&'static str>,
}

fn assign(dest: Place, rv: Rvalue) -> Statement {
    Statement { kind: StatementKind::Assign(dest, rv), loc: None }
}

fn arg(t: &Type, i: usize) -> Type {
    match t {
        Type::Named(_, a) => a.get(i).cloned().unwrap_or(Type::Unit),
        _ => Type::Unit,
    }
}

impl Baker<'_> {
    /// Builds `v` of type `ty` as an operand: a constant, or a new local
    /// that holds it.
    fn part(&mut self, v: &Value, ty: Type) -> Result<Operand, String> {
        let k = match v {
            Value::Unit => Some(Const::Unit),
            Value::Int(i) => Some(Const::Int(*i)),
            Value::Float(f) => Some(Const::Float(*f)),
            Value::Bool(b) => Some(Const::Bool(*b)),
            Value::Char(c) => Some(Const::Char(*c)),
            Value::Str(s, _) => Some(Const::Str(s.to_string())),
            _ => None,
        };
        if let Some(k) = k {
            return Ok(Operand::Const(k));
        }
        let l = LocalId(self.locals.len() as u32);
        self.locals.push(LocalDecl::new(ty.clone(), None));
        self.build(v, &ty, Place::local(l))?;
        Ok(Operand::Move(Place::local(l)))
    }

    fn call(&mut self, dest: Place, name: &'static str, text: &str) {
        if !self.intrinsics.contains(&name) {
            self.intrinsics.push(name);
        }
        self.stmts.push(Statement {
            kind: StatementKind::Call {
                dest,
                callee: Callee::Intrinsic(name.into(), Vec::new()),
                args: vec![Operand::Const(Const::Str(text.to_string()))],
            },
            loc: None,
        });
    }

    fn build(&mut self, v: &Value, ty: &Type, dest: Place) -> Result<(), String> {
        let c = |k: Const| Rvalue::Use(Operand::Const(k));
        let rv = match v {
            Value::Unit => c(Const::Unit),
            Value::Int(i) => c(Const::Int(*i)),
            Value::Float(f) => c(Const::Float(*f)),
            Value::Bool(b) => c(Const::Bool(*b)),
            Value::Char(ch) => c(Const::Char(*ch)),
            Value::Str(s, _) => c(Const::Str(s.to_string())),
            Value::String(s) => {
                self.call(dest, "String.new", s);
                return Ok(());
            }
            Value::Path(p) => {
                self.call(dest, "Path.new", p);
                return Ok(());
            }
            Value::Bytes(b) => {
                let hex: String = b.iter().map(|x| format!("{x:02x}")).collect();
                if !self.intrinsics.contains(&"bytes.from_hex") {
                    self.intrinsics.push("bytes.from_hex");
                }
                self.stmts.push(Statement {
                    kind: StatementKind::Call {
                        dest,
                        callee: Callee::Intrinsic("bytes.from_hex".into(), Vec::new()),
                        args: vec![Operand::Const(Const::Str(hex))],
                    },
                    loc: None,
                });
                return Ok(());
            }
            Value::U64(_) | Value::I128(_) | Value::U128(_) | Value::F32(_) => {
                // A value of a kind of its own: a `u64` from its bits, a
                // 128-bit integer from its decimal text, an `f32` from the
                // `f64` that holds it.
                let (name, arg) = match v {
                    Value::U64(n) => ("u64.from_bits", Const::Int(*n as i64)),
                    Value::I128(n) => ("i128.from_text", Const::Str(n.to_string())),
                    Value::U128(n) => ("u128.from_text", Const::Str(n.to_string())),
                    Value::F32(f) => ("f32.from_literal", Const::Float(f64::from(*f))),
                    _ => return Err("an integer of its own kind".into()),
                };
                if !self.intrinsics.contains(&name) {
                    self.intrinsics.push(name);
                }
                self.stmts.push(Statement {
                    kind: StatementKind::Call {
                        dest,
                        callee: Callee::Intrinsic(name.into(), Vec::new()),
                        args: vec![Operand::Const(arg)],
                    },
                    loc: None,
                });
                return Ok(());
            }
            Value::Tuple(xs) => {
                let ts = match ty {
                    Type::Tuple(ts) => ts.clone(),
                    _ => vec![Type::Unit; xs.len()],
                };
                let mut ops = Vec::new();
                for (x, t) in xs.iter().zip(ts) {
                    ops.push(self.part(x, t)?);
                }
                Rvalue::Aggregate(Aggregate::Tuple, ops)
            }
            Value::List(xs) => {
                let mut ops = Vec::new();
                for x in xs {
                    ops.push(self.part(x, arg(ty, 0))?);
                }
                Rvalue::Aggregate(Aggregate::List, ops)
            }
            Value::Set(s) => {
                let mut ops = Vec::new();
                for x in &s.items {
                    ops.push(self.part(x, arg(ty, 0))?);
                }
                Rvalue::Aggregate(Aggregate::Set, ops)
            }
            Value::Dict(d) => {
                let mut ops = Vec::new();
                for (k, x) in &d.entries {
                    ops.push(self.part(k, arg(ty, 0))?);
                    ops.push(self.part(x, arg(ty, 1))?);
                }
                Rvalue::Aggregate(Aggregate::Dict, ops)
            }
            Value::Struct(a) => {
                let mut ops = Vec::new();
                for (i, x) in a.fields.iter().enumerate() {
                    let t = self.field(ty, &a.ty, None, i)?;
                    ops.push(self.part(x, t)?);
                }
                Rvalue::Aggregate(Aggregate::Struct(a.ty.to_string()), ops)
            }
            Value::Variant(a) => {
                let name = match (&*a.ty, a.variant) {
                    ("Option", 0) => "Some".to_string(),
                    ("Option", _) => "None".to_string(),
                    ("Result", 0) => "Ok".to_string(),
                    ("Result", _) => "Err".to_string(),
                    (t, i) => match self.types.get(t) {
                        Some(TypeDecl::Enum { variants, .. }) => {
                            variants.get(i as usize).map(|v| v.name.clone()).unwrap_or_default()
                        }
                        _ => return Err(format!("a value of type `{t}`")),
                    },
                };
                let mut ops = Vec::new();
                for (i, x) in a.fields.iter().enumerate() {
                    let t = self.field(ty, &a.ty, Some(a.variant), i)?;
                    ops.push(self.part(x, t)?);
                }
                Rvalue::Aggregate(Aggregate::Variant(a.ty.to_string(), name), ops)
            }
            Value::Dyn(d) => {
                let inner = self.part(&d.value, d.ty.clone())?;
                Rvalue::Aggregate(Aggregate::Dyn(d.ty.clone()), vec![inner])
            }
            other => return Err(kind_of(other)),
        };
        self.stmts.push(assign(dest, rv));
        Ok(())
    }

    /// The type of field `i` of a value of type `ty` (declared `name`), of
    /// variant `v` for an enum.
    fn field(&self, ty: &Type, name: &str, v: Option<u32>, i: usize) -> Result<Type, String> {
        let decl = |n: &str| self.types.get(n).copied();
        // An error's payload is typed by its own error set.
        let ty = match ty {
            Type::ErrSet(_) => Type::Named(name.to_string(), Vec::new()),
            t => t.clone(),
        };
        sir::types::field_ty(&decl, &ty, v, i as u32).ok_or_else(|| format!("a value of type `{name}`"))
    }
}

fn kind_of(v: &Value) -> String {
    match v {
        Value::Ref(_) => "a reference",
        Value::Fn(_) | Value::Closure(_) => "a function",
        Value::Shared(_) => "a `Shared`",
        Value::Weak(_) => "a `Weak`",
        Value::Cell(_) => "a `Cell`",
        Value::Mutex(_) => "a `Mutex`",
        Value::Guard(_) => "a mutex guard",
        Value::ReadGuard(_) => "a read guard",
        Value::Arena(_) => "an `Arena`",
        Value::Iter(_) => "an iterator",
        Value::Entry(_) => "a dict entry",
        Value::Cap(_) | Value::Clock(_) => "a capability",
        Value::DateTime(_) => "a `DateTime`",
        _ => "a value",
    }
    .to_string()
}
