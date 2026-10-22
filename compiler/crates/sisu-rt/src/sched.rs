//! Tasks and their scheduler in natively compiled code (docs/architecture.md,
//! "Stage 2", "Tasks"; decision 203): the interpreter's seeded cooperative
//! scheduler (`sisu-interp/src/sched.rs`), with stackful tasks on the
//! stacks of the Phase 1 verdict ([`crate::stack`]).
//!
//! **The root task** runs `main` on the program's thread, whose stack is
//! large (decision 190). **Every other task** (a spawned one or a
//! generator) is a coroutine on a slot of the stack pool; corosensei, the
//! harness's context switch, moves between them. When the root task gives
//! way or waits, it runs the scheduler's loop on its own stack: the loop
//! picks the next task, resumes it until it gives way, waits or ends, and
//! picks again, until it picks the root task, whose call then returns. A
//! task that gives way or waits suspends itself back to that loop.
//!
//! **The same schedule.** The yield points are the interpreter's: the
//! entry of every function the interpreter enters by a call statement,
//! the back edges of loops (a jump to a block at or before the current
//! one), spawns, and `task.yield_now`. Generated code counts them down in
//! [`SISU_RT_BUDGET`] and calls [`sisu_rt_yield`] when it reaches zero; the
//! budgets, the choice of the next task and every other draw come from the
//! interpreter's SplitMix64 generator in the interpreter's order, so a run
//! under a seed is the interpreter's run under it. A function the runtime
//! enters where the interpreter pushes a frame outside a call statement
//! (`main`, a task's closure when it first runs, a generator's body, an
//! `invariant` check) is entered by [`neutral`], which keeps its entry from
//! counting.
//!
//! **The system stack.** Runtime code called from a task runs on the root
//! task's stack ([`system`], corosensei's `on_parent_stack`, as Go's
//! `systemstack`), so that a task's stack holds only Sisu frames and the
//! thin wrappers around the switch. Two things must stay on the task's
//! stack: what makes it wait, and what runs Sisu code. An operation that
//! must wait registers the wait and returns; its wrapper then suspends the
//! task, and tries again when the task is woken, as the interpreter
//! retries a blocked call ([`blocking`]). A drop on the system stack that
//! would run drop glue (or a mutex guard's `invariant`) moves the value
//! aside instead ([`defer`]), and the wrapper drops it on the task's stack
//! when the runtime code returns ([`drain`]), as the interpreter queues
//! the drops an intrinsic makes.

use crate::abi::{self, Kind};
use crate::rt::{self, Panic, ty};
use crate::stack::{Mark, Pool, Region, Slot};
use corosensei::{Coroutine, CoroutineResult, Yielder};
use sisu_diag::msg;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicI64, AtomicU32, AtomicU64, Ordering::Relaxed};

/// The root task's number.
pub(crate) const ROOT: u32 = 0;
/// The most yield points a task runs before the scheduler may switch.
const BUDGET_MAX: u64 = 8;
/// The counter's value while the budget is spent and no other task is
/// ready: no yield point reaches the scheduler until one is.
const STARVED: i64 = i64::MAX / 4;

/// The yield points left before the scheduler is asked (decision 85).
#[unsafe(no_mangle)]
pub static SISU_RT_BUDGET: AtomicI64 = AtomicI64::new(0);

/// The running task's stack limits ([`abi::STACK_PROBE`] and the others).
#[unsafe(no_mangle)]
pub static SISU_RT_STACK: [AtomicU64; abi::STACK_WORDS] = [const { AtomicU64::new(0) }; abi::STACK_WORDS];

/// Where the running code last called an intrinsic that may wait
/// ([`abi::SITE_FILE`] and the others).
#[unsafe(no_mangle)]
pub static SISU_RT_SITE: [AtomicU64; abi::SITE_WORDS] = [const { AtomicU64::new(0) }; abi::SITE_WORDS];

/// The seeded generator behind every scheduling choice: the interpreter's
/// SplitMix64, with its seeding, so the same seed draws the same numbers.
pub(crate) struct Rng(u64);

impl Rng {
    pub(crate) fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x2545_F491_4F6C_DD1D) ^ 0x9E37_79B9_7F4A_7C15)
    }

    pub(crate) fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub(crate) fn below(&mut self, n: usize) -> usize {
        if n <= 1 { 0 } else { (self.next() % n as u64) as usize }
    }
}

/// Why a task's coroutine handed control back to the loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Suspend {
    /// It gave way at a yield point (it is ready again).
    Preempt,
    /// It waits; the stack pointer it parked at.
    Park(usize),
}

pub(crate) type Co = Coroutine<(), Suspend, (), Slot>;

/// What a task waits for, for the deadlock report.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Wait {
    Join(u32),
    ScopeEnd,
    Recv,
    Send,
    Lock,
    Timer,
    Select,
    Cancellation,
    /// A socket that was not ready, tried again a millisecond later.
    Io,
}

/// What a waiting task is registered under, so an event wakes only the
/// tasks it concerns.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Key {
    Task(u32),
    Scope(u32),
    Chan(u32),
    Lock,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum TState {
    Ready,
    Running,
    Blocked(Wait),
    Done,
}

/// How a task ended: the words of its closure's result, or its panic.
pub(crate) enum Outcome {
    Value(Box<[u64]>),
    Panic(Panic),
}

/// What the interpreter's anchor frames mark on a task's stack: the
/// bottom of a spawned task, a scope's body, a shield's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Anchor {
    Task,
    Scope(u32),
    Shield,
}

/// What a scope was opened in, for cancellation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Node {
    Task(u32),
    Scope(u32),
}

/// Where a call that may wait is: the file, line and column of the
/// statement, and whether its function is `pure`.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Site {
    pub file: usize,
    pub file_len: usize,
    pub line: u64,
    pub col: u64,
    pub flags: u64,
}

impl Site {
    /// The site the generated code recorded before its last call of an
    /// intrinsic that may wait.
    pub(crate) fn current() -> Site {
        let w = |i: u32| SISU_RT_SITE[(i / 8) as usize].load(Relaxed);
        Site {
            file: w(abi::SITE_FILE) as usize,
            file_len: w(abi::SITE_FILE_LEN) as usize,
            line: w(abi::SITE_LINE),
            col: w(abi::SITE_COL),
            flags: w(abi::SITE_FLAGS),
        }
    }

    /// `file:line:col`, as the deadlock report names it.
    fn render(&self) -> String {
        if self.file == 0 {
            return String::new();
        }
        // SAFETY: the address and length are of a file name in the
        // program's read-only data, which lives as long as the program.
        let name =
            unsafe { crate::mem::Str { ptr: self.file as *const u8, len: self.file_len as u64 }.bytes() };
        format!("{}:{}:{}", String::from_utf8_lossy(name), self.line, self.col)
    }
}

/// What a task keeps of the runtime's state while another runs.
pub(crate) struct Ctx {
    pub depth: i64,
    pub stack: [u64; abi::STACK_WORDS],
    pub panics: Vec<Panic>,
}

pub(crate) struct Task {
    pub state: TState,
    /// The scope the task was spawned on (none for the root task and
    /// generators).
    pub scope: Option<u32>,
    /// Its position among the scope's tasks, from 0.
    pub index: u32,
    pub outcome: Option<Outcome>,
    /// A `join` took the outcome.
    pub taken: bool,
    /// Cancelled on its own: a sibling failed, or another task won a race.
    pub cancelled: bool,
    /// `spawn_with(deadline=...)`: (clock, nanoseconds).
    pub deadline: Option<(u32, i128)>,
    /// The trace context: spawn indices from the root task (`2.1`).
    pub path: String,
    pub spawned: u32,
    /// Whether the task's closure returns a result (`join` flattens it).
    pub returns_result: bool,
    /// The type of the closure's result.
    pub ret_ty: u32,
    /// A generator's item, yielded and not taken yet.
    pub yielded: Option<Box<[u64]>>,
    pub generator: bool,
    /// The type of a generator's items.
    pub item_ty: u32,
    /// The anchors of its stack, innermost last.
    pub anchors: Vec<Anchor>,
    /// Where it waits.
    pub site: Site,
    /// When it last became ready (`--stats`).
    pub ready_wall: Option<std::time::Instant>,
    /// Its coroutine, while it is not running (none for the root task).
    pub co: Option<Box<Co>>,
    pub yielder: *const Yielder<(), Suspend>,
    pub region: Option<Region>,
    pub mark: Mark,
    pub ctx: Ctx,
    /// The code and environment of the closure it runs.
    pub body: (usize, usize),
}

pub(crate) struct ScopeRec {
    pub parent: Option<Node>,
    pub cancelled: bool,
    pub children: Vec<u32>,
    pub pure: bool,
    pub race: bool,
    pub body_done: bool,
    pub ended: bool,
    pub first_failure: Option<u32>,
    pub winner: Option<u32>,
}

/// A bounded channel; its values are kept as words.
pub(crate) struct Channel {
    pub buf: VecDeque<Box<[u64]>>,
    pub cap: usize,
    pub closed: bool,
    pub senders: u32,
    pub receivers: u32,
    pub elem: u32,
}

/// A clock of the run: the host's, real, or a virtual one.
pub(crate) struct RtClock {
    pub virtual_: bool,
    pub now: i128,
}

/// The scheduler's numbers for `--stats` (the interpreter's, but the
/// statements between ready and running, which native code does not
/// count).
#[derive(Default)]
pub(crate) struct Stats {
    pub tasks: u64,
    pub switches: u64,
    pub max_live: u64,
    pub latency_micros: Vec<u64>,
    /// `Shared` cells made by construction sites tagged task-local and
    /// may-cross, and cells reached by another task than their creator.
    pub shared_local: u64,
    pub shared_crossing: u64,
    pub shared_crossed: u64,
}

pub(crate) struct Sched {
    pub tasks: Vec<Task>,
    pub cur: u32,
    pub scopes: Vec<ScopeRec>,
    pub channels: Vec<Channel>,
    pub clocks: Vec<RtClock>,
    pub rng: Rng,
    pub ready: Vec<u32>,
    pub waiters: HashMap<Key, Vec<u32>>,
    pub timers: Vec<(u32, i128, u32)>,
    /// Tasks not done.
    pub live: u64,
    pub stats: Stats,
    pub keep_stats: bool,
    /// The budget is spent and nothing was ready ([`STARVED`]).
    pub starved: bool,
    /// A [`neutral`] call whose entry will reach the scheduler.
    pub neutral_armed: bool,
    pub pool: Pool,
    /// The run's test clock (`Clock.fake`), once made.
    pub fake_clock: Option<u32>,
}

/// The type numbers of `TaskError` and `ChannelError` (`u32::MAX` when the
/// program has none).
static TASK_ERROR: AtomicU32 = AtomicU32::new(u32::MAX);
static CHANNEL_ERROR: AtomicU32 = AtomicU32::new(u32::MAX);

pub(crate) fn task_error() -> Option<u32> {
    Some(TASK_ERROR.load(Relaxed)).filter(|&t| t != u32::MAX)
}

pub(crate) fn channel_error() -> Option<u32> {
    Some(CHANNEL_ERROR.load(Relaxed)).filter(|&t| t != u32::MAX)
}

thread_local! {
    /// The scheduler. Never dropped: the program exits with coroutines
    /// suspended (a deadlock's abort, a generator nobody finished), whose
    /// frames are Sisu code and must not be unwound.
    static SCHED: std::mem::ManuallyDrop<RefCell<Option<Sched>>> =
        const { std::mem::ManuallyDrop::new(RefCell::new(None)) };
    /// The running coroutine's yielder; null in the root task.
    static YIELDER: Cell<*const Yielder<(), Suspend>> = const { Cell::new(std::ptr::null()) };
    /// Runtime code of a task runs on the system stack ([`system`]).
    static ON_SYSTEM: Cell<bool> = const { Cell::new(false) };
    /// Values whose drop runs Sisu code, moved aside on the system stack.
    static DEFERRED: RefCell<Vec<(Box<[u64]>, u32)>> = const { RefCell::new(Vec::new()) };
}

/// Runs `f` on the scheduler's state. `f` must neither run Sisu code nor
/// switch tasks: the state is not shared while it runs.
pub(crate) fn with<R>(f: impl FnOnce(&mut Sched) -> R) -> R {
    SCHED.with(|s| match s.try_borrow_mut() {
        Ok(mut g) => match g.as_mut() {
            Some(sched) => f(sched),
            None => rt::internal("the scheduler is not running"),
        },
        Err(_) => rt::internal("the scheduler was entered from inside itself"),
    })
}

/// The type number of the enum named `name` that is an error set.
fn error_type(name: &str) -> Option<u32> {
    let mut id = 0u32;
    loop {
        let t = rt::ty_checked(id)?;
        if let Kind::Enum { name: n, error: true, .. } = &t.kind
            && n == name
        {
            return Some(id);
        }
        id += 1;
    }
}

/// Starts the scheduler with the root task running: the generator seeded
/// by `seed` draws the root's first budget, as the interpreter's does.
pub(crate) fn start(seed: u64, keep_stats: bool, pool: Pool, root_limits: [u64; abi::STACK_WORDS]) {
    let mut rng = Rng::new(seed);
    let budget = 1 + rng.next() % BUDGET_MAX;
    SISU_RT_BUDGET.store(budget as i64, Relaxed);
    for (w, v) in SISU_RT_STACK.iter().zip(root_limits) {
        w.store(v, Relaxed);
    }
    let root = Task {
        state: TState::Running,
        scope: None,
        index: 0,
        outcome: None,
        taken: false,
        cancelled: false,
        deadline: None,
        path: String::new(),
        spawned: 0,
        returns_result: false,
        ret_ty: 0,
        yielded: None,
        item_ty: 0,
        generator: false,
        anchors: Vec::new(),
        site: Site::default(),
        ready_wall: None,
        co: None,
        yielder: std::ptr::null(),
        region: None,
        mark: Mark::default(),
        ctx: Ctx { depth: 0, stack: root_limits, panics: Vec::new() },
        body: (0, 0),
    };
    let sched = Sched {
        tasks: vec![root],
        cur: ROOT,
        scopes: Vec::new(),
        channels: Vec::new(),
        clocks: vec![RtClock { virtual_: false, now: 0 }],
        rng,
        ready: Vec::new(),
        waiters: HashMap::new(),
        timers: Vec::new(),
        live: 1,
        stats: Stats { tasks: 1, max_live: 1, ..Stats::default() },
        keep_stats,
        starved: false,
        neutral_armed: false,
        pool,
        fake_clock: None,
    };
    TASK_ERROR.store(error_type("TaskError").unwrap_or(u32::MAX), Relaxed);
    CHANNEL_ERROR.store(error_type("ChannelError").unwrap_or(u32::MAX), Relaxed);
    SCHED.with(|s| *s.borrow_mut() = Some(sched));
}

/// Ends the run: the coroutines of generators nobody finished stay
/// suspended, and are forgotten rather than unwound (their frames are
/// Sisu code, which has no unwinder), as the interpreter drops an
/// abandoned generator's frames without running them (decision 95).
pub(crate) fn finish() -> Option<Stats> {
    let s = SCHED.with(|s| s.borrow_mut().take())?;
    let keep = s.keep_stats;
    let mut s = std::mem::ManuallyDrop::new(s);
    let stats = std::mem::take(&mut s.stats);
    keep.then_some(stats)
}

// --------------------------------------------------------------- stacks

/// The message of a failed probe of the running task, once: the probe
/// left it pending ([`abi::STACK_FAILED`]).
pub(crate) fn take_stack_failure() -> Option<String> {
    let w = &SISU_RT_STACK[(abi::STACK_FAILED / 8) as usize];
    if w.load(Relaxed) == 0 {
        return None;
    }
    w.store(0, Relaxed);
    let bytes = if YIELDER.with(Cell::get).is_null() {
        crate::entry::STACK_BYTES as u64
    } else {
        SCHED
            .with(|s| s.try_borrow().ok().and_then(|g| g.as_ref().map(|s| s.pool.reservation as u64)))
            .unwrap_or(0)
    };
    Some(rt::render(msg::PANIC_STACK_EXHAUSTED, &[("bytes", bytes.to_string())]))
}

fn load_ctx(ctx: &mut Ctx) {
    crate::entry::SISU_RT_DEPTH.store(ctx.depth, Relaxed);
    for (w, v) in SISU_RT_STACK.iter().zip(ctx.stack) {
        w.store(v, Relaxed);
    }
    rt::swap_panics(&mut ctx.panics);
}

fn save_ctx(ctx: &mut Ctx) {
    ctx.depth = crate::entry::SISU_RT_DEPTH.load(Relaxed);
    for (v, w) in ctx.stack.iter_mut().zip(SISU_RT_STACK.iter()) {
        *v = w.load(Relaxed);
    }
    rt::swap_panics(&mut ctx.panics);
}

// ---------------------------------------------------------- the system stack

/// Runs `f` on the system stack: directly in the root task (or when
/// already there), and through the parent's stack in any other.
pub(crate) fn system<R>(f: impl FnOnce() -> R) -> R {
    let y = YIELDER.with(Cell::get);
    if y.is_null() || ON_SYSTEM.with(Cell::get) {
        return f();
    }
    struct AssertSend<F>(F);
    // SAFETY: the closure runs on this thread, on the parent's stack, and
    // returns before the task goes on; nothing crosses a thread.
    unsafe impl<F> Send for AssertSend<F> {}
    let w = AssertSend(f);
    ON_SYSTEM.with(|c| c.set(true));
    // SAFETY: `y` is the yielder of the coroutine running now, and `f`
    // neither suspends it (only `switch` does, never inside `system`) nor
    // runs Sisu code (drops that would are deferred).
    let r = unsafe {
        (*y).on_parent_stack(move || {
            let w = w;
            (w.0)()
        })
    };
    ON_SYSTEM.with(|c| c.set(false));
    r
}

/// Whether a drop must move its value aside instead of running Sisu code
/// ([`defer`]).
pub(crate) fn deferring() -> bool {
    ON_SYSTEM.with(Cell::get)
}

/// Moves the value at `p` of type `t` aside, to be dropped on the task's
/// stack by [`drain`].
///
/// # Safety
/// `p` must hold a value of type `t`, which is moved out.
pub(crate) unsafe fn defer(p: *mut u8, t: u32) -> u8 {
    let size = ty(t).size as usize;
    let mut words = vec![0u64; size.div_ceil(8)].into_boxed_slice();
    // SAFETY: `p` holds `size` bytes, and the buffer has room for them.
    unsafe { crate::mem::copy(words.as_mut_ptr() as *mut u8, p, size) };
    DEFERRED.with(|d| {
        let mut d = d.borrow_mut();
        d.push((words, t));
        PENDING.store(d.len(), Relaxed);
    });
    abi::OK
}

/// How many values [`DEFERRED`] holds: what [`drain`] reads first, since
/// most runtime code moves nothing aside and the program's one thread of
/// Sisu code is the only one that runs it.
static PENDING: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Drops what the runtime code just run moved aside, on the task's stack,
/// in the order it was moved; `status` is that code's.
#[inline]
pub(crate) fn drain(status: u8) -> u8 {
    if PENDING.load(Relaxed) == 0 { status } else { drain_pending(status) }
}

#[inline(never)]
fn drain_pending(mut status: u8) -> u8 {
    let batch = DEFERRED.with(|d| std::mem::take(&mut *d.borrow_mut()));
    PENDING.store(0, Relaxed);
    for (mut words, t) in batch {
        // SAFETY: the words hold the value moved aside, dropped once here.
        let st = unsafe { crate::value::drop_value(words.as_mut_ptr() as *mut u8, t) };
        status = crate::value::after_part(status, st);
    }
    status
}

/// Runs `f` on the system stack and drops what it moved aside.
pub(crate) fn system_status(f: impl FnOnce() -> u8) -> u8 {
    let st = system(f);
    drain(st)
}

/// What an operation that may wait did.
pub(crate) enum Attempt {
    /// It completed, with this status.
    Done(u8),
    /// The task is registered as waiting: suspend it, and try again when
    /// it is woken.
    Wait,
}

/// An operation that may wait: `attempt` runs on the system stack until it
/// completes, and between attempts the task waits.
pub(crate) fn blocking(mut attempt: impl FnMut() -> Attempt) -> u8 {
    loop {
        match system(&mut attempt) {
            Attempt::Done(st) => return drain(st),
            Attempt::Wait => {
                let st = drain(abi::OK);
                if st != abi::OK {
                    return st;
                }
                switch(Suspend::Park(0));
            }
        }
    }
}

/// Calls a Sisu function the interpreter enters without a call statement
/// (it pushes the frame itself): the function's entry is not a yield
/// point, so its count is given back first.
pub(crate) fn neutral<R>(f: impl FnOnce() -> R) -> R {
    let armed = with(|s| {
        let c = SISU_RT_BUDGET.load(Relaxed);
        let armed = !s.starved && c <= 0;
        s.neutral_armed |= armed;
        SISU_RT_BUDGET.store(c + 1, Relaxed);
        armed
    });
    let r = f();
    // The entry's count was not taken (the probe or the depth check
    // stopped the function first): take the gift back.
    if armed {
        with(|s| {
            if s.neutral_armed {
                s.neutral_armed = false;
                SISU_RT_BUDGET.fetch_sub(1, Relaxed);
            }
        });
    }
    r
}

// -------------------------------------------------------------- switching

/// A task's coroutine starts: `y` is its yielder.
pub(crate) fn enter_coroutine(y: &Yielder<(), Suspend>) {
    YIELDER.with(|c| c.set(y as *const Yielder<(), Suspend>));
}

/// The running task gives way (it is in the ready list) or waits (it is
/// registered): other tasks run until the scheduler picks it again.
pub(crate) fn switch(why: Suspend) {
    let y = YIELDER.with(Cell::get);
    if y.is_null() {
        run_loop();
        return;
    }
    if ON_SYSTEM.with(Cell::get) {
        rt::internal("a task switch on the system stack");
    }
    let why = match why {
        Suspend::Park(_) => Suspend::Park(crate::stack::current_sp()),
        other => other,
    };
    // SAFETY: `y` is the yielder of the coroutine running now, and this is
    // its own stack.
    unsafe { (*y).suspend(why) };
}

/// The scheduler's loop, on the root task's stack: runs other tasks until
/// it picks the root task.
fn run_loop() {
    let mut root_ctx = with(|s| std::mem::replace(&mut s.tasks[ROOT as usize].ctx, empty_ctx()));
    save_ctx(&mut root_ctx);
    let mut last = ROOT;
    loop {
        let next = with(|s| s.pick(last));
        if next == ROOT {
            load_ctx(&mut root_ctx);
            return;
        }
        let (co, mut ctx, yielder) = with(|s| {
            let t = &mut s.tasks[next as usize];
            (t.co.take(), std::mem::replace(&mut t.ctx, empty_ctx()), t.yielder)
        });
        let Some(mut co) = co else { rt::internal("a task without its coroutine was picked") };
        load_ctx(&mut ctx);
        YIELDER.with(|c| c.set(yielder));
        let r = co.resume(());
        YIELDER.with(|c| c.set(std::ptr::null()));
        save_ctx(&mut ctx);
        match r {
            CoroutineResult::Yield(why) => with(|s| {
                let t = &mut s.tasks[next as usize];
                let dirty = ctx.stack[(abi::STACK_DIRTY / 8) as usize] != 0;
                t.mark.dirty |= dirty;
                ctx.stack[(abi::STACK_DIRTY / 8) as usize] = 0;
                if let (Suspend::Park(sp), Some(region)) = (why, t.region) {
                    t.mark = s.pool.on_park(region, sp, t.mark);
                }
                let t = &mut s.tasks[next as usize];
                // The probe doubles as the decommit watermark; after an
                // exhaustion it stays at the floor.
                if ctx.stack[(abi::STACK_FAILED / 8) as usize] == 0
                    && ctx.stack[(abi::STACK_PROBE / 8) as usize] >= ctx.stack[(abi::STACK_REAL / 8) as usize]
                {
                    let real = ctx.stack[(abi::STACK_REAL / 8) as usize];
                    ctx.stack[(abi::STACK_PROBE / 8) as usize] =
                        if t.mark.dirty { real } else { real.max(t.mark.clean_below as u64) };
                }
                t.ctx = ctx;
                t.co = Some(co);
            }),
            CoroutineResult::Return(()) => {
                // The task has ended: its slot goes back to the pool.
                let stack = co.into_stack();
                with(|s| {
                    let t = &mut s.tasks[next as usize];
                    t.mark.dirty |= ctx.stack[(abi::STACK_DIRTY / 8) as usize] != 0;
                    let mark = t.mark;
                    t.region = None;
                    s.pool.release(stack.0, mark);
                });
            }
        }
        last = next;
    }
}

/// Runs the frames of a generator abandoned while suspended at a `yield`
/// to their end, here and now, outside the schedule: its yield returns
/// [`abi::UNWINDING`] (the task's [`abi::STACK_ABANDONED`] is set), and its
/// frames unwind with their drops forgetting what they drop, so no Sisu
/// code runs (decisions 95 and 223). The running task's state, its panics in flight
/// and what it moved aside are kept apart meanwhile. Returns whether the
/// generator ended; its stack goes back to the pool either way.
pub(crate) fn unwind_abandoned(t: u32) -> bool {
    let taken = with(|s| {
        let task = &mut s.tasks[t as usize];
        task.co.take().map(|co| (co, std::mem::replace(&mut task.ctx, empty_ctx()), task.yielder))
    });
    let Some((mut co, mut ctx, yielder)) = taken else { return false };
    // A closure whose cleanup runs code of its own keeps its frames.
    if ctx.stack[(abi::STACK_ABANDONED / 8) as usize] == abi::ABANDON_KEEP {
        // SAFETY: the generator is suspended and never resumed again.
        unsafe { co.force_reset() };
        let stack = co.into_stack();
        with(|s| {
            let task = &mut s.tasks[t as usize];
            let mark = task.mark;
            task.region = None;
            task.state = TState::Done;
            s.pool.release(stack.0, mark);
        });
        return false;
    }
    let mut mine = empty_ctx();
    save_ctx(&mut mine);
    let (my_yielder, on_system) = (YIELDER.with(Cell::get), ON_SYSTEM.with(Cell::get));
    let deferred = DEFERRED.with(|d| std::mem::take(&mut *d.borrow_mut()));
    PENDING.store(0, Relaxed);
    ctx.stack[(abi::STACK_ABANDONED / 8) as usize] = 1;
    load_ctx(&mut ctx);
    YIELDER.with(|c| c.set(yielder));
    ON_SYSTEM.with(|c| c.set(false));
    let r = crate::value::forget(|| co.resume(()));
    save_ctx(&mut ctx);
    YIELDER.with(|c| c.set(my_yielder));
    ON_SYSTEM.with(|c| c.set(on_system));
    PENDING.store(deferred.len(), Relaxed);
    DEFERRED.with(|d| *d.borrow_mut() = deferred);
    load_ctx(&mut mine);
    let ended = matches!(r, CoroutineResult::Return(()));
    if !ended {
        // SAFETY: the generator is suspended and never resumed again; its
        // frames own nothing left to free.
        unsafe { co.force_reset() };
    }
    let stack = co.into_stack();
    with(|s| {
        let task = &mut s.tasks[t as usize];
        let mark = task.mark;
        task.region = None;
        task.state = TState::Done;
        s.pool.release(stack.0, mark);
    });
    ended
}

/// Whether the running task is a generator being abandoned.
pub(crate) fn abandoning() -> bool {
    SISU_RT_STACK[(abi::STACK_ABANDONED / 8) as usize].load(Relaxed) == 1
}

fn empty_ctx() -> Ctx {
    Ctx { depth: 0, stack: [0; abi::STACK_WORDS], panics: Vec::new() }
}

/// A yield point found the budget spent ([`abi::sym::YIELD`]).
#[unsafe(no_mangle)]
extern "C" fn sisu_rt_yield() {
    if with(|s| s.yield_point(false)) {
        switch(Suspend::Preempt);
    }
}

/// `task.yield_now`: a yield point that gives way whenever another task is
/// ready.
pub(crate) fn yield_now() {
    let go = with(|s| {
        if !s.starved {
            SISU_RT_BUDGET.fetch_sub(1, Relaxed);
        }
        s.yield_point(true)
    });
    if go {
        switch(Suspend::Preempt);
    }
}

// ------------------------------------------------------------ the state

impl Sched {
    pub(crate) fn task(&mut self, t: u32) -> &mut Task {
        match self.tasks.get_mut(t as usize) {
            Some(task) => task,
            None => rt::internal("no such task"),
        }
    }

    /// A new task's record.
    pub(crate) fn new_task(&self, scope: Option<u32>, index: u32, path: String) -> Task {
        Task {
            state: TState::Ready,
            scope,
            index,
            outcome: None,
            taken: false,
            cancelled: false,
            deadline: None,
            path,
            spawned: 0,
            returns_result: false,
            ret_ty: 0,
            yielded: None,
            generator: false,
            item_ty: 0,
            anchors: vec![Anchor::Task],
            site: Site::default(),
            ready_wall: None,
            co: None,
            yielder: std::ptr::null(),
            region: None,
            mark: Mark::default(),
            ctx: Ctx { depth: 1, stack: [0; abi::STACK_WORDS], panics: Vec::new() },
            body: (0, 0),
        }
    }

    /// Puts a task in the ready list: a spent budget then switches at the
    /// next yield point.
    pub(crate) fn push_ready(&mut self, t: u32) {
        self.ready.push(t);
        if self.keep_stats {
            self.task(t).ready_wall = Some(std::time::Instant::now());
        }
        if self.starved {
            self.starved = false;
            SISU_RT_BUDGET.store(0, Relaxed);
        }
    }

    /// A spawn counts as a yield point without switching.
    pub(crate) fn spend_one(&mut self) {
        if !self.starved {
            SISU_RT_BUDGET.fetch_sub(1, Relaxed);
        }
    }

    /// The interpreter's `yield_point`, called when the budget reached zero
    /// (or for `task.yield_now`, `asked`): whether to give way; the
    /// running task is then in the ready list.
    fn yield_point(&mut self, asked: bool) -> bool {
        if !asked && self.neutral_armed {
            // The entry of a function entered by `neutral`.
            self.neutral_armed = false;
            SISU_RT_BUDGET.store(0, Relaxed);
            return false;
        }
        let spent = SISU_RT_BUDGET.load(Relaxed) <= 0 || self.starved;
        if spent && !self.starved {
            SISU_RT_BUDGET.store(0, Relaxed);
        }
        if (spent || asked) && !self.ready.is_empty() {
            let cur = self.cur;
            self.task(cur).state = TState::Ready;
            self.push_ready(cur);
            return true;
        }
        if spent && !self.starved {
            self.starved = true;
            SISU_RT_BUDGET.store(STARVED, Relaxed);
        }
        false
    }

    /// The interpreter's `switch`: the next task among the ready ones, by
    /// the generator (advancing the clocks when none is ready); its budget.
    fn pick(&mut self, last: u32) -> u32 {
        let next = loop {
            if !self.ready.is_empty() {
                let i = self.rng.below(self.ready.len());
                break self.ready.swap_remove(i);
            }
            if !self.advance_time() {
                rt::abort(&self.deadlock_report());
            }
        };
        if next != last {
            self.stats.switches += 1;
        }
        self.cur = next;
        let keep = self.keep_stats;
        let task = self.task(next);
        task.state = TState::Running;
        if keep && let Some(w) = task.ready_wall.take() {
            let us = w.elapsed().as_micros() as u64;
            self.stats.latency_micros.push(us);
        }
        let budget = 1 + self.rng.next() % BUDGET_MAX;
        SISU_RT_BUDGET.store(budget as i64, Relaxed);
        self.starved = false;
        next
    }

    /// Nothing is ready: advance the clocks to the next timer and wake what
    /// is due; whether anything woke.
    fn advance_time(&mut self) -> bool {
        if self.timers.is_empty() {
            return false;
        }
        let mut virt: Option<i128> = None;
        let mut real: Option<i128> = None;
        for &(c, at, _) in &self.timers {
            let delta = at - self.clock_now(c);
            let slot =
                if self.clocks.get(c as usize).is_some_and(|k| k.virtual_) { &mut virt } else { &mut real };
            if slot.is_none_or(|d| delta < d) {
                *slot = Some(delta);
            }
        }
        if let Some(d) = virt
            && d > 0
        {
            for k in self.clocks.iter_mut().filter(|k| k.virtual_) {
                k.now += d;
            }
        } else if virt.is_none()
            && let Some(d) = real
            && d > 0
        {
            std::thread::sleep(std::time::Duration::from_nanos(d.min(i128::from(u64::MAX)) as u64));
        }
        self.wake_due_timers()
    }

    pub(crate) fn wake_due_timers(&mut self) -> bool {
        let mut due = Vec::new();
        let timers = std::mem::take(&mut self.timers);
        for (c, at, t) in timers {
            if self.clock_now(c) >= at {
                due.push(t);
            } else {
                self.timers.push((c, at, t));
            }
        }
        let any = !due.is_empty();
        for t in due {
            self.wake(t);
        }
        any
    }

    /// The report of a deadlock (`R0080`), with what each task waits for
    /// and where (`R0081`).
    fn deadlock_report(&self) -> String {
        let mut lines = vec![rt::render(msg::RUN_DEADLOCK, &[])];
        for (i, t) in self.tasks.iter().enumerate() {
            let TState::Blocked(w) = t.state else { continue };
            let who = if i == 0 {
                rt::render(msg::RUN_TASK_ROOT, &[])
            } else {
                rt::render(msg::RUN_TASK_NAMED, &[("task", t.path.clone())])
            };
            let what = match w {
                Wait::Join(x) => {
                    format!(
                        "join task {}",
                        self.tasks.get(x as usize).map(|t| t.path.as_str()).unwrap_or("?")
                    )
                }
                Wait::ScopeEnd => "the end of a scope".into(),
                Wait::Recv => "recv".into(),
                Wait::Send => "send".into(),
                Wait::Lock => "lock".into(),
                Wait::Timer => "a timer".into(),
                Wait::Select => "select".into(),
                Wait::Cancellation => "a cancellation".into(),
                Wait::Io => "the network".into(),
            };
            lines.push(format!(
                "  {}",
                rt::render(
                    msg::RUN_DEADLOCK_TASK,
                    &[("task", who), ("wait", what), ("location", t.site.render())]
                )
            ));
        }
        lines.join("\n")
    }

    /// The run's test clock, made at the epoch on first use (the
    /// interpreter's `fake_clock`).
    pub(crate) fn fake_clock(&mut self) -> u32 {
        if let Some(c) = self.fake_clock {
            return c;
        }
        let c = self.clocks.len() as u32;
        self.clocks.push(RtClock { virtual_: true, now: 0 });
        self.fake_clock = Some(c);
        c
    }

    pub(crate) fn clock_now(&self, c: u32) -> i128 {
        match self.clocks.get(c as usize) {
            Some(k) if k.virtual_ => k.now,
            Some(_) => std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as i128)
                .unwrap_or(0),
            None => 0,
        }
    }

    // ------------------------------------------------------ waiting

    /// Makes the running task wait: registered under the keys and its timer
    /// (and the deadline on the way to the root, which wakes it to see it).
    pub(crate) fn block_on(&mut self, wait: Wait, keys: &[Key], timer: Option<(u32, i128)>, site: Site) {
        let cur = self.cur;
        for k in keys {
            self.waiters.entry(*k).or_default().push(cur);
        }
        if let Some((c, at)) = timer {
            self.timers.push((c, at, cur));
        }
        if !matches!(wait, Wait::Lock | Wait::ScopeEnd)
            && let Some((c, at)) = self.chain_deadline(self.context())
        {
            self.timers.push((c, at, cur));
        }
        let task = self.task(cur);
        task.state = TState::Blocked(wait);
        task.site = site;
    }

    pub(crate) fn wake(&mut self, t: u32) {
        let Some(task) = self.tasks.get_mut(t as usize) else { return };
        if matches!(task.state, TState::Blocked(_)) {
            task.state = TState::Ready;
            self.push_ready(t);
        }
    }

    pub(crate) fn wake_key(&mut self, k: Key) {
        if let Some(list) = self.waiters.remove(&k) {
            for t in list {
                self.wake(t);
            }
        }
    }

    pub(crate) fn wake_all_blocked(&mut self) {
        for t in 0..self.tasks.len() as u32 {
            self.wake(t);
        }
    }

    // ------------------------------------------------- cancellation

    /// The cancellation context of the running code: its innermost scope
    /// or shield, else its task (none at the top of the root task).
    pub(crate) fn context(&self) -> Option<Node> {
        // Every anchor stops the search, so the innermost decides.
        match self.tasks.get(self.cur as usize)?.anchors.last()? {
            Anchor::Scope(s) => Some(Node::Scope(*s)),
            Anchor::Shield => None,
            Anchor::Task => Some(Node::Task(self.cur)),
        }
    }

    fn deadline_passed(&self, d: Option<(u32, i128)>) -> bool {
        d.is_some_and(|(c, at)| self.clock_now(c) >= at)
    }

    pub(crate) fn node_cancelled(&self, mut n: Option<Node>) -> bool {
        let mut guard = 0;
        while let Some(node) = n {
            guard += 1;
            if guard > 100_000 {
                return false;
            }
            match node {
                Node::Scope(s) => {
                    let Some(rec) = self.scopes.get(s as usize) else { return false };
                    if rec.cancelled {
                        return true;
                    }
                    n = rec.parent;
                }
                Node::Task(t) => {
                    let Some(task) = self.tasks.get(t as usize) else { return false };
                    if task.cancelled || self.deadline_passed(task.deadline) {
                        return true;
                    }
                    n = task.scope.map(Node::Scope);
                }
            }
        }
        false
    }

    pub(crate) fn cancelled_now(&self) -> bool {
        self.node_cancelled(self.context())
    }

    /// The earliest deadline on the chain of a context.
    pub(crate) fn chain_deadline(&self, mut n: Option<Node>) -> Option<(u32, i128)> {
        let mut best: Option<(u32, i128)> = None;
        let mut guard = 0;
        while let Some(node) = n {
            guard += 1;
            if guard > 100_000 {
                break;
            }
            match node {
                Node::Scope(s) => n = self.scopes.get(s as usize).and_then(|r| r.parent),
                Node::Task(t) => {
                    let Some(task) = self.tasks.get(t as usize) else { break };
                    if let Some(d) = task.deadline
                        && best.is_none_or(|b| d.1 < b.1)
                    {
                        best = Some(d);
                    }
                    n = task.scope.map(Node::Scope);
                }
            }
        }
        best
    }

    /// Cancels the scope's tasks still running, all of them or those after
    /// `after` in spawn order.
    pub(crate) fn cancel_children(&mut self, s: u32, except: Option<u32>, after: Option<u32>) {
        let children = self.scopes.get(s as usize).map(|r| r.children.clone()).unwrap_or_default();
        for t in children {
            if Some(t) == except {
                continue;
            }
            let task = &self.tasks[t as usize];
            if matches!(task.state, TState::Done) {
                continue;
            }
            if after.is_some_and(|i| task.index <= i) {
                continue;
            }
            self.tasks[t as usize].cancelled = true;
        }
        self.wake_all_blocked();
    }
}
