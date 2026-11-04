use crate::cli::execute;
use crate::db::{Counters, Db};
use sisu_interp::TestHost;

#[test]
fn queries_are_reused_until_the_text_changes() {
    let mut db = Db::new();
    db.set_file("a.sisu", "def main():\n    print(1)\n");
    assert!(db.sir("a.sisu").is_some());
    assert!(db.checked("a.sisu").is_some());
    assert!(db.sir("a.sisu").is_some());
    let steps = |c: Counters| (c.parse, c.check, c.lower);
    assert_eq!(steps(db.counters), (1, 1, 1));
    // The same text again: nothing reruns.
    db.set_file("a.sisu", "def main():\n    print(1)\n");
    assert!(db.sir("a.sisu").is_some());
    assert_eq!(steps(db.counters), (1, 1, 1));
    // A change reruns every step after it, each for the items it changed:
    // `main`'s body is checked and lowered again, nothing else.
    db.set_file("a.sisu", "def main():\n    print(2)\n");
    assert!(db.sir("a.sisu").is_some());
    assert_eq!(steps(db.counters), (2, 2, 2));
    assert_eq!((db.counters.bodies_checked, db.counters.bodies_lowered), (1, 1));
}

#[test]
fn a_program_with_errors_has_no_sir() {
    let mut db = Db::new();
    db.set_file("b.sisu", "def main():\n    print(missing)\n");
    assert!(db.checked("b.sisu").is_some_and(|c| c.has_errors()));
    assert!(db.sir("b.sisu").is_none());
}

fn temp_file(name: &str, text: &str) -> String {
    let dir = std::env::temp_dir().join(format!("sisu-driver-test-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join(name);
    let _ = std::fs::write(&path, text);
    path.display().to_string()
}

/// The command line in this process, with the Rust front end: stage 1's
/// path runs this executable as its backend, which a test harness is not
/// (the `sisu` binary's tests and the conformance suite run that path).
fn cli(args: &[&str]) -> (i32, TestHost) {
    let mut host = TestHost::default();
    let args: Vec<String> =
        std::iter::once("--front-end=rust").chain(args.iter().copied()).map(|s| s.to_string()).collect();
    let code = execute(&args, &mut host);
    (code, host)
}

#[test]
fn check_reports_by_identifier() {
    let f = temp_file("check.sisu", "def main():\n    print(missing)\n");
    let (code, host) = cli(&["check", "--format=ids", &f]);
    assert_eq!(code, 1);
    assert_eq!(host.out, "2:11-2:18 E0200 resolve.undefined-name name=\"missing\"\n");
}

#[test]
fn run_passes_arguments_and_prints() {
    let f = temp_file("run.sisu", "def main(sys: System):\n    for a in sys.env.args():\n        print(a)\n");
    let (code, host) = cli(&["run", &f, "x", "y"]);
    assert_eq!(code, 0, "{}", host.err);
    assert_eq!(host.out, "x\ny\n");
}

#[test]
fn run_refuses_a_program_with_errors() {
    let f = temp_file("bad.sisu", "def main():\n    let x: int = \"s\"\n");
    let (code, host) = cli(&["run", &f]);
    assert_eq!(code, 1);
    assert!(host.err.contains("error[E0300]"), "{}", host.err);
    assert!(host.err.contains("the program was not run: errors found (1)"), "{}", host.err);
}

#[test]
fn the_sir_text_runs() {
    let f = temp_file("sir.sisu", "def main():\n    print(\"from SIR\")\n");
    let (code, host) = cli(&["sir", &f]);
    assert_eq!(code, 0);
    let s = temp_file("prog.sir", &host.out);
    let (code, host) = cli(&["run", "--sir", &s]);
    assert_eq!(code, 0, "{}", host.err);
    assert_eq!(host.out, "from SIR\n");
}

#[test]
fn run_from_sir_interprets_the_program_with_its_arguments() {
    let f = temp_file(
        "from_sir.sisu",
        "def main(sys: System):\n    for a in sys.env.args():\n        print(a)\n",
    );
    let (code, host) = cli(&["sir", &f]);
    assert_eq!(code, 0, "{}", host.err);
    let s = temp_file("from_sir.sir", &host.out);
    let (code, host) = cli(&["run", "--from-sir", &s, "x", "y z"]);
    assert_eq!(code, 0, "{}", host.err);
    assert_eq!(host.out, "x\ny z\n");
    // A build is native code: `--from-sir` goes with `--native` there.
    let (code, host) = cli(&["build", "--from-sir", &s]);
    assert_eq!(code, crate::cli::EXIT_USAGE);
    assert!(host.err.contains("error[D0110]"), "{}", host.err);
    // LIR, stage 2's lowering of SIR, is compiled by the native backend
    // and nothing else (decision 284).
    for cmd in ["build", "run"] {
        let (code, host) = cli(&[cmd, "--from-lir", &s]);
        assert_eq!(code, crate::cli::EXIT_USAGE, "{cmd}");
        assert!(host.err.contains("error[D0119]"), "{cmd}: {}", host.err);
    }
}

/// `sisu sir --tests` prints the program `sisu test` and `sisu bench` run,
/// with each test's tags and the benchmarks, and `sisu test --from-sir`
/// and `sisu bench --from-sir` run them as the program's source runs them
/// (decision 285).
#[test]
fn tests_and_benches_run_from_sir() {
    let f = temp_file(
        "with_tests.sisu",
        "def double(x: int) -> int:\n    return x * 2\n\ntest \"doubles\":\n    assert double(2) == 4\n\nsuite \"edges\":\n    @tag(\"slow\")\n    test \"zero\":\n        assert double(0) == 0\n\nbench \"doubling\":\n    _ = double(21)\n",
    );
    let (code, host) = cli(&["sir", "--tests", &f]);
    assert_eq!(code, 0, "{}", host.err);
    assert!(host.out.contains("\ntest \"doubles\" test#0\ntest \"edges/zero\" test#1 tags \"slow\"\nbench \"doubling\" bench#0\n"), "{}", host.out);
    // Without `--tests`, the program is built without them, as before.
    let (_, plain) = cli(&["sir", &f]);
    assert!(!plain.out.contains("\ntest ") && !plain.out.contains("\nbench "), "{}", plain.out);
    let s = temp_file("with_tests.sir", &host.out);
    let (code, from_source) = cli(&["test", "--no-cache", &f]);
    assert_eq!(code, 0, "{}", from_source.err);
    let (code, from_sir) = cli(&["test", "--from-sir", &s]);
    assert_eq!(code, 0, "{}", from_sir.err);
    assert_eq!(from_sir.out, from_source.out);
    let (code, tagged) = cli(&["test", "--from-sir", &s, "--tag", "slow"]);
    assert_eq!(code, 0, "{}", tagged.err);
    assert!(
        tagged.out.contains("test \"edges/zero\" ... ok") && !tagged.out.contains("\"doubles\""),
        "{}",
        tagged.out
    );
    let (code, bench) = cli(&["bench", "--from-sir", &s, "--iterations", "3"]);
    assert_eq!(code, 0, "{}", bench.err);
    // The median of ten samples of three iterations each (decision 468).
    assert!(
        bench.out.starts_with("bench \"doubling\" ... ")
            && bench.out.contains("the median of 10 samples of 3 iterations (95% interval of the mean"),
        "{}",
        bench.out
    );
}

#[test]
fn parse_print_is_lossless() {
    let text = "# comment\ndef  main( ):   # odd spacing\n    print( 1 )\n\n\n";
    let f = temp_file("lossless.sisu", text);
    let (_, host) = cli(&["parse", "--print", &f]);
    assert_eq!(host.out, text);
}

#[test]
fn parse_ast_prints_the_ast_the_same_every_time() {
    let f = temp_file("ast.sisu", "def double(x: int) -> int:\n    return x * 2\n");
    let (code, first) = cli(&["parse", "--ast", &f]);
    assert_eq!(code, 0, "{}", first.err);
    assert!(first.out.starts_with("Module {\n    items: [\n"), "{}", first.out);
    assert!(first.out.contains("text: \"double\""), "{}", first.out);
    assert!(first.out.contains("Binary"), "{}", first.out);
    assert!(!first.out.contains("0x"), "{}", first.out);
    let (_, second) = cli(&["parse", "--ast", &f]);
    assert_eq!(first.out, second.out);
}

#[test]
fn usage_errors() {
    assert_eq!(cli(&[]).0, crate::cli::EXIT_USAGE);
    assert_eq!(cli(&["frobnicate"]).0, crate::cli::EXIT_USAGE);
    assert_eq!(cli(&["check"]).0, crate::cli::EXIT_USAGE);
    assert_eq!(cli(&["check", "--nope", "x.sisu"]).0, crate::cli::EXIT_USAGE);
}

#[test]
fn explain_names_the_catalog_entry() {
    let (code, host) = cli(&["explain", "E0200"]);
    assert_eq!(code, 0);
    assert!(host.out.contains("resolve.undefined-name"), "{}", host.out);
    // As a reader sees the message: one brace where the catalog doubles it
    // (decision 386).
    let (code, host) = cli(&["explain", "E0121"]);
    assert_eq!(code, 0);
    assert!(host.out.contains("between `{` and `}`"), "{}", host.out);
}

/// The toolchain's version, and what `sisu explain`, `sisu new` and
/// `--stats` need, said as such (decision 386; the manual's findings R16).
#[test]
fn the_version_and_what_a_command_needs() {
    for args in [&["--version"][..], &["version"][..]] {
        let (code, host) = cli(args);
        assert_eq!((code, host.out.as_str()), (0, "sisu 0.0.1\n"), "{args:?}");
    }
    let (code, host) = cli(&["explain"]);
    assert_eq!(code, crate::cli::EXIT_USAGE);
    assert!(host.err.starts_with("error[D0129]: `sisu explain` needs a diagnostic's code"), "{}", host.err);
    let (code, host) = cli(&["new"]);
    assert_eq!(code, crate::cli::EXIT_USAGE);
    assert!(host.err.starts_with("error[D0130]: `sisu new` needs the name"), "{}", host.err);
    let path = temp_file("stats.sisu", "def main():\n    print(1)\n");
    let (code, host) = cli(&["run", "--stats", "bogus", &path]);
    assert_eq!(code, crate::cli::EXIT_USAGE);
    assert!(host.err.starts_with("error[D0094]"), "{}", host.err);
}

/// The robustness property of 18, Phase 0, checked on mutated programs:
/// every stage returns, the tree reprints its input byte for byte, and a
/// mutated program that still checks without errors lowers to valid SIR.
/// A fixed seed keeps it deterministic; the mutations are the ones a
/// typing hand makes (deleted, inserted and swapped characters).
#[test]
fn mutated_programs_never_break_the_pipeline() {
    let seeds = [
        include_str!("../../../../proposal/examples/hello.sisu"),
        include_str!("../../../../proposal/examples/wordcount.sisu"),
        include_str!("../../../../proposal/examples/errors.sisu"),
        include_str!("../../../../proposal/examples/ownership.sisu"),
        include_str!("../../../../proposal/examples/concurrency.sisu"),
        include_str!("../../../conformance/cases/run/drop_torture.sisu"),
        include_str!("../../../conformance/cases/run/structs_enums.sisu"),
    ];
    let pieces = [
        "(", ")", "[", "]", "{", "}", ":", "\n", "    ", "\t", "\"", "'", "?", "->", "=", "def ", "if ",
        "case ", "*", "|", "#", "\r", ";", ",", "é", "{x}", "\"\"\"",
    ];
    let mut state: u64 = 0x5eed_5eed_5eed_5eed;
    let mut next = |n: usize| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % n.max(1) as u64) as usize
    };
    for round in 0..300 {
        let seed = seeds[round % seeds.len()];
        let mut chars: Vec<char> = seed.chars().collect();
        for _ in 0..1 + next(3) {
            if chars.is_empty() {
                break;
            }
            let i = next(chars.len());
            match next(3) {
                0 => {
                    let n = 1 + next(4);
                    chars.drain(i..(i + n).min(chars.len()));
                }
                1 => {
                    let p = pieces[next(pieces.len())];
                    for (k, c) in p.chars().enumerate() {
                        chars.insert(i + k, c);
                    }
                }
                _ => {
                    let j = next(chars.len());
                    chars.swap(i, j);
                }
            }
        }
        let text: String = chars.into_iter().collect();
        let mut db = Db::new();
        db.set_file("m.sisu", &text);
        let parsed = db.parsed("m.sisu");
        assert_eq!(
            parsed.map(|p| p.parse.tree.to_source()),
            Some(text.clone()),
            "round {round}: the tree lost text"
        );
        let checked = db.checked("m.sisu");
        assert!(checked.is_some(), "round {round}");
        if let Some(sir) = db.sir("m.sisu") {
            let problems = sisu_sir::validate(&sir);
            assert!(problems.is_empty(), "round {round}: invalid SIR {problems:?}\n{text}");
        }
    }
}

#[test]
fn audit_counts_the_escape_hatches_per_file() {
    // `Key` is the file's own type here, so it is not counted; neither are
    // names in comments and strings, a field after a `.`, nor `copy`
    // that is not called.
    let a = temp_file(
        "audit_a.sisu",
        "# Shared, Weak and Cell in a comment\n\
         struct Key:\n    copy: int\n\n\
         def main():\n    let s = Shared(Cell(1))\n    let w = Shared.downgrade(s)\n    \
         let t = s.copy()\n    let k = Key(copy=2)\n    print(k.copy)\n    print(\"Weak\")\n",
    );
    let b = temp_file("audit_b.sisu", "def f(xs: Arena[int]) -> Key[int]?:\n    return None\n");
    let (code, host) = cli(&["audit", "--ownership", &a, &b]);
    assert_eq!(code, 0, "{}", host.err);
    assert_eq!(
        host.out,
        format!(
            "{a} lines=9 Shared=2 Weak=0 Cell=1 Arena=0 Key=0 copy=1 unsafe=0\n\
             {b} lines=2 Shared=0 Weak=0 Cell=0 Arena=1 Key=1 copy=0 unsafe=0\n"
        )
    );
    let (code, host) = cli(&["audit", &a]);
    assert_eq!(code, 2);
    assert!(host.err.contains("error[D0022]"), "{}", host.err);
}

/// `sisu check --diary`: every diagnostic, then, once a later check no
/// longer has it, what resolved it and the fix that was applied.
#[test]
fn the_diary_logs_diagnostics_and_what_resolved_them() {
    let f = temp_file("diary.sisu", "def main():\n    let count = 3\n    print(cuont)\n");
    let log = temp_file("diary.jsonl", "");
    let _ = std::fs::remove_file(&log);
    let (code, _) = cli(&["check", "--diary", &log, &f]);
    assert_eq!(code, 1);
    let _ = std::fs::write(&f, "def main():\n    let count = 3\n    print(count)\n");
    let (code, host) = cli(&["check", "--diary", &log, &f]);
    assert_eq!(code, 0, "{}", host.err);
    let text = std::fs::read_to_string(&log).unwrap_or_default();
    let records: Vec<crate::json::Json> = text.lines().filter_map(|l| crate::json::parse(l).ok()).collect();
    let kinds: Vec<&str> =
        records.iter().filter_map(|r| r.get("record").and_then(crate::json::Json::as_str)).collect();
    assert_eq!(kinds, ["check", "diagnostic", "check", "resolved"], "{text}");
    let d = &records[1];
    assert_eq!(d.get("id").and_then(crate::json::Json::as_str), Some("resolve.undefined-name"));
    assert_eq!(d.at(&["span", "start", "line"]).and_then(crate::json::Json::as_u64), Some(3));
    assert_eq!(
        d.get("fixes").and_then(crate::json::Json::as_array).map(<[_]>::len),
        Some(1),
        "the fix-its are logged: {text}"
    );
    let r = &records[3];
    assert_eq!(r.get("fix").and_then(crate::json::Json::as_str), Some("resolve.similar-name"), "{text}");
    assert_eq!(r.at(&["edit", "text"]).and_then(crate::json::Json::as_str), Some("ou"), "{text}");
}

/// `sisu run --stats stack-depth` and `--stats shared`.
#[test]
fn run_stats_report_depths_and_shared_sites() {
    let f = temp_file(
        "depth.sisu",
        "def depth(n: int) -> int:\n    if n == 0:\n        return 0\n    return 1 + depth(n - 1)\n\ndef main():\n    print(depth(9))\n",
    );
    let (code, host) = cli(&["run", "--stats", "stack-depth", &f]);
    assert_eq!(code, 0, "{}", host.err);
    assert!(host.err.contains("the deepest stack 11 frames"), "{}", host.err);
    assert!(host.err.contains("4 calls at a depth of 8 to 15 frames"), "{}", host.err);
    let f = temp_file(
        "shared.sisu",
        "struct Count:\n    n: int\n\ndef main():\n    let s = Shared(Count(n=5))\n    let t = s.copy()\n    print(t.n + 1)\n",
    );
    let (code, host) = cli(&["run", "--stats", "shared", &f]);
    assert_eq!(code, 0, "{}", host.err);
    assert!(host.err.contains("1 of 1 `Shared` construction sites tagged task-local"), "{}", host.err);
    assert!(
        host.err.contains(":5:13 in `main`, task-local: 1 cells made, 0 reached by another task"),
        "{}",
        host.err
    );
    let (code, _) = cli(&["run", "--stats=nonsense", &f]);
    assert_eq!(code, crate::cli::EXIT_USAGE);
}

/// `sisu audit --caps`: the capability parameters of the functions that
/// do I/O, the depth of each use from `main`, and the structs that bundle
/// three or more capability types (decision 240).
#[test]
fn audit_caps_counts_capability_threading() {
    let f = temp_file(
        "audit_caps.sisu",
        "struct Services:\n    fs: Fs\n    clock: Clock\n    rand: Rand\n\n\
         def read_config(fs: Fs, name: Path) -> String:\n    return fs.read_to_string(name).expect(\"readable\")\n\n\
         def load[F: FsOps](fs: F) -> bool:\n    return fs.exists(\"x\")\n\n\
         def setup(sys: System) -> String:\n    print(load(sys.fs.lend()))\n    return read_config(sys.fs, \"app.conf\")\n\n\
         def pure_math(x: int) -> int:\n    return x * 2\n\n\
         def main(sys: System):\n    let s = Services(fs=sys.fs, clock=sys.clock, rand=sys.rand)\n    \
         print(setup(sys))\n    print(s.clock.now())\n    print(pure_math(2))\n",
    );
    let (code, host) = cli(&["audit", "--caps", &f]);
    assert_eq!(code, 0, "{}", host.err);
    let lines: Vec<String> =
        host.out.lines().map(|l| l.strip_prefix(&format!("{f} ")).unwrap_or(l).to_string()).collect();
    for want in [
        "function load caps=1 io=direct depth=2",
        "function main caps=1 io=direct depth=0",
        "function read_config caps=1 io=direct depth=2",
        "function setup caps=1 io=calls depth=1",
        "use load FsOps.exists depth=2",
        "use read_config FsOps.read_to_string depth=2",
        "struct Services caps=3 Fs,Clock,Rand",
    ] {
        assert!(lines.iter().any(|l| l == want), "missing `{want}` in:\n{}", host.out);
    }
    assert!(!lines.iter().any(|l| l.contains("pure_math")), "{}", host.out);
    assert!(
        lines.last().is_some_and(|l| l.starts_with("summary functions=4 median=1 max=1 ")),
        "{}",
        host.out
    );
    let (code, host) = cli(&["audit", "--caps", "--ownership", &f]);
    assert_eq!(code, 2);
    assert!(host.err.contains("error[D0022]"), "{}", host.err);
}

/// The test cache (decision 242): a test that passed is not run again
/// while what it reaches is the same, and prints what it printed; an edit
/// of what it reaches runs it again.
#[test]
fn the_test_cache_keeps_what_passed() {
    let f = temp_file(
        "cached_tests.sisu",
        "def answer() -> int:\n    return 42\n\ndef other() -> int:\n    return 1\n\n\
         test \"prints and passes\"(fs: Fs):\n    print(\"ran\")\n    assert answer() == 42\n\n\
         test \"another\":\n    assert other() == 1\n",
    );
    let cache = std::path::Path::new(&f).with_extension("test-cache");
    let _ = std::fs::remove_file(&cache);
    let (code, first) = cli(&["test", &f]);
    assert_eq!(code, 0, "{}", first.err);
    let held = std::fs::read_to_string(&cache).unwrap_or_default();
    assert_eq!(held.lines().count(), 2, "{held}");
    assert!(held.contains("\"out\":\"ran\\n\""), "{held}");
    // The same output from the cache.
    let (code, second) = cli(&["test", &f]);
    assert_eq!((code, second.out.as_str()), (0, first.out.as_str()));
    // An edit of what `another` reaches runs it again, and it fails now;
    // the first test stays cached.
    let source = std::fs::read_to_string(&f).unwrap_or_default();
    let _ = std::fs::write(&f, source.replace("return 1", "return 2"));
    let (code, third) = cli(&["test", &f]);
    assert_eq!(code, 1);
    assert!(third.out.contains("test \"another\" ... FAILED"), "{}", third.out);
    assert!(third.out.starts_with("ran\n"), "{}", third.out);
    let held = std::fs::read_to_string(&cache).unwrap_or_default();
    assert_eq!(held.lines().count(), 1, "{held}");
    let _ = std::fs::remove_file(&cache);
}

#[test]
fn the_features_switch_turns_features_on() {
    // A package whose `@cfg`s read its features (decision 250): `--features`
    // turns them on, `--no-default-features` leaves `default` off, and a
    // name that is no feature is a usage error.
    let dir = std::env::temp_dir().join(format!("sisu-driver-features-{}", std::process::id()));
    let _ = std::fs::create_dir_all(dir.join("src"));
    let _ = std::fs::write(
        dir.join("sisu.toml"),
        "[package]\nname = \"feat\"\nversion = \"0.1.0\"\n\n[features]\ndefault = [\"fast\"]\nfast = []\nloud = []\n",
    );
    let _ = std::fs::write(
        dir.join("src/main.sisu"),
        "@cfg(feature=\"fast\")\ndef speed() -> str:\n    return \"fast\"\n\n\
         @cfg(not(feature=\"fast\"))\ndef speed() -> str:\n    return \"slow\"\n\n\
         @cfg(feature=\"loud\")\ndef volume() -> str:\n    return \"loud\"\n\n\
         @cfg(not(feature=\"loud\"))\ndef volume() -> str:\n    return \"quiet\"\n\n\
         def main():\n    print(\"{speed()} {volume()}\")\n",
    );
    let d = dir.display().to_string();
    let run = |args: &[&str]| {
        let (code, host) = cli(args);
        (code, host.out, host.err)
    };
    assert_eq!(run(&["run", &d]), (0, "fast quiet\n".to_string(), String::new()));
    assert_eq!(run(&["run", "--features", "loud", &d]), (0, "fast loud\n".to_string(), String::new()));
    assert_eq!(
        run(&["run", "--no-default-features", "--features=loud", &d]),
        (0, "slow loud\n".to_string(), String::new())
    );
    let (code, _, err) = run(&["check", "--features", "fast,nope", &d]);
    assert_eq!(code, 2);
    assert!(err.contains("error[D0112]") && err.contains("`nope`"), "{err}");
    let f = temp_file("features_file.sisu", "def main():\n    print(1)\n");
    let (code, _, err) = run(&["run", "--features", "x", &f]);
    assert_eq!(code, 2);
    assert!(err.contains("error[D0112]"), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A benchmark's samples (decision 468): the median, the samples far from
/// it left out as outliers, and the 95% interval of the others' mean.
#[test]
fn benchmark_samples_give_a_median_an_interval_and_outliers() {
    let s = crate::cli::sample_stats(&[10.0, 11.0, 9.0, 10.0, 10.5, 9.5, 100.0]);
    assert_eq!(s.median, 10.0);
    assert_eq!(s.outliers, 1);
    assert!(s.low < 10.0 && s.high > 10.0 && s.high < 11.0, "{} {}", s.low, s.high);
    let same = crate::cli::sample_stats(&[5.0, 5.0, 5.0]);
    assert_eq!((same.median, same.low, same.high, same.outliers), (5.0, 5.0, 5.0, 0));
}
