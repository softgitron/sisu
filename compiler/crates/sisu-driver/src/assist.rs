//! What the language server writes for the editor besides diagnostics and
//! hover (10, "Language server"; N-31): completion of the names in scope
//! and of a value's members as snippets with holes, rename, the fixes of
//! the catalog's diagnostics as code actions, inlay hints for inferred
//! types, inferred error sets and the conventions of `mut` and `own`
//! arguments, the "explain this borrow" command, formatting of a range of
//! lines, and a worksheet's values as inline values. Every answer comes
//! from the queries of `Db`, as the rest of the server's do.

use crate::json::Json;
use crate::lsp::{Lines, Server, Target, contains, path_uri, signature, strip_refs, uri_path, walk_block};
use sisu_diag::{FileId, LineCol, Span, msg};
use sisu_resolve::{DefId, DefKind, Program, TraitRef, Ty};
use sisu_syntax::SyntaxKind;
use sisu_typeck::{ArgMode, Callee, Expr, ExprKind, Typed};
use std::collections::BTreeMap;
use std::path::Path;

/// The command the server offers for "explain this borrow".
pub const EXPLAIN_BORROW: &str = "sisu.explainBorrow";

/// The statements a worksheet the server runs may take.
const SHEET_FUEL: u64 = 10_000_000;

/// The protocol's completion item kinds.
mod kind {
    pub const METHOD: f64 = 2.0;
    pub const FUNCTION: f64 = 3.0;
    pub const FIELD: f64 = 5.0;
    pub const VARIABLE: f64 = 6.0;
    pub const CLASS: f64 = 7.0;
    pub const INTERFACE: f64 = 8.0;
    pub const MODULE: f64 = 9.0;
    pub const ENUM: f64 = 13.0;
    pub const KEYWORD: f64 = 14.0;
    pub const ENUM_MEMBER: f64 = 20.0;
    pub const CONSTANT: f64 = 21.0;
    pub const STRUCT: f64 = 22.0;
}

/// The keywords that start a statement or a declaration.
const KEYWORDS: &[&str] = &[
    "def", "let", "var", "if", "elif", "else", "for", "while", "match", "case", "return", "break",
    "continue", "pass", "struct", "enum", "error", "trait", "impl", "import", "from", "test", "const",
    "type", "with", "defer", "assert", "pure", "own", "mut", "ref", "and", "or", "not", "in", "comptime",
];

fn is_name_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b >= 0x80
}

/// Where the postfix chain ending at `end` starts (`a.b(c)[d]` before a
/// `.`): names, member accesses, calls and indexing, and a string.
fn chain_start(text: &str, end: usize) -> usize {
    let b = text.as_bytes();
    let mut i = end;
    loop {
        let before = i;
        while i > 0 && is_name_byte(b[i - 1]) {
            i -= 1;
        }
        if i > 0 && matches!(b[i - 1], b')' | b']' | b'"') {
            let close = b[i - 1];
            let open = match close {
                b')' => b'(',
                b']' => b'[',
                _ => b'"',
            };
            let mut depth = 0i32;
            let mut j = i;
            while j > 0 {
                j -= 1;
                if b[j] == close && (close != b'"' || j == i - 1) {
                    depth += 1;
                } else if b[j] == open {
                    depth -= 1;
                    if depth == 0 || close == b'"' {
                        break;
                    }
                }
            }
            i = j;
            continue;
        }
        if i > 0 && b[i - 1] == b'.' && i != before {
            i -= 1;
            continue;
        }
        return i;
    }
}

/// A snippet calling `name` with a hole for each argument.
fn call_snippet(name: &str, params: &[String]) -> String {
    let holes: Vec<String> = params.iter().enumerate().map(|(i, p)| format!("${{{}:{p}}}", i + 1)).collect();
    format!("{name}({})", holes.join(", "))
}

fn item(label: &str, kind: f64, detail: String, snippet: Option<String>) -> Json {
    let mut fields =
        vec![("label", Json::str(label)), ("kind", Json::num(kind)), ("detail", Json::str(detail))];
    if let Some(s) = snippet {
        fields.push(("insertText", Json::str(s)));
        fields.push(("insertTextFormat", Json::num(2)));
    }
    Json::obj(fields)
}

/// The required parameters of a function, `self` left out.
fn required_params(prog: &Program, d: DefId) -> Vec<String> {
    prog.fn_info(d)
        .map(|f| f.params.iter().filter(|p| p.default.is_none()).map(|p| p.name.clone()).collect())
        .unwrap_or_default()
}

/// A completion item for a definition named `name`.
fn def_item(prog: &Program, typed: &Typed, name: &str, d: DefId) -> Option<Json> {
    if name.starts_with("__") || name.contains('#') || name.is_empty() {
        return None;
    }
    let def = prog.def(d);
    let mut extra = Vec::new();
    let detail = signature(prog, typed, d, &mut extra);
    Some(match &def.kind {
        DefKind::Fn(f) if f.comptime => return None,
        DefKind::Fn(_) => {
            item(name, kind::FUNCTION, detail, Some(call_snippet(name, &required_params(prog, d))))
        }
        DefKind::Struct(s) => {
            let fields: Vec<String> = s
                .fields
                .iter()
                .filter(|f| f.default.is_none())
                .enumerate()
                .map(|(i, f)| format!("{}=${{{}:{}}}", f.name, i + 1, f.name))
                .collect();
            let snippet =
                if s.builtin.is_some() { None } else { Some(format!("{name}({})", fields.join(", "))) };
            item(name, kind::STRUCT, detail, snippet)
        }
        DefKind::Enum(_) | DefKind::ErrorSet(_) => item(name, kind::ENUM, detail, None),
        DefKind::Const(_) => item(name, kind::CONSTANT, detail, None),
        DefKind::Module(_) => item(name, kind::MODULE, detail, None),
        DefKind::Trait(_) => item(name, kind::INTERFACE, detail, None),
        DefKind::TypeAlias(_) | DefKind::Refinement(_) => item(name, kind::CLASS, detail, None),
        DefKind::Impl(_) | DefKind::Unsupported => return None,
    })
}

/// The members of a value of type `ty`: its fields, its type's methods and
/// the methods of the traits it implements, as snippets with holes.
fn members(prog: &Program, typed: &Typed, ty: &Ty) -> Vec<Json> {
    let t = strip_refs(ty).clone();
    let mut out = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    if let Ty::Adt(d, args) = &t
        && let DefKind::Struct(s) = &prog.def(*d).kind
        && s.builtin.is_none()
    {
        for f in &s.fields {
            seen.push(f.name.clone());
            out.push(item(&f.name, kind::FIELD, prog.display(&f.ty.subst(args)), None));
        }
    }
    let mut method = |name: &str, m: DefId, out: &mut Vec<Json>| {
        let Some(f) = prog.fn_info(m) else { return };
        if f.self_mode.is_none() || name.starts_with("__") || seen.iter().any(|s| s == name) {
            return;
        }
        seen.push(name.to_string());
        let mut extra = Vec::new();
        out.push(item(
            name,
            kind::METHOD,
            signature(prog, typed, m, &mut extra),
            Some(call_snippet(name, &required_params(prog, m))),
        ));
    };
    if let Some(key) = t.key() {
        let mut own: Vec<(&String, DefId)> =
            prog.methods.iter().filter(|((k, _), _)| *k == key).map(|((_, n), d)| (n, *d)).collect();
        own.sort();
        for (n, d) in own {
            method(n, d, &mut out);
        }
    }
    // The methods of every trait the type implements (a trait with
    // parameters of its own is left out: which arguments is not known).
    for (i, def) in prog.defs.iter().enumerate() {
        let DefKind::Trait(info) = &def.kind else { continue };
        if !info.generics.is_empty() || info.methods.is_empty() {
            continue;
        }
        let tr = TraitRef { def: DefId(i as u32), args: Vec::new() };
        if !prog.implements(&t, &tr, &[]).holds() {
            continue;
        }
        for &m in &info.methods {
            method(&prog.def(m).name.clone(), m, &mut out);
        }
    }
    out
}

/// The innermost expression of a body in `file` whose span is `start..end`
/// exactly, or else the one ending at `end` that starts last.
fn expr_spanning(typed: &Typed, file: FileId, start: usize, end: usize) -> Option<Ty> {
    let mut exact: Option<(u32, Ty)> = None;
    let mut ending: Option<(u32, Ty)> = None;
    for body in &typed.bodies {
        if body.span.file != file {
            continue;
        }
        let mut visit = |e: &Expr| {
            if e.span.file != file || e.span.range.end as usize != end || e.ty.contains_error() {
                return;
            }
            let w = e.span.range.end - e.span.range.start;
            if e.span.range.start as usize == start && exact.as_ref().is_none_or(|(x, _)| w <= *x) {
                exact = Some((w, e.ty.clone()));
            }
            if (e.span.range.start as usize) >= start && ending.as_ref().is_none_or(|(x, _)| w <= *x) {
                ending = Some((w, e.ty.clone()));
            }
        };
        walk_block(&body.block, &mut visit);
        for c in &body.closures {
            walk_block(&c.body, &mut visit);
        }
    }
    exact.or(ending).map(|(_, t)| t)
}

impl Server {
    fn text_of(&self, uri: &str) -> String {
        self.docs.get(uri).map(|d| d.text.clone()).unwrap_or_default()
    }

    /// The document's file in its program's source map.
    fn file_of(
        &mut self,
        uri: &str,
    ) -> Option<(crate::lsp::Analysis, std::sync::Arc<crate::db::Checked>, FileId)> {
        let a = self.analyse(uri);
        let c = self.db.checked(&a.root)?;
        let file = c.map.files().iter().find(|f| f.name == a.name)?.id;
        Some((a, c, file))
    }

    /// `textDocument/completion`: after a `.`, the members of the value
    /// before it (analysed without the `.` and the word being typed, so
    /// that it is an expression of its own); elsewhere the names in scope:
    /// the locals of the body declared before the position, the module's
    /// items, the prelude's and the keywords. Each callable is a snippet
    /// with a hole per required argument (N-31).
    pub(crate) fn completion(&mut self, uri: &str, pos: &Json) -> Json {
        let text = self.text_of(uri);
        let offset = Lines::new(&text, self.utf8).offset(pos);
        let b = text.as_bytes();
        let mut start = offset;
        while start > 0 && is_name_byte(b[start - 1]) {
            start -= 1;
        }
        let prefix = text[start..offset].to_string();
        let mut items = if start > 0 && b[start - 1] == b'.' {
            let dot = start - 1;
            let recv = chain_start(&text, dot);
            let edited = format!("{}{}", &text[..dot], &text[offset..]);
            if let Some(doc) = self.docs.get_mut(uri) {
                doc.text = edited;
            }
            let found = self.file_of(uri).and_then(|(_, c, file)| {
                expr_spanning(&c.typed, file, recv, dot).map(|t| members(&c.program, &c.typed, &t))
            });
            if let Some(doc) = self.docs.get_mut(uri) {
                doc.text = text.clone();
            }
            found.unwrap_or_default()
        } else {
            self.scope_items(uri, offset)
        };
        items.retain(|i| i.get("label").and_then(Json::as_str).is_some_and(|l| l.starts_with(&prefix)));
        items.sort_by(|x, y| {
            x.get("label").and_then(Json::as_str).cmp(&y.get("label").and_then(Json::as_str))
        });
        items.dedup_by(|x, y| x.get("label") == y.get("label"));
        Json::obj(vec![("isIncomplete", Json::Bool(false)), ("items", Json::Arr(items))])
    }

    fn scope_items(&mut self, uri: &str, offset: usize) -> Vec<Json> {
        let mut out = Vec::new();
        if let Some((_, c, file)) = self.file_of(uri) {
            let prog = &c.program;
            // The locals of the innermost body around the position.
            let around = c
                .typed
                .bodies
                .iter()
                .filter(|b| contains(&b.block.span, file, offset))
                .min_by_key(|b| b.block.span.range.end - b.block.span.range.start);
            if let Some(body) = around {
                for l in &body.locals {
                    if l.span.file == file
                        && (l.span.range.end as usize) <= offset
                        && !l.name.starts_with("__")
                        && l.name.chars().next().is_some_and(|ch| ch.is_alphabetic() || ch == '_')
                    {
                        let kw = if l.mutable { "var" } else { "let" };
                        out.push(item(
                            &l.name,
                            kind::VARIABLE,
                            format!("{kw} {}: {}", l.name, prog.display(&l.ty)),
                            None,
                        ));
                    }
                }
            }
            // The module's own names, then the prelude's.
            let module = prog.modules.iter().position(|m| m.file == file);
            for m in module.into_iter().chain([prog.prelude.0 as usize]) {
                let mut names: Vec<(&String, &DefId)> = prog.modules[m].items.iter().collect();
                names.sort();
                for (n, d) in names {
                    if m == prog.prelude.0 as usize && prog.def(*d).vis == sisu_syntax::ast::Vis::Private {
                        continue;
                    }
                    if let Some(i) = def_item(prog, &c.typed, n, *d) {
                        out.push(i);
                    }
                }
            }
            // The variants, by their bare names.
            let mut vs: Vec<(&String, &Vec<(DefId, u32)>)> = prog.variants.iter().collect();
            vs.sort();
            for (n, ds) in vs {
                if let Some((d, _)) = ds.first()
                    && !prog.modules[prog.def(*d).module.0 as usize].is_std
                {
                    out.push(item(n, kind::ENUM_MEMBER, prog.def(*d).name.clone(), None));
                }
            }
        }
        let word = crate::cli::text(msg::LSP_KEYWORD, &[]);
        out.extend(KEYWORDS.iter().map(|k| item(k, kind::KEYWORD, word.clone(), None)));
        out
    }

    /// The occurrences of what the position names in the program's own
    /// files (not the standard library's): `None` when it names nothing
    /// that can be renamed. Each is a span of exactly the name.
    fn occurrences(&mut self, uri: &str, offset: usize) -> Option<(String, Vec<Span>)> {
        let (a, c, _) = self.file_of(uri)?;
        let target = self.target_at(&a, offset)?;
        let prog = &c.program;
        // The program's own files come first in its source map.
        let modules = self.db.program(&a.root).modules.len();
        let user = |f: FileId| (f.0 as usize) < modules && !c.map.name(f).starts_with('<');
        let named = |s: Span, name: &str| c.map.slice(s) == name;
        let mut out: Vec<Span> = Vec::new();
        let name = match &target {
            Target::Local(bi, li) => {
                let body = c.typed.bodies.get(*bi)?;
                let l = body.locals.get(*li)?;
                if !user(l.span.file) || !named(l.span, &l.name) {
                    return None;
                }
                out.push(l.span);
                let li = *li as u32;
                let mut visit = |e: &Expr| {
                    if matches!(e.kind, ExprKind::Local(id) if id.0 == li) && named(e.span, &l.name) {
                        out.push(e.span);
                    }
                };
                walk_block(&body.block, &mut visit);
                for cl in &body.closures {
                    walk_block(&cl.body, &mut visit);
                }
                l.name.clone()
            }
            Target::Def(d) => {
                let def = prog.def(*d);
                if !user(def.span.file) || !named(def.span, &def.name) {
                    return None;
                }
                let name = def.name.clone();
                // Every token of the program's files spelling the name that
                // means this definition where it is written, or, for a
                // type, that is written in a type.
                let typeish = matches!(
                    def.kind,
                    DefKind::Struct(_)
                        | DefKind::Enum(_)
                        | DefKind::ErrorSet(_)
                        | DefKind::Trait(_)
                        | DefKind::TypeAlias(_)
                        | DefKind::Refinement(_)
                );
                let files: Vec<(FileId, String)> =
                    c.map.files().iter().filter(|f| user(f.id)).map(|f| (f.id, f.name.clone())).collect();
                for (f, fname) in files {
                    let Some(ftext) = c.map.get(f).map(|x| x.text.clone()) else { continue };
                    let parse = sisu_parser::parse(f, &ftext);
                    let tree = &parse.tree;
                    for t in tree.tokens() {
                        if t.kind != SyntaxKind::IDENT
                            || ftext.get(t.range.start as usize..t.range.end as usize) != Some(name.as_str())
                        {
                            continue;
                        }
                        let span = Span::new(f, t.range);
                        let means = if fname == a.name {
                            self.target_at(&a, t.range.start as usize) == Some(Target::Def(*d))
                        } else {
                            target_in(&c, f, t.range.start as usize) == Some(Target::Def(*d))
                        };
                        let in_type = typeish && token_in_type(tree, t.range.start);
                        if means || in_type {
                            out.push(span);
                        }
                    }
                }
                name
            }
            Target::Field(d, k) => {
                let DefKind::Struct(s) = &prog.def(*d).kind else { return None };
                let f = s.fields.get(*k as usize)?;
                if !user(f.span.file) {
                    return None;
                }
                let name = f.name.clone();
                let decl = c.map.slice(f.span).find(&name).map(|i| {
                    let start = f.span.range.start + i as u32;
                    Span::new(f.span.file, sisu_diag::TextRange::new(start, start + name.len() as u32))
                })?;
                out.push(decl);
                for body in c.typed.bodies.iter().filter(|b| user(b.span.file)) {
                    let mut visit = |e: &Expr| match &e.kind {
                        ExprKind::Field { base, index } if *index == *k => {
                            if matches!(strip_refs(&base.ty), Ty::Adt(x, _) if x == d) {
                                // The name after the last `.`.
                                let written = c.map.slice(e.span);
                                if written.ends_with(&name) {
                                    let end = e.span.range.end;
                                    out.push(Span::new(
                                        e.span.file,
                                        sisu_diag::TextRange::new(end - name.len() as u32, end),
                                    ));
                                }
                            }
                        }
                        ExprKind::Struct { def, .. } if def == d => {
                            // `Point(x=1)`: the keyword before `=`.
                            let written = c.map.slice(e.span);
                            let mut from = 0;
                            while let Some(i) = written[from..].find(&format!("{name}=")) {
                                let at = from + i;
                                let ok = at == 0 || !is_name_byte(written.as_bytes()[at - 1]);
                                if ok {
                                    let start = e.span.range.start + at as u32;
                                    out.push(Span::new(
                                        e.span.file,
                                        sisu_diag::TextRange::new(start, start + name.len() as u32),
                                    ));
                                }
                                from = at + name.len();
                            }
                        }
                        _ => {}
                    };
                    walk_block(&body.block, &mut visit);
                    for cl in &body.closures {
                        walk_block(&cl.body, &mut visit);
                    }
                }
                name
            }
            Target::Expr(..) => return None,
        };
        out.sort();
        out.dedup();
        Some((name, out))
    }

    /// `textDocument/prepareRename`: the name at the position and its
    /// range, when it can be renamed.
    pub(crate) fn prepare_rename(&mut self, uri: &str, pos: &Json) -> Json {
        let text = self.text_of(uri);
        let lines = Lines::new(&text, self.utf8);
        let offset = lines.offset(pos);
        let Some((name, spans)) = self.occurrences(uri, offset) else { return Json::Null };
        let Some((_, _, file)) = self.file_of(uri) else { return Json::Null };
        match spans.iter().find(|s| contains(s, file, offset)) {
            Some(s) => Json::obj(vec![
                ("range", lines.range(s.range.start as usize, s.range.end as usize)),
                ("placeholder", Json::str(name)),
            ]),
            None => Json::Null,
        }
    }

    /// `textDocument/rename`: every occurrence in the program's own files
    /// as one workspace edit, or why not.
    pub(crate) fn rename(&mut self, uri: &str, pos: &Json, new_name: &str) -> Result<Json, String> {
        let valid = new_name.chars().next().is_some_and(|c| c.is_alphabetic() || c == '_')
            && new_name.chars().all(|c| c.is_alphanumeric() || c == '_')
            && !KEYWORDS.contains(&new_name);
        if !valid {
            return Err(crate::cli::text(msg::LSP_NOT_A_NAME, &[("name", new_name.to_string())]));
        }
        let text = self.text_of(uri);
        let offset = Lines::new(&text, self.utf8).offset(pos);
        let Some((_, spans)) = self.occurrences(uri, offset) else {
            return Err(crate::cli::text(msg::LSP_CANNOT_RENAME, &[]));
        };
        let Some((a, c, _)) = self.file_of(uri) else {
            return Err(crate::cli::text(msg::LSP_NOT_ANALYSED, &[]));
        };
        let edits: Vec<(Span, String)> = spans.into_iter().map(|s| (s, new_name.to_string())).collect();
        Ok(self.workspace_edit(uri, &a, &c.map, &edits))
    }

    /// `textDocument/references`: the occurrences rename would change, as
    /// locations (the declaration included, as the protocol's
    /// `includeDeclaration` asks by default).
    pub(crate) fn references(&mut self, uri: &str, pos: &Json) -> Json {
        let text = self.text_of(uri);
        let offset = Lines::new(&text, self.utf8).offset(pos);
        let Some((_, spans)) = self.occurrences(uri, offset) else { return Json::Arr(Vec::new()) };
        let Some((a, c, _)) = self.file_of(uri) else { return Json::Arr(Vec::new()) };
        let edits: Vec<(Span, String)> = spans.into_iter().map(|s| (s, String::new())).collect();
        let by_file = self.workspace_edit(uri, &a, &c.map, &edits);
        let mut out = Vec::new();
        if let Some(Json::Obj(files)) = by_file.get("changes") {
            for (file_uri, edits) in files {
                for e in edits.as_array().unwrap_or(&[]) {
                    out.push(Json::obj(vec![
                        ("uri", Json::str(file_uri.clone())),
                        ("range", e.get("range").cloned().unwrap_or(Json::Null)),
                    ]));
                }
            }
        }
        Json::Arr(out)
    }

    /// Edits as a workspace edit: each file's by its URI (the document's
    /// own, or a module of its package on disk).
    fn workspace_edit(
        &self,
        uri: &str,
        a: &crate::lsp::Analysis,
        map: &sisu_diag::SourceMap,
        edits: &[(Span, String)],
    ) -> Json {
        let mut by_file: BTreeMap<String, Vec<Json>> = BTreeMap::new();
        for (span, new_text) in edits {
            let Some(f) = map.get(span.file) else { continue };
            let file_uri = if f.name == a.name {
                uri.to_string()
            } else {
                let base = crate::package::locate(None, uri_path(uri).parent().unwrap_or(Path::new(".")))
                    .and_then(|t| match t {
                        crate::package::Target::Package(d) => Some(d),
                        _ => None,
                    })
                    .unwrap_or_else(|| uri_path(uri).parent().map(Path::to_path_buf).unwrap_or_default());
                path_uri(&base.join(&f.name))
            };
            // The open document's text is the one edited; another file's
            // as the program saw it.
            let open = self.docs.get(&file_uri).map(|d| d.text.clone());
            let text = open.unwrap_or_else(|| f.text.clone());
            let lines = Lines::new(&text, self.utf8);
            by_file.entry(file_uri).or_default().push(Json::obj(vec![
                ("range", lines.range(span.range.start as usize, span.range.end as usize)),
                ("newText", Json::str(new_text.clone())),
            ]));
        }
        Json::obj(vec![("changes", Json::Obj(by_file.into_iter().map(|(k, v)| (k, Json::Arr(v))).collect()))])
    }

    /// `textDocument/codeAction`: the fixes of the diagnostics over the
    /// range, each a quick fix whose title is the catalog's fix message
    /// (each diagnostic names its fix, 10, "Diagnostics").
    pub(crate) fn code_actions(&mut self, uri: &str, range: &Json) -> Json {
        let text = self.text_of(uri);
        let lines = Lines::new(&text, self.utf8);
        let from = range.get("start").map(|p| lines.offset(p)).unwrap_or(0);
        let to = range.get("end").map(|p| lines.offset(p)).unwrap_or(text.len());
        let Some((a, c, file)) = self.file_of(uri) else { return Json::Arr(Vec::new()) };
        let diags = self.db.diagnostics(&a.root);
        let mut out = Vec::new();
        for d in diags.iter().filter(|d| d.span.file == file) {
            let (s, e) = (d.span.range.start as usize, d.span.range.end as usize);
            if e < from || s > to {
                continue;
            }
            for (k, fix) in d.fixes.iter().enumerate() {
                if fix.edits.is_empty() {
                    continue;
                }
                let title = sisu_diag::render::fix_text(fix);
                let edits: Vec<(Span, String)> = fix.edits.iter().map(|e| (e.span, e.text.clone())).collect();
                out.push(Json::obj(vec![
                    ("title", Json::str(title)),
                    ("kind", Json::str("quickfix")),
                    ("isPreferred", Json::Bool(k == 0)),
                    ("diagnostics", Json::Arr(vec![crate::lsp::diagnostic_json(d, &lines)])),
                    ("edit", self.workspace_edit(uri, &a, &c.map, &edits)),
                ]));
            }
        }
        Json::Arr(out)
    }

    /// `textDocument/inlayHint`: over the range, the type of each local
    /// whose declaration does not write it, the error set a `T !` function
    /// infers, and `mut` or `own` before an argument passed to such a
    /// parameter (10, "Language server").
    pub(crate) fn inlay_hints(&mut self, uri: &str, range: &Json) -> Json {
        let text = self.text_of(uri);
        let lines = Lines::new(&text, self.utf8);
        let from = range.get("start").map(|p| lines.offset(p)).unwrap_or(0);
        let to = range.get("end").map(|p| lines.offset(p)).unwrap_or(text.len());
        let Some((_, c, file)) = self.file_of(uri) else { return Json::Arr(Vec::new()) };
        let prog = &c.program;
        let mut hints: Vec<(usize, Json)> = Vec::new();
        let hint = |at: usize, label: String, kind: f64, left: bool, right: bool| {
            Json::obj(vec![
                ("position", lines.position(at)),
                ("label", Json::str(label)),
                ("kind", Json::num(kind)),
                ("paddingLeft", Json::Bool(left)),
                ("paddingRight", Json::Bool(right)),
            ])
        };
        let in_range = |at: usize| from <= at && at <= to;
        for body in c.typed.bodies.iter().filter(|b| b.span.file == file) {
            for (i, l) in body.locals.iter().enumerate() {
                let at = l.span.range.end as usize;
                if l.span.file != file
                    || !in_range(at)
                    || body.params.iter().any(|p| p.0 as usize == i)
                    || l.name.starts_with('_')
                    || c.map.slice(l.span) != l.name
                    || l.ty.contains_error()
                {
                    continue;
                }
                // A declaration that writes its type needs no hint.
                let after = text.get(at..).unwrap_or("").trim_start_matches([' ', '\t']);
                if after.starts_with(':') {
                    continue;
                }
                hints.push((at, hint(at, format!(": {}", prog.display(&l.ty)), 1.0, false, false)));
            }
            // `mut` and `own` at the arguments that take them.
            let mut visit = |e: &Expr| {
                let ExprKind::Call { callee: Callee::Fn { def, .. }, args } = &e.kind else { return };
                let Some(f) = prog.fn_info(*def) else { return };
                let skip = usize::from(f.self_mode.is_some());
                for (a, p) in args.iter().skip(skip).zip(&f.params) {
                    let word = match (p.mode, a.mode) {
                        (sisu_syntax::ast::Mode::Mut, ArgMode::BorrowMut) => "mut",
                        (sisu_syntax::ast::Mode::Own, _) if !a.expr.ty.contains_error() => "own",
                        _ => continue,
                    };
                    let at = a.expr.span.range.start as usize;
                    if a.expr.span.file == file && in_range(at) && !text[..at].trim_end().ends_with(word) {
                        hints.push((at, hint(at, word.to_string(), 2.0, false, true)));
                    }
                }
            };
            walk_block(&body.block, &mut visit);
            for cl in &body.closures {
                walk_block(&cl.body, &mut visit);
            }
        }
        // The error set each `T !` function of the document infers.
        for (i, def) in prog.defs.iter().enumerate() {
            let DefKind::Fn(f) = &def.kind else { continue };
            if !f.err_inferred || def.span.file != file {
                continue;
            }
            let Some(ret) = f.ret_span else { continue };
            let at = ret.range.end as usize;
            if !in_range(at) {
                continue;
            }
            let set: Vec<String> = c
                .typed
                .inferred
                .get(&DefId(i as u32))
                .map(|s| s.iter().map(|e| prog.def(*e).name.clone()).collect())
                .unwrap_or_default();
            let label = if set.is_empty() { "{}".to_string() } else { format!("{{{}}}", set.join(", ")) };
            hints.push((at, hint(at, label, 1.0, true, false)));
        }
        hints.sort_by_key(|(at, _)| *at);
        Json::Arr(hints.into_iter().map(|(_, h)| h).collect())
    }

    /// `workspace/executeCommand` of `sisu.explainBorrow` at a position:
    /// every loan live on its line, each with its kind, the place it
    /// borrows, where it starts and is last used, and the locals holding it
    /// (the shape N-28's permissions overlay reads), with the line the CLI
    /// prints for it. A program the front end rejects has no loans to show.
    pub(crate) fn explain_borrow(&mut self, args: &Json) -> Result<Json, String> {
        let first = args.as_array().and_then(|a| a.first()).cloned().unwrap_or(Json::Null);
        let uri = first.at(&["textDocument", "uri"]).and_then(Json::as_str).unwrap_or("").to_string();
        if !self.docs.contains_key(&uri) {
            return Err(crate::cli::text(msg::LSP_NOT_OPEN, &[]));
        }
        let text = self.text_of(&uri);
        let lines = Lines::new(&text, self.utf8);
        let offset = lines.offset(first.get("position").unwrap_or(&Json::Null));
        let line = text[..offset.min(text.len())].matches('\n').count() as u32 + 1;
        let Some((a, c, file)) = self.file_of(&uri) else {
            return Err(crate::cli::text(msg::LSP_NOT_ANALYSED, &[]));
        };
        if c.has_errors() {
            return Err(crate::cli::text(msg::LSP_HAS_ERRORS, &[]));
        }
        let Some(lowered) = self.db.lowered(&a.root) else {
            return Err(crate::cli::text(msg::LSP_HAS_ERRORS, &[]));
        };
        let files = crate::db::source_files(&c.map, &lowered.sir);
        let Some(source) = files.iter().position(|f| *f == file) else {
            return Ok(Json::obj(vec![("line", Json::num(line)), ("loans", Json::Arr(Vec::new()))]));
        };
        let range_of = |loc: Option<sisu_sir::Loc>| -> Json {
            let Some(l) = loc else { return Json::Null };
            let Some(f) = c.map.get(file) else { return Json::Null };
            let s = f.offset(LineCol { line: l.line, col: l.col }) as usize;
            let e =
                if l.has_end() { f.offset(LineCol { line: l.end_line, col: l.end_col }) as usize } else { s };
            lines.range(s, e)
        };
        let mut loans = Vec::new();
        for fl in sisu_borrowck::explain::live_at(&lowered.sir, source as u32, line) {
            for l in &fl.loans {
                let place = match &l.place {
                    sisu_borrowck::Arg::Text(t) => t.clone(),
                    sisu_borrowck::Arg::Source(loc) => {
                        c.map.slice(sisu_borrowck::span_of(&c.map, file, Some(*loc))).to_string()
                    }
                };
                let pos = |loc: Option<sisu_sir::Loc>| {
                    loc.map(|l| format!("{}:{}", l.line, l.col)).unwrap_or_default()
                };
                let kind = crate::cli::text(
                    if l.mutable {
                        sisu_diag::msg::CLI_EXPLAIN_BORROW_MUT
                    } else {
                        sisu_diag::msg::CLI_EXPLAIN_BORROW_SHARED
                    },
                    &[],
                );
                let holders: Vec<String> = l.holders.iter().map(|h| format!("`{h}`")).collect();
                let m = if holders.is_empty() {
                    sisu_diag::msg::CLI_EXPLAIN_BORROW_TEMPORARY
                } else {
                    sisu_diag::msg::CLI_EXPLAIN_BORROW_LOAN
                };
                let message = crate::cli::text(
                    m,
                    &[
                        ("kind", kind),
                        ("place", place.clone()),
                        ("start", pos(l.start)),
                        ("holders", holders.join(", ")),
                        ("end", pos(l.until)),
                    ],
                );
                loans.push(Json::obj(vec![
                    ("function", Json::str(fl.function.clone())),
                    ("mutable", Json::Bool(l.mutable)),
                    ("place", Json::str(place)),
                    ("start", range_of(l.start)),
                    ("until", range_of(l.until)),
                    ("holders", Json::Arr(l.holders.iter().map(|h| Json::str(h.clone())).collect())),
                    ("message", Json::str(message)),
                ]));
            }
        }
        Ok(Json::obj(vec![("line", Json::num(line)), ("loans", Json::Arr(loans))]))
    }

    /// `textDocument/rangeFormatting`: the formatter's changes to the lines
    /// the range touches, each run of changed lines one edit (`sisu fmt`
    /// formats whole files; a run that reaches outside the range is left
    /// alone). A document with a syntax error is not formatted.
    pub(crate) fn range_formatting(&mut self, uri: &str, range: &Json) -> Json {
        let text = self.text_of(uri);
        let Ok(formatted) = self.format_document(uri, &text) else { return Json::Null };
        let lines = Lines::new(&text, self.utf8);
        let first = range.at(&["start", "line"]).and_then(Json::as_u64).unwrap_or(0) as usize;
        let last = range.at(&["end", "line"]).and_then(Json::as_u64).unwrap_or(u64::MAX) as usize;
        let old: Vec<&str> = text.split_inclusive('\n').collect();
        let new: Vec<&str> = formatted.split_inclusive('\n').collect();
        let starts: Vec<usize> = old
            .iter()
            .scan(0, |at, l| {
                let s = *at;
                *at += l.len();
                Some(s)
            })
            .collect();
        let offset_of = |i: usize| starts.get(i).copied().unwrap_or(text.len());
        let mut out = Vec::new();
        for (a0, a1, b0, b1) in changed_runs(&old, &new) {
            // The run's lines in the document: `a0..a1` (empty for an
            // insertion before line `a0`).
            let touches = a0 <= last && (a1.max(a0 + 1) - 1) >= first;
            let inside = a0 >= first && a1 <= last + 1;
            if touches && inside {
                out.push(Json::obj(vec![
                    ("range", lines.range(offset_of(a0), offset_of(a1))),
                    ("newText", Json::str(new[b0..b1].concat())),
                ]));
            }
        }
        Json::Arr(out)
    }

    /// `textDocument/inlineValue`: for a worksheet (a script whose
    /// top-level statements run, Q-48), each statement's value and what it
    /// printed, at the end of its last line, as `sisu ws` writes them; run
    /// against a test's fake `System`, so nothing reaches the world. A
    /// document that is not a script, or does not check, has none.
    pub(crate) fn inline_values(&mut self, uri: &str) -> Json {
        let text = self.text_of(uri);
        let parse = sisu_parser::parse(FileId(0), &text);
        let module = sisu_syntax::ast::build::module(&parse.tree);
        let script = module.items.iter().any(|i| matches!(i.kind, sisu_syntax::ast::ItemKind::Stmt(_)));
        if !script {
            return Json::Arr(Vec::new());
        }
        let key = {
            use std::hash::{Hash, Hasher};
            let mut h = std::hash::DefaultHasher::new();
            (uri, &text).hash(&mut h);
            h.finish()
        };
        if let Some((k, v)) = &self.sheet
            && *k == key
        {
            return v.clone();
        }
        let path = uri_path(uri);
        let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
        let mut host = sisu_interp::TestHost::default();
        let lines = Lines::new(&text, self.utf8);
        let mut out = Vec::new();
        // Fakes only, and a budget of statements, so that a loop that never
        // ends cannot hold the server (10, "always-on evaluation").
        if let Ok(sheet) = crate::repl::evaluate_sheet(
            &path.display().to_string(),
            &text,
            &dir,
            false,
            Some(SHEET_FUEL),
            &crate::repl::Checker::Rust,
            &mut host,
        ) {
            let ends: Vec<usize> = text
                .split_inclusive('\n')
                .scan(0, |at, l| {
                    *at += l.len();
                    Some(*at - l.len() + l.trim_end_matches(['\n', '\r']).len())
                })
                .collect();
            let mut lines_with: Vec<usize> =
                sheet.values.keys().chain(sheet.outputs.keys()).copied().collect();
            lines_with.sort();
            lines_with.dedup();
            for n in lines_with {
                let Some(&end) = ends.get(n - 1) else { continue };
                let mut parts = Vec::new();
                if let Some(vs) = sheet.values.get(&n) {
                    parts.push(crate::cli::text(sisu_diag::msg::CLI_WS_VALUE, &[("value", vs.join(", "))]));
                }
                for o in sheet.outputs.get(&n).into_iter().flatten() {
                    parts.push(crate::cli::text(sisu_diag::msg::CLI_WS_OUTPUT, &[("text", o.clone())]));
                }
                out.push(Json::obj(vec![
                    ("range", lines.range(end, end)),
                    ("text", Json::str(parts.join("  "))),
                ]));
            }
        }
        let v = Json::Arr(out);
        self.sheet = Some((key, v.clone()));
        v
    }
}

/// What a position of another file of the program names: a definition's
/// name, or what the expression there calls or names.
fn target_in(c: &crate::db::Checked, file: FileId, offset: usize) -> Option<Target> {
    for (i, def) in c.program.defs.iter().enumerate() {
        if contains(&def.span, file, offset) {
            return Some(Target::Def(DefId(i as u32)));
        }
    }
    let (e, _) = Server::expr_at(&c.typed, file, offset)?;
    Some(match &e.kind {
        ExprKind::Const(d) | ExprKind::FnRef(d, _) => Target::Def(*d),
        ExprKind::Call { callee: Callee::Fn { def, .. }, .. } => Target::Def(*def),
        ExprKind::Struct { def, .. } | ExprKind::Variant { def, .. } => Target::Def(*def),
        _ => return None,
    })
}

/// Whether the token at `at` is written in a type.
fn token_in_type(tree: &sisu_syntax::Tree, at: u32) -> bool {
    use SyntaxKind::*;
    let mut node = Some(tree.root());
    let mut found = false;
    while let Some(n) = node {
        if matches!(
            tree.kind(n),
            PATH_TYPE
                | OPTION_TYPE
                | RESULT_TYPE
                | ERROR_SET_TYPE
                | TUPLE_TYPE
                | PAREN_TYPE
                | FN_TYPE
                | REF_TYPE
                | DYN_TYPE
        ) {
            found = true;
        }
        node = tree.child_nodes(n).find(|&c| {
            let r = tree.range(c);
            r.start <= at && at < r.end
        });
    }
    found
}

/// The runs of lines that differ between `old` and `new`, as `(old start,
/// old end, new start, new end)`, from the longest common subsequence of
/// the lines both do not start and end with.
fn changed_runs(old: &[&str], new: &[&str]) -> Vec<(usize, usize, usize, usize)> {
    let prefix = old.iter().zip(new).take_while(|(x, y)| x == y).count();
    let suffix =
        old[prefix..].iter().rev().zip(new[prefix..].iter().rev()).take_while(|(x, y)| x == y).count();
    let a = &old[prefix..old.len() - suffix];
    let b = &new[prefix..new.len() - suffix];
    let mut lcs = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            lcs[i][j] = if a[i] == b[j] { lcs[i + 1][j + 1] + 1 } else { lcs[i + 1][j].max(lcs[i][j + 1]) };
        }
    }
    let mut runs = Vec::new();
    let (mut i, mut j) = (0, 0);
    let mut open: Option<(usize, usize)> = None;
    while i < a.len() || j < b.len() {
        if i < a.len() && j < b.len() && a[i] == b[j] {
            if let Some((s, t)) = open.take() {
                runs.push((prefix + s, prefix + i, prefix + t, prefix + j));
            }
            i += 1;
            j += 1;
            continue;
        }
        if open.is_none() {
            open = Some((i, j));
        }
        if i < a.len() && (j == b.len() || lcs[i + 1][j] >= lcs[i][j + 1]) {
            i += 1;
        } else {
            j += 1;
        }
    }
    if let Some((s, t)) = open {
        runs.push((prefix + s, prefix + i, prefix + t, prefix + j));
    }
    runs
}
