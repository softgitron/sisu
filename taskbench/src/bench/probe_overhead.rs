//! `probe-overhead`: the cost of software stack-limit probes on compute loops.
//!
//! The same workloads are compiled three times and run alternately `--reps` times
//! inside one task on the selected strategy's stack:
//!
//! * `without`: [`NoProbe`];
//! * `split`: [`SplitStackProbe`], `cmp rsp, fs:[0x70]; jb`, the prologue GCC's
//!   `-fsplit-stack` emits and the probe the harness's task bodies use: the cost
//!   model of a compiler-inserted check (Go's `stackguard0` check is the same
//!   shape through a pinned register);
//! * `arg`: [`LimitProbe`], the limit threaded through every call as an argument;
//! * `tls`: [`TlsProbe`], a Rust `thread_local!` read (three dependent
//!   instructions, and not migration-safe).
//!
//! The bet-3 threshold: at most 3 % on compute loops if pooled stacks with
//! software probes become the design. A compiler would also omit the check in
//! leaf functions with small frames (Go's `NOSPLIT` rule), which this harness does
//! not, so `calls` (whose callee is such a leaf) overstates the cost.
//!
//! Workloads: `fib` (recursive Fibonacci, the call-dominated worst case),
//! `calls` (a loop calling a small function per iteration), `tree` (a recursive
//! walk of a 64k-node binary tree with arithmetic per node).

use std::hint::black_box;
use std::time::Instant;

use super::{BenchCtx, Impl, params, work};
use crate::csv::Status;
use crate::rt::{self, Runtime};
use crate::stack::probe::{LimitProbe, NoProbe, SplitStackProbe, StackProbe, TlsProbe, current_sp, set_split_stack_limit, set_tls_limit};
use crate::stats::median_f64;

#[derive(Clone, Copy)]
enum Workload {
    Fib(u32),
    Calls(u64),
    Tree(u32),
}

impl Workload {
    fn name(self) -> &'static str {
        match self {
            Workload::Fib(_) => "fib",
            Workload::Calls(_) => "calls",
            Workload::Tree(_) => "tree",
        }
    }
}

fn run_once<P: StackProbe>(p: P, w: Workload, tree: &work::Tree) -> (f64, u64) {
    let t0 = Instant::now();
    let v = match w {
        Workload::Fib(n) => work::fib(p, black_box(n)),
        Workload::Calls(n) => work::call_loop(p, black_box(n)),
        Workload::Tree(_) => (0..64).fold(0u64, |a, _| a.wrapping_add(work::tree_sum(p, black_box(tree)))),
    };
    (t0.elapsed().as_nanos() as f64, black_box(v))
}

/// Median times (ns) of the four variants, rotating their order to cancel drift.
fn compare(arg: LimitProbe, w: Workload, reps: usize) -> [f64; 4] {
    let tree = work::build_tree(if let Workload::Tree(d) = w { d } else { 1 }, 1);
    let variants: [&dyn Fn() -> (f64, u64); 4] = [
        &|| run_once(NoProbe, w, &tree),
        &|| run_once(SplitStackProbe, w, &tree),
        &|| run_once(arg, w, &tree),
        &|| run_once(TlsProbe, w, &tree),
    ];
    // Warm up every variant and check they compute the same thing.
    let results: Vec<u64> = variants.iter().map(|f| f().1).collect();
    assert!(results.windows(2).all(|p| p[0] == p[1]), "a probe changed the result");
    let mut times: [Vec<f64>; 4] = Default::default();
    for i in 0..reps {
        for k in 0..4 {
            let v = (i + k) % 4;
            times[v].push(variants[v]().0);
        }
    }
    times.map(|mut t| median_f64(&mut t))
}

pub fn run(ctx: &mut BenchCtx) -> Result<(), String> {
    let reps = ctx.args.count("reps", 11)?.max(1) as usize;
    let which = ctx.args.opt("workload").unwrap_or("all").to_string();
    let fib_n = ctx.args.count("fib-n", 32)? as u32;
    let calls = ctx.args.count("calls", 20_000_000)?;
    let tree_depth = ctx.args.count("tree-depth", 15)? as u32;
    let all = [Workload::Fib(fib_n), Workload::Calls(calls), Workload::Tree(tree_depth)];
    let selected: Vec<Workload> = all.into_iter().filter(|w| which == "all" || which == w.name()).collect();
    if selected.is_empty() {
        return Err(format!("unknown --workload {which:?} (fib, calls, tree, all)"));
    }
    let p = params(&[("reps", reps.to_string()), ("fib_n", fib_n.to_string()), ("calls", calls.to_string()), ("tree_depth", tree_depth.to_string())]);
    ctx.banner(&p);
    let results: Vec<(Workload, [f64; 4])> = match ctx.args.imp.clone() {
        Impl::Taskbench(cfg) if !ctx.args.imp.is_stackless() => {
            let rt = Runtime::with_worker_stack(ctx.args.threads, &cfg, ctx.args.worker_stack)?;
            let sel = selected.clone();
            rt.block_on(move || {
                // A real limit even on guard-page strategies, so the probe does its compare.
                let region = rt::stack_region().expect("stackful task");
                let probe = LimitProbe::new(region.usable_low + 16 * 1024);
                assert!(current_sp() > probe.limit());
                // This task never yields during the measurement, so the
                // thread-local slot stays valid; the split-stack slot is set by the
                // scheduler but is pinned here to the same limit for comparability.
                set_tls_limit(probe.limit());
                set_split_stack_limit(probe.limit());
                sel.into_iter().map(|w| (w, compare(probe, w, reps))).collect()
            })
        }
        Impl::Taskbench(cfg) => {
            // Stackless: the probe reads the worker thread's limit from a thread-local.
            let rt = Runtime::with_worker_stack(ctx.args.threads, &cfg, ctx.args.worker_stack)?;
            let sel = selected.clone();
            rt.block_on_async(async move {
                let limit = crate::stack::probe::worker_limit();
                set_tls_limit(limit);
                set_split_stack_limit(limit);
                sel.into_iter().map(|w| (w, compare(LimitProbe::new(limit), w, reps))).collect()
            })
        }
        other => {
            ctx.outcome(&p, None, "probe_overhead", Status::Skipped, &format!("{} has no software probes", other.implementation()));
            return Ok(());
        }
    };
    for (w, [without, split, arg, tls]) in results {
        let name = w.name();
        ctx.metric(&p, None, &format!("{name}_without_probes"), without.round(), "ns");
        for (variant, t) in [("split", split), ("arg", arg), ("tls", tls)] {
            ctx.metric(&p, None, &format!("{name}_{variant}_probe"), t.round(), "ns");
            ctx.metric(&p, None, &format!("{name}_{variant}_probe_overhead"), (t / without - 1.0) * 100.0, "percent");
        }
    }
    Ok(())
}
