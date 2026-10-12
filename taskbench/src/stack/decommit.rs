//! Strategy D: strategy A or B plus `madvise(MADV_DONTNEED)` of the dead part of
//! a task's stack when the task parks.
//!
//! A task that made a deep excursion and came back keeps the excursion's pages
//! committed under A and B ("commit creep"): lazily committed memory is never
//! returned by itself. D returns everything below the parked stack pointer, minus
//! a margin for the context-switch frame. With B as the base, a slot is also
//! decommitted when it goes back to the pool, so a recycled slot does not carry an
//! earlier task's excursion.
//!
//! D(A) pays one system call per park. D(B) uses its software probes as a
//! watermark: after a decommit, the scheduler raises the task's probe limit to
//! the decommit point, so the probe's slow path fires if, and only if, the stack
//! grows below it again, and marks the task dirty. A park (or a release) of a
//! task that is not dirty decommits only the range between the old watermark and
//! the new park point, which is usually empty, so a task that parks at the same
//! depth over and over (a server loop, a joiner) makes no system call at all.
//! Skipping a decommit is always safe: at worst pages stay committed.

use std::sync::Arc;
use std::sync::atomic::Ordering::Relaxed;

use super::{AtomicCounters, DecommitMark, Exhaustion, StackCounters, StackError, StackRegion, StackStrategy, os};
use crate::units::{round_down, round_up};

/// Bytes below a parked task's recorded stack pointer that are never decommitted.
pub const SWITCH_MARGIN: usize = 2048;

/// Decommit-on-park wrapper.
pub struct Decommit {
    base: Arc<dyn StackStrategy>,
    /// The base has software probes, which serve as the watermark.
    watermark: bool,
    counters: AtomicCounters,
}

impl Decommit {
    pub fn new(base: Arc<dyn StackStrategy>) -> Decommit {
        let watermark = base.exhaustion() == Exhaustion::SoftwareProbe;
        Decommit { base, watermark, counters: AtomicCounters::default() }
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

    unsafe fn release_clean(&self, region: StackRegion) {
        self.counters.decommits_skipped.fetch_add(1, Relaxed);
        // SAFETY: forwarded under the caller's guarantee.
        unsafe { self.base.release(region) }
    }

    fn watermark_probes(&self) -> bool {
        self.watermark
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

    fn on_park(&self, region: &StackRegion, sp: usize, mark: &mut DecommitMark) {
        if sp <= region.usable_low || sp > region.base {
            return;
        }
        // The context switch stores the task's callee-saved registers and its
        // suspend frames just below the recorded stack pointer: 488 bytes for the
        // whole wait-park-suspend chain, measured by
        // `rt::tests::park_switch_frame_is_shallow`. Keep 4x that.
        let page = os::page_size();
        let top = round_down(sp.saturating_sub(SWITCH_MARGIN), page);
        if !self.watermark || mark.dirty || mark.clean_below == 0 || mark.clean_below > top {
            // Unknown or dirty: everything below the park point.
            self.decommit_range(region.usable_low, top);
        } else if top > mark.clean_below {
            // Not dirty: only what lies between the old watermark and this park
            // point can have been touched (frames that were live at the last park).
            self.decommit_range(mark.clean_below, top);
        } else {
            self.counters.decommits_skipped.fetch_add(1, Relaxed);
        }
        *mark = DecommitMark { clean_below: top.max(region.usable_low), dirty: false };
    }

    fn counters(&self) -> StackCounters {
        let mut c = self.base.counters();
        let own = self.counters.snapshot();
        c.decommits = own.decommits;
        c.decommitted_bytes = own.decommitted_bytes;
        c.decommits_skipped = own.decommits_skipped;
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
        assert_eq!(mem::resident_pages(r.base - 512 * 1024, 512 * 1024), 128);
        // The task parks with its stack pointer 8 KiB below the base.
        let mut mark = DecommitMark::default();
        d.on_park(&r, r.base - 8 * 1024, &mut mark);
        // Kept: everything from the stack pointer minus the switch margin, rounded
        // down to a page: base-8K-2K rounds to base-12K, three pages.
        assert_eq!(mem::resident_pages(r.base - 512 * 1024, 512 * 1024), 3);
        let c = d.counters();
        assert_eq!(c.decommits, 1);
        assert!(c.decommitted_bytes >= 500 * 1024);
        assert_eq!(mark.clean_below, r.base - 12 * 1024);
        // The pages above the margin are intact.
        // SAFETY: top word of the stack belongs to this test.
        assert_eq!(unsafe { std::ptr::read_volatile((r.base - 4096) as *const u8) }, 1);
        // SAFETY: allocated above, released once.
        unsafe { d.release(r) };
        assert_eq!(d.label(), "D(A)");
    }

    #[test]
    fn watermark_skips_clean_parks() {
        let d = Decommit::new(Arc::new(SlotPool::pooled(256 * 1024, 16 * 1024, 4 << 20, false)));
        assert!(d.watermark_probes());
        let r = d.allocate().unwrap();
        touch(r.base - 64 * 1024, r.base);
        let mut mark = DecommitMark::default();
        d.on_park(&r, r.base - 8 * 1024, &mut mark); // unknown: full decommit
        assert_eq!(d.counters().decommits, 1);
        d.on_park(&r, r.base - 8 * 1024, &mut mark); // same depth, clean: skipped
        assert_eq!(d.counters().decommits, 1);
        assert_eq!(d.counters().decommits_skipped, 1);
        d.on_park(&r, r.base - 4 * 1024, &mut mark); // shallower: only the new band
        assert_eq!(d.counters().decommits, 2);
        assert_eq!(d.counters().decommitted_bytes as usize, (r.base - 12 * 1024 - r.usable_low) + 4096);
        mark.dirty = true; // the probe saw the stack grow below the watermark
        d.on_park(&r, r.base - 4 * 1024, &mut mark);
        assert_eq!(d.counters().decommits, 3);
        // SAFETY: allocated above, released once.
        unsafe { d.release(r) };
    }

    #[test]
    fn pooled_release_decommits_slot() {
        let d = Decommit::new(Arc::new(SlotPool::pooled(256 * 1024, 16 * 1024, 4 << 20, false)));
        let r = d.allocate().unwrap();
        touch(r.base - 200 * 1024, r.base);
        assert_eq!(mem::resident_pages(r.base - 200 * 1024, 200 * 1024), 50);
        // SAFETY: allocated above, released once.
        unsafe { d.release(r) };
        assert!(d.counters().decommitted_bytes >= 190 * 1024);
        assert_eq!(mem::resident_pages(r.base - 200 * 1024, 200 * 1024), 1, "top page kept");
        assert_eq!(d.label(), "D(B)");
    }
}
