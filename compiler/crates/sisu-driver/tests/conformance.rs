//! Runs the conformance suite (compiler/conformance) against the `sisu`
//! executable this package builds. `SISU_BLESS=1 cargo test` rewrites the
//! expected files from the actual output instead of failing.

use std::path::PathBuf;

#[test]
fn conformance_suite() {
    let sisu = sisu_conformance::Sisu::new(env!("CARGO_BIN_EXE_sisu"));
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

/// The examples that have no `main` of their own run as the proposal's
/// text followed by one: each case must start with the whole original.
#[test]
fn examples_with_a_main_start_with_the_proposal() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    for name in ["traits.sisu", "errors.sisu", "capabilities.sisu", "concurrency.sisu"] {
        let original =
            std::fs::read_to_string(root.join("../proposal/examples").join(name)).unwrap_or_default();
        let case =
            std::fs::read_to_string(root.join("conformance/cases/examples").join(name)).unwrap_or_default();
        assert!(!original.is_empty(), "missing proposal/examples/{name}");
        assert!(
            case.starts_with(&original),
            "conformance/cases/examples/{name} no longer starts with proposal/examples/{name}"
        );
    }
}

/// proposal/examples/contracts.sisu runs as the case
/// `contracts/proposal_contracts.sisu`: the example unchanged, then what it
/// leaves undeclared (`Money`, `TcpListener`) and a `main`.
#[test]
fn contracts_example_starts_with_the_proposal() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let original =
        std::fs::read_to_string(root.join("../proposal/examples/contracts.sisu")).unwrap_or_default();
    assert!(!original.is_empty(), "missing proposal/examples/contracts.sisu");
    let case = std::fs::read_to_string(root.join("conformance/cases/contracts/proposal_contracts.sisu"))
        .unwrap_or_default();
    assert!(
        case.starts_with(&original),
        "contracts/proposal_contracts.sisu no longer starts with proposal/examples/contracts.sisu"
    );
}

/// The part of proposal/examples/ownership.sisu that stage 0 can run
/// (everything before `Shared`, which needs `Mutex`, `Arena` and tasks) is
/// the conformance case `ownership/proposal_core.sisu`, kept identical.
#[test]
fn ownership_core_matches_the_proposal() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let original =
        std::fs::read_to_string(root.join("../proposal/examples/ownership.sisu")).unwrap_or_default();
    let core = original.split("# Shared ownership is a type you choose.").next().unwrap_or("").trim_end();
    assert!(!core.is_empty(), "missing proposal/examples/ownership.sisu");
    let case = std::fs::read_to_string(root.join("conformance/cases/ownership/proposal_core.sisu"))
        .unwrap_or_default();
    assert!(case.starts_with(core), "ownership/proposal_core.sisu no longer starts with the proposal's text");
}

/// The `Shared` section of proposal/examples/ownership.sisu (`shared_demo`
/// and `counter_demo`, which need tasks) is the case
/// `ownership/proposal_shared.sisu`, kept identical.
#[test]
fn ownership_shared_matches_the_proposal() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let original =
        std::fs::read_to_string(root.join("../proposal/examples/ownership.sisu")).unwrap_or_default();
    let start = original.find("# Shared ownership is a type you choose.");
    let end = original.find("# Graphs: an arena with generational handles, not pointers.");
    assert!(start.is_some() && end.is_some(), "proposal/examples/ownership.sisu has no Shared section");
    let section = original[start.unwrap_or(0)..end.unwrap_or(0)].trim_end();
    let case = std::fs::read_to_string(root.join("conformance/cases/ownership/proposal_shared.sisu"))
        .unwrap_or_default();
    assert!(
        case.contains(section),
        "ownership/proposal_shared.sisu no longer holds the proposal's Shared section"
    );
}

/// The graph section of proposal/examples/ownership.sisu (`Arena` and
/// `Key`) is the case `ownership/proposal_graph.sisu`, kept identical.
#[test]
fn ownership_graph_matches_the_proposal() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let original =
        std::fs::read_to_string(root.join("../proposal/examples/ownership.sisu")).unwrap_or_default();
    let start = original.find("# Graphs: an arena with generational handles, not pointers.");
    assert!(start.is_some(), "proposal/examples/ownership.sisu has no graph section");
    let graph = original[start.unwrap_or(0)..].trim_end();
    let case = std::fs::read_to_string(root.join("conformance/cases/ownership/proposal_graph.sisu"))
        .unwrap_or_default();
    assert!(
        case.contains(graph),
        "ownership/proposal_graph.sisu no longer holds the proposal's graph section"
    );
}

/// The user-defined derive of proposal/examples/comptime.sisu (the trait
/// `Describe` with its `comptime def derive`, and `Point`) is the case
/// `comptime/proposal_derive.sisu`, kept identical.
#[test]
fn comptime_derive_matches_the_proposal() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let original =
        std::fs::read_to_string(root.join("../proposal/examples/comptime.sisu")).unwrap_or_default();
    let start = original.find("# A user-defined derive: any trait can provide one.");
    let end = original.find("# A checked string DSL");
    assert!(start.is_some() && end.is_some(), "proposal/examples/comptime.sisu has no derive section");
    let section = original[start.unwrap_or(0)..end.unwrap_or(0)].trim_end();
    let case = std::fs::read_to_string(root.join("conformance/cases/comptime/proposal_derive.sisu"))
        .unwrap_or_default();
    assert!(
        case.contains(section),
        "comptime/proposal_derive.sisu no longer holds the proposal's derive section"
    );
}

/// std.x.http's tests pass under a thousand schedules (18, Phase 6: "`sisu
/// test` of it under `--explore 1000`"); the suite itself runs fewer.
#[test]
fn http_tests_pass_under_a_thousand_schedules() {
    let sisu = PathBuf::from(env!("CARGO_BIN_EXE_sisu"));
    let case = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance/cases/http/server.sisu");
    let out = std::process::Command::new(&sisu)
        .args(["test", "--explore", "1000"])
        .arg(&case)
        .output()
        .expect("sisu runs");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "sisu test --explore 1000 failed:\n{stdout}");
    assert!(stdout.contains("0 failed"), "{stdout}");
    assert!(stdout.contains("explored 1000 schedules per test"), "{stdout}");
}

/// The sections of proposal/examples/comptime.sisu that stage 0 runs (the
/// JSON encoder over reflection, the user-defined derive, and conditional
/// compilation with its test) are in the case
/// `reflect/proposal_comptime.sisu`, each kept identical.
#[test]
fn comptime_sections_match_the_proposal() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let original =
        std::fs::read_to_string(root.join("../proposal/examples/comptime.sisu")).unwrap_or_default();
    let case = std::fs::read_to_string(root.join("conformance/cases/reflect/proposal_comptime.sisu"))
        .unwrap_or_default();
    let sections = [
        ("# Reflection: a generic JSON encoder with no macro.", Some("# A user-defined derive")),
        ("# A user-defined derive: any trait can provide one.", Some("# A checked string DSL")),
        ("# Conditional compilation: both branches must type-check.", None),
    ];
    for (start, end) in sections {
        let from = original.find(start);
        assert!(from.is_some(), "proposal/examples/comptime.sisu has no section `{start}`");
        let from = from.unwrap_or(0);
        let to = end.and_then(|e| original.find(e)).unwrap_or(original.len());
        let section = original[from..to].trim_end();
        assert!(
            case.contains(section),
            "reflect/proposal_comptime.sisu no longer holds the section `{start}`"
        );
    }
}

/// The CRC table and the ring buffer of proposal/examples/comptime.sisu
/// (`Array`, `u32`, const generics) are in the case
/// `comptime/proposal_arrays.sisu`, each kept identical.
#[test]
fn comptime_arrays_match_the_proposal() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let original =
        std::fs::read_to_string(root.join("../proposal/examples/comptime.sisu")).unwrap_or_default();
    let case = std::fs::read_to_string(root.join("conformance/cases/comptime/proposal_arrays.sisu"))
        .unwrap_or_default();
    let sections = [
        (
            "# An ordinary function, evaluated at compile time",
            "# A comptime-only check inside a generic type.",
        ),
        (
            "# A comptime-only check inside a generic type.",
            "# Reflection: a generic JSON encoder with no macro.",
        ),
    ];
    for (start, end) in sections {
        let from = original.find(start);
        let to = original.find(end);
        assert!(from.is_some() && to.is_some(), "proposal/examples/comptime.sisu has no section `{start}`");
        let section = original[from.unwrap_or(0)..to.unwrap_or(0)].trim_end();
        assert!(
            case.contains(section),
            "comptime/proposal_arrays.sisu no longer holds the section `{start}`"
        );
    }
}

/// proposal/examples/testing.sisu runs as the case
/// `testing/proposal_testing.sisu`: the example unchanged, then the
/// functions and types it tests (`parse_int`, `load_config`, `Cache`,
/// `Options`), which it leaves to the program.
#[test]
fn testing_example_starts_with_the_proposal() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let original =
        std::fs::read_to_string(root.join("../proposal/examples/testing.sisu")).unwrap_or_default();
    assert!(!original.is_empty(), "missing proposal/examples/testing.sisu");
    let case = std::fs::read_to_string(root.join("conformance/cases/testing/proposal_testing.sisu"))
        .unwrap_or_default();
    assert!(
        case.starts_with(&original),
        "testing/proposal_testing.sisu no longer starts with proposal/examples/testing.sisu"
    );
}

/// `sisu test` of a program with an error runs none of its tests: it prints
/// the diagnostics and the line `sisu run` ends with, `D0021` after the
/// ownership checker's errors and `D0006` after a `comptime` expression's,
/// and exits with 1; a program with warnings has them printed before its
/// report, as `sisu run` prints them; a test blocked by a hole exits with 3
/// (decisions 340 and 380; the manual's exercises `winner` and `journeys`).
#[test]
fn sisu_test_reports_a_program_as_sisu_run_does() {
    let sisu = PathBuf::from(env!("CARGO_BIN_EXE_sisu"));
    let cases = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance/cases/testing");
    let dir = std::env::temp_dir().join(format!("sisu-test-reports-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let test = |file: &std::path::Path| {
        let out = std::process::Command::new(&sisu)
            .args(["test", "--no-cache"])
            .arg(file)
            .current_dir(&dir)
            .output()
            .expect("sisu runs");
        (
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
            out.status.code(),
        )
    };
    let (out, err, status) = test(&cases.join("refused_program.sisu"));
    assert_eq!((out.as_str(), status), ("", Some(1)), "{err}");
    assert!(err.starts_with("error[E0408]"), "{err}");
    assert!(err.ends_with("error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor\n"), "{err}");
    let (out, err, status) = test(&cases.join("refused_comptime.sisu"));
    assert_eq!((out.as_str(), status), ("", Some(1)), "{err}");
    assert!(err.starts_with("error[E0603]"), "{err}");
    assert!(err.ends_with("error[D0006]: the program was not run: errors found (1)\n"), "{err}");
    let warned = dir.join("warned.sisu");
    std::fs::write(
        &warned,
        "def later() -> int:\n    return todo(\"later\")\n\ntest \"waits\":\n    assert later() == 1\n",
    )
    .expect("the program");
    let (out, err, status) = test(&warned);
    assert_eq!(status, Some(3), "{out}{err}");
    assert!(err.starts_with("warning[W0001]: typed hole `later` of type `int`"), "{err}");
    assert!(out.ends_with("0 passed, 0 failed, 1 blocked\n"), "{out}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// `expect`'s failures, which stage 1 lowers in a program built with its
/// tests to `expect.fail`, `expect.fail_compare` and `expect.fail_parts`
/// (its R38; `tests/expect/expect.sir`, what its `sir --tests` prints of its
/// case `testing/expect`): the interpreter records each in the running
/// test's report as `assert`'s panic would be, at its statement, and the
/// test goes on to its end, where it fails with all three; a plain run of
/// the same SIR, whose `expect` holds, prints and exits with 0 (decision
/// 388).
#[test]
fn expect_records_its_failures_and_the_test_goes_on() {
    let sisu = PathBuf::from(env!("CARGO_BIN_EXE_sisu"));
    let sir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/expect/expect.sir");
    let dir = std::env::temp_dir().join(format!("sisu-expect-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    std::fs::copy(&sir, dir.join("expect.sir")).expect("the SIR");
    let run = |args: &[&str]| {
        let out = std::process::Command::new(&sisu).args(args).current_dir(&dir).output().expect("sisu runs");
        (String::from_utf8_lossy(&out.stdout).into_owned(), out.status.code())
    };
    let (out, status) = run(&["test", "--from-sir", "expect.sir"]);
    assert_eq!(status, Some(1), "{out}");
    let failures: Vec<&str> = out.lines().filter(|l| l.starts_with("panic: assertion failed: ")).collect();
    assert_eq!(
        failures,
        [
            "panic: assertion failed: double(2) == 5",
            "panic: assertion failed: four > 5",
            "panic: assertion failed: even(double(3) + 1)",
        ],
        "{out}"
    );
    for at in ["(expect.sisu:14:5)", "(expect.sisu:15:5)", "(expect.sisu:16:5)"] {
        assert!(out.contains(at), "{out}");
    }
    assert!(out.ends_with("0 passed, 1 failed, 0 blocked\n"), "{out}");
    assert_eq!(run(&["run", "--from-sir", "expect.sir"]), ("six\n".to_string(), Some(0)));
    let _ = std::fs::remove_dir_all(&dir);
}
