//! Iterators (`Iter[T]`): a word holding the address of an object that
//! yields items of a fixed size, as the interpreter's `IterState` does
//! (`sisu-interp/src/iter.rs`). The runtime in Sisu
//! (`compiler/rt/src/iter.sisu`, decisions 423 and 542) lays the object
//! out, makes it, gives its next item, makes the iterator of an
//! `Iterable[T]` argument, collects what it has left and drops it; here
//! are the calls of those exports, the object opaque. Compiled code reads
//! its header (`abi::ITER_KIND`, decision 253): a range and a borrowed
//! list's elements keep their state there, so that a loop over one steps
//! without calling the runtime.

use crate::abi;
use crate::bridge::{self, NewIter};
use crate::dict::Table;
use crate::mem::{List, Str};
use crate::rt::ty;

/// An iterator's object, which only `iter.sisu` reads and writes.
pub(crate) enum IterObj {}

fn made(kind: NewIter<'_>) -> *mut IterObj {
    // SAFETY: the callers give live values, as each constructor says.
    unsafe { bridge::iter_new(kind) }.cast::<IterObj>()
}

/// The iterators whose `Iter.next` may wait: a generator's (its task and
/// the size of an item) and a receiver's (its channel, whose buffer knows
/// the size of its values, decision 680).
pub(crate) enum Waits {
    Gen(u32, u32),
    Recv(u32),
}

/// Whether `Iter.next` on the iterator may wait.
///
/// # Safety
/// `obj` must be an iterator's object.
pub(crate) unsafe fn waits(obj: *mut IterObj) -> Option<Waits> {
    // SAFETY: as the caller guarantees.
    match unsafe { bridge::iter_waits(obj.cast()) } {
        (1, t, size) => Some(Waits::Gen(t as u32, size as u32)),
        (2, ch, _) => Some(Waits::Recv(ch as u32)),
        _ => None,
    }
}

/// The iterator of the values a receiver gets.
pub(crate) fn receiver(chan: u32, elem: u32) -> *mut IterObj {
    made(NewIter::Receiver(chan, ty(elem).size))
}

/// The iterator of a generator's items, of `item_size` bytes.
pub(crate) fn generator(task: u32, item_size: u32) -> *mut IterObj {
    made(NewIter::Generator(task, item_size))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Part {
    Keys = 0,
    Items = 2,
}

/// An iterator over a borrowed dict's keys, values or pairs, or a set's
/// elements (`Keys`).
pub(crate) fn table_ref(table: *mut Table, part: Part) -> *mut IterObj {
    made(NewIter::TableRef(table.cast(), part as u64))
}

/// The heap object of a new iterator, as the word that holds it.
pub(crate) fn into_word(obj: *mut IterObj) -> u64 {
    obj as u64
}

pub(crate) fn range(cur: i64, stop: i64, step: i64) -> *mut IterObj {
    made(NewIter::Range(cur, stop, step))
}

/// An iterator over the elements of a borrowed list: their addresses.
pub(crate) fn list_ref(list: *mut List, elem: u32) -> *mut IterObj {
    made(NewIter::ListRef(list.cast(), ty(elem).size))
}

/// An iterator that owns a list's elements.
pub(crate) fn items(mut list: List, elem: u32) -> *mut IterObj {
    made(NewIter::Items(&mut list, elem))
}

pub(crate) fn chars(text: Str) -> *mut IterObj {
    made(NewIter::Chars(text.ptr, text.len))
}

pub(crate) fn bytes(text: Str) -> *mut IterObj {
    made(NewIter::Bytes(text.ptr, text.len))
}

/// A copy of the bytes of a `bytes` in a block the iterator owns.
pub(crate) fn owned_bytes(data: Vec<u8>) -> *mut IterObj {
    made(NewIter::OwnedBytes(&data))
}

/// # Safety
/// `inner` must be an iterator's object, which the new one owns.
pub(crate) unsafe fn enumerate(inner: *mut IterObj) -> *mut IterObj {
    made(NewIter::Enumerate(inner.cast()))
}

/// # Safety
/// `a` and `b` must be iterators' objects, which the new one owns.
pub(crate) unsafe fn zip(
    a: *mut IterObj,
    b: *mut IterObj,
    strict: bool,
    a_ty: u32,
    b_ty: u32,
) -> *mut IterObj {
    made(NewIter::Zip(a.cast(), b.cast(), strict, a_ty, b_ty))
}

/// The items of an `Iterable[T]` argument (`p`, of type `arg`): a
/// borrowed or owned list, dict or set, a receiver, or an iterator. When
/// `T` (`item`) is not a reference, or not given, the items of a borrowed
/// collection are copies.
///
/// # Safety
/// `p` must hold a value of type `arg`; an owned one is moved in.
pub(crate) unsafe fn iterable(p: *mut u8, arg: u32, item: Option<u32>) -> *mut IterObj {
    // SAFETY: as the caller guarantees.
    unsafe { crate::bridge::iter_iterable(p, arg, item).cast::<IterObj>() }
}

/// Writes the next item as an option at `out` (a tag word, then the item).
///
/// # Safety
/// `obj` must be an iterator's object, and `out` have room for a tag word
/// and an item.
pub(crate) unsafe fn next(obj: *mut IterObj, out: *mut u8) -> u8 {
    // SAFETY: as the caller guarantees.
    unsafe { crate::bridge::iter_next(obj.cast(), out) }
}

/// Drops an iterator and what it owns.
///
/// # Safety
/// `obj` must be an iterator object made here, or null.
pub(crate) unsafe fn drop_iter(obj: *mut u8) -> u8 {
    // SAFETY: as the caller guarantees.
    unsafe { crate::bridge::iter_drop(obj) }
}

/// Every remaining item of an iterator an intrinsic owns (an `Iterable[T]`
/// argument), of type `item` when the call says, in a new buffer; the
/// iterator is dropped after, and when a panic stops it, with what it has
/// left, and so are the items taken by then (decision 235).
///
/// # Safety
/// As [`next`]; the items are of type `item`.
pub(crate) unsafe fn collect_all(obj: *mut IterObj, item: Option<u32>) -> Result<List, u8> {
    let mut out = List::empty();
    // SAFETY: as the caller guarantees; `out` has room for a list.
    let st = unsafe { crate::bridge::iter_collect(obj.cast(), item, (&raw mut out).cast()) };
    if st == abi::OK { Ok(out) } else { Err(st) }
}
