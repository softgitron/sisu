//! The M:N scheduler: N worker threads over a global injector queue, one local
//! FIFO deque per worker, a one-task `next` slot per worker, and work stealing.
//!
//! The design copies the parts of Go's scheduler that matter for the benchmarks
//! and nothing else:
//!
//! * A task spawned or woken by a task goes to its worker's `next` slot (Go's
//!   `runnext`), displacing the previous occupant into the local deque, so that
//!   spawn-then-join and wake-then-block run on one worker without a cross-thread
//!   hand-off. The `next` slot is not stealable.
//! * A yielding task goes to the back of the global queue, which every worker
//!   checks first once every 61 scheduling decisions, so yields are fair.
//! * The workers poll the network themselves (every 31 decisions, and when out
//!   of work), and a task woken by readiness is queued FIFO, never into `next`
//!   (Go does the same with netpoll results; it halved the echo p99 here).
//! * Due timers are checked at every scheduling decision and fire into `next`.
//! * An idle worker spins for a few microseconds looking for work, then sleeps:
//!   one sleeper blocks in `epoll_pwait2`, one owns the earliest timer deadline.
//!   Pushing stealable work wakes a sleeper only when no worker is spinning, so a
//!   burst of pushes costs one wake-up, not one per push.
//!
//! Correctness over cleverness: queues are crossbeam's Chase-Lev deques, the idle
//! list is a mutex, and every task state transition is SeqCst.

use std::cell::Cell;
use std::mem;
use std::panic::{self, AssertUnwindSafe};
use std::ptr;
use std::sync::atomic::Ordering::{Relaxed, SeqCst};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, fence};
use std::sync::{Arc, Mutex, OnceLock};
use std::task::{Context, Poll, Waker};
use std::thread::{self, Thread};
use std::time::Instant;

use corosensei::CoroutineResult;
use crossbeam_deque::{Injector, Steal, Stealer, Worker};

use super::net::Poller;
use super::scope::{JoinSlot, ScopeInner};
use super::task::{
    Body, BoxFuture, DONE, FailSink, PARKED, RUNNING, SCHEDULED, Suspend, Task, TaskError, TaskExit, TaskRef,
};
use super::timer::Timer;
use crate::stack::fault::{self, ArmedTrap};
use crate::stack::{Exhaustion, StackError, StackStrategy, TaskStack, probe};

/// Rounds of searching for work before an idle worker sleeps.
const SPIN_ROUNDS: u32 = 64;
/// `spin_loop` hints between search rounds.
const SPIN_PAUSES: u32 = 64;
/// Every this many scheduling decisions, the global queue is checked first.
const GLOBAL_QUEUE_INTERVAL: u32 = 61;
/// Every this many scheduling decisions, a worker polls the network (without
/// blocking) even when it has work, so that I/O readiness is not starved by a
/// busy run queue.
const POLL_INTERVAL: u32 = 31;
/// `poll_blocker` and `timer_owner` value when no worker holds the role.
const NOBODY: usize = usize::MAX;
/// Stack kept free at the bottom of a worker thread for unwinding out of a
/// stackless task whose `poll` recursion hit the worker-stack probe.
const WORKER_PROBE_RESERVE: usize = 64 * 1024;

/// Spin budget, overridable for experiments with `TASKBENCH_SPIN=rounds,pauses`.
fn spin_params() -> (u32, u32) {
    static P: OnceLock<(u32, u32)> = OnceLock::new();
    *P.get_or_init(|| {
        std::env::var("TASKBENCH_SPIN")
            .ok()
            .and_then(|v| {
                let (a, b) = v.split_once(',')?;
                Some((a.parse().ok()?, b.parse().ok()?))
            })
            .unwrap_or((SPIN_ROUNDS, SPIN_PAUSES))
    })
}

/// What the worker is doing when it wakes a task (see `WorkerCtx::poll_mode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PollMode {
    None,
    /// Polling because it ran out of work: woken tasks go to its local queue.
    Idle,
    /// Polling between tasks: woken tasks go to the global queue.
    Busy,
}

/// Where a newly runnable task should go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Hint {
    /// Newly spawned.
    Spawn,
    /// Woken from park.
    Wake,
    /// Cooperative yield.
    Yield,
}

struct WorkerSlot {
    thread: OnceLock<Thread>,
    wake: AtomicBool,
}

/// Scheduler counters, reported by some benchmarks.
#[derive(Default)]
pub(crate) struct RtCounters {
    pub spawned: AtomicU64,
    pub completed: AtomicU64,
    pub steals: AtomicU64,
    pub yields: AtomicU64,
    pub parks: AtomicU64,
    pub worker_wakeups: AtomicU64,
    pub guard_traps: AtomicU64,
    pub quarantined: AtomicU64,
}

/// A snapshot of the scheduler counters.
#[derive(Clone, Copy, Debug, Default)]
pub struct RtStats {
    pub spawned: u64,
    pub completed: u64,
    pub steals: u64,
    pub yields: u64,
    pub parks: u64,
    pub worker_wakeups: u64,
    pub guard_traps: u64,
    pub quarantined: u64,
}

/// State shared by the workers and every task of one runtime.
pub(crate) struct Shared {
    pub(crate) injector: Injector<TaskRef>,
    stealers: Vec<Stealer<TaskRef>>,
    slots: Vec<WorkerSlot>,
    idle: Mutex<Vec<usize>>,
    num_idle: AtomicUsize,
    num_spinning: AtomicUsize,
    /// The idle worker blocked in `epoll_wait`, woken by the poller's eventfd
    /// instead of its thread parker.
    poll_blocker: AtomicUsize,
    /// The sleeping worker that wakes at the earliest timer deadline.
    timer_owner: AtomicUsize,
    pub(crate) shutdown: AtomicBool,
    pub(crate) live_tasks: AtomicUsize,
    pub(crate) strategy: Arc<dyn StackStrategy>,
    /// Arm the guard-page fault handler when resuming (A, D(A)).
    guard_pages: bool,
    /// Call `on_park` when a stackful task parks (D).
    decommit: bool,
    /// D(B): probe limits double as decommit watermarks.
    watermark: bool,
    pub(crate) timer: Timer,
    pub(crate) poller: OnceLock<Poller>,
    pub(crate) counters: RtCounters,
    next_id: AtomicU64,
}

/// Per-worker scheduling state; lives on the worker thread's stack.
struct WorkerCtx {
    index: usize,
    shared: *const Shared,
    local: Worker<TaskRef>,
    next: Cell<Option<TaskRef>>,
    tick: Cell<u32>,
    rng: Cell<u64>,
    /// Set while this worker processes network readiness: woken tasks are queued
    /// FIFO (Go does the same with netpoll results) instead of taking the `next`
    /// slot, which is for tasks woken by the running task. During a periodic poll
    /// by a busy worker they go to the global queue, where idle workers find them.
    poll_mode: Cell<PollMode>,
}

thread_local! {
    static WORKER: Cell<*const WorkerCtx> = const { Cell::new(ptr::null()) };
    static CURRENT: Cell<*const Task> = const { Cell::new(ptr::null()) };
}

// Thread-local accessors are never inlined: a stackful task may resume on a
// different OS thread after any suspension, and an inlined accessor would let the
// compiler reuse a thread-local address computed before the suspension.

/// The task running on this thread, or null.
#[inline(never)]
pub(crate) fn current_task_ptr() -> *const Task {
    CURRENT.with(|c| c.get())
}

#[inline(never)]
fn current_worker_ptr() -> *const WorkerCtx {
    WORKER.with(|c| c.get())
}

/// Whether the calling thread is a worker of any runtime.
pub(crate) fn on_worker_thread() -> bool {
    !current_worker_ptr().is_null()
}

/// A raw pointer that may be moved to a worker thread.
struct SendPtr(*const Shared);
// SAFETY: `Shared` is Sync and outlives the threads it is sent to (they are joined
// before it is freed).
unsafe impl Send for SendPtr {}

impl Shared {
    /// Creates the shared state and starts `threads` workers.
    pub(crate) fn start(
        threads: usize,
        strategy: Arc<dyn StackStrategy>,
        worker_stack: usize,
    ) -> std::io::Result<(*const Shared, Vec<thread::JoinHandle<()>>)> {
        let threads = threads.max(1);
        let guard_pages = strategy.exhaustion() == Exhaustion::GuardPage;
        let decommit = strategy.decommits_on_park();
        let watermark = strategy.watermark_probes();
        if guard_pages {
            fault::install_handler();
        }
        super::panic_hook::install();
        let workers: Vec<Worker<TaskRef>> = (0..threads).map(|_| Worker::new_fifo()).collect();
        let stealers = workers.iter().map(|w| w.stealer()).collect();
        let shared = Box::new(Shared {
            injector: Injector::new(),
            stealers,
            slots: (0..threads).map(|_| WorkerSlot { thread: OnceLock::new(), wake: AtomicBool::new(false) }).collect(),
            idle: Mutex::new(Vec::with_capacity(threads)),
            num_idle: AtomicUsize::new(0),
            num_spinning: AtomicUsize::new(0),
            poll_blocker: AtomicUsize::new(NOBODY),
            timer_owner: AtomicUsize::new(NOBODY),
            shutdown: AtomicBool::new(false),
            live_tasks: AtomicUsize::new(0),
            strategy,
            guard_pages,
            decommit,
            watermark,
            timer: Timer::new(),
            poller: OnceLock::new(),
            counters: RtCounters::default(),
            next_id: AtomicU64::new(1),
        });
        let ptr: *const Shared = Box::into_raw(shared);
        let mut handles = Vec::with_capacity(threads);
        for (index, local) in workers.into_iter().enumerate() {
            let sp = SendPtr(ptr);
            let h = thread::Builder::new().name(format!("taskbench-w{index}")).stack_size(worker_stack).spawn(
                move || {
                    let sp = sp;
                    worker_main(sp.0, index, local)
                },
            )?;
            handles.push(h);
        }
        Ok((ptr, handles))
    }

    pub(crate) fn stats(&self) -> RtStats {
        let c = &self.counters;
        RtStats {
            spawned: c.spawned.load(Relaxed),
            completed: c.completed.load(Relaxed),
            steals: c.steals.load(Relaxed),
            yields: c.yields.load(Relaxed),
            parks: c.parks.load(Relaxed),
            worker_wakeups: c.worker_wakeups.load(Relaxed),
            guard_traps: c.guard_traps.load(Relaxed),
            quarantined: c.quarantined.load(Relaxed),
        }
    }

    pub(crate) fn worker_count(&self) -> usize {
        self.slots.len()
    }

    fn next_task_id(&self) -> u64 {
        self.next_id.fetch_add(1, Relaxed)
    }

    /// Makes `task` (in state SCHEDULED) runnable.
    ///
    /// From a worker of this runtime: a spawned task, or one woken by the running
    /// task, takes the worker's `next` slot (displacing the previous occupant into
    /// the local queue); a task woken by network readiness is queued FIFO (local
    /// queue when the worker was idle, global queue when it polled between tasks);
    /// a yielding task goes to the back of the global queue. From any other thread
    /// the task goes to the global queue.
    pub(crate) fn schedule(&self, task: TaskRef, hint: Hint) {
        let w = current_worker_ptr();
        // SAFETY: a non-null WORKER pointer refers to the WorkerCtx on this thread's
        // stack, which lives as long as the worker loop that set it.
        let local = unsafe { w.as_ref() }.filter(|w| ptr::eq(w.shared, self));
        let Some(w) = local else {
            self.injector.push(task);
            self.notify_work();
            return;
        };
        match (hint, w.poll_mode.get()) {
            (Hint::Yield, _) | (Hint::Wake, PollMode::Busy) => self.injector.push(task),
            (Hint::Wake, PollMode::Idle) => w.local.push(task),
            (Hint::Spawn | Hint::Wake, _) => match w.next.replace(Some(task)) {
                None => return,
                Some(prev) => w.local.push(prev),
            },
        }
        self.notify_work();
    }

    /// Wakes a sleeping worker if stealable work exists and nobody is looking for it.
    fn notify_work(&self) {
        fence(SeqCst);
        if self.num_spinning.load(SeqCst) == 0 && self.num_idle.load(SeqCst) > 0 {
            self.wake_one();
        }
    }

    fn wake_one(&self) {
        // Become the one new spinner; if someone else is already spinning they will
        // find the work.
        if self.num_spinning.compare_exchange(0, 1, SeqCst, SeqCst).is_err() {
            return;
        }
        let popped = self.idle.lock().expect("idle list lock").pop();
        match popped {
            Some(i) => {
                self.num_idle.fetch_sub(1, SeqCst);
                self.wake_worker(i);
                self.counters.worker_wakeups.fetch_add(1, Relaxed);
            }
            None => {
                self.num_spinning.fetch_sub(1, SeqCst);
            }
        }
    }

    /// Gives worker `i` its wake token and wakes it wherever it sleeps. The
    /// SeqCst store of the token before the load of `poll_blocker` pairs with the
    /// worker's store of `poll_blocker` before its load of the token, so one of
    /// the two always sees the other.
    fn wake_worker(&self, i: usize) {
        self.slots[i].wake.store(true, SeqCst);
        if self.poll_blocker.load(SeqCst) == i
            && let Some(p) = self.poller.get()
        {
            p.notify();
        }
        if let Some(t) = self.slots[i].thread.get() {
            t.unpark();
        }
    }

    fn has_work(&self) -> bool {
        !self.injector.is_empty() || self.stealers.iter().any(|s| !s.is_empty())
    }

    /// Takes worker `index` off the idle list. Returns true if a waker already
    /// did (then the worker is counted as spinning and its token is consumed).
    fn leave_idle(&self, index: usize) -> bool {
        let mut idle = self.idle.lock().expect("idle list lock");
        if let Some(pos) = idle.iter().position(|&i| i == index) {
            idle.swap_remove(pos);
            self.num_idle.fetch_sub(1, SeqCst);
            return false;
        }
        drop(idle);
        // Popped by a waker, which counted us as spinning: its token is on the way.
        while !self.slots[index].wake.swap(false, SeqCst) {
            std::hint::spin_loop();
        }
        true
    }

    /// Sleeps until woken, until work or a due timer appears, or until network
    /// events were handled. Returns true when woken by `wake_one` (the caller is
    /// then counted as spinning).
    ///
    /// One sleeping worker blocks in `epoll_wait` (when there is a poller), so that
    /// readiness wakes it directly; one owns the timer and sleeps only until the
    /// earliest deadline. A waker sets the token before checking `poll_blocker`,
    /// and the sleeper sets `poll_blocker` before checking the token (and checks it
    /// again under the poll lock), so a wake-up is never lost.
    fn sleep_worker(&self, ctx: &WorkerCtx) -> bool {
        let index = ctx.index;
        {
            let mut idle = self.idle.lock().expect("idle list lock");
            idle.push(index);
            self.num_idle.fetch_add(1, SeqCst);
        }
        fence(SeqCst);
        let slot = &self.slots[index];
        let keep_sleeping = || !slot.wake.load(SeqCst) && !self.shutdown.load(SeqCst);
        loop {
            if slot.wake.swap(false, SeqCst) {
                return true;
            }
            if self.has_work() || self.shutdown.load(SeqCst) || self.timer.is_due() {
                return self.leave_idle(index);
            }
            let deadline = self.timer.next_deadline();
            let owns_timer =
                deadline.is_some() && self.timer_owner.compare_exchange(NOBODY, index, SeqCst, SeqCst).is_ok();
            let timeout = if owns_timer { deadline.map(|d| d.saturating_duration_since(Instant::now())) } else { None };
            let mut events = 0;
            let poller = self.poller.get();
            let blocker = poller.is_some() && self.poll_blocker.compare_exchange(NOBODY, index, SeqCst, SeqCst).is_ok();
            if let (true, Some(p)) = (blocker, poller) {
                ctx.poll_mode.set(PollMode::Idle);
                let polled = p.poll_blocking(&keep_sleeping, timeout);
                ctx.poll_mode.set(PollMode::None);
                self.poll_blocker.store(NOBODY, SeqCst);
                match polled {
                    Some(n) => events = n,
                    // A busy worker holds the poll lock for a non-blocking poll;
                    // let it finish rather than spin on the lock.
                    None => thread::yield_now(),
                }
            } else if keep_sleeping() {
                match timeout {
                    Some(t) => thread::park_timeout(t),
                    None => thread::park(),
                }
            }
            if owns_timer {
                self.timer_owner.store(NOBODY, SeqCst);
            }
            if slot.wake.swap(false, SeqCst) {
                return true;
            }
            if events > 0 {
                return self.leave_idle(index);
            }
        }
    }

    /// After registering a timer that became the earliest: makes the timer owner
    /// (or, if there is none, some sleeping worker) re-read the deadline. A nudge
    /// is a wake-up without a token, so the worker only loops in `sleep_worker`.
    pub(crate) fn timer_changed(&self) {
        let owner = self.timer_owner.load(SeqCst);
        let target =
            if owner != NOBODY { Some(owner) } else { self.idle.lock().expect("idle list lock").last().copied() };
        if let Some(i) = target {
            if self.poll_blocker.load(SeqCst) == i
                && let Some(p) = self.poller.get()
            {
                p.notify();
            }
            if let Some(t) = self.slots[i].thread.get() {
                t.unpark();
            }
        }
    }

    /// Stops all workers once they run out of work.
    pub(crate) fn begin_shutdown(&self) {
        self.shutdown.store(true, SeqCst);
        for slot in &self.slots {
            slot.wake.store(true, SeqCst);
            if let Some(t) = slot.thread.get() {
                t.unpark();
            }
        }
        if let Some(p) = self.poller.get() {
            p.notify();
        }
    }

    /// Spawns a stackful task in `scope`.
    ///
    /// # Safety
    /// Data borrowed by `f` (and by `T`) must stay valid until the task has
    /// finished: the caller must not return, nor let the borrowed data be dropped,
    /// before `scope`'s active count shows the task complete. `Scope::spawn` and
    /// `Runtime::block_on` guarantee this by waiting for their children.
    pub(crate) unsafe fn spawn_stackful<F, T>(
        &self,
        scope: &Arc<ScopeInner>,
        f: F,
    ) -> Result<(TaskRef, Arc<JoinSlot<T>>), StackError>
    where
        F: FnOnce() -> T + Send,
        T: Send,
    {
        let region = self.strategy.allocate()?;
        let owner: *const dyn StackStrategy = Arc::as_ptr(&self.strategy);
        // SAFETY: `region` came from this runtime's strategy, which outlives every
        // task (the runtime is only freed once no task is alive).
        let stack = unsafe { TaskStack::new(region, owner) };
        let slot = Arc::new(JoinSlot::<T>::new(scope.clone()));
        let sink: Arc<dyn FailSink + '_> = slot.clone();
        // SAFETY: erases the lifetime of `T` from the trait object. The task drops
        // its sink when it completes (`complete`), before the spawning scope can
        // end, so the sink is never used or dropped after borrowed data dies.
        let sink: Arc<dyn FailSink> = unsafe { std::mem::transmute::<Arc<dyn FailSink + '_>, Arc<dyn FailSink>>(sink) };
        let task = Arc::new(Task::new(self.next_task_id(), self, scope.clone(), sink, Some(region), false));
        if self.watermark {
            // A slot is handed out clean below its top page (fresh, or decommitted
            // when released), so the first watermark is the top page.
            let page = crate::stack::os::page_size();
            // SAFETY: the task is not yet published; we are its only user.
            unsafe { task.local_mut() }.mark.clean_below = crate::units::round_down(region.base - 1, page);
        }
        let slot2 = slot.clone();
        let body = move |y: &corosensei::Yielder<(), Suspend>, ()| -> TaskExit {
            let me = current_task_ptr();
            // SAFETY: the worker set CURRENT to this task before resuming it, and the
            // task owns its RUNNING state while its body runs.
            unsafe { (*me).local_mut().yielder = y };
            let r = panic::catch_unwind(AssertUnwindSafe(f));
            slot2.complete(r.map_err(TaskError::from_panic));
            TaskExit::Finished
        };
        // SAFETY: the closure may borrow non-'static data; the caller guarantees it
        // outlives the coroutine (function contract).
        let co = unsafe { corosensei::Coroutine::with_stack_unchecked(stack, body) };
        // SAFETY: the task is not yet published; we are its only user.
        unsafe { *task.body.get() = Body::Stackful(co) };
        self.publish(scope, &task);
        Ok((task, slot))
    }

    /// Spawns a stackless task in `scope`.
    pub(crate) fn spawn_stackless<Fut, T>(&self, scope: &Arc<ScopeInner>, fut: Fut) -> (TaskRef, Arc<JoinSlot<T>>)
    where
        Fut: std::future::Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let slot = Arc::new(JoinSlot::<T>::new(scope.clone()));
        let task = Arc::new(Task::new(self.next_task_id(), self, scope.clone(), slot.clone(), None, true));
        let slot2 = slot.clone();
        let wrapped = async move {
            let r = super::api::CatchUnwind::new(fut).await;
            slot2.complete(r.map_err(TaskError::from_panic));
        };
        let boxed: BoxFuture = Box::pin(wrapped);
        // SAFETY: the task is not yet published; we are its only user.
        unsafe { *task.body.get() = Body::Stackless(boxed) };
        self.publish(scope, &task);
        (task, slot)
    }

    fn publish(&self, scope: &Arc<ScopeInner>, task: &TaskRef) {
        let idx = scope.register(task.clone());
        task.reg_index.store(idx, SeqCst);
        self.live_tasks.fetch_add(1, SeqCst);
        self.counters.spawned.fetch_add(1, Relaxed);
        self.schedule(task.clone(), Hint::Spawn);
    }
}

fn worker_main(shared: *const Shared, index: usize, local: Worker<TaskRef>) {
    // SAFETY: `shared` outlives this thread (joined before it is freed).
    let sh = unsafe { &*shared };
    let ctx = WorkerCtx {
        index,
        shared,
        local,
        next: Cell::new(None),
        tick: Cell::new(0),
        rng: Cell::new(0x9e37_79b9_7f4a_7c15 ^ ((index as u64 + 1) << 17)),
        poll_mode: Cell::new(PollMode::None),
    };
    let _ = sh.slots[index].thread.set(thread::current());
    WORKER.with(|w| w.set(&ctx));
    let _alt = if sh.guard_pages {
        match fault::AltStack::install() {
            Ok(a) => Some(a),
            Err(e) => {
                eprintln!("taskbench: worker {index}: no alternate signal stack ({e}); guard-page traps will crash");
                None
            }
        }
    } else {
        None
    };
    probe::init_worker_limit(WORKER_PROBE_RESERVE);
    // Whether this worker is counted in `num_spinning`.
    let mut counted = false;
    loop {
        if let Some(t) = ctx.find_task() {
            if counted {
                counted = false;
                sh.stop_spinning_with_work();
            }
            ctx.run(t);
            continue;
        }
        if sh.shutdown.load(SeqCst) {
            break;
        }
        if sh.timer.fire_due() > 0 {
            continue;
        }
        if let Some(p) = sh.poller.get() {
            ctx.poll_mode.set(PollMode::Idle);
            let n = p.try_poll();
            ctx.poll_mode.set(PollMode::None);
            if matches!(n, Some(n) if n > 0) {
                continue;
            }
        }
        if !counted {
            sh.num_spinning.fetch_add(1, SeqCst);
        }
        if let Some(t) = ctx.spin_for_work() {
            counted = false;
            sh.stop_spinning_with_work();
            ctx.run(t);
            continue;
        }
        sh.num_spinning.fetch_sub(1, SeqCst);
        counted = false;
        if sh.shutdown.load(SeqCst) {
            break;
        }
        counted = sh.sleep_worker(&ctx);
    }
    if counted {
        sh.num_spinning.fetch_sub(1, SeqCst);
    }
    WORKER.with(|w| w.set(ptr::null()));
}

impl Shared {
    /// A spinner found work; if it was the last spinner, there may be more work
    /// than workers, so wake another (Go's rule).
    fn stop_spinning_with_work(&self) {
        if self.num_spinning.fetch_sub(1, SeqCst) == 1 {
            self.notify_work();
        }
    }
}

impl WorkerCtx {
    fn shared(&self) -> &Shared {
        // SAFETY: the runtime outlives its workers.
        unsafe { &*self.shared }
    }

    fn next_rand(&self) -> usize {
        // xorshift64
        let mut x = self.rng.get();
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng.set(x);
        x as usize
    }

    /// Searches for work for a few microseconds before the worker sleeps.
    fn spin_for_work(&self) -> Option<TaskRef> {
        let (rounds, pauses) = spin_params();
        for _ in 0..rounds {
            for _ in 0..pauses {
                std::hint::spin_loop();
            }
            if let Some(t) = self.find_task() {
                return Some(t);
            }
        }
        None
    }

    fn find_task(&self) -> Option<TaskRef> {
        let sh = self.shared();
        let tick = self.tick.get().wrapping_add(1);
        self.tick.set(tick);
        // Due timers are checked at every scheduling decision (one atomic load and
        // a clock read); the tasks they wake take the `next` slot.
        if sh.timer.is_due() {
            sh.timer.fire_due();
        }
        if tick.is_multiple_of(POLL_INTERVAL)
            && let Some(p) = sh.poller.get()
        {
            self.poll_mode.set(PollMode::Busy);
            p.try_poll();
            self.poll_mode.set(PollMode::None);
        }
        if tick.is_multiple_of(GLOBAL_QUEUE_INTERVAL) {
            loop {
                match sh.injector.steal() {
                    Steal::Success(t) => return Some(t),
                    Steal::Empty => break,
                    Steal::Retry => continue,
                }
            }
        }
        if let Some(t) = self.next.take() {
            return Some(t);
        }
        if let Some(t) = self.local.pop() {
            return Some(t);
        }
        loop {
            match sh.injector.steal_batch_and_pop(&self.local) {
                Steal::Success(t) => return Some(t),
                Steal::Empty => break,
                Steal::Retry => continue,
            }
        }
        let n = sh.stealers.len();
        if n > 1 {
            let start = self.next_rand() % n;
            for i in 0..n {
                let victim = (start + i) % n;
                if victim == self.index {
                    continue;
                }
                loop {
                    match sh.stealers[victim].steal_batch_and_pop(&self.local) {
                        Steal::Success(t) => {
                            sh.counters.steals.fetch_add(1, Relaxed);
                            return Some(t);
                        }
                        Steal::Empty => break,
                        Steal::Retry => continue,
                    }
                }
            }
        }
        None
    }

    fn run(&self, task: TaskRef) {
        let sh = self.shared();
        task.state.store(RUNNING, SeqCst);
        CURRENT.with(|c| c.set(Arc::as_ptr(&task)));
        // SAFETY: we own the RUNNING state, hence the body.
        let body = unsafe { &mut *task.body.get() };
        match body {
            Body::Stackful(co) => {
                // SAFETY: we own the RUNNING state, hence `local`.
                let local = unsafe { task.local_mut() };
                let limit = local.region.map(|r| r.probe_limit).unwrap_or(0);
                if sh.watermark && !local.mark.dirty {
                    probe::set_task_limits(local.mark.clean_below.max(limit), limit);
                } else {
                    probe::set_task_limits(limit, limit);
                }
                if sh.guard_pages {
                    // SAFETY: we own the RUNNING state, hence `local`.
                    if let Some(r) = unsafe { task.local_mut() }.region {
                        fault::arm(ArmedTrap {
                            handler: co.trap_handler(),
                            guard_lo: r.guard_lo,
                            guard_hi: r.guard_hi,
                        });
                    }
                }
                let res = panic::catch_unwind(AssertUnwindSafe(|| co.resume(())));
                if sh.guard_pages {
                    fault::disarm();
                }
                if probe::take_tripped() {
                    // SAFETY: the task has suspended; we still own its RUNNING state.
                    unsafe { task.local_mut() }.mark.dirty = true;
                }
                CURRENT.with(|c| c.set(ptr::null()));
                match res {
                    Ok(CoroutineResult::Yield(Suspend::Yield)) => {
                        sh.counters.yields.fetch_add(1, Relaxed);
                        task.state.store(SCHEDULED, SeqCst);
                        sh.schedule(task, Hint::Yield);
                    }
                    Ok(CoroutineResult::Yield(Suspend::Park)) => self.park_task(task, true),
                    Ok(CoroutineResult::Return(exit)) => self.finish_stackful(task, exit),
                    Err(payload) => {
                        // The body catches its own panics; this is a panic in the
                        // runtime's code on the task stack. The coroutine has ended.
                        // SAFETY: we own the RUNNING state.
                        unsafe { task.fail(TaskError::from_panic(payload)) };
                        self.finish_stackful(task, TaskExit::Finished);
                    }
                }
            }
            Body::Stackless(fut) => {
                // Stackless polls recurse on this thread's own stack.
                probe::set_split_stack_limit(probe::worker_limit());
                task.notified.store(false, SeqCst);
                let waker = Waker::from(task.clone());
                let mut cx = Context::from_waker(&waker);
                let res = panic::catch_unwind(AssertUnwindSafe(|| fut.as_mut().poll(&mut cx)));
                CURRENT.with(|c| c.set(ptr::null()));
                drop(waker);
                match res {
                    Ok(Poll::Pending) => self.park_task(task, false),
                    Ok(Poll::Ready(())) => {
                        *body = Body::Empty;
                        self.complete(task);
                    }
                    Err(payload) => {
                        *body = Body::Empty;
                        // SAFETY: we own the RUNNING state.
                        unsafe { task.fail(TaskError::from_panic(payload)) };
                        self.complete(task);
                    }
                }
            }
            Body::Quarantined(_) => {
                CURRENT.with(|c| c.set(ptr::null()));
                // The wake-up that scheduled this run is consumed before the
                // scopes are asked again: `park_task` reschedules a task whose
                // `notified` is still set, and a stackful task clears it in
                // `park`, which a quarantined task never reaches. A wake-up that
                // arrives after this (a child finishing) sets it again and is
                // honoured by `park_task`.
                task.notified.store(false, SeqCst);
                if self.open_scopes_busy(&task) {
                    self.park_task(task, false);
                } else {
                    *body = Body::Empty; // releases the stack
                    self.complete(task);
                }
            }
            Body::Empty => {
                CURRENT.with(|c| c.set(ptr::null()));
                debug_assert!(false, "scheduled a task with no body");
            }
        }
    }

    /// The task suspended to park (or a future returned Pending).
    fn park_task(&self, task: TaskRef, stackful: bool) {
        let sh = self.shared();
        sh.counters.parks.fetch_add(1, Relaxed);
        if stackful && sh.decommit {
            // SAFETY: the task is suspended and we still own its RUNNING state.
            let local = unsafe { task.local_mut() };
            if let Some(r) = local.region {
                sh.strategy.on_park(&r, local.park_sp, &mut local.mark);
            }
        }
        task.state.store(PARKED, SeqCst);
        if task.notified.load(SeqCst) && task.state.compare_exchange(PARKED, SCHEDULED, SeqCst, SeqCst).is_ok() {
            sh.schedule(task, Hint::Wake);
        }
        // Otherwise the task is kept alive by its scope's registry and by whatever
        // waker it registered, and a wake-up will reschedule it.
    }

    fn finish_stackful(&self, task: TaskRef, exit: TaskExit) {
        let sh = self.shared();
        // SAFETY: the coroutine has returned; we own the RUNNING state.
        let body = unsafe { &mut *task.body.get() };
        let Body::Stackful(co) = mem::replace(body, Body::Empty) else {
            unreachable!("finish_stackful on a task without a coroutine");
        };
        let stack = co.into_stack();
        if let TaskExit::GuardTrap { fault_addr } = exit {
            sh.counters.guard_traps.fetch_add(1, Relaxed);
            // SAFETY: we own the RUNNING state.
            unsafe {
                task.fail(TaskError::StackExhausted {
                    how: Exhaustion::GuardPage,
                    detail: format!("guard page hit at {fault_addr:#x}"),
                })
            };
            // Scopes opened by the discarded frames are still open, and their
            // children may borrow from this stack: cancel them and keep the stack
            // until they have finished.
            // SAFETY: as above.
            let local = unsafe { task.local_mut() };
            let mut s = local.open_scope.clone();
            while let Some(sc) = s {
                sc.cancel();
                s = sc.prev_open();
            }
            if self.open_scopes_busy(&task) {
                *body = Body::Quarantined(stack);
                sh.counters.quarantined.fetch_add(1, Relaxed);
                self.park_task(task, false);
                return;
            }
        }
        if sh.watermark {
            // SAFETY: the coroutine has returned; we own the RUNNING state.
            let local = unsafe { task.local_mut() };
            let page = crate::stack::os::page_size();
            if !local.mark.dirty
                && local.region.is_some_and(|r| local.mark.clean_below >= crate::units::round_down(r.base - 1, page))
            {
                stack.mark_clean();
            }
        }
        drop(stack);
        self.complete(task);
    }

    /// For a trapped task: registers it as waiter on every open scope that still
    /// has children and reports whether any has.
    fn open_scopes_busy(&self, task: &TaskRef) -> bool {
        // SAFETY: the task is not running; we own its RUNNING state.
        let local = unsafe { task.local_mut() };
        let mut busy = false;
        let mut s = local.open_scope.clone();
        while let Some(sc) = s {
            if sc.active() > 0 {
                sc.set_waiter(Waker::from(task.clone()));
                if sc.active() > 0 {
                    busy = true;
                }
            }
            s = sc.prev_open();
        }
        busy
    }

    fn complete(&self, task: TaskRef) {
        let sh = self.shared();
        task.state.store(DONE, SeqCst);
        // SAFETY: the task has ended; we own its RUNNING state.
        unsafe {
            task.local_mut().open_scope = None;
            task.drop_fail_sink();
        }
        task.scope.unregister(task.reg_index.load(SeqCst));
        sh.live_tasks.fetch_sub(1, SeqCst);
        sh.counters.completed.fetch_add(1, Relaxed);
        // Last: once the scope sees the child finished, its owner may return.
        task.scope.child_finished();
    }
}
