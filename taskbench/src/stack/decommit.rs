//! Strategy D: strategy A or B plus `madvise(MADV_DONTNEED)` of the dead part of
//! a task's stack when the task parks.
//!
//! A task that made a deep excursion and came back keeps the excursion's pages
//! committed under A and B ("commit creep"): lazily committed memory is never
//! returned by itself. D returns everything below the parked stack pointer, minus
//! a one-page margin for the context-switch frame, at the cost of one system call
//! per park. With B as the base, a slot is also decommitted when it goes back to
//! the pool, so a recycled slot does not carry an earlier task's excursion.

use std::sync::Arc;
use std::sync::atomic::Ordering::Relaxed;

use super::{AtomicCounters, Exhaustion, StackCounters, StackError, StackRegion, StackStrategy, os};
use crate::units::{round_down, round_up};

/// Decommit-on-park wrapper.
pub struct Decommit {
    base: Arc<dyn StackStrategy>,
    counters: AtomicCounters,
}

impl Decommit {
    pub fn new(base: Arc<dyn StackStrategy>) -> Decommit {
        Decommit { base, counters: AtomicCounters::default() }
    }

    /// Decommits `[low, top)` after page alignment; returns the bytes decommitted.
    fn decommit_range(&self, low: usize, top: usize) -> usize {
        let page = os::page_size();
        let low = round_up(low, page);
        let top = round_down(top, page);
        if top <= low {
            return 0;
        }
        // SAFETY: callers pass only ranges below a suspended task's stack pointer
        // (minus a margin) or of a released slot, which hold no live data.
        let r = unsafe { os::decommit(low, top - low) };
        debug_assert!(r.is_ok(), "madvise(MADV_DONTNEED) failed: {r:?}");
        self.counters.decommits.fetch_add(1, Relaxed);
        self.counters.decommitted_bytes.fetch_add((top - low) as u64, Relaxed);
        top - low
    }
}

impl StackStrategy for Decommit {
    fn label(&self) -> String {
        format!("D({})", self.base.label())
    }

    fn exhaustion(&self) -> Exhaustion {
        self.base.exhaustion()
    }

    fn stack_bytes(&self) -> usize {
        self.base.stack_bytes()
    }

    fn allocate(&self) -> Result<StackRegion, StackError> {
        self.base.allocate()
    }

    unsafe fn release(&self, region: StackRegion) {
        if !region.has_guard() {
            // Pooled slot: return its pages before it goes back on the free list,
            // keeping the top page where the next task's first frames go.
            let page = os::page_size();
            self.decommit_range(region.usable_low, round_down(region.base - 1, page));
        }
        // SAFETY: forwarded under the caller's guarantee.
        unsafe { self.base.release(region) }
    }

    fn decommits_on_park(&self) -> bool {
        true
    }

    fn on_park(&self, region: &StackRegion, sp: usize) {
        if sp <= region.usable_low || sp > region.base {
            return;
        }
        // Keep the page holding `sp` and the one below it: the context switch
        // stores the task's callee-saved registers just below the recorded stack
        // pointer (well under a page).
        let page = os::page_size();
        let top = round_down(sp, page).saturating_sub(page);
        self.decommit_range(region.usable_low, top);
    }

    fn counters(&self) -> StackCounters {
        let mut c = self.base.counters();
        let own = self.counters.snapshot();
        c.decommits = own.decommits;
        c.decommitted_bytes = own.decommitted_bytes;
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mem;
    use crate::stack::{guarded::GuardedMmap, pool::SlotPool};

    fn touch(lo: usize, hi: usize) {
        let mut a = lo;
        while a < hi {
            // SAFETY: the tests only touch ranges of stacks they allocated.
            unsafe { std::ptr::write_volatile(a as *mut u8, 1) };
            a += 4096;
        }
    }

    #[test]
    fn park_returns_excursion_pages() {
        let d = Decommit::new(Arc::new(GuardedMmap::new(1 << 20, 4096, 0)));
        let r = d.allocate().unwrap();
        // A 512 KiB excursion below the "shallow" frame.
        touch(r.base - 512 * 1024, r.base);
        let before = mem::snapshot(false).vm_rss;
        // The task parks with its stack pointer 8 KiB below the base.
        d.on_park(&r, r.base - 8 * 1024);
        let after = mem::snapshot(false).vm_rss;
        assert!(before - after >= 400 * 1024, "only {} bytes returned", before - after);
        let c = d.counters();
        assert_eq!(c.decommits, 1);
        assert!(c.decommitted_bytes >= 500 * 1024);
        // The pages above the margin are intact.
        // SAFETY: top word of the stack belongs to this test.
        assert_eq!(unsafe { std::ptr::read_volatile((r.base - 4096) as *const u8) }, 1);
        // SAFETY: allocated above, released once.
        unsafe { d.release(r) };
        assert_eq!(d.label(), "D(A)");
    }

    #[test]
    fn pooled_release_decommits_slot() {
        let d = Decommit::new(Arc::new(SlotPool::pooled(256 * 1024, 16 * 1024, 4 << 20, false)));
        let r = d.allocate().unwrap();
        touch(r.base - 200 * 1024, r.base);
        // SAFETY: allocated above, released once.
        unsafe { d.release(r) };
        assert!(d.counters().decommitted_bytes >= 190 * 1024);
        assert_eq!(d.label(), "D(B)");
    }
}
