//! A walk over every expression of a module, parents before children:
//! what the compiler needs when it writes code for what a module mentions
//! (the generators of `arbitrary[T]`, 17).

use super::*;

/// Calls `f` on every expression in the items, parents first.
pub fn exprs_in_items(items: &[Item], f: &mut dyn FnMut(&Expr)) {
    for it in items {
        item(it, f);
    }
}

fn item(it: &Item, f: &mut dyn FnMut(&Expr)) {
    for a in &it.attrs {
        a.args.iter().for_each(|x| expr(&x.value, f));
    }
    match &it.kind {
        ItemKind::Fn(d) => fn_def(d, f),
        ItemKind::Struct(s) => {
            for fd in &s.fields {
                if let Some(e) = &fd.default {
                    expr(e, f);
                }
            }
            s.methods.iter().for_each(|m| fn_def(m, f));
            s.invariants.iter().for_each(|c| expr(&c.expr, f));
            for c in &s.consts {
                if let Some(e) = &c.value {
                    expr(e, f);
                }
            }
        }
        ItemKind::Enum(e) | ItemKind::ErrorSet(e) => e.methods.iter().for_each(|m| fn_def(m, f)),
        ItemKind::Impl(i) => {
            i.methods.iter().for_each(|m| fn_def(m, f));
            for c in &i.consts {
                if let Some(e) = &c.value {
                    expr(e, f);
                }
            }
        }
        ItemKind::Trait(t) => t.methods.iter().for_each(|m| fn_def(m, f)),
        ItemKind::Test(t) | ItemKind::Bench(t) => {
            if let Some((_, e)) = &t.cases {
                expr(e, f);
            }
            if let Some(b) = &t.body {
                block(b, f);
            }
        }
        ItemKind::Suite(s) => exprs_in_items(&s.items, f),
        ItemKind::Const(c) => {
            if let Some(e) = &c.value {
                expr(e, f);
            }
        }
        ItemKind::TypeAlias(a) => {
            if let Some(e) = &a.predicate {
                expr(e, f);
            }
        }
        ItemKind::Extern(items) => exprs_in_items(items, f),
        ItemKind::Stmt(s) => stmt(s, f),
        ItemKind::Import(_) | ItemKind::FromImport(_) | ItemKind::Error => {}
    }
}

fn fn_def(d: &FnDef, f: &mut dyn FnMut(&Expr)) {
    for p in &d.params {
        if let ParamKind::Named { default: Some(e), .. } = &p.kind {
            expr(e, f);
        }
    }
    if let Some(b) = &d.body {
        block(b, f);
    }
}

pub fn block(b: &Block, f: &mut dyn FnMut(&Expr)) {
    b.stmts.iter().for_each(|s| stmt(s, f));
}

fn cond(c: &Cond, f: &mut dyn FnMut(&Expr)) {
    for p in &c.parts {
        match p {
            CondPart::Expr(e) | CondPart::Let { value: e, .. } => expr(e, f),
        }
    }
}

pub fn stmt(s: &Stmt, f: &mut dyn FnMut(&Expr)) {
    match &s.kind {
        StmtKind::Let { init, else_, .. } => {
            if let Some(e) = init {
                expr(e, f);
            }
            if let Some(le) = else_ {
                match &le.body {
                    ElseBody::Expr(e) => expr(e, f),
                    ElseBody::Diverge(st) => stmt(st, f),
                    ElseBody::Block(b) => block(b, f),
                }
            }
        }
        StmtKind::Assign { target, value, .. } => {
            expr(target, f);
            expr(value, f);
        }
        StmtKind::Expr(e) | StmtKind::Expect(e) => expr(e, f),
        StmtKind::Return(e) | StmtKind::Yield(e) => {
            if let Some(e) = e {
                expr(e, f);
            }
        }
        StmtKind::If { branches, else_block } => {
            for (c, b) in branches {
                cond(c, f);
                block(b, f);
            }
            if let Some(b) = else_block {
                block(b, f);
            }
        }
        StmtKind::While { cond: c, body } => {
            cond(c, f);
            block(body, f);
        }
        StmtKind::For { iter, body, .. } => {
            expr(iter, f);
            block(body, f);
        }
        StmtKind::Assert { cond: c, message } => {
            expr(c, f);
            if let Some(m) = message {
                expr(m, f);
            }
        }
        StmtKind::Defer(b) => block(b, f),
        StmtKind::With { value, body, .. } => {
            expr(value, f);
            block(body, f);
        }
        StmtKind::Item(it) => item(it, f),
        StmtKind::Comptime(inner) => stmt(inner, f),
        StmtKind::ReturnImpl(i) => i.methods.iter().for_each(|m| fn_def(m, f)),
        StmtKind::Select(arms) => {
            for a in arms {
                expr(&a.op, f);
                block(&a.body, f);
            }
        }
        StmtKind::Contract(c) => expr(&c.expr, f),
        StmtKind::Break
        | StmtKind::Continue
        | StmtKind::Pass
        | StmtKind::Unsupported(_)
        | StmtKind::Error => {}
    }
}

pub fn expr(e: &Expr, f: &mut dyn FnMut(&Expr)) {
    f(e);
    match &e.kind {
        ExprKind::Str(parts) => {
            for p in parts {
                if let StrPart::Interp { expr: x, .. } = p {
                    expr(x, f);
                }
            }
        }
        ExprKind::Paren(x)
        | ExprKind::Unary { expr: x, .. }
        | ExprKind::Ref { expr: x, .. }
        | ExprKind::FromEnd(x)
        | ExprKind::Try(x)
        | ExprKind::Comptime(x) => expr(x, f),
        ExprKind::Tuple(xs) | ExprKind::List(xs) | ExprKind::Set(xs) => xs.iter().for_each(|x| expr(x, f)),
        ExprKind::Dict(kv) => {
            for (k, v) in kv {
                expr(k, f);
                expr(v, f);
            }
        }
        ExprKind::Comprehension { elem, value, clauses, .. } => {
            expr(elem, f);
            if let Some(v) = value {
                expr(v, f);
            }
            for c in clauses {
                match c {
                    CompClause::For { iter, .. } => expr(iter, f),
                    CompClause::If(x) => expr(x, f),
                }
            }
        }
        ExprKind::Binary { lhs, rhs, .. } => {
            expr(lhs, f);
            expr(rhs, f);
        }
        ExprKind::Compare { first, rest } => {
            expr(first, f);
            rest.iter().for_each(|(_, x)| expr(x, f));
        }
        ExprKind::Call { callee, args, trailing } => {
            expr(callee, f);
            args.iter().for_each(|a| expr(&a.value, f));
            if let Some(c) = trailing {
                closure(c, f);
            }
        }
        ExprKind::Field { base, .. } | ExprKind::OptField { base, .. } => expr(base, f),
        ExprKind::Index { base, args } => {
            expr(base, f);
            for a in args {
                match a {
                    IndexArg::Expr(x) => expr(x, f),
                    IndexArg::Slice { lo, hi } => {
                        if let Some(x) = lo {
                            expr(x, f);
                        }
                        if let Some(x) = hi {
                            expr(x, f);
                        }
                    }
                }
            }
        }
        ExprKind::Closure(c) => closure(c, f),
        ExprKind::If { branches, else_ } => {
            for (c, v) in branches {
                expr(c, f);
                expr(v, f);
            }
            if let Some(x) = else_ {
                expr(x, f);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            expr(scrutinee, f);
            arms.iter().for_each(|a| arm(a, f));
        }
        ExprKind::FieldOf { base, field } => {
            expr(base, f);
            expr(field, f);
        }
        _ => {}
    }
}

fn arm(a: &Arm, f: &mut dyn FnMut(&Expr)) {
    if let Some(g) = &a.guard {
        expr(g, f);
    }
    match &a.body {
        ArmBody::Expr(x) => expr(x, f),
        ArmBody::Block(b) => block(b, f),
    }
    if let Some(g) = &a.generated {
        match &**g {
            ArmGen::For { iter, arms, .. } => {
                expr(iter, f);
                arms.iter().for_each(|x| arm(x, f));
            }
            ArmGen::If { cond: c, then, else_ } => {
                expr(c, f);
                then.iter().chain(else_.iter()).for_each(|x| arm(x, f));
            }
        }
    }
}

fn closure(c: &Closure, f: &mut dyn FnMut(&Expr)) {
    match &c.body {
        ClosureBody::Expr(x) => expr(x, f),
        ClosureBody::Block(b) => block(b, f),
    }
}
