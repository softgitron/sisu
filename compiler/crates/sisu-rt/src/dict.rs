//! `dict` and `set` (11, "Collections"): their tables are the runtime in
//! Sisu's (`compiler/rt/src/table.sisu`, decision 422), insertion-ordered
//! over raw memory with an index of slots over the keys' structural hash,
//! as the interpreter keeps them (`sisu-interp/src/value.rs`, `Dict` and
//! `Set`). A value of either type is a word holding its table's address.
//! Here are the table's header as the runtime in Rust reads it, the
//! addresses of its entries, and the calls of the exports that make a
//! table, look a key up, append an entry, take one out and drop what is
//! left; the runtime in Sisu's `value.sisu` copies, compares, shows and
//! drops a whole table itself.

use crate::bridge::TablePart;
use crate::value::{self, target};

/// A table, as `table.sisu` lays it out: the runtime in Rust reads its
/// header and finds its entries only through the runtime in Sisu's exports
/// (decision 506), so the layout is written once.
pub(crate) struct Table {
    _opaque: [u8; 0],
}

impl Table {
    /// A new table of keys of type `key` and values of type `val`
    /// (`u32::MAX`, `()`, for a set's).
    pub(crate) fn new(key: u32, val: u32) -> *mut Table {
        let val = if crate::rt::ty_checked(val).is_some() { i64::from(val) } else { -1 };
        crate::bridge::table_new(i64::from(key), val).cast::<Table>()
    }

    fn addr(&self) -> *mut u8 {
        (self as *const Table).cast_mut().cast()
    }

    fn part(&self, part: TablePart) -> u64 {
        // SAFETY: a `Table` is a live table's address.
        unsafe { crate::bridge::table_word(self.addr(), part) }
    }

    /// The type of its keys.
    pub(crate) fn key(&self) -> u32 {
        self.part(TablePart::KeyType) as u32
    }

    pub(crate) fn len(&self) -> u64 {
        self.part(TablePart::Len)
    }

    /// The bytes of a key.
    pub(crate) fn ksize(&self) -> u64 {
        self.part(TablePart::KeyBytes)
    }

    /// The bytes of a value (0 for a set's).
    pub(crate) fn vsize(&self) -> u64 {
        self.part(TablePart::ValueBytes)
    }

    /// The address of entry `i`'s value.
    pub(crate) fn val_at(&self, i: u64) -> *mut u8 {
        // SAFETY: a `Table` is a live table's address, and the callers ask
        // for its entries.
        unsafe { crate::bridge::table_value_at(self.addr(), i) }
    }

    /// The position of the key at `k` (of type `kt`), if it is there.
    ///
    /// # Safety
    /// `k` must hold a value of type `kt`.
    pub(crate) unsafe fn position(&mut self, k: *mut u8, kt: u32) -> Option<u64> {
        // SAFETY: as the caller guarantees.
        let i = unsafe { crate::bridge::table_lookup((self as *mut Table).cast(), k, kt) };
        u64::try_from(i).ok()
    }

    /// Appends an entry whose key is moved in from `k` (of the table's key
    /// type, and not in it yet) and whose value is left to the caller to
    /// write; its position.
    ///
    /// # Safety
    /// `k` must hold a key, which the table owns afterwards.
    pub(crate) unsafe fn push_key(&mut self, k: *const u8) -> u64 {
        // SAFETY: as the caller guarantees.
        unsafe { crate::bridge::table_push((self as *mut Table).cast(), k.cast_mut()) }
    }

    /// Removes entry `i`, moving its key and value to `out` (key then
    /// value), and keeps the order of the others.
    ///
    /// # Safety
    /// `out` must have room for an entry's key and value.
    pub(crate) unsafe fn remove(&mut self, i: u64, out: *mut u8) {
        // SAFETY: as the caller guarantees.
        unsafe { crate::bridge::table_remove((self as *mut Table).cast(), i, out) }
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

pub(crate) fn into_word(t: *mut Table) -> u64 {
    t as u64
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

/// Copies the key at `k` (of type `kt`, perhaps behind references) into
/// `dst` as the table's key type.
///
/// # Safety
/// `k` must hold a key and `dst` have room for one.
pub(crate) unsafe fn copy_key(dst: *mut u8, k: *mut u8, kt: u32) {
    // SAFETY: as the caller guarantees.
    unsafe {
        let (p, t) = target(k, kt);
        value::copy_value(dst, p, t);
    }
}
