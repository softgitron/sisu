//! The native backend (stage 2) against the conformance suite: every case
//! of `conformance/native.list` built with `sisu build --native` and run,
//! with the expectations the interpreter meets, by the debug profile's
//! backend, the one written in Sisu, and by Cranelift, its oracle
//! (decision 331).

// A test's helpers may fail loudly, as its body may (the panic policy is
// for the crates, not their tests).
#![allow(clippy::expect_used)]

use std::path::PathBuf;

fn compiler_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn native_conformance() {
    let sisu = sisu_conformance::Sisu::new(env!("CARGO_BIN_EXE_sisu"));
    let root = compiler_dir().join("conformance/cases");
    let text =
        std::fs::read_to_string(compiler_dir().join("conformance/native.list")).expect("the coverage list");
    let list = sisu_conformance::native::read_list(&text);
    let report = sisu_conformance::native::run_native(&sisu, &root, &list);
    println!("{}", report.render());
    assert!(report.cases > 0, "no cases in the coverage list");
    assert!(report.failures.is_empty(), "{}", report.render());
    // Stage 2 lowered every case: none was built through the Rust lowering
    // (decision 305); and the Sisu backend, the debug profile's, compiled
    // every one (decision 331).
    assert_eq!(report.stage2.map(|(n, _)| n), Some(report.cases), "{}", report.render());
    assert_eq!(report.sisu.map(|(n, _)| n), Some(report.cases), "{}", report.render());
}

/// The same cases built from serialised SIR (decision 212): each from its
/// own `.sir` expectation, or from what `sisu sir` prints for it, with
/// `sisu build --native --from-sir`, the front end not run.
#[test]
fn native_conformance_from_sir() {
    let sisu = sisu_conformance::Sisu::new(env!("CARGO_BIN_EXE_sisu"));
    let root = compiler_dir().join("conformance/cases");
    let text =
        std::fs::read_to_string(compiler_dir().join("conformance/native.list")).expect("the coverage list");
    let list = sisu_conformance::native::read_list(&text);
    let report = sisu_conformance::native::run_native_from_sir(&sisu, &root, &list);
    println!("{}", report.render());
    assert!(report.cases > 0, "no cases in the coverage list");
    assert!(report.failures.is_empty(), "{}", report.render());
    // Stage 2 lowered every case: none was built through the Rust lowering
    // (decision 305); the Sisu backend compiled every one (decision 331).
    assert_eq!(report.stage2.map(|(n, _)| n), Some(report.cases), "{}", report.render());
    assert_eq!(report.sisu.map(|(n, _)| n), Some(report.cases), "{}", report.render());
}

/// The same cases built with the release profile, every contract checked
/// as the interpreter's runs check them (decision 232), by its backend,
/// Cranelift (decision 331): Cranelift's optimisations change nothing a
/// program does.
#[test]
fn native_conformance_release() {
    let sisu = sisu_conformance::Sisu::new(env!("CARGO_BIN_EXE_sisu"));
    let root = compiler_dir().join("conformance/cases");
    let text =
        std::fs::read_to_string(compiler_dir().join("conformance/native.list")).expect("the coverage list");
    let list = sisu_conformance::native::read_list(&text);
    let report = sisu_conformance::native::run_native_release(&sisu, &root, &list);
    println!("{}", report.render());
    assert!(report.cases > 0, "no cases in the coverage list");
    assert!(report.failures.is_empty(), "{}", report.render());
    // Stage 2 lowered every case: none was built through the Rust lowering
    // (decision 305).
    assert_eq!(report.stage2.map(|(n, _)| n), Some(report.cases), "{}", report.render());
}

/// The same cases lowered by stage 2, the backend written in Sisu
/// (`compiler/stage2`, built here with the `sisu` under test), from their
/// SIR to LIR, and built from the LIR with `sisu build --native --from-lir`
/// (decision 284): each run meets the interpreter's expectations, and each
/// executable is byte for byte the one `--from-sir` builds from that SIR,
/// in the debug profile and in the release one.
#[test]
fn native_conformance_from_lir() {
    let sisu = sisu_conformance::Sisu::new(env!("CARGO_BIN_EXE_sisu"));
    let root = compiler_dir().join("conformance/cases");
    let stage2 = sisu_conformance::native::build_stage2(&sisu, &compiler_dir().join("stage2"))
        .unwrap_or_else(|e| panic!("stage 2 does not build:\n{e}"));
    let text =
        std::fs::read_to_string(compiler_dir().join("conformance/native.list")).expect("the coverage list");
    let list = sisu_conformance::native::read_list(&text);
    let report = sisu_conformance::native::run_native_from_lir(&sisu, &root, &list, &stage2);
    if let Some(dir) = stage2.parent() {
        let _ = std::fs::remove_dir_all(dir);
    }
    println!("{}", report.render());
    assert!(report.cases > 0, "no cases in the coverage list");
    assert!(report.failures.is_empty(), "{}", report.render());
    assert_eq!(report.identical, Some(report.cases), "{}", report.render());
}

/// The same cases built with Cranelift, the oracle of the backend written
/// in Sisu (`sisu build --native --backend=cranelift`, decision 331), its
/// objects linked with the same runtime: each run meets the interpreter's
/// expectations, as each run of the Sisu backend's does, so every case
/// behaves the same under both backends.
#[test]
fn native_conformance_cranelift() {
    let sisu = sisu_conformance::Sisu::new(env!("CARGO_BIN_EXE_sisu"));
    let root = compiler_dir().join("conformance/cases");
    let text =
        std::fs::read_to_string(compiler_dir().join("conformance/native.list")).expect("the coverage list");
    let list = sisu_conformance::native::read_list(&text);
    let report = sisu_conformance::native::run_native_cranelift(&sisu, &root, &list);
    println!("{}", report.render());
    assert!(report.cases > 0, "no cases in the coverage list");
    assert!(report.failures.is_empty(), "{}", report.render());
    assert_eq!(report.stage2.map(|(n, _)| n), Some(report.cases), "{}", report.render());
    assert_eq!(report.sisu, None, "{}", report.render());
}

/// The same cases' SIR run in the interpreter (decision 254), `sisu run
/// --from-sir` of each case's own `.sir` or of what `sisu sir` prints for
/// it: what a front end serialised meets the expectations the source does.
#[test]
fn interpreted_conformance_from_sir() {
    let sisu = sisu_conformance::Sisu::new(env!("CARGO_BIN_EXE_sisu"));
    let root = compiler_dir().join("conformance/cases");
    let text =
        std::fs::read_to_string(compiler_dir().join("conformance/native.list")).expect("the coverage list");
    let list = sisu_conformance::native::read_list(&text);
    let report = sisu_conformance::native::run_interpreted_from_sir(&sisu, &root, &list);
    println!("{}", report.render());
    assert!(report.cases > 0, "no cases in the coverage list");
    assert!(report.failures.is_empty(), "{}", report.render());
}

/// A package's native builds go to `build/PROFILE/NAME` beside its
/// lockfile (decision 231): `sisu build --native` to `build/debug`, with
/// `--release` to `build/release`, which checks only the preconditions
/// unless the manifest says otherwise (decision 232); `sisu run --native`
/// builds there too and keeps it; `sisu build` alone still only checks
/// and lowers; and `--release` goes with `--native` only (`D0111`).
#[test]
fn a_package_builds_where_its_profile_goes() {
    let sisu = PathBuf::from(env!("CARGO_BIN_EXE_sisu"));
    let dir = scratch("profiles");
    copy_dir(&compiler_dir().join("conformance/cases/packages/modules.pkg"), &dir);
    let expected = std::fs::read_to_string(compiler_dir().join("conformance/cases/packages/modules.out"))
        .expect("the expected output");
    let sisu_in = |args: &[&str]| {
        let out = std::process::Command::new(&sisu).args(args).current_dir(&dir).output().expect("sisu runs");
        (
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
            out.status.code(),
        )
    };
    // The plain build checks and lowers: nothing is written but the
    // lockfile.
    let (out, _, status) = sisu_in(&["build"]);
    assert_eq!((out.as_str(), status), ("built the package `atlas` 0.2.0 (modules: 4)\n", Some(0)));
    assert!(!dir.join("build").exists());
    for (flags, profile) in [(&["--native"][..], "debug"), (&["--native", "--release"][..], "release")] {
        let mut args = vec!["build"];
        args.extend(flags);
        let (out, err, status) = sisu_in(&args);
        assert_eq!(status, Some(0), "{err}");
        let exe = dir.join("build").join(profile).join("atlas");
        assert!(out.ends_with(&format!("build/{profile}/atlas`\n")), "{out}");
        let ran = std::process::Command::new(&exe).current_dir(&dir).output().expect("the program runs");
        assert_eq!(String::from_utf8_lossy(&ran.stdout), expected, "{profile}");
    }
    // A release build checks the preconditions only, unless the manifest
    // says `[profile.release] contracts`.
    std::fs::write(
        dir.join("src/main.sisu"),
        "struct Count:\n    n: int\n    invariant self.n >= 0\n\ndef main():\n    let c = Count(n=-1)\n    print(c.n)\n",
    )
    .expect("the program");
    let (_, _, status) = sisu_in(&["run", "--native"]);
    assert_eq!(status, Some(101), "the debug profile checks the invariant");
    assert!(dir.join("build/debug/atlas").is_file(), "`sisu run --native` keeps a package's executable");
    let (out, _, status) = sisu_in(&["run", "--native", "--release"]);
    assert_eq!((out.as_str(), status), ("-1\n", Some(0)), "the release profile checks preconditions only");
    let manifest = std::fs::read_to_string(dir.join("sisu.toml")).expect("the manifest");
    std::fs::write(dir.join("sisu.toml"), format!("{manifest}\n[profile.release]\ncontracts = \"all\"\n"))
        .expect("the manifest");
    let (_, _, status) = sisu_in(&["run", "--native", "--release"]);
    assert_eq!(status, Some(101), "the manifest asks for every contract");
    let (_, err, status) = sisu_in(&["build", "--release"]);
    assert_eq!(status, Some(2));
    assert!(err.contains("goes with `--native`"), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The serialised SIR of every case the backend compiles reads back into
/// a program that writes the same bytes (decision 212): the reader parses
/// exactly what the writer prints, whole programs with the standard
/// library's functions they reach.
#[test]
fn the_sir_of_every_native_case_reads_back_byte_for_byte() {
    let sisu = PathBuf::from(env!("CARGO_BIN_EXE_sisu"));
    let root = compiler_dir().join("conformance/cases");
    let text =
        std::fs::read_to_string(compiler_dir().join("conformance/native.list")).expect("the coverage list");
    let list = sisu_conformance::native::read_list(&text);
    let next = std::sync::atomic::AtomicUsize::new(0);
    let failures = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..4 {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some(name) = list.get(i) else { break };
                    let path = root.join(name);
                    // The case as the runner's sandbox has it: a package's
                    // directory (whose lockfile `sisu sir` writes), or the
                    // source with its files (`NAME.files/`).
                    let sandbox = scratch(&format!("sir-{}", name.replace('/', "-")));
                    let (dir, args) = if path.with_extension("pkg").is_dir() {
                        copy_dir(&path.with_extension("pkg"), &sandbox);
                        let cwd = std::fs::read_to_string(path.with_extension("cwd")).unwrap_or_default();
                        (sandbox.join(cwd.trim()), vec!["sir".to_string()])
                    } else {
                        let file =
                            path.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
                        std::fs::copy(path.with_extension("sisu"), sandbox.join(format!("{file}.sisu")))
                            .expect("the case's source");
                        copy_dir(&path.with_extension("files"), &sandbox);
                        (sandbox.clone(), vec!["sir".to_string(), format!("{file}.sisu")])
                    };
                    let out = std::process::Command::new(&sisu)
                        .args(&args)
                        .current_dir(&dir)
                        .output()
                        .expect("sisu runs");
                    let _ = std::fs::remove_dir_all(&sandbox);
                    let printed = String::from_utf8_lossy(&out.stdout).into_owned();
                    let problem = match sisu_sir::read(&printed) {
                        Ok(p) if sisu_sir::write(&p) == printed => None,
                        Ok(_) => Some("writes other bytes than it read".to_string()),
                        Err(e) => Some(format!("line {}: {}", e.line, e.message)),
                    };
                    if let Some(p) = problem {
                        failures.lock().expect("the failures").push(format!("{name}: {p}"));
                    }
                }
            });
        }
    });
    let failures = failures.into_inner().expect("the failures");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A program built from the SIR `sisu sir` prints for it is byte for byte
/// the executable its source gives (decision 212): the self-hosted check
/// compares stage 1's SIR with stage 0's, and the same SIR makes the same
/// code.
#[test]
fn building_from_sir_gives_the_same_executable() {
    let sisu = PathBuf::from(env!("CARGO_BIN_EXE_sisu"));
    let dir = scratch("from-sir");
    let cases = compiler_dir().join("conformance/cases");
    for case in ["run/closures", "run/drop_order", "concurrency/channels", "capabilities/fakes"] {
        let name = case.rsplit('/').next().unwrap_or(case);
        let source = cases.join(format!("{case}.sisu"));
        std::fs::copy(&source, dir.join(format!("{name}.sisu"))).expect("the case");
        let run = |args: &[&str]| {
            let out =
                std::process::Command::new(&sisu).args(args).current_dir(&dir).output().expect("sisu runs");
            assert!(out.status.success(), "{args:?}: {}", String::from_utf8_lossy(&out.stderr));
            out.stdout
        };
        let sir = run(&["sir", &format!("{name}.sisu")]);
        std::fs::write(dir.join(format!("{name}.sir")), sir).expect("the SIR");
        run(&["build", "--native", "--output", "from-source", &format!("{name}.sisu")]);
        run(&["build", "--native", "--from-sir", &format!("{name}.sir"), "--output", "from-sir"]);
        let a = std::fs::read(dir.join("from-source")).expect("the first executable");
        let b = std::fs::read(dir.join("from-sir")).expect("the second executable");
        assert!(a == b, "{case}: the executables differ");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// What a program allocates is freed by its end (decisions 213 to 218):
/// `tests/leaks/leaks.sisu` makes, in a loop, each thing the first round
/// leaked (decision 200) and each part the interpreter's host frees unseen.
/// Natively, under `SISU_RT_ALLOCS`, the runtime reports none of its
/// allocations live at exit, and the program prints what the interpreter
/// prints.
#[test]
fn nothing_the_program_allocates_is_live_at_exit() {
    let sisu = PathBuf::from(env!("CARGO_BIN_EXE_sisu"));
    let dir = scratch("leaks");
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/leaks/leaks.sisu");
    std::fs::copy(&fixture, dir.join("leaks.sisu")).expect("the fixture");
    let interpreted = std::process::Command::new(&sisu)
        .args(["run", "leaks.sisu"])
        .current_dir(&dir)
        .output()
        .expect("sisu runs");
    let (expected, _, status) = outcome(interpreted);
    assert_eq!(status, Some(0));
    let built = std::process::Command::new(&sisu)
        .args(["build", "--native", "--output", "leaks", "leaks.sisu"])
        .current_dir(&dir)
        .output()
        .expect("sisu builds");
    assert!(built.status.success(), "{}", String::from_utf8_lossy(&built.stderr));
    let ran = std::process::Command::new(dir.join("leaks"))
        .env("SISU_RT_ALLOCS", "1")
        .current_dir(&dir)
        .output()
        .expect("the program runs");
    let (out, err, status) = outcome(ran);
    assert_eq!(status, Some(0));
    assert_eq!(out, expected);
    assert_eq!(err, "allocations: 0 of the runtime's live at exit\n");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A struct, an enum or a tuple dropped whole frees what its fields hold
/// (lists, dicts, sets, texts, boxes), with drop glue or without (decision
/// 218), so a program that makes and drops them in a loop runs in bounded
/// memory: `tests/leaks/aggregates.sisu`, run natively under
/// `SISU_RT_ALLOCS=peak` for 20 and for 500 rounds, holds at most as many
/// allocations at once either way and none at exit (decision 229), and
/// prints what the interpreter prints.
#[test]
fn aggregates_dropped_in_a_loop_run_in_bounded_memory() {
    let sisu = PathBuf::from(env!("CARGO_BIN_EXE_sisu"));
    let dir = scratch("aggregates");
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/leaks/aggregates.sisu");
    std::fs::copy(&fixture, dir.join("aggregates.sisu")).expect("the fixture");
    let interpreted = std::process::Command::new(&sisu)
        .args(["run", "aggregates.sisu", "20"])
        .current_dir(&dir)
        .output()
        .expect("sisu runs");
    let (expected, _, status) = outcome(interpreted);
    assert_eq!(status, Some(0));
    let built = std::process::Command::new(&sisu)
        .args(["build", "--native", "--output", "aggregates", "aggregates.sisu"])
        .current_dir(&dir)
        .output()
        .expect("sisu builds");
    assert!(built.status.success(), "{}", String::from_utf8_lossy(&built.stderr));
    let peak = |rounds: &str| {
        let ran = std::process::Command::new(dir.join("aggregates"))
            .arg(rounds)
            .env("SISU_RT_ALLOCS", "peak")
            .current_dir(&dir)
            .output()
            .expect("the program runs");
        let (out, err, status) = outcome(ran);
        assert_eq!(status, Some(0), "{err}");
        let lines: Vec<&str> = err.lines().collect();
        assert_eq!(lines.len(), 2, "the runtime's two lines of report: {err}");
        assert_eq!(lines[1], "allocations: 0 of the runtime's live at exit", "{rounds} rounds");
        let most: u64 = lines[0]
            .strip_prefix("allocations: at most ")
            .and_then(|r| r.strip_suffix(" of the runtime's live at once"))
            .and_then(|n| n.parse().ok())
            .expect("the most live at once");
        (out, most)
    };
    let (out, few) = peak("20");
    assert_eq!(out, expected);
    let (_, many) = peak("500");
    assert_eq!(few, many, "the allocations live at once grow with the rounds");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The program the debuggers are tried on: a struct, an error, a function
/// with parameters and locals, and `main` calling it.
const SHAPES: &str = "struct Point:\n    x: int\n    y: int\n\nerror Problem:\n    TooBig(limit: int)\n\ndef check(n: int) -> int ! Problem:\n    if n > 2:\n        return Err(TooBig(limit=2))\n    return Ok(n)\n\ndef area(p: Point, scale: int) -> int:\n    let w = p.x * scale\n    let h = p.y * scale\n    return w * h\n\ndef main():\n    let name = \"corner\".to_string()\n    let p = Point(x=3, y=4)\n    let xs = [1, 2, 3]\n    let r = check(5)\n    let a = area(p, 2)\n    print(\"{name} {a} {xs} {r:?}\")\n";

/// The executable Cranelift builds carries DWARF for the Sisu source
/// (decision 227): a debugger stops at a line of a Sisu file, names the
/// Sisu functions and lines of the frames it unwinds, and shows the locals
/// and parameters with their Sisu types, which the Sisu backend does not
/// describe yet (decision 312). Under `gdb --batch` where there is one,
/// else `lldb`, else the line table as `readelf` decodes it.
#[test]
fn a_debugger_names_sisu_lines_and_locals() {
    let sisu = PathBuf::from(env!("CARGO_BIN_EXE_sisu"));
    let dir = scratch("debug-info");
    std::fs::write(dir.join("shapes.sisu"), SHAPES).expect("the program");
    let built = std::process::Command::new(&sisu)
        .args(["build", "--native", "--backend=cranelift", "shapes.sisu"])
        .current_dir(&dir)
        .output()
        .expect("sisu builds");
    assert!(built.status.success(), "{}", String::from_utf8_lossy(&built.stderr));
    let have = |tool: &str| {
        std::process::Command::new(tool).arg("--version").output().is_ok_and(|o| o.status.success())
    };
    let run = |tool: &str, args: &[&str]| {
        let out =
            std::process::Command::new(tool).args(args).current_dir(&dir).output().expect("the tool runs");
        format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr))
    };
    if have("gdb") {
        let commands = [
            "break area",
            "run",
            "info args",
            "print *p",
            "break shapes.sisu:15",
            "continue",
            "bt 2",
            "info locals",
            "frame 1",
            "info locals",
        ];
        let mut args = vec!["--batch"];
        for c in commands {
            args.extend(["-ex", c]);
        }
        args.push("./shapes");
        let out = run("gdb", &args);
        // A breakpoint on a function stops after its prologue, with its
        // parameters in place (a lent one is a reference).
        assert!(out.contains("area (p=") && out.contains("scale=2) at shapes.sisu:13"), "{out}");
        assert!(out.contains("scale = 2\n$1 = Point {x: 3, y: 4}"), "{out}");
        assert!(out.contains("scale=2) at shapes.sisu:15"), "{out}");
        assert!(out.contains("main () at shapes.sisu:23"), "{out}");
        assert!(out.contains("w = 6"), "{out}");
        assert!(out.contains("text: \"corner\""), "{out}");
        assert!(out.contains("[1, 2, 3]"), "{out}");
        assert!(out.contains("r = Err(TooBig{limit: 2})"), "{out}");
    } else if have("lldb") {
        let out = run(
            "lldb",
            &[
                "--batch",
                "-o",
                "breakpoint set --file shapes.sisu --line 15",
                "-o",
                "run",
                "-o",
                "bt 2",
                "-o",
                "frame variable",
                "./shapes",
            ],
        );
        assert!(out.contains("shapes.sisu:15"), "{out}");
        assert!(out.contains("shapes.sisu:23"), "{out}");
    } else {
        let out = run("readelf", &["--debug-dump=decodedline", "./shapes"]);
        assert!(
            out.lines().any(|l| l.starts_with("shapes.sisu") && l.split_whitespace().nth(1) == Some("15")),
            "{out}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// The backend is chosen by name (decision 306), or by the profile
/// (decision 331): a debug build is the Sisu backend's, the build saying
/// so, and so are `--backend=sisu` and `SISU_BACKEND=sisu`; a release
/// build, a build with the Rust lowering and `--backend=cranelift` (which
/// wins over the variable) are Cranelift's; another name is `D0123`, and
/// the Sisu backend with the Rust lowering, whose output is not LIR, is
/// `D0124`.
#[test]
fn the_backend_is_chosen_by_name() {
    let sisu = PathBuf::from(env!("CARGO_BIN_EXE_sisu"));
    let dir = scratch("backend-names");
    std::fs::write(
        dir.join("count.sisu"),
        "def main():\n    var n = 0\n    for i in range(5):\n        n += i\n    print(n)\n",
    )
    .expect("the program");
    let build = |args: &[&str], env: Option<&str>| {
        let mut cmd = std::process::Command::new(&sisu);
        cmd.arg("build").arg("--native").args(args).arg("count.sisu").current_dir(&dir);
        cmd.env("SISU_NATIVE_TIMES", "1").env_remove("SISU_BACKEND");
        if let Some(b) = env {
            cmd.env("SISU_BACKEND", b);
        }
        outcome(cmd.output().expect("sisu runs"))
    };
    for (args, env) in [(&["--backend=sisu"][..], None), (&[][..], Some("sisu")), (&[][..], None)] {
        let (_, err, status) = build(args, env);
        assert_eq!(status, Some(0), "{err}");
        assert!(err.contains("native: compiled by the Sisu backend in "), "{err}");
        let ran = std::process::Command::new(dir.join("count")).output().expect("it runs");
        assert_eq!(String::from_utf8_lossy(&ran.stdout), "10\n");
    }
    for (args, env) in [
        (&["--backend=cranelift"][..], Some("sisu")),
        (&[][..], Some("cranelift")),
        (&["--release"][..], None),
        (&["--lower=rust"][..], None),
    ] {
        let (_, err, status) = build(args, env);
        assert_eq!(status, Some(0), "{err}");
        assert!(!err.contains("Sisu backend") && err.contains(" functions, "), "{args:?}: {err}");
        let ran = std::process::Command::new(dir.join("count")).output().expect("it runs");
        assert_eq!(String::from_utf8_lossy(&ran.stdout), "10\n");
    }
    let (_, err, status) = build(&["--release", "--backend=sisu"], None);
    assert_eq!(status, Some(0), "{err}");
    assert!(err.contains("native: compiled by the Sisu backend in "), "{err}");
    let (_, err, status) = build(&["--backend=llvm"], None);
    assert_eq!(status, Some(2), "{err}");
    assert!(err.contains("D0123") && err.contains("not `llvm`"), "{err}");
    let (_, err, status) = build(&["--backend=sisu", "--lower=rust"], None);
    assert_eq!(status, Some(1), "{err}");
    assert!(err.contains("does not go with `--lower=rust`"), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The Sisu backend's objects carry a line table and a unit naming each
/// function's code (decision 312): a debugger stops at a Sisu function
/// (after its prologue, on its declaration's line) and at a Sisu line, and
/// names the Sisu functions and lines of the frames it unwinds through the
/// frame pointers. Under `gdb --batch` where there is one, else `lldb`,
/// else the line table as `readelf` decodes it.
#[test]
fn the_sisu_backends_line_table_names_sisu_lines() {
    let sisu = PathBuf::from(env!("CARGO_BIN_EXE_sisu"));
    let dir = scratch("sisu-lines");
    std::fs::write(dir.join("shapes.sisu"), SHAPES).expect("the program");
    let built = std::process::Command::new(&sisu)
        .args(["build", "--native", "--backend=sisu", "shapes.sisu"])
        .current_dir(&dir)
        .output()
        .expect("sisu builds");
    assert!(built.status.success(), "{}", String::from_utf8_lossy(&built.stderr));
    let ran = std::process::Command::new(dir.join("shapes")).current_dir(&dir).output().expect("it runs");
    assert_eq!(String::from_utf8_lossy(&ran.stdout), "corner 48 [1, 2, 3] Err(TooBig(limit=2))\n");
    let have = |tool: &str| {
        std::process::Command::new(tool).arg("--version").output().is_ok_and(|o| o.status.success())
    };
    let run = |tool: &str, args: &[&str]| {
        let out =
            std::process::Command::new(tool).args(args).current_dir(&dir).output().expect("the tool runs");
        format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr))
    };
    if have("gdb") {
        let commands = ["break area", "run", "break shapes.sisu:15", "continue", "bt 2"];
        let mut args = vec!["--batch"];
        for c in commands {
            args.extend(["-ex", c]);
        }
        args.push("./shapes");
        let out = run("gdb", &args);
        assert!(out.contains("Breakpoint 1 at ") && out.contains("file shapes.sisu, line 13."), "{out}");
        assert!(out.contains("area () at shapes.sisu:13"), "{out}");
        assert!(out.contains("area () at shapes.sisu:15"), "{out}");
        assert!(out.contains("main () at shapes.sisu:23"), "{out}");
    } else if have("lldb") {
        let out = run(
            "lldb",
            &[
                "--batch",
                "-o",
                "breakpoint set --file shapes.sisu --line 15",
                "-o",
                "run",
                "-o",
                "bt 2",
                "./shapes",
            ],
        );
        assert!(out.contains("shapes.sisu:15"), "{out}");
        assert!(out.contains("shapes.sisu:23"), "{out}");
    } else {
        let out = run("readelf", &["--debug-dump=decodedline", "./shapes"]);
        assert!(
            out.lines().any(|l| l.starts_with("shapes.sisu") && l.split_whitespace().nth(1) == Some("15")),
            "{out}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
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

/// The machine intrinsics of std.rt (22, "Machine intrinsics"; decision
/// 302), which no program can call (std.rt is internal to std), from SIR
/// written for them (`tests/machine/machine.sir`): natively the stack
/// pointer, a word written and read through the thread's slot, and a
/// switch that saves the running stack's registers and loads them again,
/// so that it returns; the interpreter, which runs its own scheduler,
/// refuses the first of them.
#[test]
fn machine_intrinsics_run_natively_and_the_interpreter_refuses_them() {
    let sisu = PathBuf::from(env!("CARGO_BIN_EXE_sisu"));
    let dir = scratch("machine");
    let sir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/machine/machine.sir");
    std::fs::copy(&sir, dir.join("machine.sir")).expect("the SIR");
    let built = std::process::Command::new(&sisu)
        .args(["build", "--native", "--from-sir", "machine.sir", "--output", "machine"])
        .current_dir(&dir)
        .output()
        .expect("sisu builds");
    assert!(built.status.success(), "{}", String::from_utf8_lossy(&built.stderr));
    let ran = std::process::Command::new(dir.join("machine")).current_dir(&dir).output().expect("it runs");
    assert_eq!(outcome(ran), ("True\n42\nswitched back\n".to_string(), String::new(), Some(0)));
    let interpreted = std::process::Command::new(&sisu)
        .args(["run", "--from-sir", "machine.sir"])
        .current_dir(&dir)
        .output()
        .expect("sisu runs");
    let (out, err, status) = outcome(interpreted);
    assert_eq!((out.as_str(), status), ("", Some(101)));
    assert!(err.starts_with("panic: `rt.stack_pointer` is a machine intrinsic of native code"), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Foreign calls whose arguments and results are narrower than a word (a C
/// `int` sign-extended back, a `uint16_t`) or floats (decision 302),
/// lowered by stage 2, the only lowering a build uses unless the Rust one
/// is asked for (decision 305): the build says stage 2 lowered it, its
/// executable through Cranelift is byte for byte the one `--lower=rust`
/// builds, and it prints what C computes, as the Sisu backend's does
/// (decision 331). The interpreter emulates none of these functions, so no
/// conformance case holds them.
#[test]
fn foreign_calls_of_every_width_lower_through_stage_2() {
    let sisu = PathBuf::from(env!("CARGO_BIN_EXE_sisu"));
    let dir = scratch("widths");
    let program = "from std.ffi import c_int\n\n# safety: the signatures as <stdlib.h>, <ctype.h>, <arpa/inet.h> and\n#   <math.h> declare them for 64-bit Linux; each returns promptly.\nextern \"C\":\n    def abs(x: c_int) -> c_int\n    def toupper(c: c_int) -> c_int\n    def htons(x: u16) -> u16\n    def ntohs(x: u16) -> u16\n    def sqrt(x: f64) -> f64\n    def ldexp(x: f64, e: c_int) -> f64\n\ndef main():\n    # safety: none of these functions has a precondition the arguments\n    #   could break.\n    unsafe:\n        print(abs(-7))\n        print(abs(-2147483647))\n        print(toupper(97))\n        print(htons(258u16))\n        print(ntohs(htons(4660u16)))\n        print(sqrt(2.0))\n        print(ldexp(1.5, -3))\n";
    std::fs::write(dir.join("widths.sisu"), program).expect("the program");
    let build = |lower: &[&str], exe: &str| {
        let out = std::process::Command::new(&sisu)
            .arg("build")
            .arg("--native")
            .args(lower)
            .args(["--output", exe, "widths.sisu"])
            .env("SISU_NATIVE_TIMES", "1")
            .current_dir(&dir)
            .output()
            .expect("sisu builds");
        let (_, err, status) = outcome(out);
        assert_eq!(status, Some(0), "{err}");
        (std::fs::read(dir.join(exe)).expect("the executable"), err)
    };
    let (by_stage2, err) = build(&["--backend=cranelift"], "by-stage2");
    assert!(err.contains("native: lowered by stage 2 in "), "{err}");
    let (by_rust, err) = build(&["--lower=rust"], "by-rust");
    assert!(!err.contains("lowered by stage 2"), "{err}");
    assert!(by_stage2 == by_rust, "the executables differ");
    let (_, err) = build(&[], "by-sisu");
    assert!(err.contains("native: compiled by the Sisu backend in "), "{err}");
    for exe in ["by-stage2", "by-sisu"] {
        let ran = std::process::Command::new(dir.join(exe)).current_dir(&dir).output().expect("it runs");
        assert_eq!(
            outcome(ran),
            ("7\n2147483647\n65\n513\n4660\n1.4142135623730951\n0.1875\n".to_string(), String::new(), Some(0)),
            "{exe}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// Copies the files under `from` (if it is a directory) into `to`.
fn copy_dir(from: &std::path::Path, to: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(from) else { return };
    for e in entries.flatten() {
        let target = to.join(e.file_name());
        if e.path().is_dir() {
            std::fs::create_dir_all(&target).expect("a directory");
            copy_dir(&e.path(), &target);
        } else {
            std::fs::copy(e.path(), &target).expect("a file");
        }
    }
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

/// A task that runs out of its stack fails cleanly (decisions 203 and
/// 234): the probe at a function's entry raises the stack overflow, which
/// names the function, the task's frames unwind with their drops, and the
/// joiner gets `TaskPanicked`. `SISU_TASK_STACK` gives tasks a larger
/// stack. The root task keeps the interpreter's limit of nested calls.
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
    assert_eq!(
        out,
        "stack overflow in `down`: the calls nest deeper than the stack holds\ndropped the last frame's\n3000\n"
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

/// The release profile leaves out the checks its facts prove and keeps the
/// rest (decision 270): of `run/proven_checks`, every index but the one
/// into a list that shrinks in its loop, two additions that cannot
/// overflow, and the postcondition its preconditions decide; and the
/// executable meets the interpreter's expectations, the one kept check
/// failing where it should.
#[test]
fn the_release_profile_leaves_out_the_checks_it_proves() {
    let sisu = PathBuf::from(env!("CARGO_BIN_EXE_sisu"));
    let dir = scratch("proven");
    let case = compiler_dir().join("conformance/cases/run/proven_checks");
    std::fs::copy(case.with_extension("sisu"), dir.join("proven_checks.sisu")).expect("the case");
    let built = std::process::Command::new(&sisu)
        .args([
            "build",
            "--native",
            "--release",
            "--contracts=all",
            "--output",
            "proven",
            "proven_checks.sisu",
        ])
        .env("SISU_NATIVE_TIMES", "1")
        .current_dir(&dir)
        .output()
        .expect("sisu runs");
    let (_, err, status) = outcome(built);
    assert_eq!(status, Some(0), "{err}");
    assert!(err.contains("index checks 7 of 8 left out, overflow checks 2 of 9, tests decided 1"), "{err}");
    let ran = std::process::Command::new(dir.join("proven")).current_dir(&dir).output().expect("it runs");
    let (out, err, status) = outcome(ran);
    let expected = |ext: &str| std::fs::read_to_string(case.with_extension(ext)).expect("an expectation");
    assert_eq!((out, err, status), (expected("out"), expected("err"), Some(101)));
    let _ = std::fs::remove_dir_all(&dir);
}
