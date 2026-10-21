//! `sisu check --diary PATH` (18, Phase 3; 20, I-3): the diagnostic log of
//! the corpus study. Each check appends to `PATH`, one JSON object per line
//! (JSONL): a `check` record (the time, the tool, the toolchain, each file
//! checked with the hash of its text), then a `diagnostic` record for every
//! diagnostic (its code and catalog identifier, severity, message and
//! parameters, its span and labels in lines and columns and in bytes, and
//! its fixes, each with its message and edits), then, for each diagnostic
//! of the file's previous check that this one no longer has, a `resolved`
//! record: the diagnostic as it was, the edit that resolved it (the one
//! change between the two texts, as a span of the old text and its
//! replacement), and the fix it applied when the edit is one of its fixes.
//! The text of every file checked is kept by its hash in `PATH.d/`, so the
//! next check can tell what changed, and an ownership rejection's source
//! can be read again (20, A2).

use crate::json::Json;
use sisu_diag::{Diagnostic, FileId, SourceMap, Span};
use std::io::Write;
use std::path::{Path, PathBuf};

/// A file's text hash, as the diary names it (FNV-1a, 64 bits, in hex).
fn hash(text: &str) -> String {
    format!("{:016x}", sisu_interp::record::content_hash(text.as_bytes()))
}

/// RFC 3339 for seconds since the epoch, in UTC.
fn timestamp(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    // Civil from days (H. Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, rem % 3600 / 60, rem % 60)
}

fn span_json(map: &SourceMap, s: Span) -> Json {
    let (a, b) = map.line_col(s);
    Json::obj(vec![
        ("file", Json::str(map.name(s.file))),
        ("start", Json::obj(vec![("line", Json::num(a.line)), ("col", Json::num(a.col))])),
        ("end", Json::obj(vec![("line", Json::num(b.line)), ("col", Json::num(b.col))])),
        ("bytes", Json::Arr(vec![Json::num(s.range.start), Json::num(s.range.end)])),
    ])
}

fn diagnostic_json(map: &SourceMap, d: &Diagnostic) -> Vec<(&'static str, Json)> {
    let catalog = sisu_diag::Catalog::builtin();
    let params = Json::Obj(d.params.iter().map(|(k, v)| (k.to_string(), Json::str(v.clone()))).collect());
    let severity = match d.severity() {
        sisu_diag::Severity::Error => "error",
        sisu_diag::Severity::Warning => "warning",
        sisu_diag::Severity::Note => "note",
    };
    let labels: Vec<Json> = d
        .labels
        .iter()
        .map(|l| {
            Json::obj(vec![
                ("id", Json::str(l.msg.id())),
                ("message", Json::str(catalog.render(l.msg.id(), &l.params))),
                ("span", span_json(map, l.span)),
            ])
        })
        .collect();
    let fixes: Vec<Json> = d
        .fixes
        .iter()
        .map(|f| {
            let edits: Vec<Json> = f
                .edits
                .iter()
                .map(|e| {
                    Json::obj(vec![("span", span_json(map, e.span)), ("text", Json::str(e.text.clone()))])
                })
                .collect();
            Json::obj(vec![
                ("id", Json::str(f.msg.id())),
                ("message", Json::str(catalog.render(f.msg.id(), &f.params))),
                ("edits", Json::Arr(edits)),
            ])
        })
        .collect();
    vec![
        ("code", Json::str(d.code())),
        ("id", Json::str(d.msg.id())),
        ("severity", Json::str(severity)),
        ("message", Json::str(d.message())),
        ("params", params),
        ("span", span_json(map, d.span)),
        ("labels", Json::Arr(labels)),
        ("fixes", Json::Arr(fixes)),
    ]
}

/// The one change between two texts: the bytes `start..end` of `old`
/// replaced by `text`.
fn change(old: &str, new: &str) -> (usize, usize, String) {
    let prefix = old.bytes().zip(new.bytes()).take_while(|(a, b)| a == b).count();
    let mut prefix = prefix;
    while !old.is_char_boundary(prefix) || !new.is_char_boundary(prefix) {
        prefix -= 1;
    }
    let max = old.len().min(new.len()) - prefix;
    let mut suffix = old.bytes().rev().zip(new.bytes().rev()).take_while(|(a, b)| a == b).count().min(max);
    while !old.is_char_boundary(old.len() - suffix) || !new.is_char_boundary(new.len() - suffix) {
        suffix -= 1;
    }
    (prefix, old.len() - suffix, new[prefix..new.len() - suffix].to_string())
}

/// What the diary needs of a diagnostic of the last check to tell whether
/// this check still has it: its identifier, parameters and bytes.
struct Earlier {
    record: Json,
    id: String,
    params: Json,
    start: usize,
    end: usize,
    /// Its fixes' edits, as (start, end, text) of the old text.
    fixes: Vec<(String, Vec<Edit>)>,
}

/// An edit of a fix: (start, end, text) of the old text.
type Edit = (usize, usize, String);

fn earlier(record: &Json) -> Option<Earlier> {
    let bytes = record.at(&["span", "bytes"])?.as_array()?;
    let fixes = record
        .get("fixes")
        .and_then(Json::as_array)
        .unwrap_or(&[])
        .iter()
        .map(|f| {
            let id = f.get("id").and_then(Json::as_str).unwrap_or("").to_string();
            let edits = f
                .get("edits")
                .and_then(Json::as_array)
                .unwrap_or(&[])
                .iter()
                .filter_map(|e| {
                    let b = e.at(&["span", "bytes"])?.as_array()?;
                    Some((
                        b.first()?.as_u64()? as usize,
                        b.get(1)?.as_u64()? as usize,
                        e.get("text")?.as_str()?.to_string(),
                    ))
                })
                .collect();
            (id, edits)
        })
        .collect();
    Some(Earlier {
        record: record.clone(),
        id: record.get("id")?.as_str()?.to_string(),
        params: record.get("params").cloned().unwrap_or(Json::Null),
        start: bytes.first()?.as_u64()? as usize,
        end: bytes.get(1)?.as_u64()? as usize,
        fixes,
    })
}

/// Appends this check's records to the diary at `path`: the program's own
/// files (by their names and texts) and every diagnostic.
pub fn append(
    path: &str,
    now: i64,
    files: &[(String, String)],
    map: &SourceMap,
    diags: &[Diagnostic],
) -> Result<(), String> {
    let diary = PathBuf::from(path);
    let store = PathBuf::from(format!("{path}.d"));
    let previous = std::fs::read_to_string(&diary).unwrap_or_default();
    let mut lines: Vec<String> = Vec::new();
    let time = timestamp(now);
    let file_list: Vec<Json> = files
        .iter()
        .map(|(n, t)| Json::obj(vec![("name", Json::str(n.clone())), ("hash", Json::str(hash(t)))]))
        .collect();
    lines.push(
        Json::obj(vec![
            ("record", Json::str("check")),
            ("time", Json::str(time.clone())),
            ("tool", Json::str("sisu check")),
            ("toolchain", Json::str(crate::replay::TOOLCHAIN)),
            ("files", Json::Arr(file_list)),
            ("diagnostics", Json::num(diags.len() as f64)),
        ])
        .write(),
    );
    for d in diags {
        let file = map.name(d.span.file).to_string();
        let text = files.iter().find(|(n, _)| *n == file).map(|(_, t)| hash(t)).unwrap_or_default();
        let mut fields = vec![
            ("record", Json::str("diagnostic")),
            ("time", Json::str(time.clone())),
            ("file", Json::str(file)),
            ("hash", Json::str(text)),
        ];
        fields.extend(diagnostic_json(map, d));
        lines.push(Json::obj(fields).write());
    }
    // What each file's last check had that this one no longer does.
    let records: Vec<Json> = previous.lines().filter_map(|l| crate::json::parse(l).ok()).collect();
    for (name, text) in files {
        let now_hash = hash(text);
        let Some(last) = records.iter().rev().find(|r| {
            r.get("record").and_then(Json::as_str) == Some("check")
                && r.get("files")
                    .and_then(Json::as_array)
                    .is_some_and(|fs| fs.iter().any(|f| f.get("name").and_then(Json::as_str) == Some(name)))
        }) else {
            continue;
        };
        let old_hash = last
            .get("files")
            .and_then(Json::as_array)
            .and_then(|fs| fs.iter().find(|f| f.get("name").and_then(Json::as_str) == Some(name)))
            .and_then(|f| f.get("hash").and_then(Json::as_str))
            .unwrap_or("")
            .to_string();
        if old_hash == now_hash {
            continue;
        }
        let Ok(old_text) = std::fs::read_to_string(store.join(&old_hash)) else { continue };
        let last_time = last.get("time").cloned().unwrap_or(Json::Null);
        let before: Vec<Earlier> = records
            .iter()
            .filter(|r| {
                r.get("record").and_then(Json::as_str) == Some("diagnostic")
                    && r.get("file").and_then(Json::as_str) == Some(name)
                    && r.get("hash").and_then(Json::as_str) == Some(old_hash.as_str())
                    && r.get("time") == Some(&last_time)
            })
            .filter_map(earlier)
            .collect();
        let (start, end, inserted) = change(&old_text, text);
        let delta = inserted.len() as i64 - (end - start) as i64;
        for e in before {
            // Where the diagnostic would be now, if the change left it.
            let moved = if e.end <= start {
                Some((e.start, e.end))
            } else if e.start >= end {
                Some(((e.start as i64 + delta) as usize, (e.end as i64 + delta) as usize))
            } else {
                None
            };
            let still = diags.iter().any(|d| {
                map.name(d.span.file) == name
                    && d.msg.id() == e.id
                    && moved.is_some_and(|(s, t)| {
                        d.span.range.start as usize == s && d.span.range.end as usize == t
                    })
                    && Json::Obj(
                        d.params.iter().map(|(k, v)| (k.to_string(), Json::str(v.clone()))).collect(),
                    ) == e.params
            });
            if still {
                continue;
            }
            // The fix it applied: one whose edits make the new text of the
            // old one.
            let applied = e
                .fixes
                .iter()
                .find(|(_, edits)| {
                    !edits.is_empty() && apply(&old_text, edits).as_deref() == Some(text.as_str())
                })
                .map(|(id, _)| Json::str(id.clone()))
                .unwrap_or(Json::Null);
            let (a, b) = (line_col(&old_text, start), line_col(&old_text, end));
            let edit = Json::obj(vec![
                (
                    "span",
                    Json::obj(vec![
                        ("start", Json::obj(vec![("line", Json::num(a.0)), ("col", Json::num(a.1))])),
                        ("end", Json::obj(vec![("line", Json::num(b.0)), ("col", Json::num(b.1))])),
                        ("bytes", Json::Arr(vec![Json::num(start as f64), Json::num(end as f64)])),
                    ]),
                ),
                ("text", Json::str(inserted.clone())),
            ]);
            lines.push(
                Json::obj(vec![
                    ("record", Json::str("resolved")),
                    ("time", Json::str(time.clone())),
                    ("file", Json::str(name.clone())),
                    ("from", Json::str(old_hash.clone())),
                    ("to", Json::str(now_hash.clone())),
                    ("diagnostic", e.record),
                    ("edit", edit),
                    ("fix", applied),
                ])
                .write(),
            );
        }
    }
    // The texts, by hash, for the next check and for reading a rejected
    // program again.
    std::fs::create_dir_all(&store).map_err(|e| e.to_string())?;
    for (_, text) in files {
        let p = store.join(hash(text));
        if !p.exists() {
            std::fs::write(&p, text).map_err(|e| e.to_string())?;
        }
    }
    if let Some(dir) = Path::new(path).parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let mut f =
        std::fs::OpenOptions::new().create(true).append(true).open(&diary).map_err(|e| e.to_string())?;
    for l in lines {
        writeln!(f, "{l}").map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// A text with edits (byte ranges of it and their replacements) made, or
/// `None` when they overlap or reach outside it.
fn apply(text: &str, edits: &[(usize, usize, String)]) -> Option<String> {
    let mut sorted = edits.to_vec();
    sorted.sort_by_key(|e| e.0);
    let mut out = String::new();
    let mut at = 0;
    for (s, e, t) in sorted {
        if s < at || e > text.len() || s > e {
            return None;
        }
        out.push_str(text.get(at..s)?);
        out.push_str(&t);
        at = e;
    }
    out.push_str(text.get(at..)?);
    Some(out)
}

/// The 1-based line and column (in characters) of a byte of a text.
fn line_col(text: &str, at: usize) -> (u32, u32) {
    let before = &text[..at.min(text.len())];
    let line = before.matches('\n').count() as u32 + 1;
    let col = before.rsplit('\n').next().unwrap_or("").chars().count() as u32 + 1;
    (line, col)
}

/// The program's own files: their names as diagnostics give them, and
/// their texts.
pub fn own_files(map: &SourceMap, count: usize) -> Vec<(String, String)> {
    (0..count as u32).filter_map(|i| map.get(FileId(i)).map(|f| (f.name.clone(), f.text.clone()))).collect()
}
