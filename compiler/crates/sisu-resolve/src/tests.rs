use super::*;
use sisu_diag::SourceMap;
use sisu_syntax::ast;

fn run(src: &str) -> (Resolved, SourceMap) {
    let mut map = SourceMap::new();
    let file = map.add("t.sisu", src);
    let parse = sisu_parser::parse(file, src);
    assert!(
        parse.diagnostics.is_empty(),
        "{:?}",
        parse.diagnostics.iter().map(|d| d.message()).collect::<Vec<_>>()
    );
    let m = ast::build::module(&parse.tree);
    (resolve(&mut map, file, &m), map)
}

fn codes(src: &str) -> Vec<&'static str> {
    run(src).0.diagnostics.iter().map(|d| d.code()).collect()
}

#[test]
fn std_modules_resolve_cleanly() {
    let (r, _) = run("import cli\nfrom std.math import PI\ndef main():\n    pass\n");
    let msgs: Vec<String> = r.diagnostics.iter().map(|d| format!("{} {}", d.code(), d.message())).collect();
    assert!(msgs.is_empty(), "{}", msgs.join("\n"));
    let p = &r.program;
    assert!(p.wk.option.is_some() && p.wk.result.is_some() && p.wk.list.is_some() && p.wk.system.is_some());
    assert!(p.main().is_some());
    assert!(p.method(TyKey::Str, "split_whitespace").is_some());
    let list = p.wk.list.unwrap();
    assert!(p.method(TyKey::Adt(list), "push").is_some());
    assert!(p.lookup(p.root, "PI").is_some());
}

#[test]
fn signatures_resolve_to_types() {
    let (r, _) = run(
        "error E:\n    A\nstruct P:\n    x: int\n    def len(self) -> int:\n        return self.x\ndef f(a: list[int], b: str?) -> int ! E:\n    return Ok(1)\n",
    );
    assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics.iter().map(|d| d.message()).collect::<Vec<_>>());
    let p = &r.program;
    let f = p.fn_info(p.lookup(p.root, "f").unwrap()).unwrap();
    assert_eq!(p.display(&f.params[0].ty), "list[int]");
    assert_eq!(p.display(&f.params[1].ty), "str?");
    assert_eq!(p.display(&f.ret), "int ! E");
    let pt = p.lookup(p.root, "P").unwrap();
    let len = p.method(TyKey::Adt(pt), "len").unwrap();
    assert_eq!(p.fn_info(len).unwrap().self_mode, Some(ast::Mode::Borrow));
}

#[test]
fn inferred_error_sets_and_unions() {
    let (r, _) = run(
        "error A:\n    X\nerror B:\n    Y\ndef f() -> int !:\n    return Ok(1)\ndef g() -> ! {A, B}:\n    return Ok(())\n",
    );
    assert!(r.diagnostics.is_empty());
    let p = &r.program;
    assert!(p.fn_info(p.lookup(p.root, "f").unwrap()).unwrap().err_inferred);
    assert_eq!(p.display(&p.fn_info(p.lookup(p.root, "g").unwrap()).unwrap().ret), "! {A, B}");
}

#[test]
fn resolution_errors() {
    assert_eq!(codes("def f(x: Nope):\n    pass\n"), vec!["E0201"]);
    assert_eq!(codes("def f():\n    pass\ndef f():\n    pass\n"), vec!["E0202"]);
    assert_eq!(codes("def f(x: list):\n    pass\n"), vec!["E0375"]);
    assert_eq!(codes("def f(x: dict[int]):\n    pass\n"), vec!["E0208"]);
    assert_eq!(codes("struct S:\n    x: int\ndef f() -> ! S:\n    pass\n"), vec!["E0212"]);
    assert_eq!(codes("import nothing.here\n"), vec!["E0220"]);
    assert_eq!(codes("def f[T](x: T):\n    pass\n"), vec!["E0210"]);
    assert_eq!(codes("def f(x: Shared[int]):\n    pass\n"), vec!["E0900"]);
    assert_eq!(codes("def f()\n"), vec!["E0224"]);
    assert_eq!(codes("struct S:\n    x: int\n    x: int\n"), vec!["E0205"]);
}

#[test]
fn drop_impls_register() {
    let (r, _) = run(
        "view struct Guard:\n    name: str\nimpl Drop for Guard:\n    def drop(mut self):\n        print(self.name)\n",
    );
    assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics.iter().map(|d| d.message()).collect::<Vec<_>>());
    let p = &r.program;
    let g = p.lookup(p.root, "Guard").unwrap();
    assert!(p.struct_info(g).unwrap().drop.is_some());
    assert_eq!(
        codes("struct G:\n    x: int\nimpl Display for G:\n    def f(self):\n        pass\n"),
        vec!["E0214"]
    );
}

#[test]
fn variants_are_indexed() {
    let (r, _) = run("enum Shape:\n    Circle(r: f64)\n    Empty\n");
    let p = &r.program;
    assert_eq!(p.variants.get("Circle").map(|v| v.len()), Some(1));
    assert!(p.variants.contains_key("Some") && p.variants.contains_key("NotFound"));
}
