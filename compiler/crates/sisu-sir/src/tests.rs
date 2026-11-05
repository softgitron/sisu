use super::*;

fn sample() -> Program {
    let l = |n| LocalId(n);
    Program {
        sources: vec!["t.sisu".into()],
        entry: Some(Entry { function: "main".into(), takes_system: false, returns_result: false }),
        exports: Vec::new(),
        runtime: None,
        types: vec![
            TypeDecl::Struct {
                name: "Point".into(),
                params: vec![],
                fields: vec![
                    FieldDecl { name: "x".into(), ty: Type::Float, default: Some("Point#x".into()) },
                    FieldDecl {
                        name: "tags".into(),
                        ty: Type::Named("list".into(), vec![Type::Str]),
                        default: None,
                    },
                ],
                drop: None,
                view: false,
                must_consume: false,
            },
            TypeDecl::Struct {
                name: "Token".into(),
                params: vec![],
                fields: vec![FieldDecl { name: "text".into(), ty: Type::Str, default: None }],
                drop: None,
                view: true,
                must_consume: true,
            },
            TypeDecl::Struct {
                name: "Stack".into(),
                params: vec!["T".into()],
                fields: vec![FieldDecl {
                    name: "items".into(),
                    ty: Type::Named("list".into(), vec![Type::Param(0)]),
                    default: None,
                }],
                drop: None,
                view: false,
                must_consume: false,
            },
            TypeDecl::Enum {
                name: "E".into(),
                params: vec![],
                variants: vec![
                    VariantDecl { name: "A".into(), fields: vec![] },
                    VariantDecl {
                        name: "B".into(),
                        fields: vec![(Some("at".into()), Type::Int), (None, Type::Char)],
                    },
                ],
                drop: None,
                error: true,
                open: true,
                must_consume: true,
            },
        ],
        intrinsics: vec![
            IntrinsicDecl {
                name: "print".into(),
                pure: false,
                from: vec![],
                inner: vec![],
                own: vec![],
                keeps: vec![],
            },
            IntrinsicDecl {
                name: "Iter.next".into(),
                pure: true,
                from: vec![],
                inner: vec![0],
                own: vec![],
                keeps: vec![0],
            },
            IntrinsicDecl {
                name: "zip".into(),
                pure: true,
                from: vec![0, 1],
                inner: vec![],
                own: vec![],
                keeps: vec![],
            },
            IntrinsicDecl {
                name: "list.push".into(),
                pure: true,
                from: vec![],
                inner: vec![],
                own: vec![1],
                keeps: vec![0, 1],
            },
        ],
        functions: vec![
            Function {
                name: "Point#x".into(),
                type_params: vec![],
                kind: FnKind::Value,
                param_count: 0,
                pure: true,
                from: vec![],
                inner: vec![],
                own: vec![],
                loc: None,
                locals: vec![LocalDecl::new(Type::Float, None)],
                blocks: vec![BasicBlock {
                    stmts: vec![Statement {
                        kind: StatementKind::Assign(
                            Place::local(l(0)),
                            Rvalue::Use(Operand::Const(Const::Float(1.0))),
                        ),
                        loc: None,
                    }],
                    term: Terminator::Return,
                    unwind: None,
                    cleanup: false,
                    loc: None,
                }],
                source: 0,
            },
            Function {
                name: "show".into(),
                type_params: vec!["T".into()],
                kind: FnKind::Normal,
                param_count: 1,
                pure: false,
                from: vec![],
                inner: vec![],
                own: vec![0],
                loc: None,
                locals: vec![
                    LocalDecl::new(Type::Unit, None),
                    LocalDecl::new(Type::Ref(Box::new(Type::Param(0)), false), Some("x".into())),
                    LocalDecl::new(Type::Dyn("Display".into(), vec![]), None),
                    LocalDecl::new(
                        Type::Assoc(Box::new(Type::Param(0)), "Container".into(), vec![], "Item".into()),
                        None,
                    ),
                    LocalDecl::new(Type::Fn(vec![], Box::new(Type::Unit), false), None),
                ],
                blocks: vec![BasicBlock {
                    stmts: vec![
                        Statement {
                            kind: StatementKind::Call {
                                dest: Place::local(l(0)),
                                callee: Callee::Trait("Display.fmt".into(), vec![Type::Param(0)]),
                                args: vec![Operand::Copy(Place::local(l(1)))],
                            },
                            loc: None,
                        },
                        Statement {
                            kind: StatementKind::Call {
                                dest: Place::local(l(0)),
                                callee: Callee::Fn("show".into(), vec![Type::Int]),
                                args: vec![Operand::Const(Const::Int(1))],
                            },
                            loc: None,
                        },
                        Statement {
                            kind: StatementKind::Assign(
                                Place::local(l(2)),
                                Rvalue::Aggregate(
                                    Aggregate::Dyn(Type::Int),
                                    vec![Operand::Const(Const::Int(2))],
                                ),
                            ),
                            loc: None,
                        },
                        Statement {
                            kind: StatementKind::Assign(
                                Place::local(l(4)),
                                Rvalue::Closure("show".into(), vec![Type::Param(0)], vec![]),
                            ),
                            loc: None,
                        },
                    ],
                    term: Terminator::Return,
                    unwind: None,
                    cleanup: false,
                    loc: None,
                }],
                source: 0,
            },
            Function {
                name: "main".into(),
                type_params: vec![],
                kind: FnKind::Normal,
                param_count: 0,
                pure: false,
                from: vec![],
                inner: vec![],
                own: vec![],
                loc: Some(Loc { line: 1, col: 5, end_line: 1, end_col: 9 }),
                locals: vec![
                    LocalDecl::new(Type::Unit, None),
                    LocalDecl {
                        ty: Type::Str,
                        name: Some("s \"q\"".into()),
                        loc: Some(Loc { line: 2, col: 9, end_line: 2, end_col: 10 }),
                        once: true,
                    },
                    LocalDecl::new(
                        Type::Ref(
                            Box::new(Type::Named(
                                "Result".into(),
                                vec![Type::Int, Type::ErrSet(vec!["E".into()])],
                            )),
                            true,
                        ),
                        None,
                    ),
                    LocalDecl::new(
                        Type::Fn(vec![Type::Int], Box::new(Type::Tuple(vec![Type::Bool])), true),
                        None,
                    ),
                    LocalDecl::new(Type::Int, None),
                    LocalDecl::new(Type::Fn(vec![], Box::new(Type::Unit), false), Some("f".into())),
                ],
                blocks: vec![
                    BasicBlock {
                        stmts: vec![
                            Statement {
                                kind: StatementKind::Assign(
                                    Place::local(l(1)),
                                    Rvalue::Use(Operand::Const(Const::Str("a\nb".into()))),
                                ),
                                loc: Some(Loc { line: 3, col: 5, end_line: 3, end_col: 12 }),
                            },
                            Statement {
                                kind: StatementKind::Call {
                                    dest: Place::local(l(0)),
                                    callee: Callee::Intrinsic("print".into(), vec![Type::Str]),
                                    args: vec![Operand::Copy(Place::local(l(1)))],
                                },
                                loc: Some(Loc::point(3, 5)),
                            },
                            Statement { kind: StatementKind::StorageDead(l(5)), loc: None },
                            Statement {
                                kind: StatementKind::Assign(
                                    Place::local(l(4)),
                                    Rvalue::Discriminant(Place { local: l(2), proj: vec![Proj::Deref] }),
                                ),
                                loc: None,
                            },
                            Statement {
                                kind: StatementKind::Assign(
                                    Place { local: l(4), proj: vec![] },
                                    Rvalue::Binary(
                                        BinOp::Add,
                                        Operand::Copy(Place {
                                            local: l(2),
                                            proj: vec![Proj::Deref, Proj::Downcast(0), Proj::Field(0)],
                                        }),
                                        Operand::Const(Const::Int(-3)),
                                    ),
                                ),
                                loc: None,
                            },
                            Statement {
                                kind: StatementKind::Drop { place: Place::local(l(1)), flag: Some(l(4)) },
                                loc: None,
                            },
                        ],
                        term: Terminator::Switch {
                            value: Operand::Copy(Place::local(l(4))),
                            cases: vec![(0, BlockId(1)), (1, BlockId(1))],
                            default: BlockId(2),
                        },
                        unwind: Some(BlockId(2)),
                        cleanup: false,
                        loc: Some(Loc::point(4, 1)),
                    },
                    BasicBlock {
                        stmts: vec![],
                        term: Terminator::Return,
                        unwind: None,
                        cleanup: false,
                        loc: None,
                    },
                    BasicBlock {
                        stmts: vec![Statement {
                            kind: StatementKind::Drop { place: Place::local(l(1)), flag: None },
                            loc: None,
                        }],
                        term: Terminator::Resume,
                        unwind: None,
                        cleanup: true,
                        loc: None,
                    },
                ],
                source: 0,
            },
        ],
        tests: vec![("a test".into(), "main".into())],
        test_tags: vec![vec!["slow".into(), "net io".into()]],
        benches: vec![("a bench".into(), "main".into())],
        trait_methods: vec![IntrinsicDecl {
            name: "Display.fmt".into(),
            pure: true,
            from: vec![],
            inner: vec![],
            own: vec![],
            keeps: vec![],
        }],
        impls: vec![
            ImplDecl {
                params: 0,
                trait_name: "Display".into(),
                trait_args: vec![],
                self_ty: Type::Int,
                methods: vec![("fmt".into(), ImplTarget::Intrinsic("structural.display".into()))],
                assoc: vec![],
            },
            ImplDecl {
                params: 1,
                trait_name: "Add".into(),
                trait_args: vec![Type::Param(0)],
                self_ty: Type::Named("Stack".into(), vec![Type::Param(0)]),
                methods: vec![("add".into(), ImplTarget::Fn("show".into()))],
                assoc: vec![("Output".into(), Type::Param(0))],
            },
        ],
    }
}

#[test]
fn write_read_round_trip() {
    let p = sample();
    let text = write(&p);
    let back = read(&text).unwrap_or_else(|e| panic!("line {}: {}\n{text}", e.line, e.message));
    assert_eq!(back, p);
    assert_eq!(write(&back), text);
    assert!(validate(&p).is_empty(), "{:?}", validate(&p));
}

#[test]
fn text_form_is_readable() {
    let text = write(&sample());
    assert!(text.starts_with("sir 0\nsource 0 \"t.sisu\"\nentry main\n"));
    // A test's tags, and a benchmark (decision 285).
    assert!(text.contains("\ntest \"a test\" main tags \"slow\" \"net io\"\nbench \"a bench\" main\n"));
    assert!(text.contains("struct Point { x: f64 = Point#x, tags: list[str] }"));
    assert!(text.contains("view struct Token must_consume { text: str }"));
    assert!(text.contains("error E open must_consume { A, B(at: int, char) }"));
    assert!(text.contains(
        "\nintrinsic print\nintrinsic Iter.next pure inner 0 keeps 0\nintrinsic zip pure from 0 1\n"
    ));
    assert!(text.contains("fn Point#x(0) value pure source 0 {"));
    assert!(text.contains("    let _1: str \"s \\\"q\\\"\" once @2:9-2:10\n"));
    assert!(text.contains("    let _3: pure fn(int) -> (bool,)\n"));
    assert!(text.contains("    _1 = const \"a\\nb\" @3:5-3:12\n"));
    assert!(text.contains("    _0 = call intrinsic print[str](copy _1) @3:5\n"));
    assert!(text.contains("    dead _5\n"));
    assert!(text.contains("_4 = add copy _2.*.as(0).0, const -3"));
    assert!(text.contains("  bb0 (unwind bb2):"));
    assert!(text.contains("  bb2 (cleanup):"));
    assert!(text.contains("switch copy _4 [0: bb1, 1: bb1, else: bb2] @4:1"));
    assert!(text.contains("struct Stack[T] { items: list[$0] }"));
    assert!(text.contains("\nmethod Display.fmt pure\n"));
    assert!(text.contains("impl Display for int { fmt: intrinsic structural.display }"));
    assert!(text.contains("impl[1] Add[$0] for Stack[$0] { add: show, type Output = $0 }"));
    assert!(text.contains("fn show[T](1) source 0 own 0 {"));
    assert!(text.contains("    let _2: dyn Display\n"));
    assert!(text.contains("    let _3: <$0 as Container>.Item\n"));
    assert!(text.contains("_0 = call trait Display.fmt[$0](copy _1)"));
    assert!(text.contains("_0 = call fn show[int](const 1)"));
    assert!(text.contains("_2 = dyn int(const 2)"));
    assert!(text.contains("_4 = closure show[$0]()"));
}

#[test]
fn floats_round_trip_exactly() {
    for f in [0.1, 1.0, -2.5e-300, 1e300, 123456789.0, f64::MIN_POSITIVE] {
        let s = write::float(f);
        assert!(s.contains('.') || s.contains('e'), "{s}");
        let back: f64 = s.parse().unwrap();
        assert_eq!(back.to_bits(), f.to_bits());
    }
}

#[test]
fn reader_reports_lines() {
    let e = read("sir 0\nbogus line\n").unwrap_err();
    assert_eq!(e.line, 2);
    assert!(read("sir 9\n").is_err());
    assert!(read("fn f(0) source 0 {\n").is_err());
}

#[test]
fn function_headers_name_their_from_arguments() {
    let mut p = sample();
    p.functions[2].param_count = 2;
    p.functions[2].from = vec![0, 1];
    let text = write(&p);
    assert!(text.contains("fn main(2) source 0 from 0 1 @1:5-1:9 {"), "{text}");
    assert_eq!(read(&text).map(|b| b.functions[2].from.clone()), Ok(vec![0, 1]));
    p.functions[2].from = vec![2];
    assert!(validate(&p).iter().any(|m| m.contains("`from` names argument 2")));
    // Arguments the result borrows through their values only.
    p.functions[2].from = vec![0];
    p.functions[2].inner = vec![1];
    let text = write(&p);
    assert!(text.contains("fn main(2) source 0 from 0 inner 1 @1:5-1:9 {"), "{text}");
    assert_eq!(read(&text).map(|b| b.functions[2].inner.clone()), Ok(vec![1]));
    p.functions[2].inner = vec![3];
    assert!(validate(&p).iter().any(|m| m.contains("`inner` names argument 3")));
}

#[test]
fn validator_catches_broken_graphs() {
    let mut p = sample();
    p.functions[2].blocks[1].term = Terminator::Goto(BlockId(9));
    p.functions[2].blocks[0].unwind = Some(BlockId(1));
    let problems = validate(&p);
    assert!(problems.iter().any(|m| m.contains("missing bb9")));
    assert!(problems.iter().any(|m| m.contains("not a cleanup block")));
}

/// One function's blocks as `sisu sir` prints them, threaded.
fn threaded(body: &str) -> String {
    let text = format!("sir 0\nsource 0 \"t.sisu\"\nentry main\n\nfn main(0) source 0 {{\n{body}}}\n");
    let mut p = read(&text).unwrap();
    simplify::program(&mut p);
    assert_eq!(validate(&p), Vec::<String>::new());
    let out = write(&p);
    let start = out.find("  bb0").unwrap();
    out[start..out.len() - 2].to_string()
}

#[test]
fn jumps_are_threaded_and_blocks_taken() {
    let body = "    let _0: ()\n    let _1: bool\n  bb0:\n    _1 = const true\n    goto bb1\n  bb1:\n    goto bb2\n  bb2:\n    switch copy _1 [0: bb3, else: bb4] @1:1\n  bb3:\n    goto bb5\n  bb4:\n    goto bb5\n  bb5:\n    _0 = const ()\n    return\n";
    // The empty blocks are gone, the switch moves up to the block that
    // jumped to it, and the block both arms reach stays a block.
    assert_eq!(
        threaded(body),
        "  bb0:\n    _1 = const true\n    switch copy _1 [0: bb1, else: bb1] @1:1\n  bb1:\n    _0 = const ()\n    return\n"
    );
}

#[test]
fn threading_keeps_cycles_drops_and_unwinding() {
    // A loop of empty blocks stays a loop, of one block.
    let spin = "    let _0: ()\n  bb0:\n    goto bb1\n  bb1:\n    goto bb2\n  bb2:\n    goto bb1\n";
    assert_eq!(threaded(spin), "  bb0:\n    goto bb1\n  bb1:\n    goto bb1\n");
    // A drop ends its block; statements under another unwinding edge stay
    // in their own block.
    let body = "    let _0: ()\n    let _1: String\n  bb0:\n    _1 = call intrinsic String.new()\n    goto bb1\n  bb1 (unwind bb4):\n    _0 = const ()\n    goto bb2\n  bb2:\n    drop _1\n    goto bb3\n  bb3:\n    dead _1\n    return\n  bb4 (cleanup):\n    drop _1\n    goto bb5\n  bb5 (cleanup):\n    resume\n";
    assert_eq!(
        threaded(body),
        "  bb0:\n    _1 = call intrinsic String.new()\n    goto bb1\n  bb1 (unwind bb4):\n    _0 = const ()\n    goto bb2\n  bb2:\n    drop _1\n    goto bb3\n  bb3:\n    dead _1\n    return\n  bb4 (cleanup):\n    drop _1\n    resume\n"
    );
}

/// A method of a trait's impl is named in traces by the trait's method and
/// the type, wherever the type and the trait are declared (decision 504).
#[test]
fn an_impl_method_is_named_by_its_trait_and_type() {
    let mut p = sample();
    p.types.push(TypeDecl::Struct {
        name: "kiuas.meter.Dial".into(),
        params: vec![],
        fields: vec![],
        drop: None,
        view: false,
        must_consume: false,
    });
    let shown = |n: &str| display_fn_name(n, &p);
    assert_eq!(shown("kiuas.meter.Dial.kiuas.meter.Meter.read"), "Meter.read for kiuas.meter.Dial");
    assert_eq!(shown("Point.kiuas.meter.Meter.read"), "Meter.read for Point");
    assert_eq!(shown("Point.Display.fmt"), "Display.fmt for Point");
    assert_eq!(shown("Point.Display.fmt#closure0"), "closure in Display.fmt for Point");
    // An inherent method, a module's function and an unknown type keep
    // their names.
    assert_eq!(shown("Point.norm"), "Point.norm");
    assert_eq!(shown("kiuas.meter.Dial.read"), "kiuas.meter.Dial.read");
    assert_eq!(shown("kiuas.meter.helper"), "kiuas.meter.helper");
    assert_eq!(shown("Nowhere.Display.fmt"), "Nowhere.Display.fmt");
}

#[test]
fn the_runtime_profile_is_read_after_the_entry_and_written_back() {
    // Stage 1's line for `[package] runtime = "single"` (its R122;
    // decision 723), before the `export` lines of decision 683.
    let text = "sir 0\nsource 0 \"t.sisu\"\nentry main\nruntime single\nexport \"tick\" tick\n\n\
                fn main(0) source 0 {\nbb0:\n    return\n}\n\nfn tick(0) source 0 {\nbb0:\n    return\n}\n";
    let p = read(text).expect("it reads");
    assert_eq!(p.runtime.as_deref(), Some("single"));
    assert!(write(&p).contains("entry main\nruntime single\nexport \"tick\" tick\n"), "{}", write(&p));
    assert!(read(&text.replace("runtime single", "runtime multi")).is_err());
}
