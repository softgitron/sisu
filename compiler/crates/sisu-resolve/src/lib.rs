//! Name resolution for Sisu (proposal/18-next-steps.md, pipeline step 3).
//!
//! [`resolve`] takes the root module's AST, loads the prelude and the std
//! modules it imports (themselves Sisu source in `compiler/std/`),
//! declares every item, resolves every signature to a [`Ty`] and registers
//! methods. The result, a [`Program`], is the "resolved module" of the
//! query skeleton; bodies are checked by `sisu-typeck`.

mod collect;
pub mod program;
pub mod std_sources;
pub mod ty;

pub use collect::{Resolved, resolve};
pub use program::*;
pub use ty::{FnTy, Ty, TyKey, is_subset, union};

#[cfg(test)]
mod tests;
