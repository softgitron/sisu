//! The lossless syntax tree of Sisu and the AST built from it.
//!
//! `kinds` holds the token and node kinds and the lexical tables; `tree`
//! the arena tree, its builder and stable node identities; `ast` the owned
//! abstract syntax that later stages consume, built by `ast::build` as a
//! total function of the tree (every input yields an AST, with `Error`
//! nodes where the tree has them).

pub mod ast;
pub mod kinds;
pub mod tree;

pub use kinds::SyntaxKind;
pub use tree::{Checkpoint, Element, NodeId, StableId, Token, TokenId, Tree, TreeBuilder};
