//! `cancel-uring`: io_uring reads into buffers on task stacks, cancelled while in
//! flight; no buffer may be written after its read has returned (amendment A1,
//! D-058, N-07).
//!
//! Each round starts `--concurrency` tasks. Each fills a buffer in its own stack
//! frame with a sentinel, keeps a copy, and reads into it through io_uring, then
//! the harness cancels reads while they are in flight. When every read has
//! returned to its task, each task checks its buffer (bytes past the returned
//! length must still be the sentinel; bytes before it must be the expected data),
//! waits while anything still in flight would land, and checks again. The next
//! round reuses the same stacks (pooled slots are recycled LIFO, A's mappings are
//! recycled by the kernel), so a write that landed after its task finished would
//! corrupt, and be caught in, a later task's buffer.
//!
//! Two sources (`--source`):
//!
//! * `pipe` (default): each task reads its own empty pipe; for a quarter of the
//!   tasks the harness writes data, for a quarter it cancels, for half it does
//!   both in either order, racing completion against cancellation; afterwards it
//!   writes "late" data into every pipe, which only a read still in flight could
//!   deliver. `--async-percent` of reads carry `IOSQE_ASYNC`.
//! * `file`: each task reads one 512-byte block of a generated file opened with
//!   `O_DIRECT`, so the read is in flight in the block device; half the tasks are
//!   cancelled as soon as their read is queued. The cancel typically finds nothing
//!   to cancel (`-ENOENT`: the read was already issued to the device) while the
//!   read is still in flight and completes later, which is the case D-058's rule
//!   exists for.
//! * `file-buffered`: as `file` without `O_DIRECT`, with the file's pages evicted
//!   before each round; an `IOSQE_ASYNC` read then blocks inside an io-wq worker
//!   waiting for the page, and a cancel finds it executing (`-EALREADY`).
//!
//! The rule under test: a cancelled read returns only on its original
//! completion. `--break-contract` is the negative control: a cancelled task stops
//! waiting as soon as it is cancelled, so late completions reach its buffer after
//! "return", and the run must report violations (the task still cancels and waits
//! before its frame dies, so the control corrupts nothing).

use std::io::Write;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::os::unix::fs::OpenOptionsExt;
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use std::time::{Duration, Instant};

use super::{BenchCtx, Impl, params};
use crate::csv::Status;
use crate::rt::uring::{ReadOutcome, UringDriver};
use crate::rt::{self, Gate, Runtime};

const MAX_BUF: usize = 2048;
const RACE_DATA: usize = 64;
const BLOCK: usize = 512;

fn sentinel(round: usize, i: usize) -> u8 {
    0x40 + ((round * 7 + i) % 32) as u8
}
fn data_byte(round: usize, i: usize) -> u8 {
    0x80 + ((round * 3 + i) % 32) as u8
}
fn late_byte(round: usize, i: usize) -> u8 {
    0xC0 + ((round * 5 + i) % 32) as u8
}
/// File contents: every byte of block `b` is `file_byte(b)`, disjoint from the
/// sentinel, data and late ranges.
fn file_byte(block: u64) -> u8 {
    1 + (block % 63) as u8
}

/// What one reader task saw.
#[derive(Clone, Copy, Debug, Default)]
pub struct Report {
    pub result: i32,
    pub cancel: Option<i32>,
    /// Bytes wrong when the read returned (beyond its length, or not the data).
    pub bad_after_return: usize,
    /// Bytes that changed after the read had returned.
    pub bad_late: usize,
    pub abandoned: bool,
}

/// What a reader reads.
#[derive(Clone, Copy)]
struct Target {
    fd: RawFd,
    offset: u64,
    /// The byte a successful read delivers.
    expect: u8,
}

/// A stack buffer aligned for `O_DIRECT`.
#[repr(C, align(512))]
struct AlignedBuf([u8; MAX_BUF]);

fn pipe() -> std::io::Result<(OwnedFd, OwnedFd)> {
    let mut fds = [0; 2];
    // SAFETY: `fds` is a valid out-array of two descriptors.
    if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: pipe2 returned two fresh descriptors we now own.
    Ok(unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) })
}

/// Creates the data file, opens it (with `O_DIRECT` when `direct`) and unlinks it.
fn data_file(dir: &std::path::Path, blocks: u64, direct: bool) -> std::io::Result<OwnedFd> {
    let path = dir.join(format!("taskbench-uring-{}.dat", std::process::id()));
    {
        let mut f = std::fs::File::create(&path)?;
        let mut chunk = Vec::with_capacity(BLOCK * 256);
        for b in 0..blocks {
            chunk.extend(std::iter::repeat_n(file_byte(b), BLOCK));
            if chunk.len() >= BLOCK * 256 {
                f.write_all(&chunk)?;
                chunk.clear();
            }
        }
        f.write_all(&chunk)?;
        f.sync_all()?;
    }
    let flags = if direct { libc::O_DIRECT } else { 0 };
    let f = std::fs::OpenOptions::new().read(true).custom_flags(flags).open(&path);
    let _ = std::fs::remove_file(&path);
    Ok(OwnedFd::from(f?))
}

/// Drops the file's clean pages from the page cache, so buffered reads go to disk.
fn evict(fd: RawFd) {
    // SAFETY: advisory call on a valid descriptor.
    unsafe { libc::posix_fadvise(fd, 0, 0, libc::POSIX_FADV_DONTNEED) };
}

fn write_bytes(fd: RawFd, byte: u8, n: usize) {
    let buf = vec![byte; n];
    // SAFETY: writes from a valid buffer; the pipe has room (64 KiB) for this.
    unsafe { libc::write(fd, buf.as_ptr() as *const libc::c_void, n) };
}

fn drain(fd: RawFd) {
    let mut buf = [0u8; 4096];
    loop {
        let mut avail: libc::c_int = 0;
        // SAFETY: FIONREAD writes an int to a valid pointer.
        if unsafe { libc::ioctl(fd, libc::FIONREAD, &mut avail) } != 0 || avail <= 0 {
            return;
        }
        let n = (avail as usize).min(buf.len());
        // SAFETY: reads at most `n` available bytes into a valid buffer (does not block).
        unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, n) };
    }
}

/// Reads the buffer with volatile loads: the kernel writes it behind the
/// compiler's back, so ordinary loads could be folded.
fn snapshot(ptr: *const u8, len: usize) -> Vec<u8> {
    // SAFETY: `ptr..ptr+len` is the caller's live buffer.
    (0..len).map(|k| unsafe { std::ptr::read_volatile(ptr.add(k)) }).collect()
}

#[allow(clippy::too_many_arguments)]
fn reader(
    driver: &UringDriver,
    t: Target,
    round: usize,
    i: usize,
    len: usize,
    force_async: bool,
    break_contract: bool,
    gate: &Gate,
    submitted: &AtomicUsize,
    returned: &AtomicUsize,
) -> Report {
    let mut storage = AlignedBuf([0u8; MAX_BUF]); // in this task's stack frame
    let s = sentinel(round, i);
    storage.0[..len].fill(s);
    let ptr = storage.0.as_ptr();
    let mut rep = Report::default();
    let mut abandoned = None;
    let outcome: Option<ReadOutcome> = if !break_contract {
        Some(driver.read(t.fd, &mut storage.0[..len], t.offset, force_async, || {
            submitted.fetch_add(1, SeqCst);
        }))
    } else {
        // SAFETY: the handle is not leaked: it is either finished by
        // `wait_or_abandon` or cancelled and waited for below.
        let mut op = unsafe { driver.start_read(t.fd, &mut storage.0[..len], t.offset, force_async) };
        submitted.fetch_add(1, SeqCst);
        match op.wait_or_abandon() {
            Some(o) => Some(o),
            None => {
                abandoned = Some(op);
                None
            }
        }
    };
    returned.fetch_add(1, SeqCst);
    // Check 1: on return, bytes past the result are untouched, bytes before it are data.
    let first = snapshot(ptr, len);
    let n = outcome.map(|o| o.result.max(0) as usize).unwrap_or(0).min(len);
    rep.bad_after_return = first[..n].iter().filter(|&&b| b != t.expect).count() + first[n..].iter().filter(|&&b| b != s).count();
    // Stay alive, frame intact, while anything still in flight lands.
    gate.wait_shielded(0);
    // Check 2: nothing changed after the read returned.
    let second = snapshot(ptr, len);
    rep.bad_late = first.iter().zip(&second).filter(|(a, b)| a != b).count();
    if let Some(o) = outcome {
        rep.result = o.result;
        rep.cancel = o.cancel;
    }
    if let Some(op) = abandoned {
        rep.abandoned = true;
        let o = op.cancel_and_wait();
        rep.result = o.result;
        rep.cancel = o.cancel;
    }
    std::hint::black_box(ptr);
    rep
}

pub fn run(ctx: &mut BenchCtx) -> Result<(), String> {
    let total = ctx.args.tasks.unwrap_or(4096) as usize;
    let mut conc = (ctx.args.count("concurrency", 512)? as usize).clamp(1, total.max(1));
    let async_pct = ctx.args.count("async-percent", 25)?.min(100) as usize;
    let break_contract = ctx.args.flag("break-contract");
    let source = ctx.args.opt("source").unwrap_or("pipe").to_string();
    let (file, direct) = match source.as_str() {
        "pipe" => (false, false),
        "file" => (true, true),
        "file-buffered" => (true, false),
        other => return Err(format!("unknown --source {other:?} (pipe, file, file-buffered)")),
    };
    let len = if file { BLOCK } else { ctx.args.size("buf-bytes", 512)?.clamp(RACE_DATA, MAX_BUF) };
    let file_blocks = (ctx.args.size("file-bytes", 64 << 20)? / BLOCK).max(1) as u64;
    let nofile = crate::machine::raise_nofile_limit() as usize;
    if !file && 2 * conc + 64 > nofile {
        conc = (nofile.saturating_sub(64) / 2).max(1);
    }
    let p = params(&[
        ("source", source.clone()),
        ("concurrency", conc.to_string()),
        ("async_percent", async_pct.to_string()),
        ("buf_bytes", len.to_string()),
        ("break_contract", break_contract.to_string()),
    ]);
    ctx.banner(&p);
    let tasks = Some(total as u64);
    let cfg = match ctx.args.imp.clone() {
        Impl::Taskbench(c) if !ctx.args.imp.is_stackless() => c,
        other => {
            let why = match other {
                Impl::Tokio => "tokio has no completion I/O into borrowed buffers (tokio-uring takes owned buffers because Rust values can be leaked; N-07)",
                _ => "the test lends buffers on task stacks; run it with a stackful strategy (A-D)",
            };
            ctx.outcome(&p, tasks, "cancel_uring", Status::Skipped, why);
            return Ok(());
        }
    };
    let driver = match UringDriver::new((2 * conc + 64).next_power_of_two() as u32) {
        Ok(d) => d,
        Err(e) => {
            ctx.outcome(&p, tasks, "cancel_uring", Status::Skipped, &format!("skipped: io_uring unavailable: {e}"));
            return Ok(());
        }
    };
    let mut pipes = Vec::new();
    let mut data_fd = None;
    if file {
        let dir = ctx.args.opt("file-dir").map(std::path::PathBuf::from).unwrap_or_else(std::env::temp_dir);
        match data_file(&dir, file_blocks, direct) {
            Ok(fd) => data_fd = Some(fd),
            Err(e) => {
                ctx.outcome(&p, tasks, "cancel_uring", Status::Skipped, &format!("skipped: no O_DIRECT file in {}: {e}", dir.display()));
                return Ok(());
            }
        }
    } else {
        for _ in 0..conc {
            pipes.push(pipe().map_err(|e| format!("pipe: {e}"))?);
        }
    }
    let rt = Runtime::with_worker_stack(ctx.args.threads, &cfg, ctx.args.worker_stack)?;
    let t0 = Instant::now();
    let reports: Vec<Report> = rt.block_on(|| {
        let mut all = Vec::with_capacity(total);
        let mut done = 0;
        let mut round = 0;
        while done < total {
            let k = conc.min(total - done);
            let gate = Gate::new();
            let submitted = AtomicUsize::new(0);
            let returned = AtomicUsize::new(0);
            let (driver, gate_r, sub, ret) = (&driver, &gate, &submitted, &returned);
            let target = |i: usize| match &data_fd {
                Some(fd) => {
                    let block = ((done + i) as u64).wrapping_mul(7919) % file_blocks;
                    Target { fd: fd.as_raw_fd(), offset: block * BLOCK as u64, expect: file_byte(block) }
                }
                None => Target { fd: pipes[i].0.as_raw_fd(), offset: 0, expect: data_byte(round, i) },
            };
            if let Some(fd) = &data_fd
                && !direct {
                    evict(fd.as_raw_fd());
                }
            let reps = rt::scope(|s| {
                let hs: Vec<_> = (0..k)
                    .map(|i| {
                        let t = target(i);
                        let force_async = (i * 100 / k.max(1)) < async_pct;
                        s.spawn(move || reader(driver, t, round, i, len, force_async, break_contract, gate_r, sub, ret))
                    })
                    .collect();
                let deadline = Instant::now() + Duration::from_secs(30);
                while submitted.load(SeqCst) < k && Instant::now() < deadline {
                    let _ = rt::yield_now();
                }
                if file {
                    // Cancel half of the reads while they are in the device.
                    for (i, h) in hs.iter().enumerate() {
                        if (i + round) % 2 == 0 {
                            h.cancel();
                        }
                    }
                } else {
                    // Let the ring thread hand the reads to the kernel, then race.
                    let _ = rt::sleep(Duration::from_millis(5));
                    for (i, h) in hs.iter().enumerate() {
                        let w = pipes[i].1.as_raw_fd();
                        match (i + round) % 4 {
                            0 => write_bytes(w, data_byte(round, i), RACE_DATA),
                            1 => h.cancel(),
                            2 => {
                                write_bytes(w, data_byte(round, i), RACE_DATA);
                                h.cancel();
                            }
                            _ => {
                                h.cancel();
                                write_bytes(w, data_byte(round, i), RACE_DATA);
                            }
                        }
                    }
                }
                while returned.load(SeqCst) < k && Instant::now() < deadline {
                    let _ = rt::sleep(Duration::from_millis(1));
                }
                if !file {
                    // Late data: only a read still in flight could deliver it.
                    for (i, (_, w)) in pipes.iter().enumerate().take(k) {
                        write_bytes(w.as_raw_fd(), late_byte(round, i), len);
                    }
                }
                // Anything still in flight lands within this window.
                let _ = rt::sleep(Duration::from_millis(10));
                gate.open();
                hs.into_iter().map(|h| h.join().unwrap_or_default()).collect::<Vec<Report>>()
            })
            .unwrap_or_default();
            for (r, _) in pipes.iter().take(k) {
                drain(r.as_raw_fd());
            }
            all.extend(reps);
            done += k;
            round += 1;
        }
        all
    });
    let elapsed = t0.elapsed();
    let st = driver.stats();
    let count = |f: &dyn Fn(&Report) -> bool| reports.iter().filter(|r| f(r)).count() as f64;
    let after = reports.iter().map(|r| r.bad_after_return).sum::<usize>();
    let late = reports.iter().map(|r| r.bad_late).sum::<usize>();
    let tasks_with_violation = count(&|r| r.bad_after_return + r.bad_late > 0);
    ctx.metric(&p, tasks, "reads", reports.len() as f64, "count");
    ctx.metric(&p, tasks, "completed_with_data", count(&|r| r.result > 0), "count");
    ctx.metric(&p, tasks, "ended_ecanceled", count(&|r| r.result == -libc::ECANCELED), "count");
    ctx.metric(&p, tasks, "ended_eintr", count(&|r| r.result == -libc::EINTR), "count");
    ctx.metric(&p, tasks, "ended_other", count(&|r| r.result <= 0 && r.result != -libc::ECANCELED && r.result != -libc::EINTR), "count");
    ctx.metric(&p, tasks, "cancels_submitted", st.cancels_submitted as f64, "count");
    ctx.metric(&p, tasks, "cancel_ok", st.cancel_ok as f64, "count");
    ctx.metric(&p, tasks, "cancel_enoent", st.cancel_enoent as f64, "count");
    ctx.metric(&p, tasks, "cancel_ealready", st.cancel_ealready as f64, "count");
    ctx.metric(&p, tasks, "completed_after_failed_cancel", count(&|r| r.result > 0 && matches!(r.cancel, Some(c) if c < 0)), "count");
    ctx.metric(&p, tasks, "abandoned_reads", count(&|r| r.abandoned), "count");
    ctx.metric(&p, tasks, "elapsed", elapsed.as_secs_f64() * 1e3, "ms");
    let mut r = ctx.row("buffer_writes_after_return", Some((after + late) as f64), "bytes");
    r.params = p.clone();
    r.tasks = tasks;
    let pass = if break_contract { tasks_with_violation > 0.0 } else { tasks_with_violation == 0.0 };
    r.status = if pass { Status::Ok } else { Status::Failed };
    r.note = format!(
        "tasks_with_violations={tasks_with_violation} bad_at_return={after} bad_late={late}{}",
        if break_contract { "; negative control: violations expected" } else { "" }
    );
    ctx.emit(r);
    Ok(())
}
