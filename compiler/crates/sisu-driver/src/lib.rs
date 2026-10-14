//! The driver: the query layer ([`db`]) that `sisu check`, `sisu run` and
//! later the language server share, the command line ([`cli`]), and the
//! counts of `sisu audit --ownership` ([`audit`]).

pub mod audit;
pub mod cli;
pub mod db;

pub use db::Db;

#[cfg(test)]
mod tests;
