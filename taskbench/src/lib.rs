//! taskbench: the bet-3 stack harness of the Sisu experiment (proposal/18,
//! Phase 1).
//!
//! The question: can stackful, colourless tasks on stacks that cannot move match
//! Go and Java virtual threads within the pre-registered factors? The crate has
//! no dependency on the Sisu compiler. It provides
//!
//! * [`stack`]: the five stack strategies A-E behind one trait, software stack
//!   probes and guard-page fault handling;
//! * [`rt`]: an M:N work-stealing scheduler with structured scopes, cancellation,
//!   parking, a timer, an epoll poller and an io_uring driver;
//! * [`bench`]: the benchmarks, each writing rows of the shared CSV schema
//!   ([`csv`]); and
//! * [`tokio_ref`]: the same workloads on tokio, the stackless reference.
//!
//! See `README.md` for how each benchmark maps to a bet-3 threshold.

pub mod bench;
pub mod cli;
pub mod csv;
pub mod machine;
pub mod mem;
pub mod perf;
pub mod rt;
pub mod stack;
pub mod stats;
pub mod tokio_ref;
pub mod units;
