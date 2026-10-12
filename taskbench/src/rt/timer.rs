//! A timer thread: a binary heap of deadlines and the wakers to call.
//!
//! The thread asks for 1 ns timer slack so that wake-ups are as precise as the
//! kernel allows; the starvation benchmark reports the idle-system lateness of a
//! sleeping task as its baseline, so timer precision is measured, not assumed.

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::atomic::{AtomicBool, Ordering::SeqCst};
use std::sync::{Condvar, Mutex, OnceLock};
use std::task::Waker;
use std::thread;
use std::time::Instant;

struct Entry {
    deadline: Instant,
    seq: u64,
    waker: Waker,
}

impl PartialEq for Entry {
    fn eq(&self, o: &Self) -> bool {
        self.deadline == o.deadline && self.seq == o.seq
    }
}
impl Eq for Entry {}
impl PartialOrd for Entry {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Entry {
    fn cmp(&self, o: &Self) -> Ordering {
        // Reversed: BinaryHeap is a max-heap and we want the earliest deadline.
        o.deadline.cmp(&self.deadline).then(o.seq.cmp(&self.seq))
    }
}

struct Inner {
    heap: BinaryHeap<Entry>,
    seq: u64,
}

/// The runtime's timer.
pub(crate) struct Timer {
    inner: Mutex<Inner>,
    cv: Condvar,
    thread: OnceLock<thread::JoinHandle<()>>,
    shutdown: AtomicBool,
}

struct TimerPtr(*const Timer);
// SAFETY: the Timer lives inside the runtime's shared state, which outlives the
// timer thread (joined in `shutdown`).
unsafe impl Send for TimerPtr {}

impl Timer {
    pub(crate) fn new() -> Timer {
        Timer {
            inner: Mutex::new(Inner { heap: BinaryHeap::new(), seq: 0 }),
            cv: Condvar::new(),
            thread: OnceLock::new(),
            shutdown: AtomicBool::new(false),
        }
    }

    /// Calls `waker` at or after `deadline`.
    pub(crate) fn register(&self, deadline: Instant, waker: Waker) {
        self.thread.get_or_init(|| {
            let p = TimerPtr(self);
            thread::Builder::new()
                .name("taskbench-timer".into())
                .spawn(move || {
                    let p = p;
                    // SAFETY: see TimerPtr.
                    unsafe { (*p.0).run() }
                })
                .expect("spawn timer thread")
        });
        let mut g = self.inner.lock().expect("timer lock");
        let seq = g.seq;
        g.seq += 1;
        let earliest = g.heap.peek().map(|e| deadline < e.deadline).unwrap_or(true);
        g.heap.push(Entry { deadline, seq, waker });
        drop(g);
        if earliest {
            self.cv.notify_one();
        }
    }

    fn run(&self) {
        // SAFETY: prctl(PR_SET_TIMERSLACK) only changes this thread's timer slack.
        unsafe {
            libc::prctl(libc::PR_SET_TIMERSLACK, 1 as libc::c_ulong, 0, 0, 0);
        }
        let mut due: Vec<Waker> = Vec::new();
        let mut g = self.inner.lock().expect("timer lock");
        loop {
            if self.shutdown.load(SeqCst) {
                return;
            }
            let now = Instant::now();
            while let Some(top) = g.heap.peek() {
                if top.deadline <= now {
                    due.push(g.heap.pop().expect("peeked").waker);
                } else {
                    break;
                }
            }
            if !due.is_empty() {
                drop(g);
                for w in due.drain(..) {
                    w.wake();
                }
                g = self.inner.lock().expect("timer lock");
                continue;
            }
            g = match g.heap.peek() {
                Some(top) => {
                    let wait = top.deadline.saturating_duration_since(now);
                    self.cv.wait_timeout(g, wait).expect("timer lock").0
                }
                None => self.cv.wait(g).expect("timer lock"),
            };
        }
    }

    /// Stops and joins the timer thread, dropping pending entries.
    pub(crate) fn shutdown(&self) {
        self.shutdown.store(true, SeqCst);
        {
            let _g = self.inner.lock().expect("timer lock");
            self.cv.notify_all();
        }
    }

    pub(crate) fn join(&self) {
        // OnceLock cannot give the JoinHandle back by value; the thread exits on
        // its own after `shutdown`, so wait for the heap lock to show it is gone.
        if self.thread.get().is_some() {
            let deadline = Instant::now() + std::time::Duration::from_secs(1);
            while Instant::now() < deadline {
                if self.thread.get().map(|h| h.is_finished()).unwrap_or(true) {
                    break;
                }
                thread::sleep(std::time::Duration::from_millis(1));
            }
        }
        let mut g = self.inner.lock().expect("timer lock");
        g.heap.clear();
    }
}
