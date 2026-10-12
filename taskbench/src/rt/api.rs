//! The task-side API: what code running inside a task calls.
//!
//! Blocking operations (`park`, `sleep`, `JoinHandle::join`, the gate, the I/O
//! helpers) are cancellation points: they return `Err(Cancelled)` once the task
//! or a scope above it is cancelled. `yield_now` is the compiler-inserted yield
//! point of proposal/06 and is also a cancellation point here.

use std::any::Any;
use std::future::Future;
use std::panic::{self, AssertUnwindSafe};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::Ordering::SeqCst;
use std::task::{Context, Poll, Wake, Waker};
use std::thread::{self, Thread};
use std::time::{Duration, Instant};

use super::task::{Suspend, Task, TaskError};
use super::worker::current_task_ptr;
use crate::stack::probe::{self, LimitProbe};

/// The running task, if the caller is one.
///
/// The returned reference is valid while the task runs: a task is kept alive by
/// its scope's registry until it has finished.
pub(crate) fn current_task<'a>() -> Option<&'a Task> {
    // SAFETY: CURRENT is set by the worker to a task it holds a reference to, and
    // the task's own code runs only while the task is alive.
    unsafe { current_task_ptr().as_ref() }
}

/// Whether the caller runs inside a task of this crate's runtime.
pub fn in_task() -> bool {
    !current_task_ptr().is_null()
}

pub(crate) fn current_is_stackless() -> bool {
    current_task().map(|t| t.stackless).unwrap_or(false)
}

/// The current task's id.
pub fn task_id() -> Option<u64> {
    current_task().map(|t| t.id)
}

/// A waker that unparks the current task.
pub fn current_waker() -> Waker {
    let t = current_task_ptr();
    assert!(!t.is_null(), "current_waker called outside a task");
    // SAFETY: the task is alive (see `current_task`) and was allocated in an Arc
    // by the runtime; bumping the count gives this waker its own reference.
    let arc = unsafe {
        Arc::increment_strong_count(t);
        Arc::from_raw(t)
    };
    Waker::from(arc)
}

struct ThreadWaker(Thread);

impl Wake for ThreadWaker {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.0.unpark();
    }
}

/// A waker that unparks the calling OS thread (for waits outside tasks).
pub(crate) fn thread_waker() -> Waker {
    Waker::from(Arc::new(ThreadWaker(thread::current())))
}

/// Blocks the current stackful task until it is unparked (by a waker obtained
/// from [`current_waker`]). May return spuriously; callers loop on a condition.
pub fn park() {
    let t = current_task().expect("park called outside a task");
    assert!(!t.stackless, "park called from a stackless task");
    if t.notified.swap(false, SeqCst) {
        return;
    }
    // SAFETY: we are the running task and own `local`.
    let local = unsafe { t.local_mut() };
    local.park_sp = probe::current_sp();
    let y = local.yielder;
    // SAFETY: `yielder` was set when this coroutine started and points to its
    // parent link at the top of this stack; we are executing on that stack.
    unsafe { (*y).suspend(Suspend::Park) };
    t.notified.store(false, SeqCst);
}

/// The yield point: lets other tasks run. Returns `Err(Cancelled)` if the task
/// has been cancelled.
pub fn yield_now() -> Result<(), TaskError> {
    let t = current_task().expect("yield_now called outside a task");
    // SAFETY: as in `park`.
    let y = unsafe { t.local_mut() }.yielder;
    // SAFETY: as in `park`.
    unsafe { (*y).suspend(Suspend::Yield) };
    check_cancelled()
}

/// Whether the current task, or a scope above it, has been cancelled.
pub fn is_cancelled() -> bool {
    current_task().map(|t| t.is_cancelled()).unwrap_or(false)
}

/// `Err(Cancelled)` if the current task has been cancelled.
pub fn check_cancelled() -> Result<(), TaskError> {
    if is_cancelled() { Err(TaskError::Cancelled) } else { Ok(()) }
}

/// Sleeps the current stackful task; a cancellation point.
pub fn sleep(d: Duration) -> Result<(), TaskError> {
    let t = current_task().expect("sleep called outside a task");
    let deadline = Instant::now() + d;
    if t.shared().timer.register(deadline, current_waker()) {
        t.shared().timer_changed();
    }
    loop {
        check_cancelled()?;
        if Instant::now() >= deadline {
            return Ok(());
        }
        park();
    }
}

/// The software probe for the current stackful task: fires below its stack's
/// probe limit (B, C, D(B)); never fires for strategies with a guard page.
pub fn probe() -> LimitProbe {
    let t = current_task().expect("probe called outside a task");
    // SAFETY: we are the running task and own `local`.
    match unsafe { t.local_mut() }.region {
        Some(r) if r.probe_limit != 0 => LimitProbe::new(r.probe_limit),
        _ => LimitProbe::disabled(),
    }
}

/// The current task's stack region (stackful tasks only).
pub fn stack_region() -> Option<crate::stack::StackRegion> {
    let t = current_task()?;
    // SAFETY: we are the running task and own `local`.
    unsafe { t.local_mut() }.region
}

/// Yield point for stackless tasks.
pub fn yield_now_async() -> impl Future<Output = Result<(), TaskError>> {
    let mut yielded = false;
    std::future::poll_fn(move |cx| {
        if yielded {
            return Poll::Ready(check_cancelled());
        }
        yielded = true;
        cx.waker().wake_by_ref();
        Poll::Pending
    })
}

/// Sleep for stackless tasks; a cancellation point.
pub fn sleep_async(d: Duration) -> impl Future<Output = Result<(), TaskError>> {
    let deadline = Instant::now() + d;
    let mut registered = false;
    std::future::poll_fn(move |cx| {
        if is_cancelled() {
            return Poll::Ready(Err(TaskError::Cancelled));
        }
        if Instant::now() >= deadline {
            return Poll::Ready(Ok(()));
        }
        if !registered {
            registered = true;
            let t = current_task().expect("sleep_async outside a task");
            if t.shared().timer.register(deadline, cx.waker().clone()) {
                t.shared().timer_changed();
            }
        }
        Poll::Pending
    })
}

/// Polls a future inside `catch_unwind`, so that a panicking stackless task
/// fails that task instead of the worker.
pub(crate) struct CatchUnwind<F> {
    inner: F,
}

impl<F> CatchUnwind<F> {
    pub(crate) fn new(inner: F) -> CatchUnwind<F> {
        CatchUnwind { inner }
    }
}

impl<F: Future> Future for CatchUnwind<F> {
    type Output = Result<F::Output, Box<dyn Any + Send>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // SAFETY: structural pinning of the only field: `inner` is never moved out
        // of the pinned `CatchUnwind`.
        let inner = unsafe { self.map_unchecked_mut(|s| &mut s.inner) };
        match panic::catch_unwind(AssertUnwindSafe(|| inner.poll(cx))) {
            Ok(Poll::Ready(v)) => Poll::Ready(Ok(v)),
            Ok(Poll::Pending) => Poll::Pending,
            Err(p) => Poll::Ready(Err(p)),
        }
    }
}
