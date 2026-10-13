//! Statements: blocks, `let`, assignment, `return`, loops, `if`, `match`,
//! `assert`, `defer`.

use crate::func::{DropEntry, FnLower, LoopCtx};
use sisu_resolve::Ty;
use sisu_sir as sir;
use sisu_sir::{BlockId, Operand, Place, Rvalue, StatementKind, Terminator};
use sisu_typeck::thir::{self, ArmBody, CondPart, ExprKind, ForMode, LetElse, PatKind, Stmt};

impl FnLower<'_, '_> {
    /// A block in its own scope.
    pub fn block(&mut self, b: &thir::Block) {
        self.push_scope();
        self.block_stmts(b);
        self.pop_scope();
    }

    pub fn block_stmts(&mut self, b: &thir::Block) {
        for s in &b.stmts {
            self.stmt(s);
        }
    }

    fn stmt(&mut self, s: &Stmt) {
        // Temporaries of a statement live until its end.
        self.push_scope();
        match s {
            Stmt::Let { pat, init, unwrap, else_, span } => {
                self.let_stmt(pat, init.as_ref(), *unwrap, else_.as_ref(), *span)
            }
            Stmt::Assign { place, value, span } => {
                let loc = self.ctx.loc(*span);
                let v = self.operand(value);
                let p = self.place_of(place);
                self.overwrite(place, p, v, loc);
            }
            Stmt::AssignTuple { places, value, span } => {
                let loc = self.ctx.loc(*span);
                let t = self.temp(&value.ty);
                self.into(Place::local(t), value);
                for (i, pe) in places.iter().enumerate() {
                    let src = Place::local(t).project(sir::Proj::Field(i as u32));
                    let v = self.copy_or_move(src, &pe.ty);
                    let p = self.place_of(pe);
                    self.overwrite(pe, p, v, loc);
                }
            }
            Stmt::Discard { value, .. } => {
                let t = self.temp(&value.ty);
                self.into(Place::local(t), value);
                if self.needs_drop(&value.ty) {
                    self.register(DropEntry::Local(t, None));
                    self.split();
                }
            }
            Stmt::AugAssign { place, op, value, span } => self.aug_assign(place, *op, value, *span),
            Stmt::Expr(e) => {
                let t = self.temp(&e.ty);
                self.into(Place::local(t), e);
                if self.needs_drop(&e.ty) && e.ty != Ty::Never {
                    self.register(DropEntry::Local(t, None));
                    self.split();
                }
            }
            Stmt::Return { value, span } => {
                let loc = self.ctx.loc(*span);
                match value {
                    Some(v) => self.into(Place::local(sir::LocalId(0)), v),
                    None => self.assign(
                        Place::local(sir::LocalId(0)),
                        Rvalue::Use(Operand::Const(sir::Const::Unit)),
                        loc,
                    ),
                }
                self.emit_exit(0);
                self.terminate(Terminator::Return, loc);
            }
            Stmt::Break(_) | Stmt::Continue(_) => {
                if let Some(l) = self.loops.last() {
                    let (target, depth) = if matches!(s, Stmt::Break(_)) {
                        (l.break_bb, l.depth)
                    } else {
                        (l.continue_bb, l.depth)
                    };
                    self.emit_exit(depth);
                    self.goto(target);
                }
            }
            Stmt::If { branches, else_, .. } => {
                let join = self.new_block();
                for (cond, block) in branches {
                    let next = self.new_block();
                    let depth = self.scopes.len();
                    self.push_scope();
                    self.cond(cond, next, depth);
                    self.block(block);
                    self.pop_scope();
                    self.goto(join);
                    self.switch_to(next);
                }
                if let Some(b) = else_ {
                    self.block(b);
                }
                self.goto(join);
                self.switch_to(join);
            }
            Stmt::While { cond, body, .. } => {
                let head = self.new_block();
                let exit = self.new_block();
                self.goto(head);
                self.switch_to(head);
                let depth = self.scopes.len();
                self.push_scope();
                let fail = self.edge_to(exit, depth);
                self.cond(cond, fail, depth);
                self.loops.push(LoopCtx { break_bb: exit, continue_bb: head, depth });
                self.block(body);
                self.loops.pop();
                self.pop_scope();
                self.goto(head);
                self.switch_to(exit);
            }
            Stmt::For { pat, iter, mode, item, body, span } => {
                self.for_loop(pat, iter, *mode, item, ForBody::Block(body), *span)
            }
            Stmt::Match { scrutinee, arms, span } => self.match_into(None, scrutinee, arms, *span),
            Stmt::Assert { cond, message, text, span } => self.assert(cond, message.as_ref(), text, *span),
            Stmt::Defer { body, .. } => {
                let i = self.defers.len();
                self.defers.push(body.clone());
                self.register_below(1, DropEntry::Defer(i));
                self.split();
            }
            Stmt::Block(b) => self.block(b),
            Stmt::Error(_) => self.terminate(Terminator::Unreachable, None),
        }
        self.pop_scope();
    }

    /// Stores `v` into the place `p` of the expression `pe`, dropping the
    /// value it held.
    fn overwrite(&mut self, pe: &thir::Expr, p: Place, v: Operand, loc: Option<sir::Loc>) {
        let whole = match &pe.kind {
            ExprKind::Local(l) if !self.alias.contains_key(l) && !self.env.contains_key(l) => Some(*l),
            _ => None,
        };
        if self.needs_drop(&pe.ty) {
            let flag = whole.and_then(|l| self.flags.get(&l).copied());
            self.split();
            self.emit(StatementKind::Drop { place: p.clone(), flag }, loc);
            self.split();
        }
        self.assign(p, Rvalue::Use(v), loc);
        if let Some(l) = whole
            && let Some(&f) = self.flags.get(&l)
        {
            self.assign(Place::local(f), Rvalue::Use(Operand::Const(sir::Const::Bool(true))), None);
        }
    }

    fn aug_assign(
        &mut self,
        place: &thir::Expr,
        op: sisu_syntax::ast::BinOp,
        value: &thir::Expr,
        span: sisu_diag::Span,
    ) {
        let loc = self.ctx.loc(span);
        let is_string = matches!(&place.ty, Ty::Adt(d, _) if self.ctx.prog.builtin(*d) == Some("String"));
        if is_string {
            let p = self.place_of(place);
            let r = self.temp_sir(sir::Type::Ref(Box::new(self.ctx.ty(&place.ty)), true));
            self.assign(Place::local(r), Rvalue::Ref(p, true), loc);
            let v = self.str_operand(value);
            let u = self.unit();
            self.emit(
                StatementKind::Call {
                    dest: u,
                    callee: sir::Callee::Intrinsic("String.push".into(), Vec::new()),
                    args: vec![Operand::Copy(Place::local(r)), v],
                },
                loc,
            );
            return;
        }
        let v = self.operand(value);
        let p = self.place_of(place);
        let t = self.temp(&place.ty);
        let bop = crate::exprs::bin_op(op, &place.ty, self.ctx.prog);
        self.assign(Place::local(t), Rvalue::Binary(bop, Operand::Copy(p.clone()), v), loc);
        self.assign(p, Rvalue::Use(Operand::Move(Place::local(t))), loc);
    }

    fn let_stmt(
        &mut self,
        pat: &thir::Pat,
        init: Option<&thir::Expr>,
        unwrap: Option<thir::Unwrap>,
        else_: Option<&LetElse>,
        span: sisu_diag::Span,
    ) {
        let loc = self.ctx.loc(span);
        let Some(init) = init else {
            // `let x: T`: the local exists from here, assigned later.
            if let PatKind::Bind { local, .. } = &pat.kind {
                let l = self.local_for(*local);
                if self.needs_drop(&pat.ty) {
                    let f = self.add_local(sir::Type::Bool, Some("flag".into()));
                    self.flags.insert(*local, f);
                    self.assign(Place::local(f), Rvalue::Use(Operand::Const(sir::Const::Bool(false))), loc);
                    self.register_below(1, DropEntry::Local(l, Some(f)));
                    self.split();
                }
            }
            return;
        };
        match (unwrap, else_) {
            (None, None) => {
                if let PatKind::Bind { local, mode: thir::BindMode::Value } = &pat.kind {
                    let l = self.local_for(*local);
                    self.into(Place::local(l), init);
                    self.bind_registered(*local, l, &pat.ty, 1);
                    return;
                }
                let p = self.place_of(init);
                self.bind_below(pat, &p, 1);
            }
            (Some(_), Some(e)) => {
                // `let x = e else ...` on an option or a result.
                let t = self.place_of(init);
                let d = self.temp_sir(sir::Type::Int);
                self.assign(Place::local(d), Rvalue::Discriminant(t.clone()), loc);
                let ok = self.new_block();
                let other = self.new_block();
                let join = self.new_block();
                self.terminate(
                    Terminator::Switch {
                        value: Operand::Copy(Place::local(d)),
                        cases: vec![(0, ok)],
                        default: other,
                    },
                    loc,
                );
                // The bindings exist on both paths.
                let binding = match &pat.kind {
                    PatKind::Bind { local, .. } => Some((*local, self.local_for(*local))),
                    _ => None,
                };
                self.switch_to(ok);
                let payload = t.project(sir::Proj::Downcast(0)).project(sir::Proj::Field(0));
                match binding {
                    Some((_, l)) => {
                        let v = self.copy_or_move(payload, &pat.ty);
                        self.assign(Place::local(l), Rvalue::Use(v), loc);
                    }
                    None => self.bind(pat, &payload),
                }
                self.goto(join);
                self.switch_to(other);
                match e {
                    LetElse::Fallback { binding: eb, value } => {
                        self.push_scope();
                        if let Some(b) = eb {
                            let bl = self.local_for(*b);
                            if unwrap == Some(thir::Unwrap::Result) {
                                let src = t.project(sir::Proj::Downcast(1)).project(sir::Proj::Field(0));
                                let ety = self.body.locals[b.0 as usize].ty.clone();
                                let v = self.copy_or_move(src, &ety);
                                self.assign(Place::local(bl), Rvalue::Use(v), loc);
                                self.bind_registered(*b, bl, &ety, 0);
                            }
                        }
                        match binding {
                            Some((_, l)) => self.into(Place::local(l), value),
                            None => {
                                let p = self.place_of(value);
                                self.bind(pat, &p);
                            }
                        }
                        self.pop_scope();
                        self.goto(join);
                    }
                    LetElse::Diverge { binding: eb, body } => {
                        self.push_scope();
                        if let Some(b) = eb {
                            let bl = self.local_for(*b);
                            if unwrap == Some(thir::Unwrap::Result) {
                                let src = t.project(sir::Proj::Downcast(1)).project(sir::Proj::Field(0));
                                let ety = self.body.locals[b.0 as usize].ty.clone();
                                let v = self.copy_or_move(src, &ety);
                                self.assign(Place::local(bl), Rvalue::Use(v), loc);
                                self.bind_registered(*b, bl, &ety, 0);
                            }
                        }
                        self.block(body);
                        self.pop_scope();
                        self.terminate(Terminator::Unreachable, None);
                    }
                }
                self.switch_to(join);
                if let Some((tl, l)) = binding {
                    self.bind_registered(tl, l, &pat.ty, 1);
                }
            }
            (None, Some(e)) => {
                // `let Pattern = e else: diverge`.
                let t = self.place_of(init);
                let fail = self.new_block();
                let join = self.new_block();
                self.test(pat, &t, fail);
                self.bind_below(pat, &t, 1);
                self.goto(join);
                self.switch_to(fail);
                match e {
                    LetElse::Diverge { body, .. } => self.block(body),
                    LetElse::Fallback { value, .. } => {
                        let tmp = self.temp(&value.ty);
                        self.into(Place::local(tmp), value);
                    }
                }
                self.terminate(Terminator::Unreachable, None);
                self.switch_to(join);
            }
            (Some(_), None) => {
                let p = self.place_of(init);
                self.bind_below(pat, &p, 1);
            }
        }
    }

    /// Registers a value binding in the scope `below` levels down.
    pub fn bind_registered(&mut self, tl: thir::LocalId, l: sir::LocalId, ty: &Ty, below: usize) {
        if self.needs_drop(ty) {
            let flag = self.flag_init(tl);
            self.register_below(below, DropEntry::Local(l, flag));
            self.split();
        }
    }

    /// A condition chain: on failure, jumps to `fail` (dropping what the
    /// chain made live, down to `depth`).
    pub fn cond(&mut self, c: &thir::Cond, fail: BlockId, depth: usize) {
        for part in &c.parts {
            match part {
                CondPart::Bool(e) => {
                    let v = self.operand(e);
                    let ok = self.new_block();
                    let f = self.edge_to(fail, depth);
                    self.terminate(
                        Terminator::Switch { value: v, cases: vec![(0, f)], default: ok },
                        self.ctx.loc(e.span),
                    );
                    self.switch_to(ok);
                }
                CondPart::Let { pat, value } => {
                    let p = self.place_of(value);
                    let f = self.edge_to(fail, depth);
                    self.test(pat, &p, f);
                    self.bind(pat, &p);
                }
            }
        }
    }

    pub fn for_loop(
        &mut self,
        pat: &thir::Pat,
        iter: &thir::Expr,
        mode: ForMode,
        item: &Ty,
        body: ForBody,
        span: sisu_diag::Span,
    ) {
        let loc = self.ctx.loc(span);
        self.push_scope();
        let iter_ty = sir::Type::Named("Iter".into(), vec![self.ctx.ty(item)]);
        let it = self.temp_sir(iter_ty.clone());
        match mode {
            ForMode::Borrow | ForMode::Mut => {
                let r = self.borrow_operand(iter, mode == ForMode::Mut);
                let name = if mode == ForMode::Mut { "iter.mut" } else { "iter.ref" };
                self.emit(
                    StatementKind::Call {
                        dest: Place::local(it),
                        callee: sir::Callee::Intrinsic(name.into(), Vec::new()),
                        args: vec![r],
                    },
                    loc,
                );
            }
            ForMode::Own => {
                let v = self.operand(iter);
                self.emit(
                    StatementKind::Call {
                        dest: Place::local(it),
                        callee: sir::Callee::Intrinsic("iter.own".into(), Vec::new()),
                        args: vec![v],
                    },
                    loc,
                );
            }
            ForMode::Iter => self.into(Place::local(it), iter),
        }
        self.register(DropEntry::Local(it, None));
        self.split();
        let head = self.new_block();
        let exit = self.new_block();
        self.goto(head);
        self.switch_to(head);
        let depth = self.scopes.len();
        self.push_scope();
        let r = self.temp_sir(sir::Type::Ref(Box::new(iter_ty), true));
        self.assign(Place::local(r), Rvalue::Ref(Place::local(it), true), loc);
        let opt_ty = sir::Type::Named("Option".into(), vec![self.ctx.ty(item)]);
        let next = self.temp_sir(opt_ty);
        self.emit(
            StatementKind::Call {
                dest: Place::local(next),
                callee: sir::Callee::Intrinsic("Iter.next".into(), Vec::new()),
                args: vec![Operand::Copy(Place::local(r))],
            },
            loc,
        );
        if self.needs_drop(item) {
            self.register(DropEntry::Local(next, None));
            self.split();
        }
        let d = self.temp_sir(sir::Type::Int);
        self.assign(Place::local(d), Rvalue::Discriminant(Place::local(next)), loc);
        let body_bb = self.new_block();
        let done = self.edge_to(exit, depth);
        self.terminate(
            Terminator::Switch {
                value: Operand::Copy(Place::local(d)),
                cases: vec![(0, body_bb)],
                default: done,
            },
            loc,
        );
        self.switch_to(body_bb);
        let payload = Place::local(next).project(sir::Proj::Downcast(0)).project(sir::Proj::Field(0));
        self.bind(pat, &payload);
        self.loops.push(LoopCtx { break_bb: exit, continue_bb: head, depth });
        match body {
            ForBody::Block(b) => self.block(b),
            ForBody::Comprehension(c) => self.comprehension_rest(*c),
        }
        self.loops.pop();
        self.pop_scope();
        self.goto(head);
        self.switch_to(exit);
        self.pop_scope();
    }

    /// `match` as a statement (`dest` is `None`) or as an expression.
    pub fn match_into(
        &mut self,
        dest: Option<Place>,
        scrutinee: &thir::Expr,
        arms: &[thir::Arm],
        span: sisu_diag::Span,
    ) {
        let loc = self.ctx.loc(span);
        self.push_scope();
        let p = self.place_of(scrutinee);
        let join = self.new_block();
        for arm in arms {
            let next = self.new_block();
            let depth = self.scopes.len();
            self.push_scope();
            self.test(&arm.pat, &p, next);
            self.bind(&arm.pat, &p);
            if let Some(g) = &arm.guard {
                let v = self.operand(g);
                let ok = self.new_block();
                let f = self.edge_to(next, depth);
                self.terminate(
                    Terminator::Switch { value: v, cases: vec![(0, f)], default: ok },
                    self.ctx.loc(g.span),
                );
                self.switch_to(ok);
            }
            match (&arm.body, &dest) {
                (ArmBody::Expr(e), Some(d)) => self.into(d.clone(), e),
                (ArmBody::Expr(e), None) => {
                    let t = self.temp(&e.ty);
                    self.into(Place::local(t), e);
                    if self.needs_drop(&e.ty) {
                        self.register(DropEntry::Local(t, None));
                        self.split();
                    }
                }
                (ArmBody::Block(b), _) => self.block(b),
            }
            self.pop_scope();
            self.goto(join);
            self.switch_to(next);
        }
        self.terminate(Terminator::Unreachable, loc);
        self.switch_to(join);
        self.pop_scope();
    }

    fn assert(&mut self, cond: &thir::Expr, message: Option<&thir::Expr>, text: &str, span: sisu_diag::Span) {
        let loc = self.ctx.loc(span);
        // `assert a == b` shows both sides when it fails (17, "Assertions").
        if let ExprKind::Compare { first, rest } = &cond.kind
            && rest.len() == 1
        {
            let (op, second) = &rest[0];
            let a = self.compare_operand(first);
            let b = self.compare_operand(second);
            let r = self.temp_sir(sir::Type::Bool);
            self.assign(
                Place::local(r),
                Rvalue::Binary(crate::exprs::cmp_op(*op), a.clone(), b.clone()),
                loc,
            );
            let ok = self.new_block();
            let bad = self.new_block();
            self.terminate(
                Terminator::Switch {
                    value: Operand::Copy(Place::local(r)),
                    cases: vec![(0, bad)],
                    default: ok,
                },
                loc,
            );
            self.switch_to(bad);
            let m = match message {
                Some(m) => self.str_operand(m),
                None => Operand::Const(sir::Const::Str(String::new())),
            };
            let u = self.temp_sir(sir::Type::Never);
            self.emit(
                StatementKind::Call {
                    dest: Place::local(u),
                    callee: sir::Callee::Intrinsic("assert.fail_compare".into(), Vec::new()),
                    args: vec![Operand::Const(sir::Const::Str(text.to_string())), m, a, b],
                },
                loc,
            );
            self.terminate(Terminator::Unreachable, loc);
            self.switch_to(ok);
            return;
        }
        let v = self.operand(cond);
        let ok = self.new_block();
        let bad = self.new_block();
        self.terminate(Terminator::Switch { value: v, cases: vec![(0, bad)], default: ok }, loc);
        self.switch_to(bad);
        let m = match message {
            Some(m) => self.str_operand(m),
            None => Operand::Const(sir::Const::Str(String::new())),
        };
        let u = self.temp_sir(sir::Type::Never);
        self.emit(
            StatementKind::Call {
                dest: Place::local(u),
                callee: sir::Callee::Intrinsic("assert.fail".into(), Vec::new()),
                args: vec![Operand::Const(sir::Const::Str(text.to_string())), m],
            },
            loc,
        );
        self.terminate(Terminator::Unreachable, loc);
        self.switch_to(ok);
    }
}

/// What a `for` loop runs per item: a block, or the rest of a comprehension.
pub enum ForBody<'b> {
    Block(&'b thir::Block),
    Comprehension(Box<crate::exprs::CompRest<'b>>),
}
