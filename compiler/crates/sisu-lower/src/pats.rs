//! Pattern tests (which branch to take) and bindings (what the arm sees).
//! Testing happens first and binds nothing, so a failed test leaves the
//! scrutinee as it was.

use crate::func::FnLower;
use sisu_resolve::Ty;
use sisu_sir as sir;
use sisu_sir::{BlockId, Const, Operand, Place, Proj, Rvalue, StatementKind, Terminator};
use sisu_typeck::thir::{self, BindMode, Lit, PatKind};

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
                    Lit::Float(f) => Const::Float(*f),
                    Lit::Bool(b) => Const::Bool(*b),
                    Lit::Char(c) => Const::Char(*c),
                    Lit::Str(s) => Const::Str(s.clone()),
                };
                let lhs = if self.ctx.prog.is_copy(&pat.ty) {
                    Operand::Copy(place.clone())
                } else {
                    let t = self.temp_sir(sir::Type::Ref(Box::new(self.ctx.ty(&pat.ty)), false));
                    self.assign(Place::local(t), Rvalue::Ref(place.clone(), false), loc);
                    Operand::Copy(Place::local(t))
                };
                let r = self.temp_sir(sir::Type::Bool);
                self.assign(Place::local(r), Rvalue::Binary(sir::BinOp::Eq, lhs, Operand::Const(c)), loc);
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
        let loc = self.ctx.loc(pat.span);
        match &pat.kind {
            PatKind::Bind { local, mode } => {
                let l = self.local_for(*local);
                match mode {
                    BindMode::Value => {
                        let v = self.copy_or_move(place.clone(), &pat.ty);
                        self.assign(Place::local(l), Rvalue::Use(v), loc);
                        let ty = pat.ty.clone();
                        self.bind_registered(*local, l, &ty, below);
                    }
                    BindMode::Ref => {
                        self.assign(Place::local(l), Rvalue::Ref(place.clone(), false), loc);
                        self.scope_local(below, l);
                    }
                    BindMode::RefMut => {
                        self.assign(Place::local(l), Rvalue::Ref(place.clone(), true), loc);
                        self.scope_local(below, l);
                    }
                }
            }
            PatKind::Variant { variant, fields, .. } => {
                for (i, f) in fields.iter().enumerate() {
                    let sub = place.project(Proj::Downcast(*variant)).project(Proj::Field(i as u32));
                    self.bind_below(f, &sub, below);
                }
            }
            PatKind::Struct { fields, .. } | PatKind::Tuple(fields) => {
                for (i, f) in fields.iter().enumerate() {
                    self.bind_below(f, &place.project(Proj::Field(i as u32)), below);
                }
            }
            PatKind::Deref(inner) => self.bind_below(inner, &place.project(Proj::Deref), below),
            PatKind::List { prefix, rest, suffix } => {
                for (i, p) in prefix.iter().enumerate() {
                    let sub = self.element(place, i, false);
                    self.bind_below(p, &sub, below);
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
                    self.bind_registered(*r, l, &ty, below);
                }
                for (j, p) in suffix.iter().enumerate() {
                    let sub = self.element(place, suffix.len() - j, true);
                    self.bind_below(p, &sub, below);
                }
            }
            PatKind::Wild | PatKind::Lit(_) | PatKind::Or(_) | PatKind::Error => {}
        }
    }
}
