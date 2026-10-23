//! `max-tasks`: how many parked tasks can exist before something runs out, and
//! which limit it was.
//!
//! With `--hold-secs S`, a run that reached the cap keeps every task parked for
//! another S seconds and then checks that all are still parked and reads memory
//! again: the scale row of the pre-registration (proposal/20, section 6.5) passes
//! when all 1M tasks park and the process survives 60 seconds.
//!
//! Tasks are spawned one at a time (each does shallow work and parks on a gate)
//! until a spawn fails, `--tasks` (the cap) is reached, or the memory headroom
//! (`MemAvailable`, or the memory cgroup's limit minus its usage when that is
//! smaller) drops below `--mem-guard` (so a development machine is not driven
//! into the OOM killer; on the full-scale machine set the guard low). The limit is reported as
//! `vma_count` (`vm.max_map_count`, which a guard page per task hits at about
//! 32k), `memory`, `threads` (OS-thread reference: `kernel.threads-max`,
//! `kernel.pid_max`, `RLIMIT_NPROC` or the cgroup's `pids.max`), `mem_guard` or
//! `cap`.

use std::hint::black_box;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::{BenchCtx, Impl, params, work};
use crate::csv::Status;
use crate::mem::{self, MemSnapshot};
use crate::rt::{self, AsyncScope, Gate, Runtime, TaskError};
use crate::stack::probe::SplitStackProbe;

/// What a max-tasks run found.
#[derive(Clone, Debug, Default)]
pub struct MaxResult {
    pub cap: u64,
    pub created: u64,
    /// `vma_count`, `memory`, `threads`, `mem_guard` or `cap`.
    pub limit: String,
    pub detail: String,
    pub base: MemSnapshot,
    pub at_stop: MemSnapshot,
    pub spawn_secs: f64,
    /// With `--hold-secs`, after the hold: seconds held, tasks still parked, memory.
    pub held: Option<Held>,
}

/// What a hold after reaching the cap found.
#[derive(Clone, Copy, Debug, Default)]
pub struct Held {
    pub secs: f64,
    pub parked: u64,
    pub mem: MemSnapshot,
}

pub fn run(ctx: &mut BenchCtx) -> Result<(), String> {
    let default_cap = if ctx.args.imp == Impl::OsThreads { 10_000 } else { 100_000 };
    let cap = ctx.args.tasks.unwrap_or(default_cap);
    let guard = ctx.args.size("mem-guard", 1 << 30)? as u64;
    let hold = ctx.args.secs("hold-secs", 0.0)?;
    let mut pairs = vec![("cap", cap.to_string()), ("mem_guard", guard.to_string())];
    if !hold.is_zero() {
        pairs.push(("hold_s", format!("{}", hold.as_secs_f64())));
    }
    let p = params(&pairs);
    ctx.banner(&p);
    let res = match ctx.args.imp.clone() {
        Impl::Tokio => crate::tokio_ref::max_tasks(ctx.args.threads, cap, guard, hold)?,
        Impl::OsThreads => os_threads(cap, guard, ctx.args.size("os-stack", 64 * 1024)?, hold),
        Impl::Taskbench(cfg) => {
            let rt = Runtime::with_worker_stack(ctx.args.threads, &cfg, ctx.args.worker_stack)?;
            if ctx.args.imp.is_stackless() { stackless(&rt, cap, guard, hold) } else { stackful(&rt, cap, guard, hold) }
        }
    };
    emit(ctx, &p, &res);
    Ok(())
}

pub fn emit(ctx: &mut BenchCtx, p: &str, res: &MaxResult) {
    let tasks = Some(res.cap);
    let max_map = mem::read_sysctl("vm/max_map_count").unwrap_or(0);
    let mut r = ctx.row("max_tasks", Some(res.created as f64), "count");
    r.params = p.to_string();
    r.tasks = tasks;
    r.note = format!("limit={} {}", res.limit, res.detail);
    ctx.emit(r);
    let mut r = ctx.row("limit", None, "");
    r.params = p.to_string();
    r.tasks = tasks;
    r.note = res.limit.clone();
    r.status = if res.limit == "cap" || res.limit == "mem_guard" { Status::Ok } else { Status::Failed };
    ctx.emit(r);
    ctx.metric(p, tasks, "vmas_at_stop", res.at_stop.vmas as f64, "count");
    ctx.metric(p, tasks, "max_map_count", max_map as f64, "count");
    ctx.metric(p, tasks, "rss_at_stop", res.at_stop.vm_rss as f64, "bytes");
    ctx.metric(p, tasks, "pte_at_stop", res.at_stop.vm_pte as f64, "bytes");
    if res.created > 0 {
        let per = (res.at_stop.rss_plus_pte() as f64 - res.base.rss_plus_pte() as f64) / res.created as f64;
        ctx.metric(p, tasks, "total_per_task_at_stop", per.round(), "bytes");
        ctx.metric(p, tasks, "spawn_rate", res.created as f64 / res.spawn_secs.max(1e-9), "1/s");
    }
    if let Some(h) = res.held {
        ctx.metric(p, tasks, "hold_secs", (h.secs * 1e3).round() / 1e3, "s");
        let mut r = ctx.row("parked_after_hold", Some(h.parked as f64), "count");
        r.params = p.to_string();
        r.tasks = tasks;
        r.status = if h.parked == res.created { Status::Ok } else { Status::Failed };
        r.note = format!("{} of {} tasks still parked after the hold", h.parked, res.created);
        ctx.emit(r);
        let per = (h.mem.rss_plus_pte() as f64 - res.base.rss_plus_pte() as f64) / res.created.max(1) as f64;
        ctx.metric(p, tasks, "total_per_task_after_hold", per.round(), "bytes");
    }
}

/// Whether a run that stopped with `created` tasks holds them (only a run that
/// reached the cap is a candidate for the scale row's survival check).
fn holds(hold: Duration, created: u64, cap: u64) -> bool {
    !hold.is_zero() && created == cap
}

fn below_guard(guard: u64) -> bool {
    mem::mem_headroom().map(|a| a < guard).unwrap_or(false)
}

fn stackful(rt: &Runtime, cap: u64, guard: u64, hold: Duration) -> MaxResult {
    let gate = Gate::new();
    rt.block_on(|| {
        let mut res = MaxResult { cap, base: mem::snapshot(false), limit: "cap".into(), ..Default::default() };
        let t0 = Instant::now();
        let _ = rt::scope(|s| {
            while res.created < cap {
                if res.created.is_multiple_of(4096) {
                    if below_guard(guard) {
                        res.limit = "mem_guard".into();
                        res.detail = format!("memory headroom below {guard} bytes");
                        break;
                    }
                    let _ = rt::yield_now();
                }
                let g = &gate;
                match s.try_spawn(move || {
                    black_box(work::shallow_work(SplitStackProbe, 1));
                    let _ = g.wait(0);
                }) {
                    Ok(h) => {
                        drop(h);
                        res.created += 1;
                    }
                    Err(TaskError::SpawnFailed(e)) => {
                        res.limit = e.limit_name().into();
                        res.detail = e.to_string();
                        break;
                    }
                    Err(e) => {
                        res.limit = "other".into();
                        res.detail = e.to_string();
                        break;
                    }
                }
            }
            res.spawn_secs = t0.elapsed().as_secs_f64();
            let deadline = Instant::now() + Duration::from_secs(60);
            while (gate.waiting() as u64) < res.created && Instant::now() < deadline {
                let _ = rt::sleep(Duration::from_millis(5));
            }
            res.at_stop = mem::snapshot(true);
            if holds(hold, res.created, cap) {
                let t = Instant::now();
                let _ = rt::sleep(hold);
                res.held = Some(Held {
                    secs: t.elapsed().as_secs_f64(),
                    parked: gate.waiting() as u64,
                    mem: mem::snapshot(false),
                });
            }
            gate.open();
        });
        res
    })
}

fn stackless(rt: &Runtime, cap: u64, guard: u64, hold: Duration) -> MaxResult {
    let gate = Arc::new(Gate::new());
    rt.block_on_async(async move {
        let mut res = MaxResult { cap, base: mem::snapshot(false), limit: "cap".into(), ..Default::default() };
        let t0 = Instant::now();
        let s = AsyncScope::new();
        while res.created < cap {
            if res.created.is_multiple_of(4096) {
                if below_guard(guard) {
                    res.limit = "mem_guard".into();
                    res.detail = format!("memory headroom below {guard} bytes");
                    break;
                }
                let _ = rt::yield_now_async().await;
            }
            let g = gate.clone();
            drop(s.spawn(async move {
                black_box(work::shallow_work(SplitStackProbe, 1));
                let _ = g.wait_async(0).await;
            }));
            res.created += 1;
        }
        res.spawn_secs = t0.elapsed().as_secs_f64();
        let deadline = Instant::now() + Duration::from_secs(60);
        while (gate.waiting() as u64) < res.created && Instant::now() < deadline {
            let _ = rt::sleep_async(Duration::from_millis(5)).await;
        }
        res.at_stop = mem::snapshot(true);
        if holds(hold, res.created, cap) {
            let t = Instant::now();
            let _ = rt::sleep_async(hold).await;
            res.held = Some(Held {
                secs: t.elapsed().as_secs_f64(),
                parked: gate.waiting() as u64,
                mem: mem::snapshot(false),
            });
        }
        gate.open();
        let _ = s.join_all().await;
        res
    })
}

/// Reference: OS threads with small stacks, parked on a condition variable.
/// Mappings per Rust thread: its stack and guard, plus the signal stack and guard
/// that std maps for its stack-overflow handler.
const VMAS_PER_THREAD: u64 = 4;
/// Threads between two checks of the VMA count.
const THREAD_BATCH: u64 = 256;

fn os_threads(cap: u64, guard: u64, stack: usize, hold: Duration) -> MaxResult {
    use std::sync::atomic::{AtomicU64, Ordering::SeqCst};
    use std::sync::{Condvar, Mutex};
    let gate = Arc::new((Mutex::new(false), Condvar::new()));
    let parked = Arc::new(AtomicU64::new(0));
    let mut res = MaxResult { cap, base: mem::snapshot(false), limit: "cap".into(), ..Default::default() };
    let max_vmas = mem::read_sysctl("vm/max_map_count").unwrap_or(65530);
    let t0 = Instant::now();
    let mut hs = Vec::new();
    while res.created < cap {
        if res.created.is_multiple_of(1024) && below_guard(guard) {
            res.limit = "mem_guard".into();
            break;
        }
        // A new thread whose signal stack cannot be mapped aborts the process
        // (std treats it as fatal), so stop one batch before the VMA limit.
        if res.created.is_multiple_of(THREAD_BATCH) {
            let vmas = mem::vma_count();
            if vmas + (THREAD_BATCH + 16) * VMAS_PER_THREAD > max_vmas {
                res.limit = "vma_count".into();
                res.detail = format!("{vmas} of {max_vmas} mappings: another {THREAD_BATCH} threads would not fit");
                break;
            }
        }
        let (g, k) = (gate.clone(), parked.clone());
        match std::thread::Builder::new().stack_size(stack).spawn(move || {
            let (m, cv) = &*g;
            let mut open = m.lock().unwrap();
            k.fetch_add(1, SeqCst);
            while !*open {
                open = cv.wait(open).unwrap();
            }
        }) {
            Ok(h) => {
                hs.push(h);
                res.created += 1;
            }
            Err(e) => {
                let vmas = mem::vma_count();
                let max = mem::read_sysctl("vm/max_map_count").unwrap_or(65530);
                res.limit = if e.raw_os_error() == Some(libc::EAGAIN) {
                    "threads".into()
                } else if vmas + 8 >= max {
                    "vma_count".into()
                } else {
                    "memory".into()
                };
                res.detail = format!("{e} (vmas={vmas})");
                break;
            }
        }
    }
    res.spawn_secs = t0.elapsed().as_secs_f64();
    std::thread::sleep(Duration::from_millis(100));
    res.at_stop = mem::snapshot(true);
    if holds(hold, res.created, cap) {
        let t = Instant::now();
        std::thread::sleep(hold);
        res.held =
            Some(Held { secs: t.elapsed().as_secs_f64(), parked: parked.load(SeqCst), mem: mem::snapshot(false) });
    }
    {
        let (m, cv) = &*gate;
        *m.lock().unwrap() = true;
        cv.notify_all();
    }
    for h in hs {
        let _ = h.join();
    }
    res
}
