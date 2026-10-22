//! The native backend (stage 2) against the conformance suite: every case
//! of `conformance/native.list` built with `sisu build --native` and run,
//! with the expectations the interpreter meets.

// A test's helpers may fail loudly, as its body may (the panic policy is
// for the crates, not their tests).
#![allow(clippy::expect_used)]

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

/// A scratch directory for one test, removed by the caller.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sisu-native-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

/// What a run printed and how it ended.
fn outcome(out: std::process::Output) -> (String, String, Option<i32>) {
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

/// A seeded schedule means the same natively (decision 203): under every
/// seed, the executable prints what the interpreter prints under it, for
/// programs whose output is their schedule (`tests/schedules`: tasks
/// printing as they go, races, `select`, generators shared by tasks, tasks
/// waiting deep in a recursion) and for the concurrency cases of the
/// coverage list.
#[test]
fn seeded_schedules_are_the_interpreters() {
    let sisu = PathBuf::from(env!("CARGO_BIN_EXE_sisu"));
    let dir = scratch("seeds");
    let mut programs: Vec<(PathBuf, u64)> = Vec::new();
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/schedules");
    let mut names: Vec<PathBuf> = std::fs::read_dir(&fixtures)
        .expect("the schedule fixtures")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "sisu"))
        .collect();
    names.sort();
    programs.extend(names.into_iter().map(|p| (p, 24)));
    let list =
        std::fs::read_to_string(compiler_dir().join("conformance/native.list")).expect("the coverage list");
    for case in sisu_conformance::native::read_list(&list) {
        if let Some(name) = case.strip_prefix("concurrency/") {
            programs.push((compiler_dir().join(format!("conformance/cases/concurrency/{name}.sisu")), 6));
        }
    }
    assert!(programs.len() > 10, "too few programs");
    let mut failures = Vec::new();
    for (path, seeds) in &programs {
        let cwd = path.parent().expect("a directory");
        let file = path.file_name().expect("a file name");
        let stem = path.file_stem().expect("a stem").to_string_lossy().into_owned();
        let exe = dir.join(&stem);
        let built = std::process::Command::new(&sisu)
            .args(["build", "--native", "--output"])
            .arg(&exe)
            .arg(file)
            .current_dir(cwd)
            .output()
            .expect("sisu builds");
        assert!(built.status.success(), "{stem}: {}", String::from_utf8_lossy(&built.stderr));
        let sir = std::process::Command::new(&sisu)
            .arg("sir")
            .arg(file)
            .current_dir(cwd)
            .output()
            .expect("sisu sir");
        let sir_file = dir.join(format!("{stem}.sir"));
        std::fs::write(&sir_file, &sir.stdout).expect("the SIR");
        for seed in 0..*seeds {
            let interp = std::process::Command::new(&sisu)
                .args(["run", "--sir", "--schedule-seed", &seed.to_string()])
                .arg(&sir_file)
                .current_dir(cwd)
                .env_clear()
                .output()
                .expect("sisu runs");
            let native = std::process::Command::new(&exe)
                .current_dir(cwd)
                .env_clear()
                .env("SISU_SCHEDULE_SEED", seed.to_string())
                .output()
                .expect("the executable runs");
            let (a, b) = (outcome(interp), outcome(native));
            if a != b {
                failures.push(format!(
                    "{stem} under seed {seed}:\n--- interpreter ({:?})\n{}{}\n--- native ({:?})\n{}{}",
                    a.2, a.0, a.1, b.2, b.0, b.1
                ));
            }
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A task that runs out of its stack fails cleanly (decision 203): the
/// probe at a function's entry raises the panic, the task's frames unwind
/// with their drops, and the joiner gets `TaskPanicked`. `SISU_TASK_STACK`
/// gives tasks a larger stack. The root task keeps the interpreter's limit
/// of nested calls.
#[test]
fn a_task_that_runs_out_of_stack_fails_cleanly() {
    let sisu = PathBuf::from(env!("CARGO_BIN_EXE_sisu"));
    let dir = scratch("stack");
    std::fs::write(
        dir.join("deep.sisu"),
        "struct Noisy:\n    n: int\n\nimpl Drop for Noisy:\n    def drop(mut self):\n        if self.n == 1:\n            print(\"dropped the last frame's\")\n\ndef down(n: int) -> int:\n    let g = Noisy(n=n)\n    if n == 0:\n        return 0\n    return down(n - 1) + 1\n\ndef main():\n    let r = task.scope do s:\n        let h = s.spawn(() -> down(40000))\n        return h.join()\n    match r:\n        case Ok(v):\n            print(\"depth {v}\")\n        case Err(TaskPanicked(message, _)):\n            print(message)\n        case Err(_):\n            print(\"cancelled\")\n    print(down(3000))\n",
    )
    .expect("the program");
    let built = std::process::Command::new(&sisu)
        .args(["build", "--native", "deep.sisu"])
        .current_dir(&dir)
        .output()
        .expect("sisu builds");
    assert!(built.status.success(), "{}", String::from_utf8_lossy(&built.stderr));
    let run = |stack: Option<&str>| {
        let mut cmd = std::process::Command::new(dir.join("deep"));
        cmd.env_clear().current_dir(&dir);
        if let Some(s) = stack {
            cmd.env("SISU_TASK_STACK", s);
        }
        outcome(cmd.output().expect("the executable runs"))
    };
    let (out, err, status) = run(None);
    assert_eq!(status, Some(0), "{err}");
    assert!(out.starts_with("stack overflow: the stack of "), "{out}");
    assert!(
        out.ends_with(
            " bytes is used up (`SISU_TASK_STACK` sets a task's)\ndropped the last frame's\n3000\n"
        ),
        "{out}"
    );
    let (out, err, status) = run(Some("64M"));
    assert_eq!(status, Some(0), "{err}");
    assert_eq!(out, "dropped the last frame's\ndepth 40000\ndropped the last frame's\n3000\n");
    let _ = std::fs::remove_dir_all(&dir);
}

/// `sisu run --native` takes `--schedule-seed`, `--explore` and `--stats`
/// (decision 203): the scheduler's numbers are the interpreter's under the
/// same seed.
#[test]
fn native_runs_take_seeds_and_report_the_schedulers_numbers() {
    let sisu = PathBuf::from(env!("CARGO_BIN_EXE_sisu"));
    let cases = compiler_dir().join("conformance/cases/concurrency");
    let run = |args: &[&str]| {
        outcome(std::process::Command::new(&sisu).args(args).current_dir(&cases).output().expect("sisu runs"))
    };
    let tasks_line = |err: &str| {
        err.lines().find(|l| l.starts_with("stats: ") && l.contains(" tasks")).map(str::to_string)
    };
    for seed in ["0", "3", "11"] {
        let (out_i, err_i, st_i) = run(&["run", "--stats", "--schedule-seed", seed, "mutex.sisu"]);
        let (out_n, err_n, st_n) =
            run(&["run", "--native", "--stats", "--schedule-seed", seed, "mutex.sisu"]);
        assert_eq!((out_i, st_i), (out_n, st_n));
        assert!(tasks_line(&err_i).is_some(), "{err_i}");
        assert_eq!(tasks_line(&err_i), tasks_line(&err_n), "{err_n}");
        assert!(err_n.contains("microseconds (native code counts no statements)"), "{err_n}");
    }
    let (out, err, status) = run(&["run", "--native", "--explore", "4", "join.sisu"]);
    assert_eq!(status, Some(0), "{err}");
    assert!(out.starts_with("Ok([45, 190, 780])\n"), "{out}");
    assert!(err.contains("explored 4 schedules (seeds 0 to 3)"), "{err}");
    let (_, err, status) = run(&["run", "--native", "--stats", "stack-depth", "join.sisu"]);
    assert_eq!(status, Some(2));
    assert!(err.contains("`--stats stack-depth`"), "{err}");
}
