//! Every catalog entry is a message some code reports: an entry nothing
//! names is a check that was planned and never written, or one that was
//! removed without its entry. A coded entry that is no longer reported is
//! retired with a comment in the catalog (codes are never reused); an
//! entry without a code is deleted. The code that reports an entry may be
//! the Rust crates' or stage 1's, the front end written in Sisu.

// A test's helpers may fail loudly, as its body may (the panic policy is
// for the crates, not their tests).
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};

fn sources(dir: &Path, out: &mut String) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut paths: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    paths.sort();
    for p in paths {
        if p.is_dir() {
            // Build output is not the compiler's own code.
            if !p.ends_with("target") {
                sources(&p, out);
            }
        } else if p.extension().is_some_and(|e| e == "rs" || e == "sisu") {
            out.push_str(&std::fs::read_to_string(&p).unwrap_or_default());
        }
    }
}

#[test]
fn every_catalog_entry_is_reported_by_some_code() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let catalog = std::fs::read_to_string(root.join("catalog/messages.catalog")).expect("the catalog");
    let mut code = String::new();
    sources(&root.join("crates"), &mut code);
    // Stage 1, the front end written in Sisu, reports entries of its own
    // (the catalog marks their sections "Stage 1's"); its sources name them.
    sources(&root.join("stage1/src"), &mut code);
    sources(&root.join("stage1/std"), &mut code);
    let mut unused = Vec::new();
    for line in catalog.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split_whitespace().collect();
        let [_, _, id, ..] = fields.as_slice() else { continue };
        let name: String =
            id.chars().map(|c| if c == '.' || c == '-' { '_' } else { c.to_ascii_uppercase() }).collect();
        let named = code.match_indices(&name).any(|(i, _)| {
            let before = code[..i].chars().next_back();
            let after = code[i + name.len()..].chars().next();
            let word = |c: Option<char>| c.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
            !word(before) && !word(after)
        });
        if !named && !code.contains(&format!("\"{id}\"")) {
            unused.push(format!("{} {id}", fields[0]));
        }
    }
    assert!(unused.is_empty(), "catalog entries no code reports:\n{}", unused.join("\n"));
}
