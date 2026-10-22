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

/// Where `sisu build --native` writes the executable: beside a file's
/// source, named by its stem (`hello.sisu` makes `hello`); for a package,
/// in its directory, named by the package.
pub(crate) fn output_for(disk: &Path, package: Option<&str>) -> PathBuf {
    match package {
        Some(name) => {
            // The root module is `src/main.sisu` in the package's directory.
            let dir = disk.parent().and_then(Path::parent).map(Path::to_path_buf).unwrap_or_default();
            dir.join(name)
        }
        None => disk.with_extension(""),
    }
}

fn text(id: msg::MsgId, params: &[(&str, String)]) -> String {
    format!("{}\n", sisu_diag::Catalog::builtin().render(id.id(), params))
}

/// Compiles the program and links it at `output`. On failure the message
/// is reported to `host` and the exit status returned.
pub(crate) fn build(prog: &sisu_sir::Program, output: &Path, host: &mut dyn Host) -> Result<(), i32> {
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
    let compiled = match sisu_codegen::compile(prog) {
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
pub(crate) fn build_command(prog: &sisu_sir::Program, output: &Path, host: &mut dyn Host) -> i32 {
    match build(prog, output, host) {
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
}

/// `sisu run --native`: the executable, built in a directory of its own
/// under the system's temporary directory, run with the program's
/// arguments and this process's standard streams and environment, and
/// removed; its exit status is the run's. Under `--explore N` it runs
/// under the seeds from `--schedule-seed` on, as the interpreter's runs do,
/// and reports the first seed whose run fails.
pub(crate) fn run_command(
    prog: &sisu_sir::Program,
    name: &str,
    args: &[String],
    how: &Run,
    host: &mut dyn Host,
) -> i32 {
    let dir = std::env::temp_dir().join(format!("sisu-native-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let exe = dir.join(if name.is_empty() { "program" } else { name });
    let code = match build(prog, &exe, host) {
        Ok(()) => match how.explore {
            None => run_once(&exe, args, how.seed, how.stats, host),
            Some(n) => explore(&exe, args, how, n, host),
        },
        Err(code) => code,
    };
    let _ = std::fs::remove_dir_all(&dir);
    code
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
