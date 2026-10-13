//! The `sisu` command line (proposal/10-tooling.md; 18, Phase 2).
//!
//! Every message the driver prints is a catalog entry (D-065). Exit
//! statuses: 0 success; 1 the program has errors, a test failed, or `main`
//! returned an error; 2 a usage error; 101 a panic; 134 an abort; 70 an
//! internal compiler error.

use crate::db::Db;
use sisu_diag::{Catalog, MsgId, msg};
use sisu_interp::{Host, Options, TestResult, exit};

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
    fn read_file(&mut self, path: &str) -> Result<String, sisu_interp::IoErrorKind> {
        self.inner.read_file(path)
    }
    fn write_file(&mut self, path: &str, t: &str) -> Result<(), sisu_interp::IoErrorKind> {
        self.inner.write_file(path, t)
    }
    fn exists(&self, path: &str) -> bool {
        self.inner.exists(path)
    }
}

/// Command-line flags before the file, and the rest.
struct Parsed<'a> {
    flags: Vec<&'a str>,
    file: Option<&'a str>,
    rest: Vec<String>,
}

fn split_args(args: &[String]) -> Parsed<'_> {
    let mut flags = Vec::new();
    let mut i = 0;
    while i < args.len() && args[i].starts_with("--") {
        flags.push(args[i].as_str());
        i += 1;
    }
    let file = args.get(i).map(String::as_str);
    let rest = args.iter().skip(i + 1).cloned().collect();
    Parsed { flags, file, rest }
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
        "run" | "check" | "sir" | "test" | "tokens" | "parse" => {
            let p = split_args(rest);
            let allowed: &[&str] = match cmd.as_str() {
                "run" => &["--sir"],
                "check" => &["--format=ids", "--format=human"],
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
            let stage = std::cell::Cell::new("the front end");
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                command(cmd, &p.flags, file, &source, p.rest.clone(), host, &stage)
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

fn command(
    cmd: &str,
    flags: &[&str],
    file: &str,
    source: &str,
    args: Vec<String>,
    host: &mut dyn Host,
    stage: &std::cell::Cell<&'static str>,
) -> i32 {
    let has = |f: &str| flags.contains(&f);
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
        return sisu_interp::run_main(&prog, &mut h, &Options::default());
    }
    let mut db = Db::new();
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
                if has("--format=ids") {
                    host.stdout(&sisu_diag::render::render_ids(&c.map, &c.diagnostics));
                } else {
                    host.stderr(&sisu_diag::render::render_human(&c.map, &c.diagnostics));
                }
                return if errors > 0 { exit::ERROR } else { exit::OK };
            }
            if errors > 0 {
                let mut out = sisu_diag::render::render_human(&c.map, &c.diagnostics);
                out.push_str(&plain_error(msg::CLI_ERRORS_FOUND, &[("count", errors.to_string())]));
                host.stderr(&out);
                return exit::ERROR;
            }
            if !c.diagnostics.is_empty() {
                host.stderr(&sisu_diag::render::render_human(&c.map, &c.diagnostics));
            }
            stage.set("lowering");
            let Some(prog) = db.sir(file) else { return exit::ERROR };
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
                "test" => {
                    stage.set("the interpreter");
                    run_tests(&prog, host)
                }
                _ => {
                    stage.set("the interpreter");
                    if prog.entry.is_none() {
                        host.stderr(&plain_error(msg::CLI_NO_MAIN, &[("file", file.to_string())]));
                        return exit::ERROR;
                    }
                    let mut h = WithArgs { inner: host, args };
                    sisu_interp::run_main(&prog, &mut h, &Options::default())
                }
            }
        }
    }
}

fn run_tests(prog: &sisu_sir::Program, host: &mut dyn Host) -> i32 {
    let results = sisu_interp::run_tests(prog, host, &Options::default());
    let (mut passed, mut failed, mut blocked) = (0, 0, 0);
    let mut out = String::new();
    let mut failures = String::new();
    for (name, r) in &results {
        let outcome = match r {
            TestResult::Passed => {
                passed += 1;
                "ok".to_string()
            }
            TestResult::Failed(m) => {
                failed += 1;
                failures.push_str(&format!(
                    "\n{}\n{m}",
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
    out.push_str(&failures);
    if !failures.is_empty() {
        out.push('\n');
    }
    out.push_str(&text(
        msg::CLI_TEST_SUMMARY,
        &[("passed", passed.to_string()), ("failed", failed.to_string()), ("blocked", blocked.to_string())],
    ));
    out.push('\n');
    host.stdout(&out);
    if failed > 0 { exit::ERROR } else { exit::OK }
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
