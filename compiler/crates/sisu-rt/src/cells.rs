//! The library types of 04 that the interpreter implements as values of
//! its own (decision 207; `sisu-interp/src/intrinsics.rs`, `shared_method`,
//! `cell_method`, `arena_method`; `drops.rs`), in the native layout
//! ([`Kind::Shared`], [`Kind::Weak`], [`Kind::Cell`], [`Kind::Arena`]), with
//! the interpreter's results, messages and order of drops.
//!
//! **`Shared` and `Weak`** are the address of the value in a heap cell
//! whose header counts the owners and the weak handles: the value is
//! dropped with its last owner, and the cell freed with the last handle
//! of either kind (the interpreter keeps every cell; port-debt.md).
//! **`Cell`** is its value. **`Arena`** is a runtime object holding slots,
//! each a generation, whether it holds a value, and the value; a key is
//! the slot and the generation it was inserted under.

use crate::abi::{self, Kind};
use crate::intrinsics::Call;
use crate::mem;
use crate::rt::{self, ty};
use crate::value;
use sisu_diag::msg;

// -------------------------------------------------------------- Shared

/// The address of a `Shared` cell's header, from its value's.
fn header(v: *mut u8) -> *mut u8 {
    // SAFETY: the header is just before the value in the same allocation.
    unsafe { v.sub(abi::SHARED_HEADER as usize) }
}

/// A new cell holding the `size` bytes at `src`, moved in: the address of
/// its value.
///
/// # Safety
/// `src` must hold `size` bytes.
unsafe fn new_shared(src: *const u8, size: u32, atomic: bool) -> *mut u8 {
    let cell = mem::alloc(abi::SHARED_HEADER as usize + (size as usize).max(8));
    let cur = crate::sched::with(|s| {
        if atomic {
            s.stats.shared_crossing += 1;
        } else {
            s.stats.shared_local += 1;
        }
        s.cur
    });
    // SAFETY: the cell has room for the header and the value.
    unsafe {
        mem::set_word(cell, abi::SHARED_STRONG, 1);
        mem::set_word(cell, abi::SHARED_WEAK, 0);
        mem::set_word(cell, abi::SHARED_CREATOR, u64::from(cur));
        mem::set_word(cell, abi::SHARED_FLAGS, if atomic { abi::SHARED_ATOMIC } else { 0 });
        let v = cell.add(abi::SHARED_HEADER as usize);
        mem::copy(v, src, size as usize);
        v
    }
}

/// A task reached a `Shared`'s cell: one another task than its maker
/// reaches counts once (`--stats`).
///
/// # Safety
/// `v` must be the value address of a live cell.
pub(crate) unsafe fn touch(v: *mut u8) {
    let h = header(v);
    // SAFETY: the header's words.
    unsafe {
        let flags = mem::word(h, abi::SHARED_FLAGS);
        if flags & abi::SHARED_CROSSED != 0 {
            return;
        }
        let creator = mem::word(h, abi::SHARED_CREATOR);
        let crossed = crate::sched::with(|s| {
            if u64::from(s.cur) == creator {
                return false;
            }
            s.stats.shared_crossed += 1;
            true
        });
        if crossed {
            mem::set_word(h, abi::SHARED_FLAGS, flags | abi::SHARED_CROSSED);
        }
    }
}

/// Code reached a `Shared`'s value ([`abi::sym::SHARED_TOUCH`]).
///
/// # Safety
/// `v` must be the value address of a live cell.
#[unsafe(no_mangle)]
unsafe extern "C" fn sisu_rt_shared_touch(v: *mut u8) {
    // SAFETY: as the caller guarantees.
    crate::sched::system(|| unsafe { touch(v) })
}

/// Drops an owner of a `Shared` whose value is of type `t`: the last one
/// drops the value, and the last handle frees the cell.
///
/// # Safety
/// `v` must be the value address of a live cell with an owner.
pub(crate) unsafe fn drop_shared(v: *mut u8, t: u32) -> u8 {
    let h = header(v);
    // SAFETY: the header's words, and the value once.
    unsafe {
        touch(v);
        let strong = mem::word(h, abi::SHARED_STRONG).saturating_sub(1);
        mem::set_word(h, abi::SHARED_STRONG, strong);
        if strong > 0 {
            return abi::OK;
        }
        let st = value::drop_value(v, t);
        if mem::word(h, abi::SHARED_WEAK) == 0 {
            mem::release(h);
        }
        st
    }
}

/// Drops a `Weak` handle: the last handle of a cell whose value is gone
/// frees it.
///
/// # Safety
/// `v` must be the value address of a cell with a weak handle.
pub(crate) unsafe fn drop_weak(v: *mut u8) -> u8 {
    let h = header(v);
    // SAFETY: the header's words.
    unsafe {
        let weak = mem::word(h, abi::SHARED_WEAK).saturating_sub(1);
        mem::set_word(h, abi::SHARED_WEAK, weak);
        if weak == 0 && mem::word(h, abi::SHARED_STRONG) == 0 {
            mem::release(h);
        }
    }
    abi::OK
}

/// `.copy()` of a `Shared` (another owner) or a `Weak` (another handle).
///
/// # Safety
/// `v` must be the value address of a live cell.
pub(crate) unsafe fn copy_handle(v: *mut u8, weak: bool) {
    let h = header(v);
    // SAFETY: the header's words.
    unsafe {
        if !weak {
            touch(v);
        }
        let w = if weak { abi::SHARED_WEAK } else { abi::SHARED_STRONG };
        mem::set_word(h, w, mem::word(h, w) + 1);
    }
}

/// Whether the cell still holds its value.
///
/// # Safety
/// `v` must be the value address of a cell.
pub(crate) unsafe fn alive(v: *mut u8) -> bool {
    // SAFETY: the header's words.
    unsafe { mem::word(header(v), abi::SHARED_STRONG) > 0 }
}

// --------------------------------------------------------------- Arena

/// An arena's slots: each a generation, whether it holds a value, then the
/// value.
pub(crate) struct ArenaObj {
    pub elem: u32,
    /// Words of a slot.
    words: usize,
    buf: Vec<u64>,
    slots: usize,
    free: Vec<usize>,
    _live: mem::Live,
}

impl ArenaObj {
    fn new(elem: u32) -> ArenaObj {
        ArenaObj {
            elem,
            words: 2 + (ty(elem).size as usize).div_ceil(8),
            buf: Vec::new(),
            slots: 0,
            free: Vec::new(),
            _live: mem::Live::new(),
        }
    }

    fn slot(&mut self, i: usize) -> *mut u64 {
        // SAFETY: the callers index slots below `self.slots`.
        unsafe { self.buf.as_mut_ptr().add(i * self.words) }
    }

    /// The value of slot `i`, if it holds one of `generation`.
    fn live(&mut self, i: usize, generation: i64) -> Option<*mut u8> {
        if i >= self.slots {
            return None;
        }
        let s = self.slot(i);
        // SAFETY: a slot's first words are its generation and whether it
        // holds a value.
        unsafe { (*s == generation as u64 && *s.add(1) != 0).then(|| s.add(2) as *mut u8) }
    }

    /// The values held, in slot order, with their keys.
    fn held(&mut self) -> Vec<(usize, i64, *mut u8)> {
        (0..self.slots)
            .filter_map(|i| {
                let s = self.slot(i);
                // SAFETY: as `live`.
                unsafe { (*s.add(1) != 0).then(|| (i, *s as i64, s.add(2) as *mut u8)) }
            })
            .collect()
    }
}

/// The arena object an `Arena` value at `p` holds.
///
/// # Safety
/// `p` must hold an `Arena`.
pub(crate) unsafe fn arena<'a>(p: *mut u8) -> &'a mut ArenaObj {
    // SAFETY: an arena is a word holding its object, made by `Arena.new`.
    unsafe { &mut *(mem::word(p, 0) as *mut ArenaObj) }
}

/// Writes a `Key` (the slot and the generation) at `out`.
///
/// # Safety
/// `out` must have room for two words.
unsafe fn write_key(out: *mut u8, i: usize, generation: i64) {
    // SAFETY: as the caller guarantees.
    unsafe {
        mem::set_word(out, 0, i as u64);
        mem::set_word(out, 8, generation as u64);
    }
}

/// The slot and generation a `Key` at `p` holds.
///
/// # Safety
/// `p` must hold a `Key`.
unsafe fn read_key(p: *const u8) -> (usize, i64) {
    // SAFETY: a key is its slot and its generation.
    unsafe { (mem::word(p, 0) as usize, mem::word(p, 8) as i64) }
}

/// The address of the value an arena holds for a key (`a{k}`, at `out`):
/// a stale key panics, or, for a drop, gives null.
///
/// # Safety
/// `a` must hold an `Arena`, `key` a `Key`, and `out` be a word.
#[unsafe(no_mangle)]
unsafe extern "C" fn sisu_rt_arena_slot(a: *mut u8, key: *mut u8, mode: u32, out: *mut u8) -> u8 {
    crate::sched::system(|| {
        // SAFETY: as the caller guarantees.
        unsafe {
            let (i, generation) = read_key(key);
            match arena(a).live(i, generation) {
                Some(v) => mem::set_word(out, 0, v as u64),
                None if mode == abi::SLOT_DROP => mem::set_word(out, 0, 0),
                None => {
                    return rt::raise_msg(
                        msg::PANIC_STALE_KEY,
                        &[("index", i.to_string()), ("generation", generation.to_string())],
                    );
                }
            }
        }
        abi::OK
    })
}

/// Drops an arena: its values first to last, then its slots.
///
/// # Safety
/// `p` must hold an `Arena`, dead afterwards.
pub(crate) unsafe fn drop_arena(p: *mut u8) -> u8 {
    // SAFETY: the object was made by `Arena.new`, and is dropped once.
    unsafe {
        let obj = Box::from_raw(mem::word(p, 0) as *mut ArenaObj);
        let mut obj = obj;
        let elem = obj.elem;
        let mut status = abi::OK;
        if rt::needs_drop(elem) {
            for (_, _, v) in obj.held() {
                status = value::after_part(status, value::drop_value(v, elem));
            }
        }
        status
    }
}

/// `.copy()` of an arena: the same keys, each value copied.
///
/// # Safety
/// `src` must hold an `Arena` and `dst` have room for one.
pub(crate) unsafe fn copy_arena(dst: *mut u8, src: *mut u8) {
    // SAFETY: as the caller guarantees.
    unsafe {
        let from = arena(src);
        let mut to = ArenaObj::new(from.elem);
        to.buf = vec![0u64; from.buf.len()];
        to.slots = from.slots;
        to.free = from.free.clone();
        for i in 0..from.slots {
            let (s, d) = (from.slot(i), to.slot(i));
            *d = *s;
            *d.add(1) = *s.add(1);
            if *s.add(1) != 0 {
                value::copy_value(d.add(2) as *mut u8, s.add(2) as *mut u8, from.elem);
            }
        }
        mem::set_word(dst, 0, Box::into_raw(Box::new(to)) as u64);
    }
}

/// The `Debug` text of an arena: its values in slot order.
///
/// # Safety
/// `p` must hold an `Arena`.
pub(crate) unsafe fn debug_arena(out: &mut Vec<u8>, p: *mut u8) {
    // SAFETY: as the caller guarantees.
    unsafe {
        let a = arena(p);
        let elem = a.elem;
        out.extend_from_slice(b"Arena([");
        for (k, (_, _, v)) in a.held().into_iter().enumerate() {
            if k > 0 {
                out.extend_from_slice(b", ");
            }
            value::debug(out, v, elem);
        }
        out.extend_from_slice(b"])");
    }
}

/// The values an arena holds.
///
/// # Safety
/// `p` must hold an `Arena`.
pub(crate) unsafe fn arena_len(p: *mut u8) -> u64 {
    // SAFETY: as the caller guarantees.
    unsafe { arena(p).held().len() as u64 }
}

// ---------------------------------------------------------- intrinsics

/// The intrinsics of `Shared`, `Weak`, `Cell` and `Arena`, on the system
/// stack; `None` for any other.
///
/// # Safety
/// As `sisu_rt_intrinsic`.
pub(crate) unsafe fn call(i: abi::Intrinsic, c: &Call<'_>) -> Option<u8> {
    use abi::Intrinsic as I;
    // SAFETY: each reads its arguments as their types say.
    unsafe {
        Some(match i {
            I::SharedNew | I::SharedNewAtomic => {
                let (p, t) = (c.arg(0), c.ty(0));
                let v = new_shared(p, ty(t).size, i == I::SharedNewAtomic);
                c.set_word(v as u64);
                abi::OK
            }
            I::SharedGet => {
                let v = c.word(0) as *mut u8;
                touch(v);
                c.set_word(v as u64);
                abi::OK
            }
            I::SharedDowngrade => {
                let v = c.word(0) as *mut u8;
                touch(v);
                copy_handle(v, true);
                c.set_word(v as u64);
                abi::OK
            }
            I::SharedCount => {
                let v = c.word(0) as *mut u8;
                touch(v);
                c.set_word(mem::word(header(v), abi::SHARED_STRONG));
                abi::OK
            }
            I::WeakUpgrade => {
                let v = c.word(0) as *mut u8;
                touch(v);
                if alive(v) {
                    copy_handle(v, false);
                    mem::set_word(c.ret, 0, 0);
                    mem::set_word(c.ret, 8, v as u64);
                } else {
                    mem::set_word(c.ret, 0, 1);
                }
                abi::OK
            }
            I::CellNew => {
                mem::copy(c.ret, c.arg(0), ty(c.ty(0)).size as usize);
                abi::OK
            }
            I::CellGet => {
                let (p, t) = c.place(0);
                let Kind::Cell(v) = ty(t).kind else { rt::internal("`Cell.get` of something not a cell") };
                mem::copy(c.ret, p, ty(v).size as usize);
                // A copy of what the cell holds: its handles count one more
                // (decision 237).
                crate::value::retain_value(c.ret, v);
                abi::OK
            }
            I::CellSet | I::CellReplace => {
                let (p, t) = c.place(0);
                let Kind::Cell(v) = ty(t).kind else {
                    rt::internal("a `Cell` method on something not a cell")
                };
                let size = ty(v).size as usize;
                if i == I::CellReplace {
                    mem::copy(c.ret, p, size);
                    mem::copy(p, c.arg(1), size);
                    abi::OK
                } else {
                    // The old value is dropped after the new one is in.
                    let mut old = vec![0u64; size.div_ceil(8)];
                    mem::copy(old.as_mut_ptr() as *mut u8, p, size);
                    mem::copy(p, c.arg(1), size);
                    value::drop_value(old.as_mut_ptr() as *mut u8, v)
                }
            }
            I::ArenaNew => {
                let Kind::Arena(elem) = ty(c.ret_ty).kind else {
                    rt::internal("an arena of an unknown type")
                };
                c.set_word(Box::into_raw(Box::new(ArenaObj::new(elem))) as u64);
                abi::OK
            }
            I::ArenaInsert => {
                let (p, _) = c.place(0);
                let a = arena(p);
                let size = ty(a.elem).size as usize;
                let i = match a.free.pop() {
                    Some(i) => i,
                    None => {
                        a.buf.resize(a.buf.len() + a.words, 0);
                        a.slots += 1;
                        a.slots - 1
                    }
                };
                let s = a.slot(i);
                *s.add(1) = 1;
                mem::copy(s.add(2) as *mut u8, c.arg(1), size);
                write_key(c.ret, i, *s as i64);
                abi::OK
            }
            I::ArenaGet | I::ArenaGetMut | I::ArenaContains => {
                let (p, _) = c.place(0);
                let (k, _) = c.place(1);
                let (idx, generation) = read_key(k);
                let v = arena(p).live(idx, generation);
                if i == I::ArenaContains {
                    c.set_bool(v.is_some());
                } else {
                    match v {
                        Some(v) => {
                            mem::set_word(c.ret, 0, 0);
                            mem::set_word(c.ret, 8, v as u64);
                        }
                        None => c.none(),
                    }
                }
                abi::OK
            }
            I::ArenaRemove | I::ArenaTake => {
                let (p, _) = c.place(0);
                let (k, _) = c.place(1);
                let (idx, generation) = read_key(k);
                let a = arena(p);
                let elem = a.elem;
                let size = ty(elem).size as usize;
                let Some(v) = a.live(idx, generation) else {
                    if i == I::ArenaTake {
                        c.none();
                    }
                    return Some(abi::OK);
                };
                // The slot's next value gets the next generation.
                let s = a.slot(idx);
                *s += 1;
                *s.add(1) = 0;
                a.free.push(idx);
                if i == I::ArenaTake {
                    mem::set_word(c.ret, 0, 0);
                    mem::copy(c.ret.add(8), v, size);
                    abi::OK
                } else {
                    let mut old = vec![0u64; size.div_ceil(8)];
                    mem::copy(old.as_mut_ptr() as *mut u8, v, size);
                    value::drop_value(old.as_mut_ptr() as *mut u8, elem)
                }
            }
            I::ArenaKeys => {
                let (p, _) = c.place(0);
                let Kind::Iter(key_ty) = ty(c.ret_ty).kind else {
                    rt::internal("arena keys of an unknown type")
                };
                let keys = arena(p).held();
                let mut l = mem::List::empty();
                let mut k = [0u64; 2];
                for (idx, generation, _) in keys {
                    write_key(k.as_mut_ptr() as *mut u8, idx, generation);
                    l.push(k.as_ptr() as *const u8, 16);
                }
                c.set_word(crate::iter::into_word(crate::iter::items(l, key_ty)));
                abi::OK
            }
            _ => return None,
        })
    }
}
