//! Contracts (07): `requires` and `ensures` clauses at the start of a
//! function's body, `old(e)` and `result` in `ensures`, a struct's
//! `invariant` lines (checked by the method the compiler writes for them),
//! and refinement types (`type Port = int where 1 <= self <= 65535`).
//!
//! A clause is a `bool` expression that only reads: it calls `pure`
//! functions and changes nothing (E0801, E0802), so a build may leave it
//! out (07, "which is what keeps them safe to disable") and a failed one
//! can be evaluated again, part by part, to show its values. Where each
//! check runs is lowering's: `requires` on entry, `ensures` and a `mut
//! self` method's invariant on every exit, a struct's invariant after it
//! is built.

use crate::check::Checker;
use crate::thir::*;
use sisu_diag::{TextRange, msg};
use sisu_resolve::{DefId, FnInfo, FnKind, Ty};
use sisu_syntax::ast::{self, ContractKind as AstKind, Mode};

/// The number of `requires` and `ensures` statements a body starts with.
pub fn leading_contracts(b: &ast::Block) -> usize {
    b.stmts
        .iter()
        .take_while(|s| {
            matches!(&s.kind, ast::StmtKind::Contract(c) if matches!(c.kind, AstKind::Requires | AstKind::Ensures))
        })
        .count()
}

/// While `ensures` clauses are checked: the scope holding `result` and
/// the labels, hidden inside `old(...)`, and the values `old` saves.
pub struct EnsuresCtx {
    pub scope: usize,
    pub olds: Vec<(LocalId, Expr)>,
}

impl Checker<'_> {
    /// The contract of the function being checked, from the clauses its
    /// body starts with, once its parameters are declared; `ret` is the
    /// type `result` is in `ensures`.
    pub fn check_contracts(&mut self, f: &FnInfo, clauses: &[&ast::Contract], ret: &Ty) -> Option<Contracts> {
        let prog = self.prog;
        let mut out = Contracts::default();
        // A `mut self` method of a type with an invariant checks it again
        // when it returns (07).
        if f.self_mode == Some(Mode::Mut)
            && !matches!(f.kind, FnKind::Invariant(_) | FnKind::Drop)
            && let Some(Ty::Adt(d, args)) = &f.owner
            && let Some(check) = prog.invariant_check(*d)
        {
            out.invariant = Some((check, args.clone()));
        }
        if matches!(f.kind, FnKind::Test(_)) {
            for c in clauses {
                let d = self.error(msg::CONTRACT_NOT_IN_TEST, c.range);
                self.report(d);
            }
            return None;
        }
        for c in clauses.iter().filter(|c| c.kind == AstKind::Requires) {
            let check = self.contract_clause(ContractKind::Requires, c);
            out.requires.push(check);
        }
        let ensures: Vec<&&ast::Contract> = clauses.iter().filter(|c| c.kind == AstKind::Ensures).collect();
        if !ensures.is_empty() {
            if self.generator.is_some() {
                let d = self.error(msg::CONTRACT_GENERATOR_ENSURES, ensures[0].range);
                self.report(d);
            } else {
                // `result` (and a labelled result's labels) name the value
                // returned, as a borrowed parameter would: by reference
                // unless it is `Copy`.
                self.push_scope();
                let scope = self.scopes.len() - 1;
                let span = self.span(ensures[0].range);
                let rty = crate::param_local_ty(prog, ret, Mode::Borrow, &self.env);
                let r = self.new_local("result", rty, false, span);
                self.declare("result", r, false, ensures[0].range);
                out.result = Some(r);
                if let Ty::Tuple(elems, labels) = &self.resolved(ret) {
                    for (i, (t, l)) in elems.iter().zip(labels).enumerate() {
                        let lt = crate::param_local_ty(prog, t, Mode::Borrow, &self.env);
                        let x = self.new_local(l, lt, false, span);
                        self.declare(l, x, false, ensures[0].range);
                        out.labels.push((x, i as u32));
                    }
                }
                self.ensures = Some(EnsuresCtx { scope, olds: Vec::new() });
                for c in ensures {
                    let check = self.contract_clause(ContractKind::Ensures, c);
                    out.ensures.push(check);
                }
                if let Some(e) = self.ensures.take() {
                    out.olds = e.olds;
                }
                self.pop_scope();
            }
        }
        let empty = out.requires.is_empty() && out.ensures.is_empty() && out.invariant.is_none();
        (!empty).then_some(out)
    }

    /// One clause: a condition that only reads.
    pub fn contract_clause(&mut self, kind: ContractKind, c: &ast::Contract) -> ContractCheck {
        let cond = self.check_condition_expr(&c.expr);
        let text = self.source_text(c.expr.range).trim_end().to_string();
        let replay = self.only_reads(&cond, true);
        ContractCheck { kind, cond, text, span: self.span(c.expr.range), replay }
    }

    /// Whether the expression only reads; with `report`, a contract's
    /// condition that does not is an error (07: "Contract expressions
    /// must be `pure`").
    pub fn only_reads(&mut self, e: &Expr, report: bool) -> bool {
        if e.ty == Ty::Error {
            return true;
        }
        let closures = &self.closures;
        let problem = crate::purity::reading_problem(self.prog, e, &|id| {
            closures.get(id.0 as usize).and_then(|c| c.as_ref()).map(|c| c.body.clone())
        });
        match problem {
            None => true,
            Some(p) => {
                if report {
                    let d = match p {
                        crate::purity::Reading::Calls(name) => {
                            self.error(msg::CONTRACT_NOT_PURE, e.span.range).with("callee", name)
                        }
                        crate::purity::Reading::Changes => self.error(msg::CONTRACT_CHANGES, e.span.range),
                    };
                    self.report(d);
                }
                false
            }
        }
    }

    /// A contract statement inside a body: an `invariant` line of the
    /// check the compiler writes for a struct, or a clause out of place.
    pub fn contract_stmt(&mut self, c: &ast::Contract, out: &mut Vec<Stmt>) {
        let span = self.span(c.range);
        let in_check = self.prog.fn_info(self.def).is_some_and(|f| matches!(f.kind, FnKind::Invariant(_)));
        match c.kind {
            AstKind::Invariant if in_check => {
                let check = self.contract_clause(ContractKind::Invariant, c);
                out.push(Stmt::Contract(check));
            }
            AstKind::Requires | AstKind::Ensures => {
                let is_test = self.prog.fn_info(self.def).is_some_and(|f| matches!(f.kind, FnKind::Test(_)));
                let d = if is_test {
                    self.error(msg::CONTRACT_NOT_IN_TEST, c.range)
                } else {
                    self.error(msg::CONTRACT_MISPLACED, c.range).with("keyword", c.kind.keyword())
                };
                self.report(d);
                out.push(Stmt::Error(span));
            }
            // Loop invariants and termination measures are reserved (07,
            // "What is deferred").
            AstKind::Invariant | AstKind::Decreases => {
                let what = if c.kind == AstKind::Invariant {
                    "a loop invariant (`invariant` in a body)"
                } else {
                    "a termination measure (`decreases`)"
                };
                self.unsupported(c.range, what, "the contract verifier (after version 1)");
                out.push(Stmt::Error(span));
            }
        }
    }

    /// `old(e)` in `ensures`: `e` as it was on entry, saved in a local
    /// then (a copy of a place that is not `Copy`).
    pub fn old_expr(&mut self, args: &[ast::Arg], trailing: Option<&ast::Closure>, range: TextRange) -> Expr {
        let span = self.span(range);
        let Some(scope) = self.ensures.as_ref().map(|e| e.scope) else {
            let d = self.error(msg::CONTRACT_OLD_OUTSIDE, range);
            self.report(d);
            self.check_args_loosely(args);
            return Expr::error(span);
        };
        let [arg] = args else {
            let d = self.error(msg::CONTRACT_OLD_ARGUMENTS, range);
            self.report(d);
            self.check_args_loosely(args);
            return Expr::error(span);
        };
        if arg.name.is_some() || trailing.is_some() {
            let d = self.error(msg::CONTRACT_OLD_ARGUMENTS, range);
            self.report(d);
            return Expr::error(span);
        }
        // `result` does not exist on entry.
        let hidden = std::mem::take(&mut self.scopes[scope]);
        let x = self.infer_expr(&arg.value, None);
        if let Some(s) = self.scopes.get_mut(scope) {
            *s = hidden;
        }
        let x = self.decay(x);
        let x = self.auto_deref(x);
        let ty = self.resolved(&x.ty);
        let value = if !self.is_copy(&ty) && x.is_place() {
            let s = x.span;
            Expr::new(ExprKind::CopyOf(Box::new(x)), ty.clone(), s)
        } else {
            x
        };
        let text = format!("old({})", self.source_text(arg.value.range));
        let l = self.new_local(&text, ty.clone(), false, span);
        if let Some(e) = &mut self.ensures {
            e.olds.push((l, value));
        }
        Expr::new(ExprKind::Local(l), ty, span)
    }

    /// A value of a refinement type as its base, the same value (07: a
    /// refinement "can be used wherever `T` can"), for operators, fields,
    /// indexing, iteration and the base's methods; a reference to one as a
    /// reference to its base. Anything else is returned as it is.
    pub fn unrefined(&mut self, x: Expr) -> Expr {
        let (ty, by_ref) = match self.resolved(&x.ty) {
            Ty::Ref(inner, m) => (self.resolved(&inner), Some(m)),
            t => (t, None),
        };
        if self.refinement_of(&ty).is_none() {
            return x;
        }
        // Through a refinement of a refinement to the type under both.
        let base = self.prog.unrefine(&ty);
        let to = match by_ref {
            Some(m) => Ty::Ref(Box::new(base), m),
            None => base,
        };
        let span = x.span;
        Expr::new(ExprKind::Coerce { expr: Box::new(x), kind: Coercion::Unrefine }, to, span)
    }

    /// A value of a refinement type as its own base, one step: a value of
    /// `Small = Port where ...` as a `Port` (whose methods it has).
    pub fn unrefined_once(&mut self, x: Expr) -> Expr {
        let (ty, by_ref) = match self.resolved(&x.ty) {
            Ty::Ref(inner, m) => (self.resolved(&inner), Some(m)),
            t => (t, None),
        };
        let Some((_, _, base)) = self.refinement_of(&ty) else { return x };
        let to = match by_ref {
            Some(m) => Ty::Ref(Box::new(base), m),
            None => base,
        };
        let span = x.span;
        Expr::new(ExprKind::Coerce { expr: Box::new(x), kind: Coercion::Unrefine }, to, span)
    }

    /// The refinements a value of `ty` satisfies, the outermost base's
    /// first: `[Port, Small]` for `Small`, each with its arguments.
    pub fn refinement_chain(&self, ty: &Ty) -> Vec<(DefId, Vec<Ty>)> {
        let mut chain = Vec::new();
        let mut at = self.resolved(ty);
        while let Ty::Adt(d, args) = &at {
            let Some(base) = self.prog.refinement_base(*d, args) else { break };
            chain.push((*d, args.clone()));
            if chain.len() > 64 {
                break;
            }
            at = base;
        }
        chain.reverse();
        chain
    }

    /// Whether `ty` is a refinement type: its definition, arguments and
    /// base.
    pub fn refinement_of(&self, ty: &Ty) -> Option<(DefId, Vec<Ty>, Ty)> {
        match self.resolved(ty) {
            Ty::Adt(d, args) => {
                let base = self.prog.refinement_base(d, &args)?;
                Some((d, args, base))
            }
            _ => None,
        }
    }

    /// A constant written where a refinement is expected (07: "constructing
    /// one checks `pred` (statically when the value is a constant)"): a
    /// `comptime` expression that checks the predicate on the value and
    /// gives it, so the check runs, and fails, at compile time.
    pub fn refine_constant(&mut self, value: Expr, d: DefId, args: Vec<Ty>, ty: Ty) -> Expr {
        let prog = self.prog;
        let span = value.span;
        if prog.refinement(d).and_then(|r| r.predicate).is_none() {
            return value;
        }
        if value.ty == Ty::Error {
            return Expr::new(value.kind, ty, span);
        }
        // Each predicate of the chain, the outermost base's first.
        let mut stmts = Vec::new();
        for (rd, rargs) in self.refinement_chain(&Ty::Adt(d, args.clone())) {
            let Some(r) = prog.refinement(rd) else { continue };
            let Some(pred) = r.predicate else { continue };
            let base = r.base.subst(&rargs);
            let mode = if self.is_copy(&base) { ArgMode::Value } else { ArgMode::Borrow };
            let arg = Expr::new(value.kind.clone(), base, span);
            let call = Expr::new(
                ExprKind::Call {
                    callee: Callee::Fn { def: pred, type_args: rargs },
                    args: vec![Arg { expr: arg, mode }],
                },
                Ty::Bool,
                span,
            );
            stmts.push(Stmt::Contract(ContractCheck {
                kind: ContractKind::Refinement,
                cond: call,
                text: r.text.clone(),
                span,
                replay: false,
            }));
        }
        let refined = Expr::new(value.kind.clone(), ty.clone(), span);
        stmts.push(Stmt::Return { value: Some(refined), span });
        let body = Block { stmts, span };
        let id = ClosureId(self.closures.len() as u32);
        let fn_ty = Ty::Fn(Box::new(sisu_resolve::FnTy { params: Vec::new(), ret: ty.clone(), pure: true }));
        self.closures.push(Some(Closure {
            params: Vec::new(),
            captures: Vec::new(),
            own: false,
            ret: ty.clone(),
            body,
            span,
            fn_ty,
            comptime: true,
            role: ClosureRole::Plain,
        }));
        self.comptime_sites.push((id, span.range));
        Expr::new(ExprKind::Comptime(id), ty, span)
    }

    /// `X.try_from(v)` for a refinement `X`: `X ! RefinementError`.
    pub fn refine_try(
        &mut self,
        ty: &Ty,
        args: &[ast::Arg],
        trailing: Option<&ast::Closure>,
        range: TextRange,
    ) -> Expr {
        let prog = self.prog;
        let span = self.span(range);
        let Some((d, targs, _)) = self.refinement_of(ty) else { return Expr::error(span) };
        let [arg] = args else {
            let name = prog.def(d).name.clone();
            let found = (args.len() + trailing.is_some() as usize).to_string();
            let diag = self
                .error(msg::TYPE_ARGUMENT_COUNT, range)
                .with("name", format!("{name}.try_from"))
                .with("expected", "1")
                .with("found", found);
            self.report(diag);
            self.check_args_loosely(args);
            return Expr::error(span);
        };
        // The value is of the type under every refinement of the chain:
        // `Small.try_from` takes an `int` and checks `Port`'s predicate too.
        let root = prog.unrefine(ty);
        let x = self.check_expr(&arg.value, &root);
        let x = self.check_movable(x);
        let err =
            prog.lookup_in(prog.prelude, "RefinementError").map(|e| Ty::ErrSet(vec![e])).unwrap_or(Ty::Error);
        let rty = prog.wk.result.map(|r| Ty::Adt(r, vec![ty.clone(), err])).unwrap_or(Ty::Error);
        Expr::new(ExprKind::RefineTry { value: Box::new(x), refinement: d, type_args: targs }, rty, span)
    }

    /// An assignment to, or a `mut` borrow of, a place inside a value of a
    /// type with an invariant, outside that type's own methods (07: "the
    /// compiler enforces this"); and a change through a refinement's base.
    pub fn check_protected_place(&mut self, e: &Expr, range: TextRange) {
        let prog = self.prog;
        let mut cur = e;
        loop {
            match &cur.kind {
                ExprKind::Field { base, .. } | ExprKind::Index { base, .. } => {
                    let owner = self.resolved(base.ty.strip_ref());
                    if let Ty::Adt(d, _) = &owner {
                        let own_method = matches!(&self.self_ty, Some(Ty::Adt(o, _)) if o == d);
                        if prog.invariant_check(*d).is_some() && !own_method {
                            let d2 = self
                                .error(msg::CONTRACT_INVARIANT_FIELD, range)
                                .with("what", self.source_text(range))
                                .with("ty", self.show(&owner));
                            self.report(d2);
                            return;
                        }
                    }
                    cur = base;
                }
                ExprKind::Coerce { expr, kind: Coercion::Unrefine } => {
                    let (Some((d, _, _)), inner_ty) = (self.refinement_of(&expr.ty), expr.ty.clone()) else {
                        return;
                    };
                    let pred = prog.refinement(d).map(|r| r.text.clone()).unwrap_or_default();
                    let d2 = self
                        .error(msg::REFINE_CHANGE_THROUGH_BASE, range)
                        .with("what", self.source_text(range))
                        .with("ty", self.show(&inner_ty))
                        .with("predicate", pred);
                    self.report(d2);
                    return;
                }
                ExprKind::Deref(inner) => cur = inner,
                _ => return,
            }
        }
    }
}

impl Checker<'_> {
    /// Whether `base` names std's `arbitrary` or `exhaustive` (17): which.
    pub fn generator_kind(&self, base: &ast::Expr) -> Option<&'static str> {
        let ast::ExprKind::Name(n) = &base.kind else { return None };
        if self.lookup_local(n).is_some() {
            return None;
        }
        let d = self.prog.lookup(self.module, n)?;
        match self.prog.fn_info(d)?.intrinsic.as_deref() {
            Some("test.arbitrary") => Some("arbitrary"),
            Some("test.exhaustive") => Some("exhaustive"),
            _ => None,
        }
    }

    /// `arbitrary[T]` and `exhaustive[T](max_len=n)`: the `Gen` of the
    /// generator the compiler wrote for `T` (`<arbitrary>/main.sisu`), as
    /// `Gen.of(g)` or `Gen.every(g, max_len=n)`. `None` when `base` is
    /// neither.
    pub fn generator_site(
        &mut self,
        whole: &ast::Expr,
        base: &ast::Expr,
        targs: &[ast::IndexArg],
        call: Option<&[ast::Arg]>,
        expected: Option<&Ty>,
    ) -> Option<Expr> {
        let kind = self.generator_kind(base)?;
        let span = self.span(whole.range);
        let [ast::IndexArg::Expr(t)] = targs else {
            let d = self
                .error(msg::PARSE_EXPECTED_TYPE, whole.range)
                .with("found", format!("`{}`", self.source_text(whole.range)));
            self.report(d);
            return Some(Expr::error(span));
        };
        let key: String = self.source_text(t.range).chars().filter(|c| !c.is_whitespace()).collect();
        let name = match self.prog.arbitrary_sites.get(&key) {
            Some(Ok(n)) => n.clone(),
            other => {
                let why = match other {
                    Some(Err(w)) => w.clone(),
                    _ => "the compiler writes generators only for the program's own module".to_string(),
                };
                let shown = self.source_text(t.range);
                let d = self.error(msg::TEST_NO_GENERATOR, t.range).with("ty", shown).with("why", why);
                self.report(d);
                if let Some(args) = call {
                    self.check_args_loosely(args);
                }
                // A `Gen` of nothing known, so what uses its cases is not
                // reported again.
                let prog = self.prog;
                let ty = prog
                    .lookup_in(prog.prelude, "Gen")
                    .map(|g| Ty::Adt(g, vec![Ty::Error]))
                    .unwrap_or(Ty::Error);
                return Some(Expr::new(ExprKind::Error, ty, span));
            }
        };
        let node = whole.node;
        let range = whole.range;
        let mk = |kind: ast::ExprKind| ast::Expr { node, range, kind };
        let nm = |s: &str| ast::Name { text: s.to_string(), range };
        let arg = |value: ast::Expr, name: Option<&str>| ast::Arg {
            node,
            range,
            name: name.map(|n| ast::Name { text: n.to_string(), range }),
            mutable: false,
            value,
        };
        let mut args = vec![arg(mk(ast::ExprKind::Name(name)), None)];
        let method = if kind == "arbitrary" {
            if let Some(extra) = call
                && !extra.is_empty()
            {
                let d = self
                    .error(msg::TYPE_ARGUMENT_COUNT, range)
                    .with("name", "arbitrary")
                    .with("expected", "0")
                    .with("found", extra.len().to_string());
                self.report(d);
            }
            "of"
        } else {
            let max_len = call
                .and_then(|a| a.iter().find(|x| x.name.as_ref().is_none_or(|n| n.text == "max_len")))
                .map(|a| a.value.clone())
                .unwrap_or(mk(ast::ExprKind::Int { text: "3".into(), value: Some(3), suffix: None }));
            args.push(arg(max_len, Some("max_len")));
            "every"
        };
        let callee = mk(ast::ExprKind::Field {
            base: Box::new(mk(ast::ExprKind::Name("Gen".into()))),
            name: nm(method),
        });
        let e = mk(ast::ExprKind::Call { callee: Box::new(callee), args, trailing: None });
        Some(self.infer_expr(&e, expected))
    }
}

/// Whether an AST expression is a constant: a literal (a number, possibly
/// negated, text, a character, a boolean), a `const`, or a list, tuple,
/// set or dict of constants. A constant where a refinement is expected is
/// checked at compile time.
pub fn is_constant(c: &Checker, e: &ast::Expr) -> bool {
    use ast::ExprKind as K;
    match &e.kind {
        K::Int { .. } | K::Float { .. } | K::Char(_) | K::RawStr(_) | K::ByteStr(_) => true,
        K::Str(parts) => parts.iter().all(|p| matches!(p, ast::StrPart::Text(_))),
        K::Name(n) => {
            if n == "True" || n == "False" {
                return true;
            }
            c.lookup_local(n).is_none()
                && c.prog
                    .lookup(c.module, n)
                    .is_some_and(|d| matches!(c.prog.def(d).kind, sisu_resolve::DefKind::Const(_)))
        }
        K::Unary { op: ast::UnOp::Neg, expr } => matches!(expr.kind, K::Int { .. } | K::Float { .. }),
        K::Paren(inner) => is_constant(c, inner),
        K::List(xs) | K::Tuple(xs) | K::Set(xs) => xs.iter().all(|x| is_constant(c, x)),
        K::Dict(kv) => kv.iter().all(|(k, v)| is_constant(c, k) && is_constant(c, v)),
        _ => false,
    }
}
