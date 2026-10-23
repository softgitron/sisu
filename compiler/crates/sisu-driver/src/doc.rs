//! `sisu doc` (09, "Documentation"; 10, "One binary"): every module of a
//! program as a Markdown page, a pure function of the modules' syntax and
//! the program's resolved names (decision 258), so that the front end in
//! Sisu can transcribe it (18, "Bootstrapping": `sisu doc` among stage 1's
//! `pure` functions over the syntax tree). Nothing is evaluated: a doc
//! example is shown as it is written, and `sisu test` runs it.
//!
//! A page is the module's path as its title, its `##!` lines, the modules
//! under it, the names it re-exports, a list of its items, and a section
//! for each item: its declaration as written (a header over several lines
//! joined into one, comments and bodies left out, a contract keeping its
//! own lines), its `##` lines, which are Markdown, and what the compiler
//! knows of it that the declaration does not say: the error set it
//! inferred for a `T !` function (05), the typed holes left in a function
//! (10), the parameters that hold capabilities (09). Decision 258 states
//! the form byte for byte; this file follows it.

use sisu_resolve::{DefId, DefKind, ModuleId, Predicate, Program, Ty, std_sources};
use sisu_syntax::SyntaxKind;
use sisu_syntax::ast::{self, ItemKind};
use sisu_syntax::tree::Token;
use std::collections::{HashMap, HashSet};

/// A module to document.
pub struct Source<'a> {
    pub id: ModuleId,
    /// The module's path as an import names it (`shapes.geometry`,
    /// `std.time`), or a single file's stem: its page is `PATH.md`.
    pub path: String,
    pub text: &'a str,
    pub tokens: &'a [Token],
    /// The module as the build has it, its `@cfg`s applied (decision 243).
    pub module: &'a ast::Module,
}

/// What the pages are made from.
pub struct Docs<'a> {
    /// The modules with pages, in the order the pages are printed.
    pub sources: &'a [Source<'a>],
    pub prog: &'a Program,
    /// The error set the checker inferred for each `T !` function (05:
    /// "`sisu doc` shows it").
    pub inferred: &'a HashMap<DefId, Vec<DefId>>,
    /// A package's pages show what another package may name: its `pub`
    /// items and their `pub` members. A single file's and the standard
    /// library's show everything (decision 147: std's items are public).
    pub public_only: bool,
}

/// One page: a module's documentation.
pub struct Page {
    /// The file's name: the module's path, then `.md`.
    pub file: String,
    pub text: String,
}

/// Every module's page.
pub fn pages(docs: &Docs<'_>) -> Vec<Page> {
    docs.sources
        .iter()
        .map(|src| Page { file: format!("{}.md", src.path), text: Writer::new(docs, src).page() })
        .collect()
}

/// What `sisu doc` prints without `--output`: each page after a line that
/// names its file, `<!-- NAME.md -->`, which Markdown shows as nothing.
pub fn printed(pages: &[Page]) -> String {
    let mut out = String::new();
    for p in pages {
        out.push_str(&format!("<!-- {} -->\n{}", p.file, p.text));
    }
    out
}

const STEP: &str = "    ";

/// The capability types, as `sisu audit --caps` has them (decision 240):
/// the handles of a `System`, the `System`, a test clock, and a `Lent` of
/// a handle.
const CAPABILITIES: [&str; 10] =
    ["Fs", "Net", "Clock", "Rand", "Env", "Proc", "Stdin", "System", "TestClock", "Lent"];

/// Each operations trait and its handle: a type parameter bounded by one
/// (`[F: FsOps]`, the form that takes a `Lent`) holds that capability.
const OPS_TRAITS: [(&str, &str); 7] = [
    ("FsOps", "Fs"),
    ("NetOps", "Net"),
    ("ClockOps", "Clock"),
    ("RandOps", "Rand"),
    ("EnvOps", "Env"),
    ("ProcOps", "Proc"),
    ("StdinOps", "Stdin"),
];

fn is_code(k: SyntaxKind) -> bool {
    !k.is_trivia()
        && !matches!(k, SyntaxKind::NEWLINE | SyntaxKind::INDENT | SyntaxKind::DEDENT | SyntaxKind::EOF)
}

/// `<a id="NAME"></a>`: where a link to an item lands, the same in every
/// renderer of Markdown.
fn anchor(name: &str) -> String {
    format!("<a id=\"{name}\"></a>")
}

/// `text` as inline code: between backticks one more than the longest run
/// inside it, with a space inside each end when it starts or ends with one.
fn code_span(text: &str) -> String {
    let mut longest = 0;
    let mut run = 0;
    for c in text.chars() {
        run = if c == '`' { run + 1 } else { 0 };
        longest = longest.max(run);
    }
    let fence = "`".repeat(longest + 1);
    if text.starts_with('`') || text.ends_with('`') {
        format!("{fence} {text} {fence}")
    } else {
        format!("{fence}{text}{fence}")
    }
}

/// The lines of a doc comment as Markdown: the blank lines at either end
/// dropped, and the opening fence of an example (a line "```" outside a
/// fenced block, which `sisu test` runs as Sisu) labelled "```sisu".
fn markdown(lines: &[String]) -> Option<String> {
    let first = lines.iter().position(|l| !l.trim().is_empty())?;
    let last = lines.iter().rposition(|l| !l.trim().is_empty())?;
    let mut out = Vec::new();
    let mut fenced = false;
    for line in &lines[first..=last] {
        let t = line.trim();
        if !fenced && t.starts_with("```") {
            fenced = true;
            if t == "```" {
                out.push(format!("{}sisu", line.trim_end()));
                continue;
            }
        } else if fenced && t == "```" {
            fenced = false;
        }
        out.push(line.clone());
    }
    Some(out.join("\n"))
}

/// The first paragraph of a doc comment, on one line: its lines up to the
/// first blank one or fence, each trimmed, joined by spaces.
fn summary(lines: &[String]) -> Option<String> {
    let words: Vec<&str> = lines
        .iter()
        .map(|l| l.trim())
        .skip_while(|l| l.is_empty())
        .take_while(|l| !l.is_empty() && !l.starts_with("```"))
        .collect();
    (!words.is_empty()).then(|| words.join(" "))
}

/// A list entry for a link, with the summary after a colon.
fn entry(text: &str, link: &str, lines: &[String]) -> String {
    match summary(lines) {
        Some(s) => format!("- [`{text}`]({link}): {s}"),
        None => format!("- [`{text}`]({link})"),
    }
}

/// The name of the type an impl is for: a path's names joined by dots
/// (`Point` for `Deque[T]`'s `Deque`, `other.Point`), `Option` for `T?`,
/// `Result` for `T ! E`, and a reference's type for the reference.
fn type_name(t: &ast::TypeExpr) -> Option<String> {
    match &t.kind {
        ast::TypeKind::Path { segments, .. } if !segments.is_empty() => {
            Some(segments.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join("."))
        }
        ast::TypeKind::Option(_) => Some("Option".into()),
        ast::TypeKind::Result { .. } => Some("Result".into()),
        ast::TypeKind::Ref { inner, .. } => type_name(inner),
        _ => None,
    }
}

fn self_name(i: &ast::ImplDef) -> Option<String> {
    type_name(&i.self_ty)
}

struct Writer<'a> {
    docs: &'a Docs<'a>,
    src: &'a Source<'a>,
    /// The module's functions, by where their names start.
    fns: HashMap<u32, DefId>,
    /// Whether `todo` names the prelude's typed hole in this module.
    todo_is_hole: bool,
}

impl<'a> Writer<'a> {
    fn new(docs: &'a Docs<'a>, src: &'a Source<'a>) -> Writer<'a> {
        let prog = docs.prog;
        let fns = prog
            .defs
            .iter()
            .enumerate()
            .filter(|(_, d)| d.module == src.id && matches!(d.kind, DefKind::Fn(_)))
            .map(|(i, d)| (d.span.range.start, DefId(i as u32)))
            .collect();
        let todo = prog
            .modules
            .get(src.id.0 as usize)
            .and_then(|m| m.items.get("todo"))
            .or_else(|| prog.modules.get(prog.prelude.0 as usize).and_then(|m| m.items.get("todo")));
        let todo_is_hole =
            todo.is_some_and(|d| prog.fn_info(*d).and_then(|f| f.intrinsic.as_deref()) == Some("todo"));
        Writer { docs, src, fns, todo_is_hole }
    }

    fn shown(&self, vis: ast::Vis) -> bool {
        !self.docs.public_only || vis == ast::Vis::Pub
    }

    fn text(&self, t: &Token) -> &str {
        self.src.text.get(t.range.start as usize..t.range.end as usize).unwrap_or("")
    }

    /// The index of the first token at or after `at`.
    fn token_at(&self, at: u32) -> usize {
        self.src.tokens.partition_point(|t| t.range.start < at)
    }

    /// The code tokens from `start` up to the end of the header line: the
    /// first line break outside brackets, or `end`; a `:` that ends it is
    /// left out, and so are the tokens of the attributes `skip`.
    fn header(&self, start: u32, end: u32, skip: &[ast::Attr]) -> String {
        let mut depth = 0i32;
        let mut picked = Vec::new();
        let mut i = self.token_at(start);
        while let Some(t) = self.src.tokens.get(i) {
            i += 1;
            if t.range.start >= end {
                break;
            }
            if skip.iter().any(|a| a.range.start <= t.range.start && t.range.end <= a.range.end) {
                continue;
            }
            match t.kind {
                SyntaxKind::L_PAREN | SyntaxKind::L_BRACKET | SyntaxKind::L_BRACE => depth += 1,
                SyntaxKind::R_PAREN | SyntaxKind::R_BRACKET | SyntaxKind::R_BRACE => depth -= 1,
                SyntaxKind::NEWLINE | SyntaxKind::INDENT if depth <= 0 && !picked.is_empty() => break,
                _ => {}
            }
            if is_code(t.kind) {
                picked.push(*t);
            }
        }
        if picked.last().is_some_and(|t| t.kind == SyntaxKind::COLON) {
            picked.pop();
        }
        self.join(&picked)
    }

    /// The code tokens in `start..end`, joined into one line.
    fn compact(&self, start: u32, end: u32) -> String {
        let from = self.token_at(start);
        let picked: Vec<Token> = self.src.tokens[from..]
            .iter()
            .take_while(|t| t.range.end <= end)
            .filter(|t| is_code(t.kind))
            .copied()
            .collect();
        self.join(&picked)
    }

    /// Tokens as written, a gap between two of them kept as one space; a
    /// gap that was a line break inside brackets goes, and so does the
    /// trailing comma of a list written one element per line.
    fn join(&self, picked: &[Token]) -> String {
        let mut out = String::new();
        for (k, t) in picked.iter().enumerate() {
            if k > 0 {
                let prev = picked[k - 1];
                let gap = self.src.text.get(prev.range.end as usize..t.range.start as usize).unwrap_or("");
                let broken = gap.contains('\n');
                let closing =
                    matches!(t.kind, SyntaxKind::R_PAREN | SyntaxKind::R_BRACKET | SyntaxKind::R_BRACE);
                if broken && closing && prev.kind == SyntaxKind::COMMA {
                    out.pop();
                    if out.ends_with(' ') {
                        out.pop();
                    }
                }
                let opening =
                    matches!(prev.kind, SyntaxKind::L_PAREN | SyntaxKind::L_BRACKET | SyntaxKind::L_BRACE);
                if !gap.is_empty() && !(broken && (opening || closing)) {
                    out.push(' ');
                }
            }
            out.push_str(self.text(t));
        }
        out
    }

    /// The `##` lines just before the token at `start`, each without its
    /// `##` and the one space after it.
    fn docs_before(&self, start: u32) -> Vec<String> {
        let mut docs = Vec::new();
        let mut i = self.token_at(start);
        while i > 0 {
            i -= 1;
            let t = &self.src.tokens[i];
            match t.kind {
                SyntaxKind::DOC_COMMENT => {
                    let s = self.text(t);
                    let s = s.strip_prefix("##").unwrap_or(s);
                    docs.push(s.strip_prefix(' ').unwrap_or(s).to_string());
                }
                SyntaxKind::WHITESPACE
                | SyntaxKind::LINE_BREAK
                | SyntaxKind::NEWLINE
                | SyntaxKind::INDENT
                | SyntaxKind::DEDENT => {}
                _ => break,
            }
        }
        docs.reverse();
        docs
    }

    /// The `##` lines before the code token that comes before `at` (the
    /// `const` of a constant whose name starts at `at`).
    fn docs_before_keyword(&self, at: u32) -> Vec<String> {
        let mut i = self.token_at(at);
        while i > 0 {
            i -= 1;
            if is_code(self.src.tokens[i].kind) {
                return self.docs_before(self.src.tokens[i].range.start);
            }
        }
        Vec::new()
    }

    /// A doc comment inside a declaration, as written, at `indent`.
    fn doc_comment(&self, docs: &[String], indent: &str, out: &mut String) {
        for d in docs {
            if d.is_empty() {
                out.push_str(&format!("{indent}##\n"));
            } else {
                out.push_str(&format!("{indent}## {d}\n"));
            }
        }
    }

    fn attrs(&self, attrs: &[ast::Attr], indent: &str, out: &mut String) {
        for a in attrs {
            out.push_str(&format!("{indent}{}\n", self.compact(a.range.start, a.range.end)));
        }
    }

    /// A contract clause as written; the lines after its first keep their
    /// indentation relative to it.
    fn contract(&self, c: &ast::Contract, indent: &str, out: &mut String) {
        let start = c.range.start as usize;
        let text = self.src.text.get(start..c.range.end as usize).unwrap_or("");
        let line_start = self.src.text.get(..start).and_then(|s| s.rfind('\n')).map_or(0, |p| p + 1);
        let column = start - line_start;
        for (k, line) in text.lines().enumerate() {
            let line = line.trim_end();
            if k == 0 {
                out.push_str(&format!("{indent}{line}\n"));
            } else if !line.trim().is_empty() {
                let lead = line.len() - line.trim_start().len();
                let rest = &line[lead.min(column)..];
                out.push_str(&format!("{indent}{rest}\n"));
            }
        }
    }

    fn constant(&self, c: &ast::ConstDef, indent: &str, out: &mut String) {
        match &c.ty {
            Some(t) => out.push_str(&format!(
                "{indent}const {}: {}\n",
                c.name.text,
                self.compact(t.range.start, t.range.end)
            )),
            None => out.push_str(&format!("{indent}const {}\n", c.name.text)),
        }
    }

    /// The path of a module as its page names it.
    fn module_path(&self, m: ModuleId) -> String {
        if let Some(s) = self.docs.sources.iter().find(|s| s.id == m) {
            return s.path.clone();
        }
        let Some(data) = self.docs.prog.modules.get(m.0 as usize) else { return String::new() };
        if !data.path.is_empty() {
            data.path.join(".")
        } else if data.is_std {
            std_sources::display_path(&data.name)
        } else {
            data.name.clone()
        }
    }

    /// The page of a module that has one in this documentation.
    fn page_of(&self, m: ModuleId) -> Option<String> {
        self.docs.sources.iter().find(|s| s.id == m).map(|s| format!("{}.md", s.path))
    }

    /// A definition's name as this module's page writes it: bare when it
    /// is this module's or the prelude's, else after its module's path.
    fn name_here(&self, d: DefId) -> String {
        let def = self.docs.prog.def(d);
        if def.module == self.src.id || def.module == self.docs.prog.prelude {
            def.name.clone()
        } else {
            format!("{}.{}", self.module_path(def.module), def.name)
        }
    }

    fn page(&self) -> String {
        let m = self.src.module;
        let mut blocks = vec![format!("# `{}`", self.src.path)];
        blocks.extend(markdown(&m.docs));
        blocks.extend(self.children());
        blocks.extend(self.reexports());
        // The types the module declares take its impls of them.
        let types: HashSet<&str> = m
            .items
            .iter()
            .filter_map(|item| match &item.kind {
                ItemKind::Struct(s) => Some(s.name.text.as_str()),
                ItemKind::Enum(e) | ItemKind::ErrorSet(e) => Some(e.name.text.as_str()),
                ItemKind::TypeAlias(a) => Some(a.name.text.as_str()),
                _ => None,
            })
            .collect();
        let mut impls_of: HashMap<String, Vec<&ast::Item>> = HashMap::new();
        let mut others: Vec<&ast::Item> = Vec::new();
        for item in &m.items {
            if let ItemKind::Impl(i) = &item.kind {
                match self_name(i) {
                    Some(n) if types.contains(n.as_str()) => impls_of.entry(n).or_default().push(item),
                    _ => others.push(item),
                }
            }
        }
        let mut contents = Vec::new();
        let mut sections = Vec::new();
        for item in &m.items {
            if !self.shown(item.vis) {
                continue;
            }
            let impls = |name: &str| impls_of.get(name).map(Vec::as_slice).unwrap_or(&[]);
            let (name, section) = match &item.kind {
                ItemKind::Fn(f) => (f.name.text.as_str(), self.function(f, &item.docs, None, None, 2)),
                ItemKind::Struct(s) => {
                    (s.name.text.as_str(), self.struct_section(item, s, impls(&s.name.text)))
                }
                ItemKind::Enum(e) | ItemKind::ErrorSet(e) => {
                    (e.name.text.as_str(), self.enum_section(item, e, impls(&e.name.text)))
                }
                ItemKind::Trait(t) => (t.name.text.as_str(), self.trait_section(item, t)),
                ItemKind::Const(c) => (c.name.text.as_str(), self.const_section(item, c)),
                ItemKind::TypeAlias(a) => {
                    (a.name.text.as_str(), self.alias_section(item, a, impls(&a.name.text)))
                }
                _ => continue,
            };
            contents.push(entry(name, &format!("#{name}"), &item.docs));
            sections.push(section);
        }
        if !contents.is_empty() {
            blocks.push(format!("## Contents\n\n{}", contents.join("\n")));
        }
        blocks.extend(sections);
        blocks.extend(self.implementations(&others));
        let mut out = blocks.join("\n\n");
        out.push('\n');
        out
    }

    /// The modules one step under this one (a package's), with the first
    /// paragraph of each one's `##!` lines.
    fn children(&self) -> Option<String> {
        let prog = self.docs.prog;
        let me = &prog.modules.get(self.src.id.0 as usize)?.path;
        if me.is_empty() {
            return None;
        }
        let mut kids: Vec<&Source<'_>> = self
            .docs
            .sources
            .iter()
            .filter(|s| {
                prog.modules
                    .get(s.id.0 as usize)
                    .is_some_and(|d| d.path.len() == me.len() + 1 && d.path.starts_with(me))
            })
            .collect();
        kids.sort_by(|a, b| a.path.cmp(&b.path));
        if kids.is_empty() {
            return None;
        }
        let lines: Vec<String> =
            kids.iter().map(|k| entry(&k.path, &format!("{}.md", k.path), &k.module.docs)).collect();
        Some(format!("## Modules\n\n{}", lines.join("\n")))
    }

    /// The names another module sees through this one that it does not
    /// declare: a package module's `pub` imports, and the prelude's names a
    /// std module names too (`std.time`'s `DateTime`).
    fn reexports(&self) -> Option<String> {
        let prog = self.docs.prog;
        let data = prog.modules.get(self.src.id.0 as usize)?;
        let mut lines = Vec::new();
        if data.is_std {
            let prelude = prog.modules.get(prog.prelude.0 as usize)?;
            for name in std_sources::reexports(&data.name) {
                if let Some(d) = prelude.items.get(*name) {
                    lines.push(self.reexport(name, *d));
                }
            }
        } else {
            for item in &self.src.module.items {
                if item.vis != ast::Vis::Pub {
                    continue;
                }
                let bound: Vec<&ast::Name> = match &item.kind {
                    ItemKind::FromImport(imp) => {
                        imp.names.iter().map(|(n, alias)| alias.as_ref().unwrap_or(n)).collect()
                    }
                    ItemKind::Import(imp) => imp.alias.as_ref().or(imp.path.last()).into_iter().collect(),
                    _ => continue,
                };
                for b in bound {
                    if let Some(d) = data.items.get(&b.text) {
                        lines.push(self.reexport(&b.text, *d));
                    }
                }
            }
        }
        (!lines.is_empty()).then(|| format!("## Re-exports\n\n{}", lines.join("\n")))
    }

    /// One re-exported name: what it names, linked to its section when
    /// that has a page here.
    fn reexport(&self, bound: &str, d: DefId) -> String {
        let def = self.docs.prog.def(d);
        let (shown, link) = match &def.kind {
            DefKind::Module(m) => (self.module_path(*m), self.page_of(*m)),
            kind => {
                let item = match kind {
                    DefKind::Fn(f) => f.owner.is_none(),
                    DefKind::Struct(_)
                    | DefKind::Enum(_)
                    | DefKind::ErrorSet(_)
                    | DefKind::Const(_)
                    | DefKind::TypeAlias(_)
                    | DefKind::Trait(_)
                    | DefKind::Refinement(_) => true,
                    _ => false,
                };
                let link = self.page_of(def.module).filter(|_| item).map(|p| format!("{p}#{}", def.name));
                (format!("{}.{}", self.module_path(def.module), def.name), link)
            }
        };
        match link {
            Some(l) => format!("- `{bound}`: [`{shown}`]({l})"),
            None => format!("- `{bound}`: `{shown}`"),
        }
    }

    /// A section's heading, after the anchor its links land on.
    fn heading(&self, name: &str, level: usize) -> String {
        format!("{}\n{} `{name}`", anchor(name), "#".repeat(level))
    }

    /// A function's section: its declaration (attributes, header and
    /// contracts), its doc comment, and what the compiler adds.
    fn function(
        &self,
        f: &ast::FnDef,
        docs: &[String],
        owner: Option<&str>,
        invariant_of: Option<&str>,
        level: usize,
    ) -> String {
        let name = match owner {
            Some(o) => format!("{o}.{}", f.name.text),
            None => f.name.text.clone(),
        };
        let mut code = String::new();
        self.attrs(&f.attrs, "", &mut code);
        let mut header = self.header(f.range.start, f.range.end, &f.attrs);
        // `T !`: the set the compiler inferred from the body (05).
        if header.ends_with('!')
            && let Some(set) = self.inferred(f)
        {
            header.push_str(&format!(" {set}  # inferred"));
        }
        code.push_str(&header);
        code.push('\n');
        for c in f.contracts() {
            self.contract(c, STEP, &mut code);
        }
        if let Some(owner) = invariant_of
            && f.params
                .first()
                .is_some_and(|p| matches!(p.kind, ast::ParamKind::SelfParam) && p.mode == ast::Mode::Mut)
        {
            code.push_str(&format!("{STEP}# re-checks the invariant of `{owner}` on return\n"));
        }
        let mut parts = vec![format!("{}\n\n```sisu\n{code}```", self.heading(&name, level))];
        parts.extend(markdown(docs));
        let holes = self.holes(f);
        if !holes.is_empty() {
            let shown: Vec<String> = holes.iter().map(|h| code_span(h)).collect();
            parts.push(format!("Unfinished: {}.", shown.join(", ")));
        }
        let caps = self.capabilities(f);
        if !caps.is_empty() {
            parts.push(format!("Capabilities: {}.", caps.join(", ")));
        }
        parts.join("\n\n")
    }

    /// The inferred error set of a `T !` function, as a type is written.
    fn inferred(&self, f: &ast::FnDef) -> Option<String> {
        let def = self.fns.get(&f.name.range.start)?;
        let set = self.docs.inferred.get(def)?;
        let names: Vec<String> = set.iter().map(|d| self.name_here(*d)).collect();
        Some(match names.as_slice() {
            [] => "{}".into(),
            [one] => one.clone(),
            _ => format!("{{{}}}", names.join(", ")),
        })
    }

    /// The typed holes in a function's body (10: `sisu doc` marks the
    /// functions that contain one): each call of the prelude's `todo`, as
    /// written, once.
    fn holes(&self, f: &ast::FnDef) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let Some(body) = &f.body else { return out };
        if !self.todo_is_hole {
            return out;
        }
        ast::visit::block(body, &mut |e| {
            if let ast::ExprKind::Call { callee, .. } = &e.kind {
                let base = match &callee.kind {
                    ast::ExprKind::Index { base, .. } => base,
                    _ => callee,
                };
                if matches!(&base.kind, ast::ExprKind::Name(n) if n == "todo") {
                    let text = self.compact(e.range.start, e.range.end);
                    if !out.contains(&text) {
                        out.push(text);
                    }
                }
            }
        });
        out
    }

    /// The parameters of a function that hold capabilities, `self` first,
    /// each with the capability types it holds: ``fs` (`Fs`)`.
    fn capabilities(&self, f: &ast::FnDef) -> Vec<String> {
        let mut out = Vec::new();
        let Some(def) = self.fns.get(&f.name.range.start) else { return out };
        let Some(info) = self.docs.prog.fn_info(*def) else { return out };
        let mut add = |name: &str, ty: &Ty| {
            let mut held = Vec::new();
            self.held(ty, &info.predicates, &mut held, &mut Vec::new());
            if !held.is_empty() {
                let types: Vec<String> = held.iter().map(|h| format!("`{h}`")).collect();
                out.push(format!("`{name}` ({})", types.join(", ")));
            }
        };
        if info.self_mode.is_some()
            && let Some(owner) = &info.owner
        {
            add("self", owner);
        }
        for p in &info.params {
            add(&p.name, &p.ty);
        }
        out
    }

    /// The capability types a type holds: itself, through references, the
    /// elements of tuples, its type arguments (options and collections),
    /// the fields of a struct or enum, and a type parameter bounded by an
    /// operations trait (decision 240's rule, read from the signature).
    fn held(&self, ty: &Ty, preds: &[Predicate], out: &mut Vec<String>, seen: &mut Vec<DefId>) {
        let prog = self.docs.prog;
        let mut add = |name: String| {
            if !out.contains(&name) {
                out.push(name);
            }
        };
        let ops = |d: DefId| -> Option<&'static str> {
            let def = prog.def(d);
            (def.module == prog.prelude)
                .then(|| OPS_TRAITS.iter().find(|(t, _)| *t == def.name).map(|(_, h)| *h))
                .flatten()
        };
        match ty {
            Ty::Ref(inner, _) => self.held(inner, preds, out, seen),
            Ty::Tuple(elems, _) => elems.iter().for_each(|e| self.held(e, preds, out, seen)),
            Ty::Param(_) => {
                for p in preds.iter().filter(|p| p.ty == *ty) {
                    if let Some(h) = ops(p.bound.def) {
                        add(h.to_string());
                    }
                }
            }
            Ty::Dyn(tr) => {
                if let Some(h) = ops(tr.def) {
                    add(h.to_string());
                }
            }
            Ty::Adt(d, args) => {
                let def = prog.def(*d);
                if def.module == prog.prelude && CAPABILITIES.contains(&def.name.as_str()) {
                    // A `Lent` is its handle's capability.
                    let name = match (def.name.as_str(), args.first()) {
                        ("Lent", Some(Ty::Adt(h, _))) => prog.def(*h).name.clone(),
                        _ => def.name.clone(),
                    };
                    add(name);
                    return;
                }
                args.iter().for_each(|a| self.held(a, preds, out, seen));
                if seen.contains(d) {
                    return;
                }
                seen.push(*d);
                // A field's type parameters are the type's, whose arguments
                // were looked at above: the function's bounds say nothing
                // of them.
                match &def.kind {
                    DefKind::Struct(s) => s.fields.iter().for_each(|f| self.held(&f.ty, &[], out, seen)),
                    DefKind::Enum(e) | DefKind::ErrorSet(e) => e
                        .variants
                        .iter()
                        .flat_map(|v| v.fields.iter())
                        .for_each(|(_, t)| self.held(t, &[], out, seen)),
                    _ => {}
                }
            }
            _ => {}
        }
    }

    /// The headers of a type's impls, after a blank line, each after its
    /// doc comment.
    fn impl_headers(&self, impls: &[&ast::Item], out: &mut String) {
        if impls.is_empty() {
            return;
        }
        out.push('\n');
        for item in impls {
            self.doc_comment(&item.docs, "", out);
            out.push_str(&self.header(item.range.start, item.range.end, &item.attrs));
            out.push('\n');
        }
    }

    /// The constants of a type's inherent impls.
    fn impl_constants(&self, impls: &[&ast::Item], out: &mut String) {
        for item in impls {
            if let ItemKind::Impl(i) = &item.kind
                && i.trait_ty.is_none()
            {
                for c in &i.consts {
                    self.doc_comment(&self.docs_before_keyword(c.name.range.start), STEP, out);
                    self.constant(c, STEP, out);
                }
            }
        }
    }

    /// The sections of a type's methods: those declared in it, then those
    /// of its inherent impls, in order.
    fn methods(
        &self,
        name: &str,
        own: &[ast::FnDef],
        impls: &[&ast::Item],
        invariant_of: Option<&str>,
    ) -> Vec<String> {
        let mut all: Vec<&ast::FnDef> = own.iter().collect();
        for item in impls {
            if let ItemKind::Impl(i) = &item.kind
                && i.trait_ty.is_none()
            {
                all.extend(i.methods.iter());
            }
        }
        all.into_iter()
            .filter(|m| self.shown(m.vis))
            .map(|m| self.function(m, &self.docs_before(m.range.start), Some(name), invariant_of, 3))
            .collect()
    }

    /// The start of a type's section: its declaration and doc comment.
    fn type_start(&self, name: &str, code: &str, docs: &[String]) -> Vec<String> {
        let mut parts = vec![format!("{}\n\n```sisu\n{code}```", self.heading(name, 2))];
        parts.extend(markdown(docs));
        parts
    }

    fn struct_section(&self, item: &ast::Item, s: &ast::StructDef, impls: &[&ast::Item]) -> String {
        let name = s.name.text.as_str();
        let mut code = String::new();
        self.attrs(&item.attrs, "", &mut code);
        code.push_str(&self.header(item.range.start, item.range.end, &item.attrs));
        code.push('\n');
        let mut hidden = false;
        for field in &s.fields {
            if !self.shown(field.vis) {
                hidden = true;
                continue;
            }
            self.doc_comment(&self.docs_before(field.range.start), STEP, &mut code);
            code.push_str(&format!("{STEP}{}\n", self.compact(field.range.start, field.range.end)));
        }
        if hidden {
            code.push_str(&format!("{STEP}# private fields\n"));
        }
        for inv in &s.invariants {
            self.contract(inv, STEP, &mut code);
        }
        for c in &s.consts {
            self.doc_comment(&self.docs_before_keyword(c.name.range.start), STEP, &mut code);
            self.constant(c, STEP, &mut code);
        }
        self.impl_constants(impls, &mut code);
        self.impl_headers(impls, &mut code);
        let mut parts = self.type_start(name, &code, &item.docs);
        let invariant_of = (!s.invariants.is_empty()).then_some(name);
        parts.extend(self.methods(name, &s.methods, impls, invariant_of));
        parts.join("\n\n")
    }

    fn enum_section(&self, item: &ast::Item, e: &ast::EnumDef, impls: &[&ast::Item]) -> String {
        let name = e.name.text.as_str();
        let mut code = String::new();
        self.attrs(&item.attrs, "", &mut code);
        code.push_str(&self.header(item.range.start, item.range.end, &item.attrs));
        code.push('\n');
        for v in &e.variants {
            self.doc_comment(&self.docs_before(v.range.start), STEP, &mut code);
            code.push_str(&format!("{STEP}{}\n", self.compact(v.range.start, v.range.end)));
        }
        if e.open {
            code.push_str(&format!("{STEP}..\n"));
        }
        self.impl_constants(impls, &mut code);
        self.impl_headers(impls, &mut code);
        let mut parts = self.type_start(name, &code, &item.docs);
        parts.extend(self.methods(name, &e.methods, impls, None));
        parts.join("\n\n")
    }

    fn trait_section(&self, item: &ast::Item, t: &ast::TraitDef) -> String {
        let name = t.name.text.as_str();
        let mut code = String::new();
        self.attrs(&item.attrs, "", &mut code);
        code.push_str(&self.header(item.range.start, item.range.end, &item.attrs));
        code.push('\n');
        for a in &t.assoc_types {
            self.doc_comment(&self.docs_before(a.range.start), STEP, &mut code);
            code.push_str(&format!("{STEP}{}\n", self.compact(a.range.start, a.range.end)));
        }
        let mut parts = self.type_start(name, &code, &item.docs);
        for m in &t.methods {
            parts.push(self.function(m, &self.docs_before(m.range.start), Some(name), None, 3));
        }
        parts.join("\n\n")
    }

    fn const_section(&self, item: &ast::Item, c: &ast::ConstDef) -> String {
        let mut code = String::new();
        self.attrs(&item.attrs, "", &mut code);
        self.constant(c, "", &mut code);
        self.type_start(&c.name.text, &code, &item.docs).join("\n\n")
    }

    fn alias_section(&self, item: &ast::Item, a: &ast::TypeAlias, impls: &[&ast::Item]) -> String {
        let name = a.name.text.as_str();
        let mut code = String::new();
        self.attrs(&item.attrs, "", &mut code);
        code.push_str(&self.header(item.range.start, item.range.end, &item.attrs));
        code.push('\n');
        // A refinement's one way in from its base (07).
        if a.refinement
            && let Some(base) = &a.ty
        {
            let args = if a.generics.is_empty() {
                String::new()
            } else {
                let names: Vec<&str> = a.generics.iter().map(|g| g.name.text.as_str()).collect();
                format!("[{}]", names.join(", "))
            };
            let base = self.compact(base.range.start, base.range.end);
            code.push_str(&format!("{STEP}def try_from(value: {base}) -> {name}{args} ! RefinementError\n"));
        }
        self.impl_constants(impls, &mut code);
        self.impl_headers(impls, &mut code);
        let mut parts = self.type_start(name, &code, &item.docs);
        parts.extend(self.methods(name, &[], impls, None));
        parts.join("\n\n")
    }

    /// The impls of types the module does not declare: their headers, and
    /// the methods of the inherent ones.
    fn implementations(&self, impls: &[&ast::Item]) -> Option<String> {
        if impls.is_empty() {
            return None;
        }
        let mut code = String::new();
        let mut methods = Vec::new();
        for item in impls {
            let ItemKind::Impl(i) = &item.kind else { continue };
            self.doc_comment(&item.docs, "", &mut code);
            code.push_str(&self.header(item.range.start, item.range.end, &item.attrs));
            code.push('\n');
            if i.trait_ty.is_some() {
                continue;
            }
            for c in &i.consts {
                self.doc_comment(&self.docs_before_keyword(c.name.range.start), STEP, &mut code);
                self.constant(c, STEP, &mut code);
            }
            let owner =
                self_name(i).unwrap_or_else(|| self.compact(i.self_ty.range.start, i.self_ty.range.end));
            for m in i.methods.iter().filter(|m| self.shown(m.vis)) {
                methods.push(self.function(m, &self.docs_before(m.range.start), Some(&owner), None, 3));
            }
        }
        let mut parts = vec![format!("## Implementations\n\n```sisu\n{code}```")];
        parts.extend(methods);
        Some(parts.join("\n\n"))
    }
}
