use super::*;
use sisu_syntax::ast;

fn lower_src(src: &str) -> sir::Program {
    let mut map = SourceMap::new();
    let file = map.add("t.sisu", src);
    let parse = sisu_parser::parse(file, src);
    assert!(parse.diagnostics.is_empty(), "syntax errors");
    let m = ast::build::module(&parse.tree);
    let r = sisu_resolve::resolve(&mut map, file, &m);
    assert!(
        r.diagnostics.is_empty(),
        "resolve errors: {}",
        sisu_diag::render::render_ids(&map, &r.diagnostics)
    );
    let (typed, diags) = sisu_typeck::check(&r.program, &map);
    assert!(diags.is_empty(), "type errors: {}", sisu_diag::render::render_ids(&map, &diags));
    lower(&r.program, &typed, &map)
}

fn valid(p: &sir::Program) {
    let problems = sir::validate(p);
    assert!(problems.is_empty(), "invalid SIR:\n{}\n\n{}", problems.join("\n"), sir::write(p));
    let text = sir::write(p);
    let back = sir::read(&text).unwrap_or_else(|e| panic!("cannot read back: {e:?}\n{text}"));
    assert_eq!(sir::write(&back), text, "text form does not round-trip");
}

fn function<'p>(p: &'p sir::Program, name: &str) -> &'p sir::Function {
    p.function(name).unwrap_or_else(|| panic!("no function {name} in\n{}", sir::write(p)))
}

/// The locals each `drop` statement of a function names, in block order.
fn drops(f: &sir::Function) -> Vec<Option<String>> {
    let mut out = Vec::new();
    for b in &f.blocks {
        if b.cleanup {
            continue;
        }
        for s in &b.stmts {
            if let sir::StatementKind::Drop { place, .. } = &s.kind {
                out.push(f.locals[place.local.0 as usize].name.clone());
            }
        }
    }
    out
}

#[test]
fn hello_lowers_to_valid_sir() {
    let p = lower_src(include_str!("../../../../proposal/examples/hello.sisu"));
    valid(&p);
    assert!(p.entry.is_some());
}

#[test]
fn wordcount_lowers_to_valid_sir() {
    let p = lower_src(include_str!("../../../../proposal/examples/wordcount.sisu"));
    valid(&p);
    let e = p.entry.as_ref().map(|e| (e.takes_system, e.returns_result));
    assert_eq!(e, Some((true, true)));
}

#[test]
fn drops_run_in_reverse_declaration_order() {
    let p =
        lower_src("def f():\n    let a = String.new()\n    let b = String.new()\n    let c = String.new()\n");
    valid(&p);
    let f = function(&p, "f");
    assert_eq!(drops(f), vec![Some("c".into()), Some("b".into()), Some("a".into())]);
    // Every block after the first `let` unwinds to a cleanup chain.
    assert!(f.blocks.iter().any(|b| b.cleanup));
}

#[test]
fn a_moved_local_gets_a_drop_flag() {
    let p = lower_src(
        "def take(own s: String):\n    pass\ndef f(c: bool):\n    let s = String.new()\n    if c:\n        take(s)\n",
    );
    valid(&p);
    let f = function(&p, "f");
    let flagged = f
        .blocks
        .iter()
        .flat_map(|b| &b.stmts)
        .any(|s| matches!(&s.kind, sir::StatementKind::Drop { flag: Some(_), .. }));
    assert!(flagged, "{}", sir::write(&p));
}

#[test]
fn a_local_never_moved_has_no_flag() {
    let p = lower_src("def f():\n    let s = String.new()\n    print(s)\n");
    valid(&p);
    let f = function(&p, "f");
    let flagged = f
        .blocks
        .iter()
        .flat_map(|b| &b.stmts)
        .any(|s| matches!(&s.kind, sir::StatementKind::Drop { flag: Some(_), .. }));
    assert!(!flagged, "{}", sir::write(&p));
}

#[test]
fn closures_become_functions_with_an_environment() {
    let p = lower_src("def f() -> int:\n    let n = 2\n    let g = x -> x * n\n    return g(3)\n");
    valid(&p);
    let c = function(&p, "f#closure0");
    assert_eq!(c.kind, sir::FnKind::Closure);
    assert!(matches!(&c.locals[1].ty, sir::Type::Tuple(es) if es.len() == 1));
}

#[test]
fn try_returns_early_and_records_the_trace() {
    let p = lower_src(
        "error E:\n    Bad\ndef a() -> int ! E:\n    return Err(Bad)\ndef b() -> int ! E:\n    let x = a()?\n    return Ok(x + 1)\n",
    );
    valid(&p);
    let text = sir::write(&p);
    assert!(text.contains("error.trace_push"), "{text}");
}

#[test]
fn defer_runs_at_scope_exit() {
    let p = lower_src("def f():\n    defer:\n        print(\"one\")\n    print(\"two\")\n");
    valid(&p);
    let text = sir::write(&p);
    let two = text.find("\"two\"").unwrap_or(usize::MAX);
    let one = text.rfind("\"one\"").unwrap_or(0);
    assert!(one > two, "{text}");
}
