//! Strategy E: no task stacks at all.
//!
//! Task bodies are Rust `async` state machines polled on the worker threads of the
//! same scheduler (`Runtime::block_on_async`, `AsyncScope`), which is the lower bound
//! a C#-style stackless compilation of Sisu task bodies would reach. The type exists
//! so that E is one more [`StackStrategy`] in configuration and reports; it never
//! hands out a stack.

use super::{Exhaustion, StackCounters, StackError, StackKind, StackRegion, StackStrategy};

/// Marker strategy for stackless tasks.
pub struct Stackless;

impl StackStrategy for Stackless {
    fn label(&self) -> String {
        "E".into()
    }

    fn kind(&self) -> StackKind {
        StackKind::Stackless
    }

    fn exhaustion(&self) -> Exhaustion {
        Exhaustion::WorkerStackProbe
    }

    fn stack_bytes(&self) -> usize {
        0
    }

    fn allocate(&self) -> Result<StackRegion, StackError> {
        Err(StackError::NotApplicable("strategy E runs stackless futures; use the async API"))
    }

    unsafe fn release(&self, _region: StackRegion) {}

    fn counters(&self) -> StackCounters {
        StackCounters::default()
    }
}
