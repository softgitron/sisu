//! The `sisu` command line (proposal/10-tooling.md; 18, Phase 2).
//!
//! Every message the driver prints is a catalog entry (D-065). Exit
//! statuses: 0 success; 1 the program has errors, a test failed, or `main`
//! returned an error; 2 a usage error; 101 a panic; 134 an abort; 70 an
//! internal compiler error.

use crate::db::Db;
use sisu_diag::{Catalog, MsgId, msg};
use sisu_interp::{Host, Options, TestResult, exit};
use std::rc::Rc;

pub const EXIT_USAGE: i32 = 2;
pub const EXIT_ICE: i32 = 70;

fn text(id: MsgId, params: &[(&str, String)]) -> String {
    Catalog::builtin().render(id.id(), params)
}

/// `error[CODE]: message` for a catalog message without a source span.
fn plain_error(id: MsgId, params: &[(&str, String)]) -> String {
    let code = Catalog::builtin().code(id.id()).unwrap_or("");
    format!("error[{code}]: {}\n", text(id, params))
}

/// Delegates to a host, with the program's own arguments.
struct WithArgs<'a> {
    inner: &'a mut dyn Host,
    args: Vec<String>,
}

impl Host for WithArgs<'_> {
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
}

/// The flags that take a value.
const VALUE_FLAGS: &[&str] = &[
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

fn split_args(args: &[String]) -> Parsed<'_> {
    let mut flags = Vec::new();
    let mut values = Vec::new();
    let mut i = 0;
    while i < args.len() && args[i].starts_with("--") {
        let a = args[i].as_str();
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
    }
    let file = args.get(i).map(String::as_str);
    let rest = args.iter().skip(i + 1).cloned().collect();
    Parsed { flags, values, file, rest }
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
struct Captured<'a> {
    inner: &'a mut dyn Host,
    out: String,
    err: String,
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
/// output and the driver's messages go to `host`.
pub fn execute(args: &[String], host: &mut dyn Host) -> i32 {
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
        "explain-borrow" => explain_borrow(rest, host),
        "replay" => replay(rest, host),
        "audit" => audit(rest, host),
        "run" | "check" | "sir" | "test" | "bench" | "tokens" | "parse" => {
            let p = split_args(rest);
            let allowed: &[&str] = match cmd.as_str() {
                "run" => &["--sir", "--monitor", "--defer-borrow", "--stats"],
                "test" => &["--no-monitor", "--defer-borrow", "--sim", "--update"],
                "check" => &["--format=ids", "--format=human", "--explain"],
                "tokens" => &["--trivia"],
                "parse" => &["--tree", "--print", "--trivia"],
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
                !matches!(cmd.as_str(), "run" | "test" | "bench") && !(*k == "--contracts" && cmd == "sir")
            }) {
                host.stderr(&plain_error(
                    msg::CLI_UNKNOWN_OPTION,
                    &[("option", bad.to_string()), ("command", cmd.clone())],
                ));
                return EXIT_USAGE;
            }
            let mut seeds = Seeds { first: 0, explore: None };
            // `--contracts=off|preconditions|all` (07): what a run checks;
            // a test checks everything.
            let mut level = sisu_lower::ContractLevel::All;
            let (mut record, mut redact, mut token) = (None, Vec::new(), None);
            let (mut scheduler, mut depth, mut faults) = ("random", None, None);
            let mut jobs = 0;
            let mut iterations = 100;
            let mut select = Select::default();
            for (k, v) in &p.values {
                let only = match *k {
                    "--record" | "--redact" => Some("run"),
                    "--replay" | "--faults" | "--jobs" | "--filter" | "--tag" | "--skip-tag" => Some("test"),
                    "--iterations" => Some("bench"),
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
                        Some(l) => level = l,
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
            let Some(file) = p.file else {
                host.stderr(&plain_error(msg::CLI_MISSING_FILE, &[("command", cmd.clone())]));
                return EXIT_USAGE;
            };
            let source = match std::fs::read_to_string(file) {
                Ok(s) => s,
                Err(e) => {
                    host.stderr(&plain_error(
                        msg::CLI_CANNOT_READ,
                        &[("path", file.to_string()), ("reason", e.to_string())],
                    ));
                    return exit::ERROR;
                }
            };
            if cmd != "run" && !p.rest.is_empty() {
                host.stderr(&plain_error(
                    msg::CLI_UNKNOWN_OPTION,
                    &[("option", p.rest[0].clone()), ("command", cmd.clone())],
                ));
                return EXIT_USAGE;
            }
            if record.is_some()
                && let Some(other) = ["--explore", "--defer-borrow", "--sir"]
                    .iter()
                    .find(|o| p.flags.contains(o) || p.values.iter().any(|(k, _)| k == *o))
            {
                host.stderr(&plain_error(msg::CLI_RECORD_ONCE, &[("option", other.to_string())]));
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
            };
            let stage = std::cell::Cell::new("the front end");
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                command(cmd, &p.flags, &opts, file, &source, p.rest.clone(), host, &stage)
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
                    let _ = std::fs::write(&dump, &source);
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
        other => {
            host.stderr(&plain_error(msg::CLI_UNKNOWN_COMMAND, &[("command", other.to_string())]));
            EXIT_USAGE
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn command(
    cmd: &str,
    flags: &[&str],
    opts: &Opts,
    file: &str,
    source: &str,
    args: Vec<String>,
    host: &mut dyn Host,
    stage: &std::cell::Cell<&'static str>,
) -> i32 {
    let has = |f: &str| flags.contains(&f);
    let seeds = opts.seeds;
    let level = opts.level;
    if cmd == "run" && has("--sir") {
        stage.set("the SIR reader");
        let prog = match sisu_sir::read(source) {
            Ok(p) => p,
            Err(e) => {
                host.stderr(&plain_error(
                    msg::SIR_PARSE,
                    &[("detail", format!("line {}: {}", e.line, e.message))],
                ));
                return exit::ERROR;
            }
        };
        if let Some(problem) = sisu_sir::validate(&prog).into_iter().next() {
            let (function, detail) =
                problem.split_once(": ").map(|(a, b)| (a.to_string(), b.to_string())).unwrap_or_default();
            host.stderr(&plain_error(msg::SIR_INVALID, &[("function", function), ("detail", detail)]));
            return exit::ERROR;
        }
        stage.set("the interpreter");
        let mut h = WithArgs { inner: host, args };
        let options = Options {
            seed: seeds.first,
            stats: has("--stats"),
            strategy: opts.strategy.clone(),
            ..Options::default()
        };
        return run_main_seeds(&prog, &mut h, &options, seeds, opts.bound);
    }
    let mut db = Db::new();
    db.contracts = level;
    db.set_file(file, source);
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
            if has("--print") {
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
                if has("--explain") {
                    // The signatures, as `sisu doc` renders them (09).
                    let Some(p) = db.parsed(file) else { return exit::ERROR };
                    let lexed = sisu_lexer::lex(p.file, source);
                    host.stdout(&crate::doc::explain(source, &lexed.tokens, &p.module, &c.program, &c.typed));
                }
                if has("--format=ids") {
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
                            std::fs::read_to_string(snap_path(file))
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
                    let hash = sisu_interp::record::content_hash(source.as_bytes());
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
                            file,
                            hash,
                            only: Some(vec![t.test]),
                            bound: opts.bound,
                            search: false,
                            transcripts: Vec::new(),
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
                    let found = if opts.select.is_all() { crate::transcript::find(file) } else { Vec::new() };
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
                    let run = TestRun { file, hash, only, bound: opts.bound, search: true, transcripts };
                    let (code, reports) = run_tests(&prog, host, &options, seeds, &run);
                    if options.update {
                        let ran: Vec<&str> =
                            which.iter().filter_map(|&k| prog.tests.get(k)).map(|t| t.0.as_str()).collect();
                        if let Err(code) =
                            update_snapshots(file, &options.snapshots, &ran, &reports, &prog, host)
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
                    let options = Options {
                        monitor: defer || has("--monitor"),
                        coverage: deferring,
                        seed: seeds.first,
                        stats: has("--stats"),
                        strategy: opts.strategy.clone(),
                        ..Options::default()
                    };
                    if deferring {
                        host.stderr(&format!("{}\n", text(msg::DEFER_HEADER, &[])));
                    }
                    // `sisu replay LOG`: the log stands in for the world.
                    if let Some(log) = &opts.log {
                        let mut rep = sisu_interp::record::Replayer::new(log, host);
                        let report = sisu_interp::run_main_report(&prog, &mut rep, &options);
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
                    let mut h = WithArgs { inner: host, args };
                    let searching = matches!(opts.strategy, sisu_interp::Strategy::Exhaustive { .. });
                    if (seeds.explore.is_some() || searching) && !deferring {
                        return run_main_seeds(&prog, &mut h, &options, seeds, opts.bound);
                    }
                    // `--record FILE`: the run, with every host call logged.
                    if let Some(path) = &opts.record {
                        let header = sisu_interp::record::Header {
                            toolchain: crate::replay::TOOLCHAIN.into(),
                            file: file.to_string(),
                            hash: sisu_interp::record::content_hash(source.as_bytes()),
                            source: source.to_string(),
                            seed: seeds.first,
                            hash_key: 0,
                            contracts: level.name().into(),
                        };
                        let mut rec = sisu_interp::record::Recorder::new(&mut h, opts.redact.clone());
                        let report = sisu_interp::run_main_report(&prog, &mut rec, &options);
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
                    let report = sisu_interp::run_main_report(&prog, &mut h, &options);
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
fn snap_path(file: &str) -> std::path::PathBuf {
    std::path::Path::new(file).with_extension("snap")
}

/// `sisu test --update`: the snapshots the tests that ran took replace
/// the ones they had, each change shown; the others stay. The file is
/// written only when something changed.
fn update_snapshots(
    file: &str,
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
fn text_line(id: sisu_diag::MsgId, params: &[(&str, String)]) -> String {
    format!("{}\n", text(id, params))
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
    let mut results = sisu_interp::run_tests_report_at(prog, host, options, &which);
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
                    .is_some_and(|f| f.param_count == 1);
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
fn audit(args: &[String], host: &mut dyn Host) -> i32 {
    let (flags, files): (Vec<&String>, Vec<&String>) = args.iter().partition(|a| a.starts_with("--"));
    if let Some(bad) = flags.iter().find(|f| f.as_str() != "--ownership") {
        host.stderr(&plain_error(
            msg::CLI_UNKNOWN_OPTION,
            &[("option", bad.to_string()), ("command", "audit".into())],
        ));
        return EXIT_USAGE;
    }
    if flags.is_empty() {
        host.stderr(&plain_error(msg::CLI_AUDIT_USAGE, &[]));
        return EXIT_USAGE;
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

/// `sisu replay LOG`: the run a capability log recorded, again, with the
/// log standing in for the world (10, "Debugging and profiling"). The
/// program is the log's own copy of the source, checked at the level it
/// was recorded at; `--monitor` runs it under the ownership monitor.
fn replay(args: &[String], host: &mut dyn Host) -> i32 {
    let p = split_args(args);
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
    if sisu_interp::record::content_hash(log.header.source.as_bytes()) != log.header.hash {
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
    };
    let stage = std::cell::Cell::new("the front end");
    let flags: Vec<&str> = p.flags.clone();
    command("run", &flags, &opts, &file, &source, Vec::new(), host, &stage)
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
