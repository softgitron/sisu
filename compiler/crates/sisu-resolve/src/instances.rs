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
use crate::template::{FieldShape, Kind, TypeShape, VariantShape};
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
fn comptime_stmts(b: &ast::Block) -> bool {
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
            if !info.template || info.generics.len() != args.len() {
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
        // compile-time code names `Self` (decision 244).
        let within = info.owner.as_ref().map(|t| {
            let self_ty = t.subst(args);
            params.push(("Self".to_string(), self.shape(&self_ty, "__S", 0, &mut aliases)));
            "__S"
        });
        // The program's types its compile-time code names (`reflect.fields_of(Point)`).
        let types: Vec<(String, TypeShape)> = named_types(&ast)
            .into_iter()
            .filter(|n| !params.iter().any(|(p, _)| p == n))
            .filter_map(|n| self.named_type(module, &n).map(|t| (n, t)))
            .enumerate()
            .map(|(i, (n, t))| {
                let shape = self.shape(&t, &format!("__C{i}"), 0, &mut aliases);
                (n, shape)
            })
            .collect();
        let inst = format!("__instance_{}", self.instances_declared);
        let text = match crate::template::unfold_fn(&ast, &source, params, types, &inst, within) {
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
        if let DefKind::Fn(f) = &mut self.prog.defs[d.0 as usize].kind {
            f.instance_of = Some((template, args.clone()));
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
        let name = prog.display(t);
        let base = prog.unrefine(t);
        let mut s = TypeShape::named(&name, written, Kind::Other);
        s.kind = match &base {
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
