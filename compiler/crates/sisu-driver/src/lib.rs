//! The driver: the query layer ([`db`]) that `sisu check`, `sisu run` and
//! later the language server share, the command line ([`cli`]), the
//! counts of `sisu audit --ownership` ([`audit`]), the pages of `sisu
//! doc` ([`doc`]) and the replay tokens of failing tests ([`replay`]).

pub mod assist;
pub mod audit;
pub mod cfg;
pub mod cli;
pub mod db;
pub mod diary;
pub mod doc;
pub mod format;
pub mod items;
pub mod json;
pub mod lsp;
pub mod manifest;
mod native;
pub mod package;
pub mod repl;
pub mod replay;
pub mod sim;
pub mod snap;
pub mod testcache;
pub mod toml;
pub mod transcript;

pub use db::Db;

#[cfg(test)]
mod tests;
