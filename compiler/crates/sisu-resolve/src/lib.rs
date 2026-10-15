//! Name resolution for Sisu (proposal/18-next-steps.md, pipeline step 3).
//!
//! [`resolve`] takes the root module's AST, loads the prelude and the std
//! modules it imports (themselves Sisu source in `compiler/std/`),
//! declares every item, resolves every signature to a [`Ty`] and registers
//! methods. The result, a [`Program`], is the "resolved module" of the
//! query skeleton; bodies are checked by `sisu-typeck`.

mod arbitrary;
pub mod auto;
pub mod borrows;
mod collect;
mod derive;
pub mod doctests;
mod instances;
pub mod program;
mod shims;
pub mod solve;
pub mod std_sources;
mod template;
mod tests_of;
mod traits;
pub mod ty;
mod types;

pub use auto::{Auto, Obstacle};
pub use borrows::{ResultBorrows, ResultShape, SigParam};
pub use collect::{Resolved, SourceModule, SourcePackage, Sources, resolve, resolve_program, resolve_with};
pub use program::*;
pub use solve::{Answer, ImplSource};
pub use ty::{AssocTy, FnTy, Ty, TyKey, is_subset, union, unsized_part};

#[cfg(test)]
mod tests;
