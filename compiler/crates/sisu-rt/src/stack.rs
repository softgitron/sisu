//! The stacks tasks run on: the Phase 1 harness's strategy D(B)
//! (taskbench, `src/stack/pool.rs` and `decommit.rs`; its report's Q-6
//! recommendation and the owner's policies 5 and 6), decision 203.
//!
//! **Pooled slots.** A stack is a slot of fixed size carved from large
//! reservations of address space (`mmap` with `MAP_NORESERVE`, committed
//! by the kernel only where a page is touched), with no guard page, so a
//! million tasks are a few mappings, not a million. Freed slots go on a
//! LIFO free list. A slot holds the reservation (`SISU_TASK_STACK`, 1 MiB
//! by default) above the probe reserve.
//!
//! **Probes.** Without a guard page, what keeps a task inside its slot is
//! the probe the code generator puts at the entry of every function:
//! the stack pointer, below the function's frame, is compared with the
//! task's limit ([`abi::STACK_PROBE`]). The harness's probe compares before
//! the frame is allocated; Cranelift allocates the frame in its prologue,
//! before any code of ours runs, so the reserve below the limit holds the
//! largest frame of the program on top of the policy's 5 KiB, and a
//! function whose probe fails returns at once, writing nothing below its
//! frame. Runtime code called from a task runs on the system stack (the
//! root task's, as Go's `systemstack`), so the reserve holds only the
//! frames of the switch, the runtime's thin wrappers and the unwinding.
//!
//! **Decommit on park.** When a task waits, the pages of its stack below
//! the stack pointer it parked at (less a margin for the switch) are given
//! back with `madvise(MADV_DONTNEED)`; a slot is decommitted when it goes
//! back to the pool, but for its top page. The probe doubles as the
//! decommit watermark: after a decommit the task's probe limit is raised
//! to the decommit point, so the probe's slow path fires if, and only if,
//! the stack grows below it again, and marks the task dirty; a park of a
//! task that is not dirty decommits at most the band between its old
//! watermark and its new park point, usually nothing, so a task that parks
//! at the same depth over and over makes no system call.

use crate::abi;

/// The probe reserve beyond the largest frame of the program: the policy's
/// 5 KiB (taskbench, policy 6), for the switch, the runtime's thin wrappers
/// on a task's stack, and unwinding.
pub(crate) const PROBE_RESERVE: usize = 5 * 1024;
/// Of the reserve, what stays below the unwinding floor: once a stack is
/// exhausted, the code that unwinds it may use the rest.
pub(crate) const FLOOR_MARGIN: usize = 2 * 1024;
/// Bytes below a parked task's stack pointer that are never decommitted:
/// the switch's own frames and saved registers (the harness measured 488
/// bytes for its whole park path).
pub(crate) const SWITCH_MARGIN: usize = 2048;
/// The usable stack of a task when `SISU_TASK_STACK` does not say.
pub(crate) const DEFAULT_RESERVATION: usize = 1 << 20;
/// The smallest reservation `SISU_TASK_STACK` may ask for.
const MIN_RESERVATION: usize = 64 * 1024;
/// Address space reserved at a time for slots.
const CHUNK: usize = 1 << 30;

/// The system's page size.
pub(crate) fn page_size() -> usize {
    // SAFETY: `sysconf` has no preconditions.
    let p = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if p > 0 && (p as usize).is_power_of_two() { p as usize } else { 4096 }
}

fn round_up(x: usize, to: usize) -> usize {
    x.div_ceil(to) * to
}

fn round_down(x: usize, to: usize) -> usize {
    x / to * to
}

/// The usable stack of a task: `SISU_TASK_STACK` (bytes, or a number with
/// `K`, `M` or `G`), else 1 MiB. A value that is not a size is ignored; a
/// size below 64 KiB is raised to it.
pub(crate) fn reservation_from_env() -> usize {
    let Some(v) = std::env::var_os("SISU_TASK_STACK") else { return DEFAULT_RESERVATION };
    parse_size(&v.to_string_lossy()).map(|n| n.max(MIN_RESERVATION)).unwrap_or(DEFAULT_RESERVATION)
}

/// `1048576`, `1024K`, `1M`, `1MiB` (case does not matter).
pub(crate) fn parse_size(text: &str) -> Option<usize> {
    let t = text.trim().to_ascii_uppercase();
    let t = t.strip_suffix("IB").or_else(|| t.strip_suffix('B')).unwrap_or(&t);
    let (digits, scale) = match t.chars().last()? {
        'K' => (&t[..t.len() - 1], 1usize << 10),
        'M' => (&t[..t.len() - 1], 1 << 20),
        'G' => (&t[..t.len() - 1], 1 << 30),
        _ => (t, 1),
    };
    digits.trim().parse::<usize>().ok()?.checked_mul(scale)
}

/// A slot: the stack runs down from `base`, and nothing may go below
/// `low`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Region {
    pub low: usize,
    pub base: usize,
}

/// What decommit knows of a stack between parks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Mark {
    /// Nothing below this address is committed (decommitted, or untouched
    /// since); zero when unknown.
    pub clean_below: usize,
    /// The stack grew below `clean_below` since (the watermark's probe
    /// fired).
    pub dirty: bool,
}

/// The pool of slots.
pub(crate) struct Pool {
    /// Usable bytes of a task's stack.
    pub reservation: usize,
    /// Bytes below the probe limit: [`PROBE_RESERVE`] and the largest
    /// frame.
    pub reserve: usize,
    /// The largest frame, which the unwinding floor keeps room for.
    max_frame: usize,
    slot: usize,
    page: usize,
    free: Vec<usize>,
    carve: usize,
    carve_end: usize,
    /// Counters, for the tests.
    pub decommits: u64,
    pub decommits_skipped: u64,
}

impl Pool {
    pub(crate) fn new(reservation: usize, max_frame: usize) -> Pool {
        let page = page_size();
        let reserve = round_up(PROBE_RESERVE + max_frame, 16);
        let slot = round_up(reservation + reserve, page);
        Pool {
            reservation: slot - reserve,
            reserve,
            max_frame,
            slot,
            page,
            free: Vec::new(),
            carve: 0,
            carve_end: 0,
            decommits: 0,
            decommits_skipped: 0,
        }
    }

    /// The limits a task on `r` starts with ([`abi::STACK_PROBE`] and the
    /// others).
    pub(crate) fn limits(&self, r: Region) -> [u64; abi::STACK_WORDS] {
        limits(r.low, self.reserve, self.max_frame)
    }

    /// A slot, from the free list or carved from a reservation; `None`
    /// when the address space is exhausted.
    pub(crate) fn allocate(&mut self) -> Option<Region> {
        let low = match self.free.pop() {
            Some(low) => low,
            None => {
                if self.carve + self.slot > self.carve_end {
                    let chunk = round_up(CHUNK.max(self.slot * 16), self.page);
                    let addr = reserve(chunk)?;
                    self.carve = addr;
                    self.carve_end = addr + chunk - chunk % self.slot;
                }
                let low = self.carve;
                self.carve += self.slot;
                low
            }
        };
        Some(Region { low, base: low + self.slot })
    }

    /// Takes a slot back: its pages are returned, but for the top one,
    /// where the next task's first frames go, unless the task's mark shows
    /// nothing below it is committed.
    pub(crate) fn release(&mut self, r: Region, mark: Mark) {
        let top = round_down(r.base - 1, self.page);
        // Below a clean mark nothing is committed.
        let from = if mark.dirty || mark.clean_below == 0 { r.low } else { mark.clean_below.max(r.low) };
        if from >= top {
            self.decommits_skipped += 1;
        } else {
            self.decommit(from, top);
        }
        self.free.push(r.low);
    }

    /// A task on `r` parked with its stack pointer at `sp`: everything below
    /// it, less [`SWITCH_MARGIN`], is dead. Returns the new mark.
    pub(crate) fn on_park(&mut self, r: Region, sp: usize, mark: Mark) -> Mark {
        if sp <= r.low || sp > r.base {
            return mark;
        }
        let top = round_down(sp.saturating_sub(SWITCH_MARGIN), self.page);
        if mark.dirty || mark.clean_below == 0 || mark.clean_below > top {
            // Unknown or dirty: everything below the park point.
            self.decommit(r.low, top);
        } else if top > mark.clean_below {
            // Clean: only the band the frames live at the last park held.
            self.decommit(mark.clean_below, top);
        } else {
            self.decommits_skipped += 1;
        }
        Mark { clean_below: top.max(r.low), dirty: false }
    }

    /// Decommits `[low, top)`, page-aligned.
    fn decommit(&mut self, low: usize, top: usize) {
        let low = round_up(low, self.page);
        let top = round_down(top, self.page);
        if top <= low {
            return;
        }
        // SAFETY: the callers pass only ranges of slots this pool owns that
        // hold no live frame: below a parked task's stack pointer (less the
        // switch's margin) or of a slot going back to the free list.
        // Failure leaves the pages committed, which is safe.
        unsafe {
            libc::madvise(low as *mut libc::c_void, top - low, libc::MADV_DONTNEED);
        }
        self.decommits += 1;
    }
}

/// The limits of a stack whose lowest usable address is `low`: the probe
/// fires below `low + reserve`; once it has failed, the unwinding may go
/// down to the floor, which keeps the largest frame and
/// [`FLOOR_MARGIN`] above `low`.
pub(crate) fn limits(low: usize, reserve: usize, max_frame: usize) -> [u64; abi::STACK_WORDS] {
    let real = (low + reserve) as u64;
    let floor = (low + max_frame + FLOOR_MARGIN).min(low + reserve) as u64;
    let mut w = [0u64; abi::STACK_WORDS];
    w[(abi::STACK_PROBE / 8) as usize] = real;
    w[(abi::STACK_REAL / 8) as usize] = real;
    w[(abi::STACK_FLOOR / 8) as usize] = floor;
    w
}

/// Reserves `len` bytes of address space, committed lazily and kept out of
/// the overcommit charge; `None` if the system has none left.
fn reserve(len: usize) -> Option<usize> {
    #[cfg(target_os = "linux")]
    let flags = libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_NORESERVE | libc::MAP_STACK;
    #[cfg(not(target_os = "linux"))]
    let flags = libc::MAP_PRIVATE | libc::MAP_ANON | libc::MAP_NORESERVE;
    // SAFETY: an anonymous private mapping with no address hint overlaps
    // nothing the program owns.
    let p =
        unsafe { libc::mmap(std::ptr::null_mut(), len, libc::PROT_READ | libc::PROT_WRITE, flags, -1, 0) };
    if p == libc::MAP_FAILED {
        return None;
    }
    // A stack backed by a transparent huge page would make a task's
    // resident size 2 MiB; the advice is harmless where it is not known.
    #[cfg(target_os = "linux")]
    // SAFETY: advice on the mapping just made changes none of its contents.
    unsafe {
        libc::madvise(p, len, libc::MADV_NOHUGEPAGE);
    }
    Some(p as usize)
}

/// A slot as corosensei's stack: frames start at `base`, 16-byte aligned.
pub(crate) struct Slot(pub Region);

// SAFETY: `base` and `limit` describe a slot this pool handed out, owned by
// one task until it goes back to the pool. There is no guard page below
// it: memory safety rests on the probe in every function the task runs and
// on runtime code running on the system stack (module documentation).
unsafe impl corosensei::stack::Stack for Slot {
    fn base(&self) -> corosensei::stack::StackPointer {
        match corosensei::stack::StackPointer::new(self.0.base) {
            Some(p) => p,
            None => crate::rt::internal("a task's stack at address 0"),
        }
    }

    fn limit(&self) -> corosensei::stack::StackPointer {
        match corosensei::stack::StackPointer::new(self.0.low) {
            Some(p) => p,
            None => crate::rt::internal("a task's stack at address 0"),
        }
    }
}

/// The current stack pointer (near enough: the address of a local).
#[inline(never)]
pub(crate) fn current_sp() -> usize {
    let marker = 0u8;
    std::hint::black_box(&marker) as *const u8 as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_read_with_units() {
        assert_eq!(parse_size("1M"), Some(1 << 20));
        assert_eq!(parse_size("512k"), Some(512 << 10));
        assert_eq!(parse_size("8MiB"), Some(8 << 20));
        assert_eq!(parse_size("65536"), Some(65536));
        assert_eq!(parse_size("2G"), Some(2 << 30));
        assert_eq!(parse_size("lots"), None);
        assert_eq!(parse_size(""), None);
    }

    #[test]
    fn slots_hold_the_reservation_above_the_reserve() {
        let mut p = Pool::new(256 * 1024, 3000);
        assert!(p.reserve >= PROBE_RESERVE + 3000);
        let a = p.allocate().expect("a slot");
        let b = p.allocate().expect("a slot");
        assert_eq!(b.low - a.low, a.base - a.low, "slots are contiguous");
        assert!(a.base - a.low >= 256 * 1024 + p.reserve);
        let l = p.limits(a);
        assert_eq!(l[0] as usize, a.low + p.reserve);
        assert!(l[2] as usize >= a.low + 3000 + FLOOR_MARGIN);
        assert!(l[2] <= l[1]);
        p.release(b, Mark::default());
        p.release(a, Mark::default());
        assert_eq!(p.allocate(), Some(a), "the free list is LIFO");
        assert_eq!(p.allocate(), Some(b));
    }

    #[test]
    fn a_clean_park_at_the_same_depth_makes_no_call() {
        let mut p = Pool::new(256 * 1024, 0);
        let r = p.allocate().expect("a slot");
        // Touch the top 64 KiB, as frames would.
        let mut a = r.base - 64 * 1024;
        while a < r.base {
            // SAFETY: the slot was just handed out to this test.
            unsafe { std::ptr::write_volatile(a as *mut u8, 1) };
            a += 4096;
        }
        let sp = r.base - 8 * 1024;
        let m = p.on_park(r, sp, Mark::default());
        assert_eq!(p.decommits, 1);
        let m2 = p.on_park(r, sp, m);
        assert_eq!(p.decommits, 1);
        assert_eq!(p.decommits_skipped, 1);
        assert_eq!(m, m2);
        let m3 = p.on_park(r, sp, Mark { dirty: true, ..m2 });
        assert_eq!(p.decommits, 2);
        assert!(!m3.dirty);
        p.release(r, m3);
    }
}
