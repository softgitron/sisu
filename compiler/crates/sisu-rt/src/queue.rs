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
    fn sisu_rt_queue_remove(ret: *mut u64, q: *const *mut u8, i: *const i64) -> u8;
    fn sisu_rt_queue_position(ret: *mut i64, q: *const *mut u8, j: *const i64, x: *const i64) -> u8;
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
    fn sisu_rt_draw_seeded(ret: *mut i64, seed: *const i64) -> u8;
    fn sisu_rt_draw_next(ret: *mut i64, state: *const *mut i64) -> u8;
    fn sisu_rt_draw_below(ret: *mut i64, state: *const *mut i64, n: *const i64) -> u8;
    fn sisu_rt_draw_pick(ret: *mut i64, state: *const *mut i64, q: *const *mut u8) -> u8;
    fn sisu_rt_choose_pct_start(
        ret: *mut u64,
        state: *const *mut i64,
        st: *const *mut u8,
        depth: *const i64,
    ) -> u8;
    fn sisu_rt_choose_pct_yield(
        ret: *mut i64,
        state: *const *mut i64,
        st: *const *mut u8,
        prios: *const *mut u8,
        cur: *const i64,
        asked: *const u64,
        ready: *const *mut u8,
    ) -> u8;
    fn sisu_rt_choose_pct_pick(
        ret: *mut i64,
        state: *const *mut i64,
        prios: *const *mut u8,
        ready: *const *mut u8,
    ) -> u8;
    fn sisu_rt_choose_next(
        ret: *mut i64,
        state: *const *mut i64,
        prefix: *const *mut u8,
        trail: *const *mut u8,
        searching: *const u64,
        options: *const i64,
        at_yield: *const u64,
    ) -> u8;
    fn sisu_rt_choose_search_yield(
        ret: *mut i64,
        state: *const *mut i64,
        prefix: *const *mut u8,
        trail: *const *mut u8,
        ready: *const *mut u8,
    ) -> u8;
    fn sisu_rt_choose_search_pick(
        ret: *mut i64,
        state: *const *mut i64,
        prefix: *const *mut u8,
        trail: *const *mut u8,
        ready: *const *mut u8,
        forced: *const i64,
    ) -> u8;
    fn sisu_rt_task_add(
        ret: *mut i64,
        recs: *const *mut u8,
        state: *const i64,
        wait: *const i64,
        flags: *const i64,
    ) -> u8;
    fn sisu_rt_task_state(ret: *mut i64, recs: *const *mut u8, t: *const i64) -> u8;
    fn sisu_rt_task_set(
        ret: *mut u64,
        recs: *const *mut u8,
        t: *const i64,
        state: *const i64,
        wait: *const i64,
    ) -> u8;
    fn sisu_rt_task_park(
        ret: *mut u64,
        recs: *const *mut u8,
        waiters: *const *mut u8,
        t: *const i64,
        wait: *const i64,
        keys: *const *const i64,
        n: *const i64,
    ) -> u8;
    fn sisu_rt_task_wake(ret: *mut i64, recs: *const *mut u8, ready: *const *mut u8, t: *const i64) -> u8;
    fn sisu_rt_task_wake_key(
        ret: *mut i64,
        recs: *const *mut u8,
        waiters: *const *mut u8,
        woken: *const *mut u8,
        ready: *const *mut u8,
        key: *const i64,
    ) -> u8;
    fn sisu_rt_task_wake_all(
        ret: *mut i64,
        recs: *const *mut u8,
        woken: *const *mut u8,
        ready: *const *mut u8,
    ) -> u8;
}

/// The seeded generator behind every scheduling choice and the faults of
/// `sim`: the interpreter's SplitMix64, with its seeding, so the same seed
/// draws the same numbers. Its state is the word kept here, which the
/// runtime in Sisu draws from (`compiler/rt/src/draw.sisu`, decision 724).
pub(crate) struct Rng(i64);

impl Rng {
    pub(crate) fn new(seed: u64) -> Rng {
        let mut state = 0i64;
        let seed = seed as i64;
        // SAFETY: the export reads the seed and writes the result's word.
        unprobed(|| unsafe { sisu_rt_draw_seeded(&mut state, &seed) });
        Rng(state)
    }

    pub(crate) fn next(&mut self) -> u64 {
        let mut drawn = 0i64;
        let state: *mut i64 = &mut self.0;
        // SAFETY: `state` is the generator's word, as the export requires.
        unprobed(|| unsafe { sisu_rt_draw_next(&mut drawn, &state) });
        drawn as u64
    }

    pub(crate) fn below(&mut self, n: usize) -> usize {
        let mut drawn = 0i64;
        let state: *mut i64 = &mut self.0;
        let n = n as i64;
        // SAFETY: as in `next`.
        unprobed(|| unsafe { sisu_rt_draw_below(&mut drawn, &state, &n) });
        drawn as usize
    }

    /// The next task, drawn among the ready ones in the run queue `q` and
    /// taken out of it, the last moving into its slot, as the interpreter's
    /// `switch` takes it; none when none is ready.
    pub(crate) fn pick(&mut self, q: &mut Queue) -> Option<i64> {
        let mut task = 0i64;
        let state: *mut i64 = &mut self.0;
        // SAFETY: `state` is the generator's word and `q.0` a live queue of
        // one word a record, the run queue's.
        unprobed(|| unsafe { sisu_rt_draw_pick(&mut task, &state, &q.0) });
        (task >= 0).then_some(task)
    }
}

/// The choices of PCT and of the exhaustive search (decision 501): the
/// runtime in Sisu's (`compiler/rt/src/choose.sisu`, decision 821), over
/// queues kept here and the generator's word, with the run queue handed to
/// each call, so that a depth and a seed, or a search's prefix, name the
/// schedule they named.
pub(crate) struct Choices {
    /// PCT's state, one word a record: the priority the next change point
    /// gives, the yield points passed, and the change points.
    pct: Queue,
    /// PCT's priorities by task, drawn when first asked.
    prios: Queue,
    /// The search's prefix, the choices it was told to make.
    prefix: Queue,
    /// The search's choices so far: taken, of how many, and whether at a
    /// yield point (1).
    trail: Queue,
}

impl Choices {
    /// No choice made yet, under a search told to begin with `prefix`.
    pub(crate) fn new(prefix: &[u32]) -> Choices {
        let mut q = Queue::new(1);
        for &c in prefix {
            q.push(i64::from(c), 0, 0);
        }
        Choices { pct: Queue::new(1), prios: Queue::new(1), prefix: q, trail: Queue::new(3) }
    }

    /// PCT's change points for a run of depth `depth`, drawn now.
    pub(crate) fn pct_start(&mut self, rng: &mut Rng, depth: u32) {
        let mut unit = 0u64;
        let (state, depth): (*mut i64, i64) = (&mut rng.0, i64::from(depth));
        // SAFETY: `state` is the generator's word and `self.pct` an empty
        // queue of one word a record.
        unprobed(|| unsafe { sisu_rt_choose_pct_start(&mut unit, &state, &self.pct.0, &depth) });
    }

    /// A yield point of the running task `cur` under PCT, `asked` when it
    /// gives way: whether it is preempted, a ready task now higher.
    pub(crate) fn pct_yield(&mut self, rng: &mut Rng, cur: u32, asked: bool, ready: &Queue) -> bool {
        let mut go = 0i64;
        let (state, cur, asked): (*mut i64, i64, u64) = (&mut rng.0, i64::from(cur), u64::from(asked));
        // SAFETY: `state` is the generator's word, the queues are PCT's
        // and the run queue.
        unprobed(|| unsafe {
            sisu_rt_choose_pct_yield(&mut go, &state, &self.pct.0, &self.prios.0, &cur, &asked, &ready.0)
        });
        go != 0
    }

    /// The next task under PCT, taken out of the run queue `ready`.
    pub(crate) fn pct_pick(&mut self, rng: &mut Rng, ready: &mut Queue) -> Option<u32> {
        let mut t = 0i64;
        let state: *mut i64 = &mut rng.0;
        // SAFETY: as in `pct_yield`.
        unprobed(|| unsafe { sisu_rt_choose_pct_pick(&mut t, &state, &self.prios.0, &ready.0) });
        u32::try_from(t).ok()
    }

    /// The next choice among `options`: the search's (`searching`), recorded,
    /// or a draw of the generator.
    pub(crate) fn choose(&mut self, rng: &mut Rng, searching: bool, options: u32, at_yield: bool) -> u32 {
        let mut c = 0i64;
        let state: *mut i64 = &mut rng.0;
        let (searching, options, at_yield) = (u64::from(searching), i64::from(options), u64::from(at_yield));
        // SAFETY: `state` is the generator's word and the queues the
        // search's.
        unprobed(|| unsafe {
            sisu_rt_choose_next(
                &mut c,
                &state,
                &self.prefix.0,
                &self.trail.0,
                &searching,
                &options,
                &at_yield,
            )
        });
        c as u32
    }

    /// A yield point under the search: the ready task it preempts for.
    pub(crate) fn search_yield(&mut self, rng: &mut Rng, ready: &Queue) -> Option<u32> {
        let mut t = 0i64;
        let state: *mut i64 = &mut rng.0;
        // SAFETY: as in `choose`, and `ready` is the run queue.
        unprobed(|| unsafe {
            sisu_rt_choose_search_yield(&mut t, &state, &self.prefix.0, &self.trail.0, &ready.0)
        });
        u32::try_from(t).ok()
    }

    /// The next task under the search, `forced` when a yield point chose
    /// it, taken out of the run queue `ready`.
    pub(crate) fn search_pick(
        &mut self,
        rng: &mut Rng,
        ready: &mut Queue,
        forced: Option<u32>,
    ) -> Option<u32> {
        let mut t = 0i64;
        let state: *mut i64 = &mut rng.0;
        let forced = forced.map_or(-1, i64::from);
        // SAFETY: as in `search_yield`.
        unprobed(|| unsafe {
            sisu_rt_choose_search_pick(&mut t, &state, &self.prefix.0, &self.trail.0, &ready.0, &forced)
        });
        u32::try_from(t).ok()
    }

    /// The search's choices so far, dotted as `taken/options/y` (`y` at a
    /// yield point, `n` elsewhere).
    pub(crate) fn trail_text(&self) -> String {
        let parts: Vec<String> = (0..self.trail.len())
            .map(|i| {
                let y = if self.trail.word(i, 2) != 0 { "y" } else { "n" };
                format!("{}/{}/{y}", self.trail.word(i, 0), self.trail.word(i, 1))
            })
            .collect();
        parts.join(".")
    }
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
}

/// The tasks' records (decision 780): the runtime in Sisu's
/// (`compiler/rt/src/tasks.sisu`), a queue of three words a record, the
/// record `t` the task `t`'s: its state, what it waits for (a word of the
/// scheduler's, [`crate::sched::Wait::word`]) and its flags. It parks a
/// task under its keys in the waiters' queue and wakes the tasks a key
/// names into the run queue; the coroutine, its context and the rest of a
/// task stay in the scheduler's `Task`, under the same number.
pub(crate) struct Records(Queue);

/// A record's states, as `tasks.sisu` numbers them.
pub(crate) const READY: i64 = 0;
pub(crate) const RUNNING: i64 = 1;
pub(crate) const BLOCKED: i64 = 2;
pub(crate) const DONE: i64 = 3;
/// A record's flag: the task is a generator.
pub(crate) const GENERATOR: i64 = 1;

impl Records {
    pub(crate) fn new() -> Records {
        Records(Queue::new(3))
    }

    /// Appends the record of a new task: its number.
    pub(crate) fn add(&mut self, state: i64, wait: i64, flags: i64) -> u32 {
        let mut t = 0i64;
        // SAFETY: `self.0` is a live queue of three words a record.
        unprobed(|| unsafe { sisu_rt_task_add(&mut t, &self.0.0, &state, &wait, &flags) });
        t as u32
    }

    /// The task's state, [`DONE`] for a number that is no task's, and
    /// what it waits for.
    pub(crate) fn state(&self, t: u32) -> i64 {
        let mut st = 0i64;
        // SAFETY: as in `add`.
        unprobed(|| unsafe { sisu_rt_task_state(&mut st, &self.0.0, &i64::from(t)) });
        st
    }

    pub(crate) fn wait(&self, t: u32) -> i64 {
        self.0.word(t as usize, 1)
    }

    pub(crate) fn flags(&self, t: u32) -> i64 {
        self.0.word(t as usize, 2)
    }

    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    /// Sets the state of the task `t`, one of the records, and its wait.
    pub(crate) fn set(&mut self, t: u32, state: i64, wait: i64) {
        if t as usize >= self.len() {
            crate::rt::internal("no such task");
        }
        let mut unit = 0u64;
        // SAFETY: as in `add`, and `t` is a record's.
        unprobed(|| unsafe { sisu_rt_task_set(&mut unit, &self.0.0, &i64::from(t), &state, &wait) });
    }

    /// Parks the task `t`, waiting for `wait`, under the `keys` in
    /// `waiters`.
    pub(crate) fn park(&mut self, waiters: &mut Queue, t: u32, wait: i64, keys: &[i64]) {
        if t as usize >= self.len() {
            crate::rt::internal("no such task");
        }
        let mut unit = 0u64;
        let (at, n) = (keys.as_ptr(), keys.len() as i64);
        // SAFETY: as in `set`; `waiters` is a live queue of two words a
        // record and `at` the `n` keys.
        unprobed(|| unsafe {
            sisu_rt_task_park(&mut unit, &self.0.0, &waiters.0, &i64::from(t), &wait, &at, &n)
        });
    }

    /// Wakes the task `t` into `ready` if it waits: whether it woke.
    pub(crate) fn wake(&mut self, ready: &mut Queue, t: u32) -> bool {
        let mut woke = 0i64;
        // SAFETY: as in `add`; `ready` is a live queue of one word a record.
        unprobed(|| unsafe { sisu_rt_task_wake(&mut woke, &self.0.0, &ready.0, &i64::from(t)) });
        woke != 0
    }

    /// Wakes the tasks waiting under `key` in `waiters` into `ready`, in
    /// the order they parked; `woken` holds those that woke: how many.
    pub(crate) fn wake_key(
        &mut self,
        waiters: &mut Queue,
        woken: &mut Queue,
        ready: &mut Queue,
        key: i64,
    ) -> usize {
        let mut n = 0i64;
        // SAFETY: as in `park` and `wake`; `woken` is a live queue of one
        // word a record.
        unprobed(|| unsafe {
            sisu_rt_task_wake_key(&mut n, &self.0.0, &waiters.0, &woken.0, &ready.0, &key)
        });
        n as usize
    }

    /// Wakes every waiting task but generators into `ready`; `woken`
    /// holds those that woke: how many.
    pub(crate) fn wake_all(&mut self, woken: &mut Queue, ready: &mut Queue) -> usize {
        let mut n = 0i64;
        // SAFETY: as in `wake_key`.
        unprobed(|| unsafe { sisu_rt_task_wake_all(&mut n, &self.0.0, &woken.0, &ready.0) });
        n as usize
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
