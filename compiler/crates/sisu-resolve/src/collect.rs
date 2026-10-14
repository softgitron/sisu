//! The resolver: declares every item of every module, loads the std
//! modules an import names, resolves signatures to [`Ty`], and registers
//! methods. Bodies are left to the type checker.

use crate::program::*;
use crate::std_sources;
use crate::ty::{FnTy, Ty};
use sisu_diag::{Diagnostic, FileId, MsgId, SourceMap, Span, TextRange, msg};
use sisu_syntax::ast::{self, ExprKind, ItemKind, ParamKind, TypeKind};
use std::collections::HashMap;

/// The resolver's output.
#[derive(Clone, Debug)]
pub struct Resolved {
    pub program: Program,
    pub diagnostics: Vec<Diagnostic>,
}

/// Type names this stage knows but does not implement, with the phase of
/// proposal/18-next-steps.md that brings them.
const LATER_TYPES: &[(&str, &str)] = &[
    ("uint", "the sized numeric types (Phase 4)"),
    ("i8", "the sized numeric types (Phase 4)"),
    ("i16", "the sized numeric types (Phase 4)"),
    ("i32", "the sized numeric types (Phase 4)"),
    ("i64", "the sized numeric types (Phase 4)"),
    ("i128", "the sized numeric types (Phase 4)"),
    ("u8", "the sized numeric types (Phase 4)"),
    ("u16", "the sized numeric types (Phase 4)"),
    ("u32", "the sized numeric types (Phase 4)"),
    ("u64", "the sized numeric types (Phase 4)"),
    ("u128", "the sized numeric types (Phase 4)"),
    ("f32", "the sized numeric types (Phase 4)"),
    ("bytes", "byte strings (Phase 4)"),
    ("Array", "`Array` and `Slice` (Phase 4)"),
    ("Slice", "`Array` and `Slice` (Phase 4)"),
    ("Box", "`Box` and shared ownership (Phase 3)"),
    ("Shared", "`Shared` and shared ownership (Phase 3)"),
    ("Weak", "`Weak` and shared ownership (Phase 3)"),
    ("Cell", "`Cell` and shared ownership (Phase 3)"),
    ("Mutex", "`Mutex` (Phase 3)"),
    ("Arena", "`Arena` and generational keys (Phase 3)"),
    ("Key", "`Arena` and generational keys (Phase 3)"),
    ("Duration", "`Duration` and the time library (Phase 6)"),
    ("Net", "the `Net` capability (Phase 6)"),
    ("Rand", "the `Rand` capability (Phase 6)"),
    ("Proc", "the `Proc` capability (Phase 6)"),
    ("Url", "`std.net` (Phase 6)"),
];

struct Owner {
    generics: Vec<String>,
    self_ty: Ty,
}

enum Pending {
    Fn { def: DefId, module: ModuleId, ast: ast::FnDef, owner: Option<Owner>, drop_for: Option<DefId> },
    Struct { def: DefId, module: ModuleId, ast: ast::StructDef, attrs: Vec<ast::Attr> },
    Enum { def: DefId, module: ModuleId, ast: ast::EnumDef, attrs: Vec<ast::Attr> },
    Impl { module: ModuleId, ast: ast::ImplDef, range: TextRange },
    Test { def: DefId, module: ModuleId, ast: ast::TestDef },
    Const { def: DefId, module: ModuleId, ast: ast::ConstDef },
}

struct Resolver<'a> {
    prog: Program,
    diags: Vec<Diagnostic>,
    sources: &'a mut SourceMap,
    std_loaded: HashMap<&'static str, ModuleId>,
    pending: Vec<Pending>,
    aliases: HashMap<DefId, (ModuleId, ast::TypeAlias)>,
    resolving_alias: Vec<DefId>,
}

struct TyCtx<'g> {
    module: ModuleId,
    generics: &'g [String],
    self_ty: Option<&'g Ty>,
    allow_ref: bool,
}

/// Resolves a program whose root module is `root` (already parsed from
/// `root_file`). The prelude and any imported std modules are added to
/// `sources`.
pub fn resolve(sources: &mut SourceMap, root_file: FileId, root: &ast::Module) -> Resolved {
    let mut r = Resolver {
        prog: Program::default(),
        diags: Vec::new(),
        sources,
        std_loaded: HashMap::new(),
        pending: Vec::new(),
        aliases: HashMap::new(),
        resolving_alias: Vec::new(),
    };
    let prelude = r.load_std("prelude", std_sources::PRELUDE);
    r.prog.prelude = prelude;
    r.fill_well_known();
    let root_id = r.add_module("main".into(), root_file, false);
    r.prog.root = root_id;
    r.prog.poisoned = root.broken_names.iter().cloned().collect();
    r.declare_items(root_id, &root.items, "");
    r.resolve_pending();
    r.build_variant_index();
    r.check_borrow_signatures();
    Resolved { program: r.prog, diagnostics: r.diags }
}

impl Resolver<'_> {
    /// The empty span at the start of `keyword` where it comes last before
    /// `span` on the same line (`struct` before a struct's name).
    fn keyword_before(&self, span: Span, keyword: &str) -> Option<Span> {
        let text = &self.sources.get(span.file)?.text;
        let before = text.get(..span.range.start as usize)?;
        let line_start = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
        let at = before[line_start..].rfind(keyword)? + line_start;
        Some(Span::new(span.file, TextRange::new(at as u32, at as u32)))
    }

    fn span(&self, module: ModuleId, range: TextRange) -> Span {
        Span::new(self.prog.modules[module.0 as usize].file, range)
    }

    fn file(&self, module: ModuleId) -> FileId {
        self.prog.modules[module.0 as usize].file
    }

    fn is_std(&self, module: ModuleId) -> bool {
        self.prog.modules[module.0 as usize].is_std
    }

    fn report(&mut self, d: Diagnostic) {
        self.diags.push(d);
    }

    fn unsupported(&mut self, span: Span, construct: &str, phase: &str) {
        self.report(
            Diagnostic::new(msg::STAGE0_UNSUPPORTED, span).with("construct", construct).with("phase", phase),
        );
    }

    fn add_module(&mut self, name: String, file: FileId, is_std: bool) -> ModuleId {
        let id = ModuleId(self.prog.modules.len() as u32);
        self.prog.modules.push(ModuleData { name, file, items: HashMap::new(), order: Vec::new(), is_std });
        id
    }

    fn load_std(&mut self, name: &'static str, source: &'static str) -> ModuleId {
        if let Some(&m) = self.std_loaded.get(name) {
            return m;
        }
        let file = self.sources.add(format!("<std>/{name}.sisu"), source);
        let parse = sisu_parser::parse(file, source);
        self.diags.extend(parse.diagnostics);
        let ast = ast::build::module(&parse.tree);
        let m = self.add_module(name.to_string(), file, true);
        self.std_loaded.insert(name, m);
        self.declare_items(m, &ast.items, "");
        m
    }

    fn fill_well_known(&mut self) {
        let p = self.prog.prelude;
        let get = |prog: &Program, n: &str| prog.lookup_in(p, n);
        let wk = WellKnown {
            option: get(&self.prog, "Option"),
            result: get(&self.prog, "Result"),
            ordering: get(&self.prog, "Ordering"),
            string: get(&self.prog, "String"),
            list: get(&self.prog, "list"),
            dict: get(&self.prog, "dict"),
            set: get(&self.prog, "set"),
            iter: get(&self.prog, "Iter"),
            entry: get(&self.prog, "Entry"),
            iterable: get(&self.prog, "Iterable"),
            path: get(&self.prog, "Path"),
            system: get(&self.prog, "System"),
            io_error: get(&self.prog, "IoError"),
            range: get(&self.prog, "range"),
        };
        self.prog.wk = wk;
    }

    fn new_def(
        &mut self,
        module: ModuleId,
        name: &str,
        range: TextRange,
        vis: ast::Vis,
        docs: Vec<String>,
        kind: DefKind,
    ) -> DefId {
        let id = DefId(self.prog.defs.len() as u32);
        let span = self.span(module, range);
        self.prog.defs.push(Def { name: name.to_string(), module, span, vis, docs, kind });
        id
    }

    /// Declares a name in a module, reporting a duplicate.
    fn bind(&mut self, module: ModuleId, name: &str, def: DefId, range: TextRange) {
        if name.is_empty() {
            return;
        }
        let m = &self.prog.modules[module.0 as usize];
        if let Some(&prev) = m.items.get(name) {
            let first = self.prog.def(prev).span;
            let span = self.span(module, range);
            self.report(Diagnostic::new(msg::RESOLVE_DUPLICATE_DEFINITION, span).with("name", name).label(
                first,
                msg::RESOLVE_FIRST_DEFINED,
                Vec::new(),
            ));
            return;
        }
        let m = &mut self.prog.modules[module.0 as usize];
        m.items.insert(name.to_string(), def);
        m.order.push(def);
    }

    fn placeholder_fn(&self, module: ModuleId, kind: FnKind) -> FnInfo {
        FnInfo {
            generics: Vec::new(),
            impl_generics: 0,
            self_mode: None,
            owner: None,
            params: Vec::new(),
            keyword_only_from: None,
            positional_only_until: 0,
            ret: Ty::Unit,
            ret_span: None,
            from: Vec::new(),
            err_inferred: false,
            pure: false,
            intrinsic: None,
            body: None,
            kind,
            file: self.file(module),
            header_broken: false,
            body_broken: false,
        }
    }

    // ---------------------------------------------------------------- pass 1

    fn declare_items(&mut self, module: ModuleId, items: &[ast::Item], test_prefix: &str) {
        for item in items {
            self.check_attrs(module, &item.attrs);
            match &item.kind {
                ItemKind::Fn(f) => {
                    let def = self.new_def(
                        module,
                        &f.name.text,
                        f.name.range,
                        item.vis,
                        item.docs.clone(),
                        DefKind::Fn(self.placeholder_fn(module, FnKind::Free)),
                    );
                    self.bind(module, &f.name.text, def, f.name.range);
                    self.pending.push(Pending::Fn {
                        def,
                        module,
                        ast: f.clone(),
                        owner: None,
                        drop_for: None,
                    });
                }
                ItemKind::Struct(s) => {
                    let info = StructInfo {
                        generics: Vec::new(),
                        fields: Vec::new(),
                        builtin: None,
                        copy: false,
                        drop: None,
                        view: s.is_view,
                        must_consume: false,
                    };
                    let def = self.new_def(
                        module,
                        &s.name.text,
                        s.name.range,
                        item.vis,
                        item.docs.clone(),
                        DefKind::Struct(info),
                    );
                    self.bind(module, &s.name.text, def, s.name.range);
                    self.declare_methods(module, def, &s.generics, &s.methods);
                    self.pending.push(Pending::Struct {
                        def,
                        module,
                        ast: s.clone(),
                        attrs: item.attrs.clone(),
                    });
                }
                ItemKind::Enum(e) | ItemKind::ErrorSet(e) => {
                    let info = EnumInfo {
                        generics: Vec::new(),
                        variants: Vec::new(),
                        open: e.open,
                        copy: false,
                        drop: None,
                        must_consume: false,
                    };
                    let kind = if matches!(item.kind, ItemKind::ErrorSet(_)) {
                        DefKind::ErrorSet(info)
                    } else {
                        DefKind::Enum(info)
                    };
                    let def =
                        self.new_def(module, &e.name.text, e.name.range, item.vis, item.docs.clone(), kind);
                    self.bind(module, &e.name.text, def, e.name.range);
                    self.declare_methods(module, def, &e.generics, &e.methods);
                    self.pending.push(Pending::Enum {
                        def,
                        module,
                        ast: e.clone(),
                        attrs: item.attrs.clone(),
                    });
                }
                ItemKind::Impl(i) => {
                    self.pending.push(Pending::Impl { module, ast: i.clone(), range: item.range })
                }
                ItemKind::Trait(t) => {
                    let span = self.span(module, item.range);
                    self.unsupported(span, "`trait`", "traits (Phase 4)");
                    let def = self.new_def(
                        module,
                        &t.name.text,
                        t.name.range,
                        item.vis,
                        item.docs.clone(),
                        DefKind::Unsupported,
                    );
                    self.bind(module, &t.name.text, def, t.name.range);
                }
                ItemKind::Import(imp) => self.import(module, imp, item.range),
                ItemKind::FromImport(imp) => self.import_from(module, imp, item.range),
                ItemKind::Test(t) => {
                    let name = if test_prefix.is_empty() {
                        t.name.clone()
                    } else {
                        format!("{test_prefix}/{}", t.name)
                    };
                    let def = self.new_def(
                        module,
                        &name,
                        t.name_range,
                        ast::Vis::Private,
                        item.docs.clone(),
                        DefKind::Fn(self.placeholder_fn(module, FnKind::Test(name.clone()))),
                    );
                    self.prog.tests.push(def);
                    self.pending.push(Pending::Test { def, module, ast: t.clone() });
                }
                ItemKind::Suite(s) => {
                    let prefix = if test_prefix.is_empty() {
                        s.name.clone()
                    } else {
                        format!("{test_prefix}/{}", s.name)
                    };
                    self.declare_items(module, &s.items, &prefix);
                }
                ItemKind::Bench(_) => {}
                ItemKind::Const(c) => {
                    let info = ConstInfo { ty: Ty::Error, value: None, file: self.file(module) };
                    let def = self.new_def(
                        module,
                        &c.name.text,
                        c.name.range,
                        item.vis,
                        item.docs.clone(),
                        DefKind::Const(info),
                    );
                    self.bind(module, &c.name.text, def, c.name.range);
                    self.pending.push(Pending::Const { def, module, ast: c.clone() });
                }
                ItemKind::TypeAlias(a) => {
                    if a.refinement {
                        let span = self.span(module, item.range);
                        self.unsupported(span, "a refinement type", "contracts (Phase 8)");
                    }
                    let def = self.new_def(
                        module,
                        &a.name.text,
                        a.name.range,
                        item.vis,
                        item.docs.clone(),
                        DefKind::TypeAlias(Ty::Error),
                    );
                    self.bind(module, &a.name.text, def, a.name.range);
                    self.aliases.insert(def, (module, a.clone()));
                }
                ItemKind::Extern(_) => {
                    let span = self.span(module, item.range);
                    self.unsupported(span, "`extern`", "foreign functions (Phase 8)");
                }
                ItemKind::Stmt(_) | ItemKind::Error => {}
            }
        }
    }

    fn check_attrs(&mut self, module: ModuleId, attrs: &[ast::Attr]) {
        if self.is_std(module) {
            return;
        }
        for a in attrs {
            if a.name == "builtin" || a.name == "intrinsic" {
                let span = self.span(module, a.range);
                self.report(
                    Diagnostic::new(msg::RESOLVE_STD_ONLY_ATTRIBUTE, span).with("name", a.name.clone()),
                );
            }
        }
    }

    /// The `from` clauses and the elision rules of every user function
    /// (04, "Borrowing rules in detail"), once every type is resolved.
    fn check_borrow_signatures(&mut self) {
        let mut found = Vec::new();
        for (i, def) in self.prog.defs.iter().enumerate() {
            let DefKind::Fn(f) = &def.kind else { continue };
            if self.prog.modules[def.module.0 as usize].is_std || f.header_broken {
                continue;
            }
            let shape = self.prog.result_shape(&f.ret);
            let params = self.prog.sig_params(f);
            let ret_span = f.ret_span.unwrap_or(def.span);
            // Where each parameter is written; `self` has no span of its own.
            let param_span =
                |name: &str| f.params.iter().find(|p| p.name == name).map(|p| p.span).unwrap_or(def.span);
            if !f.from.is_empty() {
                if shape == crate::ResultShape::Plain {
                    let span = f.from[0].1;
                    let ty = self.prog.display(&f.ret);
                    // The fix removes ` from a, b`: from the end of the
                    // result type to the end of the last name.
                    let last = f.from.last().map(|(_, s)| s.range.end).unwrap_or(span.range.end);
                    let clause = Span::new(ret_span.file, TextRange::new(ret_span.range.end, last));
                    found.push(
                        Diagnostic::new(msg::BORROW_FROM_WITHOUT_BORROW, span)
                            .with("function", def.name.clone())
                            .with("ty", ty.clone())
                            .label(ret_span, msg::BORROW_RESULT_CANNOT_BORROW, vec![("ty", ty)])
                            .fix(
                                msg::BORROW_REMOVE_FROM,
                                Vec::new(),
                                vec![sisu_diag::Edit { span: clause, text: String::new() }],
                            ),
                    );
                    continue;
                }
                for (name, span) in &f.from {
                    let ok = params.iter().any(|p| p.name == *name && self.prog.is_borrowed_param(p));
                    if !ok {
                        let label = if params.iter().any(|p| p.name == *name) {
                            (param_span(name), msg::BORROW_NOT_BORROWED_PARAM, vec![("name", name.clone())])
                        } else {
                            (
                                def.span,
                                msg::BORROW_NO_SUCH_PARAM,
                                vec![("function", def.name.clone()), ("name", name.clone())],
                            )
                        };
                        found.push(
                            Diagnostic::new(msg::BORROW_FROM_NOT_A_PARAMETER, *span)
                                .with("name", name.clone())
                                .with("function", def.name.clone())
                                .label(label.0, label.1, label.2),
                        );
                    }
                }
                continue;
            }
            if shape == crate::ResultShape::Plain {
                continue;
            }
            let borrowed: Vec<&crate::SigParam> =
                params.iter().filter(|p| self.prog.is_borrowed_param(p)).collect();
            let self_wins = f.self_mode.is_some() && borrowed.first().is_some_and(|p| p.name == "self");
            if borrowed.len() >= 2 && !self_wins {
                let names: Vec<String> = borrowed.iter().map(|p| p.name.clone()).collect();
                let _ = i;
                let mut d = Diagnostic::new(msg::BORROW_FROM_REQUIRED, ret_span)
                    .with("name", def.name.clone())
                    .with("ret", self.prog.display(&f.ret))
                    .with("params", names.join(", "));
                for n in &names {
                    d = d.label(param_span(n), msg::BORROW_BORROWED_PARAM, vec![("name", n.clone())]);
                }
                // The fix names them all, which is always allowed; the
                // author may then drop the ones the result never borrows.
                let at = Span::new(ret_span.file, TextRange::new(ret_span.range.end, ret_span.range.end));
                d = d.fix(
                    msg::BORROW_ADD_FROM,
                    vec![("param", names.join(", "))],
                    vec![sisu_diag::Edit { span: at, text: format!(" from {}", names.join(", ")) }],
                );
                found.push(d);
            }
        }
        self.diags.extend(found);
    }

    fn declare_methods(
        &mut self,
        module: ModuleId,
        owner: DefId,
        generics: &[ast::GenericParam],
        methods: &[ast::FnDef],
    ) {
        let names: Vec<String> = generics.iter().map(|g| g.name.text.clone()).collect();
        let self_ty = Ty::Adt(owner, (0..names.len() as u32).map(Ty::Param).collect());
        for m in methods {
            let def = self.new_def(
                module,
                &m.name.text,
                m.name.range,
                ast::Vis::Pub,
                Vec::new(),
                DefKind::Fn(self.placeholder_fn(module, FnKind::Method)),
            );
            self.pending.push(Pending::Fn {
                def,
                module,
                ast: m.clone(),
                owner: Some(Owner { generics: names.clone(), self_ty: self_ty.clone() }),
                drop_for: None,
            });
        }
    }

    fn import(&mut self, module: ModuleId, imp: &ast::Import, range: TextRange) {
        let span = self.span(module, range);
        if imp.c_header.is_some() {
            self.unsupported(span, "a C header import", "foreign functions (Phase 8)");
            return;
        }
        let path: Vec<&str> = imp.path.iter().map(|n| n.text.as_str()).collect();
        let Some((name, source)) = std_sources::module_for(&path) else {
            let at = match (imp.path.first(), imp.path.last()) {
                (Some(a), Some(b)) => self.span(module, a.range.cover(b.range)),
                _ => span,
            };
            self.report(Diagnostic::new(msg::RESOLVE_IMPORT_UNKNOWN, at).with("name", path.join(".")));
            return;
        };
        let m = self.load_std(name, source);
        let bound = imp
            .alias
            .as_ref()
            .map(|a| a.text.clone())
            .unwrap_or_else(|| path.last().map(|s| s.to_string()).unwrap_or_default());
        let range_of_name =
            imp.alias.as_ref().map(|a| a.range).or(imp.path.last().map(|n| n.range)).unwrap_or(range);
        let def =
            self.new_def(module, &bound, range_of_name, ast::Vis::Private, Vec::new(), DefKind::Module(m));
        self.bind(module, &bound, def, range_of_name);
    }

    fn import_from(&mut self, module: ModuleId, imp: &ast::FromImport, range: TextRange) {
        let span = self.span(module, range);
        let path: Vec<&str> = imp.path.iter().map(|n| n.text.as_str()).collect();
        let Some((name, source)) = std_sources::module_for(&path) else {
            self.report(Diagnostic::new(msg::RESOLVE_IMPORT_UNKNOWN, span).with("name", path.join(".")));
            return;
        };
        let m = self.load_std(name, source);
        for (n, alias) in &imp.names {
            match self.prog.lookup_in(m, &n.text) {
                Some(d) => {
                    let bound = alias.as_ref().unwrap_or(n);
                    let m_items = &self.prog.modules[module.0 as usize].items;
                    if m_items.contains_key(&bound.text) {
                        self.bind(module, &bound.text, d, bound.range);
                    } else {
                        let mm = &mut self.prog.modules[module.0 as usize];
                        mm.items.insert(bound.text.clone(), d);
                    }
                }
                None => {
                    let s = self.span(module, n.range);
                    self.report(
                        Diagnostic::new(msg::RESOLVE_NOT_IN_MODULE, s)
                            .with("module", path.join("."))
                            .with("name", n.text.clone()),
                    );
                }
            }
        }
    }

    // ---------------------------------------------------------------- pass 2

    fn resolve_pending(&mut self) {
        // Aliases first, on demand, so that later declarations can use them.
        let alias_ids: Vec<DefId> = self.aliases.keys().copied().collect();
        let mut alias_ids = alias_ids;
        alias_ids.sort();
        for d in alias_ids {
            self.resolve_alias(d);
        }
        // Types before functions, so method owners have their generics.
        let pending = std::mem::take(&mut self.pending);
        let (types, rest): (Vec<Pending>, Vec<Pending>) =
            pending.into_iter().partition(|p| matches!(p, Pending::Struct { .. } | Pending::Enum { .. }));
        for p in types {
            self.resolve_one(p);
        }
        let mut queue = rest;
        while !queue.is_empty() {
            for p in std::mem::take(&mut queue) {
                self.resolve_one(p);
            }
            // Impls push the functions they declare.
            queue = std::mem::take(&mut self.pending);
        }
    }

    fn resolve_alias(&mut self, d: DefId) -> Ty {
        if let DefKind::TypeAlias(t) = &self.prog.def(d).kind
            && *t != Ty::Error
        {
            return t.clone();
        }
        let Some((module, alias)) = self.aliases.get(&d).cloned() else { return Ty::Error };
        if self.resolving_alias.contains(&d) {
            let span = self.span(module, alias.name.range);
            self.report(
                Diagnostic::new(msg::RESOLVE_ALIAS_CYCLE, span).with("name", alias.name.text.clone()),
            );
            return Ty::Error;
        }
        self.resolving_alias.push(d);
        let ctx = TyCtx { module, generics: &[], self_ty: None, allow_ref: self.is_std(module) };
        let ty = match &alias.ty {
            Some(t) => self.resolve_ty(t, &ctx),
            None => Ty::Error,
        };
        self.resolving_alias.pop();
        self.prog.defs[d.0 as usize].kind = DefKind::TypeAlias(ty.clone());
        ty
    }

    fn generics_allowed(&mut self, module: ModuleId, generics: &[ast::GenericParam]) -> Vec<String> {
        if !generics.is_empty() && !self.is_std(module) {
            let range = generics[0].name.range.cover(generics[generics.len() - 1].name.range);
            let span = self.span(module, range);
            self.report(Diagnostic::new(msg::RESOLVE_USER_GENERICS, span));
        }
        generics.iter().map(|g| g.name.text.clone()).collect()
    }

    fn has_derive(attrs: &[ast::Attr], what: &str) -> bool {
        attrs.iter().any(|a| {
            a.name == "derive"
                && a.args.iter().any(|arg| matches!(&arg.value.kind, ExprKind::Name(n) if n == what))
        })
    }

    fn builtin_attr(attrs: &[ast::Attr], name: &str) -> Option<String> {
        attrs.iter().find(|a| a.name == name).and_then(|a| a.args.first()).and_then(|arg| {
            match &arg.value.kind {
                ExprKind::Str(parts) => Some(ast::build::plain_text(parts)),
                _ => None,
            }
        })
    }

    fn resolve_one(&mut self, p: Pending) {
        match p {
            Pending::Struct { def, module, ast: s, attrs } => {
                let generics = self.generics_allowed(module, &s.generics);
                let self_ty = Ty::Adt(def, (0..generics.len() as u32).map(Ty::Param).collect());
                let std = self.is_std(module);
                let mut fields: Vec<FieldInfo> = Vec::new();
                for f in &s.fields {
                    if fields.iter().any(|x| x.name == f.name.text) {
                        let span = self.span(module, f.name.range);
                        self.report(
                            Diagnostic::new(msg::RESOLVE_DUPLICATE_FIELD, span)
                                .with("name", f.name.text.clone())
                                .with("owner", s.name.text.clone()),
                        );
                        continue;
                    }
                    let ctx = TyCtx { module, generics: &generics, self_ty: Some(&self_ty), allow_ref: true };
                    let ty = self.resolve_ty(&f.ty, &ctx);
                    // Views are infectious upwards (04): only a `view struct`
                    // may hold a reference or a view.
                    if !std && !s.is_view && self.prog.is_view_type(&ty) {
                        let span = self.span(module, f.name.range.cover(f.ty.range));
                        let fty = self.prog.display(&ty);
                        let name_span = self.span(module, s.name.range);
                        // The fix writes `view ` before the `struct` keyword.
                        let edits = self
                            .keyword_before(name_span, "struct")
                            .map(|at| vec![sisu_diag::Edit { span: at, text: "view ".into() }])
                            .unwrap_or_default();
                        self.report(
                            Diagnostic::new(msg::BORROW_VIEW_FIELD_IN_PLAIN_STRUCT, span)
                                .with("ty", s.name.text.clone())
                                .with("field", f.name.text.clone())
                                .with("fty", fty)
                                .label(
                                    name_span,
                                    msg::BORROW_TYPE_DECLARED_HERE,
                                    vec![("ty", s.name.text.clone())],
                                )
                                .fix(msg::BORROW_DECLARE_VIEW, vec![("ty", s.name.text.clone())], edits),
                        );
                    }
                    fields.push(FieldInfo {
                        name: f.name.text.clone(),
                        ty,
                        default: f.default.clone(),
                        span: self.span(module, f.range),
                    });
                }
                for (range, what) in &s.unsupported {
                    let span = self.span(module, *range);
                    self.unsupported(span, what, "contracts (Phase 8)");
                }
                let builtin = Self::builtin_attr(&attrs, "builtin");
                let copy = Self::has_derive(&attrs, "Copy");
                let must_consume = attrs.iter().any(|a| a.name == "must_consume");
                if let DefKind::Struct(info) = &mut self.prog.defs[def.0 as usize].kind {
                    info.generics = generics;
                    info.fields = fields;
                    info.builtin = builtin;
                    info.copy = copy;
                    info.must_consume = must_consume;
                }
            }
            Pending::Enum { def, module, ast: e, attrs } => {
                let generics = self.generics_allowed(module, &e.generics);
                let self_ty = Ty::Adt(def, (0..generics.len() as u32).map(Ty::Param).collect());
                let std = self.is_std(module);
                let mut variants: Vec<VariantInfo> = Vec::new();
                for v in &e.variants {
                    if variants.iter().any(|x| x.name == v.name.text) {
                        let span = self.span(module, v.name.range);
                        self.report(
                            Diagnostic::new(msg::RESOLVE_DUPLICATE_VARIANT, span)
                                .with("name", v.name.text.clone())
                                .with("owner", e.name.text.clone()),
                        );
                        continue;
                    }
                    let ctx = TyCtx { module, generics: &generics, self_ty: Some(&self_ty), allow_ref: std };
                    let mut fields = Vec::new();
                    for f in &v.fields {
                        let ty = self.resolve_ty(&f.ty, &ctx);
                        // An enum has no `view` form, so it cannot hold a borrow.
                        if !std && self.prog.is_view_type(&ty) {
                            let span = self.span(module, f.ty.range);
                            let fty = self.prog.display(&ty);
                            let name_span = self.span(module, e.name.range);
                            self.report(
                                Diagnostic::new(msg::BORROW_VIEW_IN_ENUM, span)
                                    .with("ty", e.name.text.clone())
                                    .with("fty", fty)
                                    .label(
                                        name_span,
                                        msg::BORROW_TYPE_DECLARED_HERE,
                                        vec![("ty", e.name.text.clone())],
                                    ),
                            );
                        }
                        fields.push((f.name.as_ref().map(|n| n.text.clone()), ty));
                    }
                    variants.push(VariantInfo {
                        name: v.name.text.clone(),
                        fields,
                        span: self.span(module, v.range),
                    });
                }
                let copy = Self::has_derive(&attrs, "Copy");
                let must_consume = attrs.iter().any(|a| a.name == "must_consume");
                if let DefKind::Enum(info) | DefKind::ErrorSet(info) =
                    &mut self.prog.defs[def.0 as usize].kind
                {
                    info.generics = generics;
                    info.variants = variants;
                    info.copy = copy;
                    info.must_consume = must_consume;
                }
            }
            Pending::Fn { def, module, ast: f, owner, drop_for } => {
                self.resolve_fn(def, module, &f, owner, drop_for)
            }
            Pending::Test { def, module, ast: t } => {
                let mut info = self.placeholder_fn(module, FnKind::Test(self.prog.def(def).name.clone()));
                let ctx_generics: Vec<String> = Vec::new();
                for p in &t.params {
                    if let ParamKind::Named { name, ty, .. } = &p.kind {
                        let ctx = TyCtx { module, generics: &ctx_generics, self_ty: None, allow_ref: false };
                        let ty = self.resolve_ty(ty, &ctx);
                        info.params.push(ParamInfo {
                            name: name.text.clone(),
                            mode: p.mode,
                            ty,
                            default: None,
                            span: self.span(module, p.range),
                        });
                    }
                }
                if t.cases.is_some() {
                    let span = self.span(module, t.name_range);
                    self.unsupported(span, "a parametrised test", "the test runner (Phase 8)");
                }
                info.body = t.body.clone();
                self.prog.defs[def.0 as usize].kind = DefKind::Fn(info);
            }
            Pending::Const { def, module, ast: c } => {
                let ty = match &c.ty {
                    Some(t) => {
                        let ctx = TyCtx { module, generics: &[], self_ty: None, allow_ref: false };
                        self.resolve_ty(t, &ctx)
                    }
                    None => {
                        let span = self.span(module, c.name.range);
                        self.report(
                            Diagnostic::new(msg::RESOLVE_CONST_TYPE_REQUIRED, span)
                                .with("name", c.name.text.clone()),
                        );
                        Ty::Error
                    }
                };
                self.prog.defs[def.0 as usize].kind =
                    DefKind::Const(ConstInfo { ty, value: c.value.clone(), file: self.file(module) });
            }
            Pending::Impl { module, ast: imp, range } => self.resolve_impl(module, &imp, range),
        }
    }

    fn resolve_impl(&mut self, module: ModuleId, imp: &ast::ImplDef, range: TextRange) {
        let span = self.span(module, range);
        let generics = self.generics_allowed(module, &imp.generics);
        let ctx = TyCtx { module, generics: &generics, self_ty: None, allow_ref: false };
        let self_ty = self.resolve_ty(&imp.self_ty, &ctx);
        let mut drop_for = None;
        if let Some(t) = &imp.trait_ty {
            let trait_name = match &t.kind {
                TypeKind::Path { segments, .. } => {
                    segments.iter().map(|s| s.text.clone()).collect::<Vec<_>>().join(".")
                }
                _ => String::new(),
            };
            if trait_name != "Drop" {
                let at = self.span(module, t.range);
                self.report(Diagnostic::new(msg::RESOLVE_TRAIT_IMPL, at).with("trait", trait_name));
                return;
            }
            match &self_ty {
                Ty::Adt(d, _)
                    if self.prog.def(*d).module == module
                        && matches!(self.prog.def(*d).kind, DefKind::Struct(_) | DefKind::Enum(_)) =>
                {
                    drop_for = Some(*d);
                }
                Ty::Error => return,
                _ => {
                    self.report(Diagnostic::new(msg::RESOLVE_DROP_TARGET, span));
                    return;
                }
            }
            let ok = imp.methods.len() == 1
                && imp.methods[0].name.text == "drop"
                && imp.methods[0].params.len() == 1
                && matches!(imp.methods[0].params[0].kind, ParamKind::SelfParam)
                && imp.methods[0].params[0].mode == ast::Mode::Mut
                && imp.methods[0].ret.is_none();
            if !ok {
                self.report(Diagnostic::new(msg::RESOLVE_DROP_SIGNATURE, span));
                return;
            }
        }
        if self_ty.key().is_none() {
            if self_ty != Ty::Error {
                let s = self.span(module, imp.self_ty.range);
                self.report(
                    Diagnostic::new(msg::RESOLVE_NOT_A_TYPE, s).with("name", self.prog.display(&self_ty)),
                );
            }
            return;
        }
        for m in &imp.methods {
            let kind = if drop_for.is_some() { FnKind::Drop } else { FnKind::Method };
            let def = self.new_def(
                module,
                &m.name.text,
                m.name.range,
                ast::Vis::Pub,
                Vec::new(),
                DefKind::Fn(self.placeholder_fn(module, kind)),
            );
            self.pending.push(Pending::Fn {
                def,
                module,
                ast: m.clone(),
                owner: Some(Owner { generics: generics.clone(), self_ty: self_ty.clone() }),
                drop_for,
            });
        }
    }

    fn resolve_fn(
        &mut self,
        def: DefId,
        module: ModuleId,
        f: &ast::FnDef,
        owner: Option<Owner>,
        drop_for: Option<DefId>,
    ) {
        let impl_generics: Vec<String> = owner.as_ref().map(|o| o.generics.clone()).unwrap_or_default();
        let own_generics = self.generics_allowed(module, &f.generics);
        let mut generics: Vec<Generic> =
            impl_generics.iter().map(|n| Generic { name: n.clone(), bounds: Vec::new() }).collect();
        for g in &f.generics {
            let bounds = g
                .bounds
                .iter()
                .filter_map(|b| match &b.kind {
                    TypeKind::Path { segments, .. } => segments.last().map(|s| s.text.clone()),
                    _ => None,
                })
                .collect();
            generics.push(Generic { name: g.name.text.clone(), bounds });
        }
        let names: Vec<String> = impl_generics.iter().cloned().chain(own_generics).collect();
        let self_ty = owner.as_ref().map(|o| o.self_ty.clone());
        let ctx_self = self_ty.clone();
        let mut info = self.placeholder_fn(module, FnKind::Free);
        info.generics = generics;
        info.impl_generics = impl_generics.len();
        info.owner = self_ty.clone();
        info.pure = f.is_pure;
        info.body = f.body.clone();
        info.intrinsic = Self::builtin_attr(&f.attrs, "intrinsic");
        if f.is_comptime {
            let span = self.span(module, f.name.range);
            self.unsupported(span, "`comptime def`", "compile-time evaluation (Phase 4)");
        }
        if f.has_where {
            let span = self.span(module, f.name.range);
            self.unsupported(span, "a `where` clause", "traits (Phase 4)");
        }
        let mut seen_default = false;
        for (i, p) in f.params.iter().enumerate() {
            let ctx = TyCtx { module, generics: &names, self_ty: ctx_self.as_ref(), allow_ref: true };
            match &p.kind {
                ParamKind::SelfParam => {
                    if owner.is_none() {
                        let span = self.span(module, p.range);
                        self.report(Diagnostic::new(msg::RESOLVE_SELF_OUTSIDE_METHOD, span));
                    } else if i != 0 {
                        let span = self.span(module, p.range);
                        self.report(Diagnostic::new(msg::RESOLVE_SELF_NOT_FIRST, span));
                    } else {
                        info.self_mode = Some(p.mode);
                    }
                }
                ParamKind::Named { name, ty, default } => {
                    if info.params.iter().any(|x| x.name == name.text) {
                        let span = self.span(module, name.range);
                        self.report(
                            Diagnostic::new(msg::RESOLVE_DUPLICATE_PARAMETER, span)
                                .with("name", name.text.clone()),
                        );
                    }
                    if default.is_some() {
                        seen_default = true;
                    } else if seen_default && info.keyword_only_from.is_none() {
                        let span = self.span(module, name.range);
                        self.report(Diagnostic::new(msg::RESOLVE_PARAMETER_DEFAULT_ORDER, span));
                    }
                    let ty = self.resolve_ty(ty, &ctx);
                    info.params.push(ParamInfo {
                        name: name.text.clone(),
                        mode: p.mode,
                        ty,
                        default: default.clone(),
                        span: self.span(module, p.range),
                    });
                }
                ParamKind::KeywordOnly => info.keyword_only_from = Some(info.params.len()),
                ParamKind::PositionalOnly => info.positional_only_until = info.params.len(),
                ParamKind::Variadic { .. } => {
                    let span = self.span(module, p.range);
                    self.unsupported(span, "variadic parameters", "`Slice` (Phase 4)");
                }
                ParamKind::Error => {}
            }
        }
        // The result type; `T !` infers the error set from the body.
        match &f.ret {
            None => info.ret = Ty::Unit,
            Some(t) => {
                info.ret_span = Some(self.span(module, t.range));
                if let TypeKind::Result { ok, err: None } = &t.kind {
                    let ctx = TyCtx { module, generics: &names, self_ty: ctx_self.as_ref(), allow_ref: true };
                    let ok_ty = match ok {
                        Some(o) => self.resolve_ty(o, &ctx),
                        None => Ty::Unit,
                    };
                    info.err_inferred = true;
                    info.ret = match self.prog.wk.result {
                        Some(r) => Ty::Adt(r, vec![ok_ty, Ty::ErrSet(Vec::new())]),
                        None => Ty::Error,
                    };
                } else {
                    let ctx = TyCtx { module, generics: &names, self_ty: ctx_self.as_ref(), allow_ref: true };
                    info.ret = self.resolve_ty(t, &ctx);
                }
            }
        }
        info.from = f.from.iter().map(|n| (n.text.clone(), self.span(module, n.range))).collect();
        info.body_broken = f.body_broken;
        if f.header_broken {
            info.header_broken = true;
            info.ret = Ty::Error;
            info.err_inferred = false;
        }
        // A function whose header did not parse (no name) was reported by
        // the parser already.
        if info.body.is_none() && info.intrinsic.is_none() && !f.name.text.is_empty() {
            let span = self.span(module, f.name.range);
            self.report(Diagnostic::new(msg::RESOLVE_MISSING_BODY, span).with("name", f.name.text.clone()));
        }
        // Register methods and associated functions.
        if let Some(o) = &owner {
            info.kind = if drop_for.is_some() {
                FnKind::Drop
            } else if info.self_mode.is_some() {
                FnKind::Method
            } else {
                FnKind::Assoc
            };
            if let Some(key) = o.self_ty.key() {
                let k = (key, f.name.text.clone());
                if drop_for.is_some() {
                    if let Some(d) = drop_for {
                        match &mut self.prog.defs[d.0 as usize].kind {
                            DefKind::Struct(s) => s.drop = Some(def),
                            DefKind::Enum(e) => e.drop = Some(def),
                            _ => {}
                        }
                    }
                } else if let std::collections::hash_map::Entry::Vacant(e) = self.prog.methods.entry(k) {
                    e.insert(def);
                } else {
                    let span = self.span(module, f.name.range);
                    let owner_name = self.prog.display(&o.self_ty);
                    self.report(
                        Diagnostic::new(msg::RESOLVE_DUPLICATE_METHOD, span)
                            .with("owner", owner_name)
                            .with("name", f.name.text.clone()),
                    );
                }
            }
        }
        self.prog.defs[def.0 as usize].kind = DefKind::Fn(info);
    }

    // ---------------------------------------------------------------- types

    fn resolve_ty(&mut self, te: &ast::TypeExpr, ctx: &TyCtx) -> Ty {
        let span = self.span(ctx.module, te.range);
        match &te.kind {
            TypeKind::Error => Ty::Error,
            TypeKind::Path { segments, args } => {
                let args: Vec<Ty> = args.iter().map(|a| self.resolve_ty(a, ctx)).collect();
                self.resolve_path_ty(segments, args, ctx, span)
            }
            TypeKind::Option(inner) => {
                let t = self.resolve_ty(inner, ctx);
                match self.prog.wk.option {
                    Some(o) => Ty::Adt(o, vec![t]),
                    None => Ty::Error,
                }
            }
            TypeKind::Result { ok, err } => {
                let ok_ty = match ok {
                    Some(o) => self.resolve_ty(o, ctx),
                    None => Ty::Unit,
                };
                let err_ty = match err {
                    None => {
                        self.report(Diagnostic::new(msg::RESOLVE_INFERRED_SET_POSITION, span));
                        Ty::Error
                    }
                    Some(e) => self.resolve_error_set(e, ctx),
                };
                match self.prog.wk.result {
                    Some(r) => Ty::Adt(r, vec![ok_ty, err_ty]),
                    None => Ty::Error,
                }
            }
            TypeKind::Union(_) => {
                self.report(Diagnostic::new(msg::RESOLVE_UNION_OUTSIDE_RESULT, span));
                Ty::Error
            }
            TypeKind::Tuple(elems) => {
                let tys: Vec<Ty> = elems.iter().map(|(_, t)| self.resolve_ty(t, ctx)).collect();
                let labels: Vec<String> = if !elems.is_empty() && elems.iter().all(|(n, _)| n.is_some()) {
                    elems.iter().filter_map(|(n, _)| n.as_ref().map(|n| n.text.clone())).collect()
                } else {
                    Vec::new()
                };
                if tys.is_empty() { Ty::Unit } else { Ty::Tuple(tys, labels) }
            }
            TypeKind::Fn { pure, params, ret } => {
                let params = params.iter().map(|p| self.resolve_ty(p, ctx)).collect();
                let ret = self.resolve_ty(ret, ctx);
                Ty::Fn(Box::new(FnTy { params, ret, pure: *pure }))
            }
            TypeKind::Ref { mutable, inner } => {
                if !ctx.allow_ref {
                    self.unsupported(
                        span,
                        "a reference type in a declaration",
                        "references in signatures (Phase 3)",
                    );
                }
                let t = self.resolve_ty(inner, ctx);
                // A shared borrow of a view is the view: `ref str` is a
                // `str` (04, "Views"), valid while the text it views is.
                if t == Ty::Str && !*mutable {
                    return Ty::Str;
                }
                Ty::Ref(Box::new(t), *mutable)
            }
            TypeKind::ConstArg(_) => {
                self.unsupported(span, "a constant type argument", "const generics (Phase 4)");
                Ty::Error
            }
            TypeKind::Unsupported(what) => {
                let phase = if what.contains("dyn") {
                    "traits (Phase 4)"
                } else if what.contains("pointer") {
                    "foreign functions (Phase 8)"
                } else {
                    "compile-time evaluation (Phase 4)"
                };
                self.unsupported(span, what, phase);
                Ty::Error
            }
        }
    }

    fn resolve_error_set(&mut self, te: &ast::TypeExpr, ctx: &TyCtx) -> Ty {
        let span = self.span(ctx.module, te.range);
        let parts: Vec<&ast::TypeExpr> = match &te.kind {
            TypeKind::Union(list) => list.iter().collect(),
            _ => vec![te],
        };
        let mut members = Vec::new();
        for p in parts {
            match self.resolve_ty(p, ctx) {
                Ty::ErrSet(m) => members.extend(m),
                Ty::Error => return Ty::Error,
                other => {
                    let name = self.prog.display(&other);
                    let s = self.span(ctx.module, p.range);
                    self.report(Diagnostic::new(msg::RESOLVE_NOT_AN_ERROR_SET, s).with("name", name));
                    let _ = span;
                    return Ty::Error;
                }
            }
        }
        members.sort();
        members.dedup();
        Ty::ErrSet(members)
    }

    fn resolve_path_ty(&mut self, segments: &[ast::Name], args: Vec<Ty>, ctx: &TyCtx, span: Span) -> Ty {
        let Some(first) = segments.first() else { return Ty::Error };
        if segments.len() == 1 {
            let name = first.text.as_str();
            if let Some(i) = ctx.generics.iter().position(|g| g == name) {
                return Ty::Param(i as u32);
            }
            let prim = match name {
                "int" => Some(Ty::Int),
                "f64" => Some(Ty::Float),
                "bool" => Some(Ty::Bool),
                "char" => Some(Ty::Char),
                "str" => Some(Ty::Str),
                "Never" => Some(Ty::Never),
                _ => None,
            };
            if let Some(p) = prim {
                if !args.is_empty() {
                    self.report(Diagnostic::new(msg::TYPE_NOT_GENERIC, span).with("name", name));
                }
                return p;
            }
            if name == "Self" {
                return match ctx.self_ty {
                    Some(t) => t.clone(),
                    None => {
                        self.report(Diagnostic::new(msg::RESOLVE_SELF_TYPE_OUTSIDE, span));
                        Ty::Error
                    }
                };
            }
            if let Some(d) = self.prog.lookup(ctx.module, name) {
                return self.def_as_type(d, name, args, span);
            }
            if let Some((_, phase)) = LATER_TYPES.iter().find(|(n, _)| *n == name) {
                self.unsupported(span, &format!("`{name}`"), phase);
                return Ty::Error;
            }
            if !self.prog.poisoned.contains(name) {
                self.report(Diagnostic::new(msg::RESOLVE_UNDEFINED_TYPE, span).with("name", name));
            }
            return Ty::Error;
        }
        // A qualified name: module.Type.
        let mut module = ctx.module;
        for (i, seg) in segments.iter().enumerate() {
            let last = i + 1 == segments.len();
            let found = if i == 0 {
                self.prog.lookup(module, &seg.text)
            } else {
                self.prog.lookup_in(module, &seg.text)
            };
            let Some(d) = found else {
                if i == 0 {
                    self.report(
                        Diagnostic::new(msg::RESOLVE_UNDEFINED_TYPE, span).with("name", seg.text.clone()),
                    );
                } else {
                    let mname = self.prog.modules[module.0 as usize].name.clone();
                    self.report(
                        Diagnostic::new(msg::RESOLVE_NOT_IN_MODULE, span)
                            .with("module", mname)
                            .with("name", seg.text.clone()),
                    );
                }
                return Ty::Error;
            };
            if last {
                return self.def_as_type(d, &seg.text, args, span);
            }
            match &self.prog.def(d).kind {
                DefKind::Module(m) => module = *m,
                _ => {
                    self.report(
                        Diagnostic::new(msg::RESOLVE_NOT_A_MODULE, span).with("name", seg.text.clone()),
                    );
                    return Ty::Error;
                }
            }
        }
        Ty::Error
    }

    fn def_as_type(&mut self, d: DefId, name: &str, args: Vec<Ty>, span: Span) -> Ty {
        let generics_len = match &self.prog.def(d).kind {
            DefKind::Struct(s) => Some(s.generics.len()),
            DefKind::Enum(e) => Some(e.generics.len()),
            DefKind::ErrorSet(_) => {
                if !args.is_empty() {
                    self.report(Diagnostic::new(msg::TYPE_NOT_GENERIC, span).with("name", name));
                }
                return Ty::ErrSet(vec![d]);
            }
            DefKind::TypeAlias(_) => return self.resolve_alias(d),
            DefKind::Unsupported => return Ty::Error,
            _ => None,
        };
        let Some(n) = generics_len else {
            self.report(Diagnostic::new(msg::RESOLVE_NOT_A_TYPE, span).with("name", name));
            return Ty::Error;
        };
        // Generics of a type not yet resolved are counted from its AST later;
        // the prelude's are resolved first, so this is exact for builtins.
        let declared = self.declared_generics(d).unwrap_or(n);
        if args.len() != declared {
            if args.is_empty() {
                self.report(Diagnostic::new(msg::TYPE_TYPE_ARGUMENTS_NEEDED, span).with("name", name));
            } else {
                self.report(
                    Diagnostic::new(msg::RESOLVE_TYPE_ARGUMENT_COUNT, span)
                        .with("name", name)
                        .with("expected", declared.to_string())
                        .with("found", args.len().to_string()),
                );
            }
            return Ty::Adt(d, (0..declared).map(|_| Ty::Error).collect());
        }
        Ty::Adt(d, args)
    }

    /// The number of generic parameters a type declares, from its pending
    /// AST when its signature is not resolved yet.
    fn declared_generics(&self, d: DefId) -> Option<usize> {
        for p in &self.pending {
            match p {
                Pending::Struct { def, ast, .. } if *def == d => return Some(ast.generics.len()),
                Pending::Enum { def, ast, .. } if *def == d => return Some(ast.generics.len()),
                _ => {}
            }
        }
        match &self.prog.def(d).kind {
            DefKind::Struct(s) => Some(s.generics.len()),
            DefKind::Enum(e) => Some(e.generics.len()),
            _ => None,
        }
    }

    fn build_variant_index(&mut self) {
        for (i, def) in self.prog.defs.iter().enumerate() {
            if let DefKind::Enum(e) | DefKind::ErrorSet(e) = &def.kind {
                for (vi, v) in e.variants.iter().enumerate() {
                    self.prog.variants.entry(v.name.clone()).or_default().push((DefId(i as u32), vi as u32));
                }
            }
        }
    }
}

/// Reports a diagnostic message id at a span; used by tests.
#[allow(dead_code)]
fn diag(m: MsgId, span: Span) -> Diagnostic {
    Diagnostic::new(m, span)
}
