//! What a program is made of (09, "Modules" and "Package manifest"): a
//! single file, a script with its inline metadata, a package with its
//! dependencies, or a workspace's members. [`load`] reads the manifests and
//! walks the module trees, and gives a [`Layout`]: every module's path,
//! file and package, every package's dependencies and trust, the problems
//! found, and the lockfile the manifests imply.
//!
//! The module tree mirrors the file tree under `src/`: the root module is
//! `src/main.sisu` (an application) or `src/lib.sisu` (a library), named
//! by the package's name; `src/parser.sisu` is `PACKAGE.parser`; a
//! directory `src/net/` is the module `PACKAGE.net`, whose children are the
//! files and directories in it, and a file `src/net.sisu` beside it gives
//! that module items of its own.

use crate::manifest::{self, Dependency, Manifest};
use sisu_diag::{Diagnostic, FileId, Span, TextRange, msg};
use std::path::{Path, PathBuf};

/// A package of the program.
#[derive(Clone, Debug, Default)]
pub struct Package {
    /// Its `[package] name`; empty for a single file or a script.
    pub name: String,
    pub version: String,
    /// Its directory.
    pub dir: PathBuf,
    /// The packages its imports may name: its active dependencies, by
    /// name, as indices of [`Layout::packages`].
    pub deps: Vec<(String, usize)>,
    /// Whether it may contain `unsafe` and `extern`: the root package, and
    /// the dependencies the root package's `[trust] unsafe` lists.
    pub trusted: bool,
    /// Its root module is `src/main.sisu`, or it is a single file: it has a
    /// `main` to run.
    pub application: bool,
    /// Its manifest, as [`Layout::files`] numbers them.
    pub manifest: Option<usize>,
    /// What the lockfile records of its dependencies: every dependency,
    /// active on this target or not, so the lockfile is the same on every
    /// target.
    pub locked_deps: Vec<String>,
    /// Its manifest's `[profile.release] contracts`: what a release build
    /// checks (07).
    pub release_contracts: Option<String>,
    /// Its manifest's `[features]` (09).
    pub features: Vec<manifest::Feature>,
    /// Its features that are on in this build, sorted (decision 243).
    pub on: Vec<String>,
    /// Its manifest's dependency entries, as far as features go.
    pub wants: Vec<Want>,
}

/// What a dependency entry asks of the package it names (09).
#[derive(Clone, Debug, Default)]
pub struct Want {
    /// The name the entry gives the dependency.
    pub name: String,
    /// The package, unless it could not be read or is not built (a
    /// dependency's dev-dependency).
    pub package: Option<usize>,
    /// Its `when` holds on this target.
    pub active: bool,
    /// `features = [...]`.
    pub features: Vec<(String, TextRange)>,
    /// Not `default-features = false`.
    pub defaults: bool,
}

impl Want {
    fn of(dep: &Dependency, package: Option<usize>, active: bool) -> Want {
        Want {
            name: dep.name.clone(),
            package,
            active,
            features: dep.features.clone(),
            defaults: dep.default_features,
        }
    }
}

/// A module of the program: its path, its file and its package.
#[derive(Clone, Debug)]
pub struct ModuleFile {
    /// `["weather", "parser"]`; empty for a single file.
    pub path: Vec<String>,
    /// The file as diagnostics name it: relative to the root package's
    /// directory, with `/` separators; a single file as it was given.
    pub name: String,
    pub disk: PathBuf,
    /// An index of [`Layout::packages`].
    pub package: usize,
}

/// A program's layout.
#[derive(Clone, Debug, Default)]
pub struct Layout {
    /// The packages, the root package first.
    pub packages: Vec<Package>,
    /// The modules, the root module first.
    pub modules: Vec<ModuleFile>,
    /// The manifests read (the name diagnostics show, the text): the
    /// files [`Layout::diagnostics`] point into, numbered as `FileId`s
    /// from 0.
    pub files: Vec<(String, String)>,
    /// The problems found in the manifests and the module trees.
    pub diagnostics: Vec<Diagnostic>,
    /// Where the lockfile goes and what it holds; none for a single file.
    pub lock: Option<(PathBuf, String)>,
    /// The directory the files' names are relative to.
    pub base: PathBuf,
    /// The root package is a script: a single file with a `#!` line.
    pub script: bool,
}

impl Layout {
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.is_error())
    }

    /// The root package.
    pub fn root(&self) -> Option<&Package> {
        self.packages.first()
    }
}

/// What a command works on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// One file, as given.
    File(String),
    /// A package's directory.
    Package(PathBuf),
    /// A workspace's directory and its members' directories.
    Workspace(PathBuf, Vec<PathBuf>),
}

/// The manifest file of a directory.
pub const MANIFEST: &str = "sisu.toml";
/// The lockfile beside the root manifest (09).
pub const LOCKFILE: &str = "sisu.lock";

/// What the command's argument names: a `.sisu` file, or a directory with a
/// manifest; without an argument, the package or workspace whose manifest
/// is in `cwd` or the nearest directory above it. `None` when there is
/// none.
pub fn locate(arg: Option<&str>, cwd: &Path) -> Option<Target> {
    let start = match arg {
        Some(a) => {
            let p = cwd.join(a);
            if p.is_file() {
                return Some(Target::File(a.to_string()));
            }
            if !p.is_dir() {
                return Some(Target::File(a.to_string()));
            }
            p
        }
        None => cwd.to_path_buf(),
    };
    let mut dir = Some(start.as_path());
    while let Some(d) = dir {
        let m = d.join(MANIFEST);
        if m.is_file() {
            let text = std::fs::read_to_string(&m).unwrap_or_default();
            let (parsed, _) = manifest::read(FileId(0), &text);
            if let Some(Manifest { package: None, members: Some(members), .. }) = parsed {
                let ms = members.into_iter().map(|(p, _)| d.join(p)).collect();
                return Some(Target::Workspace(d.to_path_buf(), ms));
            }
            return Some(Target::Package(d.to_path_buf()));
        }
        // An argument names the directory itself, not one above it.
        if arg.is_some() {
            return None;
        }
        dir = d.parent();
    }
    None
}

/// `to` relative to the directory `from`, with `/` separators.
pub fn relative(from: &Path, to: &Path) -> String {
    let a: Vec<_> = from.components().collect();
    let b: Vec<_> = to.components().collect();
    let common = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let mut parts: Vec<String> = Vec::new();
    for _ in common..a.len() {
        parts.push("..".into());
    }
    for c in &b[common..] {
        parts.push(c.as_os_str().to_string_lossy().into_owned());
    }
    if parts.is_empty() { ".".into() } else { parts.join("/") }
}

fn canonical(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

struct Loader {
    layout: Layout,
    target: Vec<(&'static str, &'static str)>,
    /// Each package's canonical directory, by index.
    dirs: Vec<PathBuf>,
    /// For each package on the loader's stack, the dependency entry that
    /// led to it (none for the first), to label a cycle's edges.
    edges: Vec<Option<Span>>,
}

impl Loader {
    fn add_file(&mut self, name: String, text: String) -> usize {
        self.layout.files.push((name, text));
        self.layout.files.len() - 1
    }

    fn report(&mut self, d: Diagnostic) {
        self.layout.diagnostics.push(d);
    }

    fn shown(&self, p: &Path) -> String {
        relative(&self.layout.base, &canonical(p))
    }

    /// Reads the package in `dir` (once), its modules and, depth first,
    /// its dependencies. `stack` is the path of packages to it, for a
    /// cycle's report; `via` the dependency that names it.
    fn package(
        &mut self,
        dir: &Path,
        stack: &mut Vec<usize>,
        via: Option<(usize, &Dependency)>,
    ) -> Option<usize> {
        let canon = canonical(dir);
        if let Some(i) = self.dirs.iter().position(|d| *d == canon) {
            if let Some(pos) = stack.iter().position(|s| *s == i) {
                let mut names: Vec<String> =
                    stack[pos..].iter().map(|&k| self.layout.packages[k].name.clone()).collect();
                names.push(self.layout.packages[i].name.clone());
                let cycle = names.iter().map(|n| format!("`{n}`")).collect::<Vec<_>>().join(" -> ");
                let span = match via {
                    Some((from, dep)) => self.dep_span(from, dep),
                    None => Span::new(FileId(0), TextRange::empty(0)),
                };
                let mut d = Diagnostic::new(msg::PACKAGE_CYCLE, span).with("cycle", cycle);
                // The other edges of the cycle, each where its manifest
                // names the next package.
                for k in pos + 1..stack.len() {
                    if let Some(Some(at)) = self.edges.get(k) {
                        let name = self.layout.packages[stack[k - 1]].name.clone();
                        let dep = self.layout.packages[stack[k]].name.clone();
                        d = d.label(*at, msg::PACKAGE_DEPENDS_HERE, vec![("name", name), ("dep", dep)]);
                    }
                }
                self.report(d);
                return None;
            }
            return Some(i);
        }
        let manifest_path = dir.join(MANIFEST);
        let Ok(text) = std::fs::read_to_string(&manifest_path) else {
            if let Some((from, dep)) = via {
                let span = Span::new(self.manifest_file(from), dep.path_range);
                let path = self.shown(dir);
                self.report(
                    Diagnostic::new(msg::PACKAGE_NO_MANIFEST, span)
                        .with("name", dep.name.clone())
                        .with("path", path),
                );
            }
            return None;
        };
        let shown = self.shown(&manifest_path);
        let file = self.add_file(shown, text.clone());
        let (m, diags) = manifest::read(FileId(file as u32), &text);
        self.layout.diagnostics.extend(diags);
        let m = m?;
        let Some(info) = m.package.clone() else {
            if let Some((from, dep)) = via {
                let span = self.dep_span(from, dep);
                let path = self.shown(dir);
                self.report(Diagnostic::new(msg::PACKAGE_NO_PACKAGE, span).with("path", path));
            }
            return None;
        };
        if let Some((from, dep)) = via
            && dep.name != info.name
        {
            let span = self.dep_span(from, dep);
            self.report(
                Diagnostic::new(msg::PACKAGE_RENAMED_DEPENDENCY, span)
                    .with("name", dep.name.clone())
                    .with("found", info.name.clone()),
            );
        }
        let index = self.layout.packages.len();
        self.layout.packages.push(Package {
            name: info.name.clone(),
            version: info.version.clone(),
            dir: canon.clone(),
            deps: Vec::new(),
            trusted: index == 0,
            application: false,
            manifest: Some(file),
            locked_deps: m.dependencies.iter().filter(|d| !d.dev).map(|d| d.name.clone()).collect(),
            release_contracts: m.release_contracts.clone(),
            features: m.features.clone(),
            on: Vec::new(),
            wants: Vec::new(),
        });
        self.dirs.push(canon.clone());
        self.modules_of(index, &canon, &info.name, file);
        stack.push(index);
        let edge = via.map(|(from, dep)| self.dep_span(from, dep));
        self.edges.push(edge);
        for dep in &m.dependencies {
            // A dev-dependency is the package's own tests' business: only
            // the root package's are built.
            if dep.dev && index != 0 {
                self.layout.packages[index].wants.push(Want::of(dep, None, false));
                continue;
            }
            let active = dep.when.as_ref().is_none_or(|w| w.holds(&self.target));
            let d = self.package(&canon.join(&dep.path), stack, Some((index, dep)));
            self.layout.packages[index].wants.push(Want::of(dep, d, active));
            let Some(d) = d else { continue };
            if active {
                self.layout.packages[index].deps.push((dep.name.clone(), d));
            }
            if index == 0 && m.trust_unsafe.iter().any(|(t, _)| *t == dep.name) {
                self.layout.packages[d].trusted = true;
            }
        }
        stack.pop();
        self.edges.pop();
        Some(index)
    }

    fn manifest_file(&self, package: usize) -> FileId {
        FileId(self.layout.packages.get(package).and_then(|p| p.manifest).unwrap_or(0) as u32)
    }

    fn dep_span(&self, from: usize, dep: &Dependency) -> Span {
        Span::new(self.manifest_file(from), dep.name_range)
    }

    /// The modules under `dir/src`, the root module first, then the others
    /// by path.
    fn modules_of(&mut self, package: usize, dir: &Path, name: &str, manifest: usize) {
        let src = dir.join("src");
        let main = src.join("main.sisu");
        let lib = src.join("lib.sisu");
        let span = Span::new(FileId(manifest as u32), TextRange::empty(0));
        let root = match (main.is_file(), lib.is_file()) {
            (true, true) => {
                self.report(
                    Diagnostic::new(msg::PACKAGE_TWO_ROOT_MODULES, span).with("name", name.to_string()),
                );
                main
            }
            (true, false) => main,
            (false, true) => lib,
            (false, false) => {
                self.report(
                    Diagnostic::new(msg::PACKAGE_NO_ROOT_MODULE, span).with("name", name.to_string()),
                );
                return;
            }
        };
        self.layout.packages[package].application = root.ends_with("main.sisu");
        let shown = self.shown(&root);
        self.layout.modules.push(ModuleFile {
            path: vec![name.to_string()],
            name: shown,
            disk: root.clone(),
            package,
        });
        let mut others = Vec::new();
        self.walk(&src, &[name.to_string()], package, manifest, true, &mut others);
        others.sort_by(|a: &ModuleFile, b| a.path.cmp(&b.path));
        self.layout.modules.extend(others);
    }

    fn walk(
        &mut self,
        dir: &Path,
        prefix: &[String],
        package: usize,
        manifest: usize,
        top: bool,
        out: &mut Vec<ModuleFile>,
    ) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        let mut paths: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
        paths.sort();
        for p in paths {
            let Some(file_name) = p.file_name().map(|n| n.to_string_lossy().into_owned()) else { continue };
            if file_name.starts_with('.') {
                continue;
            }
            let (stem, is_dir) = if p.is_dir() {
                (file_name.clone(), true)
            } else if let Some(stem) = file_name.strip_suffix(".sisu") {
                if top && (stem == "main" || stem == "lib") {
                    continue;
                }
                (stem.to_string(), false)
            } else {
                continue;
            };
            if !manifest::is_module_name(&stem) {
                let span = Span::new(FileId(manifest as u32), TextRange::empty(0));
                let shown = self.shown(&p);
                self.report(
                    Diagnostic::new(msg::PACKAGE_MODULE_NAME, span).with("file", shown).with("name", stem),
                );
                continue;
            }
            let mut path = prefix.to_vec();
            path.push(stem);
            if is_dir {
                self.walk(&p, &path, package, manifest, false, out);
            } else {
                let shown = self.shown(&p);
                out.push(ModuleFile { path, name: shown, disk: p, package });
            }
        }
    }
}

/// The layout of a package or workspace in `dir` (for a workspace, of the
/// member `member`), with file names relative to `base`.
pub fn load_package(dir: &Path, base: &Path) -> Layout {
    let mut l = Loader {
        layout: Layout { base: canonical(base), ..Layout::default() },
        target: manifest::target_facts(),
        dirs: Vec::new(),
        edges: Vec::new(),
    };
    let mut stack = Vec::new();
    l.package(dir, &mut stack, None);
    let mut layout = l.layout;
    turn_on_features(&mut layout);
    // The lockfile: beside the workspace's manifest when the package is a
    // member of one, else beside its own.
    let canon = canonical(dir);
    let lock_dir = workspace_of(&canon).map(|(d, _)| d).unwrap_or(canon);
    layout.lock = Some((lock_dir.join(LOCKFILE), lockfile(&layout, &lock_dir)));
    layout
}

/// The workspace that lists `dir` as a member: the nearest directory above
/// it whose manifest has `[workspace] members` naming it, with its members'
/// directories.
pub fn workspace_of(dir: &Path) -> Option<(PathBuf, Vec<PathBuf>)> {
    let dir = canonical(dir);
    let mut up = dir.parent();
    while let Some(d) = up {
        let m = d.join(MANIFEST);
        if let Ok(text) = std::fs::read_to_string(&m)
            && let (Some(Manifest { members: Some(members), .. }), _) = manifest::read(FileId(0), &text)
        {
            let dirs: Vec<PathBuf> = members.iter().map(|(p, _)| canonical(&d.join(p))).collect();
            if dirs.contains(&dir) {
                return Some((d.to_path_buf(), dirs));
            }
        }
        up = d.parent();
    }
    None
}

/// The layout of a single file: one module, in a package of its own with
/// no name. A script (`#!` on its first line) may declare dependencies in
/// its inline metadata, by paths relative to its directory.
pub fn load_file(name: &str, text: &str, cwd: &Path) -> Layout {
    let disk = cwd.join(name);
    let mut layout = Layout { base: cwd.to_path_buf(), ..Layout::default() };
    layout.script = text.starts_with("#!");
    layout.packages.push(Package {
        name: String::new(),
        dir: disk.parent().map(Path::to_path_buf).unwrap_or_default(),
        trusted: true,
        application: true,
        ..Package::default()
    });
    layout.modules.push(ModuleFile {
        path: Vec::new(),
        name: name.to_string(),
        disk: disk.clone(),
        package: 0,
    });
    if !layout.script {
        return layout;
    }
    let meta = match manifest::script_metadata(text) {
        Ok(Some(m)) => m,
        Ok(None) => return layout,
        Err(range) => {
            layout.files.push((name.to_string(), text.to_string()));
            layout
                .diagnostics
                .push(Diagnostic::new(msg::PACKAGE_SCRIPT_METADATA, Span::new(FileId(0), range)));
            return layout;
        }
    };
    // The script is the first package on the stack, reached by no edge.
    let mut l =
        Loader { layout, target: manifest::target_facts(), dirs: vec![PathBuf::new()], edges: vec![None] };
    l.layout.files.push((name.to_string(), meta.clone()));
    let (m, diags) = manifest::read(FileId(0), &meta);
    l.layout.diagnostics.extend(diags);
    l.layout.packages[0].manifest = Some(0);
    let Some(m) = m else { return l.layout };
    l.layout.packages[0].features = m.features.clone();
    let dir = l.layout.packages[0].dir.clone();
    let mut stack = vec![0];
    for dep in &m.dependencies {
        let active = dep.when.as_ref().is_none_or(|w| w.holds(&l.target));
        let d = l.package(&dir.join(&dep.path), &mut stack, Some((0, dep)));
        l.layout.packages[0].wants.push(Want::of(dep, d, active));
        let Some(d) = d else { continue };
        if active {
            l.layout.packages[0].deps.push((dep.name.clone(), d));
        }
        if m.trust_unsafe.iter().any(|(t, _)| *t == dep.name) {
            l.layout.packages[d].trusted = true;
        }
    }
    let mut layout = l.layout;
    turn_on_features(&mut layout);
    layout
}

/// The features each package of the build turns on (09; decision 243):
/// the root package's `default` feature, each dependency's `default`
/// unless every entry that reaches it says `default-features = false`,
/// the features those entries list, and what each feature that is on
/// turns on in turn, of its own package or (`DEP/FEATURE`) of a
/// dependency. A name that is no feature is `E1018`.
fn turn_on_features(layout: &mut Layout) {
    let n = layout.packages.len();
    let has =
        |p: usize, f: &str| layout.packages.get(p).is_some_and(|k| k.features.iter().any(|x| x.name == f));
    let shown = |p: usize| match layout.packages.get(p) {
        Some(k) if !k.name.is_empty() => k.name.clone(),
        _ => layout.modules.first().map(|m| m.name.clone()).unwrap_or_default(),
    };
    let mut problems = Vec::new();
    for (p, pkg) in layout.packages.iter().enumerate() {
        let file = FileId(pkg.manifest.unwrap_or(0) as u32);
        let check = |at: TextRange, of: usize, name: &str, problems: &mut Vec<Diagnostic>| {
            if !has(of, name) {
                problems.push(
                    Diagnostic::new(msg::PACKAGE_UNKNOWN_FEATURE, Span::new(file, at))
                        .with("name", name.to_string())
                        .with("package", shown(of)),
                );
            }
        };
        for f in &pkg.features {
            for (e, at) in &f.on {
                match e.split_once('/') {
                    Some((dep, x)) => match pkg.wants.iter().find(|w| w.name == dep) {
                        Some(w) => {
                            if let Some(d) = w.package {
                                check(*at, d, x, &mut problems);
                            }
                        }
                        None => problems.push(
                            Diagnostic::new(msg::PACKAGE_UNKNOWN_FEATURE, Span::new(file, *at))
                                .with("name", e.clone())
                                .with("package", shown(p)),
                        ),
                    },
                    None => check(*at, p, e, &mut problems),
                }
            }
        }
        for w in &pkg.wants {
            if let Some(d) = w.package {
                for (x, at) in &w.features {
                    check(*at, d, x, &mut problems);
                }
            }
        }
    }
    layout.diagnostics.extend(problems);
    // The packages the build reaches, and what reaching them turns on.
    let mut reached = vec![false; n];
    let mut stack = vec![0];
    let mut work: Vec<(usize, String)> = vec![(0, "default".to_string())];
    while let Some(p) = stack.pop() {
        if p >= n || reached[p] {
            continue;
        }
        reached[p] = true;
        for w in layout.packages[p].wants.iter().filter(|w| w.active) {
            let Some(d) = w.package else { continue };
            stack.push(d);
            if w.defaults {
                work.push((d, "default".to_string()));
            }
            work.extend(w.features.iter().map(|(x, _)| (d, x.clone())));
        }
    }
    let mut on: Vec<Vec<String>> = vec![Vec::new(); n];
    while let Some((p, f)) = work.pop() {
        let Some(pkg) = layout.packages.get(p) else { continue };
        let Some(feature) = pkg.features.iter().find(|x| x.name == f) else { continue };
        if on[p].contains(&f) {
            continue;
        }
        for (e, _) in &feature.on {
            match e.split_once('/') {
                Some((dep, x)) => {
                    if let Some(d) =
                        pkg.wants.iter().find(|w| w.name == dep && w.active).and_then(|w| w.package)
                    {
                        work.push((d, x.to_string()));
                    }
                }
                None => work.push((p, e.clone())),
            }
        }
        on[p].push(f);
    }
    for (pkg, mut fs) in layout.packages.iter_mut().zip(on) {
        fs.sort();
        pkg.on = fs;
    }
}

/// `sisu.lock` (09): the packages the build resolved, as data. In stage 0
/// every dependency is a path, so the lock records each package's name,
/// version, source and dependencies, sorted, and is the same on every
/// target.
pub fn lockfile(layout: &Layout, dir: &Path) -> String {
    lockfile_of(std::slice::from_ref(layout), dir)
}

/// The lockfile of a workspace: every package any member's build resolved,
/// once.
pub fn lockfile_of(layouts: &[Layout], dir: &Path) -> String {
    let mut out = String::from(
        "# Written by `sisu` from the manifests: the packages this build resolved.\n# It is data; `sisu` rewrites it when a manifest changes.\nversion = 1\n",
    );
    let mut entries: Vec<(String, String, String, Vec<String>)> = layouts
        .iter()
        .flat_map(|l| l.packages.iter())
        .map(|p| {
            let source = format!("path+{}", relative(dir, &p.dir));
            let mut deps = p.locked_deps.clone();
            deps.sort();
            (p.name.clone(), p.version.clone(), source, deps)
        })
        .collect();
    entries.sort();
    entries.dedup();
    for (name, version, source, deps) in entries {
        let deps: Vec<String> = deps.iter().map(|d| crate::toml::quote(d)).collect();
        out.push_str(&format!(
            "\n[[package]]\nname = {}\nversion = {}\nsource = {}\ndependencies = [{}]\n",
            crate::toml::quote(&name),
            crate::toml::quote(&version),
            crate::toml::quote(&source),
            deps.join(", ")
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("sisu-package-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let _ = std::fs::create_dir_all(&d);
        d
    }

    fn write(p: &Path, text: &str) {
        if let Some(d) = p.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let _ = std::fs::write(p, text);
    }

    #[test]
    fn walks_the_module_tree() {
        let d = temp("tree");
        write(
            &d.join("app/sisu.toml"),
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\nutil = { path = \"../util\" }\n",
        );
        write(&d.join("app/src/main.sisu"), "def main():\n    pass\n");
        write(&d.join("app/src/parser.sisu"), "");
        write(&d.join("app/src/net/tcp.sisu"), "");
        write(&d.join("app/src/Bad.sisu"), "");
        write(&d.join("util/sisu.toml"), "[package]\nname = \"util\"\nversion = \"0.2.0\"\n");
        write(&d.join("util/src/lib.sisu"), "");
        let l = load_package(&d.join("app"), &d.join("app"));
        let paths: Vec<String> = l.modules.iter().map(|m| m.path.join(".")).collect();
        assert_eq!(paths, ["app", "app.net.tcp", "app.parser", "util"]);
        let names: Vec<&str> = l.modules.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, ["src/main.sisu", "src/net/tcp.sisu", "src/parser.sisu", "../util/src/lib.sisu"]);
        let ids: Vec<&str> = l.diagnostics.iter().map(|d| d.msg.id()).collect();
        assert_eq!(ids, ["package.module-name"]);
        assert_eq!(l.packages[0].deps, vec![("util".to_string(), 1)]);
        assert!(l.packages[0].application && !l.packages[1].application);
        let (path, lock) = l.lock.unwrap();
        assert!(path.ends_with("app/sisu.lock"));
        assert!(lock.contains("name = \"util\"\nversion = \"0.2.0\"\nsource = \"path+../util\""), "{lock}");
    }

    #[test]
    fn reports_cycles_and_renames() {
        let d = temp("cycle");
        write(
            &d.join("a/sisu.toml"),
            "[package]\nname = \"a\"\nversion = \"0.1.0\"\n[dependencies]\nb = { path = \"../b\" }\n",
        );
        write(&d.join("a/src/main.sisu"), "");
        write(
            &d.join("b/sisu.toml"),
            "[package]\nname = \"bee\"\nversion = \"0.1.0\"\n[dependencies]\na = { path = \"../a\" }\n",
        );
        write(&d.join("b/src/lib.sisu"), "");
        let l = load_package(&d.join("a"), &d.join("a"));
        let ids: Vec<&str> = l.diagnostics.iter().map(|d| d.msg.id()).collect();
        assert!(ids.contains(&"package.renamed-dependency"), "{ids:?}");
        assert!(ids.contains(&"package.cycle"), "{ids:?}");
    }

    #[test]
    fn relative_paths() {
        assert_eq!(relative(Path::new("/a/b"), Path::new("/a/b/src/x.sisu")), "src/x.sisu");
        assert_eq!(relative(Path::new("/a/b"), Path::new("/a/c/src")), "../c/src");
        assert_eq!(relative(Path::new("/a/b"), Path::new("/a/b")), ".");
    }
}
