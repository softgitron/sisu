//! The task: one unit of scheduling, stackful (a corosensei coroutine on a stack
//! from the strategy) or stackless (a boxed future).
//!
//! # State machine
//!
//! `state` is one of SCHEDULED (in exactly one run queue, or about to be),
//! RUNNING (owned by the worker executing it), PARKED (in no queue; a waker
//! will reschedule it) and DONE. `notified` is a wake-up token: `unpark` sets it
//! and moves a PARKED task to SCHEDULED; a parking task that finds the token set
//! does not sleep. The two SeqCst operations on each side (store state / load
//! token in the worker, store token / CAS state in `unpark`) guarantee that one
//! of them sees the other, and the CAS that a task is scheduled exactly once.
//!
//! # Exclusive fields
//!
//! `body` and `local` are touched only by whoever owns the RUNNING state: the
//! task's own code while it runs, and its worker just before resuming it and just
//! after it suspends. Ownership passes between threads only through the run
//! queues (release/acquire), which is the invariant behind `unsafe impl Sync`.

use std::any::Any;
use std::cell::UnsafeCell;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize};
use std::task::Wake;

use corosensei::{Coroutine, Yielder};

use super::scope::ScopeInner;
use super::worker::{Hint, Shared};
use crate::stack::probe::StackExhausted;
use crate::stack::{Exhaustion, StackRegion, TaskStack};

pub(crate) const SCHEDULED: u8 = 0;
pub(crate) const RUNNING: u8 = 1;
pub(crate) const PARKED: u8 = 2;
pub(crate) const DONE: u8 = 3;

/// Why a stackful task handed control back to its worker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Suspend {
    /// Cooperative yield: requeue at the back of the global queue.
    Yield,
    /// Block until unparked.
    Park,
}

/// How a stackful task's coroutine returned.
#[derive(Debug)]
pub enum TaskExit {
    /// The body ran to completion (or its panic was caught) and it has already
    /// reported its result to its join slot.
    Finished,
    /// A guard-page fault ended the task; the frames on its stack were discarded
    /// without running their destructors.
    GuardTrap { fault_addr: usize },
}

/// The error half of a join: how a task failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskError {
    /// The task, or a scope above it, was cancelled; observed at a blocking point.
    Cancelled,
    /// The task panicked.
    Panicked(String),
    /// The task ran out of stack and was failed cleanly.
    StackExhausted { how: Exhaustion, detail: String },
    /// No stack could be obtained for the task.
    SpawnFailed(String),
    /// An I/O operation failed.
    Io(String),
}

impl TaskError {
    /// Converts a caught panic payload.
    pub fn from_panic(payload: Box<dyn Any + Send>) -> TaskError {
        if let Some(se) = payload.downcast_ref::<StackExhausted>() {
            return TaskError::StackExhausted {
                how: if super::api::current_is_stackless() { Exhaustion::WorkerStackProbe } else { Exhaustion::SoftwareProbe },
                detail: format!("probe fired at sp {:#x} (limit {:#x})", se.sp, se.limit),
            };
        }
        if let Some(s) = payload.downcast_ref::<&'static str>() {
            return TaskError::Panicked((*s).to_string());
        }
        if let Some(s) = payload.downcast_ref::<String>() {
            return TaskError::Panicked(s.clone());
        }
        TaskError::Panicked("panic with a non-string payload".into())
    }

    pub fn is_stack_exhausted(&self) -> bool {
        matches!(self, TaskError::StackExhausted { .. })
    }

    /// Whether this failure cancels the failing task's siblings (panics and
    /// exhaustion do; cancellation itself does not).
    pub(crate) fn cancels_siblings(&self) -> bool {
        matches!(self, TaskError::Panicked(_) | TaskError::StackExhausted { .. })
    }
}

impl fmt::Display for TaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskError::Cancelled => write!(f, "cancelled"),
            TaskError::Panicked(m) => write!(f, "panicked: {m}"),
            TaskError::StackExhausted { how, detail } => write!(f, "stack exhausted ({}): {detail}", how.as_str()),
            TaskError::SpawnFailed(m) => write!(f, "spawn failed: {m}"),
            TaskError::Io(m) => write!(f, "I/O error: {m}"),
        }
    }
}

impl std::error::Error for TaskError {}

/// Receives a task's failure when its body could not report it itself (a guard
/// trap discards the body's frames).
pub(crate) trait FailSink: Send + Sync {
    fn fail(&self, err: TaskError);
}

pub(crate) type Co = Coroutine<(), Suspend, TaskExit, TaskStack>;
pub(crate) type BoxFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

pub(crate) enum Body {
    Empty,
    Stackful(Co),
    Stackless(BoxFuture),
    /// A trapped task whose open scopes still have running children, which may
    /// borrow from its stack: the stack is kept until they finish.
    Quarantined(TaskStack),
}

/// Fields used only by the thread that owns the task's RUNNING state.
pub(crate) struct TaskLocal {
    pub yielder: *const Yielder<(), Suspend>,
    /// Stack pointer recorded just before parking (strategy D).
    pub park_sp: usize,
    /// Innermost scope this task has open; each scope links to the one below it.
    pub open_scope: Option<Arc<ScopeInner>>,
    /// Copy of the stack region (absent for stackless tasks).
    pub region: Option<StackRegion>,
}

pub struct Task {
    pub(crate) id: u64,
    pub(crate) state: AtomicU8,
    pub(crate) notified: AtomicBool,
    /// Cancellation of this task alone (`JoinHandle::cancel`).
    pub(crate) cancelled: AtomicBool,
    pub(crate) stackless: bool,
    /// The runtime this task belongs to; valid for the task's whole life because
    /// the runtime outlives its tasks (see `Runtime`'s drop).
    pub(crate) rt: *const Shared,
    /// The scope the task was spawned in.
    pub(crate) scope: Arc<ScopeInner>,
    /// This task's index in `scope`'s registry.
    pub(crate) reg_index: AtomicUsize,
    pub(crate) fail: Arc<dyn FailSink>,
    pub(crate) body: UnsafeCell<Body>,
    pub(crate) local: UnsafeCell<TaskLocal>,
}

// SAFETY: see the module docs: `body` and `local` are accessed only by the owner
// of the RUNNING state, handed over through the run queues; everything else is
// atomic or immutable after construction. `rt` points to a `Shared` that is Sync.
unsafe impl Sync for Task {}
// SAFETY: as above; the coroutine inside is only resumed by the owning worker and
// task bodies are required to be `Send`.
unsafe impl Send for Task {}

pub(crate) type TaskRef = Arc<Task>;

impl Task {
    pub(crate) fn new(
        id: u64,
        rt: *const Shared,
        scope: Arc<ScopeInner>,
        fail: Arc<dyn FailSink>,
        region: Option<StackRegion>,
        stackless: bool,
    ) -> Task {
        Task {
            id,
            state: AtomicU8::new(SCHEDULED),
            notified: AtomicBool::new(false),
            cancelled: AtomicBool::new(false),
            stackless,
            rt,
            scope,
            reg_index: AtomicUsize::new(usize::MAX),
            fail,
            body: UnsafeCell::new(Body::Empty),
            local: UnsafeCell::new(TaskLocal { yielder: std::ptr::null(), park_sp: 0, open_scope: None, region }),
        }
    }

    pub(crate) fn shared(&self) -> &Shared {
        // SAFETY: the runtime outlives its tasks (field invariant).
        unsafe { &*self.rt }
    }

    /// Wakes the task if it is parked; otherwise leaves a token so that its next
    /// park returns immediately.
    pub(crate) fn unpark(self: &Arc<Self>) {
        self.notified.store(true, SeqCst);
        if self.state.compare_exchange(PARKED, SCHEDULED, SeqCst, SeqCst).is_ok() {
            self.shared().schedule(self.clone(), Hint::Wake);
        }
    }

    /// Whether this task or any scope above it has been cancelled.
    pub(crate) fn is_cancelled(&self) -> bool {
        self.cancelled.load(SeqCst) || self.scope.chain_cancelled()
    }

    /// # Safety
    /// The caller must own the task's RUNNING state (be the task, or its worker
    /// while it is suspended in the worker's hands).
    #[allow(clippy::mut_from_ref)]
    pub(crate) unsafe fn local_mut(&self) -> &mut TaskLocal {
        // SAFETY: exclusive by the caller's contract.
        unsafe { &mut *self.local.get() }
    }
}

impl Wake for Task {
    fn wake(self: Arc<Self>) {
        self.unpark();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.unpark();
    }
}
