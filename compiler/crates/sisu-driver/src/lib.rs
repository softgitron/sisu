//! The driver: the query layer ([`db`]) that `sisu check`, `sisu run` and
//! later the language server share, the command line ([`cli`]), the
//! counts of `sisu audit --ownership` ([`audit`]), the signatures of
//! `sisu check --explain` ([`doc`]) and the replay tokens of failing tests
//! ([`replay`]).

pub mod audit;
pub mod cli;
pub mod db;
pub mod doc;
pub mod json;
pub mod lsp;
pub mod manifest;
pub mod package;
pub mod repl;
pub mod replay;
pub mod sim;
pub mod snap;
pub mod toml;
pub mod transcript;

pub use db::Db;

#[cfg(test)]
mod tests;
