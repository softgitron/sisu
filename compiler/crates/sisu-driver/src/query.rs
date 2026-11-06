//! The language server's answers from stage 1's queries (decision 983):
//! `sisu1 query KIND FILE [LINE COL] --text PATH` (its R164, `sisu1
//! 0.14.0` and later), served by the driver's server of stage 1 (decision
//! 980), for the diagnostics, hover, go-to-definition, references,
//! completion and document symbols. A document's text is written to a
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
