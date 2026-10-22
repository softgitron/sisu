//! Iterators (`Iter[T]`): a word holding the address of a heap object
//! that yields items of a fixed size, as the interpreter's `IterState`
//! does (`sisu-interp/src/iter.rs`).

use crate::abi::{self, Kind};
use crate::dict::Table;
use crate::mem::{self, List, Str};
use crate::rt::{self, ty};
use crate::value;
use sisu_diag::msg;

pub(crate) struct IterObj {
    pub state: State,
    /// The bytes of each item.
    pub item_size: u32,
}

pub(crate) enum State {
    Range {
        cur: i64,
        stop: i64,
        step: i64,
    },
    /// The addresses of the elements of a borrowed list.
    ListRef {
        list: *mut List,
        idx: u64,
        elem_size: u32,
    },
    /// Owned values, moved out one by one; what is left is dropped with
    /// the iterator.
    Items {
        list: List,
        idx: u64,
        elem: u32,
    },
    /// The characters of a text, as words.
    Chars {
        text: Str,
        pos: u64,
    },
    /// The bytes of a text, as words.
    Bytes {
        text: Str,
        pos: u64,
    },
    Enumerate {
        inner: Box<IterObj>,
        n: i64,
    },
    Zip {
        a: Box<IterObj>,
        b: Box<IterObj>,
        strict: bool,
        /// The types of the items, to drop the one left over when the
        /// lengths differ.
        a_ty: u32,
        b_ty: u32,
    },
    /// The values the inner iterator's references point to, copied (a
    /// `Copy` type's items from a borrowed collection).
    Decay {
        inner: Box<IterObj>,
    },
    /// References into a borrowed dict or set: its keys, values, pairs of
    /// both, or elements.
    TableRef {
        table: *mut Table,
        idx: u64,
        part: Part,
    },
    /// The entries of a dict or set it owns, moved out one by one (a
    /// dict's as `(K, V)`); what is left is dropped with the iterator.
    TableOwn {
        table: Box<Table>,
        idx: u64,
    },
    Done,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Part {
    Keys,
    Values,
    Items,
}

/// An iterator over a borrowed dict's keys, values or pairs, or a set's
/// elements (`Keys`).
pub(crate) fn table_ref(table: *mut Table, part: Part) -> Box<IterObj> {
    let item_size = if part == Part::Items { 16 } else { 8 };
    Box::new(IterObj { state: State::TableRef { table, idx: 0, part }, item_size })
}

/// An iterator that owns a dict's or set's entries.
pub(crate) fn table_own(table: Box<Table>) -> Box<IterObj> {
    let item_size = table.ksize + table.vsize;
    Box::new(IterObj { state: State::TableOwn { table, idx: 0 }, item_size })
}

/// The heap object of a new iterator, as the word that holds it.
pub(crate) fn into_word(obj: Box<IterObj>) -> u64 {
    Box::into_raw(obj) as u64
}

pub(crate) fn range(cur: i64, stop: i64, step: i64) -> Box<IterObj> {
    Box::new(IterObj { state: State::Range { cur, stop, step }, item_size: 8 })
}

/// An iterator over the elements of a borrowed list: their addresses.
pub(crate) fn list_ref(list: *mut List, elem: u32) -> Box<IterObj> {
    Box::new(IterObj { state: State::ListRef { list, idx: 0, elem_size: ty(elem).size }, item_size: 8 })
}

/// An iterator that owns a list's elements.
pub(crate) fn items(list: List, elem: u32) -> Box<IterObj> {
    Box::new(IterObj { state: State::Items { list, idx: 0, elem }, item_size: ty(elem).size })
}

pub(crate) fn chars(text: Str) -> Box<IterObj> {
    Box::new(IterObj { state: State::Chars { text, pos: 0 }, item_size: 8 })
}

pub(crate) fn bytes(text: Str) -> Box<IterObj> {
    Box::new(IterObj { state: State::Bytes { text, pos: 0 }, item_size: 8 })
}

pub(crate) fn enumerate(inner: Box<IterObj>) -> Box<IterObj> {
    let item_size = 8 + inner.item_size;
    Box::new(IterObj { state: State::Enumerate { inner, n: 0 }, item_size })
}

pub(crate) fn zip(a: Box<IterObj>, b: Box<IterObj>, strict: bool, a_ty: u32, b_ty: u32) -> Box<IterObj> {
    let item_size = a.item_size + b.item_size;
    Box::new(IterObj { state: State::Zip { a, b, strict, a_ty, b_ty }, item_size })
}

/// The items of an `Iterable[T]` argument (`p`, of type `arg`): a
/// borrowed or owned list, or an iterator. When `T` (`item`) is not a
/// reference, or not given, the items of a borrowed collection are copies.
///
/// # Safety
/// `p` must hold a value of type `arg`; an owned one is moved in.
pub(crate) unsafe fn iterable(p: *mut u8, arg: u32, item: Option<u32>) -> Box<IterObj> {
    // SAFETY: as the caller guarantees.
    let (base, elem) = unsafe {
        match ty(arg).kind {
            Kind::Ref(inner) => match ty(inner).kind {
                Kind::List(e) => (list_ref(mem::word(p, 0) as *mut List, e), Some(e)),
                Kind::Dict(..) => {
                    (table_ref(crate::dict::table(mem::word(p, 0) as *mut u8), Part::Items), None)
                }
                Kind::Set(e) => {
                    (table_ref(crate::dict::table(mem::word(p, 0) as *mut u8), Part::Keys), Some(e))
                }
                Kind::Iter(_) => {
                    // An iterator behind a reference: its own object, which
                    // the reference does not own; take its remaining items.
                    let obj = mem::word(mem::word(p, 0) as *const u8, 0) as *mut IterObj;
                    let st = std::mem::replace(&mut (*obj).state, State::Done);
                    (Box::new(IterObj { state: st, item_size: (*obj).item_size }), None)
                }
                _ => rt::internal("iterating over a reference to something that is not a list"),
            },
            Kind::List(e) => (items((p as *mut List).read(), e), None),
            Kind::Dict(..) | Kind::Set(_) => (table_own(Box::from_raw(mem::word(p, 0) as *mut Table)), None),
            Kind::Iter(_) => (Box::from_raw(mem::word(p, 0) as *mut IterObj), None),
            _ => rt::internal("iterating over something that is not a list or an iterator"),
        }
    };
    let yields_refs = matches!(base.state, State::ListRef { .. })
        || matches!(base.state, State::TableRef { part, .. } if part != Part::Items);
    let wants_ref = item.is_some_and(|t| matches!(ty(t).kind, Kind::Ref(_)));
    if yields_refs && !wants_ref {
        let size = match (item, elem) {
            (Some(t), _) => ty(t).size,
            (None, Some(e)) => ty(e).size,
            (None, None) => 8,
        };
        Box::new(IterObj { state: State::Decay { inner: base }, item_size: size })
    } else {
        base
    }
}

/// Writes the next item as an option at `out` (a tag word, then the item).
///
/// # Safety
/// `out` must have room for a tag word and an item.
pub(crate) unsafe fn next(obj: &mut IterObj, out: *mut u8) -> u8 {
    // SAFETY: each state yields items of `item_size` bytes.
    unsafe {
        let some = |out: *mut u8| mem::set_word(out, 0, 0);
        let none = |out: *mut u8| mem::set_word(out, 0, 1);
        match &mut obj.state {
            State::Range { cur, stop, step } => {
                let more = if *step > 0 { *cur < *stop } else { *cur > *stop };
                if !more {
                    none(out);
                    return abi::OK;
                }
                let v = *cur;
                match cur.checked_add(*step) {
                    Some(n) => *cur = n,
                    None => obj.state = State::Done,
                }
                some(out);
                mem::set_word(out, 8, v as u64);
            }
            State::ListRef { list, idx, elem_size } => {
                let l = &**list;
                if *idx >= l.len {
                    none(out);
                    return abi::OK;
                }
                some(out);
                mem::set_word(out, 8, l.at(*idx, *elem_size) as u64);
                *idx += 1;
            }
            State::Items { list, idx, elem } => {
                if *idx >= list.len {
                    none(out);
                    return abi::OK;
                }
                let size = ty(*elem).size;
                some(out);
                mem::copy(out.add(8), list.at(*idx, size), size as usize);
                *idx += 1;
            }
            State::Chars { text, pos } => {
                let b = text.bytes();
                let rest = b.get(*pos as usize..).unwrap_or(&[]);
                match std::str::from_utf8(rest)
                    .ok()
                    .and_then(|s| s.chars().next())
                    .or_else(|| first_char(rest))
                {
                    Some(c) => {
                        *pos += c.len_utf8() as u64;
                        some(out);
                        mem::set_word(out, 8, u64::from(c as u32));
                    }
                    None => none(out),
                }
            }
            State::Bytes { text, pos } => {
                let b = text.bytes();
                match b.get(*pos as usize) {
                    Some(x) => {
                        *pos += 1;
                        some(out);
                        mem::set_word(out, 8, u64::from(*x));
                    }
                    None => none(out),
                }
            }
            State::Enumerate { inner, n } => {
                let mut tmp = vec![0u64; 1 + (inner.item_size as usize).div_ceil(8)];
                let t = tmp.as_mut_ptr() as *mut u8;
                let st = next(inner, t);
                if st != abi::OK {
                    return st;
                }
                if mem::word(t, 0) != 0 {
                    none(out);
                    return abi::OK;
                }
                some(out);
                mem::set_word(out, 8, *n as u64);
                mem::copy(out.add(16), t.add(8), inner.item_size as usize);
                *n += 1;
            }
            State::Zip { a, b, strict, a_ty, b_ty } => {
                let mut ta = vec![0u64; 1 + (a.item_size as usize).div_ceil(8)];
                let mut tb = vec![0u64; 1 + (b.item_size as usize).div_ceil(8)];
                let (pa, pb) = (ta.as_mut_ptr() as *mut u8, tb.as_mut_ptr() as *mut u8);
                let st = next(a, pa);
                if st != abi::OK {
                    return st;
                }
                let st = next(b, pb);
                if st != abi::OK {
                    return st;
                }
                match (mem::word(pa, 0) == 0, mem::word(pb, 0) == 0) {
                    (true, true) => {
                        some(out);
                        mem::copy(out.add(8), pa.add(8), a.item_size as usize);
                        mem::copy(out.add(8 + a.item_size as usize), pb.add(8), b.item_size as usize);
                    }
                    (false, false) => none(out),
                    (x, y) => {
                        // The item of the longer one is dropped.
                        let mut status = abi::OK;
                        if x {
                            status = value::drop_value(pa.add(8), *a_ty);
                        }
                        if y {
                            status = value::after_part(status, value::drop_value(pb.add(8), *b_ty));
                        }
                        if status != abi::OK {
                            return status;
                        }
                        if *strict {
                            return rt::raise_msg(msg::PANIC_ZIP_LENGTHS, &[]);
                        }
                        none(out);
                    }
                }
            }
            State::Decay { inner } => {
                let mut tmp = [0u64; 2];
                let t = tmp.as_mut_ptr() as *mut u8;
                let st = next(inner, t);
                if st != abi::OK {
                    return st;
                }
                if mem::word(t, 0) != 0 {
                    none(out);
                    return abi::OK;
                }
                some(out);
                let src = mem::word(t, 8) as *const u8;
                mem::copy(out.add(8), src, obj.item_size as usize);
            }
            State::TableRef { table, idx, part } => {
                let t = &**table;
                if *idx >= t.len() {
                    none(out);
                    return abi::OK;
                }
                some(out);
                match part {
                    Part::Keys => mem::set_word(out, 8, t.key_at(*idx) as u64),
                    Part::Values => mem::set_word(out, 8, t.val_at(*idx) as u64),
                    Part::Items => {
                        mem::set_word(out, 8, t.key_at(*idx) as u64);
                        mem::set_word(out, 16, t.val_at(*idx) as u64);
                    }
                }
                *idx += 1;
            }
            State::TableOwn { table, idx } => {
                if *idx >= table.len() {
                    none(out);
                    return abi::OK;
                }
                some(out);
                mem::copy(out.add(8), table.key_at(*idx), (table.ksize + table.vsize) as usize);
                *idx += 1;
            }
            State::Done => none(out),
        }
        abi::OK
    }
}

/// The first character of bytes that may not be valid UTF-8 at their end.
fn first_char(b: &[u8]) -> Option<char> {
    let n = match b.first()? {
        x if *x < 0x80 => 1,
        x if *x >= 0xF0 => 4,
        x if *x >= 0xE0 => 3,
        _ => 2,
    };
    std::str::from_utf8(b.get(..n)?).ok()?.chars().next()
}

/// Drops an iterator and what it owns.
///
/// # Safety
/// `obj` must be an iterator object made here, or null.
pub(crate) unsafe fn drop_iter(obj: *mut u8) -> u8 {
    if obj.is_null() {
        return abi::OK;
    }
    // SAFETY: the object was made by `into_word`.
    let obj = unsafe { Box::from_raw(obj as *mut IterObj) };
    // SAFETY: the object owns its remaining items.
    unsafe { drop_state(obj.state) }
}

/// # Safety
/// The state's owned items must be valid.
unsafe fn drop_state(st: State) -> u8 {
    // SAFETY: as the caller guarantees.
    unsafe {
        match st {
            State::Items { list, idx, elem } => {
                let mut status = abi::OK;
                if rt::needs_drop(elem) {
                    let size = ty(elem).size;
                    for i in idx..list.len {
                        status = value::after_part(status, value::drop_value(list.at(i, size), elem));
                    }
                }
                mem::release(list.ptr);
                status
            }
            State::TableOwn { mut table, idx } => {
                let mut status = abi::OK;
                let (kd, vd) = (rt::needs_drop(table.key), rt::needs_drop(table.val));
                for i in idx..table.len() {
                    if vd {
                        status = value::after_part(status, value::drop_value(table.val_at(i), table.val));
                    }
                    if kd {
                        status = value::after_part(status, value::drop_value(table.key_at(i), table.key));
                    }
                }
                // The entries are gone: the table's own drop frees only its
                // buffer.
                table.entries.len = 0;
                value::after_part(status, crate::dict::drop_table(Box::into_raw(table)))
            }
            State::Enumerate { inner, .. } | State::Decay { inner } => drop_state(inner.state),
            State::Zip { a, b, .. } => {
                let s = drop_state(a.state);
                value::after_part(s, drop_state(b.state))
            }
            _ => abi::OK,
        }
    }
}

/// Every remaining item, in a new buffer of `item_size`-byte items.
///
/// # Safety
/// As [`next`].
pub(crate) unsafe fn collect(obj: &mut IterObj) -> Result<List, u8> {
    let size = obj.item_size;
    let mut out = List::empty();
    let mut tmp = vec![0u64; 1 + (size as usize).div_ceil(8)];
    let t = tmp.as_mut_ptr() as *mut u8;
    loop {
        // SAFETY: `tmp` has room for a tag and an item.
        unsafe {
            let st = next(obj, t);
            if st != abi::OK {
                return Err(st);
            }
            if mem::word(t, 0) != 0 {
                return Ok(out);
            }
            out.push(t.add(8), size);
        }
    }
}
