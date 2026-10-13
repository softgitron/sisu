//! `sisu-conformance [--bless] [--sisu PATH] [ROOT]`: runs the suite with
//! the `sisu` executable at PATH (default: `sisu` on the search path)
//! over the cases under ROOT (default: `conformance/cases`).

use std::path::PathBuf;

fn main() {
    let mut bless = false;
    let mut sisu = PathBuf::from("sisu");
    let mut root = PathBuf::from("conformance/cases");
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--bless" => bless = true,
            "--sisu" => {
                if let Some(p) = args.next() {
                    sisu = PathBuf::from(p);
                }
            }
            other => root = PathBuf::from(other),
        }
    }
    let sisu = std::fs::canonicalize(&sisu).unwrap_or(sisu);
    let report = sisu_conformance::run_all(&sisu, &root, bless);
    print!("{}", report.render());
    std::process::exit(if report.failures.is_empty() { 0 } else { 1 });
}
