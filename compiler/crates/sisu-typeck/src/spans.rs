//! Every span of a typed body, for the per-item queries (docs/
//! architecture.md, "The query layer"): a body checked before is reused at
//! its item's new place by moving its spans, and only a body whose spans
//! all lie in its own item (or in the standard library) can be.

use crate::thir::*;
use sisu_diag::Span;

/// Calls `f` on every span of the body.
pub fn body(b: &mut Body, f: &mut dyn FnMut(&mut Span)) {
    f(&mut b.span);
    for l in &mut b.locals {
        f(&mut l.span);
    }
    for c in &mut b.closures {
        f(&mut c.span);
        block(&mut c.body, f);
    }
    if let Some(c) = &mut b.contracts {
        for k in c.requires.iter_mut().chain(c.ensures.iter_mut()) {
            check(k, f);
        }
        for (_, e) in &mut c.olds {
            expr(e, f);
        }
    }
    for g in &mut b.generic_asserts {
        expr(&mut g.cond, f);
        if let Some(m) = &mut g.message {
            expr(m, f);
        }
    }
    block(&mut b.block, f);
}

fn check(k: &mut ContractCheck, f: &mut dyn FnMut(&mut Span)) {
    f(&mut k.span);
    expr(&mut k.cond, f);
}

fn block(b: &mut Block, f: &mut dyn FnMut(&mut Span)) {
    f(&mut b.span);
    for s in &mut b.stmts {
        stmt(s, f);
    }
}

fn cond(c: &mut Cond, f: &mut dyn FnMut(&mut Span)) {
    for p in &mut c.parts {
        match p {
            CondPart::Bool(e) => expr(e, f),
            CondPart::Let { pat: p, value } => {
                pat(p, f);
                expr(value, f);
            }
        }
    }
}

fn stmt(s: &mut Stmt, f: &mut dyn FnMut(&mut Span)) {
    match s {
        Stmt::Let { pat: p, init, else_, span, .. } => {
            f(span);
            pat(p, f);
            if let Some(e) = init {
                expr(e, f);
            }
            match else_ {
                Some(LetElse::Fallback { value, .. }) => expr(value, f),
                Some(LetElse::Diverge { body, .. }) => block(body, f),
                None => {}
            }
        }
        Stmt::Assign { place, value, span } | Stmt::AugAssign { place, value, span, .. } => {
            f(span);
            expr(place, f);
            expr(value, f);
        }
        Stmt::AssignTuple { places, value, span } => {
            f(span);
            for p in places {
                expr(p, f);
            }
            expr(value, f);
        }
        Stmt::Discard { value, span } | Stmt::Yield { value, span } => {
            f(span);
            expr(value, f);
        }
        Stmt::Expr(value) => expr(value, f),
        Stmt::Return { value, span } => {
            f(span);
            if let Some(v) = value {
                expr(v, f);
            }
        }
        Stmt::If { branches, else_, span } => {
            f(span);
            for (c, b) in branches {
                cond(c, f);
                block(b, f);
            }
            if let Some(b) = else_ {
                block(b, f);
            }
        }
        Stmt::While { cond: c, body, span } => {
            f(span);
            cond(c, f);
            block(body, f);
        }
        Stmt::For { pat: p, iter, body, span, .. } => {
            f(span);
            pat(p, f);
            expr(iter, f);
            block(body, f);
        }
        Stmt::Match { scrutinee, arms, span } => {
            f(span);
            expr(scrutinee, f);
            for a in arms {
                arm(a, f);
            }
        }
        Stmt::Assert { cond: c, message, span, .. } => {
            f(span);
            expr(c, f);
            if let Some(m) = message {
                expr(m, f);
            }
        }
        Stmt::Contract(c) => check(c, f),
        Stmt::Defer { body, span } => {
            f(span);
            block(body, f);
        }
        Stmt::Block(body) => block(body, f),
        Stmt::Select { arms, span } => {
            f(span);
            for a in arms {
                f(&mut a.span);
                match &mut a.op {
                    SelectOp::Recv(e)
                    | SelectOp::Send(e)
                    | SelectOp::Join(e)
                    | SelectOp::Timer(e)
                    | SelectOp::Cancelled(e) => expr(e, f),
                }
                pat(&mut a.pat, f);
                block(&mut a.body, f);
            }
        }
        Stmt::Break(span) | Stmt::Continue(span) | Stmt::Error(span) => f(span),
    }
}

fn arm(a: &mut Arm, f: &mut dyn FnMut(&mut Span)) {
    f(&mut a.span);
    pat(&mut a.pat, f);
    if let Some(g) = &mut a.guard {
        expr(g, f);
    }
    match &mut a.body {
        ArmBody::Expr(e) => expr(e, f),
        ArmBody::Block(b) => block(b, f),
    }
}

fn pat(p: &mut Pat, f: &mut dyn FnMut(&mut Span)) {
    f(&mut p.span);
    match &mut p.kind {
        PatKind::Variant { fields, .. } | PatKind::Struct { fields, .. } => {
            for x in fields {
                pat(x, f);
            }
        }
        PatKind::Tuple(ps) | PatKind::Or(ps) => {
            for x in ps {
                pat(x, f);
            }
        }
        PatKind::List { prefix, suffix, .. } => {
            for x in prefix.iter_mut().chain(suffix.iter_mut()) {
                pat(x, f);
            }
        }
        PatKind::Deref(inner) => pat(inner, f),
        PatKind::Wild | PatKind::Bind { .. } | PatKind::Lit(_) | PatKind::Error => {}
    }
}

fn expr(e: &mut Expr, f: &mut dyn FnMut(&mut Span)) {
    f(&mut e.span);
    match &mut e.kind {
        ExprKind::Call { callee, args } => {
            if let Callee::Value(c) = callee {
                expr(c, f);
            }
            for a in args {
                expr(&mut a.expr, f);
            }
        }
        ExprKind::Variant { args, .. }
        | ExprKind::Tuple(args)
        | ExprKind::List(args)
        | ExprKind::Set(args) => {
            for a in args {
                expr(a, f);
            }
        }
        ExprKind::Struct { fields, .. } => {
            for x in fields {
                if let FieldInit::Value(v) = x {
                    expr(v, f);
                }
            }
        }
        ExprKind::Dict(kv) => {
            for (k, v) in kv {
                expr(k, f);
                expr(v, f);
            }
        }
        ExprKind::Field { base, .. }
        | ExprKind::Deref(base)
        | ExprKind::CopyOf(base)
        | ExprKind::Trace(base)
        | ExprKind::Borrow { place: base, .. }
        | ExprKind::Unary { operand: base, .. }
        | ExprKind::Try { expr: base, .. }
        | ExprKind::Coerce { expr: base, .. }
        | ExprKind::RefineTry { value: base, .. } => expr(base, f),
        ExprKind::Index { base, index, .. } => {
            expr(base, f);
            expr(index, f);
        }
        ExprKind::Slice { base, lo, hi } => {
            expr(base, f);
            if let Some(x) = lo {
                expr(x, f);
            }
            if let Some(x) = hi {
                expr(x, f);
            }
        }
        ExprKind::Binary { lhs, rhs, .. } | ExprKind::Logic { lhs, rhs, .. } => {
            expr(lhs, f);
            expr(rhs, f);
        }
        ExprKind::Compare { first, rest } => {
            expr(first, f);
            for (_, x) in rest {
                expr(x, f);
            }
        }
        ExprKind::Coalesce { opt, default, .. } => {
            expr(opt, f);
            expr(default, f);
        }
        ExprKind::If { cond: c, then, else_ } => {
            expr(c, f);
            expr(then, f);
            expr(else_, f);
        }
        ExprKind::Match { scrutinee, arms } => {
            expr(scrutinee, f);
            for a in arms {
                arm(a, f);
            }
        }
        ExprKind::OptChain { base, then, .. } => {
            expr(base, f);
            expr(then, f);
        }
        ExprKind::Interp(parts) => {
            for p in parts {
                if let InterpPart::Value { expr: x, .. } = p {
                    expr(x, f);
                }
            }
        }
        ExprKind::Comprehension { clauses, elem, value, .. } => {
            for c in clauses {
                match c {
                    CompClause::For { pat: p, iter, .. } => {
                        pat(p, f);
                        expr(iter, f);
                    }
                    CompClause::If(x) => expr(x, f),
                }
            }
            expr(elem, f);
            if let Some(v) = value {
                expr(v, f);
            }
        }
        ExprKind::Int(_)
        | ExprKind::WideInt(_)
        | ExprKind::Float(_)
        | ExprKind::Bool(_)
        | ExprKind::Char(_)
        | ExprKind::Str(_)
        | ExprKind::Bytes(_)
        | ExprKind::Unit
        | ExprKind::Local(_)
        | ExprKind::Const(_)
        | ExprKind::FnRef(_)
        | ExprKind::Closure(_)
        | ExprKind::Comptime(_)
        | ExprKind::Generator(_)
        | ExprKind::DefaultArg { .. }
        | ExprKind::FieldDefault { .. }
        | ExprKind::ConstParam(_)
        | ExprKind::Error => {}
    }
}
