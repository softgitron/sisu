//! Iterators (`Iter[T]`): a word holding the address of an object that
//! yields items of a fixed size, as the interpreter's `IterState` does
//! (`sisu-interp/src/iter.rs`). The runtime in Sisu
//! (`compiler/rt/src/iter.sisu`, decisions 423 and 542) lays the object
//! out, makes it, gives its next item, makes the iterator of an
//! `Iterable[T]` argument, collects what it has left and drops it, and
//! does the intrinsics that make one or take its items (decision 820);
//! here are the calls of the exports the runtime in Rust still makes, the
//! object opaque. Compiled code reads
//! its header (`abi::ITER_KIND`, decision 253): a range and a borrowed
//! list's elements keep their state there, so that a loop over one steps
//! without calling the runtime.

use crate::bridge::{self, NewIter};
use crate::mem::{List, Str};

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

/// The iterator of a generator's items, of `item_size` bytes.
pub(crate) fn generator(task: u32, item_size: u32) -> *mut IterObj {
    made(NewIter::Generator(task, item_size))
}

/// The heap object of a new iterator, as the word that holds it.
pub(crate) fn into_word(obj: *mut IterObj) -> u64 {
    obj as u64
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

/// Writes the next item as an option at `out` (a tag word, then the item).
///
/// # Safety
/// `obj` must be an iterator's object, and `out` have room for a tag word
/// and an item.
pub(crate) unsafe fn next(obj: *mut IterObj, out: *mut u8) -> u8 {
    // SAFETY: as the caller guarantees.
    unsafe { crate::bridge::iter_next(obj.cast(), out) }
}
