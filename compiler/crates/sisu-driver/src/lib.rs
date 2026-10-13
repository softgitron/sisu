//! The driver: the query layer ([`db`]) that `sisu check`, `sisu run` and
//! later the language server share, and the command line ([`cli`]).

pub mod cli;
pub mod db;

pub use db::Db;

#[cfg(test)]
mod tests;
