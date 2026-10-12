//! Software stack-limit probes, simulating the check a compiler would insert at
//! function entry for strategies B and C (and the worker-stack check for E).
//!
//! The probe task bodies use is [`SplitStackProbe`]: `cmp rsp, fs:[0x70]; jb`,
//! the prologue GCC's `-fsplit-stack` emits. On Linux x86-64 with glibc, the
//! thread control block reserves `tcbhead_t.__private_ss` at `%fs:0x70` for exactly
//! this; nothing in this process uses split stacks, so the slot is the harness's.
//! The scheduler writes the running task's limit into the slot every time it
//! resumes a task on a thread, so the check is correct however often a task
//! migrates: the instruction addresses the slot through the `fs` segment each
//! time, and no thread-local address is ever held in a register.
//!
//! Two other models are kept for `probe-overhead`: [`LimitProbe`] passes the limit
//! down as an argument, and [`TlsProbe`] reads a Rust `thread_local!`. The latter
//! is *not* migration-safe: LLVM materialises the thread-local's address and keeps
//! it in a register across calls.
//!
//! On failure the probe raises a [`StackExhausted`] panic with
//! `resume_unwind`, which skips the panic hook. The unwinder then runs inside the
//! probe reserve and every destructor on the task's stack runs. How much reserve
//! that takes is measured by `deep-recursion` (`*_probe_reserve_used`): about
//! 1.8 KiB from frames without destructors and about 5 KiB when landing pads in
//! the deepest frames run drop code (x86-64 Linux, Rust 1.97).
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

/// No probe: the baseline of `probe-overhead`.
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
        check_against(self.limit);
    }
}

/// `cmp rsp, limit; jb fail`: the two instructions a compiler emits.
#[inline(always)]
fn check_against(limit: usize) {
    #[cfg(target_arch = "x86_64")]
    // SAFETY: compares the stack pointer with a register and branches to a cold
    // Rust block; touches no memory and does not change the stack.
    unsafe {
        std::arch::asm!(
            "cmp rsp, {limit}",
            "jb {fail}",
            limit = in(reg) limit,
            fail = label { exhausted(current_sp(), limit) },
            options(nomem, nostack),
        );
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        let sp = current_sp();
        if sp < limit {
            exhausted(sp, limit);
        }
    }
}

#[cold]
#[inline(never)]
fn exhausted(sp: usize, limit: usize) -> ! {
    std::panic::resume_unwind(Box::new(StackExhausted { sp, limit }))
}

/// Writes this thread's split-stack limit slot (`%fs:0x70`). Zero disables
/// [`SplitStackProbe`] on this thread.
#[inline]
pub fn set_split_stack_limit(limit: usize) {
    #[cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]
    // SAFETY: glibc reserves `tcbhead_t.__private_ss` at %fs:0x70 for split-stack
    // support; no code in this process uses split stacks, so writing it affects
    // only `SplitStackProbe`. The slot is per thread.
    unsafe {
        std::arch::asm!("mov qword ptr fs:[0x70], {}", in(reg) limit, options(nostack, preserves_flags));
    }
    #[cfg(not(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu")))]
    TASK_LIMIT.with(|l| l.set(limit));
}

/// Reads this thread's split-stack limit slot.
#[inline]
pub fn split_stack_limit() -> usize {
    #[cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]
    {
        let v: usize;
        // SAFETY: reads the reserved slot described in `set_split_stack_limit`.
        unsafe { std::arch::asm!("mov {}, qword ptr fs:[0x70]", out(reg) v, options(nostack, readonly, preserves_flags)) };
        v
    }
    #[cfg(not(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu")))]
    TASK_LIMIT.with(|l| l.get())
}

/// The probe task bodies use: one compare of the stack pointer against the
/// split-stack slot that the scheduler keeps equal to the running task's limit
/// (zero, never firing, for guard-page strategies).
#[derive(Clone, Copy, Debug, Default)]
pub struct SplitStackProbe;

impl StackProbe for SplitStackProbe {
    const ENABLED: bool = true;
    #[inline(always)]
    fn check(self) {
        #[cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]
        // SAFETY: compares rsp with the reserved TCB slot and branches to a cold
        // Rust block; does not write memory or touch the stack.
        unsafe {
            std::arch::asm!(
                "cmp rsp, qword ptr fs:[0x70]",
                "jb {fail}",
                fail = label { exhausted(current_sp(), split_stack_limit()) },
                options(readonly, nostack),
            );
        }
        #[cfg(not(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu")))]
        check_against(split_stack_limit());
    }
}

thread_local! {
    /// The running task's probe limit for [`TlsProbe`].
    static TASK_LIMIT: Cell<usize> = const { Cell::new(0) };
}

/// Sets the limit [`TlsProbe`] compares against on this thread.
pub fn set_tls_limit(limit: usize) {
    TASK_LIMIT.with(|l| l.set(limit));
}

/// The cost model of a compiler-inserted prologue check: one compare of the stack
/// pointer against a thread-local slot (GCC's `-fsplit-stack` emits
/// `cmp %fs:0x70,%rsp; jb __morestack`; Go compares against `g.stackguard0` through
/// a pinned register). No extra argument is threaded through calls.
///
/// Only valid where the task cannot move to another thread between setting the
/// slot and the check: LLVM materialises the thread-local's address and keeps it
/// in a callee-saved register across calls (visible in the disassembly of
/// `work::fib::<TlsProbe>`), so after a migration the check would read the old
/// thread's slot. `probe-overhead` uses it inside one task that never yields. A
/// real compiler would treat every suspension point as invalidating the slot.
#[derive(Clone, Copy, Debug, Default)]
pub struct TlsProbe;

impl StackProbe for TlsProbe {
    const ENABLED: bool = true;
    #[inline(always)]
    fn check(self) {
        check_against(TASK_LIMIT.with(|l| l.get()));
    }
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
        set_split_stack_limit(low + reserve);
    }
}

/// The limit recorded by [`init_worker_limit`] for this thread, if any.
pub fn worker_limit() -> usize {
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
    fn split_stack_slot_probe() {
        let old = split_stack_limit();
        set_split_stack_limit(current_sp() - 64 * 1024);
        SplitStackProbe.check();
        set_split_stack_limit(usize::MAX);
        let r = std::panic::catch_unwind(|| SplitStackProbe.check());
        set_split_stack_limit(old);
        assert!(r.expect_err("must fire").downcast_ref::<StackExhausted>().is_some());
        assert_eq!(split_stack_limit(), old);
    }

    #[test]
    fn worker_bounds_known() {
        let (lo, hi) = thread_stack_bounds().expect("bounds");
        let sp = current_sp();
        assert!(lo < sp && sp < hi, "sp {sp:#x} outside {lo:#x}..{hi:#x}");
        let old = split_stack_limit();
        init_worker_limit(64 * 1024);
        assert_eq!(worker_limit(), lo + 64 * 1024);
        assert_eq!(split_stack_limit(), lo + 64 * 1024);
        SplitStackProbe.check();
        set_split_stack_limit(old);
    }
}
