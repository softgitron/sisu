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
    assert!(codes("def f[T](x: T):\n    pass\n").is_empty());
    assert_eq!(codes("def f(x: Array[int]):\n    pass\n"), vec!["E0208"]);
    assert!(codes("def f(x: f32) -> f64:\n    return x\n").is_empty());
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
        vec!["E0233", "E0232"]
    );
}

#[test]
fn variants_are_indexed() {
    let (r, _) = run("enum Shape:\n    Circle(r: f64)\n    Empty\n");
    let p = &r.program;
    assert_eq!(p.variants.get("Circle").map(|v| v.len()), Some(1));
    assert!(p.variants.contains_key("Some") && p.variants.contains_key("NotFound"));
}

/// The source the derives of the root module unfolded to.
fn derived_text(src: &str) -> String {
    let (r, map) = run(src);
    let msgs: Vec<String> = r.diagnostics.iter().map(|d| format!("{} {}", d.code(), d.message())).collect();
    assert!(msgs.is_empty(), "{}", msgs.join("\n"));
    map.files().iter().find(|f| f.name == "<derive>/main.sisu").map(|f| f.text.clone()).unwrap_or_default()
}

#[test]
fn derives_unfold_their_templates_at_compile_time() {
    let text = derived_text("@derive(Eq, Hash)\nstruct P:\n    x: int\n    y: String\n");
    assert_eq!(
        text,
        "
impl Eq for P:
    pure def eq(self, other: Self) -> bool:
        if self.x != other.x:
            return False
        if self.y != other.y:
            return False
        return True

impl Hash for P:
    pure def hash(self, mut h: Hasher):
        self.x.hash(mut h)
        self.y.hash(mut h)

impl Debug for P:
    pure def fmt_debug(self, mut f: Formatter):
        f.write(\"P(\")
        f.depth += 1
        f.write(\"x=\")
        self.x.fmt_debug(mut f)
        f.write(\", \")
        f.write(\"y=\")
        self.y.fmt_debug(mut f)
        f.depth -= 1
        f.write(\")\")
"
    );
}

#[test]
fn enum_derives_make_an_arm_per_variant() {
    let text = derived_text("@derive(Eq)\nenum Shape[T]:\n    Dot\n    Circle(r: T)\n    Pair(T, T)\n");
    assert_eq!(
        text,
        "
impl[T: Eq] Eq for Shape[T]:
    pure def eq(self, other: Self) -> bool:
        match self:
            case Dot:
                match other:
                    case Dot:
                        return True
                    case _:
                        return False
            case Circle(__self_0):
                match other:
                    case Circle(__other_0):
                        if __self_0 != __other_0:
                            return False
                        return True
                    case _:
                        return False
            case Pair(__self_0, __self_1):
                match other:
                    case Pair(__other_0, __other_1):
                        if __self_0 != __other_0:
                            return False
                        if __self_1 != __other_1:
                            return False
                        return True
                    case _:
                        return False

impl[T: Debug] Debug for Shape[T]:
    pure def fmt_debug(self, mut f: Formatter):
        match self:
            case Dot:
                f.write(\"Dot\")
            case Circle(__self_0):
                f.write(\"Circle(\")
                f.depth += 1
                f.write(\"r=\")
                __self_0.fmt_debug(mut f)
                f.depth -= 1
                f.write(\")\")
            case Pair(__self_0, __self_1):
                f.write(\"Pair(\")
                f.depth += 1
                __self_0.fmt_debug(mut f)
                f.write(\", \")
                __self_1.fmt_debug(mut f)
                f.depth -= 1
                f.write(\")\")
"
    );
}

#[test]
fn a_template_that_cannot_unfold_is_reported_where_it_stops() {
    let src = "trait Show:\n    def show(self) -> String\n\n    comptime def derive(T: type) -> impl Show for T:\n        return impl Show for T:\n            def show(self) -> String:\n                comptime for f in reflect.fields_of(T):\n                    print(self.[f.name])\n                return \"\"\n\n@derive(Show)\nstruct P:\n    x: int\n";
    assert_eq!(codes(src), vec!["E0608"]);
    assert_eq!(
        codes("trait Show:\n    comptime def derive(x: int) -> int:\n        return 1\n"),
        vec!["E0607"]
    );
}
