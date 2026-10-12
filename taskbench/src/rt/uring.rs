//! An io_uring driver for the cancellation test (amendment A1, D-058).
//!
//! One thread owns the ring. Tasks hand it requests through a queue and an
//! eventfd (whose read is kept in flight on the ring, so a write to it wakes the
//! ring thread), and park; the ring thread routes completions back by operation
//! id and wakes them.
//!
//! [`UringDriver::read`] has the semantics Sisu's runtime promises for a
//! blocking operation that lent a buffer to the kernel: it returns only after the
//! kernel has completed the read or confirmed it cancelled, even when the task is
//! cancelled while the read is in flight. On cancellation it submits
//! `IORING_OP_ASYNC_CANCEL`; when that reports `-EALREADY` (the read is already
//! executing in an io-wq worker and cannot be stopped) or `-ENOENT` (it had
//! already completed), the read still returns only on the original completion.
//! So a buffer on the task's own stack is never written after `read` returns.
//!
//! The unsafe [`UringDriver::start_read`] exists for the negative control
//! (`--break-contract`). Its safety contract is the one Rust cannot enforce and
//! Sisu's D-058 does: the in-flight handle must not be leaked.

use std::collections::VecDeque;
use std::io;
use std::marker::PhantomData;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::{Relaxed, SeqCst}};
use std::sync::{Arc, Mutex};
use std::task::Waker;
use std::thread;

use io_uring::{IoUring, opcode, squeue, types};

use super::api;
use super::slab::Slab;

const EVENTFD_TOKEN: u64 = u64::MAX;
const CANCEL_BIT: u64 = 1 << 62;

enum Request {
    Read { op: usize, fd: RawFd, buf: usize, len: u32, offset: u64, force_async: bool },
    Cancel { op: usize },
}

struct OpState {
    waker: Option<Waker>,
    read_res: Option<i32>,
    cancel_sent: bool,
    cancel_res: Option<i32>,
    task_gone: bool,
}

impl OpState {
    fn finished(&self) -> bool {
        self.read_res.is_some() && (!self.cancel_sent || self.cancel_res.is_some())
    }
}

/// Counts of what the kernel reported.
#[derive(Clone, Copy, Debug, Default)]
pub struct UringStats {
    pub reads_submitted: u64,
    pub reads_completed: u64,
    pub cancels_submitted: u64,
    /// Cancel results: found and cancelled (0).
    pub cancel_ok: u64,
    /// `-ENOENT`: the read had already completed.
    pub cancel_enoent: u64,
    /// `-EALREADY`: the read was executing and could not be stopped; waited for it.
    pub cancel_ealready: u64,
    pub cancel_other: u64,
}

struct Shared {
    queue: Mutex<VecDeque<Request>>,
    ops: Mutex<Slab<OpState>>,
    event_fd: OwnedFd,
    shutdown: AtomicBool,
    reads_submitted: AtomicU64,
    reads_completed: AtomicU64,
    cancels_submitted: AtomicU64,
    cancel_ok: AtomicU64,
    cancel_enoent: AtomicU64,
    cancel_ealready: AtomicU64,
    cancel_other: AtomicU64,
}

/// The driver; dropping it stops the ring thread (after in-flight operations end).
pub struct UringDriver {
    shared: Arc<Shared>,
    thread: Option<thread::JoinHandle<()>>,
}

/// What one read ended with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReadOutcome {
    /// Bytes read, or a negative errno (`-ECANCELED`, `-EINTR`, ...).
    pub result: i32,
    /// The cancel request's result, if a cancel was submitted (and has completed).
    pub cancel: Option<i32>,
}

impl UringDriver {
    /// Sets up a ring with `entries` submission entries. Fails (for the caller to
    /// report "skipped") when io_uring is unavailable or forbidden.
    pub fn new(entries: u32) -> io::Result<UringDriver> {
        let ring: IoUring = IoUring::builder().setup_cqsize(entries.saturating_mul(4)).build(entries)?;
        // SAFETY: eventfd creates a new descriptor we take ownership of.
        let efd = unsafe { libc::eventfd(0, libc::EFD_CLOEXEC) };
        if efd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: `efd` is a fresh descriptor.
        let event_fd = unsafe { OwnedFd::from_raw_fd(efd) };
        let shared = Arc::new(Shared {
            queue: Mutex::new(VecDeque::new()),
            ops: Mutex::new(Slab::new()),
            event_fd,
            shutdown: AtomicBool::new(false),
            reads_submitted: AtomicU64::new(0),
            reads_completed: AtomicU64::new(0),
            cancels_submitted: AtomicU64::new(0),
            cancel_ok: AtomicU64::new(0),
            cancel_enoent: AtomicU64::new(0),
            cancel_ealready: AtomicU64::new(0),
            cancel_other: AtomicU64::new(0),
        });
        let sh = shared.clone();
        let thread = thread::Builder::new().name("taskbench-uring".into()).spawn(move || ring_thread(ring, sh))?;
        Ok(UringDriver { shared, thread: Some(thread) })
    }

    pub fn stats(&self) -> UringStats {
        let s = &self.shared;
        UringStats {
            reads_submitted: s.reads_submitted.load(Relaxed),
            reads_completed: s.reads_completed.load(Relaxed),
            cancels_submitted: s.cancels_submitted.load(Relaxed),
            cancel_ok: s.cancel_ok.load(Relaxed),
            cancel_enoent: s.cancel_enoent.load(Relaxed),
            cancel_ealready: s.cancel_ealready.load(Relaxed),
            cancel_other: s.cancel_other.load(Relaxed),
        }
    }

    fn push(&self, r: Request) {
        self.shared.queue.lock().expect("uring queue lock").push_back(r);
        let one: u64 = 1;
        // SAFETY: writes 8 bytes from a valid u64 to our eventfd.
        unsafe { libc::write(self.shared.event_fd.as_raw_fd(), &one as *const u64 as *const libc::c_void, 8) };
    }

    /// Reads into `buf` (which may live on the calling task's stack). A blocking
    /// point: if the task is cancelled meanwhile, the read is cancelled, but this
    /// returns only once the kernel has let go of `buf`. `on_submitted` runs right
    /// after the request is queued.
    pub fn read(&self, fd: RawFd, buf: &mut [u8], offset: u64, force_async: bool, on_submitted: impl FnOnce()) -> ReadOutcome {
        // SAFETY: the handle is consumed by `wait` below, never leaked.
        let op = unsafe { self.start_read(fd, buf, offset, force_async) };
        on_submitted();
        op.wait()
    }

    /// Starts a read and returns its in-flight handle.
    ///
    /// # Safety
    /// The returned handle must not be leaked (`mem::forget`): while the read is in
    /// flight the kernel may write into `buf`, and the handle's methods and drop
    /// are what wait for it to stop. Rust cannot express "must not be leaked";
    /// Sisu's rule D-058 makes it the language's guarantee.
    pub unsafe fn start_read<'b>(&'b self, fd: RawFd, buf: &'b mut [u8], offset: u64, force_async: bool) -> InFlight<'b> {
        let waker = api::current_waker();
        let op = self.shared.ops.lock().expect("uring ops lock").insert(OpState {
            waker: Some(waker),
            read_res: None,
            cancel_sent: false,
            cancel_res: None,
            task_gone: false,
        });
        self.shared.reads_submitted.fetch_add(1, Relaxed);
        self.push(Request::Read { op, fd, buf: buf.as_mut_ptr() as usize, len: buf.len() as u32, offset, force_async });
        InFlight { driver: self, op, finished: false, _buf: PhantomData }
    }
}

impl Drop for UringDriver {
    fn drop(&mut self) {
        self.shared.shutdown.store(true, SeqCst);
        let one: u64 = 1;
        // SAFETY: as in `push`.
        unsafe { libc::write(self.shared.event_fd.as_raw_fd(), &one as *const u64 as *const libc::c_void, 8) };
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// A read the kernel may still be writing into `buf`.
pub struct InFlight<'b> {
    driver: &'b UringDriver,
    op: usize,
    finished: bool,
    _buf: PhantomData<&'b mut [u8]>,
}

impl InFlight<'_> {
    fn state(&self) -> (Option<i32>, bool, Option<i32>) {
        let ops = self.driver.shared.ops.lock().expect("uring ops lock");
        let st = ops.get(self.op).expect("op slot");
        (st.read_res, st.cancel_sent, st.cancel_res)
    }

    fn send_cancel(&self) {
        {
            let mut ops = self.driver.shared.ops.lock().expect("uring ops lock");
            let Some(st) = ops.get_mut(self.op) else { return };
            if st.cancel_sent || st.read_res.is_some() {
                return;
            }
            st.cancel_sent = true;
        }
        self.driver.shared.cancels_submitted.fetch_add(1, Relaxed);
        self.driver.push(Request::Cancel { op: self.op });
    }

    fn release(&mut self) {
        self.finished = true;
        let mut ops = self.driver.shared.ops.lock().expect("uring ops lock");
        let done = match ops.get_mut(self.op) {
            Some(st) => {
                st.task_gone = true;
                st.waker = None;
                st.finished()
            }
            None => false,
        };
        if done {
            ops.remove(self.op);
        }
    }

    /// Waits for the original completion, cancelling the read if the task is
    /// cancelled first (D-058 semantics).
    pub fn wait(mut self) -> ReadOutcome {
        loop {
            let (res, _, cancel) = self.state();
            if let Some(r) = res {
                self.release();
                return ReadOutcome { result: r, cancel };
            }
            if api::is_cancelled() {
                self.send_cancel();
            }
            api::park();
        }
    }

    /// Negative control: returns `None` as soon as the task is cancelled, leaving
    /// the read in flight (the kernel may still write into the buffer).
    pub fn wait_or_abandon(&mut self) -> Option<ReadOutcome> {
        loop {
            let (res, _, cancel) = self.state();
            if let Some(r) = res {
                self.release();
                return Some(ReadOutcome { result: r, cancel });
            }
            if api::is_cancelled() {
                return None;
            }
            api::park();
        }
    }

    /// Cancels the read and waits until the kernel has let go of the buffer.
    pub fn cancel_and_wait(mut self) -> ReadOutcome {
        self.send_cancel();
        loop {
            let (res, _, cancel) = self.state();
            if let Some(r) = res {
                self.release();
                return ReadOutcome { result: r, cancel };
            }
            api::park();
        }
    }
}

impl Drop for InFlight<'_> {
    fn drop(&mut self) {
        if !self.finished {
            // Never return with the kernel still holding the buffer.
            self.send_cancel();
            loop {
                if self.state().0.is_some() {
                    self.release();
                    return;
                }
                api::park();
            }
        }
    }
}

fn ring_thread(mut ring: IoUring, sh: Arc<Shared>) {
    let mut efd_buf = [0u8; 8];
    let arm_eventfd = |ring: &mut IoUring, buf: &mut [u8; 8]| {
        let e = opcode::Read::new(types::Fd(sh.event_fd.as_raw_fd()), buf.as_mut_ptr(), 8).build().user_data(EVENTFD_TOKEN);
        push_sqe(ring, &e);
    };
    arm_eventfd(&mut ring, &mut efd_buf);
    let mut pending_shutdown = false;
    loop {
        if let Err(e) = ring.submit_and_wait(1) {
            if e.raw_os_error() == Some(libc::EINTR) {
                continue;
            }
            eprintln!("taskbench: io_uring submit_and_wait failed: {e}");
            return;
        }
        let cqes: Vec<(u64, i32)> = ring.completion().map(|c| (c.user_data(), c.result())).collect();
        let mut rearm = false;
        for (ud, res) in cqes {
            if ud == EVENTFD_TOKEN {
                rearm = true;
                continue;
            }
            let (op, is_cancel) = if ud & CANCEL_BIT != 0 { ((ud & !CANCEL_BIT) as usize, true) } else { (ud as usize, false) };
            let waker = {
                let mut ops = sh.ops.lock().expect("uring ops lock");
                let Some(st) = ops.get_mut(op) else { continue };
                if is_cancel {
                    st.cancel_res = Some(res);
                    match -res {
                        0 => sh.cancel_ok.fetch_add(1, Relaxed),
                        libc::ENOENT => sh.cancel_enoent.fetch_add(1, Relaxed),
                        libc::EALREADY => sh.cancel_ealready.fetch_add(1, Relaxed),
                        _ => sh.cancel_other.fetch_add(1, Relaxed),
                    };
                } else {
                    st.read_res = Some(res);
                    sh.reads_completed.fetch_add(1, Relaxed);
                }
                let w = st.waker.clone();
                if st.task_gone && st.finished() {
                    ops.remove(op);
                }
                w
            };
            if let Some(w) = waker {
                w.wake();
            }
        }
        if rearm {
            // Drain the request queue into the ring.
            let reqs: Vec<Request> = sh.queue.lock().expect("uring queue lock").drain(..).collect();
            for r in reqs {
                let e = match r {
                    Request::Read { op, fd, buf, len, offset, force_async } => {
                        let e = opcode::Read::new(types::Fd(fd), buf as *mut u8, len).offset(offset).build().user_data(op as u64);
                        if force_async { e.flags(squeue::Flags::ASYNC) } else { e }
                    }
                    Request::Cancel { op } => opcode::AsyncCancel::new(op as u64).build().user_data(op as u64 | CANCEL_BIT),
                };
                push_sqe(&mut ring, &e);
            }
            if sh.shutdown.load(SeqCst) {
                pending_shutdown = true;
            } else {
                arm_eventfd(&mut ring, &mut efd_buf);
            }
        }
        if pending_shutdown && sh.ops.lock().expect("uring ops lock").iter().count() == 0 {
            return;
        }
        if pending_shutdown {
            // Keep reaping until every operation has finished.
            continue;
        }
    }
}

fn push_sqe(ring: &mut IoUring, e: &squeue::Entry) {
    loop {
        // SAFETY: the buffers referenced by read entries stay valid until their
        // completion is reaped: task buffers by the `InFlight` contract, the
        // eventfd buffer because it lives on this thread's stack for the
        // thread's lifetime.
        let pushed = unsafe { ring.submission().push(e).is_ok() };
        if pushed {
            return;
        }
        let _ = ring.submit();
    }
}
