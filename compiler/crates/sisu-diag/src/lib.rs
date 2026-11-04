//! Diagnostics infrastructure for the Sisu front end (proposal/10-tooling.md,
//! "Diagnostics"; D-065).
//!
//! A diagnostic is a message identifier with named parameters. The English
//! text lives in `compiler/catalog/messages.catalog`, never in code: code can
//! only name a [`MsgId`] constant from [`msg`], and this crate's private
//! constructor makes it impossible to build one from a string elsewhere.
//! Severity comes from the catalog entry, so code cannot disagree with it.

pub mod catalog;
pub mod diagnostic;
pub mod diff;
pub mod msg;
pub mod render;
pub mod source;
pub mod span;
pub mod trace;

pub use catalog::{Catalog, Entry, Kind};
pub use diagnostic::{Diagnostic, Edit, Fix, Label, Note, Severity};
pub use msg::MsgId;
pub use source::{LineCol, SourceFile, SourceMap};
pub use span::{FileId, Span, TextRange};
