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
    for (name, f) in &p.tests {
        let _ = writeln!(out, "test {} {f}", quote(name));
    }
    for t in &p.types {
        out.push('\n');
        write_type_decl(&mut out, t);
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
        Type::Fn(ps, r) => format!("fn({}) -> {}", ps.iter().map(ty).collect::<Vec<_>>().join(", "), ty(r)),
        Type::Ref(t, false) => format!("ref {}", ty(t)),
        Type::Ref(t, true) => format!("ref mut {}", ty(t)),
    }
}

fn write_type_decl(out: &mut String, t: &TypeDecl) {
    match t {
        TypeDecl::Struct { name, fields, drop } => {
            let _ = write!(out, "struct {name}");
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
        TypeDecl::Enum { name, variants, drop, error, open } => {
            let _ = write!(out, "{} {name}", if *error { "error" } else { "enum" });
            if *open {
                out.push_str(" open");
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

fn kind_name(k: FnKind) -> &'static str {
    match k {
        FnKind::Normal => "",
        FnKind::Closure => " closure",
        FnKind::Drop => " drop",
        FnKind::Value => " value",
        FnKind::Test => " test",
    }
}

fn write_function(out: &mut String, f: &Function) {
    let _ = writeln!(out, "fn {}({}){} source {} {{", f.name, f.param_count, kind_name(f.kind), f.source);
    for (i, l) in f.locals.iter().enumerate() {
        let _ = write!(out, "    let _{i}: {}", ty(&l.ty));
        if let Some(n) = &l.name {
            let _ = write!(out, " {}", quote(n));
        }
        out.push('\n');
    }
    for (i, b) in f.blocks.iter().enumerate() {
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
        if let Some(l) = b.loc {
            let _ = write!(out, " @{}:{}", l.line, l.col);
        }
        out.push('\n');
    }
    out.push_str("}\n");
}

fn loc(l: &Option<Loc>) -> String {
    match l {
        Some(l) => format!(" @{}:{}", l.line, l.col),
        None => String::new(),
    }
}

pub fn stmt(s: &Statement) -> String {
    let body = match &s.kind {
        StatementKind::Assign(p, r) => format!("{} = {}", place(p), rvalue(r)),
        StatementKind::Call { dest, callee, args } => {
            let c = match callee {
                Callee::Fn(n) => format!("fn {n}"),
                Callee::Intrinsic(n, tys) if tys.is_empty() => format!("intrinsic {n}"),
                Callee::Intrinsic(n, tys) => {
                    format!("intrinsic {n}[{}]", tys.iter().map(ty).collect::<Vec<_>>().join(", "))
                }
                Callee::Value(o) => operand(o),
            };
            format!("{} = call {c}({})", place(dest), args.iter().map(operand).collect::<Vec<_>>().join(", "))
        }
        StatementKind::Drop { place: p, flag } => match flag {
            Some(f) => format!("drop {} if _{}", place(p), f.0),
            None => format!("drop {}", place(p)),
        },
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
        Rvalue::Closure(n, ops) => format!("closure {n}({})", list(ops)),
        Rvalue::CopyOf(p) => format!("copyof {}", place(p)),
        Rvalue::StrView(p) => format!("strview {}", place(p)),
    }
}
