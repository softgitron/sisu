//! Workload bodies shared by the benchmarks.
//!
//! Every function a task body calls starts with `p.check()`, the probe a
//! compiler would insert at function entry. Generic over [`StackProbe`], so the
//! same source runs with real probes (B, C, D(B)), with none (A, D(A)) and, for
//! `probe-overhead`, both ways on one stack.

use std::hint::black_box;

use crate::stack::probe::{StackProbe, current_sp};

/// "Shallow use": a few nested calls with small frames (about 1 KiB of stack).
#[inline(never)]
pub fn shallow_work<P: StackProbe>(p: P, seed: u64) -> u64 {
    p.check();
    let buf = black_box([seed as u8; 192]);
    shallow_inner(p, seed ^ buf[7] as u64, 3)
}

#[inline(never)]
fn shallow_inner<P: StackProbe>(p: P, x: u64, depth: u32) -> u64 {
    p.check();
    let buf = black_box([x as u8; 128]);
    if depth == 0 {
        return buf[5] as u64 + x;
    }
    shallow_inner(p, x.wrapping_mul(31) ^ buf[3] as u64, depth - 1)
}

/// One stack excursion of about `bytes` below the caller, touching every page
/// on the way down; returns the lowest stack pointer reached.
#[inline(never)]
pub fn excursion<P: StackProbe>(p: P, bytes: usize) -> usize {
    p.check();
    let mut page = [0u8; 4000];
    let mut i = 0;
    while i < page.len() {
        page[i] = i as u8;
        i += 512;
    }
    black_box(&mut page);
    let here = current_sp();
    if bytes > 4096 {
        excursion(p, bytes - 4096).min(here)
    } else {
        here
    }
}

/// Recursive Fibonacci: the call-dominated worst case for probes.
#[inline(never)]
pub fn fib<P: StackProbe>(p: P, n: u32) -> u64 {
    p.check();
    if n < 2 {
        return n as u64;
    }
    fib(p, n - 1) + fib(p, n - 2)
}

#[inline(never)]
fn mix<P: StackProbe>(p: P, x: u64) -> u64 {
    p.check();
    (x.wrapping_mul(0x9E37_79B9_7F4A_7C15).rotate_left(23)) ^ (x >> 7)
}

/// A compute loop calling a small function per iteration.
#[inline(never)]
pub fn call_loop<P: StackProbe>(p: P, n: u64) -> u64 {
    p.check();
    let mut x = 1u64;
    for i in 0..n {
        x = mix(p, x ^ i);
    }
    x
}

/// A binary tree for the recursive-walk workload.
pub struct Tree {
    pub value: u64,
    pub left: Option<Box<Tree>>,
    pub right: Option<Box<Tree>>,
}

/// Builds a complete binary tree of the given depth.
pub fn build_tree(depth: u32, seed: u64) -> Tree {
    Tree {
        value: seed.wrapping_mul(2654435761) % 1000,
        left: (depth > 0).then(|| Box::new(build_tree(depth - 1, seed * 2 + 1))),
        right: (depth > 0).then(|| Box::new(build_tree(depth - 1, seed * 2 + 2))),
    }
}

/// Sums a tree recursively, with a little arithmetic per node.
#[inline(never)]
pub fn tree_sum<P: StackProbe>(p: P, t: &Tree) -> u64 {
    p.check();
    let mut s = t.value.wrapping_mul(t.value) % 7919;
    if let Some(l) = &t.left {
        s = s.wrapping_add(tree_sum(p, l));
    }
    if let Some(r) = &t.right {
        s = s.wrapping_add(tree_sum(p, r));
    }
    s
}

/// The compute-bound kernel of `starvation`: `n` rounds of an LCG and a mix.
#[inline(never)]
pub fn spin_iterations(x: u64, n: u64) -> u64 {
    let mut x = x;
    for _ in 0..n {
        x = x.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        x ^= x >> 29;
    }
    black_box(x)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stack::probe::{LimitProbe, NoProbe};

    #[test]
    fn workloads_agree_with_and_without_probes() {
        let lp = LimitProbe::new(current_sp() - 1024 * 1024);
        assert_eq!(fib(NoProbe, 20), 6765);
        assert_eq!(fib(lp, 20), 6765);
        assert_eq!(call_loop(NoProbe, 1000), call_loop(lp, 1000));
        let t = build_tree(6, 1);
        assert_eq!(tree_sum(NoProbe, &t), tree_sum(lp, &t));
        assert_eq!(shallow_work(NoProbe, 3), shallow_work(lp, 3));
    }

    #[test]
    fn excursion_reaches_its_depth() {
        let top = current_sp();
        let low = excursion(NoProbe, 32 * 1024);
        let span = top - low;
        assert!((28 * 1024..48 * 1024).contains(&span), "excursion span {span}");
    }
}
