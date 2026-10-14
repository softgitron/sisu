//! Types as written: names, generic arguments with defaults, bounds,
//! `dyn Trait` and associated types (`C.Item`, `Self.Item`).

use crate::collect::{LATER_TYPES, Pending, Resolver, TyCtx};
use crate::program::*;
use crate::ty::{AssocTy, FnTy, Ty};
use sisu_diag::{Diagnostic, Span, msg};
use sisu_syntax::ast::{self, TypeKind};

impl Resolver<'_> {
    /// The generic parameters of every struct, enum and trait, with their
    /// defaults, then the bounds of the parameters of structs and enums.
    /// Defaults come after the parameters without one (02, "Declarations").
    pub fn resolve_headers(&mut self) {
        let mut ids: Vec<DefId> =
            self.type_params.keys().copied().filter(|d| !self.headers_done.contains(d)).collect();
        ids.sort();
        self.headers_done.extend(ids.iter().copied());
        for &d in &ids {
            let gps = self.type_params.get(&d).cloned().unwrap_or_default();
            let module = self.prog.def(d).module;
            let is_trait = matches!(self.prog.def(d).kind, DefKind::Trait(_));
            // A trait's defaults may name `Self`, which is parameter 0.
            let mut names: Vec<String> = if is_trait { vec!["Self".into()] } else { Vec::new() };
            let mut generics = Vec::new();
            let mut seen_default = false;
            for g in &gps {
                let default = match &g.default {
                    Some(t) => {
                        seen_default = true;
                        let self_ty = Ty::Param(0);
                        let ctx = TyCtx {
                            self_ty: if is_trait { Some(&self_ty) } else { None },
                            ..TyCtx::plain(module, &names, true)
                        };
                        Some(self.resolve_ty(t, &ctx))
                    }
                    None => {
                        if seen_default {
                            let span = self.span(module, g.name.range);
                            self.report(Diagnostic::new(msg::RESOLVE_GENERIC_DEFAULT_ORDER, span));
                        }
                        None
                    }
                };
                if g.is_const {
                    let span = self.span(module, g.name.range);
                    self.unsupported(span, "a `const` generic parameter", "const generics (Phase 4)");
                }
                names.push(g.name.text.clone());
                generics.push(Generic { name: g.name.text.clone(), default });
            }
            match &mut self.prog.defs[d.0 as usize].kind {
                DefKind::Struct(s) => s.generics = generics,
                DefKind::Enum(e) | DefKind::ErrorSet(e) => e.generics = generics,
                DefKind::Trait(t) => t.generics = generics,
                DefKind::Refinement(r) => r.generics = generics,
                _ => {}
            }
        }
        for &d in &ids {
            if matches!(self.prog.def(d).kind, DefKind::Trait(_)) {
                continue;
            }
            let gps = self.type_params.get(&d).cloned().unwrap_or_default();
            let module = self.prog.def(d).module;
            let names: Vec<String> = gps.iter().map(|g| g.name.text.clone()).collect();
            let preds = self.resolve_bounds(module, &gps, &names, 0, None);
            self.type_preds.insert(d, preds);
        }
    }

    /// The bounds of the generic parameters `gps`, which are the names
    /// from `offset` on in `names`, as predicates on `Param(offset + i)`.
    pub fn resolve_bounds(
        &mut self,
        module: ModuleId,
        gps: &[ast::GenericParam],
        names: &[String],
        offset: usize,
        in_trait: Option<DefId>,
    ) -> Vec<Predicate> {
        let mut out: Vec<Predicate> = Vec::new();
        for (k, g) in gps.iter().enumerate() {
            let subject = Ty::Param((offset + k) as u32);
            // A `const` parameter's "bound" is its type (reported, E0900).
            if g.is_const {
                continue;
            }
            for b in &g.bounds {
                let ctx = TyCtx { predicates: &out, in_trait, ..TyCtx::plain(module, names, true) };
                if let Some(tr) = self.resolve_bound(b, &subject, &ctx) {
                    let span = self.span(module, b.range);
                    out.push(Predicate { ty: subject.clone(), bound: tr, span });
                }
            }
        }
        out
    }

    /// A trait named in a bound, a supertrait list or an impl header, for
    /// `subject` (which a defaulted parameter `Rhs = Self` takes).
    pub fn resolve_bound(&mut self, te: &ast::TypeExpr, subject: &Ty, ctx: &TyCtx) -> Option<TraitRef> {
        let span = self.span(ctx.module, te.range);
        let TypeKind::Path { segments, args } = &te.kind else {
            if !matches!(te.kind, TypeKind::Error) {
                self.report(
                    Diagnostic::new(msg::RESOLVE_NOT_A_TRAIT, span).with("name", self.source_of(span)),
                );
            }
            return None;
        };
        let d = self.lookup_path(segments, ctx, span)?;
        let name = segments.iter().map(|s| s.text.clone()).collect::<Vec<_>>().join(".");
        let Some(info) = self.prog.trait_info(d) else {
            self.report(Diagnostic::new(msg::RESOLVE_NOT_A_TRAIT, span).with("name", name));
            return None;
        };
        let generics = info.generics.clone();
        let mut targs: Vec<Ty> = args.iter().map(|a| self.resolve_ty(a, ctx)).collect();
        if targs.len() > generics.len() {
            self.report(
                Diagnostic::new(msg::RESOLVE_TYPE_ARGUMENT_COUNT, span)
                    .with("name", name)
                    .with("expected", generics.len().to_string())
                    .with("found", targs.len().to_string()),
            );
            return None;
        }
        // Omitted trailing arguments take their defaults, `Self` being the
        // subject.
        while targs.len() < generics.len() {
            match &generics[targs.len()].default {
                Some(def) => {
                    let mut env = vec![subject.clone()];
                    env.extend(targs.iter().cloned());
                    targs.push(def.subst(&env));
                }
                None => {
                    self.report(
                        Diagnostic::new(msg::RESOLVE_TYPE_ARGUMENT_COUNT, span)
                            .with("name", name)
                            .with("expected", generics.len().to_string())
                            .with("found", targs.len().to_string()),
                    );
                    return None;
                }
            }
        }
        Some(TraitRef { def: d, args: targs })
    }

    /// The source text of a span.
    pub fn source_of(&self, span: Span) -> String {
        self.sources.slice(span).to_string()
    }

    /// Resolves a possibly qualified name (`cli.Error`) to a definition,
    /// reporting what cannot be found.
    fn lookup_path(&mut self, segments: &[ast::Name], ctx: &TyCtx, span: Span) -> Option<DefId> {
        let mut module = ctx.module;
        for (i, seg) in segments.iter().enumerate() {
            let last = i + 1 == segments.len();
            let found = if i == 0 {
                self.prog.lookup(module, &seg.text)
            } else {
                self.prog.lookup_in(module, &seg.text)
            };
            let Some(d) = found else {
                if i == 0 {
                    if !self.prog.poisoned.contains(&seg.text) {
                        self.report(
                            Diagnostic::new(msg::RESOLVE_UNDEFINED_TYPE, span).with("name", seg.text.clone()),
                        );
                    }
                } else {
                    let mname = self.prog.modules[module.0 as usize].name.clone();
                    self.report(
                        Diagnostic::new(msg::RESOLVE_NOT_IN_MODULE, span)
                            .with("module", mname)
                            .with("name", seg.text.clone()),
                    );
                }
                return None;
            };
            if last {
                return Some(d);
            }
            match &self.prog.def(d).kind {
                DefKind::Module(m) => module = *m,
                _ => {
                    self.report(
                        Diagnostic::new(msg::RESOLVE_NOT_A_MODULE, span).with("name", seg.text.clone()),
                    );
                    return None;
                }
            }
        }
        None
    }

    /// Whether `segments` name `cap.Fake`, whose argument is a trait, in
    /// the program's code (the standard library's own code, such as what
    /// it derives, sees the plain declaration).
    pub fn is_fake_marker(&self, segments: &[ast::Name], module: ModuleId) -> bool {
        if self.is_std(module) {
            return false;
        }
        let mut d = None;
        let mut m = module;
        for (i, seg) in segments.iter().enumerate() {
            let found =
                if i == 0 { self.prog.lookup(m, &seg.text) } else { self.prog.lookup_in(m, &seg.text) };
            let Some(x) = found else { return false };
            if let DefKind::Module(next) = self.prog.def(x).kind {
                m = next;
            }
            d = Some(x);
        }
        d.is_some_and(|d| {
            let def = self.prog.def(d);
            let module = &self.prog.modules[def.module.0 as usize];
            def.name == "Fake" && module.is_std && module.name == "cap"
        })
    }

    /// `cap.Fake[Trait]`: the fake the compiler wrote for the trait (17).
    pub fn fake_ty(&mut self, arg: &ast::TypeExpr, ctx: &TyCtx, span: Span) -> Ty {
        let TypeKind::Path { segments, args } = &arg.kind else {
            let written = self.source_of(self.span(ctx.module, arg.range));
            self.report(Diagnostic::new(msg::COMPTIME_NO_FAKE, span).with("name", written));
            return Ty::Error;
        };
        let trait_def = if args.is_empty() { self.lookup_path(segments, ctx, span) } else { None };
        let name = segments.iter().map(|s| s.text.clone()).collect::<Vec<_>>().join(".");
        match trait_def.and_then(|t| self.prog.fakes.get(&t).copied()) {
            Some(fake) => Ty::Adt(fake, Vec::new()),
            None => {
                self.report(Diagnostic::new(msg::COMPTIME_NO_FAKE, span).with("name", name));
                Ty::Error
            }
        }
    }

    // ---------------------------------------------------------------- types

    pub fn resolve_ty(&mut self, te: &ast::TypeExpr, ctx: &TyCtx) -> Ty {
        let span = self.span(ctx.module, te.range);
        match &te.kind {
            TypeKind::Error => Ty::Error,
            TypeKind::Path { segments, args } => {
                if let [arg] = args.as_slice()
                    && self.is_fake_marker(segments, ctx.module)
                {
                    return self.fake_ty(arg, ctx, span);
                }
                let args: Vec<Ty> = args.iter().map(|a| self.resolve_ty(a, ctx)).collect();
                self.resolve_path_ty(segments, args, ctx, span)
            }
            TypeKind::Option(inner) => {
                let t = self.resolve_ty(inner, ctx);
                match self.prog.wk.option {
                    Some(o) => Ty::Adt(o, vec![t]),
                    None => Ty::Error,
                }
            }
            TypeKind::Result { ok, err } => {
                let ok_ty = match ok {
                    Some(o) => self.resolve_ty(o, ctx),
                    None => Ty::Unit,
                };
                let err_ty = match err {
                    None => {
                        self.report(Diagnostic::new(msg::RESOLVE_INFERRED_SET_POSITION, span));
                        Ty::Error
                    }
                    Some(e) => self.resolve_error_set(e, ctx),
                };
                match self.prog.wk.result {
                    Some(r) => Ty::Adt(r, vec![ok_ty, err_ty]),
                    None => Ty::Error,
                }
            }
            TypeKind::Union(_) => {
                self.report(Diagnostic::new(msg::RESOLVE_UNION_OUTSIDE_RESULT, span));
                Ty::Error
            }
            TypeKind::Tuple(elems) => {
                let tys: Vec<Ty> = elems.iter().map(|(_, t)| self.resolve_ty(t, ctx)).collect();
                let labels: Vec<String> = if !elems.is_empty() && elems.iter().all(|(n, _)| n.is_some()) {
                    elems.iter().filter_map(|(n, _)| n.as_ref().map(|n| n.text.clone())).collect()
                } else {
                    Vec::new()
                };
                if tys.is_empty() { Ty::Unit } else { Ty::Tuple(tys, labels) }
            }
            TypeKind::Fn { pure, params, ret } => {
                let params = params.iter().map(|p| self.resolve_ty(p, ctx)).collect();
                let ret = self.resolve_ty(ret, ctx);
                Ty::Fn(Box::new(FnTy { params, ret, pure: *pure }))
            }
            TypeKind::Ref { mutable, inner } => {
                if !ctx.allow_ref {
                    self.unsupported(
                        span,
                        "a reference type in a declaration",
                        "references in constants and aliases (a later phase)",
                    );
                }
                let t = self.resolve_ty(inner, ctx);
                // A shared borrow of a view is the view: `ref str` is a
                // `str` (04, "Views"), valid while the text it views is.
                if t == Ty::Str && !*mutable {
                    return Ty::Str;
                }
                Ty::Ref(Box::new(t), *mutable)
            }
            TypeKind::Dyn(inner) => match self.resolve_bound(inner, &Ty::Error, ctx) {
                Some(tr) => {
                    self.dyn_uses.push((tr.def, span));
                    Ty::Dyn(tr)
                }
                None => Ty::Error,
            },
            TypeKind::ConstArg(_) => {
                self.unsupported(span, "a constant type argument", "const generics (Phase 4)");
                Ty::Error
            }
            TypeKind::Unsupported(what) => {
                let phase = if what.contains("pointer") {
                    "foreign functions (Phase 8)"
                } else {
                    "reflection beyond derive (Phase 8)"
                };
                self.unsupported(span, what, phase);
                Ty::Error
            }
        }
    }

    /// A `dyn Trait` has no size of its own (03, "Dispatch"): it is held
    /// only behind `ref` or in a `Box`.
    pub fn check_sized(&mut self, ty: &Ty, span: Span) {
        if let Some(d) = crate::ty::unsized_part(&self.prog, ty) {
            let shown = self.prog.display(&d);
            self.report(Diagnostic::new(msg::TYPE_UNSIZED_DYN, span).with("ty", shown));
        }
    }

    pub fn resolve_error_set(&mut self, te: &ast::TypeExpr, ctx: &TyCtx) -> Ty {
        let parts: Vec<&ast::TypeExpr> = match &te.kind {
            TypeKind::Union(list) => list.iter().collect(),
            _ => vec![te],
        };
        let mut members = Vec::new();
        for p in parts {
            match self.resolve_ty(p, ctx) {
                Ty::ErrSet(m) => members.extend(m),
                Ty::Error => return Ty::Error,
                other => {
                    let name = self.prog.display(&other);
                    let s = self.span(ctx.module, p.range);
                    self.report(Diagnostic::new(msg::RESOLVE_NOT_AN_ERROR_SET, s).with("name", name));
                    return Ty::Error;
                }
            }
        }
        members.sort();
        members.dedup();
        Ty::ErrSet(members)
    }

    /// The associated type `name` of `subject`, from the bounds in scope
    /// (and, inside a trait, the trait itself).
    fn assoc_of(&mut self, subject: &Ty, name: &str, ctx: &TyCtx) -> Option<Ty> {
        let mut candidates: Vec<TraitRef> = Vec::new();
        if let (Some(t), Ty::Param(0)) = (ctx.in_trait, subject) {
            let args = self
                .prog
                .trait_info(t)
                .map(|i| (1..=i.generics.len() as u32).map(Ty::Param).collect())
                .unwrap_or_default();
            candidates.push(TraitRef { def: t, args });
        }
        for p in ctx.predicates {
            if p.ty == *subject {
                candidates.push(p.bound.clone());
            }
        }
        for c in candidates {
            for tr in self.prog.elaborate(subject, &c) {
                let declares =
                    self.prog.trait_info(tr.def).is_some_and(|i| i.assoc.iter().any(|a| a.name == name));
                if declares {
                    return Some(Ty::Assoc(Box::new(AssocTy {
                        self_ty: subject.clone(),
                        trait_: tr,
                        name: name.to_string(),
                    })));
                }
            }
        }
        None
    }

    fn resolve_path_ty(&mut self, segments: &[ast::Name], args: Vec<Ty>, ctx: &TyCtx, span: Span) -> Ty {
        let Some(first) = segments.first() else { return Ty::Error };
        if segments.len() == 1 {
            let name = first.text.as_str();
            if let Some(i) = ctx.generics.iter().position(|g| g == name) {
                if !args.is_empty() {
                    self.report(Diagnostic::new(msg::TYPE_NOT_GENERIC, span).with("name", name));
                }
                return Ty::Param(i as u32);
            }
            let prim = match name {
                "int" => Some(Ty::Int),
                "f64" => Some(Ty::Float),
                "bool" => Some(Ty::Bool),
                "char" => Some(Ty::Char),
                "str" => Some(Ty::Str),
                "Never" => Some(Ty::Never),
                _ => None,
            };
            if let Some(p) = prim {
                if !args.is_empty() {
                    self.report(Diagnostic::new(msg::TYPE_NOT_GENERIC, span).with("name", name));
                }
                return p;
            }
            if name == "Self" {
                return match ctx.self_ty {
                    Some(t) => t.clone(),
                    None => {
                        self.report(Diagnostic::new(msg::RESOLVE_SELF_TYPE_OUTSIDE, span));
                        Ty::Error
                    }
                };
            }
            // Inside a trait, its associated types are named bare.
            if ctx.in_trait.is_some()
                && self.prog.lookup(ctx.module, name).is_none()
                && let Some(t) = self.assoc_of(&Ty::Param(0), name, ctx)
            {
                return t;
            }
            if let Some(d) = self.prog.lookup(ctx.module, name) {
                return self.def_as_type(d, name, args, span);
            }
            if let Some((_, phase)) = LATER_TYPES.iter().find(|(n, _)| *n == name) {
                self.unsupported(span, &format!("`{name}`"), phase);
                return Ty::Error;
            }
            if !self.prog.poisoned.contains(name) {
                self.report(Diagnostic::new(msg::RESOLVE_UNDEFINED_TYPE, span).with("name", name));
            }
            return Ty::Error;
        }
        // `C.Item` and `Self.Item`: an associated type.
        if segments.len() == 2 {
            let base = first.text.as_str();
            let subject = if let Some(i) = ctx.generics.iter().position(|g| g == base) {
                Some(Ty::Param(i as u32))
            } else if base == "Self" {
                ctx.self_ty.cloned()
            } else {
                None
            };
            if let Some(subject) = subject {
                let name = segments[1].text.as_str();
                if base == "Self"
                    && let Some((_, t)) = ctx.impl_assoc.iter().find(|(n, _)| n == name)
                {
                    return t.clone();
                }
                if let Some(t) = self.assoc_of(&subject, name, ctx) {
                    return t;
                }
                let shown = self.prog.display_in(&subject, ctx.generics);
                self.report(
                    Diagnostic::new(msg::RESOLVE_NO_ASSOC_TYPE, span).with("ty", shown).with("name", name),
                );
                return Ty::Error;
            }
        }
        // A qualified name: module.Type.
        match self.lookup_path(segments, ctx, span) {
            Some(d) => {
                let last = segments.last().map(|s| s.text.clone()).unwrap_or_default();
                self.def_as_type(d, &last, args, span)
            }
            None => Ty::Error,
        }
    }

    pub fn def_as_type(&mut self, d: DefId, name: &str, args: Vec<Ty>, span: Span) -> Ty {
        let generics: Vec<Generic> = match &self.prog.def(d).kind {
            DefKind::Struct(s) => s.generics.clone(),
            DefKind::Enum(e) => e.generics.clone(),
            DefKind::Refinement(r) => r.generics.clone(),
            DefKind::ErrorSet(_) => {
                if !args.is_empty() {
                    self.report(Diagnostic::new(msg::TYPE_NOT_GENERIC, span).with("name", name));
                }
                return Ty::ErrSet(vec![d]);
            }
            DefKind::TypeAlias(_) => {
                // A generic alias stands for its type with the arguments in it.
                let declared = self.aliases.get(&d).map(|(_, a)| a.generics.len()).unwrap_or(0);
                if args.len() != declared && !(args.is_empty() && declared == 0) {
                    self.report(
                        Diagnostic::new(msg::RESOLVE_TYPE_ARGUMENT_COUNT, span)
                            .with("name", name)
                            .with("expected", declared.to_string())
                            .with("found", args.len().to_string()),
                    );
                    return Ty::Error;
                }
                return self.resolve_alias_public(d).subst(&args);
            }
            DefKind::Trait(_) => {
                let at = Span::new(span.file, sisu_diag::TextRange::empty(span.range.start));
                self.report(Diagnostic::new(msg::RESOLVE_TRAIT_AS_TYPE, span).with("name", name).fix(
                    msg::RESOLVE_ADD_DYN,
                    vec![("name", name.to_string())],
                    vec![sisu_diag::Edit { span: at, text: "dyn ".into() }],
                ));
                return Ty::Error;
            }
            DefKind::Unsupported => return Ty::Error,
            _ => {
                self.report(Diagnostic::new(msg::RESOLVE_NOT_A_TYPE, span).with("name", name));
                return Ty::Error;
            }
        };
        // Generics of a type not yet resolved are counted from its AST.
        let declared = self.declared_generics(d).unwrap_or(generics.len());
        let mut args = args;
        // Omitted trailing arguments take their defaults (D-060).
        if args.len() < declared {
            let mut filled = args.clone();
            while filled.len() < generics.len() {
                match &generics[filled.len()].default {
                    Some(def) => {
                        let t = def.subst(&filled);
                        filled.push(t);
                    }
                    None => break,
                }
            }
            if filled.len() == declared {
                args = filled;
            }
        }
        if args.len() != declared {
            if args.is_empty() {
                self.report(Diagnostic::new(msg::TYPE_TYPE_ARGUMENTS_NEEDED, span).with("name", name));
            } else {
                self.report(
                    Diagnostic::new(msg::RESOLVE_TYPE_ARGUMENT_COUNT, span)
                        .with("name", name)
                        .with("expected", declared.to_string())
                        .with("found", args.len().to_string()),
                );
            }
            return Ty::Adt(d, (0..declared).map(|_| Ty::Error).collect());
        }
        Ty::Adt(d, args)
    }

    fn resolve_alias_public(&mut self, d: DefId) -> Ty {
        self.resolve_alias(d)
    }

    /// The number of generic parameters a type declares, from its AST.
    pub fn declared_generics(&self, d: DefId) -> Option<usize> {
        if let Some(g) = self.type_params.get(&d) {
            return Some(g.len());
        }
        for p in &self.pending {
            match p {
                Pending::Struct { def, ast, .. } if *def == d => return Some(ast.generics.len()),
                Pending::Enum { def, ast, .. } if *def == d => return Some(ast.generics.len()),
                _ => {}
            }
        }
        match &self.prog.def(d).kind {
            DefKind::Struct(s) => Some(s.generics.len()),
            DefKind::Enum(e) => Some(e.generics.len()),
            _ => None,
        }
    }
}
