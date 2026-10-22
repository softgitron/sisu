//! Jump threading, run on a program the ownership checker has accepted
//! (docs/architecture.md, "The interpreter"). Lowering builds control flow
//! from small pieces: blocks that only jump on, and jumps into blocks that
//! only switch or return. The interpreter takes a step for every jump, so
//! this pass removes them before the program runs:
//!
//! - a jump to a block that only jumps on goes where that block goes;
//! - a block that ends in a jump to a block nothing else jumps to takes
//!   that block's statements and terminator;
//! - a jump to a block without statements takes that block's terminator;
//! - the blocks nothing reaches any more are removed, and the others keep
//!   their order.
//!
//! No statement is removed, duplicated or moved past another, so what a
//! function does is unchanged; what changes is how many steps it takes,
//! and with them which jumps go back to an earlier block (the scheduler's
//! yield points): a loop still has one, since a cycle of blocks cannot
//! only go forward. Drop glue is left as it is, since the interpreter
//! recognises its last drop by the block's shape.

use crate::{BlockId, FnKind, Function, Program, Terminator};

/// Threads the jumps of every function but drop glue.
pub fn program(p: &mut Program) {
    for f in &mut p.functions {
        if f.kind != FnKind::Glue {
            function(f);
        }
    }
}

fn targets(t: &Terminator) -> Vec<usize> {
    match t {
        Terminator::Goto(b) => vec![b.0 as usize],
        Terminator::Switch { cases, default, .. } => {
            cases.iter().map(|(_, b)| b.0 as usize).chain(std::iter::once(default.0 as usize)).collect()
        }
        Terminator::Return | Terminator::Resume | Terminator::Unreachable => Vec::new(),
    }
}

fn retarget(t: &mut Terminator, to: impl Fn(usize) -> usize) {
    let map = |b: &mut BlockId| *b = BlockId(to(b.0 as usize) as u32);
    match t {
        Terminator::Goto(b) => map(b),
        Terminator::Switch { cases, default, .. } => {
            for (_, b) in cases.iter_mut() {
                map(b);
            }
            map(default);
        }
        Terminator::Return | Terminator::Resume | Terminator::Unreachable => {}
    }
}

/// The block a jump to `b` reaches once past blocks that only jump on; `b`
/// itself when those blocks go round in a cycle.
fn through(f: &Function, b: usize) -> usize {
    let only_jumps = |i: usize| {
        let blk = f.blocks.get(i)?;
        match blk.term {
            Terminator::Goto(t) if blk.stmts.is_empty() && blk.loc.is_none() => Some(t.0 as usize),
            _ => None,
        }
    };
    let mut seen = vec![b];
    let mut cur = b;
    while let Some(t) = only_jumps(cur) {
        if t >= f.blocks.len() || seen.contains(&t) {
            return b;
        }
        seen.push(t);
        cur = t;
    }
    cur
}

/// Which blocks the entry reaches, through jumps and unwinding.
fn reached(f: &Function) -> Vec<bool> {
    let mut seen = vec![false; f.blocks.len()];
    let mut work = vec![0];
    while let Some(b) = work.pop() {
        if b >= seen.len() || seen[b] {
            continue;
        }
        seen[b] = true;
        let blk = &f.blocks[b];
        work.extend(targets(&blk.term));
        work.extend(blk.unwind.map(|u| u.0 as usize));
    }
    seen
}

/// Threads the jumps of one function.
pub fn function(f: &mut Function) {
    let n = f.blocks.len();
    if n == 0 {
        return;
    }
    // Jumps past blocks that only jump on.
    let to: Vec<usize> = (0..n).map(|b| through(f, b)).collect();
    for blk in &mut f.blocks {
        retarget(&mut blk.term, |b| to.get(b).copied().unwrap_or(b));
        if let Some(u) = &mut blk.unwind {
            *u = BlockId(to.get(u.0 as usize).copied().unwrap_or(u.0 as usize) as u32);
        }
    }
    // Blocks that take the block they jump to.
    let live = reached(f);
    let mut preds = vec![0usize; n];
    preds[0] += 1;
    for (b, blk) in f.blocks.iter().enumerate() {
        if live[b] {
            for t in targets(&blk.term).into_iter().chain(blk.unwind.map(|u| u.0 as usize)) {
                preds[t] += 1;
            }
        }
    }
    for b in (0..n).filter(|&b| live[b]) {
        while let Terminator::Goto(t) = f.blocks[b].term {
            let t = t.0 as usize;
            let (here, there) = (&f.blocks[b], &f.blocks[t]);
            // A panic in the block unwinds where the block says: its own
            // statements and what it takes must agree on where.
            if t == b
                || here.loc.is_some()
                || here.cleanup != there.cleanup
                || !(here.stmts.is_empty() || here.unwind == there.unwind)
            {
                break;
            }
            // A drop ends its block outside cleanup (`validate`).
            let after_drop = !here.cleanup
                && matches!(here.stmts.last(), Some(s) if matches!(s.kind, crate::StatementKind::Drop { .. }));
            let take = preds[t] == 1 && t != 0 && !(after_drop && !there.stmts.is_empty());
            if !take && (!there.stmts.is_empty() || matches!(there.term, Terminator::Goto(_))) {
                break;
            }
            // The block's unwinding edge becomes the other block's.
            if let Some(u) = here.unwind {
                preds[u.0 as usize] = preds[u.0 as usize].saturating_sub(1);
            }
            if let Some(u) = there.unwind {
                preds[u.0 as usize] += 1;
            }
            let taken = if take {
                // Nothing else jumps there: the block takes all of it.
                preds[t] = 0;
                if let Some(u) = there.unwind {
                    preds[u.0 as usize] = preds[u.0 as usize].saturating_sub(1);
                }
                std::mem::replace(
                    &mut f.blocks[t],
                    crate::BasicBlock {
                        stmts: Vec::new(),
                        term: Terminator::Unreachable,
                        unwind: None,
                        cleanup: false,
                        loc: None,
                    },
                )
            } else {
                // A block without statements: the block takes its
                // terminator, and the jump there is gone.
                preds[t] = preds[t].saturating_sub(1);
                for x in targets(&there.term) {
                    preds[x] += 1;
                }
                there.clone()
            };
            let here = &mut f.blocks[b];
            here.unwind = taken.unwind;
            here.stmts.extend(taken.stmts);
            here.term = taken.term;
            here.loc = taken.loc;
        }
    }
    // The blocks still reached, in their order.
    let live = reached(f);
    let mut index = vec![0usize; n];
    let mut kept = 0;
    for (b, l) in live.iter().enumerate() {
        index[b] = kept;
        kept += usize::from(*l);
    }
    let blocks = std::mem::take(&mut f.blocks);
    f.blocks = blocks.into_iter().zip(&live).filter(|(_, l)| **l).map(|(b, _)| b).collect();
    for blk in &mut f.blocks {
        retarget(&mut blk.term, |b| index[b]);
        if let Some(u) = &mut blk.unwind {
            *u = BlockId(index[u.0 as usize] as u32);
        }
    }
}
