//! The typing rules of tasks (06, "Tasks and scopes"), which the signatures
//! in `std/task.sisu` and the prelude cannot say:
//!
//! - the body of `task.scope`, `task.race` and `task.shield`, and the task
//!   of `spawn`, are written at the call (a closure or `do` block), so the
//!   checker sees what they capture;
//! - a scope whose body returns `T ! E` returns `T ! {E, TaskError}`
//!   widened by the error sets of the tasks spawned on it in the body,
//!   since a failure nobody joined becomes the scope's result;
//! - a race returns its tasks' result, flattened as `join` flattens it;
//! - `Handle[U ! E].join()` is `U ! {E, TaskError}`, and
//!   `Handle[T].join()` is `T ! TaskError` for any other `T`;
//! - a scope's body cannot return its scope or a handle of its tasks.
//!
//! What a task captures is decided once the types are known
//! ([`Checker::decide_captures`]): its `Copy` captures it does not change
//! are copied into it, the rest are borrowed, and an `own` task moves its
//! captures in. The `Send` and `Sync` checks on them are in `auto.rs`.

use crate::check::Checker;
use crate::thir::*;
use sisu_diag::{Span, msg};
use sisu_resolve::{DefId, Ty};

/// A call of `spawn` seen while checking a body.
#[derive(Clone, Debug)]
pub(crate) struct SpawnSite {
    /// The local the scope was reached through (`s` in `s.spawn`).
    pub scope: Option<LocalId>,
    pub closure: ClosureId,
}

/// The calls the rules apply to, by intrinsic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TaskCall {
    Scope,
    Race,
    Shield,
    /// `spawn`, with the index of the task argument.
    Spawn(usize),
    Join,
}

pub(crate) fn task_call(intrinsic: Option<&str>) -> Option<TaskCall> {
    Some(match intrinsic? {
        "task.scope" => TaskCall::Scope,
        "task.race" => TaskCall::Race,
        "task.shield" => TaskCall::Shield,
        "Scope.spawn" => TaskCall::Spawn(1),
        "Scope.spawn_with" => TaskCall::Spawn(2),
        "Handle.join" => TaskCall::Join,
        _ => return None,
    })
}

/// The closure an argument is, under coercions.
fn closure_of(e: &Expr) -> Option<ClosureId> {
    match &e.kind {
        ExprKind::Closure(id) => Some(*id),
        ExprKind::Coerce { expr, .. } => closure_of(expr),
        _ => None,
    }
}

/// The local an expression reaches a value through: `s`, `ref s`, `*s`.
fn root_local(e: &Expr) -> Option<LocalId> {
    match &e.kind {
        ExprKind::Local(l) => Some(*l),
        ExprKind::Borrow { place, .. } => root_local(place),
        ExprKind::Deref(x) | ExprKind::Coerce { expr: x, .. } => root_local(x),
        _ => None,
    }
}

impl Checker<'_> {
    /// Applies the rules to a call just checked: `args` are the checked
    /// arguments (the receiver first for a method), `ret` the type the
    /// signature gave. Returns the call's type.
    pub(crate) fn task_rules(
        &mut self,
        call: TaskCall,
        args: &[Arg],
        ret: Ty,
        spawns_before: usize,
        span: Span,
        function: &str,
    ) -> Ty {
        let prog = self.prog;
        match call {
            TaskCall::Scope | TaskCall::Race | TaskCall::Shield => {
                let Some(id) = args.first().and_then(|a| closure_of(&a.expr)) else {
                    self.closure_needed(args.first(), span, function);
                    return ret;
                };
                if call == TaskCall::Shield {
                    return ret;
                }
                self.set_role(id, ClosureRole::ScopeBody);
                let Some(cl) = self.closures.get(id.0 as usize).and_then(|c| c.clone()) else { return ret };
                let param = cl.params.first().copied();
                let body_ret = self.infer.zonk(&cl.ret);
                // A scope's body cannot hand its scope or a task's handle out.
                if self.mentions_scope(&body_ret) {
                    let d = self.error(msg::TASK_SCOPE_ESCAPES, span.range).with("ty", self.show(&body_ret));
                    self.report(d);
                }
                let sites: Vec<SpawnSite> = self.spawn_sites[spawns_before..]
                    .iter()
                    .filter(|s| s.scope.is_some() && s.scope == param)
                    .cloned()
                    .collect();
                if call == TaskCall::Race {
                    let t = self.infer.fresh();
                    for site in &sites {
                        if let Some(Some(c)) = self.closures.get(site.closure.0 as usize) {
                            let r = c.ret.clone();
                            if !self.infer.unify(&t, &r) {
                                let d = self
                                    .error(msg::TASK_RACE_TYPES, c.span.range)
                                    .with("found", self.show(&r))
                                    .with("expected", self.show(&t));
                                self.report(d);
                            }
                        }
                    }
                    return self.flatten_join(&t, span);
                }
                // The scope's result carries the failures nobody joined.
                let mut sets: Vec<DefId> = Vec::new();
                for site in &sites {
                    if let Some(Some(c)) = self.closures.get(site.closure.0 as usize) {
                        let r = c.ret.clone();
                        if let Some((_, e)) = self.as_result(&r)
                            && let Ty::ErrSet(m) = self.resolved(&e)
                        {
                            sets = sisu_resolve::ty::union(&sets, &m);
                        }
                    }
                }
                self.widen_scope(&body_ret, &sets)
            }
            TaskCall::Spawn(k) => {
                let Some(id) = args.get(k).and_then(|a| closure_of(&a.expr)) else {
                    self.closure_needed(args.get(k), span, function);
                    return ret;
                };
                self.set_role(id, ClosureRole::Task);
                let scope = args.first().and_then(|a| root_local(&a.expr));
                self.spawn_sites.push(SpawnSite { scope, closure: id });
                self.spawn_checks.push((id, span));
                let _ = prog;
                ret
            }
            TaskCall::Join => {
                let Ty::Adt(_, targs) =
                    self.resolved(args.first().map(|a| a.expr.ty.strip_ref()).unwrap_or(&Ty::Error))
                else {
                    return ret;
                };
                match targs.first() {
                    Some(t) => self.flatten_join(t, span),
                    None => ret,
                }
            }
        }
    }

    fn set_role(&mut self, id: ClosureId, role: ClosureRole) {
        if let Some(Some(c)) = self.closures.get_mut(id.0 as usize) {
            c.role = role;
        }
    }

    fn closure_needed(&mut self, arg: Option<&Arg>, span: Span, function: &str) {
        if arg.is_some_and(|a| a.expr.ty == Ty::Error) {
            return;
        }
        let at = arg.map(|a| a.expr.span).unwrap_or(span);
        let d = self.error(msg::TASK_CLOSURE_LITERAL, at.range).with("function", function.to_string());
        self.report(d);
    }

    /// Whether a type holds a `Scope` or a `Handle`, which end with their
    /// scope.
    fn mentions_scope(&self, t: &Ty) -> bool {
        let wk = &self.prog.wk;
        let mut found = false;
        t.walk(&mut |x| {
            if let Ty::Adt(d, _) = x
                && (Some(*d) == wk.scope || Some(*d) == wk.handle)
            {
                found = true;
            }
        });
        found
    }

    fn task_error_set(&self) -> Vec<DefId> {
        self.prog.wk.task_error.into_iter().collect()
    }

    fn result_of(&self, ok: Ty, err: Vec<DefId>) -> Ty {
        match self.prog.wk.result {
            Some(r) => Ty::Adt(r, vec![ok, Ty::ErrSet(err)]),
            None => Ty::Error,
        }
    }

    /// `join`'s type for a task returning `t`.
    pub(crate) fn flatten_join(&mut self, t: &Ty, span: Span) -> Ty {
        let te = self.task_error_set();
        if let Some((ok, e)) = self.as_result(t) {
            let m = match self.resolved(&e) {
                Ty::ErrSet(m) => m,
                Ty::Var(_) => {
                    self.infer.unify(&e, &Ty::ErrSet(Vec::new()));
                    Vec::new()
                }
                _ => Vec::new(),
            };
            return self.result_of(ok, sisu_resolve::ty::union(&m, &te));
        }
        match self.resolved(t) {
            Ty::Var(_) => {
                let d = self.error(msg::TASK_RESULT_UNKNOWN, span.range);
                self.report(d);
                Ty::Error
            }
            Ty::Error => Ty::Error,
            other => self.result_of(other, te),
        }
    }

    /// A scope's type for a body returning `body`: a result is widened by
    /// `TaskError` and the error sets of its tasks.
    fn widen_scope(&mut self, body: &Ty, sets: &[DefId]) -> Ty {
        let Some((ok, e)) = self.as_result(body) else { return body.clone() };
        let m = match self.resolved(&e) {
            Ty::ErrSet(m) => m,
            Ty::Var(_) => {
                self.infer.unify(&e, &Ty::ErrSet(Vec::new()));
                Vec::new()
            }
            _ => Vec::new(),
        };
        let m = sisu_resolve::ty::union(&m, sets);
        let m = sisu_resolve::ty::union(&m, &self.task_error_set());
        self.result_of(ok, m)
    }

    /// Once types are known: what each task keeps by value. An `own`
    /// closure keeps everything; a task keeps the `Copy` values it does not
    /// change, and borrows the rest.
    pub(crate) fn decide_captures(&mut self) {
        let prog = self.prog;
        for i in 0..self.closures.len() {
            let Some(Some(c)) = self.closures.get(i) else { continue };
            let (own, role) = (c.own, c.role);
            let caps = c.captures.clone();
            let mut out = Vec::new();
            for mut cap in caps {
                let ty = self.infer.zonk(&self.local_ty(cap.local));
                cap.by_value = own || (role == ClosureRole::Task && !cap.mutable && prog.is_copy(&ty));
                out.push(cap);
            }
            if let Some(Some(c)) = self.closures.get_mut(i) {
                c.captures = out;
            }
        }
    }
}

impl Checker<'_> {
    /// `select:` (06, "Channels and select"): each arm waits for a call of
    /// `recv`, `send` or `join`, or for a `Timer` or a `Cancellation`; its
    /// pattern binds what the call returns (`()` for an event) and cannot
    /// fail to match.
    pub(crate) fn check_select(&mut self, arms: &[sisu_syntax::ast::SelectArm], range: TextRange) -> Stmt {
        let span = self.span(range);
        let mut out = Vec::new();
        for a in arms {
            let x = self.infer_expr(&a.op, None);
            let (op, bound) = self.select_op(x);
            self.push_scope();
            let pat = if crate::check::pattern_is_refutable(&a.pat) {
                let d = self.error(msg::TYPE_REFUTABLE_LET, a.pat.range);
                self.report(d);
                Pat { kind: PatKind::Error, ty: Ty::Error, span: self.span(a.pat.range) }
            } else {
                self.check_pat(&a.pat, &bound, None)
            };
            let body = self.check_block(&a.body);
            self.pop_scope();
            out.push(SelectArm { op, pat, body, span: self.span(a.range) });
        }
        Stmt::Select { arms: out, span }
    }

    fn select_op(&mut self, x: Expr) -> (SelectOp, Ty) {
        if let ExprKind::Call { callee: Callee::Fn { def, .. }, .. } = &x.kind {
            let ty = x.ty.clone();
            match self.prog.fn_info(*def).and_then(|f| f.intrinsic.as_deref()) {
                Some("Receiver.recv") => return (SelectOp::Recv(x), ty),
                Some("Sender.send") => return (SelectOp::Send(x), ty),
                Some("Handle.join") => return (SelectOp::Join(x), ty),
                _ => {}
            }
        }
        if self.is_builtin(&x.ty, "Timer") {
            return (SelectOp::Timer(x), Ty::Unit);
        }
        if self.is_builtin(&x.ty, "Cancellation") {
            return (SelectOp::Cancelled(x), Ty::Unit);
        }
        if x.ty != Ty::Error {
            let d = self.error(msg::TASK_SELECT_ARM, x.span.range).with("ty", self.show(&x.ty));
            self.report(d);
        }
        let span = x.span;
        (SelectOp::Timer(Expr::error(span)), Ty::Error)
    }

    /// `yield e` in a generator: `e` is an item of the `Iter` the function
    /// returns.
    pub(crate) fn check_yield(&mut self, value: Option<&sisu_syntax::ast::Expr>, range: TextRange) -> Stmt {
        let span = self.span(range);
        let in_closure = self.ret.len() > 1;
        let Some(item) = self.generator.clone().filter(|_| !in_closure) else {
            let d = self.error(msg::TYPE_YIELD_OUTSIDE_GENERATOR, range);
            self.report(d);
            if let Some(v) = value {
                let _ = self.infer_expr(v, None);
            }
            return Stmt::Error(span);
        };
        let v = match value {
            Some(v) => {
                let x = self.check_expr(v, &item);
                self.check_movable(x)
            }
            None => Expr::new(ExprKind::Unit, Ty::Unit, span),
        };
        Stmt::Yield { value: v, span }
    }
}

/// Whether a function body yields: a `yield` outside the closures in it.
pub(crate) fn has_yield(b: &sisu_syntax::ast::Block) -> bool {
    use sisu_syntax::ast::{ArmBody, ExprKind as E, StmtKind as S};
    fn stmt(s: &sisu_syntax::ast::Stmt) -> bool {
        match &s.kind {
            S::Yield(_) => true,
            S::If { branches, else_block } => {
                branches.iter().any(|(_, b)| has_yield(b)) || else_block.as_ref().is_some_and(has_yield)
            }
            S::While { body, .. } | S::For { body, .. } | S::Defer(body) | S::With { body, .. } => {
                has_yield(body)
            }
            S::Select(arms) => arms.iter().any(|a| has_yield(&a.body)),
            S::Expr(e) => match &e.kind {
                E::Match { arms, .. } => arms.iter().any(|a| match &a.body {
                    ArmBody::Block(b) => has_yield(b),
                    ArmBody::Expr(_) => false,
                }),
                _ => false,
            },
            S::Comptime(inner) => stmt(inner),
            _ => false,
        }
    }
    b.stmts.iter().any(stmt)
}

use sisu_diag::TextRange;
