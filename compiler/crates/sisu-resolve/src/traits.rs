//! Traits and impls (03, "Traits"): declarations, impl headers, and the
//! checks that need every impl: each impl defines what its trait
//! requires with the trait's signatures, coherence (at most one impl per
//! trait and type) with the orphan rule and the root-package exception
//! (D-013, Q-11), supertraits, and object safety for `dyn`.

use crate::collect::{Owner, Pending, Resolver, TyCtx};
use crate::program::*;
use crate::solve::headers_overlap;
use crate::ty::Ty;
use sisu_diag::{Diagnostic, Span, TextRange, msg};
use sisu_syntax::ast::{self, Mode, ParamKind};

impl Resolver<'_> {
    pub fn resolve_trait(&mut self, def: DefId, module: ModuleId, t: &ast::TraitDef, attrs: &[ast::Attr]) {
        let generics = self.prog.trait_info(def).map(|i| i.generics.clone()).unwrap_or_default();
        let mut names = vec!["Self".to_string()];
        names.extend(generics.iter().map(|g| g.name.clone()));
        let self_ty = Ty::Param(0);
        // Associated types first: the methods' signatures may name them.
        let mut assoc: Vec<AssocInfo> = Vec::new();
        for a in &t.assoc_types {
            if assoc.iter().any(|x| x.name == a.name.text) {
                let span = self.span(module, a.name.range);
                self.report(
                    Diagnostic::new(msg::RESOLVE_DUPLICATE_DEFINITION, span)
                        .with("name", a.name.text.clone()),
                );
                continue;
            }
            assoc.push(AssocInfo {
                name: a.name.text.clone(),
                default: None,
                span: self.span(module, a.range),
            });
        }
        if let DefKind::Trait(info) = &mut self.prog.defs[def.0 as usize].kind {
            info.assoc = assoc.clone();
            info.sealed = attrs.iter().any(|a| a.name == "sealed");
            info.marker = attrs.iter().any(|a| a.name == "marker");
        }
        for (i, a) in t.assoc_types.iter().enumerate() {
            if let Some(te) = &a.ty {
                let ctx = TyCtx {
                    self_ty: Some(&self_ty),
                    in_trait: Some(def),
                    ..TyCtx::plain(module, &names, true)
                };
                let ty = self.resolve_ty(te, &ctx);
                if let DefKind::Trait(info) = &mut self.prog.defs[def.0 as usize].kind
                    && let Some(x) = info.assoc.get_mut(i)
                {
                    x.default = Some(ty);
                }
            }
        }
        let mut supers = Vec::new();
        for s in &t.supertraits {
            let ctx =
                TyCtx { self_ty: Some(&self_ty), in_trait: Some(def), ..TyCtx::plain(module, &names, true) };
            if let Some(tr) = self.resolve_bound(s, &self_ty, &ctx) {
                if tr.def == def {
                    let span = self.span(module, s.range);
                    self.report(
                        Diagnostic::new(msg::RESOLVE_TRAIT_CYCLE, span).with("name", t.name.text.clone()),
                    );
                    continue;
                }
                supers.push(tr);
            }
        }
        if let DefKind::Trait(info) = &mut self.prog.defs[def.0 as usize].kind {
            info.supertraits = supers;
        }
        for m in &t.methods {
            if t.methods.iter().filter(|x| x.name.text == m.name.text).count() > 1
                && t.methods.iter().position(|x| x.name.text == m.name.text)
                    != t.methods.iter().position(|x| std::ptr::eq(x, m))
            {
                let span = self.span(module, m.name.range);
                self.report(
                    Diagnostic::new(msg::RESOLVE_DUPLICATE_METHOD, span)
                        .with("owner", t.name.text.clone())
                        .with("name", m.name.text.clone()),
                );
                continue;
            }
            let fdef = self.new_def(
                module,
                &m.name.text,
                m.name.range,
                ast::Vis::Pub,
                Vec::new(),
                DefKind::Fn(self.placeholder_fn(module, FnKind::Method)),
            );
            self.pending.push(Pending::Fn {
                def: fdef,
                module,
                ast: m.clone(),
                owner: Some(Owner::Trait(def)),
            });
        }
    }

    pub fn resolve_impl(&mut self, module: ModuleId, imp: &ast::ImplDef, range: TextRange, derived: bool) {
        let span = self.span(module, range);
        let names: Vec<String> = imp.generics.iter().map(|g| g.name.text.clone()).collect();
        for g in &imp.generics {
            if g.default.is_some() {
                let s = self.span(module, g.name.range);
                self.report(Diagnostic::new(msg::RESOLVE_IMPL_DEFAULT, s));
            }
        }
        let generics: Vec<Generic> =
            names.iter().map(|n| Generic { name: n.clone(), default: None }).collect();
        let predicates = self.resolve_bounds(module, &imp.generics, &names, 0, None);
        let ctx = TyCtx { predicates: &predicates, ..TyCtx::plain(module, &names, true) };
        let self_ty = self.resolve_ty(&imp.self_ty, &ctx);
        let Some(trait_ty) = &imp.trait_ty else {
            // An inherent impl adds methods to a type.
            if self_ty.key().is_none() {
                if self_ty != Ty::Error {
                    let s = self.span(module, imp.self_ty.range);
                    self.report(
                        Diagnostic::new(msg::RESOLVE_NOT_A_TYPE, s).with("name", self.prog.display(&self_ty)),
                    );
                }
                return;
            }
            for a in &imp.assoc_types {
                let s = self.span(module, a.range);
                self.report(Diagnostic::new(msg::RESOLVE_ASSOC_OUTSIDE_TRAIT_IMPL, s));
            }
            for m in &imp.methods {
                let def = self.new_def(
                    module,
                    &m.name.text,
                    m.name.range,
                    ast::Vis::Pub,
                    Vec::new(),
                    DefKind::Fn(self.placeholder_fn(module, FnKind::Method)),
                );
                let owner = Owner::Inherent {
                    generics: generics.clone(),
                    names: names.clone(),
                    predicates: predicates.clone(),
                    self_ty: self_ty.clone(),
                };
                self.pending.push(Pending::Fn { def, module, ast: m.clone(), owner: Some(owner) });
            }
            return;
        };
        let ctx =
            TyCtx { self_ty: Some(&self_ty), predicates: &predicates, ..TyCtx::plain(module, &names, true) };
        let Some(trait_ref) = self.resolve_bound(trait_ty, &self_ty, &ctx) else { return };
        if self_ty == Ty::Error {
            return;
        }
        // `Drop` keeps its stage-0 rules: a struct or enum of this module,
        // not generic (its drop glue is computed from values, which do not
        // carry type arguments), with exactly `def drop(mut self)`.
        if Some(trait_ref.def) == self.prog.wk.drop {
            let target = match &self_ty {
                Ty::Adt(d, _)
                    if self.prog.def(*d).module == module
                        && matches!(self.prog.def(*d).kind, DefKind::Struct(_) | DefKind::Enum(_)) =>
                {
                    Some(*d)
                }
                _ => None,
            };
            let Some(target) = target else {
                self.report(Diagnostic::new(msg::RESOLVE_DROP_TARGET, span));
                return;
            };
            if !self.prog.type_generics(target).is_empty() {
                self.unsupported(
                    span,
                    "`Drop` for a generic type",
                    "drop glue in SIR (Phase 4, std in Sisu)",
                );
                return;
            }
            let ok = imp.methods.len() == 1
                && imp.methods[0].name.text == "drop"
                && imp.methods[0].params.len() == 1
                && matches!(imp.methods[0].params[0].kind, ParamKind::SelfParam)
                && imp.methods[0].params[0].mode == Mode::Mut
                && imp.methods[0].ret.is_none();
            if !ok {
                self.report(Diagnostic::new(msg::RESOLVE_DROP_SIGNATURE, span));
                return;
            }
        }
        // What the impl binds its associated types to.
        let mut assoc: Vec<(String, Ty)> = Vec::new();
        for a in &imp.assoc_types {
            let ty = match &a.ty {
                Some(te) => self.resolve_ty(te, &ctx),
                None => {
                    let s = self.span(module, a.range);
                    self.report(
                        Diagnostic::new(msg::RESOLVE_ASSOC_NEEDS_TYPE, s).with("name", a.name.text.clone()),
                    );
                    Ty::Error
                }
            };
            assoc.push((a.name.text.clone(), ty));
        }
        let name = format!(
            "impl {} for {}",
            self.prog.display_trait_in(&trait_ref, &names),
            self.prog.display_in(&self_ty, &names)
        );
        let info = ImplInfo {
            generics: generics.clone(),
            predicates: predicates.clone(),
            trait_ref: trait_ref.clone(),
            self_ty: self_ty.clone(),
            methods: Vec::new(),
            assoc,
            derived,
        };
        let def = self.new_def(module, &name, range, ast::Vis::Pub, Vec::new(), DefKind::Impl(info));
        self.prog.impls.push(def);
        for m in &imp.methods {
            let fdef = self.new_def(
                module,
                &m.name.text,
                m.name.range,
                ast::Vis::Pub,
                Vec::new(),
                DefKind::Fn(self.placeholder_fn(module, FnKind::Method)),
            );
            if Some(trait_ref.def) == self.prog.wk.drop
                && let Ty::Adt(d, _) = &self_ty
            {
                match &mut self.prog.defs[d.0 as usize].kind {
                    DefKind::Struct(s) => s.drop = Some(fdef),
                    DefKind::Enum(e) => e.drop = Some(fdef),
                    _ => {}
                }
            }
            self.pending.push(Pending::Fn {
                def: fdef,
                module,
                ast: m.clone(),
                owner: Some(Owner::Impl(def)),
            });
        }
    }

    // ---------------------------------------------------------------- checks

    /// The checks that need every trait and impl resolved.
    pub fn check_traits(&mut self) {
        let impls = self.prog.impls.clone();
        for &i in &impls {
            self.check_impl(i);
        }
        self.check_coherence(&impls);
        self.check_object_safety();
        self.check_derives();
    }

    fn check_impl(&mut self, i: DefId) {
        let Some(info) = self.prog.impl_info(i).cloned() else { return };
        let span = self.prog.def(i).span;
        let module = self.prog.def(i).module;
        let tr = &info.trait_ref;
        let Some(tinfo) = self.prog.trait_info(tr.def).cloned() else { return };
        let trait_name = self.prog.display_trait(tr);
        let names: Vec<String> = info.generics.iter().map(|g| g.name.clone()).collect();
        let ty_name = self.prog.display_in(&info.self_ty, &names);
        let in_std = self.prog.modules[module.0 as usize].is_std;
        // Only std implements a sealed trait.
        if tinfo.sealed && !in_std {
            self.report(
                Diagnostic::new(msg::RESOLVE_SEALED_TRAIT, span)
                    .with("trait", trait_name.clone())
                    .with("ty", ty_name.clone()),
            );
            return;
        }
        // The orphan rule: an impl lives in the package of the trait or of
        // the type, except in the root package, which nothing depends on.
        let package = self.prog.modules[module.0 as usize].package();
        if package != 0 {
            let trait_pkg = self.prog.modules[self.prog.def(tr.def).module.0 as usize].package();
            let type_pkg = match &info.self_ty {
                Ty::Adt(d, _) => Some(self.prog.modules[self.prog.def(*d).module.0 as usize].package()),
                _ => None,
            };
            if trait_pkg != package && type_pkg != Some(package) && !in_std {
                self.report(
                    Diagnostic::new(msg::RESOLVE_ORPHAN_IMPL, span)
                        .with("trait", trait_name.clone())
                        .with("ty", ty_name.clone()),
                );
            }
        }
        // Methods: each is the trait's, with the trait's signature.
        let mut targs = vec![info.self_ty.clone()];
        targs.extend(tr.args.iter().cloned());
        let env = self.prog.elaborate_env(&info.predicates);
        for (mname, mdef) in &info.methods {
            let Some(tm) = self.prog.trait_method(tr.def, mname) else {
                let s = self.prog.def(*mdef).span;
                self.report(
                    Diagnostic::new(msg::RESOLVE_NOT_A_TRAIT_METHOD, s)
                        .with("name", mname.clone())
                        .with("trait", trait_name.clone()),
                );
                continue;
            };
            self.check_method_signature(tm, *mdef, &targs, &info, &env, &trait_name);
            // An impl's method is as pure as the trait's: `pure` on the
            // trait's method binds every impl, written or not.
            if self.prog.fn_info(tm).is_some_and(|t| t.pure)
                && let DefKind::Fn(f) = &mut self.prog.defs[mdef.0 as usize].kind
            {
                f.pure = true;
            }
        }
        let missing: Vec<String> = tinfo
            .methods
            .iter()
            .filter(|m| {
                let f = self.prog.fn_info(**m);
                f.is_some_and(|f| f.body.is_none() && f.intrinsic.is_none() && !f.header_broken)
                    && !info.methods.iter().any(|(n, _)| *n == self.prog.def(**m).name)
            })
            .map(|m| format!("`{}`", self.prog.def(*m).name))
            .collect();
        if !missing.is_empty() {
            self.report(
                Diagnostic::new(msg::RESOLVE_MISSING_TRAIT_METHOD, span)
                    .with("trait", trait_name.clone())
                    .with("ty", ty_name.clone())
                    .with("methods", missing.join(", ")),
            );
        }
        // Associated types: each one the trait declares, bound once.
        for (n, _) in &info.assoc {
            if !tinfo.assoc.iter().any(|a| a.name == *n) {
                self.report(
                    Diagnostic::new(msg::RESOLVE_UNKNOWN_ASSOC_TYPE, span)
                        .with("trait", trait_name.clone())
                        .with("name", n.clone()),
                );
            }
        }
        for a in &tinfo.assoc {
            if a.default.is_none() && !info.assoc.iter().any(|(n, _)| *n == a.name) {
                self.report(
                    Diagnostic::new(msg::RESOLVE_MISSING_ASSOC_TYPE, span)
                        .with("trait", trait_name.clone())
                        .with("ty", ty_name.clone())
                        .with("name", a.name.clone()),
                );
            }
        }
        // Supertraits: implementing `Container(Len)` needs `Len` too.
        for s in &tinfo.supertraits {
            let st = s.subst(&targs);
            if !self.prog.implements(&info.self_ty, &st, &env).holds()
                && !matches!(self.prog.implements(&info.self_ty, &st, &env), crate::solve::Answer::Unknown)
            {
                self.report(
                    Diagnostic::new(msg::RESOLVE_SUPERTRAIT_MISSING, span)
                        .with("ty", ty_name.clone())
                        .with("trait", trait_name.clone())
                        .with("super", self.prog.display_trait_in(&st, &names)),
                );
            }
        }
    }

    /// An impl's method has the trait's signature with `Self` and the
    /// trait's parameters substituted and associated types bound.
    fn check_method_signature(
        &mut self,
        tm: DefId,
        im: DefId,
        targs: &[Ty],
        info: &ImplInfo,
        env: &[Predicate],
        trait_name: &str,
    ) {
        let (Some(t), Some(f)) = (self.prog.fn_info(tm).cloned(), self.prog.fn_info(im).cloned()) else {
            return;
        };
        if t.header_broken || f.header_broken {
            return;
        }
        let span = self.prog.def(im).span;
        let name = self.prog.def(im).name.clone();
        // The trait method's own parameters line up with the impl method's.
        let own_t = t.generics.len() - t.impl_generics;
        let own_f = f.generics.len() - f.impl_generics;
        let mut args = targs.to_vec();
        for k in 0..own_t {
            args.push(Ty::Param((f.impl_generics + k) as u32));
        }
        let norm = |p: &Program, ty: &Ty| p.normalize(&ty.subst(&args), env);
        let mut ok = own_t == own_f && t.self_mode == f.self_mode && t.params.len() == f.params.len();
        if ok {
            for (a, b) in t.params.iter().zip(&f.params) {
                if a.mode != b.mode || norm(&self.prog, &a.ty) != self.prog.normalize(&b.ty, env) {
                    ok = false;
                }
            }
            if norm(&self.prog, &t.ret) != self.prog.normalize(&f.ret, env) {
                ok = false;
            }
        }
        if !ok {
            let names: Vec<String> = f.generics.iter().map(|g| g.name.clone()).collect();
            let expected = self.signature_text(&name, &t, Some(&args), env, &names);
            let found = self.signature_text(&name, &f, None, env, &names);
            self.report(
                Diagnostic::new(msg::RESOLVE_METHOD_SIGNATURE, span)
                    .with("name", name)
                    .with("trait", trait_name.to_string())
                    .with("expected", expected)
                    .with("found", found),
            );
        }
        let _ = info;
    }

    fn signature_text(
        &self,
        name: &str,
        f: &FnInfo,
        args: Option<&[Ty]>,
        env: &[Predicate],
        names: &[String],
    ) -> String {
        let ty = |t: &Ty| {
            let t = match args {
                Some(a) => self.prog.normalize(&t.subst(a), env),
                None => t.clone(),
            };
            self.prog.display_in(&t, names)
        };
        let mode = |m: Mode| match m {
            Mode::Borrow => "",
            Mode::Mut => "mut ",
            Mode::Own => "own ",
        };
        let mut parts = Vec::new();
        if let Some(m) = f.self_mode {
            parts.push(format!("{}self", mode(m)));
        }
        for p in &f.params {
            parts.push(format!("{}{}: {}", mode(p.mode), p.name, ty(&p.ty)));
        }
        let ret = if f.ret == Ty::Unit { String::new() } else { format!(" -> {}", ty(&f.ret)) };
        format!("{}def {name}({}){ret}", if f.pure { "pure " } else { "" }, parts.join(", "))
    }

    /// At most one impl of a trait for any type (D-013).
    fn check_coherence(&mut self, impls: &[DefId]) {
        for (k, &a) in impls.iter().enumerate() {
            for &b in &impls[..k] {
                let (Some(ia), Some(ib)) = (self.prog.impl_info(a), self.prog.impl_info(b)) else { continue };
                if ia.trait_ref.def != ib.trait_ref.def {
                    continue;
                }
                let ta = Ty::Tuple(
                    std::iter::once(ia.self_ty.clone()).chain(ia.trait_ref.args.iter().cloned()).collect(),
                    Vec::new(),
                );
                let tb = Ty::Tuple(
                    std::iter::once(ib.self_ty.clone()).chain(ib.trait_ref.args.iter().cloned()).collect(),
                    Vec::new(),
                );
                if headers_overlap(&ta, ia.generics.len(), &tb, ib.generics.len()) {
                    let names: Vec<String> = ia.generics.iter().map(|g| g.name.clone()).collect();
                    let d = Diagnostic::new(msg::RESOLVE_OVERLAPPING_IMPL, self.prog.def(a).span)
                        .with("trait", self.prog.display_trait_in(&ia.trait_ref, &names))
                        .with("ty", self.prog.display_in(&ia.self_ty, &names))
                        .label(self.prog.def(b).span, msg::RESOLVE_FIRST_IMPL, Vec::new());
                    self.report(d);
                    break;
                }
            }
        }
    }

    /// Why a trait cannot be `dyn`, if it cannot: a method without `self`
    /// by reference, with its own type parameters, or mentioning `Self`
    /// elsewhere than its receiver.
    pub fn object_safety_problem(&self, t: DefId) -> Option<(String, String)> {
        for tr in self.prog.elaborate(&Ty::Param(0), &TraitRef { def: t, args: Vec::new() }) {
            let Some(info) = self.prog.trait_info(tr.def) else { continue };
            for &m in &info.methods {
                let Some(f) = self.prog.fn_info(m) else { continue };
                let name = self.prog.def(m).name.clone();
                if !matches!(f.self_mode, Some(Mode::Borrow | Mode::Mut)) {
                    return Some((name, "it does not take `self` or `mut self`".into()));
                }
                if f.generics.len() > f.impl_generics {
                    return Some((name, "it has type parameters of its own".into()));
                }
                let mentions_self = |t: &Ty| {
                    let mut found = false;
                    t.walk(&mut |x| found |= *x == Ty::Param(0));
                    found
                };
                if f.params.iter().any(|p| mentions_self(&p.ty)) || mentions_self(&f.ret) {
                    return Some((name, "its signature mentions `Self` besides `self`".into()));
                }
            }
        }
        None
    }

    fn check_object_safety(&mut self) {
        let uses = std::mem::take(&mut self.dyn_uses);
        let mut seen: Vec<(DefId, Span)> = Vec::new();
        for (t, span) in uses {
            if seen.iter().any(|(d, s)| *d == t && *s == span) {
                continue;
            }
            seen.push((t, span));
            if let Some((method, reason)) = self.object_safety_problem(t) {
                let name = self.prog.def(t).name.clone();
                self.report(
                    Diagnostic::new(msg::RESOLVE_NOT_OBJECT_SAFE, span)
                        .with("trait", name)
                        .with("method", method)
                        .with("reason", reason),
                );
            }
        }
    }
}
