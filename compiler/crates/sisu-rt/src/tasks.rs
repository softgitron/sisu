//! The intrinsics of tasks, scopes, channels, timers, `select`,
//! generators, `log` and `Mutex` (06; `sisu-interp/src/sched.rs`), with the
//! interpreter's results, messages and order of effects. Each runs on the
//! task's own stack only for what must: waiting ([`sched::blocking`]) and
//! calling Sisu code (a scope's body); the rest is on the system stack.

use crate::abi::{self, Intrinsic, Kind};
use crate::intrinsics::Call;
use crate::mem;
use crate::rt::{self, ty};
use crate::sched::{self, Anchor, Attempt, Key, Node, Outcome, ScopeRec, Site, TState, Wait, system, with};
use crate::value;
use corosensei::Yielder;
use sisu_diag::msg;

/// A closure's code called with its environment and no argument.
type Body0 = unsafe extern "C" fn(*mut u8, *mut u8) -> u8;
/// A closure's code called with its environment and one argument.
type Body1 = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8) -> u8;

/// Whether the intrinsic is this module's.
pub(crate) fn handles(i: Intrinsic) -> bool {
    use Intrinsic as I;
    matches!(
        i,
        I::TaskScope
            | I::TaskRace
            | I::TaskShield
            | I::TaskYieldNow
            | I::TaskIsCancelled
            | I::TaskCheckpoint
            | I::ScopeSpawn
            | I::ScopeSpawnWith
            | I::ScopeCancel
            | I::ScopeIsCancelled
            | I::ScopeCancelled
            | I::HandleJoin
            | I::HandleIsFinished
            | I::CancellationWait
            | I::Channel
            | I::SenderSend
            | I::SenderTrySend
            | I::SenderClose
            | I::ReceiverRecv
            | I::ReceiverTryRecv
            | I::ClockAfter
            | I::TimerWait
            | I::SelectWait
            | I::LogWrite
            | I::GenNew
            | I::GenYield
            | I::MutexNew
            | I::MutexLock
            | I::MutexIsPoisoned
            | I::MutexIntoInner
            | I::IterNext
    )
}

/// A call of one of this module's intrinsics, on the task's stack.
///
/// # Safety
/// As `sisu_rt_intrinsic`.
pub(crate) unsafe fn call(i: Intrinsic, c: &Call<'_>) -> u8 {
    use Intrinsic as I;
    // SAFETY: each reads its arguments as their types say.
    unsafe {
        match i {
            I::TaskScope => scope(c, false),
            I::TaskRace => scope(c, true),
            I::TaskShield => shield(c),
            I::TaskYieldNow => {
                sched::yield_now();
                abi::OK
            }
            I::TaskIsCancelled => {
                let v = system(|| with(|s| s.cancelled_now()));
                c.set_bool(v);
                abi::OK
            }
            I::TaskCheckpoint => system(|| {
                if with(|s| s.cancelled_now()) {
                    write_task_error(c.ret, c.ret_ty, "Cancelled", None);
                } else {
                    mem::set_word(c.ret, 0, 0);
                }
                abi::OK
            }),
            I::ScopeSpawn => system(|| spawn(c, false)),
            I::ScopeSpawnWith => system(|| spawn(c, true)),
            I::ScopeCancel => {
                let s = c.word(0) as u32;
                system(|| {
                    with(|sc| {
                        if let Some(r) = sc.scopes.get_mut(s as usize) {
                            r.cancelled = true;
                        }
                        sc.wake_all_blocked();
                    })
                });
                abi::OK
            }
            I::ScopeIsCancelled => {
                let s = c.word(0) as u32;
                let v = system(|| with(|sc| sc.node_cancelled(Some(Node::Scope(s)))));
                c.set_bool(v);
                abi::OK
            }
            I::ScopeCancelled => {
                c.set_word(c.word(0));
                abi::OK
            }
            I::HandleJoin => join(c),
            I::HandleIsFinished => {
                let t = c.word(0) as u32;
                let v = system(|| {
                    with(|s| s.tasks.get(t as usize).is_some_and(|t| matches!(t.state, TState::Done)))
                });
                c.set_bool(v);
                abi::OK
            }
            I::CancellationWait => {
                let s = c.word(0) as u32;
                let site = Site::current();
                sched::blocking(|| {
                    with(|sc| {
                        if sc.node_cancelled(Some(Node::Scope(s))) {
                            return Attempt::Done(abi::OK);
                        }
                        let timer = sc.chain_deadline(Some(Node::Scope(s)));
                        sc.block_on(Wait::Cancellation, &[], timer, site);
                        Attempt::Wait
                    })
                })
            }
            I::Channel => system(|| channel(c)),
            I::SenderSend => send(c),
            I::SenderTrySend => sched::system_status(|| try_send(c)),
            I::SenderClose => {
                let ch = c.word(0) as u32;
                system(|| {
                    with(|s| {
                        if let Some(x) = s.channels.get_mut(ch as usize) {
                            x.closed = true;
                        }
                        s.wake_key(Key::Chan(ch));
                    })
                });
                abi::OK
            }
            I::ReceiverRecv => recv(c),
            I::ReceiverTryRecv => system(|| {
                let ch = c.word(0) as u32;
                let elem = elem_of(c.ty(0));
                match with(|s| s.recv_now(ch)) {
                    Some(Ok(v)) => write_some(c.ret, &v, ty(elem).size),
                    _ => mem::set_word(c.ret, 0, 1),
                }
                abi::OK
            }),
            I::ClockAfter => system(|| {
                let clock = clock_of(c);
                let (d, _) = c.place(1);
                let nanos = mem::word(d, 0) as i64;
                let at = with(|s| s.clock_now(clock)) + i128::from(nanos);
                mem::set_word(c.ret, 0, u64::from(clock));
                mem::set_word(c.ret, 8, at as i64 as u64);
                abi::OK
            }),
            I::TimerWait => {
                let (p, _) = c.place(0);
                let (clock, at) = (mem::word(p, 0) as u32, i128::from(mem::word(p, 8) as i64));
                let site = Site::current();
                sched::blocking(|| {
                    with(|s| {
                        if s.cancelled_now() {
                            write_task_error(c.ret, c.ret_ty, "Cancelled", None);
                            return Attempt::Done(abi::OK);
                        }
                        if s.clock_now(clock) >= at {
                            mem::set_word(c.ret, 0, 0);
                            return Attempt::Done(abi::OK);
                        }
                        s.block_on(Wait::Timer, &[], Some((clock, at)), site);
                        Attempt::Wait
                    })
                })
            }
            I::SelectWait => select(c),
            I::LogWrite => system(|| {
                let level = String::from_utf8_lossy(c.text(0)).into_owned();
                let message = String::from_utf8_lossy(c.text(1)).into_owned();
                let path = with(|s| s.tasks.get(s.cur as usize).map(|t| t.path.clone()).unwrap_or_default());
                let line = if path.is_empty() {
                    rt::render(msg::RUN_LOG_RECORD, &[("level", level), ("message", message)])
                } else {
                    rt::render(
                        msg::RUN_LOG_RECORD_TASK,
                        &[("level", level), ("task", path), ("message", message)],
                    )
                };
                rt::stderr(&format!("{line}\n"));
                abi::OK
            }),
            I::GenNew => system(|| gen_new(c)),
            I::GenYield => gen_yield(c),
            I::MutexNew => system(|| {
                let (_, t) = (c.arg(0), c.ty(0));
                mem::set_word(c.ret, abi::MUTEX_LOCKED, 0);
                mem::set_word(c.ret, abi::MUTEX_HOLDER, 0);
                mem::set_word(c.ret, abi::MUTEX_POISONED, 0);
                mem::copy(c.ret.add(abi::MUTEX_HEADER as usize), c.arg(0), ty(t).size as usize);
                abi::OK
            }),
            I::MutexLock => lock(c),
            I::MutexIsPoisoned => {
                let (p, _) = c.place(0);
                c.set_bool(mem::word(p, abi::MUTEX_POISONED) != 0);
                abi::OK
            }
            I::MutexIntoInner => sched::system_status(|| {
                let (p, t) = (c.arg(0), c.ty(0));
                let Kind::Mutex(v) = ty(t).kind else {
                    rt::internal("`into_inner` of something not a mutex")
                };
                let value = p.add(abi::MUTEX_HEADER as usize);
                if mem::word(p, abi::MUTEX_POISONED) != 0 {
                    let st = value::drop_value(value, v);
                    let raised = rt::raise_msg(msg::PANIC_INTO_INNER_POISONED, &[]);
                    return value::after_part(raised, st);
                }
                mem::copy(c.ret, value, ty(v).size as usize);
                abi::OK
            }),
            I::IterNext => iter_next(c),
            _ => rt::internal("an intrinsic outside the task module"),
        }
    }
}

// ------------------------------------------------------------------ values

/// The element type a channel end's type names, through a reference.
fn elem_of(t: u32) -> u32 {
    match ty(t).kind {
        Kind::Ref(inner) => elem_of(inner),
        Kind::Sender(e) | Kind::Receiver(e) => e,
        _ => rt::internal("a channel operation on something that is not a channel"),
    }
}

/// The clock a clock's object or handle names: the host's (0) for the
/// host's object and for a `Clock` handle compiled to call it directly
/// (decision 194); a test clock is its clock's number.
pub(crate) unsafe fn clock_of(c: &Call<'_>) -> u32 {
    // SAFETY: the first argument is the clock's object or handle.
    let (p, t) = unsafe { c.place(0) };
    match &ty(t).kind {
        Kind::Opaque(n) if n == "HostClock" || n == "Clock" => 0,
        // SAFETY: a test clock's handle is a word, its clock's number.
        _ => unsafe { mem::word(p, 0) as u32 },
    }
}

/// The variant `name` of the enum type `t`: its index and fields.
fn variant(t: u32, name: &str) -> Option<(u64, &'static [abi::Field])> {
    match &ty(t).kind {
        Kind::Enum { variants, .. } => {
            variants.iter().position(|v| v.name == name).map(|i| (i as u64, variants[i].fields.as_slice()))
        }
        _ => None,
    }
}

/// Writes at `p` the error value `name` of the error set `decl`, with its
/// fields (`TaskPanicked`'s message and trace).
///
/// # Safety
/// `p` must have room for a value of `decl`.
unsafe fn write_error(p: *mut u8, decl: u32, name: &str, texts: Option<(&str, &str)>) {
    let Some((tag, fields)) = variant(decl, name) else {
        rt::internal("an error variant the program does not have")
    };
    // SAFETY: an error value is its type's number, its trace and its tag,
    // then the variant's fields.
    unsafe {
        mem::set_word(p, abi::ERROR_DECL, u64::from(decl));
        mem::set_word(p, abi::ERROR_TRACE, 0);
        mem::set_word(p, abi::ERROR_TAG, tag);
        if let Some((a, b)) = texts {
            for (f, text) in fields.iter().zip([a, b]) {
                value::write_string(p.add(f.offset as usize), text.as_bytes());
            }
        }
    }
}

/// Writes `Err(TaskError.name(...))` at `ret`, a result of type `ret_ty`.
///
/// # Safety
/// `ret` must have room for a value of `ret_ty`.
pub(crate) unsafe fn write_task_error(ret: *mut u8, ret_ty: u32, name: &str, texts: Option<(&str, &str)>) {
    let _ = ret_ty;
    let Some(decl) = sched::task_error() else { rt::internal("`TaskError` is not in the program") };
    // SAFETY: a result is its tag, then the error.
    unsafe {
        mem::set_word(ret, 0, 1);
        write_error(ret.add(8), decl, name, texts);
    }
}

/// Writes `Err(ChannelError.Closed)` at `ret`.
///
/// # Safety
/// `ret` must have room for a result whose error set holds it.
unsafe fn write_closed(ret: *mut u8) {
    let Some(decl) = sched::channel_error() else { rt::internal("`ChannelError` is not in the program") };
    // SAFETY: as `write_task_error`.
    unsafe {
        mem::set_word(ret, 0, 1);
        write_error(ret.add(8), decl, "Closed", None);
    }
}

/// Writes `Some(value)` at `ret`.
///
/// # Safety
/// `ret` must have room for an option of a value of `size` bytes.
unsafe fn write_some(ret: *mut u8, words: &[u64], size: u32) {
    // SAFETY: an option is its tag, then the value.
    unsafe {
        mem::set_word(ret, 0, 0);
        mem::copy(ret.add(8), words.as_ptr() as *const u8, size as usize);
    }
}

/// The words of `size` bytes at `p`, moved out.
///
/// # Safety
/// `p` must hold `size` bytes.
unsafe fn take_words(p: *const u8, size: u32) -> Box<[u64]> {
    let mut w = vec![0u64; (size as usize).div_ceil(8)].into_boxed_slice();
    // SAFETY: the buffer holds `size` bytes.
    unsafe { mem::copy(w.as_mut_ptr() as *mut u8, p, size as usize) };
    w
}

/// Whether a value of the result type `t`, in `words`, is an error.
fn is_err(words: &[u64], t: u32) -> bool {
    matches!(ty(t).kind, Kind::Result(..)) && words.first() == Some(&1)
}

/// Whether a result type can hold an error of the set `decl`.
fn carries(result_ty: u32, decl: u32) -> bool {
    match ty(result_ty).kind {
        Kind::Result(_, err) => match &ty(err).kind {
            Kind::ErrSet(members) => members.contains(&decl),
            _ => err == decl,
        },
        _ => false,
    }
}

impl sched::Sched {
    /// Whether a task's result is `Err(Cancelled)`.
    fn is_cancelled_error(&self, words: &[u64], t: u32) -> bool {
        let Some(te) = sched::task_error() else { return false };
        let cancelled = variant(te, "Cancelled").map(|(i, _)| i);
        is_err(words, t) && words.get(1) == Some(&u64::from(te)) && words.get(3).copied() == cancelled
    }

    /// A task's closure returned or panicked: the scope's rules.
    pub(crate) fn finish_task(&mut self, t: u32, outcome: Outcome) {
        let cancelled = self.node_cancelled(Some(Node::Task(t)));
        let (returns_result, ret_ty) = (self.task(t).returns_result, self.task(t).ret_ty);
        let failure = match &outcome {
            Outcome::Panic(_) => true,
            Outcome::Value(v) => {
                returns_result && is_err(v, ret_ty) && !(cancelled && self.is_cancelled_error(v, ret_ty))
            }
        };
        let task = self.task(t);
        task.outcome = Some(outcome);
        task.state = TState::Done;
        let (generator, scope, index) = (task.generator, task.scope, task.index);
        if !generator {
            self.live = self.live.saturating_sub(1);
        }
        if let Some(s) = scope {
            let (race, pure) =
                self.scopes.get(s as usize).map(|r| (r.race, r.pure)).unwrap_or((false, false));
            if race {
                if self.scopes[s as usize].winner.is_none() {
                    self.scopes[s as usize].winner = Some(t);
                    self.cancel_children(s, Some(t), None);
                }
            } else if failure {
                if pure {
                    self.cancel_children(s, Some(t), Some(index));
                } else {
                    if self.scopes[s as usize].first_failure.is_none() {
                        self.scopes[s as usize].first_failure = Some(t);
                    }
                    self.cancel_children(s, Some(t), None);
                }
            }
            self.wake_key(Key::Scope(s));
        }
        self.wake_key(Key::Task(t));
    }

    /// Takes a finished task's outcome as `join` gives it, into `ret` (of
    /// type `ret_ty`): a result is flattened, a value is `Ok`, a panic is
    /// `TaskPanicked`. A second take panics.
    ///
    /// # Safety
    /// `ret` must have room for a value of `ret_ty`.
    unsafe fn take_outcome(&mut self, t: u32, ret: *mut u8, ret_ty: u32) -> u8 {
        let task = self.task(t);
        if task.taken {
            return rt::raise_msg(msg::PANIC_JOINED_TWICE, &[]);
        }
        task.taken = true;
        let (returns_result, size) = (task.returns_result, ty(task.ret_ty).size);
        let room = ty(ret_ty).size;
        // SAFETY: `ret` has room for the result type, which holds the
        // task's result (its error set is the task's, widened).
        unsafe {
            match task.outcome.take() {
                Some(Outcome::Value(v)) => {
                    if returns_result {
                        mem::copy(ret, v.as_ptr() as *const u8, size.min(room) as usize);
                    } else {
                        mem::set_word(ret, 0, 0);
                        mem::copy(
                            ret.add(8),
                            v.as_ptr() as *const u8,
                            size.min(room.saturating_sub(8)) as usize,
                        );
                    }
                }
                Some(Outcome::Panic(p)) => {
                    let trace = sisu_diag::trace::shown(&p.stack).join("\n");
                    write_task_error(ret, ret_ty, "TaskPanicked", Some((&p.message, &trace)));
                }
                None => rt::internal("a finished task without an outcome"),
            }
        }
        abi::OK
    }

    /// The results of a scope's tasks that nobody took, in spawn order.
    fn unjoined(&mut self, children: &[u32]) -> Vec<(Box<[u64]>, u32)> {
        let mut out = Vec::new();
        for &t in children {
            let task = self.task(t);
            if task.taken {
                continue;
            }
            let ty_ = task.ret_ty;
            if let Some(Outcome::Value(v)) = task.outcome.take() {
                out.push((v, ty_));
            }
        }
        out
    }

    /// The next value of a channel if there is one now: `Ok(words)`,
    /// `Err(())` when it is closed and drained, `None` when a receiver
    /// would wait.
    pub(crate) fn recv_now(&mut self, ch: u32) -> Option<Result<Box<[u64]>, ()>> {
        let chan = self.channels.get_mut(ch as usize)?;
        if let Some(v) = chan.buf.pop_front() {
            self.wake_key(Key::Chan(ch));
            return Some(Ok(v));
        }
        if chan.closed || chan.senders == 0 {
            return Some(Err(()));
        }
        None
    }
}

/// Drops values in the order the interpreter's queue of drops runs them:
/// the last queued first.
fn drop_queued(queue: Vec<(Box<[u64]>, u32)>) -> u8 {
    let mut status = abi::OK;
    for (mut words, t) in queue.into_iter().rev() {
        // SAFETY: the words hold a value of type `t`, dropped once here.
        let st = unsafe { value::drop_value(words.as_mut_ptr() as *mut u8, t) };
        status = value::after_part(status, st);
    }
    status
}

// ------------------------------------------------------------------ scopes

/// Counts an anchor frame toward the limit of nested calls (the
/// interpreter's anchors are frames); false past it, with the panic raised,
/// which names the intrinsic that opened the anchor (decision 234).
fn enter_anchor(name: &str) -> bool {
    let d = crate::entry::SISU_RT_DEPTH.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
    if d > abi::MAX_DEPTH {
        crate::entry::SISU_RT_DEPTH.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        system(|| rt::raise_msg(msg::PANIC_STACK_OVERFLOW, &[("function", name.to_string())]));
        return false;
    }
    true
}

fn leave_anchor() {
    crate::entry::SISU_RT_DEPTH.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
}

/// The code and environment of the task's closure (or a generator's) an
/// argument holds: a function value is its code, then its environment,
/// for a task's closure the address of a block the runtime frees.
///
/// # Safety
/// Argument `i` must be (or refer to) a function value.
unsafe fn closure(c: &Call<'_>, i: usize) -> (usize, usize) {
    // SAFETY: a function value is its code and its environment.
    unsafe {
        let (p, _) = c.place(i);
        (mem::word(p, 0) as usize, mem::word(p, 8) as usize)
    }
}

/// The code of the closure an argument holds and the address of the
/// environment the value holds, as a call of a function value passes it
/// (decision 224): a body the runtime calls at once (`task.scope`,
/// `task.shield`).
///
/// # Safety
/// Argument `i` must be (or refer to) a function value.
unsafe fn closure_here(c: &Call<'_>, i: usize) -> (usize, usize) {
    // SAFETY: a function value is its code, then its environment.
    unsafe {
        let (p, _) = c.place(i);
        (mem::word(p, 0) as usize, p as usize + abi::WORD as usize)
    }
}

/// `task.shield(body)`: the body under a context that is never cancelled.
unsafe fn shield(c: &Call<'_>) -> u8 {
    // SAFETY: the argument is the body.
    let (code, env) = unsafe { closure_here(c, 0) };
    if !enter_anchor("task.shield") {
        return abi::UNWINDING;
    }
    system(|| with(|s| s.task(s.cur).anchors.push(Anchor::Shield)));
    // SAFETY: the code takes its result's address and its environment.
    let status = unsafe { std::mem::transmute::<usize, Body0>(code)(c.ret, env as *mut u8) };
    system(|| {
        with(|s| {
            s.task(s.cur).anchors.pop();
        })
    });
    leave_anchor();
    status
}

/// `task.scope(body)` and `task.race(body)`: the body above the scope's
/// anchor, then the scope's end: cancel as the rules say, wait for every
/// task, and give the scope's result, or let the panic go on.
unsafe fn scope(c: &Call<'_>, race: bool) -> u8 {
    let site = Site::current();
    // SAFETY: the argument is the body.
    let (code, env) = unsafe { closure_here(c, 0) };
    let ret_ty = c.ret_ty;
    let s = system(|| {
        with(|sc| {
            let parent = sc.context();
            let id = sc.scopes.len() as u32;
            sc.scopes.push(ScopeRec {
                parent,
                cancelled: false,
                children: Vec::new(),
                pure: site.flags & abi::SITE_PURE != 0,
                race,
                body_done: false,
                ended: false,
                first_failure: None,
                winner: None,
            });
            let cur = sc.cur;
            sc.task(cur).anchors.push(Anchor::Scope(id));
            id
        })
    });
    if !enter_anchor(if race { "task.race" } else { "task.scope" }) {
        system(|| {
            with(|sc| {
                sc.task(sc.cur).anchors.pop();
            })
        });
        return abi::UNWINDING;
    }
    // The body's result: the scope's, but for a race, whose body gives
    // nothing.
    let body_ty = if race { None } else { Some(ret_ty) };
    let size = body_ty.map(|t| ty(t).size).unwrap_or(0);
    let mut body = vec![0u64; (size as usize).div_ceil(8).max(1)].into_boxed_slice();
    let mut scope_word = u64::from(s);
    // SAFETY: the code takes its result's address, its environment and the
    // scope's address.
    let status = unsafe {
        std::mem::transmute::<usize, Body1>(code)(
            body.as_mut_ptr() as *mut u8,
            env as *mut u8,
            &mut scope_word as *mut u64 as *mut u8,
        )
    };
    let panicked = status != abi::OK;
    let failed = !panicked && body_ty.is_some_and(|t| is_err(&body, t));
    // The end: wait for every task.
    let _ = sched::blocking(|| {
        with(|sc| {
            let si = s as usize;
            if !sc.scopes[si].body_done {
                sc.scopes[si].body_done = true;
                if (panicked || failed) && !sc.scopes[si].race {
                    sc.scopes[si].cancelled = true;
                    sc.wake_all_blocked();
                }
            }
            let waiting =
                sc.scopes[si].children.iter().any(|&t| !matches!(sc.tasks[t as usize].state, TState::Done));
            if waiting {
                sc.block_on(Wait::ScopeEnd, &[Key::Scope(s)], None, site);
                return Attempt::Wait;
            }
            Attempt::Done(abi::OK)
        })
    });
    system(|| {
        with(|sc| {
            sc.task(sc.cur).anchors.pop();
            sc.scopes[s as usize].ended = true;
        })
    });
    leave_anchor();
    // SAFETY: `c.ret` has room for the scope's result.
    unsafe { scope_end(c, s, status, body, body_ty) }
}

/// What a scope gives once its tasks have finished.
///
/// # Safety
/// `c.ret` must have room for the scope's result.
unsafe fn scope_end(c: &Call<'_>, s: u32, status: u8, body: Box<[u64]>, body_ty: Option<u32>) -> u8 {
    let ret_ty = c.ret_ty;
    let children = system(|| with(|sc| sc.scopes[s as usize].children.clone()));
    let write_body = |body: &[u64]| {
        let size = body_ty.map(|t| ty(t).size).unwrap_or(0);
        // SAFETY: the result has room for the body's value, the scope's.
        unsafe { mem::copy(c.ret, body.as_ptr() as *const u8, size as usize) };
    };
    // A panic in the body goes on once every task has finished.
    if status != abi::OK {
        let queue = system(|| with(|sc| sc.unjoined(&children)));
        return value::after_part(status, drop_queued(queue));
    }
    if body_ty.is_none() {
        // A race: the first task to finish wins.
        let Some(winner) = system(|| with(|sc| sc.scopes[s as usize].winner)) else {
            return rt::raise_msg(msg::PANIC_EMPTY_RACE, &[]);
        };
        // SAFETY: as the function's contract.
        let st = system(|| with(|sc| unsafe { sc.take_outcome(winner, c.ret, ret_ty) }));
        if st != abi::OK {
            return st;
        }
        let queue = system(|| with(|sc| sc.unjoined(&children)));
        return drop_queued(queue);
    }
    // An error the body returns is the result.
    if body_ty.is_some_and(|t| is_err(&body, t)) {
        let queue = system(|| with(|sc| sc.unjoined(&children)));
        let st = drop_queued(queue);
        write_body(&body);
        return st;
    }
    // A failure nobody joined: the first in time, or in a `pure` scope the
    // lowest index.
    let failed = system(|| {
        with(|sc| {
            let rec = &sc.scopes[s as usize];
            let unobserved = |t: u32, sc: &sched::Sched| {
                let task = &sc.tasks[t as usize];
                !task.taken
                    && match &task.outcome {
                        Some(Outcome::Panic(_)) => true,
                        Some(Outcome::Value(v)) => {
                            task.returns_result
                                && is_err(v, task.ret_ty)
                                && !(sc.is_cancelled_error(v, task.ret_ty)
                                    && (task.cancelled || sc.scopes[s as usize].cancelled))
                        }
                        None => false,
                    }
            };
            if rec.pure {
                children.iter().copied().find(|&t| unobserved(t, sc))
            } else {
                rec.first_failure.filter(|&t| unobserved(t, sc))
            }
        })
    });
    let Some(t) = failed else {
        let queue = system(|| with(|sc| sc.unjoined(&children)));
        let st = drop_queued(queue);
        write_body(&body);
        return st;
    };
    let (outcome, task_ty) = system(|| {
        with(|sc| {
            let task = sc.task(t);
            task.taken = true;
            (task.outcome.take(), task.ret_ty)
        })
    });
    let mut queue = system(|| with(|sc| sc.unjoined(&children)));
    if let Some(bt) = body_ty {
        queue.push((body, bt));
    }
    match outcome {
        Some(Outcome::Panic(p)) => {
            if sched::task_error().is_some_and(|te| carries(ret_ty, te)) {
                let trace = sisu_diag::trace::shown(&p.stack).join("\n");
                // SAFETY: the scope's result carries `TaskError`.
                system(|| unsafe {
                    write_task_error(c.ret, ret_ty, "TaskPanicked", Some((&p.message, &trace)))
                });
                drop_queued(queue)
            } else {
                let st = drop_queued(queue);
                value::after_part(rt::raise_panic(p), st)
            }
        }
        Some(Outcome::Value(mut v)) => {
            let decl = v.get(1).copied().unwrap_or(0) as u32;
            if carries(ret_ty, decl) {
                let size = ty(task_ty).size.min(ty(ret_ty).size);
                // SAFETY: an error result: its tag and its error, which the
                // scope's result type carries.
                unsafe { mem::copy(c.ret, v.as_ptr() as *const u8, size as usize) };
                drop_queued(queue)
            } else {
                let shown = system(|| {
                    let mut out = Vec::new();
                    if let Kind::Result(_, err) = ty(task_ty).kind {
                        // SAFETY: the error is after the result's tag.
                        unsafe { value::debug(&mut out, (v.as_mut_ptr() as *mut u8).add(8), err) };
                    }
                    String::from_utf8_lossy(&out).into_owned()
                });
                queue.push((v, task_ty));
                let raised = rt::raise_msg(msg::PANIC_UNOBSERVED_ERROR, &[("error", shown)]);
                value::after_part(raised, drop_queued(queue))
            }
        }
        None => rt::internal("a failed task without an outcome"),
    }
}

// ------------------------------------------------------------------- tasks

/// `Scope.spawn` and `Scope.spawn_with`, on the system stack.
unsafe fn spawn(c: &Call<'_>, with_deadline: bool) -> u8 {
    // SAFETY: the arguments are the scope, the timer and the closure.
    unsafe {
        let s = c.word(0) as u32;
        let deadline = if with_deadline {
            let (p, _) = c.place(1);
            Some((mem::word(p, 0) as u32, i128::from(mem::word(p, 8) as i64)))
        } else {
            None
        };
        let body = closure(c, if with_deadline { 2 } else { 1 });
        let Kind::Handle(ret_ty) = ty(c.ret_ty).kind else {
            rt::internal("a spawn without its handle's type")
        };
        let r = with(|sc| {
            if sc.scopes.get(s as usize).is_none_or(|r| r.ended) {
                return Err(());
            }
            let t = sc.tasks.len() as u32;
            let cur = sc.cur;
            sc.task(cur).spawned += 1;
            let n = sc.task(cur).spawned;
            let path = match sc.task(cur).path.as_str() {
                "" => n.to_string(),
                p => format!("{p}.{n}"),
            };
            let index = sc.scopes[s as usize].children.len() as u32;
            sc.scopes[s as usize].children.push(t);
            let mut task = sc.new_task(Some(s), index, path);
            task.deadline = deadline;
            task.body = body;
            task.ret_ty = ret_ty;
            task.returns_result = matches!(ty(ret_ty).kind, Kind::Result(..));
            sc.tasks.push(task);
            make_coroutine(sc, t, false);
            sc.push_ready(t);
            sc.live += 1;
            sc.stats.tasks += 1;
            sc.stats.max_live = sc.stats.max_live.max(sc.live);
            // Spawning is a yield point.
            sc.spend_one();
            Ok(t)
        });
        match r {
            Ok(t) => {
                c.set_word(u64::from(t));
                abi::OK
            }
            Err(()) => rt::raise_msg(msg::PANIC_SCOPE_ENDED, &[]),
        }
    }
}

/// Gives task `t` a stack and the coroutine that runs its body.
fn make_coroutine(sc: &mut sched::Sched, t: u32, generator: bool) {
    let Some(region) = sc.pool.allocate() else {
        rt::abort("out of memory: no address space for a task's stack")
    };
    let limits = sc.pool.limits(region);
    let co = sched::Co::with_stack(crate::stack::Slot(region), move |y: &Yielder<(), sched::Suspend>, ()| {
        task_main(t, y, generator)
    });
    let task = sc.task(t);
    task.region = Some(region);
    task.ctx.stack = limits;
    task.co = Some(Box::new(co));
}

/// The body of a task's coroutine: its closure, then the scope's rules
/// for how it ended.
fn task_main(t: u32, y: &Yielder<(), sched::Suspend>, generator: bool) {
    sched::enter_coroutine(y);
    let ((code, env), ret_ty) = with(|s| {
        let task = s.task(t);
        task.yielder = y;
        (task.body, task.ret_ty)
    });
    let size = if generator { 0 } else { ty(ret_ty).size };
    let mut ret = vec![0u64; (size as usize).div_ceil(8)].into_boxed_slice();
    let mut unit = 0u64;
    let out = if size == 0 { &mut unit as *mut u64 as *mut u8 } else { ret.as_mut_ptr() as *mut u8 };
    // The interpreter pushed the closure's frame when it spawned the task:
    // its entry is not a yield point.
    // SAFETY: the code takes its result's address and its environment.
    let status = sched::neutral(|| unsafe { std::mem::transmute::<usize, Body0>(code)(out, env as *mut u8) });
    // The closure dropped what its environment held; its cell is the
    // runtime's to free.
    system(|| crate::entry::release_task_env(env, false));
    // An abandoned generator's frames have unwound (decision 95): nothing
    // waits for it.
    if sched::abandoning() {
        return;
    }
    let outcome = if status == abi::OK {
        Outcome::Value(ret)
    } else {
        match rt::take_panic() {
            Some(p) => Outcome::Panic(p),
            None => rt::internal("a task ended unwinding without its panic"),
        }
    };
    system(move || with(|s| s.finish_task(t, outcome)));
}

/// `Handle.join`.
unsafe fn join(c: &Call<'_>) -> u8 {
    // SAFETY: the argument is a handle.
    let t = unsafe { c.word(0) } as u32;
    let site = Site::current();
    sched::blocking(|| {
        with(|s| {
            if s.cancelled_now() {
                // SAFETY: the result is `T ! {..., TaskError}`.
                unsafe { write_task_error(c.ret, c.ret_ty, "Cancelled", None) };
                return Attempt::Done(abi::OK);
            }
            if s.tasks.get(t as usize).is_some_and(|t| matches!(t.state, TState::Done)) {
                // SAFETY: as above.
                return Attempt::Done(unsafe { s.take_outcome(t, c.ret, c.ret_ty) });
            }
            s.block_on(Wait::Join(t), &[Key::Task(t)], None, site);
            Attempt::Wait
        })
    })
}

// ---------------------------------------------------------------- channels

/// `channel[T](capacity)`: a sender and a receiver, the channel's number
/// in each.
unsafe fn channel(c: &Call<'_>) -> u8 {
    // SAFETY: the argument is an `int`; the result a pair of words.
    unsafe {
        let cap = c.int(0);
        if cap < 1 {
            return rt::raise_msg(msg::PANIC_CHANNEL_CAPACITY, &[("capacity", cap.to_string())]);
        }
        let elem = match &ty(c.ret_ty).kind {
            Kind::Tuple(slots) => slots.first().map(|s| elem_of(s.ty)).unwrap_or(0),
            _ => rt::internal("a channel of an unknown type"),
        };
        let ch = with(|s| {
            let ch = s.channels.len() as u32;
            s.channels.push(sched::Channel {
                buf: std::collections::VecDeque::new(),
                cap: cap as usize,
                closed: false,
                senders: 1,
                receivers: 1,
                elem,
            });
            ch
        });
        mem::set_word(c.ret, 0, u64::from(ch));
        mem::set_word(c.ret, 8, u64::from(ch));
    }
    abi::OK
}

/// `Sender.send(value)`.
unsafe fn send(c: &Call<'_>) -> u8 {
    // SAFETY: the arguments are a sender and the value, moved in.
    let ch = unsafe { c.word(0) } as u32;
    let (vp, vt) = (c.arg(1), c.ty(1));
    let site = Site::current();
    sched::blocking(|| {
        enum Then {
            Drop(bool),
            Sent,
            Wait,
        }
        let then = with(|s| {
            if s.cancelled_now() {
                return Then::Drop(true);
            }
            let chan = &s.channels[ch as usize];
            if chan.receivers == 0 || chan.closed {
                return Then::Drop(false);
            }
            if chan.buf.len() < chan.cap {
                // SAFETY: the value is moved into the channel.
                let v = unsafe { take_words(vp, ty(vt).size) };
                s.channels[ch as usize].buf.push_back(v);
                s.wake_key(Key::Chan(ch));
                return Then::Sent;
            }
            s.block_on(Wait::Send, &[Key::Chan(ch)], None, site);
            Then::Wait
        });
        // SAFETY: the result is `! {ChannelError, TaskError}`; a value not
        // sent is dropped.
        unsafe {
            match then {
                Then::Drop(cancelled) => {
                    let st = value::drop_value(vp, vt);
                    if cancelled {
                        write_task_error(c.ret, c.ret_ty, "Cancelled", None);
                    } else {
                        write_closed(c.ret);
                    }
                    Attempt::Done(st)
                }
                Then::Sent => {
                    mem::set_word(c.ret, 0, 0);
                    Attempt::Done(abi::OK)
                }
                Then::Wait => Attempt::Wait,
            }
        }
    })
}

/// `Sender.try_send(value)`: `None` once sent, else the value back.
unsafe fn try_send(c: &Call<'_>) -> u8 {
    // SAFETY: the arguments are a sender and the value, moved in.
    unsafe {
        let ch = c.word(0) as u32;
        let (vp, vt) = (c.arg(1), c.ty(1));
        let size = ty(vt).size;
        let sent = with(|s| {
            let chan = &s.channels[ch as usize];
            if chan.receivers == 0 || chan.closed || chan.buf.len() >= chan.cap {
                return false;
            }
            let v = take_words(vp, size);
            s.channels[ch as usize].buf.push_back(v);
            s.wake_key(Key::Chan(ch));
            true
        });
        if sent {
            mem::set_word(c.ret, 0, 1);
        } else {
            mem::set_word(c.ret, 0, 0);
            mem::copy(c.ret.add(8), vp, size as usize);
        }
    }
    abi::OK
}

/// `Receiver.recv()`.
unsafe fn recv(c: &Call<'_>) -> u8 {
    // SAFETY: the argument is a receiver.
    let ch = unsafe { c.word(0) } as u32;
    let size = ty(elem_of(c.ty(0))).size;
    let site = Site::current();
    sched::blocking(|| {
        with(|s| {
            // SAFETY: the result is `T ! {ChannelError, TaskError}`.
            unsafe {
                if s.cancelled_now() {
                    write_task_error(c.ret, c.ret_ty, "Cancelled", None);
                    return Attempt::Done(abi::OK);
                }
                match s.recv_now(ch) {
                    Some(Ok(v)) => {
                        write_some(c.ret, &v, size);
                        Attempt::Done(abi::OK)
                    }
                    Some(Err(())) => {
                        write_closed(c.ret);
                        Attempt::Done(abi::OK)
                    }
                    None => {
                        s.block_on(Wait::Recv, &[Key::Chan(ch)], None, site);
                        Attempt::Wait
                    }
                }
            }
        })
    })
}

/// A channel end dropped (`receiver` or a sender): when the receiver is
/// gone, the values still in the channel are dropped.
pub(crate) fn drop_channel_end(ch: u32, receiver: bool) -> u8 {
    let left = with(|s| {
        let Some(chan) = s.channels.get_mut(ch as usize) else { return Vec::new() };
        if receiver {
            chan.receivers = chan.receivers.saturating_sub(1);
        } else {
            chan.senders = chan.senders.saturating_sub(1);
        }
        let elem = chan.elem;
        let left: Vec<(Box<[u64]>, u32)> =
            if chan.receivers == 0 { chan.buf.drain(..).map(|v| (v, elem)).collect() } else { Vec::new() };
        s.wake_key(Key::Chan(ch));
        left
    });
    let mut status = abi::OK;
    for (mut v, t) in left {
        // SAFETY: the words hold a value of the channel's type, dropped once.
        status = value::after_part(status, unsafe { value::drop_value(v.as_mut_ptr() as *mut u8, t) });
    }
    status
}

/// `.copy()` of a channel end: another sender (or receiver).
pub(crate) fn copy_channel_end(ch: u32, receiver: bool) {
    with(|s| {
        if let Some(chan) = s.channels.get_mut(ch as usize) {
            if receiver {
                chan.receivers += 1;
            } else {
                chan.senders += 1;
            }
        }
    });
}

// ------------------------------------------------------------------ select

/// `select.wait(kind, subject, ...)`: the index of the arm to take (06,
/// "Channels and select").
unsafe fn select(c: &Call<'_>) -> u8 {
    let site = Site::current();
    // The arms: (kind, the subject's words).
    let mut arms = Vec::new();
    // SAFETY: the arguments are pairs of a kind and its subject.
    unsafe {
        for k in (0..c.args.len()).step_by(2) {
            let kind = c.int(k);
            let (p, _) = c.place(k + 1);
            let words = if kind == 3 { (mem::word(p, 0), mem::word(p, 8)) } else { (mem::word(p, 0), 0) };
            arms.push((kind, words));
        }
    }
    sched::blocking(|| {
        with(|s| {
            let cancelled = s.cancelled_now();
            let mut ready = Vec::new();
            let mut keys = Vec::new();
            let mut timer: Option<(u32, i128)> = None;
            let mut first_reporting = None;
            for (i, &(kind, (a, b))) in arms.iter().enumerate() {
                let ok = match kind {
                    0 => {
                        let ch = a as u32;
                        keys.push(Key::Chan(ch));
                        let chan = &s.channels[ch as usize];
                        !chan.buf.is_empty() || chan.closed || chan.senders == 0
                    }
                    1 => {
                        let ch = a as u32;
                        keys.push(Key::Chan(ch));
                        let chan = &s.channels[ch as usize];
                        chan.buf.len() < chan.cap || chan.receivers == 0 || chan.closed
                    }
                    2 => {
                        let t = a as u32;
                        keys.push(Key::Task(t));
                        s.tasks.get(t as usize).is_some_and(|t| matches!(t.state, TState::Done))
                    }
                    3 => {
                        let (clock, at) = (a as u32, i128::from(b as i64));
                        if timer.is_none_or(|(_, x)| at < x) {
                            timer = Some((clock, at));
                        }
                        s.clock_now(clock) >= at
                    }
                    _ => {
                        let c_ = s.node_cancelled(Some(Node::Scope(a as u32)));
                        if c_ && cancelled {
                            return Some(i);
                        }
                        c_
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
                return Some(i);
            }
            if !ready.is_empty() {
                let k = s.rng.below(ready.len());
                return Some(ready[k]);
            }
            s.block_on(Wait::Select, &keys, timer, site);
            None
        })
        .map(|i| {
            // SAFETY: the result is an `int`.
            unsafe { c.set_word(i as u64) };
            Attempt::Done(abi::OK)
        })
        .unwrap_or(Attempt::Wait)
    })
}

// -------------------------------------------------------------- generators

/// `gen.new(body)`: a generator, a task that runs its body only while its
/// consumer waits for the next item.
unsafe fn gen_new(c: &Call<'_>) -> u8 {
    // SAFETY: the argument is the body.
    let body = unsafe { closure(c, 0) };
    let Kind::Iter(item) = ty(c.ret_ty).kind else { rt::internal("a generator of an unknown type") };
    let t = with(|s| {
        let t = s.tasks.len() as u32;
        let path = s.tasks.get(s.cur as usize).map(|t| t.path.clone()).unwrap_or_default();
        let mut task = s.new_task(None, 0, path);
        task.state = TState::Blocked(Wait::Join(t));
        task.generator = true;
        task.item_ty = item;
        task.body = body;
        s.tasks.push(task);
        make_coroutine(s, t, true);
        t
    });
    // SAFETY: the result is an iterator's word.
    unsafe { c.set_word(crate::iter::into_word(crate::iter::generator(t, ty(item).size))) };
    abi::OK
}

/// `yield v` in a generator: hands `v` to the consumer and waits to be
/// asked for the next.
unsafe fn gen_yield(c: &Call<'_>) -> u8 {
    let (vp, vt) = (c.arg(0), c.ty(0));
    let site = Site::current();
    let mut handed = false;
    sched::blocking(|| {
        if handed {
            // Abandoned while it waited here: its frames unwind.
            return Attempt::Done(if sched::abandoning() { abi::UNWINDING } else { abi::OK });
        }
        handed = true;
        with(|s| {
            let t = s.cur;
            // SAFETY: the item is moved to the consumer.
            let v = unsafe { take_words(vp, ty(vt).size) };
            s.task(t).yielded = Some(v);
            s.wake_key(Key::Task(t));
            s.block_on(Wait::Select, &[], None, site);
        });
        Attempt::Wait
    })
}

/// `Iter.next`: a generator's or a receiver's iterator may wait; any other
/// is the runtime's (`iter::next`).
unsafe fn iter_next(c: &Call<'_>) -> u8 {
    // SAFETY: the argument is an iterator, through a reference.
    let (p, _) = unsafe { c.place(0) };
    // SAFETY: an iterator is a word holding its object, or null.
    let obj = unsafe { mem::word(p, 0) } as *mut crate::iter::IterObj;
    if obj.is_null() {
        // SAFETY: the result is an option.
        unsafe { c.none() };
        return abi::OK;
    }
    // SAFETY: the object is an iterator the runtime made.
    let state = unsafe { crate::iter::waits(&*obj) };
    let site = Site::current();
    match state {
        Some(crate::iter::Waits::Gen(t, size)) => sched::blocking(|| {
            with(|s| {
                let task = s.task(t);
                if let Some(v) = task.yielded.take() {
                    // SAFETY: the result is an option of the item.
                    unsafe { write_some(c.ret, &v, size) };
                    return Attempt::Done(abi::OK);
                }
                if matches!(task.state, TState::Done) {
                    return match task.outcome.take() {
                        Some(Outcome::Panic(p)) => Attempt::Done(rt::raise_panic(p)),
                        _ => {
                            // SAFETY: as above.
                            unsafe { c.none() };
                            Attempt::Done(abi::OK)
                        }
                    };
                }
                s.wake(t);
                s.block_on(Wait::Join(t), &[Key::Task(t)], None, site);
                Attempt::Wait
            })
        }),
        Some(crate::iter::Waits::Recv(ch, size)) => sched::blocking(|| {
            with(|s| {
                if s.cancelled_now() {
                    // SAFETY: the result is an option.
                    unsafe { c.none() };
                    return Attempt::Done(abi::OK);
                }
                match s.recv_now(ch) {
                    Some(Ok(v)) => {
                        // SAFETY: as above.
                        unsafe { write_some(c.ret, &v, size) };
                        Attempt::Done(abi::OK)
                    }
                    Some(Err(())) => {
                        // SAFETY: as above.
                        unsafe { c.none() };
                        Attempt::Done(abi::OK)
                    }
                    None => {
                        s.block_on(Wait::Recv, &[Key::Chan(ch)], None, site);
                        Attempt::Wait
                    }
                }
            })
        }),
        // SAFETY: as above.
        None => sched::system_status(|| unsafe { crate::iter::next(&mut *obj, c.ret) }),
    }
}

/// A generator's iterator was dropped before its end: the generator is
/// abandoned (decisions 95 and 223). What its frames hold is let go of as the
/// interpreter's host lets it go, running no Sisu code: a generator
/// suspended at a `yield` unwinds its frames with their drops forgetting
/// (`sched::unwind_abandoned`); one that never started forgets the values
/// its environment holds. Its stack goes back to the pool.
pub(crate) fn abandon(t: u32) {
    let started = with(|s| {
        let task = s.task(t);
        task.co.as_ref()?;
        Some(!task.yielder.is_null())
    });
    match started {
        None => {}
        Some(true) => {
            sched::unwind_abandoned(t);
        }
        Some(false) => with(|s| {
            let Some(mut co) = s.task(t).co.take() else { return };
            // SAFETY: the generator never started, and is never resumed.
            unsafe { co.force_reset() };
            let stack = co.into_stack();
            let mark = s.task(t).mark;
            s.task(t).region = None;
            s.task(t).state = TState::Done;
            s.pool.release(stack.0, mark);
        }),
    }
    // An item it handed over and nobody took, and the environment of a
    // closure that never started, which never took what it holds.
    let (env, item) = with(|s| {
        let task = s.task(t);
        let env = std::mem::take(&mut task.body).1;
        (env, task.yielded.take().map(|v| (v, task.item_ty)))
    });
    if let Some((mut words, item_ty)) = item {
        // SAFETY: the words hold an item of the generator's type.
        unsafe { crate::value::forget_value(words.as_mut_ptr() as *mut u8, item_ty) };
    }
    if started == Some(false) {
        crate::entry::release_task_env(env, true);
    }
}

// ------------------------------------------------------------------ mutexes

/// `Mutex.lock`: waits while another task holds the mutex, and cannot be
/// cancelled; panics when this task holds it, or it is poisoned.
unsafe fn lock(c: &Call<'_>) -> u8 {
    // SAFETY: the argument is a mutex, through a reference.
    let (p, _) = unsafe { c.place(0) };
    let site = Site::current();
    sched::blocking(|| {
        with(|s| {
            let cur = s.cur;
            // SAFETY: the mutex's header is its first words.
            unsafe {
                if mem::word(p, abi::MUTEX_POISONED) != 0 {
                    return Attempt::Done(rt::raise_msg(msg::PANIC_MUTEX_POISONED, &[]));
                }
                if mem::word(p, abi::MUTEX_LOCKED) != 0 {
                    if mem::word(p, abi::MUTEX_HOLDER) == u64::from(cur) {
                        return Attempt::Done(rt::raise_msg(msg::PANIC_MUTEX_HELD, &[]));
                    }
                    s.block_on(Wait::Lock, &[Key::Lock], None, site);
                    return Attempt::Wait;
                }
                mem::set_word(p, abi::MUTEX_LOCKED, 1);
                mem::set_word(p, abi::MUTEX_HOLDER, u64::from(cur));
                c.set_word(p.add(abi::MUTEX_HEADER as usize) as u64);
            }
            Attempt::Done(abi::OK)
        })
    })
}

/// Drops a mutex guard: unlocks the mutex. When a panic is unwinding, the
/// value's type's `invariant` is checked first, and a broken one poisons
/// the mutex (D-077).
///
/// # Safety
/// `value` must be the address of a locked mutex's value, of type `t`.
pub(crate) unsafe fn unlock(value: *mut u8, check: Option<u32>) -> u8 {
    let mut holds = true;
    let mut status = abi::OK;
    if let Some(f) = check
        && rt::unwinding()
    {
        match rt::function(f) {
            Some(code) => {
                let mut result = 0u64;
                let mut arg = value as u64;
                // The interpreter pushes the check's frame from a drop: its
                // entry is not a yield point.
                // SAFETY: the check takes a reference to the value and gives
                // a `bool`.
                status = sched::neutral(|| unsafe {
                    code(&mut result as *mut u64 as *mut u8, &mut arg as *mut u64 as *mut u8)
                });
                holds = result != 0;
            }
            None => rt::internal("an invariant check missing from the function table"),
        }
    }
    // SAFETY: the header is just before the value.
    unsafe {
        let header = value.sub(abi::MUTEX_HEADER as usize);
        if !holds {
            mem::set_word(header, abi::MUTEX_POISONED, 1);
        }
        mem::set_word(header, abi::MUTEX_LOCKED, 0);
    }
    system(|| with(|s| s.wake_key(Key::Lock)));
    status
}

/// The trace path of a task, for its handle's `Debug`.
pub(crate) fn task_path(t: u32) -> String {
    with(|s| s.tasks.get(t as usize).map(|t| t.path.clone()).unwrap_or_default())
}
