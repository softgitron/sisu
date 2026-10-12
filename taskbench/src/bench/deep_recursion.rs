//! `deep-recursion`: running out of stack inside a task must be a clean, reported
//! "stack exhausted" failure on every strategy, never a segfault.
//!
//! Two cases run in their own task (and scope):
//!
//! * `parse`: a recursive-descent parse of a generated expression nested
//!   `--parse-depth` (10k) parentheses deep; every nesting level costs four frames
//!   (expr, term, factor, primary), so whether it fits depends on the reservation.
//!   The AST's `Drop` is iterative, so dropping a deep tree cannot overflow.
//! * `naive`: a recursion `--naive-depth` (1M) calls deep, which exceeds every
//!   reservation up to 8 MiB.
//!
//! For each case the run records whether it completed or failed with "stack
//! exhausted" (and by which mechanism), the depth reached, the time, and whether
//! the destructors of the frames on the exhausted stack ran (`leaked_nodes` is
//! zero after an unwinding probe and non-zero after a guard-page trap, which
//! discards the frames). A follow-up task checks that the runtime still works.
//! A crash ends the process without these rows; `run_all.sh` records that.
//!
//! Strategy E runs the parse and a synchronous recursion on the worker thread's
//! stack (a stackless design runs non-suspending code there), plus `naive-async`,
//! a recursion of boxed futures, which is what a colourless stackless compilation
//! makes of a recursive function that may suspend.

use std::future::Future;
use std::hint::black_box;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{
    AtomicI64, AtomicU64,
    Ordering::{Relaxed, SeqCst},
};
use std::time::{Duration, Instant};

use super::{BenchCtx, Impl, params};
use crate::csv::Status;
use crate::rt::{self, AsyncScope, Runtime, TaskError};
use crate::stack::probe::{SplitStackProbe, StackProbe};

/// Live AST nodes; zero after a case means every destructor ran.
pub static LIVE_NODES: AtomicI64 = AtomicI64::new(0);
/// Live drop guards of the naive recursion (one every 256 levels).
pub static LIVE_GUARDS: AtomicI64 = AtomicI64::new(0);

/// A value with a destructor, held by some frames of the naive recursion.
struct DropGuard;

impl DropGuard {
    fn new() -> DropGuard {
        LIVE_GUARDS.fetch_add(1, Relaxed);
        DropGuard
    }
}

impl Drop for DropGuard {
    fn drop(&mut self) {
        LIVE_GUARDS.fetch_sub(1, Relaxed);
    }
}

/// An expression tree node.
pub struct Node {
    pub op: u8,
    pub value: i64,
    pub lhs: Option<Box<Node>>,
    pub rhs: Option<Box<Node>>,
}

impl Node {
    fn new(op: u8, value: i64, lhs: Option<Box<Node>>, rhs: Option<Box<Node>>) -> Box<Node> {
        LIVE_NODES.fetch_add(1, Relaxed);
        Box::new(Node { op, value, lhs, rhs })
    }
}

impl Drop for Node {
    /// Iterative, so that dropping a deep tree does not recurse.
    fn drop(&mut self) {
        LIVE_NODES.fetch_sub(1, Relaxed);
        let mut stack: Vec<Box<Node>> = Vec::new();
        stack.extend(self.lhs.take());
        stack.extend(self.rhs.take());
        while let Some(mut n) = stack.pop() {
            stack.extend(n.lhs.take());
            stack.extend(n.rhs.take());
            // `n` has no children now; dropping it does not recurse.
        }
    }
}

/// `(((1+2)+2)+2)...` nested `depth` deep; evaluates to `1 + 2 * depth`.
pub fn gen_input(depth: usize) -> String {
    let mut s = String::with_capacity(depth * 4 + 1);
    s.extend(std::iter::repeat_n('(', depth));
    s.push('1');
    for _ in 0..depth {
        s.push_str("+2)");
    }
    s
}

/// Recursive-descent parser with a probe at the entry of every production.
pub struct Parser<'a, P: StackProbe> {
    src: &'a [u8],
    pos: usize,
    probe: P,
    depth: u64,
    reached: &'a AtomicU64,
}

impl<'a, P: StackProbe> Parser<'a, P> {
    pub fn new(src: &'a str, probe: P, reached: &'a AtomicU64) -> Self {
        Parser { src: src.as_bytes(), pos: 0, probe, depth: 0, reached }
    }

    fn peek(&self) -> u8 {
        self.src.get(self.pos).copied().unwrap_or(0)
    }

    #[inline(never)]
    pub fn expr(&mut self) -> Result<Box<Node>, String> {
        self.probe.check();
        let mut lhs = self.term()?;
        while matches!(self.peek(), b'+' | b'-') {
            let op = self.peek();
            self.pos += 1;
            let rhs = self.term()?;
            lhs = Node::new(op, 0, Some(lhs), Some(rhs));
        }
        Ok(lhs)
    }

    #[inline(never)]
    fn term(&mut self) -> Result<Box<Node>, String> {
        self.probe.check();
        let mut lhs = self.factor()?;
        while matches!(self.peek(), b'*' | b'/') {
            let op = self.peek();
            self.pos += 1;
            let rhs = self.factor()?;
            lhs = Node::new(op, 0, Some(lhs), Some(rhs));
        }
        Ok(lhs)
    }

    #[inline(never)]
    fn factor(&mut self) -> Result<Box<Node>, String> {
        self.probe.check();
        if self.peek() == b'-' {
            self.pos += 1;
            let inner = self.factor()?;
            return Ok(Node::new(b'n', 0, Some(inner), None));
        }
        self.primary()
    }

    #[inline(never)]
    fn primary(&mut self) -> Result<Box<Node>, String> {
        self.probe.check();
        match self.peek() {
            b'(' => {
                self.pos += 1;
                self.depth += 1;
                // A plain store per level: the last one before exhaustion is the deepest.
                self.reached.store(self.depth, Relaxed);
                let e = self.expr()?;
                if self.peek() != b')' {
                    return Err(format!("expected ')' at {}", self.pos));
                }
                self.pos += 1;
                self.depth -= 1;
                Ok(e)
            }
            c if c.is_ascii_digit() => {
                let start = self.pos;
                while self.peek().is_ascii_digit() {
                    self.pos += 1;
                }
                let v: i64 = std::str::from_utf8(&self.src[start..self.pos]).unwrap_or("0").parse().unwrap_or(0);
                Ok(Node::new(b'#', v, None, None))
            }
            c => Err(format!("unexpected {:?} at {}", c as char, self.pos)),
        }
    }
}

/// Evaluates a tree without recursion.
pub fn eval(root: &Node) -> i64 {
    let mut values: Vec<i64> = Vec::new();
    let mut stack: Vec<(&Node, bool)> = vec![(root, false)];
    while let Some((n, visited)) = stack.pop() {
        if !visited {
            stack.push((n, true));
            if let Some(r) = &n.rhs {
                stack.push((r, false));
            }
            if let Some(l) = &n.lhs {
                stack.push((l, false));
            }
            continue;
        }
        let v = match n.op {
            b'#' => n.value,
            b'n' => -values.pop().unwrap_or(0),
            op => {
                let r = values.pop().unwrap_or(0);
                let l = values.pop().unwrap_or(0);
                match op {
                    b'+' => l.wrapping_add(r),
                    b'-' => l.wrapping_sub(r),
                    b'*' => l.wrapping_mul(r),
                    _ => {
                        if r == 0 {
                            0
                        } else {
                            l / r
                        }
                    }
                }
            }
        };
        values.push(v);
    }
    values.pop().unwrap_or(0)
}

/// Parses and evaluates the generated input.
pub fn parse_case<P: StackProbe>(probe: P, depth: usize, reached: &AtomicU64) -> Result<i64, String> {
    let src = gen_input(depth);
    let tree = Parser::new(&src, probe, reached).expr()?;
    reached.fetch_max(depth as u64, Relaxed);
    Ok(eval(&tree))
}

/// `depth` levels of plain recursion with a small frame; every 256th frame holds
/// a value with a destructor, so a run shows whether exhaustion ran them.
#[inline(never)]
pub fn naive<P: StackProbe>(p: P, depth: u64, target: u64, reached: &AtomicU64) -> u64 {
    p.check();
    reached.store(depth, Relaxed);
    if depth == target {
        return 0;
    }
    if depth.is_multiple_of(256) {
        let guard = DropGuard::new();
        let r = naive(p, black_box(depth + 1), target, reached);
        drop(guard);
        return black_box(r) + 1;
    }
    let r = naive(p, black_box(depth + 1), target, reached);
    black_box(r) + 1
}

type BoxFut = Pin<Box<dyn Future<Output = u64> + Send>>;

/// The stackless version: one boxed future per level.
pub fn naive_async(depth: u64, target: u64, reached: Arc<AtomicU64>) -> BoxFut {
    Box::pin(async move {
        SplitStackProbe.check();
        reached.store(depth, Relaxed);
        if depth == target {
            return 0;
        }
        let guard = depth.is_multiple_of(256).then(DropGuard::new);
        let r = naive_async(depth + 1, target, reached).await;
        drop(guard);
        black_box(r) + 1
    })
}

/// The outcome of one case.
#[derive(Clone, Debug)]
pub struct CaseResult {
    pub name: &'static str,
    pub target: u64,
    pub outcome: Result<i64, TaskError>,
    pub reached: u64,
    pub elapsed: Duration,
    pub leaked_nodes: i64,
    /// Drop guards of the naive recursion whose destructor never ran.
    pub leaked_guards: i64,
    /// Bytes of the probe reserve the unwinder used (B, C, D(B)), measured by
    /// painting the reserve before the case and scanning it afterwards.
    pub reserve_used: Option<usize>,
}

const PAINT: u8 = 0xA5;

/// Paints the probe reserve of the current task's stack; returns the range.
fn paint_reserve() -> Option<(usize, usize)> {
    let r = rt::stack_region()?;
    if r.probe_limit == 0 {
        return None;
    }
    for a in r.usable_low..r.probe_limit {
        // SAFETY: the reserve lies below the probe limit of this task's own stack:
        // mapped, owned by this task and not in use while the stack is shallow.
        unsafe { std::ptr::write_volatile(a as *mut u8, PAINT) };
    }
    Some((r.usable_low, r.probe_limit))
}

/// How many bytes of a painted reserve have been overwritten (from the top).
fn reserve_used((low, limit): (usize, usize)) -> usize {
    let mut a = low;
    // SAFETY: as in `paint_reserve`.
    while a < limit && unsafe { std::ptr::read_volatile(a as *const u8) } == PAINT {
        a += 1;
    }
    limit - a
}

pub fn run(ctx: &mut BenchCtx) -> Result<(), String> {
    let parse_depth = ctx.args.count("parse-depth", 10_000)? as usize;
    let naive_depth = ctx.args.count("naive-depth", 1_000_000)?;
    let p = params(&[("parse_depth", parse_depth.to_string()), ("naive_depth", naive_depth.to_string())]);
    ctx.banner(&p);
    let (cases, healthy) = match ctx.args.imp.clone() {
        Impl::Tokio => {
            crate::tokio_ref::deep_recursion(ctx.args.threads, ctx.args.worker_stack, parse_depth, naive_depth)?
        }
        Impl::OsThreads => {
            ctx.outcome(
                &p,
                None,
                "deep_recursion",
                Status::Skipped,
                "not implemented for os-threads (Rust aborts on thread stack overflow)",
            );
            return Ok(());
        }
        Impl::Taskbench(cfg) => {
            let rt = Runtime::with_worker_stack(ctx.args.threads, &cfg, ctx.args.worker_stack)?;
            if ctx.args.imp.is_stackless() {
                stackless(&rt, parse_depth, naive_depth)
            } else {
                stackful(&rt, parse_depth, naive_depth)
            }
        }
    };
    let how_expected = match &ctx.args.imp {
        Impl::Taskbench(c) => crate::stack::build(c).map(|s| s.exhaustion().as_str().to_string()).unwrap_or_default(),
        _ => "worker-stack-probe".into(),
    };
    for c in &cases {
        let (status, outcome, note) = match &c.outcome {
            Ok(v) => (Status::Ok, "completed", format!("value={v}")),
            Err(TaskError::StackExhausted { how, detail }) => {
                (Status::Ok, "stack_exhausted", format!("how={} {detail}", how.as_str()))
            }
            Err(e) => (Status::Failed, "error", e.to_string()),
        };
        let mut r = ctx.row(&format!("{}_outcome", c.name), Some(if outcome == "error" { 0.0 } else { 1.0 }), "clean");
        r.params = p.clone();
        r.status = status;
        r.note = format!("{outcome}; expected mechanism {how_expected}; {note}");
        ctx.emit(r);
        ctx.metric(&p, None, &format!("{}_target_depth", c.name), c.target as f64, "levels");
        ctx.metric(&p, None, &format!("{}_depth_reached", c.name), c.reached as f64, "levels");
        ctx.metric(&p, None, &format!("{}_elapsed", c.name), c.elapsed.as_secs_f64() * 1e3, "ms");
        if c.name == "parse" {
            ctx.metric(&p, None, "parse_leaked_nodes", c.leaked_nodes as f64, "count");
        } else {
            ctx.metric(&p, None, &format!("{}_leaked_drop_guards", c.name), c.leaked_guards as f64, "count");
        }
        if let Some(u) = c.reserve_used {
            ctx.metric(&p, None, &format!("{}_probe_reserve_used", c.name), u as f64, "bytes");
        }
    }
    let mut r = ctx.row("runtime_healthy_after", Some(if healthy { 1.0 } else { 0.0 }), "bool");
    r.params = p.clone();
    r.status = if healthy { Status::Ok } else { Status::Failed };
    ctx.emit(r);
    Ok(())
}

fn run_case<F>(name: &'static str, target: u64, f: F) -> CaseResult
where
    F: FnOnce(&AtomicU64) -> Result<Result<i64, String>, TaskError> + Send,
{
    let reached = AtomicU64::new(0);
    let used = std::sync::Mutex::new(None);
    let live0 = LIVE_NODES.load(SeqCst);
    let guards0 = LIVE_GUARDS.load(SeqCst);
    let t0 = Instant::now();
    let r = rt::scope(|s| {
        s.spawn(|| {
            let painted = paint_reserve();
            let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&reached)));
            *used.lock().expect("lock") = painted.map(reserve_used);
            match out {
                Ok(v) => v,
                Err(p) => std::panic::resume_unwind(p),
            }
        })
        .join()
    })
    .unwrap_or_else(Err);
    let elapsed = t0.elapsed();
    let outcome = match r {
        Ok(Ok(Ok(v))) => Ok(v),
        Ok(Ok(Err(msg))) => Err(TaskError::Panicked(format!("parse error: {msg}"))),
        Ok(Err(e)) | Err(e) => Err(e),
    };
    let reserve_used = used.into_inner().expect("lock");
    CaseResult {
        name,
        target,
        outcome,
        reached: reached.load(SeqCst),
        elapsed,
        leaked_nodes: LIVE_NODES.load(SeqCst) - live0,
        leaked_guards: LIVE_GUARDS.load(SeqCst) - guards0,
        reserve_used,
    }
}

fn stackful(rt: &Runtime, parse_depth: usize, naive_depth: u64) -> (Vec<CaseResult>, bool) {
    rt.block_on(|| {
        let cases = vec![
            run_case("parse", parse_depth as u64, |reached| Ok(parse_case(SplitStackProbe, parse_depth, reached))),
            run_case("naive", naive_depth, |reached| Ok(Ok(naive(SplitStackProbe, 0, naive_depth, reached) as i64))),
        ];
        let healthy = rt::scope(|s| s.spawn(|| 7).join()).map(|r| r == Ok(7)).unwrap_or(false);
        (cases, healthy)
    })
}

fn stackless(rt: &Runtime, parse_depth: usize, naive_depth: u64) -> (Vec<CaseResult>, bool) {
    rt.block_on_async(async move {
        let mut cases = Vec::new();
        let s = AsyncScope::new();
        // Synchronous code inside a stackless task runs on the worker's stack.
        for (name, target) in [("parse", parse_depth as u64), ("naive", naive_depth), ("naive-async", naive_depth)] {
            let reached = Arc::new(AtomicU64::new(0));
            let r2 = reached.clone();
            let live0 = LIVE_NODES.load(SeqCst);
            let guards0 = LIVE_GUARDS.load(SeqCst);
            let t0 = Instant::now();
            let h = match name {
                "parse" => s.spawn(async move { parse_case(SplitStackProbe, parse_depth, &r2) }),
                "naive" => s.spawn(async move { Ok(naive(SplitStackProbe, 0, naive_depth, &r2) as i64) }),
                _ => s.spawn(async move { Ok(naive_async(0, naive_depth, r2).await as i64) }),
            };
            let outcome = match h.join().await {
                Ok(Ok(v)) => Ok(v),
                Ok(Err(msg)) => Err(TaskError::Panicked(format!("parse error: {msg}"))),
                Err(e) => Err(e),
            };
            cases.push(CaseResult {
                name,
                target,
                outcome,
                reached: reached.load(SeqCst),
                elapsed: t0.elapsed(),
                leaked_nodes: LIVE_NODES.load(SeqCst) - live0,
                leaked_guards: LIVE_GUARDS.load(SeqCst) - guards0,
                reserve_used: None,
            });
        }
        let healthy = s.spawn(async { 7 }).join().await == Ok(7);
        let _ = s.join_all().await;
        (cases, healthy)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stack::probe::NoProbe;

    #[test]
    fn parser_evaluates_generated_input() {
        let reached = AtomicU64::new(0);
        assert_eq!(parse_case(NoProbe, 100, &reached).unwrap(), 201);
        let r = AtomicU64::new(0);
        let tree = Parser::new("-(3*4)-2/(1+1)", NoProbe, &r).expr().unwrap();
        assert_eq!(eval(&tree), -13);
        drop(tree);
        assert!(Parser::new("(1+", NoProbe, &r).expr().is_err());
    }

    #[test]
    fn deep_tree_drops_without_recursion() {
        // A 200k-deep left spine, dropped on a 2 MiB thread stack.
        let h = std::thread::Builder::new()
            .stack_size(2 << 20)
            .spawn(|| {
                let mut n = Node::new(b'#', 1, None, None);
                for _ in 0..200_000 {
                    n = Node::new(b'n', 0, Some(n), None);
                }
                drop(n);
            })
            .unwrap();
        h.join().unwrap();
    }
}
