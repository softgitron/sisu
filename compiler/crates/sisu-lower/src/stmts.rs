//! Statements: blocks, `let`, assignment, `return`, loops, `if`, `match`,
//! `assert`, `defer`.

use crate::func::{DropEntry, FnLower, LoopCtx, stmt_span};
use sisu_resolve::Ty;
use sisu_sir as sir;
use sisu_sir::{BlockId, Operand, Place, Rvalue, StatementKind, Terminator};
use sisu_typeck::thir::{self, ArmBody, CondPart, ExprKind, ForMode, LetElse, PatKind, Stmt};

impl FnLower<'_, '_> {
    /// A block in its own scope.
    pub fn block(&mut self, b: &thir::Block) {
        let end = self.block_end(b);
        self.push_scope_at(end);
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
        let at = self.ctx.loc(stmt_span(s));
        self.push_scope_at(self.ctx.end_loc(stmt_span(s)));
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
                    self.register(DropEntry::Local(t));
                    self.split();
                }
            }
            Stmt::AugAssign { place, op, value, span } => self.aug_assign(place, *op, value, *span),
            Stmt::Select { arms, span } => self.select(arms, *span),
            Stmt::Yield { value, span } => {
                let loc = self.ctx.loc(*span);
                let v = self.operand(value);
                let u = self.unit();
                self.intrinsic_call(u, "gen.yield", Vec::new(), vec![v], loc);
            }
            Stmt::Expr(e) => {
                let t = self.temp(&e.ty);
                self.into(Place::local(t), e);
                if self.needs_drop(&e.ty) && e.ty != Ty::Never {
                    self.register(DropEntry::Local(t));
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
                self.contract_exit(loc);
                self.emit_exit(0, loc);
                self.terminate(Terminator::Return, loc);
            }
            Stmt::Break(_) | Stmt::Continue(_) => {
                if let Some(l) = self.loops.last() {
                    let (target, depth) = if matches!(s, Stmt::Break(_)) {
                        (l.break_bb, l.depth)
                    } else {
                        (l.continue_bb, l.depth)
                    };
                    self.emit_exit(depth, at);
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
            Stmt::Assert { cond, message, text, span, replay } => {
                self.assert(cond, message.as_ref(), text, *span, *replay)
            }
            Stmt::Contract(c) => self.contract_stmt(c),
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
    /// value it held (if it holds one: the ownership checker removes the
    /// drop of a place that is not initialised there).
    fn overwrite(&mut self, pe: &thir::Expr, p: Place, v: Operand, loc: Option<sir::Loc>) {
        if self.needs_drop(&pe.ty) {
            self.split();
            self.emit(StatementKind::Drop { place: p.clone(), flag: None }, loc);
            self.split();
        }
        self.assign(p, Rvalue::Use(v), loc);
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
            // `let x: T`: the local exists from here, assigned later; its
            // drop at the end of the scope is guarded by the dataflow.
            if let PatKind::Bind { local, .. } = &pat.kind {
                let l = self.local_for(*local);
                self.scope_local(1, l);
                if self.needs_drop(&pat.ty) {
                    self.register_below(1, DropEntry::Local(l));
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
                let mut payload = t.project(sir::Proj::Downcast(0)).project(sir::Proj::Field(0));
                // The checker decays the target as a plain `let` does: a
                // `Copy` value behind a reference (`xs.first()` of a list
                // of `Copy` structs) is bound as a copy of the value.
                let payload_is_ref = match self.ctx.ty(&init.ty) {
                    sir::Type::Named(_, args) => matches!(args.first(), Some(sir::Type::Ref(..))),
                    _ => false,
                };
                if payload_is_ref && !matches!(self.ctx.ty(&pat.ty), sir::Type::Ref(..)) {
                    payload = payload.project(sir::Proj::Deref);
                }
                // A pattern's bindings, like a name's, are made on both
                // paths and registered once where they join, in the block
                // around the statement.
                match binding {
                    Some((_, l)) => {
                        let v = self.copy_or_move(payload, &pat.ty);
                        self.assign(Place::local(l), Rvalue::Use(v), loc);
                    }
                    None => self.bind_values(pat, &payload, 1),
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
                                self.bind_values(pat, &p, 1);
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
                match binding {
                    Some((tl, l)) => self.bind_registered(tl, l, &pat.ty, 1),
                    None => self.register_bindings(pat, 1),
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

    /// Registers a value binding in the scope `below` levels down: its
    /// drop and the end of its storage.
    pub fn bind_registered(&mut self, tl: thir::LocalId, l: sir::LocalId, ty: &Ty, below: usize) {
        let _ = tl;
        self.scope_local(below, l);
        if self.needs_drop(ty) {
            self.register_below(below, DropEntry::Local(l));
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
        // The loop's own statements are at its head, `for x in xs`.
        let head = if iter.span.file == span.file && iter.span.range.end >= span.range.start {
            sisu_diag::Span::new(
                span.file,
                sisu_diag::span::TextRange::new(span.range.start, iter.span.range.end),
            )
        } else {
            span
        };
        let loc = self.ctx.loc(head);
        self.push_scope();
        // A type that implements `Iterator` is driven through the trait. An
        // `Iter` behind a reference (a borrowed parameter) is driven through
        // the reference: the loop's local holds the reference, and the
        // caller keeps (and drops) the iterator.
        let borrowed_iter = mode == ForMode::Iter && matches!(iter.ty, Ty::Ref(..));
        let iter_ty = if mode == ForMode::Trait || borrowed_iter {
            self.ctx.ty(&iter.ty)
        } else {
            sir::Type::Named("Iter".into(), vec![self.ctx.ty(item)])
        };
        let it = self.temp_sir(iter_ty.clone());
        match mode {
            ForMode::Borrow | ForMode::Mut => {
                let mutable = mode == ForMode::Mut;
                // Iterating a reference (a borrowed parameter) borrows what
                // it points to, not the reference itself.
                let r = match &iter.ty {
                    Ty::Ref(inner, _) => {
                        let p = self.place_of(iter).project(sir::Proj::Deref);
                        let t = self.temp_sir(sir::Type::Ref(Box::new(self.ctx.ty(inner)), mutable));
                        let at = self.ctx.loc(iter.span);
                        self.assign(Place::local(t), Rvalue::Ref(p, mutable), at);
                        Operand::Copy(Place::local(t))
                    }
                    _ => self.borrow_operand(iter, mutable),
                };
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
            ForMode::Iter | ForMode::Trait => self.into(Place::local(it), iter),
        }
        if !borrowed_iter {
            self.register(DropEntry::Local(it));
            self.split();
        }
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
        let callee =
            match (mode, self.ctx.prog.wk.iterator.and_then(|t| self.ctx.prog.trait_method(t, "next"))) {
                (ForMode::Trait, Some(m)) => self.callee_for(m, std::slice::from_ref(&iter.ty)).0,
                _ => sir::Callee::Intrinsic("Iter.next".into(), Vec::new()),
            };
        self.emit(
            StatementKind::Call {
                dest: Place::local(next),
                callee,
                args: vec![Operand::Copy(Place::local(r))],
            },
            loc,
        );
        if self.needs_drop(item) {
            self.register(DropEntry::Local(next));
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
        // The arms at the start that test only which variant the value is
        // share one switch on its discriminant; the others test in turn.
        let (starts, rest) = self.variant_switch(arms, &p, loc);
        for (i, arm) in arms.iter().enumerate() {
            if i == starts.len()
                && let Some(r) = rest
            {
                self.switch_to(r);
            }
            let depth = self.scopes.len();
            self.push_scope();
            let next = match starts.get(i) {
                Some(&start) => {
                    self.switch_to(start);
                    None
                }
                None => {
                    let next = self.new_block();
                    self.test(&arm.pat, &p, next);
                    Some(next)
                }
            };
            self.bind(&arm.pat, &p);
            if let Some(g) = &arm.guard
                && let Some(next) = next
            {
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
                        self.register(DropEntry::Local(t));
                        self.split();
                    }
                }
                (ArmBody::Block(b), _) => self.block(b),
            }
            self.pop_scope();
            self.goto(join);
            if let Some(next) = next {
                self.switch_to(next);
            }
        }
        if starts.len() == arms.len()
            && let Some(r) = rest
        {
            self.switch_to(r);
        }
        self.terminate(Terminator::Unreachable, loc);
        self.switch_to(join);
        self.pop_scope();
    }

    /// For the arms at the start of a `match` whose patterns test only
    /// which variant the value is (a variant whose fields cannot fail, or
    /// a choice of such), one switch on the discriminant instead of a test
    /// per arm and per alternative: a `match` over many variants without
    /// payloads takes two steps rather than two per variant tried. Returns
    /// the block where each of those arms starts and the block where the
    /// others are tried; no blocks when fewer than two variants are
    /// switched on, as a single test is the same switch.
    fn variant_switch(
        &mut self,
        arms: &[thir::Arm],
        p: &Place,
        loc: Option<sir::Loc>,
    ) -> (Vec<BlockId>, Option<BlockId>) {
        let mut on: Option<Place> = None;
        let mut cases: Vec<(i64, usize)> = Vec::new();
        let mut run = 0;
        for arm in arms {
            if arm.guard.is_some() {
                break;
            }
            let Some((q, variants)) = crate::pats::variant_set(&arm.pat, p) else { break };
            if on.as_ref().is_some_and(|o| *o != q) {
                break;
            }
            on = Some(q);
            for v in variants {
                // An earlier arm takes a variant first.
                if !cases.iter().any(|(c, _)| *c == v) {
                    cases.push((v, run));
                }
            }
            run += 1;
        }
        let Some(on) = on else { return (Vec::new(), None) };
        if cases.len() < 2 {
            return (Vec::new(), None);
        }
        let d = self.temp_sir(sir::Type::Int);
        self.assign(Place::local(d), Rvalue::Discriminant(on), loc);
        let starts: Vec<BlockId> = (0..run).map(|_| self.new_block()).collect();
        let rest = self.new_block();
        let cases = cases.into_iter().map(|(v, a)| (v, starts[a])).collect();
        self.terminate(
            Terminator::Switch { value: Operand::Copy(Place::local(d)), cases, default: rest },
            loc,
        );
        (starts, Some(rest))
    }

    fn assert(
        &mut self,
        cond: &thir::Expr,
        message: Option<&thir::Expr>,
        text: &str,
        span: sisu_diag::Span,
        replay: bool,
    ) {
        let loc = self.ctx.loc(span);
        // A condition that only reads shows every part's value when it
        // fails (17, "Assertions").
        if replay && self.can_show_parts(cond) {
            self.assert_showing_parts(cond, message, span);
            return;
        }
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

impl FnLower<'_, '_> {
    /// `select:` (06, "Channels and select"): the arms' calls have their
    /// arguments evaluated and their events their values, then
    /// `select.wait(kind, subject, ...)` waits for one that can go on and
    /// gives its index (`0` a receive, `1` a send, `2` a join, `3` a
    /// timer, `4` a cancellation, each with the receiver or the event as
    /// its subject), and the arm taken makes its call, binds its pattern
    /// and runs its block.
    fn select(&mut self, arms: &[thir::SelectArm], span: sisu_diag::Span) {
        let loc = self.ctx.loc(span);
        // Each arm: its kind, its subject, and how its value is made.
        enum Made {
            Call(sir::Callee, Vec<Operand>, Ty),
            Event,
        }
        let mut made = Vec::new();
        let mut wait_args = Vec::new();
        for a in arms {
            let (kind, e) = match &a.op {
                thir::SelectOp::Recv(e) => (0, e),
                thir::SelectOp::Send(e) => (1, e),
                thir::SelectOp::Join(e) => (2, e),
                thir::SelectOp::Timer(e) => (3, e),
                thir::SelectOp::Cancelled(e) => (4, e),
            };
            wait_args.push(Operand::Const(sir::Const::Int(kind)));
            if kind <= 2 {
                let ExprKind::Call { callee: thir::Callee::Fn { def, type_args }, args } = &e.kind else {
                    made.push(Made::Event);
                    wait_args.push(Operand::Const(sir::Const::Unit));
                    continue;
                };
                let (callee, by_value) = self.callee_for(*def, type_args);
                let ops: Vec<Operand> = args
                    .iter()
                    .enumerate()
                    .map(|(i, x)| {
                        let o = self.arg_operand_for(x, by_value.get(i).copied().unwrap_or(false));
                        // Each operand in a temporary, so it is evaluated once.
                        match o {
                            Operand::Copy(p) if !p.proj.is_empty() => {
                                let t = self.temp(&x.expr.ty);
                                self.assign(Place::local(t), Rvalue::Use(Operand::Copy(p)), loc);
                                Operand::Copy(Place::local(t))
                            }
                            other => other,
                        }
                    })
                    .collect();
                wait_args.push(ops.first().cloned().unwrap_or(Operand::Const(sir::Const::Unit)));
                made.push(Made::Call(callee, ops, e.ty.clone()));
            } else {
                let t = self.temp(&e.ty);
                self.into(Place::local(t), e);
                wait_args.push(Operand::Copy(Place::local(t)));
                made.push(Made::Event);
            }
        }
        let k = self.temp_sir(sir::Type::Int);
        self.intrinsic_call(Place::local(k), "select.wait", Vec::new(), wait_args, loc);
        let join = self.new_block();
        let blocks: Vec<BlockId> = arms.iter().map(|_| self.new_block()).collect();
        let none = self.new_block();
        self.terminate(
            Terminator::Switch {
                value: Operand::Copy(Place::local(k)),
                cases: blocks.iter().enumerate().map(|(i, b)| (i as i64, *b)).collect(),
                default: none,
            },
            loc,
        );
        self.switch_to(none);
        self.terminate(Terminator::Unreachable, loc);
        for ((a, m), bb) in arms.iter().zip(made).zip(blocks) {
            self.switch_to(bb);
            self.push_scope();
            let aloc = self.ctx.loc(a.span);
            match m {
                Made::Call(callee, ops, ty) => {
                    let r = self.temp(&ty);
                    self.emit(StatementKind::Call { dest: Place::local(r), callee, args: ops }, aloc);
                    if self.needs_drop(&ty) {
                        self.register(DropEntry::Local(r));
                        self.split();
                    }
                    self.bind(&a.pat, &Place::local(r));
                }
                Made::Event => {
                    let r = self.temp(&Ty::Unit);
                    self.assign(Place::local(r), Rvalue::Use(Operand::Const(sir::Const::Unit)), aloc);
                    self.bind(&a.pat, &Place::local(r));
                }
            }
            self.block(&a.body);
            self.pop_scope();
            self.goto(join);
        }
        self.switch_to(join);
    }
}
