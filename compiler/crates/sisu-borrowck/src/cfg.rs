//! The control-flow graph of a SIR function: successors (the terminator's
//! targets), the unwind edge every statement of a block may take, and
//! predecessors of both kinds.

use sisu_sir::{BasicBlock, Function, Terminator};

pub struct Cfg {
    pub pred: Vec<Vec<usize>>,
    /// Blocks in reverse post-order from the entry (unwind edges count),
    /// then any block the entry does not reach.
    pub order: Vec<usize>,
}

pub fn targets(b: &BasicBlock) -> Vec<usize> {
    match &b.term {
        Terminator::Goto(t) => vec![t.0 as usize],
        Terminator::Switch { cases, default, .. } => {
            let mut v: Vec<usize> = cases.iter().map(|(_, t)| t.0 as usize).collect();
            v.push(default.0 as usize);
            v
        }
        _ => Vec::new(),
    }
}

impl Cfg {
    pub fn new(f: &Function) -> Cfg {
        let n = f.blocks.len();
        let mut succ = vec![Vec::new(); n];
        let mut pred = vec![Vec::new(); n];
        for (i, b) in f.blocks.iter().enumerate() {
            let mut s = targets(b);
            if let Some(u) = b.unwind {
                s.push(u.0 as usize);
            }
            s.retain(|t| *t < n);
            s.sort_unstable();
            s.dedup();
            for &t in &s {
                pred[t].push(i);
            }
            succ[i] = s;
        }
        // Reverse post-order, iteratively.
        let mut seen = vec![false; n];
        let mut post = Vec::new();
        let mut stack: Vec<(usize, usize)> = Vec::new();
        if n > 0 {
            stack.push((0, 0));
            seen[0] = true;
        }
        while let Some(top) = stack.last_mut() {
            let (b, i) = *top;
            if i < succ[b].len() {
                top.1 += 1;
                let t = succ[b][i];
                if !seen[t] {
                    seen[t] = true;
                    stack.push((t, 0));
                }
            } else {
                post.push(b);
                stack.pop();
            }
        }
        post.reverse();
        let mut order = post;
        for (b, s) in seen.iter().enumerate() {
            if !s {
                order.push(b);
            }
        }
        Cfg { pred, order }
    }
}
