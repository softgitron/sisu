//! Calls: functions, methods, associated functions, constructors and
//! variants, with keyword arguments, defaults, `mut` at the call site and
//! trailing `do` blocks.

use crate::check::Checker;
use crate::expr::{ClosureSource, mentions_it};
use crate::thir::*;
use sisu_diag::{Edit, Span, TextRange, msg};
use sisu_resolve::{DefId, DefKind, ModuleId, Ty, TyKey};
use sisu_syntax::ast::{self, Mode};

/// A name that denotes a module, a type or a function rather than a value.
#[derive(Clone, Debug)]
pub enum StaticPath {
    Module(ModuleId),
    Type(Ty),
    Fn(DefId, Option<Vec<Ty>>),
    /// A trait, whose methods are called as `Trait.method(x, ...)`.
    Trait(DefId),
}

#[derive(Clone, Debug)]
pub struct InstParam {
    pub name: String,
    pub mode: Mode,
    pub ty: Ty,
    pub has_default: bool,
    /// The declared type is a generic parameter (or an associated type of
    /// one) of a function with a body: a borrowed argument is passed by
    /// reference even when its type is `Copy`.
    pub opaque: bool,
}

#[derive(Clone, Debug)]
pub struct InstSig {
    pub def: DefId,
    pub name: String,
    pub params: Vec<InstParam>,
    pub ret: Ty,
    pub owner: Option<Ty>,
    pub type_args: Vec<Ty>,
    pub keyword_only_from: Option<usize>,
    pub positional_only_until: usize,
}

impl Checker<'_> {
    // ---------------------------------------------------------------- static paths

    pub fn static_path(&mut self, e: &ast::Expr) -> Option<StaticPath> {
        let prog = self.prog;
        match &e.kind {
            ast::ExprKind::Name(n) => {
                if self.lookup_local(n).is_some() || n == "it" {
                    return None;
                }
                // A generic parameter: `T.zero()`.
                if let Some(i) = self.generic_names.iter().position(|g| g == n) {
                    return Some(StaticPath::Type(Ty::Param(i as u32)));
                }
                let prim = match n.as_str() {
                    "int" => Some(Ty::Int),
                    "f64" => Some(Ty::Float),
                    "bool" => Some(Ty::Bool),
                    "char" => Some(Ty::Char),
                    "str" => Some(Ty::Str),
                    _ => None,
                };
                if let Some(t) = prim
                    && prog.lookup(self.module, n).is_none()
                {
                    return Some(StaticPath::Type(t));
                }
                let d = prog.lookup(self.module, n)?;
                self.classify(d, e.range)
            }
            ast::ExprKind::SelfType => self.self_ty.clone().map(StaticPath::Type),
            ast::ExprKind::Field { base, name } => match self.static_path(base)? {
                StaticPath::Module(m) => {
                    let Some(d) = prog.lookup_in(m, &name.text) else {
                        let d = self
                            .error(msg::RESOLVE_NOT_IN_MODULE, name.range)
                            .with("module", prog.modules[m.0 as usize].name.clone())
                            .with("name", name.text.clone());
                        self.report(d);
                        return Some(StaticPath::Type(Ty::Error));
                    };
                    self.classify(d, e.range)
                }
                _ => None,
            },
            ast::ExprKind::Index { base, args } => {
                let path = self.static_path(base)?;
                let tys: Vec<Ty> = args
                    .iter()
                    .map(|a| match a {
                        ast::IndexArg::Expr(x) => self.expr_as_type(x),
                        ast::IndexArg::Slice { .. } => Ty::Error,
                    })
                    .collect();
                match path {
                    StaticPath::Type(Ty::Adt(d, _)) => Some(StaticPath::Type(self.def_type(d, tys, e.range))),
                    StaticPath::Fn(d, None) => Some(StaticPath::Fn(d, Some(tys))),
                    other => Some(other),
                }
            }
            _ => None,
        }
    }

    fn classify(&mut self, d: DefId, range: TextRange) -> Option<StaticPath> {
        let prog = self.prog;
        match &prog.def(d).kind {
            DefKind::Module(m) => Some(StaticPath::Module(*m)),
            DefKind::Struct(_) | DefKind::Enum(_) | DefKind::ErrorSet(_) | DefKind::TypeAlias(_) => {
                Some(StaticPath::Type(self.def_type(d, Vec::new(), range)))
            }
            DefKind::Fn(f) if !f.generics.is_empty() => Some(StaticPath::Fn(d, None)),
            DefKind::Trait(_) => Some(StaticPath::Trait(d)),
            _ => None,
        }
    }

    /// An index argument read as a type: `Options` in `cli.parse[Options]`.
    pub fn expr_as_type(&mut self, e: &ast::Expr) -> Ty {
        match &e.kind {
            ast::ExprKind::Name(n) => match n.as_str() {
                "int" => Ty::Int,
                "f64" => Ty::Float,
                "bool" => Ty::Bool,
                "char" => Ty::Char,
                "str" => Ty::Str,
                _ => match self.static_path(e) {
                    Some(StaticPath::Type(t)) => t,
                    _ => {
                        let d = self.error(msg::RESOLVE_UNDEFINED_TYPE, e.range).with("name", n.clone());
                        self.report(d);
                        Ty::Error
                    }
                },
            },
            ast::ExprKind::Field { .. } | ast::ExprKind::Index { .. } => match self.static_path(e) {
                Some(StaticPath::Type(t)) => t,
                _ => {
                    let d =
                        self.error(msg::RESOLVE_NOT_A_TYPE, e.range).with("name", self.source_text(e.range));
                    self.report(d);
                    Ty::Error
                }
            },
            ast::ExprKind::Tuple(xs) => {
                let tys = xs.iter().map(|x| self.expr_as_type(x)).collect::<Vec<_>>();
                if tys.is_empty() { Ty::Unit } else { Ty::tuple(tys) }
            }
            _ => {
                let d = self
                    .error(msg::PARSE_EXPECTED_TYPE, e.range)
                    .with("found", format!("`{}`", self.source_text(e.range)));
                self.report(d);
                Ty::Error
            }
        }
    }

    /// `module.name`, `Type.Variant`, `Type.function`, used as a value or,
    /// with `call`, called.
    pub fn static_member(
        &mut self,
        path: StaticPath,
        name: &ast::Name,
        range: TextRange,
        call: Option<(&[ast::Arg], Option<&ast::Closure>, Option<&Ty>)>,
    ) -> Expr {
        let span = self.span(range);
        let prog = self.prog;
        match path {
            StaticPath::Module(m) => {
                let Some(d) = prog.lookup_in(m, &name.text) else {
                    let diag = self
                        .error(msg::RESOLVE_NOT_IN_MODULE, name.range)
                        .with("module", prog.modules[m.0 as usize].name.clone())
                        .with("name", name.text.clone());
                    self.report(diag);
                    if let Some((args, _, _)) = call {
                        self.check_args_loosely(args);
                    }
                    return Expr::error(span);
                };
                match call {
                    Some((args, trailing, expected)) => {
                        self.call_def(d, None, args, trailing, range, expected)
                    }
                    None => self.def_value(d, &name.text, range),
                }
            }
            StaticPath::Type(ty) => {
                if ty == Ty::Error {
                    if let Some((args, _, _)) = call {
                        self.check_args_loosely(args);
                    }
                    return Expr::error(span);
                }
                // A variant of the type.
                if let Some((def, vi)) = self.variant_in(&ty, &name.text) {
                    return match call {
                        Some((args, trailing, _)) => {
                            self.variant_call(def, vi, args, trailing, range, Some(&ty))
                        }
                        None => self.unit_variant_value(def, vi, &ty, range),
                    };
                }
                // An associated function.
                let method = ty.key().and_then(|k| prog.method(k, &name.text));
                match (method, call) {
                    (Some(d), Some((args, trailing, expected))) => {
                        let info = prog.fn_info(d);
                        if info.is_some_and(|f| f.self_mode.is_some()) {
                            let diag = self
                                .error(msg::TYPE_METHOD_NEEDS_RECEIVER, name.range)
                                .with("name", name.text.clone());
                            self.report(diag);
                            self.check_args_loosely(args);
                            return Expr::error(span);
                        }
                        self.call_fn(d, None, Some(ty), None, args, trailing, range, expected)
                    }
                    (Some(d), None) => self.def_value(d, &name.text, range),
                    (None, Some((args, trailing, expected)))
                        if self.trait_static(&ty, &name.text).is_some() =>
                    {
                        let Some((tr, m)) = self.trait_static(&ty, &name.text) else {
                            return Expr::error(span);
                        };
                        if prog.fn_info(m).is_some_and(|f| f.self_mode.is_some()) {
                            let diag = self
                                .error(msg::TYPE_METHOD_NEEDS_RECEIVER, name.range)
                                .with("name", name.text.clone());
                            self.report(diag);
                            self.check_args_loosely(args);
                            return Expr::error(span);
                        }
                        let norm = prog.normalize(&self.infer.zonk(&ty), &self.env);
                        let mut outer = vec![norm.clone()];
                        outer.extend(tr.args.iter().cloned());
                        self.call_fn_with(
                            m,
                            None,
                            Some(outer),
                            Some(norm),
                            None,
                            args,
                            trailing,
                            range,
                            expected,
                        )
                    }
                    (None, call) => {
                        let diag = self
                            .error(msg::TYPE_UNKNOWN_ASSOCIATED, name.range)
                            .with("ty", self.show(&ty))
                            .with("name", name.text.clone());
                        self.report(diag);
                        if let Some((args, _, _)) = call {
                            self.check_args_loosely(args);
                        }
                        Expr::error(span)
                    }
                }
            }
            StaticPath::Trait(t) => self.trait_path_call(t, name, range, call),
            StaticPath::Fn(d, _) => {
                let diag = self
                    .error(msg::TYPE_UNKNOWN_FIELD, name.range)
                    .with("ty", prog.def(d).name.clone())
                    .with("name", name.text.clone());
                self.report(diag);
                Expr::error(span)
            }
        }
    }

    /// The one trait method called `name` that `ty` has through the traits
    /// it implements.
    fn trait_static(&self, ty: &Ty, name: &str) -> Option<(sisu_resolve::TraitRef, DefId)> {
        let prog = self.prog;
        let norm = prog.normalize(&self.infer.zonk(ty), &self.env);
        let found = prog.trait_methods_for(&norm, name, &self.env);
        if found.len() == 1 { found.into_iter().next() } else { None }
    }

    /// `Trait.method(x, ...)`: the trait's method called on `x`, or, for a
    /// method without `self`, with `Self` inferred from the arguments and
    /// the expected type (`Default.default()`).
    fn trait_path_call(
        &mut self,
        t: DefId,
        name: &ast::Name,
        range: TextRange,
        call: Option<(&[ast::Arg], Option<&ast::Closure>, Option<&Ty>)>,
    ) -> Expr {
        let span = self.span(range);
        let prog = self.prog;
        let Some(m) = prog.trait_method(t, &name.text) else {
            let diag = self
                .error(msg::TYPE_UNKNOWN_ASSOCIATED, name.range)
                .with("ty", prog.def(t).name.clone())
                .with("name", name.text.clone());
            self.report(diag);
            if let Some((args, _, _)) = call {
                self.check_args_loosely(args);
            }
            return Expr::error(span);
        };
        let Some((args, trailing, expected)) = call else {
            let diag = self
                .error(msg::TYPE_NOT_A_VALUE, range)
                .with("name", self.source_text(range))
                .with("kind", "trait method");
            self.report(diag);
            return Expr::error(span);
        };
        // The trait's own parameters: inferred, or their defaults.
        let generics = prog.trait_info(t).map(|i| i.generics.clone()).unwrap_or_default();
        let targs: Vec<Ty> = generics.iter().map(|_| self.infer.fresh()).collect();
        let tr = sisu_resolve::TraitRef { def: t, args: targs };
        let has_self = prog.fn_info(m).is_some_and(|f| f.self_mode.is_some());
        if has_self {
            let Some((first, rest)) = args.split_first() else {
                let d = self
                    .error(msg::TYPE_ARGUMENT_COUNT, range)
                    .with("name", name.text.clone())
                    .with("expected", "1 or more")
                    .with("found", "0");
                self.report(d);
                return Expr::error(span);
            };
            let recv = self.infer_expr(&first.value, None);
            let recv = self.auto_deref(recv);
            let self_ty = prog.normalize(&self.infer.zonk(&recv.ty), &self.env);
            self.bounds.push((self_ty.clone(), tr.clone(), recv.span));
            return self.trait_call(m, tr, recv, self_ty, name, rest, trailing, range, expected);
        }
        let self_ty = self.infer.fresh();
        self.bounds.push((self_ty.clone(), tr.clone(), span));
        let mut outer = vec![self_ty];
        outer.extend(tr.args.iter().cloned());
        self.call_fn_with(m, None, Some(outer), None, None, args, trailing, range, expected)
    }

    fn unit_variant_value(&mut self, def: DefId, vi: u32, ty: &Ty, range: TextRange) -> Expr {
        let span = self.span(range);
        let prog = self.prog;
        let v = prog.enum_info(def).and_then(|e| e.variants.get(vi as usize));
        if v.is_some_and(|v| !v.fields.is_empty()) {
            let d = self
                .error(msg::TYPE_VARIANT_NEEDS_FIELDS, range)
                .with("name", v.map(|v| v.name.clone()).unwrap_or_default());
            self.report(d);
            return Expr::error(span);
        }
        let (owner, _) = self.variant_owner_ty(def, Some(ty));
        Expr::new(ExprKind::Variant { def, variant: vi, args: Vec::new() }, owner, span)
    }

    /// Checks arguments for their own errors when the callee is unknown.
    pub fn check_args_loosely(&mut self, args: &[ast::Arg]) {
        for a in args {
            let _ = self.infer_expr(&a.value, None);
        }
    }

    // ---------------------------------------------------------------- calls

    pub fn call_expr(
        &mut self,
        callee: &ast::Expr,
        args: &[ast::Arg],
        trailing: Option<&ast::Closure>,
        range: TextRange,
        expected: Option<&Ty>,
    ) -> Expr {
        let span = self.span(range);
        let prog = self.prog;
        match &callee.kind {
            ast::ExprKind::Field { base, name } => {
                if let Some(path) = self.static_path(base) {
                    return self.static_member(path, name, range, Some((args, trailing, expected)));
                }
                let recv = self.infer_expr(base, None);
                return self.method_call_on(recv, name, args, trailing, range, expected);
            }
            ast::ExprKind::OptField { base, name } => {
                return self.opt_chain(base, name, Some((args, trailing)), range);
            }
            ast::ExprKind::Index { .. } => match self.static_path(callee) {
                Some(StaticPath::Fn(d, tys)) => {
                    return self.call_fn(d, tys, None, None, args, trailing, range, expected);
                }
                Some(StaticPath::Type(ty)) => return self.construct(ty, args, trailing, range, expected),
                _ => {}
            },
            ast::ExprKind::SelfType => {
                if let Some(t) = self.self_ty.clone() {
                    return self.construct(t, args, trailing, range, expected);
                }
            }
            ast::ExprKind::Name(n) if self.lookup_local(n).is_none() && n != "it" => {
                if let Some(exp) = expected
                    && let Some((def, vi)) = self.variant_in(exp, n)
                {
                    return self.variant_call(def, vi, args, trailing, range, Some(exp));
                }
                if let Some(d) = prog.lookup(self.module, n) {
                    return self.call_def(d, None, args, trailing, range, expected);
                }
                if let Some((def, vi)) = self.global_variant(n, callee.range) {
                    return self.variant_call(def, vi, args, trailing, range, expected);
                }
                if !self.quiet_unknown_names && !prog.poisoned.contains(n.as_str()) {
                    let mut d = self.error(msg::RESOLVE_UNDEFINED_NAME, callee.range).with("name", n.clone());
                    if let Some(similar) = self.similar_name(n) {
                        let edit = Edit { span: self.span(callee.range), text: similar.clone() };
                        d = d.fix(msg::RESOLVE_SIMILAR_NAME, vec![("name", similar)], vec![edit]);
                    }
                    self.report(d);
                }
                self.check_args_loosely(args);
                return Expr::error(span);
            }
            _ => {}
        }
        let f = self.infer_expr(callee, None);
        self.call_value(f, args, trailing, range)
    }

    /// Calls a declaration: a function, a struct's constructor, a constant
    /// holding a function.
    pub fn call_def(
        &mut self,
        d: DefId,
        tys: Option<Vec<Ty>>,
        args: &[ast::Arg],
        trailing: Option<&ast::Closure>,
        range: TextRange,
        expected: Option<&Ty>,
    ) -> Expr {
        let span = self.span(range);
        let prog = self.prog;
        match &prog.def(d).kind {
            DefKind::Fn(f) => {
                if f.self_mode.is_some() {
                    let diag = self
                        .error(msg::TYPE_METHOD_NEEDS_RECEIVER, range)
                        .with("name", prog.def(d).name.clone());
                    self.report(diag);
                    self.check_args_loosely(args);
                    return Expr::error(span);
                }
                self.call_fn(d, tys, None, None, args, trailing, range, expected)
            }
            DefKind::Struct(_) | DefKind::Enum(_) | DefKind::ErrorSet(_) | DefKind::TypeAlias(_) => {
                let ty = self.def_type(d, tys.unwrap_or_default(), range);
                self.construct(ty, args, trailing, range, expected)
            }
            DefKind::Const(_) => {
                let f = self.def_value(d, &prog.def(d).name.clone(), range);
                self.call_value(f, args, trailing, range)
            }
            _ => {
                let diag = self.error(msg::TYPE_NOT_CALLABLE, range).with("ty", prog.def(d).name.clone());
                self.report(diag);
                self.check_args_loosely(args);
                Expr::error(span)
            }
        }
    }

    /// `T(...)` for a type: a struct's fields, or `new` for a builtin type.
    pub fn construct(
        &mut self,
        ty: Ty,
        args: &[ast::Arg],
        trailing: Option<&ast::Closure>,
        range: TextRange,
        expected: Option<&Ty>,
    ) -> Expr {
        let span = self.span(range);
        let prog = self.prog;
        let Ty::Adt(def, targs) = self.resolved(&ty) else {
            if ty != Ty::Error {
                let d = self.error(msg::TYPE_NOT_CALLABLE, range).with("ty", self.show(&ty));
                self.report(d);
            }
            self.check_args_loosely(args);
            return Expr::error(span);
        };
        let Some(s) = prog.struct_info(def) else {
            let d = self.error(msg::TYPE_NOT_CALLABLE, range).with("ty", self.show(&ty));
            self.report(d);
            self.check_args_loosely(args);
            return Expr::error(span);
        };
        if s.builtin.is_some() {
            if let Some(new) = prog.method(TyKey::Adt(def), "new") {
                return self.call_fn(
                    new,
                    None,
                    Some(Ty::Adt(def, targs)),
                    None,
                    args,
                    trailing,
                    range,
                    expected,
                );
            }
            let d = self.error(msg::TYPE_NO_CONSTRUCTOR, range).with("ty", self.show(&ty));
            self.report(d);
            self.check_args_loosely(args);
            return Expr::error(span);
        }
        // Fields by position, then by name.
        let n = s.fields.len();
        let mut slots: Vec<Option<&ast::Arg>> = vec![None; n];
        let mut pos = 0;
        let mut seen_kw = false;
        let tyname = self.show(&ty);
        for a in args {
            match &a.name {
                Some(k) => {
                    seen_kw = true;
                    match s.fields.iter().position(|f| f.name == k.text) {
                        Some(i) if slots[i].is_none() => slots[i] = Some(a),
                        Some(_) => {
                            let d =
                                self.error(msg::TYPE_DUPLICATE_ARGUMENT, k.range).with("arg", k.text.clone());
                            self.report(d);
                        }
                        None => {
                            let d = self
                                .error(msg::TYPE_UNKNOWN_FIELD, k.range)
                                .with("ty", tyname.clone())
                                .with("name", k.text.clone());
                            self.report(d);
                        }
                    }
                }
                None => {
                    if seen_kw {
                        let d = self.error(msg::TYPE_POSITIONAL_AFTER_KEYWORD, a.range);
                        self.report(d);
                    }
                    if pos < n {
                        slots[pos] = Some(a);
                    } else {
                        let d = self
                            .error(msg::TYPE_ARGUMENT_COUNT, range)
                            .with("name", tyname.clone())
                            .with("expected", n.to_string())
                            .with("found", args.len().to_string());
                        self.report(d);
                    }
                    pos += 1;
                }
            }
        }
        if trailing.is_some() {
            let d = self
                .error(msg::TYPE_ARGUMENT_COUNT, range)
                .with("name", tyname.clone())
                .with("expected", n.to_string())
                .with("found", (args.len() + 1).to_string());
            self.report(d);
        }
        let mut fields = Vec::new();
        for (i, f) in s.fields.iter().enumerate() {
            let fty = f.ty.subst(&targs);
            match slots[i] {
                Some(a) => {
                    let x = self.check_expr(&a.value, &fty);
                    fields.push(FieldInit::Value(self.check_movable(x)));
                }
                None if f.default.is_some() => fields.push(FieldInit::Default(i as u32)),
                None => {
                    let d = self
                        .error(msg::TYPE_MISSING_FIELD, range)
                        .with("field", f.name.clone())
                        .with("ty", tyname.clone());
                    self.report(d);
                    fields.push(FieldInit::Value(Expr::error(span)));
                }
            }
        }
        Expr::new(ExprKind::Struct { def, fields }, Ty::Adt(def, targs), span)
    }

    /// `Variant(args)`.
    pub fn variant_call(
        &mut self,
        def: DefId,
        vi: u32,
        args: &[ast::Arg],
        trailing: Option<&ast::Closure>,
        range: TextRange,
        expected: Option<&Ty>,
    ) -> Expr {
        let span = self.span(range);
        let prog = self.prog;
        let (ty, targs) = self.variant_owner_ty(def, expected);
        let Some(v) = prog.enum_info(def).and_then(|e| e.variants.get(vi as usize)).cloned() else {
            return Expr::error(span);
        };
        let n = v.fields.len();
        let mut slots: Vec<Option<&ast::Arg>> = vec![None; n];
        let mut pos = 0;
        for a in args {
            match &a.name {
                Some(k) => {
                    match v.fields.iter().position(|(fname, _)| fname.as_deref() == Some(k.text.as_str())) {
                        Some(i) if slots[i].is_none() => slots[i] = Some(a),
                        _ => {
                            let d = self
                                .error(msg::TYPE_UNKNOWN_KEYWORD, k.range)
                                .with("name", v.name.clone())
                                .with("arg", k.text.clone());
                            self.report(d);
                        }
                    }
                }
                None => {
                    if pos < n {
                        slots[pos] = Some(a);
                    }
                    pos += 1;
                }
            }
        }
        if pos > n || trailing.is_some() {
            let d = self
                .error(msg::TYPE_ARGUMENT_COUNT, range)
                .with("name", v.name.clone())
                .with("expected", n.to_string())
                .with("found", (args.len() + trailing.is_some() as usize).to_string());
            self.report(d);
        }
        let mut out = Vec::new();
        for (i, (fname, fty)) in v.fields.iter().enumerate() {
            let fty = fty.subst(&targs);
            match slots[i] {
                Some(a) => {
                    let x = self.check_expr(&a.value, &fty);
                    out.push(self.check_movable(x));
                }
                None => {
                    let shown = fname.clone().unwrap_or_else(|| i.to_string());
                    let d = self
                        .error(msg::TYPE_MISSING_ARGUMENT, range)
                        .with("arg", shown)
                        .with("name", v.name.clone());
                    self.report(d);
                    out.push(Expr::error(span));
                }
            }
        }
        Expr::new(ExprKind::Variant { def, variant: vi, args: out }, ty, span)
    }

    /// Instantiates a function's signature with fresh type variables, or
    /// the explicit type arguments for its own generic parameters; its
    /// bounds are required of the arguments.
    pub fn instantiate(&mut self, d: DefId, explicit: Option<Vec<Ty>>, range: TextRange) -> InstSig {
        let prog = self.prog;
        let span = self.span(range);
        let Some(f) = prog.fn_info(d) else {
            return InstSig {
                def: d,
                name: String::new(),
                params: Vec::new(),
                ret: Ty::Error,
                owner: None,
                type_args: Vec::new(),
                keyword_only_from: None,
                positional_only_until: 0,
            };
        };
        let own = f.generics.len() - f.impl_generics;
        if let Some(ex) = &explicit
            && ex.len() != own
        {
            let diag = if own == 0 {
                self.error(msg::TYPE_NOT_GENERIC, range).with("name", prog.def(d).name.clone())
            } else {
                self.error(msg::RESOLVE_TYPE_ARGUMENT_COUNT, range)
                    .with("name", prog.def(d).name.clone())
                    .with("expected", own.to_string())
                    .with("found", ex.len().to_string())
            };
            self.report(diag);
        }
        let mut vars = Vec::new();
        for i in 0..f.generics.len() {
            let v = if i >= f.impl_generics {
                explicit
                    .as_ref()
                    .and_then(|ex| ex.get(i - f.impl_generics).cloned())
                    .unwrap_or_else(|| self.infer.fresh())
            } else {
                self.infer.fresh()
            };
            vars.push(v);
        }
        for p in &f.predicates {
            self.bounds.push((p.ty.subst(&vars), p.bound.subst(&vars), span));
        }
        let has_body = f.intrinsic.is_none();
        let fenv = prog.elaborate_env(&f.predicates);
        let params = f
            .params
            .iter()
            .map(|p| InstParam {
                name: p.name.clone(),
                mode: p.mode,
                ty: p.ty.subst(&vars),
                has_default: p.default.is_some(),
                // Passed by reference, unless a `Copy` bound makes it a value.
                opaque: has_body && crate::is_opaque(&p.ty) && !prog.is_copy_in(&p.ty, &fenv),
            })
            .collect();
        InstSig {
            def: d,
            name: prog.def(d).name.clone(),
            params,
            ret: self.fn_ret(d).subst(&vars),
            owner: f.owner.as_ref().map(|o| o.subst(&vars)),
            type_args: vars,
            keyword_only_from: f.keyword_only_from,
            positional_only_until: f.positional_only_until,
        }
    }

    /// Calls a function given its definition. `owner` is the type a
    /// static call names (`list[int].new()`); `recv` the receiver argument
    /// of a method call.
    #[allow(clippy::too_many_arguments)]
    pub fn call_fn(
        &mut self,
        d: DefId,
        tys: Option<Vec<Ty>>,
        owner: Option<Ty>,
        recv: Option<Arg>,
        args: &[ast::Arg],
        trailing: Option<&ast::Closure>,
        range: TextRange,
        expected: Option<&Ty>,
    ) -> Expr {
        self.call_fn_with(d, tys, None, owner, recv, args, trailing, range, expected)
    }

    /// [`Checker::call_fn`] where the outer generic parameters are known:
    /// for a trait's method, `Self` and the trait's arguments.
    #[allow(clippy::too_many_arguments)]
    pub fn call_fn_with(
        &mut self,
        d: DefId,
        tys: Option<Vec<Ty>>,
        outer: Option<Vec<Ty>>,
        owner: Option<Ty>,
        recv: Option<Arg>,
        args: &[ast::Arg],
        trailing: Option<&ast::Closure>,
        range: TextRange,
        expected: Option<&Ty>,
    ) -> Expr {
        let span = self.span(range);
        let prog = self.prog;
        if prog.fn_info(d).is_some_and(|f| f.comptime) && !self.in_comptime {
            let diag = self.error(msg::COMPTIME_ONLY, range).with("name", prog.def(d).name.clone());
            self.report(diag);
        }
        if prog.fn_info(d).is_some_and(|f| f.header_broken) {
            // Its signature is unknown (the header did not parse): check the
            // arguments on their own and give the call the error type.
            self.check_args_loosely(args);
            return Expr::error(span);
        }
        let mut sig = self.instantiate(d, tys, range);
        if let Some(outer) = &outer {
            for (v, t) in sig.type_args.iter().zip(outer) {
                self.infer.try_unify(v, t);
            }
        }
        if let (Some(o), Some(so)) = (&owner, &sig.owner) {
            self.infer.try_unify(so, o);
        }
        // Associated types of what is known now.
        let env = self.env.clone();
        for p in sig.params.iter_mut() {
            p.ty = prog.normalize(&self.infer.zonk(&p.ty), &env);
        }
        sig.ret = prog.normalize(&self.infer.zonk(&sig.ret), &env);
        if let Some(exp) = expected {
            // Let the expected type drive inference (`list()`, `todo()`).
            if self.infer.has_vars(&sig.ret) {
                let snap = self.infer.snapshot();
                if !self.infer.unify(&sig.ret, exp) {
                    self.infer.rollback(snap);
                }
            }
        }
        // `range(n)` is `range(0, n)` (02, "Prelude functions").
        let args_owned: Vec<ast::Arg>;
        let args = if Some(d) == prog.wk.range && args.len() == 1 && args[0].name.is_none() {
            let zero = ast::Expr {
                node: args[0].value.node,
                range: TextRange::empty(args[0].range.start),
                kind: ast::ExprKind::Int { text: "0".into(), value: Some(0), suffix: None },
            };
            args_owned = vec![
                ast::Arg { node: args[0].node, range: zero.range, name: None, mutable: false, value: zero },
                args[0].clone(),
            ];
            &args_owned[..]
        } else {
            args
        };
        let mut out = Vec::new();
        if let Some(r) = recv {
            out.push(r);
        }
        let task_call = crate::tasks::task_call(prog.fn_info(d).and_then(|f| f.intrinsic.as_deref()));
        let spawns_before = self.spawn_sites.len();
        out.extend(self.check_args(&sig, args, trailing, range));
        let mut ret = prog.normalize(&self.infer.zonk(&sig.ret), &self.env);
        if let Some(tc) = task_call {
            let name = self.source_text(range);
            let name =
                name.split('(').next().unwrap_or("").split_whitespace().next().unwrap_or("").to_string();
            ret = self.task_rules(tc, &out, ret, spawns_before, span, &name);
        }
        // What a `Shared`, `Cell` or `Mutex` owns can be neither a view nor
        // must-consume (checked when the type is known).
        let owner = match prog.fn_info(d).and_then(|f| f.intrinsic.as_deref()) {
            Some("Shared.new") => Some("Shared"),
            Some("Cell.new") => Some("Cell"),
            Some("Mutex.new") => Some("Mutex"),
            _ => None,
        };
        if let (Some(owner), Some(t)) = (owner, sig.type_args.first()) {
            self.owner_checks.push((t.clone(), owner, span));
        }
        // A typed hole is reported with the type it stands for (D-061).
        if prog.fn_info(d).and_then(|f| f.intrinsic.as_deref()) == Some("todo") {
            let name = match out.first().map(|a| &a.expr.kind) {
                Some(ExprKind::Str(s)) => s.clone(),
                _ => String::new(),
            };
            let ty = self.show(&self.resolved(&sig.ret));
            let d = self.error(msg::TYPE_HOLE, range).with("name", name).with("ty", ty);
            self.report(d);
        }
        Expr::new(
            ExprKind::Call { callee: Callee::Fn { def: d, type_args: sig.type_args.clone() }, args: out },
            ret,
            span,
        )
    }

    /// Matches arguments to parameters and checks each.
    pub fn check_args(
        &mut self,
        sig: &InstSig,
        args: &[ast::Arg],
        trailing: Option<&ast::Closure>,
        range: TextRange,
    ) -> Vec<Arg> {
        let span = self.span(range);
        let n = sig.params.len();
        let mut slots: Vec<Option<ast::Arg>> = vec![None; n];
        let kw_from = sig.keyword_only_from.unwrap_or(n);
        let mut pos = 0;
        let mut seen_kw = false;
        let mut extra = false;
        for a in args {
            match &a.name {
                Some(k) => {
                    seen_kw = true;
                    match sig.params.iter().position(|p| p.name == k.text) {
                        Some(i) if i >= sig.positional_only_until => {
                            if slots[i].is_some() {
                                let d = self
                                    .error(msg::TYPE_DUPLICATE_ARGUMENT, k.range)
                                    .with("arg", k.text.clone());
                                self.report(d);
                            } else {
                                slots[i] = Some(a.clone());
                            }
                        }
                        _ => {
                            let d = self
                                .error(msg::TYPE_UNKNOWN_KEYWORD, k.range)
                                .with("name", sig.name.clone())
                                .with("arg", k.text.clone());
                            self.report(d);
                            let _ = self.infer_expr(&a.value, None);
                        }
                    }
                }
                None => {
                    if seen_kw {
                        let d = self.error(msg::TYPE_POSITIONAL_AFTER_KEYWORD, a.range);
                        self.report(d);
                    }
                    if pos < kw_from && pos < n {
                        slots[pos] = Some(a.clone());
                    } else {
                        extra = true;
                        let _ = self.infer_expr(&a.value, None);
                    }
                    pos += 1;
                }
            }
        }
        if let Some(c) = trailing {
            // A `do` block binds to the last function-typed parameter.
            let target = (0..n).rev().find(|&i| matches!(self.resolved(&sig.params[i].ty), Ty::Fn(_)));
            match target {
                Some(i) if slots[i].is_none() => {
                    let value = ast::Expr {
                        node: c.node,
                        range: c.range,
                        kind: ast::ExprKind::Closure(Box::new(c.clone())),
                    };
                    slots[i] =
                        Some(ast::Arg { node: c.node, range: c.range, name: None, mutable: false, value });
                }
                _ => extra = true,
            }
        }
        if extra {
            let found = args.len() + trailing.is_some() as usize;
            let d = self
                .error(msg::TYPE_ARGUMENT_COUNT, range)
                .with("name", sig.name.clone())
                .with("expected", n.to_string())
                .with("found", found.to_string());
            self.report(d);
        }
        let mut out = Vec::new();
        for (i, p) in sig.params.iter().enumerate() {
            match &slots[i] {
                Some(a) => out.push(self.check_arg(a, p)),
                None if p.has_default => out.push(Arg {
                    expr: Expr::new(
                        ExprKind::DefaultArg { def: sig.def, param: i as u32 },
                        p.ty.clone(),
                        span,
                    ),
                    mode: ArgMode::Value,
                }),
                None => {
                    if !extra {
                        let d = self
                            .error(msg::TYPE_MISSING_ARGUMENT, range)
                            .with("arg", p.name.clone())
                            .with("name", sig.name.clone());
                        self.report(d);
                    }
                    out.push(Arg { expr: Expr::error(span), mode: ArgMode::Value });
                }
            }
        }
        out
    }

    fn check_arg(&mut self, a: &ast::Arg, p: &InstParam) -> Arg {
        if p.mode == Mode::Mut && !a.mutable {
            let edit = Edit {
                span: Span::new(self.file, TextRange::empty(a.value.range.start)),
                text: "mut ".into(),
            };
            let d = self
                .error(msg::TYPE_MUT_ARGUMENT_MISSING, a.value.range)
                .with("param", p.name.clone())
                .fix(msg::TYPE_ADD_MUT, Vec::new(), vec![edit]);
            self.report(d);
        } else if p.mode != Mode::Mut && a.mutable {
            let d = self.error(msg::TYPE_MUT_ARGUMENT_UNEXPECTED, a.range).with("param", p.name.clone()).fix(
                msg::TYPE_REMOVE_MUT,
                Vec::new(),
                Vec::new(),
            );
            self.report(d);
        }
        // What earlier arguments inferred may settle an associated type.
        let pty = self.prog.normalize(&self.infer.zonk(&p.ty), &self.env);
        // An argument that mentions `it` where a one-parameter function is
        // expected is a closure over `it`.
        if let Ty::Fn(f) = &pty
            && f.params.len() == 1
            && !matches!(a.value.kind, ast::ExprKind::Closure(_))
            && mentions_it(&a.value)
        {
            let c = self.closure_from_parts(
                vec![("it".into(), None, a.value.range)],
                ClosureSource::Expr(a.value.clone()),
                Some((**f).clone()),
                a.value.range,
            );
            let c = self.coerce(c, &pty);
            return Arg { expr: c, mode: ArgMode::Value };
        }
        match p.mode {
            Mode::Own => {
                let x = self.check_expr(&a.value, &pty);
                Arg { expr: self.check_movable(x), mode: ArgMode::Value }
            }
            Mode::Mut => {
                let x = self.infer_expr(&a.value, Some(&pty));
                if let Ty::Ref(inner, true) = self.resolved(&x.ty) {
                    if !self.infer.try_unify(&inner, &pty) {
                        let x = self.coerce(x, &Ty::Ref(Box::new(pty.clone()), true));
                        return Arg { expr: x, mode: ArgMode::Value };
                    }
                    return Arg { expr: x, mode: ArgMode::Value };
                }
                if !self.infer.try_unify(&x.ty, &pty) {
                    let x = self.coerce(x, &pty);
                    return Arg { expr: x, mode: ArgMode::BorrowMut };
                }
                if x.is_place() {
                    self.require_mutable(&x, a.value.range);
                }
                Arg { expr: x, mode: ArgMode::BorrowMut }
            }
            Mode::Borrow if p.opaque => {
                // A generic parameter: always by reference.
                let x = self.infer_expr(&a.value, Some(&pty));
                if let Ty::Ref(inner, _) = self.resolved(&x.ty)
                    && self.infer.try_unify(&inner, &pty)
                {
                    return Arg { expr: x, mode: ArgMode::Value };
                }
                let x = self.coerce(x, &pty);
                Arg { expr: x, mode: ArgMode::Ref }
            }
            Mode::Borrow => {
                let prog = self.prog;
                if let Ty::Adt(d, targs) = &pty
                    && Some(*d) == prog.wk.iterable
                {
                    let x = self.infer_expr(&a.value, None);
                    let item_ty = targs.first().cloned().unwrap_or(Ty::Error);
                    return match self.iteration_item(&x.ty, Mode::Borrow) {
                        Some((item, mode)) => {
                            let item = self.decay_ty(&item);
                            if !self.infer.try_unify(&item, &item_ty) {
                                let d = self
                                    .error(msg::TYPE_MISMATCH, a.value.range)
                                    .with("expected", self.show(&pty))
                                    .with("found", self.show(&x.ty));
                                self.report(d);
                            }
                            if mode == ForMode::Iter {
                                Arg { expr: self.check_movable(x), mode: ArgMode::Value }
                            } else {
                                Arg { expr: x, mode: ArgMode::Borrow }
                            }
                        }
                        None => {
                            if x.ty != Ty::Error {
                                let d = self
                                    .error(msg::TYPE_MISMATCH, a.value.range)
                                    .with("expected", self.show(&pty))
                                    .with("found", self.show(&x.ty));
                                self.report(d);
                            }
                            Arg { expr: x, mode: ArgMode::Value }
                        }
                    };
                }
                if self.is_copy(&pty) && !matches!(pty, Ty::Var(_)) {
                    let x = self.check_expr(&a.value, &pty);
                    return Arg { expr: x, mode: ArgMode::Value };
                }
                let x = self.infer_expr(&a.value, Some(&pty));
                if let Ty::Ref(inner, _) = self.resolved(&x.ty)
                    && self.infer.try_unify(&inner, &pty)
                {
                    // A reference is passed on as it is.
                    return Arg { expr: x, mode: ArgMode::Value };
                }
                let x = self.coerce(x, &pty);
                let mode = if self.is_copy(&x.ty) { ArgMode::Value } else { ArgMode::Borrow };
                Arg { expr: x, mode }
            }
        }
    }

    /// `recv.name(args)`: a method of the receiver's type, then of a trait
    /// it implements (or, for a generic parameter, a trait it is bounded
    /// by); through references and `Box`es.
    pub fn method_call_on(
        &mut self,
        recv: Expr,
        name: &ast::Name,
        args: &[ast::Arg],
        trailing: Option<&ast::Closure>,
        range: TextRange,
        expected: Option<&Ty>,
    ) -> Expr {
        let span = self.span(range);
        let prog = self.prog;
        let mut recv = self.auto_deref(recv);
        loop {
            let base = self.resolved(&recv.ty);
            match &base {
                Ty::Error => {
                    self.check_args_loosely(args);
                    return Expr::error(span);
                }
                Ty::Var(_) => {
                    let d = self
                        .error(msg::TYPE_CANNOT_INFER, recv.span.range)
                        .with("name", self.source_text(recv.span.range));
                    self.report(d);
                    self.check_args_loosely(args);
                    return Expr::error(span);
                }
                _ => {}
            }
            if name.text == "copy" && args.is_empty() && trailing.is_none() {
                return Expr::new(ExprKind::CopyOf(Box::new(recv)), base, span);
            }
            if name.text == "trace" && args.is_empty() && matches!(base, Ty::ErrSet(_)) {
                return Expr::new(ExprKind::Trace(Box::new(recv)), self.string_ty(), span);
            }
            let mut def = base.key().and_then(|k| prog.method(k, &name.text));
            // A `Shared`'s own functions take it as an argument: a method
            // call reaches the value's methods.
            if self.smart_pointer(&base) == Some("Shared")
                && def.is_some_and(|d| prog.fn_info(d).is_some_and(|f| f.self_mode.is_none()))
            {
                def = None;
            }
            let mut base = base;
            if def.is_none()
                && self.is_string(&base)
                && let Some(d) = prog.method(TyKey::Str, &name.text)
            {
                def = Some(d);
                recv = self.coerce(recv, &Ty::Str);
                base = Ty::Str;
            }
            if let Some(d) = def {
                // A `dict` or `set` finds its keys by their hash.
                if let Ty::Adt(cd, a) = &base
                    && (Some(*cd) == prog.wk.dict || Some(*cd) == prog.wk.set)
                    && let Some(k) = a.first()
                {
                    self.require(k.clone(), prog.wk.hash, span);
                }
                return self.inherent_call(d, recv, base, name, args, trailing, range, expected);
            }
            // A method of a trait.
            let norm = prog.normalize(&self.infer.zonk(&base), &self.env);
            let candidates = prog.trait_methods_for(&norm, &name.text, &self.env);
            match candidates.len() {
                1 => {
                    let (tr, m) = candidates[0].clone();
                    return self.trait_call(m, tr, recv, norm, name, args, trailing, range, expected);
                }
                0 => {}
                _ => {
                    let traits: Vec<String> = candidates.iter().map(|(t, _)| self.show_trait(t)).collect();
                    let d = self
                        .error(msg::TYPE_AMBIGUOUS_METHOD, name.range)
                        .with("name", name.text.clone())
                        .with("ty", self.show(&base))
                        .with("traits", traits.join(", "));
                    self.report(d);
                    self.check_args_loosely(args);
                    return Expr::error(span);
                }
            }
            // A `Box` and a `Shared` lend their methods' receiver to what
            // they hold.
            if self.is_box(&base) {
                recv = self.unbox(recv);
                recv = self.auto_deref(recv);
                continue;
            }
            if self.smart_pointer(&base).is_some() {
                recv = self.deref_smart(recv);
                recv = self.auto_deref(recv);
                continue;
            }
            // A field holding a function.
            if let Ty::Adt(sd, _) = &base
                && prog.struct_info(*sd).is_some_and(|s| s.fields.iter().any(|f| f.name == name.text))
            {
                let f = self.field_of(recv, name, range);
                return self.call_value(f, args, trailing, range);
            }
            let d = self
                .error(msg::TYPE_UNKNOWN_METHOD, name.range)
                .with("ty", self.show(&base))
                .with("name", name.text.clone());
            let d = self.unknown_method_note(d, &name.text);
            self.report(d);
            self.check_args_loosely(args);
            return Expr::error(span);
        }
    }

    /// A note naming the traits that have a method called `name`, when the
    /// receiver implements none of them.
    fn unknown_method_note(&self, d: sisu_diag::Diagnostic, name: &str) -> sisu_diag::Diagnostic {
        let prog = self.prog;
        let traits: Vec<String> = prog
            .defs
            .iter()
            .enumerate()
            .filter(|(i, def)| {
                matches!(def.kind, DefKind::Trait(_)) && prog.trait_method(DefId(*i as u32), name).is_some()
            })
            .map(|(_, def)| format!("`{}`", def.name))
            .collect();
        if traits.is_empty() {
            return d;
        }
        d.note(msg::TYPE_METHOD_OF_TRAITS, vec![("name", name.to_string()), ("traits", traits.join(", "))])
    }

    pub fn show_trait(&self, t: &sisu_resolve::TraitRef) -> String {
        self.prog.display_trait_in(t, &self.generic_names)
    }

    #[allow(clippy::too_many_arguments)]
    fn inherent_call(
        &mut self,
        d: DefId,
        recv: Expr,
        base: Ty,
        name: &ast::Name,
        args: &[ast::Arg],
        trailing: Option<&ast::Closure>,
        range: TextRange,
        expected: Option<&Ty>,
    ) -> Expr {
        let span = self.span(range);
        let prog = self.prog;
        let Some(info) = prog.fn_info(d) else { return Expr::error(span) };
        if info.header_broken {
            self.check_args_loosely(args);
            return Expr::error(span);
        }
        let Some(mode) = info.self_mode else {
            let d = self
                .error(msg::TYPE_ASSOC_NOT_METHOD, name.range)
                .with("name", name.text.clone())
                .with("ty", self.show(&base));
            self.report(d);
            self.check_args_loosely(args);
            return Expr::error(span);
        };
        let recv_arg = match mode {
            Mode::Borrow => {
                if self.is_copy(&base) {
                    Arg { expr: recv, mode: ArgMode::Value }
                } else {
                    Arg { expr: recv, mode: ArgMode::Borrow }
                }
            }
            Mode::Mut => {
                if recv.is_place() {
                    self.require_mutable(&recv, recv.span.range);
                }
                Arg { expr: recv, mode: ArgMode::BorrowMut }
            }
            Mode::Own => Arg { expr: self.check_movable(recv), mode: ArgMode::Value },
        };
        self.call_fn(d, None, Some(base), Some(recv_arg), args, trailing, range, expected)
    }

    /// A call of the trait method `m` of `tr` on `recv`, of type `self_ty`.
    /// `self` is the trait's `Self`, a generic parameter, so a borrowed one
    /// is passed by reference whatever its type.
    #[allow(clippy::too_many_arguments)]
    pub fn trait_call(
        &mut self,
        m: DefId,
        tr: sisu_resolve::TraitRef,
        recv: Expr,
        self_ty: Ty,
        name: &ast::Name,
        args: &[ast::Arg],
        trailing: Option<&ast::Closure>,
        range: TextRange,
        expected: Option<&Ty>,
    ) -> Expr {
        let span = self.span(range);
        let prog = self.prog;
        let Some(info) = prog.fn_info(m) else { return Expr::error(span) };
        if info.header_broken {
            self.check_args_loosely(args);
            return Expr::error(span);
        }
        let Some(mode) = info.self_mode else {
            let d = self
                .error(msg::TYPE_ASSOC_NOT_METHOD, name.range)
                .with("name", name.text.clone())
                .with("ty", self.show(&self_ty));
            self.report(d);
            self.check_args_loosely(args);
            return Expr::error(span);
        };
        let recv_arg = match mode {
            Mode::Borrow => {
                if let Ty::Ref(..) = self.resolved(&recv.ty) {
                    Arg { expr: recv, mode: ArgMode::Value }
                } else {
                    Arg { expr: recv, mode: ArgMode::Ref }
                }
            }
            Mode::Mut => {
                if recv.is_place() {
                    self.require_mutable(&recv, recv.span.range);
                }
                Arg { expr: recv, mode: ArgMode::BorrowMut }
            }
            Mode::Own => Arg { expr: self.check_movable(recv), mode: ArgMode::Value },
        };
        let mut outer = vec![self_ty.clone()];
        outer.extend(tr.args.iter().cloned());
        self.call_fn_with(
            m,
            None,
            Some(outer),
            Some(self_ty),
            Some(recv_arg),
            args,
            trailing,
            range,
            expected,
        )
    }

    /// Calls a value of function type.
    pub fn call_value(
        &mut self,
        f: Expr,
        args: &[ast::Arg],
        trailing: Option<&ast::Closure>,
        range: TextRange,
    ) -> Expr {
        let span = self.span(range);
        // A reference to a function value calls the function (a function
        // value is `Copy`): `for g in handlers: g(1)`.
        let mut f = f;
        while let Ty::Ref(inner, _) = self.resolved(&f.ty)
            && matches!(self.resolved(&inner), Ty::Fn(_))
        {
            let fspan = f.span;
            f = Expr::new(ExprKind::Deref(Box::new(f)), *inner, fspan);
        }
        let t = self.resolved(&f.ty);
        let Ty::Fn(fty) = t else {
            if t != Ty::Error {
                let d = self.error(msg::TYPE_NOT_CALLABLE, range).with("ty", self.show(&t));
                self.report(d);
            }
            self.check_args_loosely(args);
            return Expr::error(span);
        };
        let params: Vec<InstParam> = fty
            .params
            .iter()
            .enumerate()
            .map(|(i, p)| InstParam {
                name: format!("{i}"),
                mode: Mode::Own,
                ty: p.clone(),
                has_default: false,
                opaque: false,
            })
            .collect();
        let sig = InstSig {
            def: DefId(u32::MAX),
            name: self.source_text(f.span.range),
            params,
            ret: fty.ret.clone(),
            owner: None,
            type_args: Vec::new(),
            keyword_only_from: None,
            positional_only_until: fty.params.len(),
        };
        let args = self.check_args(&sig, args, trailing, range);
        if let Some(root) = f.place_root() {
            self.note_use(root, false);
        }
        Expr::new(ExprKind::Call { callee: Callee::Value(Box::new(f)), args }, fty.ret.clone(), span)
    }
}
