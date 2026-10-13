use super::*;
use sisu_diag::FileId;
use sisu_syntax::SyntaxKind;
use sisu_syntax::SyntaxKind::*;

fn p(src: &str) -> Parse {
    let parse = parse(FileId(0), src);
    assert_eq!(parse.tree.to_source(), src, "lossless round trip");
    parse
}

fn codes(src: &str) -> Vec<&'static str> {
    p(src).diagnostics.iter().map(|d| d.code()).collect()
}

fn kinds_in(src: &str) -> Vec<SyntaxKind> {
    let parse = p(src);
    parse.tree.preorder(parse.tree.root()).into_iter().map(|n| parse.tree.kind(n)).collect()
}

const EXAMPLES: &[(&str, &str)] = &[
    ("hello", include_str!("../../../../proposal/examples/hello.sisu")),
    ("wordcount", include_str!("../../../../proposal/examples/wordcount.sisu")),
    ("errors", include_str!("../../../../proposal/examples/errors.sisu")),
    ("ownership", include_str!("../../../../proposal/examples/ownership.sisu")),
    ("traits", include_str!("../../../../proposal/examples/traits.sisu")),
    ("concurrency", include_str!("../../../../proposal/examples/concurrency.sisu")),
    ("capabilities", include_str!("../../../../proposal/examples/capabilities.sisu")),
    ("contracts", include_str!("../../../../proposal/examples/contracts.sisu")),
    ("comptime", include_str!("../../../../proposal/examples/comptime.sisu")),
    ("testing", include_str!("../../../../proposal/examples/testing.sisu")),
    ("ffi", include_str!("../../../../proposal/examples/ffi.sisu")),
];

#[test]
fn examples_parse_without_syntax_errors() {
    // `select`, `comptime`, `yield` and the others parse even where later
    // stages reject them.
    for (name, src) in EXAMPLES {
        let parse = p(src);
        let msgs: Vec<String> = parse
            .diagnostics
            .iter()
            .map(|d| format!("{name}: {} {} at {:?}", d.code(), d.message(), d.span.range))
            .collect();
        assert!(msgs.is_empty(), "{}", msgs.join("\n"));
    }
}

#[test]
fn leading_comments_belong_to_the_root() {
    let parse = p("## doc\n# c\ndef f():\n    pass\n");
    let root = parse.tree.root();
    assert_eq!(parse.tree.child_nodes(root).map(|n| parse.tree.kind(n)).collect::<Vec<_>>(), vec![FN_DEF]);
}

#[test]
fn hello_tree_shape() {
    let k = kinds_in("def main():\n    print(\"Hei, maailma!\")\n");
    assert_eq!(
        k,
        vec![
            SOURCE_FILE,
            FN_DEF,
            PARAM_LIST,
            BLOCK,
            EXPR_STMT,
            CALL_EXPR,
            NAME_REF,
            ARG_LIST,
            ARG,
            STRING_EXPR
        ]
    );
}

#[test]
fn three_syntax_errors_three_reports() {
    // The Phase 0 exit criterion: a file with three syntax errors reports
    // all three, each on its own line.
    let src =
        "def main():\n    let x = (1 +\n    let y = 2\n    if y > 1\n        print(y)\n    print(x y)\n";
    let parse = p(src);
    let lines: Vec<u32> = parse
        .diagnostics
        .iter()
        .map(|d| {
            let f = sisu_diag::SourceFile::new(FileId(0), "t".into(), src.into());
            f.line_col(d.span.range.start).line
        })
        .collect();
    assert_eq!(
        parse.diagnostics.len(),
        3,
        "{:?}",
        parse.diagnostics.iter().map(|d| d.message()).collect::<Vec<_>>()
    );
    assert_eq!(lines, vec![2, 4, 6]);
}

#[test]
fn missing_colon_and_block() {
    assert_eq!(codes("def main()\n    pass\n"), vec!["E0100"]);
    let parse = p("def main():\npass\n");
    assert_eq!(parse.diagnostics[0].code(), "E0101");
    assert_eq!(parse.diagnostics[0].param("keyword"), Some("def"));
    assert_eq!(parse.diagnostics[0].param("line"), Some("1"));
    assert_eq!(codes("def f():\n    if x: return 1\n"), vec!["E0102"]);
}

#[test]
fn operator_mixing_rules() {
    assert_eq!(codes("def f():\n    x = a and b or c\n"), vec!["E0108"]);
    assert_eq!(codes("def f():\n    x = a or b and c\n"), vec!["E0108"]);
    assert!(codes("def f():\n    x = (a and b) or c\n").is_empty());
    assert_eq!(codes("def f():\n    x = a & b | c\n"), vec!["E0109"]);
    assert_eq!(codes("def f():\n    x = a & b + c\n"), vec!["E0110"]);
    assert!(codes("def f():\n    x = c & 1 == 1\n").is_empty());
    assert_eq!(codes("def f():\n    x = a << b + c\n"), vec!["E0111"]);
    assert_eq!(codes("def f():\n    x = -2 ** 2\n"), vec!["E0112"]);
    assert!(codes("def f():\n    x = (-2) ** 2\n").is_empty());
    assert_eq!(codes("def f():\n    x = a < b > c\n"), vec!["E0113"]);
    assert!(codes("def f():\n    x = a < b <= c\n").is_empty());
    assert_eq!(codes("def f():\n    x = a != b != c\n"), vec!["E0119"]);
}

#[test]
fn assignment_in_condition() {
    assert_eq!(codes("def f():\n    if x = 1:\n        pass\n"), vec!["E0114"]);
}

#[test]
fn closures_and_trailing_blocks() {
    let k = kinds_in("def f():\n    xs.sort_by((a, b) -> b.1.cmp(a.1))\n");
    assert!(k.contains(&CLOSURE_EXPR));
    let k = kinds_in("def f():\n    let t = xs.fold(0) do acc, x:\n        return acc + x\n    print(t)\n");
    assert!(k.contains(&DO_BLOCK));
    assert_eq!(k.iter().filter(|&&k| k == LET_STMT).count(), 1);
    assert!(
        codes("def f():\n    let t = xs.fold(0) do acc, x:\n        return acc + x\n    print(t)\n")
            .is_empty()
    );
}

#[test]
fn match_forms() {
    let src = "def f(s: Shape) -> f64:\n    let a = match s:\n        case Circle(r): r * r\n        case Empty: 0.0\n    match s:\n        case Rect(w, h) if w == h:\n            return w\n        case _:\n            pass\n    return a\n";
    assert!(codes(src).is_empty(), "{:?}", codes(src));
    let k = kinds_in(src);
    assert_eq!(k.iter().filter(|&&k| k == MATCH_EXPR).count(), 2);
    assert!(k.contains(&MATCH_GUARD));
}

#[test]
fn let_else_forms() {
    let src = "def f():\n    let x = g() else: 0\n    let y = g() else as e: h(e)\n    let Some(z) = o else:\n        return\n    let w = g() else: return None\n";
    assert!(codes(src).is_empty(), "{:?}", codes(src));
    assert_eq!(kinds_in(src).iter().filter(|&&k| k == LET_ELSE).count(), 4);
}

#[test]
fn conditions_chain_with_commas() {
    let src =
        "def f():\n    if let Some(u) = users.get(id), let Some(t) = u.team, t.active:\n        notify(t)\n";
    assert!(codes(src).is_empty());
    assert_eq!(kinds_in(src).iter().filter(|&&k| k == LET_COND).count(), 2);
}

#[test]
fn literals_and_comprehensions() {
    let src = "def f():\n    let a = [x * x for x in xs if x > 0]\n    let b = {k: v for k, v in pairs}\n    let c = {1, 2}\n    let d = {}\n    let e = (1, \"a\")\n    let f = ()\n";
    assert!(codes(src).is_empty());
    let k = kinds_in(src);
    assert_eq!(k.iter().filter(|&&k| k == COMPREHENSION).count(), 2);
    assert!(k.contains(&SET_EXPR) && k.contains(&DICT_EXPR) && k.contains(&TUPLE_EXPR));
}

#[test]
fn types() {
    let src = "def f(a: ref str?, b: list[int], c: (int, str) -> bool) -> int ! {IoError, ParseError}:\n    pass\ndef g() -> !:\n    pass\ndef h() -> ! IoError:\n    pass\n";
    assert!(codes(src).is_empty(), "{:?}", codes(src));
    let k = kinds_in(src);
    assert!(
        k.contains(&OPTION_TYPE)
            && k.contains(&REF_TYPE)
            && k.contains(&FN_TYPE)
            && k.contains(&ERROR_SET_TYPE)
    );
    assert_eq!(k.iter().filter(|&&k| k == RESULT_TYPE).count(), 3);
}

#[test]
fn top_level_statement_is_reported_but_kept() {
    let parse = p("print(1)\ndef main():\n    pass\n");
    assert_eq!(parse.diagnostics.iter().map(|d| d.code()).collect::<Vec<_>>(), vec!["E0107"]);
    let root = parse.tree.root();
    assert_eq!(parse.tree.child_nodes(root).count(), 2);
}

#[test]
fn unexpected_indent_keeps_structure() {
    let src = "def f():\n    x = 1\n        y = 2\n    z = 3\n";
    assert_eq!(codes(src), vec!["E0125"]);
}

#[test]
fn stable_ids_survive_body_edits() {
    let a = "def f():\n    x = 1\n\ndef g():\n    y = 2\n";
    let b = "def f():\n    x = 1\n    x = 2\n    x = 3\n\ndef g():\n    y = 2\n";
    let pa = parse(FileId(0), a);
    let pb = parse(FileId(0), b);
    let ids = |parse: &Parse| -> Vec<sisu_syntax::StableId> {
        let t = &parse.tree;
        t.preorder(t.root())
            .into_iter()
            .filter(|&n| t.kind(n) == FN_DEF || t.kind(n) == EXPR_STMT || t.kind(n) == ASSIGN_STMT)
            .map(|n| t.stable_id(n))
            .collect()
    };
    let ia = ids(&pa);
    let ib = ids(&pb);
    // `g` and its body keep their identities although `f` grew.
    assert_eq!(ia.last(), ib.last());
    assert!(ib.contains(&ia[0]));
    assert!(ia[ia.len() - 2].item[0].starts_with("FN_DEF:g#0"));
}

/// The fuzz property: never panics, always lossless, no diagnostic storm.
#[test]
fn fuzz_never_panics_and_round_trips() {
    let mut state = 0x9e37_79b9_7f4a_7c15u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let vocabulary = [
        "def", "f", "(", ")", ":", "\n", "    ", "if", "x", "else", "elif", "return", "let", "=", "1",
        "\"s{x}\"", "[", "]", "{", "}", ",", "+", "and", "or", "match", "case", "->", "do", "?", ".",
        "struct", "enum", "..", "@a", "#c\n", "\t", "'c'", "not", "in", "for", "while", "**", "-", "|", "_",
        "Some", "(a, b)", "\"", "{", "pass\n",
    ];
    for _ in 0..3000 {
        let mut s = String::new();
        let len = next() % 60;
        for _ in 0..len {
            s.push_str(vocabulary[(next() % vocabulary.len() as u64) as usize]);
            if next() % 3 == 0 {
                s.push(' ');
            }
        }
        let parse = parse(FileId(0), &s);
        assert_eq!(parse.tree.to_source(), s);
        let lines = s.lines().count() + 1;
        assert!(
            parse.diagnostics.len() <= 2 * lines + 2,
            "storm: {} diagnostics for {lines} lines in {s:?}",
            parse.diagnostics.len()
        );
    }
    // Mutations of the example programs.
    for (_, src) in EXAMPLES {
        let bytes: Vec<char> = src.chars().collect();
        for _ in 0..40 {
            let mut m = bytes.clone();
            for _ in 0..3 {
                if m.is_empty() {
                    break;
                }
                let i = (next() % m.len() as u64) as usize;
                match next() % 3 {
                    0 => {
                        m.remove(i);
                    }
                    1 => m.insert(i, [':', '(', '"', ' ', '\n', '{'][(next() % 6) as usize]),
                    _ => {
                        let j = (next() % m.len() as u64) as usize;
                        m.swap(i, j);
                    }
                }
            }
            let s: String = m.into_iter().collect();
            let parse = parse(FileId(0), &s);
            assert_eq!(parse.tree.to_source(), s);
        }
    }
}

mod ast_tests {
    use super::*;
    use sisu_syntax::ast::{self, ExprKind, ItemKind, PatKind, StmtKind, StrPart, TypeKind};

    fn module(src: &str) -> ast::Module {
        ast::build::module(&p(src).tree)
    }

    #[test]
    fn ast_of_every_example_and_fuzz_input() {
        for (_, src) in EXAMPLES {
            let m = module(src);
            assert!(!m.items.is_empty());
        }
        let mut state = 7u64;
        for i in 0..500 {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let src = EXAMPLES[i % EXAMPLES.len()].1;
            let cut = (state >> 33) as usize % src.len().max(1);
            let s: String = src.chars().take(cut).collect();
            let _ = ast::build::module(&parse(FileId(0), &s).tree);
        }
    }

    #[test]
    fn function_shape() {
        let m = module(
            "pure def add(a: int, mut b: list[int] = [], own c: str?) -> int ! IoError:\n    return a\n",
        );
        let ItemKind::Fn(f) = &m.items[0].kind else { panic!() };
        assert!(f.is_pure);
        assert_eq!(f.name.text, "add");
        assert_eq!(f.params.len(), 3);
        assert_eq!(f.params[1].mode, ast::Mode::Mut);
        assert!(matches!(&f.ret.as_ref().unwrap().kind, TypeKind::Result { ok: Some(_), err: Some(_) }));
        let body = f.body.as_ref().unwrap();
        assert!(matches!(body.stmts[0].kind, StmtKind::Return(Some(_))));
    }

    #[test]
    fn interpolation_parts() {
        let m = module("def f():\n    print(\"a {x} b {n:>8} {y=}\")\n");
        let ItemKind::Fn(f) = &m.items[0].kind else { panic!() };
        let StmtKind::Expr(e) = &f.body.as_ref().unwrap().stmts[0].kind else { panic!() };
        let ExprKind::Call { args, .. } = &e.kind else { panic!() };
        let ExprKind::Str(parts) = &args[0].value.kind else { panic!() };
        assert_eq!(parts.len(), 6);
        let StrPart::Interp { spec, .. } = &parts[3] else { panic!() };
        assert_eq!(spec.as_deref(), Some(">8"));
        let StrPart::Interp { debug, .. } = &parts[5] else { panic!() };
        assert_eq!(debug.as_deref(), Some("y"));
    }

    #[test]
    fn struct_enum_error_shapes() {
        let m = module(
            "struct P:\n    x: int = 1\n    def len(self) -> int:\n        return 0\nenum E:\n    A\n    B(int, s: str)\nerror Er:\n    X\n    ..\n",
        );
        let ItemKind::Struct(s) = &m.items[0].kind else { panic!() };
        assert_eq!(s.fields.len(), 1);
        assert!(s.fields[0].default.is_some());
        assert_eq!(s.methods.len(), 1);
        let ItemKind::Enum(e) = &m.items[1].kind else { panic!() };
        assert_eq!(e.variants[1].fields.len(), 2);
        assert_eq!(e.variants[1].fields[1].name.as_ref().unwrap().text, "s");
        let ItemKind::ErrorSet(er) = &m.items[2].kind else { panic!() };
        assert!(er.open);
    }

    #[test]
    fn patterns_and_match() {
        let m = module(
            "def f(x: E):\n    match x:\n        case Some(Rect(w, h)) if w == h:\n            pass\n        case P(x=0, ..):\n            pass\n        case 1 | 2:\n            pass\n        case [a, *rest]:\n            pass\n        case _:\n            pass\n",
        );
        let ItemKind::Fn(f) = &m.items[0].kind else { panic!() };
        let StmtKind::Expr(e) = &f.body.as_ref().unwrap().stmts[0].kind else { panic!() };
        let ExprKind::Match { arms, .. } = &e.kind else { panic!() };
        assert_eq!(arms.len(), 5);
        assert!(arms[0].guard.is_some());
        assert!(matches!(arms[2].pat.kind, PatKind::Or(_)));
        assert!(matches!(arms[3].pat.kind, PatKind::List(_)));
        assert!(matches!(arms[4].pat.kind, PatKind::Wildcard));
    }

    #[test]
    fn broken_code_still_has_an_ast() {
        let m = module("def f(:\n    let = \n    x = (1 +\n");
        assert!(!m.items.is_empty());
    }
}

#[test]
fn an_unterminated_string_is_the_only_report() {
    // Its line also has an unclosed `(`; the string is the cause.
    assert_eq!(codes("def main():\n    print(\"no end)\n    print(1)\n"), vec!["E0003"]);
}

#[test]
fn an_unclosed_bracket_is_reported_once() {
    assert_eq!(codes("def main():\n    let xs = [1, 2\n    print(xs)\n"), vec!["E0009"]);
    // A closer missing in the middle of a later line is a second mistake.
    assert_eq!(codes("def main():\n    let xs = [1, 2\n    let y = (xs b)\n"), vec!["E0009", "E0100"]);
}

#[test]
fn a_block_header_closes_its_open_bracket() {
    // The `(` is closed at the `:` because an indented block follows.
    assert_eq!(codes("def f(x: int -> int:\n    let y = x\n    return y\n"), vec!["E0009"]);
    // A colon inside brackets that is not a block header is left alone.
    assert_eq!(codes("def f():\n    let d = {\n        \"a\":\n            1,\n    }\n"), Vec::<&str>::new());
}

#[test]
fn recovery_always_makes_progress() {
    // Found by fuzzing: a dedent inside `select` left the parser at an
    // INDENT that the `case` loop never consumed.
    let src = "def f():\n    return task.scope do s:\n        select:\n            case _ = clock.after(5):\n           **     s.cancel()\n                return Err(Timeout)\n";
    let parse = p(src);
    assert_eq!(parse.tree.to_source(), src);
}
