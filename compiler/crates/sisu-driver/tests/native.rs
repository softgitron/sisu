//! The native backend (stage 2) against the conformance suite: every case
//! of `conformance/native.list` built with `sisu build --native` and run,
//! with the expectations the interpreter meets.

use std::path::PathBuf;

fn compiler_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn native_conformance() {
    let sisu = PathBuf::from(env!("CARGO_BIN_EXE_sisu"));
    let root = compiler_dir().join("conformance/cases");
    let text =
        std::fs::read_to_string(compiler_dir().join("conformance/native.list")).expect("the coverage list");
    let list = sisu_conformance::native::read_list(&text);
    let report = sisu_conformance::native::run_native(&sisu, &root, &list);
    println!("{}", report.render());
    assert!(report.cases > 0, "no cases in the coverage list");
    assert!(report.failures.is_empty(), "{}", report.render());
}

/// `sisu run --native` builds a program, runs it with its arguments and
/// gives its exit status; the executable is gone afterwards.
#[test]
fn run_native_runs_the_program() {
    let sisu = PathBuf::from(env!("CARGO_BIN_EXE_sisu"));
    let dir = std::env::temp_dir().join(format!("sisu-native-run-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    std::fs::write(dir.join("count.sisu"), "def main():\n    var n = 0\n    for i in range(5):\n        n += i\n    print(n)\n    panic(\"stop at {n}\")\n")
        .expect("the program");
    let out = std::process::Command::new(&sisu)
        .args(["run", "--native", "count.sisu"])
        .current_dir(&dir)
        .output()
        .expect("sisu runs");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "10\n");
    assert_eq!(out.status.code(), Some(101));
    assert!(String::from_utf8_lossy(&out.stderr).starts_with("panic: stop at 10\n"));
    assert!(!dir.join("count").exists());
    let built = std::process::Command::new(&sisu)
        .args(["build", "--native", "count.sisu"])
        .current_dir(&dir)
        .output()
        .expect("sisu builds");
    assert!(built.status.success(), "{}", String::from_utf8_lossy(&built.stderr));
    assert!(dir.join("count").is_file());
    let _ = std::fs::remove_dir_all(&dir);
}
