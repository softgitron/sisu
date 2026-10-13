//! The runtime the harness measures: an M:N work-stealing scheduler running
//! stackful tasks on the stacks of a [`crate::stack::StackStrategy`], or
//! stackless futures for strategy E, with structured scopes, cancellation,
//! parking, a timer and an epoll poller.
//!
//! ```no_run
//! use taskbench::rt::{self, Runtime};
//! use taskbench::stack::{StrategyConfig, StrategyId};
//!
//! let rt = Runtime::new(4, &StrategyConfig::new(StrategyId::B)).unwrap();
//! let total = rt.block_on(|| {
//!     let data = vec![1, 2, 3];
//!     rt::scope(|s| {
//!         let hs: Vec<_> = data.iter().map(|x| s.spawn(move || x * 10)).collect();
//!         hs.into_iter().map(|h| h.join().unwrap()).sum::<i32>()
//!     })
//!     .unwrap()
//! });
//! assert_eq!(total, 60);
//! ```
//!
//! # Lifetime of the shared state
//!
//! Tasks hold a raw pointer to the runtime's shared state and their stacks a raw
//! pointer to its strategy (an `Arc` per task would put two contended atomic
//! operations on every spawn). Dropping the [`Runtime`] stops the workers and frees
//! the shared state only when no task is alive; if tasks leaked (a bug in a task
//! body), it reports them and leaks the runtime instead of freeing memory they use.

pub mod api;
pub mod gate;
pub mod net;
pub(crate) mod panic_hook;
pub mod scope;
pub(crate) mod slab;
pub mod task;
pub(crate) mod timer;
pub mod uring;
pub(crate) mod worker;

#[cfg(test)]
mod tests;

use std::future::Future;
use std::sync::atomic::Ordering::SeqCst;
use std::thread;

pub use api::{
    check_cancelled, current_waker, in_task, is_cancelled, park, probe, sleep, sleep_async, stack_region, task_id,
    yield_now, yield_now_async,
};
pub use gate::Gate;
pub use scope::{AsyncJoinHandle, AsyncScope, JoinHandle, Scope, scope};
pub use task::{TaskError, TaskExit};
pub use worker::RtStats;

use crate::stack::{StackCounters, StackKind, StrategyConfig};
use scope::ScopeInner;
use worker::Shared;

/// Default stack size of the worker OS threads (the Linux default for threads).
pub const DEFAULT_WORKER_STACK: usize = 8 * 1024 * 1024;

/// A running scheduler.
pub struct Runtime {
    shared: *const Shared,
    threads: Vec<thread::JoinHandle<()>>,
    label: String,
}

// SAFETY: `Shared` is Sync; the Runtime handle only reads it and joins threads.
unsafe impl Send for Runtime {}
// SAFETY: as above.
unsafe impl Sync for Runtime {}

impl Runtime {
    /// Starts `threads` workers over the strategy `cfg` describes.
    pub fn new(threads: usize, cfg: &StrategyConfig) -> Result<Runtime, String> {
        Runtime::with_worker_stack(threads, cfg, DEFAULT_WORKER_STACK)
    }

    /// As [`Runtime::new`], choosing the worker threads' own stack size (which
    /// bounds the `poll` recursion of stackless tasks).
    pub fn with_worker_stack(threads: usize, cfg: &StrategyConfig, worker_stack: usize) -> Result<Runtime, String> {
        // Before any task can exhaust its stack: the unwinder's first use must not
        // run the dynamic linker inside a task's probe reserve.
        crate::stack::probe::warm_unwinder();
        let strategy = crate::stack::build(cfg)?;
        let label = strategy.label();
        let (shared, threads) =
            Shared::start(threads, strategy, worker_stack).map_err(|e| format!("cannot start workers: {e}"))?;
        Ok(Runtime { shared, threads, label })
    }

    fn shared(&self) -> &Shared {
        // SAFETY: freed only in `drop`.
        unsafe { &*self.shared }
    }

    /// The strategy label (`A`, `B`, `C`, `D(A)`, `D(B)`, `E`).
    pub fn strategy_label(&self) -> &str {
        &self.label
    }

    pub fn is_stackless(&self) -> bool {
        self.shared().strategy.kind() == StackKind::Stackless
    }

    pub fn threads(&self) -> usize {
        self.shared().worker_count()
    }

    /// Strategy counters (allocations, reuse, decommits, canary failures).
    pub fn stack_counters(&self) -> StackCounters {
        self.shared().strategy.counters()
    }

    /// Scheduler counters.
    pub fn stats(&self) -> RtStats {
        self.shared().stats()
    }

    /// Tasks not yet finished.
    pub fn live_tasks(&self) -> usize {
        self.shared().live_tasks.load(SeqCst)
    }

    /// Runs `f` as the root stackful task and blocks the calling thread until it
    /// and everything it spawned have finished. Panics if the root task fails.
    pub fn block_on<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R + Send,
        R: Send,
    {
        match self.try_block_on(f) {
            Ok(v) => v,
            Err(e) => panic!("root task failed: {e}"),
        }
    }

    /// As [`Runtime::block_on`], returning the root task's failure.
    pub fn try_block_on<F, R>(&self, f: F) -> Result<R, TaskError>
    where
        F: FnOnce() -> R + Send,
        R: Send,
    {
        assert!(!worker::on_worker_thread(), "block_on called from a worker thread");
        let root = ScopeInner::root();
        // SAFETY: `f` may borrow from our caller's frame; this function does not
        // return until the root scope's active count is zero, i.e. until the task
        // has finished and its stack has been released.
        let (task, slot) = unsafe { self.shared().spawn_stackful(&root, f) }.map_err(TaskError::SpawnFailed)?;
        let r = slot.wait_blocking();
        root.wait_children_blocking();
        drop(task);
        r
    }

    /// Runs a future as the root stackless task (strategy E).
    pub fn block_on_async<Fut, R>(&self, fut: Fut) -> R
    where
        Fut: Future<Output = R> + Send + 'static,
        R: Send + 'static,
    {
        assert!(!worker::on_worker_thread(), "block_on_async called from a worker thread");
        let root = ScopeInner::root();
        let (task, slot) = self.shared().spawn_stackless(&root, fut);
        let r = slot.wait_blocking();
        root.wait_children_blocking();
        drop(task);
        match r {
            Ok(v) => v,
            Err(e) => panic!("root task failed: {e}"),
        }
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        // SAFETY: the shared state is freed at the end of this function at the earliest.
        let sh: &Shared = unsafe { &*self.shared };
        sh.begin_shutdown();
        for h in self.threads.drain(..) {
            let _ = h.join();
        }
        sh.timer.clear();
        let live = sh.live_tasks.load(SeqCst);
        if live == 0 {
            // Drain anything still queued (nothing should be) before freeing.
            // SAFETY: all workers and helper threads have stopped and no task is
            // alive, so nothing else references the shared state.
            drop(unsafe { Box::from_raw(self.shared as *mut Shared) });
        } else {
            eprintln!("taskbench: {live} tasks still alive at runtime shutdown; leaking the runtime");
        }
    }
}
