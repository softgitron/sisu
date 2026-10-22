//! The query layer: `source -> tokens -> syntax tree -> resolved module ->
//! typed bodies -> SIR -> checked SIR`, each step memoised on the text it
//! was computed from (18, Phase 0, "Query skeleton"). It is an
//! implementation detail: no result depends on whether a query was cached.
//! The last step is the ownership checker, which places the drops; its
//! SIR is the one that runs, even when it reports errors, so that the
//! counterexample mode can run a rejected program.
//!
//! A program's query is keyed by the texts of its files; inside it, the
//! checker, lowering and the ownership checker are memoised per item
//! (docs/architecture.md, "The query layer"): resolution runs again, and
//! gives the same definitions as long as the program's interface (a hash
//! of what it reads, `items.rs`) is the same, so a body whose item's text
//! did not change is not checked, lowered or borrow-checked again. The
//! standard library is fixed for the life of the process.

use sisu_diag::{Diagnostic, FileId, SourceMap};
use sisu_parser::Parse;
use sisu_resolve::Program;
use sisu_sir as sir;
use sisu_syntax::ast;
use sisu_typeck::Typed;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

/// A parsed file.
pub struct Parsed {
    pub map: SourceMap,
    pub file: FileId,
    pub parse: Parse,
    pub module: ast::Module,
    /// What resolution reads of it (`items::interface_hash`).
    pub iface: u64,
}

/// A resolved and type-checked program, with every diagnostic so far.
pub struct Checked {
    pub map: SourceMap,
    pub file: FileId,
    pub program: Program,
    pub typed: Typed,
    /// Syntax, name and type diagnostics, in source order.
    pub diagnostics: Vec<Diagnostic>,
    /// The program's interface as the last resolution saw it: the key of
    /// the per-item queries after this one.
    pub iface: u64,
}

impl Checked {
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.is_error())
    }

    /// Whether an error stops lowering: every error but the ownership
    /// rules about declarations (`E04xx` from name resolution, such as a
    /// borrow in a plain struct), which leave the program well-formed, so
    /// the counterexample mode can still run it.
    pub fn blocks_lowering(&self) -> bool {
        self.diagnostics.iter().any(|d| d.is_error() && !is_ownership_rule(d))
    }
}

/// The most times the query checks a program for the instances it needs.
const MAX_ROUNDS: usize = 32;

/// The files the program's `embed("path")` calls name, read relative to
/// the program's own file (08): each text, or why it could not be read.
fn embedded_files(
    name: &str,
    module: &ast::Module,
) -> std::collections::HashMap<String, Result<String, String>> {
    let mut out = std::collections::HashMap::new();
    let dir = std::path::Path::new(name).parent().map(|p| p.to_path_buf()).unwrap_or_default();
    ast::visit::exprs_in_items(&module.items, &mut |e| {
        if let ast::ExprKind::Call { callee, args, .. } = &e.kind
            && matches!(&callee.kind, ast::ExprKind::Name(n) if n == "embed")
            && let [a] = args.as_slice()
            && let ast::ExprKind::Str(parts) = &a.value.kind
            && parts.iter().all(|p| matches!(p, ast::StrPart::Text(_)))
        {
            let path = ast::build::plain_text(parts);
            if out.contains_key(&path) {
                return;
            }
            let read = if std::path::Path::new(&path).is_absolute() || path.split('/').any(|p| p == "..") {
                Err("the path must stay inside the program's directory".to_string())
            } else {
                // The reason in words of its own, the same on every machine.
                std::fs::read_to_string(dir.join(&path)).map_err(|e| match e.kind() {
                    std::io::ErrorKind::NotFound => "there is no such file".to_string(),
                    std::io::ErrorKind::PermissionDenied => "it may not be read".to_string(),
                    std::io::ErrorKind::InvalidData => "it is not UTF-8 text".to_string(),
                    _ => "it cannot be read".to_string(),
                })
            };
            out.insert(path, read);
        }
    });
    out
}

/// A diagnostic of the ownership family (`E04xx`).
pub fn is_ownership_rule(d: &Diagnostic) -> bool {
    d.code().starts_with("E04")
}

/// How many times each query ran (not counting cache hits), and, for the
/// per-item queries, how many items the last run of each did and reused.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counters {
    pub parse: u32,
    pub check: u32,
    pub lower: u32,
    /// Bodies the last `checked` checked (in all its rounds), and reused.
    pub bodies_checked: u32,
    pub bodies_reused: u32,
    /// Bodies the last `lowered` lowered, and reused.
    pub bodies_lowered: u32,
    pub lowered_reused: u32,
    /// Functions the last `lowered` put through the ownership checker's
    /// analyses, and the ones it took as checked.
    pub functions_checked: u32,
    pub functions_reused: u32,
}

/// The per-item memos of one program.
#[derive(Default)]
struct Memos {
    bodies: sisu_typeck::BodyCache,
    lower: sisu_lower::LowerCache,
    borrow: sisu_borrowck::FnCache,
}

/// A program without front-end errors, lowered and through the
/// ownership checker.
pub struct Lowered {
    /// The SIR with its drops placed: what runs.
    pub sir: sir::Program,
    /// The ownership checker's findings, by SIR position.
    pub findings: Vec<sisu_borrowck::Finding>,
    /// Each finding as a diagnostic, in the findings' order.
    pub diagnostics_by_finding: Vec<Diagnostic>,
    /// The diagnostics in source order.
    pub diagnostics: Vec<Diagnostic>,
    /// The errors of compile-time evaluation (also in `diagnostics`), which
    /// runs once the ownership checker accepts the program.
    pub comptime: Vec<Diagnostic>,
}

impl Lowered {
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.is_error())
    }
}

/// A program of several modules (09): each module's path, file and
/// package, the packages, and the problems the manifests and module trees
/// had, with the manifests they point into. A file without one is a
/// program of one module.
#[derive(Clone, Debug, Default)]
pub struct ProgramSource {
    /// Each module: its path, its file's name (as [`Db::set_file`] named
    /// it) and its package (an index of `packages`). The root module first.
    pub modules: Vec<(Vec<String>, String, usize)>,
    pub packages: Vec<sisu_resolve::SourcePackage>,
    /// The manifests the problems point into (`FileId(k)` is the `k`th),
    /// as (name, text).
    pub manifests: Vec<(String, String)>,
    pub problems: Vec<Diagnostic>,
    /// The root module is a script (`#!` on its first line, 10).
    pub script: bool,
    /// The directory the files' names are relative to (where `embed`
    /// finds a module's files).
    pub base: std::path::PathBuf,
    /// `sisu test --std`: the standard library's doc examples are tests of
    /// the program too.
    pub std_doc_tests: bool,
}

impl ProgramSource {
    /// The program a layout describes.
    pub fn from_layout(l: &crate::package::Layout) -> ProgramSource {
        ProgramSource {
            modules: l.modules.iter().map(|m| (m.path.clone(), m.name.clone(), m.package)).collect(),
            packages: l
                .packages
                .iter()
                .map(|p| sisu_resolve::SourcePackage {
                    name: p.name.clone(),
                    deps: p.deps.clone(),
                    trusted: p.trusted,
                })
                .collect(),
            manifests: l.files.clone(),
            problems: l.diagnostics.clone(),
            script: l.script,
            base: l.base.clone(),
            std_doc_tests: false,
        }
    }

    /// The layout as text (TOML), for a capability log, so a recorded run
    /// of a package replays without its files.
    pub fn write(&self) -> String {
        use crate::toml::quote;
        let mut out = format!("script = {}\n", self.script);
        for p in &self.packages {
            let deps: Vec<String> = p.deps.iter().map(|(n, d)| format!("[{}, {d}]", quote(n))).collect();
            out.push_str(&format!(
                "\n[[package]]\nname = {}\ntrusted = {}\ndeps = [{}]\n",
                quote(&p.name),
                p.trusted,
                deps.join(", ")
            ));
        }
        for (path, file, package) in &self.modules {
            let path: Vec<String> = path.iter().map(|x| quote(x)).collect();
            out.push_str(&format!(
                "\n[[module]]\npath = [{}]\nfile = {}\npackage = {package}\n",
                path.join(", "),
                quote(file)
            ));
        }
        out
    }

    /// Reads [`ProgramSource::write`]'s text back.
    pub fn read(text: &str) -> Option<ProgramSource> {
        use crate::toml::Value;
        let doc = crate::toml::parse(text).ok()?;
        let mut p = ProgramSource {
            script: matches!(doc.get("script")?.value, Value::Bool(true)),
            ..Default::default()
        };
        let items = |key: &str| match doc.get(key).map(|v| &v.value) {
            Some(Value::Array(xs)) => xs.clone(),
            _ => Vec::new(),
        };
        for pk in items("package") {
            let t = pk.value.as_table()?;
            let mut deps = Vec::new();
            if let Some(Value::Array(ds)) = t.get("deps").map(|v| &v.value) {
                for d in ds {
                    let Value::Array(pair) = &d.value else { return None };
                    let name = pair.first()?.value.as_str()?.to_string();
                    let Value::Int(i) = pair.get(1)?.value else { return None };
                    deps.push((name, usize::try_from(i).ok()?));
                }
            }
            p.packages.push(sisu_resolve::SourcePackage {
                name: t.get("name")?.value.as_str()?.to_string(),
                deps,
                trusted: matches!(t.get("trusted")?.value, Value::Bool(true)),
            });
        }
        for m in items("module") {
            let t = m.value.as_table()?;
            let Value::Array(path) = &t.get("path")?.value else { return None };
            let path = path.iter().filter_map(|x| x.value.as_str().map(str::to_string)).collect();
            let Value::Int(package) = t.get("package")?.value else { return None };
            p.modules.push((
                path,
                t.get("file")?.value.as_str()?.to_string(),
                usize::try_from(package).ok()?,
            ));
        }
        Some(p)
    }

    fn key(&self) -> u64 {
        let mut h = DefaultHasher::new();
        for (path, file, package) in &self.modules {
            path.hash(&mut h);
            file.hash(&mut h);
            package.hash(&mut h);
        }
        for p in &self.packages {
            p.name.hash(&mut h);
            p.deps.hash(&mut h);
            p.trusted.hash(&mut h);
        }
        for (n, t) in &self.manifests {
            n.hash(&mut h);
            t.hash(&mut h);
        }
        self.script.hash(&mut h);
        self.std_doc_tests.hash(&mut h);
        h.finish()
    }
}

/// `d` with every span in file `from` moved to file `to`.
pub fn refile(mut d: Diagnostic, from: FileId, to: FileId) -> Diagnostic {
    let mv = |s: &mut sisu_diag::Span| {
        if s.file == from {
            s.file = to;
        }
    };
    mv(&mut d.span);
    d.labels.iter_mut().for_each(|l| mv(&mut l.span));
    for f in &mut d.fixes {
        f.edits.iter_mut().for_each(|e| mv(&mut e.span));
    }
    d
}

#[derive(Default)]
pub struct Db {
    texts: HashMap<String, Arc<str>>,
    /// The programs of several modules, by their root module's file.
    programs: HashMap<String, ProgramSource>,
    parsed: HashMap<String, (u64, Arc<Parsed>)>,
    checked: HashMap<String, (u64, Arc<Checked>)>,
    lowered: HashMap<String, (u64, Arc<Lowered>)>,
    /// The per-item memos, by the program's root module.
    memos: HashMap<String, Memos>,
    pub counters: Counters,
    /// Which contracts the lowered program checks (07); part of the
    /// lowered query's key.
    pub contracts: sisu_lower::ContractLevel,
}

fn hash(text: &str) -> u64 {
    let mut h = DefaultHasher::new();
    text.hash(&mut h);
    h.finish()
}

impl Db {
    pub fn new() -> Db {
        Db::default()
    }

    /// Sets a file's text; queries over it recompute on next use if the
    /// text changed.
    pub fn set_file(&mut self, name: &str, text: &str) {
        self.texts.insert(name.to_string(), Arc::from(text));
    }

    pub fn text(&self, name: &str) -> Option<Arc<str>> {
        self.texts.get(name).cloned()
    }

    /// Makes `root` the root module of a program of several modules; the
    /// texts of its files are set with [`Db::set_file`].
    pub fn set_program(&mut self, root: &str, program: ProgramSource) {
        self.programs.insert(root.to_string(), program);
    }

    /// The program whose root module is `name`: the one set, else the file
    /// alone.
    pub fn program(&self, name: &str) -> ProgramSource {
        self.programs.get(name).cloned().unwrap_or_else(|| ProgramSource {
            modules: vec![(Vec::new(), name.to_string(), 0)],
            packages: vec![sisu_resolve::SourcePackage {
                name: String::new(),
                deps: Vec::new(),
                trusted: true,
            }],
            ..ProgramSource::default()
        })
    }

    /// The text of a file and a hash of everything a query over the
    /// program rooted at it reads: every module's text and the layout.
    fn key(&self, name: &str) -> Option<(Arc<str>, u64)> {
        let t = self.texts.get(name)?.clone();
        let Some(p) = self.programs.get(name) else {
            let h = hash(&t);
            return Some((t, h));
        };
        let mut h = DefaultHasher::new();
        p.key().hash(&mut h);
        for (_, file, _) in &p.modules {
            self.texts.get(file).map(|x| hash(x)).hash(&mut h);
        }
        Some((t, h.finish()))
    }

    pub fn parsed(&mut self, name: &str) -> Option<Arc<Parsed>> {
        let text = self.texts.get(name)?.clone();
        let h = hash(&text);
        if let Some((k, p)) = self.parsed.get(name)
            && *k == h
        {
            return Some(p.clone());
        }
        self.counters.parse += 1;
        let mut map = SourceMap::new();
        let file = map.add(name, &*text);
        let parse = sisu_parser::parse(file, &text);
        let module = ast::build::module(&parse.tree);
        let iface = crate::items::interface_hash(&parse.tree, &module);
        let p = Arc::new(Parsed { map, file, parse, module, iface });
        self.parsed.insert(name.to_string(), (h, p.clone()));
        Some(p)
    }

    pub fn checked(&mut self, name: &str) -> Option<Arc<Checked>> {
        let (_, h) = self.key(name)?;
        if let Some((k, c)) = self.checked.get(name)
            && *k == h
        {
            return Some(c.clone());
        }
        let program = self.program(name);
        let mut files = Vec::new();
        for (_, file, _) in &program.modules {
            files.push(self.parsed(file)?);
        }
        let parsed = files.first()?.clone();
        self.counters.check += 1;
        // One source map for the whole program: the modules' files first,
        // the root module's as file 0, then the manifests.
        let mut base_map = SourceMap::new();
        for p in &files {
            let f = p.map.get(p.file);
            base_map.add(
                f.map(|f| f.name.clone()).unwrap_or_default(),
                f.map(|f| f.text.clone()).unwrap_or_default(),
            );
        }
        let manifest_base = files.len() as u32;
        for (n, t) in &program.manifests {
            base_map.add(n.clone(), t.clone());
        }
        // The files `embed` names, read once, relative to each module's
        // file.
        let mut embedded = HashMap::new();
        for ((_, file, _), p) in program.modules.iter().zip(&files) {
            let on_disk = program.base.join(file).display().to_string();
            for (k, v) in embedded_files(&on_disk, &p.module) {
                embedded.entry(k).or_insert(v);
            }
        }
        let modules: Vec<sisu_resolve::SourceModule> = program
            .modules
            .iter()
            .zip(&files)
            .enumerate()
            .map(|(i, ((path, _, package), p))| sisu_resolve::SourceModule {
                path,
                file: FileId(i as u32),
                ast: &p.module,
                package: *package,
            })
            .collect();
        let sources = sisu_resolve::Sources {
            modules: &modules,
            packages: &program.packages,
            std_doc_tests: program.std_doc_tests,
        };
        // The program's interface: everything resolution reads (the
        // layout, each module's interface, the files `embed` names), and
        // in each round the instances asked for.
        let mut base = DefaultHasher::new();
        program.key().hash(&mut base);
        for p in &files {
            p.iface.hash(&mut base);
        }
        let mut embeds: Vec<(&String, &Result<String, String>)> = embedded.iter().collect();
        embeds.sort();
        embeds.hash(&mut base);
        let memos = self.memos.entry(name.to_string()).or_default();
        memos.bodies.begin();
        // The instances of templates the program needs (08, "Reflection"):
        // check, declare what the check asked for, and check again, until
        // it asks for nothing new. An instance is declared after everything
        // else, so the resolution of one round takes the next round's
        // instances, and the bodies checked are checked once; a promotion
        // (a generic function that calls a template with its own type
        // parameters becomes one) changes a declaration, and resolves again.
        let mut requests: Vec<sisu_resolve::Request> = Vec::new();
        let mut rounds = 0;
        let (map, r, typed, mut type_diags, iface) = loop {
            let mut map = base_map.clone();
            let mut h = base.clone();
            for q in &requests {
                if let sisu_resolve::Request::Promote(def) = q {
                    def.hash(&mut h);
                }
            }
            let iface = h.finish();
            let outcome = {
                let mut session = sisu_resolve::resolve_session(&mut map, sources, &requests, &embedded);
                loop {
                    let (typed, type_diags) = sisu_typeck::check_with(
                        session.program(),
                        session.sources(),
                        Some((&mut memos.bodies, iface)),
                    );
                    let fresh: Vec<sisu_resolve::Request> = typed
                        .requests
                        .iter()
                        .filter(|q| !requests.iter().any(|r| r.same(q)))
                        .cloned()
                        .collect();
                    rounds += 1;
                    if fresh.is_empty() || rounds > MAX_ROUNDS {
                        break Some((session.finish(), typed, type_diags));
                    }
                    let promotes = fresh.iter().any(|q| matches!(q, sisu_resolve::Request::Promote(_)));
                    requests.extend(fresh.iter().cloned());
                    if promotes {
                        break None;
                    }
                    session.add_instances(&fresh);
                }
            };
            if let Some((r, typed, type_diags)) = outcome {
                // What lowering and the ownership checker key on: the
                // interface with every instance.
                let mut h = DefaultHasher::new();
                iface.hash(&mut h);
                for q in &requests {
                    if let sisu_resolve::Request::Instance { def, args, .. } = q {
                        (def, args).hash(&mut h);
                    }
                }
                break (map, r, typed, type_diags, h.finish());
            }
        };
        memos.bodies.sweep();
        self.counters.bodies_checked = memos.bodies.checked;
        self.counters.bodies_reused = memos.bodies.reused;
        if let Some(sisu_resolve::Request::Instance { def, .. } | sisu_resolve::Request::Promote(def)) =
            typed.requests.iter().find(|q| !requests.iter().any(|r| r.same(q)))
        {
            type_diags.push(
                Diagnostic::new(sisu_diag::msg::COMPTIME_INSTANCES_UNBOUNDED, r.program.def(*def).span)
                    .with("name", r.program.def(*def).name.clone())
                    .with("count", requests.len().to_string()),
            );
        }
        // Each module's syntax diagnostics, in its file of the program.
        let mut diagnostics: Vec<Diagnostic> = Vec::new();
        for (i, p) in files.iter().enumerate() {
            diagnostics
                .extend(p.parse.diagnostics.iter().map(|d| refile(d.clone(), p.file, FileId(i as u32))));
        }
        // A line with a syntax error has no reliable meaning, so name and
        // type diagnostics on it would only repeat the syntax error.
        let broken_lines: Vec<(FileId, u32)> =
            diagnostics.iter().map(|d| (d.span.file, map.line_col(d.span).0.line)).collect();
        let on_broken_line = |d: &Diagnostic| {
            (d.span.file.0 as usize) < files.len()
                && broken_lines.contains(&(d.span.file, map.line_col(d.span).0.line))
        };
        // The manifests' and the module trees' problems, and a dependency
        // that `[trust]` does not let use `unsafe` or `extern` (09).
        for d in &program.problems {
            let mut d = d.clone();
            for k in 0..program.manifests.len() as u32 {
                d = refile(d, FileId(k), FileId(manifest_base + k + 1000));
            }
            for k in 0..program.manifests.len() as u32 {
                d = refile(d, FileId(manifest_base + k + 1000), FileId(manifest_base + k));
            }
            diagnostics.push(d);
        }
        for (i, ((_, _, package), p)) in program.modules.iter().zip(&files).enumerate() {
            let Some(pkg) = program.packages.get(*package) else { continue };
            if pkg.trusted || *package == 0 {
                continue;
            }
            for t in p.parse.tree.tokens() {
                let what = match t.kind {
                    sisu_syntax::SyntaxKind::UNSAFE_KW => "`unsafe`",
                    sisu_syntax::SyntaxKind::EXTERN_KW => "`extern`",
                    _ => continue,
                };
                diagnostics.push(
                    Diagnostic::new(
                        sisu_diag::msg::CAP_UNTRUSTED,
                        sisu_diag::Span::new(FileId(i as u32), t.range),
                    )
                    .with("package", pkg.name.clone())
                    .with("what", what),
                );
            }
        }
        diagnostics.extend(r.diagnostics.into_iter().filter(|d| !on_broken_line(d)));
        diagnostics.extend(type_diags.into_iter().filter(|d| !on_broken_line(d)));
        // The generators and contract tests the compiler writes repeat the
        // program's own contracts and types: when the program has an error,
        // one in them is that error again.
        let generated = |d: &Diagnostic| map.name(d.span.file).starts_with("<arbitrary>/");
        if diagnostics.iter().any(|d| d.is_error() && !generated(d)) {
            diagnostics.retain(|d| !generated(d));
        }
        sisu_diag::diagnostic::sort_unique(&mut diagnostics);
        let c = Arc::new(Checked { map, file: parsed.file, program: r.program, typed, diagnostics, iface });
        self.checked.insert(name.to_string(), (h, c.clone()));
        Some(c)
    }

    /// The lowered and ownership-checked program, for a program whose
    /// front end found no error, or only ownership errors about
    /// declarations.
    pub fn lowered(&mut self, name: &str) -> Option<Arc<Lowered>> {
        let (_, h) = self.key(name)?;
        let h = h ^ (self.contracts as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        if let Some((k, s)) = self.lowered.get(name)
            && *k == h
        {
            return Some(s.clone());
        }
        let c = self.checked(name)?;
        if c.blocks_lowering() {
            return None;
        }
        self.counters.lower += 1;
        // The standard library is checked once, not for every program
        // (`sisu test --std` checks it with the program's doc examples).
        let trust_std = !self.program(name).std_doc_tests;
        let memos = self.memos.entry(name.to_string()).or_default();
        let lowered =
            sisu_lower::lower_cached(&c.program, &c.typed, &c.map, self.contracts, &mut memos.lower);
        let options = sisu_borrowck::Options { trust_std, ..Default::default() };
        let mut ctx = DefaultHasher::new();
        (c.iface, self.contracts, trust_std).hash(&mut ctx);
        let mut checked =
            sisu_borrowck::check_with(&lowered, &options, Some((&mut memos.borrow, ctx.finish())));
        // What runs takes a step per jump: the jumps that only lead to
        // another are threaded once the checker is done with them.
        sisu_sir::simplify::program(&mut checked.program);
        self.counters.bodies_lowered = memos.lower.lowered;
        self.counters.lowered_reused = memos.lower.reused;
        self.counters.functions_checked = memos.borrow.checked;
        self.counters.functions_reused = memos.borrow.reused;
        let files = source_files(&c.map, &checked.program);
        let by_finding: Vec<Diagnostic> = checked
            .findings
            .iter()
            .map(|f| consume_note(sisu_borrowck::to_diagnostic(f, &c.map, &files), &c.program))
            .collect();
        // The compiler runs the `comptime` expressions of a program the
        // ownership checker accepts (08).
        let comptime = if checked.findings.is_empty() {
            let errors = sisu_interp::comptime::evaluate(&mut checked.program);
            errors.iter().map(|e| comptime_diagnostic(e, &c.map, &files, &checked.program)).collect()
        } else {
            Vec::new()
        };
        let mut diagnostics = by_finding.clone();
        diagnostics.extend(comptime.iter().cloned());
        sisu_diag::diagnostic::sort_unique(&mut diagnostics);
        let l = Arc::new(Lowered {
            sir: checked.program,
            findings: checked.findings,
            diagnostics_by_finding: by_finding,
            diagnostics,
            comptime,
        });
        self.lowered.insert(name.to_string(), (h, l.clone()));
        Some(l)
    }

    /// The SIR of a program the ownership checker accepted, keeping the
    /// functions `extra` names as well (the ones a transcript test's run
    /// time calls, 17); not kept, since only `sisu test` asks.
    pub fn sir_with(&mut self, name: &str, extra: &[&str]) -> Option<sir::Program> {
        let c = self.checked(name)?;
        if c.blocks_lowering() {
            return None;
        }
        let lowered = sisu_lower::lower_for(&c.program, &c.typed, &c.map, self.contracts, extra);
        let mut checked = sisu_borrowck::check(&lowered, &sisu_borrowck::Options::default());
        sisu_sir::simplify::program(&mut checked.program);
        if !checked.findings.is_empty() || !sisu_interp::comptime::evaluate(&mut checked.program).is_empty() {
            return None;
        }
        Some(checked.program)
    }

    /// The SIR of a program without front-end errors, drops placed.
    pub fn sir(&mut self, name: &str) -> Option<Arc<sir::Program>> {
        self.lowered(name).map(|l| Arc::new(l.sir.clone()))
    }

    /// Every diagnostic of a file: the front end's, then, when it found no
    /// error, the ownership checker's, in source order.
    pub fn diagnostics(&mut self, name: &str) -> Vec<Diagnostic> {
        let Some(c) = self.checked(name) else { return Vec::new() };
        let mut all = c.diagnostics.clone();
        if !c.has_errors()
            && let Some(l) = self.lowered(name)
        {
            all.extend(l.diagnostics.iter().cloned());
            sisu_diag::diagnostic::sort_unique(&mut all);
        }
        all
    }
}

/// A must-consume value dropped: the note names the methods of its type
/// that consume it (those taking `own self`), which SIR does not keep
/// unless they are called.
fn consume_note(d: Diagnostic, prog: &sisu_resolve::Program) -> Diagnostic {
    use sisu_diag::msg;
    if !matches!(d.msg, m if m == msg::BORROW_MUST_CONSUME || m == msg::BORROW_MUST_CONSUME_EXIT)
        || !d.notes.is_empty()
    {
        return d;
    }
    let Some(ty) = d.param("ty").map(str::to_string) else { return d };
    let base = ty.split('[').next().unwrap_or(&ty).to_string();
    let Some(def) = (0..prog.defs.len() as u32).map(sisu_resolve::DefId).find(|x| {
        prog.def(*x).name == base
            && matches!(prog.def(*x).kind, sisu_resolve::DefKind::Struct(_) | sisu_resolve::DefKind::Enum(_))
    }) else {
        return d;
    };
    let mut names: Vec<String> = prog
        .methods
        .iter()
        .filter(|((k, _), m)| {
            *k == sisu_resolve::TyKey::Adt(def)
                && prog.fn_info(**m).is_some_and(|f| f.self_mode == Some(sisu_syntax::ast::Mode::Own))
        })
        .map(|((_, n), _)| format!("`{n}`"))
        .collect();
    names.sort();
    if names.is_empty() {
        return d;
    }
    d.note(msg::BORROW_CONSUME_WITH, vec![("methods", names.join(", "))])
}

/// A failed `comptime` expression as a diagnostic at the expression.
fn comptime_diagnostic(
    e: &sisu_interp::comptime::Error,
    map: &SourceMap,
    files: &[FileId],
    p: &sir::Program,
) -> Diagnostic {
    use sisu_diag::msg;
    use sisu_interp::comptime::Failure;
    let f = p.function(&e.function);
    let file = f.and_then(|f| files.get(f.source as usize)).copied().unwrap_or_default();
    let span = sisu_borrowck::span_of(map, file, f.and_then(|f| f.loc));
    // The item that holds the expression: `main` for `main#comptime0`.
    let item = e.function.split("#comptime").next().unwrap_or(&e.function).to_string();
    match &e.failure {
        Failure::Panicked(m) => Diagnostic::new(msg::COMPTIME_PANICKED, span).with("message", m.clone()),
        Failure::Refinement { ty, value, predicate } => Diagnostic::new(msg::REFINE_NOT_SATISFIED, span)
            .with("value", value.clone())
            .with("ty", ty.clone())
            .with("predicate", predicate.clone()),
        Failure::Budget(n) => {
            Diagnostic::new(msg::COMPTIME_BUDGET, span).with("item", item).with("steps", n.to_string())
        }
        Failure::World(what) => Diagnostic::new(msg::COMPTIME_IO, span).with("what", *what),
        Failure::NotKept(what) => Diagnostic::new(msg::COMPTIME_NOT_KEPT, span).with("what", what.clone()),
        Failure::Internal(m) => Diagnostic::new(msg::STAGE0_INTERNAL, span)
            .with("stage", "compile-time evaluation")
            .with("detail", m.clone()),
    }
}

/// The file of each SIR source, by name.
pub fn source_files(map: &SourceMap, p: &sir::Program) -> Vec<FileId> {
    p.sources
        .iter()
        .map(|n| map.files().iter().find(|f| f.name == *n).map(|f| f.id).unwrap_or_default())
        .collect()
}
