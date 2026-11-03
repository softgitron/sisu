//! The structural operations on values in the native layout, by their
//! types: drop, `.copy()`, equality, order, and the `Display` and `Debug`
//! texts. Each follows the interpreter's rule for the same value
//! (`sisu-interp/src/drops.rs`, `ops.rs`, `format.rs`, and `copy_value`).

use crate::abi::{self, Kind, TypeEntry};
use crate::mem::{self, List, Str, Text};
use crate::rt::{self, ty};
use std::cmp::Ordering;

/// The value behind any number of references and `dyn`s, and its type.
///
/// # Safety
/// `p` must hold a value of type `t`.
pub(crate) unsafe fn target(mut p: *mut u8, mut t: u32) -> (*mut u8, u32) {
    loop {
        match ty(t).kind {
            // A reference to a `dyn` is the `dyn`'s two words.
            Kind::Ref(inner) if matches!(ty(inner).kind, Kind::Dyn) => t = inner,
            // SAFETY: a reference is a word holding the address of its
            // target, and a `dyn` the type and the address of its value.
            Kind::Ref(inner) => {
                p = unsafe { mem::word(p, 0) } as *mut u8;
                t = inner;
            }
            Kind::Dyn => unsafe {
                t = mem::word(p, 0) as u32;
                p = mem::word(p, 8) as *mut u8;
            },
            _ => return (p, t),
        }
    }
}

/// The text of a `str`, `String` or `Path`, through references; `None`
/// for any other value.
///
/// # Safety
/// `p` must hold a value of type `t`, which must outlive the slice.
pub(crate) unsafe fn text<'a>(p: *mut u8, t: u32) -> Option<&'a [u8]> {
    // SAFETY: as the caller guarantees.
    unsafe {
        let (p, t) = target(p, t);
        text_at(p, ty(t))
    }
}

/// The text of the `str`, `String` or `Path` at `p` itself, of the type
/// `entry` describes; `None` for any other value.
///
/// # Safety
/// `p` must hold a value of that type, which must outlive the slice.
unsafe fn text_at<'a>(p: *mut u8, entry: &TypeEntry) -> Option<&'a [u8]> {
    // SAFETY: as the caller guarantees.
    unsafe {
        match entry.kind {
            Kind::Str => Some(mem::str_at(p).bytes()),
            Kind::String | Kind::Path => Some(mem::at::<Text>(p).bytes()),
            _ => None,
        }
    }
}

/// The tag of the enum, option or result at `p`.
///
/// # Safety
/// `p` must hold a value of an enum type `t`.
pub(crate) unsafe fn tag(p: *const u8, t: &TypeEntry) -> u64 {
    // SAFETY: as the caller guarantees.
    unsafe { mem::word(p, t.kind.tag_offset()) }
}

/// The enum type an error value belongs to: an error set's value names
/// its declaration in its first word.
///
/// # Safety
/// `p` must hold a value of type `t`.
pub(crate) unsafe fn concrete(p: *const u8, t: u32) -> u32 {
    match ty(t).kind {
        // SAFETY: an error value's first word is its own type's number.
        Kind::ErrSet(_) | Kind::Enum { error: true, .. } => unsafe { mem::word(p, abi::ERROR_DECL) as u32 },
        _ => t,
    }
}

// --------------------------------------------------------------------- drop

/// Whether drops only free memory: no drop glue, no `Drop`, no check of a
/// guard's invariant ([`forget_value`]).
static FORGETTING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Whether the values being dropped are forgotten rather than dropped.
pub(crate) fn forgetting() -> bool {
    FORGETTING.load(std::sync::atomic::Ordering::Relaxed)
}

/// Runs `f` with drops forgetting: what they free is freed, and no Sisu
/// code runs (no drop glue, no `Drop`, no guard's check). It is how the
/// interpreter's host lets go of values no program can see again: the
/// error `main` returned, the objects of capabilities when the run ends,
/// and the frames of an abandoned generator (decisions 95 and 223).
pub(crate) fn forget<R>(f: impl FnOnce() -> R) -> R {
    let was = FORGETTING.swap(true, std::sync::atomic::Ordering::Relaxed);
    let r = f();
    FORGETTING.store(was, std::sync::atomic::Ordering::Relaxed);
    r
}

/// Frees what the value at `p` of type `t` owns, running no Sisu code.
///
/// # Safety
/// `p` must hold a value of type `t`, which is dead afterwards.
pub(crate) unsafe fn forget_value(p: *mut u8, t: u32) {
    // SAFETY: as the caller guarantees.
    forget(|| unsafe {
        let _ = drop_value(p, t);
    });
}

/// Drops the value at `p`, of type `t`, and every value it owns, in the
/// order the interpreter does: a list's elements first to last, a tuple's
/// last to first, a struct or enum by its drop glue. A part whose drop
/// panics does not stop the others, which are dropped as cleanup (a second
/// panic aborts); the status says whether a panic is leaving.
///
/// # Safety
/// `p` must hold a value of type `t`, which is dead afterwards.
pub(crate) unsafe fn drop_value(p: *mut u8, t: u32) -> u8 {
    if !rt::needs_drop(t) {
        return abi::OK;
    }
    let entry = ty(t);
    // SAFETY: the parts are where the type says, each dropped once.
    unsafe {
        // By the parts that need a drop, when the type has them listed
        // (decision 266).
        match rt::drop_parts(t) {
            rt::DropParts::Parts(parts) => return drop_parts(p, parts),
            rt::DropParts::Variants(variants) => {
                return match variants.get(tag(p, entry) as usize) {
                    Some(parts) => drop_parts(p, parts),
                    None => abi::OK,
                };
            }
            rt::DropParts::ByKind => {}
        }
        match &entry.kind {
            Kind::String | Kind::Path | Kind::Bytes => {
                mem::at::<Text>(p).release();
                abi::OK
            }
            Kind::List(e) => {
                let list = mem::at::<List>(p);
                let size = ty(*e).size;
                let mut status = abi::OK;
                if rt::needs_drop(*e) {
                    // The element's way of dropping looked at once.
                    match (rt::drop_parts(*e), &ty(*e).kind) {
                        (rt::DropParts::Parts(parts), _) => {
                            for i in 0..list.len {
                                status = after_part(status, drop_parts(list.at(i, size), parts));
                            }
                        }
                        (_, Kind::String | Kind::Path | Kind::Bytes) => {
                            for i in 0..list.len {
                                mem::at::<Text>(list.at(i, size)).release();
                            }
                        }
                        _ => {
                            for i in 0..list.len {
                                status = after_part(status, drop_value(list.at(i, size), *e));
                            }
                        }
                    }
                }
                mem::release(list.ptr);
                *list = List::empty();
                status
            }
            Kind::Iter(_) => crate::iter::drop_iter(mem::word(p, 0) as *mut u8),
            Kind::Dyn => {
                let cell = mem::word(p, 8) as *mut u8;
                let status = drop_value(cell, mem::word(p, 0) as u32);
                mem::release(cell);
                status
            }
            Kind::Dict(..) | Kind::Set(_) => {
                crate::dict::drop_table(mem::word(p, 0) as *mut crate::dict::Table)
            }
            Kind::Entry(k, _) => drop_value(p.add(8), *k),
            Kind::Option(payload) => {
                if tag(p, entry) == 0 {
                    drop_value(p.add(8), *payload)
                } else {
                    abi::OK
                }
            }
            Kind::Result(ok, err) => {
                if tag(p, entry) == 0 {
                    drop_value(p.add(8), *ok)
                } else {
                    drop_value(p.add(8), *err)
                }
            }
            Kind::Tuple(slots) => {
                let mut status = abi::OK;
                for s in slots.iter().rev() {
                    status = after_part(status, drop_value(p.add(s.offset as usize), s.ty));
                }
                status
            }
            // A box whose value was moved out has no cell (decision 216).
            Kind::Box(inner) => {
                let cell = mem::word(p, 0) as *mut u8;
                if cell.is_null() {
                    return abi::OK;
                }
                let status = drop_value(cell, *inner);
                mem::release(cell);
                status
            }
            // Glue is Sisu code: on the system stack, the value is moved
            // aside and dropped on the task's stack (`sched::defer`).
            Kind::Struct { glue: Some(_), .. } | Kind::Enum { glue: Some(_), .. } | Kind::Guard(..)
                if crate::sched::deferring() && !forgetting() =>
            {
                crate::sched::defer(p, t)
            }
            // The glue (Sisu code: the `Drop` impl, then the fields whose drop
            // is observed), then the fields it leaves out (decision 218); an
            // error lets go of its return trace (decision 215). Forgetting,
            // every field is freed and no glue runs.
            Kind::Struct { glue, fields, .. } => {
                if forgetting() {
                    return drop_fields(p, fields, false);
                }
                let status = run_glue(*glue, p);
                after_part(status, drop_silent(p, fields))
            }
            Kind::Enum { glue, variants, error, .. } => {
                if *error {
                    crate::intrinsics::release_trace(mem::word(p, abi::ERROR_TRACE));
                    mem::set_word(p, abi::ERROR_TRACE, 0);
                }
                let v = variants.get(tag(p, entry) as usize);
                if forgetting() {
                    return v.map_or(abi::OK, |v| drop_fields(p, &v.fields, false));
                }
                let status = run_glue(*glue, p);
                match v {
                    Some(v) => after_part(status, drop_silent(p, &v.fields)),
                    None => status,
                }
            }
            Kind::Guard(..) | Kind::ReadGuard(_) if forgetting() => abi::OK,
            Kind::Guard(_, check) => crate::tasks::unlock(mem::word(p, 0) as *mut u8, *check),
            Kind::ReadGuard(_) => crate::tasks::read_unlock(mem::word(p, 0) as *mut u8),
            Kind::Mutex(v) | Kind::RwLock(v) => drop_value(p.add(abi::MUTEX_HEADER as usize), *v),
            Kind::Sender(_) => crate::tasks::drop_channel_end(mem::word(p, 0) as u32, false),
            Kind::Receiver(_) => crate::tasks::drop_channel_end(mem::word(p, 0) as u32, true),
            Kind::Shared(v) => crate::cells::drop_shared(mem::word(p, 0) as *mut u8, *v),
            Kind::Weak(_) => crate::cells::drop_weak(mem::word(p, 0) as *mut u8),
            Kind::Cell(v) => drop_value(p, *v),
            Kind::Arena(_) => crate::cells::drop_arena(p),
            Kind::ErrSet(_) => {
                let decl = concrete(p, t);
                if decl == t { abi::OK } else { drop_value(p, decl) }
            }
            // A copy of a capability handle, or of a `Cap`: one count
            // fewer (decision 237).
            Kind::Opaque(n) if counted(n) => crate::caps::release_cell(mem::word(p, 0) as *mut u8),
            // A function value goes with what its environment holds
            // (decision 248).
            Kind::Fn => match fn_env(p, entry) {
                Some(env) => drop_value(p.add(8), env),
                None => abi::OK,
            },
            _ => abi::OK,
        }
    }
}

/// The type of what a function value's environment holds, when a drop or
/// a copy of the value must reach it: the value's last word, the type's
/// number plus one, or 0 (decision 248).
///
/// # Safety
/// `p` must hold a function value of the type `entry` describes.
unsafe fn fn_env(p: *mut u8, entry: &abi::TypeEntry) -> Option<u32> {
    // SAFETY: as the caller guarantees.
    let w = unsafe { mem::word(p, entry.size - 8) };
    (w != 0).then(|| (w - 1) as u32)
}

/// Whether a type the runtime knows by its name is a counted handle
/// ([`abi::counted_handle`]).
pub(crate) fn counted(name: &str) -> bool {
    abi::counted_handle(name)
}

/// One more count for every handle the value at `p` of type `t` holds: a
/// copy of it was made (decision 237).
///
/// # Safety
/// `p` must hold a value of type `t`.
pub(crate) unsafe fn retain_value(p: *mut u8, t: u32) {
    if !rt::needs_drop(t) {
        return;
    }
    let entry = ty(t);
    // SAFETY: the parts are where the type says.
    unsafe {
        match &entry.kind {
            Kind::Opaque(n) if counted(n) => crate::caps::retain_cell(mem::word(p, 0) as *mut u8),
            Kind::Option(payload) => {
                if tag(p, entry) == 0 {
                    retain_value(p.add(8), *payload);
                }
            }
            Kind::Result(ok, err) => {
                let part = if tag(p, entry) == 0 { *ok } else { *err };
                retain_value(p.add(8), part);
            }
            Kind::Tuple(slots) => {
                for s in slots {
                    retain_value(p.add(s.offset as usize), s.ty);
                }
            }
            Kind::Struct { fields, .. } => {
                for f in fields {
                    retain_value(p.add(f.offset as usize), f.ty);
                }
            }
            Kind::Enum { variants, .. } => {
                if let Some(v) = variants.get(tag(p, entry) as usize) {
                    for f in &v.fields {
                        retain_value(p.add(f.offset as usize), f.ty);
                    }
                }
            }
            Kind::Fn => {
                if let Some(env) = fn_env(p, entry) {
                    retain_value(p.add(8), env);
                }
            }
            _ => {}
        }
    }
}

/// Runs a struct's or enum's drop glue on the value at `p`, if it has one.
///
/// # Safety
/// `p` must hold a value of the type whose glue `glue` is.
unsafe fn run_glue(glue: Option<u32>, p: *mut u8) -> u8 {
    match glue.map(rt::function) {
        None => abi::OK,
        Some(Some(f)) => {
            let mut unit = 0u64;
            // SAFETY: the glue takes the value's address.
            unsafe { f(&mut unit as *mut u64 as *mut u8, p) }
        }
        Some(None) => rt::internal("drop glue missing from the function table"),
    }
}

/// Drops the parts of the value at `p` that need a drop, in order
/// ([`rt::DropParts`]).
///
/// # Safety
/// `p` must hold a value whose parts these are, dead afterwards.
unsafe fn drop_parts(p: *mut u8, parts: &[(u32, u32)]) -> u8 {
    let mut status = abi::OK;
    for &(off, t) in parts {
        // SAFETY: each part is at its offset.
        status = after_part(status, unsafe { drop_value(p.add(off as usize), t) });
    }
    status
}

/// Drops the fields at `p` that the type's glue leaves out, last to first
/// as the glue drops its own; none of them runs Sisu code.
///
/// # Safety
/// `p` must hold the fields at their offsets.
unsafe fn drop_silent(p: *mut u8, fields: &[abi::Field]) -> u8 {
    // SAFETY: as the caller guarantees.
    unsafe { drop_fields(p, fields, true) }
}

/// Drops the fields at `p`, last to first: those the glue leaves out
/// (`silent`), or every one.
///
/// # Safety
/// `p` must hold the fields at their offsets.
unsafe fn drop_fields(p: *mut u8, fields: &[abi::Field], silent: bool) -> u8 {
    let mut status = abi::OK;
    for f in fields.iter().rev().filter(|f| !(silent && f.glue) && rt::needs_drop(f.ty)) {
        // SAFETY: the field is where the type says.
        status = after_part(status, unsafe { drop_value(p.add(f.offset as usize), f.ty) });
    }
    status
}

/// The status after dropping one more part: once a part's drop has
/// panicked, the remaining parts are cleanup, and another panic aborts.
pub(crate) fn after_part(before: u8, now: u8) -> u8 {
    match (before, now) {
        (abi::OK, s) => s,
        (_, abi::OK) => before,
        _ => rt::abort_unwinding(),
    }
}

// --------------------------------------------------------------------- copy

/// Copies a part of a value: a word, or other bytes, when its copy is a
/// copy of its bytes, without a call of [`copy_value`] for it.
///
/// # Safety
/// As [`copy_value`].
unsafe fn copy_part(dst: *mut u8, src: *mut u8, t: u32) {
    // SAFETY: as the caller guarantees.
    unsafe {
        if !rt::plain(t) {
            return copy_value(dst, src, t);
        }
        match ty(t).size {
            8 => mem::set_word(dst, 0, mem::word(src, 0)),
            n => mem::copy(dst, src, n as usize),
        }
    }
}

/// Copies again, over the bytes already copied, the parts of a value that
/// own something ([`rt::CopyParts`]).
///
/// # Safety
/// `src` must hold a value whose parts these are, and `dst` its bytes.
unsafe fn copy_parts(dst: *mut u8, src: *mut u8, parts: &[(u32, u32)]) {
    for &(off, t) in parts {
        // SAFETY: each part is at its offset in both.
        unsafe { copy_value(dst.add(off as usize), src.add(off as usize), t) };
    }
}

/// `.copy()`: writes at `dst` a deep copy of the value at `src`.
///
/// # Safety
/// `src` must hold a value of type `t`, and `dst` have room for one.
pub(crate) unsafe fn copy_value(dst: *mut u8, src: *mut u8, t: u32) {
    let entry = ty(t);
    // SAFETY: the parts are where the type says.
    unsafe {
        if rt::plain(t) {
            mem::copy(dst, src, entry.size as usize);
            return;
        }
        match rt::copy_parts(t) {
            rt::CopyParts::Parts(parts) => {
                mem::copy(dst, src, entry.size as usize);
                copy_parts(dst, src, parts);
                return;
            }
            rt::CopyParts::Variants(variants) if matches!(entry.kind, Kind::Enum { error: false, .. }) => {
                mem::copy(dst, src, entry.size as usize);
                if let Some(parts) = variants.get(tag(src, entry) as usize) {
                    copy_parts(dst, src, parts);
                }
                return;
            }
            _ => {}
        }
        match &entry.kind {
            Kind::String | Kind::Path | Kind::Bytes => {
                let b = mem::at::<Text>(src).bytes();
                (dst as *mut Text).write(Text::from_bytes(b));
            }
            Kind::List(e) => {
                let from = mem::at::<List>(src);
                let size = ty(*e).size;
                let mut to = List::empty();
                to.reserve(from.len, size);
                // The elements' bytes at once; then, for elements that own
                // something, those parts again, the element's kind looked at
                // once rather than for each (decision 262).
                match rt::copy_parts(*e) {
                    _ if rt::plain(*e) => mem::copy(to.ptr, from.ptr, (from.len * u64::from(size)) as usize),
                    rt::CopyParts::Parts(parts) => {
                        mem::copy(to.ptr, from.ptr, (from.len * u64::from(size)) as usize);
                        for i in 0..from.len {
                            copy_parts(to.at(i, size), from.at(i, size), parts);
                        }
                    }
                    _ => match ty(*e).kind {
                        Kind::String | Kind::Path | Kind::Bytes => {
                            for i in 0..from.len {
                                let b = mem::at::<Text>(from.at(i, size)).bytes();
                                (to.at(i, size) as *mut Text).write(Text::from_bytes(b));
                            }
                        }
                        Kind::Option(payload) => {
                            for i in 0..from.len {
                                let (d, s) = (to.at(i, size), from.at(i, size));
                                let tg = mem::word(s, 0);
                                mem::set_word(d, 0, tg);
                                if tg == 0 {
                                    copy_part(d.add(8), s.add(8), payload);
                                }
                            }
                        }
                        _ => {
                            for i in 0..from.len {
                                copy_value(to.at(i, size), from.at(i, size), *e);
                            }
                        }
                    },
                }
                to.len = from.len;
                (dst as *mut List).write(to);
            }
            Kind::Option(payload) => {
                let tg = tag(src, entry);
                mem::set_word(dst, 0, tg);
                if tg == 0 {
                    copy_part(dst.add(8), src.add(8), *payload);
                }
            }
            Kind::Result(ok, err) => {
                let tg = tag(src, entry);
                mem::set_word(dst, 0, tg);
                copy_part(dst.add(8), src.add(8), if tg == 0 { *ok } else { *err });
            }
            Kind::Tuple(slots) => {
                for s in slots {
                    copy_part(dst.add(s.offset as usize), src.add(s.offset as usize), s.ty);
                }
            }
            Kind::Dict(..) | Kind::Set(_) => {
                let t = crate::dict::table(src);
                mem::set_word(dst, 0, crate::dict::into_word(crate::dict::copy_table(t)));
            }
            Kind::Box(inner) => {
                let size = ty(*inner).size;
                let cell = mem::alloc(size as usize);
                copy_value(cell, mem::word(src, 0) as *mut u8, *inner);
                mem::set_word(dst, 0, cell as u64);
            }
            Kind::Dyn => {
                let t = mem::word(src, 0) as u32;
                let cell = mem::alloc(ty(t).size as usize);
                copy_value(cell, mem::word(src, 8) as *mut u8, t);
                mem::set_word(dst, 0, u64::from(t));
                mem::set_word(dst, 8, cell as u64);
            }
            Kind::Struct { fields, .. } => {
                for f in fields {
                    copy_part(dst.add(f.offset as usize), src.add(f.offset as usize), f.ty);
                }
            }
            Kind::Enum { variants, error, .. } => {
                // The number of its type and its trace (shared, and held
                // once more: a trace is never changed in place), then the
                // variant's fields.
                let off = entry.kind.tag_offset();
                mem::copy(dst, src, off as usize);
                if *error {
                    crate::intrinsics::retain_trace(mem::word(dst, abi::ERROR_TRACE));
                }
                let tg = tag(src, entry);
                mem::set_word(dst, off, tg);
                if let Some(v) = variants.get(tg as usize) {
                    for f in &v.fields {
                        copy_part(dst.add(f.offset as usize), src.add(f.offset as usize), f.ty);
                    }
                }
            }
            Kind::ErrSet(_) => {
                let decl = concrete(src, t);
                if decl == t {
                    mem::copy(dst, src, entry.size as usize);
                } else {
                    copy_value(dst, src, decl);
                }
            }
            // Another sender (or receiver) of the same channel.
            Kind::Sender(_) | Kind::Receiver(_) => {
                mem::copy(dst, src, 8);
                crate::tasks::copy_channel_end(
                    mem::word(src, 0) as u32,
                    matches!(entry.kind, Kind::Receiver(_)),
                );
            }
            // Another owner (or weak handle) of the same cell.
            Kind::Shared(_) | Kind::Weak(_) => {
                mem::copy(dst, src, 8);
                crate::cells::copy_handle(mem::word(src, 0) as *mut u8, matches!(entry.kind, Kind::Weak(_)));
            }
            Kind::Cell(v) => copy_value(dst, src, *v),
            Kind::Arena(_) => crate::cells::copy_arena(dst, src),
            // Another copy of a handle (decision 237).
            Kind::Opaque(n) if counted(n) => {
                mem::copy(dst, src, 8);
                crate::caps::retain_cell(mem::word(src, 0) as *mut u8);
            }
            // Another copy of a function value and what its environment
            // holds (decision 248).
            Kind::Fn => {
                mem::copy(dst, src, entry.size as usize);
                retain_value(dst, t);
            }
            // An unlocked mutex holding a copy of the value.
            Kind::Mutex(v) | Kind::RwLock(v) => {
                mem::set_word(dst, abi::MUTEX_LOCKED, 0);
                mem::set_word(dst, abi::MUTEX_HOLDER, 0);
                mem::set_word(dst, abi::MUTEX_POISONED, mem::word(src, abi::MUTEX_POISONED));
                mem::set_word(dst, abi::MUTEX_WRITERS, 0);
                let off = abi::MUTEX_HEADER as usize;
                copy_value(dst.add(off), src.add(off), *v);
            }
            _ => mem::copy(dst, src, entry.size as usize),
        }
    }
}

// ----------------------------------------------------------- equality, order

/// Structural equality, through references; texts of any kind with the
/// same bytes are equal.
///
/// # Safety
/// `a` and `b` must hold values of types `ta` and `tb`.
pub(crate) unsafe fn equal(a: *mut u8, ta: u32, b: *mut u8, tb: u32) -> bool {
    // SAFETY: the parts are where the types say.
    unsafe {
        let (a, ta) = target(a, ta);
        let (b, tb) = target(b, tb);
        let (ea, eb) = (ty(ta), ty(tb));
        if let (Some(x), Some(y)) = (text_at(a, ea), text_at(b, eb)) {
            return x == y;
        }
        match (&ea.kind, &eb.kind) {
            (Kind::Int | Kind::Sized(_), Kind::Int | Kind::Sized(_))
            | (Kind::U64, Kind::U64)
            | (Kind::Bool, Kind::Bool)
            | (Kind::Char, Kind::Char) => mem::word(a, 0) == mem::word(b, 0),
            (Kind::I128, Kind::I128) | (Kind::U128, Kind::U128) => {
                crate::wide::bits(a) == crate::wide::bits(b)
            }
            (Kind::Float, Kind::Float) => f64::from_bits(mem::word(a, 0)) == f64::from_bits(mem::word(b, 0)),
            (Kind::F32, Kind::F32) => f32_at(a) == f32_at(b),
            (Kind::Unit, Kind::Unit) => true,
            (Kind::DateTime, Kind::DateTime) => mem::word(a, 0) == mem::word(b, 0),
            (Kind::Bytes, Kind::Bytes) => mem::at::<Text>(a).bytes() == mem::at::<Text>(b).bytes(),
            (Kind::Tuple(xs), Kind::Tuple(ys)) => {
                xs.len() == ys.len()
                    && xs
                        .iter()
                        .zip(ys)
                        .all(|(x, y)| equal(a.add(x.offset as usize), x.ty, b.add(y.offset as usize), y.ty))
            }
            (Kind::List(x), Kind::List(y)) => {
                let (la, lb) = (mem::at::<List>(a), mem::at::<List>(b));
                let (sx, sy) = (ty(*x).size, ty(*y).size);
                la.len == lb.len && (0..la.len).all(|i| equal(la.at(i, sx), *x, lb.at(i, sy), *y))
            }
            (Kind::Option(x), Kind::Option(y)) => {
                let (tx, ty_) = (tag(a, ea), tag(b, eb));
                tx == ty_ && (tx != 0 || equal(a.add(8), *x, b.add(8), *y))
            }
            (Kind::Result(x1, x2), Kind::Result(y1, y2)) => {
                let (tx, ty_) = (tag(a, ea), tag(b, eb));
                tx == ty_
                    && if tx == 0 {
                        equal(a.add(8), *x1, b.add(8), *y1)
                    } else {
                        equal(a.add(8), *x2, b.add(8), *y2)
                    }
            }
            (Kind::Box(x), Kind::Box(y)) => {
                equal(mem::word(a, 0) as *mut u8, *x, mem::word(b, 0) as *mut u8, *y)
            }
            (Kind::Set(_), Kind::Set(_)) => {
                let (x, y) = (crate::dict::table(a), crate::dict::table(b));
                x.len() == y.len() && (0..x.len()).all(|i| y.position(x.key_at(i), x.key).is_some())
            }
            (Kind::Dict(..), Kind::Dict(..)) => {
                let (x, y) = (crate::dict::table(a), crate::dict::table(b));
                x.len() == y.len()
                    && (0..x.len()).all(|i| match y.position(x.key_at(i), x.key) {
                        Some(j) => equal(x.val_at(i), x.val, y.val_at(j), y.val),
                        None => false,
                    })
            }
            (Kind::Struct { name: n1, fields: f1, .. }, Kind::Struct { name: n2, fields: f2, .. }) => {
                n1 == n2
                    && f1.len() == f2.len()
                    && f1
                        .iter()
                        .zip(f2)
                        .all(|(x, y)| equal(a.add(x.offset as usize), x.ty, b.add(y.offset as usize), y.ty))
            }
            (Kind::Enum { .. } | Kind::ErrSet(_), Kind::Enum { .. } | Kind::ErrSet(_)) => {
                let (ca, cb) = (concrete(a, ta), concrete(b, tb));
                let (ea, eb) = (ty(ca), ty(cb));
                let (Kind::Enum { name: n1, variants: v1, .. }, Kind::Enum { name: n2, variants: v2, .. }) =
                    (&ea.kind, &eb.kind)
                else {
                    return false;
                };
                let (tx, ty_) = (tag(a, ea), tag(b, eb));
                if n1 != n2 || tx != ty_ {
                    return false;
                }
                match (v1.get(tx as usize), v2.get(ty_ as usize)) {
                    (Some(x), Some(y)) => {
                        x.fields.iter().zip(&y.fields).all(|(f, g)| {
                            equal(a.add(f.offset as usize), f.ty, b.add(g.offset as usize), g.ty)
                        })
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }
}

/// # Safety
/// `p` must hold an `f32`.
unsafe fn f32_at(p: *const u8) -> f32 {
    // SAFETY: an `f32` is the low half of its word.
    unsafe { (p as *const f32).read() }
}

/// The order of two values of an ordered type; `None` for NaN and values
/// without an order.
///
/// # Safety
/// `a` and `b` must hold values of types `ta` and `tb`.
pub(crate) unsafe fn compare(a: *mut u8, ta: u32, b: *mut u8, tb: u32) -> Option<Ordering> {
    // SAFETY: the parts are where the types say.
    unsafe {
        let (a, ta) = target(a, ta);
        let (b, tb) = target(b, tb);
        let (ea, eb) = (ty(ta), ty(tb));
        if let (Some(x), Some(y)) = (text_at(a, ea), text_at(b, eb)) {
            return Some(x.cmp(y));
        }
        match (&ea.kind, &eb.kind) {
            (Kind::Int | Kind::Sized(_), Kind::Int | Kind::Sized(_)) => {
                Some((mem::word(a, 0) as i64).cmp(&(mem::word(b, 0) as i64)))
            }
            (Kind::U64, Kind::U64) | (Kind::Bool, Kind::Bool) | (Kind::Char, Kind::Char) => {
                Some(mem::word(a, 0).cmp(&mem::word(b, 0)))
            }
            (Kind::I128, Kind::I128) => {
                Some((crate::wide::bits(a) as i128).cmp(&(crate::wide::bits(b) as i128)))
            }
            (Kind::U128, Kind::U128) => Some(crate::wide::bits(a).cmp(&crate::wide::bits(b))),
            (Kind::Float, Kind::Float) => {
                f64::from_bits(mem::word(a, 0)).partial_cmp(&f64::from_bits(mem::word(b, 0)))
            }
            (Kind::F32, Kind::F32) => f32_at(a).partial_cmp(&f32_at(b)),
            (Kind::Unit, Kind::Unit) => Some(Ordering::Equal),
            (Kind::DateTime, Kind::DateTime) => Some((mem::word(a, 0) as i64).cmp(&(mem::word(b, 0) as i64))),
            (Kind::Bytes, Kind::Bytes) => Some(mem::at::<Text>(a).bytes().cmp(mem::at::<Text>(b).bytes())),
            (Kind::Tuple(xs), Kind::Tuple(ys)) => {
                for (x, y) in xs.iter().zip(ys) {
                    match compare(a.add(x.offset as usize), x.ty, b.add(y.offset as usize), y.ty) {
                        Some(Ordering::Equal) => {}
                        other => return other,
                    }
                }
                Some(xs.len().cmp(&ys.len()))
            }
            (Kind::List(x), Kind::List(y)) => {
                let (la, lb) = (mem::at::<List>(a), mem::at::<List>(b));
                let (sx, sy) = (ty(*x).size, ty(*y).size);
                for i in 0..la.len.min(lb.len) {
                    match compare(la.at(i, sx), *x, lb.at(i, sy), *y) {
                        Some(Ordering::Equal) => {}
                        other => return other,
                    }
                }
                Some(la.len.cmp(&lb.len))
            }
            (Kind::Option(x), Kind::Option(y)) => {
                let (tx, ty_) = (tag(a, ea), tag(b, eb));
                match tx.cmp(&ty_) {
                    Ordering::Equal if tx == 0 => compare(a.add(8), *x, b.add(8), *y),
                    o => Some(o),
                }
            }
            (Kind::Result(x1, x2), Kind::Result(y1, y2)) => {
                let (tx, ty_) = (tag(a, ea), tag(b, eb));
                match tx.cmp(&ty_) {
                    Ordering::Equal if tx == 0 => compare(a.add(8), *x1, b.add(8), *y1),
                    Ordering::Equal => compare(a.add(8), *x2, b.add(8), *y2),
                    o => Some(o),
                }
            }
            (Kind::Box(x), Kind::Box(y)) => {
                compare(mem::word(a, 0) as *mut u8, *x, mem::word(b, 0) as *mut u8, *y)
            }
            (Kind::Struct { fields: f1, .. }, Kind::Struct { fields: f2, .. }) => {
                for (x, y) in f1.iter().zip(f2) {
                    match compare(a.add(x.offset as usize), x.ty, b.add(y.offset as usize), y.ty) {
                        Some(Ordering::Equal) => {}
                        other => return other,
                    }
                }
                Some(f1.len().cmp(&f2.len()))
            }
            (Kind::Enum { .. } | Kind::ErrSet(_), Kind::Enum { .. } | Kind::ErrSet(_)) => {
                let (ca, cb) = (concrete(a, ta), concrete(b, tb));
                let (ea, eb) = (ty(ca), ty(cb));
                let (Kind::Enum { variants: v1, .. }, Kind::Enum { variants: v2, .. }) = (&ea.kind, &eb.kind)
                else {
                    return None;
                };
                let (tx, ty_) = (tag(a, ea), tag(b, eb));
                if tx != ty_ {
                    return Some(tx.cmp(&ty_));
                }
                let (Some(x), Some(y)) = (v1.get(tx as usize), v2.get(ty_ as usize)) else { return None };
                for (f, g) in x.fields.iter().zip(&y.fields) {
                    match compare(a.add(f.offset as usize), f.ty, b.add(g.offset as usize), g.ty) {
                        Some(Ordering::Equal) => {}
                        other => return other,
                    }
                }
                Some(x.fields.len().cmp(&y.fields.len()))
            }
            _ => None,
        }
    }
}

// -------------------------------------------------------------- formatting

/// The `Display` text of a value whose impl is structural: the plain text
/// of texts and characters, and the `Debug` text of everything else.
///
/// # Safety
/// `p` must hold a value of type `t`.
pub(crate) unsafe fn display(out: &mut Vec<u8>, p: *mut u8, t: u32) {
    // SAFETY: as the caller guarantees.
    unsafe {
        let (p, t) = target(p, t);
        let entry = ty(t);
        if let Some(b) = text_at(p, entry) {
            out.extend_from_slice(b);
            return;
        }
        match entry.kind {
            Kind::Char => push_char(out, mem::word(p, 0) as u32),
            // std's `Contextual` (prelude): `message: cause`.
            Kind::Enum { .. } | Kind::ErrSet(_) => {
                let decl = concrete(p, t);
                let entry = ty(decl);
                if let Kind::Enum { name, variants, .. } = &entry.kind
                    && name == "Contextual"
                    && let Some(v) = variants.get(tag(p, entry) as usize)
                    && let [message, cause] = v.fields.as_slice()
                {
                    display(out, p.add(message.offset as usize), message.ty);
                    out.extend_from_slice(b": ");
                    display(out, p.add(cause.offset as usize), cause.ty);
                    return;
                }
                debug(out, p, t)
            }
            _ => debug(out, p, t),
        }
    }
}

/// The decimal digits of `n`, after a `-` when it is negative, as
/// `Display` writes an `int`, written at the end of `buf`.
pub(crate) fn int_digits(n: i64, buf: &mut [u8; 20]) -> &[u8] {
    let mut m = n.unsigned_abs();
    let mut i = buf.len();
    loop {
        i -= 1;
        buf[i] = b'0' + (m % 10) as u8;
        m /= 10;
        if m == 0 {
            break;
        }
    }
    if n < 0 {
        i -= 1;
        buf[i] = b'-';
    }
    &buf[i..]
}

fn push_char(out: &mut Vec<u8>, code: u32) {
    let c = char::from_u32(code).unwrap_or('\u{fffd}');
    let mut buf = [0u8; 4];
    out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
}

/// The `Debug` text of a value: as it is written.
///
/// # Safety
/// `p` must hold a value of type `t`.
pub(crate) unsafe fn debug(out: &mut Vec<u8>, p: *mut u8, t: u32) {
    let entry = ty(t);
    // SAFETY: the parts are where the type says.
    unsafe {
        match &entry.kind {
            Kind::Unit => out.extend_from_slice(b"()"),
            Kind::Int | Kind::Sized(_) => {
                out.extend_from_slice(int_digits(mem::word(p, 0) as i64, &mut [0u8; 20]))
            }
            Kind::U64 => out.extend_from_slice(mem::word(p, 0).to_string().as_bytes()),
            Kind::I128 => out.extend_from_slice((crate::wide::bits(p) as i128).to_string().as_bytes()),
            Kind::U128 => out.extend_from_slice(crate::wide::bits(p).to_string().as_bytes()),
            Kind::Float => out.extend_from_slice(float_repr(f64::from_bits(mem::word(p, 0))).as_bytes()),
            Kind::F32 => out.extend_from_slice(format!("{:?}", f32_at(p)).as_bytes()),
            Kind::Bool => out.extend_from_slice(if mem::word(p, 0) != 0 { b"True" } else { b"False" }),
            Kind::Char => {
                let c = char::from_u32(mem::word(p, 0) as u32).unwrap_or('\u{fffd}');
                out.extend_from_slice(quote_char(c).as_bytes());
            }
            Kind::Str | Kind::String | Kind::Path => {
                let b = text(p, t).unwrap_or(&[]);
                out.extend_from_slice(quote_str(&String::from_utf8_lossy(b)).as_bytes());
            }
            Kind::Bytes => out.extend_from_slice(quote_bytes(mem::at::<Text>(p).bytes()).as_bytes()),
            Kind::DateTime => out.extend_from_slice(datetime_text(mem::word(p, 0) as i64).as_bytes()),
            Kind::Ref(_) | Kind::Dyn => {
                let (q, t) = target(p, t);
                debug(out, q, t)
            }
            // `Box` is a struct of one field, `boxed`, to the structural
            // impls.
            Kind::Box(inner) => {
                out.extend_from_slice(b"Box(boxed=");
                debug(out, mem::word(p, 0) as *mut u8, *inner);
                out.push(b')');
            }
            Kind::Tuple(slots) => {
                out.push(b'(');
                for (i, s) in slots.iter().enumerate() {
                    if i > 0 {
                        out.extend_from_slice(b", ");
                    }
                    debug(out, p.add(s.offset as usize), s.ty);
                }
                if slots.len() == 1 {
                    out.push(b',');
                }
                out.push(b')');
            }
            Kind::List(e) => {
                let list = mem::at::<List>(p);
                let size = ty(*e).size;
                out.push(b'[');
                for i in 0..list.len {
                    if i > 0 {
                        out.extend_from_slice(b", ");
                    }
                    debug(out, list.at(i, size), *e);
                }
                out.push(b']');
            }
            Kind::Option(payload) => {
                if tag(p, entry) == 0 {
                    out.extend_from_slice(b"Some(");
                    debug(out, p.add(8), *payload);
                    out.push(b')');
                } else {
                    out.extend_from_slice(b"None");
                }
            }
            Kind::Result(ok, err) => {
                if tag(p, entry) == 0 {
                    out.extend_from_slice(b"Ok(");
                    debug(out, p.add(8), *ok);
                } else {
                    out.extend_from_slice(b"Err(");
                    debug(out, p.add(8), *err);
                }
                out.push(b')');
            }
            Kind::Struct { name, fields, .. } => {
                let short = name.rsplit('.').next().unwrap_or(name);
                out.extend_from_slice(short.as_bytes());
                if short == "System" {
                    return;
                }
                out.push(b'(');
                fields_debug(out, p, fields);
                out.push(b')');
            }
            Kind::Enum { variants, .. } => {
                let tg = tag(p, entry);
                match variants.get(tg as usize) {
                    Some(v) => {
                        out.extend_from_slice(v.name.as_bytes());
                        if !v.fields.is_empty() {
                            out.push(b'(');
                            fields_debug(out, p, &v.fields);
                            out.push(b')');
                        }
                    }
                    None => out.extend_from_slice(b"?"),
                }
            }
            Kind::ErrSet(_) => {
                let decl = concrete(p, t);
                if decl != t {
                    debug(out, p, decl);
                }
            }
            Kind::Dict(..) => {
                let t = crate::dict::table(p);
                out.push(b'{');
                for i in 0..t.len() {
                    if i > 0 {
                        out.extend_from_slice(b", ");
                    }
                    debug(out, t.key_at(i), t.key);
                    out.extend_from_slice(b": ");
                    debug(out, t.val_at(i), t.val);
                }
                out.push(b'}');
            }
            Kind::Set(_) => {
                let t = crate::dict::table(p);
                if t.len() == 0 {
                    out.extend_from_slice(b"set()");
                } else {
                    out.push(b'{');
                    for i in 0..t.len() {
                        if i > 0 {
                            out.extend_from_slice(b", ");
                        }
                        debug(out, t.key_at(i), t.key);
                    }
                    out.push(b'}');
                }
            }
            Kind::Entry(..) => out.extend_from_slice(b"<entry>"),
            // As std's `Debug` for raw pointers writes one, without its
            // address (22, clause 6).
            Kind::Ptr(_) => out.extend_from_slice(b"ptr"),
            Kind::Scope => out.extend_from_slice(b"<scope>"),
            Kind::Handle(_) => {
                let path = crate::tasks::task_path(mem::word(p, 0) as u32);
                out.extend_from_slice(format!("<task {path}>").as_bytes());
            }
            Kind::Cancellation => out.extend_from_slice(b"<cancellation>"),
            Kind::Sender(_) => out.extend_from_slice(b"<sender>"),
            Kind::Receiver(_) => out.extend_from_slice(b"<receiver>"),
            Kind::Timer => out.extend_from_slice(b"<timer>"),
            Kind::Mutex(v) => {
                out.extend_from_slice(b"Mutex(");
                if mem::word(p, abi::MUTEX_LOCKED) != 0 {
                    out.extend_from_slice(b"<locked>");
                } else {
                    debug(out, p.add(abi::MUTEX_HEADER as usize), *v);
                }
                out.push(b')');
            }
            // Readers leave the value to be read; a writer does not.
            Kind::RwLock(v) => {
                out.extend_from_slice(b"RwLock(");
                if mem::word(p, abi::MUTEX_LOCKED) == 1 {
                    out.extend_from_slice(b"<locked>");
                } else {
                    debug(out, p.add(abi::MUTEX_HEADER as usize), *v);
                }
                out.push(b')');
            }
            Kind::Guard(v, _) | Kind::ReadGuard(v) => debug(out, mem::word(p, 0) as *mut u8, *v),
            Kind::Shared(v) => {
                let cell = mem::word(p, 0) as *mut u8;
                if crate::cells::alive(cell) {
                    debug(out, cell, *v);
                } else {
                    out.extend_from_slice(b"<dropped>");
                }
            }
            Kind::Weak(_) => out.extend_from_slice(b"(Weak)"),
            Kind::Cell(v) => {
                out.extend_from_slice(b"Cell(");
                debug(out, p, *v);
                out.push(b')');
            }
            Kind::Arena(_) => crate::cells::debug_arena(out, p),
            Kind::Fn => out.extend_from_slice(b"<fn>"),
            Kind::Iter(_) => out.extend_from_slice(b"<iter>"),
            // A test clock shows as the interpreter's clocks do.
            Kind::Opaque(n) if n == "TestClock" || n == "Lent TestClock" => out.extend_from_slice(b"Clock"),
            // A `Lent` shows as its handle.
            Kind::Opaque(n) => out.extend_from_slice(n.strip_prefix("Lent ").unwrap_or(n).as_bytes()),
            Kind::Const(n) => out.extend_from_slice(n.to_string().as_bytes()),
        }
    }
}

/// # Safety
/// `p` must hold the fields at their offsets.
unsafe fn fields_debug(out: &mut Vec<u8>, p: *mut u8, fields: &[abi::Field]) {
    for (i, f) in fields.iter().enumerate() {
        if i > 0 {
            out.extend_from_slice(b", ");
        }
        if let Some(n) = &f.name {
            out.extend_from_slice(n.as_bytes());
            out.push(b'=');
        }
        // SAFETY: the field is where the type says.
        unsafe { debug(out, p.add(f.offset as usize), f.ty) };
    }
}

/// The shortest text that reads back as the same `f64`, always with a
/// point or an exponent.
pub(crate) fn float_repr(f: f64) -> String {
    format!("{f:?}")
}

pub(crate) fn quote_str(s: &str) -> String {
    let mut q = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => q.push_str("\\\""),
            '\\' => q.push_str("\\\\"),
            '\n' => q.push_str("\\n"),
            '\t' => q.push_str("\\t"),
            '\r' => q.push_str("\\r"),
            c if c.is_control() => q.push_str(&format!("\\u{{{:x}}}", c as u32)),
            c => q.push(c),
        }
    }
    q.push('"');
    q
}

/// The civil date of a day counted from 1970-01-01 (Howard Hinnant's
/// `civil_from_days`, the interpreter's `civil`).
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

/// The RFC 3339 text of a time in nanoseconds since the epoch, in UTC,
/// with the fraction of a second when there is one (the interpreter's
/// `datetime_text`).
pub(crate) fn datetime_text(nanos: i64) -> String {
    let mut s = strftime(nanos, "%Y-%m-%dT%H:%M:%S");
    let frac = nanos.rem_euclid(1_000_000_000);
    if frac != 0 {
        let digits = format!("{frac:09}");
        s.push('.');
        s.push_str(digits.trim_end_matches('0'));
    }
    s.push('Z');
    s
}

/// A time by the `strftime` directives `%Y %y %m %d %H %M %S %F %T %%`, in
/// UTC; other characters are copied (the interpreter's `strftime`).
pub(crate) fn strftime(nanos: i64, pattern: &str) -> String {
    let secs = nanos.div_euclid(1_000_000_000);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (y, m, d) = civil(days);
    let (hh, mm, ss) = (rem / 3600, (rem / 60) % 60, rem % 60);
    let mut out = String::new();
    let mut cs = pattern.chars();
    while let Some(c) = cs.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        match cs.next() {
            Some('Y') => out.push_str(&format!("{y:04}")),
            Some('y') => out.push_str(&format!("{:02}", y.rem_euclid(100))),
            Some('m') => out.push_str(&format!("{m:02}")),
            Some('d') => out.push_str(&format!("{d:02}")),
            Some('H') => out.push_str(&format!("{hh:02}")),
            Some('M') => out.push_str(&format!("{mm:02}")),
            Some('S') => out.push_str(&format!("{ss:02}")),
            Some('F') => out.push_str(&format!("{y:04}-{m:02}-{d:02}")),
            Some('T') => out.push_str(&format!("{hh:02}:{mm:02}:{ss:02}")),
            Some('%') => out.push('%'),
            Some(other) => {
                out.push('%');
                out.push(other);
            }
            None => out.push('%'),
        }
    }
    out
}

/// `b"..."`: the printable ASCII bytes as they are, the others escaped
/// (the interpreter's `quote_bytes`).
pub(crate) fn quote_bytes(b: &[u8]) -> String {
    let mut out = String::from("b\"");
    for &x in b {
        match x {
            b'\n' => out.push_str("\\n"),
            b'\t' => out.push_str("\\t"),
            b'\r' => out.push_str("\\r"),
            b'\\' => out.push_str("\\\\"),
            b'"' => out.push_str("\\\""),
            0x20..=0x7e => out.push(x as char),
            _ => out.push_str(&format!("\\x{x:02x}")),
        }
    }
    out.push('"');
    out
}

pub(crate) fn quote_char(c: char) -> String {
    match c {
        '\'' => "'\\''".into(),
        '\\' => "'\\\\'".into(),
        '\n' => "'\\n'".into(),
        '\t' => "'\\t'".into(),
        '\r' => "'\\r'".into(),
        c if c.is_control() => format!("'\\u{{{:x}}}'", c as u32),
        c => format!("'{c}'"),
    }
}

/// Writes `Str` for these bytes at `dst`.
///
/// # Safety
/// `dst` must have room for a `str`.
pub(crate) unsafe fn write_str(dst: *mut u8, b: &[u8]) {
    // SAFETY: as the caller guarantees.
    unsafe { (dst as *mut Str).write(Str { ptr: b.as_ptr(), len: b.len() as u64 }) }
}

/// Writes a new `String` holding these bytes at `dst`.
///
/// # Safety
/// `dst` must have room for a `String`.
pub(crate) unsafe fn write_string(dst: *mut u8, b: &[u8]) {
    // SAFETY: as the caller guarantees.
    unsafe { (dst as *mut Text).write(Text::from_bytes(b)) }
}
