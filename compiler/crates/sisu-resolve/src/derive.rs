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
use crate::ty::Ty;
use sisu_diag::{Diagnostic, Span, TextRange, msg};
use sisu_syntax::ast::{self, ExprKind, ItemKind};

/// The traits `@derive` knows, besides `Copy`.
pub const DERIVABLE: &[&str] = &["Debug", "Eq", "PartialOrd", "Ord", "Hash"];

/// A derived impl to check once every impl is known.
#[derive(Clone, Debug)]
pub struct DeriveCheck {
    pub ty: DefId,
    pub trait_name: String,
    pub span: Span,
}

/// The shape of a type, as the derive templates need it.
struct Shape {
    name: String,
    /// Generic parameter names and the source text of their bounds.
    generics: Vec<(String, Vec<String>)>,
    body: Body,
}

enum Body {
    /// Field names.
    Struct(Vec<String>),
    /// Variant names, each with its field names (`None` for positional).
    Enum(Vec<(String, Vec<Option<String>>)>),
}

impl Resolver<'_> {
    /// Declares the derived impls of the structs and enums among `items`.
    pub fn declare_derived(&mut self, module: ModuleId, items: &[ast::Item]) {
        let file = self.file(module);
        let source = self.sources.get(file).map(|f| f.text.clone()).unwrap_or_default();
        let mut text = String::new();
        let mut checks: Vec<(String, String, Span)> = Vec::new();
        for item in items {
            let (shape, builtin) = match &item.kind {
                ItemKind::Struct(s) => (
                    Shape {
                        name: s.name.text.clone(),
                        generics: generics_text(&s.generics, &source),
                        body: Body::Struct(s.fields.iter().map(|f| f.name.text.clone()).collect()),
                    },
                    item.attrs.iter().any(|a| a.name == "builtin"),
                ),
                ItemKind::Enum(e) => (
                    Shape {
                        name: e.name.text.clone(),
                        generics: generics_text(&e.generics, &source),
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
            if shape.name.is_empty() {
                continue;
            }
            let mut wanted: Vec<(String, Span)> = Vec::new();
            for a in item.attrs.iter().filter(|a| a.name == "derive") {
                for arg in &a.args {
                    let span = self.span(module, arg.range);
                    match &arg.value.kind {
                        ExprKind::Name(n) if n == "Copy" => {}
                        ExprKind::Name(n) if DERIVABLE.contains(&n.as_str()) => {
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
            // `Ord` brings `PartialOrd`, which it determines.
            if let Some((_, span)) = wanted.iter().find(|(w, _)| w == "Ord").cloned()
                && !wanted.iter().any(|(w, _)| w == "PartialOrd")
                && !has_impl(items, "PartialOrd", &shape.name)
            {
                wanted.push(("PartialOrd".into(), span));
            }
            // Every type has `Debug` (11): the derived one unless it has its own.
            if !builtin && !wanted.iter().any(|(w, _)| w == "Debug") && !has_impl(items, "Debug", &shape.name)
            {
                let span = self.span(module, TextRange::empty(item.range.start));
                wanted.push(("Debug".into(), span));
            }
            for (t, span) in wanted {
                text.push_str(&derive_text(&shape, &t));
                checks.push((shape.name.clone(), t, span));
            }
        }
        if text.is_empty() {
            return;
        }
        // The impls go in a module of their own, with the generated file as
        // its source and the type's module as its scope.
        let parent = &self.prog.modules[module.0 as usize];
        let name = format!("<derive>/{}.sisu", parent.name);
        let is_std = parent.is_std;
        let file = self.sources.add(name.clone(), &text);
        let parse = sisu_parser::parse(file, &text);
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
        for (ty, trait_name, span) in checks {
            if let Some(d) = self.prog.lookup_in(module, &ty) {
                self.derive_checks.push(DeriveCheck { ty: d, trait_name, span });
            }
        }
    }

    /// Each derived impl's fields implement its trait.
    pub fn check_derives(&mut self) {
        let checks = std::mem::take(&mut self.derive_checks);
        for c in checks {
            let Some(trait_def) = self.prog.lookup(self.prog.prelude, &c.trait_name) else { continue };
            let Some(imp) = self.prog.impls.iter().copied().find(|&i| {
                self.prog.impl_info(i).is_some_and(|ii| {
                    ii.derived
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
            let tr = TraitRef { def: trait_def, args: Vec::new() };
            for (fname, fty) in fields {
                if fty.contains_error() {
                    continue;
                }
                if matches!(self.prog.implements(&fty, &tr, &env), crate::solve::Answer::No) {
                    let names: Vec<String> = info.generics.iter().map(|g| g.name.clone()).collect();
                    self.report(
                        Diagnostic::new(msg::RESOLVE_DERIVE_FIELD, c.span)
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
            .map(|(n, bounds)| {
                let mut all = bounds.clone();
                all.push(trait_name.to_string());
                format!("{n}: {}", all.join(" + "))
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
