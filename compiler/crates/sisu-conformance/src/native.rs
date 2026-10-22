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

use crate::{Case, Failure, Output, Sandbox, diff, discover, invoke_with_input};
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
    pub failures: Vec<Failure>,
    /// Time spent building (the front end, the backend and the linker)
    /// and running the executables, over every case.
    pub build: Duration,
    pub run: Duration,
    /// Lines of the cases' sources.
    pub lines: usize,
}

impl NativeReport {
    pub fn render(&self) -> String {
        let mut out = String::new();
        for f in &self.failures {
            out.push_str(&format!("FAIL {} [{}]\n{}\n", f.case, f.check, f.detail));
        }
        let what = match self.from_sir {
            Some(own) => format!("native from SIR: {} cases ({own} from their own `.sir`)", self.cases),
            None => format!("native: {} cases ({} lines)", self.cases, self.lines),
        };
        out.push_str(&format!(
            "{what}, built in {:.2} s, run in {:.2} s, {} failed\n",
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
        .map(|l| l.split('#').next().unwrap_or("").trim())
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

/// Builds and runs every case of `list` natively, on as many threads as
/// the machine has (at most eight).
pub fn run_native(sisu: &Path, root: &Path, list: &[String]) -> NativeReport {
    run_list(sisu, root, list, false)
}

/// [`run_native`], each case built from serialised SIR (decision 212).
pub fn run_native_from_sir(sisu: &Path, root: &Path, list: &[String]) -> NativeReport {
    run_list(sisu, root, list, true)
}

fn run_list(sisu: &Path, root: &Path, list: &[String], from_sir: bool) -> NativeReport {
    let all = discover(root);
    let mut report = NativeReport::default();
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
    let results = parallel(&cases, |case, n| native_case(sisu, case, n, from_sir));
    if from_sir {
        report.from_sir = Some(results.iter().filter(|r| r.own_sir).count());
    }
    for r in results {
        report.cases += 1;
        report.build += r.build;
        report.run += r.run;
        report.lines += r.lines;
        report.failures.extend(r.failures);
    }
    report
}

/// For each runnable case outside `list`, whether it builds and passes
/// natively: the cases a coverage list could grow by.
pub fn survey(sisu: &Path, root: &Path, list: &[String]) -> Vec<(String, String)> {
    let cases: Vec<Case> = discover(root)
        .into_iter()
        .filter(|c| !list.contains(&c.name) && ["out", "err", "status"].iter().any(|e| c.path(e).exists()))
        .collect();
    let results = parallel(&cases, |case, n| native_case(sisu, case, n, false));
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
}

fn native_case(sisu: &Path, case: &Case, n: usize, from_sir: bool) -> CaseResult {
    let mut r = CaseResult {
        failures: Vec::new(),
        build: Duration::ZERO,
        run: Duration::ZERO,
        lines: 0,
        own_sir: false,
    };
    let fail = |r: &mut CaseResult, check: &str, detail: String| {
        r.failures.push(Failure { case: case.name.clone(), check: check.to_string(), detail });
    };
    // The two modes may run at once (`cargo test`): sandboxes of their own.
    let sandbox = match Sandbox::new(case, if from_sir { 2_000_000 } else { 1_000_000 } + n) {
        Ok(s) => s,
        Err(e) => {
            fail(&mut r, "setup", e.to_string());
            return r;
        }
    };
    let exe = sandbox.dir.join("native-program");
    let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    let mut build_args = s(&["build", "--native", "--output", &exe.display().to_string()]);
    if !case.package {
        build_args.push(format!("{}.sisu", case.stem));
        r.lines = std::fs::read_to_string(case.source()).map(|t| t.lines().count()).unwrap_or(0);
    }
    let cwd = match case.expected("cwd") {
        Some(d) => sandbox.dir.join(d.trim()),
        None => sandbox.dir.clone(),
    };
    let start = Instant::now();
    // From SIR: the case's own `.sir`, or `sisu sir`'s output, which also
    // says whether the front end warns (its warnings precede the program's
    // standard error in the interpreter's run).
    let mut warned_by_sir = false;
    if from_sir {
        let mut sir_args = s(&["sir"]);
        if !case.package {
            sir_args.push(format!("{}.sisu", case.stem));
        }
        let printed = invoke_with_input(sisu, &cwd, &sir_args, None);
        warned_by_sir = !printed.stderr.is_empty();
        let text = match case.expected("sir") {
            Some(own) => {
                r.own_sir = true;
                own
            }
            None if printed.status == 0 => printed.stdout,
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
    }
    let built = invoke_with_input(sisu, &cwd, &build_args, None);
    r.build = start.elapsed();
    if built.status != 0 || !exe.is_file() {
        fail(&mut r, "sisu build --native", format!("{}{}", built.stdout, built.stderr));
        return r;
    }
    let args: Vec<String> =
        case.expected("args").map(|a| a.lines().map(str::to_string).collect()).unwrap_or_default();
    let input = case.expected("stdin");
    let start = Instant::now();
    let ran = run_exe(&exe, &cwd, &args, input.as_deref());
    r.run = start.elapsed();
    let expected = case.expected("out").unwrap_or_default();
    if expected != ran.stdout {
        fail(&mut r, "stdout", diff::diff(&expected, &ran.stdout));
    }
    let status = case.expected("status").map(|s| s.trim().to_string()).unwrap_or_else(|| "0".into());
    if status != ran.status.to_string() {
        fail(&mut r, "exit status", format!("expected {status}, found {}\n{}", ran.status, ran.stderr));
    }
    // The build printed the compiler's warnings; the program's own
    // standard error is what follows them in a `sisu run`.
    let err = case.expected("err").unwrap_or_default();
    let warned = !built.stderr.is_empty() || warned_by_sir;
    let same = if warned { err.ends_with(&ran.stderr) } else { err == ran.stderr };
    if !same {
        fail(&mut r, "stderr", diff::diff(&err, &ran.stderr));
    }
    r
}

/// Runs an executable as the runner runs `sisu`: in `cwd`, with an empty
/// environment, killed past the time limit.
fn run_exe(exe: &Path, cwd: &Path, args: &[String], input: Option<&str>) -> Output {
    let spawned = Command::new(exe)
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    crate::wait_output(spawned, input, exe)
}
