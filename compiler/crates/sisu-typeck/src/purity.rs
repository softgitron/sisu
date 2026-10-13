//! `pure` (D-074): a pure function takes no capability and calls only pure
//! functions; a closure is pure when its body is. Higher-order functions of
//! the prelude are pure when their function arguments are, which the walk
//! sees because closure bodies are walked where they are written.

use crate::thir::*;
use sisu_resolve::{Program, Ty};

/// The name of the first impure function a block calls, if any.
pub fn first_impure_in_block(prog: &Program, b: &Block, locals: &[Local]) -> Option<String> {
    let mut w = Walker { prog, locals, found: None, closures: None };
    w.block(b);
    w.found
}

/// Every impure call in a body, with its span: (callee name, span).
pub fn impure_calls(prog: &Program, body: &Body) -> Vec<(String, sisu_diag::Span)> {
    let mut w = Collector { prog, out: Vec::new(), closures: &body.closures };
    w.block(&body.block);
    w.out
}

struct Walker<'a> {
    prog: &'a Program,
    #[allow(dead_code)]
    locals: &'a [Local],
    found: Option<String>,
    closures: Option<&'a [Closure]>,
}

impl Walker<'_> {
    fn block(&mut self, b: &Block) {
        for s in &b.stmts {
            visit_stmt(s, &mut |e| self.expr(e));
        }
    }

    fn expr(&mut self, e: &Expr) {
        if self.found.is_some() {
            return;
        }
        if let Some(name) = impure_callee(self.prog, e) {
            self.found = Some(name);
        }
        if let (ExprKind::Closure(c), Some(cs)) = (&e.kind, self.closures)
            && let Some(cl) = cs.get(c.0 as usize)
        {
            self.block(&cl.body);
        }
    }
}

struct Collector<'a> {
    prog: &'a Program,
    out: Vec<(String, sisu_diag::Span)>,
    closures: &'a [Closure],
}

impl Collector<'_> {
    fn block(&mut self, b: &Block) {
        for s in &b.stmts {
            visit_stmt(s, &mut |e| self.expr(e));
        }
    }

    fn expr(&mut self, e: &Expr) {
        if let Some(name) = impure_callee(self.prog, e) {
            self.out.push((name, e.span));
        }
        if let ExprKind::Closure(c) = &e.kind
            && let Some(cl) = self.closures.get(c.0 as usize)
        {
            let body = cl.body.clone();
            self.block(&body);
        }
    }
}

fn impure_callee(prog: &Program, e: &Expr) -> Option<String> {
    match &e.kind {
        ExprKind::Call { callee: Callee::Fn { def, .. }, .. } => match prog.fn_info(*def) {
            Some(f) if !f.pure => Some(prog.def(*def).name.clone()),
            _ => None,
        },
        ExprKind::Call { callee: Callee::Value(f), .. } => match &f.ty {
            Ty::Fn(ft) if !ft.pure => Some("a function value that is not `pure`".into()),
            _ => None,
        },
        _ => None,
    }
}

/// Calls `f` on every expression of a statement, children before parents
/// is not guaranteed; every expression is visited once.
pub fn visit_stmt(s: &Stmt, f: &mut dyn FnMut(&Expr)) {
    match s {
        Stmt::Let { init, else_, .. } => {
            if let Some(e) = init {
                visit_expr(e, f);
            }
            match else_ {
                Some(LetElse::Fallback { value, .. }) => visit_expr(value, f),
                Some(LetElse::Diverge { body, .. }) => visit_block(body, f),
                None => {}
            }
        }
        Stmt::Assign { place, value, .. } | Stmt::AugAssign { place, value, .. } => {
            visit_expr(place, f);
            visit_expr(value, f);
        }
        Stmt::AssignTuple { places, value, .. } => {
            for p in places {
                visit_expr(p, f);
            }
            visit_expr(value, f);
        }
        Stmt::Discard { value, .. } | Stmt::Expr(value) => visit_expr(value, f),
        Stmt::Return { value, .. } => {
            if let Some(v) = value {
                visit_expr(v, f);
            }
        }
        Stmt::If { branches, else_, .. } => {
            for (c, b) in branches {
                visit_cond(c, f);
                visit_block(b, f);
            }
            if let Some(b) = else_ {
                visit_block(b, f);
            }
        }
        Stmt::While { cond, body, .. } => {
            visit_cond(cond, f);
            visit_block(body, f);
        }
        Stmt::For { iter, body, .. } => {
            visit_expr(iter, f);
            visit_block(body, f);
        }
        Stmt::Match { scrutinee, arms, .. } => {
            visit_expr(scrutinee, f);
            for a in arms {
                if let Some(g) = &a.guard {
                    visit_expr(g, f);
                }
                match &a.body {
                    ArmBody::Expr(e) => visit_expr(e, f),
                    ArmBody::Block(b) => visit_block(b, f),
                }
            }
        }
        Stmt::Assert { cond, message, .. } => {
            visit_expr(cond, f);
            if let Some(m) = message {
                visit_expr(m, f);
            }
        }
        Stmt::Defer { body, .. } | Stmt::Block(body) => visit_block(body, f),
        Stmt::Break(_) | Stmt::Continue(_) | Stmt::Error(_) => {}
    }
}

pub fn visit_block(b: &Block, f: &mut dyn FnMut(&Expr)) {
    for s in &b.stmts {
        visit_stmt(s, f);
    }
}

fn visit_cond(c: &Cond, f: &mut dyn FnMut(&Expr)) {
    for p in &c.parts {
        match p {
            CondPart::Bool(e) | CondPart::Let { value: e, .. } => visit_expr(e, f),
        }
    }
}

pub fn visit_expr(e: &Expr, f: &mut dyn FnMut(&Expr)) {
    f(e);
    match &e.kind {
        ExprKind::Call { callee, args } => {
            if let Callee::Value(c) = callee {
                visit_expr(c, f);
            }
            for a in args {
                visit_expr(&a.expr, f);
            }
        }
        ExprKind::Variant { args, .. }
        | ExprKind::Tuple(args)
        | ExprKind::List(args)
        | ExprKind::Set(args) => {
            for a in args {
                visit_expr(a, f);
            }
        }
        ExprKind::Struct { fields, .. } => {
            for x in fields {
                if let FieldInit::Value(v) = x {
                    visit_expr(v, f);
                }
            }
        }
        ExprKind::Dict(kv) => {
            for (k, v) in kv {
                visit_expr(k, f);
                visit_expr(v, f);
            }
        }
        ExprKind::Field { base, .. }
        | ExprKind::Deref(base)
        | ExprKind::CopyOf(base)
        | ExprKind::Trace(base) => visit_expr(base, f),
        ExprKind::Index { base, index, .. } => {
            visit_expr(base, f);
            visit_expr(index, f);
        }
        ExprKind::Slice { base, lo, hi } => {
            visit_expr(base, f);
            if let Some(x) = lo {
                visit_expr(x, f);
            }
            if let Some(x) = hi {
                visit_expr(x, f);
            }
        }
        ExprKind::Borrow { place, .. } => visit_expr(place, f),
        ExprKind::Unary { operand, .. } => visit_expr(operand, f),
        ExprKind::Binary { lhs, rhs, .. } | ExprKind::Logic { lhs, rhs, .. } => {
            visit_expr(lhs, f);
            visit_expr(rhs, f);
        }
        ExprKind::Compare { first, rest } => {
            visit_expr(first, f);
            for (_, x) in rest {
                visit_expr(x, f);
            }
        }
        ExprKind::Coalesce { opt, default, .. } => {
            visit_expr(opt, f);
            visit_expr(default, f);
        }
        ExprKind::If { cond, then, else_ } => {
            visit_expr(cond, f);
            visit_expr(then, f);
            visit_expr(else_, f);
        }
        ExprKind::Match { scrutinee, arms } => {
            visit_expr(scrutinee, f);
            for a in arms {
                if let Some(g) = &a.guard {
                    visit_expr(g, f);
                }
                match &a.body {
                    ArmBody::Expr(x) => visit_expr(x, f),
                    ArmBody::Block(b) => visit_block(b, f),
                }
            }
        }
        ExprKind::Try { expr, .. } | ExprKind::Coerce { expr, .. } => visit_expr(expr, f),
        ExprKind::OptChain { base, then, .. } => {
            visit_expr(base, f);
            visit_expr(then, f);
        }
        ExprKind::Interp(parts) => {
            for p in parts {
                if let InterpPart::Value { expr, .. } = p {
                    visit_expr(expr, f);
                }
            }
        }
        ExprKind::Comprehension { clauses, elem, value, .. } => {
            for c in clauses {
                match c {
                    CompClause::For { iter, .. } => visit_expr(iter, f),
                    CompClause::If(x) => visit_expr(x, f),
                }
            }
            visit_expr(elem, f);
            if let Some(v) = value {
                visit_expr(v, f);
            }
        }
        _ => {}
    }
}
