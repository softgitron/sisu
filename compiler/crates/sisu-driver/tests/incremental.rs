//! The per-item queries (docs/architecture.md, "The query layer") against
//! checking from nothing: for the standard library's modules and the
//! conformance cases, each opened as a program, an edit of a function body
//! is checked by the database that saw the program before and by a fresh
//! one, and the two must give the same diagnostics and the same SIR. The
//! re-check's time is printed against 10's target of 300 ms for one edited
//! body (`cargo test --release --test incremental -- --nocapture`;
//! `SISU_INCREMENTAL_ALL=1` takes every case instead of a sample).

// A test's helpers may fail loudly, as its body may (the panic policy is
// for the crates, not their tests).
#![allow(clippy::expect_used, clippy::unwrap_used)]

use sisu_driver::Db;
use sisu_syntax::ast;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn compiler_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every `.sisu` file under `dir` that is a case of its own (not a
/// package's module or a case's fixture).
fn cases(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> =
        std::fs::read_dir(dir).expect("a directory").filter_map(|e| e.ok().map(|e| e.path())).collect();
    entries.sort();
    for p in entries {
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if p.is_dir() {
            if !name.ends_with(".pkg") && !name.ends_with(".files") {
                cases(&p, out);
            }
        } else if name.ends_with(".sisu") {
            out.push(p);
        }
    }
}

/// Where each function body with a statement starts: the start of its
/// first statement.
fn bodies(text: &str) -> Vec<usize> {
    let parse = sisu_parser::parse(sisu_diag::FileId(0), text);
    let module = ast::build::module(&parse.tree);
    let mut out = Vec::new();
    let mut fns: Vec<&ast::FnDef> = Vec::new();
    for item in &module.items {
        match &item.kind {
            ast::ItemKind::Fn(f) => fns.push(f),
            ast::ItemKind::Struct(s) => fns.extend(&s.methods),
            ast::ItemKind::Enum(e) | ast::ItemKind::ErrorSet(e) => fns.extend(&e.methods),
            ast::ItemKind::Impl(i) => fns.extend(&i.methods),
            _ => {}
        }
    }
    for f in fns {
        if let Some(s) = f.body.as_ref().and_then(|b| b.stmts.first()) {
            out.push(s.range.start as usize);
        }
    }
    out.sort();
    out
}

/// Two edits of the body whose first statement starts at `at`: a `pass`
/// line before it (every later item moves down a line), and a space at the
/// end of its line (every later item moves by a byte, on the same line).
fn edits(text: &str, at: usize) -> [String; 2] {
    let line_start = text[..at].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let indent = &text[line_start..at];
    let line_end = text[at..].find('\n').map(|i| at + i).unwrap_or(text.len());
    [
        format!("{}{indent}pass\n{}", &text[..line_start], &text[line_start..]),
        format!("{} {}", &text[..line_end], &text[line_end..]),
    ]
}

/// Everything the database says of a program: its diagnostics and its SIR.
fn outcome(db: &mut Db, name: &str) -> (Vec<sisu_diag::Diagnostic>, Option<String>) {
    let d = db.diagnostics(name);
    let sir = db.sir(name).map(|p| sisu_sir::write::write(&p));
    (d, sir)
}

#[test]
fn a_body_edit_checks_what_a_fresh_check_does() {
    let all = std::env::var("SISU_INCREMENTAL_ALL").is_ok();
    let root = compiler_dir();
    let mut files: Vec<PathBuf> = Vec::new();
    let mut std_files: Vec<PathBuf> = std::fs::read_dir(root.join("std"))
        .expect("std")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "sisu"))
        .collect();
    std_files.sort();
    files.extend(std_files);
    let mut all_cases = Vec::new();
    cases(&root.join("conformance/cases"), &mut all_cases);
    let step = if all { 1 } else { 16 };
    files.extend(all_cases.into_iter().step_by(step));
    let mut warm: Vec<(Duration, String)> = Vec::new();
    let mut fresh_times: Vec<Duration> = Vec::new();
    let mut reused = 0u64;
    let mut checked = 0u64;
    for path in &files {
        let Ok(text) = std::fs::read_to_string(path) else { continue };
        let name = path.display().to_string();
        let mut db = Db::new();
        db.set_file(&name, &text);
        let _ = outcome(&mut db, &name);
        let starts = bodies(&text);
        if starts.is_empty() {
            continue;
        }
        let picks: Vec<usize> = if all {
            vec![starts[0], starts[starts.len() / 2], starts[starts.len() - 1]]
        } else {
            vec![starts[starts.len() / 2]]
        };
        for at in picks {
            for edited in edits(&text, at) {
                db.set_file(&name, &edited);
                let t = Instant::now();
                let incremental = outcome(&mut db, &name);
                let w = t.elapsed();
                checked += u64::from(db.counters.bodies_checked);
                reused += u64::from(db.counters.bodies_reused);
                let mut fresh_db = Db::new();
                fresh_db.set_file(&name, &edited);
                let t = Instant::now();
                let fresh = outcome(&mut fresh_db, &name);
                fresh_times.push(t.elapsed());
                assert!(
                    incremental.0 == fresh.0,
                    "{name}: the diagnostics differ after an edit at byte {at}"
                );
                assert!(incremental.1 == fresh.1, "{name}: the SIR differs after an edit at byte {at}");
                warm.push((w, name.clone()));
            }
        }
    }
    warm.sort();
    fresh_times.sort();
    let median = |v: &[Duration]| v.get(v.len() / 2).copied().unwrap_or_default();
    let worst = warm.last().cloned().unwrap_or_default();
    println!(
        "{} edits of {} programs: re-check median {} ms, worst {} ms ({}); from nothing median {} ms; \
         bodies checked {checked}, reused {reused}",
        warm.len(),
        files.len(),
        median(&warm.iter().map(|(d, _)| *d).collect::<Vec<_>>()).as_millis(),
        worst.0.as_millis(),
        worst.1,
        median(&fresh_times).as_millis(),
    );
    assert!(!warm.is_empty());
}

/// A one-body edit checks that body again and reuses every other.
#[test]
fn a_body_edit_rechecks_one_body() {
    let text = "\
def double(x: int) -> int:
    return x * 2

def triple(x: int) -> int:
    return x * 3

def main():
    print(double(2) + triple(3))
";
    let mut db = Db::new();
    db.set_file("p.sisu", text);
    assert!(db.diagnostics("p.sisu").is_empty());
    let edited = text.replace("return x * 2", "let y = x\n    return y * 2");
    db.set_file("p.sisu", &edited);
    assert!(db.diagnostics("p.sisu").is_empty());
    assert_eq!(db.counters.bodies_checked, 1, "{:?}", db.counters);
    assert!(db.counters.bodies_reused > 100, "{:?}", db.counters);
    // `triple` and `main` moved down a line: lowered again, since their
    // SIR names their lines, but not checked again.
    assert_eq!(db.counters.bodies_lowered, 3, "{:?}", db.counters);
    // An edit inside a line moves what follows by bytes only: only the
    // edited body is checked and lowered again.
    let edited2 = edited.replace("return x * 3", "return x * 3 + 0");
    db.set_file("p.sisu", &edited2);
    assert!(db.diagnostics("p.sisu").is_empty());
    assert_eq!(db.counters.bodies_checked, 1, "{:?}", db.counters);
    assert_eq!(db.counters.bodies_lowered, 1, "{:?}", db.counters);
    // A signature edit changes the interface: everything is checked again.
    let edited3 = edited2.replace("def triple(x: int) -> int:", "def triple(x: int, k: int = 3) -> int:");
    db.set_file("p.sisu", &edited3);
    assert!(db.diagnostics("p.sisu").is_empty());
    assert!(db.counters.bodies_checked > 100, "{:?}", db.counters);
}
