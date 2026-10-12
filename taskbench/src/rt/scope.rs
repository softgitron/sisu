//! Structured concurrency: scopes that cannot end before their children, join
//! handles, and cancellation of a scope's subtree.
//!
//! The rules simulated are those of proposal/06: a task is spawned only inside a
//! scope; the scope joins every child before it returns, even when its body
//! panics; a child that panics or exhausts its stack cancels its siblings and
//! reports the failure to its joiner, and a failure nobody joined becomes the
//! scope's result; cancelling a scope cancels its whole subtree, observed as
//! `Cancelled` at the next blocking point.
//!
//! The stackful `scope` mirrors `std::thread::scope`: children may borrow from the
//! enclosing frame because the scope cannot return while they run. The stackless
//! `AsyncScope` cannot offer that in Rust (a future can be leaked, so a scope
//! future cannot guarantee its children finish before borrowed data dies; see
//! N-07), so its children must be `'static`.

use std::marker::PhantomData;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::Ordering::SeqCst;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::{Arc, Mutex, Weak};
use std::task::{Context, Poll, Waker};
use std::thread;

use super::api;
use super::slab::Slab;
use super::task::{FailSink, TaskError, TaskRef};
use super::worker::Shared;

/// The shared state of one scope.
pub(crate) struct ScopeInner {
    parent: Option<Arc<ScopeInner>>,
    cancelled: AtomicBool,
    /// Children not yet finished (their stacks not yet released).
    active: AtomicUsize,
    /// Who to wake when `active` reaches zero.
    waiter: Mutex<Option<Waker>>,
    /// First failure nobody joined.
    failure: Mutex<Option<TaskError>>,
    /// Live children, so that cancellation can wake them.
    tasks: Mutex<Slab<TaskRef>>,
    /// Scopes opened inside this scope's tasks, for subtree cancellation.
    children: Mutex<Vec<Weak<ScopeInner>>>,
    /// The scope the owning task had open before this one.
    prev_open: Mutex<Option<Arc<ScopeInner>>>,
}

impl ScopeInner {
    fn new(parent: Option<Arc<ScopeInner>>, prev_open: Option<Arc<ScopeInner>>) -> ScopeInner {
        ScopeInner {
            parent,
            cancelled: AtomicBool::new(false),
            active: AtomicUsize::new(0),
            waiter: Mutex::new(None),
            failure: Mutex::new(None),
            tasks: Mutex::new(Slab::new()),
            children: Mutex::new(Vec::new()),
            prev_open: Mutex::new(prev_open),
        }
    }

    /// A scope with no parent (the root of `block_on`).
    pub(crate) fn root() -> Arc<ScopeInner> {
        Arc::new(ScopeInner::new(None, None))
    }

    /// A scope opened by a task of `parent`.
    pub(crate) fn child_of(parent: &Arc<ScopeInner>, prev_open: Option<Arc<ScopeInner>>) -> Arc<ScopeInner> {
        let s = Arc::new(ScopeInner::new(Some(parent.clone()), prev_open));
        let mut kids = parent.children.lock().expect("scope children lock");
        if kids.len() >= 64 && kids.len().is_power_of_two() {
            kids.retain(|w| w.strong_count() > 0);
        }
        kids.push(Arc::downgrade(&s));
        s
    }

    /// Whether this scope or any ancestor has been cancelled.
    pub(crate) fn chain_cancelled(&self) -> bool {
        let mut s = Some(self);
        while let Some(sc) = s {
            if sc.cancelled.load(SeqCst) {
                return true;
            }
            s = sc.parent.as_deref();
        }
        false
    }

    /// Cancels the subtree and wakes every task in it so that it can observe the
    /// cancellation at its blocking point.
    pub(crate) fn cancel(&self) {
        self.cancelled.store(true, SeqCst);
        self.wake_subtree();
    }

    fn wake_subtree(&self) {
        let tasks: Vec<TaskRef> = self.tasks.lock().expect("scope registry lock").iter().cloned().collect();
        for t in tasks {
            t.unpark();
        }
        let kids: Vec<Arc<ScopeInner>> = self
            .children
            .lock()
            .expect("scope children lock")
            .iter()
            .filter_map(Weak::upgrade)
            .collect();
        for k in kids {
            k.wake_subtree();
        }
    }

    /// Registers a new child; returns its registry index.
    pub(crate) fn register(&self, t: TaskRef) -> usize {
        self.active.fetch_add(1, SeqCst);
        self.tasks.lock().expect("scope registry lock").insert(t)
    }

    pub(crate) fn unregister(&self, index: usize) {
        let removed = self.tasks.lock().expect("scope registry lock").remove(index);
        drop(removed);
    }

    /// A child has completely finished (its stack is released).
    pub(crate) fn child_finished(&self) {
        if self.active.fetch_sub(1, SeqCst) == 1 {
            let w = self.waiter.lock().expect("scope waiter lock").take();
            if let Some(w) = w {
                w.wake();
            }
        }
    }

    pub(crate) fn active(&self) -> usize {
        self.active.load(SeqCst)
    }

    pub(crate) fn set_waiter(&self, w: Waker) {
        *self.waiter.lock().expect("scope waiter lock") = Some(w);
    }

    pub(crate) fn record_failure(&self, e: TaskError) {
        let mut f = self.failure.lock().expect("scope failure lock");
        if f.is_none() {
            *f = Some(e);
        }
    }

    pub(crate) fn take_failure(&self) -> Option<TaskError> {
        self.failure.lock().expect("scope failure lock").take()
    }

    pub(crate) fn prev_open(&self) -> Option<Arc<ScopeInner>> {
        self.prev_open.lock().expect("scope link lock").clone()
    }

    /// Blocks the calling stackful task until every child has finished. Not a
    /// cancellation point: a scope always joins.
    fn wait_children(&self) {
        loop {
            if self.active() == 0 {
                return;
            }
            self.set_waiter(api::current_waker());
            if self.active() == 0 {
                return;
            }
            api::park();
        }
    }

    /// Blocks the calling OS thread (not a task) until every child has finished.
    pub(crate) fn wait_children_blocking(&self) {
        let waker = api::thread_waker();
        loop {
            if self.active() == 0 {
                return;
            }
            self.set_waiter(waker.clone());
            if self.active() == 0 {
                return;
            }
            thread::park();
        }
    }

    /// Polls for "all children finished" from a stackless task.
    fn poll_children(&self, cx: &mut Context<'_>) -> Poll<()> {
        if self.active() == 0 {
            return Poll::Ready(());
        }
        self.set_waiter(cx.waker().clone());
        if self.active() == 0 { Poll::Ready(()) } else { Poll::Pending }
    }
}

struct SlotState<T> {
    result: Option<Result<T, TaskError>>,
    waiter: Option<Waker>,
    handle_alive: bool,
    done: bool,
}

/// Where a task's result goes; shared by the task and its handle.
pub(crate) struct JoinSlot<T> {
    scope: Arc<ScopeInner>,
    st: Mutex<SlotState<T>>,
}

impl<T> JoinSlot<T> {
    pub(crate) fn new(scope: Arc<ScopeInner>) -> JoinSlot<T> {
        JoinSlot { scope, st: Mutex::new(SlotState { result: None, waiter: None, handle_alive: true, done: false }) }
    }

    /// Records the task's result and wakes its joiner. A panic or stack
    /// exhaustion cancels the task's siblings; a failure whose handle is gone
    /// becomes the scope's result.
    pub(crate) fn complete(&self, r: Result<T, TaskError>) {
        let cancels = matches!(&r, Err(e) if e.cancels_siblings());
        let (waiter, unobserved) = {
            let mut st = self.st.lock().expect("join slot lock");
            st.done = true;
            let unobserved = if st.handle_alive {
                st.result = Some(r);
                None
            } else {
                r.err().filter(|e| *e != TaskError::Cancelled)
            };
            (st.waiter.take(), unobserved)
        };
        if let Some(e) = unobserved {
            self.scope.record_failure(e);
        }
        if cancels {
            self.scope.cancel();
        }
        if let Some(w) = waiter {
            w.wake();
        }
    }

    fn handle_dropped(&self) {
        let unobserved = {
            let mut st = self.st.lock().expect("join slot lock");
            st.handle_alive = false;
            st.waiter = None;
            st.result.take().and_then(|r| r.err()).filter(|e| *e != TaskError::Cancelled)
        };
        if let Some(e) = unobserved {
            self.scope.record_failure(e);
        }
    }

    fn is_done(&self) -> bool {
        self.st.lock().expect("join slot lock").done
    }

    /// Takes the result, or registers `waker` and returns `None`.
    fn take_or_register(&self, waker: &Waker) -> Option<Result<T, TaskError>> {
        let mut st = self.st.lock().expect("join slot lock");
        if let Some(r) = st.result.take() {
            return Some(r);
        }
        match &st.waiter {
            Some(w) if w.will_wake(waker) => {}
            _ => st.waiter = Some(waker.clone()),
        }
        None
    }

    /// Joins from a stackful task: parks until the result arrives or the joiner
    /// is cancelled.
    fn join_in_task(&self) -> Result<T, TaskError> {
        let waker = api::current_waker();
        loop {
            if let Some(r) = self.take_or_register(&waker) {
                return r;
            }
            if api::is_cancelled() {
                self.handle_dropped();
                return Err(TaskError::Cancelled);
            }
            api::park();
        }
    }

    /// Joins from an OS thread that is not a task.
    pub(crate) fn wait_blocking(&self) -> Result<T, TaskError> {
        let waker = api::thread_waker();
        loop {
            if let Some(r) = self.take_or_register(&waker) {
                return r;
            }
            thread::park();
        }
    }
}

impl<T: Send> FailSink for JoinSlot<T> {
    fn fail(&self, err: TaskError) {
        self.complete(Err(err));
    }
}

/// A scope in which stackful tasks can be spawned; see [`scope`].
pub struct Scope<'scope, 'env: 'scope> {
    inner: Arc<ScopeInner>,
    shared: *const Shared,
    _scope: PhantomData<&'scope mut &'scope ()>,
    _env: PhantomData<&'env mut &'env ()>,
}

// SAFETY: the scope's state is an Arc of Sync data and a pointer to the Sync
// runtime; spawning from several tasks at once is supported.
unsafe impl Sync for Scope<'_, '_> {}

/// Runs `f` with a new scope and waits for every task spawned in it.
///
/// Children may borrow anything that outlives the call. If `f` panics, the
/// children are cancelled and joined before the panic continues. The result is
/// `Err` with the first child failure that no handle observed.
///
/// Must be called from inside a stackful task.
pub fn scope<'env, F, R>(f: F) -> Result<R, TaskError>
where
    F: for<'scope> FnOnce(&'scope Scope<'scope, 'env>) -> R,
{
    let me = api::current_task().expect("taskbench::rt::scope must be called inside a task");
    assert!(!me.stackless, "use AsyncScope in stackless tasks");
    // SAFETY: we are the running task.
    let local = unsafe { me.local_mut() };
    let inner = ScopeInner::child_of(&me.scope, local.open_scope.take());
    local.open_scope = Some(inner.clone());
    let scope = Scope { inner: inner.clone(), shared: me.rt, _scope: PhantomData, _env: PhantomData };
    let result = panic::catch_unwind(AssertUnwindSafe(|| f(&scope)));
    if result.is_err() {
        inner.cancel();
    }
    inner.wait_children();
    // SAFETY: still the running task (possibly on another worker by now).
    unsafe { me.local_mut() }.open_scope = inner.prev_open();
    match result {
        Err(p) => panic::resume_unwind(p),
        Ok(v) => match inner.take_failure() {
            Some(e) => Err(e),
            None => Ok(v),
        },
    }
}

impl<'scope, 'env> Scope<'scope, 'env> {
    /// Spawns a stackful task; panics if no stack can be allocated.
    pub fn spawn<F, T>(&'scope self, f: F) -> JoinHandle<'scope, T>
    where
        F: FnOnce() -> T + Send + 'scope,
        T: Send + 'scope,
    {
        match self.try_spawn(f) {
            Ok(h) => h,
            Err(e) => panic!("spawn failed: {e}"),
        }
    }

    /// Spawns a stackful task, or reports why its stack could not be allocated.
    pub fn try_spawn<F, T>(&'scope self, f: F) -> Result<JoinHandle<'scope, T>, TaskError>
    where
        F: FnOnce() -> T + Send + 'scope,
        T: Send + 'scope,
    {
        // SAFETY: `self.shared` is the runtime of the task that opened the scope,
        // alive while the scope is. `scope()` does not return before every child
        // has finished, so data borrowed for 'scope outlives the task.
        let (task, slot) = unsafe { (*self.shared).spawn_stackful(&self.inner, f) }
            .map_err(TaskError::SpawnFailed)?;
        Ok(JoinHandle { slot, task, joined: false, _scope: PhantomData })
    }

    /// Cancels every task in this scope and below.
    pub fn cancel(&self) {
        self.inner.cancel();
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.chain_cancelled()
    }

    /// Children not yet finished.
    pub fn active(&self) -> usize {
        self.inner.active()
    }
}

/// A handle to a stackful task's result.
pub struct JoinHandle<'scope, T> {
    slot: Arc<JoinSlot<T>>,
    task: TaskRef,
    joined: bool,
    _scope: PhantomData<&'scope ()>,
}

impl<T> JoinHandle<'_, T> {
    /// Waits for the task. A blocking point: returns `Cancelled` if the joining
    /// task is cancelled first.
    pub fn join(mut self) -> Result<T, TaskError> {
        self.joined = true;
        self.slot.join_in_task()
    }

    /// Cancels this task (and the scopes it opened).
    pub fn cancel(&self) {
        self.task.cancelled.store(true, SeqCst);
        self.task.unpark();
    }

    pub fn is_finished(&self) -> bool {
        self.slot.is_done()
    }

    pub fn task_id(&self) -> u64 {
        self.task.id
    }
}

impl<T> Drop for JoinHandle<'_, T> {
    fn drop(&mut self) {
        if !self.joined {
            self.slot.handle_dropped();
        }
    }
}

/// A scope for stackless (E) tasks. Children must be `'static`; call
/// [`AsyncScope::join_all`] before dropping it (dropping it cancels the children
/// but cannot wait for them).
pub struct AsyncScope {
    inner: Arc<ScopeInner>,
    shared: *const Shared,
    joined: bool,
}

// SAFETY: as for `Scope`.
unsafe impl Send for AsyncScope {}
// SAFETY: as for `Scope`.
unsafe impl Sync for AsyncScope {}

impl AsyncScope {
    /// Opens a scope under the current (stackless) task's scope.
    pub fn new() -> AsyncScope {
        let me = api::current_task().expect("AsyncScope::new must be called inside a task");
        AsyncScope { inner: ScopeInner::child_of(&me.scope, None), shared: me.rt, joined: false }
    }

    /// Spawns a stackless task.
    pub fn spawn<Fut, T>(&self, fut: Fut) -> AsyncJoinHandle<T>
    where
        Fut: std::future::Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        // SAFETY: the runtime outlives its tasks and scopes.
        let (task, slot) = unsafe { (*self.shared).spawn_stackless(&self.inner, fut) };
        AsyncJoinHandle { slot, task, joined: false }
    }

    pub fn cancel(&self) {
        self.inner.cancel();
    }

    pub fn active(&self) -> usize {
        self.inner.active()
    }

    /// Waits for every child; returns the first failure nobody joined.
    pub async fn join_all(mut self) -> Result<(), TaskError> {
        self.joined = true;
        let inner = self.inner.clone();
        std::future::poll_fn(|cx| inner.poll_children(cx)).await;
        match inner.take_failure() {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }
}

impl Default for AsyncScope {
    fn default() -> Self {
        AsyncScope::new()
    }
}

impl Drop for AsyncScope {
    fn drop(&mut self) {
        if !self.joined && self.inner.active() > 0 {
            self.inner.cancel();
        }
    }
}

/// A handle to a stackless task's result.
pub struct AsyncJoinHandle<T> {
    slot: Arc<JoinSlot<T>>,
    task: TaskRef,
    joined: bool,
}

impl<T> AsyncJoinHandle<T> {
    /// Waits for the task; a blocking point, so cancellation of the joiner wins.
    pub async fn join(mut self) -> Result<T, TaskError> {
        self.joined = true;
        let slot = self.slot.clone();
        std::future::poll_fn(move |cx| {
            if let Some(r) = slot.take_or_register(cx.waker()) {
                return Poll::Ready(r);
            }
            if api::is_cancelled() {
                slot.handle_dropped();
                return Poll::Ready(Err(TaskError::Cancelled));
            }
            Poll::Pending
        })
        .await
    }

    pub fn cancel(&self) {
        self.task.cancelled.store(true, SeqCst);
        self.task.unpark();
    }

    pub fn is_finished(&self) -> bool {
        self.slot.is_done()
    }
}

impl<T> Drop for AsyncJoinHandle<T> {
    fn drop(&mut self) {
        if !self.joined {
            self.slot.handle_dropped();
        }
    }
}
