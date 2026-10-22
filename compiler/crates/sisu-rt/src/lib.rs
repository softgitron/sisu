//! The runtime of natively compiled Sisu (proposal/18-next-steps.md,
//! "Bootstrapping", stage 2: "the runtime ... in Rust"): what the code
//! `sisu-codegen` writes calls, linked into every native executable as a
//! static library.
//!
//! It holds the program's type table, the panic in flight and its stack
//! trace, standard output, and every operation that is not a scalar
//! instruction: allocation, the structural operations on values of the
//! builtin types (drop, `.copy()`, equality, order, `Display` and
//! `Debug`), iterators, and the intrinsics, each implementing the contract
//! the interpreter's implements (its messages and exit statuses too). The
//! values are in the layout [`abi`] describes, which the code generator
//! shares by depending on this crate.
//!
//! A runtime entry point that can panic records the panic and returns
//! [`abi::UNWINDING`]; the generated code then runs its cleanup blocks, so
//! unwinding is ordinary control flow, with no unwinder.

pub mod abi;
mod bytes;
mod caps;
mod cells;
mod dict;
mod entry;
mod host;
mod intrinsics;
mod iter;
mod mem;
mod rt;
mod sched;
mod sized;
mod stack;
mod tasks;
mod value;
mod views;
mod wide;
