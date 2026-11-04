//! The counter of yield points (`SISU_RT_BUDGET`, [`crate::tls::budget`]),
//! which holds two budgets at once (decision 540): the seeded scheduler's,
//! the yield points the running task may pass before the scheduler is
//! asked (decision 204), and a property's case's, the yield points a call
//! `test.catch(f, x, steps)` may pass before the case is `TooLong`.
//!
//! Generated code counts the word down and calls `sisu_rt_yield` when it
//! reaches zero. While a case is open the word holds the smaller of the
//! two budgets and the rest of the scheduler's is kept aside, so that the
//! scheduler's budget, the word plus what is aside, means what it means
//! without a case and a seed names the same schedule; the yield points the
//! word lost since it was last written are taken from every open case.
//! Every read and write of the word but the generated code's goes through
//! [`load`], [`store`] and [`spend`].

use std::cell::RefCell;
use std::sync::atomic::Ordering::Relaxed;

/// A case of a property (`test.catch`) being run.
struct Case {
    /// The yield points it may still pass.
    left: i64,
    /// Its budget, for `TooLong(steps)`.
    steps: i64,
    /// Whether it ran out: its panic unwinds it, without a budget.
    fired: bool,
}

struct Cases {
    /// The open cases, the innermost last.
    open: Vec<Case>,
    /// The scheduler's budget beyond the word's value.
    aside: i64,
    /// The word's value when it was last written or counted.
    mark: i64,
}

thread_local! {
    static CASES: RefCell<Cases> = const { RefCell::new(Cases { open: Vec::new(), aside: 0, mark: 0 }) };
}

fn word() -> &'static std::sync::atomic::AtomicI64 {
    crate::tls::budget()
}

/// Takes the yield points passed since the word was last counted from
/// every open case.
fn count(c: &mut Cases) {
    if c.open.is_empty() {
        return;
    }
    let now = word().load(Relaxed);
    let passed = c.mark - now;
    c.mark = now;
    for k in &mut c.open {
        k.left = k.left.saturating_sub(passed);
    }
}

/// Writes the scheduler's budget `v`: the word holds the innermost case's
/// budget left when that is smaller, the rest aside.
fn set(c: &mut Cases, v: i64) {
    let cap = c.open.last().filter(|k| !k.fired).map(|k| k.left);
    let held = cap.map_or(v, |l| v.min(l));
    c.aside = v - held;
    c.mark = held;
    word().store(held, Relaxed);
}

/// The scheduler's budget.
pub(crate) fn load() -> i64 {
    CASES.with(|c| {
        let mut c = c.borrow_mut();
        count(&mut c);
        word().load(Relaxed) + c.aside
    })
}

/// Sets the scheduler's budget.
pub(crate) fn store(v: i64) {
    CASES.with(|c| {
        let mut c = c.borrow_mut();
        count(&mut c);
        set(&mut c, v);
    })
}

/// A yield point the runtime passes itself (a spawn, `task.yield_now`).
pub(crate) fn spend() {
    word().fetch_sub(1, Relaxed);
}

/// A case of `steps` yield points begins.
pub(crate) fn open(steps: i64) {
    CASES.with(|c| {
        let mut c = c.borrow_mut();
        count(&mut c);
        let v = word().load(Relaxed) + c.aside;
        c.open.push(Case { left: steps.max(1), steps: steps.max(1), fired: false });
        set(&mut c, v);
    })
}

/// The innermost case ends: whether it ran out.
pub(crate) fn close() -> bool {
    CASES.with(|c| {
        let mut c = c.borrow_mut();
        count(&mut c);
        let v = word().load(Relaxed) + c.aside;
        let fired = c.open.pop().is_some_and(|k| k.fired);
        set(&mut c, v);
        fired
    })
}

/// What a yield point that found the word spent is.
pub(crate) enum AtYield {
    /// The innermost case ran out: its budget, for the panic.
    TooLong(i64),
    /// The case's budget only was spent: nothing for the scheduler.
    Held,
    /// The scheduler's budget is spent.
    Spent,
}

/// Decides a yield point that found the word spent; `unwinding` says a
/// panic is in flight, which a case running out does not interrupt.
pub(crate) fn at_yield(unwinding: bool) -> AtYield {
    CASES.with(|c| {
        let mut c = c.borrow_mut();
        if c.open.is_empty() {
            return AtYield::Spent;
        }
        count(&mut c);
        let v = word().load(Relaxed) + c.aside;
        if !unwinding
            && let Some(k) = c.open.last_mut()
            && !k.fired
            && k.left <= 0
        {
            k.fired = true;
            let steps = k.steps;
            set(&mut c, v);
            return AtYield::TooLong(steps);
        }
        if v > 0 {
            set(&mut c, v);
            return AtYield::Held;
        }
        AtYield::Spent
    })
}
