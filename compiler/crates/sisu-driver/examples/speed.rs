//! How fast stage 0 is (docs/port-debt.md, Phase 0, "Benchmarks in CI"):
//! the front end's milliseconds per thousand lines of source (from text
//! to the SIR that runs: lexing, parsing, names, types, lowering, the
//! ownership checker) and the interpreter's nanoseconds per step (a SIR
//! statement or terminator), for the standard library and for the
//! conformance corpus. It only prints: CI runs it on every push for the
//! record, and a slow machine fails nothing.
//!
//! From `compiler/`: `cargo run --release -p sisu-driver --example speed`,
//! or with the cases' directory as the argument (`conformance/cases`).

use sisu_driver::Db;
use sisu_interp::{Options, TestHost};
use sisu_resolve::std_sources;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// A program that brings in the standard library and nothing else.
const EMPTY: &str = "def main():\n    pass\n";

/// No program of the corpus takes more steps than this; one that loops
/// forever stops there.
const FUEL: u64 = 20_000_000;

/// The front end of one file from nothing, best of `n`, in seconds.
fn front_end(name: &str, text: &str, n: usize) -> f64 {
    (0..n)
        .map(|_| {
            let mut db = Db::new();
            db.set_file(name, text);
            let start = Instant::now();
            let _ = db.diagnostics(name);
            start.elapsed().as_secs_f64()
        })
        .fold(f64::INFINITY, f64::min)
}

/// The single-file cases of the corpus, sorted.
fn cases(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(groups) = std::fs::read_dir(dir) else { return out };
    for g in groups.flatten() {
        let Ok(files) = std::fs::read_dir(g.path()) else { continue };
        for f in files.flatten() {
            let p = f.path();
            if p.extension().is_some_and(|e| e == "sisu") && p.is_file() {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// Whether a case runs on its own: it has an expected output, and no
/// arguments, input, files or directory of its own.
fn runs_alone(case: &Path) -> bool {
    let has = |ext: &str| case.with_extension(ext).exists();
    has("out") && !["args", "stdin", "files", "cwd", "test", "watch", "repl"].iter().any(|e| has(e))
}

/// Runs a program's `main` (statements and seconds), or `None` when it
/// does not compile.
fn run(name: &str, text: &str) -> Option<(u64, f64)> {
    let mut db = Db::new();
    db.set_file(name, text);
    let prog = db.sir(name)?;
    prog.entry.as_ref()?;
    let mut host = TestHost::default();
    let options = Options { fuel: Some(FUEL), ..Options::default() };
    let start = Instant::now();
    let report = sisu_interp::run_main_report(&prog, &mut host, &options);
    Some((report.steps, start.elapsed().as_secs_f64()))
}

/// The standard library's doc examples as one program's tests, run one at
/// a time (statements and seconds).
fn run_std_examples() -> Option<(u64, f64)> {
    let mut db = Db::new();
    db.set_file("speed.sisu", EMPTY);
    let mut program = db.program("speed.sisu");
    program.std_doc_tests = true;
    db.set_program("speed.sisu", program);
    let prog = db.sir("speed.sisu")?;
    let mut host = TestHost::default();
    let options = Options { fuel: Some(FUEL), jobs: 1, ..Options::default() };
    let start = Instant::now();
    let reports = sisu_interp::run_tests_report(&prog, &mut host, &options);
    let steps = reports.iter().map(|(_, _, r)| r.steps).sum();
    Some((steps, start.elapsed().as_secs_f64()))
}

fn lines(text: &str) -> usize {
    text.lines().count()
}

fn main() {
    let dir = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "conformance/cases".into()));

    // The standard library: what every program compiles (the prelude, the
    // modules every module sees, the builtin types' impls).
    let std_lines = lines(std_sources::PRELUDE)
        + std_sources::AMBIENT.iter().map(|(_, t)| lines(t)).sum::<usize>()
        + lines(std_sources::builtin_impls());
    // The first compilation parses the standard library, which the
    // process keeps for the next ones.
    let first = front_end("speed.sisu", EMPTY, 1);
    let std_time = front_end("speed.sisu", EMPTY, 3);
    println!(
        "front end, std: {std_lines} lines in {:.0} ms, {:.1} ms per kLOC ({:.0} ms once parsed)",
        first * 1e3,
        first * 1e3 / (std_lines as f64 / 1e3),
        std_time * 1e3
    );

    // The corpus: each case's own lines, and its time beyond the
    // standard library's.
    let all = cases(&dir);
    let (mut case_lines, mut case_time) = (0usize, 0f64);
    for c in &all {
        let Ok(text) = std::fs::read_to_string(c) else { continue };
        let name = c.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        case_lines += lines(&text);
        case_time += (front_end(&name, &text, 2) - std_time).max(0.0);
    }
    if case_lines > 0 {
        println!(
            "front end, conformance: {} cases, {case_lines} lines in {:.0} ms beyond std's, {:.1} ms per kLOC, {:.1} ms per case",
            all.len(),
            case_time * 1e3,
            case_time * 1e3 / (case_lines as f64 / 1e3),
            case_time * 1e3 / all.len().max(1) as f64
        );
    }

    // The interpreter: the standard library's doc examples, and every
    // case that runs on its own.
    if let Some((steps, secs)) = run_std_examples().filter(|(s, _)| *s > 0) {
        println!(
            "interpreter, std doc examples: {steps} steps in {:.0} ms, {:.0} ns per step",
            secs * 1e3,
            secs * 1e9 / steps as f64
        );
    }
    let (mut runs, mut steps, mut secs) = (0usize, 0u64, 0f64);
    for c in all.iter().filter(|c| runs_alone(c)) {
        let Ok(text) = std::fs::read_to_string(c) else { continue };
        let name = c.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if let Some((n, t)) = run(&name, &text) {
            runs += 1;
            steps += n;
            secs += t;
        }
    }
    if steps > 0 {
        println!(
            "interpreter, conformance: {runs} programs, {steps} steps in {:.0} ms, {:.0} ns per step",
            secs * 1e3,
            secs * 1e9 / steps as f64
        );
    }
}
