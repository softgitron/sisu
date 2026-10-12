//! Strategy A: one `mmap` reservation per task with a `PROT_NONE` guard page
//! below it, committed lazily.
//!
//! Each stack costs two VMAs (the guard and the stack cannot merge), so the
//! default `vm.max_map_count` of 65530 caps a process at about 32k such stacks;
//! `max-tasks` measures exactly where. Running out of stack touches the guard and
//! is turned into a task failure by [`super::fault`].

use std::sync::Mutex;
use std::sync::atomic::Ordering::Relaxed;

use super::{AtomicCounters, Exhaustion, StackCounters, StackError, StackRegion, StackStrategy, os};
use crate::units::round_up;

/// Per-task guarded mappings, with an optional cache of freed stacks.
pub struct GuardedMmap {
    usable: usize,
    guard: usize,
    cache_cap: usize,
    cache: Mutex<Vec<StackRegion>>,
    counters: AtomicCounters,
}

impl GuardedMmap {
    /// `reservation` usable bytes per stack (rounded up to pages) above a `guard`
    /// of `guard` bytes; up to `cache` freed stacks are kept mapped for reuse.
    pub fn new(reservation: usize, guard: usize, cache: usize) -> GuardedMmap {
        let page = os::page_size();
        GuardedMmap {
            usable: round_up(reservation, page),
            guard: round_up(guard, page),
            cache_cap: cache,
            cache: Mutex::new(Vec::new()),
            counters: AtomicCounters::default(),
        }
    }

    fn total(&self) -> usize {
        self.usable + self.guard
    }
}

impl StackStrategy for GuardedMmap {
    fn label(&self) -> String {
        "A".into()
    }

    fn exhaustion(&self) -> Exhaustion {
        Exhaustion::GuardPage
    }

    fn stack_bytes(&self) -> usize {
        self.usable
    }

    fn allocate(&self) -> Result<StackRegion, StackError> {
        if self.cache_cap > 0 {
            if let Some(r) = self.cache.lock().expect("stack cache lock").pop() {
                self.counters.allocated.fetch_add(1, Relaxed);
                self.counters.reused.fetch_add(1, Relaxed);
                return Ok(r);
            }
        }
        let total = self.total();
        let addr = os::reserve(total).map_err(|e| StackError::classify("mmap", e))?;
        // SAFETY: [addr, addr+guard) is the low end of the mapping just created
        // and holds nothing.
        if let Err(e) = unsafe { os::protect_none(addr, self.guard) } {
            let err = StackError::classify("mprotect(guard)", e);
            // SAFETY: the mapping was created above and never handed out.
            let _ = unsafe { os::unmap(addr, total) };
            return Err(err);
        }
        self.counters.allocated.fetch_add(1, Relaxed);
        self.counters.reserved_bytes.fetch_add(total as u64, Relaxed);
        Ok(StackRegion {
            base: addr + total,
            limit: addr,
            usable_low: addr + self.guard,
            probe_limit: 0,
            guard_lo: addr,
            guard_hi: addr + self.guard,
        })
    }

    unsafe fn release(&self, region: StackRegion) {
        self.counters.released.fetch_add(1, Relaxed);
        if self.cache_cap > 0 {
            let mut cache = self.cache.lock().expect("stack cache lock");
            if cache.len() < self.cache_cap {
                cache.push(region);
                return;
            }
        }
        let len = region.base - region.limit;
        // SAFETY: the caller guarantees the region came from `allocate`, is
        // released once and is no longer in use.
        let r = unsafe { os::unmap(region.limit, len) };
        debug_assert!(r.is_ok(), "munmap of a task stack failed: {r:?}");
        self.counters.reserved_bytes.fetch_sub(len as u64, Relaxed);
    }

    fn counters(&self) -> StackCounters {
        self.counters.snapshot()
    }
}

impl Drop for GuardedMmap {
    fn drop(&mut self) {
        let cache = std::mem::take(self.cache.get_mut().expect("stack cache lock"));
        for r in cache {
            // SAFETY: cached stacks are owned by the strategy and unused.
            let _ = unsafe { os::unmap(r.limit, r.base - r.limit) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mem;

    #[test]
    fn allocates_guarded_lazy_stacks() {
        let s = GuardedMmap::new(64 * 1024, 4096, 0);
        let vmas_before = mem::vma_count();
        let r = s.allocate().unwrap();
        assert_eq!(r.usable_bytes(), 64 * 1024);
        assert!(r.has_guard());
        assert_eq!(r.guard_hi - r.guard_lo, 4096);
        assert_eq!(r.base % 16, 0);
        // Two new VMAs: the guard and the stack.
        assert!(mem::vma_count() >= vmas_before + 2);
        // Lazy commit: touching the top page only makes the RSS grow by about a page.
        let rss0 = mem::snapshot(false).vm_rss;
        // SAFETY: the top word of the usable range belongs to this test.
        unsafe { std::ptr::write_volatile((r.base - 8) as *mut u64, 42) };
        let rss1 = mem::snapshot(false).vm_rss;
        assert!(rss1 >= rss0, "rss shrank");
        assert!(rss1 - rss0 < 64 * 1024, "whole stack committed eagerly");
        // SAFETY: allocated above, released once, nothing runs on it.
        unsafe { s.release(r) };
        assert_eq!(s.counters().reserved_bytes, 0);
    }

    #[test]
    fn guard_is_inaccessible() {
        let s = GuardedMmap::new(64 * 1024, 4096, 0);
        let r = s.allocate().unwrap();
        let maps = std::fs::read_to_string("/proc/self/maps").unwrap();
        let guard_line = maps
            .lines()
            .find(|l| l.starts_with(&format!("{:x}-", r.guard_lo)))
            .expect("guard mapping listed");
        assert!(guard_line.contains("---p"), "guard not PROT_NONE: {guard_line}");
        // SAFETY: allocated above, released once.
        unsafe { s.release(r) };
    }

    #[test]
    fn cache_reuses_stacks() {
        let s = GuardedMmap::new(32 * 1024, 4096, 4);
        let r1 = s.allocate().unwrap();
        // SAFETY: allocated above, released once.
        unsafe { s.release(r1) };
        let r2 = s.allocate().unwrap();
        assert_eq!(r1, r2);
        assert_eq!(s.counters().reused, 1);
        // SAFETY: allocated above, released once.
        unsafe { s.release(r2) };
    }
}
