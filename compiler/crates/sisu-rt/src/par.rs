//! `std.par`'s `map` natively (06, "Data parallelism"; D-069; decision
//! 333): the bodies run on a pool of worker threads apart from the task
//! scheduler, each to completion on a stack of its own, and each result is
//! written at its element's place, so the list is the one the elision
//! (running the bodies one after another, as the interpreter does) gives.
//!
//! A body is `pure`: it neither waits nor reaches another task, so where
//! it runs is not observable but in time. It counts its calls from the
//! asking task's depth, on its own copy of the words the generated code
//! reaches (decision 332), and spends no yield point of the asking task's
//! budget. A worker keeps a scheduler of its own, which never switches, for
//! the generators a body drives. When bodies panic, the panic of the
//! lowest index is the call's, as the elision raises the first: the results
//! before it are dropped as the elision's unwinding drops them, the others
//! freed without running Sisu code, and the other panics forgotten. A call
//! made on a worker (a body that maps) runs its bodies there, one after
//! another.
//!
//! `SISU_PAR_THREADS=N` sets the number of workers (1 to 256; by default
//! the machine's processors): the result is the same for every number.

use crate::abi;
use crate::intrinsics::Call;
use crate::mem;
use crate::rt::{self, Panic, ty};
use crate::value;
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::sync::{Arc, Condvar, Mutex, OnceLock};

/// A closure's code called with its result's address, its environment and
/// one argument.
type Body1 = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8) -> u8;

/// What a worker's stack keeps below its probe limit beyond the reserve,
/// as the root task's does.
const MARGIN: usize = 256 * 1024;

/// The most workers.
const MAX_WORKERS: usize = 256;

/// What a call may write below a probe limit before a probe sees it, which
/// the program's `main` gives the runtime (decision 234).
static UNPROBED: AtomicUsize = AtomicUsize::new(0);

pub(crate) fn set_unprobed(n: usize) {
    UNPROBED.store(n, Relaxed);
}

thread_local! {
    /// Whether this thread is one of the pool's workers.
    static WORKER: Cell<bool> = const { Cell::new(false) };
}

/// One call's bodies, shared with the workers: where the elements and the
/// results are, the next element to run, and the elements that failed.
struct Batch {
    code: usize,
    env: usize,
    items: usize,
    item_size: usize,
    out: usize,
    out_size: usize,
    n: usize,
    /// The asking task's count of nested calls, which `map`'s own frame is
    /// part of.
    depth: i64,
    next: AtomicUsize,
    /// The elements whose body panicked, each with its panic.
    failed: Mutex<Vec<(usize, Option<Panic>)>>,
    /// The elements run so far, and the asking thread waiting for all.
    finished: Mutex<usize>,
    all_done: Condvar,
}

impl Batch {
    /// Runs the body of element `i` on this thread, as a call of the
    /// closure with a reference to the element.
    ///
    /// # Safety
    /// The batch's addresses must be the asking call's, alive while it
    /// waits, and `i` below `n`.
    unsafe fn run(&self, i: usize) -> Option<Option<Panic>> {
        let mut arg = (self.items + i * self.item_size) as u64;
        let ret = (self.out + i * self.out_size) as *mut u8;
        // SAFETY: the code is a closure's of `pure (ref T) -> U`, which
        // takes its result's address, its environment and the address of
        // its argument, here a reference to element `i`; the result's room
        // is element `i`'s of the results.
        let st = unsafe {
            std::mem::transmute::<usize, Body1>(self.code)(
                ret,
                self.env as *mut u8,
                &mut arg as *mut u64 as *mut u8,
            )
        };
        if st == abi::OK {
            return None;
        }
        // The body's panic, and nothing else left in flight here.
        let p = rt::take_panic();
        let mut rest = Vec::new();
        rt::swap_panics(&mut rest);
        Some(p)
    }
}

/// The workers and the batch in flight.
struct Pool {
    /// The batch being run, with a number that changes for each, so that a
    /// worker takes each batch once.
    work: Mutex<(u64, Option<Arc<Batch>>)>,
    wake: Condvar,
    workers: usize,
}

static POOL: OnceLock<Option<Pool>> = OnceLock::new();

/// The pool, started at the first call: `None` if no worker could start.
fn pool() -> Option<&'static Pool> {
    POOL.get_or_init(|| {
        let workers = std::env::var("SISU_PAR_THREADS")
            .ok()
            .and_then(|v| v.trim().parse::<usize>().ok())
            .filter(|&n| n >= 1)
            .unwrap_or_else(|| std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1))
            .min(MAX_WORKERS);
        Some(Pool { work: Mutex::new((0, None)), wake: Condvar::new(), workers })
    })
    .as_ref()
}

/// Starts the pool's workers, once the pool is in its static.
fn start_workers(p: &'static Pool) -> bool {
    static STARTED: OnceLock<bool> = OnceLock::new();
    *STARTED.get_or_init(|| {
        let mut any = false;
        for k in 0..p.workers {
            let spawned = std::thread::Builder::new()
                .name(format!("sisu-par-{k}"))
                .stack_size(crate::entry::STACK_BYTES)
                .spawn(move || worker(p));
            any |= spawned.is_ok();
        }
        any
    })
}

/// A worker: its own stack limits, budget and scheduler, then each batch's
/// elements as long as there are any.
fn worker(p: &'static Pool) {
    WORKER.with(|w| w.set(true));
    let unprobed = UNPROBED.load(Relaxed);
    let stacks = crate::stack::Pool::new(crate::stack::reservation_from_env(), unprobed);
    let low = crate::stack::current_sp().saturating_sub(crate::entry::STACK_BYTES) + MARGIN;
    let limits = crate::stack::limits(low, stacks.reserve, unprobed);
    crate::sched::start_worker(stacks, limits);
    let mut seen = 0u64;
    loop {
        let batch = {
            let Ok(mut w) = p.work.lock() else { return };
            loop {
                if w.0 != seen
                    && let Some(b) = &w.1
                {
                    seen = w.0;
                    break b.clone();
                }
                w = match p.wake.wait(w) {
                    Ok(g) => g,
                    Err(_) => return,
                };
            }
        };
        loop {
            let i = batch.next.fetch_add(1, Relaxed);
            if i >= batch.n {
                break;
            }
            // Each body starts as the asking call's would: at its depth,
            // on the worker's whole stack.
            crate::tls::depth().store(batch.depth, Relaxed);
            for (w, v) in crate::tls::stack().iter().zip(limits) {
                w.store(v, Relaxed);
            }
            // SAFETY: the asking call waits until every element has run.
            let failed = unsafe { batch.run(i) };
            if let Some(panic) = failed
                && let Ok(mut f) = batch.failed.lock()
            {
                f.push((i, panic));
            }
            if let Ok(mut done) = batch.finished.lock() {
                *done += 1;
                if *done == batch.n {
                    batch.all_done.notify_all();
                }
            }
        }
    }
}

/// `par.map[T, U](xs, f)`: `f` of each element of `xs`, in a list in the
/// elements' order.
///
/// # Safety
/// The arguments are a reference to a `list[T]` and a function value of
/// `pure (ref T) -> U`, and the call's type arguments `T` and `U`.
pub(crate) unsafe fn map(c: &Call<'_>) -> u8 {
    let (t, u) = (c.targ(0), c.targ(1));
    // SAFETY: as the caller guarantees: the list's three words, and the
    // function value's code and environment.
    let (items, n, code, env) = unsafe {
        let (l, _) = c.place(0);
        let (f, _) = c.place(1);
        (mem::word(l, 0) as usize, mem::word(l, 8) as usize, mem::word(f, 0) as usize, f as usize + 8)
    };
    let (item_size, out_size) = (ty(t).size as usize, ty(u).size as usize);
    let out = mem::alloc(n * out_size) as usize;
    let batch = Arc::new(Batch {
        code,
        env,
        items,
        item_size,
        out,
        out_size,
        n,
        depth: crate::tls::depth().load(Relaxed),
        next: AtomicUsize::new(0),
        failed: Mutex::new(Vec::new()),
        finished: Mutex::new(0),
        all_done: Condvar::new(),
    });
    let on_worker = WORKER.with(Cell::get);
    let pool = if on_worker || n == 0 { None } else { pool().filter(|p| start_workers(p)) };
    match pool {
        Some(p) => {
            if let Ok(mut w) = p.work.lock() {
                w.0 += 1;
                w.1 = Some(batch.clone());
            }
            p.wake.notify_all();
            if let Ok(mut done) = batch.finished.lock() {
                while *done < n {
                    done = match batch.all_done.wait(done) {
                        Ok(g) => g,
                        Err(_) => rt::internal("a worker of `std.par` failed"),
                    };
                }
            }
            if let Ok(mut w) = p.work.lock() {
                w.1 = None;
            }
        }
        None => {
            // The elision, on this thread: the first panic stops it.
            for i in 0..n {
                // SAFETY: the batch is this call's, alive meanwhile.
                if let Some(panic) = unsafe { batch.run(i) } {
                    if let Ok(mut f) = batch.failed.lock() {
                        f.push((i, panic));
                    }
                    break;
                }
            }
        }
    }
    let mut failed = batch.failed.lock().map(|mut f| std::mem::take(&mut *f)).unwrap_or_default();
    failed.sort_by_key(|(i, _)| *i);
    let Some((first, panic)) = failed.first_mut().map(|(i, p)| (*i, p.take())) else {
        // SAFETY: the result is a list's three words.
        unsafe {
            mem::set_word(c.ret, 0, out as u64);
            mem::set_word(c.ret, 8, n as u64);
            mem::set_word(c.ret, 16, n as u64);
        }
        return abi::OK;
    };
    // The lowest index's panic is the call's; the results before it are
    // dropped while it is in flight, those after it freed unseen.
    let ran: Vec<bool> = {
        let mut ran = vec![true; n];
        for (i, _) in &failed {
            ran[*i] = false;
        }
        ran
    };
    let status = match panic {
        Some(p) => rt::raise_panic(p),
        None => rt::internal("a body of `std.par.map` failed without its panic"),
    };
    let mut st = abi::OK;
    for (i, done) in ran.iter().enumerate() {
        if !*done {
            continue;
        }
        let at = (out + i * out_size) as *mut u8;
        if i < first {
            // SAFETY: body `i` returned its result here.
            st = value::after_part(st, unsafe { value::drop_value(at, u) });
            if st != abi::OK {
                rt::abort_unwinding();
            }
        } else {
            // SAFETY: as above; the elision never made this result.
            unsafe { value::forget_value(at, u) };
        }
    }
    mem::release(out as *mut u8);
    status
}
