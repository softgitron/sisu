//! `sisu build --native` and `sisu run --native` (stage 2): the program the
//! front end, lowering and the ownership checker made, exactly the SIR
//! `sisu sir` prints, compiled by `sisu-codegen` and linked against the
//! runtime cargo built beside this executable.

use sisu_diag::msg;
use sisu_interp::Host;
use std::path::{Path, PathBuf};

/// The exit status of a usage error and of a program the backend cannot
/// compile.
const EXIT_ERROR: i32 = 1;
const EXIT_ICE: i32 = 70;

/// A native build's profile (09, "Build"): `debug` compiles fast with
/// every check, `release` with Cranelift's optimisations, the checks still
/// on and the contracts its manifest asks for (decision 232).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Profile {
    Debug,
    Release,
}

impl Profile {
    pub(crate) fn of(release: bool) -> Profile {
        if release { Profile::Release } else { Profile::Debug }
    }

    /// The profile's name, its directory under `build/`.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Profile::Debug => "debug",
            Profile::Release => "release",
        }
    }
}

/// Where `sisu build --native` writes the executable (decision 231): for a
/// package, `build/PROFILE/NAME` in the directory of its lockfile, its
/// workspace's or its own; for a file, beside its source, named by its stem
/// (`hello.sisu` makes `hello`).
pub(crate) fn output_for(
    disk: &Path,
    package: Option<&str>,
    root: Option<&Path>,
    profile: Profile,
) -> PathBuf {
    match package {
        Some(name) => {
            // Without a lockfile's directory, the package's: the root
            // module is `src/main.sisu` in it.
            let dir = root
                .map(Path::to_path_buf)
                .or_else(|| disk.parent().and_then(Path::parent).map(Path::to_path_buf))
                .unwrap_or_default();
            dir.join("build").join(profile.name()).join(name)
        }
        None => disk.with_extension(""),
    }
}

fn text(id: msg::MsgId, params: &[(&str, String)]) -> String {
    format!("{}\n", sisu_diag::Catalog::builtin().render(id.id(), params))
}

/// Where the debug information says the sources are (decision 227): the
/// program's paths are relative to the directory the command runs in, and
/// the standard library's are in the tree this compiler was built from,
/// when it is still there.
fn sources() -> sisu_codegen::Sources {
    let comp_dir = std::env::current_dir().map(|d| d.display().to_string()).unwrap_or_default();
    let std_dir = std::fs::canonicalize(concat!(env!("CARGO_MANIFEST_DIR"), "/../../std"))
        .ok()
        .map(|d| d.display().to_string());
    sisu_codegen::Sources { comp_dir, std_dir }
}

/// How a native build lowers a SIR program (decisions 304 and 305): through
/// stage 2, the lowering written in Sisu, to LIR, which the Rust backend
/// translates one to one; or, only when asked (`--lower=rust`,
/// `SISU_LOWER=rust`), through the Rust lowering, the oracle that the
/// conformance runner compares stage 2's executables with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lowering {
    Stage2,
    Rust,
}

impl Lowering {
    pub fn parse(name: &str) -> Option<Lowering> {
        match name {
            "stage2" => Some(Lowering::Stage2),
            "rust" => Some(Lowering::Rust),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Lowering::Stage2 => "stage2",
            Lowering::Rust => "rust",
        }
    }
}

/// The lowering this process's command chose ([`set_lowering`]).
static LOWERING: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// The lowering the running command chose.
pub(crate) fn lowering() -> Lowering {
    match LOWERING.load(std::sync::atomic::Ordering::Relaxed) {
        1 => Lowering::Rust,
        _ => Lowering::Stage2,
    }
}

/// Chooses the lowering for the command this process runs.
pub(crate) fn set_lowering(l: Lowering) {
    LOWERING.store(u8::from(l == Lowering::Rust), std::sync::atomic::Ordering::Relaxed);
}

/// Which backend compiles LIR to machine code (decisions 306 and 331): the
/// x86-64 backend written in Sisu (`sisu2 x86`), the debug profile's by
/// default, or Cranelift, through the Rust translation, the release
/// profile's by default and the oracle (`--backend=NAME`,
/// `SISU_BACKEND=NAME`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    Cranelift,
    Sisu,
}

impl Backend {
    pub fn parse(name: &str) -> Option<Backend> {
        match name {
            "cranelift" => Some(Backend::Cranelift),
            "sisu" => Some(Backend::Sisu),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Backend::Cranelift => "cranelift",
            Backend::Sisu => "sisu",
        }
    }
}

/// The backend this process's command named ([`set_backend`]): 0 when it
/// named none, which leaves it to the build's profile ([`backend_for`]), 1
/// Cranelift, 2 the Sisu backend.
static BACKEND: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// The backend the running command named, if it named one.
pub(crate) fn chosen_backend() -> Option<Backend> {
    match BACKEND.load(std::sync::atomic::Ordering::Relaxed) {
        1 => Some(Backend::Cranelift),
        2 => Some(Backend::Sisu),
        _ => None,
    }
}

/// Chooses the backend for the command this process runs, or leaves it to
/// the default (`None`).
pub(crate) fn set_backend(b: Option<Backend>) {
    let code = match b {
        None => 0,
        Some(Backend::Cranelift) => 1,
        Some(Backend::Sisu) => 2,
    };
    BACKEND.store(code, std::sync::atomic::Ordering::Relaxed);
}

/// The backend that compiles a build of `profile` (decision 331): the one
/// the command named; else the Sisu backend for the debug profile, the
/// debug builds D-087 gives it, and Cranelift for the release profile,
/// whose optimisations stand in for D-088's release backend, and for the
/// Rust lowering, the oracle, which writes no LIR.
pub(crate) fn backend_for(profile: Profile) -> Backend {
    match chosen_backend() {
        Some(b) => b,
        None if profile == Profile::Debug && lowering() == Lowering::Stage2 => Backend::Sisu,
        None => Backend::Cranelift,
    }
}

/// What the backend compiles: a SIR program, or the text of a LIR program
/// (stage 2's lowering of one, decision 284).
#[derive(Clone, Copy)]
pub(crate) enum Code<'a> {
    Sir(&'a sisu_sir::Program),
    Lir(&'a str),
}

/// What a native program is linked with: the runtime in Rust's library and
/// the runtime in Sisu's object (decision 336).
pub(crate) struct Runtime {
    rust: std::path::PathBuf,
    sisu: std::path::PathBuf,
}

/// Compiles the program and links it at `output`. On failure the message
/// is reported to `host` and the exit status returned.
pub(crate) fn build(code: Code<'_>, output: &Path, profile: Profile, host: &mut dyn Host) -> Result<(), i32> {
    let exe = std::env::current_exe().unwrap_or_default();
    let runtime = match sisu_codegen::find_runtime(&exe) {
        Ok(r) => r,
        Err(e) => return Err(report(e, output, host)),
    };
    // The runtime in Sisu, linked with every program (decision 336).
    let rt = crate::front::Rt::ready(host)?;
    let runtime = Runtime { rust: runtime, sisu: rt.object };
    let prog = match code {
        Code::Sir(prog) => prog,
        Code::Lir(text) => {
            // A LIR program says its own profile.
            let profile = Profile::of(sisu_codegen::lir_is_release(text));
            if backend_for(profile) == Backend::Sisu {
                return build_with_sisu(text, &runtime, output, host);
            }
            let start = std::time::Instant::now();
            let compiled =
                sisu_codegen::compile_lir(text, &sources()).map_err(|e| report(e, output, host))?;
            return link(&compiled, start.elapsed(), &runtime, output, profile, host);
        }
    };
    // The Sisu backend compiles what stage 2 lowers (decision 306).
    if chosen_backend() == Some(Backend::Sisu) && lowering() == Lowering::Rust {
        host.stderr(&text(msg::NATIVE_BACKEND_LOWERING, &[]));
        return Err(EXIT_ERROR);
    }
    // `SISU_NATIVE_SURVEY=1`: every construct the backend does not compile
    // yet, rather than the first, as the Rust lowering, which lowers what
    // stage 2 lowers, finds them.
    if std::env::var_os("SISU_NATIVE_SURVEY").is_some() {
        match sisu_codegen::survey(prog) {
            Ok(missing) => {
                for (what, function, count) in &missing {
                    host.stderr(&format!("{count:>5}  {what} (first in `{function}`)\n"));
                }
                if !missing.is_empty() {
                    return Err(EXIT_ERROR);
                }
            }
            Err(e) => return Err(report(e, output, host)),
        }
    }
    // Through stage 2 (decisions 304 and 305): its LIR, translated. A
    // construct it does not lower is the program's `D0103`; nothing falls
    // back to the Rust lowering, which lowers only when it is asked for.
    if lowering() == Lowering::Stage2 {
        // With the Sisu backend, lowered and compiled in one process
        // (`sisu2 build`, decision 334).
        if backend_for(profile) == Backend::Sisu {
            return build_sir_with_sisu(prog, profile, &runtime, output, host);
        }
        let lir = lower_with_stage2(prog, profile, output, host)?;
        let start = std::time::Instant::now();
        let compiled = sisu_codegen::compile_lir(&lir, &sources()).map_err(|e| report(e, output, host))?;
        return link(&compiled, start.elapsed(), &runtime, output, profile, host);
    }
    let start = std::time::Instant::now();
    let options = sisu_codegen::Options { sources: sources(), release: profile == Profile::Release };
    let compiled = match sisu_codegen::compile_with_options(prog, &options) {
        Ok(c) => c,
        Err(e) => return Err(report(e, output, host)),
    };
    link(&compiled, start.elapsed(), &runtime, output, profile, host)
}

/// The LIR stage 2 lowers the program to (`sisu2 lir`, decision 304). A
/// construct stage 2 does not lower is reported as the backend's `D0103`,
/// naming it and its function as stage 2 does (`lir: not lowered: WHAT (in
/// FUNCTION)`); stage 2 failing otherwise is an internal error, reported.
fn lower_with_stage2(
    prog: &sisu_sir::Program,
    profile: Profile,
    output: &Path,
    host: &mut dyn Host,
) -> Result<String, i32> {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let stage2 = crate::front::Stage2::ready(host)?;
    let dir = std::env::temp_dir().join(format!(
        "sisu-lir-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let failed = |host: &mut dyn Host, status: i32, detail: String| {
        host.stderr(&text(msg::NATIVE_STAGE2_FAILED, &[("status", status.to_string()), ("detail", detail)]));
        EXIT_ICE
    };
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return Err(failed(host, 0, e.to_string()));
    }
    let (sir, lir) = (dir.join("program.sir"), dir.join("program.lir"));
    let start = std::time::Instant::now();
    let result =
        std::fs::write(&sir, sisu_sir::write::write(prog)).map_err(|e| e.to_string()).and_then(|()| {
            let mut cmd = std::process::Command::new(&stage2.exe);
            cmd.arg("lir").arg(&sir);
            if profile == Profile::Release {
                cmd.arg("--release");
            }
            cmd.arg("--output").arg(&lir).stdin(std::process::Stdio::null());
            // Stage 2 is a native program: the runtime's variables are the
            // program's being built, not its.
            for v in crate::front::RUNTIME_VARS {
                cmd.env_remove(v);
            }
            cmd.output().map_err(|e| e.to_string())
        });
    let lowered = match result {
        Ok(out) if out.status.success() => match std::fs::read_to_string(&lir) {
            Ok(text) => Ok(text),
            Err(e) => Err(failed(host, 0, e.to_string())),
        },
        Ok(out) => {
            let err = String::from_utf8_lossy(&out.stderr).into_owned();
            match err.lines().find_map(|l| l.strip_prefix("lir: not lowered: ")) {
                Some(what) => Err(not_lowered(what, output, host)),
                None => {
                    let status = out.status.code().unwrap_or(-1);
                    Err(failed(host, status, format!("{}{err}", String::from_utf8_lossy(&out.stdout))))
                }
            }
        }
        Err(e) => Err(failed(host, 0, e)),
    };
    let _ = std::fs::remove_dir_all(&dir);
    if std::env::var_os("SISU_NATIVE_TIMES").is_some() {
        let what = if lowered.is_ok() { "lowered by stage 2" } else { "not lowered by stage 2" };
        host.stderr(&format!("native: {what} in {:.1} ms\n", start.elapsed().as_secs_f64() * 1e3));
    }
    lowered
}

/// A LIR program compiled by the x86-64 backend written in Sisu (`sisu2
/// x86`, decision 306) into an object, which is linked at `output` with the
/// runtime as Cranelift's objects are. A construct the backend does not
/// compile is the program's `D0103`; another failure an internal error.
fn build_with_sisu(lir: &str, runtime: &Runtime, output: &Path, host: &mut dyn Host) -> Result<(), i32> {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let stage2 = crate::front::Stage2::ready(host)?;
    let dir = std::env::temp_dir().join(format!(
        "sisu-x86-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let failed = |host: &mut dyn Host, status: i32, detail: String| {
        host.stderr(&text(msg::NATIVE_SISU_FAILED, &[("status", status.to_string()), ("detail", detail)]));
        EXIT_ICE
    };
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return Err(failed(host, 0, e.to_string()));
    }
    let (file, obj) = (dir.join("program.lir"), dir.join("program.o"));
    let start = std::time::Instant::now();
    let result = std::fs::write(&file, lir).map_err(|e| e.to_string()).and_then(|()| {
        let mut cmd = std::process::Command::new(&stage2.exe);
        cmd.arg("x86").arg(&file).arg("--output").arg(&obj).stdin(std::process::Stdio::null());
        // Where the line table says the sources are (decision 312).
        let src = sources();
        cmd.arg("--comp-dir").arg(&src.comp_dir);
        if let Some(d) = &src.std_dir {
            cmd.arg("--std").arg(d);
        }
        for v in crate::front::RUNTIME_VARS {
            cmd.env_remove(v);
        }
        cmd.output().map_err(|e| e.to_string())
    });
    let compiled = match result {
        Ok(out) if out.status.success() => std::fs::read(&obj).map_err(|e| failed(host, 0, e.to_string())),
        Ok(out) => {
            let err = String::from_utf8_lossy(&out.stderr).into_owned();
            match err.lines().find_map(|l| l.strip_prefix("x86: not compiled: ")) {
                Some(what) => Err(not_lowered(what, output, host)),
                None => {
                    let status = out.status.code().unwrap_or(-1);
                    Err(failed(host, status, format!("{}{err}", String::from_utf8_lossy(&out.stdout))))
                }
            }
        }
        Err(e) => Err(failed(host, 0, e)),
    };
    let _ = std::fs::remove_dir_all(&dir);
    let object = compiled?;
    let generated = start.elapsed();
    if let Some(dir) = output.parent()
        && !dir.as_os_str().is_empty()
    {
        let _ = std::fs::create_dir_all(dir);
    }
    let start = std::time::Instant::now();
    let linked = sisu_codegen::link(&object, &runtime.rust, &runtime.sisu, output)
        .map_err(|e| report(e, output, host));
    if std::env::var_os("SISU_NATIVE_TIMES").is_some() {
        host.stderr(&format!(
            "native: compiled by the Sisu backend in {:.1} ms, linked in {:.1} ms\n",
            generated.as_secs_f64() * 1e3,
            start.elapsed().as_secs_f64() * 1e3
        ));
    }
    linked
}

/// A SIR program lowered by stage 2 and compiled by its backend in one
/// process (`sisu2 build`, decision 334), the object linked at `output`
/// with the runtime. A construct stage 2 does not lower, or its backend
/// does not compile, is the program's `D0103`; another failure an internal
/// error, of the lowering (`D0122`) or of the backend (`D0125`).
fn build_sir_with_sisu(
    prog: &sisu_sir::Program,
    profile: Profile,
    runtime: &Runtime,
    output: &Path,
    host: &mut dyn Host,
) -> Result<(), i32> {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let stage2 = crate::front::Stage2::ready(host)?;
    let dir = std::env::temp_dir().join(format!(
        "sisu-build-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let failed = |host: &mut dyn Host, id: msg::MsgId, status: i32, detail: String| {
        host.stderr(&text(id, &[("status", status.to_string()), ("detail", detail)]));
        EXIT_ICE
    };
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return Err(failed(host, msg::NATIVE_SISU_FAILED, 0, e.to_string()));
    }
    let (sir, obj) = (dir.join("program.sir"), dir.join("program.o"));
    let start = std::time::Instant::now();
    let result =
        std::fs::write(&sir, sisu_sir::write::write(prog)).map_err(|e| e.to_string()).and_then(|()| {
            let mut cmd = std::process::Command::new(&stage2.exe);
            cmd.arg("build").arg(&sir).arg("--output").arg(&obj).arg("--times");
            if profile == Profile::Release {
                cmd.arg("--release");
            }
            // Where the line table says the sources are (decision 312).
            let src = sources();
            cmd.arg("--comp-dir").arg(&src.comp_dir);
            if let Some(d) = &src.std_dir {
                cmd.arg("--std").arg(d);
            }
            cmd.stdin(std::process::Stdio::null());
            // Stage 2 is a native program: the runtime's variables are the
            // program's being built, not its.
            for v in crate::front::RUNTIME_VARS {
                cmd.env_remove(v);
            }
            cmd.output().map_err(|e| e.to_string())
        });
    let mut times = (None, None);
    let compiled = match result {
        Ok(out) if out.status.success() => {
            let err = String::from_utf8_lossy(&out.stderr);
            times = stage2_times(&err);
            std::fs::read(&obj).map_err(|e| failed(host, msg::NATIVE_SISU_FAILED, 0, e.to_string()))
        }
        Ok(out) => {
            let err = String::from_utf8_lossy(&out.stderr).into_owned();
            let missing = err.lines().find_map(|l| {
                l.strip_prefix("lir: not lowered: ").or_else(|| l.strip_prefix("x86: not compiled: "))
            });
            match missing {
                Some(what) => Err(not_lowered(what, output, host)),
                None => {
                    let status = out.status.code().unwrap_or(-1);
                    let id = if err.contains("lir: internal error") {
                        msg::NATIVE_STAGE2_FAILED
                    } else {
                        msg::NATIVE_SISU_FAILED
                    };
                    Err(failed(host, id, status, format!("{}{err}", String::from_utf8_lossy(&out.stdout))))
                }
            }
        }
        Err(e) => Err(failed(host, msg::NATIVE_SISU_FAILED, 0, e)),
    };
    let _ = std::fs::remove_dir_all(&dir);
    let object = compiled?;
    let generated = start.elapsed();
    if let Some(dir) = output.parent()
        && !dir.as_os_str().is_empty()
    {
        let _ = std::fs::create_dir_all(dir);
    }
    let start = std::time::Instant::now();
    let linked = sisu_codegen::link(&object, &runtime.rust, &runtime.sisu, output)
        .map_err(|e| report(e, output, host));
    if std::env::var_os("SISU_NATIVE_TIMES").is_some() {
        // The process's time, the SIR written and the object read, is the
        // two steps' as stage 2 measured them.
        let (lowered, compiled) = match times {
            (Some(l), Some(c)) => (l, c),
            _ => (generated.as_secs_f64() * 1e3, 0.0),
        };
        host.stderr(&format!("native: lowered by stage 2 in {lowered:.1} ms\n"));
        host.stderr(&format!(
            "native: compiled by the Sisu backend in {compiled:.1} ms, linked in {:.1} ms\n",
            start.elapsed().as_secs_f64() * 1e3
        ));
    }
    linked
}

/// The times `sisu2 build --times` printed: `sisu2: lowered in L ms,
/// compiled in C ms`.
fn stage2_times(err: &str) -> (Option<f64>, Option<f64>) {
    for line in err.lines() {
        if let Some(rest) = line.strip_prefix("sisu2: lowered in ")
            && let Some((l, c)) = rest.split_once(" ms, compiled in ")
        {
            let c = c.strip_suffix(" ms").unwrap_or(c);
            return (l.trim().parse().ok(), c.trim().parse().ok());
        }
    }
    (None, None)
}

/// Stage 2's `lir: not lowered: WHAT (in FUNCTION)`, or its backend's `x86:
/// not compiled: WHAT (in FUNCTION)`, reported as the backend's `D0103`:
/// its exit status.
fn not_lowered(what: &str, output: &Path, host: &mut dyn Host) -> i32 {
    let (what, function) = match what.rsplit_once(" (in ") {
        Some((w, f)) => (w, f.strip_suffix(')').unwrap_or(f)),
        None => (what, ""),
    };
    report(
        sisu_codegen::Error::Unsupported { what: what.to_string(), function: function.to_string() },
        output,
        host,
    )
}

/// Links a compiled program at `output`, and reports where a build's time
/// went when asked.
fn link(
    compiled: &sisu_codegen::Compiled,
    generated: std::time::Duration,
    runtime: &Runtime,
    output: &Path,
    profile: Profile,
    host: &mut dyn Host,
) -> Result<(), i32> {
    if let Some(dir) = output.parent()
        && !dir.as_os_str().is_empty()
    {
        let _ = std::fs::create_dir_all(dir);
    }
    let start = std::time::Instant::now();
    let linked = sisu_codegen::link(&compiled.object, &runtime.rust, &runtime.sisu, output)
        .map_err(|e| report(e, output, host));
    // `SISU_NATIVE_TIMES=1`: where a build's time went, for measuring the
    // backend against 10's targets (docs/architecture.md, "Stage 2").
    if std::env::var_os("SISU_NATIVE_TIMES").is_some() {
        host.stderr(&format!(
            "native: {} functions, {} statements, code generated in {:.1} ms, linked in {:.1} ms\n",
            compiled.functions,
            compiled.statements,
            generated.as_secs_f64() * 1e3,
            start.elapsed().as_secs_f64() * 1e3
        ));
        // The release profile's facts (decision 270).
        let c = compiled.checks;
        if profile == Profile::Release {
            host.stderr(&format!(
                "native: index checks {} of {} left out, overflow checks {} of {}, tests decided {}\n",
                c.index_proven, c.index, c.overflow_proven, c.overflow, c.tests_decided
            ));
        }
    }
    linked
}

fn report(e: sisu_codegen::Error, output: &Path, host: &mut dyn Host) -> i32 {
    match e {
        sisu_codegen::Error::Unsupported { what, function } => {
            host.stderr(&text(msg::NATIVE_UNSUPPORTED, &[("what", what), ("function", function)]));
            EXIT_ERROR
        }
        sisu_codegen::Error::NoRuntime(dir) => {
            host.stderr(&text(msg::NATIVE_NO_RUNTIME, &[("dir", dir)]));
            EXIT_ERROR
        }
        sisu_codegen::Error::Link(detail) => {
            host.stderr(&text(
                msg::NATIVE_LINK_FAILED,
                &[("output", output.display().to_string()), ("detail", detail)],
            ));
            EXIT_ERROR
        }
        sisu_codegen::Error::Internal(detail) => {
            host.stderr(&text(msg::NATIVE_INTERNAL, &[("detail", detail)]));
            EXIT_ICE
        }
    }
}

/// `sisu build --native`: the executable, and a line saying where it is,
/// relative to the working directory.
pub(crate) fn build_command(code: Code<'_>, output: &Path, profile: Profile, host: &mut dyn Host) -> i32 {
    match build(code, output, profile, host) {
        Ok(()) => {
            let shown =
                std::env::current_dir().map_or_else(|_| output.to_path_buf(), |cwd| relative(output, &cwd));
            host.stdout(&text(msg::NATIVE_BUILT, &[("output", shown.display().to_string())]));
            0
        }
        Err(code) => code,
    }
}

/// `path` as seen from the directory `from`: an absolute path relative to
/// it, through `..` where they part; any other as it is.
pub(crate) fn relative(path: &Path, from: &Path) -> std::path::PathBuf {
    if !path.is_absolute() || !from.is_absolute() {
        return path.to_path_buf();
    }
    let (a, b): (Vec<_>, Vec<_>) = (path.components().collect(), from.components().collect());
    let common = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    // Only the root in common: the absolute path reads better.
    if common <= 1 {
        return path.to_path_buf();
    }
    let mut out = std::path::PathBuf::new();
    for _ in common..b.len() {
        out.push("..");
    }
    for c in &a[common..] {
        out.push(c.as_os_str());
    }
    if out.as_os_str().is_empty() { std::path::PathBuf::from(".") } else { out }
}

/// How `sisu run --native` runs the executable: the seed of its schedule
/// (`--schedule-seed`), `--explore N`, and whether it reports the
/// scheduler's numbers (`--stats`), which the runtime reads from
/// `SISU_SCHEDULE_SEED` and `SISU_STATS` (decision 203).
pub(crate) struct Run {
    pub seed: u64,
    pub explore: Option<u64>,
    pub stats: bool,
    pub profile: Profile,
    /// Where a package's executable goes and stays (decision 231); none
    /// for a file, whose executable is built aside and removed.
    pub keep: Option<PathBuf>,
}

/// `sisu run --native`: the executable, built where a package's builds go
/// or, for a file, in a directory of its own under the system's temporary
/// directory, which is removed after; run with the program's arguments and
/// this process's standard streams and environment; its exit status is
/// the run's. Under `--explore N` it runs under the seeds from
/// `--schedule-seed` on, as the interpreter's runs do, and reports the
/// first seed whose run fails.
pub(crate) fn run_command(
    code: Code<'_>,
    name: &str,
    args: &[String],
    how: &Run,
    host: &mut dyn Host,
) -> i32 {
    let aside = std::env::temp_dir().join(format!("sisu-native-{}", std::process::id()));
    let exe = match &how.keep {
        Some(exe) => exe.clone(),
        None => aside.join(if name.is_empty() { "program" } else { name }),
    };
    if let Some(dir) = exe.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let code = match build(code, &exe, how.profile, host) {
        Ok(()) => run_built(&exe, args, how, host),
        Err(code) => code,
    };
    if how.keep.is_none() {
        let _ = std::fs::remove_dir_all(&aside);
    }
    code
}

/// Runs a built executable as `sisu run --native` does: once with the seed
/// of `how`, or under `--explore N` seeds; its exit status is the run's.
pub(crate) fn run_built(exe: &Path, args: &[String], how: &Run, host: &mut dyn Host) -> i32 {
    match how.explore {
        None => run_once(exe, args, how.seed, how.stats, host),
        Some(n) => explore(exe, args, how, n, host),
    }
}

fn command(exe: &Path, args: &[String], seed: u64, stats: bool) -> std::process::Command {
    let mut cmd = std::process::Command::new(exe);
    cmd.args(args);
    if seed != 0 {
        cmd.env("SISU_SCHEDULE_SEED", seed.to_string());
    }
    if stats {
        cmd.env("SISU_STATS", "1");
    }
    cmd
}

fn cannot_run(exe: &Path, e: &std::io::Error, host: &mut dyn Host) -> i32 {
    host.stderr(&text(
        msg::NATIVE_CANNOT_RUN,
        &[("path", exe.display().to_string()), ("reason", e.to_string())],
    ));
    EXIT_ERROR
}

fn run_once(exe: &Path, args: &[String], seed: u64, stats: bool, host: &mut dyn Host) -> i32 {
    match command(exe, args, seed, stats).status() {
        Ok(st) => exit_code(st),
        Err(e) => cannot_run(exe, &e, host),
    }
}

/// `--explore N`: the runs under `N` seeds, each with its output captured;
/// the first that fails is reported with its output, and when none fails
/// the first run's output is printed with a summary (the interpreter's
/// `run_main_seeds`).
fn explore(exe: &Path, args: &[String], how: &Run, n: u64, host: &mut dyn Host) -> i32 {
    let mut first: Option<(String, String)> = None;
    for k in 0..n {
        let seed = how.seed.wrapping_add(k);
        let out = match command(exe, args, seed, how.stats).stdin(std::process::Stdio::null()).output() {
            Ok(o) => o,
            Err(e) => return cannot_run(exe, &e, host),
        };
        let status = exit_code(out.status);
        let (stdout, stderr) = (
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        );
        if status != 0 {
            host.stdout(&stdout);
            host.stderr(&stderr);
            host.stderr(&text(
                msg::CLI_EXPLORE_FAILED,
                &[("seed", seed.to_string()), ("count", n.to_string()), ("status", status.to_string())],
            ));
            return status;
        }
        match &first {
            None => first = Some((stdout, stderr)),
            // A run that prints other output than the first seed's differs
            // from it too (decision 356).
            Some((out0, _)) if *out0 != stdout => {
                host.stdout(&stdout);
                host.stderr(&stderr);
                host.stderr(&text(
                    msg::CLI_EXPLORE_DIFFERS,
                    &[
                        ("seed", seed.to_string()),
                        ("count", n.to_string()),
                        ("first", how.seed.to_string()),
                        ("status", status.to_string()),
                    ],
                ));
                return EXIT_ERROR;
            }
            Some(_) => {}
        }
    }
    if let Some((out, err)) = first {
        host.stdout(&out);
        host.stderr(&err);
    }
    let last = how.seed.wrapping_add(n - 1);
    host.stderr(&text(
        msg::CLI_EXPLORE_OK,
        &[("count", n.to_string()), ("first", how.seed.to_string()), ("last", last.to_string())],
    ));
    0
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
