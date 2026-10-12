//! Software stack-limit probes, simulating the check a compiler would insert at
//! function entry for strategies B and C (and the worker-stack check for E).
//!
//! A probe compares the stack pointer with a per-task limit held in a register-
//! sized value that the task body passes down (as a compiler would keep the task
//! pointer in a register). It is not read from a thread-local on every call:
//! a stackful task can migrate between worker threads at any yield, and a
//! thread-local address cached across a yield would read another thread's value.
//!
//! On failure the probe raises a [`StackExhausted`] panic with
//! `resume_unwind`, which skips the panic hook and needs about 2 KiB of stack
//! (measured on x86-64 Linux with Rust 1.97), so the unwinder runs inside the
//! probe reserve and every destructor on the task's stack runs.
//!
//! Benchmark bodies are generic over [`StackProbe`], so the "with probes" and
//! "without probes" variants of `probe-overhead` are the same code monomorphised
//! twice.

use std::cell::Cell;

/// Payload of the panic a failing probe raises.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StackExhausted {
    /// Stack pointer when the probe fired.
    pub sp: usize,
    /// The limit it was compared against.
    pub limit: usize,
}

/// The current stack pointer.
#[inline(always)]
pub fn current_sp() -> usize {
    #[cfg(target_arch = "x86_64")]
    {
        let sp: usize;
        // SAFETY: reads the stack pointer register; no memory access, no side effects.
        unsafe { std::arch::asm!("mov {}, rsp", out(reg) sp, options(nomem, nostack, preserves_flags)) };
        sp
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        let marker = 0u8;
        std::hint::black_box(&marker) as *const u8 as usize
    }
}

/// A stack check placed at the entry of every function of a task body.
pub trait StackProbe: Copy + Send + Sync + 'static {
    /// Whether this probe does anything (for reporting).
    const ENABLED: bool;
    /// Panics with [`StackExhausted`] when the stack is (nearly) exhausted.
    fn check(self);
}

/// No probe: strategies with a guard page (A, D(A)), and the baseline of
/// `probe-overhead`.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoProbe;

impl StackProbe for NoProbe {
    const ENABLED: bool = false;
    #[inline(always)]
    fn check(self) {}
}

/// Compares the stack pointer with a fixed per-task limit (B, C, D(B)).
#[derive(Clone, Copy, Debug)]
pub struct LimitProbe {
    limit: usize,
}

impl LimitProbe {
    /// A probe that fires when the stack pointer drops below `limit`.
    pub const fn new(limit: usize) -> LimitProbe {
        LimitProbe { limit }
    }

    /// A probe that never fires.
    pub const fn disabled() -> LimitProbe {
        LimitProbe { limit: 0 }
    }

    pub fn limit(&self) -> usize {
        self.limit
    }
}

impl StackProbe for LimitProbe {
    const ENABLED: bool = true;
    #[inline(always)]
    fn check(self) {
        let sp = current_sp();
        if sp < self.limit {
            exhausted(sp, self.limit);
        }
    }
}

#[cold]
#[inline(never)]
fn exhausted(sp: usize, limit: usize) -> ! {
    std::panic::resume_unwind(Box::new(StackExhausted { sp, limit }))
}

thread_local! {
    /// Lowest safe stack pointer of the current worker thread, for stackless tasks
    /// whose nested `poll` calls recurse on the worker's own stack. Zero means unknown.
    static WORKER_LIMIT: Cell<usize> = const { Cell::new(0) };
}

/// Records the current thread's stack bounds, keeping `reserve` bytes above the
/// bottom for the unwinder; called by worker threads at start.
pub fn init_worker_limit(reserve: usize) {
    if let Some((low, _high)) = thread_stack_bounds() {
        WORKER_LIMIT.with(|l| l.set(low + reserve));
    }
}

/// The limit recorded by [`init_worker_limit`] for this thread, if any.
pub fn worker_limit() -> usize {
    WORKER_LIMIT.with(|l| l.get())
}

/// Probe for stackless (E) bodies: reads the current worker thread's limit at each
/// check. Safe for futures, which never move between threads during one `poll`.
#[derive(Clone, Copy, Debug, Default)]
pub struct WorkerProbe;

impl StackProbe for WorkerProbe {
    const ENABLED: bool = true;
    #[inline(always)]
    fn check(self) {
        let limit = worker_limit_slow();
        let sp = current_sp();
        if sp < limit {
            exhausted(sp, limit);
        }
    }
}

#[inline(never)]
fn worker_limit_slow() -> usize {
    WORKER_LIMIT.with(|l| l.get())
}

/// `(lowest, highest)` address of the calling thread's stack.
pub fn thread_stack_bounds() -> Option<(usize, usize)> {
    // SAFETY: `attr` is initialised by pthread_getattr_np before use and destroyed
    // after; the out-pointers are valid locals.
    unsafe {
        let mut attr: libc::pthread_attr_t = std::mem::zeroed();
        if libc::pthread_getattr_np(libc::pthread_self(), &mut attr) != 0 {
            return None;
        }
        let mut addr: *mut libc::c_void = std::ptr::null_mut();
        let mut size: libc::size_t = 0;
        let ok = libc::pthread_attr_getstack(&attr, &mut addr, &mut size) == 0;
        libc::pthread_attr_destroy(&mut attr);
        if ok { Some((addr as usize, addr as usize + size)) } else { None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limit_probe_fires_below_limit() {
        let sp = current_sp();
        LimitProbe::new(sp - 4096).check(); // well above the limit: no panic
        let r = std::panic::catch_unwind(|| LimitProbe::new(usize::MAX).check());
        let payload = r.expect_err("probe should fire");
        let se = payload.downcast_ref::<StackExhausted>().expect("StackExhausted payload");
        assert_eq!(se.limit, usize::MAX);
        NoProbe.check();
        LimitProbe::disabled().check();
    }

    #[test]
    fn worker_bounds_known() {
        let (lo, hi) = thread_stack_bounds().expect("bounds");
        let sp = current_sp();
        assert!(lo < sp && sp < hi, "sp {sp:#x} outside {lo:#x}..{hi:#x}");
        init_worker_limit(64 * 1024);
        assert_eq!(worker_limit(), lo + 64 * 1024);
        WorkerProbe.check();
    }
}
