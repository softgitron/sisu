//! The conformance runner (proposal/18-next-steps.md, "Bootstrapping": "The
//! conformance suite is language-agnostic: input files, expected SIR,
//! expected diagnostics by catalog identifier. A front end in any language
//! can be checked against it.").
//!
//! The runner knows nothing about the compiler: it runs a `sisu`
//! executable through its command line and compares what it prints with
//! the expected files next to each case. The case format is described in
//! `compiler/conformance/README.md`.

pub mod diff;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// One `.sisu` file and the expectations beside it.
#[derive(Clone, Debug)]
pub struct Case {
    /// The path relative to the suite root, without `.sisu`.
    pub name: String,
    pub dir: PathBuf,
    pub stem: String,
}

impl Case {
    fn path(&self, ext: &str) -> PathBuf {
        self.dir.join(format!("{}.{ext}", self.stem))
    }

    fn expected(&self, ext: &str) -> Option<String> {
        std::fs::read_to_string(self.path(ext)).ok()
    }

    fn source(&self) -> PathBuf {
        self.path("sisu")
    }
}

/// A check that did not match.
#[derive(Clone, Debug)]
pub struct Failure {
    pub case: String,
    pub check: String,
    pub detail: String,
}

#[derive(Clone, Debug, Default)]
pub struct Report {
    pub cases: usize,
    pub checks: usize,
    pub failures: Vec<Failure>,
    pub blessed: Vec<String>,
}

impl Report {
    pub fn render(&self) -> String {
        let mut out = String::new();
        for f in &self.failures {
            out.push_str(&format!("FAIL {} [{}]\n{}\n", f.case, f.check, f.detail));
        }
        out.push_str(&format!(
            "{} cases, {} checks, {} failed\n",
            self.cases,
            self.checks,
            self.failures.len()
        ));
        if !self.blessed.is_empty() {
            out.push_str(&format!("blessed {} files\n", self.blessed.len()));
        }
        out
    }
}

/// Every `.sisu` file under `root`, sorted by path.
pub fn discover(root: &Path) -> Vec<Case> {
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<Case>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            // `X.files` directories hold a case's input files.
            if !p.extension().is_some_and(|e| e == "files") {
                walk(root, &p, out);
            }
            continue;
        }
        if p.extension().is_some_and(|e| e == "sisu") {
            let stem = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            let rel = p.strip_prefix(root).unwrap_or(&p).with_extension("");
            let name = rel.to_string_lossy().replace('\\', "/");
            out.push(Case { name, dir: p.parent().map(Path::to_path_buf).unwrap_or_default(), stem });
        }
    }
}

/// What one run of the executable printed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Output {
    pub stdout: String,
    pub stderr: String,
    pub status: i32,
}

/// How long one command may run before the runner kills it and fails the
/// check, so a case that loops forever cannot hang the suite.
pub const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

/// Runs `sisu args...` in `cwd` with an empty environment.
pub fn invoke(sisu: &Path, cwd: &Path, args: &[String]) -> Output {
    let spawned = Command::new(sisu)
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = match spawned {
        Ok(c) => c,
        Err(e) => {
            return Output {
                stdout: String::new(),
                stderr: format!("cannot run {}: {e}", sisu.display()),
                status: -1,
            };
        }
    };
    // Drain the pipes on their own threads so a chatty program cannot block.
    let out = drain(child.stdout.take());
    let err = drain(child.stderr.take());
    let start = std::time::Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s.code().unwrap_or(-1),
            Ok(None) if start.elapsed() > TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                break -2;
            }
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(2)),
            Err(_) => break -1,
        }
    };
    let stdout = out.join().unwrap_or_default();
    let mut stderr = err.join().unwrap_or_default();
    if status == -2 {
        stderr.push_str(&format!("(killed after {} seconds)\n", TIMEOUT.as_secs()));
    }
    Output { stdout, stderr, status }
}

fn drain<R: std::io::Read + Send + 'static>(pipe: Option<R>) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut p) = pipe {
            let _ = p.read_to_end(&mut buf);
        }
        String::from_utf8_lossy(&buf).into_owned()
    })
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)? {
        let e = e?;
        let target = to.join(e.file_name());
        if e.file_type()?.is_dir() {
            copy_dir(&e.path(), &target)?;
        } else {
            std::fs::copy(e.path(), target)?;
        }
    }
    Ok(())
}

/// A fresh working directory holding the case's source and input files.
struct Sandbox {
    dir: PathBuf,
}

impl Sandbox {
    fn new(case: &Case, n: usize) -> std::io::Result<Sandbox> {
        let dir = std::env::temp_dir().join(format!("sisu-conformance-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir)?;
        std::fs::copy(case.source(), dir.join(format!("{}.sisu", case.stem)))?;
        let files = case.path("files");
        if files.is_dir() {
            copy_dir(&files, &dir)?;
        }
        Ok(Sandbox { dir })
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

struct Runner<'a> {
    sisu: &'a Path,
    bless: bool,
    report: Report,
    counter: usize,
}

impl Runner<'_> {
    fn compare(&mut self, case: &Case, check: &str, ext: Option<&str>, expected: &str, actual: &str) {
        self.report.checks += 1;
        if expected == actual {
            return;
        }
        if self.bless
            && let Some(ext) = ext
        {
            let path = case.path(ext);
            if std::fs::write(&path, actual).is_ok() {
                self.report.blessed.push(path.display().to_string());
                return;
            }
        }
        self.report.failures.push(Failure {
            case: case.name.clone(),
            check: check.to_string(),
            detail: diff::diff(expected, actual),
        });
    }

    fn run_case(&mut self, case: &Case) {
        self.report.cases += 1;
        self.counter += 1;
        let sandbox = match Sandbox::new(case, self.counter) {
            Ok(s) => s,
            Err(e) => {
                self.report.failures.push(Failure {
                    case: case.name.clone(),
                    check: "setup".into(),
                    detail: e.to_string(),
                });
                return;
            }
        };
        let file = format!("{}.sisu", case.stem);
        let source = std::fs::read_to_string(case.source()).unwrap_or_default();
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();

        // The tree is lossless.
        let printed = invoke(self.sisu, &sandbox.dir, &s(&["parse", "--print", &file]));
        self.compare(case, "parse --print reproduces the source", None, &source, &printed.stdout);

        // Diagnostics by identifier and position.
        let check = invoke(self.sisu, &sandbox.dir, &s(&["check", "--format=ids", &file]));
        let expected = case.expected("diag").unwrap_or_default();
        self.compare(case, "diagnostics", Some("diag"), &expected, &check.stdout);

        for (ext, args) in [
            ("tokens", vec!["tokens"]),
            ("tree", vec!["parse", "--tree"]),
            ("sir", vec!["sir"]),
            ("test", vec!["test"]),
        ] {
            let Some(expected) = case.expected(ext) else { continue };
            let mut a = s(&args);
            a.push(file.clone());
            let out = invoke(self.sisu, &sandbox.dir, &a);
            self.compare(case, &format!("sisu {}", args.join(" ")), Some(ext), &expected, &out.stdout);
        }

        let wants_run = ["out", "err", "status"].iter().any(|e| case.path(e).exists());
        if !wants_run {
            return;
        }
        let args: Vec<String> =
            case.expected("args").map(|a| a.lines().map(str::to_string).collect()).unwrap_or_default();
        let mut run_args = s(&["run", &file]);
        run_args.extend(args.iter().cloned());
        let run = invoke(self.sisu, &sandbox.dir, &run_args);
        self.compare(case, "stdout", Some("out"), &case.expected("out").unwrap_or_default(), &run.stdout);
        self.compare(case, "stderr", Some("err"), &case.expected("err").unwrap_or_default(), &run.stderr);
        let status = case.expected("status").map(|s| s.trim().to_string()).unwrap_or_else(|| "0".into());
        let actual_status = run.status.to_string();
        let blessed_status = if actual_status == "0" { status.clone() } else { format!("{actual_status}\n") };
        self.report.checks += 1;
        if status != actual_status {
            if self.bless && std::fs::write(case.path("status"), &blessed_status).is_ok() {
                self.report.blessed.push(case.path("status").display().to_string());
            } else {
                self.report.failures.push(Failure {
                    case: case.name.clone(),
                    check: "exit status".into(),
                    detail: format!("expected {status}, found {actual_status}"),
                });
            }
        }

        // The serialised SIR is a complete program: running it gives the
        // same result as running the source.
        let sir = invoke(self.sisu, &sandbox.dir, &s(&["sir", &file]));
        if sir.status == 0 {
            let sir_file = format!("{}.sir", case.stem);
            if std::fs::write(sandbox.dir.join(&sir_file), &sir.stdout).is_ok() {
                let mut a = s(&["run", "--sir", &sir_file]);
                a.extend(args);
                let again = invoke(self.sisu, &sandbox.dir, &a);
                self.compare(case, "run --sir: stdout", None, &run.stdout, &again.stdout);
                // Warnings precede the program's own output when running
                // the source; the SIR has none to print.
                let source_err = if check.stdout.is_empty() {
                    run.stderr.as_str()
                } else {
                    run.stderr.get(run.stderr.len().saturating_sub(again.stderr.len())..).unwrap_or("")
                };
                self.compare(case, "run --sir: stderr", None, source_err, &again.stderr);
                self.compare(
                    case,
                    "run --sir: exit status",
                    None,
                    &run.status.to_string(),
                    &again.status.to_string(),
                );
            }
        }
    }
}

/// Runs every case under `root` with the executable `sisu`. With `bless`,
/// mismatching expectations are rewritten from the actual output instead
/// of failing (review the result before committing it).
pub fn run_all(sisu: &Path, root: &Path, bless: bool) -> Report {
    let mut r = Runner { sisu, bless, report: Report::default(), counter: 0 };
    for case in discover(root) {
        r.run_case(&case);
    }
    r.report
}
