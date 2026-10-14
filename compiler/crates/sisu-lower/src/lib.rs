//! Lowering from the typed tree to SIR (proposal/18-next-steps.md,
//! pipeline step 5): control flow becomes basic blocks; Drop elaboration
//! inserts `drop` at every scope exit in reverse declaration order, with a
//! drop flag for a local that may have been moved and an unwind edge per
//! block; closures become a function plus a tuple of captured references;
//! `?` becomes a switch and an early return that records the error's
//! trace; `for` becomes calls to the iterator intrinsics.
//!
//! The rules are written down in docs/architecture.md ("Lowering"), so a
//! front end in another language produces the same SIR.

mod exprs;
mod func;
mod pats;
mod stmts;

use sisu_diag::SourceMap;
use sisu_resolve::{DefId, DefKind, Program, Ty};
use sisu_sir as sir;
use sisu_typeck::{Body, BodyKind, Typed};
use std::collections::{HashMap, HashSet};

/// Names of SIR functions and types, and the type translation.
pub(crate) struct Ctx<'a> {
    pub prog: &'a Program,
    pub sources: &'a SourceMap,
    pub files: HashMap<sisu_diag::FileId, u32>,
    pub file_names: Vec<String>,
}

impl Ctx<'_> {
    pub fn fn_name(&self, d: DefId) -> String {
        let prog = self.prog;
        let def = prog.def(d);
        if let Some(i) = prog.tests.iter().position(|t| *t == d) {
            return format!("test#{i}");
        }
        match prog.fn_info(d).and_then(|f| f.owner.as_ref()) {
            Some(owner) => format!("{}.{}", self.owner_name(owner), def.name),
            None => {
                if def.module == prog.root || def.module == prog.prelude {
                    def.name.clone()
                } else {
                    format!("{}.{}", prog.modules[def.module.0 as usize].name, def.name)
                }
            }
        }
    }

    fn owner_name(&self, owner: &Ty) -> String {
        match owner {
            Ty::Adt(d, _) => self.type_name(*d),
            Ty::Int => "int".into(),
            Ty::Float => "f64".into(),
            Ty::Bool => "bool".into(),
            Ty::Char => "char".into(),
            Ty::Str => "str".into(),
            _ => "?".into(),
        }
    }

    /// The SIR name of a type definition: its builtin tag, or its name
    /// qualified by a std module.
    pub fn type_name(&self, d: DefId) -> String {
        let prog = self.prog;
        if let Some(b) = prog.builtin(d) {
            return b.to_string();
        }
        prog.qualified_name(d)
    }

    pub fn body_name(&self, b: &Body) -> String {
        match &b.kind {
            BodyKind::Fn => self.fn_name(b.def),
            BodyKind::FieldDefault(s, i) => self.field_default_name(*s, *i),
            BodyKind::ParamDefault(f, i) => self.param_default_name(*f, *i),
            BodyKind::Const => self.const_name(b.def),
        }
    }

    pub fn field_default_name(&self, s: DefId, i: u32) -> String {
        let field = self
            .prog
            .struct_info(s)
            .and_then(|x| x.fields.get(i as usize))
            .map(|f| f.name.clone())
            .unwrap_or_default();
        format!("{}#{field}", self.type_name(s))
    }

    pub fn param_default_name(&self, f: DefId, i: u32) -> String {
        let p = self
            .prog
            .fn_info(f)
            .and_then(|x| x.params.get(i as usize))
            .map(|p| p.name.clone())
            .unwrap_or_default();
        format!("{}#{p}", self.fn_name(f))
    }

    pub fn const_name(&self, d: DefId) -> String {
        let prog = self.prog;
        let def = prog.def(d);
        if def.module == prog.root || def.module == prog.prelude {
            def.name.clone()
        } else {
            format!("{}.{}", prog.modules[def.module.0 as usize].name, def.name)
        }
    }

    pub fn ty(&self, t: &Ty) -> sir::Type {
        let prog = self.prog;
        match t {
            Ty::Int => sir::Type::Int,
            Ty::Float => sir::Type::Float,
            Ty::Bool => sir::Type::Bool,
            Ty::Char => sir::Type::Char,
            Ty::Str => sir::Type::Str,
            Ty::Unit | Ty::Param(_) | Ty::Var(_) | Ty::Error => sir::Type::Unit,
            Ty::Never => sir::Type::Never,
            Ty::Adt(d, args) => {
                let name = if Some(*d) == prog.wk.option {
                    "Option".to_string()
                } else if Some(*d) == prog.wk.result {
                    "Result".to_string()
                } else {
                    self.type_name(*d)
                };
                sir::Type::Named(name, args.iter().map(|a| self.ty(a)).collect())
            }
            Ty::Tuple(es, _) if es.is_empty() => sir::Type::Unit,
            Ty::Tuple(es, _) => sir::Type::Tuple(es.iter().map(|e| self.ty(e)).collect()),
            Ty::ErrSet(ms) => sir::Type::ErrSet(ms.iter().map(|d| self.type_name(*d)).collect()),
            Ty::Fn(f) => sir::Type::Fn(
                f.params.iter().map(|p| self.ty(p)).collect(),
                Box::new(self.ty(&f.ret)),
                f.pure,
            ),
            Ty::Ref(t, m) => sir::Type::Ref(Box::new(self.ty(t)), *m),
        }
    }

    pub fn source_index(&mut self, file: sisu_diag::FileId) -> u32 {
        if let Some(&i) = self.files.get(&file) {
            return i;
        }
        let i = self.file_names.len() as u32;
        self.file_names.push(self.sources.name(file).to_string());
        self.files.insert(file, i);
        i
    }

    /// The source range of a span, without the line breaks and
    /// indentation a statement's range ends with.
    pub fn loc(&self, span: sisu_diag::Span) -> Option<sir::Loc> {
        let f = self.sources.get(span.file)?;
        let a = f.line_col(span.range.start);
        let b = f.line_col(trimmed_end(&f.text, span.range));
        Some(sir::Loc { line: a.line, col: a.col, end_line: b.line, end_col: b.col })
    }

    /// The empty range at the end of a span.
    pub fn end_loc(&self, span: sisu_diag::Span) -> Option<sir::Loc> {
        let f = self.sources.get(span.file)?;
        let b = f.line_col(trimmed_end(&f.text, span.range));
        Some(sir::Loc { line: b.line, col: b.col, end_line: b.line, end_col: b.col })
    }
}

/// The end of a range, before any trailing whitespace.
fn trimmed_end(text: &str, r: sisu_diag::TextRange) -> u32 {
    let bytes = text.as_bytes();
    let mut end = (r.end as usize).min(bytes.len());
    while end > r.start as usize && bytes.get(end - 1).is_some_and(|b| b.is_ascii_whitespace()) {
        end -= 1;
    }
    end as u32
}

/// Lowers a checked program. The program must have no errors; lowering
/// does not report diagnostics.
pub fn lower(prog: &Program, typed: &Typed, sources: &SourceMap) -> sir::Program {
    let mut ctx = Ctx { prog, sources, files: HashMap::new(), file_names: Vec::new() };
    // The root module's file is source 0.
    if let Some(m) = prog.modules.get(prog.root.0 as usize) {
        ctx.source_index(m.file);
    }
    let mut functions = Vec::new();
    for b in &typed.bodies {
        let is_intrinsic =
            matches!(b.kind, BodyKind::Fn) && prog.fn_info(b.def).is_some_and(|f| f.intrinsic.is_some());
        if is_intrinsic {
            continue;
        }
        functions.extend(func::lower_body(&mut ctx, b));
    }
    let entry = prog.main().and_then(|m| {
        let f = prog.fn_info(m)?;
        Some(sir::Entry {
            function: ctx.fn_name(m),
            takes_system: f.params.len() == 1,
            returns_result: matches!(&f.ret, Ty::Adt(d, _) if Some(*d) == prog.wk.result),
        })
    });
    let tests: Vec<(String, String)> = prog
        .tests
        .iter()
        .filter(|d| prog.fn_info(**d).is_some_and(|f| f.body.is_some()))
        .map(|d| (prog.def(*d).name.clone(), ctx.fn_name(*d)))
        .collect();
    // Keep what the entry, the tests and the root module's functions reach.
    let mut roots: Vec<String> = entry.iter().map(|e| e.function.clone()).collect();
    roots.extend(tests.iter().map(|(_, f)| f.clone()));
    for b in &typed.bodies {
        if prog.def(b.def).module == prog.root && matches!(b.kind, BodyKind::Fn) {
            roots.push(ctx.body_name(b));
        }
    }
    let types = collect_types(&ctx, &functions);
    let functions = prune(functions, &roots, &types);
    let types = collect_types(&ctx, &functions);
    let intrinsics = intrinsic_decls(prog, &functions);
    sir::Program { sources: ctx.file_names.clone(), entry, types, intrinsics, functions, tests }
}

/// The facts the SIR checkers need about every intrinsic the functions
/// call: the prelude's declaration reduced to purity and the arguments the
/// result borrows from, or, for the intrinsics lowering introduces, the
/// table in docs/architecture.md ("Lowering-only intrinsics").
fn intrinsic_decls(prog: &Program, functions: &[sir::Function]) -> Vec<sir::IntrinsicDecl> {
    let mut names: Vec<String> = Vec::new();
    for f in functions {
        for b in &f.blocks {
            for s in &b.stmts {
                if let sir::StatementKind::Call { callee: sir::Callee::Intrinsic(n, _), .. } = &s.kind
                    && !names.contains(n)
                {
                    names.push(n.clone());
                }
            }
        }
    }
    names.sort();
    let mut by_name: HashMap<&str, &sisu_resolve::FnInfo> = HashMap::new();
    for d in &prog.defs {
        if let DefKind::Fn(f) = &d.kind
            && let Some(n) = &f.intrinsic
        {
            by_name.insert(n.as_str(), f);
        }
    }
    names
        .into_iter()
        .map(|name| match by_name.get(name.as_str()) {
            Some(f) => {
                let b = prog.result_borrows_of(f);
                let own = prog
                    .sig_params(f)
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| p.mode == sisu_syntax::ast::Mode::Own)
                    .map(|(i, _)| i as u32)
                    .collect();
                sir::IntrinsicDecl { name, pure: f.pure, from: b.deep, inner: b.inner, own }
            }
            None => {
                let (from, inner): (Vec<u32>, Vec<u32>) = match name.as_str() {
                    "iter.ref" | "iter.mut" | "iter.own" => (vec![0], vec![]),
                    "list.slice" | "list.slice_middle" => (vec![], vec![0]),
                    _ => (vec![], vec![]),
                };
                let own = if name == "iter.own" { vec![0] } else { Vec::new() };
                sir::IntrinsicDecl { name, pure: true, from, inner, own }
            }
        })
        .collect()
}

fn prune(functions: Vec<sir::Function>, roots: &[String], types: &[sir::TypeDecl]) -> Vec<sir::Function> {
    let by_name: HashMap<&str, &sir::Function> = functions.iter().map(|f| (f.name.as_str(), f)).collect();
    let mut keep: HashSet<String> = HashSet::new();
    let mut work: Vec<String> = roots.to_vec();
    // Defaults and drops of declared types are reached through the types.
    for t in types {
        match t {
            sir::TypeDecl::Struct { fields, drop, .. } => {
                work.extend(fields.iter().filter_map(|f| f.default.clone()));
                work.extend(drop.clone());
            }
            sir::TypeDecl::Enum { drop, .. } => work.extend(drop.clone()),
        }
    }
    while let Some(n) = work.pop() {
        if !keep.insert(n.clone()) {
            continue;
        }
        let Some(f) = by_name.get(n.as_str()) else { continue };
        for b in &f.blocks {
            for s in &b.stmts {
                match &s.kind {
                    sir::StatementKind::Call { callee: sir::Callee::Fn(c), args, .. } => {
                        work.push(c.clone());
                        refs_in_operands(args, &mut work);
                    }
                    sir::StatementKind::Call { args, .. } => refs_in_operands(args, &mut work),
                    sir::StatementKind::Assign(_, r) => match r {
                        sir::Rvalue::Closure(c, ops) => {
                            work.push(c.clone());
                            refs_in_operands(ops, &mut work);
                        }
                        sir::Rvalue::Use(o) => refs_in_operands(std::slice::from_ref(o), &mut work),
                        sir::Rvalue::Aggregate(_, ops) => refs_in_operands(ops, &mut work),
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
    }
    functions.into_iter().filter(|f| keep.contains(&f.name)).collect()
}

fn refs_in_operands(ops: &[sir::Operand], work: &mut Vec<String>) {
    for o in ops {
        if let sir::Operand::Const(sir::Const::Fn(n)) = o {
            work.push(n.clone());
        }
    }
}

/// The declarations of every non-builtin type the functions mention, in
/// definition order.
fn collect_types(ctx: &Ctx, functions: &[sir::Function]) -> Vec<sir::TypeDecl> {
    let prog = ctx.prog;
    let mut names: HashSet<String> = HashSet::new();
    fn walk(t: &sir::Type, out: &mut HashSet<String>) {
        match t {
            sir::Type::Named(n, args) => {
                out.insert(n.clone());
                for a in args {
                    walk(a, out);
                }
            }
            sir::Type::Tuple(es) => es.iter().for_each(|e| walk(e, out)),
            sir::Type::ErrSet(ms) => out.extend(ms.iter().cloned()),
            sir::Type::Fn(ps, r, _) => {
                ps.iter().for_each(|p| walk(p, out));
                walk(r, out);
            }
            sir::Type::Ref(t, _) => walk(t, out),
            _ => {}
        }
    }
    for f in functions {
        for l in &f.locals {
            walk(&l.ty, &mut names);
        }
        for b in &f.blocks {
            for s in &b.stmts {
                if let sir::StatementKind::Assign(
                    _,
                    sir::Rvalue::Aggregate(sir::Aggregate::Struct(n) | sir::Aggregate::Variant(n, _), _),
                ) = &s.kind
                {
                    names.insert(n.clone());
                }
            }
        }
    }
    // Close over field types.
    let mut decls: Vec<(DefId, sir::TypeDecl)> = Vec::new();
    let mut done: HashSet<String> = HashSet::new();
    let mut queue: Vec<String> = names.into_iter().collect();
    queue.sort();
    while let Some(n) = queue.pop() {
        if !done.insert(n.clone()) || sisu_sir::validate::BUILTIN_TYPES.contains(&n.as_str()) {
            continue;
        }
        let Some(d) = (0..prog.defs.len() as u32).map(DefId).find(|d| {
            matches!(prog.def(*d).kind, DefKind::Struct(_) | DefKind::Enum(_) | DefKind::ErrorSet(_))
                && ctx.type_name(*d) == n
        }) else {
            continue;
        };
        let decl = match &prog.def(d).kind {
            DefKind::Struct(s) => sir::TypeDecl::Struct {
                name: n.clone(),
                fields: s
                    .fields
                    .iter()
                    .enumerate()
                    .map(|(i, f)| sir::FieldDecl {
                        name: f.name.clone(),
                        ty: ctx.ty(&f.ty),
                        default: f.default.as_ref().map(|_| ctx.field_default_name(d, i as u32)),
                    })
                    .collect(),
                drop: s.drop.map(|x| ctx.fn_name(x)),
                view: s.view,
                must_consume: s.must_consume,
            },
            DefKind::Enum(e) | DefKind::ErrorSet(e) => sir::TypeDecl::Enum {
                name: n.clone(),
                variants: e
                    .variants
                    .iter()
                    .map(|v| sir::VariantDecl {
                        name: v.name.clone(),
                        fields: v.fields.iter().map(|(fname, t)| (fname.clone(), ctx.ty(t))).collect(),
                    })
                    .collect(),
                drop: e.drop.map(|x| ctx.fn_name(x)),
                error: prog.is_error_set(d),
                open: e.open,
                must_consume: e.must_consume,
            },
            _ => continue,
        };
        let mut inner = HashSet::new();
        match &decl {
            sir::TypeDecl::Struct { fields, .. } => fields.iter().for_each(|f| walk(&f.ty, &mut inner)),
            sir::TypeDecl::Enum { variants, .. } => {
                variants.iter().for_each(|v| v.fields.iter().for_each(|(_, t)| walk(t, &mut inner)))
            }
        }
        queue.extend(inner);
        decls.push((d, decl));
    }
    decls.sort_by_key(|(d, _)| *d);
    decls.into_iter().map(|(_, t)| t).collect()
}

#[cfg(test)]
mod tests;
