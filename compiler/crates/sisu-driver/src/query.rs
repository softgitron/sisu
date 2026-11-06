//! The language server's answers from stage 1's queries (decision 983):
//! `sisu1 query KIND FILE [LINE COL] --text PATH` (its R164, `sisu1
//! 0.14.0` and later), served by the driver's server of stage 1 (decision
//! 980), for the diagnostics, hover, go-to-definition, references,
//! completion and document symbols, and from `sisu1 0.15.0` (its R169,
//! decision 1060) selection ranges, rename, code actions, inlay hints and
//! formatting too. A document's text is written to a
//! file of the server's own, which `--text` names. An answer is one line
//! per item, its fields separated by tabs, positions `LINE:COL` with the
//! column in characters; the server turns them into the protocol's. A
//! query that stage 1 cannot answer (it failed, or stopped) is answered
//! from the Rust front end, as an older stage 1's, and `--front-end=rust`
//! answers everything so.

use crate::json::Json;
use crate::lsp::{Lines, Server, path_uri, uri_path};
use std::path::{Path, PathBuf};

/// The byte offset of a position of an answer, line and column from 1, the
/// column in characters; a column past a line's end runs into the next.
fn offset_at(text: &str, line: usize, col: usize) -> usize {
    let mut start = 0;
    for _ in 1..line {
        match text[start..].find('\n') {
            Some(i) => start += i + 1,
            None => return text.len(),
        }
    }
    text[start..].char_indices().nth(col.saturating_sub(1)).map_or(text.len(), |(i, _)| start + i)
}

/// `LINE:COL`.
fn position(field: &str) -> Option<(usize, usize)> {
    let (line, col) = field.split_once(':')?;
    Some((line.parse().ok()?, col.parse().ok()?))
}

/// `LINE:COL-LINE:COL`, as the protocol's range in `text`.
fn range(field: &str, lines: &Lines) -> Option<Json> {
    let (from, to) = field.split_once('-')?;
    let ((l1, c1), (l2, c2)) = (position(from)?, position(to)?);
    Some(lines.range(offset_at(lines.text, l1, c1), offset_at(lines.text, l2, c2)))
}

/// A field of an answer with its escapes undone (`\\`, `\t`, `\n`).
fn unescaped(field: &str) -> String {
    let mut out = String::new();
    let mut chars = field.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some(o) => out.push(o),
            None => out.push('\\'),
        }
    }
    out
}

/// The protocol's kind of a document symbol, by the kind `query symbols`
/// names.
fn symbol_kind(kind: &str) -> f64 {
    match kind {
        "method" => 6.0,
        "struct" => 23.0,
        "enum" | "error" => 10.0,
        "trait" => 11.0,
        "impl" => 5.0,
        "const" => 14.0,
        "type" => 26.0,
        "test" => 24.0,
        "field" => 8.0,
        "variant" => 22.0,
        _ => 12.0,
    }
}

/// The protocol's kind of a completion, by the kind `query completions`
/// names.
fn completion_kind(kind: &str) -> f64 {
    match kind {
        "method" => 2.0,
        "function" => 3.0,
        "field" => 5.0,
        "variable" => 6.0,
        "type" => 7.0,
        "trait" => 8.0,
        "module" => 9.0,
        "enum" => 13.0,
        "keyword" => 14.0,
        "variant" => 20.0,
        "const" => 21.0,
        _ => 22.0,
    }
}

impl Server {
    /// The answer of `query KIND` of a document, at a position when one is
    /// given, as its lines of fields; none without a stage 1 that answers
    /// queries, or when it failed.
    fn query(&mut self, kind: &str, uri: &str, pos: Option<&Json>) -> Option<Vec<Vec<String>>> {
        self.query_with(kind, uri, pos, &[])
    }

    /// [`Server::query`], with words after the position (`rename`'s new
    /// name).
    fn query_with(
        &mut self,
        kind: &str,
        uri: &str,
        pos: Option<&Json>,
        more: &[String],
    ) -> Option<Vec<Vec<String>>> {
        let stage1 = self.stage1.as_ref()?;
        let text = self.docs.get(uri).map(|d| d.text.clone())?;
        let path = uri_path(uri);
        let buffer = buffer_path(&path)?;
        std::fs::write(&buffer, &text).ok()?;
        let mut words = vec!["query".to_string(), kind.to_string(), path.display().to_string()];
        if let Some(pos) = pos {
            let offset = Lines::new(&text, self.utf8).offset(pos);
            let before = &text[..offset];
            let line = before.matches('\n').count() + 1;
            let col = before[before.rfind('\n').map_or(0, |i| i + 1)..].chars().count() + 1;
            words.push(line.to_string());
            words.push(col.to_string());
        }
        words.extend(more.iter().cloned());
        words.push("--text".into());
        words.push(buffer.display().to_string());
        let ran = stage1.ask(&words)?;
        if ran.status != 0 {
            return None;
        }
        Some(ran.stdout.lines().map(|l| l.split('\t').map(unescaped).collect()).collect())
    }

    /// The text of the file at `path`: an open document's, else what is on
    /// disk.
    fn text_at(&self, path: &Path) -> String {
        let canon = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
        let me = canon(path);
        self.docs
            .iter()
            .find(|(u, _)| canon(&uri_path(u)) == me)
            .map(|(_, d)| d.text.clone())
            .or_else(|| std::fs::read_to_string(path).ok())
            .unwrap_or_default()
    }

    /// The document's diagnostics, from `query diagnostics`.
    pub(crate) fn stage1_diagnostics(&mut self, uri: &str) -> Option<Vec<Json>> {
        let rows = self.query("diagnostics", uri, None)?;
        let text = self.docs.get(uri).map(|d| d.text.clone()).unwrap_or_default();
        let lines = Lines::new(&text, self.utf8);
        let mut out = Vec::new();
        for row in &rows {
            let [at, code, id, severity, message] = row.as_slice() else { continue };
            let severity = match severity.as_str() {
                "error" => 1,
                "warning" => 2,
                _ => 3,
            };
            out.push(Json::obj(vec![
                ("range", range(at, &lines)?),
                ("severity", Json::num(severity)),
                ("code", Json::str(code.clone())),
                ("source", Json::str("sisu")),
                ("message", Json::str(message.clone())),
                ("data", Json::obj(vec![("id", Json::str(id.clone()))])),
            ]));
        }
        Some(out)
    }

    /// A hover, from `query hover`: the signature as code, then what the
    /// checker knows besides and the documentation.
    pub(crate) fn stage1_hover(&mut self, uri: &str, pos: &Json) -> Option<Json> {
        let rows = self.query("hover", uri, Some(pos))?;
        let Some([at, signature, notes, doc]) = rows.first().map(Vec::as_slice) else {
            return Some(Json::Null);
        };
        let text = self.docs.get(uri).map(|d| d.text.clone()).unwrap_or_default();
        let lines = Lines::new(&text, self.utf8);
        let mut value = format!("```sisu\n{signature}\n```");
        for more in [notes, doc] {
            if !more.is_empty() {
                value.push_str("\n\n");
                value.push_str(more);
            }
        }
        let contents = Json::obj(vec![("kind", Json::str("markdown")), ("value", Json::str(value))]);
        let mut fields = vec![("contents", contents)];
        if let Some(r) = range(at, &lines) {
            fields.push(("range", r));
        }
        Some(Json::obj(fields))
    }

    /// A location of an answer, `PATH` and `RANGE`.
    fn location(&self, path: &str, at: &str) -> Option<Json> {
        let path = PathBuf::from(path);
        let text = self.text_at(&path);
        let lines = Lines::new(&text, self.utf8);
        Some(Json::obj(vec![("uri", Json::str(path_uri(&path))), ("range", range(at, &lines)?)]))
    }

    /// Go-to-definition, from `query definition`.
    pub(crate) fn stage1_definition(&mut self, uri: &str, pos: &Json) -> Option<Json> {
        let rows = self.query("definition", uri, Some(pos))?;
        Some(match rows.first().map(Vec::as_slice) {
            Some([path, at]) => self.location(path, at).unwrap_or(Json::Null),
            _ => Json::Null,
        })
    }

    /// The references, from `query references`, the declaration first.
    pub(crate) fn stage1_references(&mut self, uri: &str, pos: &Json) -> Option<Json> {
        let rows = self.query("references", uri, Some(pos))?;
        let found = rows.iter().filter_map(|r| match r.as_slice() {
            [path, at, _] => self.location(path, at),
            _ => None,
        });
        Some(Json::Arr(found.collect()))
    }

    /// The completions, from `query completions`, in its order.
    pub(crate) fn stage1_completion(&mut self, uri: &str, pos: &Json) -> Option<Json> {
        let rows = self.query("completions", uri, Some(pos))?;
        let mut items = Vec::new();
        for row in &rows {
            let [kind, label, detail, snippet] = row.as_slice() else { continue };
            let mut fields = vec![
                ("label", Json::str(label.clone())),
                ("kind", Json::num(completion_kind(kind))),
                ("detail", Json::str(detail.clone())),
            ];
            if !snippet.is_empty() {
                fields.push(("insertText", Json::str(snippet.clone())));
                fields.push(("insertTextFormat", Json::num(2)));
            }
            items.push(Json::obj(fields));
        }
        Some(Json::obj(vec![("isIncomplete", Json::Bool(false)), ("items", Json::Arr(items))]))
    }

    /// The document's symbols, from `query symbols`: each item with its
    /// members (depth 1) as its children.
    pub(crate) fn stage1_symbols(&mut self, uri: &str) -> Option<Json> {
        let rows = self.query("symbols", uri, None)?;
        let text = self.docs.get(uri).map(|d| d.text.clone()).unwrap_or_default();
        let lines = Lines::new(&text, self.utf8);
        let mut items: Vec<Symbol> = Vec::new();
        for row in &rows {
            let [kind, name, at, name_at, depth] = row.as_slice() else { continue };
            let fields = vec![
                ("name", Json::str(name.clone())),
                ("kind", Json::num(symbol_kind(kind))),
                ("range", range(at, &lines)?),
                ("selectionRange", range(name_at, &lines)?),
            ];
            match (depth.as_str(), items.last_mut()) {
                ("0", _) | (_, None) => items.push((fields, Vec::new())),
                (_, Some((_, children))) => children.push(Json::obj(fields)),
            }
        }
        Some(Json::Arr(
            items
                .into_iter()
                .map(|(mut fields, children)| {
                    if !children.is_empty() {
                        fields.push(("children", Json::Arr(children)));
                    }
                    Json::obj(fields)
                })
                .collect(),
        ))
    }
}

/// The lines of a document that a range of an answer covers, as `(first,
/// end)`, the end past its last line: a range that ends at a line's start
/// stops before that line.
fn line_span(field: &str) -> Option<(usize, usize)> {
    let (from, to) = field.split_once('-')?;
    let ((l1, _), (l2, c2)) = (position(from)?, position(to)?);
    Some((l1.saturating_sub(1), if c2 == 1 { l2.saturating_sub(1) } else { l2 }))
}

/// The protocol's kind and padding of an inlay hint, by the kind `query
/// inlay` names: a type and an error set after what they describe, a mode
/// before its argument.
fn hint_kind(kind: &str) -> (f64, bool, bool) {
    match kind {
        "errors" => (1.0, true, false),
        "mode" => (2.0, false, true),
        _ => (1.0, false, false),
    }
}

/// What `query format` answered: the runs of changed lines, each its range
/// and the lines that replace it; none when it refused the text.
type Runs = Option<Vec<(String, String)>>;

impl Server {
    /// Whether stage 1 answers the language server's other requests:
    /// selection ranges, rename, code actions, inlay hints and formatting
    /// (`sisu1 0.15.0` and later, its R169; decision 1060).
    fn assists(&self) -> bool {
        self.stage1.as_ref().is_some_and(crate::front::Stage1::assists)
    }

    /// `sisu.explainBorrow` of line `line` of a document, from `query
    /// explain` (`sisu1 0.17.0` and later, its R187; decision 1104): a loan a
    /// line, `FUNCTION KIND PLACE START HOLDERS END`, each with the line the
    /// CLI prints for it. From `sisu1 0.20.0` the query is asked with
    /// `--ranges` (its R211, decision 1242), whose two fields more are where
    /// the loan starts and is last used as ranges with their ends, which the
    /// editor marks whole; before it each range is the position. A program
    /// with an error has none to show, as `query diagnostics` says.
    pub(crate) fn stage1_explain_borrow(&mut self, uri: &str, line: u32) -> Option<Result<Json, String>> {
        if !self.stage1.as_ref().is_some_and(crate::front::Stage1::takes_the_roles) {
            return None;
        }
        let diagnostics = self.query("diagnostics", uri, None)?;
        if diagnostics.iter().any(|r| r.get(3).is_some_and(|s| s == "error")) {
            return Some(Err(crate::cli::text(sisu_diag::msg::LSP_HAS_ERRORS, &[])));
        }
        let ranges = self.stage1.as_ref().is_some_and(crate::front::Stage1::gives_the_asks);
        let mut more = vec![line.to_string()];
        if ranges {
            more.push("--ranges".into());
        }
        let rows = self.query_with("explain", uri, None, &more)?;
        let text = self.docs.get(uri).map(|d| d.text.clone()).unwrap_or_default();
        let lines = Lines::new(&text, self.utf8);
        let mut loans = Vec::new();
        for row in &rows {
            let (function, kind, place, start, holders, end, spans) = match row.as_slice() {
                [f, k, p, s, h, e] => (f, k, p, s, h, e, None),
                [f, k, p, s, h, e, from, until] => (f, k, p, s, h, e, Some((from, until))),
                _ => continue,
            };
            let mutable = kind == "mut";
            let holders: Vec<String> =
                holders.split(", ").filter(|h| !h.is_empty()).map(str::to_string).collect();
            let quoted: Vec<String> = holders.iter().map(|h| format!("`{h}`")).collect();
            let m = if quoted.is_empty() {
                sisu_diag::msg::CLI_EXPLAIN_BORROW_TEMPORARY
            } else {
                sisu_diag::msg::CLI_EXPLAIN_BORROW_LOAN
            };
            let kind_text = crate::cli::text(
                if mutable {
                    sisu_diag::msg::CLI_EXPLAIN_BORROW_MUT
                } else {
                    sisu_diag::msg::CLI_EXPLAIN_BORROW_SHARED
                },
                &[],
            );
            let message = crate::cli::text(
                m,
                &[
                    ("kind", kind_text),
                    ("place", place.clone()),
                    ("start", start.clone()),
                    ("holders", quoted.join(", ")),
                    ("end", end.clone()),
                ],
            );
            let at = |p: &str| range(&format!("{p}-{p}"), &lines).unwrap_or(Json::Null);
            let whole = |r: &str| range(r, &lines).unwrap_or(Json::Null);
            let (from, until) = match spans {
                Some((from, until)) => (whole(from), whole(until)),
                None => (at(start), at(end)),
            };
            loans.push(Json::obj(vec![
                ("function", Json::str(function.clone())),
                ("mutable", Json::Bool(mutable)),
                ("place", Json::str(place.clone())),
                ("start", from),
                ("until", until),
                ("holders", Json::Arr(holders.into_iter().map(Json::str).collect())),
                ("message", Json::str(message)),
            ]));
        }
        Some(Ok(Json::obj(vec![("line", Json::num(line)), ("loans", Json::Arr(loans))])))
    }

    /// A worksheet's inline values from `query values` (`sisu1 0.18.0`, its
    /// R196; decision 1141): the values and the printed lines of each line
    /// a statement ends on, as `sisu ws` shows them. Where stage 1's
    /// evaluator stops (a `stopped` line: the world, an intrinsic it lacks
    /// or its budget), the driver's interpreter runs stage 1's program of
    /// the worksheet against the fakes instead, as `sisu ws` does; a
    /// `panicked` line ends the values. A document that is not a script
    /// (stage 1 stops at its line 0) or does not build has none. None
    /// without a stage 1 that has the query.
    pub(crate) fn stage1_values(&mut self, uri: &str) -> Option<crate::repl::Sheet> {
        if !self.stage1.as_ref().is_some_and(crate::front::Stage1::builds) {
            return None;
        }
        let rows = self.query("values", uri, None)?;
        let mut sheet = crate::repl::Sheet {
            values: Default::default(),
            outputs: Default::default(),
            errors: String::new(),
            status: 0,
        };
        for row in &rows {
            let [line, kind, text] = row.as_slice() else { continue };
            let Ok(line) = line.parse::<usize>() else { continue };
            match kind.as_str() {
                "value" => sheet.values.entry(line).or_default().push(text.clone()),
                "output" => sheet.outputs.entry(line).or_default().push(text.clone()),
                "stopped" if line > 0 => return Some(self.stage1_sheet_run(uri).unwrap_or(sheet)),
                _ => break,
            }
        }
        Some(sheet)
    }

    /// The driver's run of stage 1's program of a worksheet (`sir
    /// --worksheet`, its R79), against a test's fakes and within the
    /// language server's budget: what `query values` leaves where stage
    /// 1's evaluator stops (decision 1141).
    fn stage1_sheet_run(&mut self, uri: &str) -> Option<crate::repl::Sheet> {
        let stage1 = self.stage1.as_ref()?;
        let text = self.docs.get(uri).map(|d| d.text.clone())?;
        let buffer = buffer_path(&uri_path(uri))?;
        std::fs::write(&buffer, &text).ok()?;
        let words = vec!["sir".to_string(), "--worksheet".to_string(), buffer.display().to_string()];
        let ran = stage1.ask(&words)?;
        if ran.status != 0 {
            return None;
        }
        let prog = sisu_sir::read(&ran.stdout).ok()?;
        let mut host = sisu_interp::TestHost::default();
        Some(crate::repl::run_sheet(&prog, false, Some(crate::assist::SHEET_FUEL), &mut host))
    }

    /// The selection ranges at each position, from `query selection`: its
    /// ranges innermost first, each the parent of the one before.
    pub(crate) fn stage1_selection(&mut self, uri: &str, positions: &[Json]) -> Option<Json> {
        if !self.assists() {
            return None;
        }
        let text = self.docs.get(uri).map(|d| d.text.clone())?;
        let mut out = Vec::new();
        for p in positions {
            let rows = self.query_with("selection", uri, Some(p), &[])?;
            let lines = Lines::new(&text, self.utf8);
            let mut sel = Json::Null;
            for row in rows.iter().rev() {
                let Some(r) = row.first().and_then(|f| range(f, &lines)) else { continue };
                let mut fields = vec![("range", r)];
                if sel != Json::Null {
                    fields.push(("parent", sel));
                }
                sel = Json::obj(fields);
            }
            out.push(sel);
        }
        Some(Json::Arr(out))
    }

    /// The URI of a file an answer names: the document's own, or the file's.
    fn uri_of(&self, path: &Path, uri: &str) -> String {
        let canon = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
        if canon(path) == canon(&uri_path(uri)) { uri.to_string() } else { path_uri(path) }
    }

    /// A rename, from `query rename`: its lines as one workspace edit, or
    /// why it cannot be made (`refused`).
    pub(crate) fn stage1_rename(
        &mut self,
        uri: &str,
        pos: &Json,
        new_name: &str,
    ) -> Option<Result<Json, String>> {
        if !self.assists() {
            return None;
        }
        let rows = self.query_with("rename", uri, Some(pos), &[new_name.to_string()])?;
        if let Some(["refused", reason]) =
            rows.first().map(|r| r.iter().map(String::as_str).collect::<Vec<_>>()).as_deref()
        {
            return Some(Err(match *reason {
                "not-a-name" => {
                    crate::cli::text(sisu_diag::msg::LSP_NOT_A_NAME, &[("name", new_name.to_string())])
                }
                _ => crate::cli::text(sisu_diag::msg::LSP_CANNOT_RENAME, &[]),
            }));
        }
        let mut by_file: Vec<(String, Vec<Json>)> = Vec::new();
        for row in &rows {
            let [path, at, new_text] = row.as_slice() else { continue };
            let path = PathBuf::from(path);
            let text = self.text_at(&path);
            let lines = Lines::new(&text, self.utf8);
            let Some(r) = range(at, &lines) else { continue };
            let file = self.uri_of(&path, uri);
            let edit = Json::obj(vec![("range", r), ("newText", Json::str(new_text.clone()))]);
            match by_file.iter_mut().find(|(f, _)| *f == file) {
                Some((_, edits)) => edits.push(edit),
                None => by_file.push((file, vec![edit])),
            }
        }
        if by_file.is_empty() {
            return Some(Err(crate::cli::text(sisu_diag::msg::LSP_CANNOT_RENAME, &[])));
        }
        by_file.sort_by(|a, b| a.0.cmp(&b.0));
        Some(Ok(Json::obj(vec![(
            "changes",
            Json::Obj(by_file.into_iter().map(|(f, e)| (f, Json::Arr(e))).collect()),
        )])))
    }

    /// `prepareRename`, from `query references` (the name at the position)
    /// and `query rename` to that name (whether it can be renamed).
    pub(crate) fn stage1_prepare_rename(&mut self, uri: &str, pos: &Json) -> Option<Json> {
        if !self.assists() {
            return None;
        }
        let text = self.docs.get(uri).map(|d| d.text.clone())?;
        let offset = Lines::new(&text, self.utf8).offset(pos);
        let rows = self.query_with("references", uri, Some(pos), &[])?;
        let found = rows.iter().find_map(|row| {
            let [path, at, _] = row.as_slice() else { return None };
            let (from, to) = at.split_once('-')?;
            let ((l1, c1), (l2, c2)) = (position(from)?, position(to)?);
            let (s, e) = (offset_at(&text, l1, c1), offset_at(&text, l2, c2));
            (self.uri_of(Path::new(path), uri) == uri && s <= offset && offset <= e).then_some((s, e))
        });
        let Some((s, e)) = found else { return Some(Json::Null) };
        let name = text[s..e].to_string();
        let renamed = self.query_with("rename", uri, Some(pos), std::slice::from_ref(&name))?;
        if renamed.first().is_none_or(|r| r.first().map(String::as_str) == Some("refused")) {
            return Some(Json::Null);
        }
        let lines = Lines::new(&text, self.utf8);
        Some(Json::obj(vec![("range", lines.range(s, e)), ("placeholder", Json::str(name))]))
    }

    /// The code actions at the start of a range, from `query actions`: each
    /// fix of a diagnostic there, a quick fix with its edits, the
    /// diagnostic as `query diagnostics` published it.
    pub(crate) fn stage1_code_actions(&mut self, uri: &str, at: &Json) -> Option<Json> {
        if !self.assists() {
            return None;
        }
        let start = at
            .get("start")
            .cloned()
            .unwrap_or_else(|| Json::obj(vec![("line", Json::num(0)), ("character", Json::num(0))]));
        let rows = self.query_with("actions", uri, Some(&start), &[])?;
        let published = self.stage1_diagnostics(uri).unwrap_or_default();
        let text = self.docs.get(uri).map(|d| d.text.clone()).unwrap_or_default();
        let lines = Lines::new(&text, self.utf8);
        let mut out = Vec::new();
        let mut open: Option<(bool, String, Json, Vec<Json>)> = None;
        let close = |open: &mut Option<(bool, String, Json, Vec<Json>)>, out: &mut Vec<Json>| {
            if let Some((preferred, title, diagnostic, edits)) = open.take() {
                out.push(Json::obj(vec![
                    ("title", Json::str(title)),
                    ("kind", Json::str("quickfix")),
                    ("isPreferred", Json::Bool(preferred)),
                    ("diagnostics", Json::Arr(vec![diagnostic])),
                    (
                        "edit",
                        Json::obj(vec![("changes", Json::Obj(vec![(uri.to_string(), Json::Arr(edits))]))]),
                    ),
                ]));
            }
        };
        for row in &rows {
            match row.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
                ["fix", k, title, at, code, id] => {
                    close(&mut open, &mut out);
                    let r = range(at, &lines)?;
                    let diagnostic = published
                        .iter()
                        .find(|d| {
                            d.get("range") == Some(&r) && d.get("code").and_then(Json::as_str) == Some(code)
                        })
                        .cloned()
                        .unwrap_or_else(|| {
                            Json::obj(vec![
                                ("range", r.clone()),
                                ("code", Json::str(*code)),
                                ("source", Json::str("sisu")),
                                ("data", Json::obj(vec![("id", Json::str(*id))])),
                            ])
                        });
                    open = Some((*k == "0", title.to_string(), diagnostic, Vec::new()));
                }
                ["edit", at, new_text] => {
                    if let Some((_, _, _, edits)) = open.as_mut() {
                        edits.push(Json::obj(vec![
                            ("range", range(at, &lines)?),
                            ("newText", Json::str(*new_text)),
                        ]));
                    }
                }
                _ => {}
            }
        }
        close(&mut open, &mut out);
        Some(Json::Arr(out))
    }

    /// The inlay hints in a range, from `query inlay`.
    pub(crate) fn stage1_inlay_hints(&mut self, uri: &str, within: &Json) -> Option<Json> {
        if !self.assists() {
            return None;
        }
        let rows = self.query_with("inlay", uri, None, &[])?;
        let text = self.docs.get(uri).map(|d| d.text.clone()).unwrap_or_default();
        let lines = Lines::new(&text, self.utf8);
        let from = within.get("start").map(|p| lines.offset(p)).unwrap_or(0);
        let to = within.get("end").map(|p| lines.offset(p)).unwrap_or(text.len());
        let mut out = Vec::new();
        for row in &rows {
            let [at, kind, label] = row.as_slice() else { continue };
            let Some((line, col)) = position(at) else { continue };
            let offset = offset_at(&text, line, col);
            if offset < from || offset > to {
                continue;
            }
            let (number, left, right) = hint_kind(kind);
            out.push(Json::obj(vec![
                ("position", lines.position(offset)),
                ("label", Json::str(label.clone())),
                ("kind", Json::num(number)),
                ("paddingLeft", Json::Bool(left)),
                ("paddingRight", Json::Bool(right)),
            ]));
        }
        Some(Json::Arr(out))
    }

    /// `query format`'s runs of changed lines; none without a stage 1 that
    /// answers it, `Some(None)` when it refused the text.
    fn stage1_runs(&mut self, uri: &str) -> Option<Runs> {
        if !self.assists() {
            return None;
        }
        let rows = self.query_with("format", uri, None, &[])?;
        if rows.first().and_then(|r| r.first()).is_some_and(|f| f == "refused") {
            return Some(None);
        }
        Some(Some(
            rows.into_iter()
                .filter_map(|r| match <[String; 2]>::try_from(r) {
                    Ok([at, text]) => Some((at, text)),
                    Err(_) => None,
                })
                .collect(),
        ))
    }

    /// The formatting of a document, from `query format`: its runs made in
    /// the text, one edit of the whole document as `sisu fmt` writes it.
    pub(crate) fn stage1_formatting(&mut self, uri: &str) -> Option<Json> {
        let Some(runs) = self.stage1_runs(uri)? else { return Some(Json::Null) };
        let text = self.docs.get(uri).map(|d| d.text.clone()).unwrap_or_default();
        let mut spans = Vec::new();
        for (at, new_text) in &runs {
            let (from, to) = at.split_once('-')?;
            let ((l1, c1), (l2, c2)) = (position(from)?, position(to)?);
            spans.push((offset_at(&text, l1, c1), offset_at(&text, l2, c2), new_text));
        }
        let mut out = text.clone();
        for (s, e, new_text) in spans.into_iter().rev() {
            out.replace_range(s..e, new_text);
        }
        if out == text {
            return Some(Json::Arr(Vec::new()));
        }
        let lines = Lines::new(&text, self.utf8);
        Some(Json::Arr(vec![Json::obj(vec![
            ("range", lines.range(0, text.len())),
            ("newText", Json::str(out)),
        ])]))
    }

    /// The formatting of a range, from `query format`: each run of changed
    /// lines that the range touches and holds, as `sisu fmt` would write it.
    pub(crate) fn stage1_range_formatting(&mut self, uri: &str, within: &Json) -> Option<Json> {
        let Some(runs) = self.stage1_runs(uri)? else { return Some(Json::Null) };
        let text = self.docs.get(uri).map(|d| d.text.clone()).unwrap_or_default();
        let lines = Lines::new(&text, self.utf8);
        let first = within.at(&["start", "line"]).and_then(Json::as_u64).unwrap_or(0) as usize;
        let last = within.at(&["end", "line"]).and_then(Json::as_u64).unwrap_or(u64::MAX) as usize;
        let mut out = Vec::new();
        for (at, new_text) in &runs {
            let Some((a0, a1)) = line_span(at) else { continue };
            let touches = a0 <= last && (a1.max(a0 + 1) - 1) >= first;
            let inside = a0 >= first && a1 <= last.saturating_add(1);
            if touches && inside {
                out.push(Json::obj(vec![
                    ("range", range(at, &lines)?),
                    ("newText", Json::str(new_text.clone())),
                ]));
            }
        }
        Some(Json::Arr(out))
    }
}

/// A document symbol's fields and its members, as `query symbols` gives
/// them.
type Symbol = (Vec<(&'static str, Json)>, Vec<Json>);

/// Where a document's text is written for `--text`: a directory of this
/// server's own under the temporary one, the document's file name in a
/// directory named by its path's hash.
fn buffer_path(path: &Path) -> Option<PathBuf> {
    let hash = sisu_interp::record::content_hash(path.display().to_string().as_bytes());
    let dir = std::env::temp_dir()
        .join(format!("sisu-lsp-buffers-{}", std::process::id()))
        .join(format!("{hash:016x}"));
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join(path.file_name()?))
}

/// The directory of the documents' texts, removed when the server ends.
pub(crate) fn clean() {
    let _ = std::fs::remove_dir_all(
        std::env::temp_dir().join(format!("sisu-lsp-buffers-{}", std::process::id())),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_position_of_an_answer_counts_characters() {
        let text = "def é():\n    pass\n";
        assert_eq!(offset_at(text, 1, 5), 4);
        assert_eq!(offset_at(text, 1, 6), 6);
        assert_eq!(offset_at(text, 2, 5), 14);
        assert_eq!(offset_at(text, 3, 1), text.len());
        assert_eq!(unescaped("a\\tb\\nc\\\\"), "a\tb\nc\\");
    }
}
