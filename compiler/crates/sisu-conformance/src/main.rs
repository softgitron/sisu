//! `sisu-conformance [--bless] [--sisu PATH] [--front-end=NAME] [ROOT]`:
//! runs the suite with the `sisu` executable at PATH (default: `sisu` on
//! the search path) over the cases under ROOT (default:
//! `conformance/cases`). `--front-end=stage1` or `--front-end=rust` asks
//! the executable for that front end (`SISU_FRONT_END`, decision 268);
//! without it, the executable's default, stage 1.
//!
//! `--native LIST` builds and runs natively the cases the coverage list
//! names instead (`sisu build --native`), `--native-from-sir LIST` builds
//! them from serialised SIR (`sisu build --native --from-sir`: each case's
//! own `.sir`, else what `sisu sir` prints), `--native-release LIST` with
//! the release profile (`sisu build --native --release`), and
//! `--native-survey LIST` tries every runnable case the list leaves out,
//! printing which would pass. `--interpreted-from-sir LIST` runs the
//! list's cases in the interpreter from serialised SIR (`sisu run
//! --from-sir`). `--native-from-lir LIST` lowers each case's SIR with stage
//! 2 (`sisu2 lir`) and builds it from LIR (`sisu build --native
//! --from-lir`), whose executable must be the one its SIR gives, byte for
//! byte; stage 2 is `--sisu2 PATH`, else built from `ROOT/../../stage2`
//! with the `sisu` under test.

use std::path::PathBuf;

fn main() {
    let mut bless = false;
    let mut sisu = PathBuf::from("sisu");
    let mut root = PathBuf::from("conformance/cases");
    let mut native: Option<PathBuf> = None;
    let mut survey = false;
    let mut from_sir = false;
    let mut release = false;
    let mut interpreted = false;
    let mut from_lir = false;
    let mut sisu2: Option<PathBuf> = None;
    let mut front_end: Option<String> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if let Some(name) = a.strip_prefix("--front-end=") {
            front_end = Some(name.to_string());
            continue;
        }
        match a.as_str() {
            "--bless" => bless = true,
            "--front-end" => front_end = args.next(),
            "--sisu" => {
                if let Some(p) = args.next() {
                    sisu = PathBuf::from(p);
                }
            }
            "--sisu2" => sisu2 = args.next().map(PathBuf::from),
            "--native"
            | "--native-survey"
            | "--native-from-sir"
            | "--native-release"
            | "--native-from-lir"
            | "--interpreted-from-sir" => {
                survey = a == "--native-survey";
                from_sir = a == "--native-from-sir";
                release = a == "--native-release";
                interpreted = a == "--interpreted-from-sir";
                from_lir = a == "--native-from-lir";
                if let Some(p) = args.next() {
                    native = Some(PathBuf::from(p));
                }
            }
            other => root = PathBuf::from(other),
        }
    }
    if let Some(name) = front_end.as_deref()
        && !matches!(name, "stage1" | "rust")
    {
        eprintln!("--front-end takes `stage1` or `rust`, not `{name}`");
        std::process::exit(2);
    }
    let sisu = std::fs::canonicalize(&sisu).unwrap_or(sisu);
    let sisu = sisu_conformance::Sisu::new(sisu).with_front_end(front_end.as_deref());
    if let Some(list) = native {
        let text = std::fs::read_to_string(&list).unwrap_or_default();
        let names = if from_lir {
            sisu_conformance::native::read_list_from_lir(&text)
        } else {
            sisu_conformance::native::read_list(&text)
        };
        if survey {
            for (case, verdict) in sisu_conformance::native::survey(&sisu, &root, &names) {
                println!("{case}: {verdict}");
            }
            return;
        }
        let report = if from_lir {
            let (stage2, built) = match sisu2 {
                Some(p) => (std::fs::canonicalize(&p).unwrap_or(p), false),
                None => {
                    let dir = root.join("../../stage2");
                    match sisu_conformance::native::build_stage2(&sisu, &dir) {
                        Ok(exe) => (exe, true),
                        Err(e) => {
                            eprintln!("stage 2 does not build in `{}`:\n{e}", dir.display());
                            std::process::exit(1);
                        }
                    }
                }
            };
            let report = sisu_conformance::native::run_native_from_lir(&sisu, &root, &names, &stage2);
            // The stage 2 built for the run goes with it.
            if built && let Some(dir) = stage2.parent() {
                let _ = std::fs::remove_dir_all(dir);
            }
            report
        } else if interpreted {
            sisu_conformance::native::run_interpreted_from_sir(&sisu, &root, &names)
        } else if from_sir {
            sisu_conformance::native::run_native_from_sir(&sisu, &root, &names)
        } else if release {
            sisu_conformance::native::run_native_release(&sisu, &root, &names)
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
