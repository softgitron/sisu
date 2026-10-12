//! Turning a guard-page fault into a task failure (strategies A and D(A)).
//!
//! A task that runs off the bottom of its stack touches the `PROT_NONE` guard and
//! the kernel raises SIGSEGV on the worker thread. The handler runs on a per-worker
//! alternate signal stack, checks that the faulting address is in the guard of the
//! task the worker is running, and uses corosensei's trap support to make the
//! coroutine return `TaskExit::GuardTrap` when the handler returns. The worker then
//! fails the task with "stack exhausted" and the process carries on.
//!
//! What this does not do: run the destructors of the frames on the exhausted
//! stack. They are discarded, as by `longjmp`. Unwinding instead would need
//! exception tables that cover the faulting instruction (a function prologue,
//! not a call site), which Rust's tables do not; a Sisu backend that wants guard
//! pages *and* unwinding needs non-call exception tables at its probe points.
//! Locks held by discarded frames stay held, so task bodies must not overflow
//! inside runtime code; the benchmark bodies recurse only in their own code.
//!
//! Faults anywhere else are passed to the previously installed handler (Rust's
//! standard library reports overflow of OS thread stacks; otherwise the default
//! action terminates the process with a core dump, as it should for a real bug).

use std::cell::Cell;
use std::io;
use std::sync::OnceLock;

use corosensei::trap::{CoroutineTrapHandler, TrapHandlerRegs};

use crate::rt::TaskExit;
use crate::stack::os;

/// What the signal handler needs to know about the task running on this thread.
#[derive(Clone, Copy)]
pub(crate) struct ArmedTrap {
    pub handler: CoroutineTrapHandler<TaskExit>,
    pub guard_lo: usize,
    pub guard_hi: usize,
}

thread_local! {
    // Const-initialised and `Copy`: reading it from a signal handler does no lazy
    // initialisation and registers no destructor.
    static ARMED: Cell<Option<ArmedTrap>> = const { Cell::new(None) };
}

/// Arms the handler for the task about to be resumed on this thread.
pub(crate) fn arm(trap: ArmedTrap) {
    ARMED.with(|a| a.set(Some(trap)));
}

/// Disarms it after the task suspended or finished.
pub(crate) fn disarm() {
    ARMED.with(|a| a.set(None));
}

struct OldActions {
    segv: libc::sigaction,
    bus: libc::sigaction,
}

// SAFETY: `sigaction` is plain data (a function address, flags and a signal set);
// it is written once before the handler is installed and only read afterwards.
unsafe impl Sync for OldActions {}
// SAFETY: as above.
unsafe impl Send for OldActions {}

static OLD: OnceLock<OldActions> = OnceLock::new();

/// Installs the SIGSEGV/SIGBUS handler once per process.
pub(crate) fn install_handler() {
    OLD.get_or_init(|| {
        // SAFETY: zeroed sigaction structs are valid "empty" values; sigaction() is
        // called with valid pointers; the handler below is async-signal-safe (it
        // reads a const thread-local, edits the interrupted context and calls
        // sigaction).
        unsafe {
            let mut act: libc::sigaction = std::mem::zeroed();
            act.sa_sigaction = handle_fault as *const () as usize;
            act.sa_flags = libc::SA_SIGINFO | libc::SA_ONSTACK;
            libc::sigemptyset(&mut act.sa_mask);
            let mut segv: libc::sigaction = std::mem::zeroed();
            let mut bus: libc::sigaction = std::mem::zeroed();
            libc::sigaction(libc::SIGSEGV, &act, &mut segv);
            libc::sigaction(libc::SIGBUS, &act, &mut bus);
            OldActions { segv, bus }
        }
    });
}

unsafe extern "C" fn handle_fault(sig: libc::c_int, info: *mut libc::siginfo_t, ctx: *mut libc::c_void) {
    // SAFETY: the kernel passes valid siginfo and ucontext pointers to an
    // SA_SIGINFO handler.
    let addr = unsafe { (*info).si_addr() } as usize;
    if let Some(trap) = ARMED.with(|a| a.get()) {
        let uc = ctx as *mut libc::ucontext_t;
        // SAFETY: `uc` is the interrupted context of this thread.
        let sp = unsafe { (*uc).uc_mcontext.gregs[libc::REG_RSP as usize] } as usize;
        if addr >= trap.guard_lo && addr < trap.guard_hi && trap.handler.stack_ptr_in_bounds(sp) {
            ARMED.with(|a| a.set(None));
            // SAFETY: the fault is a stack overflow of the coroutine `trap` was armed
            // for, which is the one running on this thread, and its stack pointer is
            // inside its stack. The closure is 'static and only builds a value.
            // The stack is reset and its frames discarded (see the module docs).
            let regs: TrapHandlerRegs = unsafe { trap.handler.setup_trap_handler(move || TaskExit::GuardTrap { fault_addr: addr }) };
            let TrapHandlerRegs { rip, rsp, rbp, rdi, rsi } = regs;
            // SAFETY: editing the saved registers of the interrupted context is how a
            // handler redirects execution; corosensei specifies these five.
            unsafe {
                let g = &mut (*uc).uc_mcontext.gregs;
                g[libc::REG_RIP as usize] = rip as i64;
                g[libc::REG_RSP as usize] = rsp as i64;
                g[libc::REG_RBP as usize] = rbp as i64;
                g[libc::REG_RDI as usize] = rdi as i64;
                g[libc::REG_RSI as usize] = rsi as i64;
            }
            return;
        }
    }
    // Not ours: reinstate the previous disposition and return, so the faulting
    // instruction runs again under it.
    if let Some(old) = OLD.get() {
        let prev = if sig == libc::SIGBUS { &old.bus } else { &old.segv };
        // SAFETY: restoring a previously valid disposition.
        unsafe { libc::sigaction(sig, prev, std::ptr::null_mut()) };
    } else {
        // SAFETY: resetting to the default action is always valid.
        unsafe { libc::signal(sig, libc::SIG_DFL) };
    }
}

/// A per-thread alternate signal stack, so the handler can run when the task stack
/// is exhausted. Restores the previous alternate stack when dropped.
pub(crate) struct AltStack {
    addr: usize,
    len: usize,
    old: libc::stack_t,
}

impl AltStack {
    const SIZE: usize = 64 * 1024;

    /// Installs a 64 KiB alternate signal stack (plus a guard page) for this thread.
    pub(crate) fn install() -> io::Result<AltStack> {
        let page = os::page_size();
        let len = Self::SIZE + page;
        let addr = os::reserve(len)?;
        // SAFETY: the low page of the mapping just created becomes its guard.
        unsafe { os::protect_none(addr, page)? };
        let st = libc::stack_t { ss_sp: (addr + page) as *mut libc::c_void, ss_flags: 0, ss_size: Self::SIZE };
        // SAFETY: zeroed stack_t is a valid out-parameter.
        let mut old: libc::stack_t = unsafe { std::mem::zeroed() };
        // SAFETY: `st` describes memory owned by this AltStack for its lifetime.
        if unsafe { libc::sigaltstack(&st, &mut old) } != 0 {
            let e = io::Error::last_os_error();
            // SAFETY: mapping created above, unused.
            let _ = unsafe { os::unmap(addr, len) };
            return Err(e);
        }
        Ok(AltStack { addr, len, old })
    }
}

impl Drop for AltStack {
    fn drop(&mut self) {
        // SAFETY: restores the stack that was installed before ours (or disables
        // the alternate stack), then unmaps ours, which no handler is running on
        // because we are on the thread's normal stack.
        unsafe {
            if self.old.ss_sp.is_null() || self.old.ss_flags & libc::SS_DISABLE != 0 {
                let off = libc::stack_t { ss_sp: std::ptr::null_mut(), ss_flags: libc::SS_DISABLE, ss_size: 0 };
                libc::sigaltstack(&off, std::ptr::null_mut());
            } else {
                libc::sigaltstack(&self.old, std::ptr::null_mut());
            }
            let _ = os::unmap(self.addr, self.len);
        }
    }
}
