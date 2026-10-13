//! Patterns. A pattern matched through a reference, or against a place,
//! binds references to the matched parts (copies for `Copy` parts); one
//! matched against a temporary takes the parts by value.

use crate::call::StaticPath;
use crate::check::Checker;
use crate::thir::*;
use sisu_diag::{TextRange, msg};
use sisu_resolve::{DefId, DefKind, Ty};
use sisu_syntax::ast;

impl Checker<'_> {
    /// A pattern of `match`, `if let`, `for`: bindings are immutable.
    pub fn check_pat(&mut self, p: &ast::Pattern, ty: &Ty, by_ref: Option<bool>) -> Pat {
        self.check_pat_with(p, ty, by_ref, false, false)
    }

    pub fn check_pat_with(
        &mut self,
        p: &ast::Pattern,
        ty: &Ty,
        by_ref: Option<bool>,
        mutable: bool,
        warn: bool,
    ) -> Pat {
        let span = self.span(p.range);
        let t = self.resolved(ty);
        if let Ty::Ref(inner, m) = &t
            && !matches!(p.kind, ast::PatKind::Bind(_) | ast::PatKind::Wildcard | ast::PatKind::Error)
        {
            let inner_p = self.check_pat_with(p, inner, Some(*m), mutable, warn);
            return Pat { kind: PatKind::Deref(Box::new(inner_p)), ty: t.clone(), span };
        }
        let kind = match &p.kind {
            ast::PatKind::Wildcard => PatKind::Wild,
            ast::PatKind::Error => PatKind::Error,
            ast::PatKind::Bind(n) => {
                let (lty, mode) = match by_ref {
                    Some(m) if !self.is_copy(&t) => {
                        (Ty::Ref(Box::new(t.clone()), m), if m { BindMode::RefMut } else { BindMode::Ref })
                    }
                    _ => (t.clone(), BindMode::Value),
                };
                let l = self.new_local(&n.text, lty.clone(), mutable, self.span(n.range));
                self.declare(&n.text, l, warn, n.range);
                return Pat { kind: PatKind::Bind { local: l, mode }, ty: t, span };
            }
            ast::PatKind::Literal(e) => self.literal_pat(e, &t),
            ast::PatKind::Ctor { path, args } => {
                self.ctor_pat(path, args.as_deref(), &t, by_ref, mutable, warn, p.range)
            }
            // `()` matches the one value of the unit type.
            ast::PatKind::Tuple(ps)
                if ps.is_empty() && (t == Ty::Unit || self.infer.try_unify(&t, &Ty::Unit)) =>
            {
                PatKind::Wild
            }
            ast::PatKind::Tuple(ps) => {
                let elems: Vec<Ty> = match &t {
                    Ty::Tuple(es, _) if es.len() == ps.len() => es.clone(),
                    Ty::Var(_) => {
                        let es: Vec<Ty> = ps.iter().map(|_| self.infer.fresh()).collect();
                        self.infer.unify(&t, &Ty::tuple(es.clone()));
                        es
                    }
                    Ty::Error => ps.iter().map(|_| Ty::Error).collect(),
                    _ => {
                        let what = if let Ty::Tuple(es, _) = &t {
                            let d = self
                                .error(msg::TYPE_UNPACK_COUNT, p.range)
                                .with("found", es.len().to_string())
                                .with("expected", ps.len().to_string());
                            self.report(d);
                            Ty::Error
                        } else {
                            let d = self.error(msg::TYPE_PATTERN_MISMATCH, p.range).with("ty", self.show(&t));
                            self.report(d);
                            Ty::Error
                        };
                        ps.iter().map(|_| what.clone()).collect()
                    }
                };
                let pats = ps
                    .iter()
                    .zip(elems.iter())
                    .map(|(sp, et)| self.check_pat_with(sp, et, by_ref, mutable, warn))
                    .collect();
                PatKind::Tuple(pats)
            }
            ast::PatKind::List(ps) => {
                let elem = match &t {
                    Ty::Adt(d, a) if Some(*d) == self.prog.wk.list => a.first().cloned().unwrap_or(Ty::Error),
                    Ty::Error => Ty::Error,
                    _ => {
                        let d = self.error(msg::TYPE_PATTERN_MISMATCH, p.range).with("ty", self.show(&t));
                        self.report(d);
                        Ty::Error
                    }
                };
                let mut prefix = Vec::new();
                let mut suffix = Vec::new();
                let mut rest: Option<Option<LocalId>> = None;
                for sp in ps {
                    if let ast::PatKind::Star(name) = &sp.kind {
                        let binding = name.as_ref().map(|n| {
                            let lt = Ty::Ref(Box::new(t.clone()), false);
                            let lt = if by_ref.is_some() { lt } else { t.clone() };
                            let l = self.new_local(&n.text, lt, mutable, self.span(n.range));
                            self.declare(&n.text, l, warn, n.range);
                            l
                        });
                        rest = Some(binding);
                        continue;
                    }
                    let pat = self.check_pat_with(sp, &elem, by_ref, mutable, warn);
                    if rest.is_some() { suffix.push(pat) } else { prefix.push(pat) }
                }
                PatKind::List { prefix, rest, suffix }
            }
            ast::PatKind::Star(_) => {
                let d = self.error(msg::TYPE_STAR_OUTSIDE_LIST, p.range);
                self.report(d);
                PatKind::Error
            }
            ast::PatKind::Rest => {
                let d = self.error(msg::TYPE_REST_OUTSIDE_ARGUMENTS, p.range);
                self.report(d);
                PatKind::Error
            }
            ast::PatKind::Or(ps) => {
                let before = self.locals.len();
                let pats: Vec<Pat> =
                    ps.iter().map(|sp| self.check_pat_with(sp, &t, by_ref, mutable, warn)).collect();
                if self.locals.len() > before {
                    let d = self.error(msg::TYPE_BINDINGS_IN_OR_PATTERN, p.range);
                    self.report(d);
                }
                PatKind::Or(pats)
            }
        };
        Pat { kind, ty: t, span }
    }

    fn literal_pat(&mut self, e: &ast::Expr, t: &Ty) -> PatKind {
        let x = self.infer_expr(e, Some(t));
        let lit = match &x.kind {
            ExprKind::Int(n) => Lit::Int(*n),
            ExprKind::Float(f) => Lit::Float(*f),
            ExprKind::Char(c) => Lit::Char(*c),
            ExprKind::Str(s) => Lit::Str(s.clone()),
            ExprKind::Bool(b) => Lit::Bool(*b),
            _ => return PatKind::Error,
        };
        let lt = self.resolved(&x.ty);
        let target = if self.is_string(t) { Ty::Str } else { t.clone() };
        let lt = if self.is_string(&lt) { Ty::Str } else { lt };
        if !self.infer.try_unify(&lt, &target) {
            let d = self.error(msg::TYPE_PATTERN_MISMATCH, e.range).with("ty", self.show(t));
            self.report(d);
            return PatKind::Error;
        }
        PatKind::Lit(lit)
    }

    #[allow(clippy::too_many_arguments)]
    fn ctor_pat(
        &mut self,
        path: &[ast::Name],
        args: Option<&[ast::PatArg]>,
        t: &Ty,
        by_ref: Option<bool>,
        mutable: bool,
        warn: bool,
        range: TextRange,
    ) -> PatKind {
        let prog = self.prog;
        let Some(last) = path.last() else { return PatKind::Error };
        let name = last.text.as_str();
        if path.len() == 1 && (name == "True" || name == "False") && args.is_none() {
            if !self.infer.try_unify(t, &Ty::Bool) {
                let d = self.error(msg::TYPE_PATTERN_MISMATCH, range).with("ty", self.show(t));
                self.report(d);
            }
            return PatKind::Lit(Lit::Bool(name == "True"));
        }
        // Resolve the variant: in the scrutinee's type first.
        let mut found: Option<(DefId, u32)> = None;
        if path.len() == 1 {
            found = self.variant_in(t, name);
            if found.is_none() {
                if let Some(d) = prog.lookup(self.module, name) {
                    match &prog.def(d).kind {
                        DefKind::Struct(_) => {
                            return self.struct_pat(d, args, t, by_ref, mutable, warn, range);
                        }
                        DefKind::Const(_) => {
                            let diag = self.error(msg::TYPE_CONST_PATTERN, range);
                            self.report(diag);
                            return PatKind::Error;
                        }
                        _ => {}
                    }
                }
                found = self.global_variant(name, last.range);
            }
        } else {
            let prefix = ast::Expr {
                node: sisu_syntax::NodeId(0),
                range: path[0].range.cover(path[path.len() - 2].range),
                kind: path_expr(&path[..path.len() - 1]),
            };
            if let Some(StaticPath::Type(ty)) = self.static_path(&prefix) {
                found = self.variant_in(&ty, name);
            }
        }
        let Some((def, vi)) = found else {
            if *t != Ty::Error {
                let d = self
                    .error(msg::TYPE_UNKNOWN_VARIANT, last.range)
                    .with("ty", self.show(t))
                    .with("name", name);
                self.report(d);
            }
            // The names the pattern binds still exist (with the error type),
            // so the arm's body reports nothing about them.
            for a in args.unwrap_or(&[]) {
                if let ast::PatArg::Positional(p) | ast::PatArg::Keyword(_, p) = a {
                    let _ = self.check_pat_with(p, &Ty::Error, by_ref, mutable, warn);
                }
            }
            return PatKind::Error;
        };
        // The variant must belong to the scrutinee's type.
        let targs: Vec<Ty> = match t {
            Ty::Adt(d, a) if *d == def => a.clone(),
            Ty::ErrSet(m) if m.contains(&def) => Vec::new(),
            Ty::Var(_) => {
                let (owner, a) = self.variant_owner_ty(def, None);
                self.infer.unify(t, &owner);
                a
            }
            Ty::Error => Vec::new(),
            _ => {
                let d = self.error(msg::TYPE_PATTERN_MISMATCH, range).with("ty", self.show(t));
                self.report(d);
                return PatKind::Error;
            }
        };
        let Some(v) = prog.enum_info(def).and_then(|e| e.variants.get(vi as usize)).cloned() else {
            return PatKind::Error;
        };
        let ftys: Vec<Ty> = v.fields.iter().map(|(_, ft)| ft.subst(&targs)).collect();
        let names: Vec<Option<String>> = v.fields.iter().map(|(n, _)| n.clone()).collect();
        let fields = match args {
            None => {
                if !ftys.is_empty() {
                    let d = self.error(msg::TYPE_VARIANT_NEEDS_FIELDS, range).with("name", v.name.clone());
                    self.report(d);
                }
                ftys.iter()
                    .map(|ft| Pat { kind: PatKind::Wild, ty: ft.clone(), span: self.span(range) })
                    .collect()
            }
            Some(list) => self.field_pats(&v.name, &names, &ftys, list, by_ref, mutable, warn, range),
        };
        PatKind::Variant { def, variant: vi, fields }
    }

    #[allow(clippy::too_many_arguments)]
    fn struct_pat(
        &mut self,
        def: DefId,
        args: Option<&[ast::PatArg]>,
        t: &Ty,
        by_ref: Option<bool>,
        mutable: bool,
        warn: bool,
        range: TextRange,
    ) -> PatKind {
        let prog = self.prog;
        let Some(s) = prog.struct_info(def) else { return PatKind::Error };
        if s.builtin.is_some() {
            let d =
                self.error(msg::TYPE_NOT_A_STRUCT_PATTERN, range).with("name", prog.def(def).name.clone());
            self.report(d);
            return PatKind::Error;
        }
        if !self.infer.try_unify(t, &Ty::Adt(def, Vec::new())) {
            let d = self.error(msg::TYPE_PATTERN_MISMATCH, range).with("ty", self.show(t));
            self.report(d);
            return PatKind::Error;
        }
        let ftys: Vec<Ty> = s.fields.iter().map(|f| f.ty.clone()).collect();
        let names: Vec<Option<String>> = s.fields.iter().map(|f| Some(f.name.clone())).collect();
        let list = args.unwrap_or(&[]);
        let fields = self.field_pats(&prog.def(def).name, &names, &ftys, list, by_ref, mutable, warn, range);
        PatKind::Struct { def, fields }
    }

    #[allow(clippy::too_many_arguments)]
    fn field_pats(
        &mut self,
        owner: &str,
        names: &[Option<String>],
        ftys: &[Ty],
        list: &[ast::PatArg],
        by_ref: Option<bool>,
        mutable: bool,
        warn: bool,
        range: TextRange,
    ) -> Vec<Pat> {
        let n = ftys.len();
        let mut slots: Vec<Option<&ast::Pattern>> = vec![None; n];
        let mut pos = 0;
        let mut rest = false;
        let mut count = 0;
        for a in list {
            match a {
                ast::PatArg::Positional(p) => {
                    if pos < n {
                        slots[pos] = Some(p);
                    }
                    pos += 1;
                    count += 1;
                }
                ast::PatArg::Keyword(k, p) => {
                    count += 1;
                    match names.iter().position(|x| x.as_deref() == Some(k.text.as_str())) {
                        Some(i) => slots[i] = Some(p),
                        None => {
                            let d = self
                                .error(msg::TYPE_UNKNOWN_FIELD, k.range)
                                .with("ty", owner)
                                .with("name", k.text.clone());
                            self.report(d);
                        }
                    }
                }
                ast::PatArg::Rest => rest = true,
            }
        }
        let filled = slots.iter().filter(|s| s.is_some()).count();
        if pos > n || (!rest && filled < n) {
            let d = self
                .error(msg::TYPE_PATTERN_FIELD_COUNT, range)
                .with("name", owner)
                .with("expected", n.to_string())
                .with("found", count.to_string());
            self.report(d);
        }
        let span = self.span(range);
        (0..n)
            .map(|i| match slots[i] {
                Some(p) => self.check_pat_with(p, &ftys[i], by_ref, mutable, warn),
                None => Pat { kind: PatKind::Wild, ty: ftys[i].clone(), span },
            })
            .collect()
    }
}

fn path_expr(names: &[ast::Name]) -> ast::ExprKind {
    let mut kind = ast::ExprKind::Name(names[0].text.clone());
    for n in &names[1..] {
        let base = ast::Expr { node: sisu_syntax::NodeId(0), range: n.range, kind };
        kind = ast::ExprKind::Field { base: Box::new(base), name: n.clone() };
    }
    kind
}
