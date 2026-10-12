//! A phase gate: tasks wait until the gate opens past the phase they saw.
//! `idle-memory` and `max-tasks` park their tasks on it, and `waiting()` tells the
//! benchmark when all of them are parked.

use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering::SeqCst};
use std::task::{Poll, Waker};

use super::api;
use super::task::TaskError;

/// Many-waiter gate with numbered phases.
pub struct Gate {
    phase: AtomicU64,
    waiters: Mutex<Vec<Waker>>,
    waiting: AtomicUsize,
}

impl Default for Gate {
    fn default() -> Self {
        Gate::new()
    }
}

impl Gate {
    pub fn new() -> Gate {
        Gate { phase: AtomicU64::new(0), waiters: Mutex::new(Vec::new()), waiting: AtomicUsize::new(0) }
    }

    /// The current phase.
    pub fn phase(&self) -> u64 {
        self.phase.load(SeqCst)
    }

    /// Tasks currently waiting.
    pub fn waiting(&self) -> usize {
        self.waiting.load(SeqCst)
    }

    fn register(&self, seen: u64, w: Waker) -> bool {
        let mut ws = self.waiters.lock().expect("gate lock");
        if self.phase.load(SeqCst) > seen {
            return false;
        }
        ws.push(w);
        self.waiting.fetch_add(1, SeqCst);
        true
    }

    /// Blocks the current stackful task until the phase exceeds `seen`.
    pub fn wait(&self, seen: u64) -> Result<(), TaskError> {
        let mut registered = false;
        loop {
            if self.phase.load(SeqCst) > seen {
                return Ok(());
            }
            api::check_cancelled()?;
            if !registered {
                if !self.register(seen, api::current_waker()) {
                    return Ok(());
                }
                registered = true;
            }
            api::park();
        }
    }

    /// As [`Gate::wait`] but not a cancellation point (Sisu's `task.shield`).
    pub fn wait_shielded(&self, seen: u64) {
        let mut registered = false;
        while self.phase.load(SeqCst) <= seen {
            if !registered {
                if !self.register(seen, api::current_waker()) {
                    return;
                }
                registered = true;
            }
            api::park();
        }
    }

    /// Stackless version of [`Gate::wait`].
    pub async fn wait_async(&self, seen: u64) -> Result<(), TaskError> {
        let mut registered = false;
        std::future::poll_fn(|cx| {
            if self.phase.load(SeqCst) > seen {
                return Poll::Ready(Ok(()));
            }
            if api::is_cancelled() {
                return Poll::Ready(Err(TaskError::Cancelled));
            }
            if !registered {
                if !self.register(seen, cx.waker().clone()) {
                    return Poll::Ready(Ok(()));
                }
                registered = true;
            }
            Poll::Pending
        })
        .await
    }

    /// Advances the phase and wakes every waiter. Returns the new phase.
    pub fn open(&self) -> u64 {
        let ws = {
            let mut ws = self.waiters.lock().expect("gate lock");
            self.phase.fetch_add(1, SeqCst);
            self.waiting.store(0, SeqCst);
            std::mem::take(&mut *ws)
        };
        for w in ws {
            w.wake();
        }
        self.phase()
    }
}
