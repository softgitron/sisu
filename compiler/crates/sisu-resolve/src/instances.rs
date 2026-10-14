//! Instances of templates (08, "Reflection"): a generic function that
//! reflects on its type parameters (`if comptime reflect.is_struct(T):`,
//! `comptime for f in reflect.fields_of(T):`, `value.[f]`) is made once for
//! each list of type arguments it is called with, by unfolding its body for
//! them (`template.rs`), and the instance is declared, checked and lowered
//! like any other function; a call of the template calls its instance.
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

/// Whether `f`'s body reflects on one of its type parameters: a `reflect`
/// query, a field reached by a compile-time descriptor, or a compile-time
/// expression that names a type parameter.
pub(crate) fn reflects(f: &ast::FnDef) -> bool {
    let Some(body) = &f.body else { return false };
    let names: Vec<&str> = f.generics.iter().map(|g| g.name.text.as_str()).collect();
    let mut found = false;
    let mentions = |e: &ast::Expr| {
        let mut m = false;
        ast::visit::expr(e, &mut |x| {
            if let ExprKind::Name(n) = &x.kind
                && names.contains(&n.as_str())
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
        _ => {}
    });
    found
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
            made += 1;
            if made > MAX_INSTANCES {
                let span = self.prog.def(*def).span;
                self.report(
                    Diagnostic::new(msg::COMPTIME_INSTANCES_UNBOUNDED, span)
                        .with("name", self.prog.def(*def).name.clone())
                        .with("count", MAX_INSTANCES.to_string()),
                );
                break;
            }
            self.declare_instance(*def, &info, args, *site);
        }
        if made > 0 {
            self.resolve_pending();
        }
    }

    fn declare_instance(&mut self, def: DefId, info: &FnInfo, args: &[Ty], site: sisu_diag::Span) {
        let module = self.prog.def(def).module;
        let name = self.prog.def(def).name.clone();
        let shown: Vec<String> = args.iter().map(|a| self.prog.display(a)).collect();
        let what = format!("{name}[{}]", shown.join(", "));
        let Some(ast) = self.template_ast(def) else { return };
        let source = self.sources.get(info.file).map(|f| f.text.clone()).unwrap_or_default();
        // Each type the unfolded code may name (an argument, a field's
        // type, an element type) is an alias in the instance's module.
        let mut aliases: Vec<(String, Ty)> = Vec::new();
        let params: Vec<(String, TypeShape)> = info
            .generics
            .iter()
            .zip(args)
            .enumerate()
            .map(|(i, (g, t))| (g.name.clone(), self.shape(t, &format!("__T{i}"), 0, &mut aliases)))
            .collect();
        let inst = format!("__instance_{}", self.prog.instances.len());
        let text = match crate::template::unfold_fn(&ast, &source, params, &inst) {
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
        if let Some(d) = self.prog.modules[m.0 as usize].items.get(&inst).copied() {
            self.prog.instances.insert((def, args.to_vec()), d);
            self.prog.instance_sites.insert(d, site);
            self.instance_of.insert(d, (def, args.to_vec()));
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
