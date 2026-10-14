//! The resolver: declares every item of every module, loads the std
//! modules an import names, resolves signatures to [`Ty`], and registers
//! methods. Bodies are left to the type checker.
//!
//! Types are resolved in `types.rs`, traits and impls in `traits.rs`, and
//! `@derive` writes its impls as Sisu source in `derive.rs`.

use crate::program::*;
use crate::std_sources;
use crate::ty::Ty;
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
pub(crate) const LATER_TYPES: &[(&str, &str)] = &[
    ("i128", "the 128-bit integers"),
    ("u128", "the 128-bit integers"),
    ("f32", "`f32`"),
    ("Array", "`Array` and const generics"),
];

/// Where a method is declared, which decides the generics, bounds and
/// `Self` of its signature.
#[derive(Clone, Debug)]
pub(crate) enum Owner {
    /// Written in a struct's or enum's body.
    Type(DefId),
    /// `impl[...] Type:`.
    Inherent { generics: Vec<Generic>, names: Vec<String>, predicates: Vec<Predicate>, self_ty: Ty },
    /// A trait's method.
    Trait(DefId),
    /// A trait impl's method.
    Impl(DefId),
}

/// What a method's owner brings to its signature: the generics, their
/// names, the bounds, `Self`, the trait (for a trait's method) and the impl
/// (for an impl's method).
type OwnerScope = (Vec<Generic>, Vec<String>, Vec<Predicate>, Option<Ty>, Option<DefId>, Option<DefId>);

pub(crate) enum Pending {
    Fn { def: DefId, module: ModuleId, ast: ast::FnDef, owner: Option<Owner> },
    Struct { def: DefId, module: ModuleId, ast: ast::StructDef, attrs: Vec<ast::Attr> },
    Enum { def: DefId, module: ModuleId, ast: ast::EnumDef, attrs: Vec<ast::Attr> },
    Trait { def: DefId, module: ModuleId, ast: ast::TraitDef, attrs: Vec<ast::Attr> },
    Impl { module: ModuleId, ast: ast::ImplDef, range: TextRange, derived: bool },
    Test { def: DefId, module: ModuleId, ast: ast::TestDef },
    Const { def: DefId, module: ModuleId, ast: ast::ConstDef },
}

pub(crate) struct Resolver<'a> {
    pub prog: Program,
    pub diags: Vec<Diagnostic>,
    pub sources: &'a mut SourceMap,
    pub std_loaded: HashMap<&'static str, ModuleId>,
    pub pending: Vec<Pending>,
    pub aliases: HashMap<DefId, (ModuleId, ast::TypeAlias)>,
    pub resolving_alias: Vec<DefId>,
    /// The generic parameter names of every struct, enum and trait, known
    /// before any of them is resolved.
    pub type_params: HashMap<DefId, Vec<ast::GenericParam>>,
    /// Uses of `dyn Trait`, checked for object safety once every trait's
    /// methods are known.
    pub dyn_uses: Vec<(DefId, Span)>,
    /// Derived impls whose fields must implement the trait, checked once
    /// every impl is known.
    pub derive_checks: Vec<crate::derive::DeriveCheck>,
    /// The bounds of every struct's and enum's generic parameters.
    pub type_preds: HashMap<DefId, Vec<Predicate>>,
    /// Each trait's `comptime def derive` (08), by module and trait name.
    pub derive_templates: HashMap<(ModuleId, String), std::rc::Rc<ast::FnDef>>,
    /// The text of each module that has a derive template, for unfolding it.
    pub template_sources: HashMap<ModuleId, std::rc::Rc<str>>,
}

/// What a type expression may mention where it is written.
pub(crate) struct TyCtx<'g> {
    pub module: ModuleId,
    pub generics: &'g [String],
    pub self_ty: Option<&'g Ty>,
    pub allow_ref: bool,
    /// Bounds in scope, which give `C.Item` its trait.
    pub predicates: &'g [Predicate],
    /// Inside a trait: its own associated types may be named bare.
    pub in_trait: Option<DefId>,
    /// Inside a trait impl: what it binds its associated types to, for
    /// `Self.Item`.
    pub impl_assoc: &'g [(String, Ty)],
}

impl<'g> TyCtx<'g> {
    pub fn plain(module: ModuleId, generics: &'g [String], allow_ref: bool) -> TyCtx<'g> {
        TyCtx { module, generics, self_ty: None, allow_ref, predicates: &[], in_trait: None, impl_assoc: &[] }
    }
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
        type_params: HashMap::new(),
        dyn_uses: Vec::new(),
        derive_checks: Vec::new(),
        type_preds: HashMap::new(),
        derive_templates: HashMap::new(),
        template_sources: HashMap::new(),
    };
    let prelude = r.load_std("prelude", std_sources::PRELUDE);
    r.prog.prelude = prelude;
    r.fill_well_known();
    // The ambient std modules, named in the prelude.
    for (name, source) in std_sources::AMBIENT {
        let m = r.load_std(name, source);
        let def =
            r.new_def(prelude, name, TextRange::empty(0), ast::Vis::Pub, Vec::new(), DefKind::Module(m));
        r.prog.modules[prelude.0 as usize].items.insert(name.to_string(), def);
    }
    // The impls of the leaf types and tuples; nothing names the module.
    r.load_std("builtin_impls", std_sources::builtin_impls());
    let root_id = r.add_module("main".into(), root_file, false);
    r.prog.root = root_id;
    r.prog.poisoned = root.broken_names.iter().cloned().collect();
    r.declare_items(root_id, &root.items, "");
    r.declare_derived(root_id, &root.items);
    r.resolve_pending();
    r.build_variant_index();
    r.check_traits();
    r.check_borrow_signatures();
    Resolved { program: r.prog, diagnostics: r.diags }
}

impl Resolver<'_> {
    /// The empty span at the start of `keyword` where it comes last before
    /// `span` on the same line (`struct` before a struct's name).
    pub fn keyword_before(&self, span: Span, keyword: &str) -> Option<Span> {
        let text = &self.sources.get(span.file)?.text;
        let before = text.get(..span.range.start as usize)?;
        let line_start = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
        let at = before[line_start..].rfind(keyword)? + line_start;
        Some(Span::new(span.file, TextRange::new(at as u32, at as u32)))
    }

    pub fn span(&self, module: ModuleId, range: TextRange) -> Span {
        Span::new(self.prog.modules[module.0 as usize].file, range)
    }

    pub fn file(&self, module: ModuleId) -> FileId {
        self.prog.modules[module.0 as usize].file
    }

    pub fn is_std(&self, module: ModuleId) -> bool {
        self.prog.modules[module.0 as usize].is_std
    }

    pub fn report(&mut self, d: Diagnostic) {
        self.diags.push(d);
    }

    pub fn unsupported(&mut self, span: Span, construct: &str, phase: &str) {
        self.report(
            Diagnostic::new(msg::STAGE0_UNSUPPORTED, span).with("construct", construct).with("phase", phase),
        );
    }

    fn add_module(&mut self, name: String, file: FileId, is_std: bool) -> ModuleId {
        let id = ModuleId(self.prog.modules.len() as u32);
        self.prog.modules.push(ModuleData {
            name,
            file,
            items: HashMap::new(),
            order: Vec::new(),
            is_std,
            parent: None,
        });
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
        self.declare_derived(m, &ast.items);
        m
    }

    fn fill_well_known(&mut self) {
        let p = self.prog.prelude;
        let get = |prog: &Program, n: &str| prog.lookup_in(p, n);
        let prog = &self.prog;
        let wk = WellKnown {
            option: get(prog, "Option"),
            result: get(prog, "Result"),
            ordering: get(prog, "Ordering"),
            string: get(prog, "String"),
            list: get(prog, "list"),
            dict: get(prog, "dict"),
            set: get(prog, "set"),
            iter: get(prog, "Iter"),
            entry: get(prog, "Entry"),
            iterable: get(prog, "Iterable"),
            path: get(prog, "Path"),
            system: get(prog, "System"),
            io_error: get(prog, "IoError"),
            range: get(prog, "range"),
            formatter: get(prog, "Formatter"),
            boxed: get(prog, "Box"),
            display: get(prog, "Display"),
            debug: get(prog, "Debug"),
            eq: get(prog, "Eq"),
            partial_ord: get(prog, "PartialOrd"),
            ord: get(prog, "Ord"),
            hash: get(prog, "Hash"),
            len: get(prog, "Len"),
            copy: get(prog, "Copy"),
            drop: get(prog, "Drop"),
            iterator: get(prog, "Iterator"),
            add: get(prog, "Add"),
            sub: get(prog, "Sub"),
            mul: get(prog, "Mul"),
            div: get(prog, "Div"),
            rem: get(prog, "Rem"),
            neg: get(prog, "Neg"),
            num: get(prog, "Num"),
            from_literal: get(prog, "FromLiteral"),
            task_error: get(prog, "TaskError"),
            scope: get(prog, "Scope"),
            handle: get(prog, "Handle"),
            sender: get(prog, "Sender"),
            receiver: get(prog, "Receiver"),
            send: get(prog, "Send"),
            sync: get(prog, "Sync"),
            escapable: get(prog, "Escapable"),
            fs_ops: get(prog, "FsOps"),
            net_ops: get(prog, "NetOps"),
            clock_ops: get(prog, "ClockOps"),
            rand_ops: get(prog, "RandOps"),
            env_ops: get(prog, "EnvOps"),
            proc_ops: get(prog, "ProcOps"),
        };
        self.prog.wk = wk;
    }

    pub fn new_def(
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

    pub fn placeholder_fn(&self, module: ModuleId, kind: FnKind) -> FnInfo {
        FnInfo {
            generics: Vec::new(),
            impl_generics: 0,
            predicates: Vec::new(),
            trait_: None,
            impl_: None,
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
            comptime: false,
            intrinsic: None,
            structural: None,
            keeps: Vec::new(),
            internal: false,
            body: None,
            kind,
            file: self.file(module),
            header_broken: false,
            body_broken: false,
        }
    }

    // ---------------------------------------------------------------- pass 1

    pub fn declare_items(&mut self, module: ModuleId, items: &[ast::Item], test_prefix: &str) {
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
                        DefKind::Fn(Box::new(self.placeholder_fn(module, FnKind::Free))),
                    );
                    self.bind(module, &f.name.text, def, f.name.range);
                    self.comptime_budget(module, def, &item.attrs);
                    self.pending.push(Pending::Fn { def, module, ast: f.clone(), owner: None });
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
                    self.type_params.insert(def, s.generics.clone());
                    self.declare_methods(module, def, &s.methods);
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
                    self.type_params.insert(def, e.generics.clone());
                    self.declare_methods(module, def, &e.methods);
                    self.pending.push(Pending::Enum {
                        def,
                        module,
                        ast: e.clone(),
                        attrs: item.attrs.clone(),
                    });
                }
                ItemKind::Impl(i) => self.pending.push(Pending::Impl {
                    module,
                    ast: i.clone(),
                    range: item.range,
                    derived: self.prog.modules[module.0 as usize].name.starts_with("<derive>/"),
                }),
                ItemKind::Trait(t) => {
                    let def = self.new_def(
                        module,
                        &t.name.text,
                        t.name.range,
                        item.vis,
                        item.docs.clone(),
                        DefKind::Trait(TraitInfo::default()),
                    );
                    self.bind(module, &t.name.text, def, t.name.range);
                    self.type_params.insert(def, t.generics.clone());
                    // The trait's derive, which `@derive` runs at compile time.
                    for m in t.methods.iter().filter(|m| m.is_comptime && m.name.text == "derive") {
                        if crate::template::template_parts(m).is_some() {
                            self.derive_templates
                                .insert((module, t.name.text.clone()), std::rc::Rc::new(m.clone()));
                            if !self.template_sources.contains_key(&module) {
                                let text = self
                                    .sources
                                    .get(self.file(module))
                                    .map(|f| f.text.as_str())
                                    .unwrap_or("");
                                self.template_sources.insert(module, std::rc::Rc::from(text));
                            }
                        } else {
                            let span = self.span(module, m.name.range);
                            self.report(
                                Diagnostic::new(msg::COMPTIME_DERIVE_TEMPLATE, span)
                                    .with("trait", t.name.text.clone()),
                            );
                        }
                    }
                    self.pending.push(Pending::Trait {
                        def,
                        module,
                        ast: t.clone(),
                        attrs: item.attrs.clone(),
                    });
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
                        DefKind::Fn(Box::new(self.placeholder_fn(module, FnKind::Test(name.clone())))),
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
                    self.comptime_budget(module, def, &item.attrs);
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

    /// `@comptime_budget(steps=N)` (08): the item's own budget for its
    /// `comptime` expressions.
    fn comptime_budget(&mut self, module: ModuleId, def: DefId, attrs: &[ast::Attr]) {
        for a in attrs.iter().filter(|a| a.name == "comptime_budget") {
            let steps = match a.args.as_slice() {
                [ast::Arg { name: Some(n), value, .. }] if n.text == "steps" => match &value.kind {
                    ast::ExprKind::Int { value: Some(v), suffix: None, .. } if *v > 0 => {
                        u64::try_from(*v).ok()
                    }
                    _ => None,
                },
                _ => None,
            };
            match steps {
                Some(n) => {
                    self.prog.comptime_budgets.insert(def, n);
                }
                None => {
                    // The attribute without the line break after it.
                    let text = self.sources.get(self.file(module)).map(|f| f.text.as_str()).unwrap_or("");
                    let written = text.get(a.range.start as usize..a.range.end as usize).unwrap_or("");
                    let end = a.range.start + written.trim_end().len() as u32;
                    let span = self.span(module, TextRange::new(a.range.start, end));
                    self.report(Diagnostic::new(msg::COMPTIME_BUDGET_ATTRIBUTE, span));
                }
            }
        }
    }

    fn check_attrs(&mut self, module: ModuleId, attrs: &[ast::Attr]) {
        if self.is_std(module) {
            return;
        }
        for a in attrs {
            if matches!(
                a.name.as_str(),
                "builtin" | "intrinsic" | "structural" | "keeps" | "sealed" | "marker" | "internal"
            ) {
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
        for def in self.prog.defs.iter() {
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

    fn declare_methods(&mut self, module: ModuleId, owner: DefId, methods: &[ast::FnDef]) {
        for m in methods {
            let def = self.new_def(
                module,
                &m.name.text,
                m.name.range,
                ast::Vis::Pub,
                Vec::new(),
                DefKind::Fn(Box::new(self.placeholder_fn(module, FnKind::Method))),
            );
            self.pending.push(Pending::Fn { def, module, ast: m.clone(), owner: Some(Owner::Type(owner)) });
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
        let prelude = self.prog.prelude;
        for (n, alias) in &imp.names {
            let found = self.prog.lookup_in(m, &n.text).or_else(|| {
                std_sources::reexports(name)
                    .contains(&n.text.as_str())
                    .then(|| self.prog.lookup_in(prelude, &n.text))
                    .flatten()
            });
            match found {
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
        // The generic parameters of every type and trait first, with their
        // defaults, so that a use site anywhere can fill omitted arguments.
        self.resolve_headers();
        // Aliases next, on demand, so that later declarations can use them.
        let mut alias_ids: Vec<DefId> = self.aliases.keys().copied().collect();
        alias_ids.sort();
        for d in alias_ids {
            self.resolve_alias(d);
        }
        // Types, then traits, then impls, then functions: a method's
        // signature needs its owner's generics and bounds, and an impl
        // needs its trait.
        let pending = std::mem::take(&mut self.pending);
        for p in &pending {
            if let Pending::Trait { def, module, ast: t, attrs } = p {
                self.resolve_trait_header(*def, *module, t, attrs);
            }
        }
        let (types, rest): (Vec<Pending>, Vec<Pending>) =
            pending.into_iter().partition(|p| matches!(p, Pending::Struct { .. } | Pending::Enum { .. }));
        for p in types {
            self.resolve_one(p);
        }
        let (traits, rest): (Vec<Pending>, Vec<Pending>) =
            rest.into_iter().partition(|p| matches!(p, Pending::Trait { .. }));
        for p in traits {
            self.resolve_one(p);
        }
        let (impls, rest): (Vec<Pending>, Vec<Pending>) =
            rest.into_iter().partition(|p| matches!(p, Pending::Impl { .. }));
        for p in impls {
            self.resolve_one(p);
        }
        let mut queue = rest;
        // Traits and impls push the functions they declare.
        queue.extend(std::mem::take(&mut self.pending));
        while !queue.is_empty() {
            for p in std::mem::take(&mut queue) {
                self.resolve_one(p);
            }
            queue = std::mem::take(&mut self.pending);
        }
    }

    pub fn resolve_alias(&mut self, d: DefId) -> Ty {
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
        let ctx = TyCtx::plain(module, &[], self.is_std(module));
        let ty = match &alias.ty {
            // An alias may name a union of error sets (05, "Error sets").
            Some(t) if matches!(t.kind, ast::TypeKind::Union(_)) => self.resolve_error_set(t, &ctx),
            Some(t) => self.resolve_ty(t, &ctx),
            None => Ty::Error,
        };
        self.resolving_alias.pop();
        self.prog.defs[d.0 as usize].kind = DefKind::TypeAlias(ty.clone());
        ty
    }

    pub fn has_derive(attrs: &[ast::Attr], what: &str) -> bool {
        attrs.iter().any(|a| {
            a.name == "derive"
                && a.args.iter().any(|arg| matches!(&arg.value.kind, ExprKind::Name(n) if n == what))
        })
    }

    pub fn builtin_attr(attrs: &[ast::Attr], name: &str) -> Option<String> {
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
                let (names, predicates) = self.type_param_scope(def, module);
                let self_ty = Ty::Adt(def, (0..names.len() as u32).map(Ty::Param).collect());
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
                    let ctx = TyCtx {
                        module,
                        generics: &names,
                        self_ty: Some(&self_ty),
                        allow_ref: true,
                        predicates: &predicates,
                        in_trait: None,
                        impl_assoc: &[],
                    };
                    let ty = self.resolve_ty(&f.ty, &ctx);
                    let fspan = self.span(module, f.ty.range);
                    self.check_sized(&ty, fspan);
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
                    info.fields = fields;
                    info.builtin = builtin;
                    info.copy = copy;
                    info.must_consume = must_consume;
                }
            }
            Pending::Enum { def, module, ast: e, attrs } => {
                let (names, predicates) = self.type_param_scope(def, module);
                let self_ty = Ty::Adt(def, (0..names.len() as u32).map(Ty::Param).collect());
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
                    let ctx = TyCtx {
                        module,
                        generics: &names,
                        self_ty: Some(&self_ty),
                        allow_ref: std,
                        predicates: &predicates,
                        in_trait: None,
                        impl_assoc: &[],
                    };
                    let mut fields = Vec::new();
                    for f in &v.fields {
                        let ty = self.resolve_ty(&f.ty, &ctx);
                        let fspan = self.span(module, f.ty.range);
                        self.check_sized(&ty, fspan);
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
                    info.variants = variants;
                    info.copy = copy;
                    info.must_consume = must_consume;
                }
            }
            Pending::Trait { def, module, ast: t, .. } => self.resolve_trait(def, module, &t),
            Pending::Fn { def, module, ast: f, owner } => self.resolve_fn(def, module, &f, owner),
            Pending::Test { def, module, ast: t } => {
                let mut info = self.placeholder_fn(module, FnKind::Test(self.prog.def(def).name.clone()));
                let ctx_generics: Vec<String> = Vec::new();
                for p in &t.params {
                    if let ParamKind::Named { name, ty, .. } = &p.kind {
                        let ctx = TyCtx::plain(module, &ctx_generics, false);
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
                self.prog.defs[def.0 as usize].kind = DefKind::Fn(Box::new(info));
            }
            Pending::Const { def, module, ast: c } => {
                let ty = match &c.ty {
                    Some(t) => {
                        let ctx = TyCtx::plain(module, &[], false);
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
            Pending::Impl { module, ast: imp, range, derived } => {
                self.resolve_impl(module, &imp, range, derived)
            }
        }
    }

    /// The names and bounds of a struct's or enum's generic parameters.
    pub fn type_param_scope(&mut self, def: DefId, _module: ModuleId) -> (Vec<String>, Vec<Predicate>) {
        let names: Vec<String> = self.prog.type_generics(def).iter().map(|g| g.name.clone()).collect();
        let predicates = self.type_preds.get(&def).cloned().unwrap_or_default();
        (names, predicates)
    }

    fn resolve_fn(&mut self, def: DefId, module: ModuleId, f: &ast::FnDef, owner: Option<Owner>) {
        // The generics, bounds and `Self` the owner brings.
        let (outer, outer_names, mut predicates, self_ty, in_trait, impl_def): OwnerScope = match &owner {
            None => (Vec::new(), Vec::new(), Vec::new(), None, None, None),
            Some(Owner::Type(t)) => {
                let generics = self.prog.type_generics(*t).to_vec();
                let names: Vec<String> = generics.iter().map(|g| g.name.clone()).collect();
                let (_, preds) = self.type_param_scope(*t, module);
                let self_ty = Ty::Adt(*t, (0..names.len() as u32).map(Ty::Param).collect());
                (generics, names, preds, Some(self_ty), None, None)
            }
            Some(Owner::Inherent { generics, names, predicates, self_ty }) => {
                (generics.clone(), names.clone(), predicates.clone(), Some(self_ty.clone()), None, None)
            }
            Some(Owner::Trait(t)) => {
                let info = self.prog.trait_info(*t).cloned().unwrap_or_default();
                let mut generics = vec![Generic { name: "Self".into(), default: None }];
                generics.extend(info.generics.iter().cloned());
                let names: Vec<String> = generics.iter().map(|g| g.name.clone()).collect();
                let tr = TraitRef { def: *t, args: (1..generics.len() as u32).map(Ty::Param).collect() };
                let preds = vec![Predicate { ty: Ty::Param(0), bound: tr, span: self.prog.def(*t).span }];
                (generics, names, preds, Some(Ty::Param(0)), Some(*t), None)
            }
            Some(Owner::Impl(i)) => match self.prog.impl_info(*i).cloned() {
                Some(info) => {
                    let names: Vec<String> = info.generics.iter().map(|g| g.name.clone()).collect();
                    (info.generics, names, info.predicates, Some(info.self_ty), None, Some(*i))
                }
                None => (Vec::new(), Vec::new(), Vec::new(), None, None, None),
            },
        };
        let mut names = outer_names.clone();
        for g in &f.generics {
            names.push(g.name.text.clone());
        }
        let mut generics = outer.clone();
        for (k, g) in f.generics.iter().enumerate() {
            let default = g.default.as_ref().map(|d| {
                let ctx = TyCtx::plain(module, &names[..outer.len() + k], true);
                self.resolve_ty(d, &ctx)
            });
            if default.is_some() {
                let span = self.span(module, g.name.range);
                self.report(Diagnostic::new(msg::RESOLVE_IMPL_DEFAULT, span));
            }
            generics.push(Generic { name: g.name.text.clone(), default: None });
        }
        let impl_assoc: Vec<(String, Ty)> =
            impl_def.and_then(|i| self.prog.impl_info(i)).map(|i| i.assoc.clone()).unwrap_or_default();
        // The function's own bounds, then its `where` clause.
        let own = self.resolve_bounds(module, &f.generics, &names, outer.len(), in_trait);
        predicates.extend(own);
        for w in &f.where_ {
            let ctx = TyCtx {
                module,
                generics: &names,
                self_ty: self_ty.as_ref(),
                allow_ref: true,
                predicates: &predicates,
                in_trait,
                impl_assoc: &impl_assoc,
            };
            let ty = self.resolve_ty(&w.ty, &ctx);
            let mut new = Vec::new();
            for b in &w.bounds {
                if let Some(tr) = self.resolve_bound(b, &ty, &ctx) {
                    new.push(Predicate { ty: ty.clone(), bound: tr, span: self.span(module, b.range) });
                }
            }
            predicates.extend(new);
        }
        let mut info = self.placeholder_fn(module, FnKind::Free);
        info.generics = generics;
        info.impl_generics = outer.len();
        info.predicates = predicates.clone();
        info.trait_ = in_trait;
        info.impl_ = impl_def;
        info.owner = self_ty.clone();
        info.pure = f.is_pure;
        info.comptime = f.is_comptime;
        info.body = f.body.clone();
        info.intrinsic = Self::builtin_attr(&f.attrs, "intrinsic");
        info.structural = Self::builtin_attr(&f.attrs, "structural");
        info.internal = f.attrs.iter().any(|a| a.name == "internal");
        let mut seen_default = false;
        for (i, p) in f.params.iter().enumerate() {
            let ctx = TyCtx {
                module,
                generics: &names,
                self_ty: self_ty.as_ref(),
                allow_ref: true,
                predicates: &predicates,
                in_trait,
                impl_assoc: &impl_assoc,
            };
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
                    let pspan = self.span(module, p.range);
                    self.check_sized(&ty, pspan);
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
        let ctx = TyCtx {
            module,
            generics: &names,
            self_ty: self_ty.as_ref(),
            allow_ref: true,
            predicates: &predicates,
            in_trait,
            impl_assoc: &impl_assoc,
        };
        match &f.ret {
            None => info.ret = Ty::Unit,
            Some(t) => {
                info.ret_span = Some(self.span(module, t.range));
                if let TypeKind::Result { ok, err: None } = &t.kind {
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
                    info.ret = self.resolve_ty(t, &ctx);
                }
                let rspan = self.span(module, t.range);
                let ret = info.ret.clone();
                self.check_sized(&ret, rspan);
            }
        }
        info.from = f.from.iter().map(|n| (n.text.clone(), self.span(module, n.range))).collect();
        // `@keeps(self, x)`: positions among `self` and the parameters.
        for a in f.attrs.iter().filter(|a| a.name == "keeps") {
            for arg in &a.args {
                let name = match &arg.value.kind {
                    ExprKind::Name(n) => n.clone(),
                    ExprKind::SelfValue => "self".to_string(),
                    _ => continue,
                };
                let offset = u32::from(info.self_mode.is_some());
                let pos = if name == "self" && info.self_mode.is_some() {
                    Some(0)
                } else {
                    info.params.iter().position(|p| p.name == name).map(|i| i as u32 + offset)
                };
                if let Some(pos) = pos {
                    info.keeps.push(pos);
                }
            }
        }
        info.body_broken = f.body_broken;
        if f.header_broken {
            info.header_broken = true;
            info.ret = Ty::Error;
            info.err_inferred = false;
        }
        // A function whose header did not parse (no name) was reported by
        // the parser already; a trait's required method has no body.
        if info.body.is_none() && info.intrinsic.is_none() && !f.name.text.is_empty() && in_trait.is_none() {
            let span = self.span(module, f.name.range);
            self.report(Diagnostic::new(msg::RESOLVE_MISSING_BODY, span).with("name", f.name.text.clone()));
        }
        // Register methods and associated functions.
        if let Some(o) = &owner {
            info.kind = if self.is_drop_impl(impl_def) {
                FnKind::Drop
            } else if info.self_mode.is_some() {
                FnKind::Method
            } else {
                FnKind::Assoc
            };
            match o {
                Owner::Type(_) | Owner::Inherent { .. } => {
                    let key = self_ty.as_ref().and_then(|t| t.key());
                    if let Some(key) = key {
                        let k = (key, f.name.text.clone());
                        if let std::collections::hash_map::Entry::Vacant(e) = self.prog.methods.entry(k) {
                            e.insert(def);
                        } else {
                            let span = self.span(module, f.name.range);
                            let owner_name = self.prog.display(self_ty.as_ref().unwrap_or(&Ty::Error));
                            self.report(
                                Diagnostic::new(msg::RESOLVE_DUPLICATE_METHOD, span)
                                    .with("owner", owner_name)
                                    .with("name", f.name.text.clone()),
                            );
                        }
                    }
                }
                Owner::Trait(t) => {
                    if let DefKind::Trait(ti) = &mut self.prog.defs[t.0 as usize].kind {
                        ti.methods.push(def);
                    }
                }
                Owner::Impl(i) => {
                    if let DefKind::Impl(ii) = &mut self.prog.defs[i.0 as usize].kind {
                        ii.methods.push((f.name.text.clone(), def));
                    }
                }
            }
        }
        self.prog.defs[def.0 as usize].kind = DefKind::Fn(Box::new(info));
    }

    /// Whether an impl is of `Drop`.
    fn is_drop_impl(&self, impl_def: Option<DefId>) -> bool {
        impl_def
            .and_then(|i| self.prog.impl_info(i))
            .is_some_and(|i| Some(i.trait_ref.def) == self.prog.wk.drop)
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
