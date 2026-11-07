//! The native mode (stage 2): every case of a coverage list built with
//! `sisu build --native` and its executable run, compared with the same
//! expectations the interpreter meets (`NAME.out`, `NAME.status`, and
//! `NAME.err`, less the compiler's warnings, which the build prints, not
//! the program). The list (`conformance/native.list`) names one case per
//! line, as the runner names them (`run/arithmetic`); `#` starts a
//! comment.
//!
//! The mode from SIR (decision 212) builds each case from serialised SIR
//! instead, with `sisu build --native --from-sir`: the case's own `NAME.sir`
//! where it has one, else what `sisu sir` prints for it. It checks the
//! path a self-hosted front end takes, whose SIR the backend compiles.
//!
//! The mode from LIR (decision 284) lowers each case's SIR with stage 2,
//! the backend written in Sisu (`compiler/stage2`, `sisu2 lir`), builds the
//! LIR with `sisu build --native --from-lir`, runs the executable, and
//! requires it to be byte for byte the one `--from-sir` builds from the
//! same SIR, and so the release profile's (`sisu2 lir --release`, against
//! `--release --from-sir`): LIR holds every decision of the Rust backend,
//! and its translation into Cranelift IR is one to one.
//!
//! The interpreter's mode from SIR (decision 254) runs the same SIR with
//! `sisu run --from-sir` instead of compiling it: the interpreter, given
//! what a front end (stage 0's or stage 1's) serialised, must meet the
//! expectations the source meets.
//!
//! A debug build's backend is the x86-64 backend written in Sisu, compiling
//! stage 2's LIR (decisions 306 and 331), so the native mode and the mode
//! from SIR build with it; the release profile's is Cranelift. The mode of
//! the oracle (`--native-cranelift`) builds each case with `sisu build
//! --native --backend=cranelift` and compares its runs with the same
//! expectations, so that every case's behaviour is held under both
//! backends; the mode from LIR names Cranelift too, whose executables it
//! compares with the Rust lowering's.
//!
//! The modes that build with the default lowering (natively, from SIR,
//! with the release profile, and with Cranelift) ask the build to report
//! its steps (`SISU_NATIVE_TIMES=1`) and require it to say that stage 2
//! lowered the program (`native: lowered by stage 2 in T ms`), so that no
//! case is built through the Rust lowering, which lowers only when asked
//! for (`--lower=rust`, decision 305), and the debug ones that the Sisu
//! backend compiled it (`native: compiled by the Sisu backend in T ms`);
//! the report counts those cases and the time each took over them.
//!
//! Either way, the executable runs with `SISU_RT_ALLOCS=1`, which makes
//! the runtime report how many of its allocations are live when the
//! program ends (decisions 217 and 226): the report must say none, and is left out
//! of the standard error compared. Only a program that aborts (status
//! 134) ends without one.

use crate::{Case, Failure, Output, Sandbox, Sisu, diff, discover, invoke_with_input};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// What a native run of the list found.
#[derive(Clone, Debug, Default)]
pub struct NativeReport {
    pub cases: usize,
    /// In the mode from SIR: the cases built from their own `.sir`
    /// expectation (the others from `sisu sir`'s output).
    pub from_sir: Option<usize>,
    /// Built with the release profile.
    pub release: bool,
    /// Interpreted from serialised SIR rather than built.
    pub interpreted: bool,
    /// In the mode from LIR: the cases whose executable is byte for byte
    /// the one built from their SIR.
    pub identical: Option<usize>,
    /// In the modes that build with the default lowering: the cases whose
    /// build said stage 2 lowered them, and the time it took.
    pub stage2: Option<(usize, Duration)>,
    /// With the Sisu backend: the cases its build said it compiled, and the
    /// time it took.
    pub sisu: Option<(usize, Duration)>,
    /// With the release profile: the cases a release build refuses as their
    /// `.release` says (a typed hole, decision 340), which nothing lowers.
    pub refused: usize,
    /// Built with Cranelift, the oracle (`--backend=cranelift`).
    pub cranelift: bool,
    pub failures: Vec<Failure>,
    /// Time spent building (the front end, the backend and the linker)
    /// and running the executables, over every case.
    pub build: Duration,
    pub run: Duration,
    /// Lines of the cases' sources.
    pub lines: usize,
    /// The cases whose program the mode built (or interpreted): not one
    /// a release build refuses, one whose errors stop it (decision 463),
    /// or one that only pins `sisu test` runs (decision 460).
    pub programs: usize,
    /// In the native mode: the cases recorded natively and replayed in both
    /// runtimes, and recorded in the interpreter and replayed natively
    /// (decision 1321).
    pub replayed: usize,
}

impl NativeReport {
    pub fn render(&self) -> String {
        let mut out = String::new();
        for f in &self.failures {
            out.push_str(&format!("FAIL {} [{}]\n{}\n", f.case, f.check, f.detail));
        }
        let what = match self.from_sir {
            Some(own) if self.identical.is_some() => format!(
                "native from LIR: {} cases ({own} from their own `.sir`), {} identical to their SIR's executables in both profiles",
                self.cases,
                self.identical.unwrap_or(0)
            ),
            Some(own) if self.interpreted => {
                format!("interpreter from SIR: {} cases ({own} from their own `.sir`)", self.cases)
            }
            Some(own) => format!("native from SIR: {} cases ({own} from their own `.sir`)", self.cases),
            None if self.release => format!("native, release: {} cases ({} lines)", self.cases, self.lines),
            None if self.cranelift => {
                format!("native, Cranelift: {} cases ({} lines)", self.cases, self.lines)
            }
            None => format!("native: {} cases ({} lines)", self.cases, self.lines),
        };
        let mut lowered = match self.stage2 {
            Some((n, t)) => format!(", {n} lowered by stage 2 in {:.2} s", t.as_secs_f64()),
            None => String::new(),
        };
        if self.refused > 0 {
            lowered.push_str(&format!(", {} refused by the release build as expected", self.refused));
        }
        if let Some((n, t)) = self.sisu {
            lowered.push_str(&format!(", {n} compiled by the Sisu backend in {:.2} s", t.as_secs_f64()));
        }
        if self.replayed > 0 {
            lowered.push_str(&format!(", {} recorded and replayed natively", self.replayed));
        }
        out.push_str(&format!(
            "{what}{lowered}, built in {:.2} s, run in {:.2} s, {} failed\n",
            self.build.as_secs_f64(),
            self.run.as_secs_f64(),
            self.failures.len()
        ));
        out
    }
}

/// The case names a coverage list holds.
pub fn read_list(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|l| l.split('#').next().unwrap_or("").split_whitespace().next())
        .map(str::to_string)
        .collect()
}

/// The cases of `native.list` marked `checked` after their name: their run
/// stops with the checked memory model's report, which a build of the
/// debug profile gives as the interpreter does (decision 544) and a release
/// build, whose raw memory is unchecked, does not; `--native-release`
/// builds them and does not run them.
pub fn read_checked(text: &str) -> Vec<String> {
    read_marked(text, "checked")
}

/// The cases of `native.list` marked `unrecorded` after their name: their
/// output is not all the host contract's, a C function writing to standard
/// output itself, so the native mode records and replays them in neither
/// runtime (decision 1321).
pub fn read_unrecorded(text: &str) -> Vec<String> {
    read_marked(text, "unrecorded")
}

fn read_marked(text: &str, mark: &str) -> Vec<String> {
    text.lines()
        .filter_map(|l| {
            let mut words = l.split('#').next().unwrap_or("").split_whitespace();
            let name = words.next()?;
            words.any(|w| w == mark).then(|| name.to_string())
        })
        .collect()
}

/// What a build reported of its steps (`SISU_NATIVE_TIMES=1`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BuildReport {
    /// What the build printed but those reports (`native: ...`, and the Rust
    /// front end's `time: ...`).
    pub rest: String,
    /// How long stage 2 took to lower the program, if the build said it did
    /// (`native: lowered by stage 2 in T ms`).
    pub lowered: Option<Duration>,
    /// How long the Sisu backend took to compile it, if it did (`native:
    /// compiled by the Sisu backend in T ms, ...`).
    pub sisu: Option<Duration>,
}

/// The reports a build printed with `SISU_NATIVE_TIMES=1`, taken off its
/// standard error.
pub fn take_build_report(stderr: &str) -> BuildReport {
    let ms = |t: &str| Duration::from_secs_f64(t.parse::<f64>().unwrap_or(0.0).max(0.0) / 1e3);
    let mut r = BuildReport::default();
    for line in stderr.split_inclusive('\n') {
        let text = line.trim_end_matches('\n');
        if let Some(t) =
            text.strip_prefix("native: lowered by stage 2 in ").and_then(|t| t.strip_suffix(" ms"))
        {
            r.lowered = Some(ms(t));
        } else if let Some(t) = text
            .strip_prefix("native: compiled by the Sisu backend in ")
            .and_then(|t| t.split_once(" ms"))
            .map(|(t, _)| t)
        {
            r.sisu = Some(ms(t));
        } else if !text.starts_with("native: ") && !text.starts_with("time: ") {
            r.rest.push_str(line);
        }
    }
    r
}

/// How [`run_list`] runs each case.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Native,
    /// Built from serialised SIR (decision 212).
    FromSir,
    /// Built with the release profile (decision 232).
    Release,
    /// Interpreted from serialised SIR (decision 254).
    Interpreted,
    /// Lowered from serialised SIR by stage 2 and built from its LIR
    /// (decision 284).
    FromLir,
    /// Built with Cranelift, the oracle of the backend written in Sisu
    /// (decision 331).
    Cranelift,
}

impl Mode {
    /// Whether the mode builds with the default lowering, whose build must
    /// say that stage 2 lowered the program.
    fn lowers(self) -> bool {
        matches!(self, Mode::Native | Mode::FromSir | Mode::Release | Mode::Cranelift)
    }

    /// Whether the mode builds with the Sisu backend, the debug profile's
    /// default (decision 331), whose build must say it compiled the program.
    fn sisu_backend(self) -> bool {
        matches!(self, Mode::Native | Mode::FromSir)
    }
}

/// Builds and runs every case of `list` natively, on as many threads as
/// the machine has (at most eight), with the debug profile's backend, the
/// one written in Sisu (decision 331).
/// Each case is recorded and replayed natively too, but those of
/// `unrecorded` ([`read_unrecorded`]).
pub fn run_native(sisu: &Sisu, root: &Path, list: &[String], unrecorded: &[String]) -> NativeReport {
    run_list_checked(sisu, root, list, Mode::Native, None, unrecorded)
}

/// [`run_native`], each case built from serialised SIR (decision 212).
pub fn run_native_from_sir(sisu: &Sisu, root: &Path, list: &[String]) -> NativeReport {
    run_list(sisu, root, list, Mode::FromSir, None)
}

/// [`run_native`], each case built with the release profile and every
/// contract checked (`sisu build --native --release --contracts=all`,
/// decision 232): Cranelift's optimisations must not change what a program
/// does.
pub fn run_native_release(sisu: &Sisu, root: &Path, list: &[String], checked: &[String]) -> NativeReport {
    run_list_checked(sisu, root, list, Mode::Release, None, checked)
}

/// [`run_native`], each case built with Cranelift, the oracle of the
/// backend written in Sisu (`sisu build --native --backend=cranelift`,
/// decision 331), and run against the same expectations.
pub fn run_native_cranelift(sisu: &Sisu, root: &Path, list: &[String]) -> NativeReport {
    run_list(sisu, root, list, Mode::Cranelift, None)
}

/// Every case of `list` interpreted from serialised SIR (`sisu run
/// --from-sir`, decision 254): the case's own `.sir`, else `sisu sir`'s.
pub fn run_interpreted_from_sir(sisu: &Sisu, root: &Path, list: &[String]) -> NativeReport {
    run_list(sisu, root, list, Mode::Interpreted, None)
}

/// [`run_native`], each case's SIR (its own `.sir`, else `sisu sir`'s)
/// lowered to LIR by stage 2's executable `stage2` (`sisu2 lir`) and built
/// with `sisu build --native --from-lir` (decision 284); the executable
/// must also be byte for byte the one `--from-sir` builds from that SIR.
pub fn run_native_from_lir(sisu: &Sisu, root: &Path, list: &[String], stage2: &Path) -> NativeReport {
    run_list(sisu, root, list, Mode::FromLir, Some(stage2))
}

/// Builds stage 2, the backend written in Sisu (the package in `dir`,
/// `compiler/stage2`), with `sisu build --native`, into a directory of the
/// system's temporary one: its executable, or what the build printed.
/// The caller removes that directory once it is done with it.
pub fn build_stage2(sisu: &Sisu, dir: &Path) -> Result<std::path::PathBuf, String> {
    let out = std::env::temp_dir().join(format!("sisu-stage2-{}", std::process::id()));
    std::fs::create_dir_all(&out).map_err(|e| format!("cannot make `{}`: {e}", out.display()))?;
    let exe = out.join("sisu2");
    // Not a case's command: no time limit, since a loaded machine may take
    // minutes over it.
    let built = sisu
        .command()
        .args(["build", "--native", "--output"])
        .arg(&exe)
        .current_dir(dir)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("cannot run {}: {e}", sisu.path.display()))?;
    if !built.status.success() || !exe.is_file() {
        return Err(format!(
            "{}{}",
            String::from_utf8_lossy(&built.stdout),
            String::from_utf8_lossy(&built.stderr)
        ));
    }
    Ok(exe)
}

fn run_list(sisu: &Sisu, root: &Path, list: &[String], mode: Mode, stage2: Option<&Path>) -> NativeReport {
    run_list_checked(sisu, root, list, mode, stage2, &[])
}

/// [`run_list`], the cases of `checked` (`read_checked`) built and not run
/// in the release profile, and in the native mode the cases of
/// `unrecorded` not recorded.
fn run_list_checked(
    sisu: &Sisu,
    root: &Path,
    list: &[String],
    mode: Mode,
    stage2: Option<&Path>,
    checked: &[String],
) -> NativeReport {
    let all = discover(root);
    let mut report = NativeReport {
        release: mode == Mode::Release,
        interpreted: mode == Mode::Interpreted,
        cranelift: mode == Mode::Cranelift,
        ..NativeReport::default()
    };
    let mut cases = Vec::new();
    for name in list {
        match all.iter().find(|c| c.name == *name) {
            Some(c) => cases.push(c.clone()),
            None => report.failures.push(Failure {
                case: name.clone(),
                check: "the coverage list".into(),
                detail: "no such case".into(),
            }),
        }
    }
    let results = parallel(&cases, |case, n| {
        let unchecked = mode == Mode::Release && checked.contains(&case.name);
        let record = mode == Mode::Native && !checked.contains(&case.name);
        native_case(sisu, case, n, mode, stage2, unchecked, record)
    });
    if matches!(mode, Mode::FromSir | Mode::Interpreted | Mode::FromLir) {
        report.from_sir = Some(results.iter().filter(|r| r.own_sir).count());
    }
    if mode == Mode::FromLir {
        report.identical = Some(results.iter().filter(|r| r.identical).count());
    }
    if mode.lowers() {
        let lowered: Vec<Duration> = results.iter().filter_map(|r| r.stage2).collect();
        report.stage2 = Some((lowered.len(), lowered.iter().sum()));
    }
    if mode.sisu_backend() {
        let compiled: Vec<Duration> = results.iter().filter_map(|r| r.sisu).collect();
        report.sisu = Some((compiled.len(), compiled.iter().sum()));
    }
    for r in results {
        report.cases += 1;
        report.refused += usize::from(r.refused);
        report.programs += usize::from(r.program && !r.refused);
        report.replayed += usize::from(r.replayed);
        report.build += r.build;
        report.run += r.run;
        report.lines += r.lines;
        report.failures.extend(r.failures);
    }
    report
}

/// For each runnable case outside `list`, whether it builds and passes
/// natively: the cases a coverage list could grow by.
pub fn survey(sisu: &Sisu, root: &Path, list: &[String]) -> Vec<(String, String)> {
    let cases: Vec<Case> =
        discover(root).into_iter().filter(|c| !list.contains(&c.name) && runnable(c)).collect();
    let results = parallel(&cases, |case, n| native_case(sisu, case, n, Mode::Native, None, false, false));
    cases
        .iter()
        .zip(results)
        .map(|(c, r)| {
            let verdict = match r.failures.first() {
                None => "pass".to_string(),
                Some(f) => format!("{}: {}", f.check, f.detail.lines().next().unwrap_or("")),
            };
            (c.name.clone(), verdict)
        })
        .collect()
}

/// Whether a case runs a program or its tests: it has an `.out`, `.err` or
/// `.status`, or a `sisu test` run it pins (decision 460).
fn runnable(c: &Case) -> bool {
    if ["out", "err", "status", "test", "explore"].iter().any(|e| c.path(e).exists()) {
        return true;
    }
    let source =
        if c.package { String::new() } else { std::fs::read_to_string(c.source()).unwrap_or_default() };
    ["# sim", "# schedule", "# select"]
        .iter()
        .any(|m| source.lines().any(|l| l.trim() == *m || l.trim().starts_with(&format!("{m} "))))
}

fn parallel<R: Send>(cases: &[Case], f: impl Fn(&Case, usize) -> R + Sync) -> Vec<R> {
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1).clamp(1, 8);
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results: std::sync::Mutex<Vec<(usize, R)>> = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some(case) = cases.get(i) else { break };
                    let r = f(case, i);
                    if let Ok(mut all) = results.lock() {
                        all.push((i, r));
                    }
                }
            });
        }
    });
    let mut all = results.into_inner().unwrap_or_default();
    all.sort_by_key(|(i, _)| *i);
    all.into_iter().map(|(_, r)| r).collect()
}

struct CaseResult {
    failures: Vec<Failure>,
    build: Duration,
    run: Duration,
    lines: usize,
    /// Built from the case's own `.sir` expectation.
    own_sir: bool,
    /// Built from LIR to the executable the case's SIR gives.
    identical: bool,
    /// How long stage 2 took to lower the case, and the Sisu backend to
    /// compile it, as its build said.
    stage2: Option<Duration>,
    sisu: Option<Duration>,
    /// A release build refused it, as its `.release` expects.
    refused: bool,
    /// Its program was built (or interpreted), and its run compared.
    program: bool,
    /// Its run was recorded and replayed natively (decision 1321).
    replayed: bool,
}

/// `unchecked`: the release build of a case whose run the checked memory
/// model stops is built, and its run left out (decision 544).
fn native_case(
    sisu: &Sisu,
    case: &Case,
    n: usize,
    mode: Mode,
    stage2: Option<&Path>,
    unchecked: bool,
    record: bool,
) -> CaseResult {
    let from_sir = matches!(mode, Mode::FromSir | Mode::Interpreted | Mode::FromLir);
    let release = mode == Mode::Release;
    let mut r = CaseResult {
        failures: Vec::new(),
        build: Duration::ZERO,
        run: Duration::ZERO,
        lines: 0,
        own_sir: false,
        identical: false,
        stage2: None,
        sisu: None,
        refused: false,
        program: false,
        replayed: false,
    };
    let fail = |r: &mut CaseResult, check: &str, detail: String| {
        r.failures.push(Failure { case: case.name.clone(), check: check.to_string(), detail });
    };
    // The modes may run at once (`cargo test`): sandboxes of their own.
    let offset = match mode {
        Mode::Native => 1_000_000,
        Mode::FromSir => 2_000_000,
        Mode::Release => 3_000_000,
        Mode::Interpreted => 4_000_000,
        Mode::FromLir => 5_000_000,
        Mode::Cranelift => 6_000_000,
    };
    let sandbox = match Sandbox::new(case, offset + n) {
        Ok(s) => s,
        Err(e) => {
            fail(&mut r, "setup", e.to_string());
            return r;
        }
    };
    // The case's tests, natively (decision 460), then its program.
    let tested = native_tests(sisu, case, &sandbox.dir, mode, &mut r);
    let wants_run = ["out", "err", "status"].iter().any(|e| case.path(e).exists());
    if !wants_run {
        if !tested {
            fail(&mut r, "setup", "the case has neither a run nor tests to run natively".into());
        }
        return r;
    }
    r.program = true;
    let exe = sandbox.dir.join("native-program");
    let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    let mut build_args = s(&["build", "--native", "--output", &exe.display().to_string()]);
    // The release profile with every contract checked, as the
    // interpreter's runs check them, so that the expectations hold: what
    // is compared is what Cranelift's optimisations do.
    if release {
        build_args.push("--release".to_string());
        build_args.push("--contracts=all".to_string());
    }
    if mode == Mode::Cranelift {
        build_args.push("--backend=cranelift".to_string());
    }
    if !case.package {
        build_args.push(format!("{}.sisu", case.stem));
        r.lines = std::fs::read_to_string(case.source()).map(|t| t.lines().count()).unwrap_or(0);
    }
    let cwd = match case.expected("cwd") {
        Some(d) => sandbox.dir.join(d.trim()),
        None => sandbox.dir.clone(),
    };
    let start = Instant::now();
    // From SIR: the case's own `.sir`, or `sisu sir`'s output. The front
    // end's warnings precede the program's standard error in the
    // interpreter's run, and the case's `.diag` says whether there are any
    // (stage 1's `sir` prints none; stage 0's prints them, decision 268).
    let mut warned_by_sir = case.expected("diag").is_some_and(|d| warns(&d));
    if from_sir {
        let mut sir_args = s(&["sir"]);
        if !case.package {
            sir_args.push(format!("{}.sisu", case.stem));
        }
        let printed = invoke_with_input(sisu, &cwd, &sir_args, None);
        warned_by_sir |= !printed.stderr.is_empty();
        let text = match case.expected("sir") {
            Some(own) => {
                r.own_sir = true;
                own
            }
            None if printed.status == 0 => printed.stdout,
            // A program its errors stop has no SIR, and is not run: the
            // errors are what `sisu run` prints (decision 463).
            None if stopped(case, printed.status, &printed.stderr) => {
                r.build = start.elapsed();
                r.program = false;
                return r;
            }
            None => {
                fail(&mut r, "sisu sir", format!("{}{}", printed.stdout, printed.stderr));
                return r;
            }
        };
        let file = sandbox.dir.join("native-program.sir");
        if let Err(e) = std::fs::write(&file, text) {
            fail(&mut r, "setup", e.to_string());
            return r;
        }
        build_args = s(&["build", "--native", "--output", &exe.display().to_string(), "--from-sir"]);
        build_args.push(file.display().to_string());
        if mode == Mode::Interpreted {
            r.build = start.elapsed();
            interpreted(sisu, case, &cwd, &file, warned_by_sir, &mut r);
            return r;
        }
        if mode == Mode::FromLir {
            // Stage 2 lowers the SIR; a construct it does not lower yet is
            // its message, this case's failure.
            let lir = sandbox.dir.join("native-program.lir");
            let Some(stage2) = stage2 else {
                fail(&mut r, "sisu2 lir", "no stage 2 to lower with".into());
                return r;
            };
            // Both profiles: the debug one's executable is run below, the
            // release one's (decision 270's facts) compared only.
            let mut identical = true;
            for release in [true, false] {
                let lowered = run_stage2(stage2, &cwd, &file, &lir, release);
                let what = if release { "sisu2 lir --release" } else { "sisu2 lir" };
                if lowered.status != 0 || !lir.is_file() {
                    fail(&mut r, what, format!("{}{}", lowered.stdout, lowered.stderr));
                    return r;
                }
                // The executable the SIR gives through the Rust lowering, the
                // oracle, which the LIR's must equal (decision 304).
                let from_sir_exe = sandbox.dir.join("native-program-from-sir");
                let mut sir_build = s(&[
                    "build",
                    "--native",
                    "--lower=rust",
                    "--output",
                    &from_sir_exe.display().to_string(),
                ]);
                if release {
                    sir_build.push("--release".to_string());
                }
                sir_build.push("--from-sir".to_string());
                sir_build.push(file.display().to_string());
                let by_sir = invoke_with_input(sisu, &cwd, &sir_build, None);
                if by_sir.status != 0 {
                    fail(
                        &mut r,
                        "sisu build --native --from-sir",
                        format!("{}{}", by_sir.stdout, by_sir.stderr),
                    );
                    return r;
                }
                // Translated by Cranelift, whose executable is the Rust
                // lowering's (decision 284), not the debug profile's default.
                build_args = s(&[
                    "build",
                    "--native",
                    "--backend=cranelift",
                    "--output",
                    &exe.display().to_string(),
                    "--from-lir",
                ]);
                build_args.push(lir.display().to_string());
                let built = invoke_with_input(sisu, &cwd, &build_args, None);
                if built.status != 0 || !exe.is_file() {
                    r.build = start.elapsed();
                    fail(
                        &mut r,
                        "sisu build --native --from-lir",
                        format!("{}{}", built.stdout, built.stderr),
                    );
                    return r;
                }
                let profile = if release { "release" } else { "debug" };
                match (std::fs::read(&exe), std::fs::read(&from_sir_exe)) {
                    (Ok(a), Ok(b)) if a == b => {}
                    (Ok(a), Ok(b)) => {
                        identical = false;
                        fail(
                            &mut r,
                            "identical executables",
                            format!(
                                "{profile}: the executable from LIR ({} bytes) differs from the one from SIR ({} bytes)",
                                a.len(),
                                b.len()
                            ),
                        );
                    }
                    (a, b) => {
                        identical = false;
                        fail(
                            &mut r,
                            "identical executables",
                            format!("{profile}: {:?} {:?}", a.err(), b.err()),
                        );
                    }
                }
                let _ = std::fs::remove_file(&from_sir_exe);
            }
            r.identical = identical;
        }
    }
    if mode == Mode::FromLir {
        r.build = start.elapsed();
        run_and_compare(case, &exe, &cwd, warned_by_sir, false, false, &mut r);
        return r;
    }
    // The build reports its steps, and must say that stage 2 lowered the
    // program: nothing is built through the Rust lowering unasked.
    let mut built = crate::invoke_with_env(sisu, &cwd, &build_args, &[("SISU_NATIVE_TIMES", "1")]);
    r.build = start.elapsed();
    let report = take_build_report(&built.stderr);
    built.stderr = report.rest;
    r.stage2 = report.lowered;
    r.sisu = report.sisu;
    // A case with a `.release` expectation is one a release build refuses
    // (a typed hole, decision 340): the build must fail and print it.
    if release && let Some(expected) = case.expected("release") {
        if built.status == 0 {
            fail(&mut r, "sisu build --native --release", "the release build succeeded".into());
        } else if built.stderr != expected {
            fail(&mut r, "release", diff::diff(&expected, &built.stderr));
        } else {
            r.refused = true;
        }
        return r;
    }
    if built.status != 0 && stopped(case, built.status, &built.stderr) {
        r.program = false;
        return r;
    }
    if built.status != 0 || !exe.is_file() {
        fail(&mut r, "sisu build --native", format!("{}{}", built.stdout, built.stderr));
        return r;
    }
    if report.lowered.is_none() {
        fail(
            &mut r,
            "lowered by stage 2",
            "the build did not say that stage 2 lowered the program (`native: lowered by stage 2`)".into(),
        );
    }
    if mode.sisu_backend() && report.sisu.is_none() {
        fail(
            &mut r,
            "compiled by the Sisu backend",
            "the build did not say that the Sisu backend compiled the program".into(),
        );
    }
    if unchecked {
        return r;
    }
    run_and_compare(case, &exe, &cwd, warned_by_sir, !built.stderr.is_empty(), !from_sir, &mut r);
    if record {
        recorded(sisu, case, &cwd, &mut r);
    }
    r
}

/// The capability log natively (N-04; decision 1321), in the native mode:
/// the case's run again by `sisu run --native --record LOG`, which prints
/// what the case's run prints, its log replayed by `sisu replay LOG` in the
/// interpreter and by `sisu replay --native LOG`, and the interpreter's
/// log of it (`sisu run --record LOG`) replayed natively, each replay
/// printing what the recorded run printed and exiting as it did. (Each
/// run writes its files again, into the same directory, as the plain run
/// did.) A program its errors stop writes no log.
fn recorded(sisu: &Sisu, case: &Case, cwd: &Path, r: &mut CaseResult) {
    let args: Vec<String> =
        case.expected("args").map(|a| a.lines().map(str::to_string).collect()).unwrap_or_default();
    let input = case.expected("stdin");
    let shown = |o: &Output| format!("{}\n--- stderr\n{}\n--- status {}\n", o.stdout, o.stderr, o.status);
    let words = |first: &[&str]| {
        let mut a: Vec<String> = first.iter().map(|x| x.to_string()).collect();
        if !case.package {
            a.push(format!("{}.sisu", case.stem));
        } else if !args.is_empty() {
            a.push("--".into());
        }
        a.extend(args.iter().cloned());
        a
    };
    let native_log = cwd.join("native-run.log").display().to_string();
    let recorded =
        invoke_with_input(sisu, cwd, &words(&["run", "--native", "--record", &native_log]), input.as_deref());
    let mut fail = |check: &str, detail: String| {
        r.failures.push(Failure { case: case.name.clone(), check: check.to_string(), detail });
    };
    let expected = case.expected("native-out").or_else(|| case.expected("out")).unwrap_or_default();
    let status = case.expected("status").map(|s| s.trim().to_string()).unwrap_or_else(|| "0".into());
    let err = case.expected("err").unwrap_or_default();
    let printed = format!("{expected}\n--- stderr\n{err}\n--- status {status}\n");
    if printed != shown(&recorded) {
        fail(
            "sisu run --native --record prints what the run prints",
            diff::diff(&printed, &shown(&recorded)),
        );
        return;
    }
    if !Path::new(&native_log).exists() {
        return;
    }
    for replay in [&["replay"][..], &["replay", "--native"]] {
        let mut a: Vec<String> = replay.iter().map(|x| x.to_string()).collect();
        a.push(native_log.clone());
        let replayed = invoke_with_input(sisu, cwd, &a, None);
        if shown(&replayed) != shown(&recorded) {
            fail(
                &format!(
                    "{} of a native log prints what the recorded run printed",
                    a[..a.len() - 1].join(" ")
                ),
                diff::diff(&shown(&recorded), &shown(&replayed)),
            );
        }
    }
    let log = cwd.join("interpreted-run.log").display().to_string();
    let interpreted = invoke_with_input(sisu, cwd, &words(&["run", "--record", &log]), input.as_deref());
    if Path::new(&log).exists() {
        let replayed = invoke_with_input(sisu, cwd, &["replay".into(), "--native".into(), log], None);
        if shown(&replayed) != shown(&interpreted) {
            fail(
                "replay --native of the interpreter's log prints what its run printed",
                diff::diff(&shown(&interpreted), &shown(&replayed)),
            );
        }
    }
    r.replayed = true;
}

/// The `sisu test` runs a case pins (`NAME.test`, and the runs that `#
/// sim`, `# schedule`, `# select` and `# explore N` with `NAME.explore`
/// ask for), each compiled and run natively by `sisu test --native`
/// (decision 460) as the mode builds, compared with the interpreter's
/// expectation, held to the status its report gives, and each failure
/// replayed natively from its token. The modes that build from SIR build
/// the tests as `--native` does, since `sisu test` builds them from stage
/// 1's `sir --tests` anyway; `--native-from-lir` builds them through the
/// Rust lowering, the oracle, and Cranelift; `--interpreted-from-sir`
/// runs them in the interpreter. Whether the case has such a run.
fn native_tests(sisu: &Sisu, case: &Case, dir: &Path, mode: Mode, r: &mut CaseResult) -> bool {
    let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    let cwd = match case.expected("cwd") {
        Some(d) => dir.join(d.trim()),
        None => dir.to_path_buf(),
    };
    let source =
        if case.package { String::new() } else { std::fs::read_to_string(case.source()).unwrap_or_default() };
    let file = format!("{}.sisu", case.stem);
    // Each run: its arguments after `test` and the expectation.
    let mut runs: Vec<(Vec<String>, String)> = Vec::new();
    if let Some(expected) = case.expected("test") {
        runs.push((Vec::new(), expected));
    }
    for (marker, ext, sim) in
        [("# sim", "sim", true), ("# schedule", "schedule", false), ("# select", "select", false)]
    {
        let Some(line) =
            source.lines().find(|l| l.trim() == marker || l.trim().starts_with(&format!("{marker} ")))
        else {
            continue;
        };
        let mut a = Vec::new();
        if sim {
            a.push("--sim".to_string());
        }
        a.extend(line.trim()[marker.len()..].split_whitespace().map(str::to_string));
        runs.push((a, case.expected(ext).unwrap_or_default()));
    }
    // A package's `NAME.select`: `sisu test` with the lines of
    // `NAME.select-args` (decision 900).
    if case.package
        && let Some(expected) = case.expected("select")
    {
        runs.push((
            case.expected("select-args").unwrap_or_default().lines().map(str::to_string).collect(),
            expected,
        ));
    }
    let explore = source
        .lines()
        .find_map(|l| l.trim().strip_prefix("# explore ").and_then(|n| n.trim().parse::<u64>().ok()));
    if let (Some(n), Some(expected)) = (explore, case.expected("explore")) {
        runs.push((s(&["--explore", &n.to_string()]), expected));
    }
    if runs.is_empty() {
        return false;
    }
    let flags: Vec<String> = match mode {
        Mode::Interpreted => Vec::new(),
        Mode::Native | Mode::FromSir => s(&["--native"]),
        // `sisu test` checks every contract, and takes no `--contracts`.
        Mode::Release => s(&["--native", "--release"]),
        Mode::FromLir => s(&["--native", "--lower=rust", "--backend=cranelift"]),
        Mode::Cranelift => s(&["--native", "--backend=cranelift"]),
    };
    let mut fail = |check: String, detail: String| {
        r.failures.push(Failure { case: case.name.clone(), check, detail });
    };
    let start = Instant::now();
    for (given, expected) in runs {
        // A native run takes nothing from the test cache, so a test the
        // interpreted run of the same sandbox took from it (`ok (cached)`,
        // decision 1145) is a plain `ok` here.
        let expected =
            if flags.is_empty() { expected } else { expected.replace(" ... ok (cached)\n", " ... ok\n") };
        let mut a = s(&["test"]);
        a.extend(flags.iter().cloned());
        a.extend(given.iter().cloned());
        if !case.package {
            a.push(file.clone());
        }
        let shown = format!("sisu {}", a.join(" "));
        let out = crate::invoke_with_limit(sisu, &cwd, &a, None, TESTS_TIMEOUT);
        if out.stdout != expected {
            fail(shown.clone(), format!("{}{}", diff::diff(&expected, &out.stdout), out.stderr));
            continue;
        }
        if let Some(status) = report_status(&out)
            && status != out.status
        {
            fail(
                format!("{shown}: the exit status its report gives"),
                format!("expected {status}, found {}", out.status),
            );
        }
        // Each failure again from its token, natively.
        for (token, block) in crate::failure_blocks(&out.stdout) {
            let mut again = s(&["test"]);
            again.extend(flags.iter().cloned());
            again.push("--replay".into());
            again.push(token.clone());
            // A package's token names the file it replays (decision 827).
            if !case.package {
                again.push(file.clone());
            } else if let Some(f) = crate::replay_file(&out.stdout, &token) {
                again.push(f);
            }
            let replayed = crate::invoke_with_limit(sisu, &cwd, &again, None, TESTS_TIMEOUT);
            let first = crate::failure_blocks(&replayed.stdout)
                .into_iter()
                .next()
                .map(|(_, b)| b)
                .unwrap_or_default();
            if first != block {
                fail(format!("sisu test --replay {token}"), diff::diff(&block, &first));
            }
        }
    }
    r.run += start.elapsed();
    true
}

/// Whether a build that failed is the run the case expects of a program
/// its errors stop (decision 463): nothing on standard output, the errors
/// and `D0006` as `.err` has them, and the status `.status` gives, which
/// `sisu run --native` prints and exits with too. A build, and `sir`,
/// end with `D0132` where the run ends with `D0006` (decision 441): the
/// comparison reads the build's line as the run's.
fn stopped(case: &Case, status: i32, stderr: &str) -> bool {
    let expected = case.expected("status").map(|s| s.trim().to_string()).unwrap_or_else(|| "0".into());
    let as_run = stderr.replace(
        "error[D0132]: the program was not compiled: errors found (",
        "error[D0006]: the program was not run: errors found (",
    );
    expected != "0"
        && expected == status.to_string()
        && case.expected("out").unwrap_or_default().is_empty()
        && case.expected("err").is_some_and(|e| e == as_run)
}

/// How long `sisu test --native` may take, which builds the program and
/// runs each test as a process of its own: a test's property draws its
/// hundred cases there too.
const TESTS_TIMEOUT: Duration = Duration::from_secs(300);

/// The status a `sisu test` report gives: 1 when a test failed, 3 when
/// none failed and one is blocked, else 0 (decision 380); none when it
/// printed no summary or an error.
fn report_status(out: &Output) -> Option<i32> {
    let (mut summaries, mut failed, mut blocked) = (0, 0u64, 0u64);
    for line in out.stdout.lines() {
        let words: Vec<&str> = line.split([' ', ',']).filter(|w| !w.is_empty()).collect();
        if let [p, "passed", f, "failed", b, "blocked"] = words.as_slice()
            && p.parse::<u64>().is_ok()
            && let (Ok(f), Ok(b)) = (f.parse::<u64>(), b.parse::<u64>())
        {
            summaries += 1;
            failed += f;
            blocked += b;
        }
    }
    if summaries == 0 || out.stderr.lines().any(|l| l.starts_with("error")) {
        return None;
    }
    Some(if failed > 0 {
        1
    } else if blocked > 0 {
        3
    } else {
        0
    })
}

/// Runs a case's executable and compares the run with its expectations:
/// `built_warned` says whether the build printed warnings, which a `sisu
/// run` prints before the program's own standard error. A program built
/// from its source is built for the native runtime, which `target.runtime`
/// names (decision 721): its standard output is `NAME.native-out` where
/// the case has one; one built from `sisu sir`'s SIR was built for the
/// interpreter's, and is compared with `NAME.out`.
fn run_and_compare(
    case: &Case,
    exe: &Path,
    cwd: &Path,
    warned_by_sir: bool,
    built_warned: bool,
    from_source: bool,
    r: &mut CaseResult,
) {
    let mut fail = |check: &str, detail: String| {
        r.failures.push(Failure { case: case.name.clone(), check: check.to_string(), detail });
    };
    let args: Vec<String> =
        case.expected("args").map(|a| a.lines().map(str::to_string).collect()).unwrap_or_default();
    let input = case.expected("stdin");
    let start = Instant::now();
    let mut ran = run_exe(exe, cwd, &args, input.as_deref());
    let run = start.elapsed();
    // What the runtime left allocated: nothing.
    match take_allocations(&mut ran.stderr) {
        Some(0) => {}
        Some(n) => fail("allocations", format!("{n} of the runtime's allocations live at exit")),
        None if ran.status == ABORTED => {}
        None => fail("allocations", "the run did not report its live allocations".into()),
    }
    let native_out = if from_source { case.expected("native-out") } else { None };
    let expected = native_out.or_else(|| case.expected("out")).unwrap_or_default();
    if expected != ran.stdout {
        fail("stdout", diff::diff(&expected, &ran.stdout));
    }
    let status = case.expected("status").map(|s| s.trim().to_string()).unwrap_or_else(|| "0".into());
    if status != ran.status.to_string() {
        fail("exit status", format!("expected {status}, found {}\n{}", ran.status, ran.stderr));
    }
    // The build printed the compiler's warnings; the program's own
    // standard error is what follows them in a `sisu run`.
    let err = case.expected("err").unwrap_or_default();
    let warned = built_warned || warned_by_sir;
    let same = if warned { err.ends_with(&ran.stderr) } else { err == ran.stderr };
    if !same {
        fail("stderr", diff::diff(&err, &ran.stderr));
    }
    r.run = run;
}

/// Stage 2's lowering of the SIR in `sir` to LIR in `lir` (`sisu2 lir`,
/// with `--release` for that profile), run as the runner runs `sisu`: in
/// `cwd`, with an empty environment.
fn run_stage2(stage2: &Path, cwd: &Path, sir: &Path, lir: &Path, release: bool) -> Output {
    let spawned = Command::new(stage2)
        .arg("lir")
        .arg(sir)
        .arg("--output")
        .arg(lir)
        .args(if release { &["--release"][..] } else { &[][..] })
        .current_dir(cwd)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    crate::wait_output(spawned, None, stage2)
}

/// The interpreter's run of the SIR in `file` (`sisu run --from-sir`),
/// compared with the case's expectations: the front end's warnings, which
/// the source's run prints first, are not the program's.
fn interpreted(sisu: &Sisu, case: &Case, cwd: &Path, file: &Path, warned: bool, r: &mut CaseResult) {
    let mut fail = |check: &str, detail: String| {
        r.failures.push(Failure { case: case.name.clone(), check: check.to_string(), detail });
    };
    let mut args = vec!["run".to_string(), "--from-sir".to_string(), file.display().to_string()];
    args.extend(
        case.expected("args").map(|a| a.lines().map(str::to_string).collect::<Vec<_>>()).unwrap_or_default(),
    );
    let input = case.expected("stdin");
    let start = Instant::now();
    let ran = invoke_with_input(sisu, cwd, &args, input.as_deref());
    r.run = start.elapsed();
    let expected = case.expected("out").unwrap_or_default();
    if expected != ran.stdout {
        fail("stdout", diff::diff(&expected, &ran.stdout));
    }
    let status = case.expected("status").map(|s| s.trim().to_string()).unwrap_or_else(|| "0".into());
    if status != ran.status.to_string() {
        fail("exit status", format!("expected {status}, found {}\n{}", ran.status, ran.stderr));
    }
    let err = case.expected("err").unwrap_or_default();
    let same = if warned { err.ends_with(&ran.stderr) } else { err == ran.stderr };
    if !same {
        fail("stderr", diff::diff(&err, &ran.stderr));
    }
}

/// Whether a `.diag` expectation holds a warning: a line whose code, the
/// second word, is a `W` code.
fn warns(diag: &str) -> bool {
    diag.lines().any(|l| l.split_whitespace().nth(1).is_some_and(|c| c.starts_with('W')))
}

/// The exit status of a program that aborted (a deadlock, a panic during
/// unwinding), which ends without its report of live allocations.
const ABORTED: i32 = 134;

/// Takes the runtime's report of its live allocations off the end of a
/// run's standard error (`allocations: N of the runtime's live at exit`),
/// and gives N.
fn take_allocations(stderr: &mut String) -> Option<u64> {
    let body = stderr.strip_suffix('\n')?;
    let start = body.rfind('\n').map_or(0, |i| i + 1);
    let n = body[start..]
        .strip_prefix("allocations: ")?
        .strip_suffix(" of the runtime's live at exit")?
        .parse()
        .ok()?;
    stderr.truncate(start);
    Some(n)
}

/// Runs an executable as the runner runs `sisu`: in `cwd`, with an
/// environment that only asks the runtime for its live allocations at exit,
/// killed past the time limit.
fn run_exe(exe: &Path, cwd: &Path, args: &[String], input: Option<&str>) -> Output {
    let spawned = Command::new(exe)
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .env("SISU_RT_ALLOCS", "1")
        .stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    crate::wait_output(spawned, input, exe)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_build_report_says_what_lowered_the_program() {
        let printed = "warning[W0003]: unreachable code\n  --> a.sisu:3:5\nnative: lowered by stage 2 in 12.5 ms\nnative: 3 functions, 40 statements, code generated in 4.0 ms, linked in 80.0 ms\n";
        let r = take_build_report(printed);
        assert_eq!(r.rest, "warning[W0003]: unreachable code\n  --> a.sisu:3:5\n");
        assert_eq!(r.lowered, Some(Duration::from_secs_f64(0.0125)));
        assert_eq!(r.sisu, None);
        // The Rust lowering's build says nothing of stage 2, and neither
        // does a stage 2 that did not lower the program.
        let r = take_build_report("time: names resolved 1.0 ms\nnative: not lowered by stage 2 in 3.0 ms\n");
        assert_eq!((r.rest.as_str(), r.lowered), ("", None));
        // The Sisu backend's build says what compiled it.
        let r = take_build_report(
            "native: lowered by stage 2 in 2.0 ms\nnative: compiled by the Sisu backend in 4.5 ms, linked in 80.0 ms\n",
        );
        assert_eq!(r.sisu, Some(Duration::from_secs_f64(0.0045)));
    }
}
