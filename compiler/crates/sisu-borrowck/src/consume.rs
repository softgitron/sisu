//! `@must_consume` (D-054, D-064): a value of a must-consume type (or a
//! container holding one) must be passed to an `own` parameter; a `drop`
//! of one on a normal path is an error. Drops in cleanup blocks are the
//! unwinding path, which is exempt, and so are the type's own methods,
//! where its values end (`close(own self)` drops `self`).
//!
//! Consuming iteration discharges a container: `for r in own receipts`
//! moves the list into an iterator, and the iterator may be dropped once
//! its `Iter.next` has returned `None` on every path to the drop. Leaving
//! the loop early (`break`, `return`, `?`) drops an iterator that may still
//! hold values, which is the error 04 describes.

use crate::Finding;
use crate::bitset::BitSet;
use crate::names;
use crate::types::Types;
use sisu_diag::msg;
use sisu_sir::{Callee, Function, LocalId, Operand, Rvalue, StatementKind, Terminator, Type};

/// The unique statement that assigns a local, if there is exactly one.
fn unique_def(f: &Function, l: LocalId) -> Option<&StatementKind> {
    let mut found = None;
    for b in &f.blocks {
        for s in &b.stmts {
            let d = match &s.kind {
                StatementKind::Assign(p, _) | StatementKind::Call { dest: p, .. } => p,
                _ => continue,
            };
            if d.local == l && d.proj.is_empty() {
                if found.is_some() {
                    return None;
                }
                found = Some(&s.kind);
            }
        }
    }
    found
}

/// For a block ending in the `for` loop's switch on `Iter.next`'s result,
/// the iterator local.
fn loop_iterator(f: &Function, b: usize) -> Option<LocalId> {
    let block = &f.blocks[b];
    let Terminator::Switch { value: Operand::Copy(d), .. } = &block.term else { return None };
    let StatementKind::Assign(_, Rvalue::Discriminant(n)) = unique_def(f, d.local)? else { return None };
    if !n.proj.is_empty() {
        return None;
    }
    let StatementKind::Call { callee: Callee::Intrinsic(name, _), args, .. } = unique_def(f, n.local)? else {
        return None;
    };
    if name != "Iter.next" {
        return None;
    }
    let Some(Operand::Copy(r)) = args.first() else { return None };
    let StatementKind::Assign(_, Rvalue::Ref(it, true)) = unique_def(f, r.local)? else { return None };
    if it.proj.is_empty() { Some(it.local) } else { None }
}

/// The intrinsics that keep or return every value they are given with
/// `own` or `mut`, by argument position: these may take must-consume
/// values. Any other intrinsic may drop what it is given (`list.clear`,
/// the key of `dict.insert`, an `Entry.or_insert` value when the key is
/// present), so a must-consume value reaching it is rejected.
const KEEPS: &[(&str, &[usize])] = &[
    ("list.push", &[0, 1]),
    ("list.pop", &[0]),
    ("list.insert", &[0, 2]),
    ("list.remove", &[0]),
    ("list.extend", &[0, 1]),
    ("list.sort", &[0]),
    ("list.sort_by", &[0]),
    ("list.reverse", &[0]),
    ("dict.insert", &[0, 2]),
    ("dict.remove", &[0]),
    ("iter.own", &[0]),
    ("Iter.next", &[0]),
];

/// Calls of intrinsics that may drop a must-consume value given to them.
fn dropping_intrinsics(f: &Function, types: &Types, out: &mut Vec<Finding>) {
    for (b, block) in f.blocks.iter().enumerate() {
        for (si, s) in block.stmts.iter().enumerate() {
            let StatementKind::Call { callee: Callee::Intrinsic(name, _), args, .. } = &s.kind else {
                continue;
            };
            for (i, a) in args.iter().enumerate() {
                let (place, given) = match a {
                    Operand::Move(p) => (p, types.place_ty(f, p)),
                    Operand::Copy(p) => match types.place_ty(f, p) {
                        Some(Type::Ref(inner, true)) => (p, Some(*inner)),
                        _ => continue,
                    },
                    Operand::Const(_) => continue,
                };
                let Some(ty) = given.filter(|t| types.must_consume(t)) else { continue };
                if KEEPS.iter().any(|(n, keep)| n == name && keep.contains(&i)) {
                    continue;
                }
                let shown = match a {
                    // A `mut` argument is a reference made for the call:
                    // name what it borrows.
                    Operand::Copy(_) => borrowed_place(f, place.local).unwrap_or_else(|| place.clone()),
                    _ => place.clone(),
                };
                let shown_name = names::place_name(types, f, &shown);
                let mut fd = Finding::new(msg::BORROW_MUST_CONSUME_INTRINSIC, f, s.loc)
                    .with("callee", name.clone())
                    .with_arg("name", shown_name.clone())
                    .with("ty", sisu_sir::write::ty(&ty))
                    .at(b, si);
                let decl = f.locals.get(shown.local.0 as usize).and_then(|l| l.loc);
                if decl.is_some() && names::is_named(f, shown.local) {
                    fd = fd.label(decl, msg::BORROW_DECLARED_HERE, vec![("name", shown_name)]);
                } else if let Some(made) = names::temp_source(f, shown.local) {
                    fd = fd.label(Some(made), msg::BORROW_MADE_HERE, Vec::new());
                }
                out.push(fd);
            }
        }
    }
}

/// The place a reference local was made to borrow, when it was made once.
fn borrowed_place(f: &Function, r: LocalId) -> Option<sisu_sir::Place> {
    match unique_def(f, r)? {
        StatementKind::Assign(_, Rvalue::Ref(p, _)) => Some(p.clone()),
        _ => None,
    }
}

/// The collection a consuming loop's iterator was made from: `it =
/// iter.own(move xs)` gives `xs`.
fn consumed_collection(f: &Function, it: LocalId) -> Option<LocalId> {
    let StatementKind::Call { callee: Callee::Intrinsic(name, _), args, .. } = unique_def(f, it)? else {
        return None;
    };
    match args.first() {
        Some(Operand::Move(p)) if name == "iter.own" && p.proj.is_empty() => Some(p.local),
        _ => None,
    }
}

/// Whether `f` is a method of the type `ty` names: the type's own methods
/// are where its values may end (a `close(own self)` drops `self`).
fn own_method(f: &Function, ty: &Type) -> bool {
    match ty {
        Type::Named(n, _) => f.name.strip_prefix(n.as_str()).is_some_and(|rest| rest.starts_with('.')),
        _ => false,
    }
}

pub fn check(f: &Function, types: &Types) -> Vec<Finding> {
    let n = f.blocks.len();
    let nl = f.locals.len();
    if n == 0 {
        return Vec::new();
    }
    // Exhausted iterators: a must analysis (intersection); a block not
    // yet reached has no state.
    let mut entry: Vec<Option<BitSet>> = vec![None; n];
    entry[0] = Some(BitSet::new(nl));
    let kill = |s: &StatementKind, st: &mut BitSet| match s {
        StatementKind::Assign(p, rv) => {
            if p.proj.is_empty() {
                st.remove(p.local.0 as usize);
            }
            if let Rvalue::Ref(q, true) = rv {
                st.remove(q.local.0 as usize);
            }
        }
        StatementKind::Call { dest, .. } if dest.proj.is_empty() => st.remove(dest.local.0 as usize),
        _ => {}
    };
    let mut changed = true;
    let mut rounds = 0;
    while changed && rounds < 1000 {
        changed = false;
        rounds += 1;
        for b in 0..n {
            let Some(mut st) = entry[b].clone() else { continue };
            for s in &f.blocks[b].stmts {
                kill(&s.kind, &mut st);
            }
            let it = loop_iterator(f, b);
            let mut succ: Vec<(usize, BitSet)> = Vec::new();
            match &f.blocks[b].term {
                Terminator::Goto(t) => succ.push((t.0 as usize, st.clone())),
                Terminator::Switch { cases, default, .. } => {
                    for (k, t) in cases {
                        let mut s2 = st.clone();
                        if let Some(i) = it {
                            if *k == 0 {
                                s2.remove(i.0 as usize);
                            } else {
                                s2.insert(i.0 as usize);
                            }
                        }
                        succ.push((t.0 as usize, s2));
                    }
                    let mut s2 = st.clone();
                    if let Some(i) = it {
                        s2.insert(i.0 as usize);
                    }
                    succ.push((default.0 as usize, s2));
                }
                _ => {}
            }
            if let Some(u) = f.blocks[b].unwind {
                succ.push((u.0 as usize, st.clone()));
            }
            for (t, s2) in succ {
                if t >= n {
                    continue;
                }
                match &mut entry[t] {
                    None => {
                        entry[t] = Some(s2);
                        changed = true;
                    }
                    Some(e) => {
                        if e.intersect(&s2) {
                            changed = true;
                        }
                    }
                }
            }
        }
    }
    // An intersection converges too; if the bound is reached anyway, no
    // iterator counts as exhausted, which only rejects more.
    if changed {
        for e in entry.iter_mut().flatten() {
            e.clear();
        }
    }
    let mut out = Vec::new();
    dropping_intrinsics(f, types, &mut out);
    for (b, (block, start)) in f.blocks.iter().zip(&entry).enumerate() {
        if block.cleanup {
            continue;
        }
        let Some(mut st) = start.clone() else { continue };
        for (si, s) in block.stmts.iter().enumerate() {
            if let StatementKind::Drop { place, .. } = &s.kind
                && let Some(ty) = types.place_ty(f, place)
                && types.must_consume(&ty)
            {
                let exhausted = place.proj.is_empty()
                    && matches!(&ty, Type::Named(n, _) if n == "Iter")
                    && st.contains(place.local.0 as usize);
                if !exhausted && !own_method(f, &ty) {
                    // A consuming loop's iterator is described by the
                    // collection it consumes.
                    let collection =
                        if place.proj.is_empty() { consumed_collection(f, place.local) } else { None };
                    let (shown, shown_ty) = match collection {
                        Some(c) => (sisu_sir::Place::local(c), f.locals[c.0 as usize].ty.clone()),
                        None => (place.clone(), ty.clone()),
                    };
                    let name = names::place_name(types, f, &shown);
                    let at_end = s.loc.is_some_and(|l| (l.line, l.col) == (l.end_line, l.end_col));
                    let overwrite = names::is_overwrite(f, b, place);
                    let m = if at_end || overwrite {
                        msg::BORROW_MUST_CONSUME
                    } else {
                        msg::BORROW_MUST_CONSUME_EXIT
                    };
                    let mut fd = Finding::new(m, f, s.loc)
                        .with_arg("name", name.clone())
                        .with("ty", sisu_sir::write::ty(&shown_ty))
                        .at(b, si);
                    let decl = f.locals.get(shown.local.0 as usize).and_then(|l| l.loc);
                    if decl.is_some() && names::is_named(f, shown.local) {
                        fd = fd.label(decl, msg::BORROW_DECLARED_HERE, vec![("name", name.clone())]);
                    } else if let Some(made) = names::temp_source(f, shown.local) {
                        fd = fd.label(Some(made), msg::BORROW_MADE_HERE, Vec::new());
                    }
                    if collection.is_some() {
                        fd = fd.note(msg::BORROW_UNVISITED_REST, vec![("name", name)]);
                    }
                    out.push(fd);
                }
            }
            kill(&s.kind, &mut st);
        }
    }
    out
}
