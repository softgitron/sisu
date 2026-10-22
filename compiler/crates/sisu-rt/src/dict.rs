//! `dict` and `set` (11, "Collections"): insertion-ordered, with a hash
//! index over the keys' structural hash, as the interpreter keeps them
//! (`sisu-interp/src/value.rs`, `Dict` and `Set`). A value of either type
//! is a word holding the address of its heap object.

use crate::abi::{self, Kind};
use crate::mem::{self, List};
use crate::rt::{self, ty};
use crate::value::{self, target};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// A dict's entries, each its key's bytes then its value's, in the order
/// they were inserted; a set's are its elements (`val` is `()`).
pub(crate) struct Table {
    pub key: u32,
    pub val: u32,
    pub ksize: u32,
    pub vsize: u32,
    pub entries: List,
    index: HashMap<u64, Vec<u64>>,
    _live: crate::mem::Live,
}

impl Table {
    pub(crate) fn new(key: u32, val: u32) -> Box<Table> {
        Box::new(Table {
            key,
            val,
            ksize: ty(key).size,
            vsize: ty(val).size,
            entries: List::empty(),
            index: HashMap::new(),
            _live: crate::mem::Live::new(),
        })
    }

    fn entry_size(&self) -> u32 {
        self.ksize + self.vsize
    }

    pub(crate) fn len(&self) -> u64 {
        self.entries.len
    }

    /// The address of entry `i`'s key.
    pub(crate) fn key_at(&self, i: u64) -> *mut u8 {
        self.entries.at(i, self.entry_size())
    }

    /// The address of entry `i`'s value.
    pub(crate) fn val_at(&self, i: u64) -> *mut u8 {
        // SAFETY: the value follows the key within the entry.
        unsafe { self.key_at(i).add(self.ksize as usize) }
    }

    /// The position of the key at `k` (of type `kt`), if it is there.
    ///
    /// # Safety
    /// `k` must hold a value of type `kt`.
    pub(crate) unsafe fn position(&self, k: *mut u8, kt: u32) -> Option<u64> {
        // SAFETY: as the caller guarantees.
        unsafe {
            let h = hash_value(k, kt);
            let candidates = self.index.get(&h)?;
            candidates.iter().copied().find(|&i| key_equal(self.key_at(i), self.key, k, kt))
        }
    }

    /// Appends an entry whose key is moved in from `k` (of the table's key
    /// type) and whose value is left to the caller to write; its position.
    ///
    /// # Safety
    /// `k` must hold a key, which the table owns afterwards.
    pub(crate) unsafe fn push_key(&mut self, k: *const u8) -> u64 {
        let size = self.entry_size();
        self.entries.reserve(1, size);
        let i = self.entries.len;
        // SAFETY: there is room for the entry.
        unsafe {
            mem::copy(self.key_at(i), k, self.ksize as usize);
            let h = hash_value(self.key_at(i), self.key);
            self.index.entry(h).or_default().push(i);
        }
        self.entries.len += 1;
        i
    }

    /// Removes entry `i`, moving its key and value to `out` (key then
    /// value), and keeps the order of the others.
    ///
    /// # Safety
    /// `out` must have room for an entry.
    pub(crate) unsafe fn remove(&mut self, i: u64, out: *mut u8) {
        let size = self.entry_size();
        // SAFETY: the entry is in the buffer.
        unsafe {
            let h = hash_value(self.key_at(i), self.key);
            mem::copy(out, self.key_at(i), size as usize);
            let rest = (self.entries.len - i - 1) * u64::from(size);
            mem::copy(self.key_at(i), self.key_at(i + 1), rest as usize);
            if let Some(v) = self.index.get_mut(&h) {
                v.retain(|&p| p != i);
                if v.is_empty() {
                    self.index.remove(&h);
                }
            }
        }
        self.entries.len -= 1;
        for v in self.index.values_mut() {
            for p in v.iter_mut() {
                if *p > i {
                    *p -= 1;
                }
            }
        }
    }
}

/// The table a `dict` or `set` value at `p` holds.
///
/// # Safety
/// `p` must hold a dict or a set.
pub(crate) unsafe fn table<'a>(p: *mut u8) -> &'a mut Table {
    // SAFETY: the word is the address of the table.
    unsafe { &mut *(mem::word(p, 0) as *mut Table) }
}

pub(crate) fn into_word(t: Box<Table>) -> u64 {
    Box::into_raw(t) as u64
}

/// Keys are equal when their values are, except that a `Path` is never
/// the same key as a text (the interpreter's `Key::Path`).
///
/// # Safety
/// Both must hold values of their types.
unsafe fn key_equal(a: *mut u8, ta: u32, b: *mut u8, tb: u32) -> bool {
    // SAFETY: as the caller guarantees.
    unsafe {
        let (_, ra) = target(a, ta);
        let (_, rb) = target(b, tb);
        let path = |t: u32| matches!(ty(t).kind, Kind::Path);
        path(ra) == path(rb) && value::equal(a, ta, b, tb)
    }
}

/// The structural hash of a key: equal keys hash alike.
///
/// # Safety
/// `p` must hold a value of type `t`.
pub(crate) unsafe fn hash_value(p: *mut u8, t: u32) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    // SAFETY: as the caller guarantees.
    unsafe { hash_into(&mut h, p, t) };
    h.finish()
}

/// # Safety
/// `p` must hold a value of type `t`.
unsafe fn hash_into(h: &mut impl Hasher, p: *mut u8, t: u32) {
    // SAFETY: the parts are where the type says.
    unsafe {
        let (p, t) = target(p, t);
        let entry = ty(t);
        match &entry.kind {
            Kind::Str | Kind::String => {
                0u8.hash(h);
                value::text(p, t).unwrap_or(&[]).hash(h);
            }
            Kind::Path => {
                1u8.hash(h);
                value::text(p, t).unwrap_or(&[]).hash(h);
            }
            Kind::Bytes => {
                2u8.hash(h);
                mem::at::<crate::mem::Text>(p).bytes().hash(h);
            }
            Kind::Int
            | Kind::Sized(_)
            | Kind::U64
            | Kind::Bool
            | Kind::Char
            | Kind::Float
            | Kind::DateTime => {
                mem::word(p, 0).hash(h);
            }
            Kind::F32 => (mem::word(p, 0) as u32).hash(h),
            Kind::I128 | Kind::U128 => crate::wide::bits(p).hash(h),
            Kind::Unit => {}
            Kind::Tuple(slots) => {
                for s in slots {
                    hash_into(h, p.add(s.offset as usize), s.ty);
                }
            }
            Kind::List(e) => {
                let l = mem::at::<List>(p);
                let size = ty(*e).size;
                l.len.hash(h);
                for i in 0..l.len {
                    hash_into(h, l.at(i, size), *e);
                }
            }
            Kind::Option(payload) => {
                let tag = value::tag(p, entry);
                tag.hash(h);
                if tag == 0 {
                    hash_into(h, p.add(8), *payload);
                }
            }
            Kind::Result(a, b) => {
                let tag = value::tag(p, entry);
                tag.hash(h);
                hash_into(h, p.add(8), if tag == 0 { *a } else { *b });
            }
            Kind::Box(inner) => hash_into(h, mem::word(p, 0) as *mut u8, *inner),
            Kind::Struct { name, fields, .. } => {
                name.hash(h);
                for f in fields {
                    hash_into(h, p.add(f.offset as usize), f.ty);
                }
            }
            Kind::Enum { .. } | Kind::ErrSet(_) => {
                let decl = value::concrete(p, t);
                let e = ty(decl);
                if let Kind::Enum { name, variants, .. } = &e.kind {
                    name.hash(h);
                    let tag = value::tag(p, e);
                    tag.hash(h);
                    if let Some(v) = variants.get(tag as usize) {
                        for f in &v.fields {
                            hash_into(h, p.add(f.offset as usize), f.ty);
                        }
                    }
                }
            }
            Kind::Opaque(_) => mem::word(p, 0).hash(h),
            _ => {}
        }
    }
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

/// Drops a table and what it holds: each entry's value, then its key, in
/// order.
///
/// # Safety
/// The table must be one made here, dropped once.
pub(crate) unsafe fn drop_table(t: *mut Table) -> u8 {
    if t.is_null() {
        return abi::OK;
    }
    // SAFETY: the table owns its entries.
    unsafe {
        let t = Box::from_raw(t);
        let mut status = abi::OK;
        let (kd, vd) = (rt::needs_drop(t.key), rt::needs_drop(t.val));
        if kd || vd {
            for i in 0..t.entries.len {
                if vd {
                    status = value::after_part(status, value::drop_value(t.val_at(i), t.val));
                }
                if kd {
                    status = value::after_part(status, value::drop_value(t.key_at(i), t.key));
                }
            }
        }
        mem::release(t.entries.ptr);
        status
    }
}

/// A deep copy of a table.
///
/// # Safety
/// `t` must be a table.
pub(crate) unsafe fn copy_table(t: &Table) -> Box<Table> {
    let mut out = Table::new(t.key, t.val);
    let size = t.entry_size();
    // SAFETY: both tables' entries are in their buffers.
    unsafe {
        for i in 0..t.entries.len {
            out.entries.reserve(1, size);
            let j = out.entries.len;
            value::copy_value(out.key_at(j), t.key_at(i), t.key);
            value::copy_value(out.val_at(j), t.val_at(i), t.val);
            let h = hash_value(out.key_at(j), out.key);
            out.index.entry(h).or_default().push(j);
            out.entries.len += 1;
        }
    }
    out
}
