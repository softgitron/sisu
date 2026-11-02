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

/// Compiles the program and links it at `output`. On failure the message
/// is reported to `host` and the exit status returned.
pub(crate) fn build(
    prog: &sisu_sir::Program,
    output: &Path,
    profile: Profile,
    host: &mut dyn Host,
) -> Result<(), i32> {
    let exe = std::env::current_exe().unwrap_or_default();
    let runtime = match sisu_codegen::find_runtime(&exe) {
        Ok(r) => r,
        Err(e) => return Err(report(e, output, host)),
    };
    // `SISU_NATIVE_SURVEY=1`: every construct the backend does not compile
    // yet, rather than the first.
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
    let start = std::time::Instant::now();
    let options = sisu_codegen::Options { sources: sources(), release: profile == Profile::Release };
    let compiled = match sisu_codegen::compile_with_options(prog, &options) {
        Ok(c) => c,
        Err(e) => return Err(report(e, output, host)),
    };
    let generated = start.elapsed();
    if let Some(dir) = output.parent()
        && !dir.as_os_str().is_empty()
    {
        let _ = std::fs::create_dir_all(dir);
    }
    let start = std::time::Instant::now();
    let linked = sisu_codegen::link(&compiled.object, &runtime, output).map_err(|e| report(e, output, host));
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

/// `sisu build --native`: the executable, and a line saying where it is.
pub(crate) fn build_command(
    prog: &sisu_sir::Program,
    output: &Path,
    profile: Profile,
    host: &mut dyn Host,
) -> i32 {
    match build(prog, output, profile, host) {
        Ok(()) => {
            host.stdout(&text(msg::NATIVE_BUILT, &[("output", output.display().to_string())]));
            0
        }
        Err(code) => code,
    }
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
    prog: &sisu_sir::Program,
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
    let code = match build(prog, &exe, how.profile, host) {
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
        if first.is_none() {
            first = Some((stdout, stderr));
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
