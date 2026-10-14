//! `@derive` (08, "Derive"; 18, Phase 4: the built-in derives). Stage 0
//! writes each derived impl as Sisu source, parses it, and declares it in
//! the module of the type, so it is checked and lowered like any other
//! impl; the structural impls follow fixed rules the interpreter's fast
//! paths reproduce (docs/architecture.md, "Traits and generics").
//!
//! Every struct and enum without an explicit `Debug` gets the derived one
//! (11, "Every type has `Debug`"). `@derive(Ord)` derives `PartialOrd`
//! too, from the same rule. `Copy` is a flag on the type, not an impl.
//! Whether each field implements the derived trait is checked once every
//! impl is known, and reported at the attribute.

use crate::collect::Resolver;
use crate::program::*;
use crate::template::{Body, Target};
use crate::ty::Ty;
use sisu_diag::{Diagnostic, Span, TextRange, msg};
use sisu_syntax::ast::{self, ExprKind, ItemKind};

/// The traits `@derive` knows, besides `Copy`.
pub const DERIVABLE: &[&str] = &["Debug", "Eq", "PartialOrd", "Ord", "Hash"];

/// The traits `@delegate` forwards (03, "Coherence": "cheap newtypes and
/// delegation"); the first six return `Self`.
pub const DELEGABLE: &[&str] =
    &["Add", "Sub", "Mul", "Div", "Rem", "Neg", "Eq", "PartialOrd", "Ord", "Hash", "Display", "Debug", "Len"];

/// A derived impl to check once every impl is known.
#[derive(Clone, Debug)]
pub struct DeriveCheck {
    pub ty: DefId,
    pub trait_name: String,
    pub span: Span,
    /// The field a delegated impl forwards to; `None` for a derived impl,
    /// which uses every field.
    pub field: Option<String>,
}

/// The shape of a type, as the derive templates need it.
struct Shape {
    name: String,
    /// Generic parameter names and the source text of their bounds.
    generics: Vec<(String, Vec<String>)>,
    /// Whether each parameter appears in a field's type: one that does not
    /// (`Key[T]`'s `T`) needs no bound for the derived impl.
    used: Vec<bool>,
    body: Body,
}


impl Resolver<'_> {
    /// Declares the derived impls of the structs and enums among `items`.
    pub fn declare_derived(&mut self, module: ModuleId, items: &[ast::Item]) {
        let file = self.file(module);
        let source = self.sources.get(file).map(|f| f.text.clone()).unwrap_or_default();
        let mut text = String::new();
        let mut delegate = String::new();
        let mut checks: Vec<(String, String, Span, Option<String>)> = Vec::new();
        for item in items {
            let (shape, builtin) = match &item.kind {
                ItemKind::Struct(s) => (
                    Shape {
                        name: s.name.text.clone(),
                        generics: generics_text(&s.generics, &source),
                        used: used_params(&s.generics, s.fields.iter().map(|f| &f.ty)),
                        body: Body::Struct(s.fields.iter().map(|f| f.name.text.clone()).collect()),
                    },
                    item.attrs.iter().any(|a| a.name == "builtin"),
                ),
                ItemKind::Enum(e) => (
                    Shape {
                        name: e.name.text.clone(),
                        generics: generics_text(&e.generics, &source),
                        used: used_params(
                            &e.generics,
                            e.variants.iter().flat_map(|v| v.fields.iter().map(|f| &f.ty)),
                        ),
                        body: Body::Enum(
                            e.variants
                                .iter()
                                .map(|v| {
                                    (
                                        v.name.text.clone(),
                                        v.fields
                                            .iter()
                                            .map(|f| f.name.as_ref().map(|n| n.text.clone()))
                                            .collect(),
                                    )
                                })
                                .collect(),
                        ),
                    },
                    false,
                ),
                _ => continue,
            };
            // A type with a `const` parameter (E0900) gets no impls.
            let has_const = match &item.kind {
                ItemKind::Struct(s) => s.generics.iter().any(|g| g.is_const),
                ItemKind::Enum(e) => e.generics.iter().any(|g| g.is_const),
                _ => false,
            };
            if shape.name.is_empty() || has_const {
                continue;
            }
            let mut wanted: Vec<(String, Span)> = Vec::new();
            for a in item.attrs.iter().filter(|a| a.name == "derive") {
                for arg in &a.args {
                    let span = self.span(module, arg.range);
                    match &arg.value.kind {
                        // A flag on the type, checked against its fields.
                        ExprKind::Name(n) if n == "Copy" => {
                            checks.push((shape.name.clone(), "Copy".into(), span, None));
                        }
                        ExprKind::Name(n)
                            if DERIVABLE.contains(&n.as_str()) || self.derive_template(module, n).is_some() =>
                        {
                            if !wanted.iter().any(|(w, _)| w == n) {
                                wanted.push((n.clone(), span));
                            }
                        }
                        _ => {
                            let name = self.source_of(span);
                            self.report(Diagnostic::new(msg::RESOLVE_NOT_DERIVABLE, span).with("name", name));
                        }
                    }
                }
            }
            // `invariant e` lines: the check D-077 runs when unwinding drops
            // a guard of a `Mutex` holding the type (the other checks of 07
            // arrive with contracts, Phase 8).
            if let ItemKind::Struct(st) = &item.kind
                && !st.invariants.is_empty()
            {
                text.push_str(&invariant_text(&shape, &st.invariants, &source));
            }
            // `@delegate(Trait, ..., to="field")`: impls that forward to a field.
            let delegated = self.delegations(module, item, &shape);
            for (t, field, span) in &delegated {
                delegate.push_str(&delegate_text(&shape, t, field));
                checks.push((shape.name.clone(), t.clone(), *span, Some(field.clone())));
            }
            // `Ord` brings `PartialOrd`, which it determines.
            if let Some((_, span)) = wanted.iter().find(|(w, _)| w == "Ord").cloned()
                && !wanted.iter().any(|(w, _)| w == "PartialOrd")
                && !has_impl(items, "PartialOrd", &shape.name)
            {
                wanted.push(("PartialOrd".into(), span));
            }
            // Every type has `Debug` (11): the derived one unless it has its own.
            if !builtin
                && !wanted.iter().any(|(w, _)| w == "Debug")
                && !delegated.iter().any(|(w, _, _)| w == "Debug")
                && !has_impl(items, "Debug", &shape.name)
            {
                let span = self.span(module, TextRange::empty(item.range.start));
                wanted.push(("Debug".into(), span));
            }
            for (t, span) in wanted {
                match self.derive_template(module, &t) {
                    // The trait's own derive, run at compile time.
                    Some((tm, template)) => {
                        let (head, written) = header(&shape, &t);
                        let target = Target { name: &shape.name, written: &written, body: &shape.body };
                        let source = self.sources.get(self.file(tm)).map(|f| f.text.clone()).unwrap_or_default();
                        match crate::template::unfold(&template, &source, &head, &target) {
                            Ok(impl_text) => text.push_str(&impl_text),
                            Err((range, reason)) => {
                                let at = self.span(tm, range);
                                self.report(
                                    Diagnostic::new(msg::COMPTIME_DERIVE_FAILED, span)
                                        .with("trait", t.clone())
                                        .with("ty", shape.name.clone())
                                        .with("reason", reason)
                                        .label(at, msg::COMPTIME_DERIVE_HERE, Vec::new()),
                                );
                                continue;
                            }
                        }
                    }
                    None => text.push_str(&derive_text(&shape, &t)),
                }
                checks.push((shape.name.clone(), t, span, None));
            }
        }
        // Delegated impls are ordinary impls, written by the compiler.
        if !delegate.is_empty() {
            self.declare_generated(module, "<delegate>", &delegate);
        }
        if !text.is_empty() {
            self.declare_generated(module, "<derive>", &text);
        }
        for (ty, trait_name, span, field) in checks {
            if let Some(d) = self.prog.lookup_in(module, &ty) {
                self.derive_checks.push(DeriveCheck { ty: d, trait_name, span, field });
            }
        }
    }

    /// The `comptime def derive` of the trait `name` as `module` sees it:
    /// its own trait of that name, or the prelude's; with the module the
    /// template is written in.
    fn derive_template(&self, module: ModuleId, name: &str) -> Option<(ModuleId, ast::FnDef)> {
        [module, self.prog.prelude]
            .into_iter()
            .find_map(|m| self.derive_templates.get(&(m, name.to_string())).map(|f| (m, f.clone())))
    }

    /// Declares generated impls in a module of their own, with the
    /// generated text as its source file and `module` as its scope.
    fn declare_generated(&mut self, module: ModuleId, dir: &str, text: &str) {
        let parent = &self.prog.modules[module.0 as usize];
        let name = format!("{dir}/{}.sisu", parent.name);
        let is_std = parent.is_std;
        let file = self.sources.add(name.clone(), text);
        let parse = sisu_parser::parse(file, text);
        self.diags.extend(parse.diagnostics);
        let ast = ast::build::module(&parse.tree);
        let id = ModuleId(self.prog.modules.len() as u32);
        self.prog.modules.push(ModuleData {
            name,
            file,
            items: Default::default(),
            order: Vec::new(),
            is_std,
            parent: Some(module),
        });
        self.declare_items(id, &ast.items, "");
    }

    /// The traits a struct's `@delegate` forwards, with the field, after
    /// checking the attribute.
    fn delegations(
        &mut self,
        module: ModuleId,
        item: &ast::Item,
        shape: &Shape,
    ) -> Vec<(String, String, Span)> {
        let mut out = Vec::new();
        for a in item.attrs.iter().filter(|a| a.name == "delegate") {
            let attr_span = self.span(module, a.range);
            let to = a.args.iter().find(|arg| arg.name.as_ref().is_some_and(|n| n.text == "to"));
            let field = to.and_then(|arg| match &arg.value.kind {
                ExprKind::Str(parts) => Some(ast::build::plain_text(parts)),
                _ => None,
            });
            let fields = match &shape.body {
                Body::Struct(fields) => fields.clone(),
                Body::Enum(_) => Vec::new(),
            };
            let Some(field) = field.filter(|f| fields.contains(f)) else {
                let span = to.map(|arg| self.span(module, arg.range)).unwrap_or(attr_span);
                self.report(Diagnostic::new(msg::RESOLVE_DELEGATE_TO, span).with("ty", shape.name.clone()));
                continue;
            };
            for arg in a.args.iter().filter(|arg| arg.name.is_none()) {
                let span = self.span(module, arg.range);
                match &arg.value.kind {
                    ExprKind::Name(n) if DELEGABLE.contains(&n.as_str()) => {
                        let returns_self = DELEGABLE[..6].contains(&n.as_str());
                        if returns_self && fields.len() != 1 {
                            self.report(
                                Diagnostic::new(msg::RESOLVE_DELEGATE_SELF, span)
                                    .with("trait", n.clone())
                                    .with("ty", shape.name.clone())
                                    .with("field", field.clone()),
                            );
                            continue;
                        }
                        if !out.iter().any(|(t, _, _): &(String, String, Span)| t == n) {
                            out.push((n.clone(), field.clone(), span));
                        }
                    }
                    _ => {
                        let name = self.source_of(span);
                        self.report(Diagnostic::new(msg::RESOLVE_NOT_DELEGABLE, span).with("name", name));
                    }
                }
            }
        }
        out
    }

    /// Each derived impl's fields implement its trait.
    pub fn check_derives(&mut self) {
        let checks = std::mem::take(&mut self.derive_checks);
        for c in checks {
            if c.trait_name == "Copy" {
                self.check_copy_fields(&c);
                continue;
            }
            let home = self.prog.def(c.ty).module;
            let Some(trait_def) =
                self.prog.lookup(home, &c.trait_name).or_else(|| self.prog.lookup(self.prog.prelude, &c.trait_name))
            else {
                continue;
            };
            // A derive of the language's traits needs the trait of every
            // field; another trait's derive needs what its code uses, which
            // checking the derived impl finds.
            let structural = DERIVABLE.contains(&c.trait_name.as_str()) || c.field.is_some();
            let generated = |prog: &Program, i: DefId| {
                let m = &prog.modules[prog.def(i).module.0 as usize].name;
                m.starts_with("<derive>/") || m.starts_with("<delegate>/")
            };
            let Some(imp) = self.prog.impls.iter().copied().find(|&i| {
                self.prog.impl_info(i).is_some_and(|ii| {
                    generated(&self.prog, i)
                        && ii.trait_ref.def == trait_def
                        && matches!(&ii.self_ty, Ty::Adt(d, _) if *d == c.ty)
                })
            }) else {
                continue;
            };
            let Some(info) = self.prog.impl_info(imp).cloned() else { continue };
            let env = self.prog.elaborate_env(&info.predicates);
            let fields: Vec<(String, Ty)> = match &self.prog.def(c.ty).kind {
                DefKind::Struct(s) => s.fields.iter().map(|f| (f.name.clone(), f.ty.clone())).collect(),
                DefKind::Enum(e) => e
                    .variants
                    .iter()
                    .flat_map(|v| {
                        v.fields.iter().enumerate().map(move |(i, (n, t))| {
                            (format!("{}.{}", v.name, n.clone().unwrap_or_else(|| i.to_string())), t.clone())
                        })
                    })
                    .collect(),
                _ => Vec::new(),
            };
            let fields: Vec<(String, Ty)> = match &c.field {
                Some(f) => fields.into_iter().filter(|(n, _)| n == f).collect(),
                None => fields,
            };
            let tr = TraitRef { def: trait_def, args: Vec::new() };
            // The trait's supertraits, which the type implements itself.
            let self_ty = info.self_ty.clone();
            let supers = self.prog.trait_info(trait_def).map(|i| i.supertraits.clone()).unwrap_or_default();
            for st in supers {
                if matches!(self.prog.implements(&self_ty, &st, &env), crate::solve::Answer::No) {
                    let names: Vec<String> = info.generics.iter().map(|g| g.name.clone()).collect();
                    self.report(
                        Diagnostic::new(msg::RESOLVE_SUPERTRAIT_MISSING, c.span)
                            .with("ty", self.prog.display_in(&self_ty, &names))
                            .with("trait", c.trait_name.clone())
                            .with("super", self.prog.display_trait(&st)),
                    );
                }
            }
            for (fname, fty) in fields {
                if fty.contains_error() || !structural {
                    continue;
                }
                if matches!(self.prog.implements(&fty, &tr, &env), crate::solve::Answer::No) {
                    let names: Vec<String> = info.generics.iter().map(|g| g.name.clone()).collect();
                    let id = if c.field.is_some() {
                        msg::RESOLVE_DELEGATE_FIELD
                    } else {
                        msg::RESOLVE_DERIVE_FIELD
                    };
                    self.report(
                        Diagnostic::new(id, c.span)
                            .with("ty", self.prog.def(c.ty).name.clone())
                            .with("trait", c.trait_name.clone())
                            .with("field", fname)
                            .with("fty", self.prog.display_in(&fty, &names)),
                    );
                    // The generated body would only repeat the error.
                    for (_, m) in &info.methods {
                        if let DefKind::Fn(f) = &mut self.prog.defs[m.0 as usize].kind {
                            f.body = None;
                            f.body_broken = true;
                        }
                    }
                    break;
                }
            }
        }
    }
}

impl Resolver<'_> {
    /// `@derive(Copy)` only for a type whose fields are all `Copy` (03,
    /// "Auto traits"); a field of a type parameter's type is, since the
    /// type is `Copy` only for `Copy` arguments.
    fn check_copy_fields(&mut self, c: &DeriveCheck) {
        let fields: Vec<(String, Ty)> = match &self.prog.def(c.ty).kind {
            DefKind::Struct(s) => s.fields.iter().map(|f| (f.name.clone(), f.ty.clone())).collect(),
            DefKind::Enum(e) => e
                .variants
                .iter()
                .flat_map(|v| {
                    v.fields.iter().enumerate().map(move |(i, (n, t))| {
                        (format!("{}.{}", v.name, n.clone().unwrap_or_else(|| i.to_string())), t.clone())
                    })
                })
                .collect(),
            _ => return,
        };
        let generics: Vec<String> = self.prog.type_generics(c.ty).iter().map(|g| g.name.clone()).collect();
        // Each parameter stands for a `Copy` type here.
        let as_copy: Vec<Ty> = (0..generics.len()).map(|_| Ty::Int).collect();
        for (fname, fty) in fields {
            if fty.contains_error() || self.prog.is_copy(&fty.subst(&as_copy)) {
                continue;
            }
            self.report(
                Diagnostic::new(msg::RESOLVE_DERIVE_FIELD, c.span)
                    .with("ty", self.prog.def(c.ty).name.clone())
                    .with("trait", "Copy")
                    .with("field", fname)
                    .with("fty", self.prog.display_in(&fty, &generics)),
            );
            if let DefKind::Struct(s) = &mut self.prog.defs[c.ty.0 as usize].kind {
                s.copy = false;
            } else if let DefKind::Enum(e) = &mut self.prog.defs[c.ty.0 as usize].kind {
                e.copy = false;
            }
            return;
        }
    }
}

/// Whether the module has its own `impl Trait for Type`.
fn has_impl(items: &[ast::Item], trait_name: &str, ty: &str) -> bool {
    items.iter().any(|it| match &it.kind {
        ItemKind::Impl(i) => {
            let names = |t: &Option<ast::TypeExpr>| match t.as_ref().map(|t| &t.kind) {
                Some(ast::TypeKind::Path { segments, .. }) => segments.last().map(|s| s.text.clone()),
                _ => None,
            };
            names(&i.trait_ty).as_deref() == Some(trait_name)
                && names(&Some(i.self_ty.clone())).as_deref() == Some(ty)
        }
        _ => false,
    })
}

/// Whether each generic parameter is named in one of the types.
fn used_params<'a>(gps: &[ast::GenericParam], tys: impl Iterator<Item = &'a ast::TypeExpr>) -> Vec<bool> {
    fn walk(t: &ast::TypeExpr, names: &mut Vec<String>) {
        match &t.kind {
            ast::TypeKind::Path { segments, args } => {
                if let Some(s) = segments.first() {
                    names.push(s.text.clone());
                }
                args.iter().for_each(|a| walk(a, names));
            }
            ast::TypeKind::Option(i) | ast::TypeKind::Dyn(i) => walk(i, names),
            ast::TypeKind::Ref { inner, .. } => walk(inner, names),
            ast::TypeKind::Result { ok, err } => {
                if let Some(o) = ok {
                    walk(o, names);
                }
                if let Some(e) = err {
                    walk(e, names);
                }
            }
            ast::TypeKind::Tuple(es) => es.iter().for_each(|(_, e)| walk(e, names)),
            ast::TypeKind::Fn { params, ret, .. } => {
                params.iter().for_each(|p| walk(p, names));
                walk(ret, names);
            }
            ast::TypeKind::Union(us) => us.iter().for_each(|u| walk(u, names)),
            _ => {}
        }
    }
    let mut names = Vec::new();
    for t in tys {
        walk(t, &mut names);
    }
    gps.iter().map(|g| names.contains(&g.name.text)).collect()
}

fn generics_text(gps: &[ast::GenericParam], source: &str) -> Vec<(String, Vec<String>)> {
    gps.iter()
        .map(|g| {
            let bounds = g
                .bounds
                .iter()
                .map(|b| source.get(b.range.start as usize..b.range.end as usize).unwrap_or("").to_string())
                .filter(|b| !b.is_empty())
                .collect();
            (g.name.text.clone(), bounds)
        })
        .collect()
}

/// `impl[A: Eq] Eq for P[A]:` and the self type `P[A]`.
fn header(shape: &Shape, trait_name: &str) -> (String, String) {
    let ty = if shape.generics.is_empty() {
        shape.name.clone()
    } else {
        format!(
            "{}[{}]",
            shape.name,
            shape.generics.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>().join(", ")
        )
    };
    let params = if shape.generics.is_empty() {
        String::new()
    } else {
        let ps: Vec<String> = shape
            .generics
            .iter()
            .enumerate()
            .map(|(i, (n, bounds))| {
                let mut all = bounds.clone();
                if shape.used.get(i).copied().unwrap_or(true) {
                    all.push(trait_name.to_string());
                }
                if all.is_empty() { n.clone() } else { format!("{n}: {}", all.join(" + ")) }
            })
            .collect();
        format!("[{}]", ps.join(", "))
    };
    (format!("impl{params} {trait_name} for {ty}:\n"), ty)
}

fn quote(s: &str) -> String {
    let mut q = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => q.push_str("\\\""),
            '\\' => q.push_str("\\\\"),
            '{' => q.push_str("{{"),
            '}' => q.push_str("}}"),
            c => q.push(c),
        }
    }
    q.push('"');
    q
}

/// The pattern for a variant with its fields bound to `prefix0, prefix1, ...`.
fn variant_pattern(name: &str, fields: &[Option<String>], prefix: &str) -> String {
    if fields.is_empty() {
        name.to_string()
    } else {
        let bs: Vec<String> = (0..fields.len()).map(|i| format!("{prefix}{i}")).collect();
        format!("{name}({})", bs.join(", "))
    }
}

/// The source of one derived impl.
fn derive_text(shape: &Shape, trait_name: &str) -> String {
    let (head, ty) = header(shape, trait_name);
    let mut out = head;
    let w = |out: &mut String, depth: usize, line: &str| {
        for _ in 0..depth {
            out.push_str("    ");
        }
        out.push_str(line);
        out.push('\n');
    };
    match (trait_name, &shape.body) {
        ("Debug", Body::Struct(fields)) => {
            w(&mut out, 1, "pure def fmt_debug(self, mut f: Formatter):");
            w(&mut out, 2, &format!("f.write({})", quote(&format!("{}(", shape.name))));
            w(&mut out, 2, "f.depth += 1");
            for (i, fname) in fields.iter().enumerate() {
                let sep = if i == 0 { String::new() } else { ", ".to_string() };
                w(&mut out, 2, &format!("f.write({})", quote(&format!("{sep}{fname}="))));
                w(&mut out, 2, &format!("self.{fname}.fmt_debug(mut f)"));
            }
            w(&mut out, 2, "f.depth -= 1");
            w(&mut out, 2, "f.write(\")\")");
        }
        ("Debug", Body::Enum(variants)) => {
            w(&mut out, 1, "pure def fmt_debug(self, mut f: Formatter):");
            if variants.is_empty() {
                w(&mut out, 2, "pass");
            } else {
                w(&mut out, 2, "match self:");
                for (v, fields) in variants {
                    w(&mut out, 3, &format!("case {}:", variant_pattern(v, fields, "a")));
                    if fields.is_empty() {
                        w(&mut out, 4, &format!("f.write({})", quote(v)));
                        continue;
                    }
                    w(&mut out, 4, &format!("f.write({})", quote(&format!("{v}("))));
                    w(&mut out, 4, "f.depth += 1");
                    for (i, fname) in fields.iter().enumerate() {
                        let sep = if i == 0 { "" } else { ", " };
                        let label = match fname {
                            Some(n) => format!("{sep}{n}="),
                            None => sep.to_string(),
                        };
                        if !label.is_empty() {
                            w(&mut out, 4, &format!("f.write({})", quote(&label)));
                        }
                        w(&mut out, 4, &format!("a{i}.fmt_debug(mut f)"));
                    }
                    w(&mut out, 4, "f.depth -= 1");
                    w(&mut out, 4, "f.write(\")\")");
                }
            }
        }
        ("Eq", Body::Struct(fields)) => {
            w(&mut out, 1, &format!("pure def eq(self, other: {ty}) -> bool:"));
            if fields.is_empty() {
                w(&mut out, 2, "return True");
            } else {
                let parts: Vec<String> = fields.iter().map(|f| format!("self.{f} == other.{f}")).collect();
                w(&mut out, 2, &format!("return {}", parts.join(" and ")));
            }
        }
        ("Eq", Body::Enum(variants)) => {
            w(&mut out, 1, &format!("pure def eq(self, other: {ty}) -> bool:"));
            enum_pairs(&mut out, variants, "return True", "return False", |out, fields| {
                let parts: Vec<String> = (0..fields.len()).map(|i| format!("a{i} == b{i}")).collect();
                let mut s = String::new();
                for _ in 0..6 {
                    s.push_str("    ");
                }
                out.push_str(&format!("{s}return {}\n", parts.join(" and ")));
            });
        }
        ("Hash", Body::Struct(fields)) => {
            w(&mut out, 1, "pure def hash(self, mut h: Hasher):");
            if fields.is_empty() {
                w(&mut out, 2, "pass");
            }
            for f in fields {
                w(&mut out, 2, &format!("self.{f}.hash(mut h)"));
            }
        }
        ("Hash", Body::Enum(variants)) => {
            w(&mut out, 1, "pure def hash(self, mut h: Hasher):");
            if variants.is_empty() {
                w(&mut out, 2, "pass");
            } else {
                w(&mut out, 2, "match self:");
                for (k, (v, fields)) in variants.iter().enumerate() {
                    w(&mut out, 3, &format!("case {}:", variant_pattern(v, fields, "a")));
                    w(&mut out, 4, &format!("h.write_int({k})"));
                    for i in 0..fields.len() {
                        w(&mut out, 4, &format!("a{i}.hash(mut h)"));
                    }
                }
            }
        }
        ("PartialOrd", Body::Struct(fields)) => {
            w(&mut out, 1, &format!("pure def partial_cmp(self, other: {ty}) -> Ordering?:"));
            for (i, f) in fields.iter().enumerate() {
                if i + 1 == fields.len() {
                    w(&mut out, 2, &format!("return self.{f}.partial_cmp(other.{f})"));
                } else {
                    w(&mut out, 2, &format!("let c{i} = self.{f}.partial_cmp(other.{f})"));
                    w(&mut out, 2, &format!("if c{i} != Some(Equal):"));
                    w(&mut out, 3, &format!("return c{i}"));
                }
            }
            if fields.is_empty() {
                w(&mut out, 2, "return Some(Equal)");
            }
        }
        ("PartialOrd", Body::Enum(variants)) => {
            w(&mut out, 1, &format!("pure def partial_cmp(self, other: {ty}) -> Ordering?:"));
            enum_ordering(&mut out, variants, "partial_cmp", true);
        }
        ("Ord", Body::Struct(fields)) => {
            w(&mut out, 1, &format!("pure def cmp(self, other: {ty}) -> Ordering:"));
            for (i, f) in fields.iter().enumerate() {
                if i + 1 == fields.len() {
                    w(&mut out, 2, &format!("return self.{f}.cmp(other.{f})"));
                } else {
                    w(&mut out, 2, &format!("let c{i} = self.{f}.cmp(other.{f})"));
                    w(&mut out, 2, &format!("if c{i} != Equal:"));
                    w(&mut out, 3, &format!("return c{i}"));
                }
            }
            if fields.is_empty() {
                w(&mut out, 2, "return Equal");
            }
        }
        ("Ord", Body::Enum(variants)) => {
            w(&mut out, 1, &format!("pure def cmp(self, other: {ty}) -> Ordering:"));
            enum_ordering(&mut out, variants, "cmp", false);
        }
        _ => {}
    }
    out
}

/// `match self:` with, per variant, `match other:` on the same variant
/// (bodies from `same`) and every other variant (`differ`).
fn enum_pairs(
    out: &mut String,
    variants: &[(String, Vec<Option<String>>)],
    unit_same: &str,
    differ: &str,
    same: impl Fn(&mut String, &[Option<String>]),
) {
    let ind = |n: usize| "    ".repeat(n);
    if variants.is_empty() {
        out.push_str(&format!("{}{unit_same}\n", ind(2)));
        return;
    }
    out.push_str(&format!("{}match self:\n", ind(2)));
    for (v, fields) in variants {
        out.push_str(&format!("{}case {}:\n", ind(3), variant_pattern(v, fields, "a")));
        out.push_str(&format!("{}match other:\n", ind(4)));
        out.push_str(&format!("{}case {}:\n", ind(5), variant_pattern(v, fields, "b")));
        if fields.is_empty() {
            out.push_str(&format!("{}{unit_same}\n", ind(6)));
        } else {
            same(out, fields);
        }
        if variants.len() > 1 {
            out.push_str(&format!("{}case _:\n", ind(5)));
            out.push_str(&format!("{}{differ}\n", ind(6)));
        }
    }
}

/// The derived `cmp` or `partial_cmp` of an enum: variants in declaration
/// order, then fields in order.
fn enum_ordering(out: &mut String, variants: &[(String, Vec<Option<String>>)], method: &str, partial: bool) {
    let ind = |n: usize| "    ".repeat(n);
    let equal = if partial { "Some(Equal)" } else { "Equal" };
    if variants.is_empty() {
        out.push_str(&format!("{}return {equal}\n", ind(2)));
        return;
    }
    // The index of `other`'s variant.
    out.push_str(&format!("{}let j = match other:\n", ind(2)));
    for (k, (v, fields)) in variants.iter().enumerate() {
        let pat = if fields.is_empty() { v.clone() } else { format!("{v}(..)") };
        out.push_str(&format!("{}case {pat}: {k}\n", ind(3)));
    }
    out.push_str(&format!("{}match self:\n", ind(2)));
    for (k, (v, fields)) in variants.iter().enumerate() {
        out.push_str(&format!("{}case {}:\n", ind(3), variant_pattern(v, fields, "a")));
        out.push_str(&format!("{}let i = {k}\n", ind(4)));
        let differ =
            if partial { "return Some(i.cmp(j))".to_string() } else { "return i.cmp(j)".to_string() };
        if variants.len() > 1 {
            out.push_str(&format!("{}if i != j:\n", ind(4)));
            out.push_str(&format!("{}{differ}\n", ind(5)));
        }
        if fields.is_empty() {
            out.push_str(&format!("{}return {equal}\n", ind(4)));
            continue;
        }
        out.push_str(&format!("{}match other:\n", ind(4)));
        out.push_str(&format!("{}case {}:\n", ind(5), variant_pattern(v, fields, "b")));
        for i in 0..fields.len() {
            if i + 1 == fields.len() {
                out.push_str(&format!("{}return a{i}.{method}(b{i})\n", ind(6)));
            } else {
                out.push_str(&format!("{}let c{i} = a{i}.{method}(b{i})\n", ind(6)));
                out.push_str(&format!("{}if c{i} != {equal}:\n", ind(6)));
                out.push_str(&format!("{}return c{i}\n", ind(7)));
            }
        }
        if variants.len() > 1 {
            out.push_str(&format!("{}case _:\n", ind(5)));
            out.push_str(&format!("{}return {equal}\n", ind(6)));
        }
    }
}

/// The source of one delegated impl: the trait's method on the field, the
/// result wrapped back into the type when the trait returns `Self`.
fn delegate_text(shape: &Shape, trait_name: &str, field: &str) -> String {
    let (head, ty) = header(shape, trait_name);
    let name = &shape.name;
    let v = field;
    let op = |sym: &str, method: &str| {
        format!(
            "    pure def {method}(self, other: {ty}) -> {ty}:\n        return {name}({v}=self.{v} {sym} other.{v})\n"
        )
    };
    let body = match trait_name {
        "Add" => op("+", "add"),
        "Sub" => op("-", "sub"),
        "Mul" => op("*", "mul"),
        "Div" => op("/", "div"),
        "Rem" => op("%", "rem"),
        "Neg" => format!("    pure def neg(self) -> {ty}:\n        return {name}({v}=-self.{v})\n"),
        "Eq" => {
            format!("    pure def eq(self, other: {ty}) -> bool:\n        return self.{v} == other.{v}\n")
        }
        "PartialOrd" => format!(
            "    pure def partial_cmp(self, other: {ty}) -> Ordering?:\n        return self.{v}.partial_cmp(other.{v})\n"
        ),
        "Ord" => format!(
            "    pure def cmp(self, other: {ty}) -> Ordering:\n        return self.{v}.cmp(other.{v})\n"
        ),
        "Hash" => format!("    pure def hash(self, mut h: Hasher):\n        self.{v}.hash(mut h)\n"),
        "Display" => format!("    pure def fmt(self, mut f: Formatter):\n        self.{v}.fmt(mut f)\n"),
        "Debug" => {
            format!("    pure def fmt_debug(self, mut f: Formatter):\n        self.{v}.fmt_debug(mut f)\n")
        }
        _ => format!("    pure def len(self) -> int:\n        return len(self.{v})\n"),
    };
    format!("\n{head}{body}")
}

/// `impl Type: pure def __invariant(self) -> bool`, the conjunction of the
/// type's `invariant` lines.
fn invariant_text(shape: &Shape, invariants: &[ast::Expr], source: &str) -> String {
    let (ty, params) = if shape.generics.is_empty() {
        (shape.name.clone(), String::new())
    } else {
        let names: Vec<String> = shape.generics.iter().map(|(n, _)| n.clone()).collect();
        let ps: Vec<String> = shape
            .generics
            .iter()
            .map(|(n, b)| if b.is_empty() { n.clone() } else { format!("{n}: {}", b.join(" + ")) })
            .collect();
        (format!("{}[{}]", shape.name, names.join(", ")), format!("[{}]", ps.join(", ")))
    };
    let parts: Vec<String> = invariants
        .iter()
        .map(|e| format!("({})", source.get(e.range.start as usize..e.range.end as usize).unwrap_or("True")))
        .collect();
    format!(
        "\nimpl{params} {ty}:\n    pure def __invariant(self) -> bool:\n        return {}\n",
        parts.join(" and ")
    )
}
