//! Timers, fired by the workers themselves (as Go fires timers on its Ps).
//!
//! A binary heap of deadlines and wakers, plus the earliest deadline in an
//! atomic so that a worker can check "is anything due?" with one load and one
//! clock read at every scheduling decision. Due timers are fired by whichever
//! worker notices; the tasks they wake take that worker's `next` slot. When the
//! workers sleep, one of them, the timer owner, sleeps only until the earliest
//! deadline; registering an earlier deadline nudges it. An earlier design used a
//! dedicated timer thread, which on a machine whose CPUs are all busy with
//! workers waits for the kernel to schedule it: that alone put the starvation
//! probe's p99 at about 0.5 ms.

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering::SeqCst};
use std::task::Waker;
use std::time::{Duration, Instant};

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

const NONE: u64 = u64::MAX;

/// The runtime's timer heap.
pub(crate) struct Timer {
    inner: Mutex<Inner>,
    /// Earliest deadline as nanoseconds since `epoch`; `NONE` when empty.
    earliest: AtomicU64,
    epoch: Instant,
}

impl Timer {
    pub(crate) fn new() -> Timer {
        Timer {
            inner: Mutex::new(Inner { heap: BinaryHeap::new(), seq: 0 }),
            earliest: AtomicU64::new(NONE),
            epoch: Instant::now(),
        }
    }

    fn encode(&self, t: Instant) -> u64 {
        t.saturating_duration_since(self.epoch).as_nanos().min((NONE - 1) as u128) as u64
    }

    fn publish(&self, g: &Inner) {
        let v = g.heap.peek().map(|e| self.encode(e.deadline)).unwrap_or(NONE);
        self.earliest.store(v, SeqCst);
    }

    /// Calls `waker` at or after `deadline`. Returns true when this became the
    /// earliest deadline (the caller then nudges the timer owner).
    pub(crate) fn register(&self, deadline: Instant, waker: Waker) -> bool {
        let mut g = self.inner.lock().expect("timer lock");
        let seq = g.seq;
        g.seq += 1;
        let earliest = g.heap.peek().map(|e| deadline < e.deadline).unwrap_or(true);
        g.heap.push(Entry { deadline, seq, waker });
        self.publish(&g);
        earliest
    }

    /// The earliest deadline, if any.
    pub(crate) fn next_deadline(&self) -> Option<Instant> {
        match self.earliest.load(SeqCst) {
            NONE => None,
            ns => Some(self.epoch + Duration::from_nanos(ns)),
        }
    }

    /// Whether a timer is due (one atomic load and one clock read).
    pub(crate) fn is_due(&self) -> bool {
        let e = self.earliest.load(SeqCst);
        e != NONE && self.encode(Instant::now()) >= e
    }

    /// Fires every due timer; returns how many. Skips (returns 0) if another
    /// thread is firing at the same moment.
    pub(crate) fn fire_due(&self) -> usize {
        if !self.is_due() {
            return 0;
        }
        let Ok(mut g) = self.inner.try_lock() else { return 0 };
        let now = Instant::now();
        let mut due = Vec::new();
        while g.heap.peek().is_some_and(|top| top.deadline <= now) {
            due.push(g.heap.pop().expect("peeked").waker);
        }
        self.publish(&g);
        drop(g);
        let n = due.len();
        for w in due {
            w.wake();
        }
        n
    }

    /// Drops pending entries (at shutdown).
    pub(crate) fn clear(&self) {
        let mut g = self.inner.lock().expect("timer lock");
        g.heap.clear();
        self.publish(&g);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::AtomicUsize;
    use std::task::Wake;

    struct Count(AtomicUsize);
    impl Wake for Count {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, SeqCst);
        }
    }

    #[test]
    fn fires_in_order_and_publishes_earliest() {
        let t = Timer::new();
        let c = Arc::new(Count(AtomicUsize::new(0)));
        let now = Instant::now();
        assert!(t.register(now + Duration::from_secs(60), Waker::from(c.clone())));
        assert!(t.register(now, Waker::from(c.clone())), "earlier deadline becomes earliest");
        assert!(!t.register(now + Duration::from_secs(30), Waker::from(c.clone())));
        assert!(t.is_due());
        assert_eq!(t.fire_due(), 1);
        assert_eq!(c.0.load(SeqCst), 1);
        assert!(!t.is_due());
        let next = t.next_deadline().unwrap();
        assert!(next > now + Duration::from_secs(29) && next < now + Duration::from_secs(31));
        t.clear();
        assert!(t.next_deadline().is_none());
    }
}
