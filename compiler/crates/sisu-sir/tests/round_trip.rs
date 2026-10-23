//! The serialised SIR form reads back exactly (decision 212): every `.sir`
//! expectation of the conformance suite, which is what `sisu sir FILE`
//! prints, reads into a program that writes the same bytes again. Stage 1
//! emits this form, and `sisu build --native --from-sir` compiles it.

use std::path::{Path, PathBuf};

fn sir_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            sir_files(&p, out);
        } else if p.extension().is_some_and(|e| e == "sir") {
            out.push(p);
        }
    }
}

#[test]
fn every_sir_expectation_of_the_suite_round_trips() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/cases");
    let mut files = Vec::new();
    sir_files(&root, &mut files);
    assert!(!files.is_empty(), "no `.sir` files under {}", root.display());
    let mut failures = Vec::new();
    for f in &files {
        let text = std::fs::read_to_string(f).unwrap_or_default();
        match sisu_sir::read(&text) {
            Ok(p) => {
                let again = sisu_sir::write(&p);
                if again != text {
                    let line = text.lines().zip(again.lines()).position(|(a, b)| a != b).unwrap_or(0);
                    failures.push(format!(
                        "{}: line {} differs:\n  read:  {}\n  wrote: {}",
                        f.display(),
                        line + 1,
                        text.lines().nth(line).unwrap_or(""),
                        again.lines().nth(line).unwrap_or("")
                    ));
                }
            }
            Err(e) => failures.push(format!("{}: line {}: {}", f.display(), e.line, e.message)),
        }
    }
    assert!(failures.is_empty(), "{} of {} files:\n{}", failures.len(), files.len(), failures.join("\n"));
}
