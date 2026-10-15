use super::*;
use sisu_syntax::ast;

/// Compiles a program and checks it; the program must have no front-end
/// errors. Returns the checked program and the findings' identifiers.
fn run(src: &str) -> (Checked, Vec<&'static str>) {
    let mut map = SourceMap::new();
    let file = map.add("t.sisu", src);
    let parse = sisu_parser::parse(file, src);
    assert!(parse.diagnostics.is_empty(), "syntax errors");
    let m = ast::build::module(&parse.tree);
    let r = sisu_resolve::resolve(&mut map, file, &m);
    assert!(r.diagnostics.is_empty(), "resolve: {}", sisu_diag::render::render_ids(&map, &r.diagnostics));
    let (typed, diags) = sisu_typeck::check(&r.program, &map);
    assert!(diags.is_empty(), "types: {}", sisu_diag::render::render_ids(&map, &diags));
    let p = sisu_lower::lower(&r.program, &typed, &map);
    let c = check(&p, &Options::default());
    let problems = sir::validate(&c.program);
    assert!(problems.is_empty(), "invalid SIR after elaboration: {problems:?}\n{}", sir::write(&c.program));
    let text = sir::write(&c.program);
    let back = sir::read(&text).unwrap_or_else(|e| panic!("cannot read back: {e:?}\n{text}"));
    assert_eq!(sir::write(&back), text, "the elaborated SIR does not round-trip");
    let ids = c.findings.iter().map(|f| f.msg.id()).collect();
    (c, ids)
}

fn ok(src: &str) -> Checked {
    let (c, ids) = run(src);
    assert!(ids.is_empty(), "unexpected findings {ids:?} in\n{src}\n{:#?}", c.findings);
    c
}

fn rejects(src: &str, id: &str) {
    let (_, ids) = run(src);
    assert!(ids.contains(&id), "expected {id}, found {ids:?} in\n{src}");
}

fn drops_of(c: &Checked, f: &str) -> Vec<String> {
    let f = c.program.function(f).unwrap_or_else(|| panic!("no {f}"));
    let mut out = Vec::new();
    for b in &f.blocks {
        if b.cleanup {
            continue;
        }
        for s in &b.stmts {
            if let sir::StatementKind::Drop { .. } = &s.kind {
                out.push(sir::write::stmt(s));
            }
        }
    }
    out
}

const LOG: &str = "view struct Log:\n    name: str\n\nimpl Drop for Log:\n    def drop(mut self):\n        print(self.name)\n\ndef take(own l: Log):\n    pass\n\n";

#[test]
fn moves_and_uses() {
    ok(&format!("{LOG}def main():\n    let a = Log(name=\"a\")\n    take(a)\n"));
    rejects(
        &format!("{LOG}def main():\n    let a = Log(name=\"a\")\n    take(a)\n    take(a)\n"),
        "borrow.use-after-move",
    );
    rejects(
        &format!("{LOG}def main():\n    let a = Log(name=\"a\")\n    for i in range(2):\n        take(a)\n"),
        "borrow.use-after-move",
    );
    rejects("def main():\n    let x: String\n    print(x)\n", "type.uninitialized");
    rejects(
        "def f(c: bool):\n    let x: int\n    if c:\n        x = 1\n    x = 2\n    print(x)\n",
        "type.reassign-let",
    );
    ok("def main():\n    let x: int\n    if True:\n        x = 1\n    else:\n        x = 2\n    print(x)\n");
}

#[test]
fn a_drop_after_a_conditional_move_gets_a_flag() {
    let c = ok(&format!("{LOG}def f(c: bool):\n    let a = Log(name=\"a\")\n    if c:\n        take(a)\n"));
    let d = drops_of(&c, "f");
    assert!(d.iter().any(|s| s.contains(" if _")), "{d:?}\n{}", sir::write(&c.program));
}

#[test]
fn a_drop_of_a_moved_value_disappears() {
    let c = ok(&format!("{LOG}def f():\n    let a = Log(name=\"a\")\n    take(a)\n"));
    assert!(drops_of(&c, "f").is_empty(), "{}", sir::write(&c.program));
}

#[test]
fn a_partly_moved_value_drops_what_remains() {
    let src = format!(
        "{LOG}view struct Pair:\n    a: Log\n    b: Log\n\ndef f():\n    let p = Pair(a=Log(name=\"a\"), b=Log(name=\"b\"))\n    take(p.a)\n"
    );
    let c = ok(&src);
    let d = drops_of(&c, "f");
    assert_eq!(d.len(), 1, "{d:?}\n{}", sir::write(&c.program));
    assert!(d[0].contains(".1"), "{d:?}");
}

#[test]
fn borrows_are_non_lexical() {
    ok("def main():\n    var xs = [1, 2, 3]\n    let first = ref xs[0]\n    print(first)\n    xs.push(4)\n");
    rejects(
        "def main():\n    var xs = [1, 2, 3]\n    let first = ref xs[0]\n    xs.push(4)\n    print(first)\n",
        "borrow.mut-while-borrowed",
    );
    // Two-phase: the argument is read before the `mut` borrow is used.
    ok("def main():\n    var xs = [1, 2, 3]\n    xs.push(len(xs))\n    print(xs)\n");
}

#[test]
fn a_reference_cannot_outlive_its_referent() {
    rejects(
        "def main():\n    let r: ref int\n    if True:\n        let x = 5\n        r = ref x\n    print(r)\n",
        "borrow.does-not-live-long-enough",
    );
    rejects(
        "def f() -> ref int:\n    let x = 5\n    return ref x\ndef main():\n    print(f())\n",
        "borrow.return-local",
    );
    ok(
        "def head(xs: list[int]) -> ref int:\n    return ref xs[0]\ndef main():\n    let xs = [1]\n    print(head(xs))\n",
    );
}

#[test]
fn iterating_while_modifying_is_rejected() {
    rejects(
        "def main():\n    var xs = [1, 2]\n    for x in xs:\n        xs.push(x)\n",
        "borrow.mut-while-borrowed",
    );
}

#[test]
fn views_of_a_string_borrow_it() {
    rejects(
        "def main():\n    var s = String.new(\"a\")\n    let v: str = s\n    s.push(\"b\")\n    print(v)\n",
        "borrow.mut-while-borrowed",
    );
    ok("def main():\n    var s = String.new(\"a\")\n    let v: str = s\n    print(v)\n    s.push(\"b\")\n");
}

#[test]
fn tokens_view_their_source() {
    ok(
        "view struct Tok:\n    text: str\n\ndef toks(src: str) -> list[Tok]:\n    var out: list[Tok] = []\n    for p in src.split_whitespace():\n        out.push(Tok(text=p))\n    return out\n\ndef main():\n    let s = String.new(\"a b\")\n    let t = toks(s)\n    print(len(t))\n",
    );
}

#[test]
fn escapes_are_rejected() {
    // A closure capturing a local, stored in a plain struct.
    rejects(
        "struct H:\n    run: (int) -> int\n\ndef main():\n    let base = 1\n    let h = H(run=x -> x + base)\n    print(h.run(1))\n",
        "borrow.store-in-plain-type",
    );
    // A view of one argument stored in another, `mut`, argument.
    rejects(
        "def into(mut out: list[str], text: str):\n    out.push(text)\n\ndef main():\n    var w: list[str] = []\n    into(mut w, \"a\")\n    print(w)\n",
        "borrow.store-into-caller",
    );
    // Storing a parameter's own view through it is fine.
    ok(
        "def again(mut out: list[str]):\n    let first = out[0]\n    out.push(first)\n\ndef main():\n    var w = [\"a\"]\n    again(mut w)\n    print(w)\n",
    );
}

#[test]
fn a_text_view_from_a_list_of_views_borrows_the_text_only() {
    let src = "def longest(words: list[str]) -> str:\n    return words[0]\n\ndef main():\n    var text = String.new(\"a b\")\n    var words = [w for w in text.split_whitespace()]\n    let best = longest(words)\n    words.clear()\n    print(best)\n";
    let c = ok(src);
    let f = c.program.function("longest").unwrap();
    assert_eq!((f.from.clone(), f.inner.clone()), (vec![], vec![0]));
    // The text itself is still borrowed.
    rejects(&src.replace("    words.clear()\n", "    text.push(\"c\")\n"), "borrow.mut-while-borrowed");
}

#[test]
fn must_consume_values_are_consumed() {
    let r =
        "@must_consume\nstruct R:\n    n: int\n\n    def done(own self) -> int:\n        return self.n\n\n";
    ok(&format!("{r}def main():\n    let x = R(n=1)\n    print(x.done())\n"));
    rejects(&format!("{r}def main():\n    let x = R(n=1)\n    print(x.n)\n"), "borrow.must-consume");
    rejects(
        &format!(
            "{r}def main():\n    var xs = [R(n=1)]\n    xs.clear()\n    for x in own xs:\n        print(x.done())\n"
        ),
        "borrow.must-consume-intrinsic",
    );
    // Consuming iteration discharges a list; leaving the loop early does not.
    ok(&format!(
        "{r}def main():\n    let xs = [R(n=1), R(n=2)]\n    for x in own xs:\n        print(x.done())\n"
    ));
    rejects(
        &format!(
            "{r}def main():\n    let xs = [R(n=1), R(n=2)]\n    for x in own xs:\n        if x.n > 1:\n            print(x.done())\n            break\n        print(x.done())\n"
        ),
        "borrow.must-consume",
    );
    // Unwinding is exempt.
    ok(&format!(
        "{r}def f(own x: R) -> int:\n    if x.n < 0:\n        panic(\"negative\")\n    return x.done()\n\ndef main():\n    print(f(R(n=1)))\n"
    ));
}

#[test]
fn pure_functions_call_and_drop_only_pure_code() {
    rejects("pure def f():\n    print(1)\n", "type.impure-call");
    rejects(
        &format!("{LOG}pure def f() -> int:\n    let l = Log(name=\"a\")\n    return len(l.name)\n"),
        "type.impure-drop",
    );
    ok("pure def f(xs: list[int]) -> int:\n    var t = 0\n    for x in xs:\n        t += x\n    return t\n");
}

#[test]
fn a_weakened_checker_accepts_what_it_should_not() {
    // Fault injection (the monitor side is in sisu-driver's soundness
    // tests): each rule, switched off, lets its own mistakes through.
    let src =
        "def main():\n    var xs = [1, 2, 3]\n    let first = ref xs[0]\n    xs.push(4)\n    print(first)\n";
    let mut map = SourceMap::new();
    let file = map.add("t.sisu", src);
    let parse = sisu_parser::parse(file, src);
    let m = ast::build::module(&parse.tree);
    let r = sisu_resolve::resolve(&mut map, file, &m);
    let (typed, _) = sisu_typeck::check(&r.program, &map);
    let p = sisu_lower::lower(&r.program, &typed, &map);
    assert!(check(&p, &Options::default()).has_errors());
    assert!(!check(&p, &Options { disabled: vec![Rule::Conflicts], ..Default::default() }).has_errors());
}
