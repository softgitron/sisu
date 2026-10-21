//! Tasks, scopes and the seeded cooperative scheduler (06; 18, Phase 6:
//! "tasks as heap-allocated frame stacks on the already-trampolined
//! interpreter; a single-OS-thread scheduler").
//!
//! A task is a frame stack. The running task's stack is `Interp::frames`;
//! every other task's is in its [`Task`] record, and switching tasks swaps
//! the two. A reference names the task whose stack holds its frame
//! (`Pointer::task`), so a task's closure reads the frames of the task that
//! opened its scope, which cannot return before the task ends.
//!
//! **Anchors.** The scheduler pushes frames of its own, with no function:
//! the bottom of every spawned task ([`Anchor::Task`]) receives what the
//! task's closure returns or the panic that unwound it; a scope's anchor
//! ([`Anchor::Scope`]) sits under the body of `task.scope` and, when the
//! body returns or panics, cancels as the rules say, waits for every task
//! of the scope, and gives the scope's result (or continues the panic); a
//! shield's anchor ([`Anchor::Shield`]) marks the code under `task.shield`.
//! The innermost scope or shield anchor of the running stack is the
//! cancellation context of the code that runs (06, "Cancellation").
//!
//! **Blocking.** An intrinsic that must wait (`join`, `recv`, `send` on a
//! full channel, `lock` of a held mutex, a timer, `select`) returns
//! `Unwind::Block` with its arguments; the statement records the call in
//! the task, and the task waits. When it is woken, the call is retried
//! with the same arguments, so it completes exactly once. Every blocking
//! operation but `lock` checks the cancellation of the code running first
//! and returns `Err(Cancelled)` when it is cancelled (unless shielded).
//!
//! **Scheduling.** Yield points are the back edges of loops, calls of Sisu
//! functions and spawns (Q-34); each task gets a budget of yield points,
//! drawn from the seeded generator, and when it is spent the scheduler
//! picks the next task among the ready ones with the same generator. A run
//! is therefore a function of the program, its input and the seed
//! (`--schedule-seed`). When no task is ready, the virtual clocks advance
//! to the next timer; when there is none, the program is deadlocked.

use crate::value::Value;
use crate::{Frame, Interp, PanicInfo, Unwind, ice, msg, panic, render};
use sisu_sir as sir;
use std::collections::HashMap;

/// The place in an anchor frame that receives what the frame above it
/// returns.
pub(crate) static RESULT_SLOT: sir::Place = sir::Place { local: sir::LocalId(0), proj: Vec::new() };

/// The most yield points a task runs before the scheduler may switch.
const BUDGET_MAX: u64 = 8;

/// The yield points among which PCT draws its priority change points.
pub const PCT_STEPS: u64 = 1000;

/// How the scheduler chooses (`--scheduler`, 17: "seeded and pluggable").
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Strategy {
    /// Seeded random budgets of yield points and a seeded random choice of
    /// the next task.
    #[default]
    Random,
    /// Probabilistic concurrency testing (Burckhardt, Kothari, Musuvathi
    /// and Nagarakatte, ASPLOS 2010): every task gets a random priority
    /// and the highest-priority ready task runs; at `depth - 1` yield
    /// points drawn from the first [`PCT_STEPS`], the running task drops
    /// below every other. A bug of depth `d` among `n` tasks and `k` steps
    /// shows with probability at least `1 / (n k^(d-1))` per run.
    Pct { depth: u32 },
    /// One schedule of a bounded exhaustive search: the choices of
    /// `prefix`, then the first option at every later choice point (no
    /// preemption, the first ready task by creation).
    Exhaustive { prefix: Vec<u32> },
}

/// A choice the scheduler made (or `sim.choose`): which option of how
/// many, and whether its other options preempt a task that could go on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Choice {
    pub taken: u32,
    pub options: u32,
    pub at_yield: bool,
}

/// The next schedule of a bounded exhaustive search after the one that
/// made `trail` (depth-first): the last choice that can take its next
/// option without the schedule preempting more than `bound` times does,
/// and the choices after it are dropped. `None` once every schedule within
/// the bound was run.
pub fn next_schedule(trail: &[Choice], bound: u32) -> Option<Vec<u32>> {
    let mut before = Vec::with_capacity(trail.len());
    let mut n = 0;
    for c in trail {
        before.push(n);
        if c.at_yield && c.taken > 0 {
            n += 1;
        }
    }
    for i in (0..trail.len()).rev() {
        let c = trail[i];
        if c.taken + 1 < c.options {
            let preempts = before[i] + u32::from(c.at_yield);
            if preempts <= bound {
                let mut p: Vec<u32> = trail[..i].iter().map(|c| c.taken).collect();
                p.push(c.taken + 1);
                return Some(p);
            }
        }
    }
    None
}

/// A frame the scheduler pushed (module documentation).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Anchor {
    Task,
    Scope(u32),
    Shield,
    /// `test.catch`: receives what the call returns, or the panic that
    /// unwound it (a typed hole goes on).
    Catch,
    /// `sim.always`: the body runs above it while its predicate is checked
    /// at every switch; it gives the body's result.
    Always,
}

/// The seeded generator behind every scheduling choice (SplitMix64).
#[derive(Clone, Debug)]
pub(crate) struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x2545_F491_4F6C_DD1D) ^ 0x9E37_79B9_7F4A_7C15)
    }

    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn below(&mut self, n: usize) -> usize {
        if n <= 1 { 0 } else { (self.next() % n as u64) as usize }
    }
}

/// What a task waits for, for the deadlock report.
#[derive(Clone, Debug)]
pub(crate) enum Wait {
    Join(u32),
    ScopeEnd,
    Recv,
    Send,
    Lock,
    Timer,
    Select,
    Cancellation,
    /// A socket of the host, polled.
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

#[derive(Debug)]
pub(crate) enum TState {
    Ready,
    Running,
    Blocked(Wait),
    Done,
}

/// How a task ended.
#[derive(Debug)]
pub(crate) enum Outcome {
    Value(Value),
    Panic(Box<PanicInfo>),
}

/// An intrinsic call a task is blocked in, retried when the task wakes.
#[derive(Debug)]
pub(crate) struct PendingCall<'p> {
    pub name: String,
    pub tys: Vec<sir::Type>,
    pub args: Vec<Value>,
    pub dest: &'p sir::Place,
    pub ret: Option<sir::Type>,
}

/// What an intrinsic that must wait returns.
#[derive(Debug)]
pub(crate) struct Blocked {
    pub args: Vec<Value>,
    pub wait: Wait,
    pub keys: Vec<Key>,
    /// A timer to be woken by: (clock, nanoseconds).
    pub timer: Option<(u32, i128)>,
}

pub(crate) struct Task<'p> {
    pub frames: Vec<Frame<'p>>,
    pub state: TState,
    /// The scope the task was spawned on (none for the root task).
    pub scope: Option<u32>,
    /// Its position among the scope's tasks, from 0.
    pub index: u32,
    pub pending: Option<PendingCall<'p>>,
    pub outcome: Option<Outcome>,
    /// A `join` took the outcome.
    pub taken: bool,
    /// Cancelled on its own: a sibling failed, or another task won a race.
    pub cancelled: bool,
    /// `spawn_with(deadline=...)`: (clock, nanoseconds).
    pub deadline: Option<(u32, i128)>,
    /// The trace context: spawn indices from the root task (`2.1`).
    pub path: String,
    /// How many tasks this task has spawned, for its children's paths.
    pub spawned: u32,
    /// Whether the task's closure returns a result (`join` flattens it).
    pub returns_result: bool,
    /// The closure's result type, to drop a result nobody joined.
    pub ret_ty: Option<sir::Type>,
    /// When the task last became ready, in statements run (`--stats`).
    pub ready_at: u64,
    pub ready_wall: Option<std::time::Instant>,
    /// A generator's item, yielded and not taken yet.
    pub yielded: Option<Value>,
    pub generator: bool,
    /// The epoch of hot replacement the task has adopted (`live.rs`): at
    /// its start, and at each `live.boundary()` since.
    pub epoch: usize,
}

/// What a scope was opened in, for cancellation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Node {
    Task(u32),
    Scope(u32),
}

pub(crate) struct ScopeRec {
    /// The context the scope was opened in; `None` at the top of the root
    /// task or inside a shield.
    pub parent: Option<Node>,
    pub cancelled: bool,
    pub children: Vec<u32>,
    /// Opened by a `pure` function: the lowest-index failure rule.
    pub pure: bool,
    pub race: bool,
    /// The body returned (or panicked).
    pub body_done: bool,
    /// The scope returned: spawning on it panics.
    pub ended: bool,
    /// The first task to fail, in time (effectful scopes).
    pub first_failure: Option<u32>,
    /// The first task to finish (races).
    pub winner: Option<u32>,
    /// The type of the scope's result, which decides how it carries a
    /// failure.
    pub result_ty: Option<sir::Type>,
}

/// A bounded channel.
pub(crate) struct Channel {
    pub buf: std::collections::VecDeque<Value>,
    pub cap: usize,
    pub closed: bool,
    pub senders: u32,
    pub receivers: u32,
    pub elem: Option<sir::Type>,
}

/// A clock of the run: the host's, or a virtual one.
pub(crate) struct RtClock {
    /// Virtual clocks advance only when every task waits (17, "Isolation
    /// and parallelism") or when told to; the host's real clock follows
    /// real time.
    pub virtual_: bool,
    pub now: i128,
}

/// Scheduling numbers for `--stats` (18, Phase 6: "`--stats` reports
/// scheduling latency so Q-34 has a number").
#[derive(Clone, Debug, Default)]
pub struct Stats {
    pub tasks: u64,
    pub switches: u64,
    pub max_live: u64,
    /// Statements other tasks ran between a task becoming ready and
    /// running, one entry per wait.
    pub latency_steps: Vec<u64>,
    /// The same in host microseconds.
    pub latency_micros: Vec<u64>,
    /// `Shared` cells made by construction sites tagged task-local and
    /// may-cross, and cells reached by another task than their creator.
    pub shared_local: u64,
    pub shared_crossing: u64,
    pub shared_crossed: u64,
    /// `--stats stack-depth`: the calls whose frame was at a depth from
    /// `2^k` to `2^(k+1) - 1`, by `k`; the deepest stack, and its estimated
    /// native size.
    pub depths: Vec<u64>,
    pub max_depth: usize,
    pub max_stack_bytes: u64,
    /// `--stats shared`: each construction site that ran, by function and
    /// location, with its tag, the cells it made and those another task
    /// reached.
    pub sites: Vec<SharedSite>,
}

/// A `Shared` construction site's numbers over a run.
#[derive(Clone, Debug, Default)]
pub struct SharedSite {
    pub function: String,
    pub loc: Option<sisu_sir::Loc>,
    pub source: u32,
    pub atomic: bool,
    pub cells: u64,
    pub crossed: u64,
}

pub(crate) struct Sched<'p> {
    pub tasks: Vec<Task<'p>>,
    pub cur: u32,
    pub scopes: Vec<ScopeRec>,
    pub channels: Vec<Channel>,
    pub clocks: Vec<RtClock>,
    pub rng: Rng,
    pub ready: Vec<u32>,
    pub waiters: HashMap<Key, Vec<u32>>,
    pub timers: Vec<(u32, i128, u32)>,
    pub budget: u64,
    /// Tasks not done.
    pub live: u64,
    /// `task.yield_now` asked for a switch.
    pub yield_asked: bool,
    pub steps: u64,
    pub stats: Stats,
    pub keep_stats: bool,
    /// The `Shared` cells a `.copy()` counted, for the monitor's check.
    pub copied_shared: Vec<u32>,
    /// The run's test clock (`Clock.fake`), once made.
    pub fake_clock: Option<u32>,
    pub strategy: Strategy,
    /// PCT: each task's priority (by task), the yield points at which the
    /// running task's priority drops, the next such priority, and the
    /// yield points so far.
    pub priorities: Vec<u64>,
    pub change_points: Vec<u64>,
    pub next_low: u64,
    pub yields: u64,
    /// The choices made, for a bounded exhaustive search.
    pub trail: Vec<Choice>,
    /// The task a preemption chose (exhaustive).
    pub forced: Option<u32>,
    /// The predicates of the `sim.always` blocks open, innermost last, and
    /// whether one is being checked (no switch is a choice then).
    pub always: Vec<(Value, String)>,
    pub checking: bool,
    /// A predicate met a lock its own task holds; the check is skipped.
    pub check_skipped: bool,
}

impl<'p> Sched<'p> {
    pub fn new(seed: u64, keep_stats: bool, host_clock: RtClock, strategy: Strategy) -> Sched<'p> {
        let root = Task {
            frames: Vec::new(),
            state: TState::Running,
            scope: None,
            index: 0,
            pending: None,
            outcome: None,
            taken: false,
            cancelled: false,
            deadline: None,
            path: String::new(),
            spawned: 0,
            returns_result: false,
            ret_ty: None,
            ready_at: 0,
            ready_wall: None,
            yielded: None,
            generator: false,
            epoch: 0,
        };
        let mut rng = Rng::new(seed);
        let budget = 1 + rng.next() % BUDGET_MAX;
        let mut change_points = Vec::new();
        let mut next_low = 0;
        if let Strategy::Pct { depth } = &strategy {
            for _ in 1..(*depth).max(1) {
                change_points.push(1 + rng.next() % PCT_STEPS);
            }
            next_low = 1 << 39;
        }
        Sched {
            tasks: vec![root],
            cur: 0,
            scopes: Vec::new(),
            channels: Vec::new(),
            clocks: vec![host_clock],
            rng,
            ready: Vec::new(),
            waiters: HashMap::new(),
            timers: Vec::new(),
            budget,
            live: 1,
            yield_asked: false,
            steps: 0,
            stats: Stats { tasks: 1, max_live: 1, ..Stats::default() },
            keep_stats,
            copied_shared: Vec::new(),
            fake_clock: None,
            strategy,
            priorities: Vec::new(),
            change_points,
            next_low,
            yields: 0,
            trail: Vec::new(),
            forced: None,
            always: Vec::new(),
            checking: false,
            check_skipped: false,
        }
    }

    /// A task's PCT priority, drawn when first asked: above every change
    /// point's.
    fn priority(&mut self, t: u32) -> u64 {
        while self.priorities.len() <= t as usize {
            let p = (1 << 40) + self.rng.next() % 1_000_000_000;
            self.priorities.push(p);
        }
        self.priorities[t as usize]
    }

    /// The next choice among `options`: the exhaustive schedule's, or a
    /// seeded random one; recorded for the exhaustive search.
    pub fn choose(&mut self, options: u32, at_yield: bool) -> u32 {
        if options <= 1 {
            return 0;
        }
        let taken = match &self.strategy {
            Strategy::Exhaustive { prefix } => {
                prefix.get(self.trail.len()).copied().unwrap_or(0).min(options - 1)
            }
            _ => self.rng.below(options as usize) as u32,
        };
        if matches!(self.strategy, Strategy::Exhaustive { .. }) {
            self.trail.push(Choice { taken, options, at_yield });
        }
        taken
    }

    /// The ready tasks in creation order, for a choice that must not
    /// depend on the order they became ready in.
    fn ready_sorted(&self) -> Vec<u32> {
        let mut r = self.ready.clone();
        r.sort_unstable();
        r
    }
}

fn is_err(v: &Value) -> bool {
    matches!(v, Value::Variant(a) if &*a.ty == "Result" && a.variant == 1)
}

/// The error inside `Err(e)`.
fn err_payload(v: &Value) -> Option<&Value> {
    match v {
        Value::Variant(a) if &*a.ty == "Result" && a.variant == 1 => a.fields.first(),
        _ => None,
    }
}

/// Whether a value of type `ty` (a result type) can hold an error of the
/// declaration `decl`.
fn carries(ty: &Option<sir::Type>, decl: &str) -> bool {
    match ty {
        Some(sir::Type::Named(n, args)) if n == "Result" => match args.get(1) {
            Some(sir::Type::ErrSet(names)) => names.iter().any(|x| x == decl),
            Some(sir::Type::Named(n, _)) => n == decl,
            _ => false,
        },
        _ => false,
    }
}

fn is_result_ty(ty: &Option<sir::Type>) -> bool {
    matches!(ty, Some(sir::Type::Named(n, _)) if n == "Result")
}

impl<'p> Interp<'p, '_> {
    // ---------------------------------------------------------------- stacks

    /// The frame stack of a task: the running one's is `self.frames`.
    pub(crate) fn frames_of(&self, task: u32) -> &Vec<Frame<'p>> {
        if task == self.sched.cur {
            &self.frames
        } else {
            match self.sched.tasks.get(task as usize) {
                Some(t) => &t.frames,
                None => &self.frames,
            }
        }
    }

    pub(crate) fn frames_of_mut(&mut self, task: u32) -> &mut Vec<Frame<'p>> {
        if task == self.sched.cur || task as usize >= self.sched.tasks.len() {
            &mut self.frames
        } else {
            &mut self.sched.tasks[task as usize].frames
        }
    }

    /// A frame the scheduler pushes, with one slot for a result.
    fn anchor_frame(&mut self, anchor: Anchor, ret: Option<&'p sir::Place>) -> Frame<'p> {
        self.serial += 1;
        Frame {
            serial: self.serial,
            func: None,
            locals: vec![Value::Moved],
            loc: None,
            bb: 0,
            next: 0,
            pending: None,
            ret,
            tys: Vec::new(),
            drops: Vec::new(),
            after: None,
            resume_after_drops: false,
            anchor: Some(anchor),
            stack_bytes: 0,
        }
    }

    /// The frame of a call of a closure value (its environment first).
    fn closure_frame(
        &mut self,
        f: Value,
        args: Vec<Value>,
        ret: Option<&'p sir::Place>,
    ) -> Result<(Frame<'p>, usize), Unwind> {
        let epoch = self.task_epoch();
        self.closure_frame_in(f, args, ret, epoch)
    }

    /// [`Interp::closure_frame`] with the closure's body from an epoch.
    fn closure_frame_in(
        &mut self,
        f: Value,
        mut args: Vec<Value>,
        ret: Option<&'p sir::Place>,
        epoch: usize,
    ) -> Result<(Frame<'p>, usize), Unwind> {
        let Value::Closure(c) = f else {
            return Err(ice(format!("a task or scope body that is not a closure: {f:?}")));
        };
        let c = std::rc::Rc::try_unwrap(c).unwrap_or_else(|rc| (*rc).clone());
        args.insert(0, Value::Tuple(c.env));
        let fi = self.fn_index_in(epoch, &c.name)?;
        let fr = self.make_frame(fi, args, ret, c.tys)?;
        Ok((fr, fi))
    }

    // ---------------------------------------------------------------- contexts

    /// The cancellation context of the running code: the innermost scope
    /// or shield anchor of its stack, else its task.
    pub(crate) fn context(&self) -> Option<Node> {
        for fr in self.frames.iter().rev() {
            match fr.anchor {
                Some(Anchor::Scope(s)) => return Some(Node::Scope(s)),
                Some(Anchor::Shield) => return None,
                Some(Anchor::Task) => return Some(Node::Task(self.sched.cur)),
                Some(Anchor::Catch | Anchor::Always) | None => {}
            }
        }
        None
    }

    /// `test.catch(f, x, steps)`: the call `f(x)` above an anchor that
    /// gives `Ok(result)`, `Err(Panicked(message))` when a panic unwinds
    /// it, or `Err(TooLong(steps))` when it runs more than `steps`
    /// statements (counted over every task, so the same under a seed).
    pub(crate) fn open_catch(&mut self, mut args: Vec<Value>, dest: &'p sir::Place) -> Result<(), Unwind> {
        if args.len() != 3 {
            return Err(ice("test.catch takes a function, its argument and a budget"));
        }
        let steps = match args.pop() {
            Some(Value::Int(n)) => n.max(1) as u64,
            _ => return Err(ice("test.catch's budget is an int")),
        };
        let x = args.pop().unwrap_or(Value::Unit);
        let f = args.pop().unwrap_or(Value::Unit);
        let (fi, vals, tys) = self.callee_of(&f, vec![x])?;
        let anchor = self.anchor_frame(Anchor::Catch, Some(dest));
        let fr = self.make_frame(fi, vals, Some(&RESULT_SLOT), tys)?;
        self.push_built(anchor)?;
        self.push_built(fr)?;
        self.case_ends.push((self.sched.steps.saturating_add(steps), steps));
        Ok(())
    }

    fn deadline_passed(&self, d: Option<(u32, i128)>) -> bool {
        match d {
            Some((c, at)) => self.clock_now(c) >= at,
            None => false,
        }
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
                    let Some(rec) = self.sched.scopes.get(s as usize) else { return false };
                    if rec.cancelled {
                        return true;
                    }
                    n = rec.parent;
                }
                Node::Task(t) => {
                    let Some(task) = self.sched.tasks.get(t as usize) else { return false };
                    if task.cancelled || self.deadline_passed(task.deadline) {
                        return true;
                    }
                    n = task.scope.map(Node::Scope);
                }
            }
        }
        false
    }

    /// Whether the running code is cancelled.
    pub(crate) fn cancelled_now(&self) -> bool {
        self.node_cancelled(self.context())
    }

    /// The earliest deadline on the chain of a context, to wake a waiting
    /// task when it passes.
    fn chain_deadline(&self, mut n: Option<Node>) -> Option<(u32, i128)> {
        let mut best: Option<(u32, i128)> = None;
        let mut guard = 0;
        while let Some(node) = n {
            guard += 1;
            if guard > 100_000 {
                break;
            }
            match node {
                Node::Scope(s) => n = self.sched.scopes.get(s as usize).and_then(|r| r.parent),
                Node::Task(t) => {
                    let Some(task) = self.sched.tasks.get(t as usize) else { break };
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

    // ---------------------------------------------------------------- values

    fn task_error(&self, variant: &str, fields: Vec<Value>) -> Value {
        let i = self.variants.get(&("TaskError", variant)).copied().unwrap_or(match variant {
            "Cancelled" => 0,
            _ => 1,
        });
        Value::variant("TaskError", i, fields)
    }

    pub(crate) fn cancelled_err(&self) -> Value {
        Value::err(self.task_error("Cancelled", Vec::new()))
    }

    fn panicked_err(&self, p: &PanicInfo) -> Value {
        Value::err(self.task_error(
            "TaskPanicked",
            vec![Value::String(p.message.clone()), Value::String(p.stack.join("\n"))],
        ))
    }

    fn closed_err(&self) -> Value {
        let i = self.variants.get(&("ChannelError", "Closed")).copied().unwrap_or(0);
        Value::err(Value::variant("ChannelError", i, Vec::new()))
    }

    /// Whether a task's value is `Err(Cancelled)`.
    fn is_cancelled_error(v: &Value) -> bool {
        match err_payload(v) {
            Some(Value::Variant(a)) => &*a.ty == "TaskError" && a.variant == 0,
            _ => false,
        }
    }

    fn id_of(&self, v: &Value) -> Result<u32, Unwind> {
        match self.target(v)? {
            Value::Scope(i)
            | Value::Handle(i)
            | Value::Cancellation(i)
            | Value::Sender(i)
            | Value::Receiver(i)
            | Value::Clock(i) => Ok(*i),
            other => Err(ice(format!("expected a task handle, found {other:?}"))),
        }
    }

    // ---------------------------------------------------------------- scopes

    /// `task.scope`, `task.race` and `task.shield`: pushes the anchor and
    /// the body's frame. The anchor gives the result to `dest`.
    pub(crate) fn open_scope(
        &mut self,
        name: &str,
        mut args: Vec<Value>,
        dest: &'p sir::Place,
        ret: Option<sir::Type>,
    ) -> Result<(), Unwind> {
        let body =
            std::mem::replace(args.get_mut(0).ok_or_else(|| ice("a scope without a body"))?, Value::Moved);
        if name == "task.shield" {
            let anchor = self.anchor_frame(Anchor::Shield, Some(dest));
            let (fr, _) = self.closure_frame(body, Vec::new(), Some(&RESULT_SLOT))?;
            self.push_built(anchor)?;
            return self.push_built(fr);
        }
        let pure = self.frames.last().and_then(|f| f.func).and_then(|i| self.func(i)).is_some_and(|f| f.pure);
        let s = self.sched.scopes.len() as u32;
        let parent = self.context();
        self.sched.scopes.push(ScopeRec {
            parent,
            cancelled: false,
            children: Vec::new(),
            pure,
            race: name == "task.race",
            body_done: false,
            ended: false,
            first_failure: None,
            winner: None,
            result_ty: ret,
        });
        let anchor = self.anchor_frame(Anchor::Scope(s), Some(dest));
        let (fr, _) = self.closure_frame(body, vec![Value::Scope(s)], Some(&RESULT_SLOT))?;
        self.push_built(anchor)?;
        self.push_built(fr)
    }

    /// `Scope.spawn` and `Scope.spawn_with`.
    fn spawn(&mut self, args: &mut [Value], with_deadline: bool) -> Result<Value, Unwind> {
        let s = self.id_of(args.first().ok_or_else(|| ice("spawn without a scope"))?)?;
        if self.sched.scopes.get(s as usize).is_none_or(|r| r.ended) {
            return Err(panic(msg::PANIC_SCOPE_ENDED, &[]));
        }
        let deadline = if with_deadline {
            match self.target(args.get(1).ok_or_else(|| ice("spawn_with without a deadline"))?)? {
                Value::Timer { clock, at } => Some((*clock, *at)),
                other => return Err(ice(format!("a deadline that is not a timer: {other:?}"))),
            }
        } else {
            None
        };
        let k = if with_deadline { 2 } else { 1 };
        let f = std::mem::replace(args.get_mut(k).ok_or_else(|| ice("spawn without a task"))?, Value::Moved);
        // A task adopts the newest epoch when it starts (`live.rs`), its
        // own body included.
        self.live_poll()?;
        let epoch = self.newest_epoch();
        let t = self.sched.tasks.len() as u32;
        let anchor = self.anchor_frame(Anchor::Task, None);
        let (fr, fi) = self.closure_frame_in(f, Vec::new(), Some(&RESULT_SLOT), epoch)?;
        let ret_ty = self.func(fi).map(|g| g.ret().clone());
        let returns_result = is_result_ty(&ret_ty);
        let cur = self.sched.cur as usize;
        self.sched.tasks[cur].spawned += 1;
        let n = self.sched.tasks[cur].spawned;
        let path = match self.sched.tasks[cur].path.as_str() {
            "" => n.to_string(),
            p => format!("{p}.{n}"),
        };
        let index = self.sched.scopes[s as usize].children.len() as u32;
        self.sched.scopes[s as usize].children.push(t);
        self.sched.tasks.push(Task {
            frames: vec![anchor, fr],
            state: TState::Ready,
            scope: Some(s),
            index,
            pending: None,
            outcome: None,
            taken: false,
            cancelled: false,
            deadline,
            path,
            spawned: 0,
            returns_result,
            ret_ty,
            ready_at: self.sched.steps,
            ready_wall: self.sched.keep_stats.then(std::time::Instant::now),
            yielded: None,
            generator: false,
            epoch,
        });
        self.sched.ready.push(t);
        self.sched.live += 1;
        self.sched.stats.tasks += 1;
        self.sched.stats.max_live = self.sched.stats.max_live.max(self.sched.live);
        // Spawning is a yield point.
        self.sched.budget = self.sched.budget.saturating_sub(1);
        Ok(Value::Handle(t))
    }

    /// One step of an anchor frame at the top of the running stack.
    pub(crate) fn step_anchor(&mut self, anchor: Anchor) -> Result<crate::exec::Event<'p>, Unwind> {
        use crate::exec::Event;
        match anchor {
            Anchor::Shield => {
                let fr = self.pop_frame()?;
                if let Some(p) = fr.pending {
                    return Err(Unwind::Panic(p));
                }
                let v = fr.locals.into_iter().next().unwrap_or(Value::Unit);
                Ok(Event::Returned(v, fr.ret))
            }
            Anchor::Task => {
                let fr = self.pop_frame()?;
                let outcome = match fr.pending {
                    Some(p) => Outcome::Panic(p),
                    None => Outcome::Value(fr.locals.into_iter().next().unwrap_or(Value::Unit)),
                };
                self.finish_task(outcome);
                Ok(Event::Switch)
            }
            Anchor::Scope(s) => self.step_scope_end(s),
            Anchor::Always => {
                let fr = self.pop_frame()?;
                if let Some(p) = fr.pending {
                    self.sched.always.pop();
                    return Err(Unwind::Panic(p));
                }
                let checked = self.check_always();
                self.sched.always.pop();
                checked?;
                let v = fr.locals.into_iter().next().unwrap_or(Value::Unit);
                Ok(Event::Returned(v, fr.ret))
            }
            Anchor::Catch => {
                let fr = self.pop_frame()?;
                self.case_ends.pop();
                let v = match fr.pending {
                    // A typed hole blocks the test; it is not a failure.
                    Some(p) if p.hole.is_some() => return Err(Unwind::Panic(p)),
                    Some(p) if p.too_long.is_some() => Value::err(Value::Variant(Value::adt(
                        "Panicked",
                        1,
                        vec![Value::Int(p.too_long.unwrap_or(0) as i64)],
                    ))),
                    Some(p) => {
                        Value::err(Value::Variant(Value::adt("Panicked", 0, vec![Value::String(p.message)])))
                    }
                    None => Value::ok(fr.locals.into_iter().next().unwrap_or(Value::Unit)),
                };
                Ok(Event::Returned(v, fr.ret))
            }
        }
    }

    fn pop_frame(&mut self) -> Result<Frame<'p>, Unwind> {
        let fr = self.frames.pop().ok_or_else(|| ice("no frame to pop"))?;
        let n = self.frames.len() as u32;
        self.mon_frame_gone(fr.serial, n, fr.locals.len());
        Ok(fr)
    }

    /// Cancels the scope's tasks (not the scope: later spawns are not
    /// cancelled) that are still running, all of them or those after
    /// `after` in spawn order.
    fn cancel_children(&mut self, s: u32, except: Option<u32>, after: Option<u32>) {
        let children = self.sched.scopes.get(s as usize).map(|r| r.children.clone()).unwrap_or_default();
        for t in children {
            if Some(t) == except {
                continue;
            }
            let task = &self.sched.tasks[t as usize];
            if matches!(task.state, TState::Done) {
                continue;
            }
            if after.is_some_and(|i| task.index <= i) {
                continue;
            }
            self.sched.tasks[t as usize].cancelled = true;
        }
        self.wake_all_blocked();
    }

    /// A task's closure returned or panicked: the scope's rules.
    fn finish_task(&mut self, outcome: Outcome) {
        let t = self.sched.cur;
        let ti = t as usize;
        let cancelled = self.node_cancelled(Some(Node::Task(t)));
        let failure = match &outcome {
            Outcome::Panic(_) => true,
            Outcome::Value(v) => {
                self.sched.tasks[ti].returns_result
                    && is_err(v)
                    && !(cancelled && Self::is_cancelled_error(v))
            }
        };
        self.sched.tasks[ti].outcome = Some(outcome);
        self.sched.tasks[ti].state = TState::Done;
        if !self.sched.tasks[ti].generator {
            self.sched.live = self.sched.live.saturating_sub(1);
        }
        if let Some(s) = self.sched.tasks[ti].scope {
            let index = self.sched.tasks[ti].index;
            let (race, pure) = match self.sched.scopes.get(s as usize) {
                Some(r) => (r.race, r.pure),
                None => (false, false),
            };
            if race {
                if self.sched.scopes[s as usize].winner.is_none() {
                    self.sched.scopes[s as usize].winner = Some(t);
                    self.cancel_children(s, Some(t), None);
                }
            } else if failure {
                if pure {
                    // Earlier siblings run to completion; later ones stop.
                    self.cancel_children(s, Some(t), Some(index));
                } else {
                    if self.sched.scopes[s as usize].first_failure.is_none() {
                        self.sched.scopes[s as usize].first_failure = Some(t);
                    }
                    self.cancel_children(s, Some(t), None);
                }
            }
            self.wake_key(Key::Scope(s));
        }
        self.wake_key(Key::Task(t));
    }

    /// Takes a finished task's outcome as `join` gives it.
    fn take_outcome(&mut self, t: u32) -> Result<Value, Unwind> {
        let task = &mut self.sched.tasks[t as usize];
        if task.taken {
            return Err(panic(msg::PANIC_JOINED_TWICE, &[]));
        }
        task.taken = true;
        let returns_result = task.returns_result;
        match task.outcome.take() {
            Some(Outcome::Value(v)) => Ok(if returns_result { v } else { Value::ok(v) }),
            Some(Outcome::Panic(p)) => Ok(self.panicked_err(&p)),
            None => Err(ice("a finished task without an outcome")),
        }
    }

    /// The end of a scope: the body returned (its value is in the anchor's
    /// slot) or panicked (the anchor's `pending`).
    fn step_scope_end(&mut self, s: u32) -> Result<crate::exec::Event<'p>, Unwind> {
        use crate::exec::Event;
        let si = s as usize;
        let top = self.frames.len().checked_sub(1).ok_or_else(|| ice("no anchor"))?;
        if !self.sched.scopes[si].body_done {
            self.sched.scopes[si].body_done = true;
            let panicked = self.frames[top].pending.is_some();
            let failed = is_result_ty(&self.sched.scopes[si].result_ty)
                && self.frames[top].locals.first().is_some_and(is_err);
            // A panic in the body, or an error it returns, cancels the
            // tasks still running.
            if (panicked || failed) && !self.sched.scopes[si].race {
                self.sched.scopes[si].cancelled = true;
                self.wake_all_blocked();
            }
        }
        let children = self.sched.scopes[si].children.clone();
        if children.iter().any(|&t| !matches!(self.sched.tasks[t as usize].state, TState::Done)) {
            return Err(self.block_on(Blocked {
                args: Vec::new(),
                wait: Wait::ScopeEnd,
                keys: vec![Key::Scope(s)],
                timer: None,
            }));
        }
        let fr = self.pop_frame()?;
        self.sched.scopes[si].ended = true;
        let result_ty = self.sched.scopes[si].result_ty.clone();
        if let Some(p) = fr.pending {
            self.drop_unjoined(&children);
            return Err(Unwind::Panic(p));
        }
        let body = fr.locals.into_iter().next().unwrap_or(Value::Unit);
        if self.sched.scopes[si].race {
            let winner = self.sched.scopes[si].winner.ok_or_else(|| panic(msg::PANIC_EMPTY_RACE, &[]))?;
            let v = self.take_outcome(winner)?;
            self.drop_unjoined(&children);
            self.queue_drop(body, None);
            return Ok(Event::Returned(v, fr.ret));
        }
        if is_result_ty(&result_ty) && is_err(&body) {
            self.drop_unjoined(&children);
            return Ok(Event::Returned(body, fr.ret));
        }
        // A failure nobody joined: the first in time, or in a `pure`
        // scope the lowest index.
        let unobserved = |t: u32, me: &Self| {
            let task = &me.sched.tasks[t as usize];
            !task.taken
                && match &task.outcome {
                    Some(Outcome::Panic(_)) => true,
                    Some(Outcome::Value(v)) => {
                        task.returns_result
                            && is_err(v)
                            && !(Self::is_cancelled_error(v)
                                && (task.cancelled || me.sched.scopes[si].cancelled))
                    }
                    None => false,
                }
        };
        let failed = if self.sched.scopes[si].pure {
            children.iter().copied().find(|&t| unobserved(t, self))
        } else {
            self.sched.scopes[si].first_failure.filter(|&t| unobserved(t, self))
        };
        let Some(t) = failed else {
            self.drop_unjoined(&children);
            return Ok(Event::Returned(body, fr.ret));
        };
        let outcome = self.sched.tasks[t as usize].outcome.take();
        self.sched.tasks[t as usize].taken = true;
        self.drop_unjoined(&children);
        self.queue_drop(body, result_ty.clone());
        match outcome {
            Some(Outcome::Panic(p)) => {
                if carries(&result_ty, "TaskError") {
                    Ok(Event::Returned(self.panicked_err(&p), fr.ret))
                } else {
                    Err(Unwind::Panic(p))
                }
            }
            Some(Outcome::Value(v)) => {
                let decl = match err_payload(&v) {
                    Some(Value::Variant(a)) => a.ty.to_string(),
                    _ => String::new(),
                };
                if carries(&result_ty, &decl) {
                    Ok(Event::Returned(v, fr.ret))
                } else {
                    let shown = err_payload(&v).map(|e| self.debug_string(e)).unwrap_or_default();
                    self.queue_drop(v, None);
                    Err(panic(msg::PANIC_UNOBSERVED_ERROR, &[("error", shown)]))
                }
            }
            None => Err(ice("a failed task without an outcome")),
        }
    }

    /// Drops the results of a scope's tasks that nobody took.
    fn drop_unjoined(&mut self, children: &[u32]) {
        for &t in children {
            let task = &mut self.sched.tasks[t as usize];
            if task.taken {
                continue;
            }
            let ty = task.ret_ty.clone();
            if let Some(Outcome::Value(v)) = task.outcome.take() {
                self.queue_drop(v, ty);
            }
        }
    }

    // ---------------------------------------------------------------- blocking and switching

    /// Makes the running task wait: registers it under the keys and its
    /// timer, and tells the run loop to switch.
    pub(crate) fn block_on(&mut self, b: Blocked) -> Unwind {
        let cur = self.sched.cur;
        for k in &b.keys {
            self.sched.waiters.entry(*k).or_default().push(cur);
        }
        if let Some((c, at)) = b.timer {
            self.sched.timers.push((c, at, cur));
        }
        // A deadline on the way to the root wakes the task to see it.
        if !matches!(b.wait, Wait::Lock | Wait::ScopeEnd)
            && let Some((c, at)) = self.chain_deadline(self.context())
        {
            self.sched.timers.push((c, at, cur));
        }
        self.sched.tasks[cur as usize].state = TState::Blocked(b.wait);
        Unwind::Switch
    }

    fn wake(&mut self, t: u32) {
        let Some(task) = self.sched.tasks.get_mut(t as usize) else { return };
        if matches!(task.state, TState::Blocked(_)) {
            task.state = TState::Ready;
            task.ready_at = self.sched.steps;
            if self.sched.keep_stats {
                task.ready_wall = Some(std::time::Instant::now());
            }
            self.sched.ready.push(t);
        }
    }

    pub(crate) fn wake_key(&mut self, k: Key) {
        if let Some(list) = self.sched.waiters.remove(&k) {
            for t in list {
                self.wake(t);
            }
        }
    }

    pub(crate) fn wake_all_blocked(&mut self) {
        for t in 0..self.sched.tasks.len() as u32 {
            self.wake(t);
        }
    }

    /// Whether the scheduler should switch at a yield point: the budget is
    /// spent and another task could run.
    pub(crate) fn yield_point(&mut self) -> bool {
        let asked = std::mem::take(&mut self.sched.yield_asked);
        // A predicate of `sim.always` runs to its end.
        if self.sched.checking {
            return false;
        }
        match self.sched.strategy.clone() {
            Strategy::Random => {
                self.sched.budget = self.sched.budget.saturating_sub(1);
                (self.sched.budget == 0 || asked) && !self.sched.ready.is_empty()
            }
            Strategy::Pct { .. } => {
                self.sched.yields += 1;
                let cur = self.sched.cur;
                if self.sched.change_points.contains(&self.sched.yields) {
                    self.sched.priority(cur);
                    self.sched.next_low = self.sched.next_low.saturating_sub(1);
                    self.sched.priorities[cur as usize] = self.sched.next_low;
                }
                // A task that gives way (`task.yield_now`) drops below
                // every other, or a spin loop would never let one run.
                if asked && !self.sched.ready.is_empty() {
                    self.sched.priority(cur);
                    let low = self.sched.priorities.iter().copied().min().unwrap_or(1).saturating_sub(1);
                    self.sched.priorities[cur as usize] = low;
                }
                let mine = self.sched.priority(cur);
                let ready = self.sched.ready.clone();
                let best = ready.iter().map(|t| self.sched.priority(*t)).max();
                best.is_some_and(|b| b > mine)
            }
            Strategy::Exhaustive { .. } => {
                if self.sched.ready.is_empty() {
                    return false;
                }
                let ready = self.sched.ready_sorted();
                let c = self.sched.choose(1 + ready.len() as u32, true);
                if c == 0 {
                    return false;
                }
                self.sched.forced = Some(ready[c as usize - 1]);
                true
            }
        }
    }

    /// The running task gives way at a yield point.
    pub(crate) fn preempt(&mut self) -> Result<(), Unwind> {
        let cur = self.sched.cur;
        self.sched.tasks[cur as usize].state = TState::Ready;
        self.sched.tasks[cur as usize].ready_at = self.sched.steps;
        self.sched.ready.push(cur);
        self.switch()
    }

    /// Runs another task: the running one waits, gave way or finished.
    pub(crate) fn switch(&mut self) -> Result<(), Unwind> {
        let next = loop {
            if !self.sched.ready.is_empty() {
                let i = match self.sched.strategy {
                    Strategy::Random => self.sched.rng.below(self.sched.ready.len()),
                    Strategy::Pct { .. } => {
                        let ready = self.sched.ready.clone();
                        let mut best = 0;
                        let mut top = None;
                        for (k, t) in ready.iter().enumerate() {
                            let p = self.sched.priority(*t);
                            if top.is_none_or(|q| p > q) {
                                top = Some(p);
                                best = k;
                            }
                        }
                        best
                    }
                    Strategy::Exhaustive { .. } => {
                        let chosen = match self.sched.forced.take() {
                            Some(t) => t,
                            None => {
                                let ready = self.sched.ready_sorted();
                                let c = self.sched.choose(ready.len() as u32, false);
                                ready[c as usize]
                            }
                        };
                        self.sched.ready.iter().position(|t| *t == chosen).unwrap_or(0)
                    }
                };
                break self.sched.ready.swap_remove(i);
            }
            if !self.advance_time()? {
                return Err(self.deadlock());
            }
        };
        let cur = self.sched.cur;
        if next != cur {
            std::mem::swap(&mut self.frames, &mut self.sched.tasks[cur as usize].frames);
            std::mem::swap(&mut self.frames, &mut self.sched.tasks[next as usize].frames);
            self.sched.cur = next;
            self.sched.stats.switches += 1;
        }
        let steps = self.sched.steps;
        let task = &mut self.sched.tasks[next as usize];
        task.state = TState::Running;
        if self.sched.keep_stats {
            self.sched.stats.latency_steps.push(steps.saturating_sub(task.ready_at));
            if let Some(w) = task.ready_wall.take() {
                self.sched.stats.latency_micros.push(w.elapsed().as_micros() as u64);
            }
        }
        self.sched.budget = 1 + self.sched.rng.next() % BUDGET_MAX;
        self.check_always()
    }

    /// Runs every predicate of the open `sim.always` blocks, on the stack
    /// of the task about to run (its blocked call, if any, waits until
    /// they are done); a predicate that does not hold stops the run (17:
    /// "an invariant checked after every step").
    pub(crate) fn check_always(&mut self) -> Result<(), Unwind> {
        if self.sched.always.is_empty() || self.sched.checking {
            return Ok(());
        }
        self.sched.checking = true;
        let cur = self.sched.cur as usize;
        let pending = self.sched.tasks.get_mut(cur).and_then(|t| t.pending.take());
        let preds = self.sched.always.clone();
        let mut result = Ok(());
        for (f, what) in preds {
            let r =
                self.callee_of(&f, Vec::new()).and_then(|(fi, vals, tys)| self.call_fn_tys(fi, vals, tys));
            match r {
                Ok(Value::Bool(true)) => {}
                Ok(_) => {
                    result = Err(Unwind::Abort(render(msg::SIM_ALWAYS_FAILED, &[("what", what)])));
                    break;
                }
                Err(_) if std::mem::take(&mut self.sched.check_skipped) => break,
                Err(e) => {
                    result = Err(e);
                    break;
                }
            }
        }
        if let Some(t) = self.sched.tasks.get_mut(cur) {
            t.pending = pending;
        }
        self.sched.checking = false;
        result
    }

    /// `sim.always(what, pred, body)`: `body()` above an anchor, with
    /// `pred` checked at every switch until it returns, and once more then.
    pub(crate) fn open_always(&mut self, mut args: Vec<Value>, dest: &'p sir::Place) -> Result<(), Unwind> {
        if args.len() != 3 {
            return Err(ice("sim.always takes a description, a predicate and a body"));
        }
        let body = args.pop().unwrap_or(Value::Unit);
        let pred = args.pop().unwrap_or(Value::Unit);
        let what =
            self.text(args.first().ok_or_else(|| ice("sim.always without a description"))?)?.to_string();
        let (fi, vals, tys) = self.callee_of(&body, Vec::new())?;
        let anchor = self.anchor_frame(Anchor::Always, Some(dest));
        let fr = self.make_frame(fi, vals, Some(&RESULT_SLOT), tys)?;
        self.sched.always.push((pred, what));
        self.check_always()?;
        self.push_built(anchor)?;
        self.push_built(fr)
    }

    /// Nothing is ready: advance the clocks to the next timer and wake what
    /// is due. Returns whether anything woke.
    fn advance_time(&mut self) -> Result<bool, Unwind> {
        if self.sched.timers.is_empty() {
            return Ok(false);
        }
        // Virtual clocks share one flow of time: they all advance by the
        // smallest wait among their timers; the host's real clock is
        // waited for.
        let mut virt: Option<i128> = None;
        let mut real: Option<i128> = None;
        for &(c, at, _) in &self.sched.timers {
            let delta = at - self.clock_now(c);
            let slot = if self.sched.clocks.get(c as usize).is_some_and(|k| k.virtual_) {
                &mut virt
            } else {
                &mut real
            };
            if slot.is_none_or(|d| delta < d) {
                *slot = Some(delta);
            }
        }
        if let Some(d) = virt
            && d > 0
        {
            for k in self.sched.clocks.iter_mut().filter(|k| k.virtual_) {
                k.now += d;
            }
        } else if virt.is_none()
            && let Some(d) = real
            && d > 0
        {
            self.host.sleep_nanos(d.min(i128::from(u64::MAX)) as u64);
        }
        Ok(self.wake_due_timers())
    }

    /// Wakes the tasks whose timers are due; returns whether any was.
    pub(crate) fn wake_due_timers(&mut self) -> bool {
        let mut due = Vec::new();
        let timers = std::mem::take(&mut self.sched.timers);
        for (c, at, t) in timers {
            if self.clock_now(c) >= at {
                due.push(t);
            } else {
                self.sched.timers.push((c, at, t));
            }
        }
        let any = !due.is_empty();
        for t in due {
            self.wake(t);
        }
        any
    }

    fn deadlock(&self) -> Unwind {
        let mut lines = vec![render(msg::RUN_DEADLOCK, &[])];
        for (i, t) in self.sched.tasks.iter().enumerate() {
            let TState::Blocked(w) = &t.state else { continue };
            let who = if i == 0 {
                render(msg::RUN_TASK_ROOT, &[])
            } else {
                render(msg::RUN_TASK_NAMED, &[("task", t.path.clone())])
            };
            let what = match w {
                Wait::Join(x) => {
                    format!(
                        "join task {}",
                        self.sched.tasks.get(*x as usize).map(|t| t.path.as_str()).unwrap_or("?")
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
            let location = self
                .frames_of(i as u32)
                .iter()
                .rev()
                .find(|f| f.func.is_some())
                .and_then(|f| {
                    let func = self.func(f.func?)?;
                    let file = self.prog.sources.get(func.source as usize)?;
                    let l = f.loc?;
                    Some(format!("{file}:{}:{}", l.line, l.col))
                })
                .unwrap_or_default();
            lines.push(format!(
                "  {}",
                render(msg::RUN_DEADLOCK_TASK, &[("task", who), ("wait", what), ("location", location)])
            ));
        }
        Unwind::Abort(lines.join("\n"))
    }

    // ---------------------------------------------------------------- clocks

    pub(crate) fn clock_now(&self, c: u32) -> i128 {
        match self.sched.clocks.get(c as usize) {
            Some(k) if k.virtual_ => k.now,
            Some(_) => self.host.now_nanos(),
            None => 0,
        }
    }

    // ---------------------------------------------------------------- intrinsics

    /// The intrinsics of tasks, channels and timers; `None` for any other.
    pub(crate) fn task_intrinsic(
        &mut self,
        name: &str,
        tys: &[sir::Type],
        args: &mut Vec<Value>,
    ) -> Option<Result<Value, Unwind>> {
        let r = match name {
            "Scope.spawn" => self.spawn(args, false),
            "Scope.spawn_with" => self.spawn(args, true),
            "Scope.cancel" => self.id_of(&args[0]).map(|s| {
                if let Some(r) = self.sched.scopes.get_mut(s as usize) {
                    r.cancelled = true;
                }
                self.wake_all_blocked();
                Value::Unit
            }),
            "Scope.is_cancelled" => {
                self.id_of(&args[0]).map(|s| Value::Bool(self.node_cancelled(Some(Node::Scope(s)))))
            }
            "Scope.cancelled" => self.id_of(&args[0]).map(Value::Cancellation),
            "Cancellation.wait" => self.cancellation_wait(args),
            "Handle.join" => self.join(args),
            "Handle.is_finished" => self.id_of(&args[0]).map(|t| {
                Value::Bool(matches!(self.sched.tasks.get(t as usize).map(|t| &t.state), Some(TState::Done)))
            }),
            "task.yield_now" => {
                self.sched.yield_asked = true;
                Ok(Value::Unit)
            }
            // A value the simulation chooses (17): one choice point of the
            // exhaustive search, or a seeded draw.
            "sim.choose" => match args.first() {
                Some(Value::Int(n)) if *n >= 1 => {
                    let n = (*n).min(i64::from(u32::MAX)) as u32;
                    Ok(Value::Int(i64::from(self.sched.choose(n, false))))
                }
                _ => Err(panic(msg::SIM_CHOOSE_NONE, &[])),
            },
            "task.is_cancelled" => Ok(Value::Bool(self.cancelled_now())),
            "task.checkpoint" => {
                Ok(if self.cancelled_now() { self.cancelled_err() } else { Value::ok(Value::Unit) })
            }
            "channel" => self.channel(tys, args),
            "Sender.send" => self.send(args),
            "Sender.try_send" => self.try_send(args),
            "Sender.close" => self.id_of(&args[0]).map(|c| {
                if let Some(ch) = self.sched.channels.get_mut(c as usize) {
                    ch.closed = true;
                }
                self.wake_key(Key::Chan(c));
                Value::Unit
            }),
            "Receiver.recv" => self.recv(args),
            "Receiver.try_recv" => self.try_recv(args),
            "clock.after" => self.clock_after(args),
            "Timer.wait" => self.timer_wait(args),
            "select.wait" => self.select_wait(args),
            "log.write" => self.log_write(args),
            "gen.new" => self.gen_new(args),
            "gen.yield" => self.gen_yield(args),
            _ => return None,
        };
        Some(r)
    }

    fn join(&mut self, args: &mut Vec<Value>) -> Result<Value, Unwind> {
        let t = self.id_of(&args[0])?;
        if self.cancelled_now() {
            return Ok(self.cancelled_err());
        }
        if matches!(self.sched.tasks.get(t as usize).map(|t| &t.state), Some(TState::Done)) {
            return self.take_outcome(t);
        }
        Err(Unwind::Block(Box::new(Blocked {
            args: std::mem::take(args),
            wait: Wait::Join(t),
            keys: vec![Key::Task(t)],
            timer: None,
        })))
    }

    fn cancellation_wait(&mut self, args: &mut Vec<Value>) -> Result<Value, Unwind> {
        let s = self.id_of(&args[0])?;
        if self.node_cancelled(Some(Node::Scope(s))) {
            return Ok(Value::Unit);
        }
        Err(Unwind::Block(Box::new(Blocked {
            args: std::mem::take(args),
            wait: Wait::Cancellation,
            keys: Vec::new(),
            timer: self.chain_deadline(Some(Node::Scope(s))),
        })))
    }

    // ---------------------------------------------------------------- channels

    fn channel(&mut self, tys: &[sir::Type], args: &[Value]) -> Result<Value, Unwind> {
        let cap = self.int(&args[0])?;
        if cap < 1 {
            return Err(panic(msg::PANIC_CHANNEL_CAPACITY, &[("capacity", cap.to_string())]));
        }
        let c = self.sched.channels.len() as u32;
        self.sched.channels.push(Channel {
            buf: std::collections::VecDeque::new(),
            cap: cap as usize,
            closed: false,
            senders: 1,
            receivers: 1,
            elem: tys.first().cloned(),
        });
        Ok(Value::Tuple(vec![Value::Sender(c), Value::Receiver(c)]))
    }

    fn chan(&self, c: u32) -> Result<&Channel, Unwind> {
        self.sched.channels.get(c as usize).ok_or_else(|| ice("no such channel"))
    }

    fn send(&mut self, args: &mut Vec<Value>) -> Result<Value, Unwind> {
        let c = self.id_of(&args[0])?;
        if self.cancelled_now() {
            let v = std::mem::replace(&mut args[1], Value::Moved);
            let ty = self.chan(c)?.elem.clone();
            self.queue_drop(v, ty);
            return Ok(self.cancelled_err());
        }
        let ch = self.chan(c)?;
        if ch.receivers == 0 || ch.closed {
            let ty = ch.elem.clone();
            let v = std::mem::replace(&mut args[1], Value::Moved);
            self.queue_drop(v, ty);
            return Ok(self.closed_err());
        }
        if ch.buf.len() < ch.cap {
            let v = std::mem::replace(&mut args[1], Value::Moved);
            if let Some(ch) = self.sched.channels.get_mut(c as usize) {
                ch.buf.push_back(v);
            }
            self.wake_key(Key::Chan(c));
            return Ok(Value::ok(Value::Unit));
        }
        Err(Unwind::Block(Box::new(Blocked {
            args: std::mem::take(args),
            wait: Wait::Send,
            keys: vec![Key::Chan(c)],
            timer: None,
        })))
    }

    fn try_send(&mut self, args: &mut [Value]) -> Result<Value, Unwind> {
        let c = self.id_of(&args[0])?;
        let ch = self.chan(c)?;
        let v = std::mem::replace(&mut args[1], Value::Moved);
        if ch.receivers == 0 || ch.closed || ch.buf.len() >= ch.cap {
            return Ok(Value::some(v));
        }
        if let Some(ch) = self.sched.channels.get_mut(c as usize) {
            ch.buf.push_back(v);
        }
        self.wake_key(Key::Chan(c));
        Ok(Value::none())
    }

    /// The next value of a channel if there is one now: `Some(Ok(v))`,
    /// `Some(Err(Closed))` when it is closed and drained, `None` when the
    /// receiver would wait.
    pub(crate) fn recv_now(&mut self, c: u32) -> Result<Option<Value>, Unwind> {
        let ch = self.chan(c)?;
        if !ch.buf.is_empty() {
            let v = self.sched.channels.get_mut(c as usize).and_then(|ch| ch.buf.pop_front());
            self.wake_key(Key::Chan(c));
            return Ok(v.map(Value::ok));
        }
        if ch.closed || ch.senders == 0 {
            return Ok(Some(self.closed_err()));
        }
        Ok(None)
    }

    fn recv(&mut self, args: &mut Vec<Value>) -> Result<Value, Unwind> {
        let c = self.id_of(&args[0])?;
        if self.cancelled_now() {
            return Ok(self.cancelled_err());
        }
        if let Some(v) = self.recv_now(c)? {
            return Ok(v);
        }
        Err(Unwind::Block(Box::new(Blocked {
            args: std::mem::take(args),
            wait: Wait::Recv,
            keys: vec![Key::Chan(c)],
            timer: None,
        })))
    }

    fn try_recv(&mut self, args: &[Value]) -> Result<Value, Unwind> {
        let c = self.id_of(&args[0])?;
        Ok(match self.recv_now(c)? {
            Some(v) if !is_err(&v) => match v {
                Value::Variant(a) => Value::some(a.fields.into_iter().next().unwrap_or(Value::Unit)),
                other => Value::some(other),
            },
            _ => Value::none(),
        })
    }

    /// `Iter.next` on a receiver's iterator: the next value, the end when
    /// the channel is closed and drained or the code is cancelled, or a
    /// wait.
    pub(crate) fn recv_next(&mut self, c: u32, args: Vec<Value>) -> Result<Value, Unwind> {
        if self.cancelled_now() {
            return Ok(Value::none());
        }
        match self.recv_now(c)? {
            Some(v) if is_err(&v) => Ok(Value::none()),
            Some(Value::Variant(a)) => Ok(Value::some(a.fields.into_iter().next().unwrap_or(Value::Unit))),
            Some(other) => Ok(Value::some(other)),
            None => Err(Unwind::Block(Box::new(Blocked {
                args,
                wait: Wait::Recv,
                keys: vec![Key::Chan(c)],
                timer: None,
            }))),
        }
    }

    /// A sender or receiver dropped.
    pub(crate) fn drop_channel_end(&mut self, v: &Value) {
        let (c, sender) = match v {
            Value::Sender(c) => (*c, true),
            Value::Receiver(c) => (*c, false),
            _ => return,
        };
        let Some(ch) = self.sched.channels.get_mut(c as usize) else { return };
        if sender {
            ch.senders = ch.senders.saturating_sub(1);
        } else {
            ch.receivers = ch.receivers.saturating_sub(1);
        }
        let gone = ch.receivers == 0;
        let ty = ch.elem.clone();
        let left: Vec<Value> = if gone { ch.buf.drain(..).collect() } else { Vec::new() };
        for x in left {
            self.queue_drop(x, ty.clone());
        }
        self.wake_key(Key::Chan(c));
    }

    /// `.copy()` of a channel end: another sender (or receiver).
    pub(crate) fn copy_channel_end(&mut self, v: &Value) {
        let (c, sender) = match v {
            Value::Sender(c) => (*c, true),
            Value::Receiver(c) => (*c, false),
            _ => return,
        };
        if let Some(ch) = self.sched.channels.get_mut(c as usize) {
            if sender {
                ch.senders += 1;
            } else {
                ch.receivers += 1;
            }
        }
    }

    // ---------------------------------------------------------------- timers

    fn clock_after(&mut self, args: &[Value]) -> Result<Value, Unwind> {
        let c = self.id_of(&args[0])?;
        let d = self.duration_nanos(&args[1])?;
        Ok(Value::Timer { clock: c, at: self.clock_now(c) + d })
    }

    /// The nanoseconds of a `Duration`.
    pub(crate) fn duration_nanos(&self, v: &Value) -> Result<i128, Unwind> {
        match self.target(v)? {
            Value::Struct(a) => match a.fields.first() {
                Some(n) => Ok(i128::from(self.int(n)?)),
                None => Err(ice("a duration without nanoseconds")),
            },
            other => Err(ice(format!("expected a Duration, found {other:?}"))),
        }
    }

    fn timer_wait(&mut self, args: &mut Vec<Value>) -> Result<Value, Unwind> {
        let (c, at) = match self.target(&args[0])? {
            Value::Timer { clock, at } => (*clock, *at),
            other => return Err(ice(format!("Timer.wait on {other:?}"))),
        };
        if self.cancelled_now() {
            return Ok(self.cancelled_err());
        }
        if self.clock_now(c) >= at {
            return Ok(Value::ok(Value::Unit));
        }
        Err(Unwind::Block(Box::new(Blocked {
            args: std::mem::take(args),
            wait: Wait::Timer,
            keys: Vec::new(),
            timer: Some((c, at)),
        })))
    }

    // ---------------------------------------------------------------- select

    /// `select.wait(kind, subject, ...)`: the index of the arm to take
    /// (06, "Channels and select"). An arm is ready when its operation
    /// would not wait; among ready arms the seeded generator chooses. When
    /// the running code is cancelled, a ready cancellation arm is taken,
    /// else the first arm whose operation reports `Cancelled` (a receive,
    /// a send or a join), which then returns `Err(Cancelled)`.
    fn select_wait(&mut self, args: &mut Vec<Value>) -> Result<Value, Unwind> {
        let mut arms = Vec::new();
        for pair in args.chunks(2) {
            let kind = self.int(&pair[0])?;
            arms.push((kind, pair.get(1).cloned().unwrap_or(Value::Unit)));
        }
        let cancelled = self.cancelled_now();
        let mut ready = Vec::new();
        let mut keys = Vec::new();
        let mut timer: Option<(u32, i128)> = None;
        let mut first_reporting = None;
        for (i, (kind, subject)) in arms.iter().enumerate() {
            let ok = match kind {
                // A receive.
                0 => {
                    let c = self.id_of(subject)?;
                    keys.push(Key::Chan(c));
                    let ch = self.chan(c)?;
                    !ch.buf.is_empty() || ch.closed || ch.senders == 0
                }
                // A send.
                1 => {
                    let c = self.id_of(subject)?;
                    keys.push(Key::Chan(c));
                    let ch = self.chan(c)?;
                    ch.buf.len() < ch.cap || ch.receivers == 0 || ch.closed
                }
                // A join.
                2 => {
                    let t = self.id_of(subject)?;
                    keys.push(Key::Task(t));
                    matches!(self.sched.tasks.get(t as usize).map(|t| &t.state), Some(TState::Done))
                }
                // A timer.
                3 => match self.target(subject)? {
                    Value::Timer { clock, at } => {
                        if timer.is_none_or(|(_, x)| *at < x) {
                            timer = Some((*clock, *at));
                        }
                        self.clock_now(*clock) >= *at
                    }
                    other => return Err(ice(format!("a timer arm on {other:?}"))),
                },
                // A scope's cancellation.
                _ => {
                    let s = self.id_of(subject)?;
                    let c = self.node_cancelled(Some(Node::Scope(s)));
                    if c && cancelled {
                        return Ok(Value::Int(i as i64));
                    }
                    c
                }
            };
            if first_reporting.is_none() && matches!(kind, 0..=2) {
                first_reporting = Some(i);
            }
            if ok {
                ready.push(i);
            }
        }
        if cancelled && let Some(i) = first_reporting {
            return Ok(Value::Int(i as i64));
        }
        if !ready.is_empty() {
            let k = self.sched.rng.below(ready.len());
            return Ok(Value::Int(ready[k] as i64));
        }
        Err(Unwind::Block(Box::new(Blocked { args: std::mem::take(args), wait: Wait::Select, keys, timer })))
    }

    // ---------------------------------------------------------------- generators

    /// `gen.new(body)`: a generator, a task that runs `body` only while
    /// its consumer waits for the next item, and stops at each `yield`.
    fn gen_new(&mut self, args: &mut [Value]) -> Result<Value, Unwind> {
        let f = std::mem::replace(
            args.get_mut(0).ok_or_else(|| ice("a generator without a body"))?,
            Value::Moved,
        );
        let t = self.sched.tasks.len() as u32;
        let anchor = self.anchor_frame(Anchor::Task, None);
        let (fr, _) = self.closure_frame(f, Vec::new(), Some(&RESULT_SLOT))?;
        let path = self.sched.tasks.get(self.sched.cur as usize).map(|t| t.path.clone()).unwrap_or_default();
        self.sched.tasks.push(Task {
            frames: vec![anchor, fr],
            state: TState::Blocked(Wait::Join(t)),
            scope: None,
            index: 0,
            pending: None,
            outcome: None,
            taken: false,
            cancelled: false,
            deadline: None,
            path,
            spawned: 0,
            returns_result: false,
            ret_ty: None,
            ready_at: 0,
            ready_wall: None,
            yielded: None,
            generator: true,
            // A generator runs as part of its consumer's iterations.
            epoch: self.task_epoch(),
        });
        Ok(Value::Iter(Box::new(crate::value::IterState::Gen(t))))
    }

    /// The item a generator has yielded, taken.
    pub(crate) fn gen_take(&mut self, t: u32) -> Result<Option<Value>, Unwind> {
        Ok(self.sched.tasks.get_mut(t as usize).and_then(|g| g.yielded.take()))
    }

    pub(crate) fn gen_done(&self, t: u32) -> bool {
        self.sched.tasks.get(t as usize).is_some_and(|g| matches!(g.state, TState::Done))
    }

    /// `Iter.next` on a generator: its next item, its end, or (when its
    /// body panicked) the panic, in the consumer. Otherwise the generator
    /// runs until it yields, and the consumer waits for it.
    pub(crate) fn gen_next(&mut self, t: u32, args: Vec<Value>) -> Result<Value, Unwind> {
        if let Some(v) = self.gen_take(t)? {
            return Ok(Value::some(v));
        }
        if self.gen_done(t) {
            return match self.sched.tasks.get_mut(t as usize).and_then(|g| g.outcome.take()) {
                Some(Outcome::Panic(p)) => Err(Unwind::Panic(p)),
                _ => Ok(Value::none()),
            };
        }
        self.wake(t);
        Err(Unwind::Block(Box::new(Blocked {
            args,
            wait: Wait::Join(t),
            keys: vec![Key::Task(t)],
            timer: None,
        })))
    }

    /// `yield v` in a generator's task: hands `v` to the consumer and
    /// waits to be asked for the next. Retried with no argument once it
    /// is, which goes on.
    fn gen_yield(&mut self, args: &mut Vec<Value>) -> Result<Value, Unwind> {
        let Some(v) = args.pop() else { return Ok(Value::Unit) };
        let t = self.sched.cur;
        if let Some(g) = self.sched.tasks.get_mut(t as usize) {
            g.yielded = Some(v);
        }
        self.wake_key(Key::Task(t));
        Err(Unwind::Block(Box::new(Blocked {
            args: Vec::new(),
            wait: Wait::Select,
            keys: Vec::new(),
            timer: None,
        })))
    }

    // ---------------------------------------------------------------- log

    fn log_write(&mut self, args: &[Value]) -> Result<Value, Unwind> {
        let level = self.text(&args[0])?.to_string();
        let message = self.text(&args[1])?.to_string();
        let path = self.sched.tasks.get(self.sched.cur as usize).map(|t| t.path.clone()).unwrap_or_default();
        let line = if path.is_empty() {
            render(msg::RUN_LOG_RECORD, &[("level", level), ("message", message)])
        } else {
            render(msg::RUN_LOG_RECORD_TASK, &[("level", level), ("task", path), ("message", message)])
        };
        self.host.stderr(&format!("{line}\n"));
        Ok(Value::Unit)
    }

    // ---------------------------------------------------------------- mutexes

    /// `Mutex.lock` across tasks: waits while another task holds it.
    pub(crate) fn lock(&mut self, args: &mut Vec<Value>) -> Result<Value, Unwind> {
        let p = self.pointer_of(&args[0])?;
        let cur = self.sched.cur;
        let wait = match self.write(&p)? {
            Value::Mutex(m) if m.poisoned => return Err(panic(msg::PANIC_MUTEX_POISONED, &[])),
            Value::Mutex(m) if m.locked && m.holder == cur => {
                // A `sim.always` predicate on the stack of a task inside a
                // critical section cannot see the state it protects: the
                // check waits for the next switch.
                if self.sched.checking {
                    self.sched.check_skipped = true;
                }
                return Err(panic(msg::PANIC_MUTEX_HELD, &[]));
            }
            Value::Mutex(m) if m.locked => true,
            Value::Mutex(m) => {
                m.locked = true;
                m.holder = cur;
                false
            }
            other => return Err(ice(format!("Mutex.lock on {other:?}"))),
        };
        if wait {
            return Err(Unwind::Block(Box::new(Blocked {
                args: std::mem::take(args),
                wait: Wait::Lock,
                keys: vec![Key::Lock],
                timer: None,
            })));
        }
        Ok(Value::Guard(Box::new(p)))
    }

    /// The tasks a `Shared` cell was touched by: under the monitor, a cell
    /// that the escape analysis tagged task-local and that another task
    /// reaches is a soundness error (18, Phase 6).
    pub(crate) fn note_shared_access(&mut self, cell: u32) -> Result<(), Unwind> {
        let cur = self.sched.cur;
        let Some(c) = self.heap.get_mut(cell as usize) else { return Ok(()) };
        if c.creator == cur || c.crossed {
            return Ok(());
        }
        c.crossed = true;
        let site = c.site;
        self.sched.stats.shared_crossed += 1;
        if let Some(s) = site.and_then(|i| self.sched.stats.sites.get_mut(i as usize)) {
            s.crossed += 1;
        }
        if !c.atomic && self.monitoring() {
            return Err(self.mon_shared_crossed(cell));
        }
        Ok(())
    }

    /// A task's trace path, for display.
    pub(crate) fn task_path(&self, t: u32) -> String {
        self.sched.tasks.get(t as usize).map(|t| t.path.clone()).unwrap_or_default()
    }

    /// The `--stats` report.
    pub(crate) fn stats_report(&self) -> String {
        match self.stats_kind {
            crate::StatsKind::Summary => self.stats_summary(),
            crate::StatsKind::StackDepth => self.stats_depth(),
            crate::StatsKind::Shared => self.stats_shared(),
        }
    }

    /// `--stats stack-depth`: the histogram of frame depths at calls, by
    /// powers of two, and the deepest stack with its estimated native size.
    fn stats_depth(&self) -> String {
        let s = &self.sched.stats;
        let calls: u64 = s.depths.iter().sum();
        let mut out = render(
            msg::RUN_STATS_DEPTH,
            &[
                ("calls", calls.to_string()),
                ("max", s.max_depth.to_string()),
                ("bytes", s.max_stack_bytes.to_string()),
            ],
        );
        out.push('\n');
        for (k, n) in s.depths.iter().enumerate() {
            if *n == 0 {
                continue;
            }
            let (lo, hi) = (1u64 << k, (1u64 << (k + 1)) - 1);
            out.push_str(&render(
                msg::RUN_STATS_DEPTH_BUCKET,
                &[("from", lo.to_string()), ("to", hi.to_string()), ("calls", n.to_string())],
            ));
            out.push('\n');
        }
        out
    }

    /// `--stats shared`: every `Shared` construction site of the program's
    /// own code (not the standard library's), with the escape analysis's
    /// tag, and the cells each made and that another task reached; the
    /// share of the sites tagged task-local is Q-2's number. A site that
    /// never ran is listed with no cells.
    fn stats_shared(&self) -> String {
        let prog = self.prog;
        let mut sites: Vec<SharedSite> = Vec::new();
        for f in &prog.functions {
            let std = prog.sources.get(f.source as usize).is_some_and(|n| n.starts_with("<std>/"));
            if std {
                continue;
            }
            for b in &f.blocks {
                for st in &b.stmts {
                    let sisu_sir::StatementKind::Call { callee: sisu_sir::Callee::Intrinsic(n, _), .. } =
                        &st.kind
                    else {
                        continue;
                    };
                    let atomic = match n.as_str() {
                        "Shared.new" => false,
                        "Shared.new_atomic" => true,
                        _ => continue,
                    };
                    let ran = self
                        .sched
                        .stats
                        .sites
                        .iter()
                        .find(|s| s.function == f.name && s.loc == st.loc && s.source == f.source);
                    sites.push(SharedSite {
                        function: f.name.clone(),
                        loc: st.loc,
                        source: f.source,
                        atomic,
                        cells: ran.map(|s| s.cells).unwrap_or(0),
                        crossed: ran.map(|s| s.crossed).unwrap_or(0),
                    });
                }
            }
        }
        sites.sort_by_key(|s| (s.source, s.loc));
        let local = sites.iter().filter(|s| !s.atomic).count();
        let mut out = render(
            msg::RUN_STATS_SHARED_SITES,
            &[
                ("local", local.to_string()),
                ("total", sites.len().to_string()),
                ("crossing", (sites.len() - local).to_string()),
            ],
        );
        out.push('\n');
        for s in &sites {
            let file = prog.sources.get(s.source as usize).cloned().unwrap_or_default();
            let at = s.loc.map(|l| format!("{file}:{}:{}", l.line, l.col)).unwrap_or(file);
            let tag =
                render(if s.atomic { msg::RUN_STATS_TAG_CROSSING } else { msg::RUN_STATS_TAG_LOCAL }, &[]);
            out.push_str(&render(
                msg::RUN_STATS_SHARED_SITE,
                &[
                    ("at", at),
                    ("function", crate::exec::display_fn_name(&s.function, prog)),
                    ("tag", tag),
                    ("cells", s.cells.to_string()),
                    ("crossed", s.crossed.to_string()),
                ],
            ));
            out.push('\n');
        }
        out
    }

    /// The summary `--stats` prints: tasks, scheduling latency, `Shared`
    /// cells by tag.
    fn stats_summary(&self) -> String {
        let s = &self.sched.stats;
        let pct = |v: &[u64], p: f64| -> u64 {
            if v.is_empty() {
                return 0;
            }
            let mut v = v.to_vec();
            v.sort_unstable();
            let i = ((v.len() as f64 - 1.0) * p).round() as usize;
            v[i.min(v.len() - 1)]
        };
        let lines = [
            render(
                msg::RUN_STATS_TASKS,
                &[
                    ("tasks", s.tasks.to_string()),
                    ("max", s.max_live.to_string()),
                    ("switches", s.switches.to_string()),
                ],
            ),
            render(
                msg::RUN_STATS_LATENCY,
                &[
                    ("p50", pct(&s.latency_steps, 0.5).to_string()),
                    ("p99", pct(&s.latency_steps, 0.99).to_string()),
                    ("max", s.latency_steps.iter().max().copied().unwrap_or(0).to_string()),
                    ("p50us", pct(&s.latency_micros, 0.5).to_string()),
                    ("p99us", pct(&s.latency_micros, 0.99).to_string()),
                ],
            ),
            render(
                msg::RUN_STATS_SHARED,
                &[
                    ("local", s.shared_local.to_string()),
                    ("crossing", s.shared_crossing.to_string()),
                    ("crossed", s.shared_crossed.to_string()),
                ],
            ),
        ];
        let mut out = String::new();
        for l in lines {
            out.push_str(&l);
            out.push('\n');
        }
        out
    }
}
