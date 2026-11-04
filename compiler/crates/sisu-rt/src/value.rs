//! The structural operations on values in the native layout, by their
//! types: drop, `.copy()`, equality, order, and the `Display` and `Debug`
//! texts. Each follows the interpreter's rule for the same value
//! (`sisu-interp/src/drops.rs`, `ops.rs`, `format.rs`, and `copy_value`),
//! and each walk is the runtime in Sisu's (`compiler/rt`, decisions 350 and
//! 383), which these functions call through `bridge.rs`; what stays here is
//! what the runtime in Rust keeps of its own: the forgetting of a drop
//! (decision 223), the counted handles, and the texts of times and quoted
//! names its messages write.

use crate::abi::{self, Kind, TypeEntry};
use crate::mem::{self, Str, Text};
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

thread_local! {
    /// Whether drops only free memory: no drop glue, no `Drop`, no check
    /// of a guard's invariant ([`forget_value`]); each thread's own, so
    /// that a worker of `std.par` forgets only what it frees (decision 333).
    static FORGETTING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Whether the values being dropped are forgotten rather than dropped.
pub(crate) fn forgetting() -> bool {
    FORGETTING.with(std::cell::Cell::get)
}

/// Runs `f` with drops forgetting: what they free is freed, and no Sisu
/// code runs (no drop glue, no `Drop`, no guard's check). It is how the
/// interpreter's host lets go of values no program can see again: the
/// error `main` returned, the objects of capabilities when the run ends,
/// and the frames of an abandoned generator (decisions 95 and 223).
pub(crate) fn forget<R>(f: impl FnOnce() -> R) -> R {
    let was = FORGETTING.with(|c| c.replace(true));
    let r = f();
    FORGETTING.with(|c| c.set(was));
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
/// order the interpreter does; the status says whether a panic is leaving.
/// The runtime in Sisu walks the value (`compiler/rt`, decision 350), on the
/// system stack, where it moves aside what would run the program's code;
/// on a task's own stack, a struct's or an enum's drop glue, and a guard's
/// check of its value's invariant, run where they are, and the walk of
/// what the glue leaves out on the system stack.
///
/// # Safety
/// `p` must hold a value of type `t`, which is dead afterwards.
pub(crate) unsafe fn drop_value(p: *mut u8, t: u32) -> u8 {
    if !rt::needs_drop(t) {
        return abi::OK;
    }
    if crate::sched::deferring() {
        // SAFETY: as the caller guarantees.
        return unsafe { crate::bridge::drop(p, t) };
    }
    let entry = ty(t);
    // SAFETY: as the caller guarantees.
    unsafe {
        match &entry.kind {
            Kind::Struct { glue: Some(g), .. } | Kind::Enum { glue: Some(g), .. } if !forgetting() => {
                // An error lets go of its return trace (decision 215).
                if let Kind::Enum { error: true, .. } = entry.kind {
                    crate::intrinsics::release_trace(mem::word(p, abi::ERROR_TRACE));
                    mem::set_word(p, abi::ERROR_TRACE, 0);
                }
                let status = crate::bridge::run_glue(*g, p);
                after_part(status, crate::sched::system_status(|| crate::bridge::drop_rest(p, t)))
            }
            Kind::Guard(..) | Kind::ReadGuard(_) => crate::bridge::object_drop(p, t),
            _ => crate::sched::system_status(|| crate::bridge::drop(p, t)),
        }
    }
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
    // SAFETY: as the caller guarantees.
    unsafe { crate::bridge::retain(p, t) }
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

// ------------------------------------------------------------ copy, equality

/// `.copy()`: writes at `dst` a deep copy of the value at `src`, by the
/// runtime in Sisu (decision 350).
///
/// # Safety
/// `src` must hold a value of type `t`, and `dst` have room for one.
pub(crate) unsafe fn copy_value(dst: *mut u8, src: *mut u8, t: u32) {
    // SAFETY: as the caller guarantees.
    unsafe {
        if rt::plain(t) {
            mem::copy(dst, src, ty(t).size as usize);
            return;
        }
        crate::bridge::copy(dst, src, t)
    }
}

/// Structural equality, through references; texts of any kind with the
/// same bytes are equal. The runtime in Sisu's (decision 350).
///
/// # Safety
/// `a` and `b` must hold values of types `ta` and `tb`.
pub(crate) unsafe fn equal(a: *mut u8, ta: u32, b: *mut u8, tb: u32) -> bool {
    // SAFETY: as the caller guarantees.
    unsafe { crate::bridge::equal(a, ta, b, tb) }
}

/// The order of two values of an ordered type; `None` for NaN and values
/// without an order. The runtime in Sisu's (decision 350).
///
/// # Safety
/// `a` and `b` must hold values of types `ta` and `tb`.
pub(crate) unsafe fn compare(a: *mut u8, ta: u32, b: *mut u8, tb: u32) -> Option<Ordering> {
    // SAFETY: as the caller guarantees.
    unsafe { crate::bridge::compare(a, ta, b, tb) }
}

// -------------------------------------------------------------- formatting

/// The `Display` text of a value whose impl is structural: the plain text
/// of texts and characters, and the `Debug` text of everything else. The
/// runtime in Sisu's (decision 383).
///
/// # Safety
/// `p` must hold a value of type `t`.
pub(crate) unsafe fn display(out: &mut Vec<u8>, p: *mut u8, t: u32) {
    // SAFETY: as the caller guarantees.
    unsafe { crate::bridge::show(out, p, t, true) }
}

/// The `Debug` text of a value: as it is written. The runtime in Sisu's
/// (decision 383).
///
/// # Safety
/// `p` must hold a value of type `t`.
pub(crate) unsafe fn debug(out: &mut Vec<u8>, p: *mut u8, t: u32) {
    // SAFETY: as the caller guarantees.
    unsafe { crate::bridge::show(out, p, t, false) }
}

/// The shortest text that reads back as the same `f64`, always with a
/// point or an exponent (the runtime in Sisu's, decision 358).
pub(crate) fn float_repr(f: f64) -> String {
    crate::bridge::float_text(f.to_bits(), crate::bridge::SHORTEST, 0)
}

/// The shortest text that reads back as the same `f32`, as [`float_repr`]
/// writes an `f64`.
pub(crate) fn float32_repr(f: f32) -> String {
    crate::bridge::float_text(u64::from(f.to_bits()), crate::bridge::SHORTEST32, 0)
}

/// A time by the `strftime` directives of `pattern` (the runtime in
/// Sisu's `time.sisu`, decision 469).
pub(crate) fn strftime(nanos: i64, pattern: &str) -> String {
    crate::bridge::time_text(nanos, Some(pattern))
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
