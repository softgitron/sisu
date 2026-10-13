//! The stackless reference control: the same workloads on tokio's
//! multi-threaded runtime (`--strategy tokio`).
//!
//! The bodies mirror the taskbench versions: the same gate (it is waker-based, so
//! it works on any executor), the same compute kernel, the same parser and
//! recursion. Scoped borrowing is not available on tokio, so tasks own their data
//! through `Arc`s. tokio's timer has millisecond resolution, which shows up in the
//! starvation baseline.

use std::hint::black_box;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{
    AtomicBool, AtomicU64, AtomicUsize,
    Ordering::{Relaxed, SeqCst},
};
use std::time::{Duration, Instant};

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::bench::deep_recursion::{CaseResult, LIVE_GUARDS, LIVE_NODES, naive, naive_async, parse_case};
use crate::bench::echo::{EchoResult, connect_all, ports_for, quota};
use crate::bench::idle_memory::IdleResult;
use crate::bench::max_tasks::MaxResult;
use crate::bench::starvation::{StarveResult, stop_after};
use crate::bench::work;
use crate::mem;
use crate::rt::{Gate, TaskError};
use crate::stack::probe::{self, SplitStackProbe, StackExhausted, current_sp};

fn runtime(threads: usize, stack: Option<usize>) -> Result<tokio::runtime::Runtime, String> {
    let mut b = tokio::runtime::Builder::new_multi_thread();
    b.worker_threads(threads).enable_all().on_thread_start(|| probe::init_worker_limit(64 * 1024));
    if let Some(s) = stack {
        b.thread_stack_size(s);
    }
    b.build().map_err(|e| format!("tokio runtime: {e}"))
}

fn join_err(e: tokio::task::JoinError) -> TaskError {
    if e.is_cancelled() {
        return TaskError::Cancelled;
    }
    let payload = e.into_panic();
    if let Some(se) = payload.downcast_ref::<StackExhausted>() {
        return TaskError::StackExhausted {
            how: crate::stack::Exhaustion::WorkerStackProbe,
            detail: format!("probe fired at sp {:#x} (limit {:#x})", se.sp, se.limit),
        };
    }
    TaskError::from_panic(payload)
}

/// `spawn-join` on tokio: `tokio::spawn(..).await` per task.
pub fn spawn_join(threads: usize, rounds: usize, warmup: usize, batch: usize) -> Result<Vec<u64>, String> {
    let rt = runtime(threads, None)?;
    rt.block_on(async move {
        tokio::spawn(async move {
            let mut samples = Vec::with_capacity(rounds);
            let mut hs = Vec::with_capacity(batch);
            for r in 0..warmup + rounds {
                let t0 = Instant::now();
                for i in 0..batch {
                    hs.push(tokio::spawn(async move { black_box(r ^ i) }));
                }
                for h in hs.drain(..) {
                    black_box(h.await.expect("join"));
                }
                if r >= warmup {
                    samples.push(t0.elapsed().as_nanos() as u64 / batch as u64);
                }
            }
            samples
        })
        .await
        .map_err(|e| e.to_string())
    })
}

async fn wait_waiting(gate: &Gate, n: u64, deadline: Instant) -> bool {
    while (gate.waiting() as u64) < n {
        if Instant::now() > deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
    true
}

/// `idle-memory` on tokio.
pub fn idle_memory(threads: usize, n: u64, excursion: usize, settle: Duration) -> Result<IdleResult, String> {
    let rt = runtime(threads, None)?;
    let gate = Arc::new(Gate::new());
    let span = Arc::new(AtomicUsize::new(0));
    rt.block_on(async move {
        tokio::spawn(async move {
            let mut res = IdleResult { requested: n, base: mem::snapshot(true), ..Default::default() };
            let mut hs = Vec::with_capacity(n as usize);
            for i in 0..n {
                let (g, sp) = (gate.clone(), span.clone());
                hs.push(tokio::spawn(async move {
                    black_box(work::shallow_work(SplitStackProbe, i));
                    let _ = g.wait_async(0).await;
                    if excursion > 0 {
                        let top = current_sp();
                        let low = work::excursion(SplitStackProbe, excursion);
                        sp.fetch_max(top - low, SeqCst);
                    }
                    let _ = g.wait_async(1).await;
                }));
                res.created += 1;
                if i % 1024 == 1023 {
                    tokio::task::yield_now().await;
                }
            }
            let deadline = Instant::now() + Duration::from_secs(60) + Duration::from_micros(50 * n);
            if wait_waiting(&gate, n, deadline).await {
                tokio::time::sleep(settle).await;
                res.shallow = Some(mem::snapshot(true));
                gate.open();
                if excursion > 0 && wait_waiting(&gate, n, deadline).await {
                    tokio::time::sleep(settle).await;
                    res.excursion = Some(mem::snapshot(true));
                }
            } else {
                res.failure = Some("tasks did not all park before the deadline".into());
            }
            gate.open();
            gate.open();
            for h in hs {
                let _ = h.await;
            }
            res.excursion_span = span.load(SeqCst);
            res.note = "join handles held by the spawner (8 bytes/task)".into();
            res
        })
        .await
        .map_err(|e| e.to_string())
    })
}

/// `max-tasks` on tokio (memory is the only limit; the guard stops first).
pub fn max_tasks(threads: usize, cap: u64, guard: u64, hold: Duration) -> Result<MaxResult, String> {
    let rt = runtime(threads, None)?;
    let gate = Arc::new(Gate::new());
    rt.block_on(async move {
        tokio::spawn(async move {
            let mut res = MaxResult { cap, base: mem::snapshot(false), limit: "cap".into(), ..Default::default() };
            let t0 = Instant::now();
            while res.created < cap {
                if res.created.is_multiple_of(4096) {
                    if mem::mem_headroom().map(|a| a < guard).unwrap_or(false) {
                        res.limit = "mem_guard".into();
                        res.detail = format!("memory headroom below {guard} bytes");
                        break;
                    }
                    tokio::task::yield_now().await;
                }
                let g = gate.clone();
                drop(tokio::spawn(async move {
                    black_box(work::shallow_work(SplitStackProbe, 1));
                    let _ = g.wait_async(0).await;
                }));
                res.created += 1;
            }
            res.spawn_secs = t0.elapsed().as_secs_f64();
            let deadline = Instant::now() + Duration::from_secs(60);
            wait_waiting(&gate, res.created, deadline).await;
            res.at_stop = mem::snapshot(true);
            if !hold.is_zero() && res.created == cap {
                let t = Instant::now();
                tokio::time::sleep(hold).await;
                res.held = Some(crate::bench::max_tasks::Held {
                    secs: t.elapsed().as_secs_f64(),
                    parked: gate.waiting() as u64,
                    mem: mem::snapshot(false),
                });
            }
            gate.open();
            res
        })
        .await
        .map_err(|e| e.to_string())
    })
}

/// `starvation` on tokio: compute tasks yield with `tokio::task::yield_now`.
pub fn starvation(
    threads: usize,
    n: u64,
    k: u64,
    duration: Duration,
    baseline: Duration,
    interval: Duration,
) -> Result<StarveResult, String> {
    async fn probe(stop: Arc<AtomicBool>, interval: Duration, spawned: Instant) -> Vec<u64> {
        let mut v = Vec::new();
        let mut deadline = spawned + interval;
        loop {
            if let Some(rest) = deadline.checked_duration_since(Instant::now()) {
                tokio::time::sleep(rest).await;
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
    let rt = runtime(threads, None)?;
    rt.block_on(async move {
        tokio::spawn(async move {
            let mut res = StarveResult::default();
            let stop = Arc::new(AtomicBool::new(false));
            let t = stop_after(stop.clone(), baseline);
            res.baseline = tokio::spawn(probe(stop, interval, Instant::now())).await.unwrap_or_default();
            let _ = t.join();
            let stop = Arc::new(AtomicBool::new(false));
            let iters = Arc::new(AtomicU64::new(0));
            let t0 = Instant::now();
            let t = stop_after(stop.clone(), duration);
            let mut hs = Vec::new();
            for _ in 0..n {
                let (stop, iters) = (stop.clone(), iters.clone());
                hs.push(tokio::spawn(async move {
                    let chunk = if k == 0 { 100_000 } else { k };
                    let (mut x, mut done) = (1u64, 0u64);
                    while !stop.load(Relaxed) {
                        x = work::spin_iterations(x, chunk);
                        done += chunk;
                        if k > 0 {
                            tokio::task::yield_now().await;
                        }
                    }
                    black_box(x);
                    iters.fetch_add(done, Relaxed);
                }));
            }
            res.loaded = tokio::spawn(probe(stop.clone(), interval, Instant::now())).await.unwrap_or_default();
            res.loaded_secs = t0.elapsed().as_secs_f64();
            for h in hs {
                let _ = h.await;
            }
            let _ = t.join();
            res.iterations = iters.load(Relaxed);
            res
        })
        .await
        .map_err(|e| e.to_string())
    })
}

/// `deep-recursion` on tokio: synchronous recursion on the worker stack, and a
/// recursion of boxed futures.
pub fn deep_recursion(
    threads: usize,
    worker_stack: usize,
    parse_depth: usize,
    naive_depth: u64,
) -> Result<(Vec<CaseResult>, bool), String> {
    let rt = runtime(threads, Some(worker_stack))?;
    let out = rt.block_on(async move {
        let mut cases = Vec::new();
        for (name, target) in [("parse", parse_depth as u64), ("naive", naive_depth), ("naive-async", naive_depth)] {
            let reached = Arc::new(AtomicU64::new(0));
            let r2 = reached.clone();
            let live0 = LIVE_NODES.load(SeqCst);
            let guards0 = LIVE_GUARDS.load(SeqCst);
            let t0 = Instant::now();
            let h = match name {
                "parse" => tokio::spawn(async move { parse_case(SplitStackProbe, parse_depth, &r2) }),
                "naive" => tokio::spawn(async move { Ok(naive(SplitStackProbe, 0, naive_depth, &r2) as i64) }),
                _ => tokio::spawn(async move { Ok(naive_async(0, naive_depth, r2).await as i64) }),
            };
            let outcome = match h.await {
                Ok(Ok(v)) => Ok(v),
                Ok(Err(msg)) => Err(TaskError::Panicked(format!("parse error: {msg}"))),
                Err(e) => Err(join_err(e)),
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
        let healthy = tokio::spawn(async { 7 }).await.map(|v| v == 7).unwrap_or(false);
        (cases, healthy)
    });
    Ok(out)
}

/// `echo` and `idle-connections` on tokio.
pub fn echo(
    threads: usize,
    n: usize,
    msg: usize,
    duration: Duration,
    warmup: Duration,
    idle: bool,
    settle: Duration,
) -> Result<EchoResult, String> {
    let rt = runtime(threads, None)?;
    rt.block_on(async move {
        tokio::spawn(async move {
            let mut res = EchoResult { connections: n, ..Default::default() };
            let ports = ports_for(n);
            let mut addrs: Vec<SocketAddr> = Vec::new();
            let parked = Arc::new(AtomicUsize::new(0));
            res.base = mem::snapshot(false);
            let mut acceptors = Vec::new();
            for pi in 0..ports {
                let sock = tokio::net::TcpSocket::new_v4().map_err(|e| e.to_string())?;
                sock.bind("127.0.0.1:0".parse().expect("addr")).map_err(|e| e.to_string())?;
                let l = sock.listen(4096).map_err(|e| e.to_string())?;
                addrs.push(l.local_addr().map_err(|e| e.to_string())?);
                let q = quota(n, ports, pi);
                let parked = parked.clone();
                acceptors.push(tokio::spawn(async move {
                    let mut servers = Vec::with_capacity(q);
                    for _ in 0..q {
                        let Ok((mut c, _)) = l.accept().await else { break };
                        let _ = c.set_nodelay(true);
                        let parked = parked.clone();
                        servers.push(tokio::spawn(async move {
                            let mut buf = vec![0u8; 4096];
                            parked.fetch_add(1, Relaxed);
                            loop {
                                match c.read(&mut buf).await {
                                    Ok(0) | Err(_) => break,
                                    Ok(k) => {
                                        if c.write_all(&buf[..k]).await.is_err() {
                                            break;
                                        }
                                    }
                                }
                            }
                        }));
                    }
                    for s in servers {
                        let _ = s.await;
                    }
                }));
            }
            let t0 = Instant::now();
            let connector = connect_all(addrs, n);
            while !connector.is_finished() {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            let streams = connector.join().expect("connector thread").map_err(|e| format!("connect: {e}"))?;
            res.connect_secs = t0.elapsed().as_secs_f64();
            if idle {
                let deadline = Instant::now() + Duration::from_secs(60);
                while parked.load(Relaxed) < n && Instant::now() < deadline {
                    tokio::time::sleep(Duration::from_millis(2)).await;
                }
                tokio::time::sleep(settle).await;
                res.idle = Some(mem::snapshot(false));
                drop(streams);
                for a in acceptors {
                    let _ = a.await;
                }
                return Ok(res);
            }
            let recording = Arc::new(AtomicBool::new(false));
            let stop = Arc::new(AtomicBool::new(false));
            let mut clients = Vec::with_capacity(n);
            for st in streams {
                st.set_nonblocking(true).map_err(|e| e.to_string())?;
                let mut c = tokio::net::TcpStream::from_std(st).map_err(|e| e.to_string())?;
                let (recording, stop) = (recording.clone(), stop.clone());
                clients.push(tokio::spawn(async move {
                    let out = vec![b'x'; msg];
                    let mut buf = vec![0u8; msg];
                    let mut lat = Vec::new();
                    while !stop.load(Relaxed) {
                        let t = Instant::now();
                        if c.write_all(&out).await.is_err() || c.read_exact(&mut buf).await.is_err() {
                            break;
                        }
                        if recording.load(Relaxed) {
                            lat.push(t.elapsed().as_nanos() as u64);
                        }
                    }
                    lat
                }));
            }
            tokio::time::sleep(warmup).await;
            recording.store(true, SeqCst);
            let w0 = Instant::now();
            tokio::time::sleep(duration).await;
            recording.store(false, SeqCst);
            res.secs = w0.elapsed().as_secs_f64();
            stop.store(true, SeqCst);
            for h in clients {
                if let Ok(l) = h.await {
                    res.requests += l.len() as u64;
                    res.latencies.extend(l);
                }
            }
            for a in acceptors {
                let _ = a.await;
            }
            Ok(res)
        })
        .await
        .map_err(|e| e.to_string())?
    })
}
