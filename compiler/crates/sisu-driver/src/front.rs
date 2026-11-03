//! The front end the `sisu` command uses (decision 268): stage 1, the
//! front end written in Sisu (`compiler/stage1`), compiled natively, unless
//! `--front-end=rust` or `SISU_FRONT_END=rust` asks for the Rust one, which
//! stays as the oracle until each of its roles has a replacement.
//!
//! A command that stage 1 serves runs stage 1's own command of the same
//! name and prints what it printed: its diagnostics, its SIR, its pages,
//! what its formatter wrote. A command that needs a program's SIR, to run
//! it (`sisu run`) or to compile it (`sisu build --native`, `sisu run
//! --native`), runs stage 1's `build --native`, which checks, lowers and
//! checks ownership as `sisu build` does and hands the SIR to a backend:
//! this executable, through a home directory of links whose
//! `target/release/sisu` is it (stage 1 calls the backend by that name).
//! To run the SIR rather than compile it, the backend is asked to keep the
//! SIR it was given ([`KEEP_SIR`]); the interpreter then runs it, as `sisu
//! run --from-sir` would.
//!
//! The stage-1 executable is `compiler/target/stage1/sisu1`, where the
//! `stage1/sisu1` script builds it, and is built (by this executable, with
//! the Rust front end, `sisu build --native` in `compiler/stage1`) when it
//! is missing or a source of stage 1 is newer, as the script does.

use sisu_diag::msg;
use sisu_interp::Host;
use std::path::{Path, PathBuf};

/// The environment variable that chooses the front end for a whole process
/// (`rust` or `stage1`); `--front-end=NAME` on the command line wins.
pub const FRONT_END_VAR: &str = "SISU_FRONT_END";

/// Set in stage 1's environment when the driver wants the SIR of the
/// program rather than an executable: the backend stage 1 calls, `sisu
/// build --native --from-sir FILE --output OUT`, then writes the SIR it was
/// given to `OUT` and compiles nothing. Internal to the driver and stage 1.
pub const KEEP_SIR: &str = "SISU_KEEP_SIR";

/// The variables of the native runtime, which stage 1 and stage 2, native
/// programs, would read for themselves: they are the program's, not theirs.
pub(crate) const RUNTIME_VARS: &[&str] =
    &["SISU_RT_ALLOCS", "SISU_SCHEDULE_SEED", "SISU_STATS", "SISU_TASK_STACK"];

/// The environment variable that chooses how a native build lowers SIR
/// (`stage2` or `rust`) for a whole process, and that the driver gives
/// stage 1 so that its backend lowers as the command chose; `--lower=NAME`
/// wins (decision 304).
pub const LOWER_VAR: &str = "SISU_LOWER";

/// The environment variable that chooses the backend that compiles LIR to
/// machine code (`cranelift` or `sisu`) for a whole process, and that the
/// driver gives stage 1 so that its backend compiles as the command chose;
/// `--backend=NAME` wins (decision 306).
pub const BACKEND_VAR: &str = "SISU_BACKEND";

/// Which front end checks, lowers and serialises a program.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrontEnd {
    /// Stage 1, the front end in Sisu: the default.
    Stage1,
    /// The Rust front end of stage 0, the oracle.
    Rust,
}

impl FrontEnd {
    fn parse(name: &str) -> Option<FrontEnd> {
        match name {
            "stage1" => Some(FrontEnd::Stage1),
            "rust" => Some(FrontEnd::Rust),
            _ => None,
        }
    }
}

/// The command line without its `--front-end=NAME` (or `--front-end
/// NAME`), which may come before the command or among the command's
/// options before its file, and the front end it and `SISU_FRONT_END`
/// choose. `value_flags` are the options followed by a value; after a
/// file, `--` or `run`'s `--from-sir FILE`, the words are the program's. A
/// name that is no front end is a usage error.
pub fn take_choice(
    args: &[String],
    env: Option<&str>,
    value_flags: &[&str],
    host: &mut dyn Host,
) -> Result<(Vec<String>, FrontEnd), i32> {
    let mut chosen: Option<&str> = None;
    let mut out = Vec::with_capacity(args.len());
    let mut command: Option<&str> = None;
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        if a == "--" || (command.is_some() && !a.starts_with("--")) {
            out.extend(args[i..].iter().cloned());
            break;
        }
        if let Some(v) = a.strip_prefix("--front-end=") {
            chosen = Some(v);
            i += 1;
            continue;
        }
        if a == "--front-end" {
            chosen = Some(args.get(i + 1).map(String::as_str).unwrap_or(""));
            i += 2;
            continue;
        }
        out.push(args[i].clone());
        i += 1;
        if !a.starts_with("--") {
            command = Some(a);
            continue;
        }
        let takes_value = value_flags.contains(&a)
            || (a == "--stats" && args.get(i).is_some_and(|k| k == "stack-depth" || k == "shared"));
        if takes_value && let Some(v) = args.get(i) {
            out.push(v.clone());
            i += 1;
        }
        // What follows `sisu run --from-sir FILE` is the program's
        // (decision 254).
        if command == Some("run") && (a == "--from-sir" || a.starts_with("--from-sir=")) {
            out.extend(args[i..].iter().cloned());
            break;
        }
    }
    let name = chosen.or(env).filter(|n| !n.is_empty());
    let front = match name {
        None => FrontEnd::Stage1,
        Some(n) => match FrontEnd::parse(n) {
            Some(f) => f,
            None => {
                host.stderr(&crate::cli::plain_error(msg::CLI_BAD_FRONT_END, &[("value", n.to_string())]));
                return Err(crate::cli::EXIT_USAGE);
            }
        },
    };
    Ok((out, front))
}

/// The command line without its `--lower=NAME` (or `--lower NAME`), taken
/// where [`take_choice`] takes `--front-end`, and the lowering it and
/// `SISU_LOWER` choose (decision 304). A name that is no lowering is a
/// usage error.
pub(crate) fn take_lowering(
    args: &[String],
    env: Option<&str>,
    value_flags: &[&str],
    host: &mut dyn Host,
) -> Result<(Vec<String>, crate::native::Lowering), i32> {
    let (out, chosen) = take_option(args, "--lower", value_flags);
    let name = chosen.or_else(|| env.map(str::to_string)).filter(|n| !n.is_empty());
    let lowering = match name {
        None => crate::native::Lowering::Stage2,
        Some(n) => match crate::native::Lowering::parse(&n) {
            Some(l) => l,
            None => {
                host.stderr(&crate::cli::plain_error(msg::CLI_BAD_LOWER, &[("value", n)]));
                return Err(crate::cli::EXIT_USAGE);
            }
        },
    };
    Ok((out, lowering))
}

/// The command line without its `--backend=NAME` (or `--backend NAME`),
/// and the backend it and `SISU_BACKEND` choose (decision 306): Cranelift
/// unless `sisu` is named. A name that is no backend is a usage error.
pub(crate) fn take_backend(
    args: &[String],
    env: Option<&str>,
    value_flags: &[&str],
    host: &mut dyn Host,
) -> Result<(Vec<String>, crate::native::Backend), i32> {
    let (out, chosen) = take_option(args, "--backend", value_flags);
    let name = chosen.or_else(|| env.map(str::to_string)).filter(|n| !n.is_empty());
    let backend = match name {
        None => crate::native::Backend::Cranelift,
        Some(n) => match crate::native::Backend::parse(&n) {
            Some(b) => b,
            None => {
                host.stderr(&crate::cli::plain_error(msg::CLI_BAD_BACKEND, &[("value", n)]));
                return Err(crate::cli::EXIT_USAGE);
            }
        },
    };
    Ok((out, backend))
}

/// The command line without the option `name` (`--name=VALUE` or `--name
/// VALUE`), taken among a command's options as [`take_choice`] takes
/// `--front-end`, and its value, the last given.
fn take_option(args: &[String], name: &str, value_flags: &[&str]) -> (Vec<String>, Option<String>) {
    let mut chosen: Option<String> = None;
    let mut out = Vec::with_capacity(args.len());
    let mut command: Option<&str> = None;
    let prefix = format!("{name}=");
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        if a == "--" || (command.is_some() && !a.starts_with("--")) {
            out.extend(args[i..].iter().cloned());
            break;
        }
        if let Some(v) = a.strip_prefix(prefix.as_str()) {
            chosen = Some(v.to_string());
            i += 1;
            continue;
        }
        if a == name {
            chosen = Some(args.get(i + 1).cloned().unwrap_or_default());
            i += 2;
            continue;
        }
        out.push(args[i].clone());
        i += 1;
        if !a.starts_with("--") {
            command = Some(a);
            continue;
        }
        if value_flags.contains(&a)
            && let Some(v) = args.get(i)
        {
            out.push(v.clone());
            i += 1;
        }
        if command == Some("run") && (a == "--from-sir" || a.starts_with("--from-sir=")) {
            out.extend(args[i..].iter().cloned());
            break;
        }
    }
    (out, chosen)
}

/// The compiler's directory, which holds `stage1/`, `std/`, `catalog/` and
/// `target/`: the tree this executable was built from.
pub fn compiler_dir() -> PathBuf {
    let dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    std::fs::canonicalize(&dir).unwrap_or(dir)
}

/// Where the stage-1 executable is built, the place `stage1/sisu1` builds
/// it.
pub fn stage1_exe(compiler: &Path) -> PathBuf {
    compiler.join("target").join("stage1").join("sisu1")
}

/// Whether the executable is missing or a `.sisu` file under `src` is newer
/// than it (`find src -name '*.sisu' -newer EXE`, as `stage1/sisu1` asks).
fn stale(exe: &Path, src: &Path) -> bool {
    let Ok(built) = std::fs::metadata(exe).and_then(|m| m.modified()) else { return true };
    newer_source(src, built)
}

fn newer_source(dir: &Path, than: std::time::SystemTime) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else { return false };
    for e in entries.flatten() {
        let p = e.path();
        let Ok(kind) = e.file_type() else { continue };
        if kind.is_dir() {
            if newer_source(&p, than) {
                return true;
            }
        } else if p.extension().is_some_and(|x| x == "sisu")
            && e.metadata().and_then(|m| m.modified()).is_ok_and(|t| t > than)
        {
            return true;
        }
    }
    false
}

/// The stage-1 front end, built and ready to run.
pub struct Stage1 {
    exe: PathBuf,
    compiler: PathBuf,
}

/// What a run of stage 1 printed, and the exit status it means: the line
/// a Sisu program's `main` prints when it returns an error (`error:
/// Failed`, or stage 1's `error: Exit(status=N)`, which gives the status)
/// taken off its standard error.
#[derive(Clone, Debug, Default)]
pub struct Ran {
    pub stdout: String,
    pub stderr: String,
    pub status: i32,
}

/// Which backend a run of stage 1 hands a program's SIR to, when its
/// command is `build --native`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    /// The command builds nothing: stage 1's home is the compiler's
    /// directory.
    Unused,
    /// This executable compiles the SIR.
    Native,
    /// This executable keeps the SIR, at the path `--output` names.
    KeepSir,
}

impl Stage1 {
    /// The stage-1 executable, built first when it is missing or older than
    /// a source of stage 1. A failed build is reported, with what the build
    /// printed, and gives the exit status.
    pub fn ready(host: &mut dyn Host) -> Result<Stage1, i32> {
        let compiler = compiler_dir();
        let exe = stage1_exe(&compiler);
        let package = compiler.join("stage1");
        let src = package.join("src");
        if !src.is_dir() {
            host.stderr(&crate::cli::plain_error(
                msg::FRONT_NO_STAGE1,
                &[("dir", package.display().to_string())],
            ));
            return Err(sisu_interp::exit::ERROR);
        }
        if stale(&exe, &src) {
            build(&exe, &package, &src, host)?;
        }
        Ok(Stage1 { exe, compiler })
    }

    /// Runs stage 1 with `args` (after `--home`) in the working directory,
    /// with no standard input, and gives what it printed.
    pub fn run(&self, args: &[String], backend: Backend, host: &mut dyn Host) -> Result<Ran, i32> {
        let home = match backend {
            Backend::Unused => None,
            Backend::Native | Backend::KeepSir => Some(Home::new(&self.compiler)),
        };
        let home_dir = home.as_ref().map(|h| h.dir.clone()).unwrap_or_else(|| self.compiler.clone());
        let mut cmd = std::process::Command::new(&self.exe);
        cmd.arg("--home").arg(&home_dir).args(args);
        cmd.stdin(std::process::Stdio::null());
        for v in RUNTIME_VARS.iter().chain([&FRONT_END_VAR, &KEEP_SIR]) {
            cmd.env_remove(v);
        }
        if backend == Backend::KeepSir {
            cmd.env(KEEP_SIR, "1");
        }
        // The backend stage 1 hands the SIR to lowers as the command chose
        // (decision 304).
        cmd.env(LOWER_VAR, crate::native::lowering().name());
        cmd.env(BACKEND_VAR, crate::native::backend().name());
        // Stage 1 knows the working directory by the shell's `PWD`.
        if let Ok(cwd) = std::env::current_dir() {
            cmd.env("PWD", cwd);
        }
        let out = match cmd.output() {
            Ok(o) => o,
            Err(e) => {
                host.stderr(&format!(
                    "{}\n",
                    crate::cli::text(
                        msg::NATIVE_CANNOT_RUN,
                        &[("path", self.exe.display().to_string()), ("reason", e.to_string())]
                    )
                ));
                return Err(sisu_interp::exit::ERROR);
            }
        };
        drop(home);
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        let status = exit_code(out.status);
        // A panic or an abort of stage 1 is an internal error of the
        // compiler, which the driver reports as its own (exit 70).
        if status == sisu_interp::exit::PANIC || status >= 128 {
            host.stdout(&stdout);
            host.stderr(&stderr);
            host.stderr(&crate::cli::plain_error(
                msg::FRONT_STAGE1_FAILED,
                &[("status", status.to_string())],
            ));
            return Err(crate::cli::EXIT_ICE);
        }
        Ok(settle(stdout, stderr, status))
    }
}

/// Stage 1's output with the line its `main` printed for the error it
/// returned taken off, and the exit status that error stands for.
fn settle(stdout: String, mut stderr: String, status: i32) -> Ran {
    if status == sisu_interp::exit::ERROR {
        let body = stderr.strip_suffix('\n').unwrap_or(&stderr);
        let start = body.rfind('\n').map_or(0, |i| i + 1);
        let last = &body[start..];
        let code = if last == "error: Failed" {
            Some(sisu_interp::exit::ERROR)
        } else {
            last.strip_prefix("error: Exit(status=")
                .and_then(|r| r.strip_suffix(')'))
                .and_then(|n| n.parse().ok())
        };
        if let Some(code) = code {
            stderr.truncate(start);
            return Ran { stdout, stderr, status: code };
        }
    }
    Ran { stdout, stderr, status }
}

/// Builds stage 1 with the Rust front end, as `stage1/sisu1` does (`sisu
/// build --native --output EXE` in `compiler/stage1`), under a lock, so
/// that the commands that start at once build it once: each waits for the
/// lock and then finds it built.
fn build(exe: &Path, package: &Path, src: &Path, host: &mut dyn Host) -> Result<(), i32> {
    let dir = exe.parent().map(Path::to_path_buf).unwrap_or_default();
    let fail = |host: &mut dyn Host, detail: String| {
        host.stderr(&crate::cli::plain_error(
            msg::FRONT_STAGE1_BUILD,
            &[("dir", package.display().to_string()), ("detail", detail)],
        ));
        sisu_interp::exit::ERROR
    };
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return Err(fail(host, e.to_string()));
    }
    let lock = match std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(dir.join(".build-lock"))
    {
        Ok(f) => f,
        Err(e) => return Err(fail(host, e.to_string())),
    };
    if let Err(e) = lock.lock() {
        return Err(fail(host, e.to_string()));
    }
    if !stale(exe, src) {
        return Ok(());
    }
    let me = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => return Err(fail(host, e.to_string())),
    };
    let part = dir.join(format!("sisu1.part-{}", std::process::id()));
    let mut cmd = std::process::Command::new(me);
    cmd.args(["build", "--native", "--output"]).arg(&part).current_dir(package);
    // Stage 1 is built with the Rust lowering, since stage 2 is built with
    // stage 1 (decision 304).
    cmd.env(FRONT_END_VAR, "rust").env(LOWER_VAR, "rust").env(BACKEND_VAR, "cranelift").env_remove(KEEP_SIR);
    for v in RUNTIME_VARS {
        cmd.env_remove(v);
    }
    cmd.stdin(std::process::Stdio::null());
    let out = match cmd.output() {
        Ok(o) => o,
        Err(e) => return Err(fail(host, e.to_string())),
    };
    if !out.status.success() || !part.is_file() {
        let _ = std::fs::remove_file(&part);
        let mut detail = String::from_utf8_lossy(&out.stdout).into_owned();
        detail.push_str(&String::from_utf8_lossy(&out.stderr));
        return Err(fail(host, detail.trim_end().to_string()));
    }
    if let Err(e) = std::fs::rename(&part, exe) {
        let _ = std::fs::remove_file(&part);
        return Err(fail(host, e.to_string()));
    }
    drop(lock);
    Ok(())
}

/// Stage 2, the lowering written in Sisu (`compiler/stage2`), built and
/// ready to run: what a native build lowers SIR with by default (decision
/// 304).
pub struct Stage2 {
    pub exe: PathBuf,
}

impl Stage2 {
    /// The stage-2 executable, `compiler/target/stage2/sisu2`, built first
    /// when it is missing or older than a source of stage 2 or of the `sir`
    /// module of stage 1 it depends on, as `stage2/sisu2` builds it.
    pub fn ready(host: &mut dyn Host) -> Result<Stage2, i32> {
        let compiler = compiler_dir();
        let exe = compiler.join("target").join("stage2").join("sisu2");
        let package = compiler.join("stage2");
        let fail = |host: &mut dyn Host, detail: String| {
            host.stderr(&crate::cli::plain_error(
                msg::NATIVE_STAGE2_BUILD,
                &[("dir", package.display().to_string()), ("detail", detail)],
            ));
            sisu_interp::exit::ERROR
        };
        if !package.join("src").is_dir() {
            return Err(fail(host, "no `src` there".into()));
        }
        if !stage2_stale(&exe, &compiler) {
            return Ok(Stage2 { exe });
        }
        let dir = exe.parent().map(Path::to_path_buf).unwrap_or_default();
        if let Err(e) = std::fs::create_dir_all(&dir) {
            return Err(fail(host, e.to_string()));
        }
        // Under a lock, so that the builds that start at once build it once.
        let lock = match std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(dir.join(".build-lock"))
        {
            Ok(f) => f,
            Err(e) => return Err(fail(host, e.to_string())),
        };
        if let Err(e) = lock.lock() {
            return Err(fail(host, e.to_string()));
        }
        if !stage2_stale(&exe, &compiler) {
            return Ok(Stage2 { exe });
        }
        let me = match std::env::current_exe() {
            Ok(p) => p,
            Err(e) => return Err(fail(host, e.to_string())),
        };
        let part = dir.join(format!("sisu2.part-{}", std::process::id()));
        let mut cmd = std::process::Command::new(me);
        cmd.args(["build", "--native", "--output"]).arg(&part).current_dir(&package);
        // Built through stage 1, with the Rust lowering: stage 2 does not
        // lower itself until it is built.
        cmd.env(LOWER_VAR, "rust").env(BACKEND_VAR, "cranelift").env_remove(FRONT_END_VAR).env_remove(KEEP_SIR);
        for v in RUNTIME_VARS {
            cmd.env_remove(v);
        }
        cmd.stdin(std::process::Stdio::null());
        let out = match cmd.output() {
            Ok(o) => o,
            Err(e) => return Err(fail(host, e.to_string())),
        };
        if !out.status.success() || !part.is_file() {
            let _ = std::fs::remove_file(&part);
            let mut detail = String::from_utf8_lossy(&out.stdout).into_owned();
            detail.push_str(&String::from_utf8_lossy(&out.stderr));
            return Err(fail(host, detail.trim_end().to_string()));
        }
        if let Err(e) = std::fs::rename(&part, &exe) {
            let _ = std::fs::remove_file(&part);
            return Err(fail(host, e.to_string()));
        }
        drop(lock);
        Ok(Stage2 { exe })
    }
}

/// Whether stage 2's executable is missing or older than a source of stage
/// 2 or of stage 1's `sir` module.
fn stage2_stale(exe: &Path, compiler: &Path) -> bool {
    let Ok(built) = std::fs::metadata(exe).and_then(|m| m.modified()) else { return true };
    let stage1 = compiler.join("stage1").join("src");
    newer_source(&compiler.join("stage2").join("src"), built)
        || newer_source(&stage1.join("sir"), built)
        || std::fs::metadata(stage1.join("sir.sisu")).and_then(|m| m.modified()).is_ok_and(|t| t > built)
}

/// A home directory for stage 1 whose `target/release/sisu`, the backend
/// stage 1 calls, is this executable, with the compiler's `catalog/` and
/// `std/` beside it, and `stage1/std/`, the standard library's modules only
/// stage 1 reads (its RULES.md, R4); removed when dropped.
struct Home {
    dir: PathBuf,
}

impl Home {
    fn new(compiler: &Path) -> Home {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("sisu-home-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let made = Home::link(&dir, compiler);
        // Where no link can be made, the compiler's own directory, whose
        // backend is its release `sisu`.
        if made.is_err() {
            let _ = std::fs::remove_dir_all(&dir);
            return Home { dir: compiler.to_path_buf() };
        }
        Home { dir }
    }

    #[cfg(unix)]
    fn link(dir: &Path, compiler: &Path) -> std::io::Result<()> {
        let release = dir.join("target").join("release");
        std::fs::create_dir_all(&release)?;
        std::os::unix::fs::symlink(compiler.join("catalog"), dir.join("catalog"))?;
        std::os::unix::fs::symlink(compiler.join("std"), dir.join("std"))?;
        std::fs::create_dir_all(dir.join("stage1"))?;
        std::os::unix::fs::symlink(compiler.join("stage1").join("std"), dir.join("stage1").join("std"))?;
        std::os::unix::fs::symlink(std::env::current_exe()?, release.join("sisu"))
    }

    #[cfg(not(unix))]
    fn link(_dir: &Path, _compiler: &Path) -> std::io::Result<()> {
        Err(std::io::Error::other("no symbolic links"))
    }
}

impl Drop for Home {
    fn drop(&mut self) {
        if self.dir.join("target").join("release").join("sisu").is_symlink() {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }
}

/// The backend's part when stage 1 asks for an executable and the driver
/// wants the SIR ([`KEEP_SIR`]): the SIR, as given, written where the
/// executable would go.
pub fn keep_sir(sir: &str, output: &Path, host: &mut dyn Host) -> i32 {
    match std::fs::write(output, sir) {
        Ok(()) => sisu_interp::exit::OK,
        Err(e) => {
            host.stderr(&crate::cli::plain_error(
                msg::CLI_CANNOT_WRITE,
                &[("path", output.display().to_string()), ("reason", e.to_string())],
            ));
            sisu_interp::exit::ERROR
        }
    }
}

/// A child's exit status; one a signal ended is 128 and its number.
fn exit_code(st: std::process::ExitStatus) -> i32 {
    if let Some(c) = st.code() {
        return c;
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt as _;
        if let Some(s) = st.signal() {
            return 128 + s;
        }
    }
    134
}

/// The executable a native build wrote, from the line `built PATH` the
/// backend printed (decision 231 decides where a package's goes, which
/// stage 1 knows).
pub fn built_path(stdout: &str) -> Option<PathBuf> {
    let marker = "\u{1}";
    let template =
        sisu_diag::Catalog::builtin().render(msg::NATIVE_BUILT.id(), &[("output", marker.to_string())]);
    let (before, after) = template.split_once(marker)?;
    stdout
        .lines()
        .rev()
        .find_map(|l| l.strip_prefix(before).and_then(|r| r.strip_suffix(after)))
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sisu_interp::TestHost;

    fn words(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    fn choice(args: &[&str], env: Option<&str>, h: &mut TestHost) -> Option<(Vec<String>, FrontEnd)> {
        take_choice(&words(args), env, &["--schedule-seed", "--from-sir"], h).ok()
    }

    #[test]
    fn the_choice_is_taken_from_the_options_only() {
        let mut h = TestHost::default();
        let rust = |v: &[&str]| Some((words(v), FrontEnd::Rust));
        let stage1 = |v: &[&str]| Some((words(v), FrontEnd::Stage1));
        assert_eq!(
            choice(&["--front-end=rust", "check", "a.sisu"], None, &mut h),
            rust(&["check", "a.sisu"])
        );
        assert_eq!(
            choice(&["run", "--front-end", "rust", "a.sisu", "--front-end=x"], None, &mut h),
            rust(&["run", "a.sisu", "--front-end=x"])
        );
        assert_eq!(
            choice(&["run", "--schedule-seed", "3", "--front-end=rust", "a.sisu"], None, &mut h),
            rust(&["run", "--schedule-seed", "3", "a.sisu"])
        );
        assert_eq!(
            choice(&["run", "--from-sir", "a.sir", "--front-end=rust"], None, &mut h),
            stage1(&["run", "--from-sir", "a.sir", "--front-end=rust"])
        );
        assert_eq!(
            choice(&["run", "--", "--front-end=rust"], Some("stage1"), &mut h),
            stage1(&["run", "--", "--front-end=rust"])
        );
        assert_eq!(choice(&["check"], Some("rust"), &mut h), rust(&["check"]));
        assert_eq!(choice(&["--front-end=stage1", "check"], Some("rust"), &mut h), stage1(&["check"]));
        assert_eq!(choice(&["--front-end=go", "check"], None, &mut h), None);
        assert!(h.err.contains("D0115"), "{}", h.err);
    }

    #[test]
    fn the_error_main_returned_is_the_status() {
        let r = settle("x\n".into(), "warning\nerror: Failed\n".into(), 1);
        assert_eq!((r.stderr.as_str(), r.status), ("warning\n", 1));
        let r = settle(String::new(), "error[D0009]: no\nerror: Exit(status=2)\n".into(), 1);
        assert_eq!((r.stderr.as_str(), r.status), ("error[D0009]: no\n", 2));
        let r = settle(String::new(), "error: NotFound(path=\"a\")\n".into(), 1);
        assert_eq!((r.stderr.as_str(), r.status), ("error: NotFound(path=\"a\")\n", 1));
        let r = settle(String::new(), "error: Failed\n".into(), 0);
        assert_eq!(r.status, 0);
    }

    #[test]
    fn the_built_line_names_the_executable() {
        let line = format!(
            "{}\n",
            sisu_diag::Catalog::builtin().render(msg::NATIVE_BUILT.id(), &[("output", "a/b c".into())])
        );
        assert_eq!(built_path(&line), Some(PathBuf::from("a/b c")));
        assert_eq!(built_path("nothing\n"), None);
    }
}
