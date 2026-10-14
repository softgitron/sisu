//! Generators for property tests (17, "Property-based testing"; D-070),
//! written by the compiler as Sisu source and declared beside the module
//! they serve (`<arbitrary>/main.sisu`), so they are checked and lowered
//! like any code:
//!
//! - `__arbitrary_T` for each struct, enum and refinement type of the root
//!   module whose parts all have a generator: a struct's fields in order,
//!   then its invariant (a case that breaks it is refused); an enum's
//!   variant, then its fields; a refinement's base, then its predicate (an
//!   integer refinement whose predicate bounds `self` draws in that range,
//!   which is what lets `exhaustive[T]` list a finite domain).
//! - `__arbitrary_site_N` for each type written in `arbitrary[T]` or
//!   `exhaustive[T](...)`, which the checker puts there.
//! - for each function with a contract whose parameters all have a
//!   generator, its cases (refused unless `requires` holds) and a test,
//!   `contract of NAME`, that calls it on them: `ensures` and the
//!   invariants are checked by the call (07: contracts as properties).
//!
//! A type parameter of a generic type is given by a generator parameter;
//! one of a function is `int`, when every bound is one `int` meets.
//! The generators are built from `Draws`'s (`std/prelude.sisu`).

use crate::collect::Resolver;
use crate::program::*;
use sisu_diag::{Diagnostic, msg};
use sisu_syntax::ast::{self, ExprKind, ItemKind, TypeKind};
use std::collections::{HashMap, HashSet};

/// A variant's name and its fields, named or not.
type VariantShape = (String, Vec<(Option<String>, ast::TypeExpr)>);

/// What the generator writer knows of a type of the module.
enum Shape {
    Struct { generics: Vec<String>, fields: Vec<(String, ast::TypeExpr)>, invariant: bool },
    Enum { generics: Vec<String>, variants: Vec<VariantShape> },
    Refinement { generics: Vec<String>, base: ast::TypeExpr, domain: Option<(i64, i64)> },
}

impl Shape {
    fn generics(&self) -> &[String] {
        match self {
            Shape::Struct { generics, .. }
            | Shape::Enum { generics, .. }
            | Shape::Refinement { generics, .. } => generics,
        }
    }
}

/// The bounds `int` meets, which a generic function's parameter may have
/// for its contract to be tested with `int` for it.
const INT_MEETS: &[&str] =
    &["Ord", "PartialOrd", "Eq", "Hash", "Display", "Debug", "Copy", "Num", "Send", "Sync"];

struct Writer<'a> {
    shapes: &'a HashMap<String, Shape>,
    /// Which shapes have a generator (decided once for all).
    ok: HashMap<String, bool>,
    out: String,
}

impl Writer<'_> {
    /// Whether every part of a type has a generator; `params` are the type
    /// parameters in scope (each given by a generator parameter).
    fn type_ok(&self, t: &ast::TypeExpr, params: &[String]) -> Result<(), String> {
        let why = || format!("`{}` has no generator", type_text(t, &HashMap::new()));
        match &t.kind {
            TypeKind::Path { segments, args } => {
                let names: Vec<&str> = segments.iter().map(|s| s.text.as_str()).collect();
                let [n] = names.as_slice() else { return Err(why()) };
                if params.iter().any(|p| p == n) && args.is_empty() {
                    return Ok(());
                }
                let all = |args: &[ast::TypeExpr]| args.iter().try_for_each(|a| self.type_ok(a, params));
                match (*n, args.len()) {
                    (
                        "int" | "i64" | "bool" | "f64" | "char" | "String" | "u8" | "u16" | "u32" | "i8"
                        | "i16" | "i32" | "Duration",
                        0,
                    ) => Ok(()),
                    ("list" | "set" | "Option" | "Box" | "Slice", 1) => all(args),
                    ("dict", 2) => all(args),
                    _ => match self.shapes.get(*n) {
                        Some(s) if s.generics().len() == args.len() && self.ok.get(*n) == Some(&true) => {
                            all(args)
                        }
                        _ => Err(why()),
                    },
                }
            }
            TypeKind::Option(inner) => self.type_ok(inner, params),
            TypeKind::Tuple(es) if !es.is_empty() => es.iter().try_for_each(|(_, e)| self.type_ok(e, params)),
            _ => Err(why()),
        }
    }

    /// An expression of type `t` drawn from the `Draws` named `d`.
    fn call(&self, t: &ast::TypeExpr, d: &str, depth: u32) -> String {
        match &t.kind {
            TypeKind::Path { segments, args } => {
                let n = segments.first().map(|s| s.text.as_str()).unwrap_or("");
                let leaf = |name: &str| format!("Draws.gen_{name}({d})");
                match (n, args.as_slice()) {
                    ("int" | "i64", []) => leaf("int"),
                    ("bool", []) => leaf("bool"),
                    ("f64", []) => leaf("f64"),
                    ("char", []) => leaf("char"),
                    ("String", []) => leaf("string"),
                    ("Duration", []) => leaf("duration"),
                    ("u8" | "u16" | "u32" | "i8" | "i16" | "i32", []) => leaf(n),
                    ("list" | "Slice", [x]) => format!("Draws.gen_list({d}, {})", self.func(x, depth)),
                    ("set", [x]) => format!("Draws.gen_set({d}, {})", self.func(x, depth)),
                    ("Option", [x]) => format!("Draws.gen_option({d}, {})", self.func(x, depth)),
                    ("Box", [x]) => format!("Draws.gen_box({d}, {})", self.func(x, depth)),
                    ("dict", [k, v]) => {
                        format!("Draws.gen_dict({d}, {}, {})", self.func(k, depth), self.func(v, depth))
                    }
                    (p, []) if !self.shapes.contains_key(p) => format!("gen_{p}({d})"),
                    (u, args) => {
                        let mut s = format!("__arbitrary_{u}({d}");
                        for a in args {
                            s.push_str(", ");
                            s.push_str(&self.func(a, depth));
                        }
                        s.push(')');
                        s
                    }
                }
            }
            TypeKind::Option(inner) => format!("Draws.gen_option({d}, {})", self.func(inner, depth)),
            TypeKind::Tuple(es) => {
                let parts: Vec<String> = es.iter().map(|(_, e)| self.call(e, d, depth)).collect();
                if parts.len() == 1 { format!("({},)", parts[0]) } else { format!("({})", parts.join(", ")) }
            }
            _ => format!("{d}.reject()"),
        }
    }

    /// A function value `(Draws) -> T` for the type `t`.
    fn func(&self, t: &ast::TypeExpr, depth: u32) -> String {
        if let TypeKind::Path { segments, args } = &t.kind
            && args.is_empty()
        {
            let n = segments.first().map(|s| s.text.as_str()).unwrap_or("");
            let named = match n {
                "int" | "i64" => Some("Draws.gen_int".to_string()),
                "bool" | "f64" | "char" | "u8" | "u16" | "u32" | "i8" | "i16" | "i32" => {
                    Some(format!("Draws.gen_{n}"))
                }
                "String" => Some("Draws.gen_string".into()),
                "Duration" => Some("Draws.gen_duration".into()),
                p if self.shapes.contains_key(p) => Some(format!("__arbitrary_{p}")),
                p => Some(format!("gen_{p}")),
            };
            if let Some(s) = named {
                return s;
            }
        }
        let e = format!("e{depth}");
        format!("(({e}: Draws) -> {})", self.call(t, &e, depth + 1))
    }

    /// `def __arbitrary_T[...](d: Draws, gen_A: (Draws) -> A, ...) -> T[...]:`.
    fn header(&mut self, name: &str, generics: &[(String, String)]) {
        let params: Vec<String> = generics.iter().map(|(g, _)| g.clone()).collect();
        let bounded: Vec<String> = generics
            .iter()
            .map(|(g, b)| if b.is_empty() { g.clone() } else { format!("{g}: {b}") })
            .collect();
        let gp = if params.is_empty() { String::new() } else { format!("[{}]", bounded.join(", ")) };
        let ta = if params.is_empty() { String::new() } else { format!("[{}]", params.join(", ")) };
        let mut sig = format!("\ndef __arbitrary_{name}{gp}(d: Draws");
        // Owned, so the result borrows from none of them.
        for p in &params {
            sig.push_str(&format!(", own gen_{p}: (Draws) -> {p}"));
        }
        sig.push_str(&format!(") -> {name}{ta}:\n"));
        self.out.push_str(&sig);
    }
}

impl Resolver<'_> {
    /// Writes and declares the generators the root module's property tests
    /// need (see the module comment).
    pub fn declare_generators(&mut self, module: ModuleId, items: &[ast::Item]) {
        let file = self.file(module);
        let source = self.sources.get(file).map(|f| f.text.clone()).unwrap_or_default();
        let slice =
            |r: sisu_diag::TextRange| source.get(r.start as usize..r.end as usize).unwrap_or("").to_string();
        let bounds_of = |gs: &[ast::GenericParam]| -> Vec<(String, String)> {
            gs.iter()
                .map(|g| {
                    let b: Vec<String> = g.bounds.iter().map(|b| slice(b.range)).collect();
                    (g.name.text.clone(), b.join(" + "))
                })
                .collect()
        };
        let mut shapes: HashMap<String, Shape> = HashMap::new();
        let mut order: Vec<(String, Vec<(String, String)>)> = Vec::new();
        for it in items {
            let builtin = it.attrs.iter().any(|a| a.name == "builtin");
            match &it.kind {
                ItemKind::Struct(s) if !s.is_view && !builtin && !s.name.text.is_empty() => {
                    if s.generics.iter().any(|g| g.is_const) {
                        continue;
                    }
                    shapes.insert(
                        s.name.text.clone(),
                        Shape::Struct {
                            generics: s.generics.iter().map(|g| g.name.text.clone()).collect(),
                            fields: s.fields.iter().map(|f| (f.name.text.clone(), f.ty.clone())).collect(),
                            invariant: !s.invariants.is_empty(),
                        },
                    );
                    order.push((s.name.text.clone(), bounds_of(&s.generics)));
                }
                ItemKind::Enum(e) if !e.name.text.is_empty() => {
                    shapes.insert(
                        e.name.text.clone(),
                        Shape::Enum {
                            generics: e.generics.iter().map(|g| g.name.text.clone()).collect(),
                            variants: e
                                .variants
                                .iter()
                                .map(|v| {
                                    let fs = v
                                        .fields
                                        .iter()
                                        .map(|f| (f.name.as_ref().map(|n| n.text.clone()), f.ty.clone()))
                                        .collect();
                                    (v.name.text.clone(), fs)
                                })
                                .collect(),
                        },
                    );
                    order.push((e.name.text.clone(), bounds_of(&e.generics)));
                }
                ItemKind::TypeAlias(a) if a.refinement && !a.name.text.is_empty() => {
                    let Some(base) = &a.ty else { continue };
                    // Narrowed by the base's below, for a refinement of one.
                    let domain = a.predicate.as_ref().and_then(crate::collect::int_domain);
                    shapes.insert(
                        a.name.text.clone(),
                        Shape::Refinement {
                            generics: a.generics.iter().map(|g| g.name.text.clone()).collect(),
                            base: base.clone(),
                            domain,
                        },
                    );
                    order.push((a.name.text.clone(), bounds_of(&a.generics)));
                }
                _ => {}
            }
        }
        // An integer refinement draws in the range its predicate and its
        // bases' bound it to; one not over `int` draws its base.
        fn root_domain(
            shapes: &HashMap<String, Shape>,
            n: &str,
            depth: u32,
        ) -> Result<Option<(i64, i64)>, ()> {
            let Some(Shape::Refinement { base, domain, .. }) = shapes.get(n) else { return Err(()) };
            if depth > 64 {
                return Err(());
            }
            let TypeKind::Path { segments, args } = &base.kind else { return Err(()) };
            let [b] = segments.as_slice() else { return Err(()) };
            if !args.is_empty() {
                return Err(());
            }
            if b.text == "int" {
                return Ok(*domain);
            }
            let inherited = root_domain(shapes, &b.text, depth + 1)?;
            Ok(match (*domain, inherited) {
                (Some((a, c)), Some((e, f))) => Some((a.max(e), c.min(f))),
                (x, y) => x.or(y),
            })
        }
        let names: Vec<String> = shapes.keys().cloned().collect();
        let domains: Vec<(String, Option<(i64, i64)>)> =
            names.iter().map(|n| (n.clone(), root_domain(&shapes, n, 0).ok().flatten())).collect();
        for (n, d) in domains {
            if let Some(Shape::Refinement { domain, .. }) = shapes.get_mut(&n) {
                *domain = d;
            }
        }
        let mut w = Writer { shapes: &shapes, ok: HashMap::new(), out: String::new() };
        // Which types have a generator: assume all do, then drop those with
        // a part that has none, until nothing changes.
        for (n, _) in &order {
            w.ok.insert(n.clone(), true);
        }
        loop {
            let mut changed = false;
            for (n, _) in &order {
                if w.ok.get(n) != Some(&true) {
                    continue;
                }
                let Some(s) = shapes.get(n) else { continue };
                let params = s.generics().to_vec();
                let fine = match s {
                    Shape::Struct { fields, .. } => fields.iter().all(|(_, t)| w.type_ok(t, &params).is_ok()),
                    Shape::Enum { variants, .. } => {
                        variants.iter().all(|(_, fs)| fs.iter().all(|(_, t)| w.type_ok(t, &params).is_ok()))
                    }
                    Shape::Refinement { base, .. } => w.type_ok(base, &params).is_ok(),
                };
                if !fine {
                    w.ok.insert(n.clone(), false);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        // Each type's generator, written out only if a property needs it.
        let mut gens: HashMap<String, String> = HashMap::new();
        for (n, bounds) in &order {
            if w.ok.get(n) != Some(&true) {
                continue;
            }
            let Some(s) = shapes.get(n) else { continue };
            let start = w.out.len();
            gens.insert(n.clone(), String::new());
            w.header(n, bounds);
            match s {
                Shape::Struct { fields, invariant, .. } => {
                    let parts: Vec<String> =
                        fields.iter().map(|(f, t)| format!("{f}={}", w.call(t, "d", 0))).collect();
                    w.out.push_str(&format!("    let v = {n}({})\n", parts.join(", ")));
                    if *invariant {
                        w.out.push_str("    if not v.__invariant():\n        d.reject()\n");
                    }
                    w.out.push_str("    return v\n");
                }
                Shape::Enum { variants, .. } => {
                    w.out.push_str(&format!("    let k = d.draw({})\n", variants.len().max(1)));
                    for (i, (vn, fs)) in variants.iter().enumerate() {
                        let value = if fs.is_empty() {
                            vn.clone()
                        } else {
                            let parts: Vec<String> = fs
                                .iter()
                                .map(|(f, t)| match f {
                                    Some(f) => format!("{f}={}", w.call(t, "d", 0)),
                                    None => w.call(t, "d", 0),
                                })
                                .collect();
                            format!("{vn}({})", parts.join(", "))
                        };
                        if i + 1 < variants.len() {
                            w.out.push_str(&format!("    if k == {i}:\n        return {value}\n"));
                        } else {
                            w.out.push_str(&format!("    return {value}\n"));
                        }
                    }
                    if variants.is_empty() {
                        w.out.push_str("    d.reject()\n");
                    }
                }
                Shape::Refinement { base, domain, .. } => {
                    let draw = match domain {
                        Some((lo, hi)) => format!("Draws.gen_in(d, {lo}, {hi})"),
                        None => w.call(base, "d", 0),
                    };
                    w.out.push_str(&format!(
                        "    match {n}.try_from({draw}):\n        case Ok(v):\n            return v\n        case Err(_):\n            d.reject()\n"
                    ));
                }
            }
            let text = w.out.split_off(start);
            gens.insert(n.clone(), text);
        }
        // The types written in `arbitrary[T]` and `exhaustive[T](...)`.
        let mut sites: Vec<(String, ast::Expr)> = Vec::new();
        ast::visit::exprs_in_items(items, &mut |e| {
            if let ast::ExprKind::Index { base, args } = &e.kind
                && matches!(&base.kind, ast::ExprKind::Name(n) if n == "arbitrary" || n == "exhaustive")
                && let [ast::IndexArg::Expr(t)] = args.as_slice()
            {
                let key: String = slice(t.range).chars().filter(|c| !c.is_whitespace()).collect();
                if !sites.iter().any(|(k, _)| *k == key) {
                    sites.push((key, t.clone()));
                }
            }
        });
        for (i, (key, t)) in sites.iter().enumerate() {
            let Some(te) = expr_as_type(t) else {
                self.prog.arbitrary_sites.insert(key.clone(), Err(format!("`{key}` is not a type")));
                continue;
            };
            match w.type_ok(&te, &[]) {
                Ok(()) => {
                    let name = format!("__arbitrary_site_{i}");
                    let ty = type_text(&te, &HashMap::new());
                    w.out.push_str(&format!(
                        "\ndef {name}(d: Draws) -> {ty}:\n    return {}\n",
                        w.call(&te, "d", 0)
                    ));
                    self.prog.arbitrary_sites.insert(key.clone(), Ok(name));
                }
                Err(why) => {
                    self.prog.arbitrary_sites.insert(key.clone(), Err(why));
                }
            }
        }
        // The contracts as properties.
        let mut tests = String::new();
        let mut k = 0;
        let mut contract_fns: Vec<(Option<String>, &ast::FnDef)> = Vec::new();
        for it in items {
            match &it.kind {
                ItemKind::Fn(f) => contract_fns.push((None, f)),
                ItemKind::Struct(s) => {
                    contract_fns.extend(s.methods.iter().map(|m| (Some(s.name.text.clone()), m)))
                }
                ItemKind::Enum(e) => {
                    contract_fns.extend(e.methods.iter().map(|m| (Some(e.name.text.clone()), m)))
                }
                ItemKind::Impl(i) if i.trait_ty.is_none() => {
                    if let TypeKind::Path { segments, args } = &i.self_ty.kind
                        && args.is_empty()
                        && segments.len() == 1
                    {
                        contract_fns.extend(i.methods.iter().map(|m| (Some(segments[0].text.clone()), m)));
                    }
                }
                _ => {}
            }
        }
        // A `mut self` method of a type with an `invariant` has a contract
        // even without clauses: the invariant holds when it returns.
        let kept: HashSet<&str> = items
            .iter()
            .filter_map(|it| match &it.kind {
                ItemKind::Struct(s) if !s.invariants.is_empty() => Some(s.name.text.as_str()),
                _ => None,
            })
            .collect();
        for (owner, f) in contract_fns {
            let clauses = f.contracts();
            let keeps_invariant = owner.as_deref().is_some_and(|o| kept.contains(o))
                && f.params
                    .first()
                    .is_some_and(|p| matches!(p.kind, ast::ParamKind::SelfParam) && p.mode == ast::Mode::Mut);
            if (clauses.is_empty() && !keeps_invariant) || f.is_comptime || f.name.text.is_empty() {
                continue;
            }
            // Each type parameter is `int`, when `int` meets its bounds.
            let mut subst: HashMap<String, String> = HashMap::new();
            let mut fine = true;
            for g in &f.generics {
                let bounds_ok = g.bounds.iter().all(|b| {
                    matches!(&b.kind, TypeKind::Path { segments, args } if args.is_empty() && segments.len() == 1 && INT_MEETS.contains(&segments[0].text.as_str()))
                });
                fine &= bounds_ok && !g.is_const;
                subst.insert(g.name.text.clone(), "int".into());
            }
            if !f.where_.is_empty() {
                fine = false;
            }
            // The receiver, then the parameters: (name, type to draw, mode).
            let mut params: Vec<(String, ast::TypeExpr, ast::Mode)> = Vec::new();
            for p in &f.params {
                match &p.kind {
                    ast::ParamKind::SelfParam => match &owner {
                        Some(o) if shapes.get(o).is_some_and(|s| s.generics().is_empty()) => {
                            let te = ast::TypeExpr {
                                node: f.node,
                                range: f.name.range,
                                kind: TypeKind::Path {
                                    segments: vec![ast::Name { text: o.clone(), range: f.name.range }],
                                    args: Vec::new(),
                                },
                            };
                            params.push(("self".into(), te, p.mode));
                        }
                        _ => fine = false,
                    },
                    ast::ParamKind::Named { name, ty, .. } => {
                        params.push((name.text.clone(), substitute(ty, &subst), p.mode))
                    }
                    _ => fine = false,
                }
            }
            if !fine || params.iter().any(|(_, t, _)| w.type_ok(&drawn(t), &[]).is_err()) || params.is_empty()
            {
                continue;
            }
            let shown = match &owner {
                Some(o) => format!("{o}.{}", f.name.text),
                None => f.name.text.clone(),
            };
            let n = k;
            k += 1;
            // `requires` as a predicate of the same parameters: a method of
            // the type for a method.
            let requires: Vec<String> = clauses
                .iter()
                .filter(|c| c.kind == ast::ContractKind::Requires)
                .map(|c| format!("({})", slice(c.expr.range).trim()))
                .collect();
            let mut decl = String::new();
            if !requires.is_empty() {
                let named: Vec<String> = f
                    .params
                    .iter()
                    .filter_map(|p| match &p.kind {
                        ast::ParamKind::Named { name, ty, .. } => {
                            Some(format!("{}: {}", name.text, type_text(ty, &HashMap::new())))
                        }
                        _ => None,
                    })
                    .collect();
                let gens: Vec<String> = f
                    .generics
                    .iter()
                    .map(|g| {
                        let b: Vec<String> = g.bounds.iter().map(|b| slice(b.range)).collect();
                        if b.is_empty() {
                            g.name.text.clone()
                        } else {
                            format!("{}: {}", g.name.text, b.join(" + "))
                        }
                    })
                    .collect();
                let gp = if gens.is_empty() { String::new() } else { format!("[{}]", gens.join(", ")) };
                let body = format!("        return {}\n", requires.join(" and "));
                match &owner {
                    Some(o) => {
                        let mut ps = vec!["self".to_string()];
                        ps.extend(named);
                        decl = format!(
                            "\nimpl {o}:\n    def __requires_{n}{gp}({}) -> bool:\n{body}",
                            ps.join(", ")
                        );
                    }
                    None => {
                        decl = format!(
                            "\ndef __requires_{n}{gp}({}) -> bool:\n{}",
                            named.join(", "),
                            &body[4..]
                        );
                    }
                }
            }
            w.out.push_str(&decl);
            let tys: Vec<String> =
                params.iter().map(|(_, t, _)| type_text(&drawn(t), &HashMap::new())).collect();
            let tuple =
                if tys.len() == 1 { format!("({},)", tys[0]) } else { format!("({})", tys.join(", ")) };
            let mut cases = format!("\ndef __contract_cases_{n}(d: Draws) -> {tuple}:\n");
            for (i, (_, t, _)) in params.iter().enumerate() {
                cases.push_str(&format!("    let a{i}: {} = {}\n", tys[i], w.call(&drawn(t), "d", 0)));
            }
            let names: Vec<String> = (0..params.len()).map(|i| format!("a{i}")).collect();
            if !requires.is_empty() {
                let call = match &owner {
                    Some(_) => format!("a0.__requires_{n}({})", names[1..].join(", ")),
                    None => format!("__requires_{n}({})", names.join(", ")),
                };
                cases.push_str(&format!("    if not {call}:\n        d.reject()\n"));
            }
            let ret =
                if names.len() == 1 { format!("({},)", names[0]) } else { format!("({})", names.join(", ")) };
            cases.push_str(&format!("    return {ret}\n"));
            w.out.push_str(&cases);
            // The test: the call, on each case; its contract is checked
            // inside it.
            let mut body = String::new();
            for (i, (_, _, mode)) in params.iter().enumerate() {
                let kw = if *mode == ast::Mode::Mut { "var" } else { "let" };
                body.push_str(&format!("        {kw} b{i} = args.{i}\n"));
            }
            let arg = |i: usize| {
                let (_, _, mode) = &params[i];
                if *mode == ast::Mode::Mut { format!("mut b{i}") } else { format!("b{i}") }
            };
            let call = match &owner {
                Some(_) => {
                    let rest: Vec<String> = (1..params.len()).map(arg).collect();
                    format!("b0.{}({})", f.name.text, rest.join(", "))
                }
                None => {
                    let all: Vec<String> = (0..params.len()).map(arg).collect();
                    format!("{}({})", f.name.text, all.join(", "))
                }
            };
            body.push_str(&format!("        _ = {call}\n        return True\n"));
            tests.push_str(&format!(
                "\ntest \"contract of {shown}\":\n    check(Gen.of(__contract_cases_{n}, quiet=True, steps=100000)) do args:\n{body}"
            ));
        }
        // `@refines(reference)` (17, D-070): the function must give what
        // its reference gives on every case, within the tolerance.
        let fns: HashMap<&str, &ast::FnDef> = items
            .iter()
            .filter_map(|it| match &it.kind {
                ItemKind::Fn(f) => Some((f.name.text.as_str(), f)),
                _ => None,
            })
            .collect();
        for it in items {
            let ItemKind::Fn(f) = &it.kind else { continue };
            for a in it.attrs.iter().filter(|a| a.name == "refines") {
                let at = self.span(module, f.name.range);
                let reference =
                    a.args.first().filter(|x| x.name.is_none()).and_then(|x| match &x.value.kind {
                        ExprKind::Name(n) => Some(n.clone()),
                        _ => None,
                    });
                let tolerance =
                    a.args.iter().find(|x| x.name.as_ref().is_some_and(|n| n.text == "tolerance"));
                let ulps = tolerance.map(|t| match &t.value.kind {
                    ExprKind::Call { callee, args, .. }
                        if matches!(&callee.kind, ExprKind::Name(n) if n == "Ulps") && args.len() == 1 =>
                    {
                        match &args[0].value.kind {
                            ExprKind::Int { value: Some(v), .. } => i64::try_from(*v).ok(),
                            _ => None,
                        }
                    }
                    _ => None,
                });
                let fail = |why: String| {
                    Diagnostic::new(msg::TEST_REFINES_MISMATCH, at)
                        .with("reference", reference.clone().unwrap_or_else(|| "?".into()))
                        .with("name", f.name.text.clone())
                        .with("why", why)
                };
                let Some(rname) = reference.clone() else {
                    self.report(fail("the reference is a function of this module, named first".into()));
                    continue;
                };
                let Some(g) = fns.get(rname.as_str()) else {
                    self.report(fail(format!("there is no function `{rname}` in this module")));
                    continue;
                };
                if let Some(None) = ulps {
                    self.report(fail(
                        "the tolerance is written `tolerance=Ulps(n)`, with a whole number".into(),
                    ));
                    continue;
                }
                let sig = |h: &ast::FnDef| -> Option<(Vec<(String, ast::Mode)>, String)> {
                    let mut ps = Vec::new();
                    for p in &h.params {
                        match &p.kind {
                            ast::ParamKind::Named { ty, .. } => {
                                ps.push((type_text(ty, &HashMap::new()), p.mode))
                            }
                            _ => return None,
                        }
                    }
                    let ret =
                        h.ret.as_ref().map(|r| type_text(r, &HashMap::new())).unwrap_or_else(|| "()".into());
                    Some((ps, ret))
                };
                let (Some((ps, ret)), Some((qs, gret))) = (sig(f), sig(g)) else {
                    self.report(fail("both must be free functions with named parameters".into()));
                    continue;
                };
                if !f.generics.is_empty() || !g.generics.is_empty() || ps.is_empty() {
                    self.report(fail("both must take parameters, and no type parameters".into()));
                    continue;
                }
                if ps != qs || ret != gret {
                    self.report(fail(format!(
                        "`{}` takes ({}) and gives `{ret}`, `{rname}` takes ({}) and gives `{gret}`",
                        f.name.text,
                        ps.iter().map(|p| p.0.clone()).collect::<Vec<_>>().join(", "),
                        qs.iter().map(|p| p.0.clone()).collect::<Vec<_>>().join(", ")
                    )));
                    continue;
                }
                if ulps.is_some() && ret != "f64" {
                    self.report(fail(format!("a tolerance in ULPs compares `f64` results, not `{ret}`")));
                    continue;
                }
                let tys: Vec<ast::TypeExpr> = f
                    .params
                    .iter()
                    .filter_map(|p| match &p.kind {
                        ast::ParamKind::Named { ty, .. } => Some(drawn(ty)),
                        _ => None,
                    })
                    .collect();
                if let Some(why) = tys.iter().find_map(|t| w.type_ok(t, &[]).err()) {
                    self.report(fail(why));
                    continue;
                }
                let n = k;
                k += 1;
                let texts: Vec<String> = tys.iter().map(|t| type_text(t, &HashMap::new())).collect();
                let tuple = if texts.len() == 1 {
                    format!("({},)", texts[0])
                } else {
                    format!("({})", texts.join(", "))
                };
                let mut cases = format!("\ndef __refines_cases_{n}(d: Draws) -> {tuple}:\n");
                for (i, t) in tys.iter().enumerate() {
                    cases.push_str(&format!("    let a{i}: {} = {}\n", texts[i], w.call(t, "d", 0)));
                }
                let names: Vec<String> = (0..tys.len()).map(|i| format!("a{i}")).collect();
                let ret_tuple = if names.len() == 1 {
                    format!("({},)", names[0])
                } else {
                    format!("({})", names.join(", "))
                };
                cases.push_str(&format!("    return {ret_tuple}\n"));
                w.out.push_str(&cases);
                // Each call gets its own copy of an argument it takes.
                let args: Vec<String> = ps
                    .iter()
                    .enumerate()
                    .map(|(i, (_, mode))| match mode {
                        ast::Mode::Own => format!("args.{i}.copy()"),
                        ast::Mode::Mut => format!("mut args.{i}.copy()"),
                        ast::Mode::Borrow => format!("args.{i}"),
                    })
                    .collect();
                let (lhs, rhs) = (
                    format!("{}({})", f.name.text, args.join(", ")),
                    format!("{rname}({})", args.join(", ")),
                );
                let holds = match ulps.flatten() {
                    Some(u) => format!("within_ulps({lhs}, {rhs}, {u})"),
                    None => format!("{lhs} == {rhs}"),
                };
                tests.push_str(&format!(
                    "\ntest \"{} refines {rname}\":\n    check(Gen.of(__refines_cases_{n}, quiet=True, steps=100000)) do args:\n        return {holds}\n",
                    f.name.text
                ));
            }
        }
        w.out.push_str(&tests);
        // The generators the sites and the contract tests call, and the
        // ones those call, in declaration order.
        let mut needed: HashSet<String> = HashSet::new();
        let mut queue = referenced(&w.out);
        while let Some(n) = queue.pop() {
            if gens.contains_key(&n) && needed.insert(n.clone()) {
                queue.extend(referenced(&gens[&n]));
            }
        }
        let mut text = String::new();
        for (n, _) in &order {
            if needed.contains(n) {
                text.push_str(&gens[n]);
            }
        }
        text.push_str(&w.out);
        if text.is_empty() {
            return;
        }
        let generated = self.declare_generated_in(module, "<arbitrary>", &text);
        // The generators are named from the module they serve.
        if let Some(g) = generated {
            let names: Vec<(String, DefId)> =
                self.prog.modules[g.0 as usize].items.iter().map(|(n, d)| (n.clone(), *d)).collect();
            for (name, d) in names {
                self.prog.modules[module.0 as usize].items.entry(name).or_insert(d);
            }
        }
    }
}

/// The types whose generators `text` calls: each `__arbitrary_T`.
fn referenced(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find("__arbitrary_") {
        rest = &rest[i + "__arbitrary_".len()..];
        let n: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        if !n.is_empty() {
            out.push(n);
        }
    }
    out
}

/// The type a parameter's cases are drawn as: a `Slice[T]` as a
/// `list[T]`, a `str` as a `String` (each becomes the parameter's type
/// where it is passed).
fn drawn(t: &ast::TypeExpr) -> ast::TypeExpr {
    match &t.kind {
        TypeKind::Path { segments, args } if segments.len() == 1 && segments[0].text == "Slice" => {
            ast::TypeExpr {
                kind: TypeKind::Path {
                    segments: vec![ast::Name { text: "list".into(), range: segments[0].range }],
                    args: args.clone(),
                },
                ..t.clone()
            }
        }
        TypeKind::Path { segments, args }
            if segments.len() == 1 && segments[0].text == "str" && args.is_empty() =>
        {
            ast::TypeExpr {
                kind: TypeKind::Path {
                    segments: vec![ast::Name { text: "String".into(), range: segments[0].range }],
                    args: Vec::new(),
                },
                ..t.clone()
            }
        }
        _ => t.clone(),
    }
}

/// The type with the type parameters replaced by the names in `subst`.
fn substitute(t: &ast::TypeExpr, subst: &HashMap<String, String>) -> ast::TypeExpr {
    let mut t = t.clone();
    fn go(t: &mut ast::TypeExpr, subst: &HashMap<String, String>) {
        match &mut t.kind {
            TypeKind::Path { segments, args } => {
                if segments.len() == 1
                    && args.is_empty()
                    && let Some(to) = subst.get(&segments[0].text)
                {
                    segments[0].text = to.clone();
                }
                args.iter_mut().for_each(|a| go(a, subst));
            }
            TypeKind::Option(i) | TypeKind::Dyn(i) => go(i, subst),
            TypeKind::Ref { inner, .. } => go(inner, subst),
            TypeKind::Tuple(es) => es.iter_mut().for_each(|(_, e)| go(e, subst)),
            TypeKind::Result { ok, err } => {
                if let Some(o) = ok {
                    go(o, subst);
                }
                if let Some(e) = err {
                    go(e, subst);
                }
            }
            TypeKind::Fn { params, ret, .. } => {
                params.iter_mut().for_each(|p| go(p, subst));
                go(ret, subst);
            }
            TypeKind::Union(us) => us.iter_mut().for_each(|u| go(u, subst)),
            _ => {}
        }
    }
    go(&mut t, subst);
    t
}

/// The spelling of a type, with the names of `subst` replaced.
pub(crate) fn type_text(t: &ast::TypeExpr, subst: &HashMap<String, String>) -> String {
    match &t.kind {
        TypeKind::Path { segments, args } => {
            let mut name: String = segments.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join(".");
            if let Some(to) = subst.get(&name) {
                name = to.clone();
            }
            if args.is_empty() {
                name
            } else {
                let a: Vec<String> = args.iter().map(|x| type_text(x, subst)).collect();
                format!("{name}[{}]", a.join(", "))
            }
        }
        TypeKind::Option(i) => format!("{}?", type_text(i, subst)),
        TypeKind::Tuple(es) => {
            let a: Vec<String> = es
                .iter()
                .map(|(n, e)| match n {
                    Some(n) => format!("{}: {}", n.text, type_text(e, subst)),
                    None => type_text(e, subst),
                })
                .collect();
            if a.len() == 1 { format!("({},)", a[0]) } else { format!("({})", a.join(", ")) }
        }
        TypeKind::Ref { mutable, inner } => {
            format!("ref {}{}", if *mutable { "mut " } else { "" }, type_text(inner, subst))
        }
        TypeKind::Result { ok, err } => {
            let ok = ok.as_ref().map(|o| type_text(o, subst)).unwrap_or_default();
            let err = err.as_ref().map(|e| type_text(e, subst)).unwrap_or_default();
            if ok.is_empty() { format!("! {err}") } else { format!("{ok} ! {err}") }
        }
        TypeKind::Fn { pure, params, ret } => {
            let p: Vec<String> = params.iter().map(|x| type_text(x, subst)).collect();
            format!("{}({}) -> {}", if *pure { "pure " } else { "" }, p.join(", "), type_text(ret, subst))
        }
        TypeKind::Union(us) => {
            let p: Vec<String> = us.iter().map(|x| type_text(x, subst)).collect();
            format!("{{{}}}", p.join(", "))
        }
        TypeKind::Dyn(i) => format!("dyn {}", type_text(i, subst)),
        _ => "()".into(),
    }
}

/// An index argument read as a type: `list[int]` in `arbitrary[list[int]]`.
fn expr_as_type(e: &ast::Expr) -> Option<ast::TypeExpr> {
    let kind = match &e.kind {
        ast::ExprKind::Name(n) => {
            TypeKind::Path { segments: vec![ast::Name { text: n.clone(), range: e.range }], args: Vec::new() }
        }
        ast::ExprKind::Index { base, args } => {
            let ast::ExprKind::Name(n) = &base.kind else { return None };
            let mut targs = Vec::new();
            for a in args {
                let ast::IndexArg::Expr(x) = a else { return None };
                targs.push(expr_as_type(x)?);
            }
            TypeKind::Path { segments: vec![ast::Name { text: n.clone(), range: base.range }], args: targs }
        }
        ast::ExprKind::Tuple(xs) if !xs.is_empty() => TypeKind::Tuple(
            xs.iter().map(|x| expr_as_type(x).map(|t| (None, t))).collect::<Option<Vec<_>>>()?,
        ),
        ast::ExprKind::Paren(inner) => return expr_as_type(inner),
        // `(int) -> bool` reads as a closure.
        ast::ExprKind::Closure(c) if c.params.iter().all(|p| p.ty.is_none()) && !c.implicit_param => {
            let ast::ClosureBody::Expr(body) = &c.body else { return None };
            let params = c
                .params
                .iter()
                .map(|p| ast::TypeExpr {
                    node: e.node,
                    range: p.name.range,
                    kind: TypeKind::Path { segments: vec![p.name.clone()], args: Vec::new() },
                })
                .collect();
            TypeKind::Fn { pure: false, params, ret: Box::new(expr_as_type(body)?) }
        }
        _ => return None,
    };
    Some(ast::TypeExpr { node: e.node, range: e.range, kind })
}
