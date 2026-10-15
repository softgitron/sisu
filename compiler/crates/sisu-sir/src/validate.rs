//! Structural checks on SIR: every reference resolves, unwind edges lead
//! to cleanup blocks, `resume` appears only in cleanup, a `drop` outside
//! cleanup ends its block (the lowering invariant that makes each block's
//! unwind edge exact), the `from` arguments of a function exist, and every
//! type with a `Drop` has its drop glue.

use crate::*;

/// The builtin types SIR knows without a declaration.
pub const BUILTIN_TYPES: &[&str] = &[
    "String", "list", "Array", "dict", "set", "Iter", "Entry", "Path", "DateTime", "Fs", "Net", "Clock",
    "Rand", "Env", "Proc", "Stdin", "System", "Option", "Result", "Iterable",
];

/// The largest `$N` a type mentions, plus one.
fn params_needed(t: &Type) -> u32 {
    match t {
        Type::Param(i) => i + 1,
        Type::Named(_, a) | Type::Tuple(a) | Type::Dyn(_, a) => {
            a.iter().map(params_needed).max().unwrap_or(0)
        }
        Type::Fn(ps, r, _) => ps.iter().map(params_needed).max().unwrap_or(0).max(params_needed(r)),
        Type::Ref(t, _) => params_needed(t),
        Type::Assoc(t, _, a, _) => a.iter().map(params_needed).max().unwrap_or(0).max(params_needed(t)),
        _ => 0,
    }
}

pub fn validate(p: &Program) -> Vec<String> {
    let mut problems = Vec::new();
    let fn_names: Vec<&str> = p.functions.iter().map(|f| f.name.as_str()).collect();
    for f in &p.functions {
        let mut bad = |m: String| problems.push(format!("{}: {m}", f.name));
        if f.locals.len() < f.param_count as usize + 1 {
            bad("fewer locals than parameters".into());
        }
        if f.blocks.is_empty() {
            bad("no blocks".into());
        }
        if f.source as usize >= p.sources.len() && !p.sources.is_empty() {
            bad(format!("source {} does not exist", f.source));
        }
        if let Some(a) = f.from.iter().find(|a| **a >= f.param_count) {
            bad(format!("`from` names argument {a}, but the function has {} parameters", f.param_count));
        }
        if let Some(a) = f.inner.iter().find(|a| **a >= f.param_count) {
            bad(format!("`inner` names argument {a}, but the function has {} parameters", f.param_count));
        }
        let ntp = f.type_params.len() as u32;
        for (i, l) in f.locals.iter().enumerate() {
            if params_needed(&l.ty) > ntp {
                bad(format!("the type of _{i} names a type parameter the function does not have"));
            }
        }
        let nb = f.blocks.len() as u32;
        let nl = f.locals.len() as u32;
        let check_place = |pl: &Place, bad: &mut dyn FnMut(String)| {
            if pl.local.0 >= nl {
                bad(format!("local _{} does not exist", pl.local.0));
            }
            for pr in &pl.proj {
                if let Proj::Index(l) | Proj::IndexFromEnd(l) | Proj::Key(l) = pr
                    && l.0 >= nl
                {
                    bad(format!("local _{} does not exist", l.0));
                }
            }
        };
        for (bi, b) in f.blocks.iter().enumerate() {
            if let Some(u) = b.unwind {
                if u.0 >= nb {
                    bad(format!("bb{bi}: unwind to missing bb{}", u.0));
                } else if !f.blocks[u.0 as usize].cleanup {
                    bad(format!("bb{bi}: unwinds to bb{}, which is not a cleanup block", u.0));
                }
            }
            if b.cleanup && b.unwind.is_some() {
                bad(format!("bb{bi}: a cleanup block cannot unwind"));
            }
            for (si, s) in b.stmts.iter().enumerate() {
                let last = si + 1 == b.stmts.len();
                match &s.kind {
                    StatementKind::Assign(pl, r) => {
                        check_place(pl, &mut bad);
                        if let Rvalue::Ref(x, _)
                        | Rvalue::Discriminant(x)
                        | Rvalue::CopyOf(x)
                        | Rvalue::StrView(x) = r
                        {
                            check_place(x, &mut bad);
                        }
                        if let Rvalue::Closure(n, _, _) = r
                            && !fn_names.contains(&n.as_str())
                        {
                            bad(format!("closure function {n} does not exist"));
                        }
                    }
                    StatementKind::Call { dest, callee, .. } => {
                        check_place(dest, &mut bad);
                        if let Callee::Fn(n, _) = callee
                            && !fn_names.contains(&n.as_str())
                        {
                            bad(format!("bb{bi}: calls missing function {n}"));
                        }
                        if let Callee::Trait(n, _) = callee
                            && p.trait_method(n).is_none()
                        {
                            bad(format!(
                                "bb{bi}: calls the trait method {n}, which has no `method` declaration"
                            ));
                        }
                        let tys = match callee {
                            Callee::Fn(_, t) | Callee::Intrinsic(_, t) | Callee::Trait(_, t) => t.as_slice(),
                            Callee::Value(_) => &[],
                        };
                        if tys.iter().any(|t| params_needed(t) > ntp) {
                            bad(format!(
                                "bb{bi}: a type argument names a type parameter the function does not have"
                            ));
                        }
                    }
                    StatementKind::Drop { place, flag } => {
                        check_place(place, &mut bad);
                        if let Some(fl) = flag
                            && fl.0 >= nl
                        {
                            bad(format!("drop flag _{} does not exist", fl.0));
                        }
                        if !b.cleanup && !last {
                            bad(format!("bb{bi}: a drop must end its block"));
                        }
                    }
                    StatementKind::StorageDead(l) => {
                        if l.0 >= nl {
                            bad(format!("local _{} does not exist", l.0));
                        }
                        if l.0 == 0 || l.0 <= f.param_count {
                            bad(format!("bb{bi}: the storage of _{} cannot end inside the function", l.0));
                        }
                    }
                }
            }
            let targets: Vec<BlockId> = match &b.term {
                Terminator::Goto(t) => vec![*t],
                Terminator::Switch { cases, default, .. } => {
                    cases.iter().map(|(_, t)| *t).chain(std::iter::once(*default)).collect()
                }
                Terminator::Resume => {
                    if !b.cleanup {
                        bad(format!("bb{bi}: resume outside a cleanup block"));
                    }
                    Vec::new()
                }
                Terminator::Return | Terminator::Unreachable => Vec::new(),
            };
            for t in targets {
                if t.0 >= nb {
                    bad(format!("bb{bi}: jumps to missing bb{}", t.0));
                }
            }
        }
    }
    // Drop glue: one function per declared type that has any, and one for
    // every type with a `Drop`.
    for f in p.functions.iter().filter(|f| f.kind == FnKind::Glue) {
        let declared = f.name.strip_suffix("#drop").and_then(|t| p.type_decl(t));
        match declared {
            None => problems.push(format!("{}: drop glue of a type that is not declared", f.name)),
            Some(t) if f.param_count != 1 || t.params().len() != f.type_params.len() => {
                problems.push(format!("{}: drop glue takes the value, with the type's parameters", f.name))
            }
            Some(_) => {}
        }
    }
    for t in &p.types {
        let drop = match t {
            TypeDecl::Struct { drop, .. } | TypeDecl::Enum { drop, .. } => drop,
        };
        let glue = types::glue_name(t.name());
        if drop.is_some() && !p.functions.iter().any(|f| f.kind == FnKind::Glue && f.name == glue) {
            problems.push(format!("type {} has a `Drop` but no drop glue {glue}", t.name()));
        }
    }
    if let Some(e) = &p.entry
        && !fn_names.contains(&e.function.as_str())
    {
        problems.push(format!("entry {} does not exist", e.function));
    }
    for i in &p.impls {
        for (m, t) in &i.methods {
            if let ImplTarget::Fn(f) = t
                && !fn_names.contains(&f.as_str())
            {
                problems.push(format!(
                    "impl {} for {}: method {m} names missing function {f}",
                    i.trait_name,
                    write::ty(&i.self_ty)
                ));
            }
        }
        if params_needed(&i.self_ty) > i.params || i.trait_args.iter().any(|t| params_needed(t) > i.params) {
            problems.push(format!(
                "impl {} for {}: a pattern names a parameter the impl does not have",
                i.trait_name,
                write::ty(&i.self_ty)
            ));
        }
    }
    problems
}
