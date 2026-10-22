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
            // What a closure keeps by value its environment holds itself.
            let own = self.closure.is_some_and(|c| {
                self.body.closures[c.0 as usize].captures.get(k as usize).is_some_and(|cap| cap.by_value)
            });
            return if own {
                Place { local: sir::LocalId(1), proj: vec![Proj::Field(k)] }
            } else {
                Place { local: sir::LocalId(1), proj: vec![Proj::Field(k), Proj::Deref] }
            };
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
            // A refinement's value is its base's: the same place.
            ExprKind::Coerce { expr, kind: Coercion::Unrefine } => self.place_of(expr),
            ExprKind::Deref(inner) => {
                if inner.is_place() {
                    self.place_of(inner).project(Proj::Deref)
                } else {
                    let t = self.temp(&inner.ty);
                    self.into(Place::local(t), inner);
                    // A `Shared` or a mutex guard made for the expression
                    // is dropped with the statement's temporaries.
                    if self.needs_drop(&inner.ty) && inner.ty != Ty::Never {
                        self.register(DropEntry::Local(t));
                        self.split();
                    }
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
        if self.ctx.prog.is_copy_in(ty, &self.bounds) || matches!(ty, Ty::Ref(..)) {
            return Operand::Copy(p);
        }
        Operand::Move(p)
    }

    fn literal(&self, e: &thir::Expr) -> Option<Const> {
        Some(match &e.kind {
            // A `u64`, `u128` or `i128` is a value of its own kind, which
            // an intrinsic makes.
            ExprKind::Int(_) if self.sized_int(&e.ty).is_some_and(sisu_resolve::std_sources::own_kind) => {
                return None;
            }
            // So is an `f32`, made from the `f64` that holds it.
            ExprKind::Float(_) if self.is_f32(&e.ty) => return None,
            ExprKind::Int(n) => Const::Int(*n),
            ExprKind::Float(f) => Const::Float(*f),
            ExprKind::Bool(b) => Const::Bool(*b),
            ExprKind::Char(c) => Const::Char(*c),
            ExprKind::Unit => Const::Unit,
            ExprKind::Str(s) if e.ty == Ty::Str => Const::Str(s.clone()),
            ExprKind::FnRef(d, _)
                if !self.ctx.prog.fn_info(*d).is_some_and(|f| crate::func::value_wrapped(self.ctx, f)) =>
            {
                Const::Fn(self.ctx.fn_name(*d))
            }
            _ => return None,
        })
    }

    /// Whether `t` is `f32`.
    pub fn is_f32(&self, t: &Ty) -> bool {
        matches!(t, Ty::Adt(d, a) if a.is_empty() && self.ctx.prog.builtin(*d) == Some("f32"))
    }

    /// The name of a sized integer type (`u8`), `None` for any other.
    pub fn sized_int(&self, t: &Ty) -> Option<&'static str> {
        match t {
            Ty::Adt(d, a) if a.is_empty() => {
                let b = self.ctx.prog.builtin(*d)?;
                sisu_resolve::std_sources::SIZED.iter().copied().find(|s| *s == b)
            }
            _ => None,
        }
    }

    /// Whether evaluating an expression may leave the statement it is part
    /// of before the statement runs: by a `?`, a `return`, a `break`, or a
    /// panic (a call, an index, arithmetic, a contract). Only literals,
    /// locals, functions, closures, and the parts, borrows and aggregates of
    /// such cannot; a struct with an invariant or a field's default can.
    pub fn may_leave(&self, e: &thir::Expr) -> bool {
        match &e.kind {
            ExprKind::Int(_)
            | ExprKind::WideInt(_)
            | ExprKind::Float(_)
            | ExprKind::Bool(_)
            | ExprKind::Char(_)
            | ExprKind::Str(_)
            | ExprKind::Bytes(_)
            | ExprKind::Unit
            | ExprKind::Local(_)
            | ExprKind::FnRef(..)
            | ExprKind::Closure(_) => false,
            ExprKind::Field { base, .. } => self.may_leave(base),
            ExprKind::Deref(inner) => self.may_leave(inner),
            ExprKind::Borrow { place, .. } => self.may_leave(place),
            ExprKind::Tuple(xs) | ExprKind::List(xs) | ExprKind::Variant { args: xs, .. } => {
                xs.iter().any(|x| self.may_leave(x))
            }
            ExprKind::Struct { def, fields } => {
                self.ctx.prog.invariant_check(*def).is_some()
                    || fields.iter().any(|f| match f {
                        FieldInit::Value(x) => self.may_leave(x),
                        FieldInit::Default(_) => true,
                    })
            }
            _ => true,
        }
    }

    /// The values of expressions as the operands of one statement, in
    /// order: each held while the ones after it are evaluated, when one of
    /// them may leave ([`FnLower::hold`]), so that leaving drops it.
    pub fn operands<'x>(&mut self, es: impl Iterator<Item = &'x thir::Expr>) -> Vec<Operand> {
        let es: Vec<&thir::Expr> = es.collect();
        let later = self.leaving_after(es.iter().map(|e| self.may_leave(e)));
        let mut held = Vec::new();
        let mut ops = Vec::new();
        for (e, hold) in es.into_iter().zip(later) {
            let mark = self.locals.len();
            let o = self.operand(e);
            if hold {
                self.hold(&o, mark, &e.ty, &mut held);
            }
            ops.push(o);
        }
        self.release(&held);
        ops
    }

    /// For each of a statement's operands, whether one after it may leave
    /// the statement, from whether each may.
    pub fn leaving_after(&self, may: impl Iterator<Item = bool>) -> Vec<bool> {
        let may: Vec<bool> = may.collect();
        let mut out = vec![false; may.len()];
        for i in (0..may.len().saturating_sub(1)).rev() {
            out[i] = out[i + 1] || may[i + 1];
        }
        out
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
        if self.ctx.prog.is_copy_in(&e.ty, &self.bounds) || matches!(e.ty, Ty::Ref(..)) {
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
        if self.ctx.prog.is_copy_in(&e.ty, &self.bounds) || matches!(e.ty, Ty::Ref(..)) {
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

    /// An argument as the callee takes it; `by_value` when the callee
    /// takes by value what the call passes by reference (a trait method
    /// resolved to the impl of a `Copy` type).
    pub(crate) fn arg_operand_for(&mut self, a: &thir::Arg, by_value: bool) -> Operand {
        if by_value && matches!(a.mode, ArgMode::Ref | ArgMode::Value) {
            if let Ty::Ref(..) = &a.expr.ty {
                let p = self.place_of(&a.expr).project(Proj::Deref);
                return Operand::Copy(p);
            }
            return self.operand(&a.expr);
        }
        self.arg_operand(a)
    }

    /// A reference to the value of an expression: the expression itself
    /// when it is one, a borrow of its place otherwise.
    pub fn ref_operand(&mut self, e: &thir::Expr) -> Operand {
        if let Ty::Ref(..) = &e.ty { self.operand(e) } else { self.borrow_operand(e, false) }
    }

    fn arg_operand(&mut self, a: &thir::Arg) -> Operand {
        match a.mode {
            ArgMode::Ref => self.ref_operand(&a.expr),
            ArgMode::Value => self.operand(&a.expr),
            ArgMode::Borrow => {
                if self.ctx.prog.is_copy_in(&a.expr.ty, &self.bounds) || matches!(a.expr.ty, Ty::Ref(..)) {
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

    pub(crate) fn intrinsic_call(
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
                self.emit(
                    StatementKind::Call { dest, callee: Callee::Fn(name, Vec::new()), args: Vec::new() },
                    loc,
                );
            }
            // A `const` generic parameter: the constant its type argument
            // is, which the frame has at run time.
            ExprKind::ConstParam(i) => {
                self.intrinsic_call(dest, "generic.value", vec![sir::Type::Param(*i)], Vec::new(), loc);
            }
            ExprKind::Call { callee, args } => {
                if let thir::Callee::Fn { def, type_args } = callee
                    && self.special_call(dest.clone(), *def, type_args, args, e, loc)
                {
                    return;
                }
                let (c, by_value) = match callee {
                    thir::Callee::Fn { def, type_args } => self.callee_for(*def, type_args),
                    thir::Callee::Value(f) => (Callee::Value(self.operand(f)), Vec::new()),
                };
                let later = self.leaving_after(args.iter().map(|a| self.may_leave(&a.expr)));
                let mut held = Vec::new();
                let mut ops = Vec::new();
                for (i, a) in args.iter().enumerate() {
                    let mark = self.locals.len();
                    let o = self.arg_operand_for(a, by_value.get(i).copied().unwrap_or(false));
                    if later[i] {
                        self.hold(&o, mark, &a.expr.ty, &mut held);
                    }
                    ops.push(o);
                }
                self.release(&held);
                self.emit(StatementKind::Call { dest, callee: c, args: ops }, loc);
                if e.ty == Ty::Never {
                    self.terminate(Terminator::Unreachable, loc);
                }
            }
            ExprKind::Variant { def, variant, args } => {
                let ops = self.operands(args.iter());
                let (t, v) = self.variant_names(*def, *variant);
                self.assign(dest, Rvalue::Aggregate(sir::Aggregate::Variant(t, v), ops), loc);
            }
            ExprKind::Struct { def, fields } => {
                let mut ops = Vec::new();
                let mut held = Vec::new();
                let later = self.leaving_after(fields.iter().map(|f| match f {
                    FieldInit::Value(x) => self.may_leave(x),
                    FieldInit::Default(_) => true,
                }));
                for (i, f) in fields.iter().enumerate() {
                    let mark = self.locals.len();
                    match f {
                        FieldInit::Value(x) => {
                            let o = self.operand(x);
                            if later[i] {
                                self.hold(&o, mark, &x.ty, &mut held);
                            }
                            ops.push(o);
                        }
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
                                    callee: Callee::Fn(name, Vec::new()),
                                    args: Vec::new(),
                                },
                                loc,
                            );
                            let o = if prog.is_copy(&fty) {
                                Operand::Copy(Place::local(t))
                            } else {
                                Operand::Move(Place::local(t))
                            };
                            if later[i] {
                                self.hold(&o, mark, &fty, &mut held);
                            }
                            ops.push(o);
                        }
                    }
                }
                self.release(&held);
                self.assign(
                    dest.clone(),
                    Rvalue::Aggregate(sir::Aggregate::Struct(self.ctx.type_name(*def)), ops),
                    loc,
                );
                // A struct with an invariant checks it once built (07); a
                // panic of the check drops it.
                let args = match &e.ty {
                    Ty::Adt(_, a) => a.clone(),
                    _ => Vec::new(),
                };
                let unheld = match &dest {
                    Place { local, proj } if proj.is_empty() && !self.is_registered(*local) => Some(*local),
                    _ => None,
                }
                .filter(|_| self.needs_drop(&e.ty) && self.ctx.prog.invariant_check(*def).is_some());
                if let Some(l) = unheld {
                    self.register(DropEntry::Local(l));
                    self.split();
                }
                self.invariant_after_construction(&dest, *def, &args, loc);
                if let Some(l) = unheld {
                    self.release(&[l]);
                }
            }
            ExprKind::Tuple(xs) => {
                let ops = self.operands(xs.iter());
                self.assign(dest, Rvalue::Aggregate(sir::Aggregate::Tuple, ops), loc);
            }
            ExprKind::List(xs) => {
                let ops = self.operands(xs.iter());
                self.assign(dest, Rvalue::Aggregate(sir::Aggregate::List, ops), loc);
            }
            ExprKind::Set(xs) => {
                let ops = self.operands(xs.iter());
                self.assign(dest, Rvalue::Aggregate(sir::Aggregate::Set, ops), loc);
            }
            ExprKind::Dict(kv) => {
                let ops = self.operands(kv.iter().flat_map(|(k, v)| [k, v]));
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
            ExprKind::Float(f) => {
                // An `f32` literal (the others are constants) travels as
                // the `f64` that holds it exactly.
                self.intrinsic_call(
                    dest,
                    "f32.from_literal",
                    Vec::new(),
                    vec![Operand::Const(Const::Float(*f))],
                    loc,
                );
            }
            ExprKind::Bytes(b) => {
                // The bytes travel as hex text, which any byte survives.
                let hex: String = b.iter().map(|x| format!("{x:02x}")).collect();
                self.intrinsic_call(
                    dest,
                    "bytes.from_hex",
                    Vec::new(),
                    vec![Operand::Const(Const::Str(hex))],
                    loc,
                );
            }
            ExprKind::Int(n) => {
                // A literal of a kind of its own (the others are constants):
                // a `u64` from its bits, a 128-bit integer from the `int`
                // it equals.
                let k = self.sized_int(&e.ty).unwrap_or("u64");
                let name = if k == "u64" { "u64.from_bits".to_string() } else { format!("{k}.widen") };
                self.intrinsic_call(dest, &name, Vec::new(), vec![Operand::Const(Const::Int(*n))], loc);
            }
            ExprKind::WideInt(bits) => {
                // A 128-bit literal beyond `int` travels as its decimal text.
                let k = self.sized_int(&e.ty).unwrap_or("i128");
                let text = if k == "i128" { (*bits as i128).to_string() } else { bits.to_string() };
                self.intrinsic_call(
                    dest,
                    &format!("{k}.from_text"),
                    Vec::new(),
                    vec![Operand::Const(Const::Str(text))],
                    loc,
                );
            }
            ExprKind::Unary { op: UnOp::BitNot, operand } if self.sized_int(&operand.ty).is_some() => {
                let k = self.sized_int(&operand.ty).unwrap_or("int");
                let v = self.operand(operand);
                self.intrinsic_call(dest, &format!("{k}.not"), Vec::new(), vec![v], loc);
            }
            ExprKind::Binary { op, lhs, rhs } if self.sized_int(&lhs.ty).is_some() => {
                // The sized integers' arithmetic is the host's, by type.
                let k = self.sized_int(&lhs.ty).unwrap_or("int");
                let name = match op {
                    BinOp::Add => "add",
                    BinOp::Sub => "sub",
                    BinOp::Mul => "mul",
                    BinOp::FloorDiv | BinOp::Div => "floordiv",
                    BinOp::Mod => "rem",
                    BinOp::Pow => "pow",
                    BinOp::WrapAdd => "wrap_add",
                    BinOp::WrapSub => "wrap_sub",
                    BinOp::WrapMul => "wrap_mul",
                    BinOp::SatAdd => "sat_add",
                    BinOp::SatSub => "sat_sub",
                    BinOp::SatMul => "sat_mul",
                    BinOp::Shl => "shl",
                    BinOp::Shr => "shr",
                    BinOp::BitAnd => "and",
                    BinOp::BitOr => "or",
                    BinOp::BitXor => "xor",
                    BinOp::And | BinOp::Or | BinOp::Coalesce => "invalid",
                };
                let (a, b) = (self.operand(lhs), self.operand(rhs));
                self.intrinsic_call(dest, &format!("{k}.{name}"), Vec::new(), vec![a, b], loc);
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
                // Equality and order through the traits when the type's
                // impls are not the structural ones the operators compute.
                let prog = self.ctx.prog;
                let base = prog.normalize(first.ty.strip_ref(), &self.bounds);
                let eq_trait = prog.wk.eq.filter(|t| !prog.is_structural(&base, *t));
                let ord_trait = prog.wk.partial_ord.filter(|t| !prog.is_structural(&base, *t));
                // `x in xs` for a list whose elements have an `Eq` of their
                // own: `list.contains`, written in Sisu.
                let bounds = self.bounds.clone();
                let contains = |x: &sisu_typeck::Expr| -> Option<sisu_resolve::DefId> {
                    let container = prog.normalize(x.ty.strip_ref(), &bounds);
                    let Ty::Adt(d, args) = &container else { return None };
                    if Some(*d) != prog.wk.list {
                        return None;
                    }
                    let elem = args.first()?;
                    let eq = prog.wk.eq?;
                    if prog.is_structural(elem, eq) {
                        return None;
                    }
                    prog.method(container.key()?, "contains")
                };
                let any_trait = rest.iter().any(|(op, x)| match op {
                    CmpOp::Eq | CmpOp::Ne => eq_trait.is_some(),
                    CmpOp::In | CmpOp::NotIn => contains(x).is_some(),
                    _ => ord_trait.is_some(),
                });
                let mut a = if any_trait { self.ref_operand(first) } else { self.compare_operand(first) };
                let join = self.new_block();
                for (i, (op, x)) in rest.iter().enumerate() {
                    let b = if any_trait { self.ref_operand(x) } else { self.compare_operand(x) };
                    let via_contains =
                        if matches!(op, CmpOp::In | CmpOp::NotIn) { contains(x) } else { None };
                    match op {
                        CmpOp::In | CmpOp::NotIn if via_contains.is_some() => {
                            let elem = match prog.normalize(x.ty.strip_ref(), &self.bounds) {
                                Ty::Adt(_, args) => args.first().cloned().unwrap_or(Ty::Error),
                                _ => Ty::Error,
                            };
                            let m = via_contains.unwrap_or(sisu_resolve::DefId(0));
                            let (callee, _) = self.callee_for(m, std::slice::from_ref(&elem));
                            let t = self.temp_sir(sir::Type::Bool);
                            self.emit(
                                StatementKind::Call {
                                    dest: Place::local(t),
                                    callee,
                                    args: vec![b.clone(), a.clone()],
                                },
                                loc,
                            );
                            let r = Rvalue::Use(Operand::Copy(Place::local(t)));
                            let r = if *op == CmpOp::NotIn {
                                Rvalue::Unary(sir::UnOp::Not, Operand::Copy(Place::local(t)))
                            } else {
                                r
                            };
                            self.assign(dest.clone(), r, loc);
                        }
                        CmpOp::NotIn => {
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
                        }
                        CmpOp::Eq | CmpOp::Ne if eq_trait.is_some() => self.eq_via_trait(
                            dest.clone(),
                            a.clone(),
                            b.clone(),
                            *op == CmpOp::Ne,
                            &base,
                            loc,
                        ),
                        CmpOp::Lt | CmpOp::Le | CmpOp::Gt | CmpOp::Ge if ord_trait.is_some() => {
                            self.order_via_trait(dest.clone(), a.clone(), b.clone(), *op, &base, loc)
                        }
                        _ => {
                            self.assign(dest.clone(), Rvalue::Binary(cmp_op(*op), a.clone(), b.clone()), loc)
                        }
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
                let caps: Vec<(thir::LocalId, bool, bool)> =
                    cl.captures.iter().map(|c| (c.local, c.mutable, c.by_value)).collect();
                let mut ops = Vec::new();
                for (l, m, by_value) in caps {
                    let p = self.local_place(l);
                    // What the closure keeps by value is copied, or moved
                    // into a task.
                    if by_value {
                        let ty = self.body.locals[l.0 as usize].ty.clone();
                        ops.push(self.copy_or_move(p, &ty));
                        continue;
                    }
                    let ty = self.body.locals[l.0 as usize].ty.clone();
                    let t = self.temp_sir(sir::Type::Ref(Box::new(self.ctx.ty(&ty)), m));
                    self.assign(Place::local(t), Rvalue::Ref(p, m), loc);
                    ops.push(Operand::Copy(Place::local(t)));
                }
                let name = closure_name(&self.ctx.body_name(self.body), *id);
                let tys = (0..self.type_params.len() as u32).map(sir::Type::Param).collect();
                self.assign(dest, Rvalue::Closure(name, tys, ops), loc);
            }
            // A generator: the closure that runs its body, in an `Iter`.
            ExprKind::Generator(id) => {
                let fty = self.body.closures[id.0 as usize].fn_ty.clone();
                let c = self.temp(&fty);
                let closure = thir::Expr::new(ExprKind::Closure(*id), fty, e.span);
                self.into(Place::local(c), &closure);
                self.intrinsic_call(dest, "gen.new", Vec::new(), vec![Operand::Move(Place::local(c))], loc);
            }
            // The compiler runs the function and replaces its body by the
            // value; what runs here is a call that builds it.
            ExprKind::Comptime(id) => {
                let name = crate::func::comptime_name(&self.ctx.body_name(self.body), *id);
                self.emit(
                    StatementKind::Call { dest, callee: Callee::Fn(name, Vec::new()), args: Vec::new() },
                    loc,
                );
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
                        // An error's trace names the program's `?`s: the
                        // standard library's (and its templates') are left
                        // out, as a panic's trace leaves out its frames.
                        let module = self.ctx.prog.def(self.body.def).module;
                        if !self.ctx.prog.modules[module.0 as usize].is_std {
                            let at = self.location_text(e.span);
                            let u = self.unit();
                            self.intrinsic_call(
                                u,
                                "error.trace_push",
                                Vec::new(),
                                vec![Operand::Copy(Place::local(r)), Operand::Const(Const::Str(at))],
                                loc,
                            );
                        }
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
                self.contract_exit(loc);
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
                            // A spec ending in `?` asks for `Debug` (`{x:>8?}`).
                            let (is_debug, spec) = match spec.strip_suffix('?') {
                                Some(rest) => (true, rest.to_string()),
                                None => (debug.is_some(), spec.clone()),
                            };
                            let name = if is_debug { "fmt.debug" } else { "fmt.display" };
                            let base = self.ctx.prog.erase_refinements(expr.ty.strip_ref());
                            let wk = &self.ctx.prog.wk;
                            let tr = if is_debug { wk.debug } else { wk.display };
                            // A spec is applied by the value's impl, in Sisu; the host
                            // writes the plain text of a structural value.
                            if !spec.is_empty() || tr.is_some_and(|t| !self.ctx.prog.is_structural(&base, t))
                            {
                                self.format_via_trait(Operand::Copy(Place::local(r)), expr, is_debug, &spec);
                                continue;
                            }
                            let v = self.compare_operand(expr);
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
                Coercion::NeverToAny | Coercion::Widen | Coercion::Mismatch | Coercion::Unrefine => {
                    self.into(dest, expr)
                }
                Coercion::FloatWiden => {
                    let v = self.operand(expr);
                    self.intrinsic_call(dest, "f32.to_f64", Vec::new(), vec![v], loc);
                }
                Coercion::IntWiden => {
                    // The same value; a `u64`, `u128` or `i128` is a kind of
                    // its own.
                    let v = self.operand(expr);
                    if let Some(k) = self.sized_int(&e.ty).filter(|k| sisu_resolve::std_sources::own_kind(k))
                    {
                        self.intrinsic_call(dest, &format!("{k}.widen"), Vec::new(), vec![v], loc);
                    } else {
                        self.assign(dest, Rvalue::Use(v), loc);
                    }
                }
                Coercion::ToCap => {
                    // The handle takes the value over, with its type for
                    // the calls through the operations trait.
                    let v = self.operand(expr);
                    let t = self.ctx.ty(&expr.ty);
                    let handle = self.ctx.ty(&e.ty);
                    self.intrinsic_call(dest, "cap.new", vec![t, handle], vec![v], loc);
                }
                Coercion::ToDyn | Coercion::RefToDyn => {
                    let v = self.operand(expr);
                    let t = self.ctx.ty(&expr.ty);
                    self.assign(dest, Rvalue::Aggregate(sir::Aggregate::Dyn(t), vec![v]), loc);
                }
                Coercion::BoxToDyn => {
                    // `Box(dyn C(b.value))`.
                    let p = self.place_of(expr);
                    let inner = match &expr.ty {
                        Ty::Adt(_, a) => a.first().cloned().unwrap_or(Ty::Error),
                        _ => Ty::Error,
                    };
                    let dyn_ty = match &e.ty {
                        Ty::Adt(_, a) => a.first().cloned().unwrap_or(Ty::Error),
                        _ => Ty::Error,
                    };
                    let t = self.temp(&dyn_ty);
                    let v = self.copy_or_move(p.project(Proj::Field(0)), &inner);
                    let ct = self.ctx.ty(&inner);
                    self.assign(Place::local(t), Rvalue::Aggregate(sir::Aggregate::Dyn(ct), vec![v]), loc);
                    let name = self.ctx.type_name(self.ctx.prog.wk.boxed.unwrap_or(sisu_resolve::DefId(0)));
                    self.assign(
                        dest,
                        Rvalue::Aggregate(sir::Aggregate::Struct(name), vec![Operand::Move(Place::local(t))]),
                        loc,
                    );
                }
            },
            ExprKind::FieldDefault { def, field } => {
                let name = self.ctx.field_default_name(*def, *field);
                self.emit(
                    StatementKind::Call { dest, callee: Callee::Fn(name, Vec::new()), args: Vec::new() },
                    loc,
                );
            }
            ExprKind::DefaultArg { def, param } => {
                let name = self.ctx.param_default_name(*def, *param);
                self.emit(
                    StatementKind::Call { dest, callee: Callee::Fn(name, Vec::new()), args: Vec::new() },
                    loc,
                );
            }
            ExprKind::CopyOf(inner) => {
                let p = self.place_of(inner);
                self.assign(dest, Rvalue::CopyOf(p), loc);
            }
            ExprKind::Trace(inner) => {
                let r = self.borrow_operand(inner, false);
                self.intrinsic_call(dest, "error.trace", Vec::new(), vec![r], loc);
            }
            ExprKind::RefineTry { value, refinement, type_args } => {
                self.refine_try(dest, value, *refinement, type_args, loc)
            }
            ExprKind::Error => self.terminate(Terminator::Unreachable, loc),
            // A function that is its value wrapper as a value (`f#value`): a
            // closure of it with nothing in its environment, for the type
            // arguments (decision 225).
            ExprKind::FnRef(d, type_args) => {
                let name = format!("{}{}", self.ctx.fn_name(*d), sir::VALUE_WRAPPER);
                let tys = type_args.iter().map(|t| self.ctx.ty(t)).collect();
                self.assign(dest, Rvalue::Closure(name, tys, Vec::new()), loc);
            }
            ExprKind::Bool(_) | ExprKind::Char(_) | ExprKind::Unit => {}
        }
    }

    /// A call of `def` with the THIR type arguments: the SIR callee, and
    /// for each argument whether the callee takes by value what the call
    /// passes by reference. A trait method whose `Self` is known here is
    /// resolved to its impl's method (or the trait's provided body); a
    /// `Self` that is a generic parameter, an associated type of one, a
    /// reference or a `dyn` is resolved at run time (`call trait`).
    pub fn callee_for(&mut self, def: sisu_resolve::DefId, type_args: &[Ty]) -> (Callee, Vec<bool>) {
        let prog = self.ctx.prog;
        let Some(f) = prog.fn_info(def) else {
            return (Callee::Fn(self.ctx.fn_name(def), Vec::new()), Vec::new());
        };
        // A template's call is a call of its instance for these arguments
        // (08, "Reflection"), which takes by value what is `Copy`.
        if f.template
            && let Some(inst) = prog.instances.get(&(def, type_args.to_vec()))
        {
            return self.direct_callee(*inst, &[]);
        }
        // A refinement is its base here: it has its base's impls (07).
        let type_args: Vec<Ty> =
            type_args.iter().map(|t| prog.normalize(&prog.erase_refinements(t), &self.bounds)).collect();
        if let Some(t) = f.trait_ {
            let n = prog.trait_info(t).map(|i| i.generics.len()).unwrap_or(0);
            let self_ty = type_args.first().cloned().unwrap_or(Ty::Error);
            if !matches!(self_ty, Ty::Param(_) | Ty::Assoc(_) | Ty::Dyn(_) | Ty::Ref(..) | Ty::ErrSet(_)) {
                let trait_args = type_args.get(1..1 + n).unwrap_or(&[]);
                let own = type_args.get(1 + n..).unwrap_or(&[]);
                if let Some((target, mut targs)) =
                    prog.resolve_method(def, trait_args, &self_ty, &self.bounds)
                {
                    // A capability handle (or a `Lent` of one) forwards to
                    // its object, whose type only the run knows.
                    if prog.fn_info(target).and_then(|f| f.intrinsic.as_deref()) == Some("cap.forward") {
                        let tys = type_args.iter().map(|t| self.ctx.ty(t)).collect();
                        return (Callee::Trait(self.ctx.fn_name(def), tys), Vec::new());
                    }
                    targs.extend(own.iter().cloned());
                    return self.direct_callee(target, &targs);
                }
            }
            let tys = type_args.iter().map(|t| self.ctx.ty(t)).collect();
            return (Callee::Trait(self.ctx.fn_name(def), tys), Vec::new());
        }
        self.direct_callee(def, &type_args)
    }

    fn direct_callee(&mut self, def: sisu_resolve::DefId, type_args: &[Ty]) -> (Callee, Vec<bool>) {
        let prog = self.ctx.prog;
        let Some(f) = prog.fn_info(def) else {
            return (Callee::Fn(self.ctx.fn_name(def), Vec::new()), Vec::new());
        };
        let fast = f.structural.as_ref().filter(|_| {
            // Every bound met by a structural impl: the host computes it.
            f.predicates.iter().all(|p| {
                let subject = prog.normalize(&p.ty.subst(type_args), &self.bounds);
                prog.is_structural(&subject, p.bound.def)
            })
        });
        if let Some(n) = f.intrinsic.as_ref().or(fast) {
            let tys = if self.needs_type_args(def) {
                type_args.iter().map(|t| self.ctx.ty(t)).collect()
            } else {
                Vec::new()
            };
            return (Callee::Intrinsic(n.clone(), tys), Vec::new());
        }
        // Which borrowed parameters the callee takes by value: `Copy` ones
        // whose declared type is not a generic parameter.
        let mut by_value = Vec::new();
        let mut slots: Vec<(Ty, sisu_syntax::ast::Mode)> = Vec::new();
        if let (Some(m), Some(o)) = (f.self_mode, &f.owner) {
            slots.push((o.clone(), m));
        }
        slots.extend(f.params.iter().map(|p| (p.ty.clone(), p.mode)));
        let env = prog.elaborate_env(&f.predicates);
        for (ty, mode) in slots {
            let local = sisu_typeck::param_local_ty(prog, &ty, mode, &env);
            let concrete = prog.normalize(&local.subst(type_args), &self.bounds);
            by_value.push(mode == sisu_syntax::ast::Mode::Borrow && !matches!(concrete, Ty::Ref(..)));
        }
        let tys = type_args.iter().map(|t| self.ctx.ty(t)).collect();
        (Callee::Fn(self.ctx.fn_name(def), tys), by_value)
    }

    /// The referent of an operand that holds a reference, copied.
    fn deref_operand(&mut self, o: Operand) -> Operand {
        match o {
            Operand::Copy(p) | Operand::Move(p) => Operand::Copy(p.project(Proj::Deref)),
            c => c,
        }
    }

    /// Calls `method` of the trait `tr` for `ty` with references `args`.
    fn trait_method_call(
        &mut self,
        dest: Place,
        tr: Option<sisu_resolve::DefId>,
        method: &str,
        ty: &Ty,
        args: Vec<Operand>,
        loc: Option<sir::Loc>,
    ) {
        let prog = self.ctx.prog;
        let Some(m) = tr.and_then(|t| prog.trait_method(t, method)) else {
            self.terminate(Terminator::Unreachable, loc);
            return;
        };
        let (callee, by_value) = self.callee_for(m, std::slice::from_ref(ty));
        let args = args
            .into_iter()
            .enumerate()
            .map(|(i, a)| if by_value.get(i).copied().unwrap_or(false) { self.deref_operand(a) } else { a })
            .collect();
        self.emit(StatementKind::Call { dest, callee, args }, loc);
    }

    /// `a == b` (or `!=`) through `Eq` for references `a` and `b`.
    fn eq_via_trait(
        &mut self,
        dest: Place,
        a: Operand,
        b: Operand,
        negate: bool,
        ty: &Ty,
        loc: Option<sir::Loc>,
    ) {
        let eq = self.ctx.prog.wk.eq;
        if negate {
            let t = self.temp_sir(sir::Type::Bool);
            self.trait_method_call(Place::local(t), eq, "eq", ty, vec![a, b], loc);
            self.assign(dest, Rvalue::Unary(sir::UnOp::Not, Operand::Copy(Place::local(t))), loc);
        } else {
            self.trait_method_call(dest, eq, "eq", ty, vec![a, b], loc);
        }
    }

    /// `a < b` and the other orderings through `PartialOrd`: the order is
    /// `Some(o)` and `o` is what the operator asks for; `None` (unordered)
    /// makes every ordering false.
    fn order_via_trait(
        &mut self,
        dest: Place,
        a: Operand,
        b: Operand,
        op: CmpOp,
        ty: &Ty,
        loc: Option<sir::Loc>,
    ) {
        let prog = self.ctx.prog;
        let ord_ty = sir::Type::Named("Ordering".into(), Vec::new());
        let o = self.temp_sir(sir::Type::Named("Option".into(), vec![ord_ty]));
        self.trait_method_call(Place::local(o), prog.wk.partial_ord, "partial_cmp", ty, vec![a, b], loc);
        let d = self.temp_sir(sir::Type::Int);
        self.assign(Place::local(d), Rvalue::Discriminant(Place::local(o)), loc);
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
        let v = self.temp_sir(sir::Type::Int);
        let payload = Place::local(o).project(Proj::Downcast(0)).project(Proj::Field(0));
        self.assign(Place::local(v), Rvalue::Discriminant(payload), loc);
        // Less is 0, Equal 1, Greater 2.
        let (bop, k) = match op {
            CmpOp::Lt => (sir::BinOp::Eq, 0),
            CmpOp::Le => (sir::BinOp::Ne, 2),
            CmpOp::Gt => (sir::BinOp::Eq, 2),
            _ => (sir::BinOp::Ne, 0),
        };
        self.assign(
            dest.clone(),
            Rvalue::Binary(bop, Operand::Copy(Place::local(v)), Operand::Const(Const::Int(k))),
            loc,
        );
        self.goto(join);
        self.switch_to(none);
        self.assign(dest, Rvalue::Use(Operand::Const(Const::Bool(false))), loc);
        self.goto(join);
        self.switch_to(join);
    }

    /// Writes `expr` to the `String` that `buf` refers to through its
    /// `Display` (or `Debug`) impl: a `Formatter` with the spec, the trait's
    /// method, and `Formatter.finish`, which pads by what is left of the
    /// spec.
    fn format_via_trait(&mut self, buf: Operand, expr: &thir::Expr, debug: bool, spec: &str) {
        let prog = self.ctx.prog;
        let loc = self.ctx.loc(expr.span);
        let Some(fdef) = prog.wk.formatter else { return };
        let fty = Ty::Adt(fdef, Vec::new());
        let f = self.temp(&fty);
        self.intrinsic_call(
            Place::local(f),
            "fmt.formatter",
            Vec::new(),
            vec![Operand::Const(Const::Str(spec.into()))],
            loc,
        );
        self.register(DropEntry::Local(f));
        self.split();
        let fr = self.temp_sir(sir::Type::Ref(Box::new(self.ctx.ty(&fty)), true));
        self.assign(Place::local(fr), Rvalue::Ref(Place::local(f), true), loc);
        let base = prog.normalize(&prog.erase_refinements(expr.ty.strip_ref()), &self.bounds);
        let v = self.ref_operand(expr);
        let (tr, method) = if debug { (prog.wk.debug, "fmt_debug") } else { (prog.wk.display, "fmt") };
        let u = self.unit();
        self.trait_method_call(u, tr, method, &base, vec![v, Operand::Copy(Place::local(fr))], loc);
        let Some(finish) = prog.method(sisu_resolve::TyKey::Adt(fdef), "finish") else { return };
        let u = self.unit();
        let callee = Callee::Fn(self.ctx.fn_name(finish), Vec::new());
        self.emit(
            StatementKind::Call { dest: u, callee, args: vec![Operand::Move(Place::local(f)), buf] },
            loc,
        );
    }

    /// `print` and `print_err` of a value whose `Display` is its own impl,
    /// and `len` of a value of a type that implements `Len` itself: through
    /// the traits. Returns whether it lowered the call.
    fn special_call(
        &mut self,
        dest: Place,
        def: sisu_resolve::DefId,
        type_args: &[Ty],
        args: &[thir::Arg],
        _e: &thir::Expr,
        loc: Option<sir::Loc>,
    ) -> bool {
        let prog = self.ctx.prog;
        let Some(intr) = prog.fn_info(def).and_then(|f| f.intrinsic.clone()) else { return false };
        let Some(arg) = args.first() else { return false };
        let first = prog.erase_refinements(type_args.first().unwrap_or(&Ty::Error));
        let base = prog.normalize(first.strip_ref(), &self.bounds);
        match intr.as_str() {
            "print" | "print_err" => {
                if prog.wk.display.is_none_or(|d| prog.is_structural(&base, d)) {
                    return false;
                }
                let sty = Ty::Adt(prog.wk.string.unwrap_or(def), Vec::new());
                let s = self.temp(&sty);
                self.intrinsic_call(Place::local(s), "fmt.new", Vec::new(), Vec::new(), loc);
                self.register(DropEntry::Local(s));
                self.split();
                let r = self.temp_sir(sir::Type::Ref(Box::new(self.ctx.ty(&sty)), true));
                self.assign(Place::local(r), Rvalue::Ref(Place::local(s), true), loc);
                self.format_via_trait(Operand::Copy(Place::local(r)), &arg.expr, false, "");
                self.intrinsic_call(dest, &intr, Vec::new(), vec![Operand::Move(Place::local(s))], loc);
                true
            }
            "len" => {
                let builtin = match &base {
                    Ty::Str => true,
                    Ty::Adt(d, _) => {
                        matches!(prog.builtin(*d), Some("String" | "list" | "Array" | "dict" | "set"))
                    }
                    _ => false,
                };
                if builtin {
                    return false;
                }
                let v = self.ref_operand(&arg.expr);
                self.trait_method_call(dest, prog.wk.len, "len", &base, vec![v], loc);
                true
            }
            _ => false,
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
                    let mark = self.locals.len();
                    let k = self.operand(c.elem);
                    let mut held = Vec::new();
                    if c.value.is_some_and(|v| self.may_leave(v)) {
                        self.hold(&k, mark, &c.elem.ty, &mut held);
                    }
                    let v = match c.value {
                        Some(v) => self.operand(v),
                        None => Operand::Const(Const::Unit),
                    };
                    self.release(&held);
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
