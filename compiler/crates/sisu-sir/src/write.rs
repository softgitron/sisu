//! The SIR text form. One construct per line; see docs/architecture.md
//! for the grammar.

use crate::*;
use std::fmt::Write as _;

pub fn write(p: &Program) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "sir {VERSION}");
    for (i, s) in p.sources.iter().enumerate() {
        let _ = writeln!(out, "source {i} {}", quote(s));
    }
    if let Some(e) = &p.entry {
        let _ = write!(out, "entry {}", e.function);
        if e.takes_system {
            out.push_str(" system");
        }
        if e.returns_result {
            out.push_str(" result");
        }
        out.push('\n');
    }
    for (k, (name, f)) in p.tests.iter().enumerate() {
        let _ = write!(out, "test {} {f}", quote(name));
        // The test's `@tag`s, when it has any (decision 285).
        if let Some(tags) = p.test_tags.get(k).filter(|t| !t.is_empty()) {
            out.push_str(" tags");
            for t in tags {
                let _ = write!(out, " {}", quote(t));
            }
        }
        out.push('\n');
    }
    for (name, f) in &p.benches {
        let _ = writeln!(out, "bench {} {f}", quote(name));
    }
    for t in &p.types {
        out.push('\n');
        write_type_decl(&mut out, t);
    }
    if !p.intrinsics.is_empty() {
        out.push('\n');
    }
    for i in &p.intrinsics {
        write_intrinsic(&mut out, i);
    }
    if !p.trait_methods.is_empty() {
        out.push('\n');
    }
    for i in &p.trait_methods {
        write_decl(&mut out, "method", i);
    }
    if !p.impls.is_empty() {
        out.push('\n');
    }
    for i in &p.impls {
        write_impl(&mut out, i);
    }
    for f in &p.functions {
        out.push('\n');
        write_function(&mut out, f);
    }
    out
}

pub fn quote(s: &str) -> String {
    let mut q = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => q.push_str("\\\""),
            '\\' => q.push_str("\\\\"),
            '\n' => q.push_str("\\n"),
            '\t' => q.push_str("\\t"),
            '\r' => q.push_str("\\r"),
            '\0' => q.push_str("\\0"),
            c if (c as u32) < 0x20 => {
                let _ = write!(q, "\\u{{{:x}}}", c as u32);
            }
            c => q.push(c),
        }
    }
    q.push('"');
    q
}

fn quote_char(c: char) -> String {
    match c {
        '\'' => "'\\''".into(),
        '\\' => "'\\\\'".into(),
        '\n' => "'\\n'".into(),
        '\t' => "'\\t'".into(),
        '\r' => "'\\r'".into(),
        '\0' => "'\\0'".into(),
        c if (c as u32) < 0x20 => format!("'\\u{{{:x}}}'", c as u32),
        c => format!("'{c}'"),
    }
}

pub fn ty(t: &Type) -> String {
    match t {
        Type::Int => "int".into(),
        Type::Float => "f64".into(),
        Type::Bool => "bool".into(),
        Type::Char => "char".into(),
        Type::Str => "str".into(),
        Type::Unit => "()".into(),
        Type::Never => "Never".into(),
        Type::Named(n, args) if args.is_empty() => n.clone(),
        Type::Named(n, args) => format!("{n}[{}]", args.iter().map(ty).collect::<Vec<_>>().join(", ")),
        Type::Tuple(es) if es.len() == 1 => format!("({},)", ty(&es[0])),
        Type::Tuple(es) => format!("({})", es.iter().map(ty).collect::<Vec<_>>().join(", ")),
        Type::ErrSet(ms) => format!("{{{}}}", ms.join(", ")),
        Type::Fn(ps, r, pure) => format!(
            "{}fn({}) -> {}",
            if *pure { "pure " } else { "" },
            ps.iter().map(ty).collect::<Vec<_>>().join(", "),
            ty(r)
        ),
        Type::Ref(t, false) => format!("ref {}", ty(t)),
        Type::Ref(t, true) => format!("ref mut {}", ty(t)),
        Type::Param(i) => format!("${i}"),
        Type::Const(n) => n.to_string(),
        Type::ConstOp(op, args) => {
            format!("{}({})", op.name(), args.iter().map(ty).collect::<Vec<_>>().join(", "))
        }
        Type::Dyn(n, args) => format!("dyn {}", named(n, args)),
        Type::Assoc(t, tr, args, n) => format!("<{} as {}>.{n}", ty(t), named(tr, args)),
    }
}

/// `Name` or `Name[A, B]`.
fn named(n: &str, args: &[Type]) -> String {
    if args.is_empty() {
        n.to_string()
    } else {
        format!("{n}[{}]", args.iter().map(ty).collect::<Vec<_>>().join(", "))
    }
}

/// `[A, B]` after a name, or nothing.
fn type_args(args: &[Type]) -> String {
    if args.is_empty() {
        String::new()
    } else {
        format!("[{}]", args.iter().map(ty).collect::<Vec<_>>().join(", "))
    }
}

/// `[T, U]` after a declaration's name, or nothing.
fn params(ps: &[String]) -> String {
    if ps.is_empty() { String::new() } else { format!("[{}]", ps.join(", ")) }
}

fn write_impl(out: &mut String, i: &ImplDecl) {
    let mut ms: Vec<String> = i
        .methods
        .iter()
        .map(|(m, t)| match t {
            ImplTarget::Fn(f) => format!("{m}: {f}"),
            ImplTarget::Intrinsic(f) => format!("{m}: intrinsic {f}"),
        })
        .collect();
    ms.extend(i.assoc.iter().map(|(n, t)| format!("type {n} = {}", ty(t))));
    let n = if i.params == 0 { String::new() } else { format!("[{}]", i.params) };
    let _ = writeln!(
        out,
        "impl{n} {} for {} {{ {} }}",
        named(&i.trait_name, &i.trait_args),
        ty(&i.self_ty),
        ms.join(", ")
    );
}

fn write_type_decl(out: &mut String, t: &TypeDecl) {
    match t {
        TypeDecl::Struct { name, params: ps, fields, drop, view, must_consume } => {
            let _ = write!(out, "{}struct {name}{}", if *view { "view " } else { "" }, params(ps));
            if *must_consume {
                out.push_str(" must_consume");
            }
            if let Some(d) = drop {
                let _ = write!(out, " drop {d}");
            }
            let fs: Vec<String> = fields
                .iter()
                .map(|f| match &f.default {
                    Some(d) => format!("{}: {} = {d}", f.name, ty(&f.ty)),
                    None => format!("{}: {}", f.name, ty(&f.ty)),
                })
                .collect();
            let _ = writeln!(out, " {{ {} }}", fs.join(", "));
        }
        TypeDecl::Enum { name, params: ps, variants, drop, error, open, must_consume } => {
            let _ = write!(out, "{} {name}{}", if *error { "error" } else { "enum" }, params(ps));
            if *open {
                out.push_str(" open");
            }
            if *must_consume {
                out.push_str(" must_consume");
            }
            if let Some(d) = drop {
                let _ = write!(out, " drop {d}");
            }
            let vs: Vec<String> = variants
                .iter()
                .map(|v| {
                    if v.fields.is_empty() {
                        v.name.clone()
                    } else {
                        let fs: Vec<String> = v
                            .fields
                            .iter()
                            .map(|(n, t)| match n {
                                Some(n) => format!("{n}: {}", ty(t)),
                                None => ty(t),
                            })
                            .collect();
                        format!("{}({})", v.name, fs.join(", "))
                    }
                })
                .collect();
            let _ = writeln!(out, " {{ {} }}", vs.join(", "));
        }
    }
}

fn indices(word: &str, xs: &[u32]) -> String {
    if xs.is_empty() {
        return String::new();
    }
    let list: Vec<String> = xs.iter().map(|x| x.to_string()).collect();
    format!(" {word} {}", list.join(" "))
}

fn write_intrinsic(out: &mut String, i: &IntrinsicDecl) {
    write_decl(out, "intrinsic", i);
}

fn write_decl(out: &mut String, word: &str, i: &IntrinsicDecl) {
    let _ = writeln!(
        out,
        "{word} {}{}{}{}{}{}",
        i.name,
        if i.pure { " pure" } else { "" },
        indices("from", &i.from),
        indices("inner", &i.inner),
        indices("own", &i.own),
        indices("keeps", &i.keeps)
    );
}

fn kind_name(k: FnKind) -> String {
    match k {
        FnKind::Normal => "".into(),
        FnKind::Closure(crate::ClosureRole::Plain) => " closure".into(),
        FnKind::Closure(crate::ClosureRole::ScopeBody) => " closure scope".into(),
        FnKind::Closure(crate::ClosureRole::Task) => " closure task".into(),
        FnKind::Drop => " drop".into(),
        FnKind::Glue => " glue".into(),
        FnKind::Value => " value".into(),
        FnKind::Test => " test".into(),
        FnKind::Comptime(steps) => format!(" comptime {steps}"),
    }
}

/// One function in the text form (the ownership monitor's SIR dump).
pub fn function(f: &Function) -> String {
    let mut out = String::new();
    write_function(&mut out, f);
    out
}

fn write_function(out: &mut String, f: &Function) {
    let _ = writeln!(
        out,
        "fn {}{}({}){}{} source {}{}{}{}{} {{",
        f.name,
        params(&f.type_params),
        f.param_count,
        kind_name(f.kind),
        if f.pure { " pure" } else { "" },
        f.source,
        indices("from", &f.from),
        indices("inner", &f.inner),
        indices("own", &f.own),
        loc(&f.loc)
    );
    for (i, l) in f.locals.iter().enumerate() {
        let _ = write!(out, "    let _{i}: {}", ty(&l.ty));
        if let Some(n) = &l.name {
            let _ = write!(out, " {}", quote(n));
        }
        if l.once {
            out.push_str(" once");
        }
        out.push_str(&loc(&l.loc));
        out.push('\n');
    }
    for (i, b) in f.blocks.iter().enumerate() {
        write_block(out, i, b);
    }
    out.push_str("}\n");
}

fn write_block(out: &mut String, i: usize, b: &BasicBlock) {
    let _ = write!(out, "  bb{i}");
    if b.cleanup {
        out.push_str(" (cleanup)");
    } else if let Some(u) = b.unwind {
        let _ = write!(out, " (unwind bb{})", u.0);
    }
    out.push_str(":\n");
    for s in &b.stmts {
        out.push_str("    ");
        out.push_str(&stmt(s));
        out.push('\n');
    }
    out.push_str("    ");
    out.push_str(&term(&b.term));
    out.push_str(&loc(&b.loc));
    out.push('\n');
}

/// The function's blocks at these indices, in order, under its first
/// line and without its locals, `...` standing for the blocks left out
/// between them: the part of a function a report points into (the
/// counterexample mode's conflict, decision 467).
pub fn excerpt(f: &Function, keep: &[usize]) -> String {
    let mut keep: Vec<usize> = keep.iter().copied().filter(|&i| i < f.blocks.len()).collect();
    keep.sort_unstable();
    keep.dedup();
    let full = function(f);
    let mut out = full.lines().next().unwrap_or_default().to_string();
    out.push('\n');
    let mut next = 0;
    for i in keep {
        if i > next {
            out.push_str("  ...\n");
        }
        write_block(&mut out, i, &f.blocks[i]);
        next = i + 1;
    }
    if next < f.blocks.len() {
        out.push_str("  ...\n");
    }
    out.push_str("}\n");
    out
}

/// ` @L:C` or ` @L:C-L:C`, or nothing.
pub fn loc(l: &Option<Loc>) -> String {
    match l {
        Some(l) if l.has_end() => format!(" @{}:{}-{}:{}", l.line, l.col, l.end_line, l.end_col),
        Some(l) => format!(" @{}:{}", l.line, l.col),
        None => String::new(),
    }
}

pub fn stmt(s: &Statement) -> String {
    let body = match &s.kind {
        StatementKind::Assign(p, r) => format!("{} = {}", place(p), rvalue(r)),
        StatementKind::Call { dest, callee, args } => {
            let c = match callee {
                Callee::Fn(n, tys) => format!("fn {n}{}", type_args(tys)),
                Callee::Intrinsic(n, tys) => format!("intrinsic {n}{}", type_args(tys)),
                Callee::Trait(n, tys) => format!("trait {n}{}", type_args(tys)),
                Callee::Value(o) => operand(o),
            };
            format!("{} = call {c}({})", place(dest), args.iter().map(operand).collect::<Vec<_>>().join(", "))
        }
        StatementKind::Drop { place: p, flag } => match flag {
            Some(f) => format!("drop {} if _{}", place(p), f.0),
            None => format!("drop {}", place(p)),
        },
        StatementKind::StorageDead(l) => format!("dead _{}", l.0),
    };
    format!("{body}{}", loc(&s.loc))
}

pub fn term(t: &Terminator) -> String {
    match t {
        Terminator::Goto(b) => format!("goto bb{}", b.0),
        Terminator::Switch { value, cases, default } => {
            let mut parts: Vec<String> = cases.iter().map(|(v, b)| format!("{v}: bb{}", b.0)).collect();
            parts.push(format!("else: bb{}", default.0));
            format!("switch {} [{}]", operand(value), parts.join(", "))
        }
        Terminator::Return => "return".into(),
        Terminator::Resume => "resume".into(),
        Terminator::Unreachable => "unreachable".into(),
    }
}

pub fn place(p: &Place) -> String {
    let mut s = format!("_{}", p.local.0);
    for pr in &p.proj {
        match pr {
            Proj::Deref => s.push_str(".*"),
            Proj::Field(i) => {
                let _ = write!(s, ".{i}");
            }
            Proj::Downcast(v) => {
                let _ = write!(s, ".as({v})");
            }
            Proj::Index(l) => {
                let _ = write!(s, "[_{}]", l.0);
            }
            Proj::IndexFromEnd(l) => {
                let _ = write!(s, "[^_{}]", l.0);
            }
            Proj::Key(l) => {
                let _ = write!(s, "{{_{}}}", l.0);
            }
        }
    }
    s
}

pub fn operand(o: &Operand) -> String {
    match o {
        Operand::Copy(p) => format!("copy {}", place(p)),
        Operand::Move(p) => format!("move {}", place(p)),
        Operand::Const(c) => format!("const {}", constant(c)),
    }
}

pub fn constant(c: &Const) -> String {
    match c {
        Const::Int(n) => n.to_string(),
        Const::Float(f) => float(*f),
        Const::Bool(b) => b.to_string(),
        Const::Char(c) => quote_char(*c),
        Const::Str(s) => quote(s),
        Const::Unit => "()".into(),
        Const::Fn(n) => format!("fn {n}"),
    }
}

/// A float spelled so that reading it back gives the same bits and it
/// never looks like an integer.
pub fn float(f: f64) -> String {
    if f.is_nan() {
        return "nan".into();
    }
    if f.is_infinite() {
        return if f > 0.0 { "inf".into() } else { "-inf".into() };
    }
    let s = format!("{f:?}");
    if s.contains('.') || s.contains('e') { s } else { format!("{s}.0") }
}

pub fn rvalue(r: &Rvalue) -> String {
    let list = |ops: &[Operand]| ops.iter().map(operand).collect::<Vec<_>>().join(", ");
    match r {
        Rvalue::Use(o) => operand(o),
        Rvalue::Ref(p, false) => format!("ref {}", place(p)),
        Rvalue::Ref(p, true) => format!("ref mut {}", place(p)),
        Rvalue::Binary(op, a, b) => format!("{} {}, {}", op.name(), operand(a), operand(b)),
        Rvalue::Unary(op, a) => format!("{} {}", op.name(), operand(a)),
        Rvalue::Aggregate(Aggregate::Tuple, ops) => format!("tuple({})", list(ops)),
        Rvalue::Aggregate(Aggregate::Struct(n), ops) => format!("struct {n}({})", list(ops)),
        Rvalue::Aggregate(Aggregate::Variant(t, v), ops) => format!("variant {t}::{v}({})", list(ops)),
        Rvalue::Aggregate(Aggregate::List, ops) => format!("list({})", list(ops)),
        Rvalue::Aggregate(Aggregate::Set, ops) => format!("set({})", list(ops)),
        Rvalue::Aggregate(Aggregate::Dict, ops) => format!("dict({})", list(ops)),
        Rvalue::Discriminant(p) => format!("discriminant {}", place(p)),
        Rvalue::Closure(n, tys, ops) => format!("closure {n}{}({})", type_args(tys), list(ops)),
        Rvalue::Aggregate(Aggregate::Dyn(t), ops) => format!("dyn {}({})", ty(t), list(ops)),
        Rvalue::CopyOf(p) => format!("copyof {}", place(p)),
        Rvalue::StrView(p) => format!("strview {}", place(p)),
    }
}
