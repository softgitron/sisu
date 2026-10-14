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
        "explain-borrow" => explain_borrow(rest, host),
        "run" | "check" | "sir" | "test" | "tokens" | "parse" => {
            let p = split_args(rest);
            let allowed: &[&str] = match cmd.as_str() {
                "run" => &["--sir", "--monitor", "--defer-borrow"],
                "test" => &["--no-monitor", "--defer-borrow"],
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
                stage.set("the ownership checker");
                let all = db.diagnostics(file);
                let errors = all.iter().filter(|d| d.is_error()).count();
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
                "test" => {
                    stage.set("the interpreter");
                    let options = Options {
                        monitor: defer || !has("--no-monitor"),
                        coverage: deferring,
                        ..Options::default()
                    };
                    if deferring {
                        host.stderr(&format!("{}\n", text(msg::DEFER_HEADER, &[])));
                    }
                    let (code, reports) = run_tests(&prog, host, &options);
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
                        ..Options::default()
                    };
                    if deferring {
                        host.stderr(&format!("{}\n", text(msg::DEFER_HEADER, &[])));
                    }
                    let mut h = WithArgs { inner: host, args };
                    let report = sisu_interp::run_main_report(&prog, &mut h, &options);
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

fn run_tests(
    prog: &sisu_sir::Program,
    host: &mut dyn Host,
    options: &Options,
) -> (i32, Vec<sisu_interp::RunReport>) {
    let results = sisu_interp::run_tests_report(prog, host, options);
    let (mut passed, mut failed, mut blocked) = (0, 0, 0);
    let mut out = String::new();
    let mut failures = String::new();
    for (name, r, _) in &results {
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
