//! Interpreter tests over hand-written SIR, so they hold for the SIR
//! semantics alone, whatever front end produced the program.

use super::*;
use crate::format::{float_repr, strftime};

fn program(text: &str) -> sir::Program {
    match sir::read(text) {
        Ok(p) => {
            let problems = sir::validate(&p);
            assert!(problems.is_empty(), "{problems:?}");
            p
        }
        Err(e) => panic!("bad SIR at line {}: {}", e.line, e.message),
    }
}

/// Runs `main` on the calling thread (not the big-stack thread), so a
/// test also shows that Sisu calls do not recurse on the host stack.
fn run(text: &str) -> (i32, String, String) {
    let p = program(text);
    let mut host = TestHost::default();
    let code = Interp::new(&p, &mut host, &Options::default()).run_main();
    (code, host.out, host.err)
}

const NOISY: &str = r#"sir 0
source 0 "t.sisu"
entry main

struct Noisy drop Noisy.drop { id: int }

fn Noisy.drop(1) drop source 0 {
    let _0: ()
    let _1: ref mut Noisy "self"
    let _2: String
    let _3: ref mut String
    let _4: ()
    let _5: ref String
  bb0:
    _2 = call intrinsic fmt.new()
    _3 = ref mut _2
    _4 = call intrinsic fmt.text(copy _3, const "drop ")
    _4 = call intrinsic fmt.display(copy _3, copy _1.*.0, const "")
    _5 = ref _2
    _4 = call intrinsic print(copy _5)
    _0 = const ()
    return
}

fn Noisy#drop(1) glue source 0 own 0 {
    let _0: ()
    let _1: Noisy "self"
    let _2: ref mut Noisy
  bb0:
    _2 = ref mut _1
    _0 = call fn Noisy.drop(move _2)
    goto bb1
  bb1:
    return
}
"#;

#[test]
fn drops_run_in_reverse_order_at_scope_exit() {
    let text = format!(
        "{NOISY}
fn main(0) source 0 {{
    let _0: ()
    let _1: Noisy \"a\"
    let _2: Noisy \"b\"
  bb0:
    _1 = struct Noisy(const 1)
    goto bb1
  bb1:
    _2 = struct Noisy(const 2)
    goto bb2
  bb2:
    drop _2
    goto bb3
  bb3:
    drop _1
    goto bb4
  bb4:
    _0 = const ()
    return
}}
"
    );
    let (code, out, _) = run(&text);
    assert_eq!(code, exit::OK);
    assert_eq!(out, "drop 2\ndrop 1\n");
}

#[test]
fn a_panic_runs_the_cleanup_blocks_then_reports() {
    let text = format!(
        "{NOISY}
fn main(0) source 0 {{
    let _0: ()
    let _1: Noisy \"a\"
    let _2: Never
  bb0:
    _1 = struct Noisy(const 7)
    goto bb1
  bb1 (unwind bb2):
    _2 = call intrinsic panic(const \"stop\") @4:5
    unreachable
  bb2 (cleanup):
    drop _1
    goto bb3
  bb3 (cleanup):
    resume
}}
"
    );
    let (code, out, err) = run(&text);
    assert_eq!(code, exit::PANIC);
    assert_eq!(out, "drop 7\n");
    assert_eq!(err, "panic: stop\n  at main (t.sisu:4:5)\n");
}

#[test]
fn a_drop_flag_skips_a_moved_local() {
    let text = format!(
        "{NOISY}
fn main(0) source 0 {{
    let _0: ()
    let _1: Noisy \"a\"
    let _2: bool \"flag\"
    let _3: Noisy
  bb0:
    _1 = struct Noisy(const 1)
    _2 = const true
    goto bb1
  bb1:
    _3 = move _1
    _2 = const false
    goto bb2
  bb2:
    drop _3
    goto bb3
  bb3:
    drop _1 if _2
    goto bb4
  bb4:
    _0 = const ()
    return
}}
"
    );
    let (_, out, _) = run(&text);
    assert_eq!(out, "drop 1\n");
}

const COUNT_DOWN: &str = r#"sir 0
source 0 "t.sisu"
entry main

fn down(1) source 0 {
    let _0: int
    let _1: int "n"
    let _2: bool
    let _3: int
    let _4: int
  bb0:
    _2 = eq copy _1, const 0
    switch copy _2 [0: bb2, else: bb1]
  bb1:
    _0 = const 0
    return
  bb2:
    _3 = sub copy _1, const 1
    _4 = call fn down(copy _3)
    _0 = add copy _4, const 1
    return
}
"#;

#[test]
fn deep_recursion_does_not_use_the_host_stack() {
    // 50 000 nested calls on a test thread's small stack: the run loop is
    // a trampoline.
    let text = format!(
        "{COUNT_DOWN}
fn main(0) source 0 {{
    let _0: ()
    let _1: int
    let _2: ()
  bb0:
    _1 = call fn down(const 50000)
    _2 = call intrinsic print(copy _1)
    _0 = const ()
    return
}}
"
    );
    let (code, out, _) = run(&text);
    assert_eq!(code, exit::OK);
    assert_eq!(out, "50000\n");
}

#[test]
fn unbounded_recursion_is_a_stack_overflow_panic() {
    let text = format!(
        "{COUNT_DOWN}
fn main(0) source 0 {{
    let _0: ()
    let _1: int
  bb0:
    _1 = call fn down(const -1)
    _0 = const ()
    return
}}
"
    );
    let (code, _, err) = run(&text);
    assert_eq!(code, exit::PANIC);
    // The overflow names the function that did not fit (decision 234),
    // and the trace of 100 000 frames shows its ends.
    assert!(
        err.starts_with(
            "panic: stack overflow in `down`: the calls nest deeper than the stack holds\n  at down (t.sisu)\n"
        ),
        "{err}"
    );
    let lines: Vec<&str> = err.lines().collect();
    assert_eq!(lines.len(), 2 + sisu_diag::trace::INNER + sisu_diag::trace::OUTER, "{err}");
    assert_eq!(lines[1 + sisu_diag::trace::INNER], "  ... (frames left out)");
    assert_eq!(lines.last(), Some(&"  at main (t.sisu)"));
    const { assert!(MAX_DEPTH > sisu_diag::trace::INNER + sisu_diag::trace::OUTER) };
}

#[test]
fn the_step_limit_aborts() {
    let p = program(
        r#"sir 0
source 0 "t.sisu"
entry main

fn main(0) source 0 {
    let _0: ()
    let _1: int
  bb0:
    _1 = const 1
    goto bb0
}
"#,
    );
    let mut host = TestHost::default();
    let code = Interp::new(&p, &mut host, &Options { fuel: Some(1000), ..Options::default() }).run_main();
    assert_eq!(code, exit::ABORT);
    assert!(host.err.contains("1000 statements"), "{}", host.err);
}

#[test]
fn integer_arithmetic_traps_and_floors() {
    use sisu_sir::BinOp::*;
    assert_eq!(ops::int_op(FloorDiv, 7, 2).ok(), Some(3));
    assert_eq!(ops::int_op(FloorDiv, -7, 2).ok(), Some(-4));
    assert_eq!(ops::int_op(Mod, -7, 2).ok(), Some(1));
    assert_eq!(ops::int_op(Mod, 7, -2).ok(), Some(-1));
    assert_eq!(ops::int_op(Pow, 2, 10).ok(), Some(1024));
    assert_eq!(ops::int_op(WrapAdd, i64::MAX, 1).ok(), Some(i64::MIN));
    assert_eq!(ops::int_op(SatAdd, i64::MAX, 1).ok(), Some(i64::MAX));
    for (op, a, b) in [
        (Add, i64::MAX, 1),
        (Mul, i64::MIN, -1),
        (FloorDiv, 1, 0),
        (Mod, 1, 0),
        (Pow, 2, 64),
        (Pow, 2, -1),
        (Shl, 1, 64),
    ] {
        assert!(matches!(ops::int_op(op, a, b), Err(Unwind::Panic(_))), "{op:?} {a} {b}");
    }
    assert_eq!(ops::float_op(Mod, -7.0, 2.0), 1.0);
}

#[test]
fn floats_print_shortest_with_a_point() {
    assert_eq!(float_repr(3.0), "3.0");
    assert_eq!(float_repr(0.1), "0.1");
    assert_eq!(float_repr(1e100), "1e100");
    assert_eq!(float_repr(f64::NAN), "NaN");
}

#[test]
fn civil_time_formats() {
    // 2026-10-13T12:34:56Z.
    let t = 1_791_894_896 * 1_000_000_000;
    assert_eq!(strftime(t, "%Y-%m-%dT%H:%M:%SZ"), "2026-10-13T12:34:56Z");
    assert_eq!(strftime(t, "%H:%M"), "12:34");
    assert_eq!(strftime(0, "%F %T"), "1970-01-01 00:00:00");
    assert_eq!(crate::format::datetime_text(t + 500_000_000), "2026-10-13T12:34:56.5Z");
    assert_eq!(crate::format::datetime_text(-1), "1969-12-31T23:59:59.999999999Z");
}

/// The host's part of a format spec: digits and float text. Specs
/// themselves are applied in Sisu (`Spec` in std/prelude.sisu; the
/// conformance cases `run/interpolation` and `run/format_specs`).
#[test]
fn format_digits_come_from_the_host() {
    use crate::format::{digits, float_digits};
    assert_eq!(digits(255, 16, false), "ff");
    assert_eq!(digits(255, 16, true), "FF");
    assert_eq!(digits(-8, 8, false), "10");
    assert_eq!(digits(i64::MIN, 10, false), "9223372036854775808");
    assert_eq!(digits(5, 2, false), "101");
    assert_eq!(float_digits(1.23456, None, Some(2)), "1.23");
    assert_eq!(float_digits(1.23456, Some('f'), None), "1.234560");
    assert_eq!(float_digits(1500.0, Some('e'), Some(2)), "1.50e+03");
    assert_eq!(float_digits(1500.0, Some('E'), Some(2)), "1.50E+03");
    assert_eq!(float_digits(0.25, Some('%'), Some(0)), "25%");
    assert_eq!(float_digits(f64::INFINITY, Some('f'), None), "inf");
    assert_eq!(float_digits(0.1, None, None), "0.1");
    let p = program("sir 0\nsource 0 \"t.sisu\"\n");
    let mut host = TestHost::default();
    let i = Interp::new(&p, &mut host, &Options::default());
    assert_eq!(i.debug_string(&Value::str("a\"b")), "\"a\\\"b\"");
}

#[test]
fn dicts_keep_insertion_order() {
    use crate::value::{Dict, Key};
    let mut d = Dict::default();
    for (k, v) in [("b", 1), ("a", 2), ("c", 3)] {
        d.insert(Key::Text(k.into()), Value::str(k), Value::Int(v));
    }
    d.insert(Key::Text("a".into()), Value::str("a"), Value::Int(20));
    d.remove(&Key::Text("b".into()));
    let keys: Vec<String> = d.entries.iter().map(|(k, _)| format!("{k:?}")).collect();
    assert_eq!(keys.len(), 2);
    assert_eq!(d.position(&Key::Text("c".into())), Some(1));
    assert!(matches!(d.entries[0].1, Value::Int(20)));
}

/// A module's fixture (stage 1's R219; decision 1202): the tests that name
/// it share one value, made before the first of them and dropped, by its
/// glue, after the last; each receives a `ref` to it, and a test that
/// names no fixture runs alone.
#[test]
fn a_modules_fixture_is_made_once_for_its_tests() {
    let reader = |name: &str| {
        format!(
            "fn {name}(1) test source 0 {{
    let _0: ()
    let _1: ref Noisy \"n\"
    let _2: String
    let _3: ref mut String
    let _4: ()
    let _5: ref String
  bb0:
    _2 = call intrinsic fmt.new()
    _3 = ref mut _2
    _4 = call intrinsic fmt.text(copy _3, const \"seen \")
    _4 = call intrinsic fmt.display(copy _3, copy _1.*.0, const \"\")
    _5 = ref _2
    _4 = call intrinsic print(copy _5)
    _0 = const ()
    return
}}
"
        )
    };
    let text = format!(
        "{NOISY}
fn fresh(0) source 0 {{
    let _0: Noisy
    let _1: ()
  bb0:
    _1 = call intrinsic print(const \"made\")
    _0 = struct Noisy(const 7)
    return
}}

{}
{}
fn alone(0) test source 0 {{
    let _0: ()
  bb0:
    _0 = const ()
    return
}}
",
        reader("t#0"),
        reader("t#1")
    )
    .replacen(
        "entry main\n",
        "test \"first\" t#0 fixture n fresh module 0\ntest \"alone\" alone\ntest \"second\" t#1 fixture n fresh module 0\n",
        1,
    );
    let p = program(&text);
    assert_eq!(p.test_fixtures[0][0].module, Some(0));
    assert_eq!(sir::read(&sir::write(&p)).map(|q| q.test_fixtures).ok(), Some(p.test_fixtures.clone()));
    let mut host = TestHost::default();
    let results = run_tests_report_at(&p, &mut host, &Options::default(), &[0, 1, 2]);
    let outs: Vec<(&str, &str)> = results.iter().map(|(n, _, r)| (n.as_str(), r.out.as_str())).collect();
    assert_eq!(outs, vec![("first", "made\nseen 7\n"), ("alone", ""), ("second", "seen 7\ndrop 7\n")]);
    assert!(results.iter().all(|(_, r, _)| matches!(r, TestResult::Passed)), "{results:?}");
}

/// A module's fixture of a type without glue that holds values with a
/// `Drop` (decision 1244): the list's elements are dropped after the last
/// test that reads it, first to last, as a `drop` of the list drops them.
#[test]
fn a_modules_fixture_of_a_list_drops_its_elements() {
    let text = format!(
        "{NOISY}
fn fresh(0) source 0 {{
    let _0: list[Noisy]
    let _1: Noisy
    let _2: Noisy
  bb0:
    _1 = struct Noisy(const 7)
    _2 = struct Noisy(const 8)
    _0 = list(move _1, move _2)
    return
}}

fn t#0(1) test source 0 {{
    let _0: ()
    let _1: ref list[Noisy] \"n\"
    let _2: ()
  bb0:
    _2 = call intrinsic print(const \"read\")
    _0 = const ()
    return
}}
"
    )
    .replacen("entry main\n", "test \"only\" t#0 fixture n fresh module 0\n", 1);
    let p = program(&text);
    let mut host = TestHost::default();
    let results = run_tests_report_at(&p, &mut host, &Options::default(), &[0]);
    let outs: Vec<(&str, &str)> = results.iter().map(|(n, _, r)| (n.as_str(), r.out.as_str())).collect();
    assert_eq!(outs, vec![("only", "read\ndrop 7\ndrop 8\n")]);
    assert!(results.iter().all(|(_, r, _)| matches!(r, TestResult::Passed)), "{results:?}");
}
