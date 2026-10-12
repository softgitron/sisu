//! `echo` and `idle-connections`: a TCP echo server with N connections, driven
//! by an in-process client.
//!
//! `echo`: N client tasks each run a closed loop (write `--msg-bytes`, read the
//! echo) for `--warmup-secs` unrecorded and then `--duration` recorded; the
//! report gives p50, p99 and p99.9 of the round-trip latency and the throughput.
//! `idle-connections`: N connections whose server tasks are parked in `read`;
//! the report gives resident memory per connection (the client ends are plain
//! sockets held by the harness, not tasks).
//!
//! Connections are made by a helper OS thread with blocking `connect` while the
//! runtime accepts, so the setup does not depend on the scheduler. With more than
//! 20k connections the server listens on several ports (the ephemeral port range
//! allows about 28k connections per destination port). One process holds both
//! ends, so it needs `RLIMIT_NOFILE` of at least 2N + 64; the harness raises the
//! soft limit to the hard limit, as Go and the JVM do, and reports a failed row
//! when that is still too low.

use std::net::{SocketAddr, TcpStream as StdStream};
use std::sync::Arc;
use std::sync::atomic::{
    AtomicBool, AtomicU64, AtomicUsize,
    Ordering::{Relaxed, SeqCst},
};
use std::time::{Duration, Instant};

use super::{BenchCtx, Impl, emit_summary, params};
use crate::csv::Status;
use crate::mem::{self, MemSnapshot, per_item};
use crate::rt::net::{TcpConn, TcpListener};
use crate::rt::{self, AsyncScope, Runtime};

/// Connections per listening port.
pub const PER_PORT: usize = 20_000;

/// What an echo or idle-connections run measured.
#[derive(Clone, Debug, Default)]
pub struct EchoResult {
    pub latencies: Vec<u64>,
    pub requests: u64,
    pub secs: f64,
    pub connect_secs: f64,
    pub connections: usize,
    pub base: MemSnapshot,
    pub idle: Option<MemSnapshot>,
    pub error: Option<String>,
}

/// Opens `n` blocking client connections, round-robin over `addrs`, from an OS thread.
pub fn connect_all(addrs: Vec<SocketAddr>, n: usize) -> std::thread::JoinHandle<std::io::Result<Vec<StdStream>>> {
    std::thread::spawn(move || {
        let mut v = Vec::with_capacity(n);
        for i in 0..n {
            let s = StdStream::connect(addrs[i % addrs.len()])?;
            s.set_nodelay(true)?;
            v.push(s);
        }
        Ok(v)
    })
}

pub fn ports_for(n: usize) -> usize {
    n.div_ceil(PER_PORT).max(1)
}

/// How many of `n` connections go to port `p` of `ports` (round-robin).
pub fn quota(n: usize, ports: usize, p: usize) -> usize {
    n / ports + usize::from(p < n % ports)
}

pub fn run(ctx: &mut BenchCtx, idle: bool) -> Result<(), String> {
    let default = if idle { 5_000 } else { 1_000 };
    let n = match ctx.args.tasks {
        Some(t) => t,
        None => ctx.args.count("connections", default)?,
    } as usize;
    ctx.args.tasks = Some(n as u64);
    let msg = ctx.args.size("msg-bytes", 64)?.max(1);
    let duration = ctx.args.secs("duration", 3.0)?;
    let warmup = ctx.args.secs("warmup-secs", 1.0)?;
    let settle = Duration::from_millis(ctx.args.count("settle-ms", 200)?);
    let p = if idle {
        params(&[("connections", n.to_string()), ("ports", ports_for(n).to_string())])
    } else {
        params(&[
            ("connections", n.to_string()),
            ("msg_bytes", msg.to_string()),
            ("duration_s", format!("{}", duration.as_secs_f64())),
            ("warmup_s", format!("{}", warmup.as_secs_f64())),
        ])
    };
    ctx.banner(&p);
    let tasks = Some(n as u64);
    let nofile = crate::machine::raise_nofile_limit() as usize;
    if 2 * n + 64 > nofile {
        let metric = if idle { "idle_connections" } else { "echo" };
        ctx.outcome(
            &p,
            tasks,
            metric,
            Status::Failed,
            &format!("RLIMIT_NOFILE={nofile} is too low for {n} connections in one process (needs {})", 2 * n + 64),
        );
        return Ok(());
    }
    let res = match ctx.args.imp.clone() {
        Impl::Tokio => crate::tokio_ref::echo(ctx.args.threads, n, msg, duration, warmup, idle, settle)?,
        Impl::OsThreads => {
            ctx.outcome(&p, tasks, "echo", Status::Skipped, "not implemented for os-threads");
            return Ok(());
        }
        Impl::Taskbench(cfg) => {
            let rt = Runtime::with_worker_stack(ctx.args.threads, &cfg, ctx.args.worker_stack)?;
            if ctx.args.imp.is_stackless() {
                stackless(&rt, n, msg, duration, warmup, idle, settle)
            } else {
                stackful(&rt, n, msg, duration, warmup, idle, settle)
            }
        }
    };
    if let Some(e) = &res.error {
        ctx.outcome(&p, tasks, if idle { "idle_connections" } else { "echo" }, Status::Failed, e);
        return Ok(());
    }
    ctx.metric(&p, tasks, "connect_time", res.connect_secs * 1e3, "ms");
    if idle {
        if let Some(s) = res.idle {
            let b = res.base;
            let k = res.connections as u64;
            ctx.metric(&p, tasks, "rss_per_connection", per_item(b.vm_rss, s.vm_rss, k).round(), "bytes");
            ctx.metric(&p, tasks, "pte_per_connection", per_item(b.vm_pte, s.vm_pte, k).round(), "bytes");
            ctx.metric(
                &p,
                tasks,
                "total_per_connection",
                per_item(b.rss_plus_pte(), s.rss_plus_pte(), k).round(),
                "bytes",
            );
            ctx.metric(&p, tasks, "rss_total", s.vm_rss as f64, "bytes");
        }
    } else {
        let mut lat = res.latencies.clone();
        emit_summary(ctx, &p, tasks, "echo_rtt", &mut lat);
        ctx.metric(&p, tasks, "echo_requests", res.requests as f64, "count");
        ctx.metric(&p, tasks, "echo_throughput", res.requests as f64 / res.secs.max(1e-9), "req/s");
    }
    Ok(())
}

/// Runs the recorded window: warm up, record, stop.
fn drive_window(recording: &AtomicBool, stop: &AtomicBool, warmup: Duration, duration: Duration) -> f64 {
    let _ = rt::sleep(warmup);
    recording.store(true, SeqCst);
    let t0 = Instant::now();
    let _ = rt::sleep(duration);
    recording.store(false, SeqCst);
    let secs = t0.elapsed().as_secs_f64();
    stop.store(true, SeqCst);
    secs
}

fn stackful(
    rt: &Runtime,
    n: usize,
    msg: usize,
    duration: Duration,
    warmup: Duration,
    idle: bool,
    settle: Duration,
) -> EchoResult {
    rt.block_on(|| {
        let mut res = EchoResult { connections: n, ..Default::default() };
        let ports = ports_for(n);
        let mut listeners = Vec::new();
        for _ in 0..ports {
            match TcpListener::bind("127.0.0.1:0".parse().expect("addr"), 4096) {
                Ok(l) => listeners.push(l),
                Err(e) => {
                    res.error = Some(format!("bind: {e}"));
                    return res;
                }
            }
        }
        let addrs: Vec<SocketAddr> = listeners.iter().map(|l| l.local_addr().expect("local addr")).collect();
        let parked = AtomicUsize::new(0);
        let recording = AtomicBool::new(false);
        let stop = AtomicBool::new(false);
        res.base = mem::snapshot(false);
        let r = rt::scope(|s| {
            for (pi, l) in listeners.iter().enumerate() {
                let q = quota(n, ports, pi);
                let parked = &parked;
                s.spawn(move || {
                    // The acceptor's own scope holds the server tasks.
                    let _ = rt::scope(|servers| {
                        for _ in 0..q {
                            let Ok(c) = l.accept() else { break };
                            servers.spawn(move || {
                                let mut buf = vec![0u8; 4096];
                                parked.fetch_add(1, Relaxed);
                                loop {
                                    match c.read(&mut buf) {
                                        Ok(0) | Err(_) => break,
                                        Ok(k) => {
                                            if c.write_all(&buf[..k]).is_err() {
                                                break;
                                            }
                                        }
                                    }
                                }
                            });
                        }
                    });
                });
            }
            let t0 = Instant::now();
            let connector = connect_all(addrs.clone(), n);
            while !connector.is_finished() {
                let _ = rt::sleep(Duration::from_millis(1));
            }
            let streams = match connector.join().expect("connector thread") {
                Ok(v) => v,
                Err(e) => {
                    res.error = Some(format!("connect: {e}"));
                    s.cancel();
                    return;
                }
            };
            res.connect_secs = t0.elapsed().as_secs_f64();
            if idle {
                let deadline = Instant::now() + Duration::from_secs(60);
                while parked.load(Relaxed) < n && Instant::now() < deadline {
                    let _ = rt::sleep(Duration::from_millis(2));
                }
                let _ = rt::sleep(settle);
                res.idle = Some(mem::snapshot(false));
                drop(streams); // EOF ends the servers
                return;
            }
            let mut clients = Vec::with_capacity(n);
            for st in streams {
                let c = match TcpConn::from_std(st) {
                    Ok(c) => c,
                    Err(e) => {
                        res.error = Some(format!("register client: {e}"));
                        s.cancel();
                        return;
                    }
                };
                let (recording, stop) = (&recording, &stop);
                clients.push(s.spawn(move || {
                    let out = vec![b'x'; msg];
                    let mut buf = vec![0u8; msg];
                    let mut lat = Vec::new();
                    while !stop.load(Relaxed) {
                        let t = Instant::now();
                        if c.write_all(&out).is_err() || c.read_exact(&mut buf).is_err() {
                            break;
                        }
                        if recording.load(Relaxed) {
                            lat.push(t.elapsed().as_nanos() as u64);
                        }
                    }
                    lat
                }));
            }
            res.secs = drive_window(&recording, &stop, warmup, duration);
            for h in clients {
                if let Ok(l) = h.join() {
                    res.requests += l.len() as u64;
                    res.latencies.extend(l);
                }
            }
        });
        if let Err(e) = r {
            res.error.get_or_insert_with(|| format!("scope: {e}"));
        }
        res
    })
}

fn stackless(
    rt: &Runtime,
    n: usize,
    msg: usize,
    duration: Duration,
    warmup: Duration,
    idle: bool,
    settle: Duration,
) -> EchoResult {
    rt.block_on_async(async move {
        let mut res = EchoResult { connections: n, ..Default::default() };
        let ports = ports_for(n);
        let mut listeners = Vec::new();
        for _ in 0..ports {
            match TcpListener::bind("127.0.0.1:0".parse().expect("addr"), 4096) {
                Ok(l) => listeners.push(Arc::new(l)),
                Err(e) => {
                    res.error = Some(format!("bind: {e}"));
                    return res;
                }
            }
        }
        let addrs: Vec<SocketAddr> = listeners.iter().map(|l| l.local_addr().expect("local addr")).collect();
        let parked = Arc::new(AtomicUsize::new(0));
        let recording = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        res.base = mem::snapshot(false);
        let s = AsyncScope::new();
        for (pi, l) in listeners.iter().enumerate() {
            let (l, parked) = (l.clone(), parked.clone());
            let q = quota(n, ports, pi);
            drop(s.spawn(async move {
                let servers = AsyncScope::new();
                for _ in 0..q {
                    let Ok(c) = l.accept_async().await else { break };
                    let parked = parked.clone();
                    drop(servers.spawn(async move {
                        let mut buf = vec![0u8; 4096];
                        parked.fetch_add(1, Relaxed);
                        loop {
                            match c.read_async(&mut buf).await {
                                Ok(0) | Err(_) => break,
                                Ok(k) => {
                                    if c.write_all_async(&buf[..k]).await.is_err() {
                                        break;
                                    }
                                }
                            }
                        }
                    }));
                }
                let _ = servers.join_all().await;
            }));
        }
        let t0 = Instant::now();
        let connector = connect_all(addrs, n);
        while !connector.is_finished() {
            let _ = rt::sleep_async(Duration::from_millis(1)).await;
        }
        let streams = match connector.join().expect("connector thread") {
            Ok(v) => v,
            Err(e) => {
                res.error = Some(format!("connect: {e}"));
                s.cancel();
                let _ = s.join_all().await;
                return res;
            }
        };
        res.connect_secs = t0.elapsed().as_secs_f64();
        if idle {
            let deadline = Instant::now() + Duration::from_secs(60);
            while parked.load(Relaxed) < n && Instant::now() < deadline {
                let _ = rt::sleep_async(Duration::from_millis(2)).await;
            }
            let _ = rt::sleep_async(settle).await;
            res.idle = Some(mem::snapshot(false));
            drop(streams);
            let _ = s.join_all().await;
            return res;
        }
        let total = Arc::new(AtomicU64::new(0));
        let mut clients = Vec::with_capacity(n);
        for st in streams {
            let c = TcpConn::from_std(st).expect("register client");
            let (recording, stop) = (recording.clone(), stop.clone());
            clients.push(s.spawn(async move {
                let out = vec![b'x'; msg];
                let mut buf = vec![0u8; msg];
                let mut lat = Vec::new();
                while !stop.load(Relaxed) {
                    let t = Instant::now();
                    if c.write_all_async(&out).await.is_err() || c.read_exact_async(&mut buf).await.is_err() {
                        break;
                    }
                    if recording.load(Relaxed) {
                        lat.push(t.elapsed().as_nanos() as u64);
                    }
                }
                lat
            }));
        }
        let _ = rt::sleep_async(warmup).await;
        recording.store(true, SeqCst);
        let w0 = Instant::now();
        let _ = rt::sleep_async(duration).await;
        recording.store(false, SeqCst);
        res.secs = w0.elapsed().as_secs_f64();
        stop.store(true, SeqCst);
        for h in clients {
            if let Ok(l) = h.join().await {
                total.fetch_add(l.len() as u64, Relaxed);
                res.latencies.extend(l);
            }
        }
        res.requests = total.load(Relaxed);
        let _ = s.join_all().await;
        res
    })
}
