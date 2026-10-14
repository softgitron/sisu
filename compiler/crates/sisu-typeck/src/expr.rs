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
        let x = self.infer_expr(e, Some(ty));
        self.coerce(x, ty)
    }

    /// Wraps an expression in a coercion node.
    fn wrap(&self, x: Expr, kind: Coercion, ty: Ty) -> Expr {
        let span = x.span;
        Expr::new(ExprKind::Coerce { expr: Box::new(x), kind }, ty, span)
    }

    /// Converts `x` to `expected`, or reports a mismatch.
    pub fn coerce(&mut self, x: Expr, expected: &Ty) -> Expr {
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
        let mut d = self
            .error(msg::TYPE_MISMATCH, self.range_of(&x))
            .with("expected", expected_s)
            .with("found", found_s.clone());
        if matches!(actual, Ty::Int) && matches!(exp, Ty::Float) {
            d = d.note(
                msg::TYPE_MIXED_NUMBERS,
                vec![("op", "=".into()), ("lhs", "f64".into()), ("rhs", "int".into())],
            );
        }
        self.report(d);
        self.wrap(x, Coercion::Mismatch, exp)
    }

    pub fn range_of(&self, x: &Expr) -> TextRange {
        x.span.range
    }

    /// A function passed where a `pure` one is required must be pure.
    fn check_purity_coercion(&mut self, x: &Expr, expected: &Ty) {
        let Ty::Fn(f) = self.infer.zonk(expected) else { return };
        if !f.pure {
            return;
        }
        let callee = match &x.kind {
            ExprKind::FnRef(d) if !self.prog.fn_info(*d).is_some_and(|i| i.pure) => {
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

    // ---------------------------------------------------------------- inference

    pub fn infer_expr(&mut self, e: &ast::Expr, expected: Option<&Ty>) -> Expr {
        let span = self.span(e.range);
        match &e.kind {
            ast::ExprKind::Int { value, suffix, text } => {
                self.int_literal(*value, suffix.as_deref(), text, false, e.range, expected)
            }
            ast::ExprKind::Float { value, suffix, .. } => {
                if let Some(s) = suffix
                    && s != "f64"
                {
                    self.unsupported(e.range, &format!("`{s}`"), "the sized numeric types (Phase 4)");
                }
                Expr::new(ExprKind::Float(*value), Ty::Float, span)
            }
            ast::ExprKind::Char(c) => Expr::new(ExprKind::Char(*c), Ty::Char, span),
            ast::ExprKind::Str(parts) => self.string_literal(parts, e.range, expected),
            ast::ExprKind::RawStr(s) => self.plain_string(s.clone(), span, expected),
            ast::ExprKind::ByteStr(_) => {
                self.unsupported(e.range, "a byte string", "byte strings (Phase 4)");
                Expr::error(span)
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
                self.bounds.push((ty.clone(), "Hash".into(), span));
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
                self.bounds.push((kt.clone(), "Hash".into(), span));
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
            ast::ExprKind::Index { base, args } => self.index_expr(base, args, e.range),
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
            ast::ExprKind::Unsupported(what) => {
                let phase = if what.contains("comptime") || what.contains("compile-time") {
                    "compile-time evaluation (Phase 4)"
                } else {
                    "foreign functions (Phase 8)"
                };
                self.unsupported(e.range, what, phase);
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
        match suffix {
            None | Some("int") => {}
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
        let to_float = suffix.is_none() && expected.map(|t| self.resolved(t)) == Some(Ty::Float);
        let limit: u128 = if negative { 1u128 << 63 } else { (1u128 << 63) - 1 };
        match value {
            Some(v) if v <= limit => {
                if to_float {
                    let f = v as f64;
                    return Expr::new(ExprKind::Float(if negative { -f } else { f }), Ty::Float, span);
                }
                let n = if negative { (v as i128).wrapping_neg() as i64 } else { v as i64 };
                Expr::new(ExprKind::Int(n), Ty::Int, span)
            }
            _ => {
                let t = if negative { format!("-{text}") } else { text.to_string() };
                let d = self.error(msg::TYPE_LITERAL_OUT_OF_RANGE, range).with("text", t).with("ty", "int");
                self.report(d);
                Expr::error(span)
            }
        }
    }

    fn plain_string(&mut self, s: String, span: Span, expected: Option<&Ty>) -> Expr {
        // A literal takes the expected type when it is `String` or `Path`
        // (FromLiteral, 02 "Literals").
        let ty = match expected.map(|t| self.resolved(t)) {
            Some(t @ Ty::Adt(d, _)) if matches!(self.prog.builtin(d), Some("String" | "Path")) => t,
            _ => Ty::Str,
        };
        Expr::new(ExprKind::Str(s), ty, span)
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
                    if debug.is_none() && spec != "?" && matches!(self.resolved(&x.ty), Ty::Fn(_)) {
                        let shown = self.show(&x.ty);
                        let d = self
                            .error(msg::TYPE_NO_DISPLAY, expr.range)
                            .with("ty", shown)
                            .with("spec", format!("{{{}}}", self.source_text(expr.range)));
                        self.report(d);
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
        let limit = (n.chars().count() / 3).max(1);
        let mut names: Vec<String> = self.scopes.iter().flatten().map(|(name, _)| name.clone()).collect();
        if let Some(m) = prog.modules.get(self.module.0 as usize) {
            names.extend(m.items.keys().cloned());
        }
        if let Some(m) = prog.modules.get(prog.prelude.0 as usize) {
            names.extend(m.items.keys().filter(|k| !k.starts_with('_')).cloned());
        }
        names.sort();
        names.dedup();
        names
            .into_iter()
            .filter(|c| c != n)
            .map(|c| (edit_distance(n, &c), c))
            .filter(|(d, _)| *d <= limit)
            .min_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)))
            .map(|(_, c)| c)
    }

    /// A declaration used as a value.
    pub fn def_value(&mut self, d: DefId, n: &str, range: TextRange) -> Expr {
        let span = self.span(range);
        let prog = self.prog;
        match &prog.def(d).kind {
            DefKind::Fn(f) => {
                if !f.generics.is_empty() || f.self_mode.is_some() {
                    let kind = if f.self_mode.is_some() { "method" } else { "generic function" };
                    let diag = self.error(msg::TYPE_NOT_A_VALUE, range).with("name", n).with("kind", kind);
                    self.report(diag);
                    return Expr::error(span);
                }
                let ret = self.fn_ret(d);
                let ty = Ty::Fn(Box::new(FnTy {
                    params: f.params.iter().map(|p| p.ty.clone()).collect(),
                    ret,
                    pure: f.pure,
                }));
                Expr::new(ExprKind::FnRef(d), ty, span)
            }
            DefKind::Const(c) => Expr::new(ExprKind::Const(d), c.ty.clone(), span),
            other => {
                let kind = match other {
                    DefKind::Struct(_) | DefKind::Enum(_) | DefKind::ErrorSet(_) | DefKind::TypeAlias(_) => {
                        "type"
                    }
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
        let hint = expected.map(|t| self.resolved(t)).filter(|t| matches!(t, Ty::Int | Ty::Float));
        let l = self.infer_expr(lhs, hint.as_ref());
        let l = self.decay(l);
        let lt = self.resolved(&l.ty);
        let rhint = if matches!(lt, Ty::Int | Ty::Float) { Some(lt.clone()) } else { None };
        let r = self.infer_expr(rhs, rhint.as_ref());
        let r = self.decay(r);
        // A literal on the left takes the type of a float on the right.
        let l = match (&l.kind, self.resolved(&r.ty)) {
            (ExprKind::Int(n), Ty::Float) => Expr::new(ExprKind::Float(*n as f64), Ty::Float, l.span),
            _ => l,
        };
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
            (_, Ty::Int, Ty::Float) | (_, Ty::Float, Ty::Int) => {
                let d = self
                    .error(msg::TYPE_MIXED_NUMBERS, range)
                    .with("op", op.text())
                    .with("lhs", self.show(&lt))
                    .with("rhs", self.show(&rt));
                self.report(d);
                return (Ty::Error, false);
            }
            _ => None,
        };
        match result {
            Some(t) => (t, true),
            None => {
                let d = self
                    .error(msg::TYPE_BINARY_OPERATOR, range)
                    .with("op", op.text())
                    .with("lhs", self.show(&lt))
                    .with("rhs", self.show(&rt));
                self.report(d);
                (Ty::Error, false)
            }
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
                return self.int_literal(*value, suffix.as_deref(), text, true, range, expected);
            }
            if let ast::ExprKind::Float { value, .. } = &inner.kind {
                return Expr::new(ExprKind::Float(-*value), Ty::Float, span);
            }
        }
        let x = match op {
            UnOp::Not => self.check_condition_expr(inner),
            _ => {
                let hint = expected.map(|t| self.resolved(t)).filter(|t| matches!(t, Ty::Int | Ty::Float));
                let x = self.infer_expr(inner, hint.as_ref());
                self.decay(x)
            }
        };
        let t = self.resolved(&x.ty);
        let ty = match (op, &t) {
            (_, Ty::Error) => Ty::Error,
            (UnOp::Not, _) => Ty::Bool,
            (UnOp::Neg, Ty::Int | Ty::Float) => t.clone(),
            (UnOp::BitNot, Ty::Int) => Ty::Int,
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
        let mut prev = f.clone();
        let mut out = Vec::new();
        for (op, e) in rest {
            let pt = self.decay_ty(&prev.ty);
            let hint = if matches!(op, CmpOp::In | CmpOp::NotIn) { None } else { Some(pt.clone()) };
            let x = self.infer_expr(e, hint.as_ref());
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
                    Ty::Adt(d, a) if Some(*d) == self.prog.wk.list || Some(*d) == self.prog.wk.set => {
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
                    let t = self.infer.zonk(&a);
                    if t != Ty::Float && !matches!(t, Ty::Var(_)) && !self.prog.satisfies(&t, "Ord") {
                        let d = self
                            .error(msg::TYPE_NOT_ORDERED, range)
                            .with("ty", self.show(&t))
                            .with("op", op.text());
                        self.report(d);
                    }
                } else if matches!(self.resolved(&a), Ty::Fn(_)) {
                    let d = self
                        .error(msg::TYPE_COMPARISON_TYPES, range)
                        .with("lhs", self.show(&lt))
                        .with("rhs", self.show(&rt));
                    self.report(d);
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
        let opt_hint = expected.map(|t| self.option_of(t.clone()));
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
        let behind_ref = matches!(self.resolved(ty), Ty::Ref(..));
        let Ty::Adt(d, args) = &t else {
            return if t == Ty::Error { Some((Ty::Error, ForMode::Borrow)) } else { None };
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
            Some("list") | Some("set") => {
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
            _ => None,
        }
    }

    /// The iterable of a `for` loop or comprehension clause.
    pub fn check_iterable(&mut self, iter: &ast::Expr, mode: ast::Mode) -> (Expr, ForMode, Ty) {
        let x = self.infer_expr(iter, None);
        match self.iteration_item(&x.ty, mode) {
            Some((item, fmode)) => {
                let x = match fmode {
                    ForMode::Own | ForMode::Iter => self.check_movable(x),
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
        for v in all {
            let x = match &target {
                Some(t) => {
                    let t = t.clone();
                    self.check_expr(v, &t)
                }
                None => {
                    let x = self.infer_expr(v, None);
                    let x = self.decay(x);
                    if self.resolved(&x.ty) != Ty::Never {
                        target = Some(x.ty.clone());
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
        let mut out = Vec::new();
        for a in arms {
            self.push_scope();
            let p = self.check_pat(&a.pat, &sty, by_ref);
            let guard = a.guard.as_ref().map(|g| self.check_condition_expr(g));
            let body = match &a.body {
                ast::ArmBody::Expr(e) => {
                    let x = match &target {
                        Some(t) => {
                            let t = t.clone();
                            self.check_expr(e, &t)
                        }
                        None => {
                            let x = self.infer_expr(e, None);
                            let x = self.decay(x);
                            if self.resolved(&x.ty) != Ty::Never {
                                target = Some(x.ty.clone());
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

    /// A `match` in statement position: arms may hold blocks.
    pub fn check_match_stmt(&mut self, scrutinee: &ast::Expr, arms: &[ast::Arm], range: TextRange) -> Stmt {
        let span = self.span(range);
        let s = self.infer_expr(scrutinee, None);
        let by_ref = self.scrutinee_by_ref(&s);
        let sty = s.ty.clone();
        let mut out = Vec::new();
        for a in arms {
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
        if c.own {
            self.unsupported(c.range, "an `own` closure", "closures with captures by move (Phase 3)");
        }
        self.closure_from_parts(names, body, expected, c.range)
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
        self.closures[id.0 as usize] =
            Some(Closure { params, captures, ret, body: block, span, fn_ty: fn_ty.clone() });
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
        let d = self
            .error(msg::TYPE_UNKNOWN_FIELD, name.range)
            .with("ty", self.show(&t))
            .with("name", name.text.clone());
        self.report(d);
        Expr::error(span)
    }

    fn index_expr(&mut self, base: &ast::Expr, args: &[ast::IndexArg], range: TextRange) -> Expr {
        let span = self.span(range);
        let b = self.infer_expr(base, None);
        let b = self.auto_deref(b);
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
            (Ty::Adt(d, a), ast::IndexArg::Expr(e)) if prog.builtin(*d) == Some("list") => {
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
            (Ty::Adt(d, a), ast::IndexArg::Expr(e)) if prog.builtin(*d) == Some("dict") => {
                let k = a.first().cloned().unwrap_or(Ty::Error);
                let v = a.get(1).cloned().unwrap_or(Ty::Error);
                let idx = self.check_expr(e, &k);
                Expr::new(
                    ExprKind::Index { base: Box::new(b), index: Box::new(idx), kind: IndexKind::Dict },
                    v,
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

/// The edit distance between two names, by characters, where swapping
/// two adjacent characters counts as one edit (optimal string alignment).
pub fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut d = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in d[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut v = (d[i - 1][j] + 1).min(d[i][j - 1] + 1).min(d[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                v = v.min(d[i - 2][j - 2] + 1);
            }
            d[i][j] = v;
        }
    }
    d[a.len()][b.len()]
}
