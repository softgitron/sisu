//! The boundary with the runtime in Sisu (`compiler/rt`, decision 350): the
//! structural operations the runtime in Rust calls there (equality, the
//! structural hash, `.copy()`, the drop, and the counts of a copy's
//! capability handles), which walk a value by the type table's words
//! (`desc.rs`), and the functions that code calls back here, for what the
//! runtime in Rust keeps as objects of its own (iterators, the tables of
//! dicts and sets, channels, the guards of mutexes, `Shared` cells, arenas,
//! the cells of capability handles, return traces) and for the program's
//! drop glue.
//!
//! Each export of the runtime in Sisu has Sisu's convention (decision 189):
//! the address of its result, the address of each argument, and its status,
//! which is 0 unless the runtime's own code failed.

use crate::abi::{self, Kind};
use crate::mem;
use crate::rt::{self, ty};

unsafe extern "C" {
    fn sisu_rt_value_eq(
        ret: *mut u64,
        types: *const *const u64,
        a: *const *mut u8,
        ta: *const i64,
        b: *const *mut u8,
        tb: *const i64,
    ) -> u8;
    fn sisu_rt_value_cmp(
        ret: *mut i64,
        types: *const *const u64,
        a: *const *mut u8,
        ta: *const i64,
        b: *const *mut u8,
        tb: *const i64,
    ) -> u8;
    fn sisu_rt_value_hash(
        ret: *mut i64,
        types: *const *const u64,
        state: *const i64,
        p: *const *mut u8,
        t: *const i64,
    ) -> u8;
    fn sisu_rt_value_copy(
        ret: *mut u64,
        types: *const *const u64,
        dst: *const *mut u8,
        src: *const *mut u8,
        t: *const i64,
    ) -> u8;
    fn sisu_rt_value_drop(
        ret: *mut i64,
        types: *const *const u64,
        p: *const *mut u8,
        t: *const i64,
        mode: *const i64,
    ) -> u8;
    fn sisu_rt_value_drop_rest(
        ret: *mut i64,
        types: *const *const u64,
        p: *const *mut u8,
        t: *const i64,
        mode: *const i64,
    ) -> u8;
    fn sisu_rt_value_retain(ret: *mut u64, types: *const *const u64, p: *const *mut u8, t: *const i64) -> u8;
    fn sisu_rt_find(
        ret: *mut i64,
        text: *const *const u8,
        n: *const i64,
        part: *const *const u8,
        m: *const i64,
    ) -> u8;
    fn sisu_rt_has_edge(
        ret: *mut u64,
        text: *const *const u8,
        n: *const i64,
        part: *const *const u8,
        m: *const i64,
        at_end: *const u64,
    ) -> u8;
    fn sisu_rt_int_digits(
        ret: *mut u64,
        dst: *const *mut u8,
        low: *const i64,
        high: *const i64,
        base: *const i64,
        upper: *const u64,
    ) -> u8;
    fn sisu_rt_float_digits(
        ret: *mut u64,
        dst: *const *mut u8,
        bits: *const i64,
        mode: *const i64,
        precision: *const i64,
    ) -> u8;
}

/// `mode` of a drop: it forgets (decision 223), or it defers what would run
/// Sisu code, being on the system stack (`sched::defer`).
const FORGETTING: i64 = 1;
const DEFERRING: i64 = 2;

/// The runtime in Sisu failed in its own code, which only a broken program
/// or table makes it do.
fn failed(what: &str) -> ! {
    rt::internal(&format!("the runtime in Sisu failed in its {what}"))
}

/// Whether the type table has type `t`. A number outside it is `()`, as
/// `rt::ty` reads it: the values of a set's table, which has none
/// (`u32::MAX`), and only a broken program's otherwise; the runtime in Sisu,
/// which reads the table's words, is not given one.
fn known(t: u32) -> bool {
    rt::ty_checked(t).is_some()
}

/// Structural equality, through references.
///
/// # Safety
/// `a` and `b` must hold values of types `ta` and `tb`.
pub(crate) unsafe fn equal(a: *mut u8, ta: u32, b: *mut u8, tb: u32) -> bool {
    if !known(ta) || !known(tb) {
        return !known(ta) && !known(tb);
    }
    let types = crate::desc::base();
    let (ta, tb) = (i64::from(ta), i64::from(tb));
    let mut ret = 0u64;
    // SAFETY: the arguments are in memory of this frame, as the export's
    // convention asks, and the caller guarantees the values.
    if unsafe { sisu_rt_value_eq(&mut ret, &types, &a, &ta, &b, &tb) } != abi::OK {
        failed("equality");
    }
    ret != 0
}

/// The order of two values of an ordered type; `None` for NaN and values
/// without an order.
///
/// # Safety
/// `a` and `b` must hold values of types `ta` and `tb`.
pub(crate) unsafe fn compare(a: *mut u8, ta: u32, b: *mut u8, tb: u32) -> Option<std::cmp::Ordering> {
    if !known(ta) || !known(tb) {
        return (!known(ta) && !known(tb)).then_some(std::cmp::Ordering::Equal);
    }
    let types = crate::desc::base();
    let (ta, tb) = (i64::from(ta), i64::from(tb));
    let mut ret = 0i64;
    // SAFETY: as for [`equal`].
    if unsafe { sisu_rt_value_cmp(&mut ret, &types, &a, &ta, &b, &tb) } != abi::OK {
        failed("order");
    }
    match ret {
        -1 => Some(std::cmp::Ordering::Less),
        0 => Some(std::cmp::Ordering::Equal),
        1 => Some(std::cmp::Ordering::Greater),
        _ => None,
    }
}

/// The structural hash of the value at `p` mixed into `state` (decision
/// 256).
///
/// # Safety
/// `p` must hold a value of type `t`.
pub(crate) unsafe fn hash(state: u64, p: *mut u8, t: u32) -> u64 {
    if !known(t) {
        return state;
    }
    let types = crate::desc::base();
    let (state, t) = (state as i64, i64::from(t));
    let mut ret = 0i64;
    // SAFETY: as for [`equal`].
    if unsafe { sisu_rt_value_hash(&mut ret, &types, &state, &p, &t) } != abi::OK {
        failed("structural hash");
    }
    ret as u64
}

/// `.copy()`: writes at `dst` a deep copy of the value at `src`.
///
/// # Safety
/// `src` must hold a value of type `t`, and `dst` have room for one.
pub(crate) unsafe fn copy(dst: *mut u8, src: *mut u8, t: u32) {
    if !known(t) {
        return;
    }
    let types = crate::desc::base();
    let t = i64::from(t);
    let mut ret = 0u64;
    // SAFETY: as for [`equal`].
    if unsafe { sisu_rt_value_copy(&mut ret, &types, &dst, &src, &t) } != abi::OK {
        failed("copy");
    }
}

/// The way the running code drops: whether it forgets, and whether it is on
/// the system stack, where what would run the program's code is moved aside.
fn mode() -> i64 {
    i64::from(crate::value::forgetting()) * FORGETTING + i64::from(crate::sched::deferring()) * DEFERRING
}

/// Drops the value at `p` of type `t` and what it owns; the status says
/// whether a panic is leaving.
///
/// # Safety
/// `p` must hold a value of type `t`, which is dead afterwards.
pub(crate) unsafe fn drop(p: *mut u8, t: u32) -> u8 {
    if !known(t) {
        return abi::OK;
    }
    let types = crate::desc::base();
    let (t, mode) = (i64::from(t), mode());
    let mut ret = 0i64;
    // SAFETY: as for [`equal`].
    match unsafe { sisu_rt_value_drop(&mut ret, &types, &p, &t, &mode) } {
        abi::OK if ret == 0 => abi::OK,
        _ => abi::UNWINDING,
    }
}

/// Drops the fields the glue of the struct or enum at `p` leaves out, once
/// the glue has run (decision 218).
///
/// # Safety
/// `p` must hold a value of type `t` whose glue has run, dead afterwards.
pub(crate) unsafe fn drop_rest(p: *mut u8, t: u32) -> u8 {
    if !known(t) {
        return abi::OK;
    }
    let types = crate::desc::base();
    let (t, mode) = (i64::from(t), mode());
    let mut ret = 0i64;
    // SAFETY: as for [`equal`].
    match unsafe { sisu_rt_value_drop_rest(&mut ret, &types, &p, &t, &mode) } {
        abi::OK if ret == 0 => abi::OK,
        _ => abi::UNWINDING,
    }
}

/// One more count for every capability handle the value at `p` holds.
///
/// # Safety
/// `p` must hold a value of type `t`.
pub(crate) unsafe fn retain(p: *mut u8, t: u32) {
    if !known(t) {
        return;
    }
    let types = crate::desc::base();
    let t = i64::from(t);
    let mut ret = 0u64;
    // SAFETY: as for [`equal`].
    if unsafe { sisu_rt_value_retain(&mut ret, &types, &p, &t) } != abi::OK {
        failed("count of handles");
    }
}

/// Writes at `dst` a new `String` of the digits of `mag` in `base` (2, 8,
/// 10 or 16), capitals with `upper`: `fmt.digits` and `fmt.wide_digits`.
///
/// # Safety
/// `dst` must have room for a `String`.
pub(crate) unsafe fn digits(dst: *mut u8, mag: u128, base: i64, upper: bool) {
    let (low, high) = (mag as u64 as i64, (mag >> 64) as u64 as i64);
    let upper = u64::from(upper);
    let mut ret = 0u64;
    // SAFETY: as for [`equal`].
    if unsafe { sisu_rt_int_digits(&mut ret, &dst, &low, &high, &base, &upper) } != abi::OK {
        failed("digits");
    }
}

/// The forms of a float's text (`floats.sisu`): the shortest of an `f64`
/// and of an `f32`'s bits, as `Debug` writes them, and of an `f64` the fixed
/// form and the form with an exponent, `precision` digits after the point.
pub(crate) const SHORTEST: i64 = 0;
pub(crate) const SHORTEST32: i64 = 1;
pub(crate) const FIXED: i64 = 2;
pub(crate) const EXPONENT: i64 = 3;

/// The text of the float whose bits are `bits` in the form `mode` (the
/// runtime in Sisu's `floats.write`, decision 358).
pub(crate) fn float_text(bits: u64, mode: i64, precision: usize) -> String {
    let mut text = mem::Text::empty();
    let dst = (&raw mut text).cast::<u8>();
    let bits = bits as i64;
    let precision = i64::try_from(precision).unwrap_or(i64::MAX);
    let mut ret = 0u64;
    // SAFETY: as for [`equal`]; `dst` has room for a `String`.
    if unsafe { sisu_rt_float_digits(&mut ret, &dst, &bits, &mode, &precision) } != abi::OK {
        failed("float digits");
    }
    // SAFETY: the export wrote a `String` there, which nothing else holds.
    let shown = String::from_utf8_lossy(unsafe { text.bytes() }).into_owned();
    mem::release(text.ptr);
    shown
}

/// Where `part` first occurs in `text`, by Two-Way (the runtime in Sisu's
/// `search.find`); an empty part occurs at 0.
pub(crate) fn find(text: &[u8], part: &[u8]) -> Option<usize> {
    let (t, n) = (text.as_ptr(), text.len() as i64);
    let (p, m) = (part.as_ptr(), part.len() as i64);
    let mut ret = -1i64;
    // SAFETY: as for [`equal`]; the slices are live for the call.
    if unsafe { sisu_rt_find(&mut ret, &t, &n, &p, &m) } != abi::OK {
        failed("text search");
    }
    usize::try_from(ret).ok()
}

/// Whether `text` starts with `part`, or ends with it (`at_end`).
pub(crate) fn has_edge(text: &[u8], part: &[u8], at_end: bool) -> bool {
    let (t, n) = (text.as_ptr(), text.len() as i64);
    let (p, m) = (part.as_ptr(), part.len() as i64);
    let at_end = u64::from(at_end);
    let mut ret = 0u64;
    // SAFETY: as for [`find`].
    if unsafe { sisu_rt_has_edge(&mut ret, &t, &n, &p, &m, &at_end) } != abi::OK {
        failed("text search");
    }
    ret != 0
}

// ------------------------------------------------- what the runtime in Sisu calls

/// Runs the drop glue at index `glue` of the function table on the value at
/// `p`.
///
/// # Safety
/// `p` must hold a value of the type whose glue it is.
#[unsafe(no_mangle)]
unsafe extern "C" fn sisu_rt_glue(glue: i64, p: *mut u8) -> u8 {
    // SAFETY: as the caller guarantees.
    unsafe { run_glue(glue as u32, p) }
}

/// Runs the drop glue at index `glue` of the function table on the value at
/// `p`, on the running stack.
///
/// # Safety
/// `p` must hold a value of the type whose glue it is.
pub(crate) unsafe fn run_glue(glue: u32, p: *mut u8) -> u8 {
    match rt::function(glue) {
        Some(f) => {
            let mut unit = 0u64;
            // SAFETY: the glue takes the value's address.
            unsafe { f(&mut unit as *mut u64 as *mut u8, p) }
        }
        None => rt::internal("drop glue missing from the function table"),
    }
}

/// Moves the value at `p` of type `t` aside, to be dropped on the task's
/// stack: the drop runs on the system stack, and the value's would run Sisu
/// code.
///
/// # Safety
/// `p` must hold a value of type `t`, which is moved out.
#[unsafe(no_mangle)]
unsafe extern "C" fn sisu_rt_defer(p: *mut u8, t: i64) -> u8 {
    // SAFETY: as the caller guarantees.
    unsafe { crate::sched::defer(p, t as u32) }
}

/// Drops the value at `p` of a type whose objects the runtime in Rust keeps.
///
/// # Safety
/// `p` must hold a value of type `t`, which is dead afterwards.
#[unsafe(no_mangle)]
unsafe extern "C" fn sisu_rt_object_drop(p: *mut u8, t: i64) -> u8 {
    // SAFETY: as the caller guarantees.
    unsafe { object_drop(p, t as u32) }
}

/// Drops the value at `p` of a type whose objects the runtime in Rust keeps.
///
/// # Safety
/// `p` must hold a value of type `t`, which is dead afterwards.
pub(crate) unsafe fn object_drop(p: *mut u8, t: u32) -> u8 {
    let forgetting = crate::value::forgetting();
    // SAFETY: as the caller guarantees: the word is the object's address or
    // number.
    unsafe {
        let w = mem::word(p, 0);
        match &ty(t).kind {
            Kind::Iter(_) => crate::iter::drop_iter(w as *mut u8),
            Kind::Dict(..) | Kind::Set(_) => crate::dict::drop_table(w as *mut crate::dict::Table),
            // A guard's drop checks its value's invariant, Sisu code: on
            // the system stack the guard is moved aside.
            Kind::Guard(..) if crate::sched::deferring() && !forgetting => crate::sched::defer(p, t),
            Kind::Guard(..) | Kind::ReadGuard(_) if forgetting => abi::OK,
            Kind::Guard(_, check) => crate::tasks::unlock(w as *mut u8, *check),
            Kind::ReadGuard(_) => crate::tasks::read_unlock(w as *mut u8),
            Kind::Sender(_) => crate::tasks::drop_channel_end(w as u32, false),
            Kind::Receiver(_) => crate::tasks::drop_channel_end(w as u32, true),
            Kind::Shared(v) => crate::cells::drop_shared(w as *mut u8, *v),
            Kind::Weak(_) => crate::cells::drop_weak(w as *mut u8),
            Kind::Arena(_) => crate::cells::drop_arena(p),
            // A copy of a capability handle, or of a `Cap`: one count fewer
            // (decision 237).
            Kind::Opaque(n) if crate::value::counted(n) => crate::caps::release_cell(w as *mut u8),
            _ => abi::OK,
        }
    }
}

/// Copies the value at `src` of a type whose objects the runtime in Rust
/// keeps to `dst`: a dict's or a set's table copied deeply, another end of
/// the same channel, another owner or weak handle of the same `Shared`
/// cell, another arena, another copy of a capability handle.
///
/// # Safety
/// `src` must hold a value of type `t`, and `dst` have room for one.
#[unsafe(no_mangle)]
unsafe extern "C" fn sisu_rt_object_copy(dst: *mut u8, src: *mut u8, t: i64) {
    let entry = ty(t as u32);
    // SAFETY: as the caller guarantees.
    unsafe {
        match &entry.kind {
            Kind::Dict(..) | Kind::Set(_) => {
                let table = crate::dict::table(src);
                mem::set_word(dst, 0, crate::dict::into_word(crate::dict::copy_table(table)));
            }
            Kind::Sender(_) | Kind::Receiver(_) => {
                mem::copy(dst, src, 8);
                crate::tasks::copy_channel_end(
                    mem::word(src, 0) as u32,
                    matches!(entry.kind, Kind::Receiver(_)),
                );
            }
            Kind::Shared(_) | Kind::Weak(_) => {
                mem::copy(dst, src, 8);
                crate::cells::copy_handle(mem::word(src, 0) as *mut u8, matches!(entry.kind, Kind::Weak(_)));
            }
            Kind::Arena(_) => crate::cells::copy_arena(dst, src),
            Kind::Opaque(n) if crate::value::counted(n) => {
                mem::copy(dst, src, 8);
                crate::caps::retain_cell(mem::word(src, 0) as *mut u8);
            }
            _ => mem::copy(dst, src, entry.size as usize),
        }
    }
}

/// One more count for the capability handle at `p` (decision 237).
///
/// # Safety
/// `p` must hold a handle of type `t`.
#[unsafe(no_mangle)]
unsafe extern "C" fn sisu_rt_object_retain(p: *mut u8, _t: i64) {
    // SAFETY: a handle's word is the address of its cell.
    unsafe { crate::caps::retain_cell(mem::word(p, 0) as *mut u8) }
}

/// Whether the dicts or the sets at `a` and `b` hold equal entries: the
/// same number, each key of one in the other, with an equal value.
///
/// # Safety
/// `a` and `b` must hold dicts or sets of types `ta` and `tb`.
#[unsafe(no_mangle)]
unsafe extern "C" fn sisu_rt_object_eq(a: *mut u8, _ta: i64, b: *mut u8, _tb: i64) -> bool {
    // SAFETY: as the caller guarantees.
    unsafe {
        let (x, y) = (crate::dict::table(a), crate::dict::table(b));
        if x.len() != y.len() {
            return false;
        }
        let dict = !matches!(ty(x.val).kind, Kind::Unit);
        (0..x.len()).all(|i| match y.position(x.key_at(i), x.key) {
            Some(j) => !dict || equal(x.val_at(i), x.val, y.val_at(j), y.val),
            None => false,
        })
    }
}

/// The error value at `p` is a copy: its return trace is held once more (a
/// trace never changes in place).
///
/// # Safety
/// `p` must hold an error value.
#[unsafe(no_mangle)]
unsafe extern "C" fn sisu_rt_trace_retain(p: *mut u8) {
    // SAFETY: an error value's second word is its trace.
    unsafe { crate::intrinsics::retain_trace(mem::word(p, abi::ERROR_TRACE)) }
}
