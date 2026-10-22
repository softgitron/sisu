//! Expressions: literals, names, operators, collections, interpolation,
//! `if` and `match` expressions, closures, `?`, `??`, `?.`, fields and
//! indexing, and the coercions between them.

use crate::check::{Checker, RetCtx};
use crate::thir::*;
use sisu_diag::{Edit, Span, TextRange, msg};
use sisu_resolve::{DefId, DefKind, FnTy, Ty, is_subset};
use sisu_syntax::ast::{self, BinOp, CmpOp, UnOp};

impl Checker<'_> {
    /// Checks an expression against a type: infer with the type as a hint,
    /// then coerce.
    pub fn check_expr(&mut self, e: &ast::Expr, ty: &Ty) -> Expr {
        // A constant where a refinement is expected is checked at compile
        // time (07); it is written as a value of the base.
        if let Some((d, args, base)) = self.refinement_of(ty)
            && crate::contracts::is_constant(self, e)
        {
            let x = self.check_expr(e, &base);
            let exp = self.resolved(ty);
            return self.refine_constant(x, d, args, exp);
        }
        let x = self.infer_expr(e, Some(ty));
        self.coerce(x, ty)
    }

    /// `check_expr` against a type written at `because` (a `let`'s
    /// annotation), which a mismatch points at.
    pub fn check_expr_because(&mut self, e: &ast::Expr, ty: &Ty, because: TextRange) -> Expr {
        if self.refinement_of(ty).is_some() && crate::contracts::is_constant(self, e) {
            return self.check_expr(e, ty);
        }
        let x = self.infer_expr(e, Some(ty));
        self.because = Some(because);
        let x = self.coerce(x, ty);
        self.because = None;
        x
    }

    /// Wraps an expression in a coercion node.
    fn wrap(&self, x: Expr, kind: Coercion, ty: Ty) -> Expr {
        let span = x.span;
        Expr::new(ExprKind::Coerce { expr: Box::new(x), kind }, ty, span)
    }

    /// Converts `x` to `expected`, or reports a mismatch.
    pub fn coerce(&mut self, x: Expr, expected: &Ty) -> Expr {
        let branch = self.branch.take();
        let because = self.because.take();
        let actual = self.resolved(&x.ty);
        let exp = self.resolved(expected);
        if actual == Ty::Error || exp == Ty::Error {
            return x;
        }
        if actual == Ty::Never {
            return if exp == Ty::Never { x } else { self.wrap(x, Coercion::NeverToAny, exp) };
        }
        if self.infer.try_unify(&actual, &exp) {
            self.check_purity_coercion(&x, &exp);
            return x;
        }
        let prog = self.prog;
        // A refinement where its base (or anything its base becomes) is
        // expected: the same value.
        if let Some((d, _, _)) = self.refinement_of(&actual)
            && !matches!(&exp, Ty::Adt(e, _) if *e == d)
        {
            let x = self.unrefined_once(x);
            return self.coerce(x, &exp);
        }
        // A value of the base where a refinement is expected: only a
        // constant is checked where it is written; anything else is checked
        // where it runs, with `X.try_from` (07).
        if let Some((d, _, base)) = self.refinement_of(&exp)
            && !matches!(&actual, Ty::Adt(a, _) if *a == d)
        {
            // Of its base, or of anything under it (an `int` where a
            // `Small = Port where ...` is expected).
            let mut under = vec![base];
            while let Some(Ty::Adt(b, args)) = under.last() {
                match self.prog.refinement_base(*b, args) {
                    Some(t) if under.len() < 64 => under.push(t),
                    _ => break,
                }
            }
            let base_ok = under.iter().any(|base| {
                let snap = self.infer.snapshot();
                let ok = self.infer.unify(&actual, base);
                self.infer.rollback(snap);
                ok
            });
            if base_ok || actual == Ty::Never {
                let name = prog.def(d).name.clone();
                let diag = self
                    .error(msg::REFINE_NEEDS_TRY_FROM, self.range_of(&x))
                    .with("expected", self.show(&exp))
                    .with("found", self.show(&actual))
                    .with("name", name.clone())
                    .note(msg::REFINE_USE_TRY_FROM, vec![("name", name)]);
                self.report(diag);
                return self.wrap(x, Coercion::Mismatch, exp);
            }
        }
        match (&actual, &exp) {
            (Ty::Adt(d, _), Ty::Str) if prog.builtin(*d) == Some("String") => {
                return self.wrap(x, Coercion::StrView, Ty::Str);
            }
            (Ty::Ref(inner, _), Ty::Str) if self.is_string(inner) => {
                return self.wrap(x, Coercion::RefStrView, Ty::Str);
            }
            (Ty::Ref(inner, true), Ty::Ref(target, false)) if self.infer.try_unify(inner, target) => {
                return self.wrap(x, Coercion::MutToShared, exp);
            }
            (Ty::ErrSet(a), Ty::ErrSet(b)) if is_subset(a, b) => return self.wrap(x, Coercion::Widen, exp),
            _ => {}
        }
        if let Ty::Ref(inner, _) = &actual {
            if self.is_copy(inner) {
                let snap = self.infer.snapshot();
                if self.infer.unify(inner, &exp) {
                    return self.wrap(x, Coercion::DerefCopy, exp);
                }
                self.infer.rollback(snap);
            }
            if let Ty::Str = exp {
                // `ref str` is a `str`.
                if self.resolved(inner) == Ty::Str {
                    return self.wrap(x, Coercion::DerefCopy, Ty::Str);
                }
            }
        }
        // `T ! A` where `T ! {A, B}` is expected.
        if let (Some((t1, e1)), Some((t2, e2))) = (self.as_result(&actual), self.as_result(&exp)) {
            let (e1, e2) = (self.resolved(&e1), self.resolved(&e2));
            if let (Ty::ErrSet(a), Ty::ErrSet(b)) = (&e1, &e2)
                && is_subset(a, b)
                && self.infer.try_unify(&t1, &t2)
            {
                return self.wrap(x, Coercion::Widen, exp);
            }
        }
        // A value of a type that implements a trait, as `dyn Trait`; a
        // `ref T` as a `ref dyn Trait`; a `Box[T]` as a `Box[dyn Trait]`.
        if let (Ty::Ref(inner, m1), Ty::Ref(target, m2)) = (&actual, &exp)
            && (*m1 || !*m2)
            && let Ty::Dyn(tr) = self.resolved(target)
            && !matches!(self.resolved(inner), Ty::Dyn(_) | Ty::Var(_))
        {
            let inner = self.resolved(inner);
            return self.coerce_to_dyn(x, &inner, &tr, &exp, Coercion::RefToDyn);
        }
        if let Ty::Dyn(tr) = &exp
            && !matches!(actual, Ty::Dyn(_) | Ty::Var(_))
        {
            // A `dyn` expected by value was reported where it is written
            // (E0395); a reference to one is no second error.
            if let Ty::Ref(inner, _) = &actual
                && matches!(self.resolved(inner), Ty::Dyn(_))
            {
                return self.wrap(x, Coercion::Mismatch, exp.clone());
            }
            return self.coerce_to_dyn(x, &actual, tr, &exp, Coercion::ToDyn);
        }
        if let (Ty::Adt(d1, a1), Ty::Adt(d2, a2)) = (&actual, &exp)
            && d1 == d2
            && Some(*d1) == prog.wk.boxed
            && let (Some(inner), Some(Ty::Dyn(tr))) = (a1.first(), a2.first().map(|t| self.resolved(t)))
            && !matches!(self.resolved(inner), Ty::Dyn(_) | Ty::Var(_))
        {
            let inner = self.resolved(inner);
            return self.coerce_to_dyn(x, &inner, &tr, &exp, Coercion::BoxToDyn);
        }
        // A `list[T]` as a `Slice[T]`, and `bytes` as a `Slice[u8]`: a view.
        if let Ty::Adt(sd, sargs) = &exp
            && prog.builtin(*sd) == Some("Slice")
            && let Ty::Adt(src, srcargs) = self.resolved(actual.strip_ref())
        {
            let elem = sargs.first().cloned().unwrap_or(Ty::Error);
            let src_ty = Ty::Adt(src, srcargs.clone());
            let ok = match prog.builtin(src) {
                Some("bytes") => {
                    let u8_ty = self.sized_ty("u8");
                    self.infer.try_unify(&elem, &u8_ty)
                }
                Some("list") => srcargs.first().is_some_and(|t| self.infer.try_unify(t, &elem)),
                _ => false,
            };
            if ok && let Some(call) = self.builtin_call(x.clone(), &src_ty, "as_slice", exp.clone()) {
                return call;
            }
        }
        // An integer as a wider integer type that holds all its values.
        if let (Some(a), Some(b)) = (self.int_kind(&actual), self.int_kind(&exp))
            && sisu_resolve::std_sources::widens(a, b)
        {
            return self.wrap(x, Coercion::IntWiden, exp);
        }
        // An `f32` as an `f64`, which holds every one (03, "Numbers").
        if self.float_kind(&actual) == Some("f32") && exp == Ty::Float {
            return self.wrap(x, Coercion::FloatWiden, exp);
        }
        // A value of a type that implements a capability's operations
        // trait, as the capability (06, "Capabilities").
        if let Ty::Adt(d, _) = &exp
            && let Some(ops) = prog.capability_ops(*d)
        {
            let (x, concrete) = match &actual {
                Ty::Ref(inner, _) if self.is_copy(inner) => {
                    let inner = self.resolved(inner);
                    (self.wrap(x, Coercion::DerefCopy, inner.clone()), inner)
                }
                _ => (x, actual.clone()),
            };
            let concrete = prog.normalize(&self.infer.zonk(&concrete), &self.env);
            if concrete == exp {
                return x;
            }
            let tr = sisu_resolve::TraitRef { def: ops, args: Vec::new() };
            if !matches!(concrete, Ty::Var(_))
                && prog.implements(&concrete, &tr, &self.env) != sisu_resolve::Answer::No
            {
                return self.coerce_to_cap(x, &concrete, tr, &exp);
            }
            let found_s = self.show(&concrete);
            let d = self
                .error(msg::TYPE_MISMATCH, self.range_of(&x))
                .with("expected", self.show(&exp))
                .with("found", found_s);
            self.report(d);
            return self.wrap(x, Coercion::Mismatch, exp);
        }
        // `Iterable[T]` accepts any collection or iterator of `T`.
        if let Ty::Adt(d, args) = &exp
            && Some(*d) == prog.wk.iterable
            && let Some((item, _)) = self.iteration_item(&actual, ast::Mode::Borrow)
        {
            let item = self.decay_ty(&item);
            if let Some(t) = args.first()
                && self.infer.try_unify(&item, t)
            {
                return x;
            }
        }
        // An error the expected error set does not include.
        let sets = match (&actual, &exp) {
            (Ty::ErrSet(a), Ty::ErrSet(b)) => Some((a.clone(), b.clone())),
            _ => match (self.as_result(&actual), self.as_result(&exp)) {
                (Some((t1, e1)), Some((t2, e2))) if self.infer.try_unify(&t1, &t2) => {
                    match (self.resolved(&e1), self.resolved(&e2)) {
                        (Ty::ErrSet(a), Ty::ErrSet(b)) => Some((a, b)),
                        _ => None,
                    }
                }
                _ => None,
            },
        };
        if let Some((from, to)) = sets
            && !is_subset(&from, &to)
        {
            let missing: Vec<String> =
                from.iter().filter(|d| !to.contains(d)).map(|d| self.prog.qualified_name(*d)).collect();
            let d = self
                .error(msg::TYPE_ERROR_NOT_IN_SET, self.range_of(&x))
                .with("from", self.prog.display(&Ty::ErrSet(from.clone())))
                .with("to", self.prog.display(&Ty::ErrSet(to.clone())))
                .fix(msg::TYPE_ADD_ERROR_SET, vec![("set", missing.join(", "))], Vec::new());
            self.report(d);
            return self.wrap(x, Coercion::Mismatch, exp);
        }
        let expected_s = self.show(&exp);
        let found_s = self.show(&actual);
        let mut d = match branch {
            // Two branches that disagree, where nothing else asked for a
            // type: neither is the one expected.
            Some((keyword, first)) => self
                .error(msg::TYPE_IF_BRANCHES, self.range_of(&x))
                .with("keyword", keyword)
                .with("first", expected_s.clone())
                .with("second", found_s.clone())
                .label(self.span(first), msg::TYPE_FOUND_HERE, vec![("ty", expected_s)]),
            None => {
                let d = self
                    .error(msg::TYPE_MISMATCH, self.range_of(&x))
                    .with("expected", expected_s)
                    .with("found", found_s.clone());
                match because {
                    Some(r) => d.label(self.span(r), msg::TYPE_EXPECTED_BECAUSE, Vec::new()),
                    None => d,
                }
            }
        };
        if matches!(actual, Ty::Int)
            && let Some(k) = self.float_kind(&exp)
        {
            d = d.note(
                msg::TYPE_MIXED_NUMBERS,
                vec![("op", "=".into()), ("lhs", k.into()), ("rhs", "int".into())],
            );
        }
        d = self.mismatch_fix(d, &x, &actual, &exp);
        self.report(d);
        self.wrap(x, Coercion::Mismatch, exp)
    }

    /// The fix of a value of the wrong type, where one is plain: a number
    /// written as text loses its quotes; a value whose type has a method of
    /// no arguments giving the type expected gets the call (`.count()` of
    /// an `Iter` where an `int` is expected), the first by name.
    fn mismatch_fix(
        &self,
        d: sisu_diag::Diagnostic,
        x: &Expr,
        actual: &Ty,
        exp: &Ty,
    ) -> sisu_diag::Diagnostic {
        let r = x.span.range;
        let Some(text) = self.source.get(r.start as usize..r.end as usize) else { return d };
        if let ExprKind::Str(s) = &x.kind
            && text.starts_with('"')
        {
            let number = match exp {
                Ty::Int => s.parse::<i64>().is_ok(),
                t if self.float_kind(t).is_some() => s.parse::<f64>().is_ok() && s.contains('.'),
                t => self.int_kind(t).is_some() && s.parse::<i128>().is_ok(),
            };
            if number && !s.is_empty() {
                let edit = sisu_diag::Edit { span: x.span, text: s.clone() };
                return d.fix(msg::TYPE_UNQUOTE, Vec::new(), vec![edit]);
            }
        }
        // A literal wants another literal, not a call on it: no
        // `"text".byte_len()` where an `int` is expected.
        let literal = matches!(
            x.kind,
            ExprKind::Int(_)
                | ExprKind::WideInt(_)
                | ExprKind::Float(_)
                | ExprKind::Bool(_)
                | ExprKind::Char(_)
                | ExprKind::Str(_)
                | ExprKind::Bytes(_)
                | ExprKind::Interp(_)
                | ExprKind::List(_)
                | ExprKind::Tuple(_)
                | ExprKind::Unit
        );
        if literal {
            return d;
        }
        let prog = self.prog;
        let mut keys: Vec<sisu_resolve::TyKey> = actual.key().into_iter().collect();
        if self.is_string(actual) {
            keys.push(sisu_resolve::TyKey::Str);
        }
        let gives = |f: &sisu_resolve::FnInfo| {
            matches!(f.self_mode, Some(ast::Mode::Borrow | ast::Mode::Own))
                && f.params.is_empty()
                && f.generics.len() == f.impl_generics
                && !f.err_inferred
                && f.ret == *exp
        };
        let mut found: Vec<&String> = prog
            .methods
            .iter()
            .filter(|((k, m), def)| {
                keys.contains(k) && !m.starts_with('_') && prog.fn_info(**def).is_some_and(gives)
            })
            .map(|((_, m), _)| m)
            .collect();
        // The methods of the traits it implements (without parameters of
        // their own): `count` of an `Iter`.
        for (i, def) in prog.defs.iter().enumerate() {
            let DefKind::Trait(info) = &def.kind else { continue };
            if !info.generics.is_empty() {
                continue;
            }
            let wanted: Vec<&String> = info
                .methods
                .iter()
                .filter(|m| prog.fn_info(**m).is_some_and(gives))
                .map(|m| &prog.def(*m).name)
                .collect();
            if wanted.is_empty() {
                continue;
            }
            let tr = sisu_resolve::TraitRef { def: DefId(i as u32), args: Vec::new() };
            if prog.implements(actual, &tr, &self.env).holds() {
                found.extend(wanted);
            }
        }
        found.sort();
        let Some(m) = found.first() else { return d };
        let simple = text.bytes().all(|b| {
            b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'(' | b')' | b'[' | b']' | b'"')
        }) && !text.is_empty();
        let edits = if simple {
            vec![sisu_diag::Edit {
                span: Span::new(self.file, TextRange::empty(r.end)),
                text: format!(".{m}()"),
            }]
        } else {
            vec![
                sisu_diag::Edit { span: Span::new(self.file, TextRange::empty(r.start)), text: "(".into() },
                sisu_diag::Edit {
                    span: Span::new(self.file, TextRange::empty(r.end)),
                    text: format!(").{m}()"),
                },
            ]
        };
        d.fix(msg::TYPE_CALL_CONVERSION, vec![("method", m.to_string())], edits)
    }

    pub fn range_of(&self, x: &Expr) -> TextRange {
        x.span.range
    }

    /// A coercion to `dyn Trait` of a value of type `concrete`: it must
    /// implement the trait, and it must own what it holds (a view cannot
    /// hide behind `dyn`, which owns its value; 04, "Views").
    fn coerce_to_dyn(
        &mut self,
        x: Expr,
        concrete: &Ty,
        tr: &sisu_resolve::TraitRef,
        exp: &Ty,
        kind: Coercion,
    ) -> Expr {
        let prog = self.prog;
        let concrete = prog.normalize(&self.infer.zonk(concrete), &self.env);
        if prog.can_borrow(&concrete) && !concrete.contains_error() {
            let d = self
                .error(msg::TYPE_VIEW_TO_DYN, self.range_of(&x))
                .with("ty", self.show(&concrete))
                .with("dyn", self.show(&Ty::Dyn(tr.clone())));
            self.report(d);
            return self.wrap(x, Coercion::Mismatch, exp.clone());
        }
        self.bounds.push((concrete, tr.clone(), x.span));
        self.wrap(x, kind, exp.clone())
    }

    /// A value as a capability handle: it implements the operations trait,
    /// and since the handle can be stored and reach every task, it owns
    /// what it holds, is not must-consume (the handle never drops it), and
    /// is `Send` and `Sync` (checked once types are known).
    fn coerce_to_cap(&mut self, x: Expr, concrete: &Ty, tr: sisu_resolve::TraitRef, exp: &Ty) -> Expr {
        let prog = self.prog;
        let cap_name = self.show(exp);
        let owner: &'static str = match exp {
            Ty::Adt(d, _) => match prog.builtin(*d) {
                Some("Fs") => "Fs",
                Some("Net") => "Net",
                Some("Clock") => "Clock",
                Some("Rand") => "Rand",
                Some("Env") => "Env",
                Some("Stdin") => "Stdin",
                _ => "Proc",
            },
            _ => "Proc",
        };
        self.bounds.push((concrete.clone(), tr, x.span));
        self.owner_checks.push((concrete.clone(), owner, x.span));
        self.cap_checks.push((concrete.clone(), cap_name, x.span));
        self.wrap(x, Coercion::ToCap, exp.clone())
    }

    /// A function passed where a `pure` one is required must be pure.
    fn check_purity_coercion(&mut self, x: &Expr, expected: &Ty) {
        let Ty::Fn(f) = self.infer.zonk(expected) else { return };
        if !f.pure {
            return;
        }
        let callee = match &x.kind {
            ExprKind::FnRef(d, _) if !self.prog.fn_info(*d).is_some_and(|i| i.pure) => {
                Some(self.prog.def(*d).name.clone())
            }
            ExprKind::Closure(c) => self
                .closures
                .get(c.0 as usize)
                .and_then(|c| c.as_ref())
                .and_then(|c| crate::purity::first_impure_in_block(self.prog, &c.body, &self.locals)),
            _ => None,
        };
        if let Some(callee) = callee {
            let d = self.error(msg::TYPE_IMPURE_ARGUMENT, x.span.range).with("callee", callee);
            self.report(d);
        }
    }

    /// A `ref T` of a `Copy` type used as a value becomes the `T` (the
    /// rule that lets `(word.copy(), n)` hold an `int`, not a reference).
    pub fn decay(&mut self, x: Expr) -> Expr {
        // An explicit `ref` keeps its reference: `let first = ref xs[0]`
        // borrows `xs` (04, "Borrowing rules in detail").
        if matches!(x.kind, ExprKind::Borrow { .. }) {
            return x;
        }
        match self.resolved(&x.ty) {
            Ty::Ref(inner, _) if self.is_copy(&inner) => self.wrap(x, Coercion::DerefCopy, *inner),
            _ => x,
        }
    }

    pub fn decay_ty(&self, ty: &Ty) -> Ty {
        match self.resolved(ty) {
            Ty::Ref(inner, _) if self.is_copy(&inner) => *inner,
            t => t,
        }
    }

    /// Reports a move out of a borrowed place or an element.
    pub fn check_movable(&mut self, x: Expr) -> Expr {
        if x.is_place() && x.is_behind_ref() && !self.is_copy(&x.ty) && self.resolved(&x.ty) != Ty::Error {
            let what = self.source_text(x.span.range);
            let edit = Edit {
                span: Span::new(self.file, TextRange::empty(x.span.range.end)),
                text: ".copy()".into(),
            };
            let d = self.error(msg::TYPE_MOVE_OUT_OF_BORROW, x.span.range).with("what", what).fix(
                msg::TYPE_ADD_COPY,
                Vec::new(),
                vec![edit],
            );
            self.report(d);
        }
        x
    }

    /// Dereferences a reference value, so that its place can be used.
    pub fn auto_deref(&mut self, x: Expr) -> Expr {
        let mut x = x;
        while let Ty::Ref(inner, _) = self.resolved(&x.ty) {
            let span = x.span;
            x = Expr::new(ExprKind::Deref(Box::new(x)), *inner, span);
        }
        x
    }

    /// Whether the type is a `Box[T]`.
    pub fn is_box(&self, ty: &Ty) -> bool {
        matches!(self.resolved(ty), Ty::Adt(d, _) if Some(d) == self.prog.wk.boxed)
    }

    /// The builtin name of the type, when it is one of the std's
    /// pointer-like types (`Shared`, `MutexGuard`).
    pub fn smart_pointer(&self, ty: &Ty) -> Option<&'static str> {
        match self.resolved(ty) {
            Ty::Adt(d, _) => match self.prog.builtin(d) {
                Some("Shared") => Some("Shared"),
                Some("MutexGuard") => Some("MutexGuard"),
                Some("ReadGuard") => Some("ReadGuard"),
                _ => None,
            },
            _ => None,
        }
    }

    /// The value a `Shared` owns or a mutex guard holds, as a place.
    pub fn deref_smart(&mut self, x: Expr) -> Expr {
        let inner = match self.resolved(&x.ty) {
            Ty::Adt(_, a) => a.first().cloned().unwrap_or(Ty::Error),
            _ => Ty::Error,
        };
        let span = x.span;
        Expr::new(ExprKind::Deref(Box::new(x)), inner, span)
    }

    /// The value a `Box` holds, as a place inside it.
    pub fn unbox(&mut self, x: Expr) -> Expr {
        let inner = match self.resolved(&x.ty) {
            Ty::Adt(_, a) => a.first().cloned().unwrap_or(Ty::Error),
            _ => Ty::Error,
        };
        let span = x.span;
        Expr::new(ExprKind::Field { base: Box::new(x), index: 0 }, inner, span)
    }

    // ---------------------------------------------------------------- inference

    pub fn infer_expr(&mut self, e: &ast::Expr, expected: Option<&Ty>) -> Expr {
        let span = self.span(e.range);
        match &e.kind {
            ast::ExprKind::Int { value, suffix, text } => {
                let x = self.int_literal(*value, suffix.as_deref(), text, false, e.range, expected);
                if suffix.is_none() { self.literal_via_trait(x, expected) } else { x }
            }
            ast::ExprKind::Float { value, suffix, text } => {
                let x = self.float_literal(*value, suffix.as_deref(), text, false, e.range, expected);
                if suffix.is_none() { self.literal_via_trait(x, expected) } else { x }
            }
            ast::ExprKind::Char(c) => Expr::new(ExprKind::Char(*c), Ty::Char, span),
            ast::ExprKind::Str(parts) => self.string_literal(parts, e.range, expected),
            ast::ExprKind::RawStr(s) => self.plain_string(s.clone(), span, expected),
            ast::ExprKind::ByteStr(b) => {
                let ty = self.prog.lookup_in(self.prog.prelude, "bytes").map(|d| Ty::Adt(d, Vec::new()));
                Expr::new(ExprKind::Bytes(b.clone()), ty.unwrap_or(Ty::Error), span)
            }
            ast::ExprKind::Name(n) => self.name_expr(n, e.range, expected),
            ast::ExprKind::SelfValue => match self.lookup_local("self") {
                Some(l) => {
                    self.note_use(l, false);
                    Expr::new(ExprKind::Local(l), self.local_ty(l), span)
                }
                None => {
                    if !self.quiet_unknown_names {
                        let d = self.error(msg::TYPE_NO_SELF, e.range);
                        self.report(d);
                    }
                    Expr::error(span)
                }
            },
            ast::ExprKind::SelfType => {
                let d = self.error(msg::TYPE_NOT_A_VALUE, e.range).with("name", "Self").with("kind", "type");
                self.report(d);
                Expr::error(span)
            }
            ast::ExprKind::Paren(inner) => {
                let mut x = self.infer_expr(inner, expected);
                x.span = span;
                x
            }
            ast::ExprKind::Tuple(items) => self.tuple_expr(items, e.range, expected),
            ast::ExprKind::List(items) => {
                let elem = match expected.map(|t| self.resolved(t)) {
                    Some(Ty::Adt(d, a)) if Some(d) == self.prog.wk.list => a.first().cloned(),
                    _ => None,
                };
                let (elems, ty) = self.homogeneous(items, elem);
                Expr::new(ExprKind::List(elems), self.list_of(ty), span)
            }
            ast::ExprKind::Set(items) => {
                let elem = match expected.map(|t| self.resolved(t)) {
                    Some(Ty::Adt(d, a)) if Some(d) == self.prog.wk.set => a.first().cloned(),
                    _ => None,
                };
                let (elems, ty) = self.homogeneous(items, elem);
                self.require(ty.clone(), self.prog.wk.hash, span);
                let set_ty = self.prog.wk.set.map(|s| Ty::Adt(s, vec![ty])).unwrap_or(Ty::Error);
                Expr::new(ExprKind::Set(elems), set_ty, span)
            }
            ast::ExprKind::Dict(entries) => {
                let (kt, vt) = match expected.map(|t| self.resolved(t)) {
                    Some(Ty::Adt(d, a)) if Some(d) == self.prog.wk.dict && a.len() == 2 => {
                        (Some(a[0].clone()), Some(a[1].clone()))
                    }
                    _ => (None, None),
                };
                let keys: Vec<ast::Expr> = entries.iter().map(|(k, _)| k.clone()).collect();
                let values: Vec<ast::Expr> = entries.iter().map(|(_, v)| v.clone()).collect();
                let (ks, kt) = self.homogeneous(&keys, kt);
                let (vs, vt) = self.homogeneous(&values, vt);
                self.require(kt.clone(), self.prog.wk.hash, span);
                let ty = self.prog.wk.dict.map(|d| Ty::Adt(d, vec![kt, vt])).unwrap_or(Ty::Error);
                Expr::new(ExprKind::Dict(ks.into_iter().zip(vs).collect()), ty, span)
            }
            ast::ExprKind::Comprehension { kind, elem, value, clauses } => {
                self.comprehension(*kind, elem, value.as_deref(), clauses, e.range)
            }
            ast::ExprKind::Binary { op, lhs, rhs } => self.binary_expr(*op, lhs, rhs, e.range, expected),
            ast::ExprKind::Compare { first, rest } => self.compare_expr(first, rest, e.range),
            ast::ExprKind::Unary { op, expr } => self.unary_expr(*op, expr, e.range, expected),
            ast::ExprKind::Ref { mutable, expr } => {
                let x = self.infer_expr(expr, None);
                if !x.is_place() && x.ty != Ty::Error {
                    let d = self.error(msg::TYPE_NOT_A_PLACE, expr.range);
                    self.report(d);
                }
                if *mutable {
                    self.require_mutable(&x, expr.range);
                }
                let ty = Ty::Ref(Box::new(x.ty.clone()), *mutable);
                Expr::new(ExprKind::Borrow { place: Box::new(x), mutable: *mutable }, ty, span)
            }
            ast::ExprKind::FromEnd(_) => {
                let d = self.error(msg::TYPE_FROM_END_OUTSIDE_INDEX, e.range);
                self.report(d);
                Expr::error(span)
            }
            ast::ExprKind::Call { callee, args, trailing } => {
                self.call_expr(callee, args, trailing.as_deref(), e.range, expected)
            }
            ast::ExprKind::Field { base, name } => self.field_expr(base, name, e.range),
            ast::ExprKind::OptField { base, name } => self.opt_chain(base, name, None, e.range),
            ast::ExprKind::Index { base, args } => {
                if let Some(x) = self.generator_site(e, base, args, None, expected) {
                    return x;
                }
                self.index_expr(base, args, e.range)
            }
            ast::ExprKind::Try(inner) => self.try_expr(inner, e.range),
            ast::ExprKind::Closure(c) => {
                let fn_ty = expected.and_then(|t| match self.resolved(t) {
                    Ty::Fn(f) => Some(*f),
                    _ => None,
                });
                self.check_closure(c, fn_ty)
            }
            ast::ExprKind::If { branches, else_ } => {
                self.if_expr(branches, else_.as_deref(), e.range, expected)
            }
            ast::ExprKind::Match { scrutinee, arms } => self.match_expr(scrutinee, arms, e.range, expected),
            ast::ExprKind::Comptime(inner) => self.comptime_expr(inner, expected, e.range),
            ast::ExprKind::FieldOf { .. } => {
                self.no_unfolding(e.range, "a field named by a compile-time value (`x.[f]`)");
                Expr::error(span)
            }
            ast::ExprKind::Unsupported(what) => {
                self.unsupported(e.range, what, "foreign functions (Phase 8)");
                Expr::error(span)
            }
            ast::ExprKind::Error => Expr::error(span),
        }
    }

    fn int_literal(
        &mut self,
        value: Option<u128>,
        suffix: Option<&str>,
        text: &str,
        negative: bool,
        range: TextRange,
        expected: Option<&Ty>,
    ) -> Expr {
        let span = self.span(range);
        // A sized integer: the suffix's, or the expected type's.
        let sized = match suffix {
            None => expected.and_then(|t| self.int_kind(t)).filter(|k| *k != "int"),
            Some("uint") => Some("u64"),
            Some(s) => sisu_resolve::std_sources::SIZED.iter().copied().find(|k| *k == s),
        };
        if let Some(k) = sized {
            return self.sized_literal(k, value, text, negative, range);
        }
        match suffix {
            None | Some("int") | Some("i64") => {}
            Some("f64") => {
                return Expr::new(
                    ExprKind::Float(value.unwrap_or(0) as f64 * if negative { -1.0 } else { 1.0 }),
                    Ty::Float,
                    span,
                );
            }
            Some(s) => {
                self.unsupported(range, &format!("`{s}`"), "the sized numeric types (Phase 4)");
                return Expr::error(span);
            }
        }
        let to_float = if suffix.is_none() { expected.and_then(|t| self.float_kind(t)) } else { None };
        let limit: u128 = if negative { 1u128 << 63 } else { (1u128 << 63) - 1 };
        match value {
            Some(v) if v <= limit => {
                // An integer literal where a float is expected is that
                // float, rounded once (`v as f32` rounds to the nearest).
                if to_float == Some("f32") {
                    let f = if negative { -(v as f32) } else { v as f32 };
                    return Expr::new(ExprKind::Float(f64::from(f)), self.sized_ty("f32"), span);
                }
                if to_float.is_some() {
                    let f = v as f64;
                    return Expr::new(ExprKind::Float(if negative { -f } else { f }), Ty::Float, span);
                }
                let n = if negative { (v as i128).wrapping_neg() as i64 } else { v as i64 };
                Expr::new(ExprKind::Int(n), Ty::Int, span)
            }
            _ => {
                let t = if negative { format!("-{text}") } else { text.to_string() };
                let d =
                    self.error(msg::TYPE_LITERAL_OUT_OF_RANGE, range).with("text", t).with("ty", "int").note(
                        msg::TYPE_INT_RANGE,
                        vec![
                            ("ty", "int".into()),
                            ("min", i64::MIN.to_string()),
                            ("max", i64::MAX.to_string()),
                        ],
                    );
                self.report(d);
                Expr::error(span)
            }
        }
    }

    /// An integer literal of the sized type `k`, checked to fit. A `u64`
    /// literal is its 64-bit pattern.
    fn sized_literal(
        &mut self,
        k: &str,
        value: Option<u128>,
        text: &str,
        negative: bool,
        range: TextRange,
    ) -> Expr {
        let span = self.span(range);
        let (bits, signed) = sisu_resolve::std_sources::int_width(k).unwrap_or((64, true));
        let ty =
            self.prog.lookup_in(self.prog.prelude, k).map(|d| Ty::Adt(d, Vec::new())).unwrap_or(Ty::Error);
        let max: u128 = if signed { (1u128 << (bits - 1)) - 1 } else { u128::MAX >> (128 - bits) };
        let fits = match value {
            Some(v) if negative => signed && v <= max + 1,
            Some(v) => v <= max,
            None => false,
        };
        if !fits {
            let t = if negative { format!("-{text}") } else { text.to_string() };
            let min = if signed { format!("-{}", max + 1) } else { "0".to_string() };
            let d = self.error(msg::TYPE_LITERAL_OUT_OF_RANGE, range).with("text", t).with("ty", k).note(
                msg::TYPE_INT_RANGE,
                vec![("ty", k.to_string()), ("min", min), ("max", max.to_string())],
            );
            self.report(d);
            return Expr::error(span);
        }
        let v = value.unwrap_or(0);
        // A 128-bit literal beyond `i64` keeps its 128 bits.
        if bits == 128 {
            let bits = if negative { v.wrapping_neg() } else { v };
            return match i64::try_from(bits as i128) {
                Ok(n) if signed || !negative && v <= i64::MAX as u128 => {
                    Expr::new(ExprKind::Int(n), ty, span)
                }
                _ => Expr::new(ExprKind::WideInt(bits), ty, span),
            };
        }
        let n = if negative { (v as i128).wrapping_neg() as i64 } else { v as u64 as i64 };
        Expr::new(ExprKind::Int(n), ty, span)
    }

    /// The sized integer type named `k` (`u8`), or `f32`.
    pub fn sized_ty(&self, k: &str) -> Ty {
        self.prog.lookup_in(self.prog.prelude, k).map(|d| Ty::Adt(d, Vec::new())).unwrap_or(Ty::Error)
    }

    /// `f64` for `f64`, `f32` for `f32`, `None` for any other type.
    pub fn float_kind(&self, t: &Ty) -> Option<&'static str> {
        match self.resolved(t) {
            Ty::Float => Some("f64"),
            Ty::Adt(d, a) if a.is_empty() && self.prog.builtin(d) == Some("f32") => Some("f32"),
            _ => None,
        }
    }

    /// Whether `t` is a number type: an integer type or a float type.
    pub fn is_number(&self, t: &Ty) -> bool {
        self.int_kind(t).is_some() || self.float_kind(t).is_some()
    }

    /// A float literal: an `f64`, or an `f32` by its suffix or where one is
    /// expected, rounded once from its text (03, "Numbers"); one whose
    /// magnitude its type cannot hold is `E0374`.
    fn float_literal(
        &mut self,
        value: f64,
        suffix: Option<&str>,
        text: &str,
        negative: bool,
        range: TextRange,
        expected: Option<&Ty>,
    ) -> Expr {
        let span = self.span(range);
        let kind = match suffix {
            Some("f32") => "f32",
            Some("f64") => "f64",
            Some(s) => {
                let d = self.error(msg::TYPE_FLOAT_SUFFIX, range).with("suffix", s);
                self.report(d);
                return Expr::error(span);
            }
            None => expected.and_then(|t| self.float_kind(t)).unwrap_or("f64"),
        };
        let (value, ty, max) = if kind == "f32" {
            let digits = text.strip_suffix("f32").unwrap_or(text).replace('_', "");
            let v: f32 = digits.parse().unwrap_or(0.0);
            (f64::from(if negative { -v } else { v }), self.sized_ty("f32"), format!("{:e}", f32::MAX))
        } else {
            (if negative { -value } else { value }, Ty::Float, format!("{:e}", f64::MAX))
        };
        if value.is_infinite() {
            let t = if negative { format!("-{text}") } else { text.to_string() };
            let d = self
                .error(msg::TYPE_LITERAL_OUT_OF_RANGE, range)
                .with("text", t)
                .with("ty", kind)
                .note(msg::TYPE_FLOAT_RANGE, vec![("ty", kind.to_string()), ("max", max)]);
            self.report(d);
            return Expr::error(span);
        }
        Expr::new(ExprKind::Float(value), ty, span)
    }

    /// An expression as the borrowed receiver of a builtin method: a
    /// reference is passed on, a `Copy` value copied, anything else
    /// borrowed.
    pub fn receiver_arg(&self, x: Expr) -> Arg {
        let mode = match self.resolved(&x.ty) {
            Ty::Ref(..) => ArgMode::Value,
            t if self.is_copy(&t) => ArgMode::Value,
            _ => ArgMode::Borrow,
        };
        Arg { expr: x, mode }
    }

    /// A call of the builtin method `name` of `owner` on `recv`, with the
    /// owner's type arguments and the result type `ret`.
    fn builtin_call(&self, recv: Expr, owner: &Ty, name: &str, ret: Ty) -> Option<Expr> {
        let Ty::Adt(d, a) = owner else { return None };
        let m = self.prog.method(sisu_resolve::TyKey::Adt(*d), name)?;
        let span = recv.span;
        let recv = self.receiver_arg(recv);
        Some(Expr::new(
            ExprKind::Call { callee: Callee::Fn { def: m, type_args: a.clone() }, args: vec![recv] },
            ret,
            span,
        ))
    }

    /// `int` for `int`, the type's name for a sized integer type.
    pub fn int_kind(&self, t: &Ty) -> Option<&'static str> {
        match self.resolved(t) {
            Ty::Int => Some("int"),
            Ty::Adt(d, a) if a.is_empty() => {
                let b = self.prog.builtin(d)?;
                sisu_resolve::std_sources::SIZED.iter().copied().find(|s| *s == b)
            }
            _ => None,
        }
    }

    fn plain_string(&mut self, s: String, span: Span, expected: Option<&Ty>) -> Expr {
        // A literal takes the expected type when it is `String` or `Path`
        // (their `FromLiteral[str]`, 02 "Literals", which lowering does
        // directly), or another type that implements `FromLiteral[str]`.
        let ty = match expected.map(|t| self.resolved(t)) {
            Some(t @ Ty::Adt(d, _)) if matches!(self.prog.builtin(d), Some("String" | "Path")) => t,
            _ => Ty::Str,
        };
        let x = Expr::new(ExprKind::Str(s), ty.clone(), span);
        if ty == Ty::Str { self.literal_via_trait(x, expected) } else { x }
    }

    /// A literal whose expected type is not the literal's own but
    /// implements `FromLiteral[L]` for the literal's type `L`: the call
    /// `FromLiteral.from_literal(x)` (02, "Literals").
    fn literal_via_trait(&mut self, x: Expr, expected: Option<&Ty>) -> Expr {
        let prog = self.prog;
        let (Some(exp), Some(tr_def)) = (expected, prog.wk.from_literal) else { return x };
        let exp = prog.normalize(&self.infer.zonk(exp), &self.env);
        if !matches!(exp, Ty::Adt(..) | Ty::Param(_) | Ty::Assoc(_)) || x.ty == Ty::Error {
            return x;
        }
        let tr = sisu_resolve::TraitRef { def: tr_def, args: vec![x.ty.clone()] };
        if !prog.implements(&exp, &tr, &self.env).holds() {
            return x;
        }
        let Some(m) = prog.trait_method(tr_def, "from_literal") else { return x };
        let span = x.span;
        let lit = x.ty.clone();
        Expr::new(
            ExprKind::Call {
                callee: Callee::Fn { def: m, type_args: vec![exp.clone(), lit] },
                args: vec![Arg { expr: x, mode: ArgMode::Ref }],
            },
            exp,
            span,
        )
    }

    fn string_literal(&mut self, parts: &[ast::StrPart], range: TextRange, expected: Option<&Ty>) -> Expr {
        let span = self.span(range);
        if parts.iter().all(|p| matches!(p, ast::StrPart::Text(_))) {
            return self.plain_string(ast::build::plain_text(parts), span, expected);
        }
        let mut out = Vec::new();
        for p in parts {
            match p {
                ast::StrPart::Text(t) => out.push(InterpPart::Text(t.clone())),
                ast::StrPart::Interp { expr, debug, spec, .. } => {
                    let x = self.infer_expr(expr, None);
                    let spec = spec.clone().unwrap_or_default();
                    // A spec ending in `?` asks for `Debug` (`{x:>8?}`).
                    let wanted = if debug.is_none() && !spec.ends_with('?') {
                        self.prog.wk.display
                    } else {
                        self.prog.wk.debug
                    };
                    let span = self.span(expr.range);
                    self.require(x.ty.clone(), wanted, span);
                    // A `DateTime` takes a `strftime` pattern instead.
                    let is_time = matches!(self.resolved(x.ty.strip_ref()), Ty::Adt(d, _) if self.prog.builtin(d) == Some("DateTime"));
                    if !spec.is_empty() && !is_time && !is_format_spec(&spec) {
                        let d = self.error(msg::TYPE_BAD_FORMAT_SPEC, expr.range).with("spec", spec.clone());
                        self.report(d);
                    } else if !is_time
                        && let Some(letter) = spec.chars().last().filter(|c| "boxXdeEfF%".contains(*c))
                    {
                        // The spec's letter must fit a value whose rendering
                        // the compiler knows (E0380): `x` is for integers,
                        // `f` for numbers, and text takes neither. A type of
                        // the program's own applies its spec in its impl.
                        let t = self.resolved(x.ty.strip_ref());
                        let integers_only = "boxXd".contains(letter);
                        let wrong = match &t {
                            t if self.float_kind(t).is_some() => integers_only,
                            Ty::Bool | Ty::Char | Ty::Str => true,
                            t => self.is_string(t),
                        };
                        if wrong {
                            let d = self
                                .error(msg::TYPE_NO_DISPLAY, expr.range)
                                .with("ty", self.show(&t))
                                .with("spec", spec.clone())
                                .note(msg::TYPE_SPEC_LETTERS, Vec::new());
                            self.report(d);
                        }
                    }
                    out.push(InterpPart::Value { expr: x, debug: debug.clone(), spec });
                }
            }
        }
        Expr::new(ExprKind::Interp(out), self.string_ty(), span)
    }

    fn tuple_expr(&mut self, items: &[ast::Expr], range: TextRange, expected: Option<&Ty>) -> Expr {
        let span = self.span(range);
        if items.is_empty() {
            return Expr::new(ExprKind::Unit, Ty::Unit, span);
        }
        let exp: Option<Vec<Ty>> = match expected.map(|t| self.resolved(t)) {
            Some(Ty::Tuple(ts, _)) if ts.len() == items.len() => Some(ts),
            _ => None,
        };
        let mut elems = Vec::new();
        for (i, it) in items.iter().enumerate() {
            let x = match &exp {
                Some(ts) => self.check_expr(it, &ts[i]),
                None => {
                    let x = self.infer_expr(it, None);
                    self.decay(x)
                }
            };
            elems.push(self.check_movable(x));
        }
        let labels = match expected.map(|t| self.resolved(t)) {
            Some(Ty::Tuple(ts, l)) if ts.len() == items.len() => l,
            _ => Vec::new(),
        };
        let tys = elems.iter().map(|x| x.ty.clone()).collect();
        Expr::new(ExprKind::Tuple(elems), Ty::Tuple(tys, labels), span)
    }

    /// Elements that share one type: the expected one, or the first's.
    fn homogeneous(&mut self, items: &[ast::Expr], elem: Option<Ty>) -> (Vec<Expr>, Ty) {
        let mut ty = elem;
        let mut out = Vec::new();
        for it in items {
            let x = match &ty {
                Some(t) => {
                    let t = t.clone();
                    self.check_expr(it, &t)
                }
                None => {
                    let x = self.infer_expr(it, None);
                    let x = self.decay(x);
                    ty = Some(x.ty.clone());
                    x
                }
            };
            out.push(self.check_movable(x));
        }
        let ty = match ty {
            Some(t) => t,
            None => self.infer.fresh(),
        };
        (out, ty)
    }

    // ---------------------------------------------------------------- names

    fn name_expr(&mut self, n: &str, range: TextRange, expected: Option<&Ty>) -> Expr {
        let span = self.span(range);
        if let Some(l) = self.lookup_local(n) {
            self.note_use(l, false);
            return Expr::new(ExprKind::Local(l), self.local_ty(l), span);
        }
        match n {
            "True" => return Expr::new(ExprKind::Bool(true), Ty::Bool, span),
            "False" => return Expr::new(ExprKind::Bool(false), Ty::Bool, span),
            "it" => {
                let d = self.error(msg::TYPE_IT_OUTSIDE_CLOSURE, range);
                self.report(d);
                return Expr::error(span);
            }
            _ => {}
        }
        // A `const` generic parameter is an `int` (03, "Const generics").
        if let Some(i) = self.generic_names.iter().position(|g| g == n)
            && self.const_generics.get(i) == Some(&true)
        {
            return Expr::new(ExprKind::ConstParam(i as u32), Ty::Int, span);
        }
        // A variant of the expected type wins over a declaration.
        if let Some(exp) = expected
            && let Some((def, vi)) = self.variant_in(exp, n)
        {
            return self.unit_variant(def, vi, Some(exp), range);
        }
        let prog = self.prog;
        if let Some(d) = prog.lookup(self.module, n) {
            return self.def_value(d, n, range);
        }
        if let Some((def, vi)) = self.global_variant(n, range) {
            return self.unit_variant(def, vi, expected, range);
        }
        if self.quiet_unknown_names || self.prog.poisoned.contains(n) {
            return Expr::error(span);
        }
        // The compile-time module `reflect` (08) exists only in a derive's
        // template and in a template, which is made for its type arguments.
        match n {
            "reflect" => {
                self.no_unfolding(range, "`reflect`");
                return Expr::error(span);
            }
            "target" => {
                self.unsupported(range, "`target`", "conditional compilation on the target (Phase 8)");
                return Expr::error(span);
            }
            _ => {}
        }
        // Another field of the struct whose default this is (E0221).
        if let Some(s) = self.field_default_of
            && let DefKind::Struct(info) = &self.prog.def(s).kind
            && info.fields.iter().any(|f| f.name == n)
        {
            let d = self
                .error(msg::RESOLVE_FIELD_DEFAULT_ORDER, range)
                .note(msg::RESOLVE_DEFAULT_IN_FUNCTION, vec![("name", n.to_string())]);
            self.report(d);
            return Expr::error(span);
        }
        let mut d = self.error(msg::RESOLVE_UNDEFINED_NAME, range).with("name", n);
        if let Some(similar) = self.similar_name(n) {
            let edit = sisu_diag::Edit { span, text: similar.clone() };
            d = d.fix(msg::RESOLVE_SIMILAR_NAME, vec![("name", similar)], vec![edit]);
        }
        self.report(d);
        Expr::error(span)
    }

    /// The local or module-level name closest to a misspelt one, if one is
    /// close enough to be the likely intent (an edit distance of at most a
    /// third of the length, and at least one).
    pub fn similar_name(&self, n: &str) -> Option<String> {
        let prog = self.prog;
        let mut names: Vec<String> = self.scopes.iter().flatten().map(|(name, _)| name.clone()).collect();
        if let Some(m) = prog.modules.get(self.module.0 as usize) {
            names.extend(m.items.keys().cloned());
        }
        if let Some(m) = prog.modules.get(prog.prelude.0 as usize) {
            names.extend(m.items.keys().filter(|k| !k.starts_with('_')).cloned());
        }
        // The literals spelt as keywords: `true` is `True`.
        names.extend(["True", "False"].map(String::from));
        closest(n, names)
    }

    /// A declaration used as a value.
    pub fn def_value(&mut self, d: DefId, n: &str, range: TextRange) -> Expr {
        let span = self.span(range);
        let prog = self.prog;
        match &prog.def(d).kind {
            DefKind::Fn(f) => {
                // A generic intrinsic has no body to make an instance of.
                let generic_intrinsic = !f.generics.is_empty() && f.intrinsic.is_some();
                if f.self_mode.is_some() || generic_intrinsic {
                    let kind = if f.self_mode.is_some() { "method" } else { "generic function" };
                    let diag = self.error(msg::TYPE_NOT_A_VALUE, range).with("name", n).with("kind", kind);
                    self.report(diag);
                    return Expr::error(span);
                }
                if f.comptime && !self.in_comptime {
                    let diag = self.error(msg::COMPTIME_ONLY, range).with("name", n);
                    self.report(diag);
                }
                self.check_internal(d, range);
                // A function value takes its arguments by value, so a
                // function that returns a borrow of a parameter it borrows
                // cannot be one (decision 225).
                let env = prog.elaborate_env(&f.predicates);
                let deep = prog.result_borrows_of(f).deep;
                let lent_and_kept = f.params.iter().enumerate().find(|(i, p)| {
                    deep.contains(&(*i as u32))
                        && !matches!(p.ty, Ty::Ref(..))
                        && matches!(crate::param_local_ty(prog, &p.ty, p.mode, &env), Ty::Ref(..))
                });
                if let Some((_, p)) = lent_and_kept {
                    let diag = self
                        .error(msg::TYPE_VALUE_RETURNS_BORROW, range)
                        .with("name", n)
                        .with("param", p.name.clone());
                    self.report(diag);
                    return Expr::error(span);
                }
                // A generic function's type arguments are inferred where the
                // value is used, and its bounds required of them, as a
                // call's are (decision 225).
                if !f.generics.is_empty() {
                    let sig = self.instantiate(d, None, range);
                    let ty = Ty::Fn(Box::new(FnTy {
                        params: sig.params.iter().map(|p| p.ty.clone()).collect(),
                        ret: sig.ret,
                        pure: f.pure,
                    }));
                    return Expr::new(ExprKind::FnRef(d, sig.type_args), ty, span);
                }
                let ret = self.fn_ret(d);
                let ty = Ty::Fn(Box::new(FnTy {
                    params: f.params.iter().map(|p| p.ty.clone()).collect(),
                    ret,
                    pure: f.pure,
                }));
                Expr::new(ExprKind::FnRef(d, Vec::new()), ty, span)
            }
            DefKind::Const(c) => Expr::new(ExprKind::Const(d), c.ty.clone(), span),
            other => {
                let kind = match other {
                    DefKind::Struct(_)
                    | DefKind::Enum(_)
                    | DefKind::ErrorSet(_)
                    | DefKind::TypeAlias(_)
                    | DefKind::Refinement(_) => "type",
                    DefKind::Module(_) => "module",
                    _ => "declaration",
                };
                let diag = self.error(msg::TYPE_NOT_A_VALUE, range).with("name", n).with("kind", kind);
                self.report(diag);
                Expr::error(span)
            }
        }
    }

    /// The result type of a function, with its inferred error set filled in.
    pub fn fn_ret(&self, d: DefId) -> Ty {
        let Some(f) = self.prog.fn_info(d) else { return Ty::Error };
        if f.err_inferred
            && let Some((ok, _)) = match &f.ret {
                Ty::Adt(r, a) if Some(*r) == self.prog.wk.result && a.len() == 2 => {
                    Some((a[0].clone(), a[1].clone()))
                }
                _ => None,
            }
        {
            let set = self.inferred.get(&d).cloned().unwrap_or_default();
            let mut reads = self.inferred_reads.borrow_mut();
            if !reads.contains(&d) {
                reads.push(d);
            }
            drop(reads);
            return self.prog.wk.result.map(|r| Ty::Adt(r, vec![ok, Ty::ErrSet(set)])).unwrap_or(Ty::Error);
        }
        f.ret.clone()
    }

    /// The variant `name` of an expected enum, error set, option or result.
    pub fn variant_in(&self, expected: &Ty, name: &str) -> Option<(DefId, u32)> {
        let prog = self.prog;
        let defs: Vec<DefId> = match self.resolved(expected) {
            Ty::Adt(d, _) => vec![d],
            Ty::ErrSet(m) => m,
            _ => return None,
        };
        for d in defs {
            if let Some(e) = prog.enum_info(d)
                && let Some(i) = e.variants.iter().position(|v| v.name == name)
            {
                return Some((d, i as u32));
            }
        }
        None
    }

    /// A variant found by its unqualified name alone; ambiguity is reported.
    pub fn global_variant(&mut self, name: &str, range: TextRange) -> Option<(DefId, u32)> {
        let prog = self.prog;
        let candidates = prog.variants.get(name)?;
        let visible: Vec<(DefId, u32)> = candidates
            .iter()
            .copied()
            .filter(|(d, _)| {
                let m = prog.def(*d).module;
                m == self.module
                    || m == prog.prelude
                    || prog.modules[self.module.0 as usize]
                        .items
                        .values()
                        .any(|x| matches!(prog.def(*x).kind, DefKind::Module(mm) if mm == m))
            })
            .collect();
        // A variant declared in this module wins over one from the prelude
        // or an imported module.
        let local: Vec<(DefId, u32)> =
            visible.iter().copied().filter(|(d, _)| prog.def(*d).module == self.module).collect();
        let visible = if local.len() == 1 { local } else { visible };
        match visible.len() {
            0 => None,
            1 => Some(visible[0]),
            _ => {
                let names: Vec<String> = visible.iter().map(|(d, _)| prog.qualified_name(*d)).collect();
                let example = format!("{}.{name}", names[0]);
                let d = self
                    .error(msg::RESOLVE_AMBIGUOUS_VARIANT, range)
                    .with("name", name)
                    .with("candidates", names.join(", "))
                    .with("example", example);
                self.report(d);
                Some(visible[0])
            }
        }
    }

    /// The type of a value of an enum or error-set definition, with fresh
    /// arguments for a generic enum (unified with `expected` if given).
    pub fn variant_owner_ty(&mut self, def: DefId, expected: Option<&Ty>) -> (Ty, Vec<Ty>) {
        let prog = self.prog;
        if prog.is_error_set(def) {
            return (Ty::ErrSet(vec![def]), Vec::new());
        }
        let n = prog.enum_info(def).map(|e| e.generics.len()).unwrap_or(0);
        let args: Vec<Ty> = (0..n).map(|_| self.infer.fresh()).collect();
        let ty = Ty::Adt(def, args.clone());
        if let Some(exp) = expected {
            let exp = self.resolved(exp);
            if matches!(&exp, Ty::Adt(d, _) if *d == def) {
                self.infer.try_unify(&ty, &exp);
            }
        }
        (ty, args)
    }

    fn unit_variant(&mut self, def: DefId, vi: u32, expected: Option<&Ty>, range: TextRange) -> Expr {
        let span = self.span(range);
        let prog = self.prog;
        let name = prog
            .enum_info(def)
            .and_then(|e| e.variants.get(vi as usize))
            .map(|v| v.name.clone())
            .unwrap_or_default();
        let has_fields = prog
            .enum_info(def)
            .and_then(|e| e.variants.get(vi as usize))
            .is_some_and(|v| !v.fields.is_empty());
        if has_fields {
            let d = self.error(msg::TYPE_VARIANT_NEEDS_FIELDS, range).with("name", name);
            self.report(d);
            return Expr::error(span);
        }
        let (ty, _) = self.variant_owner_ty(def, expected);
        Expr::new(ExprKind::Variant { def, variant: vi, args: Vec::new() }, ty, span)
    }

    // ---------------------------------------------------------------- operators

    fn binary_expr(
        &mut self,
        op: BinOp,
        lhs: &ast::Expr,
        rhs: &ast::Expr,
        range: TextRange,
        expected: Option<&Ty>,
    ) -> Expr {
        let span = self.span(range);
        match op {
            BinOp::And | BinOp::Or => {
                let l = self.check_condition_expr(lhs);
                let r = self.check_condition_expr(rhs);
                return Expr::new(
                    ExprKind::Logic { and: op == BinOp::And, lhs: Box::new(l), rhs: Box::new(r) },
                    Ty::Bool,
                    span,
                );
            }
            BinOp::Coalesce => return self.coalesce(lhs, rhs, range, expected),
            _ => {}
        }
        let hint = expected.map(|t| self.resolved(t)).filter(|t| self.is_number(t));
        let l = self.infer_expr(lhs, hint.as_ref());
        let l = self.decay(l);
        let l = self.unrefined(l);
        let lt = self.resolved(&l.ty);
        let rhint = if self.is_number(&lt) { Some(lt.clone()) } else { None };
        let r = self.infer_expr(rhs, rhint.as_ref());
        // A reference to what is not known yet, against a number, reads a
        // number (`n % it` over `found.iter()` before `found` is known).
        if let Some(num) = &rhint
            && let Ty::Ref(inner, _) = self.resolved(&r.ty)
            && matches!(self.resolved(&inner), Ty::Var(_))
        {
            self.infer.unify(&inner, num);
        }
        let r = self.decay(r);
        let r = self.unrefined(r);
        // A literal on the left takes the type of a float on the right.
        let l = match (&l.kind, self.resolved(&r.ty)) {
            (ExprKind::Int(n), Ty::Float) => Expr::new(ExprKind::Float(*n as f64), Ty::Float, l.span),
            (ExprKind::Int(_) | ExprKind::Float(_), rt)
                if self.float_kind(&rt) == Some("f32")
                    && matches!(self.resolved(&l.ty), Ty::Int | Ty::Float) =>
            {
                self.infer_expr(lhs, Some(&rt))
            }
            _ => l,
        };
        // A literal on the left takes the sized integer type on the right.
        let rt = self.resolved(&r.ty);
        let l = match self.int_kind(&rt) {
            Some(k)
                if k != "int" && matches!(l.kind, ExprKind::Int(_)) && self.resolved(&l.ty) == Ty::Int =>
            {
                self.infer_expr(lhs, Some(&rt))
            }
            _ => l,
        };
        // Integers of two types: the narrower widens when that loses
        // nothing (03, "Numbers").
        let (l, r) = match (self.int_kind(&l.ty), self.int_kind(&r.ty)) {
            (Some(a), Some(b)) if a != b && sisu_resolve::std_sources::widens(a, b) => {
                let to = self.resolved(&r.ty);
                (self.wrap(l, Coercion::IntWiden, to), r)
            }
            (Some(a), Some(b)) if a != b && sisu_resolve::std_sources::widens(b, a) => {
                let to = self.resolved(&l.ty);
                let r = self.wrap(r, Coercion::IntWiden, to);
                (l, r)
            }
            _ => match (self.float_kind(&l.ty), self.float_kind(&r.ty)) {
                // An `f32` beside an `f64` widens, as an integer does.
                (Some("f32"), Some("f64")) => (self.wrap(l, Coercion::FloatWiden, Ty::Float), r),
                (Some("f64"), Some("f32")) => {
                    let r = self.wrap(r, Coercion::FloatWiden, Ty::Float);
                    (l, r)
                }
                _ => (l, r),
            },
        };
        if let Some(call) = self.operator_call(op, l.clone(), Some(r.clone()), range) {
            return call;
        }
        let (ty, _) = self.binary_result(op, &l, &r, range);
        let (l, r) = if ty == Ty::Str || self.is_string(&ty) {
            // String concatenation views both sides as `str`.
            let l = self.coerce(l, &Ty::Str);
            let r = self.coerce(r, &Ty::Str);
            (l, r)
        } else {
            (l, r)
        };
        Expr::new(ExprKind::Binary { op, lhs: Box::new(l), rhs: Box::new(r) }, ty, span)
    }

    /// The trait behind an operator of the fixed set (D-022), and its method.
    pub fn operator_trait(&self, op: BinOp) -> Option<(sisu_resolve::DefId, &'static str)> {
        let wk = &self.prog.wk;
        match op {
            BinOp::Add => wk.add.map(|d| (d, "add")),
            BinOp::Sub => wk.sub.map(|d| (d, "sub")),
            BinOp::Mul => wk.mul.map(|d| (d, "mul")),
            BinOp::Div => wk.div.map(|d| (d, "div")),
            BinOp::Mod => wk.rem.map(|d| (d, "rem")),
            _ => None,
        }
    }

    /// `l op r` (or `-l`, for `r` = `None`) on a type that is not a number
    /// or text: a call of the operator's trait method, when the type
    /// implements the trait. `None` leaves it to the builtin operators.
    pub fn operator_call(&mut self, op: BinOp, l: Expr, r: Option<Expr>, range: TextRange) -> Option<Expr> {
        let prog = self.prog;
        let base =
            |c: &Self, t: &Ty| -> Ty { prog.normalize(&c.infer.zonk(c.resolved(t).strip_ref()), &c.env) };
        let lt = base(self, &l.ty);
        let builtin = |c: &Self, t: &Ty| {
            matches!(t, Ty::Int | Ty::Float | Ty::Str | Ty::Var(_) | Ty::Error | Ty::Never)
                || c.is_string(t)
                || c.float_kind(t).is_some()
        };
        if builtin(self, &lt) {
            return None;
        }
        let (tdef, method) = match &r {
            Some(_) => self.operator_trait(op)?,
            None => (prog.wk.neg?, "neg"),
        };
        let rt = r.as_ref().map(|r| base(self, &r.ty));
        let tr = sisu_resolve::TraitRef { def: tdef, args: rt.iter().cloned().collect() };
        if prog.implements(&lt, &tr, &self.env) == sisu_resolve::Answer::No {
            return None;
        }
        let m = prog.trait_method(tdef, method)?;
        let span = self.span(range);
        let sig = self.instantiate(m, None, range);
        if let Some(v) = sig.type_args.first() {
            self.infer.try_unify(v, &lt);
        }
        if let (Some(v), Some(rt)) = (sig.type_args.get(1), &rt) {
            self.infer.try_unify(v, rt);
        }
        let as_ref = |c: &Self, x: Expr| match c.resolved(&x.ty) {
            Ty::Ref(..) => Arg { expr: x, mode: ArgMode::Value },
            _ => Arg { expr: x, mode: ArgMode::Ref },
        };
        let mut args = vec![as_ref(self, l)];
        if let Some(r) = r {
            args.push(as_ref(self, r));
        }
        let ret = prog.normalize(&self.infer.zonk(&sig.ret), &self.env);
        Some(Expr::new(
            ExprKind::Call { callee: Callee::Fn { def: m, type_args: sig.type_args.clone() }, args },
            ret,
            span,
        ))
    }

    /// The type of `lhs op rhs`, reporting a misuse.
    pub fn binary_result(&mut self, op: BinOp, l: &Expr, r: &Expr, range: TextRange) -> (Ty, bool) {
        let lt = self.resolved(&l.ty);
        let rt = self.resolved(&r.ty);
        if lt == Ty::Error || rt == Ty::Error {
            return (Ty::Error, false);
        }
        // Unify variables with the other side when one side is known.
        if matches!(lt, Ty::Var(_)) && !matches!(rt, Ty::Var(_)) {
            self.infer.unify(&lt, &rt);
        } else if matches!(rt, Ty::Var(_)) && !matches!(lt, Ty::Var(_)) {
            self.infer.unify(&rt, &lt);
        }
        let lt = self.resolved(&lt);
        let rt = self.resolved(&rt);
        let is_text = |c: &Checker, t: &Ty| *t == Ty::Str || c.is_string(t);
        // Two values of one sized integer type: what `int` allows.
        if let (Some(a), Some(b)) = (self.int_kind(&lt), self.int_kind(&rt))
            && a == b
            && a != "int"
        {
            return match op {
                BinOp::Div => {
                    let d = self.error(msg::TYPE_INTEGER_DIVISION, range);
                    self.report(d);
                    (lt.clone(), false)
                }
                BinOp::And | BinOp::Or | BinOp::Coalesce => (Ty::Error, false),
                _ => (lt.clone(), true),
            };
        }
        // Two `f32`s: what two `f64`s allow, giving an `f32`.
        if self.float_kind(&lt) == Some("f32") && self.float_kind(&rt) == Some("f32") {
            match op {
                BinOp::FloorDiv => {
                    let d = self.error(msg::TYPE_FLOAT_FLOOR_DIVISION, range);
                    self.report(d);
                    return (lt.clone(), false);
                }
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod | BinOp::Pow => {
                    return (lt.clone(), true);
                }
                _ => {}
            }
        }
        let result = match (op, &lt, &rt) {
            (BinOp::Div, Ty::Int, Ty::Int) => {
                let op_span = self.op_span(l, r, "/");
                let edit = Edit { span: op_span, text: "//".into() };
                let d = self.error(msg::TYPE_INTEGER_DIVISION, range).fix(
                    msg::TYPE_USE_FLOOR_DIVISION,
                    Vec::new(),
                    vec![edit],
                );
                self.report(d);
                return (Ty::Int, false);
            }
            (BinOp::FloorDiv, Ty::Float, Ty::Float) => {
                let d = self.error(msg::TYPE_FLOAT_FLOOR_DIVISION, range);
                self.report(d);
                return (Ty::Float, false);
            }
            (
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::FloorDiv | BinOp::Mod | BinOp::Pow,
                Ty::Int,
                Ty::Int,
            ) => Some(Ty::Int),
            (
                BinOp::WrapAdd
                | BinOp::WrapSub
                | BinOp::WrapMul
                | BinOp::SatAdd
                | BinOp::SatSub
                | BinOp::SatMul,
                Ty::Int,
                Ty::Int,
            ) => Some(Ty::Int),
            (BinOp::Shl | BinOp::Shr | BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor, Ty::Int, Ty::Int) => {
                Some(Ty::Int)
            }
            (
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod | BinOp::Pow,
                Ty::Float,
                Ty::Float,
            ) => Some(Ty::Float),
            (BinOp::Add, a, b) if is_text(self, a) && is_text(self, b) => Some(self.string_ty()),
            (_, a, b)
                if (*a == Ty::Int && self.float_kind(b).is_some())
                    || (self.float_kind(a).is_some() && *b == Ty::Int) =>
            {
                // The fix converts the `int` side: a literal gets its `.0`,
                // anything else `.to_f64()` (or `.to_f32()`).
                let (int_side, float) = if lt == Ty::Int { (l, &rt) } else { (r, &lt) };
                let kind = self.float_kind(float).unwrap_or("f64");
                let d = self
                    .error(msg::TYPE_MIXED_NUMBERS, range)
                    .with("op", op.text())
                    .with("lhs", self.show(&lt))
                    .with("rhs", self.show(&rt))
                    .fix(
                        msg::TYPE_CONVERT_TO_FLOAT,
                        vec![("ty", kind.to_string())],
                        self.to_float_edits(int_side, kind),
                    );
                self.report(d);
                return (Ty::Error, false);
            }
            _ => None,
        };
        match result {
            Some(t) => (t, true),
            None => {
                let mut d = self
                    .error(msg::TYPE_BINARY_OPERATOR, range)
                    .with("op", op.text())
                    .with("lhs", self.show(&lt))
                    .with("rhs", self.show(&rt));
                // Text and a number, a `bool` or a `char` joined by `+`:
                // interpolation writes the value into the text.
                let shown = |t: &Ty| matches!(t, Ty::Int | Ty::Bool | Ty::Char) || self.is_number(t);
                if op == BinOp::Add
                    && ((is_text(self, &lt) && shown(&rt)) || (shown(&lt) && is_text(self, &rt)))
                    && let Some(text) = self.interpolated(l, r)
                {
                    let edit = Edit { span: self.span(range), text };
                    d = d.fix(msg::TYPE_INTERPOLATE, Vec::new(), vec![edit]);
                }
                // An optional value used as the value it may hold.
                if self.is_option(&lt) || self.is_option(&rt) {
                    d = d.note(msg::TYPE_OPTION_NOT_VALUE, Vec::new());
                }
                self.report(d);
                (Ty::Error, false)
            }
        }
    }

    /// The edits that make an `int` operand a float of kind `kind` (`f64`
    /// or `f32`): `2` becomes `2.0`, a name or a call gets `.to_f64()` (or
    /// `.to_f32()`), anything else is parenthesised first.
    fn to_float_edits(&self, x: &Expr, kind: &str) -> Vec<Edit> {
        let r = x.span.range;
        let text = self.source.get(r.start as usize..r.end as usize).unwrap_or("");
        if !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit() || b == b'_') {
            return vec![Edit { span: Span::new(self.file, TextRange::empty(r.end)), text: ".0".into() }];
        }
        let simple = text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'(' | b')' | b'[' | b']'))
            && text.bytes().next().is_some_and(|b| b.is_ascii_alphabetic() || b == b'_');
        if simple {
            vec![Edit { span: Span::new(self.file, TextRange::empty(r.end)), text: format!(".to_{kind}()") }]
        } else {
            vec![
                Edit { span: Span::new(self.file, TextRange::empty(r.start)), text: "(".into() },
                Edit { span: Span::new(self.file, TextRange::empty(r.end)), text: format!(").to_{kind}()") },
            ]
        }
    }

    /// `"text" + x` (or `x + "text"`) as one interpolated string, when the
    /// text side is a plain string literal.
    fn interpolated(&self, l: &Expr, r: &Expr) -> Option<String> {
        let src = |x: &Expr| self.source.get(x.span.range.start as usize..x.span.range.end as usize);
        let literal = |t: &str| -> Option<String> {
            let inner = t.strip_prefix('"')?.strip_suffix('"')?;
            (!inner.contains('"') && !inner.contains('{')).then(|| inner.to_string())
        };
        let (a, b) = (src(l)?, src(r)?);
        match (literal(a), literal(b)) {
            (Some(text), None) => Some(format!("\"{text}{{{b}}}\"")),
            (None, Some(text)) => Some(format!("\"{{{a}}}{text}\"")),
            _ => None,
        }
    }

    /// The span of the operator between two operands, for a fix-it.
    fn op_span(&self, l: &Expr, r: &Expr, op: &str) -> Span {
        let between = self.source.get(l.span.range.end as usize..r.span.range.start as usize).unwrap_or("");
        match between.find(op) {
            Some(i) => {
                let start = l.span.range.end + i as u32;
                Span::new(self.file, TextRange::new(start, start + op.len() as u32))
            }
            None => Span::new(self.file, TextRange::empty(l.span.range.end)),
        }
    }

    fn unary_expr(&mut self, op: UnOp, inner: &ast::Expr, range: TextRange, expected: Option<&Ty>) -> Expr {
        let span = self.span(range);
        if op == UnOp::Neg {
            if let ast::ExprKind::Int { value, suffix, text } = &inner.kind {
                let x = self.int_literal(*value, suffix.as_deref(), text, true, range, expected);
                return if suffix.is_none() { self.literal_via_trait(x, expected) } else { x };
            }
            if let ast::ExprKind::Float { value, suffix, text } = &inner.kind {
                let x = self.float_literal(*value, suffix.as_deref(), text, true, range, expected);
                return if suffix.is_none() { self.literal_via_trait(x, expected) } else { x };
            }
        }
        let x = match op {
            UnOp::Not => self.check_condition_expr(inner),
            _ => {
                let hint = expected
                    .map(|t| self.resolved(t))
                    .filter(|t| matches!(t, Ty::Int) || self.float_kind(t).is_some());
                let x = self.infer_expr(inner, hint.as_ref());
                let x = self.decay(x);
                self.unrefined(x)
            }
        };
        if op == UnOp::Neg
            && let Some(call) = self.operator_call(BinOp::Sub, x.clone(), None, range)
        {
            return call;
        }
        let t = self.resolved(&x.ty);
        let ty = match (op, &t) {
            (_, Ty::Error) => Ty::Error,
            (UnOp::Not, _) => Ty::Bool,
            (UnOp::Neg, Ty::Int | Ty::Float) => t.clone(),
            (UnOp::Neg, t) if self.float_kind(t).is_some() => t.clone(),
            (UnOp::BitNot, Ty::Int) => Ty::Int,
            (UnOp::BitNot, t) if self.int_kind(t).is_some() => t.clone(),
            _ => {
                let d = self
                    .error(msg::TYPE_UNARY_OPERATOR, range)
                    .with("op", op.text())
                    .with("ty", self.show(&t));
                self.report(d);
                Ty::Error
            }
        };
        Expr::new(ExprKind::Unary { op, operand: Box::new(x) }, ty, span)
    }

    fn compare_expr(&mut self, first: &ast::Expr, rest: &[(CmpOp, ast::Expr)], range: TextRange) -> Expr {
        let span = self.span(range);
        let f = self.infer_expr(first, None);
        let f = self.unrefined(f);
        let mut prev = f.clone();
        let mut out = Vec::new();
        for (op, e) in rest {
            let pt = self.decay_ty(&prev.ty);
            let hint = if matches!(op, CmpOp::In | CmpOp::NotIn) { None } else { Some(pt.clone()) };
            let x = self.infer_expr(e, hint.as_ref());
            let x = self.unrefined(x);
            self.check_comparison(*op, &prev, &x, range);
            prev = x.clone();
            out.push((*op, x));
        }
        Expr::new(ExprKind::Compare { first: Box::new(f), rest: out }, Ty::Bool, span)
    }

    fn check_comparison(&mut self, op: CmpOp, l: &Expr, r: &Expr, range: TextRange) {
        let lt = self.infer.zonk(&l.ty);
        let rt = self.infer.zonk(&r.ty);
        if lt.contains_error() || rt.contains_error() {
            return;
        }
        let strip = |c: &Checker, t: &Ty| -> Ty {
            let t = c.resolved(t.strip_ref());
            if c.is_string(&t) { Ty::Str } else { t }
        };
        match op {
            CmpOp::In | CmpOp::NotIn => {
                let container = self.resolved(rt.strip_ref());
                let elem = match &container {
                    Ty::Adt(d, a)
                        if Some(*d) == self.prog.wk.list
                            || Some(*d) == self.prog.wk.set
                            || self.prog.builtin(*d) == Some("Array") =>
                    {
                        a.first().cloned()
                    }
                    Ty::Adt(d, a) if Some(*d) == self.prog.wk.dict => a.first().cloned(),
                    Ty::Str => Some(Ty::Str),
                    t if self.is_string(t) => Some(Ty::Str),
                    _ => None,
                };
                match elem {
                    Some(el) => {
                        let a = strip(self, &lt);
                        let b = strip(self, &el);
                        let span = self.span(range);
                        self.require(b.clone(), self.prog.wk.eq, span);
                        // A `set` or `dict` finds the key by its hash.
                        if matches!(&container, Ty::Adt(d, _) if Some(*d) == self.prog.wk.set || Some(*d) == self.prog.wk.dict)
                        {
                            self.require(b.clone(), self.prog.wk.hash, span);
                        }
                        if !self.infer.try_unify(&a, &b) {
                            let d = self
                                .error(msg::TYPE_COMPARISON_TYPES, range)
                                .with("lhs", self.show(&lt))
                                .with("rhs", self.show(&el));
                            self.report(d);
                        }
                    }
                    None => {
                        let d = self
                            .error(msg::TYPE_BINARY_OPERATOR, range)
                            .with("op", op.text())
                            .with("lhs", self.show(&lt))
                            .with("rhs", self.show(&rt));
                        self.report(d);
                    }
                }
            }
            _ => {
                let a = strip(self, &lt);
                let b = strip(self, &rt);
                if !self.infer.try_unify(&a, &b) {
                    let d = self
                        .error(msg::TYPE_COMPARISON_TYPES, range)
                        .with("lhs", self.show(&lt))
                        .with("rhs", self.show(&rt));
                    self.report(d);
                    return;
                }
                if matches!(op, CmpOp::Lt | CmpOp::Le | CmpOp::Gt | CmpOp::Ge) {
                    let t = self.prog.normalize(&self.infer.zonk(&a), &self.env);
                    let ordered = match self.prog.wk.partial_ord {
                        Some(def) => {
                            let tr = sisu_resolve::TraitRef { def, args: Vec::new() };
                            !matches!(self.prog.implements(&t, &tr, &self.env), sisu_resolve::Answer::No)
                        }
                        None => true,
                    };
                    if !ordered {
                        let mut d = self
                            .error(msg::TYPE_NOT_ORDERED, range)
                            .with("ty", self.show(&t))
                            .with("op", op.text());
                        if let Ty::Param(i) = t
                            && let Some((name, edit)) = self.bound_edit(i, "PartialOrd")
                        {
                            d = d.fix(
                                msg::TYPE_ADD_BOUND,
                                vec![("param", name), ("bound", "PartialOrd".into())],
                                vec![edit],
                            );
                        }
                        self.report(d);
                    }
                } else if matches!(self.resolved(&a), Ty::Fn(_)) {
                    let d = self
                        .error(msg::TYPE_COMPARISON_TYPES, range)
                        .with("lhs", self.show(&lt))
                        .with("rhs", self.show(&rt));
                    self.report(d);
                } else {
                    let span = self.span(range);
                    self.require(a.clone(), self.prog.wk.eq, span);
                }
            }
        }
    }

    fn coalesce(
        &mut self,
        lhs: &ast::Expr,
        rhs: &ast::Expr,
        range: TextRange,
        expected: Option<&Ty>,
    ) -> Expr {
        let span = self.span(range);
        // An open expected type (a generic call's parameter) says nothing
        // yet, and binding it to the option's payload would forbid the
        // copy a `ref` payload of a `Copy` type decays to.
        let opt_hint =
            expected.filter(|t| !matches!(self.resolved(t), Ty::Var(_))).map(|t| self.option_of(t.clone()));
        let o = self.infer_expr(lhs, opt_hint.as_ref());
        let Some(inner) = self.as_option(&o.ty) else {
            if o.ty != Ty::Error {
                let d = self.error(msg::TYPE_COALESCE, lhs.range).with("ty", self.show(&o.ty));
                self.report(d);
            }
            let r = self.infer_expr(rhs, None);
            return Expr::new(
                ExprKind::Coalesce { opt: Box::new(o), default: Box::new(r.clone()), deref: false },
                r.ty,
                span,
            );
        };
        let inner_r = self.resolved(&inner);
        // `a ?? b ?? c`: an optional default keeps the option.
        let d = self.infer_expr(rhs, Some(&inner_r));
        if self.as_option(&d.ty).is_some() && self.infer.try_unify(&d.ty, &o.ty) {
            let ty = o.ty.clone();
            return Expr::new(
                ExprKind::Coalesce { opt: Box::new(o), default: Box::new(d), deref: false },
                ty,
                span,
            );
        }
        if let Ty::Ref(target, _) = &inner_r {
            if self.is_copy(target) {
                let target = (**target).clone();
                let d = self.coerce(d, &target);
                return Expr::new(
                    ExprKind::Coalesce { opt: Box::new(o), default: Box::new(d), deref: true },
                    target,
                    span,
                );
            }
            if self.is_string(target) && self.resolved(&d.ty) == Ty::Str {
                let view = self.option_of(Ty::Str);
                let o2 = self.wrap(o, Coercion::Mismatch, view);
                let _ = o2;
                let d = self
                    .error(msg::TYPE_MISMATCH, rhs.range)
                    .with("expected", self.show(&inner_r))
                    .with("found", "str");
                self.report(d);
                return Expr::error(span);
            }
        }
        let d = self.coerce(d, &inner_r);
        Expr::new(ExprKind::Coalesce { opt: Box::new(o), default: Box::new(d), deref: false }, inner_r, span)
    }

    // ---------------------------------------------------------------- collections

    /// The items a `for` loop over a value of type `ty` yields, and how.
    pub fn iteration_item(&mut self, ty: &Ty, mode: ast::Mode) -> Option<(Ty, ForMode)> {
        let prog = self.prog;
        let t = self.resolved(ty.strip_ref());
        if let Some((_, _, base)) = self.refinement_of(&t) {
            let b = if let Ty::Ref(_, m) = self.resolved(ty) { Ty::Ref(Box::new(base), m) } else { base };
            return self.iteration_item(&b, mode);
        }
        let behind_ref = matches!(self.resolved(ty), Ty::Ref(..));
        let Ty::Adt(d, args) = &t else {
            return if t == Ty::Error { Some((Ty::Error, ForMode::Borrow)) } else { self.iterator_item(&t) };
        };
        let fmode = match mode {
            ast::Mode::Borrow => ForMode::Borrow,
            ast::Mode::Mut => ForMode::Mut,
            ast::Mode::Own => ForMode::Own,
        };
        let wrap = |t: Ty, m: ForMode| match m {
            ForMode::Borrow => Ty::Ref(Box::new(t), false),
            ForMode::Mut => Ty::Ref(Box::new(t), true),
            _ => t,
        };
        match prog.builtin(*d) {
            Some("list") | Some("set") | Some("Array") => {
                let el = args.first().cloned().unwrap_or(Ty::Error);
                let m = if behind_ref && fmode == ForMode::Own { ForMode::Borrow } else { fmode };
                Some((wrap(el, m), m))
            }
            Some("dict") => {
                let k = args.first().cloned().unwrap_or(Ty::Error);
                let v = args.get(1).cloned().unwrap_or(Ty::Error);
                let m = if behind_ref && fmode == ForMode::Own { ForMode::Borrow } else { fmode };
                let item = match m {
                    ForMode::Own => Ty::tuple(vec![k, v]),
                    ForMode::Mut => Ty::tuple(vec![Ty::Ref(Box::new(k), false), Ty::Ref(Box::new(v), true)]),
                    _ => Ty::tuple(vec![Ty::Ref(Box::new(k), false), Ty::Ref(Box::new(v), false)]),
                };
                Some((item, m))
            }
            Some("Iter") => Some((args.first().cloned().unwrap_or(Ty::Error), ForMode::Iter)),
            // A receiver gives its values until the channel is closed and
            // drained; the loop borrows it.
            Some("Receiver") => Some((args.first().cloned().unwrap_or(Ty::Error), ForMode::Borrow)),
            _ => self.iterator_item(&t),
        }
    }

    /// The item of a type that implements `Iterator`, which a `for` loop
    /// drives through the trait.
    fn iterator_item(&self, t: &Ty) -> Option<(Ty, ForMode)> {
        let prog = self.prog;
        let def = prog.wk.iterator?;
        let tr = sisu_resolve::TraitRef { def, args: Vec::new() };
        let t = prog.normalize(&self.infer.zonk(t), &self.env);
        if !prog.implements(&t, &tr, &self.env).holds() {
            return None;
        }
        let item = Ty::Assoc(Box::new(sisu_resolve::AssocTy { self_ty: t, trait_: tr, name: "Item".into() }));
        Some((prog.normalize(&item, &self.env), ForMode::Trait))
    }

    /// The iterable of a `for` loop or comprehension clause.
    pub fn check_iterable(&mut self, iter: &ast::Expr, mode: ast::Mode) -> (Expr, ForMode, Ty) {
        let x = self.infer_expr(iter, None);
        // `bytes` and a `Slice` are iterated through their iterators.
        let base = self.resolved(x.ty.strip_ref());
        let x = match &base {
            Ty::Adt(d, a) if matches!(self.prog.builtin(*d), Some("bytes" | "Slice")) => {
                let item = match self.prog.builtin(*d) {
                    Some("bytes") => self.sized_ty("u8"),
                    _ => Ty::Ref(Box::new(a.first().cloned().unwrap_or(Ty::Error)), false),
                };
                let iter_ty = self.prog.wk.iter.map(|i| Ty::Adt(i, vec![item])).unwrap_or(Ty::Error);
                self.builtin_call(x.clone(), &base, "iter", iter_ty).unwrap_or(x)
            }
            _ => x,
        };
        match self.iteration_item(&x.ty, mode) {
            Some((item, fmode)) => {
                let x = match fmode {
                    ForMode::Own | ForMode::Iter | ForMode::Trait => self.check_movable(x),
                    ForMode::Mut => {
                        self.require_mutable(&x, iter.range);
                        x
                    }
                    ForMode::Borrow => x,
                };
                (x, fmode, item)
            }
            None => {
                if !matches!(self.resolved(&x.ty), Ty::Var(_)) {
                    let d = self.error(msg::TYPE_NOT_ITERABLE, iter.range).with("ty", self.show(&x.ty));
                    self.report(d);
                } else {
                    let d = self
                        .error(msg::TYPE_CANNOT_INFER, iter.range)
                        .with("name", self.source_text(iter.range));
                    self.report(d);
                }
                (x, ForMode::Borrow, Ty::Error)
            }
        }
    }

    fn comprehension(
        &mut self,
        kind: ast::CompKind,
        elem: &ast::Expr,
        value: Option<&ast::Expr>,
        clauses: &[ast::CompClause],
        range: TextRange,
    ) -> Expr {
        let span = self.span(range);
        self.push_scope();
        let mut out = Vec::new();
        for c in clauses {
            match c {
                ast::CompClause::For { pat, iter } => {
                    let (iter, mode, item) = self.check_iterable(iter, ast::Mode::Borrow);
                    let p = self.check_pat(pat, &item, None);
                    out.push(CompClause::For { pat: p, iter, mode, item });
                }
                ast::CompClause::If(e) => {
                    let c = self.check_condition_expr(e);
                    out.push(CompClause::If(c));
                }
            }
        }
        let el = self.infer_expr(elem, None);
        let el = self.decay(el);
        let el = self.check_movable(el);
        let val = value.map(|v| {
            let x = self.infer_expr(v, None);
            let x = self.decay(x);
            self.check_movable(x)
        });
        self.pop_scope();
        let prog = self.prog;
        let (kind, ty) = match (kind, &val) {
            (ast::CompKind::Dict, Some(v)) => (
                CompKind::Dict,
                prog.wk.dict.map(|d| Ty::Adt(d, vec![el.ty.clone(), v.ty.clone()])).unwrap_or(Ty::Error),
            ),
            (ast::CompKind::Set, _) => {
                (CompKind::Set, prog.wk.set.map(|d| Ty::Adt(d, vec![el.ty.clone()])).unwrap_or(Ty::Error))
            }
            (ast::CompKind::Gen, _) => {
                self.unsupported(range, "a generator expression", "generators (Phase 8)");
                (CompKind::List, self.list_of(el.ty.clone()))
            }
            _ => (CompKind::List, self.list_of(el.ty.clone())),
        };
        Expr::new(
            ExprKind::Comprehension { kind, clauses: out, elem: Box::new(el), value: val.map(Box::new) },
            ty,
            span,
        )
    }

    // ---------------------------------------------------------------- if and match expressions

    /// A later branch of an `if` or `match`, checked against `ty`: the type
    /// expected of the whole, or with `first` the type of the first
    /// branch, which a mismatch reports as two branches that disagree.
    fn branch_expr(&mut self, e: &ast::Expr, ty: &Ty, first: Option<(&'static str, TextRange)>) -> Expr {
        let Some(first) = first else { return self.check_expr(e, ty) };
        let x = self.infer_expr(e, Some(ty));
        self.branch = Some(first);
        let x = self.coerce(x, ty);
        self.branch = None;
        x
    }

    fn if_expr(
        &mut self,
        branches: &[(ast::Expr, ast::Expr)],
        else_: Option<&ast::Expr>,
        range: TextRange,
        expected: Option<&Ty>,
    ) -> Expr {
        let span = self.span(range);
        let mut target = expected.cloned();
        let mut conds = Vec::new();
        let mut values = Vec::new();
        let all: Vec<&ast::Expr> = branches.iter().map(|(_, v)| v).chain(else_).collect();
        for (c, _) in branches {
            conds.push(self.check_condition_expr(c));
        }
        // The branch whose type the later ones must have, when no type
        // is expected.
        let mut first: Option<TextRange> = None;
        for v in all {
            let x = match &target {
                Some(t) => {
                    let t = t.clone();
                    self.branch_expr(v, &t, first.map(|r| ("if", r)))
                }
                None => {
                    let x = self.infer_expr(v, None);
                    let x = self.decay(x);
                    if self.resolved(&x.ty) != Ty::Never {
                        target = Some(x.ty.clone());
                        first = Some(v.range);
                    }
                    x
                }
            };
            values.push(x);
        }
        let ty = target.unwrap_or(Ty::Never);
        // Build the chain from the end: `if c1: v1 elif c2: v2 else: v3`.
        let else_val = if else_.is_some() { values.pop() } else { None };
        let mut acc = else_val.unwrap_or_else(|| Expr::error(span));
        while let (Some(c), Some(v)) = (conds.pop(), values.pop()) {
            let v = self.coerce(v, &ty);
            acc = Expr::new(
                ExprKind::If { cond: Box::new(c), then: Box::new(v), else_: Box::new(acc) },
                ty.clone(),
                span,
            );
        }
        acc
    }

    fn match_expr(
        &mut self,
        scrutinee: &ast::Expr,
        arms: &[ast::Arm],
        range: TextRange,
        expected: Option<&Ty>,
    ) -> Expr {
        let scrutinee_end = scrutinee.range.end;
        let span = self.span(range);
        let s = self.infer_expr(scrutinee, None);
        let by_ref = self.scrutinee_by_ref(&s);
        let sty = s.ty.clone();
        let mut target = expected.cloned();
        let mut first: Option<TextRange> = None;
        let mut out = Vec::new();
        for a in arms {
            if self.arm_made_at_compile_time(a) {
                continue;
            }
            self.push_scope();
            let p = self.check_pat(&a.pat, &sty, by_ref);
            let guard = a.guard.as_ref().map(|g| self.check_condition_expr(g));
            let body = match &a.body {
                ast::ArmBody::Expr(e) => {
                    let x = match &target {
                        Some(t) => {
                            let t = t.clone();
                            self.branch_expr(e, &t, first.map(|r| ("match", r)))
                        }
                        None => {
                            let x = self.infer_expr(e, None);
                            let x = self.decay(x);
                            if self.resolved(&x.ty) != Ty::Never {
                                target = Some(x.ty.clone());
                                first = Some(e.range);
                            }
                            x
                        }
                    };
                    ArmBody::Expr(x)
                }
                ast::ArmBody::Block(b) => {
                    let d = self.error(msg::TYPE_ARM_NEEDS_EXPRESSION, b.range);
                    self.report(d);
                    ArmBody::Block(self.check_block(b))
                }
            };
            self.pop_scope();
            out.push(Arm { pat: p, guard, body, span: self.span(a.range) });
        }
        let ty = target.unwrap_or(Ty::Never);
        for arm in out.iter_mut() {
            if let ArmBody::Expr(x) = &mut arm.body {
                let taken = std::mem::replace(x, Expr::error(span));
                *x = self.coerce(taken, &ty);
            }
        }
        self.check_exhaustive(&sty, &out, TextRange::new(range.start, scrutinee_end));
        Expr::new(ExprKind::Match { scrutinee: Box::new(s), arms: out }, ty, span)
    }

    /// Arms made by `comptime for` or `comptime if` exist only in a
    /// derive's template and in a template (08); anywhere else they are
    /// reported.
    fn arm_made_at_compile_time(&mut self, a: &ast::Arm) -> bool {
        if a.generated.is_none() {
            return false;
        }
        self.no_unfolding(a.range, "arms made at compile time");
        true
    }

    /// A `match` in statement position: arms may hold blocks.
    pub fn check_match_stmt(&mut self, scrutinee: &ast::Expr, arms: &[ast::Arm], range: TextRange) -> Stmt {
        let span = self.span(range);
        let s = self.infer_expr(scrutinee, None);
        let by_ref = self.scrutinee_by_ref(&s);
        let sty = s.ty.clone();
        let mut out = Vec::new();
        for a in arms {
            if self.arm_made_at_compile_time(a) {
                continue;
            }
            self.push_scope();
            let p = self.check_pat(&a.pat, &sty, by_ref);
            let guard = a.guard.as_ref().map(|g| self.check_condition_expr(g));
            let body = match &a.body {
                ast::ArmBody::Expr(e) => ArmBody::Expr(self.infer_expr(e, None)),
                ast::ArmBody::Block(b) => ArmBody::Block(self.check_block(b)),
            };
            self.pop_scope();
            out.push(Arm { pat: p, guard, body, span: self.span(a.range) });
        }
        self.check_exhaustive(&sty, &out, TextRange::new(range.start, scrutinee.range.end));
        Stmt::Match { scrutinee: s, arms: out, span }
    }

    /// How a scrutinee's bindings take their parts: a place is borrowed
    /// (`Some(false)`), a temporary is moved (`None`).
    pub fn scrutinee_by_ref(&self, s: &Expr) -> Option<bool> {
        if s.is_place() { Some(false) } else { None }
    }

    // ---------------------------------------------------------------- closures

    /// Checks a closure literal or `do` block against an expected function
    /// type, if one is known.
    pub fn check_closure(&mut self, c: &ast::Closure, expected: Option<FnTy>) -> Expr {
        let names: Vec<(String, Option<ast::TypeExpr>, TextRange)> = if c.implicit_param {
            match &expected {
                Some(f) if f.params.len() == 1 => vec![("it".to_string(), None, c.range)],
                _ => Vec::new(),
            }
        } else {
            c.params.iter().map(|p| (p.name.text.clone(), p.ty.clone(), p.name.range)).collect()
        };
        let body = match &c.body {
            ast::ClosureBody::Expr(e) => ClosureSource::Expr(e.clone()),
            ast::ClosureBody::Block(b) => ClosureSource::Block(b.clone()),
        };
        let x = self.closure_from_parts(names, body, expected, c.range);
        if c.own
            && let ExprKind::Closure(id) = x.kind
        {
            self.own_closure(id, c.range);
        }
        x
    }

    /// `comptime e` (08): `e` is checked as the body of a closure that
    /// takes nothing, which must capture nothing (a local or parameter is a
    /// run-time value) and name none of the function's type parameters
    /// (checked once inference is done). The compiler runs it, refusing
    /// anything that would reach the world, and keeps its value.
    pub fn comptime_expr(&mut self, inner: &ast::Expr, expected: Option<&Ty>, range: TextRange) -> Expr {
        let span = self.span(range);
        let ret = expected.cloned().unwrap_or_else(|| self.infer.fresh());
        let fn_ty = FnTy { params: Vec::new(), ret: ret.clone(), pure: true };
        let was = std::mem::replace(&mut self.in_comptime, true);
        let x = self.closure_from_parts(Vec::new(), ClosureSource::Expr(inner.clone()), Some(fn_ty), range);
        self.in_comptime = was;
        let ExprKind::Closure(id) = x.kind else { return Expr::error(span) };
        self.comptime_closure(id, range);
        Expr::new(ExprKind::Comptime(id), ret, span)
    }

    /// `comptime assert c, m` and other statements run at compile time: a
    /// `comptime` closure whose body is the statement.
    pub fn comptime_block(&mut self, b: &ast::Block, range: TextRange) -> Expr {
        let span = self.span(range);
        let fn_ty = FnTy { params: Vec::new(), ret: Ty::Unit, pure: true };
        let was = std::mem::replace(&mut self.in_comptime, true);
        let x = self.closure_from_parts(Vec::new(), ClosureSource::Block(b.clone()), Some(fn_ty), range);
        self.in_comptime = was;
        let ExprKind::Closure(id) = x.kind else { return Expr::error(span) };
        self.comptime_closure(id, range);
        Expr::new(ExprKind::Comptime(id), Ty::Unit, span)
    }

    fn comptime_closure(&mut self, id: ClosureId, range: TextRange) {
        let Some(Some(cl)) = self.closures.get(id.0 as usize) else { return };
        let mut names: Vec<String> = Vec::new();
        for cap in &cl.captures {
            let n = self.locals[cap.local.0 as usize].name.clone();
            if !names.contains(&n) {
                names.push(n);
            }
        }
        for n in names {
            let d = self.error(msg::COMPTIME_NOT_CONSTANT, range).with("name", n);
            self.report(d);
        }
        if let Some(Some(c)) = self.closures.get_mut(id.0 as usize) {
            c.comptime = true;
        }
        self.comptime_sites.push((id, range));
    }

    /// An `own` closure keeps what it captures (04, "Closures"), so it
    /// holds no borrow of it and may be stored and returned. Its captures
    /// are checked once the types are known ([`Self::check_own_closures`]).
    fn own_closure(&mut self, id: ClosureId, range: TextRange) {
        if let Some(Some(c)) = self.closures.get_mut(id.0 as usize) {
            c.own = true;
        }
        self.own_checks.push((id, range));
    }

    /// The captures of `own` closures: a task's are moved in (it runs once
    /// and owns its environment); any other `own` closure copies them, so
    /// each must be `Copy`, and it cannot change one (a call works on its
    /// own copy).
    pub fn check_own_closures(&mut self) {
        for (id, range) in std::mem::take(&mut self.own_checks) {
            let Some(Some(c)) = self.closures.get(id.0 as usize) else { continue };
            if c.role == ClosureRole::Task {
                continue;
            }
            for cap in c.captures.clone() {
                let local = &self.locals[cap.local.0 as usize];
                let (name, ty) = (local.name.clone(), local.ty.clone());
                if cap.mutable {
                    let d = self.error(msg::TYPE_OWN_CLOSURE_CHANGES, range).with("name", name);
                    self.report(d);
                } else if !self.is_copy(&ty) && self.infer.zonk(&ty) != Ty::Error {
                    let d = self
                        .error(msg::TYPE_OWN_CLOSURE_CAPTURE, range)
                        .with("name", name)
                        .with("ty", self.show(&ty));
                    self.report(d);
                }
            }
        }
    }

    pub fn closure_from_parts(
        &mut self,
        names: Vec<(String, Option<ast::TypeExpr>, TextRange)>,
        body: ClosureSource,
        expected: Option<FnTy>,
        range: TextRange,
    ) -> Expr {
        let span = self.span(range);
        if let Some(f) = &expected
            && f.params.len() != names.len()
        {
            let d = self
                .error(msg::TYPE_CLOSURE_PARAMETER_COUNT, range)
                .with("found", names.len().to_string())
                .with("expected", f.params.len().to_string());
            self.report(d);
        }
        let id = ClosureId(self.closures.len() as u32);
        self.closures.push(None);
        let ret = match &expected {
            Some(f) => f.ret.clone(),
            None => self.infer.fresh(),
        };
        self.ret.push(RetCtx {
            ty: ret.clone(),
            closure: Some(id),
            first_local: self.locals.len() as u32,
            loops: 0,
            captures: Vec::new(),
        });
        self.push_scope();
        let mut params = Vec::new();
        let mut ptys = Vec::new();
        for (i, (name, ann, r)) in names.iter().enumerate() {
            let ty = match ann {
                Some(t) => self.resolve_ty(t),
                None => expected
                    .as_ref()
                    .and_then(|f| f.params.get(i).cloned())
                    .unwrap_or_else(|| self.infer.fresh()),
            };
            let l = self.new_local(name, ty.clone(), false, self.span(*r));
            self.declare(name, l, false, *r);
            params.push(l);
            ptys.push(ty);
        }
        let block = match body {
            ClosureSource::Expr(e) => {
                let x = self.check_expr(&e, &ret);
                let x = self.check_movable(x);
                Block {
                    stmts: vec![Stmt::Return { value: Some(x), span: self.span(e.range) }],
                    span: self.span(e.range),
                }
            }
            ClosureSource::Block(b) => {
                let block = self.check_block_in_scope(&b);
                let r = self.resolved(&ret);
                if matches!(r, Ty::Var(_)) {
                    self.infer.unify(&r, &Ty::Unit);
                } else if r != Ty::Unit && r != Ty::Error && !self.block_diverges(&block) {
                    let d = self
                        .error(msg::TYPE_MISSING_RETURN, b.range)
                        .with("name", "this closure")
                        .with("ty", self.show(&r));
                    self.report(d);
                }
                block
            }
        };
        self.pop_scope();
        let ctx = self.ret.pop();
        let captures = ctx.map(|c| c.captures).unwrap_or_default();
        let pure = crate::purity::first_impure_in_block(self.prog, &block, &self.locals).is_none();
        let fn_ty = Ty::Fn(Box::new(FnTy { params: ptys, ret: ret.clone(), pure }));
        self.closures[id.0 as usize] = Some(Closure {
            params,
            captures,
            own: false,
            ret,
            body: block,
            span,
            fn_ty: fn_ty.clone(),
            comptime: false,
            role: ClosureRole::Plain,
        });
        Expr::new(ExprKind::Closure(id), fn_ty, span)
    }

    // ---------------------------------------------------------------- `?`, `?.`

    fn try_expr(&mut self, inner: &ast::Expr, range: TextRange) -> Expr {
        let span = self.span(range);
        let x = self.infer_expr(inner, None);
        if x.ty == Ty::Error {
            return Expr::error(span);
        }
        let ret = self.ret.last().map(|r| r.ty.clone()).unwrap_or(Ty::Unit);
        // A closure whose result is not known yet returns what `?` needs:
        // an option for an option, a result for a result.
        if matches!(self.resolved(&ret), Ty::Var(_)) && !self.infer.is_absorbing(&ret) {
            let shape = if self.as_option(&x.ty).is_some() {
                self.prog.wk.option.map(|o| Ty::Adt(o, vec![self.infer.fresh()]))
            } else if self.as_result(&x.ty).is_some() {
                self.prog.wk.result.map(|r| Ty::Adt(r, vec![self.infer.fresh(), self.infer.fresh()]))
            } else {
                None
            };
            if let Some(shape) = shape {
                self.infer.unify(&ret, &shape);
            }
        }
        if let Some(t) = self.as_option(&x.ty) {
            if self.as_option(&ret).is_none() && self.resolved(&ret) != Ty::Error {
                let d = self
                    .error(msg::TYPE_TRY_CONTEXT, range)
                    .with("ty", self.show(&x.ty))
                    .with("needed", "T?")
                    .with("found", self.show(&ret));
                self.report(d);
            }
            return Expr::new(ExprKind::Try { expr: Box::new(x), kind: TryKind::Option }, t, span);
        }
        if let Some((t, e)) = self.as_result(&x.ty) {
            let Some((_, fe)) = self.as_result(&ret) else {
                if self.resolved(&ret) != Ty::Error {
                    let d = self
                        .error(msg::TYPE_TRY_CONTEXT, range)
                        .with("ty", self.show(&x.ty))
                        .with("needed", "T ! E")
                        .with("found", self.show(&ret));
                    self.report(d);
                }
                return Expr::new(
                    ExprKind::Try {
                        expr: Box::new(x),
                        kind: TryKind::Result { from: Vec::new(), to: Vec::new() },
                    },
                    t,
                    span,
                );
            };
            let from = match self.resolved(&e) {
                Ty::ErrSet(m) => m,
                _ => Vec::new(),
            };
            if self.infer.is_absorbing(&fe) {
                self.infer.unify(&fe, &Ty::ErrSet(from.clone()));
                return Expr::new(
                    ExprKind::Try {
                        expr: Box::new(x),
                        kind: TryKind::Result { from: from.clone(), to: from },
                    },
                    t,
                    span,
                );
            }
            let to = match self.resolved(&fe) {
                Ty::ErrSet(m) => m,
                Ty::Var(_) => {
                    self.infer.unify(&fe, &Ty::ErrSet(from.clone()));
                    from.clone()
                }
                _ => Vec::new(),
            };
            if !is_subset(&from, &to) {
                let missing: Vec<String> =
                    from.iter().filter(|d| !to.contains(d)).map(|d| self.prog.qualified_name(*d)).collect();
                let d = self
                    .error(msg::TYPE_ERROR_NOT_IN_SET, range)
                    .with("from", self.prog.display(&Ty::ErrSet(from.clone())))
                    .with("to", self.prog.display(&Ty::ErrSet(to.clone())))
                    .fix(msg::TYPE_ADD_ERROR_SET, vec![("set", missing.join(", "))], Vec::new());
                self.report(d);
            }
            return Expr::new(
                ExprKind::Try { expr: Box::new(x), kind: TryKind::Result { from, to } },
                t,
                span,
            );
        }
        let d = self.error(msg::TYPE_TRY_NEEDS_FALLIBLE, range).with("ty", self.show(&x.ty));
        self.report(d);
        Expr::error(span)
    }

    /// `base?.name` and `base?.name(args)`.
    pub fn opt_chain(
        &mut self,
        base: &ast::Expr,
        name: &ast::Name,
        call: Option<(&[ast::Arg], Option<&ast::Closure>)>,
        range: TextRange,
    ) -> Expr {
        let span = self.span(range);
        let b = self.infer_expr(base, None);
        let Some(inner) = self.as_option(&b.ty) else {
            if b.ty != Ty::Error {
                let d = self.error(msg::TYPE_OPTIONAL_CHAIN, base.range).with("ty", self.show(&b.ty));
                self.report(d);
            }
            return Expr::error(span);
        };
        self.push_scope();
        let binder = self.new_local("?", inner.clone(), false, b.span);
        let recv = Expr::new(ExprKind::Local(binder), inner, b.span);
        let then = match call {
            None => self.field_of(recv, name, range),
            Some((args, trailing)) => self.method_call_on(recv, name, args, trailing, range, None),
        };
        self.pop_scope();
        let then = self.decay(then);
        // Through a reference, a field that is not `Copy` cannot be moved
        // out: the chain borrows it, a `String` as a `str` view
        // (`eldest?.name ?? "nobody"`).
        let then = if then.is_place() && then.is_behind_ref() && !self.is_copy(&then.ty) {
            let ty = self.resolved(&then.ty);
            if self.is_string(&ty) {
                self.wrap(then, Coercion::StrView, Ty::Str)
            } else if ty == Ty::Error {
                then
            } else {
                let span = then.span;
                Expr::new(
                    ExprKind::Borrow { place: Box::new(then), mutable: false },
                    Ty::Ref(Box::new(ty), false),
                    span,
                )
            }
        } else {
            then
        };
        let flatten = self.as_option(&then.ty).is_some();
        let ty = if flatten { then.ty.clone() } else { self.option_of(then.ty.clone()) };
        Expr::new(ExprKind::OptChain { base: Box::new(b), binder, then: Box::new(then), flatten }, ty, span)
    }

    // ---------------------------------------------------------------- fields and indexing

    fn field_expr(&mut self, base: &ast::Expr, name: &ast::Name, range: TextRange) -> Expr {
        // `module.item`, `Type.Variant`, `Type.function`.
        if let Some(path) = self.static_path(base) {
            return self.static_member(path, name, range, None);
        }
        let b = self.infer_expr(base, None);
        self.field_of(b, name, range)
    }

    /// Field `name` of a value (through references).
    pub fn field_of(&mut self, b: Expr, name: &ast::Name, range: TextRange) -> Expr {
        let span = self.span(range);
        let prog = self.prog;
        let b = self.auto_deref(b);
        let mut b = self.unrefined(b);
        // A `Box` lends its fields to its value: `boxed` is its own. So
        // does a `Shared`, and a mutex guard's `value` is the value.
        loop {
            if self.is_box(&b.ty) && name.text != "boxed" {
                b = self.unbox(b);
                b = self.auto_deref(b);
                continue;
            }
            match self.smart_pointer(&b.ty) {
                Some("Shared") => {
                    b = self.deref_smart(b);
                    b = self.auto_deref(b);
                }
                Some("MutexGuard" | "ReadGuard") if name.text == "value" => return self.deref_smart(b),
                // Any other field is the value's.
                Some("MutexGuard" | "ReadGuard") => {
                    b = self.deref_smart(b);
                    b = self.auto_deref(b);
                }
                _ => break,
            }
        }
        let t = self.resolved(&b.ty);
        match &t {
            Ty::Error => return Expr::error(span),
            Ty::Tuple(elems, labels) => {
                let idx =
                    name.text.parse::<usize>().ok().or_else(|| labels.iter().position(|l| *l == name.text));
                return match idx {
                    Some(i) if i < elems.len() => {
                        let ty = elems[i].clone();
                        Expr::new(ExprKind::Field { base: Box::new(b), index: i as u32 }, ty, span)
                    }
                    _ => {
                        let d = self
                            .error(msg::TYPE_TUPLE_INDEX, name.range)
                            .with("ty", self.show(&t))
                            .with("index", name.text.clone());
                        self.report(d);
                        Expr::error(span)
                    }
                };
            }
            Ty::Adt(d, args) => {
                if let Some(s) = prog.struct_info(*d)
                    && let Some(i) = s.fields.iter().position(|f| f.name == name.text)
                {
                    self.check_field_vis(*d, i, name.range);
                    let ty = s.fields[i].ty.subst(args);
                    return Expr::new(ExprKind::Field { base: Box::new(b), index: i as u32 }, ty, span);
                }
            }
            Ty::Var(_) => {
                let d = self
                    .error(msg::TYPE_CANNOT_INFER, b.span.range)
                    .with("name", self.source_text(b.span.range));
                self.report(d);
                return Expr::error(span);
            }
            _ => {}
        }
        if let Some(key) = t.key()
            && (prog.method(key, &name.text).is_some()
                || (self.is_string(&t) && prog.method(sisu_resolve::TyKey::Str, &name.text).is_some()))
        {
            let d = self.error(msg::TYPE_METHOD_NEEDS_RECEIVER, name.range).with("name", name.text.clone());
            self.report(d);
            return Expr::error(span);
        }
        let mut d = self
            .error(msg::TYPE_UNKNOWN_FIELD, name.range)
            .with("ty", self.show(&t))
            .with("name", name.text.clone());
        if let Some(similar) = self.similar_member(&t, &name.text) {
            let edit = sisu_diag::Edit { span: self.span(name.range), text: similar };
            d = d.fix(msg::TYPE_SIMILAR_MEMBER, Vec::new(), vec![edit]);
        }
        self.report(d);
        Expr::error(span)
    }

    /// The edit that bounds the function's own type parameter `i` by
    /// `bound` in its header (`[T]` becomes `[T: Ord]`, `[T: Hash]`
    /// becomes `[T: Hash + Ord]`), with the parameter's name; `None` for an
    /// impl's parameter or a header that does not read as expected.
    pub fn bound_edit(&self, i: u32, bound: &str) -> Option<(String, sisu_diag::Edit)> {
        let f = self.prog.fn_info(self.def)?;
        if (i as usize) < f.impl_generics {
            return None;
        }
        let name = f.generics.get(i as usize)?.name.clone();
        let after = self.prog.def(self.def).span.range.end as usize;
        let rest = self.source.get(after..)?;
        if !rest.starts_with('[') {
            return None;
        }
        // The parameter list, split at its top-level commas.
        let mut depth = 0i32;
        let mut seg_start = after + 1;
        let mut segments: Vec<(usize, usize)> = Vec::new();
        for (k, c) in rest.char_indices() {
            match c {
                '[' | '(' => depth += 1,
                ']' | ')' => {
                    depth -= 1;
                    if depth == 0 {
                        segments.push((seg_start, after + k));
                        break;
                    }
                }
                ',' if depth == 1 => {
                    segments.push((seg_start, after + k));
                    seg_start = after + k + 1;
                }
                _ => {}
            }
        }
        for (a, b) in segments {
            let seg = self.source.get(a..b)?;
            let lead = seg.len() - seg.trim_start().len();
            let word: String =
                seg.trim_start().chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            if word != name {
                continue;
            }
            let name_end = (a + lead + word.len()) as u32;
            let edit = match seg.find(':') {
                None => sisu_diag::Edit {
                    span: Span::new(self.file, TextRange::empty(name_end)),
                    text: format!(": {bound}"),
                },
                Some(_) => {
                    // After the last bound, before a default.
                    let end = seg.find('=').map(|e| a + e).unwrap_or(b);
                    let text = self.source.get(a..end)?;
                    let at = (a + text.trim_end().len()) as u32;
                    sisu_diag::Edit {
                        span: Span::new(self.file, TextRange::empty(at)),
                        text: format!(" + {bound}"),
                    }
                }
            };
            return Some((name, edit));
        }
        None
    }

    /// The field or method of a type closest to a misspelt one, by the
    /// rule of [`Checker::similar_name`].
    pub fn similar_member(&self, t: &Ty, n: &str) -> Option<String> {
        let prog = self.prog;
        let mut names: Vec<String> = Vec::new();
        if let Ty::Adt(d, _) = t
            && let Some(s) = prog.struct_info(*d)
        {
            names.extend(s.fields.iter().map(|f| f.name.clone()));
        }
        let mut keys: Vec<sisu_resolve::TyKey> = t.key().into_iter().collect();
        if self.is_string(t) {
            keys.push(sisu_resolve::TyKey::Str);
        }
        names.extend(
            prog.methods
                .keys()
                .filter(|(k, m)| keys.contains(k) && !m.starts_with('_'))
                .map(|(_, m)| m.clone()),
        );
        closest(n, names)
    }

    /// A field of a struct of another module is reached only when it is
    /// `pub` (09).
    pub fn check_field_vis(&mut self, d: DefId, i: usize, range: TextRange) {
        let prog = self.prog;
        let Some(f) = prog.struct_info(d).and_then(|s| s.fields.get(i)) else { return };
        if !prog.vis_allows(f.vis, prog.def(d).module, self.module) {
            let diag = self
                .error(msg::RESOLVE_PRIVATE_FIELD, range)
                .with("field", f.name.clone())
                .with("ty", prog.def(d).name.clone())
                .with("module", prog.module_path(prog.def(d).module));
            self.report(diag);
        }
    }

    /// A method or associated function of a type of another module is
    /// called only when it is `pub` (09); a trait's methods are as visible
    /// as the trait.
    pub fn check_method_vis(&mut self, m: DefId, range: TextRange) {
        let prog = self.prog;
        let Some(f) = prog.fn_info(m) else { return };
        if f.trait_.is_some() || f.impl_.is_some() || prog.visible_from(m, self.module) {
            return;
        }
        let owner = f.owner.as_ref().map(|o| prog.display(o)).unwrap_or_default();
        let diag = self
            .error(msg::RESOLVE_PRIVATE_METHOD, range)
            .with("name", prog.def(m).name.clone())
            .with("ty", owner)
            .with("module", prog.module_path(prog.def(m).module));
        self.report(diag);
    }

    fn index_expr(&mut self, base: &ast::Expr, args: &[ast::IndexArg], range: TextRange) -> Expr {
        let span = self.span(range);
        let b = self.infer_expr(base, None);
        let b = self.auto_deref(b);
        let b = self.unrefined(b);
        let t = self.resolved(&b.ty);
        if t == Ty::Error {
            for a in args {
                if let ast::IndexArg::Expr(e) = a {
                    let _ = self.infer_expr(e, None);
                }
            }
            return Expr::error(span);
        }
        let prog = self.prog;
        let Some(arg) = args.first() else { return Expr::error(span) };
        match (&t, arg) {
            (Ty::Adt(d, a), ast::IndexArg::Expr(e)) if matches!(prog.builtin(*d), Some("list" | "Array")) => {
                let elem = a.first().cloned().unwrap_or(Ty::Error);
                let (idx, kind) = match &e.kind {
                    ast::ExprKind::FromEnd(inner) => {
                        (self.check_expr(inner, &Ty::Int), IndexKind::ListFromEnd)
                    }
                    ast::ExprKind::Unary { op: UnOp::Neg, expr }
                        if matches!(expr.kind, ast::ExprKind::Int { .. }) =>
                    {
                        // `xs[-1]` does not compile: Python's meaning would be silent (D-062).
                        let digits = self.source_text(expr.range);
                        let fix = format!("{}[^{digits}]", self.source_text(base.range));
                        let edit = Edit { span: self.span(e.range), text: format!("^{digits}") };
                        let d = self.error(msg::TYPE_CONSTANT_NEGATIVE_INDEX, e.range).with("fix", fix).fix(
                            msg::TYPE_FROM_END_INDEX,
                            Vec::new(),
                            vec![edit],
                        );
                        self.report(d);
                        (self.check_expr(expr, &Ty::Int), IndexKind::ListFromEnd)
                    }
                    _ => (self.check_expr(e, &Ty::Int), IndexKind::List),
                };
                Expr::new(ExprKind::Index { base: Box::new(b), index: Box::new(idx), kind }, elem, span)
            }
            (Ty::Adt(d, a), ast::IndexArg::Slice { lo, hi }) if prog.builtin(*d) == Some("list") => {
                let lo = lo.as_ref().map(|x| Box::new(self.check_expr(x, &Ty::Int)));
                let hi = hi.as_ref().map(|x| Box::new(self.check_expr(x, &Ty::Int)));
                let ty = Ty::Adt(*d, a.clone());
                Expr::new(ExprKind::Slice { base: Box::new(b), lo, hi }, ty, span)
            }
            // `arena[key]`: the value of a live key; a stale one panics.
            (Ty::Adt(d, a), ast::IndexArg::Expr(e)) if prog.builtin(*d) == Some("Arena") => {
                let v = a.first().cloned().unwrap_or(Ty::Error);
                let key = prog
                    .lookup_in(prog.prelude, "Key")
                    .map(|k| Ty::Adt(k, vec![v.clone()]))
                    .unwrap_or(Ty::Error);
                let idx = self.check_expr(e, &key);
                Expr::new(
                    ExprKind::Index { base: Box::new(b), index: Box::new(idx), kind: IndexKind::Dict },
                    v,
                    span,
                )
            }
            (Ty::Adt(d, a), ast::IndexArg::Expr(e)) if prog.builtin(*d) == Some("dict") => {
                let k = a.first().cloned().unwrap_or(Ty::Error);
                let v = a.get(1).cloned().unwrap_or(Ty::Error);
                self.require(k.clone(), prog.wk.hash, span);
                let idx = self.check_expr(e, &k);
                Expr::new(
                    ExprKind::Index { base: Box::new(b), index: Box::new(idx), kind: IndexKind::Dict },
                    v,
                    span,
                )
            }
            // `b[i]` and `s[i]`: a byte, a reference to a value; `b[i:j]`
            // and `s[i:j]`: a copy of the bytes, a view of the values.
            (Ty::Adt(d, a), _) if matches!(prog.builtin(*d), Some("bytes" | "Slice")) => {
                let (name, args) = match arg {
                    ast::IndexArg::Expr(e) => ("at", vec![self.check_expr(e, &Ty::Int)]),
                    ast::IndexArg::Slice { lo, hi } => {
                        let bound = |c: &mut Self, x: &Option<ast::Expr>| match x {
                            Some(x) => c.check_expr(x, &Ty::Int),
                            None => Expr::new(ExprKind::Int(i64::MIN), Ty::Int, span),
                        };
                        let lo = bound(self, lo);
                        let hi = bound(self, hi);
                        ("cut", vec![lo, hi])
                    }
                };
                let Some(m) = prog.method(sisu_resolve::TyKey::Adt(*d), name) else {
                    return Expr::error(span);
                };
                let elem = a.first().cloned().unwrap_or(Ty::Error);
                let ret = match (prog.builtin(*d), name) {
                    (Some("bytes"), "at") => self.sized_ty("u8"),
                    (Some("Slice"), "at") => Ty::Ref(Box::new(elem), false),
                    _ => t.clone(),
                };
                let recv = self.receiver_arg(b);
                let mut call_args = vec![recv];
                call_args.extend(args.into_iter().map(|x| Arg { expr: x, mode: ArgMode::Value }));
                Expr::new(
                    ExprKind::Call { callee: Callee::Fn { def: m, type_args: a.clone() }, args: call_args },
                    ret,
                    span,
                )
            }
            (t, _) if *t == Ty::Str || self.is_string(t) => {
                let d = self.error(msg::TYPE_STR_NOT_INDEXABLE, range).with("ty", self.show(t));
                self.report(d);
                Expr::error(span)
            }
            _ => {
                let d = self.error(msg::TYPE_NOT_INDEXABLE, range).with("ty", self.show(&t));
                self.report(d);
                Expr::error(span)
            }
        }
    }
}

/// The body of a closure being checked.
pub enum ClosureSource {
    Expr(ast::Expr),
    Block(ast::Block),
}

/// Whether an expression mentions `it` outside any nested closure: such an
/// argument, where a one-parameter function is expected, is a closure over
/// `it` (02, "Closures").
pub fn mentions_it(e: &ast::Expr) -> bool {
    use ast::ExprKind as K;
    match &e.kind {
        K::Name(n) => n == "it",
        K::Closure(_) => false,
        K::Paren(x) | K::Try(x) | K::FromEnd(x) => mentions_it(x),
        K::Unary { expr, .. } | K::Ref { expr, .. } => mentions_it(expr),
        K::Binary { lhs, rhs, .. } => mentions_it(lhs) || mentions_it(rhs),
        K::Compare { first, rest } => mentions_it(first) || rest.iter().any(|(_, x)| mentions_it(x)),
        K::Tuple(xs) | K::List(xs) | K::Set(xs) => xs.iter().any(mentions_it),
        K::Dict(kv) => kv.iter().any(|(k, v)| mentions_it(k) || mentions_it(v)),
        K::Call { callee, args, .. } => mentions_it(callee) || args.iter().any(|a| mentions_it(&a.value)),
        K::Field { base, .. } | K::OptField { base, .. } => mentions_it(base),
        K::Index { base, args } => {
            mentions_it(base)
                || args.iter().any(|a| match a {
                    ast::IndexArg::Expr(x) => mentions_it(x),
                    ast::IndexArg::Slice { lo, hi } => {
                        lo.as_ref().is_some_and(mentions_it) || hi.as_ref().is_some_and(mentions_it)
                    }
                })
        }
        K::If { branches, else_ } => {
            branches.iter().any(|(c, v)| mentions_it(c) || mentions_it(v))
                || else_.as_ref().is_some_and(|x| mentions_it(x))
        }
        K::Str(parts) => {
            parts.iter().any(|p| matches!(p, ast::StrPart::Interp { expr, .. } if mentions_it(expr)))
        }
        _ => false,
    }
}

use sisu_resolve::closest;

/// Whether `spec` is a format spec (03, "Strings"):
/// `[[fill]align][sign][#][0][width][,|_][.precision][type]`, the grammar
/// `Spec.parse` in std/prelude.sisu reads.
fn is_format_spec(spec: &str) -> bool {
    let cs: Vec<char> = spec.chars().collect();
    let align = |c: char| matches!(c, '<' | '>' | '^' | '=');
    let mut i =
        if cs.len() >= 2 && align(cs[1]) { 2 } else { usize::from(cs.first().is_some_and(|c| align(*c))) };
    let eat = |ok: &dyn Fn(char) -> bool, i: &mut usize| {
        if cs.get(*i).is_some_and(|c| ok(*c)) {
            *i += 1;
        }
    };
    eat(&|c| matches!(c, '+' | '-' | ' '), &mut i);
    eat(&|c| c == '#', &mut i);
    eat(&|c| c == '0', &mut i);
    while cs.get(i).is_some_and(|c| c.is_ascii_digit()) {
        i += 1;
    }
    eat(&|c| c == ',' || c == '_', &mut i);
    if cs.get(i) == Some(&'.') {
        i += 1;
        while cs.get(i).is_some_and(|c| c.is_ascii_digit()) {
            i += 1;
        }
    }
    eat(&|c| "boxXdeEfF%s?".contains(c), &mut i);
    i == cs.len()
}
