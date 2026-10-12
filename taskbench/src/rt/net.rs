//! Colourless TCP for tasks: non-blocking sockets plus an epoll poller thread.
//!
//! A read that would block registers the task's waker on the socket and parks;
//! the poller wakes it on readiness. Edge-triggered readiness is tracked as a
//! per-direction event counter: a task snapshots it before trying the system call
//! and, after registering, re-checks it, so an event that lands between the
//! attempt and the registration is never lost.
//!
//! This is the simplest correct design (one dedicated poller thread), not Go's
//! (idle workers poll the network themselves). The echo benchmark's latency
//! includes the poller-to-worker hand-off; the report must say so.

use std::io;
use std::net::{SocketAddr, TcpListener as StdListener, TcpStream as StdStream};
use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd, OwnedFd, RawFd};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::SeqCst};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::thread;

use super::api;
use super::slab::Slab;
use super::task::TaskError;
use super::worker::Shared;

const WAKE_TOKEN: u64 = u64::MAX;

/// Readiness state of one registered file descriptor.
pub(crate) struct IoSource {
    read_gen: AtomicU64,
    write_gen: AtomicU64,
    read_waiter: Mutex<Option<Waker>>,
    write_waiter: Mutex<Option<Waker>>,
}

#[derive(Clone, Copy)]
enum Dir {
    Read,
    Write,
}

impl IoSource {
    fn generation(&self, d: Dir) -> u64 {
        match d {
            Dir::Read => self.read_gen.load(SeqCst),
            Dir::Write => self.write_gen.load(SeqCst),
        }
    }

    fn set_waiter(&self, d: Dir, w: Waker) {
        let slot = match d {
            Dir::Read => &self.read_waiter,
            Dir::Write => &self.write_waiter,
        };
        let mut g = slot.lock().expect("io waiter lock");
        match &*g {
            Some(old) if old.will_wake(&w) => {}
            _ => *g = Some(w),
        }
    }

    fn fire(&self, d: Dir) {
        let (counter, slot) = match d {
            Dir::Read => (&self.read_gen, &self.read_waiter),
            Dir::Write => (&self.write_gen, &self.write_waiter),
        };
        counter.fetch_add(1, SeqCst);
        let w = slot.lock().expect("io waiter lock").take();
        if let Some(w) = w {
            w.wake();
        }
    }
}

/// The epoll poller of one runtime.
pub(crate) struct Poller {
    epfd: OwnedFd,
    wake_fd: OwnedFd,
    sources: Mutex<Slab<Arc<IoSource>>>,
    thread: Mutex<Option<thread::JoinHandle<()>>>,
    shutdown: AtomicBool,
}

struct PollerPtr(*const Poller);
// SAFETY: the poller lives in the runtime's shared state, which outlives the
// poller thread (joined at shutdown).
unsafe impl Send for PollerPtr {}

fn cvt(r: libc::c_int) -> io::Result<libc::c_int> {
    if r < 0 { Err(io::Error::last_os_error()) } else { Ok(r) }
}

impl Poller {
    fn new() -> io::Result<Poller> {
        // SAFETY: plain system calls creating new descriptors we take ownership of.
        let epfd = unsafe { OwnedFd::from_raw_fd(cvt(libc::epoll_create1(libc::EPOLL_CLOEXEC))?) };
        // SAFETY: as above.
        let wake_fd = unsafe { OwnedFd::from_raw_fd(cvt(libc::eventfd(0, libc::EFD_NONBLOCK | libc::EFD_CLOEXEC))?) };
        let mut ev = libc::epoll_event { events: libc::EPOLLIN as u32, u64: WAKE_TOKEN };
        // SAFETY: both descriptors are valid; `ev` is a valid event.
        cvt(unsafe { libc::epoll_ctl(epfd.as_raw_fd(), libc::EPOLL_CTL_ADD, wake_fd.as_raw_fd(), &mut ev) })?;
        Ok(Poller {
            epfd,
            wake_fd,
            sources: Mutex::new(Slab::new()),
            thread: Mutex::new(None),
            shutdown: AtomicBool::new(false),
        })
    }

    fn start(&self) {
        let p = PollerPtr(self);
        let h = thread::Builder::new()
            .name("taskbench-poller".into())
            .spawn(move || {
                let p = p;
                // SAFETY: see PollerPtr.
                unsafe { (*p.0).run() }
            })
            .expect("spawn poller thread");
        *self.thread.lock().expect("poller thread lock") = Some(h);
    }

    fn run(&self) {
        let mut events = vec![libc::epoll_event { events: 0, u64: 0 }; 1024];
        let mut ready: Vec<(Arc<IoSource>, u32)> = Vec::with_capacity(1024);
        loop {
            // SAFETY: `events` is a valid buffer of the length passed.
            let n = unsafe { libc::epoll_wait(self.epfd.as_raw_fd(), events.as_mut_ptr(), events.len() as i32, -1) };
            if self.shutdown.load(SeqCst) {
                return;
            }
            if n < 0 {
                continue; // EINTR
            }
            {
                let sources = self.sources.lock().expect("poller sources lock");
                for ev in &events[..n as usize] {
                    let token = ev.u64;
                    if token == WAKE_TOKEN {
                        continue;
                    }
                    if let Some(src) = sources.get(token as usize) {
                        ready.push((src.clone(), ev.events));
                    }
                }
            }
            for (src, bits) in ready.drain(..) {
                let err = (libc::EPOLLERR | libc::EPOLLHUP) as u32;
                if bits & (libc::EPOLLIN as u32 | libc::EPOLLRDHUP as u32 | err) != 0 {
                    src.fire(Dir::Read);
                }
                if bits & (libc::EPOLLOUT as u32 | err) != 0 {
                    src.fire(Dir::Write);
                }
            }
        }
    }

    fn register(&self, fd: RawFd) -> io::Result<(usize, Arc<IoSource>)> {
        let src = Arc::new(IoSource {
            read_gen: AtomicU64::new(0),
            write_gen: AtomicU64::new(0),
            read_waiter: Mutex::new(None),
            write_waiter: Mutex::new(None),
        });
        let idx = self.sources.lock().expect("poller sources lock").insert(src.clone());
        let mut ev = libc::epoll_event {
            events: (libc::EPOLLIN | libc::EPOLLOUT | libc::EPOLLRDHUP | libc::EPOLLET) as u32,
            u64: idx as u64,
        };
        // SAFETY: valid descriptors and event.
        if let Err(e) = cvt(unsafe { libc::epoll_ctl(self.epfd.as_raw_fd(), libc::EPOLL_CTL_ADD, fd, &mut ev) }) {
            self.sources.lock().expect("poller sources lock").remove(idx);
            return Err(e);
        }
        Ok((idx, src))
    }

    fn deregister(&self, idx: usize, fd: RawFd) {
        // SAFETY: valid descriptors; a null event is allowed for EPOLL_CTL_DEL.
        unsafe { libc::epoll_ctl(self.epfd.as_raw_fd(), libc::EPOLL_CTL_DEL, fd, std::ptr::null_mut()) };
        self.sources.lock().expect("poller sources lock").remove(idx);
    }

    /// Stops the poller thread and joins it.
    pub(crate) fn shutdown(&self) {
        self.shutdown.store(true, SeqCst);
        let one: u64 = 1;
        // SAFETY: writing 8 bytes from a valid u64 to our eventfd.
        unsafe { libc::write(self.wake_fd.as_raw_fd(), &one as *const u64 as *const libc::c_void, 8) };
        let h = self.thread.lock().expect("poller thread lock").take();
        if let Some(h) = h {
            let _ = h.join();
        }
    }
}

impl Shared {
    /// The runtime's poller, started on first use.
    pub(crate) fn poller(&self) -> io::Result<&Poller> {
        if let Some(p) = self.poller.get() {
            return Ok(p);
        }
        let p = Poller::new()?;
        if self.poller.set(p).is_ok() {
            self.poller.get().expect("just set").start();
        }
        Ok(self.poller.get().expect("set"))
    }
}

fn current_shared<'a>() -> io::Result<&'a Shared> {
    api::current_task()
        .map(|t| t.shared())
        .ok_or_else(|| io::Error::other("network calls must run inside a task"))
}

fn io_err(e: io::Error) -> TaskError {
    TaskError::Io(e.to_string())
}

/// A registered non-blocking descriptor.
struct Registered {
    fd: OwnedFd,
    idx: usize,
    src: Arc<IoSource>,
    rt: *const Shared,
}

// SAFETY: the descriptor and source are thread-safe; `rt` points to the Sync
// runtime that outlives every task using this object.
unsafe impl Send for Registered {}
// SAFETY: as above.
unsafe impl Sync for Registered {}

impl Registered {
    fn new(fd: OwnedFd) -> io::Result<Registered> {
        let sh = current_shared()?;
        let (idx, src) = sh.poller()?.register(fd.as_raw_fd())?;
        Ok(Registered { fd, idx, src, rt: sh })
    }

    /// Runs `op` until it stops returning EAGAIN, parking the task in between.
    fn blocking<T>(&self, d: Dir, mut op: impl FnMut(RawFd) -> isize, done: impl Fn(isize) -> T) -> Result<T, TaskError> {
        loop {
            let g = self.src.generation(d);
            let r = op(self.fd.as_raw_fd());
            if r >= 0 {
                return Ok(done(r));
            }
            let e = io::Error::last_os_error();
            match e.raw_os_error() {
                Some(libc::EAGAIN) => {}
                Some(libc::EINTR) => continue,
                _ => return Err(io_err(e)),
            }
            api::check_cancelled()?;
            self.src.set_waiter(d, api::current_waker());
            if self.src.generation(d) == g {
                api::park();
            }
        }
    }

    fn poll_op<T>(&self, cx: &mut Context<'_>, d: Dir, mut op: impl FnMut(RawFd) -> isize, done: impl Fn(isize) -> T) -> Poll<Result<T, TaskError>> {
        loop {
            let g = self.src.generation(d);
            let r = op(self.fd.as_raw_fd());
            if r >= 0 {
                return Poll::Ready(Ok(done(r)));
            }
            let e = io::Error::last_os_error();
            match e.raw_os_error() {
                Some(libc::EAGAIN) => {}
                Some(libc::EINTR) => continue,
                _ => return Poll::Ready(Err(io_err(e))),
            }
            if api::is_cancelled() {
                return Poll::Ready(Err(TaskError::Cancelled));
            }
            self.src.set_waiter(d, cx.waker().clone());
            if self.src.generation(d) == g {
                return Poll::Pending;
            }
        }
    }
}

impl Drop for Registered {
    fn drop(&mut self) {
        // SAFETY: the runtime outlives its tasks and so this object.
        let sh = unsafe { &*self.rt };
        if let Some(p) = sh.poller.get() {
            p.deregister(self.idx, self.fd.as_raw_fd());
        }
    }
}

fn read_op(buf: &mut [u8]) -> impl FnMut(RawFd) -> isize + '_ {
    // SAFETY: reads into a valid, writable buffer of the given length.
    move |fd| unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) }
}

fn write_op(buf: &[u8]) -> impl FnMut(RawFd) -> isize + '_ {
    // SAFETY: writes from a valid buffer; MSG_NOSIGNAL avoids SIGPIPE.
    move |fd| unsafe { libc::send(fd, buf.as_ptr() as *const libc::c_void, buf.len(), libc::MSG_NOSIGNAL) }
}

fn accept_op(fd: RawFd) -> isize {
    // SAFETY: accept4 with null address out-parameters is valid.
    unsafe { libc::accept4(fd, std::ptr::null_mut(), std::ptr::null_mut(), libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC) as isize }
}

/// Sets TCP_NODELAY (echo latency would otherwise measure Nagle's algorithm).
pub fn set_nodelay(fd: RawFd) {
    let one: libc::c_int = 1;
    // SAFETY: valid socket and option buffer.
    unsafe {
        libc::setsockopt(fd, libc::IPPROTO_TCP, libc::TCP_NODELAY, &one as *const _ as *const libc::c_void, 4);
    }
}

/// A listening socket whose `accept` parks the task.
pub struct TcpListener {
    reg: Registered,
}

impl TcpListener {
    /// Binds and listens with the given backlog; must be called inside a task.
    pub fn bind(addr: SocketAddr, backlog: i32) -> io::Result<TcpListener> {
        let l = StdListener::bind(addr)?;
        l.set_nonblocking(true)?;
        // SAFETY: re-listen on the bound socket with a larger backlog.
        unsafe { libc::listen(l.as_raw_fd(), backlog) };
        // SAFETY: we take ownership of the descriptor from the std listener.
        let fd = unsafe { OwnedFd::from_raw_fd(l.into_raw_fd()) };
        Ok(TcpListener { reg: Registered::new(fd)? })
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        // SAFETY: borrow the descriptor as a std listener without taking ownership.
        let l = std::mem::ManuallyDrop::new(unsafe { StdListener::from_raw_fd(self.reg.fd.as_raw_fd()) });
        l.local_addr()
    }

    /// Accepts one connection (a blocking point).
    pub fn accept(&self) -> Result<TcpConn, TaskError> {
        let fd = self.reg.blocking(Dir::Read, accept_op, |r| r as RawFd)?;
        // SAFETY: accept4 returned a new descriptor we now own.
        let fd = unsafe { OwnedFd::from_raw_fd(fd) };
        set_nodelay(fd.as_raw_fd());
        Ok(TcpConn { reg: Registered::new(fd).map_err(io_err)? })
    }

    /// Stackless `accept`.
    pub async fn accept_async(&self) -> Result<TcpConn, TaskError> {
        let fd = std::future::poll_fn(|cx| self.reg.poll_op(cx, Dir::Read, accept_op, |r| r as RawFd)).await?;
        // SAFETY: accept4 returned a new descriptor we now own.
        let fd = unsafe { OwnedFd::from_raw_fd(fd) };
        set_nodelay(fd.as_raw_fd());
        Ok(TcpConn { reg: Registered::new(fd).map_err(io_err)? })
    }
}

/// A connected socket whose reads and writes park the task.
pub struct TcpConn {
    reg: Registered,
}

impl TcpConn {
    /// Adopts a connected std stream (made non-blocking); must be called inside a task.
    pub fn from_std(s: StdStream) -> io::Result<TcpConn> {
        s.set_nonblocking(true)?;
        set_nodelay(s.as_raw_fd());
        // SAFETY: we take ownership of the descriptor from the std stream.
        let fd = unsafe { OwnedFd::from_raw_fd(s.into_raw_fd()) };
        Ok(TcpConn { reg: Registered::new(fd)? })
    }

    pub fn raw_fd(&self) -> RawFd {
        self.reg.fd.as_raw_fd()
    }

    /// Reads some bytes; 0 means end of stream. A blocking point.
    pub fn read(&self, buf: &mut [u8]) -> Result<usize, TaskError> {
        self.reg.blocking(Dir::Read, read_op(buf), |r| r as usize)
    }

    /// Fills `buf` completely, or fails with an I/O error at end of stream.
    pub fn read_exact(&self, buf: &mut [u8]) -> Result<(), TaskError> {
        let mut off = 0;
        while off < buf.len() {
            let n = self.read(&mut buf[off..])?;
            if n == 0 {
                return Err(TaskError::Io("unexpected end of stream".into()));
            }
            off += n;
        }
        Ok(())
    }

    /// Writes all of `buf`. A blocking point.
    pub fn write_all(&self, buf: &[u8]) -> Result<(), TaskError> {
        let mut off = 0;
        while off < buf.len() {
            off += self.reg.blocking(Dir::Write, write_op(&buf[off..]), |r| r as usize)?;
        }
        Ok(())
    }

    /// Stackless `read`.
    pub async fn read_async(&self, buf: &mut [u8]) -> Result<usize, TaskError> {
        std::future::poll_fn(|cx| self.reg.poll_op(cx, Dir::Read, read_op(&mut *buf), |r| r as usize)).await
    }

    /// Stackless `read_exact`.
    pub async fn read_exact_async(&self, buf: &mut [u8]) -> Result<(), TaskError> {
        let mut off = 0;
        while off < buf.len() {
            let n = self.read_async(&mut buf[off..]).await?;
            if n == 0 {
                return Err(TaskError::Io("unexpected end of stream".into()));
            }
            off += n;
        }
        Ok(())
    }

    /// Stackless `write_all`.
    pub async fn write_all_async(&self, buf: &[u8]) -> Result<(), TaskError> {
        let mut off = 0;
        while off < buf.len() {
            let rest = &buf[off..];
            off += std::future::poll_fn(|cx| self.reg.poll_op(cx, Dir::Write, write_op(rest), |r| r as usize)).await?;
        }
        Ok(())
    }
}
