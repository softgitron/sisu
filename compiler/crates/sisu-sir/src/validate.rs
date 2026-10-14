//! Structural checks on SIR: every reference resolves, unwind edges lead
//! to cleanup blocks, `resume` appears only in cleanup, a `drop` outside
//! cleanup ends its block (the lowering invariant that makes each block's
//! unwind edge exact), and the `from` arguments of a function exist.

use crate::*;

/// The builtin types SIR knows without a declaration.
pub const BUILTIN_TYPES: &[&str] = &[
    "String", "list", "dict", "set", "Iter", "Entry", "Path", "DateTime", "Fs", "Env", "Clock", "System",
    "Option", "Result", "Iterable",
];

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
                        if let Rvalue::Closure(n, _) = r
                            && !fn_names.contains(&n.as_str())
                        {
                            bad(format!("closure function {n} does not exist"));
                        }
                    }
                    StatementKind::Call { dest, callee, .. } => {
                        check_place(dest, &mut bad);
                        if let Callee::Fn(n) = callee
                            && !fn_names.contains(&n.as_str())
                        {
                            bad(format!("bb{bi}: calls missing function {n}"));
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
    if let Some(e) = &p.entry
        && !fn_names.contains(&e.function.as_str())
    {
        problems.push(format!("entry {} does not exist", e.function));
    }
    problems
}
