//! The final pass over a body: every type is resolved, and a variable left
//! unbound becomes `()` (or the empty error set in error-set position), so
//! lowering never sees an inference variable.

use crate::infer::InferTable;
use crate::thir::*;
use sisu_resolve::{FnTy, Program, Ty};

pub fn finalize(prog: &Program, infer: &InferTable, t: &Ty) -> Ty {
    fn go(prog: &Program, t: Ty) -> Ty {
        match t {
            Ty::Var(_) => Ty::Unit,
            Ty::Adt(d, args) if Some(d) == prog.wk.result && args.len() == 2 => {
                let ok = go(prog, args[0].clone());
                let err = match &args[1] {
                    Ty::Var(_) => Ty::ErrSet(Vec::new()),
                    e => go(prog, e.clone()),
                };
                Ty::Adt(d, vec![ok, err])
            }
            Ty::Adt(d, args) => Ty::Adt(d, args.into_iter().map(|a| go(prog, a)).collect()),
            Ty::Tuple(e, l) => Ty::Tuple(e.into_iter().map(|a| go(prog, a)).collect(), l),
            Ty::Fn(f) => Ty::Fn(Box::new(FnTy {
                params: f.params.into_iter().map(|a| go(prog, a)).collect(),
                ret: go(prog, f.ret),
                pure: f.pure,
            })),
            Ty::Ref(t, m) => Ty::Ref(Box::new(go(prog, *t)), m),
            other => other,
        }
    }
    go(prog, infer.zonk(t))
}

pub struct Zonker<'a> {
    pub prog: &'a Program,
    pub infer: &'a InferTable,
}

impl Zonker<'_> {
    fn ty(&self, t: &mut Ty) {
        *t = finalize(self.prog, self.infer, t);
    }

    pub fn body(&self, b: &mut Body) {
        for l in &mut b.locals {
            self.ty(&mut l.ty);
        }
        for c in &mut b.closures {
            self.ty(&mut c.ret);
            self.ty(&mut c.fn_ty);
            self.block(&mut c.body);
        }
        self.ty(&mut b.ret);
        if let Some(g) = &mut b.generator {
            self.ty(g);
        }
        if let Some(c) = &mut b.contracts {
            for k in c.requires.iter_mut().chain(c.ensures.iter_mut()) {
                self.expr(&mut k.cond);
            }
            for (_, e) in &mut c.olds {
                self.expr(e);
            }
            if let Some((_, args)) = &mut c.invariant {
                for t in args {
                    self.ty(t);
                }
            }
        }
        self.block(&mut b.block);
    }

    fn block(&self, b: &mut Block) {
        for s in &mut b.stmts {
            self.stmt(s);
        }
    }

    fn cond(&self, c: &mut Cond) {
        for p in &mut c.parts {
            match p {
                CondPart::Bool(e) => self.expr(e),
                CondPart::Let { pat, value } => {
                    self.pat(pat);
                    self.expr(value);
                }
            }
        }
    }

    fn stmt(&self, s: &mut Stmt) {
        match s {
            Stmt::Let { pat, init, else_, .. } => {
                self.pat(pat);
                if let Some(e) = init {
                    self.expr(e);
                }
                match else_ {
                    Some(LetElse::Fallback { value, .. }) => self.expr(value),
                    Some(LetElse::Diverge { body, .. }) => self.block(body),
                    None => {}
                }
            }
            Stmt::Assign { place, value, .. } | Stmt::AugAssign { place, value, .. } => {
                self.expr(place);
                self.expr(value);
            }
            Stmt::AssignTuple { places, value, .. } => {
                for p in places {
                    self.expr(p);
                }
                self.expr(value);
            }
            Stmt::Discard { value, .. } | Stmt::Expr(value) => self.expr(value),
            Stmt::Return { value, .. } => {
                if let Some(v) = value {
                    self.expr(v);
                }
            }
            Stmt::If { branches, else_, .. } => {
                for (c, b) in branches {
                    self.cond(c);
                    self.block(b);
                }
                if let Some(b) = else_ {
                    self.block(b);
                }
            }
            Stmt::While { cond, body, .. } => {
                self.cond(cond);
                self.block(body);
            }
            Stmt::For { pat, iter, item, body, .. } => {
                self.pat(pat);
                self.expr(iter);
                self.ty(item);
                self.block(body);
            }
            Stmt::Match { scrutinee, arms, .. } => {
                self.expr(scrutinee);
                for a in arms {
                    self.arm(a);
                }
            }
            Stmt::Assert { cond, message, .. } => {
                self.expr(cond);
                if let Some(m) = message {
                    self.expr(m);
                }
            }
            Stmt::Contract(c) => self.expr(&mut c.cond),
            Stmt::Defer { body, .. } | Stmt::Block(body) => self.block(body),
            Stmt::Select { arms, .. } => {
                for a in arms {
                    match &mut a.op {
                        SelectOp::Recv(e)
                        | SelectOp::Send(e)
                        | SelectOp::Join(e)
                        | SelectOp::Timer(e)
                        | SelectOp::Cancelled(e) => self.expr(e),
                    }
                    self.pat(&mut a.pat);
                    self.block(&mut a.body);
                }
            }
            Stmt::Yield { value, .. } => self.expr(value),
            Stmt::Break(_) | Stmt::Continue(_) | Stmt::Error(_) => {}
        }
    }

    fn arm(&self, a: &mut Arm) {
        self.pat(&mut a.pat);
        if let Some(g) = &mut a.guard {
            self.expr(g);
        }
        match &mut a.body {
            ArmBody::Expr(e) => self.expr(e),
            ArmBody::Block(b) => self.block(b),
        }
    }

    fn pat(&self, p: &mut Pat) {
        self.ty(&mut p.ty);
        match &mut p.kind {
            PatKind::Variant { fields, .. } | PatKind::Struct { fields, .. } => {
                for f in fields {
                    self.pat(f);
                }
            }
            PatKind::Tuple(ps) | PatKind::Or(ps) => {
                for x in ps {
                    self.pat(x);
                }
            }
            PatKind::List { prefix, suffix, .. } => {
                for x in prefix.iter_mut().chain(suffix.iter_mut()) {
                    self.pat(x);
                }
            }
            PatKind::Deref(inner) => self.pat(inner),
            _ => {}
        }
    }

    fn expr(&self, e: &mut Expr) {
        self.ty(&mut e.ty);
        match &mut e.kind {
            ExprKind::Call { callee, args } => {
                match callee {
                    Callee::Fn { type_args, .. } => {
                        for t in type_args {
                            self.ty(t);
                        }
                    }
                    Callee::Value(f) => self.expr(f),
                }
                for a in args {
                    self.expr(&mut a.expr);
                }
            }
            ExprKind::Variant { args, .. }
            | ExprKind::Tuple(args)
            | ExprKind::List(args)
            | ExprKind::Set(args) => {
                for a in args {
                    self.expr(a);
                }
            }
            ExprKind::Struct { fields, .. } => {
                for f in fields {
                    if let FieldInit::Value(v) = f {
                        self.expr(v);
                    }
                }
            }
            ExprKind::Dict(kv) => {
                for (k, v) in kv {
                    self.expr(k);
                    self.expr(v);
                }
            }
            ExprKind::Field { base, .. }
            | ExprKind::Deref(base)
            | ExprKind::CopyOf(base)
            | ExprKind::Trace(base) => self.expr(base),
            ExprKind::Index { base, index, .. } => {
                self.expr(base);
                self.expr(index);
            }
            ExprKind::Slice { base, lo, hi } => {
                self.expr(base);
                if let Some(x) = lo {
                    self.expr(x);
                }
                if let Some(x) = hi {
                    self.expr(x);
                }
            }
            ExprKind::Borrow { place, .. } => self.expr(place),
            ExprKind::Unary { operand, .. } => self.expr(operand),
            ExprKind::Binary { lhs, rhs, .. } | ExprKind::Logic { lhs, rhs, .. } => {
                self.expr(lhs);
                self.expr(rhs);
            }
            ExprKind::Compare { first, rest } => {
                self.expr(first);
                for (_, x) in rest {
                    self.expr(x);
                }
            }
            ExprKind::Coalesce { opt, default, .. } => {
                self.expr(opt);
                self.expr(default);
            }
            ExprKind::If { cond, then, else_ } => {
                self.expr(cond);
                self.expr(then);
                self.expr(else_);
            }
            ExprKind::Match { scrutinee, arms } => {
                self.expr(scrutinee);
                for a in arms {
                    self.arm(a);
                }
            }
            ExprKind::Try { expr, .. } | ExprKind::Coerce { expr, .. } => self.expr(expr),
            ExprKind::RefineTry { value, type_args, .. } => {
                self.expr(value);
                for t in type_args {
                    self.ty(t);
                }
            }
            ExprKind::OptChain { base, then, .. } => {
                self.expr(base);
                self.expr(then);
            }
            ExprKind::Interp(parts) => {
                for p in parts {
                    if let InterpPart::Value { expr, .. } = p {
                        self.expr(expr);
                    }
                }
            }
            ExprKind::Comprehension { clauses, elem, value, .. } => {
                for c in clauses {
                    match c {
                        CompClause::For { pat, iter, item, .. } => {
                            self.pat(pat);
                            self.expr(iter);
                            self.ty(item);
                        }
                        CompClause::If(x) => self.expr(x),
                    }
                }
                self.expr(elem);
                if let Some(v) = value {
                    self.expr(v);
                }
            }
            _ => {}
        }
    }
}
