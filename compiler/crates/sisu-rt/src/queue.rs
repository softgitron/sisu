//! The scheduler's queues (decision 645): the run queue, the waiters and
//! the timers are the runtime in Sisu's (`compiler/rt/src/queue.sisu`),
//! records of one to three words in the order they came; the scheduler
//! keeps here only the address of each and calls the exports, which have
//! Sisu's convention (the result's address, each argument's, the status).
//! The blocks are given here, kept for the run and not counted among the
//! program's allocations, as the `Vec`s and the map they replace were not.
//! The buffers of channels are the runtime in Sisu's too
//! (`compiler/rt/src/channel.sisu`, decision 680), kept the same way.

unsafe extern "C" {
    fn sisu_rt_queue_init(
        ret: *mut u64,
        q: *const *mut u8,
        buffer: *const *mut u8,
        capacity: *const i64,
        width: *const i64,
    ) -> u8;
    fn sisu_rt_queue_len(ret: *mut i64, q: *const *mut u8) -> u8;
    fn sisu_rt_queue_push(
        ret: *mut u64,
        q: *const *mut u8,
        a: *const i64,
        b: *const i64,
        c: *const i64,
    ) -> u8;
    fn sisu_rt_queue_word(ret: *mut i64, q: *const *mut u8, i: *const i64, j: *const i64) -> u8;
    fn sisu_rt_queue_swap_remove(ret: *mut i64, q: *const *mut u8, i: *const i64) -> u8;
    fn sisu_rt_queue_remove(ret: *mut u64, q: *const *mut u8, i: *const i64) -> u8;
    fn sisu_rt_queue_position(ret: *mut i64, q: *const *mut u8, j: *const i64, x: *const i64) -> u8;
    fn sisu_rt_queue_clear(ret: *mut u64, q: *const *mut u8) -> u8;
    fn sisu_rt_queue_take_key(ret: *mut u64, q: *const *mut u8, key: *const i64, out: *const *mut u8) -> u8;
    fn sisu_rt_channel_init(
        ret: *mut u64,
        c: *const *mut u8,
        slots: *const *mut u8,
        room: *const i64,
        size: *const i64,
    ) -> u8;
    fn sisu_rt_channel_len(ret: *mut i64, c: *const *mut u8) -> u8;
    fn sisu_rt_channel_push(ret: *mut u64, c: *const *mut u8, src: *const *const u8, size: *const i64) -> u8;
    fn sisu_rt_channel_pop(ret: *mut u64, c: *const *mut u8, dst: *const *mut u8, size: *const i64) -> u8;
}

/// The records a queue has room for before it first grows.
const CAPACITY: i64 = 16;

/// A queue of the scheduler: the address of its header.
pub(crate) struct Queue(*mut u8);

/// Runs a call of the runtime in Sisu with the running task's stack probe
/// lifted, as [`crate::sched::system`] does: the scheduler calls these
/// from the root task's stack while another task's limits are loaded, and
/// from a task's stack while it switches, where the limits do not describe
/// the stack the call runs on; the calls are shallow and nest nothing.
fn unprobed(f: impl FnOnce() -> u8) {
    let probe = &crate::tls::stack()[(crate::abi::STACK_PROBE / 8) as usize];
    let limit = probe.swap(0, std::sync::atomic::Ordering::Relaxed);
    let status = f();
    probe.store(limit, std::sync::atomic::Ordering::Relaxed);
    if status != 0 {
        crate::rt::internal("the runtime in Sisu failed in a queue of the scheduler or a channel's buffer");
    }
}

impl Queue {
    /// An empty queue of records of `width` words (1 to 3).
    pub(crate) fn new(width: i64) -> Queue {
        let q = crate::mem::alloc_kept(32);
        let buffer = crate::mem::alloc_kept((CAPACITY * width * 8) as usize);
        if q.is_null() || buffer.is_null() {
            crate::rt::abort("out of memory");
        }
        let mut unit = 0u64;
        // SAFETY: `q` is four words and `buffer` room for `CAPACITY`
        // records, both from malloc, as the export requires.
        unprobed(|| unsafe { sisu_rt_queue_init(&mut unit, &q, &buffer, &CAPACITY, &width) });
        Queue(q)
    }

    pub(crate) fn len(&self) -> usize {
        let mut n = 0i64;
        // SAFETY: `self.0` is a live queue.
        unprobed(|| unsafe { sisu_rt_queue_len(&mut n, &self.0) });
        n as usize
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Appends a record of `a`, `b` and `c`, as many as its width.
    pub(crate) fn push(&mut self, a: i64, b: i64, c: i64) {
        let mut unit = 0u64;
        // SAFETY: `self.0` is a live queue.
        unprobed(|| unsafe { sisu_rt_queue_push(&mut unit, &self.0, &a, &b, &c) });
    }

    /// The word `j` of the record `i`, which the caller has seen is one.
    pub(crate) fn word(&self, i: usize, j: usize) -> i64 {
        let mut w = 0i64;
        // SAFETY: `self.0` is a live queue, `i` one of its records and `j`
        // within the width the scheduler gave it.
        unprobed(|| unsafe { sisu_rt_queue_word(&mut w, &self.0, &(i as i64), &(j as i64)) });
        w
    }

    /// The first words of the records, in order.
    pub(crate) fn firsts(&self) -> Vec<i64> {
        (0..self.len()).map(|i| self.word(i, 0)).collect()
    }

    /// Takes out the record `i`, the last moving into its slot: its first
    /// word.
    pub(crate) fn swap_remove(&mut self, i: usize) -> i64 {
        if i >= self.len() {
            crate::rt::internal("a record outside a queue of the scheduler");
        }
        let mut w = 0i64;
        // SAFETY: `self.0` is a live queue and `i` one of its records.
        unprobed(|| unsafe { sisu_rt_queue_swap_remove(&mut w, &self.0, &(i as i64)) });
        w
    }

    /// Takes out the record `i`, keeping the others' order.
    pub(crate) fn remove(&mut self, i: usize) {
        if i >= self.len() {
            crate::rt::internal("a record outside a queue of the scheduler");
        }
        let mut unit = 0u64;
        // SAFETY: `self.0` is a live queue and `i` one of its records.
        unprobed(|| unsafe { sisu_rt_queue_remove(&mut unit, &self.0, &(i as i64)) });
    }

    /// The first record whose word `j` is `x`.
    pub(crate) fn position(&self, j: usize, x: i64) -> Option<usize> {
        let mut i = 0i64;
        // SAFETY: `self.0` is a live queue and `j` within its width.
        unprobed(|| unsafe { sisu_rt_queue_position(&mut i, &self.0, &(j as i64), &x) });
        usize::try_from(i).ok()
    }

    /// Takes out every record whose first word is `key`, keeping the
    /// others' order, and puts their second words in `out`, emptied first,
    /// in order.
    pub(crate) fn take_key(&mut self, key: i64, out: &mut Queue) {
        let mut unit = 0u64;
        // SAFETY: both are live queues, `self.0` of two words a record.
        unprobed(|| unsafe { sisu_rt_queue_clear(&mut unit, &out.0) });
        unprobed(|| unsafe { sisu_rt_queue_take_key(&mut unit, &self.0, &key, &out.0) });
    }
}

/// The buffer of a channel (decision 680): the values sent and not yet
/// received, in the order they were sent, the runtime in Sisu's
/// (`compiler/rt/src/channel.sisu`), a ring of slots of the value's size
/// that grows when it is full, whose blocks are given here and kept for the
/// run, not counted among the program's allocations, as the `VecDeque` it
/// replaces was not. The channel's capacity is a bound the scheduler checks.
pub(crate) struct ChanBuf {
    at: *mut u8,
    size: i64,
}

/// The slots a channel's buffer has before it first grows, fewer when the
/// channel's capacity is smaller.
const CHAN_ROOM: usize = 8;

impl ChanBuf {
    /// An empty buffer of values of `size` bytes, for a channel of
    /// `capacity`.
    pub(crate) fn new(size: u32, capacity: usize) -> ChanBuf {
        let room = capacity.clamp(1, CHAN_ROOM) as i64;
        let slot = (i64::from(size).max(1) + 7) / 8 * 8;
        let at = crate::mem::alloc_kept(40);
        let slots = crate::mem::alloc_kept((room * slot) as usize);
        if at.is_null() || slots.is_null() {
            crate::rt::abort("out of memory");
        }
        let size = i64::from(size);
        let mut unit = 0u64;
        // SAFETY: `at` is five words and `slots` room for `room` slots, both
        // from malloc, as the export requires.
        unprobed(|| unsafe { sisu_rt_channel_init(&mut unit, &at, &slots, &room, &size) });
        ChanBuf { at, size }
    }

    pub(crate) fn len(&self) -> usize {
        let mut n = 0i64;
        // SAFETY: `self.at` is a live buffer.
        unprobed(|| unsafe { sisu_rt_channel_len(&mut n, &self.at) });
        n as usize
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Moves the value at `src` in after the others.
    ///
    /// # Safety
    /// `src` must hold a value of the buffer's size, the buffer's after.
    pub(crate) unsafe fn push(&mut self, src: *const u8) {
        let mut unit = 0u64;
        // SAFETY: `self.at` is a live buffer and `src` a value of its size.
        unprobed(|| unsafe { sisu_rt_channel_push(&mut unit, &self.at, &src, &self.size) });
    }

    /// Moves the first value out to `dst`: whether there was one.
    ///
    /// # Safety
    /// `dst` must have room for a value of the buffer's size.
    pub(crate) unsafe fn pop(&mut self, dst: *mut u8) -> bool {
        let mut got = 0u64;
        // SAFETY: `self.at` is a live buffer and `dst` has room for a value.
        unprobed(|| unsafe { sisu_rt_channel_pop(&mut got, &self.at, &dst, &self.size) });
        got & 0xff != 0
    }
}
