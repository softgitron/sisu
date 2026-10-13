//! Message identifiers. The constants are generated from the catalog by
//! `build.rs`; a [`MsgId`] cannot be made from a string outside this crate.

/// The identifier of a catalog entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MsgId(&'static str);

impl MsgId {
    pub(crate) const fn new(id: &'static str) -> MsgId {
        MsgId(id)
    }

    pub const fn id(self) -> &'static str {
        self.0
    }
}

include!(concat!(env!("OUT_DIR"), "/msg_generated.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;

    #[test]
    fn every_constant_is_in_the_catalog() {
        let catalog = Catalog::parse(crate::catalog::BUILTIN_CATALOG).expect("catalog parses");
        assert_eq!(ALL.len(), catalog.entries().len());
        for m in ALL {
            assert!(catalog.get(m.id()).is_some(), "{} missing", m.id());
        }
        assert_eq!(PARSE_EXPECTED.id(), "parse.expected");
    }
}
