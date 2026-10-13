//! The message catalog: identifiers mapped to codes, kinds and templates.
//!
//! The format is line based so that a front end in any language can read it:
//!
//! ```text
//! # a comment
//! CODE  KIND  IDENTIFIER  TEMPLATE...
//! ```
//!
//! `CODE` is a stable code such as `E0101`, or `-` for a message that is not
//! a diagnostic of its own (a label, a fix or a note). `KIND` is one of
//! `error`, `warning`, `note`, `help`, `label`, `fix`, `runtime`, `cli`.
//! The template is the rest of the line: `{name}` is a parameter, `{{` and
//! `}}` are literal braces and `\n` is a line break.

use std::collections::HashMap;
use std::sync::OnceLock;

/// The English catalog shipped with the compiler.
pub const BUILTIN_CATALOG: &str = include_str!("../../../catalog/messages.catalog");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Error,
    Warning,
    Note,
    Help,
    Label,
    Fix,
    Runtime,
    Cli,
}

impl Kind {
    fn parse(s: &str) -> Option<Kind> {
        Some(match s {
            "error" => Kind::Error,
            "warning" => Kind::Warning,
            "note" => Kind::Note,
            "help" => Kind::Help,
            "label" => Kind::Label,
            "fix" => Kind::Fix,
            "runtime" => Kind::Runtime,
            "cli" => Kind::Cli,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Entry {
    /// `None` for `-`.
    pub code: Option<String>,
    pub kind: Kind,
    pub id: String,
    pub template: String,
}

#[derive(Clone, Debug, Default)]
pub struct Catalog {
    entries: Vec<Entry>,
    by_id: HashMap<String, usize>,
}

impl Catalog {
    /// Parses a catalog. Every problem is reported as `line: description`.
    pub fn parse(text: &str) -> Result<Catalog, Vec<String>> {
        let mut catalog = Catalog::default();
        let mut problems = Vec::new();
        let mut codes: HashMap<String, usize> = HashMap::new();
        for (index, raw) in text.lines().enumerate() {
            let line_no = index + 1;
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut rest = line;
            let mut fields = Vec::new();
            for _ in 0..3 {
                let trimmed = rest.trim_start();
                let end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
                fields.push(&trimmed[..end]);
                rest = &trimmed[end..];
            }
            let template = rest.trim();
            let (code, kind, id) = (fields[0], fields[1], fields[2]);
            if id.is_empty() || template.is_empty() {
                problems
                    .push(format!("{line_no}: an entry needs a code, a kind, an identifier and a template"));
                continue;
            }
            let Some(kind) = Kind::parse(kind) else {
                problems.push(format!("{line_no}: unknown kind `{kind}`"));
                continue;
            };
            let code = if code == "-" { None } else { Some(code.to_string()) };
            if let Some(c) = &code
                && let Some(prev) = codes.insert(c.clone(), line_no)
            {
                problems.push(format!("{line_no}: code {c} is already used on line {prev}"));
            }
            if catalog.by_id.contains_key(id) {
                problems.push(format!("{line_no}: identifier {id} is defined twice"));
                continue;
            }
            catalog.by_id.insert(id.to_string(), catalog.entries.len());
            catalog.entries.push(Entry { code, kind, id: id.to_string(), template: template.to_string() });
        }
        if problems.is_empty() { Ok(catalog) } else { Err(problems) }
    }

    /// The built-in English catalog. It is checked by a unit test; if it were
    /// malformed at run time, messages render as their identifiers.
    pub fn builtin() -> &'static Catalog {
        static CATALOG: OnceLock<Catalog> = OnceLock::new();
        CATALOG.get_or_init(|| Catalog::parse(BUILTIN_CATALOG).unwrap_or_default())
    }

    pub fn get(&self, id: &str) -> Option<&Entry> {
        self.by_id.get(id).map(|&i| &self.entries[i])
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// Renders a message. An unknown identifier or parameter renders
    /// visibly rather than failing, so a catalog mistake cannot hide a
    /// diagnostic.
    pub fn render(&self, id: &str, params: &[(&str, String)]) -> String {
        match self.get(id) {
            Some(entry) => fill_template(&entry.template, params),
            None => {
                let mut s = id.to_string();
                for (k, v) in params {
                    s.push_str(&format!(" {k}={v}"));
                }
                s
            }
        }
    }

    pub fn code(&self, id: &str) -> Option<&str> {
        self.get(id).and_then(|e| e.code.as_deref())
    }

    pub fn kind(&self, id: &str) -> Option<Kind> {
        self.get(id).map(|e| e.kind)
    }
}

/// Substitutes `{name}` parameters into a template.
pub fn fill_template(template: &str, params: &[(&str, String)]) -> String {
    let mut out = String::new();
    let chars: Vec<char> = template.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '{' && chars.get(i + 1) == Some(&'{') {
            out.push('{');
            i += 2;
        } else if c == '}' && chars.get(i + 1) == Some(&'}') {
            out.push('}');
            i += 2;
        } else if c == '\\' && chars.get(i + 1) == Some(&'n') {
            out.push('\n');
            i += 2;
        } else if c == '{' {
            let mut j = i + 1;
            let mut name = String::new();
            while j < chars.len() && chars[j] != '}' {
                name.push(chars[j]);
                j += 1;
            }
            match params.iter().find(|(k, _)| *k == name) {
                Some((_, v)) => out.push_str(v),
                None => {
                    out.push('{');
                    out.push_str(&name);
                    out.push('}');
                }
            }
            i = j + 1;
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

/// The parameter names a template mentions, in order.
pub fn template_params(template: &str) -> Vec<String> {
    let mut names = Vec::new();
    let chars: Vec<char> = template.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '{' && chars.get(i + 1) == Some(&'{') {
            i += 2;
        } else if chars[i] == '{' {
            let mut j = i + 1;
            let mut name = String::new();
            while j < chars.len() && chars[j] != '}' {
                name.push(chars[j]);
                j += 1;
            }
            names.push(name);
            i = j + 1;
        } else {
            i += 1;
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_catalog_parses() {
        if let Err(problems) = Catalog::parse(BUILTIN_CATALOG) {
            panic!("catalog problems:\n{}", problems.join("\n"));
        }
    }

    #[test]
    fn templates_fill() {
        let p = [("name", "x".to_string())];
        assert_eq!(fill_template("cannot find `{name}`", &p), "cannot find `x`");
        assert_eq!(fill_template("{{literal}} {missing}", &p), "{literal} {missing}");
        assert_eq!(template_params("a {b} {{c}} {d}"), vec!["b", "d"]);
    }

    #[test]
    fn rejects_duplicates_and_bad_kinds() {
        assert!(Catalog::parse("E1 error a.b x\nE1 error a.c y\n").is_err());
        assert!(Catalog::parse("E1 frob a.b x\n").is_err());
        assert!(Catalog::parse("- label a.b x\n- label a.b y\n").is_err());
    }
}
