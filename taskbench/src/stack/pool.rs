//! Strategies B and C: stacks carved as fixed-size slots from large reservations,
//! with no guard pages.
//!
//! * B (`pooled`): slots of the reservation size (default 1 MiB) plus the probe
//!   reserve, so a task can use as much stack as under A before its probe fires.
//! * C (`exact`): slots of an exact small size (2-16 KiB) plus the probe reserve,
//!   simulating a compiler that computed the task's stack bound.
//!
//! One large reservation is one VMA however many slots it holds, so neither hits
//! `vm.max_map_count`. Freed slots go on a LIFO free list and are reused without
//! decommit, so a recycled slot's pages are already resident (the fast path for
//! spawn-plus-join). Without guard pages, an overflow would silently write into the
//! neighbouring slot; the software probe ([`super::probe`]) in every function of a
//! task body is what prevents that, and the optional canary detects a missed one.

use std::sync::Mutex;
use std::sync::atomic::Ordering::Relaxed;

use super::{AtomicCounters, Exhaustion, StackCounters, StackError, StackRegion, StackStrategy, os};
use crate::units::round_up;

const CANARY: u64 = 0x5afe_57ac_c0de_d00d;

struct PoolInner {
    /// Low addresses of free slots, most recently freed last.
    free: Vec<usize>,
    /// Next unused slot in the current chunk, and that chunk's end.
    carve: usize,
    carve_end: usize,
    /// Every reservation made, for unmapping on drop.
    chunks: Vec<(usize, usize)>,
}

/// A pool of equally sized stack slots.
pub struct SlotPool {
    exact: bool,
    /// Usable stack bytes before the probe fires.
    stack: usize,
    /// Probe reserve at the bottom of each slot.
    reserve: usize,
    /// Bytes per slot (`stack + reserve`, 16-byte aligned; page aligned for B).
    slot: usize,
    chunk: usize,
    canary: bool,
    inner: Mutex<PoolInner>,
    counters: AtomicCounters,
}

impl SlotPool {
    /// Strategy B: `reservation` bytes of stack per slot.
    pub fn pooled(reservation: usize, reserve: usize, chunk: usize, canary: bool) -> SlotPool {
        let page = os::page_size();
        let slot = round_up(reservation + reserve, page);
        SlotPool::with_geometry(false, slot - reserve, reserve, slot, chunk, canary)
    }

    /// Strategy C: exactly `size` bytes of stack per slot.
    pub fn exact(size: usize, reserve: usize, chunk: usize, canary: bool) -> SlotPool {
        let slot = round_up(size + reserve, 16);
        SlotPool::with_geometry(true, slot - reserve, reserve, slot, chunk, canary)
    }

    fn with_geometry(exact: bool, stack: usize, reserve: usize, slot: usize, chunk: usize, canary: bool) -> SlotPool {
        let page = os::page_size();
        // A chunk holds a whole number of slots and at least 16 of them.
        let chunk = round_up(chunk.max(slot * 16), page);
        SlotPool {
            exact,
            stack,
            reserve,
            slot,
            chunk,
            canary,
            inner: Mutex::new(PoolInner { free: Vec::new(), carve: 0, carve_end: 0, chunks: Vec::new() }),
            counters: AtomicCounters::default(),
        }
    }

    /// Bytes per slot, including the probe reserve.
    pub fn slot_bytes(&self) -> usize {
        self.slot
    }

    fn region_for(&self, low: usize) -> StackRegion {
        StackRegion {
            base: low + self.slot,
            limit: low,
            usable_low: low,
            probe_limit: low + self.reserve,
            guard_lo: 0,
            guard_hi: 0,
        }
    }
}

impl StackStrategy for SlotPool {
    fn label(&self) -> String {
        if self.exact { "C".into() } else { "B".into() }
    }

    fn exhaustion(&self) -> Exhaustion {
        Exhaustion::SoftwareProbe
    }

    fn stack_bytes(&self) -> usize {
        self.stack
    }

    fn allocate(&self) -> Result<StackRegion, StackError> {
        let low = {
            let mut g = self.inner.lock().expect("slot pool lock");
            if let Some(low) = g.free.pop() {
                self.counters.reused.fetch_add(1, Relaxed);
                low
            } else {
                if g.carve + self.slot > g.carve_end {
                    let addr = os::reserve(self.chunk).map_err(|e| StackError::classify("mmap(chunk)", e))?;
                    g.chunks.push((addr, self.chunk));
                    g.carve = addr;
                    g.carve_end = addr + self.chunk - (self.chunk % self.slot);
                    self.counters.reserved_bytes.fetch_add(self.chunk as u64, Relaxed);
                }
                let low = g.carve;
                g.carve += self.slot;
                low
            }
        };
        self.counters.allocated.fetch_add(1, Relaxed);
        if self.canary {
            // SAFETY: `low` is the first word of a slot this pool just handed out.
            unsafe { std::ptr::write_volatile(low as *mut u64, CANARY) };
        }
        Ok(self.region_for(low))
    }

    unsafe fn release(&self, region: StackRegion) {
        if self.canary {
            // SAFETY: the slot is owned by this pool and no longer in use.
            let v = unsafe { std::ptr::read_volatile(region.limit as *const u64) };
            if v != CANARY {
                self.counters.canary_failures.fetch_add(1, Relaxed);
                eprintln!(
                    "taskbench: stack canary at {:#x} overwritten: a task ran past its probe reserve",
                    region.limit
                );
            }
        }
        self.counters.released.fetch_add(1, Relaxed);
        self.inner.lock().expect("slot pool lock").free.push(region.limit);
    }

    fn counters(&self) -> StackCounters {
        self.counters.snapshot()
    }
}

impl Drop for SlotPool {
    fn drop(&mut self) {
        let g = self.inner.get_mut().expect("slot pool lock");
        for &(addr, len) in &g.chunks {
            // SAFETY: the pool owns its chunks; the runtime drops the strategy only
            // after every task (and so every slot user) has finished.
            let _ = unsafe { os::unmap(addr, len) };
        }
        g.chunks.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mem;

    #[test]
    fn pooled_slots_share_one_mapping() {
        let p = SlotPool::pooled(64 * 1024, 16 * 1024, 4 << 20, false);
        let regions: Vec<_> = (0..32).map(|_| p.allocate().unwrap()).collect();
        // One chunk holds all 32 slots of 80 KiB: one mapping covers them all.
        let first = mem::mapping_containing(regions[0].limit).expect("mapped");
        let last = mem::mapping_containing(regions[31].base - 1).expect("mapped");
        assert_eq!(first, last, "slots span several mappings");
        for w in regions.windows(2) {
            assert_eq!(w[1].limit - w[0].limit, p.slot_bytes(), "slots are contiguous");
        }
        // Lazily committed: carving slots touches nothing.
        assert_eq!(mem::resident_pages(regions[0].limit, 32 * p.slot_bytes()), 0);
        for r in &regions {
            assert!(!r.has_guard());
            assert_eq!(r.probe_limit, r.limit + 16 * 1024);
            assert_eq!(r.base % 16, 0);
        }
        for r in regions {
            // SAFETY: allocated above, released once.
            unsafe { p.release(r) };
        }
        assert_eq!(p.counters().released, 32);
    }

    #[test]
    fn freed_slots_are_reused_lifo() {
        let p = SlotPool::pooled(32 * 1024, 4096, 1 << 20, false);
        let a = p.allocate().unwrap();
        let b = p.allocate().unwrap();
        // SAFETY: allocated above, released once.
        unsafe { p.release(a) };
        // SAFETY: allocated above, released once.
        unsafe { p.release(b) };
        assert_eq!(p.allocate().unwrap(), b);
        assert_eq!(p.allocate().unwrap(), a);
        assert_eq!(p.counters().reused, 2);
    }

    #[test]
    fn exact_slots_are_small() {
        let p = SlotPool::exact(2048, 4096, 1 << 20, false);
        let r = p.allocate().unwrap();
        assert_eq!(r.base - r.limit, 6144);
        assert_eq!(r.base - r.probe_limit, 2048);
        assert_eq!(p.label(), "C");
        // SAFETY: allocated above, released once.
        unsafe { p.release(r) };
    }

    #[test]
    fn canary_detects_overrun() {
        let p = SlotPool::exact(4096, 4096, 1 << 20, true);
        let r = p.allocate().unwrap();
        // Simulate a task that ran past its reserve.
        // SAFETY: the slot's bottom word belongs to this test.
        unsafe { std::ptr::write_volatile(r.limit as *mut u64, 0) };
        // SAFETY: allocated above, released once.
        unsafe { p.release(r) };
        assert_eq!(p.counters().canary_failures, 1);
    }
}
