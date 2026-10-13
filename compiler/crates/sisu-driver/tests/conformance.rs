//! Runs the conformance suite (compiler/conformance) against the `sisu`
//! executable this package builds. `SISU_BLESS=1 cargo test` rewrites the
//! expected files from the actual output instead of failing.

use std::path::PathBuf;

#[test]
fn conformance_suite() {
    let sisu = PathBuf::from(env!("CARGO_BIN_EXE_sisu"));
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance/cases");
    let bless = std::env::var_os("SISU_BLESS").is_some();
    let report = sisu_conformance::run_all(&sisu, &root, bless);
    println!("{}", report.render());
    assert!(report.cases > 0, "no cases under {}", root.display());
    assert!(report.failures.is_empty(), "{}", report.render());
}

/// The suite's copies of the proposal's examples must stay identical to
/// the originals.
#[test]
fn example_copies_match_the_proposal() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    for name in ["hello.sisu", "wordcount.sisu"] {
        let original =
            std::fs::read_to_string(root.join("../proposal/examples").join(name)).unwrap_or_default();
        let copy =
            std::fs::read_to_string(root.join("conformance/cases/examples").join(name)).unwrap_or_default();
        assert!(!original.is_empty(), "missing proposal/examples/{name}");
        assert_eq!(original, copy, "conformance/cases/examples/{name} differs from proposal/examples/{name}");
    }
}
