//! Instances of templates (08, "Reflection"): a function or a method
//! whose body runs code at compile time (`if comptime
//! reflect.is_struct(T):`, `comptime for f in reflect.fields_of(T):`,
//! `value.[f]`) is made once for each list of type arguments it is called
//! with, by unfolding its body for them (`template.rs`), and the instance
//! is declared, checked and lowered like any other function; a call of the
//! template calls its instance. One without type parameters (a function or
//! a method that reflects on the program's own types, decision 244) is
//! made once, whether anything calls it or not, and its instance takes its
//! name, so `main`, a trait's call and a function value run it. A method's
//! instance is a method of an inherent impl of its type, whose
//! compile-time code names the type `Self`.
//!
//! Which instances a program needs is known only once its calls are
//! typed, so the driver checks the program, collects the type checker's
//! requests and resolves it again with them, until no new one comes (a
//! fixpoint; stage 0 resolves the whole program each time). A generic
//! function that calls a template with its own type parameters cannot
//! have that call's instance until it is called itself, so it becomes a
//! template too (promoted): monomorphisation by need, of the part of the
//! call graph that reflects.
//!
//! An instance is declared in a module of its own whose scope is the
//! template's module, with each type argument bound to `__T0`, `__T1`, ...
//! there, so the unfolded code names the argument whatever module it
//! comes from.

use crate::collect::Resolver;
use crate::program::*;
use crate::template::{FieldShape, Kind, MethodShape, TypeShape, VariantShape};
use crate::ty::Ty;
use sisu_diag::{Diagnostic, msg};
use sisu_syntax::ast::{self, ExprKind};

/// Whether `f`'s body has code that runs at compile time by unfolding
/// (08, "Reflection"): a `reflect` query, a field reached by a
/// compile-time descriptor (`x.[f]`), a `comptime for` or `comptime if`
/// (among statements or a `match`'s arms), or a compile-time expression
/// that names one of `names` (the type parameters in scope).
pub(crate) fn unfolds(f: &ast::FnDef, names: &[String]) -> bool {
    let Some(body) = &f.body else { return false };
    let mut found = comptime_stmts(body);
    let mentions = |e: &ast::Expr| {
        let mut m = false;
        ast::visit::expr(e, &mut |x| {
            if let ExprKind::Name(n) = &x.kind
                && names.contains(n)
            {
                m = true;
            }
        });
        m
    };
    ast::visit::block(body, &mut |e| match &e.kind {
        ExprKind::FieldOf { .. } => found = true,
        ExprKind::Call { callee, .. } if matches!(&callee.kind, ExprKind::Field { base, .. } if matches!(&base.kind, ExprKind::Name(n) if n == "reflect")) => {
            found = true
        }
        ExprKind::Comptime(inner) if mentions(inner) => found = true,
        ExprKind::Match { arms, .. } => {
            found |= arms.iter().any(|a| {
                a.generated.is_some() || matches!(&a.body, ast::ArmBody::Block(b) if comptime_stmts(b))
            })
        }
        ExprKind::Closure(c) => found |= matches!(&c.body, ast::ClosureBody::Block(b) if comptime_stmts(b)),
        _ => {}
    });
    found
}

/// Whether a block has a `comptime for` or `comptime if` among its
/// statements, at any depth of statements.
pub(crate) fn comptime_stmts(b: &ast::Block) -> bool {
    b.stmts.iter().any(comptime_stmt)
}

fn comptime_stmt(s: &ast::Stmt) -> bool {
    use ast::StmtKind as K;
    match &s.kind {
        K::Comptime(inner) => !matches!(inner.kind, K::Assert { .. } | K::Expr(_)),
        K::If { branches, else_block } => {
            branches.iter().any(|(_, b)| comptime_stmts(b)) || else_block.as_ref().is_some_and(comptime_stmts)
        }
        K::While { body, .. } | K::For { body, .. } | K::With { body, .. } | K::Defer(body) => {
            comptime_stmts(body)
        }
        K::Let { else_: Some(le), .. } => matches!(&le.body, ast::ElseBody::Block(b) if comptime_stmts(b)),
        _ => false,
    }
}

/// The names the compile-time code of `f` passes to `reflect`'s queries
/// or computes with (`comptime ...`), in order: the types it may name.
fn named_types(f: &ast::FnDef) -> Vec<String> {
    let Some(body) = &f.body else { return Vec::new() };
    let mut out: Vec<String> = Vec::new();
    let mut names = |e: &ast::Expr| {
        ast::visit::expr(e, &mut |x| {
            if let ExprKind::Name(n) = &x.kind
                && !out.contains(n)
            {
                out.push(n.clone());
            }
        })
    };
    ast::visit::block(body, &mut |e| match &e.kind {
        ExprKind::Call { callee, args, .. } if matches!(&callee.kind, ExprKind::Field { base, .. } if matches!(&base.kind, ExprKind::Name(n) if n == "reflect")) => {
            args.iter().for_each(|a| names(&a.value))
        }
        ExprKind::Comptime(inner) => names(inner),
        _ => {}
    });
    out
}

/// Whether a value (a constant's, a field's default) runs code at compile
/// time by unfolding: a `reflect` query or a field reached by a
/// compile-time descriptor (decision 251).
pub(crate) fn value_unfolds(e: &ast::Expr) -> bool {
    let mut found = false;
    ast::visit::expr(e, &mut |x| match &x.kind {
        ExprKind::FieldOf { .. } => found = true,
        ExprKind::Call { callee, .. } if matches!(&callee.kind, ExprKind::Field { base, .. } if matches!(&base.kind, ExprKind::Name(n) if n == "reflect")) => {
            found = true
        }
        _ => {}
    });
    found
}

/// The names a value passes to `reflect`'s queries: the types it may name.
fn value_types(e: &ast::Expr) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    ast::visit::expr(e, &mut |x| {
        if let ExprKind::Call { callee, args, .. } = &x.kind
            && matches!(&callee.kind, ExprKind::Field { base, .. } if matches!(&base.kind, ExprKind::Name(n) if n == "reflect"))
        {
            for a in args {
                ast::visit::expr(&a.value, &mut |y| {
                    if let ExprKind::Name(n) = &y.kind
                        && !out.contains(n)
                    {
                        out.push(n.clone());
                    }
                });
            }
        }
    });
    out
}

/// A shape written as the module it is unfolded in names its types: the
/// name the code wrote for the type itself, the shown name for the types
/// in it.
fn written_plainly(mut s: TypeShape, written: &str) -> TypeShape {
    fn plain(s: &mut TypeShape) {
        s.written = s.name.clone();
        for f in &mut s.fields {
            if let Some(t) = &mut f.ty {
                plain(t);
            }
        }
        for v in &mut s.variants {
            for (_, t) in v.fields.iter_mut() {
                if let Some(t) = t {
                    plain(t);
                }
            }
        }
        if let Some(e) = &mut s.elem {
            plain(e);
        }
    }
    plain(&mut s);
    s.written = written.to_string();
    s
}

/// The most instances one resolution declares, so that a template whose
/// instances need ever larger types stops (`to_json[T]` calling
/// `to_json[list[T]]`).
pub const MAX_INSTANCES: usize = 256;

impl Resolver<'_> {
    /// Declares the instances `requests` ask for, after the program's own
    /// declarations are resolved, and resolves them.
    pub(crate) fn declare_instances(&mut self, requests: &[Request]) {
        let mut made = 0;
        for q in requests {
            let Request::Instance { def, args, site } = q else { continue };
            if self.prog.instances.contains_key(&(*def, args.clone())) {
                continue;
            }
            let Some(info) = self.prog.fn_info(*def).cloned() else { continue };
            // A template made once is made for no arguments (decision 251).
            let fits = if info.made_once() { args.is_empty() } else { info.generics.len() == args.len() };
            if !info.template || !fits {
                continue;
            }
            // Counted over the whole resolution, however many times it is
            // given more; reported once.
            if self.instances_asked >= MAX_INSTANCES {
                if self.instances_bounded {
                    break;
                }
                self.instances_bounded = true;
                let span = self.prog.def(*def).span;
                self.report(
                    Diagnostic::new(msg::COMPTIME_INSTANCES_UNBOUNDED, span)
                        .with("name", self.prog.def(*def).name.clone())
                        .with("count", MAX_INSTANCES.to_string()),
                );
                break;
            }
            self.instances_asked += 1;
            made += 1;
            self.declare_instance(*def, &info, args, *site);
        }
        if made > 0 {
            self.resolve_pending();
            self.record_method_instances();
        }
    }

    fn declare_instance(&mut self, def: DefId, info: &FnInfo, args: &[Ty], site: sisu_diag::Span) {
        let module = self.prog.def(def).module;
        let name = self.prog.def(def).name.clone();
        let shown: Vec<String> = args.iter().map(|a| self.prog.display(a)).collect();
        let what = if args.is_empty() { name.clone() } else { format!("{name}[{}]", shown.join(", ")) };
        let Some(ast) = self.template_ast(def) else { return };
        let source = self.sources.get(info.file).map(|f| f.text.clone()).unwrap_or_default();
        // A method of a trait impl is made once, as generic as it is: its
        // type parameters (the impl's, its own) stay parameters, which
        // reflection shows by their names and cannot look into (decision
        // 251).
        let generic = args.is_empty() && !info.generics.is_empty();
        if generic {
            self.shape_names = info.generics.iter().map(|g| g.name.clone()).collect();
        }
        // Each type the unfolded code may name (an argument, a field's
        // type, an element type) is an alias in the instance's module.
        let mut aliases: Vec<(String, Ty)> = Vec::new();
        let mut params: Vec<(String, TypeShape)> = info
            .generics
            .iter()
            .zip(args)
            .enumerate()
            .map(|(i, (g, t))| (g.name.clone(), self.shape(t, &format!("__T{i}"), 0, &mut aliases)))
            .collect();
        // A method's instance is a method of its type, which its
        // compile-time code names `Self` (decision 244), in an impl with
        // the generic impl's parameters: named as the impl names them, or
        // apart when the method is the trait's, whose code never names
        // them (decision 251).
        let within = info.owner.as_ref().map(|t| {
            let self_ty = if generic { t.clone() } else { t.subst(args) };
            params.push(("Self".to_string(), self.shape(&self_ty, "__S", 0, &mut aliases)));
            if !generic {
                return "impl __S".to_string();
            }
            let outer = &info.generics[..info.impl_generics];
            if outer.is_empty() {
                return "impl __S".to_string();
            }
            let names: Vec<String> = if self.provided_copies.contains(&def) {
                (0..outer.len()).map(|i| format!("__G{i}")).collect()
            } else {
                outer.iter().map(|g| g.name.clone()).collect()
            };
            format!("impl[{}] __S", names.join(", "))
        });
        // An impl for every type (`impl[T: B] Trait for T`) has no type
        // whose method the instance could be.
        if generic && let Some(t) = info.owner.as_ref().filter(|t| t.key().is_none()) {
            let shown = self.prog.display_in(t, &self.shape_names);
            self.shape_names.clear();
            self.report(Diagnostic::new(msg::COMPTIME_INSTANCE_FAILED, site).with("name", what).with(
                "reason",
                format!("`Self` is `{shown}`, a type parameter, which is not known at compile time"),
            ));
            return;
        }
        // The type parameters its code names, which it may ask the names
        // of: the impl's, unless the trait wrote the method, and its own.
        if generic {
            let from = if self.provided_copies.contains(&def) { info.impl_generics } else { 0 };
            for g in &info.generics[from..] {
                params.push((g.name.clone(), TypeShape::named(&g.name, &g.name, Kind::Param)));
            }
        }
        // The program's types its compile-time code names (`reflect.fields_of(Point)`),
        // and the traits (`reflect.methods_of(Store)`, decision 251).
        let mut types: Vec<(String, TypeShape)> = Vec::new();
        for (i, n) in
            named_types(&ast).into_iter().filter(|n| !params.iter().any(|(p, _)| p == n)).enumerate()
        {
            if let Some(t) = self.named_type(module, &n) {
                let shape = self.shape(&t, &format!("__C{i}"), 0, &mut aliases);
                types.push((n, shape));
            } else if let Some(shape) = self.named_trait(module, &n) {
                types.push((n, shape));
            }
        }
        self.shape_names.clear();
        let inst = format!("__instance_{}", self.instances_declared);
        let text =
            match crate::template::unfold_fn(&ast, &source, params, types, &inst, within.as_deref(), generic)
            {
                Ok(t) => t,
                Err((range, reason)) => {
                    let mut d = Diagnostic::new(msg::COMPTIME_INSTANCE_FAILED, site)
                        .with("name", what)
                        .with("reason", reason);
                    // Where the program's own template stopped; a standard
                    // library template's reason is the whole story.
                    if !self.is_std(module) {
                        d = d.label(self.span(module, range), msg::COMPTIME_INSTANCE_HERE, Vec::new());
                    }
                    self.report(d);
                    return;
                }
            };
        let Some(m) = self.declare_generated_in(module, &format!("<{what}>"), &text) else { return };
        self.instances_declared += 1;
        self.instance_modules.insert(m);
        // The types, by the names the unfolded code uses.
        for (alias, t) in aliases {
            let d = self.new_def(
                m,
                &alias,
                sisu_diag::TextRange::empty(0),
                ast::Vis::Private,
                Vec::new(),
                DefKind::TypeAlias(t),
            );
            self.prog.modules[m.0 as usize].items.insert(alias, d);
        }
        // A function is an item of the module; a method exists once its
        // impl is resolved.
        match self.prog.modules[m.0 as usize].items.get(&inst).copied() {
            Some(d) => self.record_instance(d, def, args.to_vec(), site),
            None => self.method_instances.push((m, inst, def, args.to_vec(), site)),
        }
    }

    fn record_instance(&mut self, d: DefId, template: DefId, args: Vec<Ty>, site: sisu_diag::Span) {
        self.prog.instances.insert((template, args.clone()), d);
        self.prog.instance_sites.insert(d, site);
        let Some(t) = self.prog.fn_info(template).cloned() else { return };
        if let DefKind::Fn(f) = &mut self.prog.defs[d.0 as usize].kind {
            f.instance_of = Some((template, args.clone()));
            // A test's instance is the test (decision 251).
            if matches!(t.kind, FnKind::Test(_)) {
                f.kind = t.kind.clone();
            }
            // A generic impl's method made once keeps the impl's bounds
            // and its own `where` clause, which its text does not write
            // (decision 251).
            if args.is_empty() && !t.generics.is_empty() && f.generics.len() == t.generics.len() {
                f.predicates = t.predicates.clone();
            }
        }
        self.instance_of.insert(d, (template, args));
    }

    /// The methods [`Resolver::declare_instance`] declared, now that their
    /// impls are resolved.
    fn record_method_instances(&mut self) {
        for (m, inst, template, args, site) in std::mem::take(&mut self.method_instances) {
            let found = (0..self.prog.defs.len()).rev().map(|i| DefId(i as u32)).find(|d| {
                let x = self.prog.def(*d);
                x.module == m && x.name == inst && matches!(x.kind, DefKind::Fn(_))
            });
            if let Some(d) = found {
                self.record_instance(d, template, args, site);
            }
        }
    }

    /// A constant's or a field default's value whose code runs at compile
    /// time by unfolding (decision 251), unfolded: the value with its
    /// compile-time parts written as literals, parsed from a text of its
    /// own (`<NAME>/MODULE.sisu`), which its diagnostics name. The types it
    /// names are written as `module` names them. A value that cannot be
    /// unfolded is `E0609`, at `site`.
    pub(crate) fn unfold_value(
        &mut self,
        module: ModuleId,
        name: &str,
        e: &ast::Expr,
        file: sisu_diag::FileId,
        site: sisu_diag::Span,
    ) -> Option<(ast::Expr, sisu_diag::FileId)> {
        let source = self.sources.get(file).map(|f| f.text.clone()).unwrap_or_default();
        let mut aliases = Vec::new();
        let mut types: Vec<(String, TypeShape)> = Vec::new();
        for n in value_types(e) {
            if let Some(t) = self.named_type(module, &n) {
                let shape = self.shape(&t, &n, 0, &mut aliases);
                types.push((n.clone(), written_plainly(shape, &n)));
            } else if let Some(shape) = self.named_trait(module, &n) {
                types.push((n, shape));
            }
        }
        let text = match crate::template::unfold_expr(e, &source, types) {
            Ok(t) => t,
            Err((range, reason)) => {
                let mut d = Diagnostic::new(msg::COMPTIME_INSTANCE_FAILED, site)
                    .with("name", name.to_string())
                    .with("reason", reason);
                if !self.is_std(module) {
                    d = d.label(self.span(module, range), msg::COMPTIME_INSTANCE_HERE, Vec::new());
                }
                self.report(d);
                return None;
            }
        };
        let text = format!("const __value = {text}\n");
        let parent = &self.prog.modules[module.0 as usize];
        let file_name = format!("<{name}>/{}.sisu", parent.name);
        let file_name = if parent.is_std { format!("<std>/{file_name}") } else { file_name };
        let new_file = self.sources.add(file_name, &text);
        let (ast, diags) = crate::collect::parse_generated(&text);
        self.diags.extend(diags.iter().cloned().map(|mut d| {
            d.spans_mut().for_each(|s| s.file = new_file);
            d
        }));
        ast.items.iter().find_map(|i| match &i.kind {
            ast::ItemKind::Const(c) => c.value.clone().map(|v| (v, new_file)),
            _ => None,
        })
    }

    /// The type `name` names in `module`, when it is a type without type
    /// parameters: a struct, an enum, an error set or an alias.
    fn named_type(&self, module: ModuleId, name: &str) -> Option<Ty> {
        let d = self.prog.lookup(module, name)?;
        match &self.prog.def(d).kind {
            DefKind::Struct(_) | DefKind::Enum(_) | DefKind::ErrorSet(_)
                if self.prog.type_generics(d).is_empty() =>
            {
                Some(Ty::Adt(d, Vec::new()))
            }
            DefKind::TypeAlias(t) => Some(t.clone()),
            _ => None,
        }
    }

    /// The trait `name` names in `module`, as reflection sees it: its
    /// methods (decision 251).
    fn named_trait(&self, module: ModuleId, name: &str) -> Option<TypeShape> {
        let d = self.prog.lookup(module, name)?;
        let info = self.prog.trait_info(d)?;
        let mut names = vec!["Self".to_string()];
        names.extend(info.generics.iter().map(|g| g.name.clone()));
        let mut s = TypeShape::named(name, name, Kind::Trait);
        s.methods = info.methods.iter().filter_map(|m| self.method_shape(*m, &names)).collect();
        Some(s)
    }

    /// What reflection sees of the method `m`, its types shown with
    /// `names` for the generic parameters in scope.
    fn method_shape(&self, m: DefId, names: &[String]) -> Option<MethodShape> {
        let f = self.prog.fn_info(m)?;
        let ty = |t: &Ty| {
            let shown = self.prog.display_in(t, names);
            let kind = match self.prog.unrefine(t) {
                Ty::Param(_) | Ty::Assoc(_) => Kind::Param,
                Ty::Int => Kind::Int,
                Ty::Float => Kind::Float,
                Ty::Bool => Kind::Bool,
                Ty::Char => Kind::Char,
                Ty::Str => Kind::Str,
                Ty::Tuple(..) => Kind::Tuple,
                _ => Kind::Other,
            };
            Box::new(TypeShape::named(&shown, &shown, kind))
        };
        let mode = |m: ast::Mode| match m {
            ast::Mode::Borrow => "",
            ast::Mode::Mut => "mut",
            ast::Mode::Own => "own",
        };
        let receiver = match f.self_mode {
            None => String::new(),
            Some(ast::Mode::Borrow) => "self".to_string(),
            Some(m) => format!("{} self", mode(m)),
        };
        Some(MethodShape {
            name: self.prog.def(m).name.clone(),
            receiver,
            params: f.params.iter().map(|p| (p.name.clone(), mode(p.mode).to_string(), ty(&p.ty))).collect(),
            ret: ty(&f.ret),
            provided: f.body.is_some(),
        })
    }

    /// A type's own methods, declared in it or in its inherent impls, in
    /// the order they are declared (decision 251).
    fn own_methods(&self, t: &Ty) -> Vec<MethodShape> {
        let Some(key) = t.key() else { return Vec::new() };
        let mut ms: Vec<DefId> = self
            .prog
            .methods
            .iter()
            .filter(|((k, _), d)| *k == key && self.prog.fn_info(**d).is_some_and(|f| f.impl_.is_none()))
            .map(|(_, d)| *d)
            .collect();
        ms.sort();
        let names: Vec<String> = match t {
            Ty::Adt(d, _) => self.prog.type_generics(*d).iter().map(|g| g.name.clone()).collect(),
            _ => Vec::new(),
        };
        ms.into_iter()
            .filter(|m| !self.prog.def(*m).name.starts_with("__"))
            .filter_map(|m| self.method_shape(m, &names))
            .collect()
    }

    /// The syntax of a template, kept when it was declared.
    fn template_ast(&self, def: DefId) -> Option<ast::FnDef> {
        self.template_asts.get(&def).cloned()
    }

    /// What reflection sees of `t`, written `written` in the unfolded code
    /// (an alias, added to `aliases`, whose nested types are aliases named
    /// after it); a field's type is looked into one level deep.
    pub(crate) fn shape(
        &self,
        t: &Ty,
        written: &str,
        depth: u32,
        aliases: &mut Vec<(String, Ty)>,
    ) -> TypeShape {
        aliases.push((written.to_string(), t.clone()));
        let prog = &self.prog;
        let name = prog.display_in(t, &self.shape_names);
        let base = prog.unrefine(t);
        let mut s = TypeShape::named(&name, written, Kind::Other);
        s.kind = match &base {
            Ty::Param(_) | Ty::Assoc(_) => Kind::Param,
            Ty::Int => Kind::Int,
            Ty::Float => Kind::Float,
            Ty::Bool => Kind::Bool,
            Ty::Char => Kind::Char,
            Ty::Str => Kind::Str,
            Ty::Tuple(..) => Kind::Tuple,
            Ty::Adt(d, a) => match prog.builtin(*d) {
                Some("String") => Kind::Str,
                Some("list") => {
                    s.elem = a
                        .first()
                        .map(|e| Box::new(self.shape(e, &format!("{written}_e"), depth + 1, aliases)));
                    Kind::List
                }
                _ if Some(*d) == prog.wk.option => {
                    s.elem = a
                        .first()
                        .map(|e| Box::new(self.shape(e, &format!("{written}_e"), depth + 1, aliases)));
                    Kind::Option
                }
                Some(_) => Kind::Other,
                None => match &prog.def(*d).kind {
                    DefKind::Struct(si) => {
                        if depth == 0 {
                            s.fields = si
                                .fields
                                .iter()
                                .enumerate()
                                .map(|(k, f)| {
                                    let ft = f.ty.subst(a);
                                    let alias = format!("{written}_f{k}");
                                    FieldShape {
                                        name: f.name.clone(),
                                        ty: Some(Box::new(self.shape(&ft, &alias, depth + 1, aliases))),
                                        default: f.default.is_some(),
                                    }
                                })
                                .collect();
                        }
                        Kind::Struct
                    }
                    DefKind::Enum(ei) | DefKind::ErrorSet(ei) => {
                        if depth == 0 {
                            s.variants = ei
                                .variants
                                .iter()
                                .enumerate()
                                .map(|(vi, v)| VariantShape {
                                    name: v.name.clone(),
                                    fields: v
                                        .fields
                                        .iter()
                                        .enumerate()
                                        .map(|(k, (n, ft))| {
                                            let ft = ft.subst(a);
                                            let alias = format!("{written}_v{vi}_{k}");
                                            (
                                                n.clone(),
                                                Some(Box::new(self.shape(&ft, &alias, depth + 1, aliases))),
                                            )
                                        })
                                        .collect(),
                                })
                                .collect();
                        }
                        Kind::Enum
                    }
                    _ => Kind::Other,
                },
            },
            _ => Kind::Other,
        };
        if let Ty::Adt(d, _) = t {
            s.attrs = self.attrs_of.get(d).cloned().unwrap_or_default();
        }
        s.impls = self.impl_names(t);
        if depth == 0 {
            s.methods = self.own_methods(t);
        }
        s
    }

    /// The names of the traits `t` has an impl of.
    fn impl_names(&self, t: &Ty) -> Vec<String> {
        let prog = &self.prog;
        let mut out = Vec::new();
        for i in &prog.impls {
            let Some(info) = prog.impl_info(*i) else { continue };
            let matches = match (&info.self_ty, t) {
                (Ty::Adt(a, _), Ty::Adt(b, _)) => a == b,
                (x, y) => x == y,
            };
            if matches {
                out.push(prog.def(info.trait_ref.def).name.clone());
            }
        }
        out.sort();
        out.dedup();
        out
    }
}
