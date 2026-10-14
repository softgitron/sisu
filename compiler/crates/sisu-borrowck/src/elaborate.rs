//! Drop elaboration: every `drop` that lowering emitted is decided by the
//! initialisation dataflow at its point.
//!
//! - initialised on every path: kept;
//! - uninitialised on every path (moved, or a payload of a variant that is
//!   not active there): removed;
//! - moved as a whole on some paths: guarded by a drop flag, `drop _3 if
//!   _9`, where the flag is a `bool` local that is `true` exactly while the
//!   path holds a value;
//! - partly moved: opened into drops of the parts that remain, fields in
//!   reverse declaration order (04, rule 6), each decided the same way. A
//!   type that implements `Drop` is never opened (moving out of one is an
//!   error), so its `Drop` always sees the whole value.
//!
//! Flags are set after every statement that initialises their path (or a
//! path containing it) and cleared before every statement that moves, drops
//! or ends it; a flagged `drop` clears its own flag as it runs. A drop
//! outside a cleanup block still ends its block afterwards.

use crate::init::{Analysis, State};
use crate::paths::{Cover, PathId};
use crate::types::Types;
use sisu_sir::{
    BasicBlock, BlockId, Const, Function, LocalDecl, LocalId, Operand, Place, Proj, Rvalue, Statement,
    StatementKind, Terminator, Type,
};
use std::collections::HashMap;

enum Decision {
    Keep,
    Remove,
    Flag(PathId),
    Parts(Vec<(Place, Option<PathId>)>),
}

pub fn elaborate(f: &mut Function, a: &Analysis, types: &Types) {
    if f.blocks.is_empty() {
        return;
    }
    let mut decisions: HashMap<(usize, usize), Decision> = HashMap::new();
    for b in 0..f.blocks.len() {
        let states = a.states(f, types, b);
        for (si, s) in f.blocks[b].stmts.iter().enumerate() {
            if let StatementKind::Drop { place, flag: None } = &s.kind {
                let d =
                    if a.reachable[b] { decide(a, types, f, &states[si], place) } else { Decision::Remove };
                decisions.insert((b, si), d);
            }
        }
    }
    // The flags the decisions read.
    let mut flagged: Vec<PathId> = Vec::new();
    for d in decisions.values() {
        match d {
            Decision::Flag(p) => flagged.push(*p),
            Decision::Parts(parts) => flagged.extend(parts.iter().filter_map(|(_, f)| *f)),
            _ => {}
        }
    }
    flagged.sort_unstable();
    flagged.dedup();
    let mut flags: HashMap<PathId, LocalId> = HashMap::new();
    for p in &flagged {
        let l = LocalId(f.locals.len() as u32);
        f.locals.push(LocalDecl::new(Type::Bool, Some("flag".into())));
        flags.insert(*p, l);
    }
    let set = |l: LocalId, v: bool| Statement {
        kind: StatementKind::Assign(Place::local(l), Rvalue::Use(Operand::Const(Const::Bool(v)))),
        loc: None,
    };
    // Rewrite every block.
    let old_blocks = std::mem::take(&mut f.blocks);
    let mut blocks: Vec<BasicBlock> = Vec::new();
    let mut extra: Vec<BasicBlock> = Vec::new();
    let base = old_blocks.len();
    for (b, block) in old_blocks.into_iter().enumerate() {
        let mut stmts: Vec<Statement> = Vec::new();
        for (si, s) in block.stmts.into_iter().enumerate() {
            let decision = decisions.remove(&(b, si));
            let reads: Vec<PathId> = match &decision {
                Some(Decision::Flag(p)) => vec![*p],
                Some(Decision::Parts(parts)) => parts.iter().filter_map(|(_, f)| *f).collect(),
                _ => Vec::new(),
            };
            for id in a.removed_paths(&s) {
                for d in a.paths.descendants(id) {
                    if let Some(&fl) = flags.get(&d)
                        && !reads.contains(&d)
                    {
                        stmts.push(set(fl, false));
                    }
                }
            }
            let init = a.init_path(&s);
            match decision {
                None | Some(Decision::Keep) => stmts.push(s),
                Some(Decision::Remove) => {}
                Some(Decision::Flag(p)) => {
                    if let StatementKind::Drop { place, .. } = &s.kind {
                        stmts.push(Statement {
                            kind: StatementKind::Drop { place: place.clone(), flag: flags.get(&p).copied() },
                            loc: s.loc,
                        });
                    }
                }
                Some(Decision::Parts(parts)) => {
                    for (place, fl) in parts {
                        stmts.push(Statement {
                            kind: StatementKind::Drop {
                                place,
                                flag: fl.and_then(|p| flags.get(&p).copied()),
                            },
                            loc: s.loc,
                        });
                    }
                }
            }
            if let Some(id) = init {
                for d in a.paths.descendants(id) {
                    if let Some(&fl) = flags.get(&d) {
                        stmts.push(set(fl, true));
                    }
                }
            }
        }
        // Outside cleanup, each drop ends its block: split after it.
        let mut pieces: Vec<Vec<Statement>> = vec![Vec::new()];
        for s in stmts {
            let is_drop = matches!(s.kind, StatementKind::Drop { .. });
            if let Some(last) = pieces.last_mut() {
                last.push(s);
            }
            if is_drop && !block.cleanup {
                pieces.push(Vec::new());
            }
        }
        if pieces.len() > 1 && pieces.last().is_some_and(|p| p.is_empty()) {
            pieces.pop();
        }
        let n = pieces.len();
        let mut ids: Vec<usize> = vec![b];
        for _ in 1..n {
            ids.push(base + extra.len() + ids.len() - 1);
        }
        let mut it = pieces.into_iter().enumerate();
        let mut first: Option<BasicBlock> = None;
        let mut rest: Vec<BasicBlock> = Vec::new();
        for (i, piece) in &mut it {
            let last = i + 1 == n;
            let bb = BasicBlock {
                stmts: piece,
                term: if last { block.term.clone() } else { Terminator::Goto(BlockId(ids[i + 1] as u32)) },
                unwind: block.unwind,
                cleanup: block.cleanup,
                loc: if last { block.loc } else { None },
            };
            if i == 0 {
                first = Some(bb);
            } else {
                rest.push(bb);
            }
        }
        if let Some(bb) = first {
            blocks.push(bb);
        }
        extra.extend(rest);
    }
    blocks.extend(extra);
    f.blocks = blocks;
    // Flags start false, except those of parameters, which hold values.
    if !flags.is_empty() {
        let mut init: Vec<(PathId, LocalId)> = flags.iter().map(|(p, l)| (*p, *l)).collect();
        init.sort_by_key(|(_, l)| *l);
        let stmts: Vec<Statement> = init
            .into_iter()
            .map(|(p, l)| {
                let local = a.paths.local_of(p).0;
                set(l, local >= 1 && local <= f.param_count)
            })
            .collect();
        prepend_to_entry(f, stmts);
    }
}

/// Puts statements at the start of the function. When the entry block is
/// a jump target, they go in a new entry block in front of it.
fn prepend_to_entry(f: &mut Function, stmts: Vec<Statement>) {
    let targeted =
        f.blocks.iter().any(|b| crate::cfg::targets(b).contains(&0) || b.unwind == Some(BlockId(0)));
    if !targeted {
        let mut s = stmts;
        s.append(&mut f.blocks[0].stmts);
        f.blocks[0].stmts = s;
        return;
    }
    let n = f.blocks.len() as u32;
    let old = std::mem::replace(
        &mut f.blocks[0],
        BasicBlock { stmts, term: Terminator::Goto(BlockId(n)), unwind: None, cleanup: false, loc: None },
    );
    f.blocks.push(old);
    let fix = |t: &mut BlockId| {
        if t.0 == 0 {
            t.0 = n;
        }
    };
    for (i, b) in f.blocks.iter_mut().enumerate() {
        if i == 0 {
            continue;
        }
        match &mut b.term {
            Terminator::Goto(t) => fix(t),
            Terminator::Switch { cases, default, .. } => {
                for (_, t) in cases.iter_mut() {
                    fix(t);
                }
                fix(default);
            }
            _ => {}
        }
        if let Some(u) = &mut b.unwind {
            fix(u);
        }
    }
}

fn decide(a: &Analysis, types: &Types, f: &Function, st: &State, place: &Place) -> Decision {
    let (id, cover) = a.paths.lookup(place);
    if cover != Cover::Exact {
        // A place through a reference, index or key always holds a value.
        return Decision::Keep;
    }
    if a.fully_uninit(st, id) {
        return Decision::Remove;
    }
    // A variant that cannot be active here holds nothing to drop: open the
    // drop so its payload's drop goes (a `None` of must-consume values).
    let vacuous =
        a.paths.descendants(id).into_iter().skip(1).any(|d| !st.mi.contains(d) && !st.mu.contains(d));
    if a.fully_init(st, id)
        && !(vacuous && types.place_ty(f, place).is_some_and(|t| !types.implements_drop(&t)))
    {
        return Decision::Keep;
    }
    let own = (st.mi.contains(id), st.mu.contains(id));
    let uniform = a.paths.descendants(id).iter().all(|d| (st.mi.contains(*d), st.mu.contains(*d)) == own);
    let ty = types.place_ty(f, place);
    match ty {
        Some(t) if !uniform && !types.implements_drop(&t) => {
            let mut parts = Vec::new();
            open(a, types, f, st, place, &t, &mut parts);
            Decision::Parts(parts)
        }
        _ => Decision::Flag(id),
    }
}

/// The parts of a partly moved value that may still hold something.
fn open(
    a: &Analysis,
    types: &Types,
    f: &Function,
    st: &State,
    place: &Place,
    ty: &Type,
    out: &mut Vec<(Place, Option<PathId>)>,
) {
    let mut part_places: Vec<Place> = Vec::new();
    match types.variant_count(ty) {
        Some(n) => {
            for v in 0..n as u32 {
                let k = types.field_count(ty, Some(v));
                for i in (0..k as u32).rev() {
                    part_places.push(place.project(Proj::Downcast(v)).project(Proj::Field(i)));
                }
            }
        }
        None => {
            let k = types.field_count(ty, None);
            for i in (0..k as u32).rev() {
                part_places.push(place.project(Proj::Field(i)));
            }
        }
    }
    for q in part_places {
        if types.place_ty(f, &q).is_some_and(|t| types.trivial_drop(&t)) {
            continue;
        }
        let (qid, cover) = a.paths.lookup(&q);
        if cover == Cover::Exact {
            match decide(a, types, f, st, &q) {
                Decision::Keep => out.push((q, None)),
                Decision::Remove => {}
                Decision::Flag(p) => out.push((q, Some(p))),
                Decision::Parts(ps) => out.extend(ps),
            }
            continue;
        }
        // No path of its own: it is in the state of its closest path.
        let (mi, mu) = (st.mi.contains(qid), st.mu.contains(qid));
        match (mi, mu) {
            (false, _) => {}
            (true, false) => out.push((q, None)),
            (true, true) => out.push((q, Some(qid))),
        }
    }
}
