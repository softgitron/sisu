//! Iterators (`Iter[T]`): a word holding the address of an object of
//! eleven words that yields items of a fixed size, as the interpreter's
//! `IterState` does (`sisu-interp/src/iter.rs`). The runtime in Sisu
//! (`compiler/rt/src/iter.sisu`, decision 423) gives an iterator's next
//! item, makes the iterator of an `Iterable[T]` argument, collects what an
//! iterator has left and drops one; here is the object as both read it,
//! the iterators the runtime in Rust makes, and the calls of those exports.
//! The object starts with the header compiled code reads
//! (`abi::ITER_KIND`, decision 253): a range and a borrowed list's elements
//! keep their state there, so that a loop over one steps without calling
//! the runtime.

use crate::abi;
use crate::dict::Table;
use crate::mem::{self, List, Str};
use crate::rt::ty;

/// An iterator's object, as `iter.sisu` lays it out: the header compiled
/// code reads (`abi::ITER_RANGE`, whose `a`, `b` and `c` are the current
/// value, the end and the step; `abi::ITER_LIST_REF`, the list's address,
/// the next index and the size of an element; or `abi::ITER_OTHER`), the
/// state that gives the other kinds' items, the bytes of an item, and the
/// state's five words.
#[repr(C)]
pub(crate) struct IterObj {
    pub kind: u64,
    pub a: u64,
    pub b: u64,
    pub c: u64,
    state: u64,
    pub item_size: u64,
    p: [u64; 5],
}

/// The states, as `iter.sisu` numbers them.
const S_RANGE: u64 = 0;
const S_LIST_REF: u64 = 1;
const S_ITEMS: u64 = 2;
const S_CHARS: u64 = 3;
const S_BYTES: u64 = 4;
const S_OWNED_BYTES: u64 = 5;
const S_ENUMERATE: u64 = 6;
const S_ZIP: u64 = 7;
const S_TABLE_REF: u64 = 9;
const S_GEN: u64 = 11;
const S_RECV: u64 = 12;

/// A new object of an iterator, in the runtime's own block.
fn make(kind: u64, header: [u64; 3], state: u64, item_size: u64, p: [u64; 5]) -> *mut IterObj {
    let obj = mem::alloc(std::mem::size_of::<IterObj>()).cast::<IterObj>();
    let [a, b, c] = header;
    // SAFETY: the block has room for the object.
    unsafe { obj.write(IterObj { kind, a, b, c, state, item_size, p }) };
    obj
}

/// The iterators whose `Iter.next` may wait: a generator's (its task and
/// the size of an item) and a receiver's (its channel and the size of a
/// value).
pub(crate) enum Waits {
    Gen(u32, u32),
    Recv(u32, u32),
}

/// Whether `Iter.next` on the iterator may wait.
pub(crate) fn waits(obj: &IterObj) -> Option<Waits> {
    match obj.state {
        S_GEN => Some(Waits::Gen(obj.p[0] as u32, obj.item_size as u32)),
        S_RECV => Some(Waits::Recv(obj.p[0] as u32, obj.item_size as u32)),
        _ => None,
    }
}

/// The iterator of the values a receiver gets.
pub(crate) fn receiver(chan: u32, elem: u32) -> *mut IterObj {
    make(abi::ITER_OTHER, [0; 3], S_RECV, u64::from(ty(elem).size), [u64::from(chan), 0, 0, 0, 0])
}

/// The iterator of a generator's items, of `item_size` bytes.
pub(crate) fn generator(task: u32, item_size: u32) -> *mut IterObj {
    make(abi::ITER_OTHER, [0; 3], S_GEN, u64::from(item_size), [u64::from(task), 0, 0, 0, 0])
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Part {
    Keys = 0,
    Values = 1,
    Items = 2,
}

/// An iterator over a borrowed dict's keys, values or pairs, or a set's
/// elements (`Keys`).
pub(crate) fn table_ref(table: *mut Table, part: Part) -> *mut IterObj {
    let item_size = if part == Part::Items { 16 } else { 8 };
    make(abi::ITER_OTHER, [0; 3], S_TABLE_REF, item_size, [table as u64, 0, part as u64, 0, 0])
}

/// The heap object of a new iterator, as the word that holds it.
pub(crate) fn into_word(obj: *mut IterObj) -> u64 {
    obj as u64
}

pub(crate) fn range(cur: i64, stop: i64, step: i64) -> *mut IterObj {
    make(abi::ITER_RANGE, [cur as u64, stop as u64, step as u64], S_RANGE, 8, [0; 5])
}

/// An iterator over the elements of a borrowed list: their addresses.
pub(crate) fn list_ref(list: *mut List, elem: u32) -> *mut IterObj {
    make(abi::ITER_LIST_REF, [list as u64, 0, u64::from(ty(elem).size)], S_LIST_REF, 8, [0; 5])
}

/// An iterator that owns a list's elements.
pub(crate) fn items(list: List, elem: u32) -> *mut IterObj {
    let size = u64::from(ty(elem).size);
    make(abi::ITER_OTHER, [0; 3], S_ITEMS, size, [list.ptr as u64, list.len, list.cap, 0, u64::from(elem)])
}

pub(crate) fn chars(text: Str) -> *mut IterObj {
    make(abi::ITER_OTHER, [0; 3], S_CHARS, 8, [text.ptr as u64, text.len, 0, 0, 0])
}

pub(crate) fn bytes(text: Str) -> *mut IterObj {
    make(abi::ITER_OTHER, [0; 3], S_BYTES, 8, [text.ptr as u64, text.len, 0, 0, 0])
}

/// A copy of the bytes of a `bytes` in a block the iterator owns.
pub(crate) fn owned_bytes(data: Vec<u8>) -> *mut IterObj {
    let block = mem::alloc(data.len());
    // SAFETY: the block has room for the bytes.
    unsafe { mem::copy(block, data.as_ptr(), data.len()) };
    make(abi::ITER_OTHER, [0; 3], S_OWNED_BYTES, 8, [block as u64, data.len() as u64, 0, 0, 0])
}

/// # Safety
/// `inner` must be an iterator's object, which the new one owns.
pub(crate) unsafe fn enumerate(inner: *mut IterObj) -> *mut IterObj {
    // SAFETY: as the caller guarantees.
    let item_size = 8 + unsafe { (*inner).item_size };
    make(abi::ITER_OTHER, [0; 3], S_ENUMERATE, item_size, [inner as u64, 0, 0, 0, 0])
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
    // SAFETY: as the caller guarantees.
    let item_size = unsafe { (*a).item_size + (*b).item_size };
    let p = [a as u64, b as u64, u64::from(strict), u64::from(a_ty), u64::from(b_ty)];
    make(abi::ITER_OTHER, [0; 3], S_ZIP, item_size, p)
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
