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
pub mod native;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// One `.sisu` file, or one package (a `NAME.pkg` directory), and the
/// expectations beside it.
#[derive(Clone, Debug)]
pub struct Case {
    /// The path relative to the suite root, without `.sisu` or `.pkg`.
    pub name: String,
    pub dir: PathBuf,
    pub stem: String,
    /// A package case: `NAME.pkg/` holds a package (or a workspace, or
    /// several packages), and every command runs in it without naming a
    /// file (09).
    pub package: bool,
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
    /// Cases that ran, each also under the ownership monitor.
    pub monitored: usize,
    /// Counterexample-mode runs of rejected programs (`.defer`).
    pub deferred: usize,
    /// Cases run again under `--explore N` (`# explore N`), and the
    /// schedules that makes.
    pub explored: usize,
    pub schedules: u64,
    /// Cases whose run was recorded and replayed (N-04).
    pub replayed: usize,
    /// Failing tests run again from their replay tokens.
    pub tokens: usize,
    /// Test runs repeated on one thread (`--jobs 1`), which must print
    /// what the parallel run printed.
    pub serial: usize,
    /// Package cases (`NAME.pkg`), among `cases`.
    pub packages: usize,
    /// Files formatted, each checked to keep its tree and to be left as
    /// it is by a second `sisu fmt`.
    pub formatted: usize,
    /// Cases run under `sisu run --watch` (`NAME.watch`), each also
    /// recorded and replayed.
    pub watched: usize,
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
            "{} cases ({} packages), {} checks, {} files formatted, {} runs under --watch, {} runs also under the monitor, {} recorded and replayed, {} failing tests replayed from their tokens, {} test runs repeated on one thread, {} counterexample runs, {} explored ({} schedules), {} failed\n",
            self.cases,
            self.packages,
            self.checks,
            self.formatted,
            self.watched,
            self.monitored,
            self.replayed,
            self.tokens,
            self.serial,
            self.deferred,
            self.explored,
            self.schedules,
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
        let case = |p: &Path, package: bool| {
            let stem = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            let rel = p.strip_prefix(root).unwrap_or(p).with_extension("");
            let name = rel.to_string_lossy().replace('\\', "/");
            Case { name, dir: p.parent().map(Path::to_path_buf).unwrap_or_default(), stem, package }
        };
        if p.is_dir() {
            // `X.pkg` directories are package cases; `X.files` hold a
            // case's input files.
            if p.extension().is_some_and(|e| e == "pkg") {
                out.push(case(&p, true));
            } else if !p.extension().is_some_and(|e| e == "files") {
                walk(root, &p, out);
            }
            continue;
        }
        if p.extension().is_some_and(|e| e == "sisu") {
            out.push(case(&p, false));
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
    invoke_with_input(sisu, cwd, args, None)
}

/// [`invoke`], with `input` as the standard input (else none).
pub fn invoke_with_input(sisu: &Path, cwd: &Path, args: &[String], input: Option<&str>) -> Output {
    let spawned = Command::new(sisu)
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    wait_output(spawned, input, sisu)
}

/// Waits for a spawned command (killing it past [`TIMEOUT`]), feeding it
/// `input`, and collects what it printed.
pub(crate) fn wait_output(
    spawned: std::io::Result<std::process::Child>,
    input: Option<&str>,
    program: &Path,
) -> Output {
    let mut child = match spawned {
        Ok(c) => c,
        Err(e) => {
            return Output {
                stdout: String::new(),
                stderr: format!("cannot run {}: {e}", program.display()),
                status: -1,
            };
        }
    };
    // The input is written on a thread of its own and the pipe closed, so
    // the program sees its end.
    if let (Some(text), Some(mut stdin)) = (input, child.stdin.take()) {
        let text = text.to_string();
        std::thread::spawn(move || {
            use std::io::Write as _;
            let _ = stdin.write_all(text.as_bytes());
        });
    }
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
        if case.package {
            copy_dir(&case.path("pkg"), &dir)?;
            return Ok(Sandbox { dir });
        }
        std::fs::copy(case.source(), dir.join(format!("{}.sisu", case.stem)))?;
        let files = case.path("files");
        if files.is_dir() {
            copy_dir(&files, &dir)?;
        }
        Ok(Sandbox { dir })
    }
}

/// Every `.sisu` file under `dir`, relative to it, sorted.
fn sisu_files(dir: &Path, prefix: &str, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    entries.sort();
    for p in entries {
        let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let rel = if prefix.is_empty() { name.clone() } else { format!("{prefix}/{name}") };
        if p.is_dir() {
            sisu_files(&p, &rel, out);
        } else if name.ends_with(".sisu") {
            out.push(rel);
        }
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
    /// Numbers the sandboxes, across the runner's threads.
    counter: &'a std::sync::atomic::AtomicUsize,
}

/// Each failing test's block of `sisu test` output, by the token that
/// replays it: from its `test ... FAILED` line to its `replay:` line, with
/// the property's case number and shrink count left out (a replay starts
/// from the shrunk case), and so is the line naming the schedule a search
/// found it under (the token holds the schedule).
fn failure_blocks(out: &str) -> Vec<(String, String)> {
    let mut blocks = Vec::new();
    let mut current: Option<String> = None;
    for line in out.lines() {
        if line.starts_with("test ") && line.ends_with(" ... FAILED") {
            current = Some(String::new());
        }
        if let Some(text) = current.as_mut() {
            if line.starts_with("with schedule ") {
                continue;
            }
            if let Some(rest) = line.strip_prefix("replay: sisu test --replay ") {
                let token = rest.split(' ').next().unwrap_or("").to_string();
                blocks.push((token, std::mem::take(text)));
                current = None;
                continue;
            }
            // `(case N, shrunk K times)` and `(case N, shrunk once)`.
            let shown = match line.find(" (case ") {
                Some(a) if line[a..].contains(", shrunk ") => match line[a..].find(')') {
                    Some(b) => format!("{}{}", &line[..a], &line[a + b + 1..]),
                    None => line.to_string(),
                },
                _ => line.to_string(),
            };
            text.push_str(&shown);
            text.push('\n');
        }
    }
    blocks
}

impl Runner<'_> {
    /// Every failing test of the `sisu test` output `out`, again from its
    /// replay token: it must fail the same way (17, "Simulation and
    /// replay").
    fn replay_tokens(&mut self, case: &Case, sandbox: &Sandbox, out: &str) {
        self.replay_tokens_in(case, &sandbox.dir, out);
    }

    /// [`Runner::replay_tokens`] in the directory `cwd`; a package case
    /// names no file.
    fn replay_tokens_in(&mut self, case: &Case, cwd: &Path, out: &str) {
        let file = format!("{}.sisu", case.stem);
        let first_test = |o: &str| failure_blocks(o).into_iter().next().map(|(_, b)| b).unwrap_or_default();
        for (token, block) in failure_blocks(out) {
            self.report.tokens += 1;
            let mut args = vec!["test".to_string(), "--replay".to_string(), token.clone()];
            if !case.package {
                args.push(file.clone());
            }
            let again = invoke(self.sisu, cwd, &args);
            self.compare(
                case,
                &format!("sisu test --replay {token}"),
                None,
                &block,
                &first_test(&again.stdout),
            );
        }
    }

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
        if case.package {
            return self.run_package_case(case);
        }
        self.report.cases += 1;
        let n = self.counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let sandbox = match Sandbox::new(case, n) {
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
        self.run_file_case(case, &sandbox);
        // Last, because it rewrites the case's file.
        self.format_checks(case, &sandbox.dir, &[format!("{}.sisu", case.stem)]);
    }

    /// The formatter (02, "Formatting"; 10, "Formatter"): every file of the
    /// case that parses keeps its tree when `sisu fmt` rewrites it (`sisu
    /// parse --shape` before and after: the tree up to what formatting
    /// changes on purpose), and formatting it again changes nothing. A
    /// case without a `NAME.unformatted` beside it must already be in the
    /// form `sisu fmt` writes.
    fn format_checks(&mut self, case: &Case, dir: &Path, files: &[String]) {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        let canonical = !case.path("unformatted").exists();
        for f in files {
            let before = invoke(self.sisu, dir, &s(&["parse", "--shape", f]));
            // A file with a syntax error is not formatted.
            if before.status != 0 {
                continue;
            }
            self.report.formatted += 1;
            if canonical {
                let check = invoke(self.sisu, dir, &s(&["fmt", "--check", f]));
                self.report.checks += 1;
                if check.status != 0 {
                    self.report.failures.push(Failure {
                        case: case.name.clone(),
                        check: format!("{f} is in canonical form (sisu fmt --check)"),
                        detail: format!("{}{}", check.stdout, check.stderr),
                    });
                }
            }
            let read = || std::fs::read_to_string(dir.join(f)).unwrap_or_default();
            let once = invoke(self.sisu, dir, &s(&["fmt", f]));
            self.compare(case, &format!("sisu fmt {f} succeeds"), None, "", &once.stderr);
            let formatted = read();
            let after = invoke(self.sisu, dir, &s(&["parse", "--shape", f]));
            self.compare(
                case,
                &format!("sisu fmt keeps the tree of {f}"),
                None,
                &before.stdout,
                &after.stdout,
            );
            let _ = invoke(self.sisu, dir, &s(&["fmt", f]));
            self.compare(case, &format!("formatting {f} again changes nothing"), None, &formatted, &read());
        }
    }

    /// `NAME.watch` (10, "Debugging and profiling"): the program under
    /// `sisu run --watch ARGS...`, which saves new versions of its own
    /// files (from its `NAME.files/`) to drive the swaps, so it runs in a
    /// sandbox of its own; then again recorded, and replayed from its log,
    /// whose `edit` records stand in for the saves.
    fn watch_checks(&mut self, case: &Case, expected: &str, watch_args: &[String], input: Option<&str>) {
        self.report.watched += 1;
        let shown = |o: &Output| format!("{}--- stderr\n{}--- status {}\n", o.stdout, o.stderr, o.status);
        let fresh = |r: &mut Runner<'_>| {
            let n = r.counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Sandbox::new(case, n)
        };
        let (Ok(first), Ok(second)) = (fresh(self), fresh(self)) else {
            self.report.failures.push(Failure {
                case: case.name.clone(),
                check: "setup".into(),
                detail: "a sandbox for `sisu run --watch`".into(),
            });
            return;
        };
        let cwd = |sb: &Sandbox| match case.expected("cwd") {
            Some(d) => sb.dir.join(d.trim()),
            None => sb.dir.clone(),
        };
        let out = invoke_with_input(self.sisu, &cwd(&first), watch_args, input);
        self.compare(case, "sisu run --watch", Some("watch"), expected, &shown(&out));
        let log = second.dir.join("watch.log").display().to_string();
        let mut recorded_args = watch_args.to_vec();
        recorded_args.splice(2..2, ["--record".to_string(), log.clone()]);
        let recorded = invoke_with_input(self.sisu, &cwd(&second), &recorded_args, input);
        self.compare(
            case,
            "sisu run --watch --record prints what the watched run printed",
            None,
            &shown(&out),
            &shown(&recorded),
        );
        let replayed = invoke(self.sisu, &cwd(&second), &["replay".to_string(), log]);
        self.compare(
            case,
            "sisu replay of a watched run prints what it printed",
            None,
            &shown(&recorded),
            &shown(&replayed),
        );
    }

    fn run_file_case(&mut self, case: &Case, sandbox: &Sandbox) {
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
            ("doc", vec!["check", "--explain"]),
        ] {
            let Some(expected) = case.expected(ext) else { continue };
            let mut a = s(&args);
            a.push(file.clone());
            let out = invoke(self.sisu, &sandbox.dir, &a);
            self.compare(case, &format!("sisu {}", args.join(" ")), Some(ext), &expected, &out.stdout);
            if ext == "test" {
                self.replay_tokens(case, sandbox, &out.stdout);
                // Tests run in parallel by default; on one thread they
                // must print the same (17, "Isolation and parallelism").
                self.report.serial += 1;
                // Run again, not taken from the test cache the first run
                // filled (decision 242).
                let one = invoke(self.sisu, &sandbox.dir, &s(&["test", "--jobs", "1", "--no-cache", &file]));
                self.compare(
                    case,
                    "sisu test --jobs 1 prints what the parallel run printed",
                    None,
                    &out.stdout,
                    &one.stdout,
                );
            }
        }

        let args: Vec<String> =
            case.expected("args").map(|a| a.lines().map(str::to_string).collect()).unwrap_or_default();
        // `NAME.stdin`: what every run of the program reads.
        let input = case.expected("stdin");
        let input = input.as_deref();
        // The counterexample mode (A2) on a program the checker rejects:
        // the monitor's verdict for this input.
        if let Some(expected) = case.expected("defer") {
            self.report.deferred += 1;
            let mut a = s(&["run", "--defer-borrow", &file]);
            a.extend(args.iter().cloned());
            let out = invoke(self.sisu, &sandbox.dir, &a);
            let actual = format!("{}--- stderr\n{}--- status {}\n", out.stdout, out.stderr, out.status);
            self.compare(case, "sisu run --defer-borrow", Some("defer"), &expected, &actual);
        }
        // `# sim ARGS` and `# schedule ARGS`: the tests again in the `sim`
        // profile, or under another scheduler, and each failure again from
        // its replay token.
        // `# select ARGS`: the tests the selection flags pick.
        for (marker, ext, sim) in
            [("# sim", "sim", true), ("# schedule", "schedule", false), ("# select", "select", false)]
        {
            let Some(line) =
                source.lines().find(|l| l.trim() == marker || l.trim().starts_with(&format!("{marker} ")))
            else {
                continue;
            };
            let mut a = s(&["test"]);
            if sim {
                a.push("--sim".into());
            }
            a.extend(line.trim()[marker.len()..].split_whitespace().map(str::to_string));
            a.push(file.clone());
            let out = invoke(self.sisu, &sandbox.dir, &a);
            let expected = case.expected(ext).unwrap_or_default();
            self.compare(case, &format!("sisu {}", a.join(" ")), Some(ext), &expected, &out.stdout);
            self.replay_tokens(case, sandbox, &out.stdout);
        }
        // `sisu explain-borrow` at the line a `# explain-borrow N` comment names.
        if let Some(expected) = case.expected("explain") {
            let line = source
                .lines()
                .find_map(|l| l.trim().strip_prefix("# explain-borrow ").map(|n| n.trim().to_string()))
                .unwrap_or_default();
            let out = invoke(self.sisu, &sandbox.dir, &s(&["explain-borrow", &format!("{file}:{line}")]));
            self.compare(case, "sisu explain-borrow", Some("explain"), &expected, &out.stdout);
        }
        // `NAME.watch`: the program under `sisu run --watch`.
        if let Some(expected) = case.expected("watch") {
            let mut a = s(&["run", "--watch", &file]);
            a.extend(args.iter().cloned());
            self.watch_checks(case, &expected, &a, input);
        }
        // `NAME.repl`: a session of `sisu repl NAME.sisu`, its inputs from
        // `NAME.stdin`; `NAME.ws`: `sisu ws NAME.sisu`, the worksheet.
        for (ext, cmd) in [("repl", "repl"), ("ws", "ws")] {
            let Some(expected) = case.expected(ext) else { continue };
            let out = invoke_with_input(self.sisu, &sandbox.dir, &s(&[cmd, &file]), input);
            let shown = format!("{}--- stderr\n{}--- status {}\n", out.stdout, out.stderr, out.status);
            self.compare(case, &format!("sisu {cmd}"), Some(ext), &expected, &shown);
        }
        // `# explore N`: the run (and the tests) again under N schedule
        // seeds; every seed must give the expected result.
        let explore = source
            .lines()
            .find_map(|l| l.trim().strip_prefix("# explore ").and_then(|n| n.trim().parse::<u64>().ok()));
        let wants_run = ["out", "err", "status"].iter().any(|e| case.path(e).exists());
        if let Some(n) = explore {
            self.report.explored += 1;
            self.report.schedules += n;
            if let Some(expected) = case.expected("explore") {
                let out = invoke(self.sisu, &sandbox.dir, &s(&["test", "--explore", &n.to_string(), &file]));
                self.compare(case, "sisu test --explore", Some("explore"), &expected, &out.stdout);
            }
            if wants_run {
                let mut a = s(&["run", "--explore", &n.to_string(), &file]);
                a.extend(args.iter().cloned());
                let out = invoke_with_input(self.sisu, &sandbox.dir, &a, input);
                let status =
                    case.expected("status").map(|s| s.trim().to_string()).unwrap_or_else(|| "0".into());
                self.compare(
                    case,
                    "sisu run --explore: every schedule's result",
                    None,
                    &format!("{}--- status {status}\n", case.expected("out").unwrap_or_default()),
                    &format!("{}--- status {}\n", out.stdout, out.status),
                );
            }
        }
        if !wants_run {
            return;
        }
        let mut run_args = s(&["run", &file]);
        run_args.extend(args.iter().cloned());
        let run = invoke_with_input(self.sisu, &sandbox.dir, &run_args, input);
        self.compare(case, "stdout", Some("out"), &case.expected("out").unwrap_or_default(), &run.stdout);
        // Principle 5 of 18: the checker checks itself. A program the
        // checker accepted runs the same under the dynamic ownership
        // monitor; any difference is the monitor rejecting it, which means
        // the static checker has a soundness hole.
        let mut monitored_args = s(&["run", "--monitor", &file]);
        monitored_args.extend(args.iter().cloned());
        let monitored = invoke_with_input(self.sisu, &sandbox.dir, &monitored_args, input);
        self.report.monitored += 1;
        self.compare(
            case,
            "the monitor agrees with the checker (a difference is a soundness hole)",
            None,
            &format!("{}\n--- stderr\n{}\n--- status {}\n", run.stdout, run.stderr, run.status),
            &format!(
                "{}\n--- stderr\n{}\n--- status {}\n",
                monitored.stdout, monitored.stderr, monitored.status
            ),
        );
        // Record and replay (N-04): the run again with every capability
        // result logged, then the log replayed with nothing of the world;
        // both print what the run printed and exit the same way. (The run
        // writes its files again, into the same directory, as the plain
        // run did.)
        let log = format!("{}.log", case.stem);
        let mut recorded_args = s(&["run", "--record", &log, &file]);
        recorded_args.extend(args.iter().cloned());
        let recorded = invoke_with_input(self.sisu, &sandbox.dir, &recorded_args, input);
        let shown = |o: &Output| format!("{}\n--- stderr\n{}\n--- status {}\n", o.stdout, o.stderr, o.status);
        self.report.replayed += 1;
        self.compare(
            case,
            "sisu run --record prints what the run printed",
            None,
            &shown(&run),
            &shown(&recorded),
        );
        let replayed = invoke(self.sisu, &sandbox.dir, &s(&["replay", &log]));
        self.compare(
            case,
            "sisu replay prints what the recorded run printed",
            None,
            &shown(&recorded),
            &shown(&replayed),
        );
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
                let again = invoke_with_input(self.sisu, &sandbox.dir, &a, input);
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

impl Runner<'_> {
    /// A package case (09): the package (or workspace) in `NAME.pkg/`,
    /// every command run in it, or in the directory `NAME.cwd` names,
    /// without naming a file. The checks are a file case's, over every
    /// file of the package, and the lockfile the commands wrote is
    /// compared with `NAME.lock`.
    fn run_package_case(&mut self, case: &Case) {
        self.report.cases += 1;
        self.report.packages += 1;
        let n = self.counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let sandbox = match Sandbox::new(case, n) {
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
        self.package_checks(case, &sandbox);
        let mut files = Vec::new();
        sisu_files(&sandbox.dir, "", &mut files);
        self.format_checks(case, &sandbox.dir, &files);
    }

    fn package_checks(&mut self, case: &Case, sandbox: &Sandbox) {
        let cwd = match case.expected("cwd") {
            Some(d) => sandbox.dir.join(d.trim()),
            None => sandbox.dir.clone(),
        };
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        // Every file of the package reprints itself.
        let mut files = Vec::new();
        sisu_files(&sandbox.dir, "", &mut files);
        for f in &files {
            let source = std::fs::read_to_string(sandbox.dir.join(f)).unwrap_or_default();
            let printed = invoke(self.sisu, &sandbox.dir, &s(&["parse", "--print", f]));
            self.compare(case, &format!("parse --print reproduces {f}"), None, &source, &printed.stdout);
        }
        let check = invoke(self.sisu, &cwd, &s(&["check", "--format=ids"]));
        let expected = case.expected("diag").unwrap_or_default();
        self.compare(case, "diagnostics", Some("diag"), &expected, &check.stdout);
        // The lockfile the check wrote, beside the package's manifest or its
        // workspace's.
        if let Some(expected) = case.expected("lock") {
            let mut found = String::new();
            let mut dir = Some(cwd.as_path());
            while let Some(d) = dir {
                if let Ok(t) = std::fs::read_to_string(d.join("sisu.lock")) {
                    found = t;
                    break;
                }
                if d == sandbox.dir {
                    break;
                }
                dir = d.parent();
            }
            self.compare(case, "sisu.lock", Some("lock"), &expected, &found);
        }
        if let Some(expected) = case.expected("build") {
            let out = invoke(self.sisu, &cwd, &s(&["build"]));
            let shown = format!("{}--- stderr\n{}--- status {}\n", out.stdout, out.stderr, out.status);
            self.compare(case, "sisu build", Some("build"), &expected, &shown);
        }
        if let Some(expected) = case.expected("test") {
            let out = invoke(self.sisu, &cwd, &s(&["test"]));
            self.compare(case, "sisu test", Some("test"), &expected, &out.stdout);
            self.replay_tokens_in(case, &cwd, &out.stdout);
            self.report.serial += 1;
            let one = invoke(self.sisu, &cwd, &s(&["test", "--jobs", "1", "--no-cache"]));
            self.compare(
                case,
                "sisu test --jobs 1 prints what the parallel run printed",
                None,
                &out.stdout,
                &one.stdout,
            );
        }
        let args: Vec<String> =
            case.expected("args").map(|a| a.lines().map(str::to_string).collect()).unwrap_or_default();
        let input = case.expected("stdin");
        let input = input.as_deref();
        if let Some(expected) = case.expected("watch") {
            let mut a = s(&["run", "--watch"]);
            if !args.is_empty() {
                a.push("--".into());
                a.extend(args.iter().cloned());
            }
            self.watch_checks(case, &expected, &a, input);
        }
        // `NAME.repl`: a session of `sisu repl` with the package loaded.
        if let Some(expected) = case.expected("repl") {
            let out = invoke_with_input(self.sisu, &cwd, &s(&["repl"]), input);
            let shown = format!("{}--- stderr\n{}--- status {}\n", out.stdout, out.stderr, out.status);
            self.compare(case, "sisu repl", Some("repl"), &expected, &shown);
        }
        let wants_run = ["out", "err", "status"].iter().any(|e| case.path(e).exists());
        if !wants_run {
            return;
        }
        let with_args = |cmd: &[&str]| {
            let mut a = s(cmd);
            if !args.is_empty() {
                a.push("--".into());
                a.extend(args.iter().cloned());
            }
            a
        };
        let run = invoke_with_input(self.sisu, &cwd, &with_args(&["run"]), input);
        self.compare(case, "stdout", Some("out"), &case.expected("out").unwrap_or_default(), &run.stdout);
        self.compare(case, "stderr", Some("err"), &case.expected("err").unwrap_or_default(), &run.stderr);
        let status = case.expected("status").map(|s| s.trim().to_string()).unwrap_or_else(|| "0".into());
        self.report.checks += 1;
        if status != run.status.to_string() {
            if self.bless && std::fs::write(case.path("status"), format!("{}\n", run.status)).is_ok() {
                self.report.blessed.push(case.path("status").display().to_string());
            } else {
                self.report.failures.push(Failure {
                    case: case.name.clone(),
                    check: "exit status".into(),
                    detail: format!("expected {status}, found {}", run.status),
                });
            }
        }
        let shown = |o: &Output| format!("{}\n--- stderr\n{}\n--- status {}\n", o.stdout, o.stderr, o.status);
        let monitored = invoke_with_input(self.sisu, &cwd, &with_args(&["run", "--monitor"]), input);
        self.report.monitored += 1;
        self.compare(
            case,
            "the monitor agrees with the checker (a difference is a soundness hole)",
            None,
            &shown(&run),
            &shown(&monitored),
        );
        let log = sandbox.dir.join("run.log").display().to_string();
        let recorded = invoke_with_input(self.sisu, &cwd, &with_args(&["run", "--record", &log]), input);
        self.report.replayed += 1;
        self.compare(
            case,
            "sisu run --record prints what the run printed",
            None,
            &shown(&run),
            &shown(&recorded),
        );
        let replayed = invoke(self.sisu, &cwd, &s(&["replay", &log]));
        self.compare(
            case,
            "sisu replay prints what the recorded run printed",
            None,
            &shown(&recorded),
            &shown(&replayed),
        );
        let sir = invoke(self.sisu, &cwd, &s(&["sir"]));
        if sir.status == 0 {
            let sir_file = sandbox.dir.join("program.sir").display().to_string();
            if std::fs::write(&sir_file, &sir.stdout).is_ok() {
                let mut a = s(&["run", "--sir", &sir_file]);
                a.extend(args.iter().cloned());
                let again = invoke_with_input(self.sisu, &cwd, &a, input);
                self.compare(case, "run --sir: stdout", None, &run.stdout, &again.stdout);
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
///
/// The cases run on as many threads as the machine has (at most eight),
/// each case in a sandbox of its own; the report lists the failures in the
/// cases' order, as one thread would.
pub fn run_all(sisu: &Path, root: &Path, bless: bool) -> Report {
    let cases = discover(root);
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1).clamp(1, 8);
    let counter = std::sync::atomic::AtomicUsize::new(0);
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results: std::sync::Mutex<Vec<(usize, Report)>> = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some(case) = cases.get(i) else { break };
                    let mut r = Runner { sisu, bless, report: Report::default(), counter: &counter };
                    r.run_case(case);
                    if let Ok(mut all) = results.lock() {
                        all.push((i, r.report));
                    }
                }
            });
        }
    });
    let mut all = results.into_inner().unwrap_or_default();
    all.sort_by_key(|(i, _)| *i);
    let mut report = Report::default();
    for (_, r) in all {
        report.cases += r.cases;
        report.checks += r.checks;
        report.monitored += r.monitored;
        report.deferred += r.deferred;
        report.explored += r.explored;
        report.schedules += r.schedules;
        report.replayed += r.replayed;
        report.tokens += r.tokens;
        report.serial += r.serial;
        report.packages += r.packages;
        report.formatted += r.formatted;
        report.watched += r.watched;
        report.failures.extend(r.failures);
        report.blessed.extend(r.blessed);
    }
    report
}
