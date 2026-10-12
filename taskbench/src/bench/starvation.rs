//! `starvation`: the scheduling latency a latency-probe task sees while N
//! compute-bound tasks run.
//!
//! The probe sleeps `--probe-interval-us` (1 ms) in a loop and records how late
//! each wake-up is. It runs first alone (`probe_baseline_*`, which measures timer
//! precision) and then beside N compute tasks (`probe_loaded_*`) that run an LCG
//! kernel and reach a yield point every `--yield-every` iterations (the
//! compiler-inserted loop yield of proposal/06; `0` means no yield points at all,
//! the pure-cooperative worst case). The bet-3 threshold: loaded p99 no worse
//! than Go's (which preempts asynchronously) and under 10 ms.
//!
//! The run is stopped by an OS thread outside the runtime, so it ends even when
//! the compute tasks never yield. The probe's first deadline is its spawn time
//! plus one interval, and it always records at least one sample, so a probe that
//! cannot even start reports how late it started rather than nothing. The Go and
//! Java controls use the same definition.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::time::{Duration, Instant};

use super::{BenchCtx, Impl, emit_summary, params, work};
use crate::rt::{self, AsyncScope, Runtime};

/// Iterations per compute chunk when there are no yield points (the stop flag
/// is checked once per chunk).
const NO_YIELD_CHUNK: u64 = 100_000;

/// What a starvation run measured.
#[derive(Clone, Debug, Default)]
pub struct StarveResult {
    pub baseline: Vec<u64>,
    pub loaded: Vec<u64>,
    pub iterations: u64,
    pub loaded_secs: f64,
}

/// Sets `flag` after `d`, from an OS thread.
pub fn stop_after(flag: Arc<AtomicBool>, d: Duration) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        std::thread::sleep(d);
        flag.store(true, Relaxed);
    })
}

pub fn run(ctx: &mut BenchCtx) -> Result<(), String> {
    let n = ctx.args.tasks.unwrap_or(2 * ctx.args.threads as u64);
    let k = ctx.args.count("yield-every", 10_000)?;
    let duration = ctx.args.secs("duration", 2.0)?;
    let baseline = ctx.args.secs("baseline-secs", 1.0)?;
    let interval = Duration::from_micros(ctx.args.count("probe-interval-us", 1000)?);
    // Calibrate the length of one slice between yield points.
    let chunk = if k == 0 { NO_YIELD_CHUNK } else { k };
    let t0 = Instant::now();
    let reps = (2_000_000 / chunk).max(1);
    let mut x = 1;
    for _ in 0..reps {
        x = work::spin_iterations(x, chunk);
    }
    std::hint::black_box(x);
    let slice_ns = t0.elapsed().as_nanos() as f64 / reps as f64;
    let p = params(&[
        ("yield_every", k.to_string()),
        ("duration_s", format!("{}", duration.as_secs_f64())),
        ("probe_interval_us", interval.as_micros().to_string()),
    ]);
    ctx.banner(&format!("{p}; slice ~{:.1} us", slice_ns / 1e3));
    let res = match ctx.args.imp.clone() {
        Impl::Tokio => crate::tokio_ref::starvation(ctx.args.threads, n, k, duration, baseline, interval)?,
        Impl::OsThreads => os_threads(n, k, duration, baseline, interval),
        Impl::Taskbench(cfg) => {
            let rt = Runtime::with_worker_stack(ctx.args.threads, &cfg, ctx.args.worker_stack)?;
            if ctx.args.imp.is_stackless() {
                stackless(&rt, n, k, duration, baseline, interval)
            } else {
                stackful(&rt, n, k, duration, baseline, interval)
            }
        }
    };
    let tasks = Some(n);
    ctx.metric(&p, tasks, "slice_between_yields", slice_ns.round(), "ns");
    let mut b = res.baseline.clone();
    emit_summary(ctx, &p, tasks, "probe_baseline_late", &mut b);
    let mut l = res.loaded.clone();
    emit_summary(ctx, &p, tasks, "probe_loaded_late", &mut l);
    ctx.metric(&p, tasks, "compute_iterations_per_sec", res.iterations as f64 / res.loaded_secs.max(1e-9), "1/s");
    Ok(())
}

fn probe_loop(stop: &AtomicBool, interval: Duration, spawned: Instant) -> Vec<u64> {
    let mut v = Vec::new();
    let mut deadline = spawned + interval;
    loop {
        if let Some(rest) = deadline.checked_duration_since(Instant::now())
            && rt::sleep(rest).is_err() {
                break;
            }
        let now = Instant::now();
        v.push(now.saturating_duration_since(deadline).as_nanos() as u64);
        if stop.load(Relaxed) {
            break;
        }
        deadline = now + interval;
    }
    v
}

fn compute(stop: &AtomicBool, k: u64) -> u64 {
    let chunk = if k == 0 { NO_YIELD_CHUNK } else { k };
    let (mut x, mut done) = (1u64, 0u64);
    while !stop.load(Relaxed) {
        x = work::spin_iterations(x, chunk);
        done += chunk;
        if k > 0 && rt::yield_now().is_err() {
            break;
        }
    }
    std::hint::black_box(x);
    done
}

fn stackful(rt: &Runtime, n: u64, k: u64, duration: Duration, baseline: Duration, interval: Duration) -> StarveResult {
    rt.block_on(|| {
        let mut res = StarveResult::default();
        let stop = Arc::new(AtomicBool::new(false));
        let t = stop_after(stop.clone(), baseline);
        res.baseline = rt::scope(|s| {
            let (t, st) = (Instant::now(), &stop);
            s.spawn(move || probe_loop(st, interval, t)).join().unwrap_or_default()
        })
        .unwrap_or_default();
        let _ = t.join();
        let stop = Arc::new(AtomicBool::new(false));
        let iters = AtomicU64::new(0);
        let t0 = Instant::now();
        let t = stop_after(stop.clone(), duration);
        res.loaded = rt::scope(|s| {
            for _ in 0..n {
                s.spawn(|| iters.fetch_add(compute(&stop, k), Relaxed));
            }
            let (t, st) = (Instant::now(), &stop);
            s.spawn(move || probe_loop(st, interval, t)).join().unwrap_or_default()
        })
        .unwrap_or_default();
        res.loaded_secs = t0.elapsed().as_secs_f64();
        let _ = t.join();
        res.iterations = iters.load(Relaxed);
        res
    })
}

fn stackless(rt: &Runtime, n: u64, k: u64, duration: Duration, baseline: Duration, interval: Duration) -> StarveResult {
    async fn probe(stop: Arc<AtomicBool>, interval: Duration, spawned: Instant) -> Vec<u64> {
        let mut v = Vec::new();
        let mut deadline = spawned + interval;
        loop {
            if let Some(rest) = deadline.checked_duration_since(Instant::now())
                && rt::sleep_async(rest).await.is_err() {
                    break;
                }
            let now = Instant::now();
            v.push(now.saturating_duration_since(deadline).as_nanos() as u64);
            if stop.load(Relaxed) {
                break;
            }
            deadline = now + interval;
        }
        v
    }
    rt.block_on_async(async move {
        let mut res = StarveResult::default();
        let s = AsyncScope::new();
        let stop = Arc::new(AtomicBool::new(false));
        let t = stop_after(stop.clone(), baseline);
        res.baseline = s.spawn(probe(stop.clone(), interval, Instant::now())).join().await.unwrap_or_default();
        let _ = t.join();
        let stop = Arc::new(AtomicBool::new(false));
        let iters = Arc::new(AtomicU64::new(0));
        let t0 = Instant::now();
        let t = stop_after(stop.clone(), duration);
        for _ in 0..n {
            let (stop, iters) = (stop.clone(), iters.clone());
            drop(s.spawn(async move {
                let chunk = if k == 0 { NO_YIELD_CHUNK } else { k };
                let (mut x, mut done) = (1u64, 0u64);
                while !stop.load(Relaxed) {
                    x = work::spin_iterations(x, chunk);
                    done += chunk;
                    if k > 0 && rt::yield_now_async().await.is_err() {
                        break;
                    }
                }
                std::hint::black_box(x);
                iters.fetch_add(done, Relaxed);
            }));
        }
        res.loaded = s.spawn(probe(stop.clone(), interval, Instant::now())).join().await.unwrap_or_default();
        res.loaded_secs = t0.elapsed().as_secs_f64();
        let _ = s.join_all().await;
        let _ = t.join();
        res.iterations = iters.load(Relaxed);
        res
    })
}

/// Reference: N spinning OS threads and a sleeping probe thread; the kernel
/// preempts.
fn os_threads(n: u64, k: u64, duration: Duration, baseline: Duration, interval: Duration) -> StarveResult {
    let plain_probe = |stop: Arc<AtomicBool>| {
        let mut v = Vec::new();
        let mut deadline = Instant::now() + interval;
        loop {
            if let Some(rest) = deadline.checked_duration_since(Instant::now()) {
                std::thread::sleep(rest);
            }
            let now = Instant::now();
            v.push(now.saturating_duration_since(deadline).as_nanos() as u64);
            if stop.load(Relaxed) {
                break;
            }
            deadline = now + interval;
        }
        v
    };
    let mut res = StarveResult::default();
    let stop = Arc::new(AtomicBool::new(false));
    let t = stop_after(stop.clone(), baseline);
    res.baseline = plain_probe(stop.clone());
    let _ = t.join();
    let stop = Arc::new(AtomicBool::new(false));
    let iters = Arc::new(AtomicU64::new(0));
    let t0 = Instant::now();
    let t = stop_after(stop.clone(), duration);
    let workers: Vec<_> = (0..n)
        .map(|_| {
            let (stop, iters) = (stop.clone(), iters.clone());
            std::thread::spawn(move || {
                let chunk = if k == 0 { NO_YIELD_CHUNK } else { k };
                let (mut x, mut done) = (1u64, 0u64);
                while !stop.load(Relaxed) {
                    x = work::spin_iterations(x, chunk);
                    done += chunk;
                }
                std::hint::black_box(x);
                iters.fetch_add(done, Relaxed);
            })
        })
        .collect();
    res.loaded = plain_probe(stop.clone());
    res.loaded_secs = t0.elapsed().as_secs_f64();
    for w in workers {
        let _ = w.join();
    }
    let _ = t.join();
    res.iterations = iters.load(Relaxed);
    res
}
