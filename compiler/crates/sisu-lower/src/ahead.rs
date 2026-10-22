//! Bodies lowered ahead of the worklist, on threads (decision 228): when
//! a kept function names a function whose body is not lowered yet, the
//! body is handed to the threads, and the worklist takes the functions
//! when it comes to them, in its own order. Lowering a body reads only
//! the program and the context, so the functions are the ones the
//! worklist would have made itself, and the program is the same, byte
//! for byte, however the threads ran.

use crate::Ctx;
use sisu_sir as sir;
use sisu_typeck::Body;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};

/// A body's functions, and the memo's copy of them ([`crate::lower_kept`]).
type Lowered = (Vec<sir::Function>, Option<Vec<sir::Function>>);
type Done = std::thread::Result<Lowered>;

#[derive(Default)]
struct State {
    /// The bodies asked for and not yet taken by a thread: the last asked
    /// first, as the worklist takes the last name it was given.
    queue: VecDeque<usize>,
    /// What the threads lowered, by body, until the worklist takes it.
    done: HashMap<usize, Done>,
    /// No more bodies will be asked for: the threads stop.
    closed: bool,
}

/// The bodies handed to the threads, and what they made of them.
pub(crate) struct Ahead<'a> {
    ctx: &'a Ctx<'a>,
    bodies: &'a [Arc<Body>],
    /// Lowering keeps a memo, which takes a copy of a body's functions:
    /// made by the thread that lowers it.
    memoizing: bool,
    state: Mutex<State>,
    /// Signalled when a body is asked for, when one is lowered, and when
    /// the worklist is done.
    changed: Condvar,
}

impl<'a> Ahead<'a> {
    pub(crate) fn new(ctx: &'a Ctx<'a>, bodies: &'a [Arc<Body>], memoizing: bool) -> Ahead<'a> {
        Ahead { ctx, bodies, memoizing, state: Mutex::new(State::default()), changed: Condvar::new() }
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        // A thread that panicked holding the lock left the state whole
        // (it panics only while lowering, outside the lock).
        self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn lower(&self, b: usize) -> Done {
        match self.bodies.get(b) {
            Some(body) => std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                crate::lower_kept(self.ctx, body, self.memoizing)
            })),
            None => Ok((Vec::new(), None)),
        }
    }

    /// Hands the body `b` to the threads.
    pub(crate) fn ask(&self, b: usize) {
        self.lock().queue.push_back(b);
        self.changed.notify_one();
    }

    /// The functions of the body `b`, which was asked for, and the memo's
    /// copy: lowered by a thread, or here when no thread has taken it yet.
    /// While a thread lowers it, this one lowers the others asked for. A
    /// panic in its lowering is raised again here, as one on this thread
    /// would be.
    pub(crate) fn take(&self, b: usize) -> Lowered {
        let mut st = self.lock();
        let done = loop {
            if let Some(d) = st.done.remove(&b) {
                break d;
            }
            let next = match st.queue.iter().position(|&x| x == b) {
                Some(at) => st.queue.remove(at),
                None => st.queue.pop_back(),
            };
            match next {
                Some(x) => {
                    drop(st);
                    let d = self.lower(x);
                    if x == b {
                        break d;
                    }
                    st = self.lock();
                    st.done.insert(x, d);
                }
                None => st = self.changed.wait(st).unwrap_or_else(std::sync::PoisonError::into_inner),
            }
        };
        done.unwrap_or_else(|e| std::panic::resume_unwind(e))
    }

    /// What each thread does until the worklist is done: lowers the body
    /// asked for last.
    pub(crate) fn work(&self) {
        let mut st = self.lock();
        loop {
            if let Some(b) = st.queue.pop_back() {
                drop(st);
                let d = self.lower(b);
                st = self.lock();
                st.done.insert(b, d);
                self.changed.notify_all();
                continue;
            }
            if st.closed {
                return;
            }
            st = self.changed.wait(st).unwrap_or_else(std::sync::PoisonError::into_inner);
        }
    }

    /// The worklist is done: the threads stop once idle.
    pub(crate) fn close(&self) {
        let mut st = self.lock();
        st.closed = true;
        st.queue.clear();
        drop(st);
        self.changed.notify_all();
    }
}

