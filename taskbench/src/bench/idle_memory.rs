//! `idle-memory`: resident memory per parked task.
//!
//! N tasks each do a little shallow work and park on a gate; the process's
//! `VmRSS` and `VmPTE` are read (phase "shallow"). The gate opens, every task makes
//! one stack excursion of `--excursion` bytes (default 32 KiB) and parks again;
//! memory is read again (phase "excursion"). The difference between the phases is
//! commit creep: lazily committed stack pages stay resident after the excursion
//! unless the strategy decommits them (D). Per-task figures are differences from
//! a baseline read just before spawning, divided by the number of tasks.
//!
//! Strategy C sizes its exact stacks for the whole task body (shallow part plus the
//! excursion), as a compiler computing the bound would.
//!
//! Without decommit, every task keeps its excursion's pages: 1M tasks need about
//! 35 GiB after a 32 KiB excursion. So that a run at that scale on a 16 GiB
//! machine fails cleanly instead of meeting the OOM killer, the excursion phase
//! watches the memory headroom; once it is below `--mem-guard` (default 1 GiB)
//! the tasks that have not made their excursion yet skip it, and the phase is
//! reported as `failed` with the count. Spawning stops at the same guard.

use std::hint::black_box;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering::SeqCst};
use std::time::{Duration, Instant};

use super::{BenchCtx, Impl, params, work};
use crate::csv::Status;
use crate::mem::{self, MemSnapshot, per_item};
use crate::rt::{self, AsyncScope, Gate, Runtime};
use crate::stack::probe::{SplitStackProbe, current_sp};
use crate::stack::{StrategyId, probe as sprobe};

/// What an idle-memory run measured.
#[derive(Clone, Debug, Default)]
pub struct IdleResult {
    pub requested: u64,
    pub created: u64,
    pub failure: Option<String>,
    pub base: MemSnapshot,
    pub shallow: Option<MemSnapshot>,
    pub excursion: Option<MemSnapshot>,
    pub excursion_span: usize,
    /// Why the excursion phase has no valid figure (tasks skipped it), if so.
    pub excursion_failure: Option<String>,
    pub note: String,
}

pub fn run(ctx: &mut BenchCtx) -> Result<(), String> {
    let n = ctx.args.tasks.unwrap_or(10_000);
    let excursion = ctx.args.size("excursion", 32 * 1024)?;
    let settle = Duration::from_millis(ctx.args.count("settle-ms", 200)?);
    let guard = ctx.args.size("mem-guard", 1 << 30)? as u64;
    let mut p = params(&[
        ("excursion", excursion.to_string()),
        ("settle_ms", settle.as_millis().to_string()),
        ("mem_guard", guard.to_string()),
    ]);
    let res = match ctx.args.imp.clone() {
        Impl::Tokio => {
            ctx.banner(&p);
            crate::tokio_ref::idle_memory(ctx.args.threads, n, excursion, settle)?
        }
        Impl::OsThreads => {
            ctx.banner(&p);
            os_threads(n, excursion, settle, ctx.args.size("os-stack", 64 * 1024).unwrap_or(64 * 1024))
        }
        Impl::Taskbench(mut cfg) => {
            if cfg.id == StrategyId::C && excursion > 0 {
                cfg.exact += excursion;
                p.push_str(&format!(";c_exact_bytes={}", cfg.exact));
            }
            ctx.args.imp = Impl::Taskbench(cfg.clone());
            ctx.banner(&p);
            let rt = Runtime::with_worker_stack(ctx.args.threads, &cfg, ctx.args.worker_stack)?;
            if cfg.id == StrategyId::E {
                stackless(&rt, n, excursion, settle, guard)
            } else {
                stackful(&rt, n, excursion, settle, guard)
            }
        }
    };
    emit(ctx, &p, &res);
    Ok(())
}

/// Emits the per-task figures of `res`.
pub fn emit(ctx: &mut BenchCtx, p: &str, res: &IdleResult) {
    let tasks = Some(res.requested);
    let status = if res.failure.is_some() { Status::Failed } else { Status::Ok };
    let note = match &res.failure {
        Some(f) => format!("created {} of {}: {f}", res.created, res.requested),
        None => res.note.clone(),
    };
    let n = res.created;
    let put = |ctx: &mut BenchCtx, metric: &str, v: f64, unit: &str| {
        let mut r = ctx.row(metric, Some(v), unit);
        r.params = p.to_string();
        r.tasks = tasks;
        r.status = status;
        r.note = note.clone();
        ctx.emit(r);
    };
    put(ctx, "tasks_created", n as f64, "count");
    for (phase, snap) in [("shallow", res.shallow), ("excursion", res.excursion)] {
        if phase == "excursion"
            && let Some(why) = &res.excursion_failure
        {
            ctx.outcome(p, tasks, "total_per_task_excursion", Status::Failed, why);
            continue;
        }
        let Some(s) = snap else { continue };
        let b = &res.base;
        put(ctx, &format!("rss_per_task_{phase}"), per_item(b.vm_rss, s.vm_rss, n).round(), "bytes");
        put(ctx, &format!("pte_per_task_{phase}"), per_item(b.vm_pte, s.vm_pte, n).round(), "bytes");
        put(ctx, &format!("total_per_task_{phase}"), per_item(b.rss_plus_pte(), s.rss_plus_pte(), n).round(), "bytes");
        put(ctx, &format!("pss_per_task_{phase}"), per_item(b.pss, s.pss, n).round(), "bytes");
        put(ctx, &format!("rss_total_{phase}"), s.vm_rss as f64, "bytes");
        put(ctx, &format!("vmas_{phase}"), s.vmas as f64, "count");
    }
    if res.excursion.is_some() {
        put(ctx, "excursion_span", res.excursion_span as f64, "bytes");
    }
    if ctx.args.imp.config().is_some() {
        put(ctx, "baseline_rss", res.base.vm_rss as f64, "bytes");
    }
}

/// Waits (from a stackful task) until `gate` has `n` waiters, or a deadline.
/// With `guard`, also sets the stop flag once the memory headroom is below the
/// guard's bytes. The waiting task can do this while a million tasks are queued
/// because its sleep timer wakes it into a worker's `next` slot; Go's scheduler
/// queues the equivalent goroutine behind them, so the Go control checks the
/// guard in the goroutines instead.
fn wait_waiting(gate: &Gate, n: u64, deadline: Instant, guard: Option<(u64, &AtomicBool)>) -> bool {
    while (gate.waiting() as u64) < n {
        if Instant::now() > deadline {
            return false;
        }
        if let Some((bytes, stop)) = guard
            && !stop.load(SeqCst)
            && mem::mem_headroom().is_some_and(|h| h < bytes)
        {
            stop.store(true, SeqCst);
        }
        let _ = rt::sleep(Duration::from_millis(2));
    }
    true
}

const NOT_PARKED_AGAIN: &str = "tasks did not all park again after the excursion before the deadline";

fn deadline_for(n: u64) -> Instant {
    Instant::now() + Duration::from_secs(60) + Duration::from_micros(50 * n)
}

fn stackful(rt: &Runtime, n: u64, excursion: usize, settle: Duration, guard: u64) -> IdleResult {
    let gate = Gate::new();
    let span = AtomicUsize::new(0);
    let (stop, skipped) = (AtomicBool::new(false), AtomicU64::new(0));
    rt.block_on(|| {
        let mut res = IdleResult { requested: n, base: mem::snapshot(true), ..Default::default() };
        let r = rt::scope(|s| {
            for i in 0..n {
                let (g, sp, stop, skipped) = (&gate, &span, &stop, &skipped);
                let spawned = s.try_spawn(move || {
                    let p = SplitStackProbe;
                    black_box(work::shallow_work(p, i));
                    let _ = g.wait(0);
                    if excursion > 0 && stop.load(SeqCst) {
                        skipped.fetch_add(1, SeqCst);
                    } else if excursion > 0 {
                        let top = current_sp();
                        let low = work::excursion(p, excursion);
                        sp.fetch_max(top - low, SeqCst);
                    }
                    let _ = g.wait(1);
                });
                match spawned {
                    Ok(h) => {
                        drop(h);
                        res.created += 1;
                    }
                    Err(e) => {
                        res.failure = Some(e.to_string());
                        break;
                    }
                }
                if i % 1024 == 1023 {
                    let _ = rt::yield_now();
                }
                if i % 4096 == 4095 && mem::mem_headroom().is_some_and(|h| h < guard) {
                    res.failure = Some(format!("memory headroom below {guard} bytes"));
                    break;
                }
            }
            let deadline = deadline_for(n);
            if wait_waiting(&gate, res.created, deadline, None) {
                let _ = rt::sleep(settle);
                res.shallow = Some(mem::snapshot(true));
                if excursion > 0 {
                    gate.open();
                    if wait_waiting(&gate, res.created, deadline, Some((guard, &stop))) {
                        let _ = rt::sleep(settle);
                        res.excursion = Some(mem::snapshot(true));
                    } else {
                        res.excursion_failure = Some(NOT_PARKED_AGAIN.into());
                    }
                    let skipped = skipped.load(SeqCst);
                    if skipped > 0 {
                        res.excursion_failure = Some(format!(
                            "{skipped} of {} tasks skipped the excursion: memory headroom below {guard} bytes",
                            res.created
                        ));
                    }
                } else {
                    gate.open();
                }
            } else {
                res.failure.get_or_insert_with(|| "tasks did not all park before the deadline".into());
            }
            gate.open();
            gate.open();
        });
        if let Err(e) = r {
            res.failure.get_or_insert_with(|| format!("scope: {e}"));
        }
        let c = rt.stack_counters();
        res.note = format!(
            "reserved_bytes={} decommits={} decommitted_bytes={}",
            c.reserved_bytes, c.decommits, c.decommitted_bytes
        );
        res.excursion_span = span.load(SeqCst);
        res
    })
}

fn stackless(rt: &Runtime, n: u64, excursion: usize, settle: Duration, guard: u64) -> IdleResult {
    let gate = Arc::new(Gate::new());
    let span = Arc::new(AtomicUsize::new(0));
    rt.block_on_async(async move {
        let mut res = IdleResult { requested: n, base: mem::snapshot(true), ..Default::default() };
        let s = AsyncScope::new();
        for i in 0..n {
            let (g, sp) = (gate.clone(), span.clone());
            drop(s.spawn(async move {
                black_box(work::shallow_work(SplitStackProbe, i));
                let _ = g.wait_async(0).await;
                if excursion > 0 {
                    // Stackless: the excursion runs on the worker's stack inside one poll.
                    let top = current_sp();
                    let low = work::excursion(SplitStackProbe, excursion);
                    sp.fetch_max(top - low, SeqCst);
                }
                let _ = g.wait_async(1).await;
            }));
            res.created += 1;
            if i % 1024 == 1023 {
                let _ = rt::yield_now_async().await;
            }
            if i % 4096 == 4095 && mem::mem_headroom().is_some_and(|h| h < guard) {
                res.failure = Some(format!("memory headroom below {guard} bytes"));
                break;
            }
        }
        let deadline = deadline_for(n);
        let mut ok = true;
        while (gate.waiting() as u64) < res.created {
            if Instant::now() > deadline {
                ok = false;
                break;
            }
            let _ = rt::sleep_async(Duration::from_millis(2)).await;
        }
        if ok {
            let _ = rt::sleep_async(settle).await;
            res.shallow = Some(mem::snapshot(true));
            gate.open();
            if excursion > 0 {
                while (gate.waiting() as u64) < res.created && Instant::now() < deadline {
                    let _ = rt::sleep_async(Duration::from_millis(2)).await;
                }
                if (gate.waiting() as u64) < res.created {
                    res.excursion_failure = Some(NOT_PARKED_AGAIN.into());
                } else {
                    let _ = rt::sleep_async(settle).await;
                    res.excursion = Some(mem::snapshot(true));
                }
            }
        } else {
            res.failure.get_or_insert_with(|| "tasks did not all park before the deadline".into());
        }
        gate.open();
        gate.open();
        let _ = s.join_all().await;
        res.excursion_span = span.load(SeqCst);
        res.note = format!("future_task_bytes~{}", std::mem::size_of::<crate::rt::task::Task>());
        res
    })
}

/// Reference: one parked OS thread per task.
fn os_threads(n: u64, excursion: usize, settle: Duration, stack: usize) -> IdleResult {
    use std::sync::{Condvar, Mutex};
    let gate = Arc::new((Mutex::new((0u64, 0u64)), Condvar::new())); // (phase, waiting)
    let mut res = IdleResult { requested: n, base: mem::snapshot(true), ..Default::default() };
    let mut handles = Vec::new();
    let span = Arc::new(AtomicUsize::new(0));
    for i in 0..n {
        let (g, sp) = (gate.clone(), span.clone());
        let h = std::thread::Builder::new().stack_size(stack.max(excursion + 32 * 1024)).spawn(move || {
            let wait = |seen: u64| {
                let (m, cv) = &*g;
                let mut st = m.lock().unwrap();
                st.1 += 1;
                while st.0 <= seen {
                    st = cv.wait(st).unwrap();
                }
            };
            black_box(work::shallow_work(sprobe::NoProbe, i));
            wait(0);
            if excursion > 0 {
                let top = current_sp();
                sp.fetch_max(top - work::excursion(sprobe::NoProbe, excursion), SeqCst);
            }
            wait(1);
        });
        match h {
            Ok(h) => {
                handles.push(h);
                res.created += 1;
            }
            Err(e) => {
                res.failure = Some(format!("thread spawn: {e}"));
                break;
            }
        }
    }
    let wait_all = |expect: u64| {
        let (m, _) = &*gate;
        let deadline = Instant::now() + Duration::from_secs(60);
        while m.lock().unwrap().1 < expect && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(2));
        }
    };
    let open = || {
        let (m, cv) = &*gate;
        let mut st = m.lock().unwrap();
        st.0 += 1;
        st.1 = 0;
        cv.notify_all();
    };
    wait_all(res.created);
    std::thread::sleep(settle);
    res.shallow = Some(mem::snapshot(true));
    if excursion > 0 {
        open();
        wait_all(res.created);
        std::thread::sleep(settle);
        res.excursion = Some(mem::snapshot(true));
    } else {
        open();
    }
    open();
    open();
    for h in handles {
        let _ = h.join();
    }
    res.excursion_span = span.load(SeqCst);
    res.note = format!("os_thread_stack={stack}");
    res
}
