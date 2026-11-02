//! The `sisu` command line (proposal/10-tooling.md; 18, Phase 2).
//!
//! Every message the driver prints is a catalog entry (D-065). Exit
//! statuses: 0 success; 1 the program has errors, a test failed, or `main`
//! returned an error; 2 a usage error; 101 a panic; 134 an abort; 70 an
//! internal compiler error.

use crate::db::Db;
use crate::front::FrontEnd;
use sisu_diag::{Catalog, MsgId, msg};
use sisu_interp::{Host, Options, TestResult, exit};
use std::rc::Rc;

pub const EXIT_USAGE: i32 = 2;
pub const EXIT_ICE: i32 = 70;

pub(crate) fn text(id: MsgId, params: &[(&str, String)]) -> String {
    Catalog::builtin().render(id.id(), params)
}

/// `error[CODE]: message` for a catalog message without a source span.
pub(crate) fn plain_error(id: MsgId, params: &[(&str, String)]) -> String {
    let code = Catalog::builtin().code(id.id()).unwrap_or("");
    format!("error[{code}]: {}\n", text(id, params))
}

/// Delegates to a host, with the program's own arguments.
pub(crate) struct WithArgs<'a> {
    pub(crate) inner: &'a mut dyn Host,
    pub(crate) args: Vec<String>,
    /// `sisu run --watch`: the program's files, each with its name as the
    /// program names it, its path, and the hash of the text last seen.
    pub(crate) watch: Vec<(String, std::path::PathBuf, u64)>,
}

impl Host for WithArgs<'_> {
    /// The files whose text changed since the last poll. A file that
    /// cannot be read (an editor between writing and renaming it) waits
    /// for the next poll.
    fn poll_edits(&mut self) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for (name, disk, seen) in &mut self.watch {
            let Ok(text) = std::fs::read_to_string(&*disk) else { continue };
            let h = sisu_interp::record::content_hash(text.as_bytes());
            if h != *seen {
                *seen = h;
                out.push((name.clone(), text));
            }
        }
        out
    }
    fn stdout(&mut self, t: &str) {
        self.inner.stdout(t)
    }
    fn stderr(&mut self, t: &str) {
        self.inner.stderr(t)
    }
    fn args(&self) -> Vec<String> {
        self.args.clone()
    }
    fn env_var(&self, name: &str) -> Option<String> {
        self.inner.env_var(name)
    }
    fn now(&self) -> i64 {
        self.inner.now()
    }
    fn now_nanos(&self) -> i128 {
        self.inner.now_nanos()
    }
    fn virtual_time(&self) -> bool {
        self.inner.virtual_time()
    }
    fn sleep_nanos(&mut self, nanos: u64) {
        self.inner.sleep_nanos(nanos)
    }
    fn read_file(&mut self, path: &str) -> Result<String, sisu_interp::IoErrorKind> {
        self.inner.read_file(path)
    }
    fn write_file(&mut self, path: &str, t: &str) -> Result<(), sisu_interp::IoErrorKind> {
        self.inner.write_file(path, t)
    }
    fn exists(&self, path: &str) -> bool {
        self.inner.exists(path)
    }
    fn read_bytes(&mut self, path: &str) -> Result<Vec<u8>, sisu_interp::IoErrorKind> {
        self.inner.read_bytes(path)
    }
    fn write_bytes(&mut self, path: &str, data: &[u8]) -> Result<(), sisu_interp::IoErrorKind> {
        self.inner.write_bytes(path, data)
    }
    fn metadata(&self, path: &str) -> Result<(i64, i64, bool), sisu_interp::IoErrorKind> {
        self.inner.metadata(path)
    }
    fn remove_file(&mut self, path: &str) -> Result<(), sisu_interp::IoErrorKind> {
        self.inner.remove_file(path)
    }
    fn list_dir(&self, path: &str) -> Result<Vec<String>, sisu_interp::IoErrorKind> {
        self.inner.list_dir(path)
    }
    fn create_dir(&mut self, path: &str) -> Result<(), sisu_interp::IoErrorKind> {
        self.inner.create_dir(path)
    }
    fn sync_path(&mut self, path: &str) -> Result<(), sisu_interp::IoErrorKind> {
        self.inner.sync_path(path)
    }
    fn random_bits(&mut self) -> u64 {
        self.inner.random_bits()
    }
    fn tcp_connect(&mut self, host: &str, port: u16) -> Result<u64, sisu_interp::NetFail> {
        self.inner.tcp_connect(host, port)
    }
    fn tcp_listen(&mut self, host: &str, port: u16) -> Result<(u64, u16), sisu_interp::NetFail> {
        self.inner.tcp_listen(host, port)
    }
    fn tcp_accept(&mut self, l: u64) -> Result<sisu_interp::Poll<(u64, String)>, sisu_interp::NetFail> {
        self.inner.tcp_accept(l)
    }
    fn tcp_read(&mut self, c: u64, max: usize) -> Result<sisu_interp::Poll<Vec<u8>>, sisu_interp::NetFail> {
        self.inner.tcp_read(c, max)
    }
    fn tcp_write(&mut self, c: u64, data: &[u8]) -> Result<sisu_interp::Poll<usize>, sisu_interp::NetFail> {
        self.inner.tcp_write(c, data)
    }
    fn tcp_close(&mut self, h: u64) {
        self.inner.tcp_close(h)
    }
    fn read_line(&mut self) -> Option<String> {
        self.inner.read_line()
    }
    fn run_process(
        &mut self,
        program: &str,
        args: &[String],
    ) -> Result<(i64, String, String), sisu_interp::IoErrorKind> {
        self.inner.run_process(program, args)
    }
}

/// Command-line flags before the file, and the rest. A flag that takes a
/// value is written `--flag=N` or `--flag N`.
struct Parsed<'a> {
    flags: Vec<&'a str>,
    values: Vec<(&'a str, &'a str)>,
    file: Option<&'a str>,
    rest: Vec<String>,
    /// `--stats stack-depth` or `--stats shared`: which numbers `--stats`
    /// reports (a bare `--stats` reports the scheduler's).
    stats: Option<&'a str>,
}

/// What `--stats` may be followed by.
const STATS_KINDS: &[&str] = &["stack-depth", "shared"];

/// The flags that take a value.
pub(crate) const VALUE_FLAGS: &[&str] = &[
    "--schedule-seed",
    "--explore",
    "--contracts",
    "--record",
    "--redact",
    "--replay",
    "--scheduler",
    "--depth",
    "--faults",
    "--jobs",
    "--filter",
    "--tag",
    "--skip-tag",
    "--iterations",
    "--diary",
    "--output",
    "--from-sir",
    "--features",
];

/// The most schedules a bounded exhaustive search runs of one test (or of
/// `main`) unless `--explore N` says otherwise.
const EXHAUSTIVE_CAP: u64 = 10_000;

/// The fault rate of `--sim` unless `--faults` says otherwise, in parts per
/// million.
const SIM_FAULTS: u32 = 10_000;

/// The capabilities `--redact` names.
const CAPABILITIES: &[&str] = &["fs", "net", "clock", "rand", "env", "proc", "stdin"];

/// What a command was asked besides its flags.
struct Opts {
    seeds: Seeds,
    /// `--contracts` (07).
    level: sisu_lower::ContractLevel,
    /// `sisu run --record FILE`, and the capabilities left out of the log.
    record: Option<String>,
    redact: Vec<String>,
    /// `sisu test --replay TOKEN`.
    token: Option<crate::replay::Token>,
    /// `sisu replay LOG`: the log that stands in for the world.
    log: Option<sisu_interp::record::Log>,
    /// `--scheduler` and `--depth`: PCT's depth, or the preemption bound
    /// of a bounded exhaustive search.
    strategy: sisu_interp::Strategy,
    bound: u32,
    /// `sisu test --sim`, with its fault rate in parts per million.
    sim: Option<u32>,
    /// `sisu test --jobs N`: the threads that run the tests (0: the
    /// default).
    jobs: usize,
    /// `sisu test --filter TEXT --tag T --skip-tag T`: which tests run.
    select: Select,
    /// `sisu bench --iterations N`: how often each benchmark's loop runs.
    iterations: u64,
    /// What `--stats` reports.
    stats: sisu_interp::StatsKind,
    /// `sisu check --diary PATH`: the log the check appends to.
    diary: Option<String>,
    /// `sisu build --native --output PATH`: where the executable goes.
    output: Option<String>,
    /// `--native --from-sir FILE`: the input is a serialised SIR program.
    from_sir: bool,
    /// `--native --release`: the native backend's release profile (09,
    /// decision 232).
    release: bool,
    /// `--contracts` was given, which a release build's manifest does not
    /// override.
    contracts_given: bool,
}

/// What a command works on, read from disk (09): a file, or the root
/// module of a program of several modules with the rest of its files.
pub(crate) struct Input {
    /// The root module's file, as diagnostics name it.
    pub(crate) file: String,
    pub(crate) source: String,
    /// For a program of several modules: its layout and every other file's
    /// name and text.
    pub(crate) program: Option<(crate::db::ProgramSource, Vec<(String, String)>)>,
    /// The root module's file on disk: snapshots and transcripts are
    /// beside it.
    disk: std::path::PathBuf,
    /// The package's name and version and its module count, for `sisu
    /// build`.
    package: Option<(String, String)>,
    modules: usize,
    /// How a command line names it again: the file, or the package's
    /// directory relative to the working directory (`.`), as a failing
    /// test's replay line gives it.
    shown: String,
    /// Every file of the program, by its name, with its path on disk (what
    /// `sisu run --watch` watches).
    disks: Vec<(String, std::path::PathBuf)>,
    /// A package's: the directory of its lockfile, its workspace's or its
    /// own, under which its native builds go (`build/PROFILE/NAME`).
    build_root: Option<std::path::PathBuf>,
    /// The root package's `[profile.release] contracts` (07).
    release_contracts: Option<String>,
}

impl Input {
    pub(crate) fn single(file: &str, source: String) -> Input {
        Input {
            file: file.to_string(),
            source,
            program: None,
            disk: std::path::PathBuf::from(file),
            package: None,
            modules: 1,
            shown: file.to_string(),
            disks: vec![(file.to_string(), std::path::PathBuf::from(file))],
            build_root: None,
            release_contracts: None,
        }
    }

    /// The files `sisu run --watch` watches, with the hashes of the texts
    /// the program was compiled from.
    fn watched(&self) -> Vec<(String, std::path::PathBuf, u64)> {
        let text = |name: &str| -> &str {
            if name == self.file {
                return &self.source;
            }
            self.program
                .as_ref()
                .and_then(|(_, files)| files.iter().find(|(n, _)| n == name))
                .map(|(_, t)| t.as_str())
                .unwrap_or("")
        };
        self.disks
            .iter()
            .map(|(n, d)| (n.clone(), d.clone(), sisu_interp::record::content_hash(text(n).as_bytes())))
            .collect()
    }

    /// The content hash of everything the program is made of: the file's
    /// text for a single file (as replay tokens have always named it),
    /// every file's name and text for a package.
    fn hash(&self) -> u64 {
        match &self.program {
            None => sisu_interp::record::content_hash(self.source.as_bytes()),
            Some((_, files)) => {
                let mut all = format!("{}\n{}", self.file, self.source);
                for (n, t) in files {
                    all.push_str(&format!("\n{n}\n{t}"));
                }
                sisu_interp::record::content_hash(all.as_bytes())
            }
        }
    }

    /// Puts the program's files and layout in the query database.
    pub(crate) fn load(&self, db: &mut Db) {
        db.set_file(&self.file, &self.source);
        if let Some((program, files)) = &self.program {
            for (n, t) in files {
                db.set_file(n, t);
            }
            db.set_program(&self.file, program.clone());
        }
    }
}

/// Which tests a `sisu test` runs: those whose name holds one of
/// `filters`, that have one of `tags`, and that have none of `skip`
/// (17, "Grouping is one level deep; tags select and skip"). Empty lists
/// select everything.
#[derive(Clone, Debug, Default)]
struct Select {
    filters: Vec<String>,
    tags: Vec<String>,
    skip: Vec<String>,
}

impl Select {
    fn is_all(&self) -> bool {
        self.filters.is_empty() && self.tags.is_empty() && self.skip.is_empty()
    }

    fn picks(&self, name: &str, tags: &[String]) -> bool {
        (self.filters.is_empty() || self.filters.iter().any(|f| name.contains(f.as_str())))
            && (self.tags.is_empty() || self.tags.iter().any(|t| tags.contains(t)))
            && !self.skip.iter().any(|t| tags.contains(t))
    }
}

/// The flags, the values of the flags that take one, the file and what
/// follows it. With `program_after_sir` (`sisu run`), what follows `--from-sir
/// FILE` is the program's (decision 254), as what follows a source file is.
fn split_args(args: &[String], program_after_sir: bool) -> Parsed<'_> {
    let mut flags = Vec::new();
    let mut values = Vec::new();
    let mut stats = None;
    let mut i = 0;
    while i < args.len() && args[i].starts_with("--") {
        let a = args[i].as_str();
        // `--` ends the flags; what follows is the program's (`sisu run --
        // ARGS` on a package).
        if a == "--" {
            return Parsed { flags, values, file: None, rest: args[i + 1..].to_vec(), stats };
        }
        if a == "--stats" || a.starts_with("--stats=") {
            flags.push("--stats");
            if let Some(k) = a.strip_prefix("--stats=") {
                stats = Some(k);
            } else if let Some(k) = args.get(i + 1).map(String::as_str).filter(|k| STATS_KINDS.contains(k)) {
                stats = Some(k);
                i += 1;
            }
            i += 1;
            continue;
        }
        if let Some((k, v)) = a.split_once('=')
            && VALUE_FLAGS.contains(&k)
        {
            values.push((k, v));
        } else if VALUE_FLAGS.contains(&a) {
            values.push((a, args.get(i + 1).map(String::as_str).unwrap_or("")));
            i += 1;
        } else {
            flags.push(a);
        }
        i += 1;
        if program_after_sir && values.last().is_some_and(|(k, _)| *k == "--from-sir") {
            return Parsed { flags, values, file: None, rest: args.get(i..).unwrap_or(&[]).to_vec(), stats };
        }
    }
    let file = args.get(i).map(String::as_str);
    let rest = args.iter().skip(i + 1).cloned().collect();
    Parsed { flags, values, file, rest, stats }
}

/// How the scheduler is seeded: one seed, or `--explore N` seeds from the
/// first.
#[derive(Clone, Copy, Debug)]
struct Seeds {
    first: u64,
    explore: Option<u64>,
}

/// Captures a run's standard streams, for `--explore`; everything else goes
/// to the host.
pub(crate) struct Captured<'a> {
    pub(crate) inner: &'a mut dyn Host,
    pub(crate) out: String,
    pub(crate) err: String,
}

impl Host for Captured<'_> {
    fn stdout(&mut self, t: &str) {
        self.out.push_str(t)
    }
    fn stderr(&mut self, t: &str) {
        self.err.push_str(t)
    }
    fn args(&self) -> Vec<String> {
        self.inner.args()
    }
    fn env_var(&self, name: &str) -> Option<String> {
        self.inner.env_var(name)
    }
    fn now(&self) -> i64 {
        self.inner.now()
    }
    fn now_nanos(&self) -> i128 {
        self.inner.now_nanos()
    }
    fn virtual_time(&self) -> bool {
        self.inner.virtual_time()
    }
    fn sleep_nanos(&mut self, nanos: u64) {
        self.inner.sleep_nanos(nanos)
    }
    fn read_file(&mut self, path: &str) -> Result<String, sisu_interp::IoErrorKind> {
        self.inner.read_file(path)
    }
    fn write_file(&mut self, path: &str, t: &str) -> Result<(), sisu_interp::IoErrorKind> {
        self.inner.write_file(path, t)
    }
    fn exists(&self, path: &str) -> bool {
        self.inner.exists(path)
    }
    fn read_bytes(&mut self, path: &str) -> Result<Vec<u8>, sisu_interp::IoErrorKind> {
        self.inner.read_bytes(path)
    }
    fn write_bytes(&mut self, path: &str, data: &[u8]) -> Result<(), sisu_interp::IoErrorKind> {
        self.inner.write_bytes(path, data)
    }
    fn metadata(&self, path: &str) -> Result<(i64, i64, bool), sisu_interp::IoErrorKind> {
        self.inner.metadata(path)
    }
    fn remove_file(&mut self, path: &str) -> Result<(), sisu_interp::IoErrorKind> {
        self.inner.remove_file(path)
    }
    fn list_dir(&self, path: &str) -> Result<Vec<String>, sisu_interp::IoErrorKind> {
        self.inner.list_dir(path)
    }
    fn create_dir(&mut self, path: &str) -> Result<(), sisu_interp::IoErrorKind> {
        self.inner.create_dir(path)
    }
    fn sync_path(&mut self, path: &str) -> Result<(), sisu_interp::IoErrorKind> {
        self.inner.sync_path(path)
    }
    fn random_bits(&mut self) -> u64 {
        self.inner.random_bits()
    }
    fn tcp_connect(&mut self, host: &str, port: u16) -> Result<u64, sisu_interp::NetFail> {
        self.inner.tcp_connect(host, port)
    }
    fn tcp_listen(&mut self, host: &str, port: u16) -> Result<(u64, u16), sisu_interp::NetFail> {
        self.inner.tcp_listen(host, port)
    }
    fn tcp_accept(&mut self, l: u64) -> Result<sisu_interp::Poll<(u64, String)>, sisu_interp::NetFail> {
        self.inner.tcp_accept(l)
    }
    fn tcp_read(&mut self, c: u64, max: usize) -> Result<sisu_interp::Poll<Vec<u8>>, sisu_interp::NetFail> {
        self.inner.tcp_read(c, max)
    }
    fn tcp_write(&mut self, c: u64, data: &[u8]) -> Result<sisu_interp::Poll<usize>, sisu_interp::NetFail> {
        self.inner.tcp_write(c, data)
    }
    fn tcp_close(&mut self, h: u64) {
        self.inner.tcp_close(h)
    }
    fn read_line(&mut self) -> Option<String> {
        self.inner.read_line()
    }
    fn run_process(
        &mut self,
        program: &str,
        args: &[String],
    ) -> Result<(i64, String, String), sisu_interp::IoErrorKind> {
        self.inner.run_process(program, args)
    }
}

/// Runs the command line `args` (without the program name). Program
/// output and the driver's messages go to `host`. The front end is stage
/// 1's unless `--front-end=rust` or `SISU_FRONT_END=rust` asks for the Rust
/// one (decision 268).
pub fn execute(args: &[String], host: &mut dyn Host) -> i32 {
    let env = std::env::var(crate::front::FRONT_END_VAR).ok();
    match crate::front::take_choice(args, env.as_deref(), VALUE_FLAGS, host) {
        Ok((args, front)) => execute_with(&args, front, host),
        Err(code) => code,
    }
}

/// [`execute`], with the front end chosen.
fn execute_with(args: &[String], front: FrontEnd, host: &mut dyn Host) -> i32 {
    let Some(cmd) = args.first() else {
        host.stderr(&format!("{}\n", text(msg::CLI_USAGE, &[])));
        return EXIT_USAGE;
    };
    let rest = &args[1..];
    match cmd.as_str() {
        "help" | "--help" | "-h" => {
            host.stdout(&format!("{}\n", text(msg::CLI_USAGE, &[])));
            exit::OK
        }
        "explain" => explain(rest, host),
        "repl" => crate::repl::repl(rest, host),
        "lsp" => {
            if let Some(bad) = rest.first() {
                host.stderr(&plain_error(
                    msg::CLI_UNKNOWN_OPTION,
                    &[("option", bad.clone()), ("command", "lsp".into())],
                ));
                return EXIT_USAGE;
            }
            crate::lsp::serve()
        }
        "ws" => crate::repl::worksheet(rest, host),
        "new" => new_package(rest, host),
        // Stage 1's formatter and `sisu doc` take the command's arguments
        // as `sisu` does, with its messages and exit statuses.
        "fmt" | "doc" if front == FrontEnd::Stage1 => stage1_as_is(cmd, rest, host),
        "fmt" => fmt(rest, host),
        "explain-borrow" => explain_borrow(rest, host),
        "doc" => doc(rest, host),
        "replay" => replay(rest, host),
        "audit" => audit(rest, host),
        "run" | "check" | "sir" | "test" | "bench" | "build" | "tokens" | "parse" => {
            let p = split_args(rest, cmd == "run");
            let allowed: &[&str] = match cmd.as_str() {
                "run" => &[
                    "--sir",
                    "--monitor",
                    "--defer-borrow",
                    "--stats",
                    "--locked",
                    "--watch",
                    "--native",
                    "--release",
                    "--no-default-features",
                ],
                "test" => &[
                    "--no-monitor",
                    "--defer-borrow",
                    "--sim",
                    "--update",
                    "--locked",
                    "--std",
                    "--no-cache",
                    "--no-default-features",
                ],
                "check" => &["--format=ids", "--format=human", "--locked", "--no-default-features"],
                "build" => &["--locked", "--native", "--release", "--no-default-features"],
                "bench" => &["--locked", "--no-default-features"],
                "tokens" => &["--trivia"],
                "parse" => &["--tree", "--print", "--trivia", "--shape", "--ast"],
                _ => &[],
            };
            if let Some(bad) = p.flags.iter().find(|f| !allowed.contains(f)) {
                host.stderr(&plain_error(
                    msg::CLI_UNKNOWN_OPTION,
                    &[("option", bad.to_string()), ("command", cmd.clone())],
                ));
                return EXIT_USAGE;
            }
            if let Some((bad, _)) = p.values.iter().find(|(k, _)| {
                !matches!(cmd.as_str(), "run" | "test" | "bench")
                    && !(*k == "--contracts" && cmd == "sir")
                    && !(*k == "--diary" && cmd == "check")
                    && !(matches!(*k, "--output" | "--contracts")
                        && cmd == "build"
                        && p.flags.contains(&"--native"))
                    && !(*k == "--from-sir" && cmd == "build")
                    && !(*k == "--features" && matches!(cmd.as_str(), "check" | "build"))
            }) {
                host.stderr(&plain_error(
                    msg::CLI_UNKNOWN_OPTION,
                    &[("option", bad.to_string()), ("command", cmd.clone())],
                ));
                return EXIT_USAGE;
            }
            let stats = match p.stats {
                None => sisu_interp::StatsKind::Summary,
                Some("stack-depth") => sisu_interp::StatsKind::StackDepth,
                Some("shared") => sisu_interp::StatsKind::Shared,
                Some(other) => {
                    host.stderr(&plain_error(msg::CLI_BAD_STATS, &[("value", other.to_string())]));
                    return EXIT_USAGE;
                }
            };
            let mut seeds = Seeds { first: 0, explore: None };
            // `--contracts=off|preconditions|all` (07): what a run checks;
            // a test checks everything.
            let mut level = sisu_lower::ContractLevel::All;
            let mut contracts_given = false;
            let (mut record, mut redact, mut token) = (None, Vec::new(), None);
            let (mut scheduler, mut depth, mut faults) = ("random", None, None);
            let mut jobs = 0;
            let mut iterations = 100;
            let mut select = Select::default();
            let mut diary = None;
            let mut output = None;
            let mut from_sir = None;
            // `--features a,b` (09, decision 250), given any number of times.
            let mut features = crate::package::FeatureRequest {
                names: Vec::new(),
                no_default: p.flags.contains(&"--no-default-features"),
            };
            for (k, v) in &p.values {
                let only = match *k {
                    "--record" | "--redact" => Some("run"),
                    "--replay" | "--faults" | "--jobs" | "--filter" | "--tag" | "--skip-tag" => Some("test"),
                    "--iterations" => Some("bench"),
                    "--diary" => Some("check"),
                    "--output" => Some("build"),
                    _ => None,
                };
                if let Some(c) = only
                    && c != cmd
                {
                    host.stderr(&plain_error(
                        msg::CLI_UNKNOWN_OPTION,
                        &[("option", k.to_string()), ("command", cmd.clone())],
                    ));
                    return EXIT_USAGE;
                }
                match *k {
                    "--record" => {
                        record = Some(v.to_string());
                        continue;
                    }
                    "--diary" => {
                        diary = Some(v.to_string());
                        continue;
                    }
                    "--output" => {
                        output = Some(v.to_string());
                        continue;
                    }
                    "--from-sir" if matches!(cmd.as_str(), "run" | "build") => {
                        from_sir = Some(*v);
                        continue;
                    }
                    "--features" => {
                        features.names.extend(
                            v.split(|c: char| c == ',' || c.is_whitespace())
                                .filter(|f| !f.is_empty())
                                .map(str::to_string),
                        );
                        continue;
                    }
                    "--redact" => {
                        for c in v.split(',').filter(|c| !c.is_empty()) {
                            if !CAPABILITIES.contains(&c) {
                                host.stderr(&plain_error(msg::CLI_BAD_REDACT, &[("value", c.to_string())]));
                                return EXIT_USAGE;
                            }
                            redact.push(c.to_string());
                        }
                        continue;
                    }
                    "--scheduler" => {
                        if !matches!(*v, "random" | "pct" | "exhaustive") {
                            host.stderr(&plain_error(msg::CLI_BAD_SCHEDULER, &[("value", v.to_string())]));
                            return EXIT_USAGE;
                        }
                        scheduler = v;
                        continue;
                    }
                    "--filter" => {
                        select.filters.push(v.to_string());
                        continue;
                    }
                    "--tag" => {
                        select.tags.push(v.to_string());
                        continue;
                    }
                    "--skip-tag" => {
                        select.skip.push(v.to_string());
                        continue;
                    }
                    "--faults" => {
                        match v.parse::<f64>() {
                            Ok(r) if (0.0..=1.0).contains(&r) => {
                                faults = Some((r * 1_000_000.0).round() as u32)
                            }
                            _ => {
                                host.stderr(&plain_error(
                                    msg::CLI_BAD_FAULT_RATE,
                                    &[("value", v.to_string())],
                                ));
                                return EXIT_USAGE;
                            }
                        }
                        continue;
                    }
                    "--replay" => {
                        match crate::replay::Token::parse(v) {
                            Some(t) => token = Some(t),
                            None => {
                                host.stderr(&plain_error(
                                    msg::CLI_BAD_REPLAY_TOKEN,
                                    &[("token", v.to_string())],
                                ));
                                return EXIT_USAGE;
                            }
                        }
                        continue;
                    }
                    _ => {}
                }
                if *k == "--contracts" {
                    if cmd == "test" {
                        host.stderr(&plain_error(msg::CLI_TEST_CHECKS_CONTRACTS, &[]));
                        return EXIT_USAGE;
                    }
                    match sisu_lower::ContractLevel::parse(v) {
                        Some(l) => {
                            level = l;
                            contracts_given = true;
                        }
                        None => {
                            host.stderr(&plain_error(
                                msg::CLI_BAD_CONTRACT_LEVEL,
                                &[("value", v.to_string())],
                            ));
                            return EXIT_USAGE;
                        }
                    }
                    continue;
                }
                let Ok(n) = v.parse::<u64>() else {
                    host.stderr(&plain_error(
                        msg::CLI_BAD_OPTION_VALUE,
                        &[("option", k.to_string()), ("value", v.to_string())],
                    ));
                    return EXIT_USAGE;
                };
                match *k {
                    "--schedule-seed" => seeds.first = n,
                    "--depth" => depth = Some(n.min(u64::from(u32::MAX)) as u32),
                    "--jobs" => jobs = n.clamp(1, 256) as usize,
                    "--iterations" => iterations = n.clamp(1, 1_000_000_000),
                    _ => seeds.explore = Some(n.max(1)),
                }
            }
            let locked = p.flags.contains(&"--locked");
            // `--from-sir FILE` reads a serialised SIR program (decision
            // 212): FILE is the program, and what follows it is the
            // program's arguments. `sisu run` interprets it, or compiles it
            // with `--native` (decision 254); `sisu build` compiles it, so
            // only with `--native`.
            if from_sir.is_some() && cmd == "build" && !p.flags.contains(&"--native") {
                host.stderr(&plain_error(msg::NATIVE_FROM_SIR, &[]));
                return EXIT_USAGE;
            }
            // `--release` is the native backend's release profile (09,
            // decision 232); the interpreter has no profiles.
            let release = p.flags.contains(&"--release");
            if release && !p.flags.contains(&"--native") {
                host.stderr(&plain_error(msg::NATIVE_RELEASE, &[]));
                return EXIT_USAGE;
            }
            let (target, program_args) = match from_sir {
                Some(f) => (Some(f), p.file.map(str::to_string).into_iter().chain(p.rest.clone()).collect()),
                None => (p.file, p.rest.clone()),
            };
            // Stage 1 reads the program itself; the driver only finds what
            // the command works on, as it does for its own front end.
            let stage1 = front == FrontEnd::Stage1 && stage1_serves(cmd, &p);
            let mut target_of_stage1 = None;
            let inputs = if stage1 {
                match stage1_target(cmd, target, host) {
                    Ok(t) => target_of_stage1 = Some(t),
                    Err(code) => return code,
                }
                Vec::new()
            } else {
                match inputs_for(cmd, target, locked, has_ids(&p.flags), from_sir.is_some(), &features, host)
                {
                    Ok(i) => i,
                    Err(code) => return code,
                }
            };
            if cmd != "run"
                && let Some(extra) = program_args.first()
            {
                host.stderr(&plain_error(
                    msg::CLI_UNKNOWN_OPTION,
                    &[("option", extra.clone()), ("command", cmd.clone())],
                ));
                return EXIT_USAGE;
            }
            if record.is_some()
                && let Some(other) = ["--explore", "--defer-borrow", "--sir", "--from-sir"]
                    .iter()
                    .find(|o| p.flags.contains(o) || p.values.iter().any(|(k, _)| k == *o))
            {
                host.stderr(&plain_error(msg::CLI_RECORD_ONCE, &[("option", other.to_string())]));
                return EXIT_USAGE;
            }
            if p.flags.contains(&"--watch")
                && let Some(other) = ["--explore", "--defer-borrow", "--sir", "--from-sir"]
                    .iter()
                    .find(|o| p.flags.contains(o) || p.values.iter().any(|(k, _)| k == *o))
                    .map(|o| o.to_string())
                    .or_else(|| (scheduler == "exhaustive").then(|| "--scheduler exhaustive".to_string()))
            {
                host.stderr(&plain_error(msg::CLI_WATCH_ALONE, &[("option", other)]));
                return EXIT_USAGE;
            }
            // A native run takes the seed of its schedule, `--explore` and
            // the scheduler's `--stats` (decision 203); the interpreter's
            // other modes are its own.
            if p.flags.contains(&"--native")
                && let Some(other) = ["--sir", "--monitor", "--defer-borrow", "--watch"]
                    .iter()
                    .find(|o| p.flags.contains(o))
                    .map(|o| o.to_string())
                    .or_else(|| p.stats.map(|k| format!("--stats {k}")))
                    .or_else(|| {
                        ["--record", "--scheduler"]
                            .iter()
                            .find(|o| p.values.iter().any(|(k, _)| k == *o))
                            .map(|o| o.to_string())
                    })
            {
                host.stderr(&plain_error(msg::NATIVE_ALONE, &[("option", other)]));
                return EXIT_USAGE;
            }
            let sim = p.flags.contains(&"--sim");
            if faults.is_some() && !sim {
                host.stderr(&plain_error(msg::CLI_FAULTS_WITHOUT_SIM, &[]));
                return EXIT_USAGE;
            }
            let (strategy, bound) = match scheduler {
                "pct" => (sisu_interp::Strategy::Pct { depth: depth.unwrap_or(3).max(1) }, 0),
                "exhaustive" => {
                    (sisu_interp::Strategy::Exhaustive { prefix: Vec::new() }, depth.unwrap_or(2))
                }
                _ => (sisu_interp::Strategy::Random, 0),
            };
            let sim = sim.then(|| faults.unwrap_or(SIM_FAULTS));
            let opts = Opts {
                seeds,
                level,
                record,
                redact,
                token,
                log: None,
                strategy,
                bound,
                sim,
                jobs,
                select,
                iterations,
                stats,
                diary,
                output,
                from_sir: from_sir.is_some(),
                release,
                contracts_given,
            };
            if let Some(t) = &target_of_stage1 {
                return stage1_command(cmd, &p, &opts, t, program_args, &features, locked, host);
            }
            // Each member of a workspace in turn (09); the worst status wins.
            let mut status = exit::OK;
            for input in &inputs {
                let code = guarded(cmd, &p.flags, &opts, input, program_args.clone(), host);
                status = status.max(code);
            }
            status
        }
        // `#!/usr/bin/env sisu` runs a script as `sisu FILE ARGS...` (10,
        // "REPL and scripts").
        other if other.ends_with(".sisu") && std::path::Path::new(other).is_file() => {
            let mut again = vec!["run".to_string(), other.to_string()];
            again.extend(rest.iter().cloned());
            execute_with(&again, front, host)
        }
        other => {
            host.stderr(&plain_error(msg::CLI_UNKNOWN_COMMAND, &[("command", other.to_string())]));
            EXIT_USAGE
        }
    }
}

/// A serialised SIR program (`sisu sir`'s form), read and validated; a
/// text that is not one is reported.
fn read_sir(source: &str, host: &mut dyn Host) -> Result<sisu_sir::Program, i32> {
    let prog = match sisu_sir::read(source) {
        Ok(p) => p,
        Err(e) => {
            host.stderr(&plain_error(
                msg::SIR_PARSE,
                &[("detail", format!("line {}: {}", e.line, e.message))],
            ));
            return Err(exit::ERROR);
        }
    };
    if let Some(problem) = sisu_sir::validate(&prog).into_iter().next() {
        let (function, detail) =
            problem.split_once(": ").map(|(a, b)| (a.to_string(), b.to_string())).unwrap_or_default();
        host.stderr(&plain_error(msg::SIR_INVALID, &[("function", function), ("detail", detail)]));
        return Err(exit::ERROR);
    }
    Ok(prog)
}

/// `sisu build --native --from-sir FILE` and `sisu run --native --from-sir
/// FILE`: a serialised SIR program (what `sisu sir` prints, and what stage
/// 1 emits) compiled by the native backend without the front end
/// (decision 212). The executable is named by the file's stem. Without
/// `--native`, `sisu run --from-sir FILE` interprets the program, as `sisu
/// run --sir` does (decision 254).
fn from_sir(
    cmd: &str,
    flags: &[&str],
    opts: &Opts,
    input: &Input,
    args: Vec<String>,
    host: &mut dyn Host,
    stage: &std::cell::Cell<&'static str>,
) -> i32 {
    // The backend of stage 1's build, when the driver asked stage 1 for the
    // SIR rather than an executable (decision 268): the SIR, kept.
    if cmd == "build"
        && std::env::var_os(crate::front::KEEP_SIR).is_some()
        && let Some(output) = &opts.output
    {
        return crate::front::keep_sir(&input.source, std::path::Path::new(output), host);
    }
    stage.set("the SIR reader");
    let prog = match read_sir(&input.source, host) {
        Ok(p) => p,
        Err(code) => return code,
    };
    if prog.entry.is_none() {
        host.stderr(&plain_error(msg::CLI_NO_MAIN, &[("file", input.file.clone())]));
        return exit::ERROR;
    }
    if cmd == "run" && !flags.contains(&"--native") {
        stage.set("the interpreter");
        let mut h = WithArgs { inner: host, args, watch: Vec::new() };
        let options = Options {
            seed: opts.seeds.first,
            stats: flags.contains(&"--stats"),
            stats_kind: opts.stats,
            strategy: opts.strategy.clone(),
            ..Options::default()
        };
        return run_main_seeds(&prog, &mut h, &options, opts.seeds, opts.bound);
    }
    stage.set("the native backend");
    let profile = crate::native::Profile::of(opts.release);
    if cmd == "build" {
        let output = match &opts.output {
            Some(o) => std::path::PathBuf::from(o),
            None => crate::native::output_for(&input.disk, None, None, profile),
        };
        return crate::native::build_command(&prog, &output, profile, host);
    }
    let name = input.disk.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let how = crate::native::Run {
        seed: opts.seeds.first,
        explore: opts.seeds.explore,
        stats: flags.contains(&"--stats"),
        profile,
        keep: None,
    };
    crate::native::run_command(&prog, &name, &args, &how, host)
}

/// Whether stage 1 serves the command with these options (decision 268).
/// It has no SIR of a program built with its tests, so `sisu test` and
/// `sisu bench` keep the Rust front end, and so do the modes that need
/// more of the front end than its output: `run --watch` (the front end
/// again at each save), `run --record` (whose log `sisu replay` compiles
/// again), `run --defer-borrow` (the ownership checker's findings beside
/// the run), `check --diary` (the diagnostics as data) and `parse --shape`.
/// A file of SIR needs no front end at all.
fn stage1_serves(cmd: &str, p: &Parsed<'_>) -> bool {
    let value = |k: &str| p.values.iter().any(|(v, _)| *v == k);
    match cmd {
        "check" => !value("--diary"),
        "sir" | "tokens" => true,
        "build" => !value("--from-sir"),
        "run" => {
            !p.flags.iter().any(|f| matches!(*f, "--sir" | "--watch" | "--defer-borrow"))
                && !value("--from-sir")
                && !value("--record")
        }
        "parse" => !p.flags.contains(&"--shape"),
        _ => false,
    }
}

/// What a command served by stage 1 works on: the path given, which
/// stage 1 reads, or none for the package of the working directory; and
/// whether it is one file, or else the package or workspace directory.
struct Stage1Target {
    path: Option<String>,
    file: bool,
    /// The package directory, for a message about it.
    dir: Option<std::path::PathBuf>,
}

/// Finds what the command works on as [`inputs_for`] does, with its
/// messages, without reading the program: a file must be readable, a
/// directory or the working directory must have a package, and `run` in a
/// workspace runs its one application, whose directory stage 1 is given.
fn stage1_target(cmd: &str, target: Option<&str>, host: &mut dyn Host) -> Result<Stage1Target, i32> {
    let cwd = std::env::current_dir().unwrap_or_default();
    let readable = |host: &mut dyn Host, f: &str| -> Result<(), i32> {
        std::fs::read(cwd.join(f)).map(|_| ()).map_err(|e| {
            host.stderr(&plain_error(
                msg::CLI_CANNOT_READ,
                &[("path", f.to_string()), ("reason", e.to_string())],
            ));
            exit::ERROR
        })
    };
    if matches!(cmd, "tokens" | "parse") {
        let Some(f) = target else {
            host.stderr(&plain_error(msg::CLI_MISSING_FILE, &[("command", cmd.to_string())]));
            return Err(EXIT_USAGE);
        };
        readable(host, f)?;
        return Ok(Stage1Target { path: Some(f.to_string()), file: true, dir: None });
    }
    if let Some(f) = target
        && !cwd.join(f).is_dir()
    {
        readable(host, f)?;
        return Ok(Stage1Target { path: Some(f.to_string()), file: true, dir: None });
    }
    let Some(found) = crate::package::locate(target, &cwd) else {
        let dir = target.map(str::to_string).unwrap_or_else(|| cwd.display().to_string());
        host.stderr(&plain_error(msg::CLI_NO_PACKAGE, &[("dir", dir), ("command", cmd.to_string())]));
        return Err(EXIT_USAGE);
    };
    match found {
        crate::package::Target::Workspace(_, members) if cmd == "run" => {
            let apps: Vec<&std::path::PathBuf> =
                members.iter().filter(|m| m.join("src/main.sisu").is_file()).collect();
            if let [app] = apps.as_slice() {
                return Ok(Stage1Target {
                    path: Some(app.display().to_string()),
                    file: false,
                    dir: Some(app.to_path_buf()),
                });
            }
            let names: Vec<String> =
                members.iter().map(|m| format!("`{}`", crate::package::relative(&cwd, m))).collect();
            host.stderr(&plain_error(
                msg::CLI_WORKSPACE_RUN,
                &[("command", cmd.to_string()), ("members", names.join(", "))],
            ));
            Err(EXIT_USAGE)
        }
        crate::package::Target::Package(d) | crate::package::Target::Workspace(d, _) => {
            Ok(Stage1Target { path: target.map(str::to_string), file: false, dir: Some(d) })
        }
        crate::package::Target::File(f) => Ok(Stage1Target { path: Some(f), file: true, dir: None }),
    }
}

/// `sisu fmt` and `sisu doc` with stage 1: its command with the same
/// arguments, which it checks as `sisu` does.
fn stage1_as_is(cmd: &str, rest: &[String], host: &mut dyn Host) -> i32 {
    let stage1 = match crate::front::Stage1::ready(host) {
        Ok(s) => s,
        Err(code) => return code,
    };
    let mut args = vec![cmd.to_string()];
    args.extend(rest.iter().cloned());
    match stage1.run(&args, crate::front::Backend::Unused, host) {
        Ok(r) => {
            host.stdout(&r.stdout);
            host.stderr(&r.stderr);
            r.status
        }
        Err(code) => code,
    }
}

/// A command served by stage 1 (decision 268): stage 1's own command, its
/// output and status as they are; for the SIR of a program to run or
/// compile, stage 1's `build --native`, whose backend this executable is.
#[allow(clippy::too_many_arguments)]
fn stage1_command(
    cmd: &str,
    p: &Parsed<'_>,
    opts: &Opts,
    target: &Stage1Target,
    args: Vec<String>,
    features: &crate::package::FeatureRequest,
    locked: bool,
    host: &mut dyn Host,
) -> i32 {
    use crate::front::Backend;
    let has = |f: &str| p.flags.contains(&f);
    let stage1 = match crate::front::Stage1::ready(host) {
        Ok(s) => s,
        Err(code) => return code,
    };
    // The options every program command passes on: the lockfile and the
    // features (decision 250), then the path.
    let program = |words: &mut Vec<String>| {
        if locked {
            words.push("--locked".into());
        }
        if !features.names.is_empty() {
            words.push("--features".into());
            words.push(features.names.join(","));
        }
        if features.no_default {
            words.push("--no-default-features".into());
        }
        if let Some(path) = &target.path {
            words.push(path.clone());
        }
    };
    let contracts = opts.contracts_given.then(|| format!("--contracts={}", opts.level.name()));
    let shown = |host: &mut dyn Host, r: &crate::front::Ran| {
        host.stdout(&r.stdout);
        host.stderr(&r.stderr);
        r.status
    };
    let run = |host: &mut dyn Host, words: &[String], backend: Backend| stage1.run(words, backend, host);
    match cmd {
        "tokens" | "parse" => {
            let mut words = vec![cmd.to_string()];
            words.extend(p.flags.iter().map(|f| f.to_string()));
            words.extend(target.path.clone());
            match run(host, &words, Backend::Unused) {
                Ok(r) => shown(host, &r),
                Err(code) => code,
            }
        }
        "check" => {
            let mut words = vec!["check".to_string()];
            if has_ids(&p.flags) {
                words.push("--format=ids".into());
            }
            program(&mut words);
            match run(host, &words, Backend::Unused) {
                Ok(r) => shown(host, &r),
                Err(code) => code,
            }
        }
        "sir" => {
            let mut words = vec!["sir".to_string()];
            words.extend(contracts);
            program(&mut words);
            let r = match run(host, &words, Backend::Unused) {
                Ok(r) => r,
                Err(code) => return code,
            };
            if !(r.stdout.is_empty() && r.stderr.is_empty() && r.status == exit::OK) {
                return shown(host, &r);
            }
            // Stage 1's `sir` prints nothing for a program with an error of
            // the front end; its plain `build` says which, as `sisu sir` does.
            let mut words = vec!["build".to_string()];
            program(&mut words);
            match run(host, &words, Backend::Unused) {
                Ok(r) => {
                    host.stderr(&r.stderr);
                    r.status.max(exit::ERROR)
                }
                Err(code) => code,
            }
        }
        "build" => {
            let native = has("--native");
            let mut words = vec!["build".to_string()];
            if native {
                words.push("--native".into());
            }
            if opts.release {
                words.push("--release".into());
            }
            words.extend(contracts);
            if let Some(o) = &opts.output {
                words.push("--output".into());
                words.push(o.clone());
            }
            program(&mut words);
            let backend = if native { Backend::Native } else { Backend::Unused };
            match run(host, &words, backend) {
                Ok(r) => shown(host, &r),
                Err(code) => code,
            }
        }
        _ if has("--native") => {
            // `sisu run --native`: a file's executable built aside and
            // removed after, a package's where its builds go (decision 231),
            // which the backend's `built` line names.
            let aside = std::env::temp_dir().join(format!("sisu-native-{}", std::process::id()));
            let stem = |p: &str| {
                std::path::Path::new(p)
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default()
            };
            let exe = target.file.then(|| aside.join(target.path.as_deref().map(stem).unwrap_or_default()));
            let mut words = vec!["build".to_string(), "--native".to_string()];
            if opts.release {
                words.push("--release".into());
            }
            words.extend(contracts);
            program(&mut words);
            if let Some(e) = &exe {
                words.push("--output".into());
                words.push(e.display().to_string());
            }
            let r = match run(host, &words, Backend::Native) {
                Ok(r) => r,
                Err(code) => return code,
            };
            host.stderr(&r.stderr);
            let code = if r.status != exit::OK {
                r.status
            } else {
                match exe.clone().or_else(|| crate::front::built_path(&r.stdout)).filter(|e| e.is_file()) {
                    Some(e) => {
                        let how = crate::native::Run {
                            seed: opts.seeds.first,
                            explore: opts.seeds.explore,
                            stats: has("--stats"),
                            profile: crate::native::Profile::of(opts.release),
                            keep: None,
                        };
                        crate::native::run_built(&e, &args, &how, host)
                    }
                    None => no_main(target, host),
                }
            };
            let _ = std::fs::remove_dir_all(&aside);
            code
        }
        _ => {
            // `sisu run`: the SIR stage 1's build hands its backend, kept
            // rather than compiled, run by the interpreter.
            let dir = std::env::temp_dir().join(format!("sisu-sir-{}", std::process::id()));
            let _ = std::fs::create_dir_all(&dir);
            let sir_file = dir.join("program.sir");
            let mut words = vec!["build".to_string(), "--native".to_string()];
            words.extend(contracts);
            program(&mut words);
            words.push("--output".into());
            words.push(sir_file.display().to_string());
            let r = run(host, &words, Backend::KeepSir);
            let text = std::fs::read_to_string(&sir_file).ok();
            let _ = std::fs::remove_dir_all(&dir);
            let r = match r {
                Ok(r) => r,
                Err(code) => return code,
            };
            host.stderr(&r.stderr);
            if r.status != exit::OK {
                return r.status;
            }
            let Some(text) = text else { return no_main(target, host) };
            let prog = match read_sir(&text, host) {
                Ok(p) => p,
                Err(code) => return code,
            };
            if prog.entry.is_none() {
                return no_main(target, host);
            }
            let mut h = WithArgs { inner: host, args, watch: Vec::new() };
            let options = Options {
                monitor: has("--monitor"),
                seed: opts.seeds.first,
                stats: has("--stats"),
                stats_kind: opts.stats,
                strategy: opts.strategy.clone(),
                ..Options::default()
            };
            run_main_seeds(&prog, &mut h, &options, opts.seeds, opts.bound)
        }
    }
}

/// `sisu run` of a program without `main`: a file's, or a library
/// package's.
fn no_main(target: &Stage1Target, host: &mut dyn Host) -> i32 {
    let name = target.dir.as_ref().and_then(|d| {
        let text = std::fs::read_to_string(d.join(crate::package::MANIFEST)).ok()?;
        crate::manifest::read(sisu_diag::FileId(0), &text).0?.package.map(|p| p.name)
    });
    match name {
        Some(name) => host.stderr(&plain_error(msg::CLI_NO_APPLICATION, &[("name", name)])),
        None => host.stderr(&plain_error(
            msg::CLI_NO_MAIN,
            &[("file", target.path.clone().unwrap_or_else(|| ".".to_string()))],
        )),
    }
    exit::ERROR
}

/// Whether diagnostics are asked for in the conformance form.
fn has_ids(flags: &[&str]) -> bool {
    flags.contains(&"--format=ids")
}

/// Runs one command, turning a host panic (an internal compiler error,
/// which the lints are meant to make impossible) into its report.
fn guarded(
    cmd: &str,
    flags: &[&str],
    opts: &Opts,
    input: &Input,
    args: Vec<String>,
    host: &mut dyn Host,
) -> i32 {
    let stage = std::cell::Cell::new("the front end");
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        command(cmd, flags, opts, input, args, host, &stage)
    }));
    match r {
        Ok(code) => code,
        Err(payload) => {
            let detail = payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "unknown".into());
            let dump = std::env::temp_dir().join("sisu-ice.sisu");
            let _ = std::fs::write(&dump, &input.source);
            host.stderr(&plain_error(
                msg::CLI_ICE,
                &[
                    ("stage", stage.get().to_string()),
                    ("detail", detail),
                    ("dump", dump.display().to_string()),
                ],
            ));
            EXIT_ICE
        }
    }
}

/// What a command works on (09): the file it names; else the package or
/// workspace in the directory it names, or in the working directory or the
/// nearest one above it. A package's files are read and its lockfile
/// written (unless `--locked`, which refuses to change it).
fn inputs_for(
    cmd: &str,
    target: Option<&str>,
    locked: bool,
    ids: bool,
    from_sir: bool,
    features: &crate::package::FeatureRequest,
    host: &mut dyn Host,
) -> Result<Vec<Input>, i32> {
    // A name `--features` gives that is not a feature (D0112).
    let unknown = |host: &mut dyn Host, layout: &crate::package::Layout| -> Result<(), i32> {
        match layout.unknown_requested.first() {
            Some((name, package)) => {
                host.stderr(&plain_error(
                    msg::CLI_UNKNOWN_FEATURE,
                    &[("name", name.clone()), ("package", package.clone())],
                ));
                Err(EXIT_USAGE)
            }
            None => Ok(()),
        }
    };
    let cwd = std::env::current_dir().unwrap_or_default();
    let read = |host: &mut dyn Host, file: &str| -> Result<String, i32> {
        std::fs::read_to_string(file).map_err(|e| {
            host.stderr(&plain_error(
                msg::CLI_CANNOT_READ,
                &[("path", file.to_string()), ("reason", e.to_string())],
            ));
            exit::ERROR
        })
    };
    // A file of SIR is read as it is, as are the files `tokens` and `parse`
    // take.
    let file_level = matches!(cmd, "tokens" | "parse") || from_sir;
    if let Some(f) = target
        && (file_level || !cwd.join(f).is_dir())
    {
        let source = read(host, f)?;
        if file_level {
            return Ok(vec![Input::single(f, source)]);
        }
        let layout = crate::package::load_file_with(f, &source, &cwd, features);
        unknown(host, &layout)?;
        if layout.packages.len() == 1 && layout.diagnostics.is_empty() {
            return Ok(vec![Input::single(f, source)]);
        }
        let mut input = input_of(&layout, host, ids)?;
        input.disk = std::path::PathBuf::from(f);
        return Ok(vec![input]);
    }
    if file_level {
        host.stderr(&plain_error(msg::CLI_MISSING_FILE, &[("command", cmd.to_string())]));
        return Err(EXIT_USAGE);
    }
    let Some(found) = crate::package::locate(target, &cwd) else {
        let dir = target.map(str::to_string).unwrap_or_else(|| cwd.display().to_string());
        host.stderr(&plain_error(msg::CLI_NO_PACKAGE, &[("dir", dir), ("command", cmd.to_string())]));
        return Err(EXIT_USAGE);
    };
    // The workspace the package belongs to, or that the directory is: its
    // one lockfile covers every member (09).
    let found_workspace = match &found {
        crate::package::Target::Workspace(root, members) => Some((root.clone(), members.clone())),
        crate::package::Target::Package(d) => crate::package::workspace_of(d),
        crate::package::Target::File(_) => None,
    };
    let dirs = match found {
        crate::package::Target::File(f) => {
            let source = read(host, &f)?;
            unknown(host, &crate::package::load_file_with(&f, &source, &cwd, features))?;
            return Ok(vec![Input::single(&f, source)]);
        }
        crate::package::Target::Package(d) => vec![d],
        crate::package::Target::Workspace(root, members) => {
            // A member must be a package (09): its directory has a manifest.
            let manifest = root.join(crate::package::MANIFEST);
            let text = std::fs::read_to_string(&manifest).unwrap_or_default();
            let listed = crate::manifest::read(sisu_diag::FileId(0), &text)
                .0
                .and_then(|m| m.members)
                .unwrap_or_default();
            let missing: Vec<sisu_diag::Diagnostic> = listed
                .iter()
                .filter(|(p, _)| !root.join(p).join(crate::package::MANIFEST).is_file())
                .map(|(p, range)| {
                    sisu_diag::Diagnostic::new(
                        msg::PACKAGE_NO_MEMBER,
                        sisu_diag::Span::new(sisu_diag::FileId(0), *range),
                    )
                    .with("path", p.clone())
                })
                .collect();
            if !missing.is_empty() {
                let mut map = sisu_diag::SourceMap::new();
                map.add(crate::package::relative(&cwd, &manifest), text);
                if ids {
                    host.stdout(&sisu_diag::render::render_ids_with_files(&map, &missing));
                } else {
                    host.stderr(&sisu_diag::render::render_human(&map, &missing));
                }
                return Err(exit::ERROR);
            }
            if matches!(cmd, "run" | "sir" | "bench") {
                // The one member with a `main`, or the member to name.
                let apps: Vec<std::path::PathBuf> =
                    members.iter().filter(|m| m.join("src/main.sisu").is_file()).cloned().collect();
                if apps.len() != 1 {
                    let names: Vec<String> = members
                        .iter()
                        .map(|m| crate::package::relative(&cwd, m))
                        .map(|n| format!("`{n}`"))
                        .collect();
                    host.stderr(&plain_error(
                        msg::CLI_WORKSPACE_RUN,
                        &[("command", cmd.to_string()), ("members", names.join(", "))],
                    ));
                    return Err(EXIT_USAGE);
                }
                apps
            } else {
                members
            }
        }
    };
    let layouts: Vec<crate::package::Layout> =
        dirs.iter().map(|dir| crate::package::load_package_with(dir, dir, features)).collect();
    for layout in &layouts {
        unknown(host, layout)?;
    }
    // One lockfile for a workspace's members (09): what all of them resolved.
    let lock = match &found_workspace {
        Some((root, members)) => {
            let all: Vec<crate::package::Layout> =
                members.iter().map(|m| crate::package::load_package(m, m)).collect();
            let path = root.join(crate::package::LOCKFILE);
            (!all.iter().any(|l| l.has_errors())).then(|| (path, crate::package::lockfile_of(&all, root)))
        }
        None => layouts.first().and_then(|l| l.lock.clone()),
    };
    let mut out = Vec::new();
    for (k, layout) in layouts.into_iter().enumerate() {
        if k == 0
            && let Some((path, text)) = &lock
            && !layout.has_errors()
        {
            let old = std::fs::read_to_string(path).ok();
            if old.as_deref() != Some(text.as_str()) {
                if locked {
                    let what = match old {
                        None => "there is none".to_string(),
                        Some(o) => sisu_interp::diff::lines(&o, text),
                    };
                    host.stderr(&plain_error(msg::CLI_LOCKED, &[("what", what)]));
                    return Err(exit::ERROR);
                }
                if let Err(e) = std::fs::write(path, text) {
                    host.stderr(&plain_error(
                        msg::CLI_CANNOT_WRITE,
                        &[("path", path.display().to_string()), ("reason", e.to_string())],
                    ));
                    return Err(exit::ERROR);
                }
            }
        }
        if cmd == "run" && !layout.packages.first().is_none_or(|p| p.application) && !layout.has_errors() {
            let name = layout.packages.first().map(|p| p.name.clone()).unwrap_or_default();
            host.stderr(&plain_error(msg::CLI_NO_APPLICATION, &[("name", name)]));
            return Err(exit::ERROR);
        }
        out.push(input_of(&layout, host, ids)?);
    }
    Ok(out)
}

/// The input a layout describes, with its files read. A layout whose
/// manifests are broken past reading any module reports them here.
fn input_of(layout: &crate::package::Layout, host: &mut dyn Host, ids: bool) -> Result<Input, i32> {
    input_with_root(layout, host, ids, None)
}

/// [`input_of`], with the root module's text given rather than read (a
/// REPL session's program, which is on no disk).
pub(crate) fn input_with_root(
    layout: &crate::package::Layout,
    host: &mut dyn Host,
    ids: bool,
    root_text: Option<&str>,
) -> Result<Input, i32> {
    let Some(root) = layout.modules.first() else {
        let mut map = sisu_diag::SourceMap::new();
        for (n, t) in &layout.files {
            map.add(n.clone(), t.clone());
        }
        let mut diags = layout.diagnostics.clone();
        sisu_diag::diagnostic::sort(&mut diags);
        let layout = crate::package::Layout { diagnostics: diags, ..layout.clone() };
        if ids {
            host.stdout(&sisu_diag::render::render_ids_with_files(&map, &layout.diagnostics));
        } else {
            host.stderr(&sisu_diag::render::render_human(&map, &layout.diagnostics));
        }
        return Err(exit::ERROR);
    };
    let mut files = Vec::new();
    let mut source = String::new();
    for (i, m) in layout.modules.iter().enumerate() {
        if i == 0
            && let Some(t) = root_text
        {
            source = t.to_string();
            continue;
        }
        let text = std::fs::read_to_string(&m.disk).map_err(|e| {
            host.stderr(&plain_error(
                msg::CLI_CANNOT_READ,
                &[("path", m.disk.display().to_string()), ("reason", e.to_string())],
            ));
            exit::ERROR
        })?;
        if i == 0 {
            source = text;
        } else {
            files.push((m.name.clone(), text));
        }
    }
    let mut program = crate::db::ProgramSource::from_layout(layout);
    program.base = layout.base.clone();
    let package =
        layout.packages.first().filter(|p| !p.name.is_empty()).map(|p| (p.name.clone(), p.version.clone()));
    let cwd = std::env::current_dir().unwrap_or_default();
    let shown = match layout.packages.first() {
        Some(p) if !p.name.is_empty() => {
            crate::package::relative(&std::fs::canonicalize(&cwd).unwrap_or(cwd), &p.dir)
        }
        _ => root.name.clone(),
    };
    Ok(Input {
        file: root.name.clone(),
        source,
        program: Some((program, files)),
        disk: root.disk.clone(),
        package,
        modules: layout.modules.iter().filter(|m| m.package == 0).count(),
        shown,
        disks: layout.modules.iter().map(|m| (m.name.clone(), m.disk.clone())).collect(),
        build_root: layout.lock.as_ref().and_then(|(p, _)| p.parent()).map(std::path::Path::to_path_buf),
        release_contracts: layout.packages.first().and_then(|p| p.release_contracts.clone()),
    })
}

/// `sisu run --watch`: the program again from its saved files, through the
/// queries `sisu run` uses (`sisu_interp::Rebuild`). It keeps the query
/// database the run was compiled with, so a save checks, lowers and
/// borrow-checks again only the items it changed (the per-item queries).
struct Rebuilder {
    file: String,
    source: String,
    program: Option<(crate::db::ProgramSource, Vec<(String, String)>)>,
    db: Db,
}

impl Rebuilder {
    fn of(input: &Input, db: Db) -> Rebuilder {
        Rebuilder {
            file: input.file.clone(),
            source: input.source.clone(),
            program: input.program.clone(),
            db,
        }
    }
}

impl sisu_interp::Rebuild for Rebuilder {
    fn rebuild(&mut self, edits: &[(String, String)]) -> Result<sisu_sir::Program, String> {
        for (name, text) in edits {
            if *name == self.file {
                self.source = text.clone();
            } else if let Some((_, files)) = &mut self.program
                && let Some(f) = files.iter_mut().find(|(n, _)| n == name)
            {
                f.1 = text.clone();
            }
        }
        let input = Input { program: self.program.clone(), ..Input::single(&self.file, self.source.clone()) };
        input.load(&mut self.db);
        program_to_run(&mut self.db, &self.file)
    }
}

/// The SIR program `sisu run` would run, or the diagnostics that stop it,
/// rendered for a person.
fn program_to_run(db: &mut Db, file: &str) -> Result<sisu_sir::Program, String> {
    check_to_run(db, file, &[]).map_err(|e| match e {
        NotRun::Diagnostics(map, diags) => sisu_diag::render::render_human(&map, &diags),
        NotRun::Message(m) => m,
    })
}

/// Why a program was not run: its diagnostics (with the files they point
/// into), or a message of the driver.
pub(crate) enum NotRun {
    Diagnostics(sisu_diag::SourceMap, Vec<sisu_diag::Diagnostic>),
    Message(String),
}

/// The SIR program `sisu run` would run, with these functions kept too
/// (the compiler keeps only what `main` reaches), or why it would not run.
pub(crate) fn check_to_run(db: &mut Db, file: &str, keep: &[&str]) -> Result<sisu_sir::Program, NotRun> {
    let Some(c) = db.checked(file) else { return Err(NotRun::Message(String::new())) };
    if c.diagnostics.iter().any(|d| d.is_error()) {
        return Err(NotRun::Diagnostics(c.map.clone(), c.diagnostics.clone()));
    }
    let Some(lowered) = db.lowered(file) else { return Err(NotRun::Message(String::new())) };
    if !lowered.comptime.is_empty() {
        return Err(NotRun::Diagnostics(c.map.clone(), lowered.comptime.clone()));
    }
    if lowered.has_errors() {
        return Err(NotRun::Diagnostics(c.map.clone(), lowered.diagnostics.clone()));
    }
    if lowered.sir.entry.is_none() {
        return Err(NotRun::Message(plain_error(msg::CLI_NO_MAIN, &[("file", file.to_string())])));
    }
    let sir = if keep.is_empty() {
        lowered.sir.clone()
    } else {
        match db.sir_with(file, keep) {
            Some(p) => p,
            None => return Err(NotRun::Message(String::new())),
        }
    };
    if let Some(problem) = sisu_sir::validate(&sir).into_iter().next() {
        return Err(NotRun::Message(plain_error(
            msg::STAGE0_INTERNAL,
            &[("stage", "lowering".into()), ("detail", format!("invalid SIR in {problem}"))],
        )));
    }
    Ok(sir)
}

/// A command's query database, freed on a thread of its own when the
/// command is done (decision 219): a large program's memory takes a good
/// part of a second to free, which a process about to exit need not wait
/// for.
struct FreedAside(Db);

impl std::ops::Deref for FreedAside {
    type Target = Db;
    fn deref(&self) -> &Db {
        &self.0
    }
}

impl std::ops::DerefMut for FreedAside {
    fn deref_mut(&mut self) -> &mut Db {
        &mut self.0
    }
}

impl Drop for FreedAside {
    fn drop(&mut self) {
        let db = std::mem::take(&mut self.0);
        // Where no thread can be made, the database is freed here, with
        // the closure that holds it.
        let _ = std::thread::Builder::new().spawn(move || drop(db));
    }
}

#[allow(clippy::too_many_arguments)]
fn command(
    cmd: &str,
    flags: &[&str],
    opts: &Opts,
    input: &Input,
    args: Vec<String>,
    host: &mut dyn Host,
    stage: &std::cell::Cell<&'static str>,
) -> i32 {
    let file = input.file.as_str();
    let source = input.source.as_str();
    let has = |f: &str| flags.contains(&f);
    let seeds = opts.seeds;
    // A release build checks what `--contracts` or the root package's
    // `[profile.release] contracts` says, preconditions by default (07).
    let level = match (opts.release && !opts.contracts_given, input.release_contracts.as_deref()) {
        (true, Some(l)) => {
            sisu_lower::ContractLevel::parse(l).unwrap_or(sisu_lower::ContractLevel::Preconditions)
        }
        (true, None) => sisu_lower::ContractLevel::Preconditions,
        (false, _) => opts.level,
    };
    if opts.from_sir {
        return from_sir(cmd, flags, opts, input, args, host, stage);
    }
    if cmd == "run" && has("--sir") {
        stage.set("the SIR reader");
        let prog = match read_sir(source, host) {
            Ok(p) => p,
            Err(code) => return code,
        };
        stage.set("the interpreter");
        let mut h = WithArgs { inner: host, args, watch: Vec::new() };
        let options = Options {
            seed: seeds.first,
            stats: has("--stats"),
            stats_kind: opts.stats,
            strategy: opts.strategy.clone(),
            ..Options::default()
        };
        return run_main_seeds(&prog, &mut h, &options, seeds, opts.bound);
    }
    let mut db = FreedAside(Db::new());
    db.contracts = level;
    // What runs or builds the program builds it without its tests
    // (decision 243).
    db.no_tests = matches!(cmd, "run" | "build" | "sir");
    input.load(&mut db);
    // `sisu test --std`: the standard library's doc examples run too.
    if cmd == "test" && has("--std") {
        let mut program = db.program(file);
        program.std_doc_tests = true;
        db.set_program(file, program);
    }
    let multi = input.program.is_some();
    match cmd {
        "tokens" => {
            stage.set("the lexer");
            let mut map = sisu_diag::SourceMap::new();
            let id = map.add(file, source);
            let lexed = sisu_lexer::lex(id, source);
            let mut out = String::new();
            for t in &lexed.tokens {
                if t.kind.is_trivia() && !has("--trivia") {
                    continue;
                }
                let s = source.get(t.range.start as usize..t.range.end as usize).unwrap_or("");
                out.push_str(&format!(
                    "{} {}..{} {}\n",
                    t.kind.name(),
                    t.range.start,
                    t.range.end,
                    sisu_diag::render::quote(s)
                ));
            }
            host.stdout(&out);
            host.stderr(&sisu_diag::render::render_ids(&map, &lexed.diagnostics));
            exit::OK
        }
        "parse" => {
            stage.set("the parser");
            let Some(p) = db.parsed(file) else { return exit::ERROR };
            if has("--shape") {
                // What formatting keeps (`sisu fmt`'s self-check).
                return match sisu_fmt::shape(source) {
                    Some(s) => {
                        host.stdout(&s);
                        exit::OK
                    }
                    None => {
                        host.stderr(&sisu_diag::render::render_ids(&p.map, &p.parse.diagnostics));
                        exit::ERROR
                    }
                };
            }
            if has("--ast") {
                // The AST `ast::build` makes, as its `Debug` (stage 1
                // compares its own with it): positions and node indices,
                // no addresses, every list in source order.
                host.stdout(&format!("{:#?}\n", p.module));
            } else if has("--print") {
                host.stdout(&p.parse.tree.to_source());
            } else {
                host.stdout(&p.parse.tree.dump(has("--trivia")));
            }
            host.stderr(&sisu_diag::render::render_ids(&p.map, &p.parse.diagnostics));
            exit::OK
        }
        _ => {
            stage.set("the checker");
            let Some(c) = db.checked(file) else { return exit::ERROR };
            let errors = c.diagnostics.iter().filter(|d| d.is_error()).count();
            if cmd == "check" {
                stage.set("the ownership checker");
                let all = db.diagnostics(file);
                let errors = all.iter().filter(|d| d.is_error()).count();
                // `--diary PATH` (18, Phase 3): every diagnostic, and what
                // resolved the last check's, appended to the log.
                if let Some(path) = &opts.diary {
                    let files = crate::diary::own_files(&c.map, input.modules.max(1));
                    if let Err(reason) = crate::diary::append(path, host.now(), &files, &c.map, &all) {
                        host.stderr(&plain_error(
                            msg::CLI_CANNOT_WRITE,
                            &[("path", path.clone()), ("reason", reason)],
                        ));
                        return exit::ERROR;
                    }
                }
                if has("--format=ids") && multi {
                    host.stdout(&sisu_diag::render::render_ids_with_files(&c.map, &all));
                } else if has("--format=ids") {
                    host.stdout(&sisu_diag::render::render_ids(&c.map, &all));
                } else {
                    host.stderr(&sisu_diag::render::render_human(&c.map, &all));
                }
                return if errors > 0 { exit::ERROR } else { exit::OK };
            }
            let defer = has("--defer-borrow") && matches!(cmd, "run" | "test");
            // The counterexample mode defers the ownership rules about
            // declarations too; any other error stops here.
            if errors > 0 && (!defer || c.blocks_lowering()) {
                let mut out = sisu_diag::render::render_human(&c.map, &c.diagnostics);
                out.push_str(&plain_error(msg::CLI_ERRORS_FOUND, &[("count", errors.to_string())]));
                host.stderr(&out);
                return exit::ERROR;
            }
            let deferred: Vec<sisu_diag::Diagnostic> = c
                .diagnostics
                .iter()
                .filter(|d| d.is_error() && crate::db::is_ownership_rule(d))
                .cloned()
                .collect();
            let warnings: Vec<sisu_diag::Diagnostic> =
                c.diagnostics.iter().filter(|d| !d.is_error()).cloned().collect();
            if !warnings.is_empty() {
                host.stderr(&sisu_diag::render::render_human(&c.map, &warnings));
            }
            stage.set("the ownership checker");
            let Some(lowered) = db.lowered(file) else { return exit::ERROR };
            // A `comptime` expression that failed is an error of the program.
            if !lowered.comptime.is_empty() && cmd != "sir" {
                let mut out = sisu_diag::render::render_human(&c.map, &lowered.comptime);
                out.push_str(&plain_error(
                    msg::CLI_ERRORS_FOUND,
                    &[("count", lowered.comptime.len().to_string())],
                ));
                host.stderr(&out);
                return exit::ERROR;
            }
            // `sisu sir` prints the SIR of a rejected program too.
            if lowered.has_errors() && !defer && cmd != "sir" {
                let n = lowered.diagnostics.iter().filter(|d| d.is_error()).count();
                let mut out = sisu_diag::render::render_human(&c.map, &lowered.diagnostics);
                out.push_str(&plain_error(msg::CLI_OWNERSHIP_ERRORS, &[("count", n.to_string())]));
                host.stderr(&out);
                return exit::ERROR;
            }
            // The counterexample mode defers the ownership rules only:
            // purity and a second assignment to a `let` are not something
            // the monitor observes.
            let impure: Vec<sisu_diag::Diagnostic> = lowered
                .findings
                .iter()
                .zip(&lowered.diagnostics_by_finding)
                .filter(|(f, d)| d.is_error() && !f.is_ownership())
                .map(|(_, d)| d.clone())
                .collect();
            if defer && !impure.is_empty() {
                let mut out = sisu_diag::render::render_human(&c.map, &impure);
                out.push_str(&plain_error(msg::CLI_ERRORS_FOUND, &[("count", impure.len().to_string())]));
                host.stderr(&out);
                return exit::ERROR;
            }
            // A program the checker accepts runs under the monitor as usual,
            // without the counterexample account.
            let deferring = defer && (lowered.has_errors() || !deferred.is_empty());
            stage.set("lowering");
            let prog = Rc::new(lowered.sir.clone());
            if let Some(problem) = sisu_sir::validate(&prog).into_iter().next() {
                let (function, detail) =
                    problem.split_once(": ").map(|(a, b)| (a.to_string(), b.to_string())).unwrap_or_default();
                host.stderr(&plain_error(
                    msg::STAGE0_INTERNAL,
                    &[
                        ("stage", "lowering".into()),
                        ("detail", format!("invalid SIR in {function}: {detail}")),
                    ],
                ));
                return EXIT_ICE;
            }
            match cmd {
                "sir" => {
                    host.stdout(&sisu_sir::write(&prog));
                    exit::OK
                }
                // `sisu build` (09): the program checked, lowered and
                // through the ownership checker, its `comptime` values
                // computed: everything but running it. Stage 0 builds for
                // the interpreter, so there is no artefact to write.
                // A library has no `main`: there is nothing to link, and the
                // build is the plain one.
                "build" if has("--native") && prog.entry.is_some() => {
                    stage.set("the native backend");
                    let profile = crate::native::Profile::of(opts.release);
                    let output = match &opts.output {
                        Some(o) => std::path::PathBuf::from(o),
                        None => crate::native::output_for(
                            &input.disk,
                            input.package.as_ref().map(|(n, _)| n.as_str()),
                            input.build_root.as_deref(),
                            profile,
                        ),
                    };
                    crate::native::build_command(&prog, &output, profile, host)
                }
                "build" => {
                    let line = match input.package.clone() {
                        Some((name, version)) => text_line(
                            msg::CLI_BUILT,
                            &[("name", name), ("version", version), ("modules", input.modules.to_string())],
                        ),
                        None => text_line(msg::CLI_BUILT_FILE, &[("file", file.to_string())]),
                    };
                    host.stdout(&line);
                    exit::OK
                }
                "bench" => {
                    stage.set("the interpreter");
                    let options = Options { seed: seeds.first, ..Options::default() };
                    let mut failed = 0;
                    for (i, d) in c.program.benches.iter().enumerate() {
                        let name = sisu_diag::render::quote(&c.program.def(*d).name);
                        let function = format!("bench#{i}");
                        let start = host.now_nanos();
                        let r = sisu_interp::run_bench(&prog, host, &options, &function, opts.iterations);
                        let took = host.now_nanos() - start;
                        let line = match r {
                            sisu_interp::TestResult::Passed => {
                                let each = took / i128::from(opts.iterations.max(1));
                                text(
                                    msg::CLI_BENCH_RESULT,
                                    &[
                                        ("name", name),
                                        ("iterations", opts.iterations.to_string()),
                                        ("each", nanos(each)),
                                    ],
                                )
                            }
                            sisu_interp::TestResult::Failed(m) | sisu_interp::TestResult::Blocked(m) => {
                                failed += 1;
                                text(msg::CLI_BENCH_FAILED, &[("name", name), ("message", m)])
                            }
                        };
                        host.stdout(&format!("{line}\n"));
                    }
                    host.stdout(&format!(
                        "{}\n",
                        text(
                            msg::CLI_BENCH_SUMMARY,
                            &[("count", c.program.benches.len().to_string()), ("failed", failed.to_string())]
                        )
                    ));
                    if failed > 0 { exit::ERROR } else { exit::OK }
                }
                "test" => {
                    stage.set("the interpreter");
                    let options = Options {
                        monitor: defer || !has("--no-monitor"),
                        coverage: deferring,
                        seed: seeds.first,
                        strategy: opts.strategy.clone(),
                        sim: opts.sim,
                        jobs: opts.jobs,
                        // The snapshots beside the program (17).
                        snapshots: std::sync::Arc::new(
                            std::fs::read_to_string(snap_path(&input.disk))
                                .map(|t| crate::snap::parse(&t))
                                .unwrap_or_default(),
                        ),
                        update: has("--update"),
                        ..Options::default()
                    };
                    if deferring {
                        host.stderr(&format!("{}\n", text(msg::DEFER_HEADER, &[])));
                    }
                    // A `sim` build reaches nothing the simulation does not
                    // model (17).
                    if opts.sim.is_some() {
                        let escapes = crate::sim::escapes(&prog, &std::collections::HashSet::new());
                        if !escapes.is_empty() {
                            let mut out = String::new();
                            for e in &escapes {
                                let at = match e.loc {
                                    Some(l) => format!("{file}:{}:{}", l.line, l.col),
                                    None => file.to_string(),
                                };
                                out.push_str(&plain_error(
                                    msg::SIM_UNMODELLED_EXTERN,
                                    &[
                                        ("callee", e.callee.clone()),
                                        ("function", e.function.clone()),
                                        ("at", at),
                                    ],
                                ));
                            }
                            host.stderr(&out);
                            return exit::ERROR;
                        }
                    }
                    let hash = input.hash();
                    // `--replay TOKEN`: the one test, with the token's seeds
                    // and case.
                    if let Some(t) = &opts.token {
                        if t.toolchain != crate::replay::TOOLCHAIN {
                            host.stderr(&plain_error(
                                msg::CLI_OTHER_TOOLCHAIN,
                                &[
                                    ("what", "the token".into()),
                                    ("expected", t.toolchain.clone()),
                                    ("found", crate::replay::TOOLCHAIN.into()),
                                ],
                            ));
                            return exit::ERROR;
                        }
                        if t.hash != hash {
                            host.stderr(&plain_error(
                                msg::CLI_TOKEN_OTHER_CODE,
                                &[
                                    ("expected", format!("{:016x}", t.hash)),
                                    ("file", file.to_string()),
                                    ("found", format!("{hash:016x}")),
                                ],
                            ));
                            return exit::ERROR;
                        }
                        if prog.tests.get(t.test).is_none() {
                            let count = match prog.tests.len() {
                                1 => "1 test".to_string(),
                                n => format!("{n} tests"),
                            };
                            host.stderr(&plain_error(
                                msg::CLI_TOKEN_NO_TEST,
                                &[
                                    ("index", t.test.to_string()),
                                    ("file", file.to_string()),
                                    ("count", count),
                                ],
                            ));
                            return exit::ERROR;
                        }
                        let options = Options {
                            seed: t.seed,
                            case_seed: t.case_seed,
                            replay_case: t.choices.clone(),
                            strategy: t.strategy.clone(),
                            sim: t.sim,
                            ..options
                        };
                        let seeds = Seeds { first: t.seed, explore: None };
                        let run = TestRun {
                            file: &input.shown,
                            hash,
                            only: Some(vec![t.test]),
                            bound: opts.bound,
                            search: false,
                            transcripts: Vec::new(),
                            cache: None,
                        };
                        let (code, _) = run_tests(&prog, host, &options, seeds, &run);
                        return code;
                    }
                    // The tests the filters select, by their names and tags.
                    let only = (!opts.select.is_all()).then(|| {
                        (0..prog.tests.len())
                            .filter(|&k| {
                                let tags = c
                                    .program
                                    .tests
                                    .get(k)
                                    .and_then(|d| c.program.test_tags.get(d))
                                    .cloned()
                                    .unwrap_or_default();
                                opts.select.picks(&prog.tests[k].0, &tags)
                            })
                            .collect::<Vec<usize>>()
                    });
                    let which = only.clone().unwrap_or_else(|| (0..prog.tests.len()).collect());
                    // The transcripts beside the program, when every test runs.
                    let mut transcripts = Vec::new();
                    let disk = input.disk.display().to_string();
                    let found =
                        if opts.select.is_all() { crate::transcript::find(&disk) } else { Vec::new() };
                    if !found.is_empty() {
                        let sir = db.sir_with(file, &["transcript_world", "transcript_system"]);
                        for path in found {
                            let shown = path.display().to_string();
                            let outcome = match (&sir, &prog.entry) {
                                (Some(sir), Some(_)) => run_transcript(sir, host, &options, &path),
                                _ => Err(text(msg::CLI_TRANSCRIPT_NO_MAIN, &[("file", file.to_string())])),
                            };
                            transcripts.push((shown, outcome.err()));
                        }
                    }
                    // The test cache (decision 242), for a run whose results
                    // the program's code and these options decide: not one
                    // that searches schedules, simulates, updates snapshots
                    // or defers the ownership checker.
                    let cached = !has("--no-cache")
                        && seeds.explore.is_none()
                        && opts.sim.is_none()
                        && !options.update
                        && !deferring
                        && matches!(options.strategy, sisu_interp::Strategy::Random);
                    let cache = cached.then(|| {
                        let path = match &input.package {
                            Some((name, _)) => crate::native::output_for(
                                &input.disk,
                                Some(name),
                                input.build_root.as_deref(),
                                crate::native::Profile::of(false),
                            )
                            .with_file_name("test-cache"),
                            None => input.disk.with_extension("test-cache"),
                        };
                        let salt = format!(
                            "seed={} monitor={} fuel={:?} snapshots={:?}",
                            options.seed, options.monitor, options.fuel, options.snapshots
                        );
                        (path, salt)
                    });
                    let run = TestRun {
                        file: &input.shown,
                        hash,
                        only,
                        bound: opts.bound,
                        search: true,
                        transcripts,
                        cache,
                    };
                    let (code, reports) = run_tests(&prog, host, &options, seeds, &run);
                    if options.update {
                        let ran: Vec<&str> =
                            which.iter().filter_map(|&k| prog.tests.get(k)).map(|t| t.0.as_str()).collect();
                        if let Err(code) =
                            update_snapshots(&input.disk, &options.snapshots, &ran, &reports, &prog, host)
                        {
                            return code;
                        }
                    }
                    if deferring {
                        let violation = reports.iter().any(|r| r.violation);
                        let coverage = merge_coverage(&reports);
                        host.stderr(&defer_report(&lowered, &deferred, &c.map, violation, &coverage));
                    }
                    code
                }
                _ => {
                    stage.set("the interpreter");
                    if prog.entry.is_none() {
                        host.stderr(&plain_error(msg::CLI_NO_MAIN, &[("file", file.to_string())]));
                        return exit::ERROR;
                    }
                    if has("--native") {
                        stage.set("the native backend");
                        let name = input.package.as_ref().map(|(n, _)| n.clone()).unwrap_or_else(|| {
                            input
                                .disk
                                .file_stem()
                                .map(|s| s.to_string_lossy().into_owned())
                                .unwrap_or_default()
                        });
                        let profile = crate::native::Profile::of(opts.release);
                        let how = crate::native::Run {
                            seed: seeds.first,
                            explore: seeds.explore,
                            stats: has("--stats"),
                            profile,
                            // A package's executable stays where its builds
                            // go; a file's is built aside and removed.
                            keep: input.build_root.as_ref().and_then(|root| {
                                input.package.as_ref().map(|(n, _)| {
                                    crate::native::output_for(&input.disk, Some(n), Some(root), profile)
                                })
                            }),
                        };
                        return crate::native::run_command(&prog, &name, &args, &how, host);
                    }
                    let options = Options {
                        monitor: defer || has("--monitor"),
                        coverage: deferring,
                        seed: seeds.first,
                        stats: has("--stats"),
                        stats_kind: opts.stats,
                        strategy: opts.strategy.clone(),
                        ..Options::default()
                    };
                    if deferring {
                        host.stderr(&format!("{}\n", text(msg::DEFER_HEADER, &[])));
                    }
                    // `sisu replay LOG`: the log stands in for the world.
                    if let Some(log) = &opts.log {
                        let mut rep = sisu_interp::record::Replayer::new(log, host);
                        // A run under `--watch` replays its saves too.
                        let report = if log.header.watch {
                            let mut rebuild = Rebuilder::of(input, std::mem::take(&mut *db));
                            sisu_interp::run_watch((*prog).clone(), &mut rep, &options, &mut rebuild)
                        } else {
                            sisu_interp::run_main_report(&prog, &mut rep, &options)
                        };
                        let diverged = rep.divergence();
                        drop(rep);
                        if let Some(detail) = diverged {
                            host.stderr(&plain_error(msg::CLI_REPLAY_DIVERGED, &[("detail", detail)]));
                            return exit::ERROR;
                        }
                        if let Some(expected) = log.status()
                            && expected != report.status
                        {
                            host.stderr(&plain_error(
                                msg::CLI_REPLAY_STATUS,
                                &[("found", report.status.to_string()), ("expected", expected.to_string())],
                            ));
                            return exit::ERROR;
                        }
                        return report.status;
                    }
                    let mut h = WithArgs { inner: host, args, watch: Vec::new() };
                    let searching = matches!(opts.strategy, sisu_interp::Strategy::Exhaustive { .. });
                    let watch = has("--watch");
                    if watch {
                        // Hot replacement (10): the saves of the program's
                        // files are polled at its boundaries.
                        h.watch = input.watched();
                    }
                    if (seeds.explore.is_some() || searching) && !deferring {
                        return run_main_seeds(&prog, &mut h, &options, seeds, opts.bound);
                    }
                    // `--record FILE`: the run, with every host call logged.
                    if let Some(path) = &opts.record {
                        let header = sisu_interp::record::Header {
                            toolchain: crate::replay::TOOLCHAIN.into(),
                            file: file.to_string(),
                            hash: input.hash(),
                            source: source.to_string(),
                            seed: seeds.first,
                            hash_key: 0,
                            contracts: level.name().into(),
                            files: input.program.as_ref().map(|(_, f)| f.clone()).unwrap_or_default(),
                            layout: input.program.as_ref().map(|(p, _)| p.write()),
                            watch,
                        };
                        let mut rec = sisu_interp::record::Recorder::new(&mut h, opts.redact.clone());
                        let report = if watch {
                            let mut rebuild = Rebuilder::of(input, std::mem::take(&mut *db));
                            sisu_interp::run_watch((*prog).clone(), &mut rec, &options, &mut rebuild)
                        } else {
                            sisu_interp::run_main_report(&prog, &mut rec, &options)
                        };
                        let text = rec.finish(&header, report.status);
                        drop(rec);
                        if let Err(e) = std::fs::write(path, text) {
                            h.stderr(&plain_error(
                                msg::CLI_CANNOT_WRITE,
                                &[("path", path.clone()), ("reason", e.to_string())],
                            ));
                            return exit::ERROR;
                        }
                        return report.status;
                    }
                    let report = if watch {
                        let mut rebuild = Rebuilder::of(input, std::mem::take(&mut *db));
                        sisu_interp::run_watch((*prog).clone(), &mut h, &options, &mut rebuild)
                    } else {
                        sisu_interp::run_main_report(&prog, &mut h, &options)
                    };
                    if let Some(st) = &report.stats {
                        h.stderr(st);
                    }
                    if deferring {
                        host.stderr(&defer_report(
                            &lowered,
                            &deferred,
                            &c.map,
                            report.violation,
                            &report.coverage,
                        ));
                    }
                    report.status
                }
            }
        }
    }
}

type Coverage = std::collections::HashMap<(String, u32, u32), u32>;

fn merge_coverage(reports: &[sisu_interp::RunReport]) -> Coverage {
    let mut c = Coverage::new();
    for r in reports {
        for (k, n) in &r.coverage {
            *c.entry(k.clone()).or_default() += n;
        }
    }
    c
}

/// The counterexample mode's account (A2): the conflict was reached, or
/// no conflict happened and each rejected statement was reached (a
/// candidate false positive) or not (an untested path).
fn defer_report(
    lowered: &crate::db::Lowered,
    declarations: &[sisu_diag::Diagnostic],
    map: &sisu_diag::SourceMap,
    violation: bool,
    coverage: &Coverage,
) -> String {
    let mut out = String::new();
    if violation {
        out.push_str(&text(msg::DEFER_CONFLICT, &[]));
        out.push('\n');
        return out;
    }
    out.push_str(&text(msg::DEFER_NO_CONFLICT, &[]));
    out.push('\n');
    // Rules about declarations hold or not whatever the program does.
    for d in declarations {
        let params = [
            ("line", map.line_col(d.span).0.line.to_string()),
            ("code", d.code().to_string()),
            ("id", d.msg.id().to_string()),
        ];
        out.push_str("  ");
        out.push_str(&text(msg::DEFER_SIGNATURE, &params));
        out.push('\n');
    }
    let mut all_reached = true;
    for (f, d) in lowered.findings.iter().zip(&lowered.diagnostics_by_finding) {
        let line = f.loc.map(|l| l.line).unwrap_or(0).to_string();
        let count = f
            .loc
            .map(|l| coverage.get(&(f.function.clone(), l.line, l.col)).copied().unwrap_or(0))
            .unwrap_or(0);
        let params = [
            ("line", line),
            ("code", d.code().to_string()),
            ("id", d.msg.id().to_string()),
            ("count", count.to_string()),
        ];
        let m = if count > 0 { msg::DEFER_REACHED } else { msg::DEFER_NOT_REACHED };
        if count == 0 {
            all_reached = false;
        }
        out.push_str("  ");
        out.push_str(&text(m, &params));
        out.push('\n');
    }
    let verdict = if lowered.findings.is_empty() {
        msg::DEFER_DECLARATIONS_ONLY
    } else if all_reached {
        msg::DEFER_FALSE_POSITIVE
    } else {
        msg::DEFER_INCONCLUSIVE
    };
    out.push_str(&text(verdict, &[]));
    out.push('\n');
    out
}

/// Runs `main` once, or under each seed of `--explore N` with its output
/// captured: the first run that fails is reported with its output, and
/// when none fails the first run's output is printed with a summary.
fn run_main_seeds(
    prog: &sisu_sir::Program,
    host: &mut dyn Host,
    options: &Options,
    seeds: Seeds,
    bound: u32,
) -> i32 {
    if matches!(options.strategy, sisu_interp::Strategy::Exhaustive { .. }) {
        return run_main_search(prog, host, options, seeds, bound);
    }
    let Some(n) = seeds.explore else {
        let report = sisu_interp::run_main_report(prog, host, options);
        if let Some(st) = &report.stats {
            host.stderr(st);
        }
        return report.status;
    };
    let mut first: Option<(String, String)> = None;
    for k in 0..n {
        let seed = seeds.first.wrapping_add(k);
        let mut cap = Captured { inner: &mut *host, out: String::new(), err: String::new() };
        let status = sisu_interp::run_main(prog, &mut cap, &Options { seed, ..options.clone() });
        let (out, err) = (std::mem::take(&mut cap.out), std::mem::take(&mut cap.err));
        if status != exit::OK {
            host.stdout(&out);
            host.stderr(&err);
            let params =
                [("seed", seed.to_string()), ("count", n.to_string()), ("status", status.to_string())];
            let line = match &options.strategy {
                sisu_interp::Strategy::Pct { depth } => {
                    let mut p = params.to_vec();
                    p.push(("depth", depth.to_string()));
                    text(msg::CLI_PCT_FAILED, &p)
                }
                _ => text(msg::CLI_EXPLORE_FAILED, &params),
            };
            host.stderr(&format!("{line}\n"));
            return status;
        }
        if first.is_none() {
            first = Some((out, err));
        }
    }
    if let Some((out, err)) = first {
        host.stdout(&out);
        host.stderr(&err);
    }
    let last = seeds.first.wrapping_add(n - 1);
    let range = [("count", n.to_string()), ("first", seeds.first.to_string()), ("last", last.to_string())];
    let line = match &options.strategy {
        sisu_interp::Strategy::Pct { depth } => {
            let mut p = range.to_vec();
            p.push(("depth", depth.to_string()));
            text(msg::CLI_PCT_OK, &p)
        }
        _ => text(msg::CLI_EXPLORE_OK, &range),
    };
    host.stderr(&format!("{line}\n"));
    exit::OK
}

/// `sisu run --scheduler exhaustive`: `main` under every schedule with at
/// most `bound` preemptions, depth first, up to `--explore N` of them; the
/// first that fails is reported with its output, and when none fails the
/// first run's output is printed with a summary.
fn run_main_search(
    prog: &sisu_sir::Program,
    host: &mut dyn Host,
    options: &Options,
    seeds: Seeds,
    bound: u32,
) -> i32 {
    let cap = seeds.explore.unwrap_or(EXHAUSTIVE_CAP);
    let mut first: Option<(String, String)> = None;
    let mut prefix = Vec::new();
    let mut count = 0u64;
    let mut stopped = false;
    loop {
        count += 1;
        let mut captured = Captured { inner: &mut *host, out: String::new(), err: String::new() };
        let o = Options {
            strategy: sisu_interp::Strategy::Exhaustive { prefix: prefix.clone() },
            ..options.clone()
        };
        let report = sisu_interp::run_main_report(prog, &mut captured, &o);
        let (out, err) = (std::mem::take(&mut captured.out), std::mem::take(&mut captured.err));
        if report.status != exit::OK {
            host.stdout(&out);
            host.stderr(&err);
            let schedule: Vec<u32> = report.trail.iter().map(|c| c.taken).collect();
            host.stderr(&format!(
                "{}\n",
                text(
                    msg::CLI_EXHAUSTIVE_FAILED,
                    &[
                        ("schedule", dotted(&schedule)),
                        ("status", report.status.to_string()),
                        ("count", count.to_string()),
                        ("bound", bound.to_string()),
                    ]
                )
            ));
            return report.status;
        }
        if first.is_none() {
            first = Some((out, err));
        }
        match sisu_interp::next_schedule(&report.trail, bound) {
            Some(p) if count < cap => prefix = p,
            Some(_) => {
                stopped = true;
                break;
            }
            None => break,
        }
    }
    if let Some((out, err)) = first {
        host.stdout(&out);
        host.stderr(&err);
    }
    let stop =
        if stopped { text(msg::CLI_EXHAUSTIVE_STOPPED, &[("cap", cap.to_string())]) } else { String::new() };
    host.stderr(&format!(
        "{}\n",
        text(
            msg::CLI_EXHAUSTIVE_OK,
            &[("count", count.to_string()), ("bound", bound.to_string()), ("stopped", stop)]
        )
    ));
    exit::OK
}

/// Numbers joined by `.`, as a schedule is shown.
fn dotted(xs: &[u32]) -> String {
    xs.iter().map(u32::to_string).collect::<Vec<_>>().join(".")
}

/// What a failing test's replay token is made from besides its run.
struct TestRun<'a> {
    file: &'a str,
    hash: u64,
    /// The tests to run (`--replay`, `--filter`, `--tag`), by index; else
    /// every test.
    only: Option<Vec<usize>>,
    /// The preemption bound of a bounded exhaustive search, and whether
    /// to search (a replayed schedule runs alone).
    bound: u32,
    search: bool,
    /// The transcripts that ran, by path, each with why it failed (17).
    transcripts: Vec<(String, Option<String>)>,
    /// The test cache's file and the options its keys hash, when results
    /// may come from it (decision 242).
    cache: Option<(std::path::PathBuf, String)>,
}

/// A time in nanoseconds as a person reads it: `850 ns`, `12.4 µs`, `3.1 ms`.
fn nanos(n: i128) -> String {
    let n = n.max(0) as f64;
    if n < 1_000.0 {
        format!("{n} ns")
    } else if n < 1_000_000.0 {
        format!("{:.1} µs", n / 1_000.0)
    } else if n < 1_000_000_000.0 {
        format!("{:.1} ms", n / 1_000_000.0)
    } else {
        format!("{:.2} s", n / 1_000_000_000.0)
    }
}

/// Runs the transcript at `path` (17): `Err` with what differs, unless
/// `--update` takes what the program printed instead.
fn run_transcript(
    sir: &sisu_sir::Program,
    host: &mut dyn Host,
    options: &Options,
    path: &std::path::Path,
) -> Result<(), String> {
    let source = std::fs::read_to_string(path).unwrap_or_default();
    let t = crate::transcript::parse(&source);
    let runs: Vec<(Vec<String>, Vec<String>)> =
        t.runs.iter().map(|r| (r.args.clone(), r.stdin.clone())).collect();
    let o = Options { monitor: false, ..options.clone() };
    let results = sisu_interp::run_transcript(sir, host, &o, &crate::transcript::fixtures(path), &runs);
    let outputs: Vec<Vec<String>> =
        results.iter().map(|(out, status)| crate::transcript::shown(out, *status)).collect();
    let expected = crate::transcript::write(&t, &[]);
    let actual = crate::transcript::write(&t, &outputs);
    if actual == expected {
        return Ok(());
    }
    let diff = sisu_interp::diff::lines(&expected, &actual);
    if options.update {
        host.stdout(&text_line(
            msg::CLI_TRANSCRIPT_UPDATED,
            &[("name", sisu_diag::render::quote(&path.display().to_string())), ("diff", diff)],
        ));
        return std::fs::write(path, actual).map_err(|e| e.to_string());
    }
    Err(text(msg::CLI_TRANSCRIPT_DIFFERS, &[("diff", diff)]))
}

/// `PROGRAM.snap`, beside `PROGRAM.sisu`.
fn snap_path(file: &std::path::Path) -> std::path::PathBuf {
    file.with_extension("snap")
}

/// `sisu test --update`: the snapshots the tests that ran took replace
/// the ones they had, each change shown; the others stay. The file is
/// written only when something changed.
fn update_snapshots(
    file: &std::path::Path,
    old: &std::collections::BTreeMap<String, String>,
    ran: &[&str],
    reports: &[sisu_interp::RunReport],
    prog: &sisu_sir::Program,
    host: &mut dyn Host,
) -> Result<(), i32> {
    let took: Vec<(String, String)> = reports.iter().flat_map(|r| r.snapshots.iter().cloned()).collect();
    let owner = |key: &str| -> Option<String> {
        prog.tests
            .iter()
            .map(|t| t.0.clone())
            .filter(|n| key == n || key.strip_prefix(n.as_str()).is_some_and(|r| r.starts_with(" #")))
            .max_by_key(|n| n.len())
    };
    let mut changed = false;
    let mut notes = String::new();
    for (key, text) in &took {
        match old.get(key) {
            Some(t) if t == text => {}
            Some(t) => {
                changed = true;
                notes.push_str(&text_line(
                    msg::CLI_SNAPSHOT_UPDATED,
                    &[("name", key.clone()), ("diff", sisu_interp::diff::lines(t, text))],
                ));
            }
            None => {
                changed = true;
                notes.push_str(&text_line(
                    msg::CLI_SNAPSHOT_NEW,
                    &[("name", key.clone()), ("text", text.clone())],
                ));
            }
        }
    }
    // A snapshot of a test that ran and no longer takes it goes; those of
    // tests that did not run stay.
    let kept: Vec<(String, String)> = old
        .iter()
        .filter(|(k, _)| !took.iter().any(|(t, _)| t == *k))
        .filter(|(k, _)| owner(k).is_none_or(|n| !ran.contains(&n.as_str())))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    if kept.len() + took.len() != old.len() {
        changed = true;
    }
    if !changed {
        return Ok(());
    }
    // In the order of the tests, then of the snapshots within a test.
    let mut all: Vec<(String, String)> = kept.into_iter().chain(took).collect();
    let order =
        |k: &str| owner(k).and_then(|n| prog.tests.iter().position(|t| t.0 == n)).unwrap_or(usize::MAX);
    all.sort_by_key(|(k, _)| order(k));
    host.stdout(&notes);
    let path = snap_path(file);
    let name =
        std::path::Path::new(file).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    if let Err(e) = std::fs::write(&path, crate::snap::write(&name, &all)) {
        host.stderr(&plain_error(
            msg::CLI_SNAPSHOT_UNWRITABLE,
            &[("path", path.display().to_string()), ("reason", e.to_string())],
        ));
        return Err(exit::ERROR);
    }
    Ok(())
}

/// A catalog message as a line of output.
pub(crate) fn text_line(id: sisu_diag::MsgId, params: &[(&str, String)]) -> String {
    format!("{}\n", text(id, params))
}

/// The tests at `which`, those the cache holds taken from it (each prints
/// what it printed when it ran), the others run; the cache then holds the
/// tests that passed (decision 242). When every test ran (`all`), a key
/// no test has any more goes.
fn cached_tests(
    prog: &sisu_sir::Program,
    host: &mut dyn Host,
    options: &Options,
    which: &[usize],
    path: &std::path::Path,
    salt: &str,
    all: bool,
) -> Vec<(String, TestResult, sisu_interp::RunReport)> {
    let have = crate::testcache::read(path);
    let keys: Vec<Option<u64>> = which.iter().map(|&k| crate::testcache::key(prog, k, salt)).collect();
    let held = |k: &Option<u64>| k.and_then(|k| have.get(&k).map(|e| (k, e.clone())));
    let to_run: Vec<usize> =
        which.iter().zip(&keys).filter(|(_, k)| held(k).is_none()).map(|(&w, _)| w).collect();
    let mut quiet = Captured { inner: &mut *host, out: String::new(), err: String::new() };
    let mut fresh = sisu_interp::run_tests_report_at(prog, &mut quiet, options, &to_run).into_iter();
    let mut keep = if all { std::collections::BTreeMap::new() } else { have.clone() };
    let mut out = Vec::new();
    for (&w, k) in which.iter().zip(&keys) {
        let (name, result, report) = match held(k) {
            Some((key, (o, e))) => {
                keep.insert(key, (o.clone(), e.clone()));
                let report = sisu_interp::RunReport { out: o, err: e, ..Default::default() };
                (prog.tests[w].0.clone(), TestResult::Passed, report)
            }
            None => {
                let Some(r) = fresh.next() else { break };
                if let (TestResult::Passed, Some(key)) = (&r.1, k) {
                    keep.insert(*key, (r.2.out.clone(), r.2.err.clone()));
                }
                r
            }
        };
        host.stdout(&report.out);
        host.stderr(&report.err);
        out.push((name, result, report));
    }
    if keep != have {
        crate::testcache::write(path, &keep);
    }
    out
}

fn run_tests(
    prog: &sisu_sir::Program,
    host: &mut dyn Host,
    options: &Options,
    seeds: Seeds,
    run: &TestRun,
) -> (i32, Vec<sisu_interp::RunReport>) {
    let which: Vec<usize> = match &run.only {
        Some(only) => only.clone(),
        None => (0..prog.tests.len()).collect(),
    };
    let mut results = match &run.cache {
        Some((path, salt)) => cached_tests(prog, host, options, &which, path, salt, run.only.is_none()),
        None => sisu_interp::run_tests_report_at(prog, host, options, &which),
    };
    // `--explore N`: every test again under the other seeds, quietly; a
    // test fails with the first seed it fails under. Under the exhaustive
    // scheduler, every schedule within the bound instead, depth first.
    let mut failed_seed: Vec<Option<u64>> = vec![None; results.len()];
    let mut schedules: Vec<Vec<u32>> =
        results.iter().map(|(_, _, rep)| rep.trail.iter().map(|c| c.taken).collect()).collect();
    let mut searched = 0u64;
    let mut stopped = false;
    let exhaustive = matches!(options.strategy, sisu_interp::Strategy::Exhaustive { .. });
    if exhaustive && run.search {
        let cap = seeds.explore.unwrap_or(EXHAUSTIVE_CAP);
        for (i, &index) in which.iter().enumerate() {
            searched += 1;
            let mut count = 1;
            let mut trail = results[i].2.trail.clone();
            while !matches!(results[i].1, TestResult::Failed(_)) {
                let Some(prefix) = sisu_interp::next_schedule(&trail, run.bound) else { break };
                if count >= cap {
                    stopped = true;
                    break;
                }
                count += 1;
                searched += 1;
                let mut quiet = Captured { inner: &mut *host, out: String::new(), err: String::new() };
                let o = Options { strategy: sisu_interp::Strategy::Exhaustive { prefix }, ..options.clone() };
                let mut again = sisu_interp::run_tests_report_at(prog, &mut quiet, &o, &[index]);
                let Some((name, r, rep)) = again.pop() else { break };
                trail = rep.trail.clone();
                if matches!(r, TestResult::Failed(_)) {
                    schedules[i] = rep.trail.iter().map(|c| c.taken).collect();
                    results[i] = (name, r, rep);
                }
            }
        }
    } else if let Some(n) = seeds.explore
        && run.search
    {
        for k in 1..n {
            let seed = seeds.first.wrapping_add(k);
            let mut quiet = Captured { inner: &mut *host, out: String::new(), err: String::new() };
            let again = sisu_interp::run_tests_report_at(
                prog,
                &mut quiet,
                &Options { seed, ..options.clone() },
                &which,
            );
            for (i, (_, r, rep)) in again.into_iter().enumerate() {
                if failed_seed[i].is_some() || matches!(results[i].1, TestResult::Failed(_)) {
                    continue;
                }
                if let TestResult::Failed(m) = r {
                    failed_seed[i] = Some(seed);
                    results[i].1 = TestResult::Failed(m);
                    results[i].2 = rep;
                }
            }
        }
        for (i, (_, r, _)) in results.iter().enumerate() {
            if failed_seed[i].is_none() && matches!(r, TestResult::Failed(_)) {
                failed_seed[i] = Some(seeds.first);
            }
        }
    }
    let (mut passed, mut failed, mut blocked) = (0, 0, 0);
    let mut out = String::new();
    let mut failures = String::new();
    for (i, (name, r, report)) in results.iter().enumerate() {
        let outcome = match r {
            TestResult::Passed => {
                passed += 1;
                "ok".to_string()
            }
            TestResult::Failed(m) => {
                failed += 1;
                let seed = match (&options.strategy, failed_seed.get(i).copied().flatten()) {
                    (sisu_interp::Strategy::Exhaustive { .. }, _) if run.search => format!(
                        "{}\n",
                        text(
                            msg::CLI_EXHAUSTIVE_TEST_FAILED,
                            &[("schedule", dotted(&schedules[i])), ("bound", run.bound.to_string())]
                        )
                    ),
                    (sisu_interp::Strategy::Pct { depth }, Some(s)) => format!(
                        "{}\n",
                        text(
                            msg::CLI_PCT_TEST_FAILED,
                            &[("seed", s.to_string()), ("depth", depth.to_string())]
                        )
                    ),
                    (_, Some(s)) => {
                        format!("{}\n", text(msg::CLI_EXPLORE_TEST_FAILED, &[("seed", s.to_string())]))
                    }
                    (_, None) => String::new(),
                };
                // The token that runs this failure again (17).
                let index = which.get(i).copied().unwrap_or(i);
                let takes_system = prog
                    .tests
                    .get(index)
                    .and_then(|(_, f)| prog.functions.iter().find(|x| x.name == *f))
                    .is_some_and(sisu_sir::takes_real_system);
                // In the `sim` profile its `System` is simulated: it replays.
                let replay = if takes_system && options.sim.is_none() {
                    text(msg::CLI_REPLAY_NO_TOKEN, &[])
                } else {
                    let token = crate::replay::Token {
                        hash: run.hash,
                        toolchain: crate::replay::TOOLCHAIN.into(),
                        test: index,
                        seed: failed_seed.get(i).copied().flatten().unwrap_or(seeds.first),
                        case_seed: options.case_seed,
                        choices: report.case.clone(),
                        strategy: match &options.strategy {
                            sisu_interp::Strategy::Exhaustive { .. } => {
                                sisu_interp::Strategy::Exhaustive { prefix: schedules[i].clone() }
                            }
                            other => other.clone(),
                        },
                        sim: options.sim,
                    };
                    text(msg::CLI_REPLAY_TOKEN, &[("token", token.write()), ("file", run.file.to_string())])
                };
                let nl = if m.ends_with('\n') { "" } else { "\n" };
                failures.push_str(&format!(
                    "\n{}\n{seed}{m}{nl}{replay}\n",
                    text(
                        msg::CLI_TEST_RESULT,
                        &[("name", sisu_diag::render::quote(name)), ("outcome", "FAILED".into())]
                    )
                ));
                "FAILED".to_string()
            }
            TestResult::Blocked(h) => {
                blocked += 1;
                format!("blocked by todo({})", sisu_diag::render::quote(h))
            }
        };
        out.push_str(&text(
            msg::CLI_TEST_RESULT,
            &[("name", sisu_diag::render::quote(name)), ("outcome", outcome)],
        ));
        out.push('\n');
    }
    for (name, failure) in &run.transcripts {
        let name = sisu_diag::render::quote(name);
        let outcome = match failure {
            None => {
                passed += 1;
                "ok"
            }
            Some(why) => {
                failed += 1;
                let head =
                    text(msg::CLI_TRANSCRIPT_RESULT, &[("name", name.clone()), ("outcome", "FAILED".into())]);
                failures.push_str(&format!("\n{head}\n{why}\n"));
                "FAILED"
            }
        };
        out.push_str(&text_line(msg::CLI_TRANSCRIPT_RESULT, &[("name", name), ("outcome", outcome.into())]));
    }
    out.push_str(&failures);
    if !failures.is_empty() {
        out.push('\n');
    }
    out.push_str(&text(
        msg::CLI_TEST_SUMMARY,
        &[("passed", passed.to_string()), ("failed", failed.to_string()), ("blocked", blocked.to_string())],
    ));
    out.push('\n');
    // A replay runs its one test; the filters say how many they left out.
    if run.search && which.len() < prog.tests.len() {
        let count = prog.tests.len() - which.len();
        out.push_str(&text(msg::CLI_TESTS_UNSELECTED, &[("count", count.to_string())]));
        out.push('\n');
    }
    if exhaustive && run.search {
        let stop = if stopped {
            text(msg::CLI_EXHAUSTIVE_STOPPED, &[("cap", seeds.explore.unwrap_or(EXHAUSTIVE_CAP).to_string())])
        } else {
            String::new()
        };
        out.push_str(&text(
            msg::CLI_EXHAUSTIVE_SUMMARY,
            &[("count", searched.to_string()), ("bound", run.bound.to_string()), ("stopped", stop)],
        ));
        out.push('\n');
    } else if let Some(n) = seeds.explore
        && run.search
    {
        let last = seeds.first.wrapping_add(n - 1);
        let range =
            [("count", n.to_string()), ("first", seeds.first.to_string()), ("last", last.to_string())];
        match &options.strategy {
            sisu_interp::Strategy::Pct { depth } => {
                let mut params = range.to_vec();
                params.push(("depth", depth.to_string()));
                out.push_str(&text(msg::CLI_PCT_SUMMARY, &params));
            }
            _ => out.push_str(&text(msg::CLI_EXPLORE_SUMMARY, &range)),
        }
        out.push('\n');
    }
    host.stdout(&out);
    let reports = results.into_iter().map(|(_, _, r)| r).collect();
    (if failed > 0 { exit::ERROR } else { exit::OK }, reports)
}

/// The root module of the program `sisu doc --std` documents: an import of
/// every module of the standard library, so that each is loaded.
fn std_root() -> String {
    let mut text = String::new();
    for (name, _) in sisu_resolve::std_sources::DOCUMENTED {
        let path = sisu_resolve::std_sources::display_path(name);
        text.push_str(&format!("import {path} as std_{name}\n"));
    }
    text
}

/// `sisu doc [PATH] [--output DIR]` and `sisu doc --std [MODULE...]` (09,
/// "Documentation"; decision 258): every module of a program (a file, a
/// package, each member of a workspace), or of the standard library (the
/// modules named by their import paths, or all), as a Markdown page,
/// printed each after a line naming its file, or written into `DIR`.
/// The program is built as `sisu build` builds it, without its tests
/// (decision 243); one with an error is reported as `sisu check` reports
/// it, and nothing is written.
fn doc(args: &[String], host: &mut dyn Host) -> i32 {
    let mut targets: Vec<&str> = Vec::new();
    let mut output: Option<String> = None;
    let (mut std_lib, mut locked) = (false, false);
    let mut features = crate::package::FeatureRequest::default();
    let mut i = 0;
    while let Some(a) = args.get(i) {
        let value = |i: &mut usize, flag: &str| -> Option<String> {
            match a.strip_prefix(flag).and_then(|v| v.strip_prefix('=')) {
                Some(v) => Some(v.to_string()),
                None => {
                    *i += 1;
                    args.get(*i).cloned()
                }
            }
        };
        match a.as_str() {
            "--std" => std_lib = true,
            "--locked" => locked = true,
            "--no-default-features" => features.no_default = true,
            _ if a == "--output" || a.starts_with("--output=") => output = value(&mut i, "--output"),
            _ if a == "--features" || a.starts_with("--features=") => {
                let names = value(&mut i, "--features").unwrap_or_default();
                features.names.extend(
                    names
                        .split(|c: char| c == ',' || c.is_whitespace())
                        .filter(|f| !f.is_empty())
                        .map(str::to_string),
                );
            }
            _ if a.starts_with('-') => {
                host.stderr(&plain_error(
                    msg::CLI_UNKNOWN_OPTION,
                    &[("option", a.clone()), ("command", "doc".into())],
                ));
                return EXIT_USAGE;
            }
            _ => targets.push(a.as_str()),
        }
        i += 1;
    }
    // With `--std`, the modules by their import paths; else one path.
    if std_lib {
        let known = |p: &str| {
            p == "std.core"
                || sisu_resolve::std_sources::DOCUMENTED
                    .iter()
                    .any(|(n, _)| sisu_resolve::std_sources::display_path(n) == p)
        };
        if let Some(bad) = targets.iter().find(|t| !known(t)) {
            host.stderr(&plain_error(msg::DOC_STD_PATH, &[("path", bad.to_string())]));
            return EXIT_USAGE;
        }
    } else if let Some(extra) = targets.get(1) {
        host.stderr(&plain_error(
            msg::CLI_UNKNOWN_OPTION,
            &[("option", extra.to_string()), ("command", "doc".into())],
        ));
        return EXIT_USAGE;
    }
    let inputs = if std_lib {
        vec![Input::single("std.sisu", std_root())]
    } else {
        match inputs_for("doc", targets.first().copied(), locked, false, false, &features, host) {
            Ok(i) => i,
            Err(code) => return code,
        }
    };
    let only: Option<&[&str]> = (std_lib && !targets.is_empty()).then_some(targets.as_slice());
    let mut pages = Vec::new();
    for input in &inputs {
        let r =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| doc_pages(input, only, std_lib, host)));
        match r {
            Ok(Ok(p)) => pages.extend(p),
            Ok(Err(code)) => return code,
            Err(payload) => {
                let detail = payload
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_else(|| "unknown".into());
                let dump = std::env::temp_dir().join("sisu-ice.sisu");
                let _ = std::fs::write(&dump, &input.source);
                host.stderr(&plain_error(
                    msg::CLI_ICE,
                    &[("stage", "sisu doc".into()), ("detail", detail), ("dump", dump.display().to_string())],
                ));
                return EXIT_ICE;
            }
        }
    }
    let Some(dir) = output else {
        host.stdout(&crate::doc::printed(&pages));
        return exit::OK;
    };
    let dir = std::path::PathBuf::from(dir);
    for page in &pages {
        let path = dir.join(&page.file);
        if let Err(e) = std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&path, &page.text)) {
            host.stderr(&plain_error(
                msg::CLI_CANNOT_WRITE,
                &[("path", path.display().to_string()), ("reason", e.to_string())],
            ));
            return exit::ERROR;
        }
    }
    exit::OK
}

/// The pages of one program: the root package's modules, or with `std_lib`
/// the standard library's, those `only` names if it names some (decision
/// 258).
fn doc_pages(
    input: &Input,
    only: Option<&[&str]>,
    std_lib: bool,
    host: &mut dyn Host,
) -> Result<Vec<crate::doc::Page>, i32> {
    use sisu_resolve::std_sources;
    let mut db = Db::new();
    db.no_tests = true;
    input.load(&mut db);
    let Some(c) = db.checked(&input.file) else { return Err(exit::ERROR) };
    let errors = c.diagnostics.iter().filter(|d| d.is_error()).count();
    if errors > 0 {
        let mut out = sisu_diag::render::render_human(&c.map, &c.diagnostics);
        out.push_str(&plain_error(msg::DOC_ERRORS_FOUND, &[("count", errors.to_string())]));
        host.stderr(&out);
        return Err(exit::ERROR);
    }
    let prog = &c.program;
    // Each module with a page: its id, path, text and syntax as the build
    // has it.
    let mut found: Vec<(sisu_resolve::ModuleId, String, String, sisu_syntax::ast::Module)> = Vec::new();
    if std_lib {
        let mut modules: Vec<(&str, &str)> = vec![("prelude", std_sources::PRELUDE)];
        modules.extend(std_sources::DOCUMENTED.iter().copied());
        for (name, text) in modules {
            let id = if name == "prelude" {
                Some(prog.prelude)
            } else {
                prog.modules
                    .iter()
                    .position(|m| m.is_std && m.name == name)
                    .map(|k| sisu_resolve::ModuleId(k as u32))
            };
            let Some(id) = id else { continue };
            let path = std_sources::display_path(name);
            if only.is_some_and(|o| !o.contains(&path.as_str())) {
                continue;
            }
            let parse = sisu_parser::parse(sisu_diag::FileId(0), text);
            let module = sisu_syntax::ast::build::module(&parse.tree);
            found.push((id, path, text.to_string(), module));
        }
    } else {
        let program = db.program(&input.file);
        for (k, (path, file, package)) in program.modules.iter().enumerate() {
            if *package != 0 {
                continue;
            }
            let (Some(&id), Some(text), Some(parsed)) =
                (prog.user_modules.get(k), db.text(file), db.parsed(file))
            else {
                continue;
            };
            let module = match crate::cfg::filter(&parsed.module, false, program.features.get(*package)).0 {
                Some(m) => m,
                None => parsed.module.clone(),
            };
            let shown = if path.is_empty() {
                std::path::Path::new(file)
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default()
            } else {
                path.join(".")
            };
            found.push((id, shown, text.to_string(), module));
        }
    }
    found.sort_by(|a, b| a.1.cmp(&b.1));
    let tokens: Vec<Vec<sisu_syntax::tree::Token>> =
        found.iter().map(|(_, _, text, _)| sisu_lexer::lex(sisu_diag::FileId(0), text).tokens).collect();
    let sources: Vec<crate::doc::Source<'_>> = found
        .iter()
        .zip(&tokens)
        .map(|((id, path, text, module), tokens)| crate::doc::Source {
            id: *id,
            path: path.clone(),
            text,
            tokens,
            module,
        })
        .collect();
    let public_only = !std_lib && prog.packages.first().is_some_and(|p| !p.name.is_empty());
    let docs = crate::doc::Docs { sources: &sources, prog, inferred: &c.typed.inferred, public_only };
    Ok(crate::doc::pages(&docs))
}

/// `sisu explain-borrow FILE:LINE`: every borrow live on the line, with
/// where it starts, who holds it and where it is last used (10, "Language
/// server": the "explain this borrow" command).
fn explain_borrow(args: &[String], host: &mut dyn Host) -> i32 {
    let target = args.first().and_then(|a| {
        let (file, line) = a.rsplit_once(':')?;
        Some((file.to_string(), line.parse::<u32>().ok()?))
    });
    let Some((file, line)) = target else {
        host.stderr(&plain_error(msg::CLI_EXPLAIN_BORROW_USAGE, &[]));
        return EXIT_USAGE;
    };
    let source = match std::fs::read_to_string(&file) {
        Ok(s) => s,
        Err(e) => {
            host.stderr(&plain_error(
                msg::CLI_CANNOT_READ,
                &[("path", file.clone()), ("reason", e.to_string())],
            ));
            return exit::ERROR;
        }
    };
    let mut db = Db::new();
    db.set_file(&file, &source);
    let Some(c) = db.checked(&file) else { return exit::ERROR };
    if c.has_errors() {
        let errors = c.diagnostics.iter().filter(|d| d.is_error()).count();
        let mut out = sisu_diag::render::render_human(&c.map, &c.diagnostics);
        out.push_str(&plain_error(msg::CLI_ERRORS_FOUND, &[("count", errors.to_string())]));
        host.stderr(&out);
        return exit::ERROR;
    }
    let Some(lowered) = db.lowered(&file) else { return exit::ERROR };
    let files = crate::db::source_files(&c.map, &lowered.sir);
    let root = files.first().copied().unwrap_or_default();
    let mut out = String::new();
    let mut any = false;
    for fl in sisu_borrowck::explain::live_at(&lowered.sir, 0, line) {
        if fl.loans.is_empty() {
            continue;
        }
        any = true;
        out.push_str(&text(
            msg::CLI_EXPLAIN_BORROW_HEADER,
            &[("line", line.to_string()), ("file", file.clone()), ("function", fl.function.clone())],
        ));
        out.push('\n');
        for l in &fl.loans {
            let kind = text(
                if l.mutable { msg::CLI_EXPLAIN_BORROW_MUT } else { msg::CLI_EXPLAIN_BORROW_SHARED },
                &[],
            );
            let pos =
                |loc: Option<sisu_sir::Loc>| loc.map(|l| format!("{}:{}", l.line, l.col)).unwrap_or_default();
            let place = match &l.place {
                sisu_borrowck::Arg::Text(t) => t.clone(),
                sisu_borrowck::Arg::Source(loc) => {
                    c.map.slice(sisu_borrowck::span_of(&c.map, root, Some(*loc))).to_string()
                }
            };
            let holders: Vec<String> = l.holders.iter().map(|h| format!("`{h}`")).collect();
            let m = if holders.is_empty() {
                msg::CLI_EXPLAIN_BORROW_TEMPORARY
            } else {
                msg::CLI_EXPLAIN_BORROW_LOAN
            };
            out.push_str("  ");
            out.push_str(&text(
                m,
                &[
                    ("kind", kind),
                    ("place", place),
                    ("start", pos(l.start)),
                    ("holders", holders.join(", ")),
                    ("end", pos(l.until)),
                ],
            ));
            out.push('\n');
        }
    }
    if !any {
        out.push_str(&text(
            msg::CLI_EXPLAIN_BORROW_NONE,
            &[("line", line.to_string()), ("file", file.clone())],
        ));
        out.push('\n');
    }
    host.stdout(&out);
    exit::OK
}

/// `sisu audit --ownership FILE...`: per file, the uses of the ownership
/// model's escape hatches ([`crate::audit`]), one line of data per file.
/// `sisu audit --caps FILE...`: per program, its capability parameters,
/// uses and bundles ([`crate::audit::caps`]).
fn audit(args: &[String], host: &mut dyn Host) -> i32 {
    let (flags, files): (Vec<&String>, Vec<&String>) = args.iter().partition(|a| a.starts_with("--"));
    if let Some(bad) = flags.iter().find(|f| !matches!(f.as_str(), "--ownership" | "--caps")) {
        host.stderr(&plain_error(
            msg::CLI_UNKNOWN_OPTION,
            &[("option", bad.to_string()), ("command", "audit".into())],
        ));
        return EXIT_USAGE;
    }
    if flags.len() != 1 {
        host.stderr(&plain_error(msg::CLI_AUDIT_USAGE, &[]));
        return EXIT_USAGE;
    }
    if flags[0] == "--caps" {
        return audit_caps(&files, host);
    }
    if files.is_empty() {
        host.stderr(&plain_error(msg::CLI_MISSING_FILE, &[("command", "audit".into())]));
        return EXIT_USAGE;
    }
    let mut out = String::new();
    for file in files {
        let source = match std::fs::read_to_string(file) {
            Ok(s) => s,
            Err(e) => {
                host.stdout(&out);
                host.stderr(&plain_error(
                    msg::CLI_CANNOT_READ,
                    &[("path", file.clone()), ("reason", e.to_string())],
                ));
                return exit::ERROR;
            }
        };
        let mut db = Db::new();
        db.set_file(file, &source);
        let Some(p) = db.parsed(file) else { return exit::ERROR };
        let lexed = sisu_lexer::lex(p.file, &source);
        out.push_str(&crate::audit::ownership(&source, &lexed.tokens, &p.module).line(file));
        out.push('\n');
    }
    host.stdout(&out);
    exit::OK
}

/// `sisu audit --caps FILE...`: each program checked and lowered, then
/// its capability audit, read from its SIR.
fn audit_caps(files: &[&String], host: &mut dyn Host) -> i32 {
    if files.is_empty() {
        host.stderr(&plain_error(msg::CLI_MISSING_FILE, &[("command", "audit".into())]));
        return EXIT_USAGE;
    }
    let mut out = String::new();
    for file in files {
        let source = match std::fs::read_to_string(file) {
            Ok(s) => s,
            Err(e) => {
                host.stdout(&out);
                host.stderr(&plain_error(
                    msg::CLI_CANNOT_READ,
                    &[("path", file.to_string()), ("reason", e.to_string())],
                ));
                return exit::ERROR;
            }
        };
        let mut db = Db::new();
        db.set_file(file, &source);
        let Some(c) = db.checked(file) else { return exit::ERROR };
        let lowered = if c.has_errors() { None } else { db.lowered(file) };
        let Some(lowered) = lowered.filter(|l| !l.has_errors()) else {
            let mut all: Vec<sisu_diag::Diagnostic> = c.diagnostics.clone();
            if !c.has_errors()
                && let Some(l) = db.lowered(file)
            {
                all.extend(l.diagnostics.iter().cloned());
            }
            all.retain(|d| d.is_error());
            let mut text = sisu_diag::render::render_human(&c.map, &all);
            text.push_str(&plain_error(msg::CLI_ERRORS_FOUND, &[("count", all.len().to_string())]));
            host.stdout(&out);
            host.stderr(&text);
            return exit::ERROR;
        };
        out.push_str(&crate::audit::caps(file, &lowered.sir, &crate::audit::own_structs(&c.program)));
    }
    host.stdout(&out);
    exit::OK
}

/// `sisu replay LOG`: the run a capability log recorded, again, with the
/// log standing in for the world (10, "Debugging and profiling"). The
/// program is the log's own copy of the source, checked at the level it
/// was recorded at; `--monitor` runs it under the ownership monitor.
fn replay(args: &[String], host: &mut dyn Host) -> i32 {
    let p = split_args(args, false);
    if let Some(bad) = p.flags.iter().find(|f| **f != "--monitor") {
        host.stderr(&plain_error(
            msg::CLI_UNKNOWN_OPTION,
            &[("option", bad.to_string()), ("command", "replay".into())],
        ));
        return EXIT_USAGE;
    }
    if let Some((bad, _)) = p.values.first() {
        host.stderr(&plain_error(
            msg::CLI_UNKNOWN_OPTION,
            &[("option", bad.to_string()), ("command", "replay".into())],
        ));
        return EXIT_USAGE;
    }
    let Some(path) = p.file else {
        host.stderr(&plain_error(msg::CLI_MISSING_FILE, &[("command", "replay".into())]));
        return EXIT_USAGE;
    };
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            host.stderr(&plain_error(
                msg::CLI_CANNOT_READ,
                &[("path", path.to_string()), ("reason", e.to_string())],
            ));
            return exit::ERROR;
        }
    };
    let bad_log = |host: &mut dyn Host, reason: String| {
        host.stderr(&plain_error(msg::CLI_BAD_LOG, &[("path", path.to_string()), ("reason", reason)]));
        exit::ERROR
    };
    let log = match sisu_interp::record::Log::parse(&text) {
        Ok(l) => l,
        Err(e) => return bad_log(host, e),
    };
    if log.header.toolchain != crate::replay::TOOLCHAIN {
        host.stderr(&plain_error(
            msg::CLI_OTHER_TOOLCHAIN,
            &[
                ("what", format!("`{path}`")),
                ("expected", log.header.toolchain.clone()),
                ("found", crate::replay::TOOLCHAIN.into()),
            ],
        ));
        return exit::ERROR;
    }
    // The hash covers every file of a program of several modules, as a
    // replay token's does.
    let hashed = match log.header.layout.as_deref().and_then(crate::db::ProgramSource::read) {
        Some(program) => Input {
            program: Some((program, log.header.files.clone())),
            ..Input::single(&log.header.file, log.header.source.clone())
        }
        .hash(),
        None => sisu_interp::record::content_hash(log.header.source.as_bytes()),
    };
    if hashed != log.header.hash {
        return bad_log(host, "its source does not match its content hash".into());
    }
    let Some(level) = sisu_lower::ContractLevel::parse(&log.header.contracts) else {
        return bad_log(host, format!("`{}` is not a level of contract checking", log.header.contracts));
    };
    let file = log.header.file.clone();
    let source = log.header.source.clone();
    let opts = Opts {
        seeds: Seeds { first: log.header.seed, explore: None },
        level,
        record: None,
        redact: Vec::new(),
        token: None,
        log: Some(log),
        strategy: sisu_interp::Strategy::Random,
        bound: 0,
        sim: None,
        jobs: 0,
        select: Select::default(),
        iterations: 0,
        stats: sisu_interp::StatsKind::Summary,
        diary: None,
        output: None,
        from_sir: false,
        release: false,
        contracts_given: true,
    };
    let stage = std::cell::Cell::new("the front end");
    let flags: Vec<&str> = p.flags.clone();
    // A program of several modules replays from the files and layout the
    // log holds.
    let input = match opts
        .log
        .as_ref()
        .and_then(|l| l.header.layout.as_deref())
        .and_then(crate::db::ProgramSource::read)
    {
        Some(program) => {
            let files = opts.log.as_ref().map(|l| l.header.files.clone()).unwrap_or_default();
            Input {
                file: file.clone(),
                source: source.clone(),
                program: Some((program, files)),
                disk: std::path::PathBuf::from(&file),
                package: None,
                modules: 0,
                shown: ".".into(),
                disks: Vec::new(),
                build_root: None,
                release_contracts: None,
            }
        }
        None => Input::single(&file, source.clone()),
    };
    command("run", &flags, &opts, &input, Vec::new(), host, &stage)
}

/// Every `.sisu` file under `dir`, sorted, leaving out hidden directories,
/// `target` and a case's `.files` and `.pkg` inputs only when asked.
fn sisu_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut paths: Vec<std::path::PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    paths.sort();
    for p in paths {
        let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if name.starts_with('.') || name == "target" {
            continue;
        }
        if p.is_dir() {
            sisu_files(&p, out);
        } else if name.ends_with(".sisu") {
            out.push(p);
        }
    }
}

/// `sisu fmt [--check] [PATH...]` (10, "Formatter"): formats each file, and
/// each `.sisu` file under each directory, in place; without a path, the
/// package's (the files under the directory of its manifest). There are
/// no options of style. `--check` writes nothing and reports each file
/// that is not formatted, with what would change. A file with a syntax
/// error is reported and left as it is.
fn fmt(args: &[String], host: &mut dyn Host) -> i32 {
    let (flags, paths): (Vec<&String>, Vec<&String>) = args.iter().partition(|a| a.starts_with("--"));
    if let Some(bad) = flags.iter().find(|f| f.as_str() != "--check") {
        host.stderr(&plain_error(
            msg::CLI_UNKNOWN_OPTION,
            &[("option", bad.to_string()), ("command", "fmt".into())],
        ));
        return EXIT_USAGE;
    }
    let check = !flags.is_empty();
    let cwd = std::env::current_dir().unwrap_or_default();
    let mut files: Vec<std::path::PathBuf> = Vec::new();
    if paths.is_empty() {
        match crate::package::locate(None, &cwd) {
            Some(crate::package::Target::Package(d) | crate::package::Target::Workspace(d, _)) => {
                sisu_files(&d, &mut files)
            }
            _ => {
                host.stderr(&plain_error(
                    msg::CLI_NO_PACKAGE,
                    &[("dir", cwd.display().to_string()), ("command", "fmt".into())],
                ));
                return EXIT_USAGE;
            }
        }
    }
    for p in &paths {
        let path = std::path::PathBuf::from(p.as_str());
        if path.is_dir() {
            let before = files.len();
            sisu_files(&path, &mut files);
            if files.len() == before {
                host.stderr(&plain_error(msg::CLI_FMT_NOTHING, &[("path", p.to_string())]));
            }
        } else {
            files.push(path);
        }
    }
    let mut status = exit::OK;
    for f in files {
        let shown = f.display().to_string();
        let text = match std::fs::read_to_string(&f) {
            Ok(t) => t,
            Err(e) => {
                host.stderr(&plain_error(
                    msg::CLI_CANNOT_READ,
                    &[("path", shown), ("reason", e.to_string())],
                ));
                status = exit::ERROR;
                continue;
            }
        };
        match crate::format::format_file(&shown, &text) {
            Ok(out) if out == text => {}
            Ok(out) => {
                if check {
                    let diff = sisu_interp::diff::hunks(&text, &out, 2);
                    host.stdout(&text_line(msg::CLI_FMT_WOULD_CHANGE, &[("file", shown), ("diff", diff)]));
                    status = exit::ERROR;
                } else if let Err(e) = std::fs::write(&f, out) {
                    host.stderr(&plain_error(
                        msg::CLI_CANNOT_WRITE,
                        &[("path", shown), ("reason", e.to_string())],
                    ));
                    status = exit::ERROR;
                }
            }
            Err(sisu_fmt::Refusal::Syntax(diags)) => {
                let mut map = sisu_diag::SourceMap::new();
                map.add(shown.clone(), text.clone());
                let mut out = sisu_diag::render::render_human(&map, &diags);
                out.push_str(&plain_error(msg::CLI_FMT_SYNTAX, &[("file", shown)]));
                host.stderr(&out);
                status = exit::ERROR;
            }
            Err(sisu_fmt::Refusal::Changed { at }) => {
                host.stderr(&plain_error(msg::CLI_FMT_UNSTABLE, &[("file", shown), ("at", at.to_string())]));
                status = EXIT_ICE;
            }
        }
    }
    status
}

/// `sisu new NAME [--lib]` (10): a package in a new directory `NAME`, with
/// its manifest and a root module, `src/main.sisu` (or `src/lib.sisu`).
fn new_package(args: &[String], host: &mut dyn Host) -> i32 {
    let (flags, names): (Vec<&String>, Vec<&String>) = args.iter().partition(|a| a.starts_with("--"));
    if let Some(bad) = flags.iter().find(|f| f.as_str() != "--lib") {
        host.stderr(&plain_error(
            msg::CLI_UNKNOWN_OPTION,
            &[("option", bad.to_string()), ("command", "new".into())],
        ));
        return EXIT_USAGE;
    }
    let [name] = names.as_slice() else {
        host.stderr(&plain_error(msg::CLI_MISSING_FILE, &[("command", "new".into())]));
        return EXIT_USAGE;
    };
    let lib = !flags.is_empty();
    if !crate::manifest::is_module_name(name) || crate::manifest::RESERVED.contains(&name.as_str()) {
        host.stderr(&plain_error(msg::CLI_BAD_PACKAGE_NAME, &[("name", name.to_string())]));
        return EXIT_USAGE;
    }
    let dir = std::path::Path::new(name.as_str());
    if dir.exists() {
        host.stderr(&plain_error(msg::CLI_EXISTS, &[("path", name.to_string())]));
        return exit::ERROR;
    }
    let manifest =
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2027\"\n\n[dependencies]\n");
    let (root, text) = if lib {
        (
            "lib.sisu",
            format!(
                "##! The library `{name}`.\n\npub def hello() -> str:\n    return \"hello from {name}\"\n\ntest \"hello\":\n    assert hello() == \"hello from {name}\"\n"
            ),
        )
    } else {
        ("main.sisu", "def main():\n    print(\"Hei, maailma!\")\n".to_string())
    };
    let written = std::fs::create_dir_all(dir.join("src"))
        .and_then(|_| std::fs::write(dir.join("sisu.toml"), manifest))
        .and_then(|_| std::fs::write(dir.join("src").join(root), text));
    if let Err(e) = written {
        host.stderr(&plain_error(
            msg::CLI_CANNOT_WRITE,
            &[("path", name.to_string()), ("reason", e.to_string())],
        ));
        return exit::ERROR;
    }
    host.stdout(&text_line(msg::CLI_CREATED, &[("name", name.to_string()), ("root", format!("src/{root}"))]));
    exit::OK
}

fn explain(args: &[String], host: &mut dyn Host) -> i32 {
    let Some(code) = args.first() else {
        host.stderr(&plain_error(msg::CLI_MISSING_FILE, &[("command", "explain".into())]));
        return EXIT_USAGE;
    };
    let catalog = Catalog::builtin();
    let entry = catalog.entries().iter().find(|e| e.code.as_deref() == Some(code.as_str()) || e.id == *code);
    match entry {
        Some(e) => {
            let kind = format!("{:?}", e.kind).to_lowercase();
            host.stdout(&format!(
                "{}\n",
                text(
                    msg::CLI_EXPLAIN,
                    &[
                        ("code", e.code.clone().unwrap_or_default()),
                        ("kind", kind),
                        ("id", e.id.clone()),
                        ("template", e.template.clone())
                    ]
                )
            ));
            exit::OK
        }
        None => {
            host.stderr(&plain_error(msg::CLI_EXPLAIN_UNKNOWN, &[("code", code.clone())]));
            exit::ERROR
        }
    }
}
