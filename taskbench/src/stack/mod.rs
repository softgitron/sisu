//! Stack strategies: how a stackful task gets the memory its frames live in.
//!
//! The harness answers Q-6 by running the same scheduler and the same benchmark
//! bodies over five strategies (proposal/18, Phase 1):
//!
//! | Id | Strategy | Exhaustion becomes a task failure by |
//! |----|----------|--------------------------------------|
//! | A  | one `mmap` reservation per task, `PROT_NONE` guard page, lazy commit ([`guarded`]) | the guard-page fault handler ([`fault`]) |
//! | B  | slots carved from large reservations, no guard pages ([`pool`]) | software probes at function entry ([`probe`]) |
//! | C  | exact small slots (2-16 KiB) for bounded call trees ([`pool`]) | software probes |
//! | D  | A or B plus `MADV_DONTNEED` of the used pages below the stack pointer when a task parks ([`decommit`]) | as its base |
//! | E  | no stack: stackless state machines (Rust `async`) on the same scheduler ([`stackless`]) | a probe of the worker thread's own stack |
//!
//! Stacks never move: a frame's address is fixed for the life of the task, which
//! is the constraint a runtime without a moving GC lives under.

pub mod decommit;
pub mod fault;
pub mod guarded;
pub mod os;
pub mod pool;
pub mod probe;
pub mod stackless;

use std::fmt;
use std::io;
use std::sync::Arc;

use crate::units::{GIB, KIB, MIB, fmt_size};

/// The five strategies under test.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StrategyId {
    A,
    B,
    C,
    D,
    E,
}

impl StrategyId {
    pub fn parse(s: &str) -> Option<StrategyId> {
        match s.trim().to_ascii_uppercase().as_str() {
            "A" => Some(StrategyId::A),
            "B" => Some(StrategyId::B),
            "C" => Some(StrategyId::C),
            "D" => Some(StrategyId::D),
            "E" => Some(StrategyId::E),
            _ => None,
        }
    }
}

/// Stackful (a coroutine on a stack) or stackless (a future polled on the worker's stack).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StackKind {
    Stackful,
    Stackless,
}

/// How running out of stack is detected and turned into a task failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exhaustion {
    /// A `PROT_NONE` guard page faults; the SIGSEGV handler ends the task. Frames on
    /// the exhausted stack are discarded without running their destructors.
    GuardPage,
    /// A compare of the stack pointer against a per-task limit at function entry
    /// raises a "stack exhausted" panic that unwinds normally (destructors run).
    SoftwareProbe,
    /// Stackless: the probe compares against the worker thread's own stack, on
    /// which nested `poll` calls recurse.
    WorkerStackProbe,
}

impl Exhaustion {
    pub fn as_str(self) -> &'static str {
        match self {
            Exhaustion::GuardPage => "guard-page",
            Exhaustion::SoftwareProbe => "software-probe",
            Exhaustion::WorkerStackProbe => "worker-stack-probe",
        }
    }
}

/// A stack handed out by a strategy. Addresses grow down from `base`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StackRegion {
    /// Highest address (exclusive), 16-byte aligned.
    pub base: usize,
    /// Lowest address of the region including any guard (corosensei's `Stack::limit`).
    pub limit: usize,
    /// Lowest address frames may occupy (just above the guard, if any).
    pub usable_low: usize,
    /// Threshold for software probes: a probe that sees the stack pointer below
    /// this raises "stack exhausted". Zero when the strategy uses a guard page.
    pub probe_limit: usize,
    /// `PROT_NONE` guard range `[guard_lo, guard_hi)`; empty when there is none.
    pub guard_lo: usize,
    pub guard_hi: usize,
}

impl StackRegion {
    /// Usable bytes between `usable_low` and `base`.
    pub fn usable_bytes(&self) -> usize {
        self.base - self.usable_low
    }

    pub fn has_guard(&self) -> bool {
        self.guard_hi > self.guard_lo
    }

    /// Whether `addr` lies in this region's guard.
    pub fn in_guard(&self, addr: usize) -> bool {
        addr >= self.guard_lo && addr < self.guard_hi
    }
}

/// Why a stack could not be allocated. Classifying the limit is part of the
/// `max-tasks` result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StackError {
    /// The process hit `vm.max_map_count` (each guard page splits a mapping).
    VmaLimit { vmas: u64, max_map_count: u64, detail: String },
    /// Address space or memory could not be obtained.
    Memory { detail: String },
    /// The strategy has no stacks (E).
    NotApplicable(&'static str),
}

impl StackError {
    /// Short name of the limit that failed, for CSV notes.
    pub fn limit_name(&self) -> &'static str {
        match self {
            StackError::VmaLimit { .. } => "vma_count",
            StackError::Memory { .. } => "memory",
            StackError::NotApplicable(_) => "not_applicable",
        }
    }

    /// Classifies a failed `mmap`/`mprotect`: `ENOMEM` with the VMA count at the
    /// sysctl limit is the VMA limit; anything else is memory.
    pub fn classify(op: &str, err: io::Error) -> StackError {
        let max = crate::mem::read_sysctl("vm/max_map_count").unwrap_or(65530);
        let vmas = crate::mem::vma_count();
        let detail = format!("{op}: {err}");
        if err.raw_os_error() == Some(libc::ENOMEM) && vmas + 8 >= max {
            StackError::VmaLimit { vmas, max_map_count: max, detail }
        } else {
            StackError::Memory { detail }
        }
    }
}

impl fmt::Display for StackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StackError::VmaLimit { vmas, max_map_count, detail } => {
                write!(f, "VMA limit reached ({vmas} mappings, vm.max_map_count={max_map_count}): {detail}")
            }
            StackError::Memory { detail } => write!(f, "out of memory or address space: {detail}"),
            StackError::NotApplicable(why) => write!(f, "not applicable: {why}"),
        }
    }
}

/// Counters each strategy keeps, reported alongside benchmark results.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StackCounters {
    pub allocated: u64,
    pub released: u64,
    /// Bytes of address space currently reserved by the strategy.
    pub reserved_bytes: u64,
    /// Allocations served from a free list or cache rather than a fresh mapping.
    pub reused: u64,
    /// `madvise(MADV_DONTNEED)` calls made on park and release (D).
    pub decommits: u64,
    pub decommitted_bytes: u64,
    /// Parks and releases where the watermark showed nothing to decommit (D(B)).
    pub decommits_skipped: u64,
    /// B and C with `canary`: slots whose bottom canary was overwritten (a missed probe).
    pub canary_failures: u64,
}

/// What strategy D knows about a stack between decommits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DecommitMark {
    /// Everything below this address is known not to be committed (decommitted,
    /// or never touched since). Zero: unknown.
    pub clean_below: usize,
    /// The task's stack has grown below `clean_below` since the mark was set
    /// (reported by the watermark probe).
    pub dirty: bool,
}

/// The interface the scheduler uses to obtain and return stacks.
pub trait StackStrategy: Send + Sync + 'static {
    /// `A`, `B`, `C`, `D(A)`, `D(B)` or `E`.
    fn label(&self) -> String;
    fn kind(&self) -> StackKind {
        StackKind::Stackful
    }
    fn exhaustion(&self) -> Exhaustion;
    /// Usable bytes per stack (the reservation, or the exact size for C).
    fn stack_bytes(&self) -> usize;
    /// Hands out a stack. Pages are committed lazily by the kernel on first touch.
    fn allocate(&self) -> Result<StackRegion, StackError>;
    /// Takes a stack back.
    ///
    /// # Safety
    /// `region` must have come from `allocate` on this strategy, must not have been
    /// released already, and no code may run on it or reference memory in it any more.
    unsafe fn release(&self, region: StackRegion);
    /// Whether [`StackStrategy::on_park`] does anything, so that a parking task only
    /// records its stack pointer when it matters.
    fn decommits_on_park(&self) -> bool {
        false
    }
    /// Called by the worker after the task owning `region` has suspended itself to
    /// park, with `sp` the stack pointer the task recorded just before suspending.
    /// Everything below `sp` minus a safety margin is dead. `mark` carries what is
    /// known about the stack since the last decommit.
    fn on_park(&self, _region: &StackRegion, _sp: usize, _mark: &mut DecommitMark) {}
    /// Whether the strategy uses the software probe as a decommit watermark (D over
    /// a probed base): the scheduler then raises each task's probe limit to its
    /// mark's `clean_below`, and a probe that fires above the real limit only
    /// reports the task dirty.
    fn watermark_probes(&self) -> bool {
        false
    }
    /// As [`StackStrategy::release`], for a stack known not to be committed below
    /// its top page. Strategies that decommit on release may skip it.
    ///
    /// # Safety
    /// As for `release`.
    unsafe fn release_clean(&self, region: StackRegion) {
        // SAFETY: forwarded under the caller's guarantee.
        unsafe { self.release(region) }
    }
    fn counters(&self) -> StackCounters;
}

/// Parameters of a strategy, from the command line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StrategyConfig {
    pub id: StrategyId,
    /// A, B and D: usable bytes per task (default 1 MiB; the proposal also asks for 128 KiB and 8 MiB).
    pub reservation: usize,
    /// C: exact stack bytes (2-16 KiB; default 8 KiB).
    pub exact: usize,
    /// B and C: bytes below the probe limit kept for the unwinder and for callees
    /// without probes (library code). `None` picks the default (16 KiB for B, 8 KiB
    /// for C). `deep-recursion` measures what unwinding uses: about 1.8 KiB from a
    /// frame without destructors and about 5 KiB when landing pads run drop code.
    pub probe_reserve: Option<usize>,
    /// A: guard bytes below each stack (default one page).
    pub guard: usize,
    /// B and C: bytes per large reservation (default 1 GiB of address space).
    pub chunk: usize,
    /// D: the strategy D decommits on top of (A or B).
    pub decommit_base: StrategyId,
    /// A: freed stacks kept mapped for reuse (default 0: every task maps and unmaps).
    pub cache: usize,
    /// B and C: write a canary at the bottom of every slot and check it on release
    /// (catches a missed probe; touches one extra page per task, so off for memory runs).
    pub canary: bool,
}

impl StrategyConfig {
    pub fn new(id: StrategyId) -> StrategyConfig {
        StrategyConfig {
            id,
            reservation: MIB,
            exact: 8 * KIB,
            probe_reserve: None,
            guard: 4 * KIB,
            chunk: GIB,
            decommit_base: StrategyId::A,
            cache: 0,
            canary: false,
        }
    }

    /// The label written to the CSV `strategy` column.
    pub fn label(&self) -> String {
        match self.id {
            StrategyId::A => "A".into(),
            StrategyId::B => "B".into(),
            StrategyId::C => "C".into(),
            StrategyId::D => format!("D({:?})", self.decommit_base),
            StrategyId::E => "E".into(),
        }
    }

    /// Settings that differ from their defaults and have no CSV column of their
    /// own, as `key=value` pairs for the `params` column, so that two runs of one
    /// strategy (say, with and without a stack cache) stay apart in the results.
    pub fn tuning_params(&self) -> Vec<(&'static str, String)> {
        let defaults = StrategyConfig::new(self.id);
        let base = if self.id == StrategyId::D { self.decommit_base } else { self.id };
        let mmap = base == StrategyId::A;
        let pooled = matches!(base, StrategyId::B | StrategyId::C);
        let mut out = Vec::new();
        if mmap && self.cache != defaults.cache {
            out.push(("stack_cache", self.cache.to_string()));
        }
        if mmap && self.guard != defaults.guard {
            out.push(("guard_bytes", self.guard.to_string()));
        }
        if pooled && let Some(reserve) = self.probe_reserve {
            out.push(("probe_reserve", reserve.to_string()));
        }
        if pooled && self.chunk != defaults.chunk {
            out.push(("chunk_bytes", self.chunk.to_string()));
        }
        if pooled && self.canary {
            out.push(("canary", "true".into()));
        }
        out
    }

    /// The probe reserve in effect.
    pub fn effective_probe_reserve(&self) -> usize {
        self.probe_reserve.unwrap_or(match self.id {
            StrategyId::C => 8 * KIB,
            _ => 16 * KIB,
        })
    }

    /// Stack bytes per task for the CSV `stack_bytes` column.
    pub fn stack_bytes(&self) -> Option<usize> {
        match self.id {
            StrategyId::C => Some(self.exact),
            StrategyId::E => None,
            _ => Some(self.reservation),
        }
    }

    /// Human-readable description for logs.
    pub fn describe(&self) -> String {
        match self.id {
            StrategyId::A => format!(
                "A: mmap per task, {} reservation, {} guard, cache {}",
                fmt_size(self.reservation),
                fmt_size(self.guard),
                self.cache
            ),
            StrategyId::B => format!(
                "B: pooled {} slots in {} reservations, probe reserve {}",
                fmt_size(self.reservation),
                fmt_size(self.chunk),
                fmt_size(self.effective_probe_reserve())
            ),
            StrategyId::C => format!(
                "C: exact {} stacks (+{} probe reserve) in {} reservations",
                fmt_size(self.exact),
                fmt_size(self.effective_probe_reserve()),
                fmt_size(self.chunk)
            ),
            StrategyId::D => format!(
                "D: {:?} with MADV_DONTNEED on park, {} reservation",
                self.decommit_base,
                fmt_size(self.reservation)
            ),
            StrategyId::E => "E: stackless futures on the same scheduler".into(),
        }
    }

    /// Validates the parameters.
    pub fn validate(&self) -> Result<(), String> {
        let page = os::page_size();
        if matches!(self.id, StrategyId::A | StrategyId::B | StrategyId::D) && self.reservation < 16 * KIB {
            return Err(format!("reservation {} is below 16K", fmt_size(self.reservation)));
        }
        if self.id == StrategyId::C && !(2 * KIB..=64 * KIB).contains(&self.exact) {
            return Err(format!("exact stack size {} outside 2K..64K", fmt_size(self.exact)));
        }
        if !self.guard.is_multiple_of(page) || self.guard == 0 {
            return Err(format!("guard {} must be a non-zero multiple of the page size", self.guard));
        }
        if self.id == StrategyId::D && !matches!(self.decommit_base, StrategyId::A | StrategyId::B) {
            return Err("D's base must be A or B".into());
        }
        Ok(())
    }
}

/// Builds the strategy described by `cfg`.
pub fn build(cfg: &StrategyConfig) -> Result<Arc<dyn StackStrategy>, String> {
    cfg.validate()?;
    let reserve = cfg.effective_probe_reserve();
    Ok(match cfg.id {
        StrategyId::A => Arc::new(guarded::GuardedMmap::new(cfg.reservation, cfg.guard, cfg.cache)),
        StrategyId::B => Arc::new(pool::SlotPool::pooled(cfg.reservation, reserve, cfg.chunk, cfg.canary)),
        StrategyId::C => Arc::new(pool::SlotPool::exact(cfg.exact, reserve, cfg.chunk, cfg.canary)),
        StrategyId::D => {
            let base: Arc<dyn StackStrategy> = match cfg.decommit_base {
                StrategyId::B => Arc::new(pool::SlotPool::pooled(cfg.reservation, reserve, cfg.chunk, cfg.canary)),
                _ => Arc::new(guarded::GuardedMmap::new(cfg.reservation, cfg.guard, cfg.cache)),
            };
            Arc::new(decommit::Decommit::new(base))
        }
        StrategyId::E => Arc::new(stackless::Stackless),
    })
}

/// A stack owned by one task: the region plus the strategy to return it to.
///
/// Implements corosensei's [`corosensei::stack::Stack`]; dropping it releases the
/// region to its strategy.
pub struct TaskStack {
    region: StackRegion,
    owner: *const dyn StackStrategy,
    clean: std::sync::atomic::AtomicBool,
}

// SAFETY: a TaskStack is an exclusive handle to memory owned by its strategy,
// which is `Send + Sync`; moving the handle to another thread moves nothing else.
unsafe impl Send for TaskStack {}
// SAFETY: the handle is immutable after construction; shared access only reads
// the region's addresses.
unsafe impl Sync for TaskStack {}

impl TaskStack {
    /// Wraps a region.
    ///
    /// # Safety
    /// `owner` must point to the strategy that allocated `region` and must stay
    /// valid until this `TaskStack` is dropped (the runtime keeps its strategy alive
    /// until every task has finished; see `rt::Runtime`).
    pub unsafe fn new(region: StackRegion, owner: *const dyn StackStrategy) -> TaskStack {
        TaskStack { region, owner, clean: std::sync::atomic::AtomicBool::new(false) }
    }

    pub fn region(&self) -> &StackRegion {
        &self.region
    }

    /// Records that nothing below the top page is committed, so that release can
    /// skip decommitting it.
    pub fn mark_clean(&self) {
        self.clean.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

impl Drop for TaskStack {
    fn drop(&mut self) {
        // SAFETY: `owner` is valid (constructor contract); the stack is dropped only
        // after the coroutine on it has finished or been unwound, so nothing runs on
        // the region any more.
        unsafe {
            if self.clean.load(std::sync::atomic::Ordering::Relaxed) {
                (*self.owner).release_clean(self.region)
            } else {
                (*self.owner).release(self.region)
            }
        }
    }
}

// SAFETY: base and limit describe memory exclusively owned by this handle.
// For A and D(A) the limit includes a guard page, as corosensei requires. For B
// and C there is no guard page; memory safety then rests on the software probes
// in every function of the task body (see `probe`), which is exactly the property
// strategy B is in this harness to measure. Task bodies on B/C stacks must call
// the probe at function entry and leave the probe reserve for unprobed callees.
unsafe impl corosensei::stack::Stack for TaskStack {
    #[inline]
    fn base(&self) -> corosensei::stack::StackPointer {
        corosensei::stack::StackPointer::new(self.region.base).expect("stack base is non-zero")
    }

    #[inline]
    fn limit(&self) -> corosensei::stack::StackPointer {
        corosensei::stack::StackPointer::new(self.region.limit).expect("stack limit is non-zero")
    }
}

/// Atomic counters shared by the strategy implementations.
#[derive(Default)]
pub(crate) struct AtomicCounters {
    pub allocated: std::sync::atomic::AtomicU64,
    pub released: std::sync::atomic::AtomicU64,
    pub reserved_bytes: std::sync::atomic::AtomicU64,
    pub reused: std::sync::atomic::AtomicU64,
    pub decommits: std::sync::atomic::AtomicU64,
    pub decommitted_bytes: std::sync::atomic::AtomicU64,
    pub decommits_skipped: std::sync::atomic::AtomicU64,
    pub canary_failures: std::sync::atomic::AtomicU64,
}

impl AtomicCounters {
    pub fn snapshot(&self) -> StackCounters {
        use std::sync::atomic::Ordering::Relaxed;
        StackCounters {
            allocated: self.allocated.load(Relaxed),
            released: self.released.load(Relaxed),
            reserved_bytes: self.reserved_bytes.load(Relaxed),
            reused: self.reused.load(Relaxed),
            decommits: self.decommits.load(Relaxed),
            decommitted_bytes: self.decommitted_bytes.load(Relaxed),
            decommits_skipped: self.decommits_skipped.load(Relaxed),
            canary_failures: self.canary_failures.load(Relaxed),
        }
    }
}
