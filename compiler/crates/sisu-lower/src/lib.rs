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

mod contracts;
mod exprs;
mod func;

pub use contracts::ContractLevel;
mod glue;
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
    /// The prefix of the names of each trait impl's methods: `Circle.Shape`.
    pub impl_names: HashMap<DefId, String>,
    /// Which contracts the build checks.
    pub level: ContractLevel,
    /// The names of the program's own free functions (and the ones the
    /// compiler writes for it): a prelude function of the same name, which
    /// the program's hides, is `std.NAME` in SIR.
    pub root_fns: HashSet<String>,
    /// Each declared type by its SIR name (the first of a name).
    pub type_defs: HashMap<String, DefId>,
}

impl Ctx<'_> {
    pub fn fn_name(&self, d: DefId) -> String {
        let prog = self.prog;
        let def = prog.def(d);
        if let Some(i) = prog.tests.iter().position(|t| *t == d) {
            return format!("test#{i}");
        }
        if let Some(i) = prog.benches.iter().position(|t| *t == d) {
            return format!("bench#{i}");
        }
        if let Some(f) = prog.fn_info(d) {
            // A refinement's predicate, which the compiler writes.
            if let sisu_resolve::FnKind::Predicate(r) = f.kind {
                return format!("{}#where", prog.qualified_name(r));
            }
            // An instance of a template: `to_json#Point` (08).
            if let Some((t, args)) = &f.instance_of {
                let shown: Vec<String> = args.iter().map(|a| prog.display(a)).collect();
                let key: String = shown
                    .join(",")
                    .chars()
                    .map(|c| if c.is_alphanumeric() || c == '.' || c == '_' { c } else { '_' })
                    .collect();
                return format!("{}#{key}", self.fn_name(*t));
            }
            if let Some(i) = f.impl_ {
                let prefix = self.impl_names.get(&i).cloned().unwrap_or_else(|| "impl".into());
                return format!("{prefix}.{}", def.name);
            }
            if let Some(t) = f.trait_ {
                return format!("{}.{}", prog.qualified_name(t), def.name);
            }
        }
        match prog.fn_info(d).and_then(|f| f.owner.as_ref()) {
            Some(owner) => format!("{}.{}", self.owner_name(owner), def.name),
            None => {
                // The generators the compiler writes (`<arbitrary>/...`)
                // are bound in the root module, by names a program cannot
                // declare again.
                let generated = prog.modules[def.module.0 as usize].name.starts_with("<arbitrary>/");
                if def.module == prog.prelude && self.root_fns.contains(&def.name) {
                    format!("std.{}", def.name)
                } else if def.module == prog.root || def.module == prog.prelude || generated {
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
            Ty::Unit => "unit".into(),
            Ty::Tuple(es, _) => format!("tuple{}", es.len()),
            Ty::Ref(..) => "ref".into(),
            Ty::Fn(_) => "fn".into(),
            Ty::Dyn(tr) => format!("dyn{}", self.prog.def(tr.def).name),
            Ty::ErrSet(ms) if ms.len() == 1 => self.type_name(ms[0]),
            _ => "any".into(),
        }
    }

    /// Names every trait impl's methods: `Type.Trait.method`, with `#2` and
    /// on for a second impl of one trait for one type constructor.
    fn name_impls(&mut self) {
        let prog = self.prog;
        let mut used: HashMap<String, u32> = HashMap::new();
        for &i in &prog.impls {
            let Some(info) = prog.impl_info(i) else { continue };
            let base =
                format!("{}.{}", self.owner_name(&info.self_ty), prog.qualified_name(info.trait_ref.def));
            let n = used.entry(base.clone()).or_insert(0);
            *n += 1;
            let name = if *n == 1 { base } else { format!("{base}#{n}") };
            self.impl_names.insert(i, name);
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
            Ty::Unit | Ty::Var(_) | Ty::Error => sir::Type::Unit,
            Ty::Param(i) => sir::Type::Param(*i),
            Ty::Const(n) => sir::Type::Const(*n),
            Ty::ConstOp(op, args) => sir::Type::ConstOp(*op, args.iter().map(|a| self.ty(a)).collect()),
            Ty::Dyn(tr) => {
                sir::Type::Dyn(prog.qualified_name(tr.def), tr.args.iter().map(|a| self.ty(a)).collect())
            }
            Ty::Assoc(a) => sir::Type::Assoc(
                Box::new(self.ty(&a.self_ty)),
                prog.qualified_name(a.trait_.def),
                a.trait_.args.iter().map(|t| self.ty(t)).collect(),
                a.name.clone(),
            ),
            Ty::Never => sir::Type::Never,
            // A refinement is represented as its base (07).
            Ty::Adt(d, args) if prog.refinement(*d).is_some() => {
                let base = prog.refinement_base(*d, args).unwrap_or(Ty::Error);
                self.ty(&base)
            }
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

/// Lowers a checked program with every contract checked. The program
/// must have no errors; lowering does not report diagnostics.
pub fn lower(prog: &Program, typed: &Typed, sources: &SourceMap) -> sir::Program {
    lower_with(prog, typed, sources, ContractLevel::All)
}

/// [`lower`] at a level of contract checking (07).
pub fn lower_with(prog: &Program, typed: &Typed, sources: &SourceMap, level: ContractLevel) -> sir::Program {
    lower_for(prog, typed, sources, level, &[])
}

/// `lower_with`, keeping the functions `extra` names too (and what they
/// reach): the ones the run time calls for a transcript test (17).
pub fn lower_for(
    prog: &Program,
    typed: &Typed,
    sources: &SourceMap,
    level: ContractLevel,
    extra: &[&str],
) -> sir::Program {
    lower_reaching(prog, typed, sources, level, extra, false, None)
}

/// [`lower_with`], reusing what `cache` holds: a body is lowered again only
/// when its typed body changed (its [`sisu_typeck::Memo`] key) or its item
/// moved to another line (docs/architecture.md, "The query layer"). The
/// result is the same as [`lower_with`]'s.
pub fn lower_cached(
    prog: &Program,
    typed: &Typed,
    sources: &SourceMap,
    level: ContractLevel,
    cache: &mut LowerCache,
) -> sir::Program {
    lower_reaching(prog, typed, sources, level, &[], false, Some(cache))
}

/// The per-item memo of lowering, kept by the query layer between
/// lowerings of one program: each body's SIR functions by the body's memo
/// key, the line its item starts on and the contract level.
#[derive(Default)]
pub struct LowerCache {
    entries: HashMap<(u64, u32, ContractLevel), (Vec<sir::Function>, u64)>,
    generation: u64,
    /// The bodies the last lowering lowered, and the ones it reused.
    pub lowered: u32,
    pub reused: u32,
}

impl LowerCache {
    /// Drops what the last lowering did not use, and starts another.
    fn begin(&mut self) {
        let g = self.generation;
        self.entries.retain(|_, (_, used)| *used == g);
        self.generation += 1;
        self.lowered = 0;
        self.reused = 0;
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Lowers a body, through the memo when there is one. An entry is kept
/// only when every location in its functions lies on the lines of the
/// body's own item, so that the same key always gives the same SIR.
fn lower_body_memo(ctx: &mut Ctx, body: &Body, cache: &mut Option<&mut LowerCache>) -> Vec<sir::Function> {
    let Some(cache) = cache.as_mut() else { return func::lower_body(ctx, body) };
    let m = body.memo;
    let Some(f) = ctx.sources.get(m.file).filter(|_| m.key != 0) else {
        cache.lowered += 1;
        return func::lower_body(ctx, body);
    };
    let (first, last) = (f.line_col(m.start).line, f.line_col(m.end).line);
    let key = (m.key, first, ctx.level);
    let generation = cache.generation;
    if let Some((fs, used)) = cache.entries.get_mut(&key) {
        *used = generation;
        cache.reused += 1;
        return fs.clone();
    }
    cache.lowered += 1;
    let fs = func::lower_body(ctx, body);
    let inside =
        |l: &Option<sir::Loc>| l.is_none_or(|l| first <= l.line && l.line <= last && l.end_line <= last);
    let keep = fs.iter().all(|f| {
        inside(&f.loc)
            && f.locals.iter().all(|l| inside(&l.loc))
            && f.blocks.iter().all(|b| inside(&b.loc) && b.stmts.iter().all(|s| inside(&s.loc)))
    });
    if keep {
        cache.entries.insert(key, (fs.clone(), generation));
    }
    fs
}

/// Every body lowered, whether the program reaches it or not: what the
/// test that puts the whole standard library through the ownership
/// checker checks.
pub fn lower_every_body(
    prog: &Program,
    typed: &Typed,
    sources: &SourceMap,
    level: ContractLevel,
) -> sir::Program {
    lower_reaching(prog, typed, sources, level, &[], true, None)
}

fn lower_reaching(
    prog: &Program,
    typed: &Typed,
    sources: &SourceMap,
    level: ContractLevel,
    extra: &[&str],
    every_body: bool,
    mut cache: Option<&mut LowerCache>,
) -> sir::Program {
    if let Some(c) = cache.as_mut() {
        c.begin();
    }
    let mut ctx = Ctx {
        prog,
        sources,
        files: HashMap::new(),
        file_names: Vec::new(),
        impl_names: HashMap::new(),
        level,
        root_fns: HashSet::new(),
        type_defs: HashMap::new(),
    };
    for d in (0..prog.defs.len() as u32).map(DefId) {
        if matches!(prog.def(d).kind, DefKind::Struct(_) | DefKind::Enum(_) | DefKind::ErrorSet(_)) {
            ctx.type_defs.entry(ctx.type_name(d)).or_insert(d);
        }
    }
    ctx.root_fns = prog
        .defs
        .iter()
        .filter(|d| {
            let m = &prog.modules[d.module.0 as usize];
            (d.module == prog.root || m.name.starts_with("<arbitrary>/"))
                && matches!(&d.kind, DefKind::Fn(f) if f.owner.is_none() && f.trait_.is_none() && f.impl_.is_none())
        })
        .map(|d| d.name.clone())
        .collect();
    ctx.name_impls();
    // The root module's file is source 0.
    if let Some(m) = prog.modules.get(prog.root.0 as usize) {
        ctx.source_index(m.file);
    }
    // Each body by the name of the function it lowers to. A body is lowered
    // when a kept function first names it; its files are numbered in the
    // order the bodies come, as lowering every body numbered them.
    let mut bodies: HashMap<String, usize> = HashMap::new();
    for (i, b) in typed.bodies.iter().enumerate() {
        let is_intrinsic =
            matches!(b.kind, BodyKind::Fn) && prog.fn_info(b.def).is_some_and(|f| f.intrinsic.is_some());
        if is_intrinsic {
            continue;
        }
        ctx.source_index(b.span.file);
        bodies.entry(ctx.body_name(b)).or_insert(i);
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
    roots.extend(extra.iter().map(|s| s.to_string()));
    // A test that declares a `System` gets `System.sim` in the `sim`
    // profile, which the run time calls (17).
    if prog.tests.iter().any(|d| prog.fn_info(*d).is_some_and(|f| f.params.len() == 1)) {
        roots.push("System.sim".to_string());
    }
    // A constant is computed even when nothing uses it: a `comptime` in it
    // that fails is an error of the program (08).
    for b in &typed.bodies {
        if prog.def(b.def).module == prog.root && matches!(b.kind, BodyKind::Fn | BodyKind::Const) {
            roots.push(ctx.body_name(b));
        }
    }
    if every_body {
        let mut all: Vec<String> = bodies.keys().cloned().collect();
        all.sort();
        roots.extend(all);
    }
    let mut functions = lower_reached(&mut ctx, typed, &bodies, &roots, &mut cache);
    let types = collect_types(&ctx, &functions, &HashSet::new());
    functions.extend(glue::glue_functions(&types));
    let intrinsics = intrinsic_decls(prog, &functions);
    let (trait_methods, impls) = trait_decls(&ctx, &functions);
    let sources = renumber_sources(&ctx.file_names, &mut functions);
    sir::Program { sources, entry, types, intrinsics, functions, tests, trait_methods, impls }
}

/// The trait methods the functions call through `call trait`, with their
/// facts, and the impls of their traits, which the interpreter dispatches
/// to.
fn trait_decls(ctx: &Ctx, functions: &[sir::Function]) -> (Vec<sir::IntrinsicDecl>, Vec<sir::ImplDecl>) {
    let prog = ctx.prog;
    let called = trait_calls(functions);
    let mut methods = Vec::new();
    let mut traits: Vec<DefId> = Vec::new();
    for name in &called {
        let Some(m) = trait_method_def(ctx, name) else { continue };
        let Some(f) = prog.fn_info(m) else { continue };
        let b = prog.result_borrows_of(f);
        let own = prog
            .sig_params(f)
            .iter()
            .enumerate()
            .filter(|(_, p)| p.mode == sisu_syntax::ast::Mode::Own)
            .map(|(i, _)| i as u32)
            .collect();
        methods.push(sir::IntrinsicDecl {
            name: name.clone(),
            pure: f.pure,
            from: b.deep,
            inner: b.inner,
            own,
            keeps: f.keeps.clone(),
        });
        if let Some(t) = f.trait_
            && !traits.contains(&t)
        {
            traits.push(t);
        }
    }
    let kept: HashSet<&str> = functions.iter().map(|f| f.name.as_str()).collect();
    // The traits whose associated types the functions name: the interpreter
    // finds `<T as Trait>.Name` in the impl for `T`.
    let mut projected: HashSet<String> = HashSet::new();
    let mut mentioned: HashSet<String> = HashSet::new();
    for f in functions {
        for l in &f.locals {
            assoc_traits(&l.ty, &mut projected);
            type_names(&l.ty, &mut mentioned);
        }
        for b in &f.blocks {
            for st in &b.stmts {
                let tys: &[sir::Type] = match &st.kind {
                    sir::StatementKind::Call {
                        callee:
                            sir::Callee::Fn(_, t) | sir::Callee::Trait(_, t) | sir::Callee::Intrinsic(_, t),
                        ..
                    } => t,
                    sir::StatementKind::Assign(_, sir::Rvalue::Closure(_, t, _)) => t,
                    _ => &[],
                };
                tys.iter().for_each(|t| assoc_traits(t, &mut projected));
            }
        }
    }
    let mut impls = Vec::new();
    for &i in &prog.impls {
        let Some(info) = prog.impl_info(i) else { continue };
        let trait_name = prog.qualified_name(info.trait_ref.def);
        let dispatched = traits.contains(&info.trait_ref.def);
        let self_ty = ctx.ty(&info.self_ty);
        let has_assoc = !info.assoc.is_empty()
            && projected.contains(&trait_name)
            && match &self_ty {
                sir::Type::Named(n, _) => mentioned.contains(n),
                _ => true,
            };
        if !dispatched && !has_assoc {
            continue;
        }
        // An impl for a type no kept function makes was pruned with its
        // methods.
        let pruned = info.methods.iter().any(|(_, m)| {
            prog.fn_info(*m).is_some_and(|f| f.body.is_some() && f.intrinsic.is_none())
                && !kept.contains(ctx.fn_name(*m).as_str())
        });
        if pruned && !has_assoc {
            continue;
        }
        let methods = info
            .methods
            .iter()
            .filter(|(_, m)| prog.fn_info(*m).is_some_and(|f| f.body.is_some() || f.intrinsic.is_some()))
            .filter_map(|(n, m)| {
                let target = match prog.fn_info(*m).and_then(|f| f.intrinsic.clone()) {
                    Some(intr) => sir::ImplTarget::Intrinsic(intr),
                    None if kept.contains(ctx.fn_name(*m).as_str()) => sir::ImplTarget::Fn(ctx.fn_name(*m)),
                    None => return None,
                };
                Some((n.clone(), target))
            })
            .collect();
        impls.push(sir::ImplDecl {
            params: info.generics.len() as u32,
            trait_name,
            trait_args: info.trait_ref.args.iter().map(|t| ctx.ty(t)).collect(),
            self_ty,
            methods,
            assoc: info.assoc.iter().map(|(n, t)| (n.clone(), ctx.ty(t))).collect(),
        });
    }
    (methods, impls)
}

/// The traits of the associated types a type names.
fn assoc_traits(t: &sir::Type, out: &mut HashSet<String>) {
    match t {
        sir::Type::Assoc(s, tr, args, _) => {
            out.insert(tr.clone());
            assoc_traits(s, out);
            args.iter().for_each(|a| assoc_traits(a, out));
        }
        sir::Type::Named(_, args) | sir::Type::Dyn(_, args) => args.iter().for_each(|a| assoc_traits(a, out)),
        sir::Type::Tuple(es) => es.iter().for_each(|e| assoc_traits(e, out)),
        sir::Type::Fn(ps, r, _) => {
            ps.iter().for_each(|p| assoc_traits(p, out));
            assoc_traits(r, out);
        }
        sir::Type::Ref(t, _) => assoc_traits(t, out),
        _ => {}
    }
}

/// The names of the trait methods called through `call trait`.
fn trait_calls(functions: &[sir::Function]) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for f in functions {
        for b in &f.blocks {
            for s in &b.stmts {
                if let sir::StatementKind::Call { callee: sir::Callee::Trait(n, _), .. } = &s.kind
                    && !names.contains(n)
                {
                    names.push(n.clone());
                }
            }
        }
    }
    names.sort();
    names
}

/// The trait method a `Trait.method` name stands for.
fn trait_method_def(ctx: &Ctx, name: &str) -> Option<DefId> {
    let prog = ctx.prog;
    let (t, m) = name.rsplit_once('.')?;
    (0..prog.defs.len() as u32).map(DefId).find_map(|d| {
        if matches!(prog.def(d).kind, DefKind::Trait(_)) && prog.qualified_name(d) == t {
            prog.trait_method(d, m)
        } else {
            None
        }
    })
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
            && let Some(n) = f.intrinsic.as_ref().or(f.structural.as_ref())
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
                sir::IntrinsicDecl {
                    name,
                    pure: f.pure,
                    from: b.deep,
                    inner: b.inner,
                    own,
                    keeps: f.keeps.clone(),
                }
            }
            None => {
                let (from, inner): (Vec<u32>, Vec<u32>) = match name.as_str() {
                    "iter.ref" | "iter.mut" | "iter.own" | "gen.new" => (vec![0], vec![]),
                    "list.slice" | "list.slice_middle" => (vec![], vec![0]),
                    _ => (vec![], vec![]),
                };
                // `iter.own` takes the collection and keeps every element;
                // a generator keeps its body, and `yield` gives its item
                // away; a capability handle keeps its object for good.
                let own = if matches!(name.as_str(), "iter.own" | "gen.new" | "gen.yield" | "cap.new") {
                    vec![0]
                } else {
                    Vec::new()
                };
                let keeps = own.clone();
                // `select` observes the schedule (06, "Effects").
                let pure = name != "select.wait";
                sir::IntrinsicDecl { name, pure, from, inner, own, keeps }
            }
        })
        .collect()
}

/// The functions the roots reach. A call through a trait reaches the
/// trait's provided body and the methods of every impl of the trait for a
/// type some kept function names: a value of a type that no kept function
/// mentions is never made, so it is never dispatched to.
/// The body a function's name comes from: its own, or the one its closure
/// or `comptime` expression is in (`f#closure2`, `T#field#comptime0`).
fn owner_body(name: &str, bodies: &HashMap<String, usize>) -> Option<usize> {
    let mut n = name;
    loop {
        if let Some(&i) = bodies.get(n) {
            return Some(i);
        }
        let cut = n.rfind("#closure").into_iter().chain(n.rfind("#comptime")).max()?;
        n = &n[..cut];
    }
}

/// Lowers what the roots reach, and only that: from each kept function,
/// the functions it calls or names, the defaults, drops and invariant
/// checks of the types it uses, and the methods of the impls of the
/// traits it calls through `call trait` for the types the kept functions
/// mention. The functions come in the order of their bodies.
fn lower_reached(
    ctx: &mut Ctx,
    typed: &Typed,
    bodies: &HashMap<String, usize>,
    roots: &[String],
    cache: &mut Option<&mut LowerCache>,
) -> Vec<sir::Function> {
    let prog = ctx.prog;
    let mut lowered: HashMap<usize, Vec<sir::Function>> = HashMap::new();
    let mut where_is: HashMap<String, usize> = HashMap::new();
    let mut dispatched: HashSet<DefId> = HashSet::new();
    let mut added_impls: HashSet<DefId> = HashSet::new();
    let mut keep: HashSet<String> = HashSet::new();
    let mut mentioned: HashSet<String> = HashSet::new();
    let mut typed_names: HashSet<String> = HashSet::new();
    let mut work: Vec<String> = roots.to_vec();
    loop {
        while let Some(n) = work.pop() {
            if !keep.insert(n.clone()) {
                continue;
            }
            if !where_is.contains_key(&n)
                && let Some(b) = owner_body(&n, bodies)
                && !lowered.contains_key(&b)
            {
                let fs = lower_body_memo(ctx, &typed.bodies[b], cache);
                for f in &fs {
                    where_is.insert(f.name.clone(), b);
                }
                lowered.insert(b, fs);
            }
            let Some(f) =
                where_is.get(&n).and_then(|b| lowered.get(b)).and_then(|fs| fs.iter().find(|f| f.name == n))
            else {
                continue;
            };
            for l in &f.locals {
                type_names(&l.ty, &mut mentioned);
            }
            reach_from(ctx, f, &mut work, &mut dispatched);
            // Defaults and drops of declared types are reached through the
            // types, and so is a type's invariant check, which the run time
            // calls when unwinding drops a mutex guard (D-077). The types
            // met already are not looked at again: what their fields reach
            // was met with them.
            for t in collect_types(ctx, std::slice::from_ref(f), &typed_names) {
                if !typed_names.insert(t.name().to_string()) {
                    continue;
                }
                match t {
                    sir::TypeDecl::Struct { name, fields, drop, .. } => {
                        work.extend(fields.iter().filter_map(|f| f.default.clone()));
                        work.extend(drop);
                        work.push(format!("{name}.__invariant"));
                    }
                    sir::TypeDecl::Enum { drop, .. } => work.extend(drop),
                }
            }
        }
        for &i in &prog.impls {
            let Some(info) = prog.impl_info(i) else { continue };
            if added_impls.contains(&i) || !dispatched.contains(&info.trait_ref.def) {
                continue;
            }
            let needed = match ctx.ty(&info.self_ty) {
                sir::Type::Named(n, _) => mentioned.contains(&n),
                _ => true,
            };
            if needed {
                added_impls.insert(i);
                work.extend(info.methods.iter().map(|(_, m)| ctx.fn_name(*m)));
            }
        }
        if work.is_empty() {
            break;
        }
    }
    let mut order: Vec<usize> = lowered.keys().copied().collect();
    order.sort_unstable();
    order
        .into_iter()
        .flat_map(|b| lowered.remove(&b).unwrap_or_default())
        .filter(|f| keep.contains(&f.name))
        .collect()
}

/// The names of the named types in a type.
fn type_names(t: &sir::Type, out: &mut HashSet<String>) {
    match t {
        sir::Type::Named(n, args) => {
            out.insert(n.clone());
            args.iter().for_each(|a| type_names(a, out));
        }
        sir::Type::Tuple(es) => es.iter().for_each(|e| type_names(e, out)),
        sir::Type::Fn(ps, r, _) => {
            ps.iter().for_each(|p| type_names(p, out));
            type_names(r, out);
        }
        sir::Type::Ref(t, _) => type_names(t, out),
        sir::Type::Dyn(_, args) => args.iter().for_each(|a| type_names(a, out)),
        sir::Type::Assoc(t, _, args, _) => {
            type_names(t, out);
            args.iter().for_each(|a| type_names(a, out));
        }
        sir::Type::ErrSet(ms) => out.extend(ms.iter().cloned()),
        _ => {}
    }
}

/// What one function reaches: its callees, the functions it names as
/// values, and the traits it calls through.
fn reach_from(ctx: &Ctx, f: &sir::Function, work: &mut Vec<String>, dispatched: &mut HashSet<DefId>) {
    let prog = ctx.prog;
    for b in &f.blocks {
        for s in &b.stmts {
            match &s.kind {
                sir::StatementKind::Call { callee: sir::Callee::Fn(c, _), args, .. } => {
                    work.push(c.clone());
                    refs_in_operands(args, work);
                }
                sir::StatementKind::Call { callee: sir::Callee::Trait(c, _), args, .. } => {
                    refs_in_operands(args, work);
                    work.push(c.clone());
                    if let Some(m) = trait_method_def(ctx, c)
                        && let Some(t) = prog.fn_info(m).and_then(|f| f.trait_)
                    {
                        dispatched.insert(t);
                    }
                }
                sir::StatementKind::Call { args, .. } => refs_in_operands(args, work),
                sir::StatementKind::Assign(_, r) => match r {
                    sir::Rvalue::Closure(c, _, ops) => {
                        work.push(c.clone());
                        refs_in_operands(ops, work);
                    }
                    sir::Rvalue::Use(o) => refs_in_operands(std::slice::from_ref(o), work),
                    sir::Rvalue::Aggregate(_, ops) => refs_in_operands(ops, work),
                    _ => {}
                },
                _ => {}
            }
        }
    }
}

/// The sources the kept functions come from: the root module's file
/// first, then the others in the order they were first met; each
/// function's index is rewritten to match.
fn renumber_sources(names: &[String], functions: &mut [sir::Function]) -> Vec<String> {
    let mut map: HashMap<u32, u32> = HashMap::new();
    let mut out = Vec::new();
    if let Some(root) = names.first() {
        map.insert(0, 0);
        out.push(root.clone());
    }
    let mut used: Vec<u32> = functions.iter().map(|f| f.source).collect();
    used.sort_unstable();
    used.dedup();
    for i in used {
        if let (std::collections::hash_map::Entry::Vacant(e), Some(n)) = (map.entry(i), names.get(i as usize))
        {
            e.insert(out.len() as u32);
            out.push(n.clone());
        }
    }
    for f in functions {
        f.source = map.get(&f.source).copied().unwrap_or(0);
    }
    out
}

fn refs_in_operands(ops: &[sir::Operand], work: &mut Vec<String>) {
    for o in ops {
        if let sir::Operand::Const(sir::Const::Fn(n)) = o {
            work.push(n.clone());
        }
    }
}

/// The declarations of every non-builtin type the functions mention, in
/// definition order, but for those named in `known` and the types their
/// fields reach (which `known` must hold as well).
fn collect_types(ctx: &Ctx, functions: &[sir::Function], known: &HashSet<String>) -> Vec<sir::TypeDecl> {
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
            sir::Type::Dyn(_, args) => args.iter().for_each(|a| walk(a, out)),
            sir::Type::Assoc(t, _, args, _) => {
                walk(t, out);
                args.iter().for_each(|a| walk(a, out));
            }
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
                if let sir::StatementKind::Assign(_, sir::Rvalue::Aggregate(sir::Aggregate::Dyn(t), _)) =
                    &s.kind
                {
                    walk(t, &mut names);
                }
                if let sir::StatementKind::Call {
                    callee:
                        sir::Callee::Fn(_, tys) | sir::Callee::Trait(_, tys) | sir::Callee::Intrinsic(_, tys),
                    ..
                } = &s.kind
                {
                    tys.iter().for_each(|t| walk(t, &mut names));
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
        if known.contains(&n)
            || !done.insert(n.clone())
            || sisu_sir::validate::BUILTIN_TYPES.contains(&n.as_str())
        {
            continue;
        }
        let Some(&d) = ctx.type_defs.get(&n) else {
            continue;
        };
        let decl = match &prog.def(d).kind {
            DefKind::Struct(s) => sir::TypeDecl::Struct {
                name: n.clone(),
                params: s.generics.iter().map(|g| g.name.clone()).collect(),
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
                params: e.generics.iter().map(|g| g.name.clone()).collect(),
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
