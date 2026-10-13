//! Scheduler unit tests: scope join, borrowing, cancellation, failure
//! propagation, work stealing, parking, stackless tasks, and stack exhaustion on
//! every strategy.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst};
use std::time::{Duration, Instant};

use super::*;
use crate::stack::probe::{LimitProbe, NoProbe, StackProbe};
use crate::stack::{Exhaustion, StrategyConfig, StrategyId};

fn cfg(id: StrategyId) -> StrategyConfig {
    let mut c = StrategyConfig::new(id);
    c.reservation = 256 * 1024;
    c.chunk = 64 << 20;
    c.canary = matches!(id, StrategyId::B | StrategyId::C);
    c
}

fn stackful() -> Vec<StrategyConfig> {
    let mut d_b = cfg(StrategyId::D);
    d_b.decommit_base = StrategyId::B;
    vec![cfg(StrategyId::A), cfg(StrategyId::B), cfg(StrategyId::C), cfg(StrategyId::D), d_b]
}

#[test]
fn block_on_every_strategy() {
    for c in stackful() {
        let rt = Runtime::new(2, &c).unwrap();
        assert_eq!(rt.block_on(|| 6 * 7), 42, "{}", c.label());
        assert_eq!(rt.live_tasks(), 0);
    }
}

#[test]
fn scope_joins_children_and_borrows_frame() {
    for c in stackful() {
        let rt = Runtime::new(4, &c).unwrap();
        let sum = rt.block_on(|| {
            let counter = AtomicUsize::new(0);
            let data: Vec<usize> = (1..=100).collect();
            let joined: usize = scope(|s| {
                let hs: Vec<_> = data
                    .iter()
                    .map(|x| {
                        let counter = &counter;
                        s.spawn(move || {
                            counter.fetch_add(1, SeqCst);
                            *x * 2
                        })
                    })
                    .collect();
                // Spawn some more and drop their handles: the scope still waits.
                for _ in 0..50 {
                    let counter = &counter;
                    drop(s.spawn(move || {
                        yield_now().unwrap();
                        counter.fetch_add(1, SeqCst);
                    }));
                }
                hs.into_iter().map(|h| h.join().unwrap()).sum()
            })
            .unwrap();
            assert_eq!(counter.load(SeqCst), 150, "scope returned before its children");
            joined
        });
        assert_eq!(sum, 10100, "{}", c.label());
        assert_eq!(rt.live_tasks(), 0);
        if c.canary {
            assert_eq!(rt.stack_counters().canary_failures, 0);
        }
    }
}

#[test]
fn nested_scopes() {
    let rt = Runtime::new(3, &cfg(StrategyId::B)).unwrap();
    let total = rt.block_on(|| {
        scope(|s| {
            let hs: Vec<_> = (0..8)
                .map(|i| {
                    s.spawn(move || {
                        scope(|inner| {
                            (0..8).map(|j| inner.spawn(move || i * 8 + j)).map(|h| h.join().unwrap()).sum::<usize>()
                        })
                        .unwrap()
                    })
                })
                .collect();
            hs.into_iter().map(|h| h.join().unwrap()).sum::<usize>()
        })
        .unwrap()
    });
    assert_eq!(total, (0..64).sum());
}

#[test]
fn cancellation_reaches_blocked_tasks_in_subtree() {
    for c in [cfg(StrategyId::A), cfg(StrategyId::B)] {
        let rt = Runtime::new(2, &c).unwrap();
        let (cancelled, finished) = rt.block_on(|| {
            let gate = Gate::new();
            let cancelled = AtomicUsize::new(0);
            let r = scope(|s| {
                for _ in 0..20 {
                    s.spawn(|| match sleep(Duration::from_secs(30)) {
                        Err(TaskError::Cancelled) => cancelled.fetch_add(1, SeqCst),
                        _ => 0,
                    });
                    s.spawn(|| {
                        // Nested scope: its tasks see the outer cancellation too.
                        scope(|inner| {
                            inner.spawn(|| {
                                if gate.wait(0) == Err(TaskError::Cancelled) {
                                    cancelled.fetch_add(1, SeqCst);
                                }
                            });
                        })
                        .unwrap();
                    });
                }
                // Let them block, then cancel the whole subtree.
                while gate.waiting() < 20 {
                    yield_now().unwrap();
                }
                sleep(Duration::from_millis(5)).unwrap();
                let t0 = Instant::now();
                s.cancel();
                t0
            });
            let t0 = r.unwrap();
            (cancelled.load(SeqCst), t0.elapsed())
        });
        assert_eq!(cancelled, 40, "{}", c.label());
        assert!(finished < Duration::from_secs(5), "cancellation was not prompt: {finished:?}");
    }
}

#[test]
fn handle_cancel_cancels_one_task() {
    let rt = Runtime::new(2, &cfg(StrategyId::B)).unwrap();
    rt.block_on(|| {
        scope(|s| {
            let a = s.spawn(|| sleep(Duration::from_secs(30)));
            let b = s.spawn(|| sleep(Duration::from_millis(10)));
            a.cancel();
            assert_eq!(a.join().unwrap(), Err(TaskError::Cancelled));
            assert_eq!(b.join().unwrap(), Ok(()));
        })
        .unwrap();
    });
}

#[test]
fn panics_are_reported_and_cancel_siblings() {
    let rt = Runtime::new(2, &cfg(StrategyId::B)).unwrap();
    rt.block_on(|| {
        // Joined panic: the joiner sees it; the sibling is cancelled.
        let sibling_cancelled = AtomicBool::new(false);
        let r = scope(|s| {
            let sib = s.spawn(|| {
                if sleep(Duration::from_secs(30)) == Err(TaskError::Cancelled) {
                    sibling_cancelled.store(true, SeqCst);
                }
            });
            let bad = s.spawn(|| -> u32 { panic!("boom") });
            let e = bad.join().unwrap_err();
            assert_eq!(e, TaskError::Panicked("boom".into()));
            sib.join().unwrap();
        });
        assert!(r.is_ok(), "a joined failure is not the scope's result");
        assert!(sibling_cancelled.load(SeqCst));
        // Unjoined panic: becomes the scope's result.
        let r = scope(|s| {
            drop(s.spawn(|| panic!("unobserved")));
        });
        assert_eq!(r, Err(TaskError::Panicked("unobserved".into())));
    });
}

#[test]
fn scope_body_panic_still_joins() {
    let rt = Runtime::new(2, &cfg(StrategyId::B)).unwrap();
    let done = Arc::new(AtomicUsize::new(0));
    let d2 = done.clone();
    let r = rt.try_block_on(move || {
        let _ = scope(|s| {
            for _ in 0..10 {
                let d = d2.clone();
                s.spawn(move || {
                    let _ = sleep(Duration::from_secs(30));
                    d.fetch_add(1, SeqCst);
                });
            }
            panic!("scope body fails");
        });
    });
    assert!(matches!(r, Err(TaskError::Panicked(m)) if m.contains("scope body fails")));
    assert_eq!(done.load(SeqCst), 10, "children not joined before the panic propagated");
}

#[test]
fn work_is_stolen_across_workers() {
    let rt = Runtime::new(4, &cfg(StrategyId::B)).unwrap();
    let threads = rt.block_on(|| {
        let seen = std::sync::Mutex::new(std::collections::HashSet::new());
        scope(|s| {
            for _ in 0..64 {
                s.spawn(|| {
                    // Busy for ~2 ms so that idle workers steal.
                    let t0 = Instant::now();
                    while t0.elapsed() < Duration::from_millis(2) {
                        std::hint::spin_loop();
                    }
                    seen.lock().unwrap().insert(std::thread::current().id());
                });
            }
        })
        .unwrap();
        seen.into_inner().unwrap().len()
    });
    assert!(threads >= 2, "all tasks ran on {threads} thread(s)");
    assert!(rt.stats().steals > 0 || threads >= 2);
}

#[test]
fn park_and_unpark_by_waker() {
    let rt = Runtime::new(2, &cfg(StrategyId::A)).unwrap();
    rt.block_on(|| {
        let slot: std::sync::Mutex<Option<std::task::Waker>> = std::sync::Mutex::new(None);
        let flag = AtomicBool::new(false);
        scope(|s| {
            s.spawn(|| {
                *slot.lock().unwrap() = Some(current_waker());
                while !flag.load(SeqCst) {
                    park();
                }
            });
            loop {
                if let Some(w) = slot.lock().unwrap().take() {
                    flag.store(true, SeqCst);
                    w.wake();
                    break;
                }
                yield_now().unwrap();
            }
        })
        .unwrap();
    });
    assert!(rt.stats().parks >= 1);
}

#[test]
fn yields_interleave_many_tasks() {
    let rt = Runtime::new(2, &cfg(StrategyId::C)).unwrap();
    let n = rt.block_on(|| {
        let n = AtomicUsize::new(0);
        scope(|s| {
            for _ in 0..200 {
                s.spawn(|| {
                    for _ in 0..50 {
                        n.fetch_add(1, SeqCst);
                        yield_now().unwrap();
                    }
                });
            }
        })
        .unwrap();
        n.load(SeqCst)
    });
    assert_eq!(n, 10_000);
    assert!(rt.stats().yields >= 10_000);
}

#[test]
fn stackless_scope_join_and_cancel() {
    let rt = Runtime::new(2, &StrategyConfig::new(StrategyId::E)).unwrap();
    assert!(rt.is_stackless());
    let (sum, cancelled) = rt.block_on_async(async {
        let s = AsyncScope::new();
        let hs: Vec<_> = (0..100u64).map(|i| s.spawn(async move { i })).collect();
        let mut sum = 0;
        for h in hs {
            sum += h.join().await.unwrap();
        }
        let sleeper = s.spawn(async { sleep_async(Duration::from_secs(30)).await });
        s.cancel();
        let cancelled = sleeper.join().await;
        s.join_all().await.unwrap();
        (sum, cancelled)
    });
    assert_eq!(sum, 4950);
    // The joiner is in the cancelled scope's parent, so it sees the child's result.
    assert_eq!(cancelled, Ok(Err(TaskError::Cancelled)));
}

#[test]
fn stackless_rejects_stackful_spawn() {
    let rt = Runtime::new(1, &StrategyConfig::new(StrategyId::E)).unwrap();
    assert!(matches!(rt.try_block_on(|| 1), Err(TaskError::SpawnFailed(_))));
}

/// Recursion with a 256-byte frame and a probe at entry.
#[inline(never)]
fn recurse<P: StackProbe>(p: P, depth: u64, guard: &DropCounter) -> u64 {
    p.check();
    let pad = std::hint::black_box([depth as u8; 256]);
    if depth == 0 {
        return pad[0] as u64;
    }
    let _local = guard.token();
    std::hint::black_box(recurse(p, depth - 1, guard)) + pad[1] as u64
}

struct DropCounter(AtomicUsize, AtomicUsize);
struct Token<'a>(&'a DropCounter);
impl DropCounter {
    fn token(&self) -> Token<'_> {
        self.0.fetch_add(1, SeqCst);
        Token(self)
    }
}
impl Drop for Token<'_> {
    fn drop(&mut self) {
        self.0.1.fetch_add(1, SeqCst);
    }
}

#[test]
fn exhaustion_is_a_clean_task_failure_on_every_strategy() {
    // At the default probe reserve (the owner's 5 KiB). The recursion has a
    // 256-byte local and a destructor in every frame; the canary checks that the
    // unwind stays inside the reserve. It did not before `warm_unwinder`: the
    // first unwind of the process also ran the dynamic linker's resolver there.
    for c in stackful() {
        let rt = Runtime::new(2, &c).unwrap();
        let probes = rt.stack_counters();
        let _ = probes;
        let (err, created, dropped) = rt.block_on(|| {
            let counter = DropCounter(AtomicUsize::new(0), AtomicUsize::new(0));
            let r = scope(|s| {
                s.spawn(|| {
                    let p = probe();
                    if p.limit() == 0 { recurse(NoProbe, 1 << 30, &counter) } else { recurse(p, 1 << 30, &counter) }
                })
                .join()
            })
            .unwrap();
            (r.unwrap_err(), counter.0.load(SeqCst), counter.1.load(SeqCst))
        });
        let TaskError::StackExhausted { how, .. } = &err else {
            panic!("{}: expected exhaustion, got {err:?}", c.label())
        };
        match c.id {
            StrategyId::A => assert_eq!(*how, Exhaustion::GuardPage),
            StrategyId::D if c.decommit_base == StrategyId::A => assert_eq!(*how, Exhaustion::GuardPage),
            _ => {
                assert_eq!(*how, Exhaustion::SoftwareProbe, "{}", c.label());
                assert_eq!(created, dropped, "{}: destructors skipped", c.label());
            }
        }
        // The runtime keeps working, on a recycled stack.
        assert_eq!(rt.block_on(|| scope(|s| s.spawn(|| 5).join().unwrap()).unwrap()), 5);
        assert_eq!(rt.live_tasks(), 0);
        if c.canary {
            assert_eq!(rt.stack_counters().canary_failures, 0, "{}: probe missed", c.label());
        }
    }
}

#[test]
fn trapped_task_with_open_scope_is_quarantined() {
    let rt = Runtime::new(2, &cfg(StrategyId::A)).unwrap();
    let outcome = rt.block_on(|| {
        let gate = Gate::new();
        let child_done = AtomicBool::new(false);
        let r = scope(|outer| {
            let h = outer.spawn(|| {
                scope(|s| {
                    s.spawn(|| {
                        let _ = gate.wait(0);
                        child_done.store(true, SeqCst);
                    });
                    while gate.waiting() == 0 {
                        yield_now().unwrap();
                    }
                    let c = DropCounter(AtomicUsize::new(0), AtomicUsize::new(0));
                    recurse(NoProbe, 1 << 30, &c)
                })
            });
            // The trap cancels the inner scope; its child wakes with Cancelled and
            // finishes, after which the trapped task completes.
            h.join()
        })
        .unwrap();
        (r, child_done.load(SeqCst))
    });
    assert!(matches!(outcome.0, Err(TaskError::StackExhausted { how: Exhaustion::GuardPage, .. })), "{:?}", outcome.0);
    assert!(outcome.1, "child of the trapped task did not finish first");
    assert_eq!(rt.stats().quarantined, 1);
    assert_eq!(rt.live_tasks(), 0);
}

#[test]
fn decommit_on_park_returns_excursion_pages() {
    let mut c = cfg(StrategyId::D);
    c.reservation = 1 << 20;
    let rt = Runtime::new(1, &c).unwrap();
    rt.block_on(|| {
        let gate = Gate::new();
        scope(|s| {
            s.spawn(|| {
                excursion(256 * 1024);
                let _ = gate.wait(0);
            });
            while gate.waiting() == 0 {
                yield_now().unwrap();
            }
            gate.open();
        })
        .unwrap();
    });
    let counters = rt.stack_counters();
    assert!(counters.decommits >= 1);
    assert!(counters.decommitted_bytes >= 200 * 1024, "{counters:?}");
}

/// Touches `bytes` of stack below the caller.
#[inline(never)]
fn excursion(bytes: usize) {
    let p = LimitProbe::disabled();
    p.check();
    let mut buf = [0u8; 16 * 1024];
    let mut i = 0;
    while i < buf.len() {
        buf[i] = 1;
        i += 4096;
    }
    std::hint::black_box(&mut buf);
    if bytes > buf.len() {
        excursion(bytes - buf.len());
    }
}

/// How far below the stack pointer recorded by `park` the context switch writes.
/// Strategy D keeps this much (plus a safety factor) when it decommits on park.
#[test]
fn park_switch_frame_is_shallow() {
    let rt = Runtime::new(1, &cfg(StrategyId::B)).unwrap();
    let depth = rt.block_on(|| {
        let gate = Gate::new();
        let depth = AtomicUsize::new(0);
        scope(|s| {
            s.spawn(|| {
                // Paint 8 KiB below the current stack pointer, then park.
                let sp = crate::stack::probe::current_sp();
                let low = sp - 8192;
                for a in low..sp - 64 {
                    // SAFETY: below the stack pointer of this task's own stack, which
                    // the strategy (B, 256 KiB slots) guarantees is mapped and unused.
                    unsafe { std::ptr::write_volatile(a as *mut u8, 0xA5) };
                }
                let _ = gate.wait(0);
                // The highest painted byte that was overwritten bounds the switch frame.
                let mut lowest_touched = sp - 64;
                for a in low..sp - 64 {
                    // SAFETY: as above.
                    if unsafe { std::ptr::read_volatile(a as *const u8) } != 0xA5 {
                        lowest_touched = lowest_touched.min(a);
                    }
                }
                depth.store(sp - lowest_touched, SeqCst);
            });
            while gate.waiting() == 0 {
                yield_now().unwrap();
            }
            gate.open();
        })
        .unwrap();
        depth.load(SeqCst)
    });
    eprintln!("park writes {depth} bytes below the caller's stack pointer");
    assert!(depth < crate::stack::decommit::SWITCH_MARGIN, "switch frame deeper than D's margin: {depth}");
}

/// A panic on a 2 KiB exact stack is reported without overflowing it: the hook
/// runs on the worker's stack.
#[test]
fn panic_on_small_stack_is_reported() {
    let mut c = cfg(StrategyId::C);
    c.exact = 2048;
    let rt = Runtime::new(2, &c).unwrap();
    let r = rt.block_on(|| scope(|s| s.spawn(|| -> u8 { panic!("small stack {}", 42) }).join()).unwrap());
    assert_eq!(r, Err(TaskError::Panicked("small stack 42".into())));
    assert_eq!(rt.stack_counters().canary_failures, 0);
}
