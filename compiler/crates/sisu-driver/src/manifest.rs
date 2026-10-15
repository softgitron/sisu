//! `sisu.toml`, the package manifest (09, "Package manifest"), as data:
//! its tables read into a [`Manifest`], every mistake a diagnostic with its
//! position in the file, so the language server can show it where it is.
//!
//! The manifest stays data (D-063): no conditionals or expressions. A
//! dependency that applies only on some targets says so with a structured
//! condition over the build target's facts, `when = { os = "windows" }`,
//! which is data too: the build compares it with `target`'s constants.
//! Stage 0 has no registry, so every dependency is a `path`.

use crate::toml::{self, Spanned, Table, Value};
use sisu_diag::{Diagnostic, FileId, Span, TextRange, msg};

/// The build target's facts a `when` condition may name: `target.os`,
/// `target.arch` and `target.runtime` (08, "Conditional compilation").
pub const TARGET_FACTS: &[&str] = &["os", "arch", "runtime"];

/// What a dependency's `when` asks of the target: for each fact named,
/// the values that admit it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct When {
    pub facts: Vec<(String, Vec<String>)>,
}

impl When {
    /// Whether the target with these facts meets the condition.
    pub fn holds(&self, target: &[(&str, &str)]) -> bool {
        self.facts.iter().all(|(fact, values)| {
            target.iter().find(|(f, _)| f == fact).is_some_and(|(_, v)| values.iter().any(|x| x == v))
        })
    }

    /// The condition as the manifest writes it, for the lockfile.
    pub fn text(&self) -> String {
        let parts: Vec<String> = self
            .facts
            .iter()
            .map(|(f, vs)| match vs.as_slice() {
                [one] => format!("{f} = {}", toml::quote(one)),
                _ => format!("{f} = [{}]", vs.iter().map(|v| toml::quote(v)).collect::<Vec<_>>().join(", ")),
            })
            .collect();
        format!("{{ {} }}", parts.join(", "))
    }
}

/// The facts of the target stage 0 builds for: the host it runs on, and the
/// interpreter.
pub fn target_facts() -> Vec<(&'static str, &'static str)> {
    vec![("os", std::env::consts::OS), ("arch", std::env::consts::ARCH), ("runtime", "interpreter")]
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dependency {
    /// The name the package's imports name it by: its key.
    pub name: String,
    pub name_range: TextRange,
    /// The directory of its package, as written (relative to the manifest).
    pub path: String,
    pub path_range: TextRange,
    pub when: Option<When>,
    /// A dev-dependency: only the package's tests may import it.
    pub dev: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PackageInfo {
    pub name: String,
    pub name_range: TextRange,
    pub version: String,
    pub edition: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Manifest {
    /// `[package]`; a workspace's own manifest may have none.
    pub package: Option<PackageInfo>,
    pub dependencies: Vec<Dependency>,
    /// `[trust] unsafe`: the dependencies that may contain `unsafe` and
    /// `extern` (09).
    pub trust_unsafe: Vec<(String, TextRange)>,
    /// `[workspace] members`: the members' directories, as written.
    pub members: Option<Vec<(String, TextRange)>>,
    /// `[features]`: each feature and what it turns on. Recorded, and used
    /// by nothing yet: stage 0 has no conditional compilation on features.
    pub features: Vec<(String, Vec<String>)>,
    /// `[profile.release] contracts`.
    pub release_contracts: Option<String>,
}

/// The names stage 0's standard library answers `import NAME` with, which
/// no package of a program may take (09; the stage-0 short names of
/// std's modules).
pub const RESERVED: &[&str] =
    &["std", "cli", "math", "task", "log", "cap", "net", "time", "http", "sim", "json", "test"];

/// Whether `name` can name a package or a module: lowercase letters,
/// digits and `_`, starting with a letter, and not a keyword.
pub fn is_module_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && sisu_syntax::SyntaxKind::from_keyword(name).is_none()
}

struct Reader {
    file: FileId,
    diags: Vec<Diagnostic>,
}

impl Reader {
    fn span(&self, r: TextRange) -> Span {
        Span::new(self.file, r)
    }

    fn unknown(&mut self, key: &Spanned<String>, table: &str) {
        let d = Diagnostic::new(msg::PACKAGE_UNKNOWN_KEY, self.span(key.range))
            .with("key", key.value.clone())
            .with("table", table.to_string());
        self.diags.push(d);
    }

    /// A value of the wrong kind, or of the right kind but not one the key
    /// takes (then shown as written).
    fn bad(&mut self, key: &str, v: &Spanned<Value>, expected: &str) {
        let found = match &v.value {
            Value::Str(s)
                if expected != "a list of strings" && expected != "a table" && expected != "a string" =>
            {
                format!("`{}`", toml::quote(s))
            }
            other => other.describe().to_string(),
        };
        let d = Diagnostic::new(msg::PACKAGE_BAD_VALUE, self.span(v.range))
            .with("key", key.to_string())
            .with("expected", expected.to_string())
            .with("found", found);
        self.diags.push(d);
    }

    fn string(&mut self, key: &str, v: &Spanned<Value>) -> Option<String> {
        match &v.value {
            Value::Str(s) => Some(s.clone()),
            _ => {
                self.bad(key, v, "a string");
                None
            }
        }
    }

    fn strings(&mut self, key: &str, v: &Spanned<Value>) -> Vec<(String, TextRange)> {
        match &v.value {
            Value::Array(items) => {
                let mut out = Vec::new();
                for i in items {
                    match &i.value {
                        Value::Str(s) => out.push((s.clone(), i.range)),
                        _ => self.bad(key, i, "a list of strings"),
                    }
                }
                out
            }
            _ => {
                self.bad(key, v, "a list of strings");
                Vec::new()
            }
        }
    }

    fn table<'t>(&mut self, key: &str, v: &'t Spanned<Value>) -> Option<&'t Table> {
        match &v.value {
            Value::Table(t) => Some(t),
            _ => {
                self.bad(key, v, "a table");
                None
            }
        }
    }

    fn package(&mut self, t: &Table, header: TextRange) -> Option<PackageInfo> {
        let mut info = PackageInfo::default();
        let (mut name, mut version) = (false, false);
        for (k, v) in &t.entries {
            match k.value.as_str() {
                "name" => {
                    if let Some(s) = self.string("name", v) {
                        info.name = s;
                        info.name_range = v.range;
                        name = true;
                        self.check_name(&info.name, v.range);
                    }
                }
                "version" => {
                    if let Some(s) = self.string("version", v) {
                        let parts: Vec<&str> = s.split('.').collect();
                        if parts.len() == 3
                            && parts.iter().all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
                        {
                            info.version = s;
                            version = true;
                        } else {
                            self.bad("version", v, "a version `MAJOR.MINOR.PATCH`, as in `\"0.3.1\"`");
                        }
                    }
                }
                "edition" => {
                    if let Some(s) = self.string("edition", v) {
                        if s.len() == 4 && s.chars().all(|c| c.is_ascii_digit()) {
                            info.edition = Some(s);
                        } else {
                            self.bad("edition", v, "a year, as in `\"2027\"`");
                        }
                    }
                }
                "description" => info.description = self.string("description", v),
                "license" => info.license = self.string("license", v),
                // Recorded by `sisu publish` in a later stage.
                "authors" | "repository" | "readme" | "keywords" | "profile" => {}
                _ => self.unknown(k, "[package]"),
            }
        }
        for (missing, key) in [(!name, "name"), (!version, "version")] {
            if missing {
                let d = Diagnostic::new(msg::PACKAGE_MISSING_KEY, self.span(header))
                    .with("table", "[package]")
                    .with("key", key);
                self.diags.push(d);
            }
        }
        (name && version).then_some(info)
    }

    fn check_name(&mut self, name: &str, range: TextRange) {
        if RESERVED.contains(&name) {
            let d =
                Diagnostic::new(msg::PACKAGE_RESERVED_NAME, self.span(range)).with("name", name.to_string());
            self.diags.push(d);
        } else if !is_module_name(name) {
            let d = Diagnostic::new(msg::PACKAGE_BAD_NAME, self.span(range))
                .with("name", name.to_string())
                .with("why", "a package is named in lowercase letters, digits and `_`, starting with a letter, as a module is");
            self.diags.push(d);
        }
    }

    fn dependencies(&mut self, t: &Table, dev: bool, out: &mut Vec<Dependency>) {
        let table = if dev { "[dev-dependencies]" } else { "[dependencies]" };
        for (k, v) in &t.entries {
            self.check_name(&k.value, k.range);
            let Value::Table(spec) = &v.value else {
                let d = Diagnostic::new(msg::PACKAGE_NOT_A_PATH, self.span(v.range))
                    .with("name", k.value.clone());
                self.diags.push(d);
                continue;
            };
            let mut path = None;
            let mut when = None;
            let mut registry = false;
            for (sk, sv) in &spec.entries {
                match sk.value.as_str() {
                    "path" => path = self.string("path", sv).map(|p| (p, sv.range)),
                    "when" => when = self.when(sv),
                    "version" | "git" | "rev" | "branch" | "tag" => registry = true,
                    // Additive booleans of the dependency (09); recorded by
                    // nothing yet.
                    "features" | "default-features" | "optional" => {}
                    _ => self.unknown(sk, &format!("{table} {}", k.value)),
                }
            }
            match path {
                Some((path, path_range)) => out.push(Dependency {
                    name: k.value.clone(),
                    name_range: k.range,
                    path,
                    path_range,
                    when,
                    dev,
                }),
                None => {
                    let _ = registry;
                    let d = Diagnostic::new(msg::PACKAGE_NOT_A_PATH, self.span(v.range))
                        .with("name", k.value.clone());
                    self.diags.push(d);
                }
            }
        }
    }

    fn when(&mut self, v: &Spanned<Value>) -> Option<When> {
        let bad = |r: &mut Reader| {
            r.diags.push(Diagnostic::new(msg::PACKAGE_BAD_WHEN, r.span(v.range)));
            None
        };
        let Value::Table(t) = &v.value else { return bad(self) };
        let mut facts = Vec::new();
        for (k, x) in &t.entries {
            if !TARGET_FACTS.contains(&k.value.as_str()) {
                return bad(self);
            }
            let values = match &x.value {
                Value::Str(s) => vec![s.clone()],
                Value::Array(items) if items.iter().all(|i| matches!(i.value, Value::Str(_))) => {
                    items.iter().filter_map(|i| i.value.as_str().map(str::to_string)).collect()
                }
                _ => return bad(self),
            };
            facts.push((k.value.clone(), values));
        }
        Some(When { facts })
    }
}

/// Reads a manifest. A syntax error leaves nothing to read, so it is the
/// only diagnostic; otherwise every table is read and every mistake
/// reported.
pub fn read(file: FileId, text: &str) -> (Option<Manifest>, Vec<Diagnostic>) {
    let mut r = Reader { file, diags: Vec::new() };
    let doc = match toml::parse(text) {
        Ok(doc) => doc,
        Err(toml::Problem::Syntax(e)) => {
            let d = Diagnostic::new(msg::PACKAGE_TOML_SYNTAX, r.span(e.range))
                .with("expected", e.expected)
                .with("found", e.found);
            return (None, vec![d]);
        }
        Err(toml::Problem::Duplicate(e)) => {
            let table = if e.table.is_empty() { "sisu.toml".to_string() } else { format!("[{}]", e.table) };
            let d = Diagnostic::new(msg::PACKAGE_TOML_DUPLICATE, r.span(e.range))
                .with("key", e.key)
                .with("table", table);
            return (None, vec![d]);
        }
    };
    let mut m = Manifest::default();
    for (k, v) in &doc.entries {
        match k.value.as_str() {
            "package" => {
                if let Some(t) = r.table("package", v) {
                    m.package = r.package(t, k.range);
                }
            }
            "dependencies" | "dev-dependencies" => {
                if let Some(t) = r.table(&k.value, v) {
                    let mut deps = std::mem::take(&mut m.dependencies);
                    r.dependencies(t, k.value == "dev-dependencies", &mut deps);
                    m.dependencies = deps;
                }
            }
            "trust" => {
                if let Some(t) = r.table("trust", v) {
                    for (tk, tv) in &t.entries {
                        match tk.value.as_str() {
                            "unsafe" => m.trust_unsafe = r.strings("unsafe", tv),
                            _ => r.unknown(tk, "[trust]"),
                        }
                    }
                }
            }
            "workspace" => {
                if let Some(t) = r.table("workspace", v) {
                    let mut members = Vec::new();
                    for (wk, wv) in &t.entries {
                        match wk.value.as_str() {
                            "members" => members = r.strings("members", wv),
                            _ => r.unknown(wk, "[workspace]"),
                        }
                    }
                    m.members = Some(members);
                }
            }
            "features" => {
                if let Some(t) = r.table("features", v) {
                    for (fk, fv) in &t.entries {
                        let on = r.strings(&fk.value, fv).into_iter().map(|(s, _)| s).collect();
                        m.features.push((fk.value.clone(), on));
                    }
                }
            }
            "profile" => {
                if let Some(t) = r.table("profile", v) {
                    for (pk, pv) in &t.entries {
                        let Some(pt) = r.table(&pk.value, pv) else { continue };
                        for (ck, cv) in &pt.entries {
                            match (pk.value.as_str(), ck.value.as_str()) {
                                ("release" | "debug", "contracts") => {
                                    if let Some(s) = r.string("contracts", cv) {
                                        if sisu_lower::ContractLevel::parse(&s).is_some() {
                                            if pk.value == "release" {
                                                m.release_contracts = Some(s);
                                            }
                                        } else {
                                            r.bad("contracts", cv, "`off`, `preconditions` or `all`");
                                        }
                                    }
                                }
                                _ => r.unknown(ck, &format!("[profile.{}]", pk.value)),
                            }
                        }
                    }
                }
            }
            // The review gate of 10 is data the toolchain reads later.
            "review" => {}
            _ => r.unknown(k, "sisu.toml"),
        }
    }
    // `[trust]` names dependencies.
    for (name, range) in &m.trust_unsafe {
        if !m.dependencies.iter().any(|d| d.name == *name) {
            let d = Diagnostic::new(msg::PACKAGE_TRUST_UNKNOWN, r.span(*range)).with("name", name.clone());
            r.diags.push(d);
        }
    }
    (Some(m), r.diags)
}

/// The inline metadata of a script (10, "REPL and scripts"; PEP 723's
/// shape): after the `#!` line, a comment block that opens with
/// `# /// script` and closes with `# ///`, whose lines with `# ` taken off
/// are a manifest. Returns the manifest's text, with each line where it is
/// in the script (so a diagnostic's position is the script's), or the
/// position of a block that never closes.
pub fn script_metadata(text: &str) -> Result<Option<String>, TextRange> {
    // A line as spaces of the same length, keeping its line break, so the
    // manifest's offsets are the script's.
    let blank = |line: &str| {
        let body = line.trim_end_matches(['\n', '\r']);
        format!("{}{}", " ".repeat(body.len()), &line[body.len()..])
    };
    let mut out = String::new();
    let mut offset = 0usize;
    let mut open: Option<usize> = None;
    for line in text.split_inclusive('\n') {
        let start = offset;
        offset += line.len();
        let bare = line.trim_end_matches(['\n', '\r']);
        let ending = &line[bare.len()..];
        match open {
            None => {
                if start == 0 && bare.starts_with("#!") {
                    out.push_str(&blank(line));
                    continue;
                }
                if bare.trim_end() == "# /// script" {
                    open = Some(start);
                    out.push_str(&blank(line));
                    continue;
                }
                if !bare.starts_with('#') && !bare.trim().is_empty() {
                    return Ok(None);
                }
                out.push_str(&blank(line));
            }
            Some(at) => {
                if bare.trim_end() == "# ///" {
                    return Ok(Some(out));
                }
                let opener = TextRange::new(at as u32, (at + "# /// script".len()) as u32);
                // `# key = value`: the two characters become spaces, so the
                // manifest's columns are the script's.
                if let Some(rest) = bare.strip_prefix("# ") {
                    out.push_str("  ");
                    out.push_str(rest);
                } else if bare == "#" {
                    out.push(' ');
                } else {
                    return Err(opener);
                }
                out.push_str(ending);
            }
        }
    }
    match open {
        Some(at) => Err(TextRange::new(at as u32, (at + "# /// script".len()) as u32)),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_ok(text: &str) -> (Manifest, Vec<String>) {
        let (m, d) = read(FileId(0), text);
        (m.unwrap_or_default(), d.iter().map(|d| d.msg.id().to_string()).collect())
    }

    #[test]
    fn reads_the_proposal_shape() {
        let (m, d) = read_ok(
            "[package]\nname = \"weather\"\nversion = \"0.3.1\"\nedition = \"2027\"\ndescription = \"Fetches\"\nlicense = \"MIT\"\n\n[dependencies]\nmylib = { path = \"../mylib\" }\nwinapi = { path = \"../winapi\", when = { os = \"windows\" } }\n\n[trust]\nunsafe = [\"mylib\"]\n\n[profile.release]\ncontracts = \"preconditions\"\n",
        );
        assert!(d.is_empty(), "{d:?}");
        let p = m.package.unwrap();
        assert_eq!(
            (p.name.as_str(), p.version.as_str(), p.edition.as_deref()),
            ("weather", "0.3.1", Some("2027"))
        );
        assert_eq!(m.dependencies.len(), 2);
        let w = m.dependencies[1].when.clone().unwrap();
        assert!(!w.holds(&[("os", "linux")]));
        assert!(w.holds(&[("os", "windows")]));
        assert_eq!(m.trust_unsafe.len(), 1);
        assert_eq!(m.release_contracts.as_deref(), Some("preconditions"));
    }

    #[test]
    fn reports_mistakes() {
        let (_, d) = read_ok("[package]\nname = \"Bad-Name\"\nfrob = 1\n\n[dependencies]\nhttp2 = \"1.2\"\n");
        assert!(d.contains(&"package.bad-name".to_string()));
        assert!(d.contains(&"package.unknown-key".to_string()));
        assert!(d.contains(&"package.missing-key".to_string()));
        assert!(d.contains(&"package.not-a-path".to_string()));
        let (_, d) = read_ok("[package]\nname = \"json\"\nversion = \"1.0.0\"\n");
        assert_eq!(d, vec!["package.reserved-name".to_string()]);
        let (_, d) = read_ok("[package]\nname = 3\n");
        assert!(d.contains(&"package.bad-value".to_string()));
    }

    #[test]
    fn script_metadata_keeps_positions() {
        let s =
            "#!/usr/bin/env sisu\n# /// script\n# [dependencies]\n# a = { path = \"a\" }\n# ///\nprint(1)\n";
        let m = script_metadata(s).unwrap().unwrap();
        assert_eq!(m.lines().nth(2).unwrap(), "  [dependencies]");
        assert_eq!(m.len(), s.find("# ///\nprint").unwrap());
        assert!(script_metadata("#!/usr/bin/env sisu\nprint(1)\n").unwrap().is_none());
        assert!(script_metadata("# /// script\n# [package]\n").is_err());
    }
}
