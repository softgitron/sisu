//! The checker checks itself (18, principle 5; Phase 3 exit criteria).
//!
//! Two tests of the cross-check between the static ownership checker and
//! the dynamic monitor:
//!
//! - **Fault injection**: switch one of the checker's rules off and the
//!   programs it existed to reject are accepted; each of them must then
//!   stop under the monitor with an ownership violation. A checker with a
//!   soundness hole is caught by the monitor, which is what the
//!   conformance suite's monitor rerun relies on.
//! - **Mutation fuzzing**: random reorderings of adjacent statements in
//!   the SIR of accepted programs, before drop elaboration. A mutant the
//!   checker accepts must run without a monitor violation and without an
//!   internal error in the interpreter (which would mean the checker
//!   accepted a use of an unassigned local). The exit criterion asks for
//!   24 hours of this; the test runs a fixed, small number of mutants
//!   (`SISU_FUZZ_MUTANTS` per program raises it) from a fixed seed, so it
//!   is deterministic.

use sisu_borrowck::{Options as CheckOptions, Rule};
use sisu_diag::SourceMap;
use sisu_interp::{Options, TestHost};
use sisu_sir as sir;
use sisu_syntax::ast;
use std::path::PathBuf;

/// Lowers a program that has no front-end errors (`None` otherwise).
fn lower(src: &str) -> Option<sir::Program> {
    let mut map = SourceMap::new();
    let file = map.add("t.sisu", src);
    let parse = sisu_parser::parse(file, src);
    if !parse.diagnostics.is_empty() {
        return None;
    }
    let m = ast::build::module(&parse.tree);
    let r = sisu_resolve::resolve(&mut map, file, &m);
    if r.diagnostics.iter().any(|d| d.is_error()) {
        return None;
    }
    let (typed, diags) = sisu_typeck::check(&r.program, &map);
    if diags.iter().any(|d| d.is_error()) {
        return None;
    }
    Some(sisu_lower::lower(&r.program, &typed, &map))
}

/// Runs `main` under the monitor; returns whether the monitor stopped it,
/// whether the interpreter hit an internal error, and what it printed.
fn monitored(p: &sir::Program) -> (bool, bool, String) {
    let mut host = TestHost::default();
    let options = Options { fuel: Some(200_000), monitor: true, ..Options::default() };
    let report = sisu_interp::run_main_report(p, &mut host, &options);
    let internal = host.err.contains("internal error in the interpreter");
    (report.violation, internal, host.err)
}

const LOG: &str = "view struct Log:\n    name: str\n\nimpl Drop for Log:\n    def drop(mut self):\n        print(self.name)\n\n";

/// (rule switched off, program the full checker rejects).
fn holes() -> Vec<(Rule, String)> {
    vec![
        (
            Rule::Conflicts,
            "def main():\n    var xs = [1, 2, 3]\n    let first = ref xs[0]\n    xs.push(4)\n    print(first)\n".into(),
        ),
        (
            Rule::Conflicts,
            "def main():\n    var n = 1\n    let r = ref mut n\n    print(n)\n    r += 1\n    print(r)\n".into(),
        ),
        (
            Rule::Conflicts,
            "def main():\n    let y = 1\n    let r: ref int\n    if True:\n        let x = 5\n        r = ref x\n    else:\n        r = ref y\n    print(r)\n"
                .into(),
        ),
        (
            Rule::Conflicts,
            "def main():\n    var s = String.new(\"a\")\n    let v: str = s\n    s.push(\"b\")\n    print(v)\n".into(),
        ),
        (
            Rule::Conflicts,
            "def main():\n    var xs = [1, 2]\n    for x in xs:\n        xs.push(x)\n".into(),
        ),
        (
            Rule::Moves,
            "struct U:\n    name: String\n\ndef take(own u: U):\n    pass\n\ndef main():\n    let u = U(name=\"a\".to_string())\n    take(u)\n    print(u.name)\n"
                .into(),
        ),
        (
            Rule::Moves,
            "def main():\n    let label: String\n    if len(\"abc\") > 5:\n        label = \"long\".to_string()\n    print(label)\n"
                .into(),
        ),
        (
            Rule::Escapes,
            "def biggest(xs: list[int]) -> ref int:\n    var best = xs[0]\n    return ref best\n\ndef main():\n    print(biggest([3, 9]))\n"
                .into(),
        ),
        (
            Rule::Escapes,
            "def split_into(mut out: list[str], text: str):\n    for w in text.split_whitespace():\n        out.push(w)\n\ndef main():\n    var words: list[str] = []\n    if True:\n        let s = String.new(\"a b\")\n        split_into(mut words, s)\n    print(words)\n"
                .into(),
        ),
        (
            Rule::MustConsume,
            "@must_consume\nstruct Receipt:\n    amount: int\n\n    def settled(own self) -> int:\n        return self.amount\n\ndef main():\n    let r = Receipt(amount=1)\n    print(r.amount)\n"
                .into(),
        ),
        (
            Rule::Moves,
            format!("{LOG}def take(own l: Log):\n    pass\n\ndef main():\n    let a = Log(name=\"a\")\n    take(a)\n    print(a.name)\n"),
        ),
        // A task borrowing a local of the scope's body, which is gone
        // before the scope joins the task (06).
        (
            Rule::Escapes,
            "def main():\n    task.scope do s:\n        let name = String.new(\"x\")\n        s.spawn(() -> print(name))\n"
                .into(),
        ),
    ]
}

/// Runs `main` under the monitor with several schedule seeds; returns
/// whether any run stopped at a violation.
fn monitored_seeds(p: &sir::Program, seeds: u64) -> (bool, String) {
    for seed in 0..seeds {
        let mut host = TestHost::default();
        let options = Options { fuel: Some(200_000), monitor: true, seed, ..Options::default() };
        let report = sisu_interp::run_main_report(p, &mut host, &options);
        if report.violation {
            return (true, host.err);
        }
    }
    (false, String::new())
}

/// The pinned loans of a task (06): with the conflict rules off, a body
/// that reads what a task it spawned changes is accepted, and some
/// schedule stops under the monitor.
#[test]
fn the_monitor_catches_a_task_race() {
    let src = "def main():\n    var c = 0\n    task.scope do s:\n        s.spawn do:\n            c += 1\n        s.spawn(() -> print(c))\n";
    let p = lower(src).unwrap_or_else(|| panic!("front-end errors in\n{src}"));
    let full = sisu_borrowck::check(&p, &CheckOptions::default());
    assert!(full.has_errors(), "the full checker accepts\n{src}");
    let weak =
        sisu_borrowck::check(&p, &CheckOptions { disabled: vec![Rule::Conflicts], ..Default::default() });
    assert!(!weak.has_errors(), "with the conflict rules off the checker still rejects\n{src}");
    let (violation, _) = monitored_seeds(&weak.program, 64);
    assert!(violation, "no schedule of 64 stopped under the monitor:\n{src}");
}

/// The `Shared` escape analysis checked by the monitor (18, Phase 6): with
/// the analysis off every site is task-local, and a `Shared` that tasks
/// share is the monitor's soundness error; with it on, the run is clean.
#[test]
fn the_monitor_checks_shared_atomicity() {
    let src = "def main():\n    let hits = Shared(Mutex(0))\n    task.scope do s:\n        for _ in range(4):\n            s.spawn do:\n                hits.lock().value += 1\n    print(hits.lock().value)\n";
    let p = lower(src).unwrap_or_else(|| panic!("front-end errors in\n{src}"));
    let full = sisu_borrowck::check(&p, &CheckOptions::default());
    assert!(!full.has_errors());
    assert_eq!(full.shared.crossing, 1, "the site is may-cross");
    let (violation, err) = monitored_seeds(&full.program, 8);
    assert!(!violation, "the tagged program stopped under the monitor:\n{err}");
    let weak =
        sisu_borrowck::check(&p, &CheckOptions { disabled: vec![Rule::SharedEscape], ..Default::default() });
    let (violation, _) = monitored_seeds(&weak.program, 8);
    assert!(violation, "a task-local tag on a shared `Shared` ran without a violation");
}

#[test]
fn the_monitor_catches_a_weakened_checker() {
    for (rule, src) in holes() {
        let p = lower(&src).unwrap_or_else(|| panic!("front-end errors in\n{src}"));
        let full = sisu_borrowck::check(&p, &CheckOptions::default());
        assert!(full.has_errors(), "the full checker accepts\n{src}");
        let weak = sisu_borrowck::check(&p, &CheckOptions { disabled: vec![rule], ..Default::default() });
        assert!(
            !weak.has_errors(),
            "with {rule:?} off the checker still rejects\n{src}\n{:?}",
            weak.findings.iter().map(|f| f.msg.id()).collect::<Vec<_>>()
        );
        let (violation, internal, err) = monitored(&weak.program);
        assert!(violation, "the monitor ran without a violation (rule {rule:?} off):\n{src}\nstderr:\n{err}");
        assert!(!internal, "internal error instead of a monitor report:\n{src}\n{err}");
    }
}

/// A small deterministic generator (xorshift64*).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

/// Reorders the statements of a random block that has at least two:
/// swaps two neighbours, or moves one statement elsewhere in the block.
fn mutate(p: &mut sir::Program, rng: &mut Rng) -> bool {
    let candidates: Vec<(usize, usize)> = p
        .functions
        .iter()
        .enumerate()
        .flat_map(|(fi, f)| {
            f.blocks
                .iter()
                .enumerate()
                .filter(|(_, b)| b.stmts.len() >= 2 && !b.cleanup)
                .map(move |(bi, _)| (fi, bi))
        })
        .collect();
    if candidates.is_empty() {
        return false;
    }
    let (fi, bi) = candidates[rng.below(candidates.len())];
    let block = &mut p.functions[fi].blocks[bi];
    let n = block.stmts.len();
    if rng.below(2) == 0 {
        let i = rng.below(n - 1);
        block.stmts.swap(i, i + 1);
    } else {
        let s = block.stmts.remove(rng.below(n));
        block.stmts.insert(rng.below(n), s);
    }
    true
}

/// The accepted conformance programs that run on their own (no
/// arguments, no input files).
fn corpus() -> Vec<(String, String)> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance/cases");
    let mut out = Vec::new();
    for group in ["ownership", "run"] {
        let Ok(entries) = std::fs::read_dir(root.join(group)) else { continue };
        let mut paths: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
        paths.sort();
        for path in paths {
            if path.extension().is_none_or(|e| e != "sisu") {
                continue;
            }
            let stem = path.with_extension("");
            if stem.with_extension("diag").exists()
                || stem.with_extension("args").exists()
                || stem.with_extension("files").exists()
            {
                continue;
            }
            if let Ok(src) = std::fs::read_to_string(&path) {
                out.push((path.display().to_string(), src));
            }
        }
    }
    out
}

#[test]
fn accepted_mutants_never_trip_the_monitor() {
    let per_program: usize =
        std::env::var("SISU_FUZZ_MUTANTS").ok().and_then(|v| v.parse().ok()).unwrap_or(12);
    let mut rng = Rng(0x5157_5f6d_6f6e_6974);
    let (mut tried, mut accepted) = (0, 0);
    for (name, src) in corpus() {
        let Some(base) = lower(&src) else { continue };
        if sisu_borrowck::check(&base, &CheckOptions::default()).has_errors() {
            continue;
        }
        for _ in 0..per_program {
            let mut p = base.clone();
            // One to three swaps per mutant.
            let swaps = 1 + rng.below(3);
            for _ in 0..swaps {
                mutate(&mut p, &mut rng);
            }
            tried += 1;
            let checked = sisu_borrowck::check(&p, &CheckOptions::default());
            if checked.has_errors() || !sir::validate(&checked.program).is_empty() {
                continue;
            }
            accepted += 1;
            let (violation, internal, err) = monitored(&checked.program);
            assert!(
                !violation && !internal,
                "a mutant of {name} that the checker accepts {}:\n{err}\n{}",
                if violation { "trips the monitor" } else { "breaks the interpreter" },
                sir::write(&checked.program)
            );
        }
    }
    assert!(tried > 0, "no programs to mutate");
    println!("{tried} mutants, {accepted} accepted by the checker, none tripped the monitor");
}
