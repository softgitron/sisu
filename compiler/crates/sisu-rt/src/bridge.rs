//! The boundary with the runtime in Sisu (`compiler/rt`, decision 350): the
//! structural operations the runtime in Rust calls there (equality, the
//! structural hash, `.copy()`, the drop, the counts of a copy's capability
//! handles, and the `Display` and `Debug` texts, decision 383), which walk
//! a value by the type table's words (`desc.rs`), and the functions that
//! code calls back here, for what the runtime in Rust keeps as objects of
//! its own (iterators, channels, the guards of mutexes, `Shared` cells,
//! arenas, the cells of capability handles, return traces, tasks) and for
//! the program's drop glue; and the tables of dicts and sets (decision
//! 422), which the runtime in Sisu makes, looks keys up in, grows and
//! shrinks.
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
    fn sisu_rt_value_display(
        ret: *mut u64,
        types: *const *const u64,
        out: *const *mut u8,
        p: *const *mut u8,
        t: *const i64,
    ) -> u8;
    fn sisu_rt_value_debug(
        ret: *mut u64,
        types: *const *const u64,
        out: *const *mut u8,
        p: *const *mut u8,
        t: *const i64,
    ) -> u8;
    fn sisu_rt_list_grow(ret: *mut u64, l: *const *mut u8, extra: *const i64, size: *const i64) -> u8;
    fn sisu_rt_list_pop(
        ret: *mut u64,
        types: *const *const u64,
        l: *const *mut u8,
        e: *const i64,
        out: *const *mut u8,
    ) -> u8;
    fn sisu_rt_list_insert(
        ret: *mut u64,
        l: *const *mut u8,
        k: *const i64,
        value: *const *mut u8,
        size: *const i64,
    ) -> u8;
    fn sisu_rt_list_remove(
        ret: *mut u64,
        l: *const *mut u8,
        k: *const i64,
        out: *const *mut u8,
        size: *const i64,
    ) -> u8;
    fn sisu_rt_list_extend(ret: *mut u64, l: *const *mut u8, other: *const *mut u8, size: *const i64) -> u8;
    fn sisu_rt_list_reverse(ret: *mut u64, l: *const *mut u8, size: *const i64) -> u8;
    fn sisu_rt_list_swap(
        ret: *mut u64,
        l: *const *mut u8,
        i: *const i64,
        j: *const i64,
        size: *const i64,
    ) -> u8;
    fn sisu_rt_list_copies(
        ret: *mut u64,
        types: *const *const u64,
        dst: *const *mut u8,
        l: *const *mut u8,
        e: *const i64,
        lo: *const i64,
        hi: *const i64,
    ) -> u8;
    fn sisu_rt_list_contains(
        ret: *mut u64,
        types: *const *const u64,
        l: *const *mut u8,
        e: *const i64,
        x: *const *mut u8,
        tx: *const i64,
    ) -> u8;
    fn sisu_rt_list_join(
        ret: *mut u64,
        types: *const *const u64,
        out: *const *mut u8,
        l: *const *mut u8,
        e: *const i64,
        sep: *const *const u8,
        n: *const i64,
    ) -> u8;
    fn sisu_rt_list_sort(ret: *mut u64, types: *const *const u64, l: *const *mut u8, e: *const i64) -> u8;
    fn sisu_rt_table_new(ret: *mut *mut u8, types: *const *const u64, key: *const i64, val: *const i64)
    -> u8;
    fn sisu_rt_table_lookup(
        ret: *mut i64,
        types: *const *const u64,
        tb: *const *mut u8,
        k: *const *mut u8,
        kt: *const i64,
    ) -> u8;
    fn sisu_rt_table_push(
        ret: *mut i64,
        types: *const *const u64,
        tb: *const *mut u8,
        k: *const *mut u8,
    ) -> u8;
    fn sisu_rt_table_remove(ret: *mut u64, tb: *const *mut u8, i: *const i64, out: *const *mut u8) -> u8;
    fn sisu_rt_iter_next(
        ret: *mut i64,
        types: *const *const u64,
        it: *const *mut u8,
        out: *const *mut u8,
        mode: *const i64,
    ) -> u8;
    fn sisu_rt_iter_drop(ret: *mut i64, types: *const *const u64, it: *const *mut u8, mode: *const i64)
    -> u8;
    fn sisu_rt_iter_iterable(
        ret: *mut *mut u8,
        types: *const *const u64,
        p: *const *mut u8,
        arg: *const i64,
        item: *const i64,
    ) -> u8;
    fn sisu_rt_iter_collect(
        ret: *mut i64,
        types: *const *const u64,
        it: *const *mut u8,
        item: *const i64,
        out: *const *mut u8,
        mode: *const i64,
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

/// Appends the `Display` text (`display`) or the `Debug` text of the value
/// at `p` to `out` (the runtime in Sisu's `show.sisu`, decision 383).
///
/// # Safety
/// `p` must hold a value of type `t`.
pub(crate) unsafe fn show(out: &mut Vec<u8>, p: *mut u8, t: u32, display: bool) {
    if !known(t) {
        out.extend_from_slice(b"()");
        return;
    }
    let types = crate::desc::base();
    let mut text = mem::Text::empty();
    let dst = (&raw mut text).cast::<u8>();
    let t = i64::from(t);
    let mut ret = 0u64;
    // SAFETY: as for [`equal`]; `dst` holds a `String`.
    let status = unsafe {
        if display {
            sisu_rt_value_display(&mut ret, &types, &dst, &p, &t)
        } else {
            sisu_rt_value_debug(&mut ret, &types, &dst, &p, &t)
        }
    };
    if status != abi::OK {
        failed("text of a value");
    }
    // SAFETY: the export appended to the `String`, which nothing else holds.
    out.extend_from_slice(unsafe { text.bytes() });
    mem::release(text.ptr);
}

// ------------------------------------------------------------------- lists

/// Room in the list at `l` for `extra` more elements of `size` bytes (the
/// runtime in Sisu's `list.reserve`, decision 385).
///
/// # Safety
/// `l` must hold a list of elements of `size` bytes.
pub(crate) unsafe fn list_grow(l: *mut u8, extra: u64, size: u32) {
    let (extra, size) = (extra as i64, i64::from(size));
    let mut ret = 0u64;
    // SAFETY: as for [`equal`].
    if unsafe { sisu_rt_list_grow(&mut ret, &l, &extra, &size) } != abi::OK {
        failed("list's growth");
    }
}

/// The last element of the list at `l`, of type `e`, moved out into the
/// option at `out`.
///
/// # Safety
/// `l` must hold a list of elements of type `e`, and `out` have room for
/// an option of one.
pub(crate) unsafe fn list_pop(l: *mut u8, e: u32, out: *mut u8) {
    let types = crate::desc::base();
    let e = i64::from(e);
    let mut ret = 0u64;
    // SAFETY: as for [`equal`].
    if unsafe { sisu_rt_list_pop(&mut ret, &types, &l, &e, &out) } != abi::OK {
        failed("list's pop");
    }
}

/// The element at `value` moved into the list at `l` at index `k`, which
/// the caller checked.
///
/// # Safety
/// `l` must hold a list of elements of `size` bytes, `value` one, which
/// moves in, and `k` be at most its length.
pub(crate) unsafe fn list_insert(l: *mut u8, k: u64, value: *mut u8, size: u32) {
    let (k, size) = (k as i64, i64::from(size));
    let mut ret = 0u64;
    // SAFETY: as for [`equal`].
    if unsafe { sisu_rt_list_insert(&mut ret, &l, &k, &value, &size) } != abi::OK {
        failed("list's insert");
    }
}

/// Element `k` of the list at `l`, which the caller checked, moved out to
/// `out`.
///
/// # Safety
/// `l` must hold a list of elements of `size` bytes with an element `k`,
/// and `out` have room for one.
pub(crate) unsafe fn list_remove(l: *mut u8, k: u64, out: *mut u8, size: u32) {
    let (k, size) = (k as i64, i64::from(size));
    let mut ret = 0u64;
    // SAFETY: as for [`equal`].
    if unsafe { sisu_rt_list_remove(&mut ret, &l, &k, &out, &size) } != abi::OK {
        failed("list's remove");
    }
}

/// The elements of the list at `other` moved to the end of the one at `l`.
///
/// # Safety
/// `l` and `other` must hold lists of elements of `size` bytes, and
/// `other` not be used again.
pub(crate) unsafe fn list_extend(l: *mut u8, other: *mut u8, size: u32) {
    let size = i64::from(size);
    let mut ret = 0u64;
    // SAFETY: as for [`equal`].
    if unsafe { sisu_rt_list_extend(&mut ret, &l, &other, &size) } != abi::OK {
        failed("list's extend");
    }
}

/// The elements of the list at `l` in the reverse order.
///
/// # Safety
/// `l` must hold a list of elements of `size` bytes.
pub(crate) unsafe fn list_reverse(l: *mut u8, size: u32) {
    let size = i64::from(size);
    let mut ret = 0u64;
    // SAFETY: as for [`equal`].
    if unsafe { sisu_rt_list_reverse(&mut ret, &l, &size) } != abi::OK {
        failed("list's reverse");
    }
}

/// Elements `i` and `j` of the list at `l`, which the caller checked,
/// swapped.
///
/// # Safety
/// `l` must hold a list of elements of `size` bytes with both indices.
pub(crate) unsafe fn list_swap(l: *mut u8, i: u64, j: u64, size: u32) {
    let (i, j, size) = (i as i64, j as i64, i64::from(size));
    let mut ret = 0u64;
    // SAFETY: as for [`equal`].
    if unsafe { sisu_rt_list_swap(&mut ret, &l, &i, &j, &size) } != abi::OK {
        failed("list's swap");
    }
}

/// Writes at `dst` a new list of copies of the elements from `lo` to `hi`
/// of the list at `l`, of type `e`.
///
/// # Safety
/// `l` must hold a list of elements of type `e`, `dst` have room for a
/// list, and `lo <= hi <=` its length.
pub(crate) unsafe fn list_copies(dst: *mut u8, l: *mut u8, e: u32, lo: u64, hi: u64) {
    let types = crate::desc::base();
    let (e, lo, hi) = (i64::from(e), lo as i64, hi as i64);
    let mut ret = 0u64;
    // SAFETY: as for [`equal`].
    if unsafe { sisu_rt_list_copies(&mut ret, &types, &dst, &l, &e, &lo, &hi) } != abi::OK {
        failed("list's slice");
    }
}

/// Whether the list at `l`, of type `e`, holds an element equal to the
/// value at `x` of type `tx`.
///
/// # Safety
/// `l` must hold a list of elements of type `e`, and `x` a value of type
/// `tx`.
pub(crate) unsafe fn list_contains(l: *mut u8, e: u32, x: *mut u8, tx: u32) -> bool {
    let types = crate::desc::base();
    let (e, tx) = (i64::from(e), i64::from(tx));
    let mut ret = 0u64;
    // SAFETY: as for [`equal`].
    if unsafe { sisu_rt_list_contains(&mut ret, &types, &l, &e, &x, &tx) } != abi::OK {
        failed("list's `in`");
    }
    ret != 0
}

/// Writes at `dst` a new `String` of each element's `Display` text of the
/// list at `l`, of type `e`, `sep` between them.
///
/// # Safety
/// `l` must hold a list of elements of type `e`, and `dst` have room for a
/// `String`.
pub(crate) unsafe fn list_join(dst: *mut u8, l: *mut u8, e: u32, sep: &[u8]) {
    let types = crate::desc::base();
    // SAFETY: `dst` has room for a `String`.
    unsafe { (dst as *mut mem::Text).write(mem::Text::empty()) };
    let (e, b, n) = (i64::from(e), sep.as_ptr(), sep.len() as i64);
    let mut ret = 0u64;
    // SAFETY: as for [`equal`]; `sep` lives for the call.
    if unsafe { sisu_rt_list_join(&mut ret, &types, &dst, &l, &e, &b, &n) } != abi::OK {
        failed("list's join");
    }
}

/// Sorts the list at `l`, of type `e`, stably by its elements' order;
/// `false`, leaving it as it was, when two have none.
///
/// # Safety
/// `l` must hold a list of elements of type `e`.
pub(crate) unsafe fn list_sort(l: *mut u8, e: u32) -> bool {
    let types = crate::desc::base();
    let e = i64::from(e);
    let mut ret = 0u64;
    // SAFETY: as for [`equal`].
    if unsafe { sisu_rt_list_sort(&mut ret, &types, &l, &e) } != abi::OK {
        failed("list's sort");
    }
    ret != 0
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
/// keeps to `dst`: another end of the same channel, another owner or weak
/// handle of the same `Shared` cell, another arena, another copy of a
/// capability handle.
///
/// # Safety
/// `src` must hold a value of type `t`, and `dst` have room for one.
#[unsafe(no_mangle)]
unsafe extern "C" fn sisu_rt_object_copy(dst: *mut u8, src: *mut u8, t: i64) {
    let entry = ty(t as u32);
    // SAFETY: as the caller guarantees.
    unsafe {
        match &entry.kind {
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

/// Whether the `Shared` cell at `cell` still holds its value.
///
/// # Safety
/// `cell` must be a `Shared` cell.
#[unsafe(no_mangle)]
unsafe extern "C" fn sisu_rt_shared_alive(cell: *mut u8) -> bool {
    // SAFETY: as the caller guarantees.
    unsafe { crate::cells::alive(cell) }
}

/// Appends to the `String` at `out` the `Debug` text of a task's handle,
/// `<task PATH>`.
///
/// # Safety
/// `out` must hold a `String`.
#[unsafe(no_mangle)]
unsafe extern "C" fn sisu_rt_show_task(out: *mut u8, task: i64) {
    let shown = format!("<task {}>", crate::tasks::task_path(task as u32));
    // SAFETY: as the caller guarantees.
    unsafe { mem::at::<mem::Text>(out).push(shown.as_bytes()) }
}

/// Appends to the `String` at `out` the `Debug` text of the arena at `p`.
///
/// # Safety
/// `out` must hold a `String`, and `p` an `Arena`.
#[unsafe(no_mangle)]
unsafe extern "C" fn sisu_rt_show_arena(out: *mut u8, p: *mut u8) {
    let mut shown = Vec::new();
    // SAFETY: as the caller guarantees.
    unsafe {
        crate::cells::debug_arena(&mut shown, p);
        mem::at::<mem::Text>(out).push(&shown);
    }
}

/// Appends to the `String` at `out` the RFC 3339 text of a time in
/// nanoseconds since the epoch, as a `DateTime`'s `Debug` writes it.
///
/// # Safety
/// `out` must hold a `String`.
#[unsafe(no_mangle)]
unsafe extern "C" fn sisu_rt_show_datetime(out: *mut u8, nanos: i64) {
    let shown = crate::value::datetime_text(nanos);
    // SAFETY: as the caller guarantees.
    unsafe { mem::at::<mem::Text>(out).push(shown.as_bytes()) }
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

// ------------------------------------------------------------------ tables

/// A new table of a dict's keys of type `key` and values of type `val`, or
/// of a set's elements (`val` -1), with no entries (the runtime in Sisu's
/// `table.new`, decision 422): its address.
pub(crate) fn table_new(key: i64, val: i64) -> *mut u8 {
    let types = crate::desc::base();
    let mut ret = std::ptr::null_mut();
    // SAFETY: as for [`equal`].
    if unsafe { sisu_rt_table_new(&mut ret, &types, &key, &val) } != abi::OK {
        failed("new table");
    }
    ret
}

/// The position of the entry of the table at `tb` whose key is the key at
/// `k` of type `kt`, or -1.
///
/// # Safety
/// `tb` must be a table, and `k` hold a value of type `kt`.
pub(crate) unsafe fn table_lookup(tb: *mut u8, k: *mut u8, kt: u32) -> i64 {
    let types = crate::desc::base();
    let kt = i64::from(kt);
    let mut ret = 0i64;
    // SAFETY: as for [`equal`].
    if unsafe { sisu_rt_table_lookup(&mut ret, &types, &tb, &k, &kt) } != abi::OK {
        failed("table's lookup");
    }
    ret
}

/// Appends to the table at `tb` an entry whose key moves in from `k`: its
/// position, where the caller writes its value.
///
/// # Safety
/// `tb` must be a table without the key, and `k` hold one, which moves in.
pub(crate) unsafe fn table_push(tb: *mut u8, k: *mut u8) -> u64 {
    let types = crate::desc::base();
    let mut ret = 0i64;
    // SAFETY: as for [`equal`].
    if unsafe { sisu_rt_table_push(&mut ret, &types, &tb, &k) } != abi::OK {
        failed("table's push");
    }
    ret as u64
}

/// Moves entry `i` of the table at `tb` out to `out`, key then value.
///
/// # Safety
/// `tb` must be a table of more than `i` entries, and `out` have room for
/// a key and a value.
pub(crate) unsafe fn table_remove(tb: *mut u8, i: u64, out: *mut u8) {
    let i = i as i64;
    let mut ret = 0u64;
    // SAFETY: as for [`equal`].
    if unsafe { sisu_rt_table_remove(&mut ret, &tb, &i, &out) } != abi::OK {
        failed("table's remove");
    }
}

// --------------------------------------------------------------- iterators

/// A drop's status as the runtime in Sisu gives it, 1 when a panic is
/// leaving, as the runtime in Rust's.
fn status_of(ret: i64) -> u8 {
    if ret == 0 { abi::OK } else { abi::UNWINDING }
}

/// Writes the next item of the iterator at `it` as an option at `out`
/// (the runtime in Sisu's `iter.next`, decision 423): its status.
///
/// # Safety
/// `it` must be an iterator's object, and `out` have room for a tag word
/// and an item.
pub(crate) unsafe fn iter_next(it: *mut u8, out: *mut u8) -> u8 {
    let types = crate::desc::base();
    let mode = mode();
    let mut ret = 0i64;
    // SAFETY: as for [`equal`].
    if unsafe { sisu_rt_iter_next(&mut ret, &types, &it, &out, &mode) } != abi::OK {
        failed("iterator's next item");
    }
    status_of(ret)
}

/// Drops the iterator at `it`, or nothing when it is null, and what it
/// owns: its status.
///
/// # Safety
/// `it` must be an iterator's object, or null, dead afterwards.
pub(crate) unsafe fn iter_drop(it: *mut u8) -> u8 {
    let types = crate::desc::base();
    let mode = mode();
    let mut ret = 0i64;
    // SAFETY: as for [`equal`].
    if unsafe { sisu_rt_iter_drop(&mut ret, &types, &it, &mode) } != abi::OK {
        failed("iterator's drop");
    }
    status_of(ret)
}

/// The iterator of the items of an `Iterable[T]` argument at `p`, of type
/// `arg`, whose items are `item` when the call says.
///
/// # Safety
/// `p` must hold a value of type `arg`; an owned one is moved in.
pub(crate) unsafe fn iter_iterable(p: *mut u8, arg: u32, item: Option<u32>) -> *mut u8 {
    let types = crate::desc::base();
    let (arg, item) = (i64::from(arg), item.filter(|&t| known(t)).map_or(-1, i64::from));
    let mut ret = std::ptr::null_mut();
    // SAFETY: as for [`equal`].
    if unsafe { sisu_rt_iter_iterable(&mut ret, &types, &p, &arg, &item) } != abi::OK {
        failed("iterator of an argument");
    }
    ret
}

/// Every item the iterator at `it` has left, of type `item` when the call
/// says, moved into a new list at `out`; the iterator dropped after. Its
/// status: when a panic stopped it, the list is empty.
///
/// # Safety
/// `it` must be an iterator's object, dead afterwards, and `out` have room
/// for a list.
pub(crate) unsafe fn iter_collect(it: *mut u8, item: Option<u32>, out: *mut u8) -> u8 {
    let types = crate::desc::base();
    let (item, mode) = (item.filter(|&t| known(t)).map_or(-1, i64::from), mode());
    let mut ret = 0i64;
    // SAFETY: as for [`equal`].
    if unsafe { sisu_rt_iter_collect(&mut ret, &types, &it, &item, &out, &mode) } != abi::OK {
        failed("iterator's items");
    }
    status_of(ret)
}

/// The next item of the generator whose task is `task`, `size` bytes,
/// written at `out`, for the runtime in Sisu's iterators: 0 for an item, 1
/// for its end, else 2 plus the status of the panic it ended with, raised
/// (`tasks::gen_item`).
///
/// # Safety
/// `out` must have room for an item of the generator's.
#[unsafe(no_mangle)]
unsafe extern "C" fn sisu_rt_gen_item(task: i64, out: *mut u8, size: i64) -> i64 {
    match crate::tasks::gen_item(task as u32) {
        Ok(Some(v)) => {
            // SAFETY: as the caller guarantees.
            unsafe { mem::copy(out, v.as_ptr().cast::<u8>(), size as usize) };
            0
        }
        Ok(None) => 1,
        Err(status) => 2 + i64::from(status),
    }
}

/// The value the channel `chan` holds now, `size` bytes, written at `out`
/// and taken: 0 for a value, 1 for its end, else 2 plus the status of the
/// panic raised for a receiver that would wait inside another iterator
/// (port-debt.md).
///
/// # Safety
/// `out` must have room for a value of the channel's.
#[unsafe(no_mangle)]
unsafe extern "C" fn sisu_rt_recv_item(chan: i64, out: *mut u8, size: i64) -> i64 {
    match crate::sched::with(|s| s.recv_now(chan as u32)) {
        Some(Ok(v)) => {
            // SAFETY: as the caller guarantees.
            unsafe { mem::copy(out, v.as_ptr().cast::<u8>(), size as usize) };
            0
        }
        Some(Err(())) => 1,
        None => 2 + i64::from(rt::raise_msg(sisu_diag::msg::PANIC_NESTED_RECV, &[])),
    }
}

/// Closes the generator whose task is `task`, dropped before its end
/// (decision 236): the status of what it drops.
#[unsafe(no_mangle)]
extern "C" fn sisu_rt_gen_abandon(task: i64) -> u8 {
    crate::tasks::abandon(task as u32)
}

/// Raises the panic of `zip` given two lengths: its status.
#[unsafe(no_mangle)]
extern "C" fn sisu_rt_zip_lengths() -> u8 {
    rt::raise_msg(sisu_diag::msg::PANIC_ZIP_LENGTHS, &[])
}
