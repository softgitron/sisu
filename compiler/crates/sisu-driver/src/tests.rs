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
    assert_eq!(db.counters, Counters { parse: 1, check: 1, lower: 1 });
    // The same text again: nothing reruns.
    db.set_file("a.sisu", "def main():\n    print(1)\n");
    assert!(db.sir("a.sisu").is_some());
    assert_eq!(db.counters, Counters { parse: 1, check: 1, lower: 1 });
    // A change reruns every step after it.
    db.set_file("a.sisu", "def main():\n    print(2)\n");
    assert!(db.sir("a.sisu").is_some());
    assert_eq!(db.counters, Counters { parse: 2, check: 2, lower: 2 });
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

fn cli(args: &[&str]) -> (i32, TestHost) {
    let mut host = TestHost::default();
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
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
fn parse_print_is_lossless() {
    let text = "# comment\ndef  main( ):   # odd spacing\n    print( 1 )\n\n\n";
    let f = temp_file("lossless.sisu", text);
    let (_, host) = cli(&["parse", "--print", &f]);
    assert_eq!(host.out, text);
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
