//! `spawn-join`: the latency of spawning a trivial task and joining it.
//!
//! One round spawns `batch` tasks in a long-lived scope (or, with
//! `--scope-per-round`, in a fresh scope) and joins them; the round's time
//! divided by `batch` is one sample. The bet-3 threshold compares the median with
//! Go's (`go f(); <-done`) for pooled stacks (B).

use std::hint::black_box;
use std::time::Instant;

use super::{BenchCtx, Impl, emit_summary, params};
use crate::rt::{self, AsyncScope};

pub fn run(ctx: &mut BenchCtx) -> Result<(), String> {
    let rounds = ctx.args.tasks_or("rounds", 20_000)? as usize;
    let warmup = ctx.args.count("warmup", 2_000)? as usize;
    let batch = ctx.args.count("batch", 1)?.max(1) as usize;
    let per_round_scope = ctx.args.flag("scope-per-round");
    let p = params(&[
        ("rounds", rounds.to_string()),
        ("warmup", warmup.to_string()),
        ("batch", batch.to_string()),
        ("scope_per_round", per_round_scope.to_string()),
    ]);
    ctx.banner(&p);
    let tasks = Some((rounds * batch) as u64);
    let mut samples = match ctx.args.imp.clone() {
        Impl::Tokio => crate::tokio_ref::spawn_join(ctx.args.threads, rounds, warmup, batch)?,
        Impl::OsThreads => os_threads(rounds.min(20_000), warmup.min(1_000), batch),
        Impl::Taskbench(_) if ctx.args.imp.is_stackless() => {
            let rt = ctx.runtime()?;
            rt.block_on_async(async move {
                let s = AsyncScope::new();
                let mut samples = Vec::with_capacity(rounds);
                let mut hs = Vec::with_capacity(batch);
                for r in 0..warmup + rounds {
                    let t0 = Instant::now();
                    for i in 0..batch {
                        hs.push(s.spawn(async move { black_box(r ^ i) }));
                    }
                    for h in hs.drain(..) {
                        black_box(h.join().await.expect("join"));
                    }
                    let dt = t0.elapsed().as_nanos() as u64 / batch as u64;
                    if r >= warmup {
                        samples.push(dt);
                    }
                }
                s.join_all().await.expect("scope");
                samples
            })
        }
        Impl::Taskbench(_) => {
            let rt = ctx.runtime()?;
            let samples = rt.block_on(|| {
                let mut samples = Vec::with_capacity(rounds);
                rt::scope(|s| {
                    for r in 0..warmup + rounds {
                        let t0 = Instant::now();
                        if per_round_scope {
                            rt::scope(|inner| {
                                let mut hs = Vec::with_capacity(batch);
                                for i in 0..batch {
                                    hs.push(inner.spawn(move || black_box(r ^ i)));
                                }
                                for h in hs {
                                    black_box(h.join().expect("join"));
                                }
                            })
                            .expect("scope");
                        } else {
                            let mut local = Vec::with_capacity(batch);
                            for i in 0..batch {
                                local.push(s.spawn(move || black_box(r ^ i)));
                            }
                            for h in local {
                                black_box(h.join().expect("join"));
                            }
                        }
                        let dt = t0.elapsed().as_nanos() as u64 / batch as u64;
                        if r >= warmup {
                            samples.push(dt);
                        }
                    }
                })
                .expect("scope");
                samples
            });
            let c = rt.stack_counters();
            let st = rt.stats();
            let mut row = ctx.row("stack_reuse_ratio", Some(c.reused as f64 / c.allocated.max(1) as f64), "ratio");
            row.params = p.clone();
            row.tasks = tasks;
            row.note = format!("allocated={} reused={} steals={} worker_wakeups={}", c.allocated, c.reused, st.steals, st.worker_wakeups);
            ctx.emit(row);
            samples
        }
    };
    let s = emit_summary(ctx, &p, tasks, "spawn_join", &mut samples);
    if s.mean > 0.0 {
        ctx.metric(&p, tasks, "spawn_join_per_sec", 1e9 / s.mean, "1/s");
    }
    Ok(())
}

/// Reference: one OS thread per task (`std::thread::scope`).
fn os_threads(rounds: usize, warmup: usize, batch: usize) -> Vec<u64> {
    let mut samples = Vec::with_capacity(rounds);
    for r in 0..warmup + rounds {
        let t0 = Instant::now();
        std::thread::scope(|s| {
            let hs: Vec<_> = (0..batch).map(|i| s.spawn(move || black_box(r ^ i))).collect();
            for h in hs {
                black_box(h.join().expect("join"));
            }
        });
        if r >= warmup {
            samples.push(t0.elapsed().as_nanos() as u64 / batch as u64);
        }
    }
    samples
}
