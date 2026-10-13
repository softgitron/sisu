use super::*;

fn sample() -> Program {
    let l = |n| LocalId(n);
    Program {
        sources: vec!["t.sisu".into()],
        entry: Some(Entry { function: "main".into(), takes_system: false, returns_result: false }),
        types: vec![
            TypeDecl::Struct {
                name: "Point".into(),
                fields: vec![
                    FieldDecl { name: "x".into(), ty: Type::Float, default: Some("Point#x".into()) },
                    FieldDecl {
                        name: "tags".into(),
                        ty: Type::Named("list".into(), vec![Type::Str]),
                        default: None,
                    },
                ],
                drop: None,
            },
            TypeDecl::Enum {
                name: "E".into(),
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
            },
        ],
        functions: vec![
            Function {
                name: "Point#x".into(),
                kind: FnKind::Value,
                param_count: 0,
                locals: vec![LocalDecl { ty: Type::Float, name: None }],
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
                name: "main".into(),
                kind: FnKind::Normal,
                param_count: 0,
                locals: vec![
                    LocalDecl { ty: Type::Unit, name: None },
                    LocalDecl { ty: Type::Str, name: Some("s \"q\"".into()) },
                    LocalDecl {
                        ty: Type::Ref(
                            Box::new(Type::Named(
                                "Result".into(),
                                vec![Type::Int, Type::ErrSet(vec!["E".into()])],
                            )),
                            true,
                        ),
                        name: None,
                    },
                    LocalDecl {
                        ty: Type::Fn(vec![Type::Int], Box::new(Type::Tuple(vec![Type::Bool]))),
                        name: None,
                    },
                    LocalDecl { ty: Type::Int, name: None },
                ],
                blocks: vec![
                    BasicBlock {
                        stmts: vec![
                            Statement {
                                kind: StatementKind::Assign(
                                    Place::local(l(1)),
                                    Rvalue::Use(Operand::Const(Const::Str("a\nb".into()))),
                                ),
                                loc: Some(Loc { line: 3, col: 5 }),
                            },
                            Statement {
                                kind: StatementKind::Call {
                                    dest: Place::local(l(0)),
                                    callee: Callee::Intrinsic("print".into(), vec![Type::Str]),
                                    args: vec![Operand::Copy(Place::local(l(1)))],
                                },
                                loc: Some(Loc { line: 3, col: 5 }),
                            },
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
                        loc: Some(Loc { line: 4, col: 1 }),
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
    assert!(text.contains("struct Point { x: f64 = Point#x, tags: list[str] }"));
    assert!(text.contains("error E open { A, B(at: int, char) }"));
    assert!(text.contains("    _0 = call intrinsic print[str](copy _1) @3:5"));
    assert!(text.contains("_4 = add copy _2.*.as(0).0, const -3"));
    assert!(text.contains("  bb0 (unwind bb2):"));
    assert!(text.contains("  bb2 (cleanup):"));
    assert!(text.contains("switch copy _4 [0: bb1, 1: bb1, else: bb2] @4:1"));
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
fn validator_catches_broken_graphs() {
    let mut p = sample();
    p.functions[1].blocks[1].term = Terminator::Goto(BlockId(9));
    p.functions[1].blocks[0].unwind = Some(BlockId(1));
    let problems = validate(&p);
    assert!(problems.iter().any(|m| m.contains("missing bb9")));
    assert!(problems.iter().any(|m| m.contains("not a cleanup block")));
}
