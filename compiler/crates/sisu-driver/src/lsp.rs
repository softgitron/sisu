//! `sisu lsp` (10, "Language server"): the language server, over standard
//! input and output (JSON-RPC 2.0 with `Content-Length` framing, LSP 3.17).
//!
//! It is the same front end as the compiler: every answer comes from the
//! queries of [`Db`], which `sisu check` and `sisu run` use, so the three
//! never disagree, and the checker types broken code, so hover and
//! go-to-definition keep working in it. A document is analysed as part
//! of its program: the package whose `src/` holds it (the open documents'
//! texts standing in for the files on disk), or the file alone.
//!
//! It provides diagnostics (published on every change, each with its
//! catalog code and identifier), hover (an expression's type; a
//! function's signature with its inferred error set, the capabilities it
//! takes and its contracts; a type's fields; the documentation), go-to-
//! definition (into the standard library's source too), document
//! symbols, selection ranges (the syntax tree's nodes around a position)
//! and formatting (`sisu fmt`). Documents are synchronised whole. Positions
//! are UTF-16 code units unless the client offers UTF-8.

use crate::db::{Db, ProgramSource};
use crate::json::Json;
use sisu_diag::{FileId, Severity, Span};
use sisu_resolve::{DefId, DefKind, Program, Ty};
use sisu_syntax::SyntaxKind::{self, *};
use sisu_syntax::{Element, NodeId, Tree};
use sisu_typeck::{Body, Expr, ExprKind, Typed};
use std::collections::BTreeMap;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

/// The JSON-RPC error codes the server answers with.
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;
/// LSP's `RequestFailed`: a request that was understood but cannot be
/// answered (a rename of something that cannot be renamed).
const REQUEST_FAILED: i64 = -32803;

/// Where the standard library's source is on disk, for go-to-definition
/// into it.
pub(crate) fn std_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../std")
}

/// A `file://` URI's path.
pub fn uri_path(uri: &str) -> PathBuf {
    let rest = uri.strip_prefix("file://").unwrap_or(uri);
    let mut bytes = Vec::new();
    let b = rest.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && let Some(h) = rest.get(i + 1..i + 3).and_then(|h| u8::from_str_radix(h, 16).ok())
        {
            bytes.push(h);
            i += 3;
            continue;
        }
        bytes.push(b[i]);
        i += 1;
    }
    PathBuf::from(String::from_utf8_lossy(&bytes).into_owned())
}

/// A path's `file://` URI.
pub fn path_uri(path: &Path) -> String {
    let mut out = String::from("file://");
    for b in path.display().to_string().bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Line starts of a text, to turn byte offsets into protocol positions
/// and back.
pub(crate) struct Lines<'a> {
    pub(crate) text: &'a str,
    starts: Vec<usize>,
    utf8: bool,
}

impl<'a> Lines<'a> {
    pub(crate) fn new(text: &'a str, utf8: bool) -> Lines<'a> {
        let mut starts = vec![0];
        starts.extend(text.match_indices('\n').map(|(i, _)| i + 1));
        Lines { text, starts, utf8 }
    }

    pub(crate) fn position(&self, offset: usize) -> Json {
        let offset = offset.min(self.text.len());
        let line = self.starts.partition_point(|&s| s <= offset).saturating_sub(1);
        let start = self.starts[line];
        let before = self.text.get(start..offset).unwrap_or("");
        let character = if self.utf8 { before.len() } else { before.encode_utf16().count() };
        Json::obj(vec![("line", Json::num(line as f64)), ("character", Json::num(character as f64))])
    }

    pub(crate) fn range(&self, start: usize, end: usize) -> Json {
        Json::obj(vec![("start", self.position(start)), ("end", self.position(end))])
    }

    pub(crate) fn offset(&self, pos: &Json) -> usize {
        let line = pos.get("line").and_then(Json::as_u64).unwrap_or(0) as usize;
        let character = pos.get("character").and_then(Json::as_u64).unwrap_or(0) as usize;
        let Some(&start) = self.starts.get(line) else { return self.text.len() };
        let end = self.starts.get(line + 1).copied().unwrap_or(self.text.len());
        let line_text = &self.text[start..end];
        if self.utf8 {
            return start + character.min(line_text.len());
        }
        let mut units = 0;
        for (i, c) in line_text.char_indices() {
            if units >= character {
                return start + i;
            }
            units += c.len_utf16();
        }
        end
    }
}

/// An open document.
pub(crate) struct Doc {
    pub(crate) text: String,
    pub(crate) version: f64,
}

/// What a document was analysed as: its program's root module, and the
/// document's own name among the program's files.
pub(crate) struct Analysis {
    pub(crate) root: String,
    pub(crate) name: String,
}

pub struct Server {
    pub(crate) docs: BTreeMap<String, Doc>,
    pub(crate) db: Db,
    pub(crate) utf8: bool,
    shutdown: bool,
    /// Set by `exit`: the status the server ends with.
    pub exit: Option<i32>,
    /// The inline values of the last worksheet run, by a hash of its
    /// document and text.
    pub(crate) sheet: Option<(u64, Json)>,
}

impl Default for Server {
    fn default() -> Server {
        Server::new()
    }
}

pub(crate) fn respond(id: &Json, result: Json) -> Json {
    Json::obj(vec![("jsonrpc", Json::str("2.0")), ("id", id.clone()), ("result", result)])
}

pub(crate) fn fail(id: &Json, code: i64, message: &str) -> Json {
    let error = Json::obj(vec![("code", Json::num(code as f64)), ("message", Json::str(message))]);
    Json::obj(vec![("jsonrpc", Json::str("2.0")), ("id", id.clone()), ("error", error)])
}

fn notify(method: &str, params: Json) -> Json {
    Json::obj(vec![("jsonrpc", Json::str("2.0")), ("method", Json::str(method)), ("params", params)])
}

pub(crate) fn contains(span: &Span, file: FileId, offset: usize) -> bool {
    span.file == file && (span.range.start as usize) <= offset && offset <= span.range.end as usize
}

fn width(span: &Span) -> u32 {
    span.range.end - span.range.start
}

/// Whether a type is a capability, or holds one (06).
fn is_capability(prog: &Program, ty: &Ty) -> bool {
    let shown = prog.display(ty);
    ["System", "Fs", "Net", "Clock", "Rand", "Env", "Proc", "Stdin"]
        .iter()
        .any(|c| shown == *c || shown.starts_with(&format!("{c}[")) || shown.contains(&format!("ref {c}")))
}

impl Server {
    pub fn new() -> Server {
        Server { docs: BTreeMap::new(), db: Db::new(), utf8: false, shutdown: false, exit: None, sheet: None }
    }

    /// Answers one message: its response, and the notifications it causes.
    pub fn handle(&mut self, msg: &Json) -> Vec<Json> {
        let method = msg.get("method").and_then(Json::as_str).unwrap_or("");
        let id = msg.get("id").cloned();
        let params = msg.get("params").cloned().unwrap_or(Json::Null);
        let reply = |result: Json| id.as_ref().map(|i| respond(i, result));
        match method {
            "initialize" => {
                let offered =
                    params.at(&["capabilities", "general", "positionEncodings"]).and_then(Json::as_array);
                self.utf8 = offered.is_some_and(|e| e.iter().any(|x| x.as_str() == Some("utf-8")));
                let caps = Json::obj(vec![
                    ("positionEncoding", Json::str(if self.utf8 { "utf-8" } else { "utf-16" })),
                    ("textDocumentSync", Json::num(1)),
                    ("hoverProvider", Json::Bool(true)),
                    ("definitionProvider", Json::Bool(true)),
                    ("documentSymbolProvider", Json::Bool(true)),
                    ("selectionRangeProvider", Json::Bool(true)),
                    ("documentFormattingProvider", Json::Bool(true)),
                    ("documentRangeFormattingProvider", Json::Bool(true)),
                    (
                        "completionProvider",
                        Json::obj(vec![
                            ("triggerCharacters", Json::Arr(vec![Json::str(".")])),
                            ("resolveProvider", Json::Bool(false)),
                        ]),
                    ),
                    ("renameProvider", Json::obj(vec![("prepareProvider", Json::Bool(true))])),
                    ("referencesProvider", Json::Bool(true)),
                    (
                        "codeActionProvider",
                        Json::obj(vec![("codeActionKinds", Json::Arr(vec![Json::str("quickfix")]))]),
                    ),
                    ("inlayHintProvider", Json::Bool(true)),
                    ("inlineValueProvider", Json::Bool(true)),
                    (
                        "executeCommandProvider",
                        Json::obj(vec![(
                            "commands",
                            Json::Arr(vec![Json::str(crate::assist::EXPLAIN_BORROW)]),
                        )]),
                    ),
                ]);
                let info = Json::obj(vec![
                    ("name", Json::str("sisu")),
                    ("version", Json::str(crate::replay::TOOLCHAIN)),
                ]);
                reply(Json::obj(vec![("capabilities", caps), ("serverInfo", info)])).into_iter().collect()
            }
            "initialized"
            | "$/cancelRequest"
            | "textDocument/didSave"
            | "workspace/didChangeConfiguration" => Vec::new(),
            "shutdown" => {
                self.shutdown = true;
                reply(Json::Null).into_iter().collect()
            }
            "exit" => {
                self.exit = Some(if self.shutdown { 0 } else { 1 });
                Vec::new()
            }
            "textDocument/didOpen" => {
                let uri =
                    params.at(&["textDocument", "uri"]).and_then(Json::as_str).unwrap_or("").to_string();
                let text =
                    params.at(&["textDocument", "text"]).and_then(Json::as_str).unwrap_or("").to_string();
                let version = match params.at(&["textDocument", "version"]) {
                    Some(Json::Num(n)) => *n,
                    _ => 0.0,
                };
                self.docs.insert(uri.clone(), Doc { text, version });
                vec![self.publish(&uri)]
            }
            "textDocument/didChange" => {
                let uri =
                    params.at(&["textDocument", "uri"]).and_then(Json::as_str).unwrap_or("").to_string();
                let version = match params.at(&["textDocument", "version"]) {
                    Some(Json::Num(n)) => *n,
                    _ => 0.0,
                };
                // Whole-document synchronisation: the last change is the text.
                let text = params
                    .get("contentChanges")
                    .and_then(Json::as_array)
                    .and_then(|c| c.last())
                    .and_then(|c| c.get("text"))
                    .and_then(Json::as_str)
                    .map(str::to_string);
                if let (Some(text), Some(doc)) = (text, self.docs.get_mut(&uri)) {
                    doc.text = text;
                    doc.version = version;
                }
                vec![self.publish(&uri)]
            }
            "textDocument/didClose" => {
                let uri =
                    params.at(&["textDocument", "uri"]).and_then(Json::as_str).unwrap_or("").to_string();
                self.docs.remove(&uri);
                let p = Json::obj(vec![("uri", Json::str(uri)), ("diagnostics", Json::Arr(Vec::new()))]);
                vec![notify("textDocument/publishDiagnostics", p)]
            }
            "workspace/executeCommand" => {
                let Some(id) = id else { return Vec::new() };
                let command = params.get("command").and_then(Json::as_str).unwrap_or("");
                if command != crate::assist::EXPLAIN_BORROW {
                    let m = crate::cli::text(
                        sisu_diag::msg::LSP_NOT_A_COMMAND,
                        &[("command", command.to_string())],
                    );
                    return vec![fail(&id, INVALID_PARAMS, &m)];
                }
                match self.explain_borrow(params.get("arguments").unwrap_or(&Json::Null)) {
                    Ok(r) => vec![respond(&id, r)],
                    Err(m) => vec![fail(&id, REQUEST_FAILED, &m)],
                }
            }
            "textDocument/rename" => {
                let Some(id) = id else { return Vec::new() };
                let uri =
                    params.at(&["textDocument", "uri"]).and_then(Json::as_str).unwrap_or("").to_string();
                if !self.docs.contains_key(&uri) {
                    return vec![fail(
                        &id,
                        INVALID_PARAMS,
                        &crate::cli::text(sisu_diag::msg::LSP_NOT_OPEN, &[]),
                    )];
                }
                let new_name = params.get("newName").and_then(Json::as_str).unwrap_or("").to_string();
                match self.rename(&uri, params.get("position").unwrap_or(&Json::Null), &new_name) {
                    Ok(r) => vec![respond(&id, r)],
                    Err(m) => vec![fail(&id, REQUEST_FAILED, &m)],
                }
            }
            "textDocument/hover"
            | "textDocument/definition"
            | "textDocument/documentSymbol"
            | "textDocument/selectionRange"
            | "textDocument/formatting"
            | "textDocument/rangeFormatting"
            | "textDocument/completion"
            | "textDocument/prepareRename"
            | "textDocument/references"
            | "textDocument/codeAction"
            | "textDocument/inlayHint"
            | "textDocument/inlineValue" => {
                let Some(id) = id else { return Vec::new() };
                let uri =
                    params.at(&["textDocument", "uri"]).and_then(Json::as_str).unwrap_or("").to_string();
                if !self.docs.contains_key(&uri) {
                    return vec![fail(
                        &id,
                        INVALID_PARAMS,
                        &crate::cli::text(sisu_diag::msg::LSP_NOT_OPEN, &[]),
                    )];
                }
                let result = match method {
                    "textDocument/hover" => self.hover(&uri, params.get("position").unwrap_or(&Json::Null)),
                    "textDocument/definition" => {
                        self.definition(&uri, params.get("position").unwrap_or(&Json::Null))
                    }
                    "textDocument/documentSymbol" => self.symbols(&uri),
                    "textDocument/selectionRange" => {
                        let ps = params.get("positions").and_then(Json::as_array).unwrap_or(&[]).to_vec();
                        self.selection_ranges(&uri, &ps)
                    }
                    "textDocument/rangeFormatting" => {
                        self.range_formatting(&uri, params.get("range").unwrap_or(&Json::Null))
                    }
                    "textDocument/completion" => {
                        self.completion(&uri, params.get("position").unwrap_or(&Json::Null))
                    }
                    "textDocument/prepareRename" => {
                        self.prepare_rename(&uri, params.get("position").unwrap_or(&Json::Null))
                    }
                    "textDocument/references" => {
                        self.references(&uri, params.get("position").unwrap_or(&Json::Null))
                    }
                    "textDocument/codeAction" => {
                        self.code_actions(&uri, params.get("range").unwrap_or(&Json::Null))
                    }
                    "textDocument/inlayHint" => {
                        self.inlay_hints(&uri, params.get("range").unwrap_or(&Json::Null))
                    }
                    "textDocument/inlineValue" => self.inline_values(&uri),
                    _ => self.formatting(&uri),
                };
                vec![respond(&id, result)]
            }
            _ => match id {
                Some(id) => vec![fail(
                    &id,
                    METHOD_NOT_FOUND,
                    &crate::cli::text(sisu_diag::msg::LSP_NOT_SERVED, &[("method", method.to_string())]),
                )],
                None => Vec::new(),
            },
        }
    }

    /// Puts a document's program in the database and names its root
    /// module and the document's file.
    pub(crate) fn analyse(&mut self, uri: &str) -> Analysis {
        let path = uri_path(uri);
        let text = self.docs.get(uri).map(|d| d.text.clone()).unwrap_or_default();
        let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
        // The texts of the open documents stand in for their files.
        let open: BTreeMap<PathBuf, String> =
            self.docs.iter().map(|(u, d)| (uri_path(u), d.text.clone())).collect();
        if let Some(crate::package::Target::Package(pkg)) = crate::package::locate(None, &dir) {
            let layout = crate::package::load_package(&pkg, &pkg);
            let canon = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
            let me = canon(&path);
            if let Some(m) = layout.modules.iter().find(|m| canon(&m.disk) == me)
                && !layout.modules.is_empty()
            {
                let name = m.name.clone();
                for module in &layout.modules {
                    let t = open
                        .get(&module.disk)
                        .cloned()
                        .or_else(|| std::fs::read_to_string(&module.disk).ok())
                        .unwrap_or_default();
                    let t = if canon(&module.disk) == me { text.clone() } else { t };
                    self.db.set_file(&module.name, &t);
                }
                let root = layout.modules[0].name.clone();
                let mut program = ProgramSource::from_layout(&layout);
                program.base = layout.base.clone();
                self.db.set_program(&root, program);
                return Analysis { root, name };
            }
        }
        let name = path.display().to_string();
        let layout = crate::package::load_file(&name, &text, &dir);
        self.db.set_file(&name, &text);
        if layout.packages.len() > 1 {
            for module in layout.modules.iter().skip(1) {
                let t = open
                    .get(&module.disk)
                    .cloned()
                    .or_else(|| std::fs::read_to_string(&module.disk).ok())
                    .unwrap_or_default();
                self.db.set_file(&module.name, &t);
            }
            self.db.set_program(&name, ProgramSource::from_layout(&layout));
        }
        Analysis { root: name.clone(), name }
    }

    /// `textDocument/publishDiagnostics` for a document.
    fn publish(&mut self, uri: &str) -> Json {
        let a = self.analyse(uri);
        let text = self.docs.get(uri).map(|d| d.text.clone()).unwrap_or_default();
        let version = self.docs.get(uri).map(|d| d.version).unwrap_or(0.0);
        let lines = Lines::new(&text, self.utf8);
        let diags = self.db.diagnostics(&a.root);
        let file = self
            .db
            .checked(&a.root)
            .and_then(|c| c.map.files().iter().find(|f| f.name == a.name).map(|f| f.id));
        let mut out = Vec::new();
        for d in &diags {
            if Some(d.span.file) != file {
                continue;
            }
            out.push(diagnostic_json(d, &lines));
        }
        notify(
            "textDocument/publishDiagnostics",
            Json::obj(vec![
                ("uri", Json::str(uri)),
                ("version", Json::Num(version)),
                ("diagnostics", Json::Arr(out)),
            ]),
        )
    }

    /// What is at an offset of a document: the innermost expression of a
    /// body there, and the body.
    pub(crate) fn expr_at<'t>(typed: &'t Typed, file: FileId, offset: usize) -> Option<(&'t Expr, &'t Body)> {
        let mut best: Option<(&Expr, &Body)> = None;
        for body in &typed.bodies {
            if body.span.file != file {
                continue;
            }
            let mut visit = |e: &'t Expr| {
                if contains(&e.span, file, offset)
                    && best.is_none_or(|(b, _)| width(&e.span) <= width(&b.span))
                {
                    best = Some((e, body));
                }
            };
            walk_block(&body.block, &mut visit);
            for c in &body.closures {
                walk_block(&c.body, &mut visit);
            }
        }
        best
    }

    /// The definition at an offset of a document: a definition's name, or
    /// what the innermost expression there names. A local is its body's
    /// local.
    pub(crate) fn target_at(&mut self, a: &Analysis, offset: usize) -> Option<Target> {
        let c = self.db.checked(&a.root)?;
        let file = c.map.files().iter().find(|f| f.name == a.name)?.id;
        let prog = &c.program;
        // A definition's own name.
        let mut best: Option<(u32, Target)> = None;
        for (i, def) in prog.defs.iter().enumerate() {
            if contains(&def.span, file, offset) && best.as_ref().is_none_or(|(w, _)| width(&def.span) < *w) {
                best = Some((width(&def.span), Target::Def(DefId(i as u32))));
            }
        }
        // A field's name in its struct.
        for (i, def) in prog.defs.iter().enumerate() {
            if let DefKind::Struct(s) = &def.kind {
                for (k, f) in s.fields.iter().enumerate() {
                    if contains(&f.span, file, offset) {
                        best = Some((width(&f.span), Target::Field(DefId(i as u32), k as u32)));
                    }
                }
            }
        }
        if let Some((_, t)) = best {
            return Some(t);
        }
        // A local's declaration.
        for (bi, body) in c.typed.bodies.iter().enumerate() {
            for (li, l) in body.locals.iter().enumerate() {
                if contains(&l.span, file, offset) {
                    return Some(Target::Local(bi, li));
                }
            }
        }
        let (e, body) = Self::expr_at(&c.typed, file, offset)?;
        let bi = c.typed.bodies.iter().position(|b| std::ptr::eq(&**b, body))?;
        Some(match &e.kind {
            ExprKind::Local(l) => Target::Local(bi, l.0 as usize),
            ExprKind::Const(d) | ExprKind::FnRef(d, _) => Target::Def(*d),
            ExprKind::Call { callee: sisu_typeck::Callee::Fn { def, .. }, .. } => Target::Def(*def),
            ExprKind::Struct { def, .. } | ExprKind::Variant { def, .. } => Target::Def(*def),
            ExprKind::Field { base, index } => match strip_refs(&base.ty) {
                Ty::Adt(d, _) if matches!(prog.def(*d).kind, DefKind::Struct(_)) => Target::Field(*d, *index),
                _ => Target::Expr(bi, e.span),
            },
            _ => Target::Expr(bi, e.span),
        })
    }

    fn hover(&mut self, uri: &str, pos: &Json) -> Json {
        let text = self.docs.get(uri).map(|d| d.text.clone()).unwrap_or_default();
        let lines = Lines::new(&text, self.utf8);
        let offset = lines.offset(pos);
        let a = self.analyse(uri);
        let Some(target) = self.target_at(&a, offset) else { return Json::Null };
        let Some(c) = self.db.checked(&a.root) else { return Json::Null };
        let prog = &c.program;
        let body_of = |bi: usize| c.typed.bodies.get(bi);
        let (value, span) = match &target {
            Target::Def(d) => (describe_def(prog, &c.typed, *d), None),
            Target::Field(d, k) => match &prog.def(*d).kind {
                DefKind::Struct(s) => match s.fields.get(*k as usize) {
                    Some(f) => (format!("```sisu\n{}: {}\n```", f.name, prog.display(&f.ty)), None),
                    None => return Json::Null,
                },
                _ => return Json::Null,
            },
            Target::Local(bi, li) => match body_of(*bi).and_then(|b| b.locals.get(*li)) {
                Some(l) => {
                    let kw = if l.mutable { "var" } else { "let" };
                    (format!("```sisu\n{kw} {}: {}\n```", l.name, prog.display(&l.ty)), None)
                }
                None => return Json::Null,
            },
            Target::Expr(bi, span) => {
                let file = span.file;
                match body_of(*bi).and_then(|_| Self::expr_at(&c.typed, file, offset)) {
                    Some((e, _)) => (format!("```sisu\n{}\n```", prog.display(&e.ty)), Some(*span)),
                    None => return Json::Null,
                }
            }
        };
        let contents = Json::obj(vec![("kind", Json::str("markdown")), ("value", Json::str(value))]);
        let mut fields = vec![("contents", contents)];
        if let Some(s) = span {
            fields.push(("range", lines.range(s.range.start as usize, s.range.end as usize)));
        }
        Json::obj(fields)
    }

    fn definition(&mut self, uri: &str, pos: &Json) -> Json {
        let text = self.docs.get(uri).map(|d| d.text.clone()).unwrap_or_default();
        let offset = Lines::new(&text, self.utf8).offset(pos);
        let a = self.analyse(uri);
        let Some(target) = self.target_at(&a, offset) else { return Json::Null };
        let Some(c) = self.db.checked(&a.root) else { return Json::Null };
        let span = match &target {
            Target::Def(d) => c.program.def(*d).span,
            Target::Field(d, k) => match &c.program.def(*d).kind {
                DefKind::Struct(s) => match s.fields.get(*k as usize) {
                    Some(f) => f.span,
                    None => return Json::Null,
                },
                _ => return Json::Null,
            },
            Target::Local(bi, li) => match c.typed.bodies.get(*bi).and_then(|b| b.locals.get(*li)) {
                Some(l) => l.span,
                None => return Json::Null,
            },
            Target::Expr(..) => return Json::Null,
        };
        let Some(f) = c.map.get(span.file) else { return Json::Null };
        // The file on disk: the document's own, a module of its package,
        // or the standard library's source.
        let path = if f.name == a.name {
            uri_path(uri)
        } else if let Some(rest) = f.name.strip_prefix("<std>/") {
            std_dir().join(rest)
        } else {
            let base = crate::package::locate(None, uri_path(uri).parent().unwrap_or(Path::new(".")))
                .and_then(|t| match t {
                    crate::package::Target::Package(d) => Some(d),
                    _ => None,
                })
                .unwrap_or_else(|| uri_path(uri).parent().map(Path::to_path_buf).unwrap_or_default());
            base.join(&f.name)
        };
        let lines = Lines::new(&f.text, self.utf8);
        Json::obj(vec![
            ("uri", Json::str(path_uri(&std::fs::canonicalize(&path).unwrap_or(path)))),
            ("range", lines.range(span.range.start as usize, span.range.end as usize)),
        ])
    }

    fn symbols(&mut self, uri: &str) -> Json {
        let text = self.docs.get(uri).map(|d| d.text.clone()).unwrap_or_default();
        let lines = Lines::new(&text, self.utf8);
        let parse = sisu_parser::parse(FileId(0), &text);
        let tree = &parse.tree;
        let items: Vec<Json> =
            tree.child_nodes(tree.root()).filter_map(|n| symbol(tree, n, &lines)).collect();
        Json::Arr(items)
    }

    fn selection_ranges(&mut self, uri: &str, positions: &[Json]) -> Json {
        let text = self.docs.get(uri).map(|d| d.text.clone()).unwrap_or_default();
        let lines = Lines::new(&text, self.utf8);
        let parse = sisu_parser::parse(FileId(0), &text);
        let tree = &parse.tree;
        let mut out = Vec::new();
        for p in positions {
            let offset = lines.offset(p);
            // The nodes around the offset, innermost first, each range once.
            let mut chain: Vec<(usize, usize)> = Vec::new();
            let mut node = Some(tree.root());
            let mut path = Vec::new();
            while let Some(n) = node {
                path.push(n);
                node = tree.child_nodes(n).find(|&c| {
                    let r = tree.range(c);
                    (r.start as usize) <= offset && offset < r.end as usize
                });
            }
            for n in path.into_iter().rev() {
                let r = tree.range(n);
                // A node's range without its trailing line break and blanks.
                let raw = &text[r.start as usize..r.end as usize];
                let end = r.start as usize + raw.trim_end().len();
                let span = (r.start as usize, end.max(r.start as usize));
                if chain.last() != Some(&span) {
                    chain.push(span);
                }
            }
            let mut sel = Json::Null;
            for (s, e) in chain.into_iter().rev() {
                let mut fields = vec![("range", lines.range(s, e))];
                if sel != Json::Null {
                    fields.push(("parent", sel));
                }
                sel = Json::obj(fields);
            }
            out.push(sel);
        }
        Json::Arr(out)
    }

    fn formatting(&mut self, uri: &str) -> Json {
        let text = self.docs.get(uri).map(|d| d.text.clone()).unwrap_or_default();
        match self.format_document(uri, &text) {
            Ok(out) if out != text => {
                let lines = Lines::new(&text, self.utf8);
                Json::Arr(vec![Json::obj(vec![
                    ("range", lines.range(0, text.len())),
                    ("newText", Json::str(out)),
                ])])
            }
            Ok(_) => Json::Arr(Vec::new()),
            Err(_) => Json::Null,
        }
    }
}

impl Server {
    /// The canonical text of a document, with the names of its program:
    /// `T ! E` where `E` names an error set.
    pub(crate) fn format_document(&mut self, uri: &str, text: &str) -> Result<String, sisu_fmt::Refusal> {
        if !text.contains("Result[") {
            return sisu_fmt::format(text);
        }
        let a = self.analyse(uri);
        match self.db.checked(&a.root) {
            Some(c) => crate::format::format_in(&c, &a.name, text),
            None => sisu_fmt::format(text),
        }
    }
}

/// A diagnostic as the protocol's, with its catalog code as `code` and its
/// identifier as `data.id`.
pub(crate) fn diagnostic_json(d: &sisu_diag::Diagnostic, lines: &Lines) -> Json {
    let severity = match d.severity() {
        Severity::Error => 1,
        Severity::Warning => 2,
        _ => 3,
    };
    Json::obj(vec![
        ("range", lines.range(d.span.range.start as usize, d.span.range.end as usize)),
        ("severity", Json::num(severity)),
        ("code", Json::str(d.code())),
        ("source", Json::str("sisu")),
        ("message", Json::str(d.message())),
        ("data", Json::obj(vec![("id", Json::str(d.msg.id()))])),
    ])
}

/// What a position names.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Target {
    Def(DefId),
    /// A struct's field, by index.
    Field(DefId, u32),
    /// A local of the body at this index.
    Local(usize, usize),
    /// An expression that names nothing: its type is what there is to say.
    Expr(usize, Span),
}

/// The type a reference refers to, through every `ref`.
pub(crate) fn strip_refs(t: &Ty) -> &Ty {
    match t {
        Ty::Ref(inner, _) => strip_refs(inner),
        other => other,
    }
}

pub(crate) fn walk_block<'t>(b: &'t sisu_typeck::Block, f: &mut dyn FnMut(&'t Expr)) {
    sisu_typeck::purity::visit_block(b, f);
}

/// The hover text of a definition: its signature, what the checker knows
/// of it (the inferred error set, the capabilities it takes, its
/// contracts), and its documentation.
pub(crate) fn describe_def(prog: &Program, typed: &Typed, d: DefId) -> String {
    let def = prog.def(d);
    let mut extra: Vec<String> = Vec::new();
    let sig = signature(prog, typed, d, &mut extra);
    let mut out = format!("```sisu\n{sig}\n```");
    for x in extra {
        out.push_str("\n\n");
        out.push_str(&x);
    }
    if !def.docs.is_empty() {
        out.push_str("\n\n");
        out.push_str(&def.docs.join("\n"));
    }
    out
}

/// A definition's signature as it would be written, and what the checker
/// knows of it besides (its inferred error set, the capabilities it takes,
/// its contracts) in `extra`.
pub(crate) fn signature(prog: &Program, typed: &Typed, d: DefId, extra: &mut Vec<String>) -> String {
    let def = prog.def(d);
    match &def.kind {
        DefKind::Fn(f) => {
            let owner = f.owner.as_ref().map(|o| format!("{}.", prog.display(o))).unwrap_or_default();
            let mut params: Vec<String> = Vec::new();
            if let Some(m) = f.self_mode {
                params.push(match m {
                    sisu_syntax::ast::Mode::Own => "own self".into(),
                    sisu_syntax::ast::Mode::Mut => "mut self".into(),
                    sisu_syntax::ast::Mode::Borrow => "self".into(),
                });
            }
            let mut caps = Vec::new();
            for p in &f.params {
                let mode = match p.mode {
                    sisu_syntax::ast::Mode::Own => "own ",
                    sisu_syntax::ast::Mode::Mut => "mut ",
                    sisu_syntax::ast::Mode::Borrow => "",
                };
                let ty = prog.display(&p.ty);
                params.push(format!("{mode}{}: {ty}", p.name));
                if is_capability(prog, &p.ty) {
                    caps.push(format!("`{}: {ty}`", p.name));
                }
            }
            let pure = if f.pure { "pure " } else { "" };
            let ret = prog.display(&f.ret);
            let ret = if f.err_inferred {
                let set: Vec<String> = typed
                    .inferred
                    .get(&d)
                    .map(|s| s.iter().map(|e| prog.def(*e).name.clone()).collect())
                    .unwrap_or_default();
                extra.push(match set.len() {
                    0 => "error set (inferred): none".to_string(),
                    _ => format!("error set (inferred): `{{{}}}`", set.join(", ")),
                });
                ret
            } else {
                ret
            };
            if !caps.is_empty() {
                extra.push(format!("capabilities: {}", caps.join(", ")));
            }
            if let Some(b) = typed.body_of(d, &sisu_typeck::BodyKind::Fn)
                && let Some(c) = &b.contracts
            {
                for r in &c.requires {
                    extra.push(format!("requires `{}`", r.text));
                }
                for e in &c.ensures {
                    extra.push(format!("ensures `{}`", e.text));
                }
            }
            format!("{pure}def {owner}{}({}) -> {ret}", def.name, params.join(", "))
        }
        DefKind::Struct(s) => {
            let fields: Vec<String> =
                s.fields.iter().map(|f| format!("    {}: {}", f.name, prog.display(&f.ty))).collect();
            format!("struct {}:\n{}", def.name, fields.join("\n"))
        }
        DefKind::Enum(e) | DefKind::ErrorSet(e) => {
            let kw = if matches!(def.kind, DefKind::ErrorSet(_)) { "error" } else { "enum" };
            let vs: Vec<String> = e.variants.iter().map(|v| format!("    {}", v.name)).collect();
            format!("{kw} {}:\n{}", def.name, vs.join("\n"))
        }
        DefKind::Const(c) => format!("const {}: {}", def.name, prog.display(&c.ty)),
        DefKind::Trait(_) => format!("trait {}", def.name),
        DefKind::Module(_) => format!("module {}", prog.qualified_name(d)),
        DefKind::TypeAlias(t) => format!("type {} = {}", def.name, prog.display(t)),
        DefKind::Refinement(_) => format!("type {}", def.name),
        DefKind::Impl(_) | DefKind::Unsupported => def.name.clone(),
    }
}

/// A document symbol of an item, with its members.
fn symbol(tree: &Tree, n: NodeId, lines: &Lines) -> Option<Json> {
    let kind = tree.kind(n);
    let number = match kind {
        FN_DEF => 12,
        STRUCT_DEF => 23,
        ENUM_DEF | ERROR_DEF => 10,
        TRAIT_DEF => 11,
        IMPL_DEF => 5,
        CONST_DEF => 14,
        TYPE_ALIAS => 26,
        TEST_DEF | BENCH_DEF | SUITE_DEF => 24,
        FIELD_DEF => 8,
        VARIANT_DEF => 22,
        _ => return None,
    };
    let r = tree.range(n);
    let raw = &lines.text[r.start as usize..r.end as usize];
    let end = r.start as usize + raw.trim_end().len();
    // The name: an item's identifier, a test's string, an impl's header.
    let (name, sel) = match kind {
        TEST_DEF | BENCH_DEF | SUITE_DEF => {
            let s = tree.child_nodes(n).find(|&c| tree.kind(c) == STRING_EXPR)?;
            let sr = tree.range(s);
            (tree.node_text(s).to_string(), (sr.start as usize, sr.end as usize))
        }
        IMPL_DEF => {
            let header: String = tree
                .children(n)
                .iter()
                .take_while(|e| !matches!(e, Element::Token(t) if tree.token_kind(*t) == COLON))
                .map(|e| match e {
                    Element::Token(t) => tree.token_text(*t).to_string(),
                    Element::Node(c) => tree.node_text(*c).to_string(),
                })
                .collect();
            (
                header.split_whitespace().collect::<Vec<_>>().join(" "),
                (r.start as usize, r.start as usize + 4),
            )
        }
        _ => {
            // The name follows the keyword (`pure def area`: `pure` is an
            // identifier to the lexer).
            let kw = tree.child_tokens(n).position(|t| {
                matches!(
                    tree.token_kind(t),
                    DEF_KW | STRUCT_KW | ENUM_KW | ERROR_KW | TRAIT_KW | CONST_KW | TYPE_KW
                )
            });
            let t = tree
                .child_tokens(n)
                .skip(kw.map(|k| k + 1).unwrap_or(0))
                .find(|&t| tree.token_kind(t) == IDENT)?;
            let tr = tree.token_range(t);
            (tree.token_text(t).to_string(), (tr.start as usize, tr.end as usize))
        }
    };
    let mut children = Vec::new();
    for c in tree.preorder(n).into_iter().skip(1) {
        // Members are items one level down: methods, fields, variants.
        if tree.parent(c).is_some_and(|p| p == n || tree.parent(p) == Some(n))
            && matches!(tree.kind(c), FN_DEF | FIELD_DEF | VARIANT_DEF | CONST_DEF)
            && let Some(s) = symbol(tree, c, lines)
        {
            children.push(s);
        }
    }
    let number = if kind == FN_DEF && tree.ancestors_any(n, &[STRUCT_DEF, IMPL_DEF, TRAIT_DEF, ENUM_DEF]) {
        6
    } else {
        number
    };
    let mut fields = vec![
        ("name", Json::str(name)),
        ("kind", Json::num(number as f64)),
        ("range", lines.range(r.start as usize, end)),
        ("selectionRange", lines.range(sel.0, sel.1)),
    ];
    if !children.is_empty() {
        fields.push(("children", Json::Arr(children)));
    }
    Some(Json::obj(fields))
}

trait Ancestors {
    fn ancestors_any(&self, n: NodeId, kinds: &[SyntaxKind]) -> bool;
}

impl Ancestors for Tree {
    fn ancestors_any(&self, n: NodeId, kinds: &[SyntaxKind]) -> bool {
        let mut cur = self.parent(n);
        while let Some(p) = cur {
            if kinds.contains(&self.kind(p)) {
                return true;
            }
            cur = self.parent(p);
        }
        false
    }
}

/// Reads one message: its `Content-Length` header block, then the body.
pub fn read_message(input: &mut dyn BufRead) -> Option<String> {
    let mut length: Option<usize> = None;
    loop {
        let mut line = String::new();
        if input.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let line = line.trim_end();
        if line.is_empty() {
            if length.is_some() {
                break;
            }
            continue;
        }
        if let Some(v) = line.strip_prefix("Content-Length:") {
            length = v.trim().parse().ok();
        }
    }
    let mut body = vec![0u8; length?];
    input.read_exact(&mut body).ok()?;
    String::from_utf8(body).ok()
}

pub fn write_message(out: &mut dyn Write, body: &str) {
    let _ = write!(out, "Content-Length: {}\r\n\r\n{body}", body.len());
    let _ = out.flush();
}

/// `sisu lsp`: serves standard input and output until `exit`.
pub fn serve() -> i32 {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    let stdout = std::io::stdout();
    let mut server = Server::new();
    while let Some(text) = read_message(&mut input) {
        let messages = match crate::json::parse(&text) {
            Ok(Json::Arr(batch)) => batch,
            Ok(m) => vec![m],
            Err(_) => continue,
        };
        for m in messages {
            for out in server.handle(&m) {
                write_message(&mut stdout.lock(), &out.write());
            }
        }
        if let Some(status) = server.exit {
            return status;
        }
    }
    1
}
