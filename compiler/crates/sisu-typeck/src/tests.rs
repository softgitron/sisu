use super::*;
use sisu_diag::SourceMap;
use sisu_syntax::ast;

fn run(src: &str) -> (Program, Typed, Vec<Diagnostic>, SourceMap) {
    let mut map = SourceMap::new();
    let file = map.add("t.sisu", src);
    let parse = sisu_parser::parse(file, src);
    let m = ast::build::module(&parse.tree);
    let r = sisu_resolve::resolve(&mut map, file, &m);
    let (typed, mut diags) = check(&r.program, &map);
    let mut all = parse.diagnostics;
    all.extend(r.diagnostics);
    all.append(&mut diags);
    sisu_diag::diagnostic::sort(&mut all);
    (r.program, typed, all, map)
}

fn codes(src: &str) -> Vec<&'static str> {
    run(src).2.iter().map(|d| d.code()).collect()
}

fn messages(src: &str) -> String {
    let (_, _, diags, map) = run(src);
    sisu_diag::render::render_ids(&map, &diags)
}

fn clean(src: &str) {
    let m = messages(src);
    assert!(m.is_empty(), "unexpected diagnostics:\n{m}");
}

#[test]
fn hello_checks() {
    clean(include_str!("../../../../proposal/examples/hello.sisu"));
}

#[test]
fn wordcount_checks() {
    clean(include_str!("../../../../proposal/examples/wordcount.sisu"));
}

#[test]
fn core_constructs_check() {
    clean(
        "struct Point:\n    x: f64\n    y: f64\n    def length(self) -> f64:\n        return (self.x * self.x + self.y * self.y).sqrt()\n\
         enum Shape:\n    Circle(r: f64)\n    Rect(w: f64, h: f64)\n    Empty\n\
         def area(s: Shape) -> f64:\n    match s:\n        case Circle(r):\n            return 3.0 * r * r\n        case Rect(w, h) if w == h:\n            return w * w\n        case Rect(w, h):\n            return w * h\n        case Empty:\n            return 0.0\n\
         def main():\n    var total = 0\n    for i in range(10):\n        total += i\n    let xs = [1, 2, 3]\n    let ys = [x * 2 for x in xs if x > 1]\n    let d = {\"a\": 1}\n    let sign = if total > 0: 1 elif total < 0: -1 else: 0\n    print(\"{total} {sign} {len(ys)} {d[\"a\"]}\")\n    let p = Point(x=3.0, y=4.0)\n    print(p.length())\n    print(area(Circle(r=1.0)))\n",
    );
}

#[test]
fn errors_and_options() {
    clean(
        "error ParseError:\n    UnexpectedEnd\n    BadDigit(at: int, found: char)\n\
         def parse_int(text: str) -> int ! ParseError:\n    if text.is_empty():\n        return Err(UnexpectedEnd)\n    var n = 0\n    for i, c in enumerate(text.chars()):\n        let d = c.to_digit() else:\n            return Err(BadDigit(at=i, found=c))\n        n = n * 10 + d\n    return Ok(n)\n\
         def twice(text: str) -> int !:\n    let n = parse_int(text)?\n    return Ok(n * 2)\n\
         def main():\n    let a = parse_int(\"42\") else: 0\n    let b = twice(\"21\") else as e: 1\n    let c: int? = None\n    print(\"{a} {b} {c ?? 7}\")\n    match parse_int(\"x\"):\n        case Ok(n):\n            print(n)\n        case Err(BadDigit(at, found)):\n            print(\"bad {found} at {at}\")\n        case Err(UnexpectedEnd):\n            print(\"empty\")\n",
    );
}

#[test]
fn inferred_error_set_is_the_union() {
    let (p, typed, diags, _) = run(
        "error A:\n    X\nerror B:\n    Y\ndef a() -> int ! A:\n    return Err(X)\ndef b() -> int ! B:\n    return Err(Y)\ndef both(f: bool) -> int !:\n    if f:\n        return Ok(a()?)\n    return Ok(b()?)\n",
    );
    assert!(diags.is_empty(), "{:?}", diags.iter().map(|d| d.message()).collect::<Vec<_>>());
    let both = p.lookup(p.root, "both").unwrap();
    let set = typed.inferred.get(&both).unwrap();
    assert_eq!(set.len(), 2);
}

#[test]
fn type_errors_have_codes() {
    assert_eq!(codes("def f() -> int:\n    return 1 / 2\n"), vec!["E0301"]);
    assert_eq!(codes("def f():\n    if 1:\n        pass\n"), vec!["E0302"]);
    assert_eq!(codes("def f() -> int:\n    return \"x\"\n"), vec!["E0300"]);
    assert_eq!(codes("def f():\n    let x = 1\n    x = 2\n"), vec!["E0312"]);
    assert_eq!(codes("def f() -> int:\n    let x = 1\n"), vec!["E0315"]);
    assert_eq!(codes("def f():\n    print(nope)\n"), vec!["E0200"]);
    // The arms all return, so only the missing case is reported.
    assert_eq!(
        codes(
            "enum E:\n    A\n    B\ndef f(e: E) -> int:\n    match e:\n        case A:\n            return 1\n"
        ),
        vec!["E0314"]
    );
    assert_eq!(codes("def g() -> int?:\n    return None\ndef f():\n    g()\n"), vec!["E0318"]);
    assert_eq!(codes("def f(xs: list[int]) -> int:\n    return xs[-1]\n"), vec!["E0344"]);
    assert_eq!(
        codes(
            "error A:\n    X\nerror B:\n    Y\ndef a() -> int ! A:\n    return Err(X)\ndef f() -> int ! B:\n    return Ok(a()?)\n"
        ),
        vec!["E0321"]
    );
    assert_eq!(codes("def f(x: int):\n    let y = x.nope()\n"), vec!["E0304"]);
    assert_eq!(codes("def f():\n    break\n"), vec!["E0322"]);
    assert_eq!(
        codes("def g(mut xs: list[int]):\n    xs.push(1)\ndef f():\n    var xs = [1]\n    g(xs)\n"),
        vec!["E0339"]
    );
    assert_eq!(codes("def f():\n    let xs = [1]\n    xs.push(2)\n"), vec!["E0341"]);
    assert_eq!(codes("pure def f():\n    print(1)\n"), vec!["E0333"]);
    assert_eq!(codes("def f(x: int) -> int:\n    return x + 1.5\n"), vec!["E0373"]);
    assert_eq!(codes("def f() -> f64:\n    return 7.0 // 2.0\n"), vec!["E0372"]);
}

#[test]
fn poisoned_types_do_not_cascade() {
    // One unknown name, used three times: one report.
    assert_eq!(codes("def f():\n    let x = nope\n    let y = x + 1\n    print(y.foo())\n"), vec!["E0200"]);
}

#[test]
fn broken_code_still_has_a_typed_tree() {
    let (_, typed, diags, _) =
        run("def f(x: int) -> int:\n    let y = x +\n    return y\ndef g():\n    pass\n");
    assert!(!diags.is_empty());
    assert!(typed.bodies.iter().any(|b| !b.block.stmts.is_empty()));
}

#[test]
fn moves_out_of_borrows_are_reported() {
    assert_eq!(
        codes("struct U:\n    name: String\ndef f(u: U) -> String:\n    return u.name\n"),
        vec!["E0338"]
    );
    clean("struct U:\n    name: String\ndef f(u: U) -> String:\n    return u.name.copy()\n");
}

#[test]
fn it_closures_and_do_blocks() {
    clean(
        "def main():\n    let xs = [1, 2, 3]\n    let n = xs.iter().filter(it > 1).count()\n    let total = xs.iter().fold(0) do acc, x:\n        return acc + x\n    var ys = [3, 1, 2]\n    ys.sort_by((a, b) -> b.cmp(a))\n    print(\"{n} {total} {ys}\")\n",
    );
}

#[test]
fn list_patterns_split_by_length() {
    clean(
        "def f(xs: list[int]) -> int:\n    match xs:\n        case []:\n            return 0\n        case [x]:\n            return x\n        case [a, b, *rest]:\n            return a + b\n",
    );
    assert_eq!(
        codes(
            "def f(xs: list[int]) -> int:\n    match xs:\n        case [x, *rest]:\n            return x\n"
        ),
        vec!["E0314"]
    );
}

#[test]
fn a_misspelt_name_suggests_the_close_one() {
    let (_, _, diags, _) = run("def main():\n    let count = 1\n    print(cuont)\n");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].fixes.first().and_then(|f| f.edits.first()).map(|e| e.text.as_str()), Some("count"));
}

#[test]
fn unit_patterns_and_holes() {
    clean(
        "error E:\n    X\ndef f() -> ! E:\n    return Ok(())\ndef main():\n    match f():\n        case Ok(()):\n            pass\n        case Err(_):\n            pass\n",
    );
    assert_eq!(codes("def f() -> int:\n    return todo(\"later\")\n"), vec!["W0001"]);
}

#[test]
fn returning_an_error_outside_the_set() {
    assert_eq!(
        codes("error A:\n    X\nerror B:\n    Y\ndef f() -> int ! A:\n    return Err(Y)\n"),
        vec!["E0321"]
    );
}

#[test]
fn a_broken_header_does_not_cascade() {
    // The header lost `x` and the result type; neither the body nor the
    // call reports anything more.
    for header in ["def f(x int) -> int:", "def f x: int) -> int:", "def f(x: in-t) -> int:"] {
        let src = format!("{header}\n    let y = x + 1\n    return y\ndef main():\n    print(f(1))\n");
        let all = codes(&src);
        assert_eq!(all.len(), 1, "{header}: {all:?}");
    }
}
