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
//!
//! `fib` is an upper bound, not an estimate. LLVM turns its second recursive call
//! into a loop, and the probe, written at the top of the source function, stays in
//! that loop: it runs about twice per machine-level call and keeps one more
//! callee-saved register live. A body of about six instructions per iteration
//! makes the two added instructions roughly a third of the time. A compiler
//! inserting the check into the prologue after optimisation would run it once
//! per call.

use std::hint::black_box;
use std::time::Instant;

use super::{BenchCtx, Impl, params, work};
use crate::csv::Status;
use crate::rt::{self, Runtime};
use crate::stack::probe::{
    LimitProbe, NoProbe, SplitStackProbe, StackProbe, TlsProbe, current_sp, set_split_stack_limit, set_tls_limit,
};
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

/// Instructions retired by one run of each variant, when hardware counters work.
fn count_instructions(arg: LimitProbe, w: Workload) -> Result<[u64; 4], String> {
    let c = crate::perf::Counters::open()?;
    let tree = work::build_tree(if let Workload::Tree(d) = w { d } else { 1 }, 1);
    let mut out = [0u64; 4];
    out[0] = c.measure(|| run_once(NoProbe, w, &tree)).2;
    out[1] = c.measure(|| run_once(SplitStackProbe, w, &tree)).2;
    out[2] = c.measure(|| run_once(arg, w, &tree)).2;
    out[3] = c.measure(|| run_once(TlsProbe, w, &tree)).2;
    Ok(out)
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
    let p = params(&[
        ("reps", reps.to_string()),
        ("fib_n", fib_n.to_string()),
        ("calls", calls.to_string()),
        ("tree_depth", tree_depth.to_string()),
    ]);
    ctx.banner(&p);
    type Measured = (Workload, [f64; 4], Result<[u64; 4], String>);
    let results: Vec<Measured> = match ctx.args.imp.clone() {
        Impl::Taskbench(cfg) if !ctx.args.imp.is_stackless() => {
            let rt = Runtime::with_worker_stack(ctx.args.threads, &cfg, ctx.args.worker_stack)?;
            let sel = selected.clone();
            rt.block_on(move || {
                // The strategy's probe limit, or (guard-page strategies) a real limit
                // anyway, so that every variant does its compare.
                let region = rt::stack_region().expect("stackful task");
                let limit = if region.probe_limit != 0 { region.probe_limit } else { region.usable_low + 16 * 1024 };
                let probe = LimitProbe::new(limit);
                assert!(current_sp() > probe.limit(), "stack too small for probe-overhead");
                // This task never yields during the measurement, so the
                // thread-local slot stays valid; the split-stack slot is set by the
                // scheduler but is pinned here to the same limit for comparability.
                set_tls_limit(probe.limit());
                set_split_stack_limit(probe.limit());
                sel.into_iter().map(|w| (w, compare(probe, w, reps), count_instructions(probe, w))).collect()
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
                let probe = LimitProbe::new(limit);
                sel.into_iter().map(|w| (w, compare(probe, w, reps), count_instructions(probe, w))).collect()
            })
        }
        other => {
            ctx.outcome(
                &p,
                None,
                "probe_overhead",
                Status::Skipped,
                &format!("{} has no software probes", other.implementation()),
            );
            return Ok(());
        }
    };
    let mut counters_note = None;
    for (w, [without, split, arg, tls], instr) in results {
        let name = w.name();
        ctx.metric(&p, None, &format!("{name}_without_probes"), without.round(), "ns");
        for (variant, t) in [("split", split), ("arg", arg), ("tls", tls)] {
            ctx.metric(&p, None, &format!("{name}_{variant}_probe"), t.round(), "ns");
            ctx.metric(&p, None, &format!("{name}_{variant}_probe_overhead"), (t / without - 1.0) * 100.0, "percent");
        }
        match instr {
            Ok([base, split, arg, tls]) => {
                ctx.metric(&p, None, &format!("{name}_instructions_without_probes"), base as f64, "count");
                for (variant, n) in [("split", split), ("arg", arg), ("tls", tls)] {
                    let pct = (n as f64 / base.max(1) as f64 - 1.0) * 100.0;
                    ctx.metric(&p, None, &format!("{name}_{variant}_probe_instruction_overhead"), pct, "percent");
                }
            }
            Err(e) => counters_note = Some(e),
        }
    }
    if let Some(e) = counters_note {
        ctx.outcome(&p, None, "instruction_counts", Status::Skipped, &e);
    }
    Ok(())
}
