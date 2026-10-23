//! The wrong-program corpus (`conformance/cases/wrong/`, docs/architecture.md,
//! "Corpus tooling"): each program carries one intended mistake, and the
//! checker must report exactly one error for it, and that error must name
//! its fix, as the catalog asks of every diagnostic: a fix, or a note that
//! says what to write. A fix with edits must be one that fixes: applying
//! it, and then the fix of each error that follows from it (a parameter
//! made `mut` asks for `mut` at its calls), leaves a program with no
//! errors. The `.diag` beside each case pins the error's identifier and
//! spans, and the conformance runner compares it.
//!
//! The programs whose error is a rule of the ownership model are the
//! wrong-ownership corpus of 18's Phase 3 ("a 60-file wrong-ownership
//! corpus with expected codes and named fixes"): the ownership checker's
//! codes (`E04xx`), and the type checker's that are about ownership
//! ([`OWNERSHIP_CODES`]).

// A test's helpers may fail loudly, as its body may (the panic policy is
// for the crates, not their tests).
#![allow(clippy::expect_used, clippy::unwrap_used)]

use sisu_diag::{Diagnostic, Edit};
use sisu_driver::Db;
use std::path::PathBuf;

/// The type checker's codes that are rules of the ownership model: a move
/// out of a borrowed value, `mut` at a call, a change through what does
/// not own or lend it with `mut`, a function value that returns a borrow,
/// a use before assignment and a second assignment to a `let` (the
/// initialisation dataflow's), a view made a `dyn`, and a must-consume
/// value given to a generic parameter.
const OWNERSHIP_CODES: [&str; 9] =
    ["E0338", "E0339", "E0340", "E0341", "E0352", "E0366", "E0367", "E0396", "E0397"];

/// `text` with `edits` applied, the last first so the earlier offsets
/// stay right.
fn apply(text: &str, edits: &[&Edit]) -> String {
    let mut edits = edits.to_vec();
    edits.sort_by_key(|e| std::cmp::Reverse(e.span.range.start));
    let mut out = text.to_string();
    for e in edits {
        out.replace_range(e.span.range.start as usize..e.span.range.end as usize, &e.text);
    }
    out
}

fn errors(name: &str, text: &str) -> Vec<Diagnostic> {
    let mut db = Db::new();
    db.set_file(name, text);
    db.diagnostics(name).into_iter().filter(|d| d.is_error()).collect()
}

/// The edits of a fix that fall in the program's file.
fn edits(d: &Diagnostic, fix: &sisu_diag::Fix) -> Vec<Edit> {
    fix.edits.iter().filter(|e| e.span.file == d.span.file).cloned().collect()
}

/// Applies the fix of the first error left, round after round, until no
/// error is left: the codes of the errors left when one has no fix with
/// edits, or after five rounds.
fn follow(name: &str, mut text: String) -> Result<(), Vec<&'static str>> {
    for _ in 0..5 {
        let left = errors(name, &text);
        let Some(d) = left.first() else { return Ok(()) };
        let Some(edits) = d.fixes.iter().map(|f| edits(d, f)).find(|e| !e.is_empty()) else {
            return Err(left.iter().map(|d| d.code()).collect());
        };
        text = apply(&text, &edits.iter().collect::<Vec<_>>());
    }
    Err(errors(name, &text).iter().map(|d| d.code()).collect())
}

#[test]
fn every_wrong_program_has_one_error_that_names_its_fix() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance/cases/wrong");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("the corpus")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "sisu"))
        .collect();
    files.sort();
    assert!(files.len() >= 40, "the corpus has {} programs", files.len());
    let mut failures: Vec<String> = Vec::new();
    let mut fixes = 0;
    let mut ownership = 0;
    for path in &files {
        let text = std::fs::read_to_string(path).expect("a case");
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        if !text.starts_with("# A wrong program") {
            failures.push(format!("{name}: the first line does not say what the mistake is"));
        }
        let found = errors(&name, &text);
        let [d] = found.as_slice() else {
            let codes: Vec<&str> = found.iter().map(|d| d.code()).collect();
            failures.push(format!("{name}: {} errors ({codes:?}), not one", found.len()));
            continue;
        };
        if d.fixes.is_empty() && d.notes.is_empty() {
            failures.push(format!("{name}: {} names no fix", d.code()));
        }
        if d.code().starts_with("E04") || OWNERSHIP_CODES.contains(&d.code()) {
            ownership += 1;
        }
        for fix in &d.fixes {
            let edits = edits(d, fix);
            if edits.is_empty() {
                continue;
            }
            fixes += 1;
            if let Err(left) = follow(&name, apply(&text, &edits.iter().collect::<Vec<_>>())) {
                failures.push(format!("{name}: after its fix {}, errors remain: {left:?}", fix.msg.id()));
            }
        }
    }
    if ownership < 60 {
        failures.push(format!("the wrong-ownership corpus has {ownership} programs, not 60"));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    println!(
        "{} wrong programs ({ownership} of the ownership model), one error each; {fixes} fixes applied and checked",
        files.len()
    );
}
