//! The body checker: state, scopes, locals, annotations, blocks and
//! statements. Expressions are in `expr.rs`, calls in `call.rs`, patterns
//! in `pat.rs`.

use crate::infer::InferTable;
use crate::thir::*;
use sisu_diag::{Diagnostic, FileId, MsgId, Span, TextRange, msg};
use sisu_resolve::{DefId, DefKind, ModuleId, Predicate, Program, TraitRef, Ty};
use sisu_syntax::ast::{self, TypeKind};
use std::collections::HashMap;

/// What a `return` (or `?`) in the current function or closure targets.
pub(crate) struct RetCtx {
    pub ty: Ty,
    pub closure: Option<ClosureId>,
    /// Locals with a smaller index were declared outside this closure.
    pub first_local: u32,
    pub loops: u32,
    pub captures: Vec<Capture>,
}

pub(crate) struct Checker<'p> {
    pub prog: &'p Program,
    pub inferred: &'p HashMap<DefId, Vec<DefId>>,
    pub file: FileId,
    pub module: ModuleId,
    pub def: DefId,
    pub source: &'p str,
    pub infer: InferTable,
    pub locals: Vec<Local>,
    pub closures: Vec<Option<Closure>>,
    pub scopes: Vec<Vec<(String, LocalId)>>,
    pub diags: Vec<Diagnostic>,
    pub ret: Vec<RetCtx>,
    pub self_ty: Option<Ty>,
    /// Bounds to check once inference is done: (type, bound, span).
    pub bounds: Vec<(Ty, TraitRef, Span)>,
    /// The impls a call of a trait method asks for: like `bounds`, but an
    /// impl's associated types are its own.
    pub impl_bounds: Vec<(Ty, TraitRef, Span)>,
    /// The bounds in scope: the function's own, elaborated with
    /// supertraits.
    pub env: Vec<Predicate>,
    /// The names of the function's generic parameters, for diagnostics.
    pub generic_names: Vec<String>,
    /// Which of them are `const` (an `int` in expressions).
    pub const_generics: Vec<bool>,
    /// The type arguments written on the method call being checked
    /// (`x.truncate_to[u32]()`).
    pub explicit_method_args: Option<Vec<Ty>>,
    /// The `comptime assert`s on `const` generic parameters, for the calls
    /// to check (`Body::generic_asserts`).
    pub generic_asserts: Vec<crate::thir::GenericAssert>,
    /// Values put in a `Shared`, `Cell` or `Mutex`, with the owner's name:
    /// none may be a view or must-consume (checked once types are known).
    pub owner_checks: Vec<(Ty, &'static str, Span)>,
    /// The function's header or body did not parse: a name the body cannot
    /// find is most likely a parameter or binding lost in that error, so it
    /// is not reported.
    pub quiet_unknown_names: bool,
    /// While a later branch of an `if` or `match` with no expected type is
    /// converted to the type of the first: the keyword and the first
    /// branch, for E0365 instead of a plain mismatch.
    pub branch: Option<(&'static str, TextRange)>,
    /// While a value is converted to a type written in the source (a
    /// `let`'s annotation): where, for the mismatch's label.
    pub because: Option<TextRange>,
    /// While a field's default is checked: the struct, whose other fields
    /// the default cannot use (E0221).
    pub field_default_of: Option<DefId>,
    /// Checking code that runs at compile time: the body of a `comptime def`
    /// or a `comptime` expression, where a `comptime def` may be called.
    pub in_comptime: bool,
    /// The `comptime` closures, for the checks once types are known.
    pub comptime_sites: Vec<(ClosureId, TextRange)>,
    /// The `spawn` calls of the body, for the type of their scope.
    pub spawn_sites: Vec<crate::tasks::SpawnSite>,
    /// The tasks spawned, for the `Send` and `Sync` checks once types are
    /// known.
    pub spawn_checks: Vec<(ClosureId, Span)>,
    /// The `own` closures, for the checks of their captures once types are
    /// known (a task's may move what it captures).
    pub own_checks: Vec<(ClosureId, TextRange)>,
    /// In a generator, the type of what `yield` gives.
    pub generator: Option<Ty>,
    /// Values made into capability handles, with the handle's name, for
    /// the `Send` and `Sync` checks once types are known.
    pub cap_checks: Vec<(Ty, String, Span)>,
    /// While `ensures` clauses are checked: `result`'s scope and the
    /// values `old(...)` saves on entry.
    pub ensures: Option<crate::contracts::EnsuresCtx>,
    /// The `T !` functions whose inferred error set the body read: what
    /// the per-item query of the body depends on besides the interface.
    pub inferred_reads: std::cell::RefCell<Vec<DefId>>,
}

impl<'p> Checker<'p> {
    pub fn new(
        prog: &'p Program,
        inferred: &'p HashMap<DefId, Vec<DefId>>,
        def: DefId,
        file: FileId,
        source: &'p str,
    ) -> Checker<'p> {
        let module = prog.def(def).module;
        Checker {
            prog,
            inferred,
            file,
            module,
            def,
            source,
            infer: InferTable::default(),
            locals: Vec::new(),
            closures: Vec::new(),
            scopes: vec![Vec::new()],
            diags: Vec::new(),
            ret: Vec::new(),
            self_ty: None,
            bounds: Vec::new(),
            impl_bounds: Vec::new(),
            env: Vec::new(),
            generic_names: Vec::new(),
            const_generics: Vec::new(),
            explicit_method_args: None,
            generic_asserts: Vec::new(),
            owner_checks: Vec::new(),
            quiet_unknown_names: prog.fn_info(def).is_some_and(|f| f.header_broken || f.body_broken),
            branch: None,
            because: None,
            field_default_of: None,
            in_comptime: prog.fn_info(def).is_some_and(|f| f.comptime),
            comptime_sites: Vec::new(),
            spawn_sites: Vec::new(),
            spawn_checks: Vec::new(),
            own_checks: Vec::new(),
            generator: None,
            cap_checks: Vec::new(),
            ensures: None,
            inferred_reads: std::cell::RefCell::new(Vec::new()),
        }
    }

    // ---------------------------------------------------------------- diagnostics

    pub fn span(&self, range: TextRange) -> Span {
        Span::new(self.file, range)
    }

    pub fn report(&mut self, d: Diagnostic) {
        self.diags.push(d);
    }

    /// A diagnostic at `range`, without the line breaks and indentation a
    /// statement's range ends with.
    pub fn error(&mut self, m: MsgId, range: TextRange) -> Diagnostic {
        let mut end = range.end as usize;
        let bytes = self.source.as_bytes();
        while end > range.start as usize && bytes.get(end - 1).is_some_and(|b| b.is_ascii_whitespace()) {
            end -= 1;
        }
        Diagnostic::new(m, self.span(TextRange::new(range.start, end as u32)))
    }

    pub fn unsupported(&mut self, range: TextRange, construct: &str, phase: &str) {
        let d = self.error(msg::STAGE0_UNSUPPORTED, range).with("construct", construct).with("phase", phase);
        self.report(d);
    }

    /// A function only the standard library may call (`@internal`): the
    /// forwarding of lent capabilities, the assembling of a `System`.
    pub fn check_internal(&mut self, d: DefId, range: TextRange) {
        let prog = self.prog;
        let Some(f) = prog.fn_info(d) else { return };
        // Code the compiler writes (`<arbitrary>/...`) may call them too.
        let m = &prog.modules[self.module.0 as usize];
        if f.internal && !m.is_std && !m.name.starts_with('<') {
            let name = match &f.owner {
                Some(o) => format!("{}.{}", prog.display(o), prog.def(d).name),
                None => prog.qualified_name(d),
            };
            let diag = self.error(msg::CAP_INTERNAL, range).with("name", name);
            self.report(diag);
        }
    }

    /// The display form of a type, with variables resolved as far as known.
    pub fn show(&self, ty: &Ty) -> String {
        self.prog.display_in(&self.infer.zonk(ty), &self.generic_names)
    }

    /// Requires `ty` to implement the trait `tr` (a well-known one, when
    /// `tr` is `None` the prelude lacks it and nothing is required).
    pub fn require(&mut self, ty: Ty, tr: Option<DefId>, span: Span) {
        if let Some(def) = tr {
            self.bounds.push((ty, TraitRef { def, args: Vec::new() }, span));
        }
    }

    /// The fix of a missing impl (10, "Missing-trait errors list the
    /// candidates"): when the type has every method of the trait as its
    /// own, adding the impl is enough.
    pub fn missing_impl_fix(&self, d: Diagnostic, ty: &Ty, tr: &TraitRef) -> Diagnostic {
        let prog = self.prog;
        let Some(info) = prog.trait_info(tr.def) else { return d };
        let Some(key) = ty.key() else { return d };
        if info.methods.is_empty() {
            return d;
        }
        let all = info.methods.iter().all(|m| prog.method(key, &prog.def(*m).name).is_some());
        if !all {
            return d;
        }
        d.fix(msg::TYPE_ADD_IMPL, vec![("trait", prog.display_trait(tr)), ("ty", self.show(ty))], Vec::new())
    }

    pub fn source_text(&self, range: TextRange) -> String {
        self.source.get(range.start as usize..range.end as usize).unwrap_or("").to_string()
    }

    // ---------------------------------------------------------------- scopes and locals

    pub fn push_scope(&mut self) {
        self.scopes.push(Vec::new());
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn current_closure(&self) -> Option<ClosureId> {
        self.ret.last().and_then(|r| r.closure)
    }

    pub fn new_local(&mut self, name: &str, ty: Ty, mutable: bool, span: Span) -> LocalId {
        let id = LocalId(self.locals.len() as u32);
        let closure = self.current_closure();
        self.locals.push(Local { name: name.to_string(), ty, mutable, once: false, span, closure });
        id
    }

    /// Declares a name in the innermost scope. Rebinding in the same scope
    /// is allowed; shadowing a name of an enclosing scope warns (D-024).
    pub fn declare(&mut self, name: &str, local: LocalId, warn_shadow: bool, range: TextRange) {
        if name.is_empty() || name == "_" {
            return;
        }
        if warn_shadow {
            let depth = self.scopes.len();
            let shadows =
                self.scopes[..depth.saturating_sub(1)].iter().any(|s| s.iter().any(|(n, _)| n == name));
            if shadows && !self.scopes.last().is_some_and(|s| s.iter().any(|(n, _)| n == name)) {
                let d = self.error(msg::RESOLVE_SHADOW_OUTER, range).with("name", name);
                self.report(d);
            }
        }
        if let Some(scope) = self.scopes.last_mut() {
            scope.push((name.to_string(), local));
        }
    }

    pub fn lookup_local(&self, name: &str) -> Option<LocalId> {
        for scope in self.scopes.iter().rev() {
            if let Some((_, l)) = scope.iter().rev().find(|(n, _)| n == name) {
                return Some(*l);
            }
        }
        None
    }

    /// Records that the current closures capture `local`, if it was
    /// declared outside them.
    pub fn note_use(&mut self, local: LocalId, mutable: bool) {
        for ctx in self.ret.iter_mut().rev() {
            if ctx.closure.is_none() || local.0 >= ctx.first_local {
                break;
            }
            match ctx.captures.iter_mut().find(|c| c.local == local) {
                Some(c) => c.mutable |= mutable,
                None => ctx.captures.push(Capture { local, mutable, by_value: false }),
            }
        }
    }

    pub fn local_ty(&self, l: LocalId) -> Ty {
        self.locals.get(l.0 as usize).map(|x| x.ty.clone()).unwrap_or(Ty::Error)
    }

    // ---------------------------------------------------------------- annotations

    /// Resolves a type written inside a body (`let x: T`).
    pub fn resolve_ty(&mut self, te: &ast::TypeExpr) -> Ty {
        let prog = self.prog;
        match &te.kind {
            TypeKind::Error => Ty::Error,
            TypeKind::Path { segments, args } => {
                let names: Vec<&str> = segments.iter().map(|s| s.text.as_str()).collect();
                // `cap.Fake[Trait]`: the fake the compiler wrote (17).
                if let [arg] = args.as_slice()
                    && let Some(d) = self.lookup_type_path(&names)
                    && self.is_fake_marker(d)
                {
                    let as_expr = match &arg.kind {
                        TypeKind::Path { segments, args } if segments.len() == 1 && args.is_empty() => {
                            Some(segments[0].text.clone())
                        }
                        _ => None,
                    };
                    return self.fake_named(as_expr, arg.range);
                }
                let args: Vec<Ty> = args.iter().map(|a| self.resolve_ty(a)).collect();
                if let [n] = names.as_slice()
                    && let Some(i) = self.generic_names.iter().position(|g| g == n)
                {
                    return Ty::Param(i as u32);
                }
                // A constant where a type's constant argument goes.
                if let [n] = names.as_slice()
                    && args.is_empty()
                    && prog
                        .lookup(self.module, n)
                        .is_some_and(|d| matches!(prog.def(d).kind, DefKind::Const(_)))
                {
                    return match sisu_resolve::consts::const_item(prog, self.module, n, te.range) {
                        Ok(t) => t,
                        Err(err) => {
                            let file = self.file;
                            let d = sisu_resolve::consts::diagnostic(&err, |r| Span::new(file, r));
                            self.report(d);
                            Ty::Error
                        }
                    };
                }
                // `C.Item`, `Self.Item`: an associated type of a parameter.
                if let [base, name] = names.as_slice() {
                    let subject = if let Some(i) = self.generic_names.iter().position(|g| g == base) {
                        Some(Ty::Param(i as u32))
                    } else if *base == "Self" {
                        self.self_ty.clone()
                    } else {
                        None
                    };
                    if let Some(subject) = subject {
                        return match self.assoc_of(&subject, name) {
                            Some(t) => prog.normalize(&t, &self.env),
                            None => {
                                let d = self
                                    .error(msg::RESOLVE_NO_ASSOC_TYPE, te.range)
                                    .with("ty", self.show(&subject))
                                    .with("name", name.to_string());
                                self.report(d);
                                Ty::Error
                            }
                        };
                    }
                }
                match names.as_slice() {
                    ["int"] => Ty::Int,
                    ["f64"] => Ty::Float,
                    ["bool"] => Ty::Bool,
                    ["char"] => Ty::Char,
                    ["str"] => Ty::Str,
                    ["Never"] => Ty::Never,
                    ["Self"] => match &self.self_ty {
                        Some(t) => t.clone(),
                        None => {
                            let d = self.error(msg::TYPE_SELF_TYPE_OUTSIDE, te.range);
                            self.report(d);
                            Ty::Error
                        }
                    },
                    _ => {
                        let found = self.lookup_type_path(&names);
                        // Inside a trait, its associated types are named bare.
                        if found.is_none()
                            && let [n] = names.as_slice()
                            && self.self_ty == Some(Ty::Param(0))
                            && let Some(t) = self.assoc_of(&Ty::Param(0), n)
                        {
                            return prog.normalize(&t, &self.env);
                        }
                        match found {
                            Some(d) => self.def_type(d, args, te.range),
                            None => {
                                let d = self
                                    .error(msg::RESOLVE_UNDEFINED_TYPE, te.range)
                                    .with("name", names.join("."));
                                self.report(d);
                                Ty::Error
                            }
                        }
                    }
                }
            }
            TypeKind::Option(inner) => {
                let t = self.resolve_ty(inner);
                prog.wk.option.map(|o| Ty::Adt(o, vec![t])).unwrap_or(Ty::Error)
            }
            TypeKind::Result { ok, err } => {
                let ok = ok.as_ref().map(|o| self.resolve_ty(o)).unwrap_or(Ty::Unit);
                let err = match err {
                    Some(e) => self.resolve_err_set(e),
                    None => {
                        let d = self.error(msg::RESOLVE_INFERRED_SET_POSITION, te.range);
                        self.report(d);
                        Ty::Error
                    }
                };
                prog.wk.result.map(|r| Ty::Adt(r, vec![ok, err])).unwrap_or(Ty::Error)
            }
            TypeKind::Tuple(elems) => {
                let tys: Vec<Ty> = elems.iter().map(|(_, t)| self.resolve_ty(t)).collect();
                if tys.is_empty() { Ty::Unit } else { Ty::tuple(tys) }
            }
            TypeKind::Fn { pure, params, ret } => {
                let params = params.iter().map(|p| self.resolve_ty(p)).collect();
                let ret = self.resolve_ty(ret);
                Ty::Fn(Box::new(sisu_resolve::FnTy { params, ret, pure: *pure }))
            }
            TypeKind::Ref { mutable, inner } => {
                let t = self.resolve_ty(inner);
                Ty::Ref(Box::new(t), *mutable)
            }
            TypeKind::Dyn(inner) => {
                let TypeKind::Path { segments, args } = &inner.kind else { return Ty::Error };
                let names: Vec<&str> = segments.iter().map(|s| s.text.as_str()).collect();
                let args: Vec<Ty> = args.iter().map(|a| self.resolve_ty(a)).collect();
                match self.lookup_type_path(&names) {
                    Some(d) if prog.trait_info(d).is_some() => Ty::Dyn(TraitRef { def: d, args }),
                    _ => {
                        let d =
                            self.error(msg::RESOLVE_NOT_A_TRAIT, inner.range).with("name", names.join("."));
                        self.report(d);
                        Ty::Error
                    }
                }
            }
            TypeKind::Union(_) => {
                let d = self.error(msg::RESOLVE_UNION_OUTSIDE_RESULT, te.range);
                self.report(d);
                Ty::Error
            }
            TypeKind::ConstArg(n) => Ty::Const(*n),
            TypeKind::ConstExpr(e) => self.const_arg(e),
            TypeKind::Unsupported(_) => {
                self.unsupported(te.range, "this type", "a later phase");
                Ty::Error
            }
        }
    }

    /// A constant argument written as an expression, inside a body: the
    /// function's `const` parameters are its operands (`consts.rs`).
    pub fn const_arg(&mut self, e: &ast::Expr) -> Ty {
        use sisu_resolve::consts::Local;
        let names = self.generic_names.clone();
        let consts = self.const_generics.clone();
        let locals: Vec<String> = self.scopes.iter().flatten().map(|(n, _)| n.clone()).collect();
        let param = |n: &str| match names.iter().position(|g| g == n) {
            Some(i) if consts.get(i) == Some(&true) => Local::ConstParam(i as u32),
            Some(_) => Local::NotConstant,
            None if locals.iter().any(|l| l == n) => Local::NotConstant,
            None => Local::Item,
        };
        match sisu_resolve::consts::const_arg(self.prog, self.module, e, &param) {
            Ok(t) => t,
            Err(err) => {
                let file = self.file;
                let d = sisu_resolve::consts::diagnostic(&err, |r| sisu_diag::Span::new(file, r));
                self.report(d);
                Ty::Error
            }
        }
    }

    /// The associated type `name` of `subject`, from the bounds in scope.
    pub fn assoc_of(&self, subject: &Ty, name: &str) -> Option<Ty> {
        let prog = self.prog;
        for p in &self.env {
            if p.ty != *subject {
                continue;
            }
            let declares =
                prog.trait_info(p.bound.def).is_some_and(|i| i.assoc.iter().any(|a| a.name == name));
            if declares {
                return Some(Ty::Assoc(Box::new(sisu_resolve::AssocTy {
                    self_ty: subject.clone(),
                    trait_: p.bound.clone(),
                    name: name.to_string(),
                })));
            }
        }
        // A concrete type: whichever trait it implements declares the name.
        for (i, def) in prog.defs.iter().enumerate() {
            let DefKind::Trait(info) = &def.kind else { continue };
            if !info.assoc.iter().any(|a| a.name == name) || !info.generics.is_empty() {
                continue;
            }
            let tr = TraitRef { def: DefId(i as u32), args: Vec::new() };
            if prog.implements(subject, &tr, &self.env).holds() {
                return Some(Ty::Assoc(Box::new(sisu_resolve::AssocTy {
                    self_ty: subject.clone(),
                    trait_: tr,
                    name: name.to_string(),
                })));
            }
        }
        None
    }

    fn resolve_err_set(&mut self, te: &ast::TypeExpr) -> Ty {
        let parts: Vec<&ast::TypeExpr> = match &te.kind {
            TypeKind::Union(list) => list.iter().collect(),
            _ => vec![te],
        };
        let mut members = Vec::new();
        for p in parts {
            match self.resolve_ty(p) {
                Ty::ErrSet(m) => members.extend(m),
                Ty::Error => return Ty::Error,
                other => {
                    let name = self.show(&other);
                    let d = self.error(msg::RESOLVE_NOT_AN_ERROR_SET, p.range).with("name", name);
                    self.report(d);
                    return Ty::Error;
                }
            }
        }
        members.sort();
        members.dedup();
        Ty::ErrSet(members)
    }

    pub fn lookup_type_path(&self, names: &[&str]) -> Option<DefId> {
        let first = names.first()?;
        let mut d = self.prog.lookup(self.module, first)?;
        for n in &names[1..] {
            match &self.prog.def(d).kind {
                DefKind::Module(m) => d = self.prog.lookup_in(*m, n)?,
                _ => return None,
            }
        }
        Some(d)
    }

    pub fn def_type(&mut self, d: DefId, args: Vec<Ty>, range: TextRange) -> Ty {
        let prog = self.prog;
        let name = prog.def(d).name.clone();
        let generics = match &prog.def(d).kind {
            DefKind::Struct(s) => s.generics.len(),
            DefKind::Enum(e) => e.generics.len(),
            DefKind::Refinement(r) => r.generics.len(),
            DefKind::ErrorSet(_) => return Ty::ErrSet(vec![d]),
            DefKind::TypeAlias(t) => return t.subst(&args),
            DefKind::Trait(_) => {
                let at = Span::new(self.file, TextRange::empty(range.start));
                let diag = self.error(msg::RESOLVE_TRAIT_AS_TYPE, range).with("name", name.clone()).fix(
                    msg::RESOLVE_ADD_DYN,
                    vec![("name", name)],
                    vec![sisu_diag::Edit { span: at, text: "dyn ".into() }],
                );
                self.report(diag);
                return Ty::Error;
            }
            _ => {
                let diag = self.error(msg::RESOLVE_NOT_A_TYPE, range).with("name", name);
                self.report(diag);
                return Ty::Error;
            }
        };
        let mut args = args;
        // Omitted trailing arguments take their defaults (D-060).
        if !args.is_empty() && args.len() < generics {
            let gs = prog.type_generics(d);
            while args.len() < generics {
                match gs.get(args.len()).and_then(|g| g.default.clone()) {
                    Some(t) => {
                        let t = t.subst(&args);
                        args.push(t);
                    }
                    None => break,
                }
            }
        }
        if args.len() != generics {
            if args.is_empty() {
                // `list` in a body annotation: the arguments are inferred.
                return Ty::Adt(d, (0..generics).map(|_| self.infer.fresh()).collect());
            }
            let diag = self
                .error(msg::RESOLVE_TYPE_ARGUMENT_COUNT, range)
                .with("name", name)
                .with("expected", generics.to_string())
                .with("found", args.len().to_string());
            self.report(diag);
            return Ty::Adt(d, (0..generics).map(|_| Ty::Error).collect());
        }
        // A `const` parameter takes a constant, any other a type.
        let gs = prog.type_generics(d);
        for (i, a) in args.iter().enumerate() {
            let Some(g) = gs.get(i) else { continue };
            let wrong = if g.is_const {
                let param_ok =
                    matches!(a, Ty::Param(k) if self.const_generics.get(*k as usize) == Some(&true));
                (!matches!(a, Ty::Const(_) | Ty::ConstOp(..) | Ty::Error | Ty::Var(_)) && !param_ok)
                    .then_some(msg::RESOLVE_CONST_ARGUMENT)
            } else {
                let const_param =
                    matches!(a, Ty::Param(k) if self.const_generics.get(*k as usize) == Some(&true));
                (matches!(a, Ty::Const(_) | Ty::ConstOp(..)) || const_param)
                    .then_some(msg::RESOLVE_TYPE_ARGUMENT)
            };
            if let Some(m) = wrong {
                let diag = self.error(m, range).with("name", name.clone()).with("param", g.name.clone());
                self.report(diag);
                return Ty::Adt(d, (0..generics).map(|_| Ty::Error).collect());
            }
        }
        Ty::Adt(d, args)
    }

    // ---------------------------------------------------------------- helpers on types

    pub fn resolved(&self, ty: &Ty) -> Ty {
        self.infer.resolve(ty)
    }

    pub fn is_copy(&self, ty: &Ty) -> bool {
        let t = self.infer.zonk(ty);
        self.prog.is_copy_in(&t, &self.env)
    }

    pub fn option_of(&self, t: Ty) -> Ty {
        self.prog.wk.option.map(|o| Ty::Adt(o, vec![t])).unwrap_or(Ty::Error)
    }

    pub fn list_of(&self, t: Ty) -> Ty {
        self.prog.wk.list.map(|o| Ty::Adt(o, vec![t])).unwrap_or(Ty::Error)
    }

    pub fn string_ty(&self) -> Ty {
        self.prog.wk.string.map(|s| Ty::Adt(s, vec![])).unwrap_or(Ty::Error)
    }

    /// `Some(T)` when the type is `T?`.
    pub fn as_option(&self, ty: &Ty) -> Option<Ty> {
        match self.resolved(ty) {
            Ty::Adt(d, a) if Some(d) == self.prog.wk.option => a.first().cloned(),
            _ => None,
        }
    }

    /// `Some((T, E))` when the type is `T ! E`.
    pub fn as_result(&self, ty: &Ty) -> Option<(Ty, Ty)> {
        match self.resolved(ty) {
            Ty::Adt(d, a) if Some(d) == self.prog.wk.result && a.len() == 2 => {
                Some((a[0].clone(), a[1].clone()))
            }
            _ => None,
        }
    }

    pub fn is_builtin(&self, ty: &Ty, name: &str) -> bool {
        match self.resolved(ty) {
            Ty::Adt(d, _) => self.prog.builtin(d) == Some(name),
            _ => false,
        }
    }

    pub fn is_string(&self, ty: &Ty) -> bool {
        self.is_builtin(ty, "String")
    }

    /// Whether a type is an option (`T?`).
    pub fn is_option(&self, ty: &Ty) -> bool {
        matches!(ty, Ty::Adt(d, _) if Some(*d) == self.prog.wk.option)
    }

    // ---------------------------------------------------------------- blocks and statements

    pub fn check_block(&mut self, b: &ast::Block) -> Block {
        self.push_scope();
        let block = self.check_block_in_scope(b);
        self.pop_scope();
        block
    }

    /// Checks a block's statements in the current scope.
    pub fn check_block_in_scope(&mut self, b: &ast::Block) -> Block {
        let mut stmts = Vec::new();
        let mut diverged = false;
        let mut warned = false;
        for s in &b.stmts {
            if diverged && !warned && !matches!(s.kind, ast::StmtKind::Pass | ast::StmtKind::Error) {
                let d = self.error(msg::TYPE_UNREACHABLE_CODE, s.range);
                self.report(d);
                warned = true;
            }
            let before = stmts.len();
            self.check_stmt(s, &mut stmts);
            if stmts[before..].iter().any(|st| self.stmt_diverges(st)) {
                diverged = true;
            }
        }
        Block { stmts, span: self.span(b.range) }
    }

    pub fn check_stmt(&mut self, s: &ast::Stmt, out: &mut Vec<Stmt>) {
        let span = self.span(s.range);
        match &s.kind {
            ast::StmtKind::Let { mutable, pat, ty, init, else_ } => {
                self.check_let(*mutable, pat, ty.as_ref(), init.as_ref(), else_.as_ref(), s.range, out)
            }
            ast::StmtKind::Assign { target, op, value } => {
                out.push(self.check_assign(target, *op, value, s.range))
            }
            ast::StmtKind::Expr(e) => out.push(self.check_expr_stmt(e)),
            ast::StmtKind::Return(v) => out.push(self.check_return(v.as_ref(), s.range)),
            ast::StmtKind::Break => {
                if self.ret.last().is_some_and(|r| r.loops > 0) {
                    out.push(Stmt::Break(span));
                } else {
                    let d = self
                        .error(msg::TYPE_BREAK_OUTSIDE_LOOP, s.range)
                        .note(msg::TYPE_BREAK_USE_RETURN, Vec::new());
                    self.report(d);
                    out.push(Stmt::Error(span));
                }
            }
            ast::StmtKind::Continue => {
                if self.ret.last().is_some_and(|r| r.loops > 0) {
                    out.push(Stmt::Continue(span));
                } else {
                    let d = self.error(msg::TYPE_CONTINUE_OUTSIDE_LOOP, s.range);
                    self.report(d);
                    out.push(Stmt::Error(span));
                }
            }
            ast::StmtKind::Pass => {}
            ast::StmtKind::If { branches, else_block } => {
                let mut bs = Vec::new();
                for (cond, block) in branches {
                    self.push_scope();
                    let c = self.check_cond(cond);
                    let b = self.check_block(block);
                    self.pop_scope();
                    bs.push((c, b));
                }
                let e = else_block.as_ref().map(|b| self.check_block(b));
                out.push(Stmt::If { branches: bs, else_: e, span });
            }
            ast::StmtKind::While { cond, body } => {
                self.push_scope();
                let c = self.check_cond(cond);
                self.enter_loop();
                let b = self.check_block(body);
                self.exit_loop();
                self.pop_scope();
                out.push(Stmt::While { cond: c, body: b, span });
            }
            ast::StmtKind::For { pat, mode, iter, body } => {
                self.push_scope();
                let (iter, fmode, item) = self.check_iterable(iter, *mode);
                let p = self.check_pat(pat, &item, None);
                self.enter_loop();
                let b = self.check_block(body);
                self.exit_loop();
                self.pop_scope();
                out.push(Stmt::For { pat: p, iter, mode: fmode, item, body: b, span });
            }
            ast::StmtKind::Assert { cond, message } => {
                let c = self.check_condition_expr(cond);
                let m = message.as_ref().map(|m| self.assert_message(m));
                let text = self.source_text(cond.range);
                let replay = self.only_reads(&c, false);
                out.push(Stmt::Assert { cond: c, message: m, text, span, replay });
            }
            ast::StmtKind::Expect(cond) => {
                // `expect` records and continues in tests (17); stage 0 has no
                // test runner that continues, so it checks like `assert`.
                let c = self.check_condition_expr(cond);
                let text = self.source_text(cond.range);
                let replay = self.only_reads(&c, false);
                out.push(Stmt::Assert { cond: c, message: None, text, span, replay });
            }
            ast::StmtKind::Contract(c) => self.contract_stmt(c, out),
            ast::StmtKind::Defer(block) => {
                let b = self.check_block(block);
                out.push(Stmt::Defer { body: b, span });
            }
            ast::StmtKind::With { value, name, body } => {
                // `with e as x:` is a scoped binding dropped at the end of the block.
                self.push_scope();
                let v = self.infer_expr(value, None);
                let v = self.decay(v);
                let ty = v.ty.clone();
                let l = self.new_local(&name.text, ty.clone(), false, self.span(name.range));
                self.declare(&name.text, l, false, name.range);
                let pat = Pat {
                    kind: PatKind::Bind { local: l, mode: BindMode::Value },
                    ty,
                    span: self.span(name.range),
                };
                let mut stmts = vec![Stmt::Let { pat, init: Some(v), unwrap: None, else_: None, span }];
                let b = self.check_block(body);
                stmts.push(Stmt::Block(b));
                self.pop_scope();
                out.push(Stmt::Block(Block { stmts, span }));
            }
            ast::StmtKind::Item(item) => {
                let what = match &item.kind {
                    ast::ItemKind::Fn(_) => "a nested `def`",
                    ast::ItemKind::Const(_) => "a local `const`",
                    _ => "a nested declaration",
                };
                self.unsupported(item.range, what, "nested declarations (a later phase)");
                out.push(Stmt::Error(span));
            }
            ast::StmtKind::Comptime(inner) => match &inner.kind {
                // An assert on `const` generic parameters: decided at each
                // call with constant arguments (`generic_asserts.rs`).
                ast::StmtKind::Assert { cond, message }
                    if self.names_const_param(cond)
                        || message.as_ref().is_some_and(|m| self.names_const_param(m)) =>
                {
                    let c = self.check_condition_expr(cond);
                    let m = message.as_ref().map(|m| self.check_expr(m, &Ty::Str));
                    let text = self.source_text(cond.range);
                    self.generic_asserts.push(crate::thir::GenericAssert { cond: c, message: m, text });
                }
                // Run at compile time; nothing is left for run time.
                ast::StmtKind::Assert { .. } | ast::StmtKind::Expr(_) => {
                    let b =
                        ast::Block { node: inner.node, range: inner.range, stmts: vec![(**inner).clone()] };
                    let x = self.comptime_block(&b, s.range);
                    out.push(Stmt::Expr(x));
                }
                _ => {
                    self.unsupported(
                        s.range,
                        "`comptime for` and `comptime if` outside a derive or a generic function",
                        "compile-time code in methods and non-generic functions (Phase 8)",
                    );
                    out.push(Stmt::Error(span));
                }
            },
            ast::StmtKind::ReturnImpl(_) => {
                self.unsupported(
                    s.range,
                    "`return impl` outside a trait's `comptime def derive`",
                    "reflection beyond derive (Phase 8)",
                );
                out.push(Stmt::Error(span));
            }
            ast::StmtKind::Select(arms) => out.push(self.check_select(arms, s.range)),
            ast::StmtKind::Yield(value) => out.push(self.check_yield(value.as_ref(), s.range)),
            ast::StmtKind::Unsupported(what) => {
                let phase = match *what {
                    "`yield`" => "generators (Phase 8)",
                    "`select`" => "tasks (Phase 6)",
                    "an `unsafe` block" => "foreign functions (Phase 8)",
                    _ => "contracts (Phase 8)",
                };
                self.unsupported(s.range, what, phase);
                out.push(Stmt::Error(span));
            }
            ast::StmtKind::Error => out.push(Stmt::Error(span)),
        }
    }

    /// Whether an expression names one of the function's `const` generic
    /// parameters.
    fn names_const_param(&self, e: &ast::Expr) -> bool {
        let mut found = false;
        ast::visit::expr(e, &mut |x| {
            if let ast::ExprKind::Name(n) = &x.kind
                && let Some(i) = self.generic_names.iter().position(|g| g == n)
                && self.const_generics.get(i) == Some(&true)
            {
                found = true;
            }
        });
        found
    }

    pub fn enter_loop(&mut self) {
        if let Some(r) = self.ret.last_mut() {
            r.loops += 1;
        }
    }

    pub fn exit_loop(&mut self) {
        if let Some(r) = self.ret.last_mut() {
            r.loops = r.loops.saturating_sub(1);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn check_let(
        &mut self,
        mutable: bool,
        pat: &ast::Pattern,
        ty: Option<&ast::TypeExpr>,
        init: Option<&ast::Expr>,
        else_: Option<&ast::LetElse>,
        range: TextRange,
        out: &mut Vec<Stmt>,
    ) {
        let span = self.span(range);
        let declared = ty.map(|t| {
            let d = self.resolve_ty(t);
            // A `dyn Trait` lives behind `ref` or in a `Box`.
            if let Some(u) = sisu_resolve::unsized_part(self.prog, &d) {
                let e = self.error(msg::TYPE_UNSIZED_DYN, t.range).with("ty", self.show(&u));
                self.report(e);
            }
            d
        });
        let Some(init) = init else {
            // `let x: T`: declared now, assigned later.
            let ty = match declared {
                Some(t) => t,
                None => {
                    let name = match &pat.kind {
                        ast::PatKind::Bind(n) => n.text.clone(),
                        _ => String::new(),
                    };
                    let d = self.error(msg::TYPE_LET_NEEDS_TYPE, pat.range).with("name", name);
                    self.report(d);
                    Ty::Error
                }
            };
            let p = self.bind_simple(pat, &ty, true);
            if !mutable && let PatKind::Bind { local, .. } = &p.kind {
                self.locals[local.0 as usize].once = true;
            }
            out.push(Stmt::Let { pat: p, init: None, unwrap: None, else_: None, span });
            return;
        };
        let refutable = pattern_is_refutable(pat);
        match else_ {
            Some(e) if !refutable => {
                // `let x = e else: v`: unwrap an option or a result.
                let value = self.infer_expr(init, None);
                let (inner, err_ty, unwrap) = if let Some(t) = self.as_option(&value.ty) {
                    (t, Ty::Unit, Some(Unwrap::Option))
                } else if let Some((t, e)) = self.as_result(&value.ty) {
                    (t, e, Some(Unwrap::Result))
                } else {
                    if value.ty != Ty::Error {
                        let shown = self.show(&value.ty);
                        let d = self.error(msg::TYPE_ELSE_NOT_FALLIBLE, init.range).with("ty", shown);
                        self.report(d);
                    }
                    (Ty::Error, Ty::Error, None)
                };
                let target = match &declared {
                    Some(d) => {
                        let tmp = Expr::new(ExprKind::Error, inner.clone(), value.span);
                        let _ = self.coerce(tmp, d);
                        d.clone()
                    }
                    None => inner.clone(),
                };
                let target = self.decay_ty(&target);
                let else_ = self.check_let_else(e, &target, &err_ty);
                let p = self.check_pat_with(pat, &target, None, mutable, true);
                out.push(Stmt::Let { pat: p, init: Some(value), unwrap, else_: Some(else_), span });
            }
            Some(e) => {
                // `let Some(x) = e else:` with a block that leaves the scope.
                let value = self.infer_expr(init, None);
                let vty = value.ty.clone();
                let else_ = self.check_let_else(e, &Ty::Never, &Ty::Unit);
                if let LetElse::Fallback { value: v, .. } = &else_
                    && v.ty != Ty::Never
                    && v.ty != Ty::Error
                {
                    let d = self.error(msg::TYPE_ELSE_MUST_DIVERGE, e.range);
                    self.report(d);
                }
                let p = self.check_pat_with(pat, &vty, None, mutable, true);
                out.push(Stmt::Let { pat: p, init: Some(value), unwrap: None, else_: Some(else_), span });
            }
            None => {
                let value = match (&declared, ty) {
                    (Some(t), Some(written)) => self.check_expr_because(init, t, written.range),
                    (Some(t), None) => self.check_expr(init, t),
                    (None, _) => {
                        let v = self.infer_expr(init, None);
                        self.decay(v)
                    }
                };
                let value = self.check_movable(value);
                if refutable {
                    let d = self.error(msg::TYPE_REFUTABLE_LET, pat.range);
                    self.report(d);
                }
                let vty = declared.unwrap_or(value.ty.clone());
                let p = self.check_pat_with(pat, &vty, None, mutable, true);
                out.push(Stmt::Let { pat: p, init: Some(value), unwrap: None, else_: None, span });
            }
        }
    }

    fn check_let_else(&mut self, e: &ast::LetElse, target: &Ty, err_ty: &Ty) -> LetElse {
        self.push_scope();
        let binding = e.binding.as_ref().map(|b| {
            let l = self.new_local(&b.text, err_ty.clone(), false, self.span(b.range));
            self.declare(&b.text, l, false, b.range);
            l
        });
        let result = match &e.body {
            ast::ElseBody::Expr(x) => {
                let v =
                    if *target == Ty::Never { self.infer_expr(x, None) } else { self.check_expr(x, target) };
                LetElse::Fallback { binding, value: v }
            }
            ast::ElseBody::Diverge(stmt) => {
                let mut stmts = Vec::new();
                self.check_stmt(stmt, &mut stmts);
                LetElse::Diverge { binding, body: Block { stmts, span: self.span(stmt.range) } }
            }
            ast::ElseBody::Block(b) => {
                let block = self.check_block(b);
                if !self.block_diverges(&block) {
                    let d = self.error(msg::TYPE_ELSE_MUST_DIVERGE, b.range);
                    self.report(d);
                }
                LetElse::Diverge { binding, body: block }
            }
        };
        self.pop_scope();
        result
    }

    /// A simple binding pattern for `let x: T` without a value.
    fn bind_simple(&mut self, pat: &ast::Pattern, ty: &Ty, mutable: bool) -> Pat {
        match &pat.kind {
            ast::PatKind::Bind(n) => {
                let l = self.new_local(&n.text, ty.clone(), mutable, self.span(n.range));
                self.declare(&n.text, l, true, n.range);
                Pat {
                    kind: PatKind::Bind { local: l, mode: BindMode::Value },
                    ty: ty.clone(),
                    span: self.span(pat.range),
                }
            }
            _ => self.check_pat_with(pat, ty, None, mutable, true),
        }
    }

    fn check_assign(
        &mut self,
        target: &ast::Expr,
        op: Option<ast::BinOp>,
        value: &ast::Expr,
        range: TextRange,
    ) -> Stmt {
        let span = self.span(range);
        if op.is_none() {
            if let ast::ExprKind::Name(n) = &target.kind
                && n == "_"
            {
                let v = self.infer_expr(value, None);
                return Stmt::Discard { value: v, span };
            }
            if let ast::ExprKind::Tuple(targets) = &target.kind {
                let places: Vec<Expr> = targets.iter().map(|t| self.check_place(t, true)).collect();
                let tys: Vec<Ty> = places.iter().map(|p| p.ty.clone()).collect();
                let v = self.check_expr(value, &Ty::tuple(tys));
                return Stmt::AssignTuple { places, value: v, span };
            }
        }
        let place = self.check_place(target, true);
        match op {
            None => {
                let v = self.check_expr(value, &place.ty.clone());
                let v = self.check_movable(v);
                Stmt::Assign { place, value: v, span }
            }
            Some(op) => {
                let pty = self.decay_ty(&place.ty);
                let v = self.infer_expr(value, Some(&pty));
                let v = self.decay(v);
                // On a type with an operator trait, `x -= y` is `x = x - y`
                // through the trait (D-022).
                let read = self.decay(place.clone());
                if let Some(call) = self.operator_call(op, read, Some(v.clone()), range) {
                    let call = self.coerce(call, &pty);
                    return Stmt::Assign { place, value: call, span };
                }
                let lhs = Expr::new(ExprKind::Error, pty.clone(), place.span);
                let (result, _) = self.binary_result(op, &lhs, &v, range);
                if result != Ty::Error {
                    let shown_place = self.show(&pty);
                    let shown_result = self.show(&result);
                    if !self.infer.try_unify(&result, &pty) {
                        let d = self
                            .error(msg::TYPE_MISMATCH, value.range)
                            .with("expected", shown_place)
                            .with("found", shown_result);
                        self.report(d);
                    }
                }
                Stmt::AugAssign { place, op, value: v, span }
            }
        }
    }

    /// An expression used as an assignment target: a mutable place.
    pub fn check_place(&mut self, target: &ast::Expr, for_write: bool) -> Expr {
        let e = self.infer_expr(target, None);
        // A call returning `ref mut T` is a place too (`entry(k).or_insert(0) += 1`).
        let e = match self.resolved(&e.ty) {
            Ty::Ref(inner, true) if !e.is_place() || matches!(e.kind, ExprKind::Local(_)) => {
                let span = e.span;
                Expr::new(ExprKind::Deref(Box::new(e)), *inner, span)
            }
            _ => e,
        };
        if e.ty == Ty::Error {
            return e;
        }
        if !e.is_place() {
            let d = self.error(msg::TYPE_NOT_A_PLACE, target.range);
            self.report(d);
            return e;
        }
        if for_write {
            self.require_mutable_as(&e, target.range, true);
        }
        e
    }

    /// Reports an assignment to, or a `mut` borrow of, an immutable place.
    pub fn require_mutable(&mut self, e: &Expr, range: TextRange) {
        self.require_mutable_as(e, range, false);
    }

    /// `assign` distinguishes `x = v` (E0312) from a `mut` borrow (E0341).
    pub fn require_mutable_as(&mut self, e: &Expr, range: TextRange, assign: bool) {
        if let Some(root) = mutable_root_problem(e, &self.locals, &self.infer, self.prog) {
            match root {
                Problem::Immutable(l) => {
                    let name = self.locals[l.0 as usize].name.clone();
                    let decl = self.locals[l.0 as usize].span;
                    let m = if assign && matches!(e.kind, ExprKind::Local(_)) {
                        msg::TYPE_ASSIGN_IMMUTABLE
                    } else {
                        msg::TYPE_NOT_MUTABLE
                    };
                    let what = if matches!(e.kind, ExprKind::Local(_)) {
                        name.clone()
                    } else {
                        self.source_text(range)
                    };
                    // A borrowed parameter is changed by taking it with
                    // `mut`; a local by declaring it with `var`.
                    let param = self.prog.fn_info(self.def).and_then(|f| {
                        f.params.iter().find(|p| p.span == decl && p.mode == ast::Mode::Borrow).cloned()
                    });
                    let d = self.error(m, range).with("name", name.clone()).with("what", what);
                    let d = match param {
                        Some(p) => d.fix(
                            msg::TYPE_TAKE_MUT,
                            vec![("name", p.name.clone())],
                            vec![sisu_diag::Edit {
                                span: Span::new(p.span.file, TextRange::empty(p.span.range.start)),
                                text: "mut ".into(),
                            }],
                        ),
                        None => d.fix(msg::TYPE_DECLARE_VAR, vec![("name", name)], Vec::new()),
                    };
                    let d = d.label(
                        decl,
                        msg::RESOLVE_DEFINED_HERE,
                        vec![("name", self.locals[l.0 as usize].name.clone())],
                    );
                    self.report(d);
                }
                Problem::BehindSharedRef => {
                    let what = self.source_text(range);
                    let mut d = self.error(msg::TYPE_NOT_MUTABLE, range).with("what", what);
                    // A borrowed parameter: taking it with `mut` lets the
                    // function change what the caller lends.
                    // Through the reference the parameter is.
                    let mut base = e;
                    while let ExprKind::Deref(inner) | ExprKind::Field { base: inner, .. } = &base.kind {
                        base = inner;
                    }
                    if let Some(root) = base.place_root()
                        && let Some(p) = self.prog.fn_info(self.def).and_then(|f| {
                            f.params.iter().find(|p| {
                                p.span == self.locals[root.0 as usize].span && p.mode == ast::Mode::Borrow
                            })
                        })
                    {
                        let at = Span::new(p.span.file, TextRange::empty(p.span.range.start));
                        d = d.fix(
                            msg::TYPE_TAKE_MUT,
                            vec![("name", p.name.clone())],
                            vec![sisu_diag::Edit { span: at, text: "mut ".into() }],
                        );
                    }
                    self.report(d);
                }
            }
        }
        self.check_protected_place(e, range);
        if let Some(root) = e.place_root() {
            self.note_use(root, true);
        }
    }

    fn check_expr_stmt(&mut self, e: &ast::Expr) -> Stmt {
        let span = self.span(e.range);
        match &e.kind {
            ast::ExprKind::Match { scrutinee, arms } => {
                return self.check_match_stmt(scrutinee, arms, e.range);
            }
            ast::ExprKind::If { branches, else_ } => {
                // An inline `if` in statement position: its values are discarded.
                let mut bs = Vec::new();
                for (c, v) in branches {
                    let cond = self.check_condition_expr(c);
                    let value = self.infer_expr(v, None);
                    bs.push((
                        Cond { parts: vec![CondPart::Bool(cond)] },
                        Block { stmts: vec![Stmt::Expr(value)], span: self.span(v.range) },
                    ));
                }
                let el = else_.as_ref().map(|v| {
                    let value = self.infer_expr(v, None);
                    Block { stmts: vec![Stmt::Expr(value)], span: self.span(v.range) }
                });
                return Stmt::If { branches: bs, else_: el, span };
            }
            _ => {}
        }
        let x = self.infer_expr(e, None);
        let ty = self.resolved(&x.ty);
        let unused = match &ty {
            Ty::Adt(d, _) => Some(*d) == self.prog.wk.option || Some(*d) == self.prog.wk.result,
            _ => false,
        };
        if unused {
            let shown = self.show(&ty);
            let edit = sisu_diag::Edit {
                span: Span::new(self.file, TextRange::empty(e.range.start)),
                text: "_ = ".into(),
            };
            let d = self.error(msg::TYPE_UNUSED_RESULT, e.range).with("ty", shown).fix(
                msg::TYPE_DISCARD,
                Vec::new(),
                vec![edit],
            );
            self.report(d);
        }
        let _ = span;
        Stmt::Expr(x)
    }

    /// The message of an `assert`: a string (E0379), or with the fix that
    /// writes the value into one.
    fn assert_message(&mut self, m: &ast::Expr) -> Expr {
        let x = self.infer_expr(m, Some(&Ty::Str));
        let t = self.resolved(&x.ty);
        if matches!(t, Ty::Str | Ty::Error | Ty::Never | Ty::Var(_)) || self.is_string(&t) {
            return self.coerce(x, &Ty::Str);
        }
        let mut d = self.error(msg::TYPE_ASSERT_MESSAGE, m.range);
        if let Some(text) = self.source.get(m.range.start as usize..m.range.end as usize)
            && !text.contains(['"', '\n'])
        {
            let edit = sisu_diag::Edit { span: self.span(m.range), text: format!("\"{{{text}}}\"") };
            d = d.fix(msg::TYPE_INTERPOLATE, Vec::new(), vec![edit]);
        }
        self.report(d);
        Expr::new(
            ExprKind::Coerce { expr: Box::new(x), kind: Coercion::Mismatch },
            Ty::Str,
            self.span(m.range),
        )
    }

    fn check_return(&mut self, v: Option<&ast::Expr>, range: TextRange) -> Stmt {
        let span = self.span(range);
        let ret = self.ret.last().map(|r| r.ty.clone()).unwrap_or(Ty::Unit);
        let fn_name = self.prog.def(self.def).name.clone();
        match v {
            None => {
                let r = self.resolved(&ret);
                if r != Ty::Unit && r != Ty::Error && !matches!(r, Ty::Var(_)) {
                    let shown = self.show(&r);
                    let d = self.error(msg::TYPE_RETURN_NEEDS_VALUE, range).with("ty", shown);
                    self.report(d);
                } else if matches!(r, Ty::Var(_)) {
                    self.infer.unify(&r, &Ty::Unit);
                }
                Stmt::Return { value: None, span }
            }
            Some(e) => {
                if self.resolved(&ret) == Ty::Unit && self.current_closure().is_none() {
                    let is_test = self
                        .prog
                        .fn_info(self.def)
                        .is_some_and(|f| matches!(f.kind, sisu_resolve::FnKind::Test(_)));
                    let d = if is_test {
                        self.error(msg::TYPE_RETURN_IN_TEST, e.range)
                    } else {
                        self.error(msg::TYPE_RETURN_VALUE_IN_UNIT, e.range).with("name", fn_name)
                    };
                    // The fix keeps the `return` and drops its value.
                    let keyword_end = range.start + "return".len() as u32;
                    let d = if self.source.get(range.start as usize..keyword_end as usize) == Some("return") {
                        let edit = sisu_diag::Edit {
                            span: self.span(TextRange::new(keyword_end, e.range.end)),
                            text: String::new(),
                        };
                        d.fix(msg::TYPE_DROP_RETURN_VALUE, Vec::new(), vec![edit])
                    } else {
                        d
                    };
                    self.report(d);
                    let x = self.infer_expr(e, None);
                    return Stmt::Return { value: Some(x), span };
                }
                // A refinement's predicate: a `bool` over `self` (E0814).
                if let Some(sisu_resolve::FnKind::Predicate(r)) =
                    self.prog.fn_info(self.def).map(|f| f.kind.clone())
                    && self.current_closure().is_none()
                {
                    let x = self.infer_expr(e, Some(&Ty::Bool));
                    let found = self.resolved(&x.ty);
                    if !matches!(found, Ty::Bool | Ty::Error | Ty::Never | Ty::Var(_)) {
                        let shown = self.show(&found);
                        let d = self
                            .error(msg::REFINE_PREDICATE_TYPE, e.range)
                            .with("ty", self.prog.def(r).name.clone())
                            .with("found", shown);
                        self.report(d);
                        return Stmt::Return { value: Some(Expr::error(x.span)), span };
                    }
                    let x = self.coerce(x, &Ty::Bool);
                    return Stmt::Return { value: Some(x), span };
                }
                let x = self.check_expr(e, &ret);
                let x = self.check_movable(x);
                Stmt::Return { value: Some(x), span }
            }
        }
    }

    // ---------------------------------------------------------------- conditions

    pub fn check_condition_expr(&mut self, e: &ast::Expr) -> Expr {
        // `if comptime a or b:` makes the whole condition compile-time (08),
        // though as an operator `comptime` takes the operand after it.
        let whole;
        let e = match whole_comptime(e) {
            Some(w) => {
                whole = w;
                &whole
            }
            None => e,
        };
        let x = self.infer_expr(e, Some(&Ty::Bool));
        let x = self.decay(x);
        let t = self.resolved(&x.ty);
        match t {
            Ty::Bool | Ty::Error | Ty::Never => x,
            Ty::Var(_) => {
                self.infer.unify(&t, &Ty::Bool);
                x
            }
            other => {
                let shown = self.show(&other);
                let d = self
                    .error(msg::TYPE_NOT_BOOL, e.range)
                    .with("found", shown)
                    .note(msg::TYPE_TRUTHINESS, Vec::new());
                self.report(d);
                let span = x.span;
                Expr::new(ExprKind::Coerce { expr: Box::new(x), kind: Coercion::Mismatch }, Ty::Bool, span)
            }
        }
    }

    /// A condition chain; its bindings are declared in the current scope,
    /// which the caller opens around the guarded block.
    pub fn check_cond(&mut self, c: &ast::Cond) -> Cond {
        let mut parts = Vec::new();
        for p in &c.parts {
            match p {
                ast::CondPart::Expr(e) => parts.push(CondPart::Bool(self.check_condition_expr(e))),
                ast::CondPart::Let { pat, value } => {
                    let v = self.infer_expr(value, None);
                    let by_ref = self.scrutinee_by_ref(&v);
                    let ty = v.ty.clone();
                    let p = self.check_pat(pat, &ty, by_ref);
                    parts.push(CondPart::Let { pat: p, value: v });
                }
            }
        }
        Cond { parts }
    }

    // ---------------------------------------------------------------- divergence

    pub fn block_diverges(&self, b: &Block) -> bool {
        b.stmts.iter().any(|s| self.stmt_diverges(s))
    }

    pub fn stmt_diverges(&self, s: &Stmt) -> bool {
        match s {
            Stmt::Return { .. } | Stmt::Break(_) | Stmt::Continue(_) => true,
            Stmt::Expr(e) => self.resolved(&e.ty) == Ty::Never,
            Stmt::Let { init: Some(e), else_: None, .. } => self.resolved(&e.ty) == Ty::Never,
            Stmt::If { branches, else_, .. } => match else_ {
                Some(e) => branches.iter().all(|(_, b)| self.block_diverges(b)) && self.block_diverges(e),
                None => {
                    branches.len() == 1
                        && branches[0].0.parts.is_empty()
                        && self.block_diverges(&branches[0].1)
                }
            },
            Stmt::Match { arms, .. } => {
                !arms.is_empty()
                    && arms.iter().all(|a| match &a.body {
                        ArmBody::Block(b) => self.block_diverges(b),
                        ArmBody::Expr(e) => self.resolved(&e.ty) == Ty::Never,
                    })
            }
            Stmt::Block(b) => self.block_diverges(b),
            Stmt::Select { arms, .. } => {
                !arms.is_empty() && arms.iter().all(|a| self.block_diverges(&a.body))
            }
            Stmt::While { cond, body, .. } => {
                let infinite = matches!(
                    cond.parts.as_slice(),
                    [CondPart::Bool(Expr { kind: ExprKind::Bool(true), .. })]
                );
                infinite && !block_breaks(body)
            }
            _ => false,
        }
    }
}

/// Whether a block contains a `break` of the loop it is the body of.
fn block_breaks(b: &Block) -> bool {
    b.stmts.iter().any(stmt_breaks)
}

fn stmt_breaks(s: &Stmt) -> bool {
    match s {
        Stmt::Break(_) => true,
        Stmt::If { branches, else_, .. } => {
            branches.iter().any(|(_, b)| block_breaks(b)) || else_.as_ref().is_some_and(block_breaks)
        }
        Stmt::Match { arms, .. } => {
            arms.iter().any(|a| matches!(&a.body, ArmBody::Block(b) if block_breaks(b)))
        }
        Stmt::Let { else_: Some(LetElse::Diverge { body, .. }), .. } => block_breaks(body),
        Stmt::Defer { body, .. } | Stmt::Block(body) => block_breaks(body),
        _ => false,
    }
}

pub(crate) enum Problem {
    Immutable(LocalId),
    BehindSharedRef,
}

/// Why a place cannot be written, if it cannot.
pub(crate) fn mutable_root_problem(
    e: &Expr,
    locals: &[Local],
    infer: &InferTable,
    prog: &sisu_resolve::Program,
) -> Option<Problem> {
    match &e.kind {
        ExprKind::Local(l) => {
            let local = locals.get(l.0 as usize)?;
            if local.mutable || matches!(infer.resolve(&local.ty), Ty::Ref(_, true)) {
                None
            } else {
                Some(Problem::Immutable(*l))
            }
        }
        ExprKind::Field { base, .. } | ExprKind::Index { base, .. } => {
            mutable_root_problem(base, locals, infer, prog)
        }
        ExprKind::Deref(inner) => match infer.resolve(&inner.ty) {
            Ty::Ref(_, true) => None,
            Ty::Ref(_, false) => Some(Problem::BehindSharedRef),
            // A `Shared` value is changed only through a `Cell` or a
            // `Mutex` in it; a mutex guard's value is the guard's to change.
            Ty::Adt(d, _) if prog.builtin(d) == Some("Shared") => Some(Problem::BehindSharedRef),
            _ => None,
        },
        _ => None,
    }
}

/// Whether a pattern can fail to match (anything but names, `_` and tuples of those).
pub(crate) fn pattern_is_refutable(p: &ast::Pattern) -> bool {
    match &p.kind {
        ast::PatKind::Wildcard | ast::PatKind::Bind(_) | ast::PatKind::Error => false,
        ast::PatKind::Tuple(ps) => ps.iter().any(pattern_is_refutable),
        _ => true,
    }
}

/// A condition whose first operand is `comptime x`, as `comptime` of the
/// whole condition.
fn whole_comptime(e: &ast::Expr) -> Option<ast::Expr> {
    fn strip(e: &ast::Expr) -> Option<ast::Expr> {
        let kind = match &e.kind {
            ast::ExprKind::Comptime(inner) => return Some((**inner).clone()),
            ast::ExprKind::Binary { op, lhs, rhs } => {
                ast::ExprKind::Binary { op: *op, lhs: Box::new(strip(lhs)?), rhs: rhs.clone() }
            }
            ast::ExprKind::Compare { first, rest } => {
                ast::ExprKind::Compare { first: Box::new(strip(first)?), rest: rest.clone() }
            }
            _ => return None,
        };
        Some(ast::Expr { kind, ..e.clone() })
    }
    if matches!(e.kind, ast::ExprKind::Comptime(_)) {
        return None;
    }
    let inner = strip(e)?;
    Some(ast::Expr { node: e.node, range: e.range, kind: ast::ExprKind::Comptime(Box::new(inner)) })
}
