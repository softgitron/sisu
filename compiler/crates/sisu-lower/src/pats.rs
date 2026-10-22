//! Pattern tests (which branch to take) and bindings (what the arm sees).
//! Testing happens first and binds nothing, so a failed test leaves the
//! scrutinee as it was.

use crate::func::FnLower;
use sisu_resolve::Ty;
use sisu_sir as sir;
use sisu_sir::{BlockId, Const, Operand, Place, Proj, Rvalue, StatementKind, Terminator};
use sisu_typeck::thir::{self, BindMode, Lit, PatKind};

/// The variants `pat` tests for, and the place whose discriminant
/// decides, when that is all the pattern tests: a variant whose fields
/// cannot fail, or a choice of such variants of one place.
pub fn variant_set(pat: &thir::Pat, place: &Place) -> Option<(Place, Vec<i64>)> {
    match &pat.kind {
        PatKind::Variant { variant, fields, .. } => {
            // A union of error sets tests which set first (`test`).
            if matches!(&pat.ty, Ty::ErrSet(ms) if ms.len() > 1) || !fields.iter().all(cannot_fail) {
                return None;
            }
            Some((place.clone(), vec![*variant as i64]))
        }
        PatKind::Or(alts) => {
            let mut out: Option<(Place, Vec<i64>)> = None;
            for alt in alts {
                let (q, vs) = variant_set(alt, place)?;
                match &mut out {
                    None => out = Some((q, vs)),
                    Some((o, ws)) if *o == q => ws.extend(vs),
                    Some(_) => return None,
                }
            }
            out
        }
        PatKind::Deref(inner) => variant_set(inner, &place.project(Proj::Deref)),
        _ => None,
    }
}

/// Whether a pattern matches every value of its type, so that testing it
/// emits nothing.
fn cannot_fail(pat: &thir::Pat) -> bool {
    match &pat.kind {
        PatKind::Wild | PatKind::Bind { .. } | PatKind::Error => true,
        PatKind::Struct { fields, .. } | PatKind::Tuple(fields) => fields.iter().all(cannot_fail),
        PatKind::Deref(inner) => cannot_fail(inner),
        PatKind::Lit(_) | PatKind::Variant { .. } | PatKind::Or(_) | PatKind::List { .. } => false,
    }
}

impl FnLower<'_, '_> {
    /// Continues in the current block when `pat` matches the value at
    /// `place`, and jumps to `fail` otherwise.
    pub fn test(&mut self, pat: &thir::Pat, place: &Place, fail: BlockId) {
        let loc = self.ctx.loc(pat.span);
        match &pat.kind {
            PatKind::Wild | PatKind::Bind { .. } | PatKind::Error => {}
            PatKind::Lit(l) => {
                let c = match l {
                    Lit::Int(n) => Const::Int(*n),
                    Lit::WideInt(_) => Const::Int(0),
                    Lit::Float(f) => Const::Float(*f),
                    Lit::Bool(b) => Const::Bool(*b),
                    Lit::Char(c) => Const::Char(*c),
                    Lit::Str(s) => Const::Str(s.clone()),
                };
                // A literal of a `u64`, `u128`, `i128` or `f32` is a value
                // an intrinsic makes, as in an expression.
                let made = self.sized_int(&pat.ty).is_some_and(sisu_resolve::std_sources::own_kind)
                    || self.is_f32(&pat.ty);
                let c = match made {
                    true => {
                        let kind = match l {
                            Lit::WideInt(b) => thir::ExprKind::WideInt(*b),
                            Lit::Float(f) => thir::ExprKind::Float(*f),
                            _ => thir::ExprKind::Int(match c {
                                Const::Int(n) => n,
                                _ => 0,
                            }),
                        };
                        let lit = thir::Expr::new(kind, pat.ty.clone(), pat.span);
                        let t = self.temp(&pat.ty);
                        self.into(Place::local(t), &lit);
                        Operand::Copy(Place::local(t))
                    }
                    false => Operand::Const(c),
                };
                let lhs = if self.ctx.prog.is_copy_in(&pat.ty, &self.bounds) {
                    Operand::Copy(place.clone())
                } else {
                    let t = self.temp_sir(sir::Type::Ref(Box::new(self.ctx.ty(&pat.ty)), false));
                    self.assign(Place::local(t), Rvalue::Ref(place.clone(), false), loc);
                    Operand::Copy(Place::local(t))
                };
                let r = self.temp_sir(sir::Type::Bool);
                self.assign(Place::local(r), Rvalue::Binary(sir::BinOp::Eq, lhs, c), loc);
                let ok = self.new_block();
                self.terminate(
                    Terminator::Switch {
                        value: Operand::Copy(Place::local(r)),
                        cases: vec![(0, fail)],
                        default: ok,
                    },
                    loc,
                );
                self.switch_to(ok);
            }
            PatKind::Variant { def, variant, fields } => {
                // A value of a union of error sets is a variant of one of
                // them: test which before testing the variant index.
                if matches!(&pat.ty, Ty::ErrSet(ms) if ms.len() > 1) {
                    let r = self.temp_sir(sir::Type::Ref(Box::new(self.ctx.ty(&pat.ty)), false));
                    self.assign(Place::local(r), Rvalue::Ref(place.clone(), false), loc);
                    let is = self.temp_sir(sir::Type::Bool);
                    let decl = sir::Type::Named(self.ctx.type_name(*def), Vec::new());
                    self.emit(
                        StatementKind::Call {
                            dest: Place::local(is),
                            callee: sir::Callee::Intrinsic("error.is".into(), vec![decl]),
                            args: vec![Operand::Copy(Place::local(r))],
                        },
                        loc,
                    );
                    let ok = self.new_block();
                    self.terminate(
                        Terminator::Switch {
                            value: Operand::Copy(Place::local(is)),
                            cases: vec![(0, fail)],
                            default: ok,
                        },
                        loc,
                    );
                    self.switch_to(ok);
                }
                let d = self.temp_sir(sir::Type::Int);
                self.assign(Place::local(d), Rvalue::Discriminant(place.clone()), loc);
                let ok = self.new_block();
                self.terminate(
                    Terminator::Switch {
                        value: Operand::Copy(Place::local(d)),
                        cases: vec![(*variant as i64, ok)],
                        default: fail,
                    },
                    loc,
                );
                self.switch_to(ok);
                for (i, f) in fields.iter().enumerate() {
                    let sub = place.project(Proj::Downcast(*variant)).project(Proj::Field(i as u32));
                    self.test(f, &sub, fail);
                }
            }
            PatKind::Struct { fields, .. } | PatKind::Tuple(fields) => {
                for (i, f) in fields.iter().enumerate() {
                    self.test(f, &place.project(Proj::Field(i as u32)), fail);
                }
            }
            PatKind::Deref(inner) => self.test(inner, &place.project(Proj::Deref), fail),
            PatKind::Or(alts) => {
                let cont = self.new_block();
                for (i, alt) in alts.iter().enumerate() {
                    if i + 1 == alts.len() {
                        self.test(alt, place, fail);
                        self.goto(cont);
                    } else {
                        let next = self.new_block();
                        self.test(alt, place, next);
                        self.goto(cont);
                        self.switch_to(next);
                    }
                }
                self.switch_to(cont);
            }
            PatKind::List { prefix, rest, suffix } => {
                let n = self.temp_sir(sir::Type::Int);
                let r = self.temp_sir(sir::Type::Ref(Box::new(self.ctx.ty(&pat.ty)), false));
                self.assign(Place::local(r), Rvalue::Ref(place.clone(), false), loc);
                self.emit(
                    StatementKind::Call {
                        dest: Place::local(n),
                        callee: sir::Callee::Intrinsic("len".into(), Vec::new()),
                        args: vec![Operand::Copy(Place::local(r))],
                    },
                    loc,
                );
                let need = (prefix.len() + suffix.len()) as i64;
                let op = if rest.is_some() { sir::BinOp::Ge } else { sir::BinOp::Eq };
                let c = self.temp_sir(sir::Type::Bool);
                self.assign(
                    Place::local(c),
                    Rvalue::Binary(op, Operand::Copy(Place::local(n)), Operand::Const(Const::Int(need))),
                    loc,
                );
                let ok = self.new_block();
                self.terminate(
                    Terminator::Switch {
                        value: Operand::Copy(Place::local(c)),
                        cases: vec![(0, fail)],
                        default: ok,
                    },
                    loc,
                );
                self.switch_to(ok);
                for (i, p) in prefix.iter().enumerate() {
                    let sub = self.element(place, i, false);
                    self.test(p, &sub, fail);
                }
                for (j, p) in suffix.iter().enumerate() {
                    let sub = self.element(place, suffix.len() - j, true);
                    self.test(p, &sub, fail);
                }
            }
        }
    }

    fn element(&mut self, place: &Place, i: usize, from_end: bool) -> Place {
        let idx = self.temp_sir(sir::Type::Int);
        self.assign(Place::local(idx), Rvalue::Use(Operand::Const(Const::Int(i as i64))), None);
        place.project(if from_end { Proj::IndexFromEnd(idx) } else { Proj::Index(idx) })
    }

    /// Binds the names of a pattern that matched; value bindings are
    /// dropped with the innermost scope.
    pub fn bind(&mut self, pat: &thir::Pat, place: &Place) {
        self.bind_below(pat, place, 0);
    }

    /// Binds, registering drops in the scope `below` levels under the
    /// innermost (a `let` registers in its block, not its statement).
    pub fn bind_below(&mut self, pat: &thir::Pat, place: &Place, below: usize) {
        self.bind_with(pat, place, Some(below));
    }

    /// Assigns a pattern's bindings without registering them, for a
    /// binding made on several paths and registered once where they join
    /// ([`FnLower::register_bindings`]): the alternatives of `|`, and the
    /// two paths of `let PATTERN = e else: fallback`.
    pub fn bind_values(&mut self, pat: &thir::Pat, place: &Place) {
        self.bind_with(pat, place, None);
    }

    /// Registers the bindings of a pattern that [`FnLower::bind_values`]
    /// assigned, in the scope `below` levels under the innermost.
    pub fn register_bindings(&mut self, pat: &thir::Pat, below: usize) {
        match &pat.kind {
            PatKind::Bind { local, mode } => {
                let l = self.local_for(*local);
                match mode {
                    BindMode::Value => {
                        let ty = pat.ty.clone();
                        self.bind_registered(*local, l, &ty, below);
                    }
                    BindMode::Ref | BindMode::RefMut => self.scope_local(below, l),
                }
            }
            PatKind::Variant { fields, .. } | PatKind::Struct { fields, .. } | PatKind::Tuple(fields) => {
                for f in fields {
                    self.register_bindings(f, below);
                }
            }
            PatKind::Deref(inner) => self.register_bindings(inner, below),
            PatKind::Or(alts) => {
                if let Some(first) = alts.first() {
                    self.register_bindings(first, below);
                }
            }
            PatKind::List { prefix, rest, suffix } => {
                for p in prefix {
                    self.register_bindings(p, below);
                }
                if let Some(Some(r)) = rest {
                    let l = self.local_for(*r);
                    let ty = self.body.locals[r.0 as usize].ty.clone();
                    self.bind_registered(*r, l, &ty, below);
                }
                for p in suffix {
                    self.register_bindings(p, below);
                }
            }
            PatKind::Wild | PatKind::Lit(_) | PatKind::Error => {}
        }
    }

    /// Binds a value binding, registering it in the scope `below` levels
    /// down, or, when `reg` is `None`, ending the block after a value that
    /// needs drop (the lowering invariant) and leaving its registration to
    /// the caller.
    fn bind_value(&mut self, local: thir::LocalId, l: sir::LocalId, ty: &Ty, reg: Option<usize>) {
        match reg {
            Some(below) => self.bind_registered(local, l, ty, below),
            None => {
                if self.needs_drop(ty) {
                    self.split();
                }
            }
        }
    }

    fn bind_with(&mut self, pat: &thir::Pat, place: &Place, reg: Option<usize>) {
        let loc = self.ctx.loc(pat.span);
        match &pat.kind {
            PatKind::Bind { local, mode } => {
                let l = self.local_for(*local);
                match mode {
                    BindMode::Value => {
                        let v = self.copy_or_move(place.clone(), &pat.ty);
                        self.assign(Place::local(l), Rvalue::Use(v), loc);
                        let ty = pat.ty.clone();
                        self.bind_value(*local, l, &ty, reg);
                    }
                    BindMode::Ref => {
                        self.assign(Place::local(l), Rvalue::Ref(place.clone(), false), loc);
                        if let Some(below) = reg {
                            self.scope_local(below, l);
                        }
                    }
                    BindMode::RefMut => {
                        self.assign(Place::local(l), Rvalue::Ref(place.clone(), true), loc);
                        if let Some(below) = reg {
                            self.scope_local(below, l);
                        }
                    }
                }
            }
            PatKind::Variant { variant, fields, .. } => {
                for (i, f) in fields.iter().enumerate() {
                    let sub = place.project(Proj::Downcast(*variant)).project(Proj::Field(i as u32));
                    self.bind_with(f, &sub, reg);
                }
            }
            PatKind::Struct { fields, .. } | PatKind::Tuple(fields) => {
                for (i, f) in fields.iter().enumerate() {
                    self.bind_with(f, &place.project(Proj::Field(i as u32)), reg);
                }
            }
            PatKind::Deref(inner) => self.bind_with(inner, &place.project(Proj::Deref), reg),
            // The alternatives bind the same locals: test again which one
            // matched (testing binds nothing and changes nothing) and bind
            // from it; the bindings are registered once, after the join.
            PatKind::Or(alts) if !sisu_typeck::pattern_bindings(pat).is_empty() => {
                let done = self.new_block();
                for (i, alt) in alts.iter().enumerate() {
                    if i + 1 < alts.len() {
                        let next = self.new_block();
                        self.test(alt, place, next);
                        self.bind_values(alt, place);
                        self.goto(done);
                        self.switch_to(next);
                    } else {
                        self.bind_values(alt, place);
                        self.goto(done);
                    }
                }
                self.switch_to(done);
                if let (Some(below), Some(first)) = (reg, alts.first()) {
                    self.register_bindings(first, below);
                }
            }
            PatKind::List { prefix, rest, suffix } => {
                for (i, p) in prefix.iter().enumerate() {
                    let sub = self.element(place, i, false);
                    self.bind_with(p, &sub, reg);
                }
                if let Some(Some(r)) = rest {
                    let l = self.local_for(*r);
                    let refp = self.temp_sir(sir::Type::Ref(Box::new(self.ctx.ty(&pat.ty)), false));
                    self.assign(Place::local(refp), Rvalue::Ref(place.clone(), false), loc);
                    self.emit(
                        StatementKind::Call {
                            dest: Place::local(l),
                            callee: sir::Callee::Intrinsic("list.slice_middle".into(), Vec::new()),
                            args: vec![
                                Operand::Copy(Place::local(refp)),
                                Operand::Const(Const::Int(prefix.len() as i64)),
                                Operand::Const(Const::Int(suffix.len() as i64)),
                            ],
                        },
                        loc,
                    );
                    let ty = self.body.locals[r.0 as usize].ty.clone();
                    self.bind_value(*r, l, &ty, reg);
                }
                for (j, p) in suffix.iter().enumerate() {
                    let sub = self.element(place, suffix.len() - j, true);
                    self.bind_with(p, &sub, reg);
                }
            }
            PatKind::Wild | PatKind::Lit(_) | PatKind::Or(_) | PatKind::Error => {}
        }
    }
}
