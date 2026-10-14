//! Contracts (07) in SIR: a `requires` clause is checked on entry, the
//! values `old(e)` names are saved then, and on every exit (`return`,
//! `?`, the end of the body) the `ensures` clauses are checked with
//! `result` bound to the returned value, as is the invariant of a `mut
//! self` method's type; a struct with an invariant checks it once built.
//! The level of the build decides which run (`ContractLevel`).
//!
//! A failed check panics through `contract.fail` with the clause and, for
//! a clause that only reads, the value of each of its parts, which the
//! failure path evaluates again (17, "Assertions": every sub-expression's
//! value); a failed `assert` does the same through `assert.fail_parts`.

use crate::func::{DropEntry, FnLower};
use sisu_resolve::Ty;
use sisu_sir as sir;
use sisu_sir::{Callee, Const, Operand, Place, Proj, Rvalue, StatementKind, Terminator};
use sisu_typeck::thir::{self, ContractCheck, ContractKind, ExprKind, InterpPart};

/// Which contracts a build checks (07: `contracts = "off" |
/// "preconditions" | "all"`; debug and test builds check all). The
/// invariant `Mutex` re-checks when unwinding drops a guard ignores it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContractLevel {
    Off,
    Preconditions,
    #[default]
    All,
}

impl ContractLevel {
    pub fn parse(s: &str) -> Option<ContractLevel> {
        match s {
            "off" => Some(ContractLevel::Off),
            "preconditions" => Some(ContractLevel::Preconditions),
            "all" => Some(ContractLevel::All),
            _ => None,
        }
    }
}

/// The most parts of a condition a failure shows.
const MAX_PARTS: usize = 12;

impl FnLower<'_, '_> {
    /// The function's name in a contract's message: `withdraw` or, for a
    /// method, `Account.withdraw`.
    fn contract_owner(&self) -> String {
        let prog = self.ctx.prog;
        let d = self.body.def;
        let name = prog.def(d).name.clone();
        match prog.fn_info(d).and_then(|f| f.owner.as_ref()) {
            Some(o) => format!("{}.{name}", prog.display(o)),
            None => name,
        }
    }

    /// On entry: the `requires` clauses, then the values `old(e)` saves.
    pub fn contract_entry(&mut self) {
        let body = self.body;
        let Some(c) = &body.contracts else { return };
        if self.closure.is_some() || self.ctx.level == ContractLevel::Off {
            return;
        }
        for r in &c.requires {
            self.contract_check(r);
        }
        if self.ctx.level < ContractLevel::All {
            return;
        }
        for (l, e) in &c.olds {
            let sl = self.local_for(*l);
            self.into(Place::local(sl), e);
            let ty = body.locals[l.0 as usize].ty.clone();
            if self.needs_drop(&ty) {
                self.register(DropEntry::Local(sl));
                self.split();
            }
        }
    }

    /// On an exit, once the result is in `_0`: the invariant of a `mut
    /// self` method's type, then the `ensures` clauses.
    pub fn contract_exit(&mut self, loc: Option<sir::Loc>) {
        let body = self.body;
        let Some(c) = &body.contracts else { return };
        if self.closure.is_some() || self.ctx.level < ContractLevel::All || self.terminated() {
            return;
        }
        if let Some((check, args)) = &c.invariant
            && let Some(p) = body.params.first()
        {
            let me = self.local_for(*p);
            let target = Place::local(me).project(Proj::Deref);
            self.call_invariant(*check, args, target, loc);
        }
        if c.ensures.is_empty() {
            return;
        }
        let ret = Place::local(sir::LocalId(0));
        if let Some(r) = c.result {
            let rl = self.local_for(r);
            let rv = if matches!(body.locals[r.0 as usize].ty, Ty::Ref(..)) {
                Rvalue::Ref(ret.clone(), false)
            } else {
                Rvalue::Use(Operand::Copy(ret.clone()))
            };
            self.assign(Place::local(rl), rv, loc);
        }
        for (l, i) in &c.labels {
            let ll = self.local_for(*l);
            let part = ret.project(Proj::Field(*i));
            let rv = if matches!(body.locals[l.0 as usize].ty, Ty::Ref(..)) {
                Rvalue::Ref(part, false)
            } else {
                Rvalue::Use(Operand::Copy(part))
            };
            self.assign(Place::local(ll), rv, loc);
        }
        for e in &c.ensures {
            self.contract_check(e);
        }
    }

    /// The invariant check of a struct, on the value at `target`.
    fn call_invariant(&mut self, check: sisu_resolve::DefId, args: &[Ty], target: Place, loc: Option<sir::Loc>) {
        let prog = self.ctx.prog;
        let Some(f) = prog.fn_info(check) else { return };
        let owner = f.owner.clone().unwrap_or(Ty::Error).subst(args);
        let env = prog.elaborate_env(&f.predicates);
        let local = sisu_typeck::param_local_ty(prog, &owner, sisu_syntax::ast::Mode::Borrow, &env);
        let arg = if matches!(local, Ty::Ref(..)) {
            let t = self.temp_sir(sir::Type::Ref(Box::new(self.ctx.ty(&owner)), false));
            self.assign(Place::local(t), Rvalue::Ref(target, false), loc);
            Operand::Copy(Place::local(t))
        } else {
            Operand::Copy(target)
        };
        let tys = args.iter().map(|t| self.ctx.ty(t)).collect();
        let u = self.unit();
        self.emit(StatementKind::Call { dest: u, callee: Callee::Fn(self.ctx.fn_name(check), tys), args: vec![arg] }, loc);
    }

    /// After a struct is built at `dest`: its invariant, if it has one.
    pub fn invariant_after_construction(
        &mut self,
        dest: &Place,
        def: sisu_resolve::DefId,
        args: &[Ty],
        loc: Option<sir::Loc>,
    ) {
        if self.ctx.level < ContractLevel::All {
            return;
        }
        let Some(check) = self.ctx.prog.invariant_check(def) else { return };
        self.call_invariant(check, args, dest.clone(), loc);
    }

    /// A contract statement in a body: an `invariant` line, or a
    /// refinement's predicate checked at compile time.
    pub fn contract_stmt(&mut self, c: &ContractCheck) {
        if c.kind == ContractKind::Invariant && self.bool_invariant {
            // `T.__invariant`: `False` instead of a panic (D-077).
            let v = self.operand(&c.cond);
            let ok = self.new_block();
            let bad = self.new_block();
            let loc = self.ctx.loc(c.span);
            self.terminate(Terminator::Switch { value: v, cases: vec![(0, bad)], default: ok }, loc);
            self.switch_to(bad);
            self.assign(Place::local(sir::LocalId(0)), Rvalue::Use(Operand::Const(Const::Bool(false))), loc);
            self.emit_exit(0, loc);
            self.terminate(Terminator::Return, loc);
            self.switch_to(ok);
            return;
        }
        self.contract_check(c);
    }

    /// Checks one clause: each conjunct of a top-level `and` in turn, so
    /// a failure names the one that failed.
    pub fn contract_check(&mut self, c: &ContractCheck) {
        let kind = match c.kind {
            ContractKind::Requires => 0,
            ContractKind::Ensures => 1,
            ContractKind::Invariant => 2,
            ContractKind::Refinement => 3,
        };
        let owner = match c.kind {
            ContractKind::Invariant => match self.ctx.prog.fn_info(self.body.def).map(|f| &f.kind) {
                Some(sisu_resolve::FnKind::Invariant(t)) => self.ctx.prog.qualified_name(*t),
                _ => self.contract_owner(),
            },
            ContractKind::Refinement => match &c.cond.kind {
                ExprKind::Call { callee: thir::Callee::Fn { def, type_args }, .. } => {
                    match self.ctx.prog.fn_info(*def).map(|f| &f.kind) {
                        Some(sisu_resolve::FnKind::Predicate(r)) => {
                            let name = self.ctx.prog.qualified_name(*r);
                            if type_args.is_empty() {
                                name
                            } else {
                                let shown: Vec<String> =
                                    type_args.iter().map(|t| self.ctx.prog.display(t)).collect();
                                format!("{name}[{}]", shown.join(", "))
                            }
                        }
                        _ => String::new(),
                    }
                }
                _ => String::new(),
            },
            _ => self.contract_owner(),
        };
        let parts: Vec<&thir::Expr> =
            if c.kind == ContractKind::Refinement { vec![&c.cond] } else { conjuncts(&c.cond) };
        let whole = parts.len() == 1;
        for part in parts {
            let loc = self.ctx.loc(part.span);
            let v = self.operand(part);
            let ok = self.new_block();
            let bad = self.new_block();
            self.terminate(Terminator::Switch { value: v, cases: vec![(0, bad)], default: ok }, loc);
            self.switch_to(bad);
            self.push_scope();
            let text = if whole { c.text.clone() } else { self.source_of(part.span) };
            let shown: Vec<(u32, &thir::Expr)> = match c.kind {
                // The value that is not one.
                ContractKind::Refinement => match &part.kind {
                    ExprKind::Call { args, .. } => args.first().map(|a| (0, &a.expr)).into_iter().collect(),
                    _ => Vec::new(),
                },
                _ if c.replay => self.parts_to_show(part),
                _ => Vec::new(),
            };
            let list = self.rendered_parts(&shown, loc);
            let u = self.temp_sir(sir::Type::Never);
            self.emit(
                StatementKind::Call {
                    dest: Place::local(u),
                    callee: Callee::Intrinsic("contract.fail".into(), Vec::new()),
                    args: vec![
                        Operand::Const(Const::Int(kind)),
                        Operand::Const(Const::Str(owner.clone())),
                        Operand::Const(Const::Str(text)),
                        list,
                    ],
                },
                loc,
            );
            self.terminate(Terminator::Unreachable, loc);
            self.pop_scope();
            self.switch_to(ok);
        }
    }

    /// Whether a failed `assert` of `cond` can show its parts: it fits on
    /// one line and has a part to show.
    pub fn can_show_parts(&self, cond: &thir::Expr) -> bool {
        !self.source_of(cond.span).contains('\n') && !self.parts_to_show(cond).is_empty()
    }

    /// `assert cond` whose failure shows every part of `cond`.
    pub fn assert_showing_parts(&mut self, cond: &thir::Expr, message: Option<&thir::Expr>, span: sisu_diag::Span) {
        let loc = self.ctx.loc(span);
        let v = self.operand(cond);
        let ok = self.new_block();
        let bad = self.new_block();
        self.terminate(Terminator::Switch { value: v, cases: vec![(0, bad)], default: ok }, loc);
        self.switch_to(bad);
        self.push_scope();
        let m = match message {
            Some(m) => self.str_operand(m),
            None => Operand::Const(Const::Str(String::new())),
        };
        let shown = self.parts_to_show(cond);
        let list = self.rendered_parts(&shown, loc);
        let text = self.source_of(cond.span);
        let u = self.temp_sir(sir::Type::Never);
        self.emit(
            StatementKind::Call {
                dest: Place::local(u),
                callee: Callee::Intrinsic("assert.fail_parts".into(), Vec::new()),
                args: vec![Operand::Const(Const::Str(text)), m, list],
            },
            loc,
        );
        self.terminate(Terminator::Unreachable, loc);
        self.pop_scope();
        self.switch_to(ok);
    }

    /// The source text of a span, trimmed.
    fn source_of(&self, span: sisu_diag::Span) -> String {
        self.ctx.sources.slice(span).trim().to_string()
    }

    /// The parts of a condition to show when it fails, each with the
    /// column (in characters, from the condition's start) its value is
    /// drawn under: names, calls (under a method's name), fields,
    /// indexing, arithmetic (under the operator) and variants with
    /// arguments, outermost first, one per column. Only the parts every
    /// evaluation of the condition evaluates are shown: nothing on the
    /// right of `and`, `or` or `??`, nor inside `if`, `match` or a closure.
    fn parts_to_show<'e>(&self, cond: &'e thir::Expr) -> Vec<(u32, &'e thir::Expr)> {
        let Some(file) = self.ctx.sources.get(cond.span.file) else { return Vec::new() };
        let text = file.text.as_str();
        let start = cond.span.range.start as usize;
        let mut out: Vec<(u32, &thir::Expr)> = Vec::new();
        let col = |at: usize| -> Option<u32> {
            let s = text.get(start..at)?;
            if s.contains('\n') {
                return None;
            }
            Some(s.chars().count() as u32)
        };
        fn walk<'e>(
            me: &FnLower,
            e: &'e thir::Expr,
            top: bool,
            text: &str,
            col: &dyn Fn(usize) -> Option<u32>,
            out: &mut Vec<(u32, &'e thir::Expr)>,
        ) {
            if out.len() >= MAX_PARTS {
                return;
            }
            let mut push = |at: usize, x: &'e thir::Expr, out: &mut Vec<(u32, &'e thir::Expr)>| {
                if top || matches!(x.ty, Ty::Fn(_) | Ty::Never | Ty::Unit | Ty::Error) {
                    return;
                }
                if let Some(c) = col(at)
                    && !out.iter().any(|(k, _)| *k == c)
                {
                    out.push((c, x));
                }
            };
            let start = e.span.range.start as usize;
            match &e.kind {
                ExprKind::Local(l) => {
                    let name = &me.body.locals[l.0 as usize].name;
                    if !name.is_empty() && name != "?" && name != "self" {
                        push(start, e, out);
                    }
                }
                ExprKind::Call { callee, args } => {
                    let method = matches!(callee, thir::Callee::Fn { def, .. }
                        if me.ctx.prog.fn_info(*def).is_some_and(|f| f.self_mode.is_some()))
                        && args.first().is_some_and(|a| a.expr.span.range.start == e.span.range.start);
                    let at = if method {
                        let end = args[0].expr.span.range.end as usize;
                        text.get(end..).and_then(|rest| rest.find('.')).map(|i| end + i + 1).unwrap_or(start)
                    } else {
                        start
                    };
                    push(at, e, out);
                    if let thir::Callee::Value(f) = callee {
                        walk(me, f, false, text, col, out);
                    }
                    for a in args {
                        walk(me, &a.expr, false, text, col, out);
                    }
                }
                ExprKind::Field { base, .. } => {
                    push(start, e, out);
                    walk(me, base, false, text, col, out);
                }
                ExprKind::Index { base, index, .. } => {
                    push(start, e, out);
                    walk(me, base, false, text, col, out);
                    walk(me, index, false, text, col, out);
                }
                ExprKind::Variant { args, .. } if !args.is_empty() => {
                    push(start, e, out);
                    for a in args {
                        walk(me, a, false, text, col, out);
                    }
                }
                ExprKind::Binary { lhs, rhs, .. } => {
                    let end = lhs.span.range.end as usize;
                    let at = text
                        .get(end..)
                        .and_then(|rest| rest.find(|c: char| !c.is_whitespace() && c != ')'))
                        .map(|i| end + i)
                        .unwrap_or(start);
                    push(at, e, out);
                    walk(me, lhs, false, text, col, out);
                    walk(me, rhs, false, text, col, out);
                }
                ExprKind::Unary { op, operand } => {
                    if *op != sisu_syntax::ast::UnOp::Not {
                        push(start, e, out);
                    }
                    walk(me, operand, false, text, col, out);
                }
                ExprKind::Compare { first, rest } => {
                    walk(me, first, false, text, col, out);
                    for (_, x) in rest {
                        walk(me, x, false, text, col, out);
                    }
                }
                ExprKind::Logic { lhs, .. } => walk(me, lhs, false, text, col, out),
                ExprKind::Coalesce { opt, .. } => walk(me, opt, false, text, col, out),
                ExprKind::Coerce { expr, .. } | ExprKind::Deref(expr) | ExprKind::Borrow { place: expr, .. } => {
                    walk(me, expr, top, text, col, out)
                }
                ExprKind::CopyOf(inner) => {
                    push(start, e, out);
                    walk(me, inner, false, text, col, out);
                }
                ExprKind::Tuple(xs) | ExprKind::List(xs) => {
                    for x in xs {
                        walk(me, x, false, text, col, out);
                    }
                }
                _ => {}
            }
        }
        walk(self, cond, true, text, &col, &mut out);
        out
    }

    /// The parts' values rendered with `Debug`, as a `list[(int,
    /// String)]` of columns and texts.
    fn rendered_parts(&mut self, parts: &[(u32, &thir::Expr)], loc: Option<sir::Loc>) -> Operand {
        let prog = self.ctx.prog;
        let string_ty = prog.wk.string.map(|s| Ty::Adt(s, Vec::new())).unwrap_or(Ty::Error);
        let pair_ty = Ty::tuple(vec![Ty::Int, string_ty.clone()]);
        let mut items = Vec::new();
        for (c, e) in parts {
            let shown = thir::Expr::new(
                ExprKind::Interp(vec![InterpPart::Value { expr: (*e).clone(), debug: None, spec: "?".into() }]),
                string_ty.clone(),
                e.span,
            );
            let t = self.temp(&string_ty);
            self.into(Place::local(t), &shown);
            self.register(DropEntry::Local(t));
            self.split();
            let pair = self.temp(&pair_ty);
            self.assign(
                Place::local(pair),
                Rvalue::Aggregate(
                    sir::Aggregate::Tuple,
                    vec![Operand::Const(Const::Int(i64::from(*c))), Operand::Move(Place::local(t))],
                ),
                loc,
            );
            self.register(DropEntry::Local(pair));
            self.split();
            items.push(Operand::Move(Place::local(pair)));
        }
        let list_ty = prog.wk.list.map(|l| Ty::Adt(l, vec![pair_ty])).unwrap_or(Ty::Error);
        let list = self.temp(&list_ty);
        self.assign(Place::local(list), Rvalue::Aggregate(sir::Aggregate::List, items), loc);
        self.register(DropEntry::Local(list));
        self.split();
        Operand::Move(Place::local(list))
    }

    /// `X.try_from(v)`: `Ok(v)` when the predicate holds for `v`, else
    /// `Err(Unsatisfied(ty=..., predicate=...))`.
    pub fn refine_try(
        &mut self,
        dest: Place,
        value: &thir::Expr,
        refinement: sisu_resolve::DefId,
        type_args: &[Ty],
        loc: Option<sir::Loc>,
    ) {
        let prog = self.ctx.prog;
        let Some(r) = prog.refinement(refinement) else { return };
        let Some(pred) = r.predicate else { return };
        let base = r.base.subst(type_args);
        let t = self.temp(&value.ty);
        self.into(Place::local(t), value);
        if self.needs_drop(&value.ty) {
            self.register(DropEntry::Local(t));
            self.split();
        }
        let arg = if self.needs_drop(&base) || !prog.is_copy(&base) {
            let r = self.temp_sir(sir::Type::Ref(Box::new(self.ctx.ty(&base)), false));
            self.assign(Place::local(r), Rvalue::Ref(Place::local(t), false), loc);
            Operand::Copy(Place::local(r))
        } else {
            Operand::Copy(Place::local(t))
        };
        let ok = self.temp_sir(sir::Type::Bool);
        let tys = type_args.iter().map(|x| self.ctx.ty(x)).collect();
        self.emit(
            StatementKind::Call { dest: Place::local(ok), callee: Callee::Fn(self.ctx.fn_name(pred), tys), args: vec![arg] },
            loc,
        );
        let yes = self.new_block();
        let no = self.new_block();
        let join = self.new_block();
        self.terminate(
            Terminator::Switch { value: Operand::Copy(Place::local(ok)), cases: vec![(0, no)], default: yes },
            loc,
        );
        self.switch_to(yes);
        let v = self.copy_or_move(Place::local(t), &value.ty);
        self.assign(dest.clone(), Rvalue::Aggregate(sir::Aggregate::Variant("Result".into(), "Ok".into()), vec![v]), loc);
        self.goto(join);
        self.switch_to(no);
        let shown: Vec<String> = type_args.iter().map(|x| prog.display(x)).collect();
        let name = if shown.is_empty() {
            prog.qualified_name(refinement)
        } else {
            format!("{}[{}]", prog.qualified_name(refinement), shown.join(", "))
        };
        let string_ty = prog.wk.string.map(|s| Ty::Adt(s, Vec::new())).unwrap_or(Ty::Error);
        let a = self.temp(&string_ty);
        self.intrinsic_call(Place::local(a), "String.new", Vec::new(), vec![Operand::Const(Const::Str(name))], loc);
        self.register(DropEntry::Local(a));
        self.split();
        let b = self.temp(&string_ty);
        self.intrinsic_call(Place::local(b), "String.new", Vec::new(), vec![Operand::Const(Const::Str(r.text.clone()))], loc);
        self.register(DropEntry::Local(b));
        self.split();
        let err = prog.lookup_in(prog.prelude, "RefinementError").map(|d| Ty::ErrSet(vec![d])).unwrap_or(Ty::Error);
        let e = self.temp(&err);
        self.assign(
            Place::local(e),
            Rvalue::Aggregate(
                sir::Aggregate::Variant("RefinementError".into(), "Unsatisfied".into()),
                vec![Operand::Move(Place::local(a)), Operand::Move(Place::local(b))],
            ),
            loc,
        );
        self.register(DropEntry::Local(e));
        self.split();
        self.assign(
            dest,
            Rvalue::Aggregate(sir::Aggregate::Variant("Result".into(), "Err".into()), vec![Operand::Move(Place::local(e))]),
            loc,
        );
        self.goto(join);
        self.switch_to(join);
    }
}

/// The conjuncts of a top-level `and`: `a and b and c` is checked as `a`,
/// then `b`, then `c`, which evaluates the same parts in the same order.
fn conjuncts(e: &thir::Expr) -> Vec<&thir::Expr> {
    match &e.kind {
        ExprKind::Logic { and: true, lhs, rhs } => {
            let mut v = conjuncts(lhs);
            v.extend(conjuncts(rhs));
            v
        }
        _ => vec![e],
    }
}
