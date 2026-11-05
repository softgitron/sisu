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
use std::collections::{HashMap, HashSet};

/// The resolver's output.
#[derive(Clone, Debug)]
pub struct Resolved {
    pub program: Program,
    pub diagnostics: Vec<Diagnostic>,
}

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
    /// The predicate of a refinement type: its parameters are the
    /// refinement's, `self` is a value of its base, and it is nobody's
    /// method.
    Refinement(DefId),
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
    Refinement { def: DefId, module: ModuleId, ast: ast::TypeAlias, range: TextRange },
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
    /// The definitions whose generic parameters are resolved, so that the
    /// pass over the instances of templates does not report them again.
    pub headers_done: HashSet<DefId>,
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
    /// Functions the compiler writes whose kind their declaration does not
    /// tell: a refinement's predicate, a struct's invariant check.
    pub special_kinds: HashMap<DefId, FnKind>,
    /// Functions the compiler writes whose diagnostics are another's.
    pub quiet_fns: std::collections::HashSet<DefId>,
    /// Generic functions the last check found calling a template with their
    /// own type parameters: templates too (08, "Reflection").
    pub promoted: std::collections::HashSet<DefId>,
    /// The syntax of each template, for its instances.
    pub template_asts: HashMap<DefId, ast::FnDef>,
    /// Values that reflect (a constant's, a field's or a parameter's
    /// default), unfolded once everything they may name is resolved
    /// (decision 251).
    pub(crate) unfold_later: Vec<Later>,
    /// Each trait's provided methods that reflect, with the trait's module:
    /// an impl that does not write one gets it as a template of its own,
    /// made for its type (decision 251).
    pub trait_templates: HashMap<DefId, Vec<(ModuleId, ast::FnDef)>>,
    /// The provided methods that reflect each impl got from its trait
    /// (decision 251): their instances name the impl's type parameters
    /// apart, since the trait's code never names them.
    pub provided_copies: std::collections::HashSet<DefId>,
    /// The names reflection shows for the type parameters of the template
    /// being made (a generic trait impl's, decision 251).
    pub shape_names: Vec<String>,
    /// The template and type arguments of each instance declared.
    pub instance_of: HashMap<DefId, (DefId, Vec<Ty>)>,
    /// The attributes of each struct and enum, as data (`reflect.attributes_of`).
    pub attrs_of: HashMap<DefId, Vec<crate::template::Attribute>>,
    /// While the program's own modules are declared, their imports, which
    /// wait until every module is declared (09: the modules of a package
    /// may name each other freely).
    pub deferred_imports: Option<Vec<(ModuleId, ast::Item)>>,
    /// `sisu test --std`: each std module's doc examples are tests.
    pub std_doc_tests: bool,
    /// The items of each std module loaded, for its doc examples.
    pub std_items: HashMap<&'static str, std::sync::Arc<ast::Module>>,
    /// How many instances this resolution was asked for, made or not
    /// (`MAX_INSTANCES` bounds it).
    pub instances_asked: usize,
    /// Whether it was asked for more than that, which is reported once.
    pub instances_bounded: bool,
    /// How many instances this resolution declared: the next one's number.
    pub instances_declared: usize,
    /// The modules that hold an instance, whose functions are never
    /// templates themselves.
    pub instance_modules: std::collections::HashSet<ModuleId>,
    /// The instances that are methods, found once their impls are resolved:
    /// the module, the instance's name, the template, its type arguments
    /// and where the program needs it.
    pub method_instances: Vec<(ModuleId, String, DefId, Vec<Ty>, Span)>,
    /// The definitions whose `from` clauses are checked already.
    pub borrow_checked: usize,
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

/// A module of the program's own source (09, "Modules"): the path an
/// import names it by, its file and its syntax, and its package.
#[derive(Clone, Copy, Debug)]
pub struct SourceModule<'a> {
    /// `["weather", "parser"]` for `src/parser.sisu` of the package
    /// `weather`, `["weather"]` for its root module; empty for a single
    /// file, which no import names.
    pub path: &'a [String],
    pub file: FileId,
    pub ast: &'a ast::Module,
    /// An index of [`Sources::packages`].
    pub package: usize,
}

/// A package of the program's source (09, "Package manifest").
#[derive(Clone, Debug, Default)]
pub struct SourcePackage {
    /// Its `[package] name`; empty for a single file.
    pub name: String,
    /// The packages its imports may name, by name, as indices of
    /// [`Sources::packages`].
    pub deps: Vec<(String, usize)>,
    /// `[trust] unsafe` lists it (the root package always may).
    pub trusted: bool,
}

/// Everything the program is made of: its modules, the root module first,
/// and its packages, the root package first.
#[derive(Clone, Copy, Debug)]
pub struct Sources<'a> {
    pub modules: &'a [SourceModule<'a>],
    pub packages: &'a [SourcePackage],
    /// Also declare the standard library's doc examples as tests, every
    /// std module loaded (`sisu test --std`).
    pub std_doc_tests: bool,
    /// The program is built with its tests, so the root package's doc
    /// examples are tests of it (decision 243).
    pub tests: bool,
}

/// The package id of [`Sources::packages`]`[k]` in the resolved program,
/// where std is [`STD_PACKAGE`].
fn package_id(k: usize) -> u32 {
    if k == 0 { ROOT_PACKAGE } else { k as u32 + 1 }
}

/// Resolves a program whose root module is `root` (already parsed from
/// `root_file`). The prelude and any imported std modules are added to
/// `sources`.
pub fn resolve(sources: &mut SourceMap, root_file: FileId, root: &ast::Module) -> Resolved {
    resolve_with(sources, root_file, root, &[], &HashMap::new())
}

/// [`resolve`], with the instances and promotions the last check asked for
/// (08, "Reflection") and the files `embed` names.
pub fn resolve_with(
    sources: &mut SourceMap,
    root_file: FileId,
    root: &ast::Module,
    requests: &[Request],
    embedded: &HashMap<String, Result<String, String>>,
) -> Resolved {
    let modules = [SourceModule { path: &[], file: root_file, ast: root, package: 0 }];
    let packages = [SourcePackage { name: String::new(), deps: Vec::new(), trusted: true }];
    resolve_program(
        sources,
        Sources { modules: &modules, packages: &packages, std_doc_tests: false, tests: true },
        requests,
        embedded,
    )
}

/// Resolves a program of several modules and packages (09): every module
/// of `program` is declared before any import is resolved, so the modules
/// of a package may name each other in any order.
pub fn resolve_program(
    sources: &mut SourceMap,
    program: Sources<'_>,
    requests: &[Request],
    embedded: &HashMap<String, Result<String, String>>,
) -> Resolved {
    resolve_session(sources, program, requests, embedded).finish()
}

/// A resolution that can be given more instances of its templates (08,
/// "Reflection"): an instance is declared after everything else, so
/// declaring the instances a check asked for on top of the program it
/// checked gives what resolving again with every request would, without
/// the work (docs/architecture.md, "The query layer").
pub struct Resolution<'a> {
    r: Resolver<'a>,
}

impl<'a> Resolution<'a> {
    pub fn program(&self) -> &Program {
        &self.r.prog
    }

    pub fn sources(&self) -> &SourceMap {
        self.r.sources
    }

    /// Declares the instances `requests` asks for that the program does
    /// not have. A promotion makes a declared function a template, which
    /// only resolving again with it can do: it is ignored here.
    pub fn add_instances(&mut self, requests: &[Request]) {
        self.r.declare_instances(requests);
        self.r.check_borrow_signatures();
    }

    pub fn finish(self) -> Resolved {
        Resolved { program: self.r.prog, diagnostics: self.r.diags }
    }
}

/// [`resolve_program`], keeping the resolver for more instances.
pub fn resolve_session<'a>(
    sources: &'a mut SourceMap,
    program: Sources<'_>,
    requests: &[Request],
    embedded: &HashMap<String, Result<String, String>>,
) -> Resolution<'a> {
    let mut r = Resolver {
        prog: Program::default(),
        diags: Vec::new(),
        sources,
        std_loaded: HashMap::new(),
        pending: Vec::new(),
        aliases: HashMap::new(),
        resolving_alias: Vec::new(),
        type_params: HashMap::new(),
        headers_done: HashSet::new(),
        dyn_uses: Vec::new(),
        derive_checks: Vec::new(),
        type_preds: HashMap::new(),
        derive_templates: HashMap::new(),
        template_sources: HashMap::new(),
        special_kinds: HashMap::new(),
        quiet_fns: std::collections::HashSet::new(),
        promoted: requests
            .iter()
            .filter_map(|q| match q {
                Request::Promote(d) => Some(*d),
                Request::Instance { .. } => None,
            })
            .collect(),
        template_asts: HashMap::new(),
        trait_templates: HashMap::new(),
        provided_copies: std::collections::HashSet::new(),
        shape_names: Vec::new(),
        unfold_later: Vec::new(),
        instance_of: HashMap::new(),
        attrs_of: HashMap::new(),
        deferred_imports: None,
        std_doc_tests: false,
        std_items: HashMap::new(),
        instances_asked: 0,
        instances_bounded: false,
        instances_declared: 0,
        instance_modules: std::collections::HashSet::new(),
        method_instances: Vec::new(),
        borrow_checked: 0,
    };
    r.prog.embedded = embedded.clone();
    // The packages: the root package, std, then the dependencies.
    for (k, p) in program.packages.iter().enumerate() {
        let data = PackageData {
            name: p.name.clone(),
            trusted: p.trusted || k == 0,
            deps: p.deps.iter().map(|(n, d)| (n.clone(), package_id(*d))).collect(),
            root: None,
        };
        if k == 0 {
            r.prog.packages.push(data);
            r.prog.packages.push(PackageData { name: "std".into(), trusted: true, ..Default::default() });
        } else {
            r.prog.packages.push(data);
        }
    }
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
    // `target`, the build target's facts (08), is ambient too.
    {
        let m = r.load_std("target", std_sources::target());
        let def =
            r.new_def(prelude, "target", TextRange::empty(0), ast::Vis::Pub, Vec::new(), DefKind::Module(m));
        r.prog.modules[prelude.0 as usize].items.insert("target".to_string(), def);
    }
    // The impls of the leaf types and tuples; nothing names the module.
    r.load_std("builtin_impls", std_sources::builtin_impls());
    // `sisu test --std`: every std module, with its doc examples as tests.
    if program.std_doc_tests {
        r.std_doc_tests = true;
        let prelude_items = r.std_items.get("prelude").cloned().unwrap_or_default();
        r.declare_doc_tests(prelude, &prelude_items.items, "std.core");
        for (name, source) in std_sources::DOCUMENTED {
            // The ambient modules are loaded already.
            match r.std_loaded.get(name).copied() {
                Some(m) => {
                    let items = r.std_items.get(name).cloned().unwrap_or_default();
                    r.declare_doc_tests(m, &items.items, &std_sources::display_path(name));
                }
                None => {
                    r.load_std(name, source);
                }
            }
        }
    }
    // The program's own modules, the root module first: each is a module
    // of its package, and a child of the module its path's prefix names.
    let mut ids = Vec::new();
    for (i, m) in program.modules.iter().enumerate() {
        let name = if i == 0 { "main".to_string() } else { m.path.join(".") };
        let id = r.add_module(name, m.file, false);
        let data = &mut r.prog.modules[id.0 as usize];
        data.package = package_id(m.package);
        data.path = m.path.to_vec();
        r.prog.user_modules.push(id);
        ids.push(id);
    }
    if let Some(&root_id) = ids.first() {
        r.prog.root = root_id;
    }
    if let Some(root) = program.modules.first() {
        r.prog.poisoned = root.ast.broken_names.iter().cloned().collect();
    }
    for (i, m) in program.modules.iter().enumerate() {
        r.link_module(ids[i], m);
    }
    // Every module's items, then the imports between them.
    r.deferred_imports = Some(Vec::new());
    for (i, m) in program.modules.iter().enumerate() {
        let id = ids[i];
        let items = r.own_items(m);
        let prefix = r.test_prefix(m);
        r.declare_items(id, &items, &prefix);
        // The root package's doc examples are its tests too (10), when the
        // program is built with its tests.
        if m.package == 0 && program.tests {
            r.declare_doc_tests(id, &items, &prefix);
        }
    }
    let deferred = r.deferred_imports.take().unwrap_or_default();
    for (module, item) in deferred {
        match &item.kind {
            ItemKind::Import(imp) => r.import(module, imp, item.range, item.vis),
            ItemKind::FromImport(imp) => r.import_from(module, imp, item.range, item.vis),
            _ => {}
        }
    }
    // Each module's derived impls: planned for every module first, their
    // templates run and their texts parsed on threads, then declared in
    // the modules' order (decision 228).
    let plans: Vec<crate::derive::Plan> = program
        .modules
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let items = r.own_items(m);
            r.plan_derived(ids[i], &items)
        })
        .collect();
    let unfolded = crate::derive::unfold_plans(&plans);
    Resolver::parse_ahead(&plans, &unfolded);
    for ((i, m), (plan, u)) in program.modules.iter().enumerate().zip(plans.into_iter().zip(unfolded)) {
        r.apply_derived(plan, &u);
        let items = r.own_items(m);
        r.declare_generators(ids[i], &items);
    }
    r.resolve_pending();
    r.build_variant_index();
    r.check_traits();
    r.declare_instances(requests);
    r.check_borrow_signatures();
    Resolution { r }
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
            is_std,
            package: if is_std { STD_PACKAGE } else { ROOT_PACKAGE },
            ..ModuleData::default()
        });
        id
    }

    /// Makes a module of the program's source the child of the module its
    /// path's prefix names (a directory with no file of its own is a module
    /// too, whose items are its children), and a package's root module the
    /// one its imports name the package by.
    fn link_module(&mut self, id: ModuleId, m: &SourceModule) {
        let package = package_id(m.package);
        if m.path.len() == 1 {
            if let Some(p) = self.prog.packages.get_mut(package as usize)
                && p.root.is_none()
            {
                p.root = Some(id);
            }
            return;
        }
        let Some((last, parent_path)) = m.path.split_last() else { return };
        if parent_path.is_empty() {
            return;
        }
        let parent = self.module_at(parent_path, package, m.file);
        let def =
            self.new_def(parent, last, TextRange::empty(0), ast::Vis::Pub, Vec::new(), DefKind::Module(id));
        let data = &mut self.prog.modules[parent.0 as usize];
        data.items.insert(last.clone(), def);
    }

    /// The module of the program at `path`, made (as a directory's module,
    /// with no items but its children) when no file is.
    fn module_at(&mut self, path: &[String], package: u32, file: FileId) -> ModuleId {
        if let Some(&m) = self.prog.user_modules.iter().find(|m| self.prog.modules[m.0 as usize].path == path)
        {
            return m;
        }
        let id = self.add_module(path.join("."), file, false);
        let data = &mut self.prog.modules[id.0 as usize];
        data.package = package;
        data.path = path.to_vec();
        self.prog.user_modules.push(id);
        // Linked as `link_module` would, with this package.
        if path.len() == 1 {
            if let Some(p) = self.prog.packages.get_mut(package as usize)
                && p.root.is_none()
            {
                p.root = Some(id);
            }
        } else if let Some((last, parent_path)) = path.split_last() {
            let parent = self.module_at(parent_path, package, file);
            let def = self.new_def(
                parent,
                last,
                TextRange::empty(0),
                ast::Vis::Pub,
                Vec::new(),
                DefKind::Module(id),
            );
            self.prog.modules[parent.0 as usize].items.insert(last.clone(), def);
        }
        id
    }

    /// The items of a module that this build declares: a dependency's
    /// tests and benchmarks are its own package's business (09). A
    /// script's top-level statements are its `main` (10).
    fn own_items<'m>(&mut self, m: &SourceModule<'m>) -> std::borrow::Cow<'m, [ast::Item]> {
        let dependency = m.package != 0;
        let left_out = |i: &ast::Item| {
            dependency && matches!(i.kind, ItemKind::Test(_) | ItemKind::Suite(_) | ItemKind::Bench(_))
        };
        // The module's own items, copied only when some are left out.
        let items: std::borrow::Cow<'m, [ast::Item]> = if m.ast.items.iter().any(left_out) {
            std::borrow::Cow::Owned(m.ast.items.iter().filter(|i| !left_out(i)).cloned().collect())
        } else {
            std::borrow::Cow::Borrowed(&m.ast.items)
        };
        let script = self.sources.get(m.file).is_some_and(|f| f.text.starts_with("#!"));
        if !script || !items.iter().any(|i| matches!(i.kind, ItemKind::Stmt(_))) {
            return items;
        }
        let own_main = items.iter().find_map(|i| match &i.kind {
            ItemKind::Fn(f) if f.name.text == "main" => Some(f.name.range),
            _ => None,
        });
        if let Some(range) = own_main {
            let span = Span::new(m.file, range);
            if !self.diags.iter().any(|d| d.msg == msg::RESOLVE_SCRIPT_MAIN && d.span == span) {
                self.report(Diagnostic::new(msg::RESOLVE_SCRIPT_MAIN, span));
            }
            return items;
        }
        let mut out: Vec<ast::Item> =
            items.iter().filter(|i| !matches!(i.kind, ItemKind::Stmt(_))).cloned().collect();
        out.extend(script_main(&items));
        std::borrow::Cow::Owned(out)
    }

    /// The prefix of the names of a module's tests: its path within its
    /// package (`parser/accepts leading zeros`); none for a root module.
    fn test_prefix(&self, m: &SourceModule) -> String {
        if m.path.len() <= 1 { String::new() } else { m.path[1..].join(".") }
    }

    fn load_std(&mut self, name: &'static str, source: &'static str) -> ModuleId {
        if let Some(&m) = self.std_loaded.get(name) {
            return m;
        }
        let file = self.sources.add(format!("<std>/{name}.sisu"), source);
        let (ast, diags) = std_ast(name, source);
        // The syntax tree has no file in it; its diagnostics name the file
        // it was first parsed as.
        self.diags.extend(diags.iter().cloned().map(|mut d| {
            d.span.file = file;
            d
        }));
        let m = self.add_module(name.to_string(), file, true);
        self.std_loaded.insert(name, m);
        self.declare_items(m, &ast.items, "");
        self.declare_derived(m, &ast.items);
        if self.std_doc_tests {
            let prefix = std_sources::display_path(name);
            self.declare_doc_tests(m, &ast.items, &prefix);
        }
        self.std_items.insert(name, ast);
        m
    }

    /// A module's doc examples (10, "Testing"): one test per fenced block
    /// of its `##` comments, declared in a module of their own whose text
    /// keeps each example where it is in the file and whose scope is the
    /// module's.
    pub(crate) fn declare_doc_tests(&mut self, module: ModuleId, items: &[ast::Item], prefix: &str) {
        let file = self.file(module);
        let Some(text) = self.sources.get(file).map(|f| f.text.clone()) else { return };
        let starts = item_starts(items);
        let examples = crate::doctests::examples(&text, &starts);
        if examples.is_empty() {
            return;
        }
        let generated = crate::doctests::test_module(&text, &examples);
        // The file keeps the module's name: its lines are the module's.
        let name = self.sources.name(file).to_string();
        let gen_file = self.sources.add(name, generated.clone());
        let (ast, diags) = parse_generated(&generated);
        self.diags.extend(diags.iter().cloned().map(|mut d| {
            d.spans_mut().for_each(|s| s.file = gen_file);
            d
        }));
        let parent = &self.prog.modules[module.0 as usize];
        let data = ModuleData {
            name: format!("<doc>/{}.sisu", parent.name),
            file: gen_file,
            is_std: parent.is_std,
            parent: Some(module),
            package: parent.package,
            ..ModuleData::default()
        };
        let id = ModuleId(self.prog.modules.len() as u32);
        self.prog.modules.push(data);
        self.declare_items(id, &ast.items, prefix);
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
            contains: get(prog, "Contains"),
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
            stdin_ops: get(prog, "StdinOps"),
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
            self.report(
                Diagnostic::new(msg::RESOLVE_DUPLICATE_DEFINITION, span)
                    .with("name", name)
                    .label(first, msg::RESOLVE_FIRST_DEFINED, Vec::new())
                    .note(msg::RESOLVE_RENAME_DUPLICATE, Vec::new()),
            );
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
            export: None,
            quiet: false,
            template: false,
            instance_of: None,
            type_params: Vec::new(),
            body: None,
            kind,
            file: self.file(module),
            header_broken: false,
            body_broken: false,
        }
    }

    // ---------------------------------------------------------------- pass 1

    pub fn declare_items(&mut self, module: ModuleId, items: &[ast::Item], test_prefix: &str) {
        let mut attributed: Vec<(String, Vec<crate::template::Attribute>)> = Vec::new();
        for item in items {
            self.check_attrs(module, &item.attrs);
            // The attributes of a type, as data for `reflect.attributes_of`.
            if let ItemKind::Struct(ast::StructDef { name, .. })
            | ItemKind::Enum(ast::EnumDef { name, .. })
            | ItemKind::ErrorSet(ast::EnumDef { name, .. }) = &item.kind
                && !item.attrs.is_empty()
            {
                let text = self.sources.get(self.file(module)).map(|f| f.text.clone()).unwrap_or_default();
                let attrs = item
                    .attrs
                    .iter()
                    .map(|a| {
                        let args = a
                            .args
                            .iter()
                            .map(|x| {
                                text.get(x.range.start as usize..x.range.end as usize)
                                    .unwrap_or("")
                                    .to_string()
                            })
                            .collect();
                        (a.name.clone(), args)
                    })
                    .collect();
                attributed.push((name.text.clone(), attrs));
            }
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
                    for c in &s.consts {
                        self.declare_assoc_const(module, crate::ty::TyKey::Adt(def), &s.name.text, c);
                    }
                    // `invariant e` lines (07): a method of the struct that
                    // checks them, which construction, every `mut self`
                    // method and the `Mutex` rule call.
                    if !s.invariants.is_empty() {
                        let check = invariant_check_def(item.node, s);
                        let m = self.new_def(
                            module,
                            &check.name.text,
                            check.name.range,
                            ast::Vis::Private,
                            Vec::new(),
                            DefKind::Fn(Box::new(self.placeholder_fn(module, FnKind::Method))),
                        );
                        self.special_kinds.insert(m, FnKind::Invariant(def));
                        self.pending.push(Pending::Fn {
                            def: m,
                            module,
                            ast: check,
                            owner: Some(Owner::Type(def)),
                        });
                        // The invariant as a `bool`, for D-077's check and the
                        // generators of property tests: quiet, since the
                        // check above reports what is wrong in it.
                        let holds = invariant_holds_def(item.node, s);
                        let h = self.new_def(
                            module,
                            &holds.name.text,
                            holds.name.range,
                            ast::Vis::Private,
                            Vec::new(),
                            DefKind::Fn(Box::new(self.placeholder_fn(module, FnKind::Method))),
                        );
                        self.quiet_fns.insert(h);
                        self.pending.push(Pending::Fn {
                            def: h,
                            module,
                            ast: holds,
                            owner: Some(Owner::Type(def)),
                        });
                    }
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
                ItemKind::Import(_) | ItemKind::FromImport(_) if self.deferred_imports.is_some() => {
                    if let Some(d) = self.deferred_imports.as_mut() {
                        d.push((module, item.clone()));
                    }
                }
                ItemKind::Import(imp) => self.import(module, imp, item.range, item.vis),
                ItemKind::FromImport(imp) => self.import_from(module, imp, item.range, item.vis),
                ItemKind::Test(t) => {
                    let name = if test_prefix.is_empty() {
                        t.name.clone()
                    } else {
                        format!("{test_prefix}/{}", t.name)
                    };
                    // A parametrised test is a test for each case (17).
                    let source =
                        self.sources.get(self.file(module)).map(|f| f.text.clone()).unwrap_or_default();
                    for (name, t) in crate::tests_of::cases(&name, t, &source) {
                        let def = self.new_def(
                            module,
                            &name,
                            t.name_range,
                            ast::Vis::Private,
                            item.docs.clone(),
                            DefKind::Fn(Box::new(self.placeholder_fn(module, FnKind::Test(name.clone())))),
                        );
                        self.prog.tests.push(def);
                        self.test_tags(def, &item.attrs, module);
                        self.pending.push(Pending::Test { def, module, ast: t });
                    }
                }
                ItemKind::Suite(s) => {
                    let prefix = if test_prefix.is_empty() {
                        s.name.clone()
                    } else {
                        format!("{test_prefix}/{}", s.name)
                    };
                    self.declare_items(module, &s.items, &prefix);
                }
                // A benchmark is checked like a test; `sisu bench` runs it.
                ItemKind::Bench(t) => {
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
                    self.prog.benches.push(def);
                    self.pending.push(Pending::Test { def, module, ast: t.clone() });
                }
                ItemKind::Const(c) => {
                    // The value is known from here on, for a type's
                    // constant argument that names it (`consts.rs`).
                    let info = ConstInfo { ty: Ty::Error, value: c.value.clone(), file: self.file(module) };
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
                ItemKind::TypeAlias(a) if a.refinement => {
                    let text = match &a.predicate {
                        Some(p) => self
                            .sources
                            .get(self.file(module))
                            .and_then(|f| f.text.get(p.range.start as usize..p.range.end as usize))
                            .unwrap_or("")
                            .trim()
                            .to_string(),
                        None => String::new(),
                    };
                    let info = RefinementInfo {
                        generics: Vec::new(),
                        base: Ty::Error,
                        predicate: None,
                        text,
                        domain: None,
                    };
                    let def = self.new_def(
                        module,
                        &a.name.text,
                        a.name.range,
                        item.vis,
                        item.docs.clone(),
                        DefKind::Refinement(info),
                    );
                    self.bind(module, &a.name.text, def, a.name.range);
                    self.type_params.insert(def, a.generics.clone());
                    self.pending.push(Pending::Refinement { def, module, ast: a.clone(), range: item.range });
                }
                ItemKind::TypeAlias(a) => {
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
        for (name, attrs) in attributed {
            if let Some(d) = self.prog.modules[module.0 as usize].items.get(&name).copied() {
                self.attrs_of.insert(d, attrs);
            }
        }
        self.declare_trait_shims(module, items);
    }

    /// An associated constant of a type (03), named `Type.NAME`.
    pub fn declare_assoc_const(
        &mut self,
        module: ModuleId,
        key: crate::ty::TyKey,
        owner: &str,
        c: &ast::ConstDef,
    ) {
        let info = ConstInfo { ty: Ty::Error, value: None, file: self.file(module) };
        let name = format!("{owner}.{}", c.name.text);
        let def = self.new_def(module, &name, c.name.range, ast::Vis::Pub, Vec::new(), DefKind::Const(info));
        let k = (key, c.name.text.clone());
        if self.prog.assoc_consts.contains_key(&k) || self.prog.methods.contains_key(&k) {
            let span = self.span(module, c.name.range);
            self.report(
                Diagnostic::new(msg::RESOLVE_DUPLICATE_METHOD, span)
                    .with("owner", owner.to_string())
                    .with("name", c.name.text.clone()),
            );
            return;
        }
        self.prog.assoc_consts.insert(k, def);
        self.pending.push(Pending::Const { def, module, ast: c.clone() });
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
        let from = std::mem::replace(&mut self.borrow_checked, self.prog.defs.len());
        for def in self.prog.defs.iter().skip(from) {
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
                                .label(label.0, label.1, label.2)
                                .note(msg::BORROW_FROM_ONLY_PARAMETERS, Vec::new()),
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
            let vis = if self.is_std(module) { ast::Vis::Pub } else { m.vis };
            let def = self.new_def(
                module,
                &m.name.text,
                m.name.range,
                vis,
                Vec::new(),
                DefKind::Fn(Box::new(self.placeholder_fn(module, FnKind::Method))),
            );
            self.pending.push(Pending::Fn { def, module, ast: m.clone(), owner: Some(Owner::Type(owner)) });
        }
    }

    /// The module an import path names (09, "Modules"): a module of the
    /// importing package (by the package's own name), of one of its
    /// dependencies (by the dependency's name), or of the standard library
    /// (`std.json`, and stage 0's short names `cli`, `http`, `json`, ...).
    /// `None` when it names none, after reporting why.
    fn import_target(&mut self, module: ModuleId, path: &[ast::Name], at: Span) -> Option<ModuleId> {
        let words: Vec<&str> = path.iter().map(|n| n.text.as_str()).collect();
        let first = *words.first()?;
        let package = self.prog.modules[module.0 as usize].package();
        let own = self.prog.packages.get(package as usize).filter(|p| !p.name.is_empty() && p.name == first);
        let dep = self
            .prog
            .packages
            .get(package as usize)
            .and_then(|p| p.deps.iter().find(|(n, _)| n == first).map(|(_, d)| *d));
        let root = match (own, dep) {
            (Some(p), _) => p.root,
            (None, Some(d)) => self.prog.packages.get(d as usize).and_then(|p| p.root),
            (None, None) => None,
        };
        let Some(mut m) = root else {
            // A package of the program that this package does not depend on.
            if !self.is_std(module)
                && std_sources::module_for(&words).is_none()
                && let Some(p) = self.prog.packages.iter().skip(2).find(|p| p.name == first)
            {
                let this =
                    self.prog.packages.get(package as usize).map(|p| p.name.clone()).unwrap_or_default();
                let name = p.name.clone();
                self.report(
                    Diagnostic::new(msg::RESOLVE_NOT_A_DEPENDENCY, at)
                        .with("name", words.join("."))
                        .with("package", name)
                        .with("this", this),
                );
                return None;
            }
            let Some((name, source)) = std_sources::module_for(&words) else {
                let written = words.join(".");
                let mut d = Diagnostic::new(msg::RESOLVE_IMPORT_UNKNOWN, at).with("name", written.clone());
                // The closest module a path can name: std's, or a package's.
                let mut candidates: Vec<String> =
                    std_sources::MODULE_PATHS.iter().map(|p| p.to_string()).collect();
                candidates
                    .extend(self.prog.packages.iter().filter(|p| !p.name.is_empty()).map(|p| p.name.clone()));
                if let Some(similar) = crate::program::closest(&written, candidates) {
                    d = d.fix(
                        msg::RESOLVE_SIMILAR_MODULE,
                        Vec::new(),
                        vec![sisu_diag::Edit { span: at, text: similar }],
                    );
                }
                self.report(d);
                return None;
            };
            return Some(self.load_std(name, source));
        };
        for (i, seg) in path.iter().enumerate().skip(1) {
            let next = self.prog.lookup_in(m, &seg.text).and_then(|d| match self.prog.def(d).kind {
                DefKind::Module(c) if !self.prog.modules[m.0 as usize].imported.contains_key(&seg.text) => {
                    Some(c)
                }
                _ => None,
            });
            match next {
                Some(c) => m = c,
                None => {
                    self.report(
                        Diagnostic::new(msg::RESOLVE_UNKNOWN_MODULE, at).with("name", words[..=i].join(".")),
                    );
                    return None;
                }
            }
        }
        Some(m)
    }

    /// The name an import that failed would have bound: an undefined use
    /// of it is not reported, since the import was. With a fix that names
    /// a similar module, the name that module would bind is too
    /// (`import std.jsn` and then `json.parse`).
    fn poison_import(&mut self, name: String) {
        if let Some(fix) = self.diags.last().and_then(|d| d.fixes.first())
            && fix.msg == msg::RESOLVE_SIMILAR_MODULE
            && let Some(last) = fix.edits.first().and_then(|e| e.text.rsplit('.').next())
        {
            self.prog.poisoned.insert(last.to_string());
        }
        self.prog.poisoned.insert(name);
    }

    /// Records that an import bound `name` in `module`, with the import's
    /// visibility (a `pub` import re-exports it).
    fn bind_import(&mut self, module: ModuleId, name: &str, def: DefId, range: TextRange, vis: ast::Vis) {
        let m_items = &self.prog.modules[module.0 as usize].items;
        // Importing a module by the name it already has here (a child of
        // this module, or one imported twice) binds nothing new.
        if let Some(&old) = m_items.get(name)
            && (old == def
                || matches!((&self.prog.def(old).kind, &self.prog.def(def).kind),
                    (DefKind::Module(a), DefKind::Module(b)) if a == b))
        {
            return;
        }
        if m_items.contains_key(name) {
            // Reported as a second definition of the name.
            self.bind(module, name, def, range);
            return;
        }
        let mm = &mut self.prog.modules[module.0 as usize];
        mm.items.insert(name.to_string(), def);
        mm.imported.insert(name.to_string(), vis);
    }

    fn import(&mut self, module: ModuleId, imp: &ast::Import, range: TextRange, vis: ast::Vis) {
        let span = self.span(module, range);
        if imp.c_header.is_some() {
            self.unsupported(span, "a C header import", "foreign functions (Phase 8)");
            return;
        }
        let at = match (imp.path.first(), imp.path.last()) {
            (Some(a), Some(b)) => self.span(module, a.range.cover(b.range)),
            _ => span,
        };
        let bound = imp
            .alias
            .as_ref()
            .map(|a| a.text.clone())
            .unwrap_or_else(|| imp.path.last().map(|s| s.text.clone()).unwrap_or_default());
        let Some(m) = self.import_target(module, &imp.path, at) else {
            // The import was reported; its uses are not reported again.
            self.poison_import(bound);
            return;
        };
        let range_of_name =
            imp.alias.as_ref().map(|a| a.range).or(imp.path.last().map(|n| n.range)).unwrap_or(range);
        let def = self.new_def(module, &bound, range_of_name, vis, Vec::new(), DefKind::Module(m));
        if self.is_std(module) {
            self.bind(module, &bound, def, range_of_name);
        } else {
            self.bind_import(module, &bound, def, range_of_name, vis);
        }
    }

    fn import_from(&mut self, module: ModuleId, imp: &ast::FromImport, range: TextRange, vis: ast::Vis) {
        let span = match (imp.path.first(), imp.path.last()) {
            (Some(a), Some(b)) => self.span(module, a.range.cover(b.range)),
            _ => self.span(module, range),
        };
        let Some(m) = self.import_target(module, &imp.path, span) else {
            for (n, alias) in &imp.names {
                self.poison_import(alias.as_ref().unwrap_or(n).text.clone());
            }
            return;
        };
        let path: Vec<&str> = imp.path.iter().map(|n| n.text.as_str()).collect();
        let name = self.prog.modules[m.0 as usize].name.clone();
        let prelude = self.prog.prelude;
        for (n, alias) in &imp.names {
            let found = self.prog.member(m, &n.text, module).or_else(|| {
                let is_std = self.is_std(m);
                (is_std && std_sources::reexports(&name).contains(&n.text.as_str()))
                    .then(|| self.prog.lookup_in(prelude, &n.text).map(|d| (d, true)))
                    .flatten()
            });
            match found {
                Some((d, visible)) => {
                    if !visible {
                        let s = self.span(module, n.range);
                        self.report(self.private_item(d, m, s));
                    }
                    let bound = alias.as_ref().unwrap_or(n);
                    self.bind_import(module, &bound.text, d, bound.range, vis);
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

    /// `name` of `m` is not visible where it is named: private to its
    /// module, or `pub(package)` in another package.
    pub fn private_item(&self, d: DefId, m: ModuleId, span: Span) -> Diagnostic {
        self.prog.private_item(d, m, span)
    }

    // ---------------------------------------------------------------- pass 2

    pub(crate) fn resolve_pending(&mut self) {
        // The generic parameters of every type and trait first, with their
        // defaults, so that a use site anywhere can fill omitted arguments.
        self.resolve_headers();
        // Aliases next, on demand, so that later declarations can use them.
        let mut alias_ids: Vec<DefId> =
            self.aliases.keys().copied().filter(|d| !self.headers_done.contains(d)).collect();
        alias_ids.sort();
        self.headers_done.extend(alias_ids.iter().copied());
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
        let (types, rest): (Vec<Pending>, Vec<Pending>) = pending.into_iter().partition(|p| {
            matches!(p, Pending::Struct { .. } | Pending::Enum { .. } | Pending::Refinement { .. })
        });
        let refinements: Vec<DefId> = types
            .iter()
            .filter_map(|p| match p {
                Pending::Refinement { def, .. } => Some(*def),
                _ => None,
            })
            .collect();
        for p in types {
            self.resolve_one(p);
        }
        self.refinement_domains(&refinements);
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
        self.unfold_values();
    }

    /// Unfolds the values that reflect, now that every type and trait they
    /// may name is resolved (decision 251).
    fn unfold_values(&mut self) {
        for later in std::mem::take(&mut self.unfold_later) {
            match later {
                Later::Const { def, module, value } => {
                    let site = self.span(module, value.range);
                    let file = self.file(module);
                    let name = self.prog.def(def).name.clone();
                    let unfolded = self.unfold_value(module, &name, &value, file, site);
                    if let DefKind::Const(c) = &mut self.prog.defs[def.0 as usize].kind {
                        match unfolded {
                            Some((v, f)) => {
                                c.value = Some(v);
                                c.file = f;
                            }
                            None => c.value = None,
                        }
                    }
                }
                Later::Field { def, index, module, label, value } => {
                    let (v, f) = self.unfolded_default(module, &label, Some(&value));
                    if let DefKind::Struct(st) = &mut self.prog.defs[def.0 as usize].kind
                        && let Some(field) = st.fields.get_mut(index)
                    {
                        field.default = v;
                        field.default_file = f;
                    }
                }
                Later::Param { def, index, module, label, value } => {
                    let (v, f) = self.unfolded_default(module, &label, Some(&value));
                    if let DefKind::Fn(info) = &mut self.prog.defs[def.0 as usize].kind
                        && let Some(p) = info.params.get_mut(index)
                    {
                        p.default = v;
                        p.default_file = f;
                    }
                }
            }
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
        // `type Pair[T] = (T, T)`: the alias's parameters are `Param(i)` in
        // what it stands for, replaced by the arguments where it is used.
        let names: Vec<String> = alias.generics.iter().map(|g| g.name.text.clone()).collect();
        let ctx = TyCtx::plain(module, &names, self.is_std(module));
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
                    if let Some(v) = &f.default
                        && crate::instances::value_unfolds(v)
                    {
                        let label = format!("{}.{}", s.name.text, f.name.text);
                        let index = fields.len();
                        self.unfold_later.push(Later::Field { def, index, module, label, value: v.clone() });
                    }
                    fields.push(FieldInfo {
                        name: f.name.text.clone(),
                        ty,
                        default: f.default.clone(),
                        default_file: self.file(module),
                        span: self.span(module, f.range),
                        vis: f.vis,
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
                                    )
                                    .note(msg::BORROW_OWNED_FIELD, Vec::new()),
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
                            default_file: self.file(module),
                            span: self.span(module, p.range),
                        });
                    }
                }
                info.body = t.body.clone();
                // A test whose body runs code at compile time by unfolding is
                // a template, made once as a function without type
                // parameters is (decision 251), and so is each case of a
                // parametrised one.
                let f = test_fn(&self.prog.def(def).name, &t);
                if crate::instances::unfolds(&f, &[]) && !self.instance_modules.contains(&module) {
                    info.template = true;
                    self.template_asts.insert(def, f);
                }
                info.instance_of = self.instance_of.get(&def).cloned();
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
                // A value that reflects is unfolded where it is written, once
                // everything is resolved, and checked as the text that makes
                // (decision 251).
                if let Some(v) = &c.value
                    && crate::instances::value_unfolds(v)
                {
                    self.unfold_later.push(Later::Const { def, module, value: v.clone() });
                }
                let file = self.file(module);
                self.prog.defs[def.0 as usize].kind =
                    DefKind::Const(ConstInfo { ty, value: c.value.clone(), file });
            }
            Pending::Impl { module, ast: imp, range, derived } => {
                self.resolve_impl(module, &imp, range, derived)
            }
            Pending::Refinement { def, module, ast: a, range } => {
                self.resolve_refinement(def, module, &a, range)
            }
        }
    }

    /// A refinement type's base, and its predicate as a function the
    /// compiler writes: `def X#where(self) -> bool: return pred`, `pure`,
    /// with `self` a value of the base.
    fn resolve_refinement(&mut self, def: DefId, module: ModuleId, a: &ast::TypeAlias, range: TextRange) {
        let names: Vec<String> = a.generics.iter().map(|g| g.name.text.clone()).collect();
        let predicates = self.type_preds.get(&def).cloned().unwrap_or_default();
        let ctx = TyCtx {
            module,
            generics: &names,
            self_ty: None,
            allow_ref: false,
            predicates: &predicates,
            in_trait: None,
            impl_assoc: &[],
        };
        let base = match &a.ty {
            Some(t) => self.resolve_ty(t, &ctx),
            None => Ty::Error,
        };
        // A refinement of a refinement keeps it as its base: its values
        // are values of that one too, and checking one checks both
        // predicates, the base's first. A chain that comes back is a cycle.
        let mut at = base.clone();
        let mut cycle = false;
        for _ in 0..64 {
            match &at {
                Ty::Adt(d, _) if *d == def => {
                    cycle = true;
                    break;
                }
                Ty::Adt(d, args) => match self.prog.refinement_base(*d, args) {
                    Some(b) => at = b,
                    None => break,
                },
                _ => break,
            }
        }
        let base = if cycle {
            let span = self.span(module, a.name.range);
            self.report(Diagnostic::new(msg::RESOLVE_ALIAS_CYCLE, span).with("name", a.name.text.clone()));
            Ty::Error
        } else {
            base
        };
        // The range an integer predicate bounds `self` to, narrowed by the
        // base's once every refinement is resolved.
        let domain = match (&base, &a.predicate) {
            (Ty::Int, Some(p)) => int_domain(p),
            (Ty::Adt(..), Some(p)) => int_domain(p),
            _ => None,
        };
        let pred = a.predicate.clone().unwrap_or(ast::Expr {
            node: sisu_syntax::tree::NodeId(0),
            range,
            kind: ast::ExprKind::Error,
        });
        let pred_def = self.new_def(
            module,
            "where",
            pred.range,
            ast::Vis::Private,
            Vec::new(),
            DefKind::Fn(Box::new(self.placeholder_fn(module, FnKind::Free))),
        );
        if let DefKind::Refinement(r) = &mut self.prog.defs[def.0 as usize].kind {
            r.base = base;
            r.predicate = Some(pred_def);
            r.domain = domain;
        }
        self.special_kinds.insert(pred_def, FnKind::Predicate(def));
        let f = predicate_def(&pred);
        self.pending.push(Pending::Fn { def: pred_def, module, ast: f, owner: Some(Owner::Refinement(def)) });
    }

    /// Narrows each integer refinement's domain by its base's: `Small =
    /// Port where self < 1024` draws from `[1, 1024)`. A refinement whose
    /// root base is not `int` has none.
    fn refinement_domains(&mut self, refinements: &[DefId]) {
        fn domain_of(prog: &Program, d: DefId, depth: u32) -> Option<(i64, i64)> {
            let r = prog.refinement(d)?;
            if depth > 64 || prog.unrefine(&r.base) != Ty::Int {
                return None;
            }
            let inherited = match &r.base {
                Ty::Adt(b, _) => domain_of(prog, *b, depth + 1),
                _ => None,
            };
            match (r.domain, inherited) {
                (Some((a, b)), Some((c, e))) => Some((a.max(c), b.min(e))),
                (x, y) => x.or(y),
            }
        }
        let narrowed: Vec<(DefId, Option<(i64, i64)>)> =
            refinements.iter().map(|d| (*d, domain_of(&self.prog, *d, 0))).collect();
        for (d, dom) in narrowed {
            if let DefKind::Refinement(r) = &mut self.prog.defs[d.0 as usize].kind {
                r.domain = dom;
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
                let mut generics = vec![Generic { name: "Self".into(), default: None, is_const: false }];
                generics.extend(info.generics.iter().cloned());
                let names: Vec<String> = generics.iter().map(|g| g.name.clone()).collect();
                let tr = TraitRef { def: *t, args: (1..generics.len() as u32).map(Ty::Param).collect() };
                let preds = vec![Predicate { ty: Ty::Param(0), bound: tr, span: self.prog.def(*t).span }];
                (generics, names, preds, Some(Ty::Param(0)), Some(*t), None)
            }
            Some(Owner::Refinement(r)) => {
                let generics = self.prog.type_generics(*r).to_vec();
                let names: Vec<String> = generics.iter().map(|g| g.name.clone()).collect();
                let (_, preds) = self.type_param_scope(*r, module);
                let base = self.prog.refinement(*r).map(|x| x.base.clone()).unwrap_or(Ty::Error);
                (generics, names, preds, Some(base), None, None)
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
        // A `comptime def`'s `T: type` parameters are generic parameters
        // whose arguments a call writes among its arguments (decision 251).
        let type_params: Vec<(usize, String)> =
            if f.is_comptime { crate::template::type_params(f) } else { Vec::new() };
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
            generics.push(Generic { name: g.name.text.clone(), default: None, is_const: g.is_const });
        }
        let mut type_param_info = Vec::new();
        for (at, n) in &type_params {
            type_param_info.push((*at, n.clone(), generics.len() as u32));
            names.push(n.clone());
            generics.push(Generic { name: n.clone(), default: None, is_const: false });
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
        info.type_params = type_param_info;
        info.body = f.body.clone();
        info.intrinsic = Self::builtin_attr(&f.attrs, "intrinsic");
        info.structural = Self::builtin_attr(&f.attrs, "structural");
        info.internal = f.attrs.iter().any(|a| a.name == "internal");
        info.export = Self::builtin_attr(&f.attrs, "export");
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
                ParamKind::Named { name, .. } if type_params.iter().any(|(_, n)| *n == name.text) => {}
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
                    if let Some(v) = default
                        && crate::instances::value_unfolds(v)
                    {
                        let label = format!("{}.{}", f.name.text, name.text);
                        let index = info.params.len();
                        self.unfold_later.push(Later::Param { def, index, module, label, value: v.clone() });
                    }
                    info.params.push(ParamInfo {
                        name: name.text.clone(),
                        mode: p.mode,
                        ty,
                        default: default.clone(),
                        default_file: self.file(module),
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
                // A refinement's predicate is nobody's method.
                Owner::Refinement(_) => {}
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
        if let Some(k) = self.special_kinds.get(&def) {
            info.kind = k.clone();
        }
        info.quiet = self.quiet_fns.contains(&def);
        // A function whose body runs code at compile time by unfolding, or
        // that was promoted for calling a template with its own type
        // parameters, is a template (08, "Reflection"; decision 244): a
        // free function, a method of a type or of an impl, and a `comptime
        // def` that reflects or has `T: type` parameters, which are type
        // parameters made per type as a template's are (decision 251). It
        // is made for each list of type arguments it is called with, and
        // once, before anything calls it, when it has none; a method of a
        // trait impl is made once whatever its type parameters, which stay
        // parameters, so that a trait's call reaches it (decision 251).
        let unfolding = crate::instances::unfolds(f, &names)
            || self.promoted.contains(&def)
            || !info.type_params.is_empty();
        // A trait's own method that reflects is made in each impl that does
        // not write it, as that impl's template (decision 251): it is a
        // template only so that nothing checks it as written.
        info.template = unfolding
            && !self.instance_modules.contains(&module)
            && match &owner {
                None | Some(Owner::Type(_)) | Some(Owner::Inherent { .. }) => true,
                Some(Owner::Impl(_)) => true,
                Some(Owner::Trait(_)) => f.body.is_some(),
                Some(Owner::Refinement(_)) => false,
            };
        if info.template {
            self.template_asts.insert(def, f.clone());
        }
        info.instance_of = self.instance_of.get(&def).cloned();
        // A test's instance is the test (decision 251), also once it is
        // resolved, which makes its `FnInfo` again (decision 259).
        if let Some((template, _)) = &info.instance_of
            && let Some(kind @ FnKind::Test(_)) = self.prog.fn_info(*template).map(|t| &t.kind)
        {
            info.kind = kind.clone();
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

impl Resolver<'_> {
    /// A field's or a parameter's default as it is checked, and the file it
    /// is in: as written, or unfolded when it reflects (decision 251).
    fn unfolded_default(
        &mut self,
        module: ModuleId,
        label: &str,
        default: Option<&ast::Expr>,
    ) -> (Option<ast::Expr>, FileId) {
        let file = self.file(module);
        match default {
            Some(e) if crate::instances::value_unfolds(e) => {
                let site = self.span(module, e.range);
                match self.unfold_value(module, label, e, file, site) {
                    Some((v, f)) => (Some(v), f),
                    None => (None, file),
                }
            }
            other => (other.cloned(), file),
        }
    }
}

/// A value that reflects, unfolded once everything is resolved (decision
/// 251): a constant's, a struct field's default, a parameter's default.
pub(crate) enum Later {
    Const { def: DefId, module: ModuleId, value: ast::Expr },
    Field { def: DefId, index: usize, module: ModuleId, label: String, value: ast::Expr },
    Param { def: DefId, index: usize, module: ModuleId, label: String, value: ast::Expr },
}

/// A test as the function its body is, for unfolding it (decision 251):
/// its name's range starts the header the instance's is cut from.
fn test_fn(name: &str, t: &ast::TestDef) -> ast::FnDef {
    let node = t.body.as_ref().map(|b| b.node).unwrap_or(sisu_syntax::tree::NodeId(0));
    ast::FnDef {
        node,
        range: t.name_range,
        name: ast::Name { text: name.to_string(), range: t.name_range },
        is_pure: false,
        is_comptime: false,
        generics: Vec::new(),
        params: t.params.clone(),
        ret: None,
        from: Vec::new(),
        where_: Vec::new(),
        body: t.body.clone(),
        attrs: Vec::new(),
        header_broken: false,
        body_broken: false,
        vis: ast::Vis::Private,
    }
}

/// Reports a diagnostic message id at a span; used by tests.
#[allow(dead_code)]
fn diag(m: MsgId, span: Span) -> Diagnostic {
    Diagnostic::new(m, span)
}

/// The method that checks a struct's `invariant` lines (07): `pure def
/// __check_invariant(self)`, whose body is the lines themselves, each a
/// check that panics naming the line and the values in it. It is
/// `@internal`: only the compiler's checks call it.
fn invariant_check_def(node: sisu_syntax::tree::NodeId, s: &ast::StructDef) -> ast::FnDef {
    let range = s.invariants.first().map(|c| c.range).unwrap_or(s.name.range);
    let stmts = s
        .invariants
        .iter()
        .map(|c| ast::Stmt { node, range: c.range, kind: ast::StmtKind::Contract(c.clone()) })
        .collect();
    ast::FnDef {
        node,
        range,
        name: ast::Name { text: "__check_invariant".into(), range },
        is_pure: true,
        is_comptime: false,
        generics: Vec::new(),
        params: vec![ast::Param { node, range, mode: ast::Mode::Borrow, kind: ParamKind::SelfParam }],
        ret: None,
        from: Vec::new(),
        where_: Vec::new(),
        body: Some(std::sync::Arc::new(ast::Block { node, range, stmts })),
        attrs: vec![ast::Attr { name: "internal".into(), args: Vec::new(), range }],
        header_broken: false,
        body_broken: false,
        vis: ast::Vis::Private,
    }
}

/// The invariant as a `bool`: `pure def __invariant(self) -> bool:
/// return (e1) and (e2) ...`, `@internal`.
fn invariant_holds_def(node: sisu_syntax::tree::NodeId, s: &ast::StructDef) -> ast::FnDef {
    let range = s.invariants.first().map(|c| c.range).unwrap_or(s.name.range);
    let mut cond: Option<ast::Expr> = None;
    for c in &s.invariants {
        let part =
            ast::Expr { node, range: c.expr.range, kind: ast::ExprKind::Paren(Box::new(c.expr.clone())) };
        cond = Some(match cond {
            None => part,
            Some(l) => ast::Expr {
                node,
                range,
                kind: ast::ExprKind::Binary { op: ast::BinOp::And, lhs: Box::new(l), rhs: Box::new(part) },
            },
        });
    }
    let cond = cond.unwrap_or(ast::Expr { node, range, kind: ast::ExprKind::Name("True".into()) });
    let mut f = predicate_def(&cond);
    f.name = ast::Name { text: "__invariant".into(), range };
    f.attrs = vec![ast::Attr { name: "internal".into(), args: Vec::new(), range }];
    f
}

/// A refinement's predicate as a function of `self`: `pure def
/// where(self) -> bool: return pred`.
fn predicate_def(pred: &ast::Expr) -> ast::FnDef {
    let node = pred.node;
    let range = pred.range;
    let ret = ast::TypeExpr {
        node,
        range,
        kind: TypeKind::Path { segments: vec![ast::Name { text: "bool".into(), range }], args: Vec::new() },
    };
    let body = ast::Block {
        node,
        range,
        stmts: vec![ast::Stmt { node, range, kind: ast::StmtKind::Return(Some(pred.clone())) }],
    };
    ast::FnDef {
        node,
        range,
        name: ast::Name { text: "where".into(), range },
        is_pure: true,
        is_comptime: false,
        generics: Vec::new(),
        params: vec![ast::Param { node, range, mode: ast::Mode::Borrow, kind: ParamKind::SelfParam }],
        ret: Some(ret),
        from: Vec::new(),
        where_: Vec::new(),
        body: Some(std::sync::Arc::new(body)),
        attrs: Vec::new(),
        header_broken: false,
        body_broken: false,
        vis: ast::Vis::Private,
    }
}

/// Where each item that can carry a doc comment starts, with the name its
/// doc examples' tests are named after: a method as `Type.method`.
fn item_starts(items: &[ast::Item]) -> Vec<(u32, String)> {
    let mut out = Vec::new();
    for item in items {
        let start = item.range.start;
        let methods = |owner: &str, ms: &[ast::FnDef], out: &mut Vec<(u32, String)>| {
            for m in ms {
                out.push((m.range.start, format!("{owner}.{}", m.name.text)));
            }
        };
        match &item.kind {
            ItemKind::Fn(f) => out.push((start, f.name.text.clone())),
            ItemKind::Struct(s) => {
                out.push((start, s.name.text.clone()));
                methods(&s.name.text, &s.methods, &mut out);
            }
            ItemKind::Enum(e) | ItemKind::ErrorSet(e) => {
                out.push((start, e.name.text.clone()));
                methods(&e.name.text, &e.methods, &mut out);
            }
            ItemKind::Trait(t) => {
                out.push((start, t.name.text.clone()));
                methods(&t.name.text, &t.methods, &mut out);
            }
            ItemKind::Impl(i) => methods(&owner_name(&i.self_ty), &i.methods, &mut out),
            ItemKind::Const(c) => out.push((start, c.name.text.clone())),
            ItemKind::TypeAlias(a) => out.push((start, a.name.text.clone())),
            _ => {}
        }
    }
    out
}

/// The name a doc test gives the type an `impl` is for: its path, with
/// `T?` named `Option` and `T ! E` named `Result`, as they are declared.
fn owner_name(ty: &ast::TypeExpr) -> String {
    match &ty.kind {
        TypeKind::Path { segments, .. } => {
            segments.iter().map(|s| s.text.clone()).collect::<Vec<_>>().join(".")
        }
        TypeKind::Option(_) => "Option".to_string(),
        TypeKind::Result { .. } => "Result".to_string(),
        TypeKind::Ref { inner, .. } => owner_name(inner),
        _ => "impl".to_string(),
    }
}

/// A standard-library module's syntax, parsed once per process: its text
/// never changes, and every program resolves it again. The tree holds no
/// file; the diagnostics name `FileId(0)`.
fn std_ast(
    name: &'static str,
    source: &'static str,
) -> (std::sync::Arc<ast::Module>, Vec<sisu_diag::Diagnostic>) {
    type Parsed = (std::sync::Arc<ast::Module>, Vec<sisu_diag::Diagnostic>);
    static CACHE: std::sync::OnceLock<std::sync::Mutex<HashMap<&'static str, Parsed>>> =
        std::sync::OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Ok(c) = cache.lock()
        && let Some(p) = c.get(name)
    {
        return p.clone();
    }
    let parse = sisu_parser::parse(FileId(0), source);
    let parsed: Parsed = (std::sync::Arc::new(ast::build::module(&parse.tree)), parse.diagnostics);
    if let Ok(mut c) = cache.lock() {
        c.insert(name, parsed.clone());
    }
    parsed
}

/// The syntax of source text the compiler wrote (a derived impl, an
/// instance of a template, the generators and shims it writes), parsed once
/// per process for each text: the same declarations give the same text in
/// every resolution of every program (the per-item queries of
/// docs/architecture.md). Its diagnostics name `FileId(0)`.
pub(crate) fn parse_generated(text: &str) -> (std::sync::Arc<ast::Module>, Vec<sisu_diag::Diagnostic>) {
    type Parsed = (std::sync::Arc<str>, std::sync::Arc<ast::Module>, Vec<sisu_diag::Diagnostic>);
    static CACHE: std::sync::OnceLock<std::sync::Mutex<HashMap<u64, Parsed>>> = std::sync::OnceLock::new();
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut h);
    let key = h.finish();
    let cache = CACHE.get_or_init(Default::default);
    if let Ok(c) = cache.lock()
        && let Some((t, m, d)) = c.get(&key)
        && **t == *text
    {
        return (m.clone(), d.clone());
    }
    let parse = sisu_parser::parse(FileId(0), text);
    let module = std::sync::Arc::new(ast::build::module(&parse.tree));
    if let Ok(mut c) = cache.lock() {
        // A long session writes many texts; the oldest are not kept.
        if c.len() > 4096 {
            c.clear();
        }
        c.insert(key, (std::sync::Arc::from(text), module.clone(), parse.diagnostics.clone()));
    }
    (module, parse.diagnostics)
}

/// A script's top-level statements as the body of its `main` (10, "REPL
/// and scripts"): `def main(sys: System) -> !:` with the statements in
/// order and `return Ok(())` after them, so a script may use `sys` and
/// `?` as a function may.
fn script_main(items: &[ast::Item]) -> Option<ast::Item> {
    let stmts: Vec<ast::Stmt> = items
        .iter()
        .filter_map(|i| match &i.kind {
            ItemKind::Stmt(s) => Some(s.clone()),
            _ => None,
        })
        .collect();
    let first = stmts.first()?;
    let last = stmts.last()?;
    let node = first.node;
    let at = TextRange::empty(first.range.start);
    let end = TextRange::empty(last.range.end);
    let name = |t: &str| ast::Name { text: t.into(), range: at };
    let path = |t: &str| ast::TypeExpr {
        node,
        range: at,
        kind: TypeKind::Path { segments: vec![name(t)], args: Vec::new() },
    };
    let expr = |kind: ExprKind| ast::Expr { node, range: end, kind };
    let ok = expr(ExprKind::Call {
        callee: Box::new(expr(ExprKind::Name("Ok".into()))),
        args: vec![ast::Arg {
            node,
            range: end,
            name: None,
            mutable: false,
            value: expr(ExprKind::Tuple(Vec::new())),
        }],
        trailing: None,
    });
    let mut body = stmts.clone();
    if !matches!(last.kind, ast::StmtKind::Return(_)) {
        body.push(ast::Stmt { node, range: end, kind: ast::StmtKind::Return(Some(ok)) });
    }
    let range = TextRange::new(first.range.start, last.range.end);
    let f = ast::FnDef {
        node,
        range,
        name: name("main"),
        is_pure: false,
        is_comptime: false,
        generics: Vec::new(),
        params: vec![ast::Param {
            node,
            range: at,
            mode: ast::Mode::Borrow,
            kind: ParamKind::Named { name: name("sys"), ty: path("System"), default: None },
        }],
        ret: Some(ast::TypeExpr { node, range: at, kind: TypeKind::Result { ok: None, err: None } }),
        from: Vec::new(),
        where_: Vec::new(),
        body: Some(std::sync::Arc::new(ast::Block { node, range, stmts: body })),
        attrs: Vec::new(),
        header_broken: false,
        body_broken: false,
        vis: ast::Vis::Private,
    };
    Some(ast::Item {
        node,
        range,
        attrs: Vec::new(),
        vis: ast::Vis::Private,
        docs: Vec::new(),
        kind: ItemKind::Fn(f),
    })
}

/// The range `[lo, hi)` an integer refinement's predicate bounds `self`
/// to: a conjunction (`and`) of comparisons of `self` with integer
/// literals, chains included (`1 <= self <= 65535`). Any other predicate
/// gives no domain.
pub(crate) fn int_domain(p: &ast::Expr) -> Option<(i64, i64)> {
    fn lit(e: &ast::Expr) -> Option<i64> {
        match &e.kind {
            ExprKind::Int { value: Some(v), suffix: None, .. } => i64::try_from(*v).ok(),
            ExprKind::Unary { op: ast::UnOp::Neg, expr } => lit(expr).map(|v| -v),
            ExprKind::Paren(inner) => lit(inner),
            _ => None,
        }
    }
    fn is_self(e: &ast::Expr) -> bool {
        match &e.kind {
            ExprKind::SelfValue => true,
            ExprKind::Paren(inner) => is_self(inner),
            _ => false,
        }
    }
    // Narrows `[lo, hi)` by `a op b` where one side is `self`.
    fn narrow(lo: &mut i64, hi: &mut i64, a: &ast::Expr, op: ast::CmpOp, b: &ast::Expr) -> Option<()> {
        use ast::CmpOp::*;
        let (op, k) = if is_self(a) {
            (op, lit(b)?)
        } else if is_self(b) {
            // `k op self` is `self op' k`.
            let flipped = match op {
                Lt => Gt,
                Le => Ge,
                Gt => Lt,
                Ge => Le,
                Eq => Eq,
                _ => return None,
            };
            (flipped, lit(a)?)
        } else {
            return None;
        };
        match op {
            Lt => *hi = (*hi).min(k),
            Le => *hi = (*hi).min(k.checked_add(1)?),
            Gt => *lo = (*lo).max(k.checked_add(1)?),
            Ge => *lo = (*lo).max(k),
            Eq => {
                *lo = (*lo).max(k);
                *hi = (*hi).min(k.checked_add(1)?);
            }
            _ => return None,
        }
        Some(())
    }
    fn walk(e: &ast::Expr, lo: &mut i64, hi: &mut i64) -> Option<()> {
        match &e.kind {
            ExprKind::Paren(inner) => walk(inner, lo, hi),
            ExprKind::Binary { op: ast::BinOp::And, lhs, rhs } => {
                walk(lhs, lo, hi)?;
                walk(rhs, lo, hi)
            }
            ExprKind::Compare { first, rest } => {
                let mut left: &ast::Expr = first;
                for (op, right) in rest {
                    narrow(lo, hi, left, *op, right)?;
                    left = right;
                }
                Some(())
            }
            _ => None,
        }
    }
    let (mut lo, mut hi) = (i64::MIN, i64::MAX);
    walk(p, &mut lo, &mut hi)?;
    (lo > i64::MIN && hi < i64::MAX && lo < hi).then_some((lo, hi))
}
