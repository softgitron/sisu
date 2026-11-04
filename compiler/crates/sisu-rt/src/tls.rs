//! The words generated code reaches by symbol, one copy for each thread
//! (decision 332): the count of nested calls ([`abi::sym::DEPTH`]), the
//! yield points left ([`abi::sym::BUDGET`]), the running task's stack
//! limits ([`abi::sym::STACK`]) and where it last called an intrinsic that
//! may wait ([`abi::sym::SITE`]), and the runtime's own word, the stack
//! pointer of the generated code's last call of an intrinsic
//! ([`caller_sp`]). They are the executable's thread-local
//! data (`.tbss`), so that Sisu code running on a worker of `std.par`
//! (decision 333) counts its own calls, its own budget and its own stack.
//!
//! Stable Rust cannot give a thread-local variable a symbol of its own, so
//! they are defined in assembly, and the runtime reaches them as the code
//! both backends write does: the thread pointer plus the variable's offset
//! from it, read from the GOT (the initial-exec model, which the linker
//! relaxes to a constant in an executable).

use crate::abi;
use std::sync::atomic::{AtomicI64, AtomicU64};

#[cfg(all(target_os = "linux", any(target_arch = "x86_64", target_arch = "aarch64")))]
std::arch::global_asm!(
    ".pushsection .tbss,\"awT\",%nobits",
    ".p2align 3",
    ".globl SISU_RT_DEPTH",
    ".type SISU_RT_DEPTH, %object",
    ".size SISU_RT_DEPTH, 8",
    "SISU_RT_DEPTH:",
    ".zero 8",
    ".globl SISU_RT_BUDGET",
    ".type SISU_RT_BUDGET, %object",
    ".size SISU_RT_BUDGET, 8",
    "SISU_RT_BUDGET:",
    ".zero 8",
    ".globl SISU_RT_STACK",
    ".type SISU_RT_STACK, %object",
    ".size SISU_RT_STACK, {stack}",
    "SISU_RT_STACK:",
    ".zero {stack}",
    ".globl SISU_RT_SITE",
    ".type SISU_RT_SITE, %object",
    ".size SISU_RT_SITE, {site}",
    "SISU_RT_SITE:",
    ".zero {site}",
    ".globl SISU_RT_CALLER_SP",
    ".type SISU_RT_CALLER_SP, %object",
    ".size SISU_RT_CALLER_SP, 8",
    "SISU_RT_CALLER_SP:",
    ".zero 8",
    ".popsection",
    stack = const abi::STACK_WORDS * 8,
    site = const abi::SITE_WORDS * 8,
);

/// The address of the running thread's copy of the thread-local symbol
/// `$name`.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
macro_rules! tls_address {
    ($name:literal) => {{
        let a: usize;
        // SAFETY: the initial-exec sequence: the thread pointer (the TCB's
        // first word holds it) plus the symbol's offset from it, from the
        // GOT. It reads the thread's control block and the GOT, writes
        // nothing, and gives the same address on a thread every time.
        unsafe {
            std::arch::asm!(
                "mov {a}, qword ptr fs:[0]",
                concat!("add {a}, qword ptr [rip + ", $name, "@GOTTPOFF]"),
                a = out(reg) a,
                options(nostack, pure, readonly),
            );
        }
        a
    }};
}

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
macro_rules! tls_address {
    ($name:literal) => {{
        let a: usize;
        // SAFETY: the initial-exec sequence: the thread pointer plus the
        // symbol's offset from it, from the GOT. It reads a system register
        // and the GOT and writes nothing.
        unsafe {
            std::arch::asm!(
                "mrs {a}, tpidr_el0",
                concat!("adrp {t}, :gottprel:", $name),
                concat!("ldr {t}, [{t}, :gottprel_lo12:", $name, "]"),
                "add {a}, {a}, {t}",
                a = out(reg) a,
                t = out(reg) _,
                options(nostack, pure, readonly),
            );
        }
        a
    }};
}

/// Elsewhere the words are ordinary data, which only a program of one
/// thread can share with its code.
#[cfg(not(all(target_os = "linux", any(target_arch = "x86_64", target_arch = "aarch64"))))]
mod plain {
    use std::sync::atomic::{AtomicI64, AtomicU64};
    #[unsafe(no_mangle)]
    pub static SISU_RT_DEPTH: AtomicI64 = AtomicI64::new(0);
    #[unsafe(no_mangle)]
    pub static SISU_RT_BUDGET: AtomicI64 = AtomicI64::new(0);
    #[unsafe(no_mangle)]
    pub static SISU_RT_STACK: [AtomicU64; super::abi::STACK_WORDS] =
        [const { AtomicU64::new(0) }; super::abi::STACK_WORDS];
    #[unsafe(no_mangle)]
    pub static SISU_RT_SITE: [AtomicU64; super::abi::SITE_WORDS] =
        [const { AtomicU64::new(0) }; super::abi::SITE_WORDS];
    #[unsafe(no_mangle)]
    pub static SISU_RT_CALLER_SP: AtomicU64 = AtomicU64::new(0);
}

#[cfg(not(all(target_os = "linux", any(target_arch = "x86_64", target_arch = "aarch64"))))]
macro_rules! tls_address {
    ("SISU_RT_DEPTH") => {
        &plain::SISU_RT_DEPTH as *const _ as usize
    };
    ("SISU_RT_BUDGET") => {
        &plain::SISU_RT_BUDGET as *const _ as usize
    };
    ("SISU_RT_STACK") => {
        &plain::SISU_RT_STACK as *const _ as usize
    };
    ("SISU_RT_SITE") => {
        &plain::SISU_RT_SITE as *const _ as usize
    };
    ("SISU_RT_CALLER_SP") => {
        &plain::SISU_RT_CALLER_SP as *const _ as usize
    };
}

/// The running thread's count of nested calls.
pub(crate) fn depth() -> &'static AtomicI64 {
    let a = tls_address!("SISU_RT_DEPTH");
    // SAFETY: the address is this thread's copy of an 8-byte word aligned
    // to 8, which lives as long as the thread; the runtime uses it on the
    // thread that asked, and only through atomics, as the generated code
    // reads and writes it with plain loads and stores of the same thread.
    unsafe { &*(a as *const AtomicI64) }
}

/// The yield points the running thread's task may pass before it asks the
/// scheduler.
pub(crate) fn budget() -> &'static AtomicI64 {
    let a = tls_address!("SISU_RT_BUDGET");
    // SAFETY: as for [`depth`].
    unsafe { &*(a as *const AtomicI64) }
}

/// The running task's stack limits, by [`abi::STACK_PROBE`] and the others.
pub(crate) fn stack() -> &'static [AtomicU64; abi::STACK_WORDS] {
    let a = tls_address!("SISU_RT_STACK");
    // SAFETY: as for [`depth`]: `STACK_WORDS` words of this thread.
    unsafe { &*(a as *const [AtomicU64; abi::STACK_WORDS]) }
}

/// Where the running code last called an intrinsic that may wait, by
/// [`abi::SITE_FILE`] and the others.
pub(crate) fn site() -> &'static [AtomicU64; abi::SITE_WORDS] {
    let a = tls_address!("SISU_RT_SITE");
    // SAFETY: as for [`depth`]: `SITE_WORDS` words of this thread.
    unsafe { &*(a as *const [AtomicU64; abi::SITE_WORDS]) }
}

/// The stack pointer of the generated code where it last called
/// `sisu_rt_intrinsic` on this thread, which that entry writes before
/// anything else runs (decision 560); 0 where the target has no such entry.
/// Every frame of the running task that is still live lies at or above it.
pub(crate) fn caller_sp() -> u64 {
    let a = tls_address!("SISU_RT_CALLER_SP");
    // SAFETY: as for [`depth`].
    unsafe { &*(a as *const AtomicU64) }.load(std::sync::atomic::Ordering::Relaxed)
}
