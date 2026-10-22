//! `sisu-conformance [--bless] [--sisu PATH] [ROOT]`: runs the suite with
//! the `sisu` executable at PATH (default: `sisu` on the search path)
//! over the cases under ROOT (default: `conformance/cases`).
//!
//! `--native LIST` builds and runs natively the cases the coverage list
//! names instead (`sisu build --native`), `--native-from-sir LIST` builds
//! them from serialised SIR (`sisu build --native --from-sir`: each case's
//! own `.sir`, else what `sisu sir` prints), and `--native-survey LIST`
//! tries every runnable case the list leaves out, printing which would
//! pass.

use std::path::PathBuf;

fn main() {
    let mut bless = false;
    let mut sisu = PathBuf::from("sisu");
    let mut root = PathBuf::from("conformance/cases");
    let mut native: Option<PathBuf> = None;
    let mut survey = false;
    let mut from_sir = false;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--bless" => bless = true,
            "--sisu" => {
                if let Some(p) = args.next() {
                    sisu = PathBuf::from(p);
                }
            }
            "--native" | "--native-survey" | "--native-from-sir" => {
                survey = a == "--native-survey";
                from_sir = a == "--native-from-sir";
                if let Some(p) = args.next() {
                    native = Some(PathBuf::from(p));
                }
            }
            other => root = PathBuf::from(other),
        }
    }
    let sisu = std::fs::canonicalize(&sisu).unwrap_or(sisu);
    if let Some(list) = native {
        let text = std::fs::read_to_string(&list).unwrap_or_default();
        let names = sisu_conformance::native::read_list(&text);
        if survey {
            for (case, verdict) in sisu_conformance::native::survey(&sisu, &root, &names) {
                println!("{case}: {verdict}");
            }
            return;
        }
        let report = if from_sir {
            sisu_conformance::native::run_native_from_sir(&sisu, &root, &names)
        } else {
            sisu_conformance::native::run_native(&sisu, &root, &names)
        };
        print!("{}", report.render());
        std::process::exit(if report.failures.is_empty() { 0 } else { 1 });
    }
    let report = sisu_conformance::run_all(&sisu, &root, bless);
    print!("{}", report.render());
    std::process::exit(if report.failures.is_empty() { 0 } else { 1 });
}
