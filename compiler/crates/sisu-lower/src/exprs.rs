//! Expressions: places, operands, and evaluation into a destination.

use crate::func::{DropEntry, FnLower, closure_name};
use crate::stmts::ForBody;
use sisu_resolve::{Program, Ty};
use sisu_sir as sir;
use sisu_sir::{Callee, Const, Operand, Place, Proj, Rvalue, StatementKind, Terminator};
use sisu_syntax::ast::{BinOp, CmpOp, UnOp};
use sisu_typeck::thir::{
    self, ArgMode, Coercion, CompClause, CompKind, ExprKind, FieldInit, IndexKind, InterpPart, TryKind,
};

/// The rest of a comprehension inside its loops.
pub struct CompRest<'b> {
    pub clauses: &'b [CompClause],
    pub elem: &'b thir::Expr,
    pub value: Option<&'b thir::Expr>,
    pub dest: Place,
    /// The type of the collection being built.
    pub dest_ty: sir::Type,
    pub kind: CompKind,
    pub item_tys: (Ty, Option<Ty>),
}

pub fn bin_op(op: BinOp, ty: &Ty, prog: &Program) -> sir::BinOp {
    let is_string = matches!(ty, Ty::Adt(d, _) if prog.builtin(*d) == Some("String"));
    match op {
        BinOp::Add if is_string => sir::BinOp::Concat,
        BinOp::Add => sir::BinOp::Add,
        BinOp::Sub => sir::BinOp::Sub,
        BinOp::Mul => sir::BinOp::Mul,
        BinOp::Div => sir::BinOp::Div,
        BinOp::FloorDiv => sir::BinOp::FloorDiv,
        BinOp::Mod => sir::BinOp::Mod,
        BinOp::Pow => sir::BinOp::Pow,
        BinOp::WrapAdd => sir::BinOp::WrapAdd,
        BinOp::WrapSub => sir::BinOp::WrapSub,
        BinOp::WrapMul => sir::BinOp::WrapMul,
        BinOp::SatAdd => sir::BinOp::SatAdd,
        BinOp::SatSub => sir::BinOp::SatSub,
        BinOp::SatMul => sir::BinOp::SatMul,
        BinOp::Shl => sir::BinOp::Shl,
        BinOp::Shr => sir::BinOp::Shr,
        BinOp::BitAnd => sir::BinOp::BitAnd,
        BinOp::BitOr => sir::BinOp::BitOr,
        BinOp::BitXor => sir::BinOp::BitXor,
        BinOp::And | BinOp::Or | BinOp::Coalesce => sir::BinOp::Eq,
    }
}

pub fn cmp_op(op: CmpOp) -> sir::BinOp {
    match op {
        CmpOp::Eq => sir::BinOp::Eq,
        CmpOp::Ne => sir::BinOp::Ne,
        CmpOp::Lt => sir::BinOp::Lt,
        CmpOp::Le => sir::BinOp::Le,
        CmpOp::Gt => sir::BinOp::Gt,
        CmpOp::Ge => sir::BinOp::Ge,
        CmpOp::In | CmpOp::NotIn => sir::BinOp::In,
    }
}

impl FnLower<'_, '_> {
    // ---------------------------------------------------------------- places and operands

    pub fn local_place(&mut self, l: thir::LocalId) -> Place {
        if let Some(p) = self.alias.get(&l) {
            return p.clone();
        }
        if let Some(&k) = self.env.get(&l) {
            return Place { local: sir::LocalId(1), proj: vec![Proj::Field(k), Proj::Deref] };
        }
        Place::local(self.local_for(l))
    }

    /// The place of an expression; a temporary (dropped at the end of the
    /// statement) for one that is not a place.
    pub fn place_of(&mut self, e: &thir::Expr) -> Place {
        match &e.kind {
            ExprKind::Local(l) => self.local_place(*l),
            ExprKind::Field { base, index } => self.place_of(base).project(Proj::Field(*index)),
            ExprKind::Index { base, index, kind } => {
                let b = self.place_of(base);
                let i = self.temp(&index.ty);
                self.into(Place::local(i), index);
                if *kind == IndexKind::Dict && self.needs_drop(&index.ty) {
                    self.register(DropEntry::Local(i));
                    self.split();
                }
                b.project(match kind {
                    IndexKind::List => Proj::Index(i),
                    IndexKind::ListFromEnd => Proj::IndexFromEnd(i),
                    IndexKind::Dict => Proj::Key(i),
                })
            }
            ExprKind::Deref(inner) => {
                if inner.is_place() {
                    self.place_of(inner).project(Proj::Deref)
                } else {
                    let t = self.temp(&inner.ty);
                    self.into(Place::local(t), inner);
                    Place::local(t).project(Proj::Deref)
                }
            }
            _ => {
                let t = self.temp(&e.ty);
                self.into(Place::local(t), e);
                if self.needs_drop(&e.ty) && e.ty != Ty::Never {
                    self.register(DropEntry::Local(t));
                    self.split();
                }
                Place::local(t)
            }
        }
    }

    /// Reads a place by value: a copy, or a move of a non-`Copy` value.
    pub fn copy_or_move(&mut self, p: Place, ty: &Ty) -> Operand {
        if self.ctx.prog.is_copy(ty) || matches!(ty, Ty::Ref(..)) {
            return Operand::Copy(p);
        }
        Operand::Move(p)
    }

    fn literal(&self, e: &thir::Expr) -> Option<Const> {
        Some(match &e.kind {
            ExprKind::Int(n) => Const::Int(*n),
            ExprKind::Float(f) => Const::Float(*f),
            ExprKind::Bool(b) => Const::Bool(*b),
            ExprKind::Char(c) => Const::Char(*c),
            ExprKind::Unit => Const::Unit,
            ExprKind::Str(s) if e.ty == Ty::Str => Const::Str(s.clone()),
            ExprKind::FnRef(d) => Const::Fn(self.ctx.fn_name(*d)),
            _ => return None,
        })
    }

    /// The value of an expression as an operand.
    pub fn operand(&mut self, e: &thir::Expr) -> Operand {
        if let Some(c) = self.literal(e) {
            return Operand::Const(c);
        }
        if e.is_place() {
            let p = self.place_of(e);
            return self.copy_or_move(p, &e.ty);
        }
        let t = self.temp(&e.ty);
        self.into(Place::local(t), e);
        if self.ctx.prog.is_copy(&e.ty) || matches!(e.ty, Ty::Ref(..)) {
            Operand::Copy(Place::local(t))
        } else {
            Operand::Move(Place::local(t))
        }
    }

    /// A reference to the expression's place (or to a temporary).
    pub fn borrow_operand(&mut self, e: &thir::Expr, mutable: bool) -> Operand {
        let p = self.place_of(e);
        let t = self.temp_sir(sir::Type::Ref(Box::new(self.ctx.ty(&e.ty)), mutable));
        let loc = self.ctx.loc(e.span);
        self.assign(Place::local(t), Rvalue::Ref(p, mutable), loc);
        Operand::Copy(Place::local(t))
    }

    /// An operand for reading without consuming: a copy of a `Copy`
    /// value, a reference otherwise.
    pub fn compare_operand(&mut self, e: &thir::Expr) -> Operand {
        if self.ctx.prog.is_copy(&e.ty) || matches!(e.ty, Ty::Ref(..)) {
            self.operand(e)
        } else {
            self.borrow_operand(e, false)
        }
    }

    /// The expression as a `str` value.
    pub fn str_operand(&mut self, e: &thir::Expr) -> Operand {
        match &e.ty {
            Ty::Str => self.operand(e),
            Ty::Ref(inner, _) => {
                let p = self.place_of(e).project(Proj::Deref);
                if **inner == Ty::Str {
                    return Operand::Copy(p);
                }
                let t = self.temp_sir(sir::Type::Str);
                let loc = self.ctx.loc(e.span);
                self.assign(Place::local(t), Rvalue::StrView(p), loc);
                Operand::Copy(Place::local(t))
            }
            _ => {
                let p = self.place_of(e);
                let t = self.temp_sir(sir::Type::Str);
                let loc = self.ctx.loc(e.span);
                self.assign(Place::local(t), Rvalue::StrView(p), loc);
                Operand::Copy(Place::local(t))
            }
        }
    }

    fn arg_operand(&mut self, a: &thir::Arg) -> Operand {
        match a.mode {
            ArgMode::Value => self.operand(&a.expr),
            ArgMode::Borrow => {
                if self.ctx.prog.is_copy(&a.expr.ty) || matches!(a.expr.ty, Ty::Ref(..)) {
                    self.operand(&a.expr)
                } else {
                    self.borrow_operand(&a.expr, false)
                }
            }
            ArgMode::BorrowMut => {
                if matches!(a.expr.ty, Ty::Ref(_, true)) {
                    self.operand(&a.expr)
                } else {
                    self.borrow_operand(&a.expr, true)
                }
            }
        }
    }

    fn variant_names(&self, def: sisu_resolve::DefId, v: u32) -> (String, String) {
        let prog = self.ctx.prog;
        let tname = if Some(def) == prog.wk.option {
            "Option".to_string()
        } else if Some(def) == prog.wk.result {
            "Result".to_string()
        } else {
            self.ctx.type_name(def)
        };
        let vname = prog
            .enum_info(def)
            .and_then(|e| e.variants.get(v as usize))
            .map(|x| x.name.clone())
            .unwrap_or_default();
        (tname, vname)
    }

    fn intrinsic_call(
        &mut self,
        dest: Place,
        name: &str,
        tys: Vec<sir::Type>,
        args: Vec<Operand>,
        loc: Option<sir::Loc>,
    ) {
        self.emit(StatementKind::Call { dest, callee: Callee::Intrinsic(name.into(), tys), args }, loc);
    }

    // ---------------------------------------------------------------- evaluation into a place

    pub fn into(&mut self, dest: Place, e: &thir::Expr) {
        let loc = self.ctx.loc(e.span);
        let prog = self.ctx.prog;
        if let Some(c) = self.literal(e) {
            self.assign(dest, Rvalue::Use(Operand::Const(c)), loc);
            return;
        }
        match &e.kind {
            ExprKind::Str(s) => {
                // A literal that took the type `String` or `Path` (FromLiteral).
                let name = if matches!(&e.ty, Ty::Adt(d, _) if prog.builtin(*d) == Some("Path")) {
                    "Path.new"
                } else {
                    "String.new"
                };
                self.intrinsic_call(dest, name, Vec::new(), vec![Operand::Const(Const::Str(s.clone()))], loc);
            }
            ExprKind::Local(_) | ExprKind::Field { .. } | ExprKind::Index { .. } | ExprKind::Deref(_) => {
                let p = self.place_of(e);
                let v = self.copy_or_move(p, &e.ty);
                self.assign(dest, Rvalue::Use(v), loc);
            }
            ExprKind::Const(d) => {
                let name = self.ctx.const_name(*d);
                self.emit(StatementKind::Call { dest, callee: Callee::Fn(name), args: Vec::new() }, loc);
            }
            ExprKind::Call { callee, args } => {
                let c = match callee {
                    thir::Callee::Fn { def, type_args } => {
                        match prog.fn_info(*def).and_then(|f| f.intrinsic.clone()) {
                            Some(n) => {
                                let tys = if self.needs_type_args(*def) {
                                    type_args.iter().map(|t| self.ctx.ty(t)).collect()
                                } else {
                                    Vec::new()
                                };
                                Callee::Intrinsic(n, tys)
                            }
                            None => Callee::Fn(self.ctx.fn_name(*def)),
                        }
                    }
                    thir::Callee::Value(f) => Callee::Value(self.operand(f)),
                };
                let ops = args.iter().map(|a| self.arg_operand(a)).collect();
                self.emit(StatementKind::Call { dest, callee: c, args: ops }, loc);
                if e.ty == Ty::Never {
                    self.terminate(Terminator::Unreachable, loc);
                }
            }
            ExprKind::Variant { def, variant, args } => {
                let ops = args.iter().map(|a| self.operand(a)).collect();
                let (t, v) = self.variant_names(*def, *variant);
                self.assign(dest, Rvalue::Aggregate(sir::Aggregate::Variant(t, v), ops), loc);
            }
            ExprKind::Struct { def, fields } => {
                let mut ops = Vec::new();
                for (i, f) in fields.iter().enumerate() {
                    match f {
                        FieldInit::Value(x) => ops.push(self.operand(x)),
                        FieldInit::Default(fi) => {
                            let fty = prog
                                .struct_info(*def)
                                .and_then(|s| s.fields.get(i))
                                .map(|f| f.ty.clone())
                                .unwrap_or(Ty::Error);
                            let t = self.temp(&fty);
                            let name = self.ctx.field_default_name(*def, *fi);
                            self.emit(
                                StatementKind::Call {
                                    dest: Place::local(t),
                                    callee: Callee::Fn(name),
                                    args: Vec::new(),
                                },
                                loc,
                            );
                            ops.push(if prog.is_copy(&fty) {
                                Operand::Copy(Place::local(t))
                            } else {
                                Operand::Move(Place::local(t))
                            });
                        }
                    }
                }
                self.assign(
                    dest,
                    Rvalue::Aggregate(sir::Aggregate::Struct(self.ctx.type_name(*def)), ops),
                    loc,
                );
            }
            ExprKind::Tuple(xs) => {
                let ops = xs.iter().map(|x| self.operand(x)).collect();
                self.assign(dest, Rvalue::Aggregate(sir::Aggregate::Tuple, ops), loc);
            }
            ExprKind::List(xs) => {
                let ops = xs.iter().map(|x| self.operand(x)).collect();
                self.assign(dest, Rvalue::Aggregate(sir::Aggregate::List, ops), loc);
            }
            ExprKind::Set(xs) => {
                let ops = xs.iter().map(|x| self.operand(x)).collect();
                self.assign(dest, Rvalue::Aggregate(sir::Aggregate::Set, ops), loc);
            }
            ExprKind::Dict(kv) => {
                let mut ops = Vec::new();
                for (k, v) in kv {
                    ops.push(self.operand(k));
                    ops.push(self.operand(v));
                }
                self.assign(dest, Rvalue::Aggregate(sir::Aggregate::Dict, ops), loc);
            }
            ExprKind::Slice { base, lo, hi } => {
                let b = self.borrow_operand(base, false);
                let lo = match lo {
                    Some(x) => self.operand(x),
                    None => Operand::Const(Const::Int(0)),
                };
                let hi = match hi {
                    Some(x) => self.operand(x),
                    None => Operand::Const(Const::Int(i64::MAX)),
                };
                self.intrinsic_call(dest, "list.slice", Vec::new(), vec![b, lo, hi], loc);
            }
            ExprKind::Borrow { place, mutable } => {
                let p = self.place_of(place);
                self.assign(dest, Rvalue::Ref(p, *mutable), loc);
            }
            ExprKind::Unary { op, operand } => {
                let v = self.operand(operand);
                let u = match op {
                    UnOp::Neg => sir::UnOp::Neg,
                    UnOp::Not => sir::UnOp::Not,
                    UnOp::BitNot => sir::UnOp::BitNot,
                };
                self.assign(dest, Rvalue::Unary(u, v), loc);
            }
            ExprKind::Binary { op, lhs, rhs } => {
                let bop = bin_op(*op, &e.ty, prog);
                let (a, b) = if bop == sir::BinOp::Concat {
                    (self.str_operand(lhs), self.str_operand(rhs))
                } else {
                    (self.operand(lhs), self.operand(rhs))
                };
                self.assign(dest, Rvalue::Binary(bop, a, b), loc);
            }
            ExprKind::Logic { and, lhs, rhs } => {
                self.into(dest.clone(), lhs);
                let rhs_bb = self.new_block();
                let join = self.new_block();
                let (zero, other) = if *and { (join, rhs_bb) } else { (rhs_bb, join) };
                self.terminate(
                    Terminator::Switch {
                        value: Operand::Copy(dest.clone()),
                        cases: vec![(0, zero)],
                        default: other,
                    },
                    loc,
                );
                self.switch_to(rhs_bb);
                self.push_scope();
                self.into(dest, rhs);
                self.pop_scope();
                self.goto(join);
                self.switch_to(join);
            }
            ExprKind::Compare { first, rest } => {
                let mut a = self.compare_operand(first);
                let join = self.new_block();
                for (i, (op, x)) in rest.iter().enumerate() {
                    let b = self.compare_operand(x);
                    if *op == CmpOp::NotIn {
                        let t = self.temp_sir(sir::Type::Bool);
                        self.assign(
                            Place::local(t),
                            Rvalue::Binary(sir::BinOp::In, a.clone(), b.clone()),
                            loc,
                        );
                        self.assign(
                            dest.clone(),
                            Rvalue::Unary(sir::UnOp::Not, Operand::Copy(Place::local(t))),
                            loc,
                        );
                    } else {
                        self.assign(dest.clone(), Rvalue::Binary(cmp_op(*op), a.clone(), b.clone()), loc);
                    }
                    if i + 1 < rest.len() {
                        let next = self.new_block();
                        self.terminate(
                            Terminator::Switch {
                                value: Operand::Copy(dest.clone()),
                                cases: vec![(0, join)],
                                default: next,
                            },
                            loc,
                        );
                        self.switch_to(next);
                    }
                    a = b;
                }
                self.goto(join);
                self.switch_to(join);
            }
            ExprKind::Coalesce { opt, default, deref } => {
                let p = self.place_of(opt);
                let inner = match &opt.ty {
                    Ty::Adt(_, a) => a.first().cloned().unwrap_or(Ty::Error),
                    _ => Ty::Error,
                };
                let d = self.temp_sir(sir::Type::Int);
                self.assign(Place::local(d), Rvalue::Discriminant(p.clone()), loc);
                let some = self.new_block();
                let none = self.new_block();
                let join = self.new_block();
                self.terminate(
                    Terminator::Switch {
                        value: Operand::Copy(Place::local(d)),
                        cases: vec![(0, some)],
                        default: none,
                    },
                    loc,
                );
                self.switch_to(some);
                let payload = p.project(Proj::Downcast(0)).project(Proj::Field(0));
                let v = if *deref {
                    Operand::Copy(payload.project(Proj::Deref))
                } else {
                    self.copy_or_move(payload, &inner)
                };
                self.assign(dest.clone(), Rvalue::Use(v), loc);
                self.goto(join);
                self.switch_to(none);
                self.push_scope();
                self.into(dest, default);
                self.pop_scope();
                self.goto(join);
                self.switch_to(join);
            }
            ExprKind::If { cond, then, else_ } => {
                let c = self.operand(cond);
                let t_bb = self.new_block();
                let e_bb = self.new_block();
                let join = self.new_block();
                self.terminate(Terminator::Switch { value: c, cases: vec![(0, e_bb)], default: t_bb }, loc);
                self.switch_to(t_bb);
                self.push_scope();
                self.into(dest.clone(), then);
                self.pop_scope();
                self.goto(join);
                self.switch_to(e_bb);
                self.push_scope();
                self.into(dest, else_);
                self.pop_scope();
                self.goto(join);
                self.switch_to(join);
            }
            ExprKind::Match { scrutinee, arms } => self.match_into(Some(dest), scrutinee, arms, e.span),
            ExprKind::Closure(id) => {
                let cl = &self.body.closures[id.0 as usize];
                let caps: Vec<(thir::LocalId, bool)> =
                    cl.captures.iter().map(|c| (c.local, c.mutable)).collect();
                let mut ops = Vec::new();
                for (l, m) in caps {
                    let p = self.local_place(l);
                    let ty = self.body.locals[l.0 as usize].ty.clone();
                    let t = self.temp_sir(sir::Type::Ref(Box::new(self.ctx.ty(&ty)), m));
                    self.assign(Place::local(t), Rvalue::Ref(p, m), loc);
                    ops.push(Operand::Copy(Place::local(t)));
                }
                let name = closure_name(&self.ctx.body_name(self.body), *id);
                self.assign(dest, Rvalue::Closure(name, ops), loc);
            }
            ExprKind::Try { expr, kind } => {
                let p = self.place_of(expr);
                let d = self.temp_sir(sir::Type::Int);
                self.assign(Place::local(d), Rvalue::Discriminant(p.clone()), loc);
                let ok = self.new_block();
                let err = self.new_block();
                self.terminate(
                    Terminator::Switch {
                        value: Operand::Copy(Place::local(d)),
                        cases: vec![(0, ok)],
                        default: err,
                    },
                    loc,
                );
                self.switch_to(err);
                match kind {
                    TryKind::Result { .. } => {
                        let err_ty = match &expr.ty {
                            Ty::Adt(_, a) => a.get(1).cloned().unwrap_or(Ty::Error),
                            _ => Ty::Error,
                        };
                        let t = self.temp(&err_ty);
                        self.assign(
                            Place::local(t),
                            Rvalue::Use(Operand::Move(p.project(Proj::Downcast(1)).project(Proj::Field(0)))),
                            loc,
                        );
                        let r = self.temp_sir(sir::Type::Ref(Box::new(self.ctx.ty(&err_ty)), true));
                        self.assign(Place::local(r), Rvalue::Ref(Place::local(t), true), loc);
                        let at = self.location_text(e.span);
                        let u = self.unit();
                        self.intrinsic_call(
                            u,
                            "error.trace_push",
                            Vec::new(),
                            vec![Operand::Copy(Place::local(r)), Operand::Const(Const::Str(at))],
                            loc,
                        );
                        self.assign(
                            Place::local(sir::LocalId(0)),
                            Rvalue::Aggregate(
                                sir::Aggregate::Variant("Result".into(), "Err".into()),
                                vec![Operand::Move(Place::local(t))],
                            ),
                            loc,
                        );
                    }
                    TryKind::Option => {
                        self.assign(
                            Place::local(sir::LocalId(0)),
                            Rvalue::Aggregate(
                                sir::Aggregate::Variant("Option".into(), "None".into()),
                                Vec::new(),
                            ),
                            loc,
                        );
                    }
                }
                self.emit_exit(0, loc);
                self.terminate(Terminator::Return, loc);
                self.switch_to(ok);
                let v = self.copy_or_move(p.project(Proj::Downcast(0)).project(Proj::Field(0)), &e.ty);
                self.assign(dest, Rvalue::Use(v), loc);
            }
            ExprKind::OptChain { base, binder, then, flatten } => {
                let p = self.place_of(base);
                let d = self.temp_sir(sir::Type::Int);
                self.assign(Place::local(d), Rvalue::Discriminant(p.clone()), loc);
                let some = self.new_block();
                let none = self.new_block();
                let join = self.new_block();
                self.terminate(
                    Terminator::Switch {
                        value: Operand::Copy(Place::local(d)),
                        cases: vec![(0, some)],
                        default: none,
                    },
                    loc,
                );
                self.switch_to(some);
                self.push_scope();
                self.alias.insert(*binder, p.project(Proj::Downcast(0)).project(Proj::Field(0)));
                let t = self.temp(&then.ty);
                self.into(Place::local(t), then);
                let v = self.copy_or_move(Place::local(t), &then.ty);
                if *flatten {
                    self.assign(dest.clone(), Rvalue::Use(v), loc);
                } else {
                    self.assign(
                        dest.clone(),
                        Rvalue::Aggregate(sir::Aggregate::Variant("Option".into(), "Some".into()), vec![v]),
                        loc,
                    );
                }
                self.pop_scope();
                self.goto(join);
                self.switch_to(none);
                self.assign(
                    dest,
                    Rvalue::Aggregate(sir::Aggregate::Variant("Option".into(), "None".into()), Vec::new()),
                    loc,
                );
                self.goto(join);
                self.switch_to(join);
            }
            ExprKind::Interp(parts) => {
                self.intrinsic_call(dest.clone(), "fmt.new", Vec::new(), Vec::new(), loc);
                let r = self.temp_sir(sir::Type::Ref(Box::new(self.ctx.ty(&e.ty)), true));
                self.assign(Place::local(r), Rvalue::Ref(dest, true), loc);
                for part in parts {
                    match part {
                        InterpPart::Text(s) => {
                            let u = self.unit();
                            self.intrinsic_call(
                                u,
                                "fmt.text",
                                Vec::new(),
                                vec![Operand::Copy(Place::local(r)), Operand::Const(Const::Str(s.clone()))],
                                loc,
                            );
                        }
                        InterpPart::Value { expr, debug, spec } => {
                            if let Some(src) = debug {
                                let u = self.unit();
                                self.intrinsic_call(
                                    u,
                                    "fmt.text",
                                    Vec::new(),
                                    vec![
                                        Operand::Copy(Place::local(r)),
                                        Operand::Const(Const::Str(format!("{src}="))),
                                    ],
                                    loc,
                                );
                            }
                            let v = self.compare_operand(expr);
                            let (name, spec) = if debug.is_some() || spec == "?" {
                                ("fmt.debug", if spec == "?" { String::new() } else { spec.clone() })
                            } else {
                                ("fmt.display", spec.clone())
                            };
                            let u = self.unit();
                            self.intrinsic_call(
                                u,
                                name,
                                Vec::new(),
                                vec![Operand::Copy(Place::local(r)), v, Operand::Const(Const::Str(spec))],
                                self.ctx.loc(expr.span),
                            );
                        }
                    }
                }
            }
            ExprKind::Comprehension { kind, clauses, elem, value } => {
                let (new, tys) = match (kind, &e.ty) {
                    (CompKind::List, Ty::Adt(_, a)) => {
                        ("list.new", a.iter().map(|t| self.ctx.ty(t)).collect())
                    }
                    (CompKind::Set, Ty::Adt(_, a)) => ("set.new", a.iter().map(|t| self.ctx.ty(t)).collect()),
                    (CompKind::Dict, Ty::Adt(_, a)) => {
                        ("dict.new", a.iter().map(|t| self.ctx.ty(t)).collect())
                    }
                    _ => ("list.new", Vec::new()),
                };
                self.intrinsic_call(dest.clone(), new, tys, Vec::new(), loc);
                let item_tys = (elem.ty.clone(), value.as_ref().map(|v| v.ty.clone()));
                let dest_ty = self.ctx.ty(&e.ty);
                self.comprehension_rest(CompRest {
                    clauses,
                    elem,
                    value: value.as_deref(),
                    dest,
                    dest_ty,
                    kind: *kind,
                    item_tys,
                });
            }
            ExprKind::Coerce { expr, kind } => match kind {
                Coercion::StrView => {
                    let p = self.place_of(expr);
                    self.assign(dest, Rvalue::StrView(p), loc);
                }
                Coercion::RefStrView => {
                    let p = self.place_of(expr).project(Proj::Deref);
                    self.assign(dest, Rvalue::StrView(p), loc);
                }
                Coercion::DerefCopy => {
                    let p = self.place_of(expr).project(Proj::Deref);
                    self.assign(dest, Rvalue::Use(Operand::Copy(p)), loc);
                }
                Coercion::MutToShared => {
                    let p = self.place_of(expr);
                    self.assign(dest, Rvalue::Use(Operand::Copy(p)), loc);
                }
                Coercion::NeverToAny | Coercion::Widen | Coercion::Mismatch => self.into(dest, expr),
            },
            ExprKind::DefaultArg { def, param } => {
                let name = self.ctx.param_default_name(*def, *param);
                self.emit(StatementKind::Call { dest, callee: Callee::Fn(name), args: Vec::new() }, loc);
            }
            ExprKind::CopyOf(inner) => {
                let p = self.place_of(inner);
                self.assign(dest, Rvalue::CopyOf(p), loc);
            }
            ExprKind::Trace(inner) => {
                let r = self.borrow_operand(inner, false);
                self.intrinsic_call(dest, "error.trace", Vec::new(), vec![r], loc);
            }
            ExprKind::Error => self.terminate(Terminator::Unreachable, loc),
            ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::Bool(_)
            | ExprKind::Char(_)
            | ExprKind::Unit
            | ExprKind::FnRef(_) => {}
        }
    }

    /// Whether an intrinsic needs its type arguments at run time: one of
    /// its generic parameters appears in its result but in no parameter,
    /// or a parameter is an `Iterable[T]` (whose items are copies when `T`
    /// is not a reference, and references otherwise).
    fn needs_type_args(&self, def: sisu_resolve::DefId) -> bool {
        let Some(f) = self.ctx.prog.fn_info(def) else { return false };
        let iterable = self.ctx.prog.wk.iterable;
        if f.params.iter().any(|p| matches!(&p.ty, Ty::Adt(d, _) if Some(*d) == iterable)) {
            return true;
        }
        fn params_of(t: &Ty, out: &mut Vec<u32>) {
            match t {
                Ty::Param(i) => out.push(*i),
                Ty::Adt(_, a) => a.iter().for_each(|x| params_of(x, out)),
                Ty::Tuple(e, _) => e.iter().for_each(|x| params_of(x, out)),
                Ty::Fn(fty) => {
                    fty.params.iter().for_each(|x| params_of(x, out));
                    params_of(&fty.ret, out);
                }
                Ty::Ref(t, _) => params_of(t, out),
                _ => {}
            }
        }
        let mut in_params = Vec::new();
        for p in &f.params {
            params_of(&p.ty, &mut in_params);
        }
        if let Some(o) = &f.owner
            && f.self_mode.is_some()
        {
            params_of(o, &mut in_params);
        }
        let mut in_ret = Vec::new();
        params_of(&f.ret, &mut in_ret);
        in_ret.iter().any(|i| !in_params.contains(i))
    }

    fn location_text(&self, span: sisu_diag::Span) -> String {
        let name = self.ctx.sources.name(span.file);
        match self.ctx.loc(span) {
            Some(l) => format!("{name}:{}:{}", l.line, l.col),
            None => name.to_string(),
        }
    }

    pub fn comprehension_rest(&mut self, c: CompRest) {
        let Some((first, rest)) = c.clauses.split_first() else {
            let r = self.temp_sir(sir::Type::Ref(Box::new(c.dest_ty.clone()), true));
            self.assign(Place::local(r), Rvalue::Ref(c.dest.clone(), true), None);
            let loc = self.ctx.loc(c.elem.span);
            match c.kind {
                CompKind::List => {
                    let v = self.operand(c.elem);
                    let u = self.unit();
                    self.intrinsic_call(
                        u,
                        "list.push",
                        Vec::new(),
                        vec![Operand::Copy(Place::local(r)), v],
                        loc,
                    );
                }
                CompKind::Set => {
                    let v = self.operand(c.elem);
                    let t = self.temp_sir(sir::Type::Bool);
                    self.intrinsic_call(
                        Place::local(t),
                        "set.add",
                        Vec::new(),
                        vec![Operand::Copy(Place::local(r)), v],
                        loc,
                    );
                }
                CompKind::Dict => {
                    let k = self.operand(c.elem);
                    let v = match c.value {
                        Some(v) => self.operand(v),
                        None => Operand::Const(Const::Unit),
                    };
                    let vty = c.item_tys.1.clone().unwrap_or(Ty::Unit);
                    let opt =
                        self.ctx.prog.wk.option.map(|o| Ty::Adt(o, vec![vty.clone()])).unwrap_or(Ty::Unit);
                    let t = self.temp(&opt);
                    self.intrinsic_call(
                        Place::local(t),
                        "dict.insert",
                        Vec::new(),
                        vec![Operand::Copy(Place::local(r)), k, v],
                        loc,
                    );
                    if self.needs_drop(&vty) {
                        self.split();
                        self.emit(StatementKind::Drop { place: Place::local(t), flag: None }, loc);
                        self.split();
                    }
                }
            }
            return;
        };
        match first {
            CompClause::For { pat, iter, mode, item } => {
                let span = iter.span;
                let next = CompRest {
                    clauses: rest,
                    elem: c.elem,
                    value: c.value,
                    dest: c.dest,
                    dest_ty: c.dest_ty,
                    kind: c.kind,
                    item_tys: c.item_tys,
                };
                self.for_loop(pat, iter, *mode, item, ForBody::Comprehension(Box::new(next)), span);
            }
            CompClause::If(cond) => {
                let after = self.new_block();
                let depth = self.scopes.len();
                self.push_scope();
                let v = self.operand(cond);
                let ok = self.new_block();
                let f = self.edge_to(after, depth);
                self.terminate(
                    Terminator::Switch { value: v, cases: vec![(0, f)], default: ok },
                    self.ctx.loc(cond.span),
                );
                self.switch_to(ok);
                let next = CompRest {
                    clauses: rest,
                    elem: c.elem,
                    value: c.value,
                    dest: c.dest,
                    dest_ty: c.dest_ty,
                    kind: c.kind,
                    item_tys: c.item_tys,
                };
                self.comprehension_rest(next);
                self.pop_scope();
                self.goto(after);
                self.switch_to(after);
            }
        }
    }
}
