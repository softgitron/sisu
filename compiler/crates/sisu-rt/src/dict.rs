//! `dict` and `set` (11, "Collections"): their tables are the runtime in
//! Sisu's (`compiler/rt/src/table.sisu`, decision 422), insertion-ordered
//! over raw memory with an index of slots over the keys' structural hash,
//! as the interpreter keeps them (`sisu-interp/src/value.rs`, `Dict` and
//! `Set`). A value of either type is a word holding its table's address.
//! Here is the table's header as the runtime in Rust reads it, its length;
//! a table's literal, the generated code's lookups and `d{k}`'s slot
//! (decision 681) are the runtime in Sisu's exports; the intrinsics of dicts and sets are the
//! runtime in Sisu's, which decodes their calls (`dispatch.sisu`, decision
//! 625), and its `value.sisu` copies, compares, shows and drops a whole
//! table itself.

use crate::bridge::TablePart;

/// A table, as `table.sisu` lays it out: the runtime in Rust reads its
/// header and finds its entries only through the runtime in Sisu's exports
/// (decision 506), so the layout is written once.
pub(crate) struct Table {
    _opaque: [u8; 0],
}

impl Table {
    fn addr(&self) -> *mut u8 {
        (self as *const Table).cast_mut().cast()
    }

    fn part(&self, part: TablePart) -> u64 {
        // SAFETY: a `Table` is a live table's address.
        unsafe { crate::bridge::table_word(self.addr(), part) }
    }

    pub(crate) fn len(&self) -> u64 {
        self.part(TablePart::Len)
    }
}

/// The table a `dict` or `set` value at `p` holds.
///
/// # Safety
/// `p` must hold a dict or a set.
pub(crate) unsafe fn table<'a>(p: *mut u8) -> &'a mut Table {
    // SAFETY: the word is the address of the table.
    unsafe { &mut *(crate::mem::word(p, 0) as *mut Table) }
}

/// The structural hash of the value at `p` (decision 256), as the
/// interpreter computes it (`Interp::structural_hash`): its parts mixed
/// into `state` as `Hasher.write_int` mixes an `int`, `(state ^ n) *%
/// 1099511628211`, in the order the derived impls and the prelude's own
/// impls hash them. The runtime in Sisu's (decision 350).
///
/// # Safety
/// `p` must hold a value of type `t`.
pub(crate) unsafe fn structural_hash(state: u64, p: *mut u8, t: u32) -> u64 {
    // SAFETY: as the caller guarantees.
    unsafe { crate::bridge::hash(state, p, t) }
}
