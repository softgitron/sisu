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
    fn sisu_rt_time_text(
        ret: *mut u64,
        out: *const *mut u8,
        nanos: *const i64,
        pat: *const *const u8,
        n: *const i64,
    ) -> u8;
    fn sisu_rt_quote(ret: *mut u64, out: *const *mut u8, b: *const *const u8, n: *const i64) -> u8;
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
    fn sisu_rt_list_reverse(ret: *mut u64, l: *const *mut u8, size: *const i64) -> u8;
    fn sisu_rt_table_literal(
        ret: *mut i64,
        types: *const *const u64,
        t: *const i64,
        items: *const *mut u8,
        n: *const i64,
        out: *const *mut u8,
    ) -> u8;
    fn sisu_rt_table_inline_find(
        ret: *mut u64,
        types: *const *const u64,
        p: *const *mut u8,
        key: *const *mut u8,
        kt: *const i64,
        want_value: *const u64,
    ) -> u8;
    fn sisu_rt_contains(
        ret: *mut u64,
        types: *const *const u64,
        c: *const *mut u8,
        ct: *const i64,
        x: *const *mut u8,
        xt: *const i64,
    ) -> u8;
    fn sisu_rt_list_sort(ret: *mut u64, types: *const *const u64, l: *const *mut u8, e: *const i64) -> u8;
    fn sisu_rt_collection(
        ret: *mut i64,
        types: *const *const u64,
        desc: *const *const u32,
        result: *const *mut u8,
        args: *const *const *mut u8,
    ) -> u8;
    fn sisu_rt_table_slot(
        ret: *mut i64,
        types: *const *const u64,
        d: *const *mut u8,
        key: *const *mut u8,
        kt: *const i64,
        mode: *const i64,
    ) -> u8;
    fn sisu_rt_table_len(ret: *mut i64, tb: *const *mut u8) -> u8;
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
    fn sisu_rt_iter_range(ret: *mut *mut u8, cur: *const i64, stop: *const i64, step: *const i64) -> u8;
    fn sisu_rt_iter_list_ref(ret: *mut *mut u8, xs: *const *mut u8, size: *const i64) -> u8;
    fn sisu_rt_iter_items(
        ret: *mut *mut u8,
        types: *const *const u64,
        xs: *const *mut u8,
        elem: *const i64,
    ) -> u8;
    fn sisu_rt_iter_chars(ret: *mut *mut u8, b: *const *const u8, n: *const i64) -> u8;
    fn sisu_rt_iter_bytes(ret: *mut *mut u8, b: *const *const u8, n: *const i64) -> u8;
    fn sisu_rt_iter_owned_bytes(ret: *mut *mut u8, b: *const *const u8, n: *const i64) -> u8;
    fn sisu_rt_iter_enumerate(ret: *mut *mut u8, inner: *const *mut u8) -> u8;
    fn sisu_rt_iter_zip(
        ret: *mut *mut u8,
        a: *const *mut u8,
        b: *const *mut u8,
        strict: *const u64,
        ta: *const i64,
        tb: *const i64,
    ) -> u8;
    fn sisu_rt_iter_table_ref(ret: *mut *mut u8, tb: *const *mut u8, part: *const i64) -> u8;
    fn sisu_rt_iter_generator(ret: *mut *mut u8, task: *const i64, size: *const i64) -> u8;
    fn sisu_rt_iter_receiver(ret: *mut *mut u8, chan: *const i64, size: *const i64) -> u8;
    fn sisu_rt_iter_waits(ret: *mut i64, it: *const *mut u8, out: *const *mut u8) -> u8;
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

/// A `dict` or `set` of type `t` from a literal's `n` entries, moved in
/// from `items` (each the key's bytes, then the value's), its table's word
/// written at `out`: the runtime in Sisu's (decision 646). The status of
/// the drops of the keys given again.
///
/// # Safety
/// `items` must hold `n` entries of the type's key and value, and `out`
/// have room for a word.
pub(crate) unsafe fn table_literal(t: u32, items: *mut u8, n: u64, out: *mut u8) -> u8 {
    let types = crate::desc::base();
    let (t, n) = (i64::from(t), n as i64);
    let mut status = 0i64;
    // SAFETY: as the caller guarantees.
    if unsafe { sisu_rt_table_literal(&mut status, &types, &t, &items, &n, &out) } != abi::OK {
        failed("a table's literal");
    }
    status as u8
}

/// The generated code's inline lookup of the key at `key`, of type `kt`, in
/// the dict or set at `t` (decision 265): the value's address, for a
/// `dict.get` (`value`), else 1, when the key is there; else 0 (the
/// runtime in Sisu's, decision 646).
///
/// # Safety
/// `t` must hold a dict or a set, and `key` a value of type `kt`.
pub(crate) unsafe fn table_find(t: *mut u8, key: *mut u8, kt: u32, value: bool) -> u64 {
    let types = crate::desc::base();
    let (kt, value) = (i64::from(kt), u64::from(value));
    let mut ret = 0u64;
    // SAFETY: as the caller guarantees.
    if unsafe { sisu_rt_table_inline_find(&mut ret, &types, &t, &key, &kt, &value) } != abi::OK {
        failed("a table's lookup");
    }
    ret
}

/// `item in container`, the operator: whether the value at `c` of type
/// `ct`, behind any references, holds the value at `x` of type `xt` (the
/// runtime in Sisu's, decision 646).
///
/// # Safety
/// `c` must hold a value of type `ct`, and `x` one of type `xt`.
pub(crate) unsafe fn contains(c: *mut u8, ct: u32, x: *mut u8, xt: u32) -> bool {
    let types = crate::desc::base();
    let (ct, xt) = (i64::from(ct), i64::from(xt));
    let mut ret = 0u64;
    // SAFETY: as for [`equal`].
    if unsafe { sisu_rt_contains(&mut ret, &types, &c, &ct, &x, &xt) } != abi::OK {
        failed("`in`");
    }
    ret != 0
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

/// The text of the time `nanos` by the `strftime` directives of `pattern`,
/// or its RFC 3339 text without one (the runtime in Sisu's `time.sisu`,
/// decision 469).
pub(crate) fn time_text(nanos: i64, pattern: Option<&str>) -> String {
    let mut text = mem::Text::empty();
    let out = (&raw mut text).cast::<u8>();
    let (pat, n) = match pattern {
        Some(p) => (p.as_ptr(), p.len() as i64),
        None => (std::ptr::null(), -1),
    };
    let mut ret = 0u64;
    // SAFETY: as for [`equal`]; `out` holds a `String` and the pattern is
    // live for the call.
    if unsafe { sisu_rt_time_text(&mut ret, &out, &nanos, &pat, &n) } != abi::OK {
        failed("time text");
    }
    // SAFETY: the export appended to the `String`, which nothing else holds.
    let shown = String::from_utf8_lossy(unsafe { text.bytes() }).into_owned();
    mem::release(text.ptr);
    shown
}

/// `s` quoted as `Debug` writes a text (the runtime in Sisu's
/// `show.quote_str`, decision 469).
pub(crate) fn quote(s: &str) -> String {
    let mut text = mem::Text::empty();
    let out = (&raw mut text).cast::<u8>();
    let (b, n) = (s.as_ptr(), s.len() as i64);
    let mut ret = 0u64;
    // SAFETY: as for [`time_text`].
    if unsafe { sisu_rt_quote(&mut ret, &out, &b, &n) } != abi::OK {
        failed("quoting a text");
    }
    // SAFETY: as for [`time_text`].
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

/// The address of the value under the key at `key` (of type `kt`, behind
/// any references) in the dict at `d`, for `d{k}` in `mode`
/// (`abi::SLOT_READ` and its kin): a copy of a missing key appended for a
/// write, 0 for a drop and -1 for a read, which panics (the runtime in
/// Sisu's, decision 681).
///
/// # Safety
/// `d` must hold a dict and `key` a value of type `kt`.
pub(crate) unsafe fn table_slot(d: *mut u8, key: *mut u8, kt: u32, mode: u32) -> i64 {
    let types = crate::desc::base();
    let (kt, mode) = (i64::from(kt), i64::from(mode));
    let mut ret = 0i64;
    // SAFETY: as for [`equal`].
    if unsafe { sisu_rt_table_slot(&mut ret, &types, &d, &key, &kt, &mode) } != abi::OK {
        failed("`d{k}`'s slot");
    }
    ret
}

/// The parts of the table at `tb`'s header the runtime in Rust reads,
/// through the runtime in Sisu's exports (decision 506): its number of
/// entries.
///
/// # Safety
/// `tb` must be the address of a live table.
pub(crate) unsafe fn table_word(tb: *mut u8, part: TablePart) -> u64 {
    let mut ret = 0i64;
    // SAFETY: as for [`equal`].
    let status = unsafe {
        match part {
            TablePart::Len => sisu_rt_table_len(&mut ret, &tb),
        }
    };
    if status != abi::OK {
        failed("table's header");
    }
    ret as u64
}

/// What [`table_word`] reads.
#[derive(Clone, Copy)]
pub(crate) enum TablePart {
    Len,
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

/// A new iterator's object, made by the runtime in Sisu (decision 542),
/// whose layout is `iter.sisu`'s alone.
pub(crate) enum NewIter<'a> {
    Range(i64, i64, i64),
    /// A borrowed list's elements, of `size` bytes.
    ListRef(*mut u8, u32),
    /// The elements of a list it owns, of type `elem`.
    Items(&'a mut crate::mem::List, u32),
    Chars(*const u8, u64),
    Bytes(*const u8, u64),
    OwnedBytes(&'a [u8]),
    Enumerate(*mut u8),
    Zip(*mut u8, *mut u8, bool, u32, u32),
    /// A borrowed table's keys (0), values (1) or both (2).
    TableRef(*mut u8, u64),
    /// A generator's items, its task and their size.
    Generator(u32, u32),
    /// A receiver's values, its channel and their size.
    Receiver(u32, u32),
}

/// Makes a new iterator's object.
///
/// # Safety
/// What each kind names must be live: a list, a text that outlives the
/// iterator, an iterator's object it takes, a table.
pub(crate) unsafe fn iter_new(kind: NewIter<'_>) -> *mut u8 {
    let mut ret: *mut u8 = std::ptr::null_mut();
    let i = |v: u32| i64::from(v);
    // SAFETY: as for [`equal`]; each argument is in memory of its own.
    let status = unsafe {
        match kind {
            NewIter::Range(cur, stop, step) => sisu_rt_iter_range(&mut ret, &cur, &stop, &step),
            NewIter::ListRef(xs, size) => sisu_rt_iter_list_ref(&mut ret, &xs, &i(size)),
            NewIter::Items(list, elem) => {
                let types = crate::desc::base();
                let xs = (list as *mut crate::mem::List).cast::<u8>();
                sisu_rt_iter_items(&mut ret, &types, &xs, &i(elem))
            }
            NewIter::Chars(b, n) => sisu_rt_iter_chars(&mut ret, &b, &(n as i64)),
            NewIter::Bytes(b, n) => sisu_rt_iter_bytes(&mut ret, &b, &(n as i64)),
            NewIter::OwnedBytes(data) => {
                sisu_rt_iter_owned_bytes(&mut ret, &data.as_ptr(), &(data.len() as i64))
            }
            NewIter::Enumerate(inner) => sisu_rt_iter_enumerate(&mut ret, &inner),
            NewIter::Zip(a, b, strict, ta, tb) => {
                sisu_rt_iter_zip(&mut ret, &a, &b, &u64::from(strict), &i(ta), &i(tb))
            }
            NewIter::TableRef(tb, part) => sisu_rt_iter_table_ref(&mut ret, &tb, &(part as i64)),
            NewIter::Generator(task, size) => sisu_rt_iter_generator(&mut ret, &i(task), &i(size)),
            NewIter::Receiver(chan, size) => sisu_rt_iter_receiver(&mut ret, &i(chan), &i(size)),
        }
    };
    if status != abi::OK {
        failed("iterator's making");
    }
    ret
}

/// What `Iter.next` of the iterator at `it` may wait for: 1 for a
/// generator's item, 2 for a receiver's value, with its task or channel
/// and the size of an item; 0 when it does not wait.
///
/// # Safety
/// `it` must be an iterator's object.
pub(crate) unsafe fn iter_waits(it: *mut u8) -> (i64, u64, u64) {
    let mut ret = 0i64;
    let mut words = [0u64; 2];
    let out = words.as_mut_ptr().cast::<u8>();
    // SAFETY: as for [`equal`].
    if unsafe { sisu_rt_iter_waits(&mut ret, &it, &out) } != abi::OK {
        failed("iterator's waits");
    }
    (ret, words[0], words[1])
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
    let _ = size;
    // SAFETY: as the caller guarantees.
    match crate::sched::with(|s| unsafe { s.recv_now(chan as u32, out) }) {
        Some(Ok(())) => 0,
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

/// The runtime in Sisu found an index outside a list (decision 562): the
/// panic of `insert` when `insert`, else of an index out of bounds,
/// raised, and its status.
#[unsafe(no_mangle)]
extern "C" fn sisu_rt_list_bounds(insert: bool, index: i64, len: i64) -> u8 {
    let (index, len) = (index.to_string(), len.to_string());
    if insert {
        rt::raise_msg(sisu_diag::msg::PANIC_INSERT_OUT_OF_BOUNDS, &[("index", index), ("len", len)])
    } else {
        rt::raise_msg(sisu_diag::msg::PANIC_INDEX_OUT_OF_BOUNDS, &[("len", len), ("index", index)])
    }
}

/// Raises the panic of a key that a dict does not hold, the key at `key`
/// of type `key_ty` named in its `Debug` form: `d{k}`'s and
/// `dict.index_str`'s (decisions 681 and 784). Its status.
///
/// # Safety
/// `key` must hold a live value of type `key_ty`, behind any references.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn sisu_rt_missing_key(key: *mut u8, key_ty: i64) -> u8 {
    let mut shown = Vec::new();
    // SAFETY: as the caller guarantees.
    unsafe {
        let (p, kt) = crate::value::target(key, key_ty as u32);
        crate::value::debug(&mut shown, p, kt);
    }
    rt::raise_msg(sisu_diag::msg::PANIC_MISSING_KEY, &[("key", String::from_utf8_lossy(&shown).into_owned())])
}

/// Whether the intrinsic numbered `id` is a list's, a dict's or a set's,
/// whose call the runtime in Sisu decodes and does (decision 625).
pub(crate) fn collects(id: u32) -> bool {
    use abi::Intrinsic as I;
    matches!(
        abi::Intrinsic::from_id(id),
        Some(
            I::ListNew
                | I::ListWithCapacity
                | I::ListPush
                | I::ListPop
                | I::ListLast
                | I::ListFirst
                | I::ListGet
                | I::ListIsEmpty
                | I::ListInsert
                | I::ListRemove
                | I::ListExtend
                | I::ListReverse
                | I::ListSwap
                | I::ListClear
                | I::ListSlice
                | I::ListSliceMiddle
                | I::ListContains
                | I::ListIter
                | I::ListJoin
                | I::ListSort
                | I::DictNew
                | I::DictGet
                | I::DictContains
                | I::DictIsEmpty
                | I::DictInsert
                | I::DictRemove
                | I::DictEntry
                | I::DictKeys
                | I::DictValues
                | I::DictItems
                | I::EntryOrInsert
                | I::SetNew
                | I::SetAdd
                | I::SetRemove
                | I::SetContains
                | I::SetIsEmpty
                | I::SetIter
                | I::ListAsSlice
                | I::SliceToList
                | I::SliceAt
                | I::SliceGet
                | I::SliceIsEmpty
                | I::SliceSlice
                | I::SliceCut
                | I::SliceIter
                | I::DictGetStr
                | I::DictRemoveStr
                | I::DictIndexStr
                | I::SetContainsStr
                | I::SetRemoveStr
        )
    )
}

/// A call of a list's, a dict's or a set's intrinsic, which the runtime in
/// Sisu decodes from its descriptor and does (`dispatch.sisu`, decision
/// 625): its status.
///
/// # Safety
/// As `sisu_rt_intrinsic`, of an intrinsic [`collects`] names.
pub(crate) unsafe fn collection(desc: *const u32, ret: *mut u8, args: *const *mut u8) -> u8 {
    let types = crate::desc::base();
    let mut status = 0i64;
    // SAFETY: as the caller guarantees.
    if unsafe { sisu_rt_collection(&mut status, &types, &desc, &ret, &args) } != abi::OK {
        failed("intrinsic of a collection");
    }
    status as u8
}

/// A slice's bounds below 0 (decision 625): the panic raised, and its
/// status.
#[unsafe(no_mangle)]
extern "C" fn sisu_rt_slice_bounds(start: i64, end: i64) -> u8 {
    rt::raise_msg(
        sisu_diag::msg::PANIC_SLICE_BOUNDS,
        &[("start", start.to_string()), ("end", end.to_string())],
    )
}

/// A sort that met two elements without an order: the panic raised, and its
/// status.
#[unsafe(no_mangle)]
extern "C" fn sisu_rt_unordered() -> u8 {
    rt::raise_msg(sisu_diag::msg::PANIC_UNORDERED, &[])
}

/// The drop of the value at `p` of type `t`, as the runtime in Rust drops
/// one (`value::drop_value`): its status.
///
/// # Safety
/// `p` must hold a value of type `t`, dead afterwards.
#[unsafe(no_mangle)]
unsafe extern "C" fn sisu_rt_drop_value(p: *mut u8, t: i64) -> u8 {
    // SAFETY: as the caller guarantees.
    unsafe { crate::value::drop_value(p, t as u32) }
}

/// A view of the elements `lo` to `hi`, `size` bytes each, of the list at
/// `l`: a `*rest` binding (decision 214).
///
/// # Safety
/// `l` must hold a list of elements of `size` bytes, and `lo <= hi <= len`.
#[unsafe(no_mangle)]
unsafe extern "C" fn sisu_rt_list_view(l: *mut u8, size: i64, lo: i64, hi: i64) -> *mut u8 {
    // SAFETY: as the caller guarantees.
    unsafe { crate::views::view(l, size as u32, lo as u64, hi as u64) }
}

#[cfg(test)]
mod tests {
    /// The numbers `dispatch.sisu` gives the intrinsics it does are the
    /// runtime's, and they are the ones [`super::collects`] hands it
    /// (decision 625).
    #[test]
    fn the_collections_numbers_are_the_runtimes() {
        let text = include_str!("../../../rt/src/dispatch.sisu");
        let mut seen = Vec::new();
        let mut name = None;
        for line in text.lines() {
            if let Some(n) = line.strip_prefix("## `").and_then(|l| l.strip_suffix('`')) {
                name = Some(n.to_string());
            } else if let Some(rest) = line.strip_prefix("pub(package) const ")
                && let Some(n) = name.take()
            {
                let id: u32 = rest.rsplit(' ').next().and_then(|v| v.parse().ok()).expect("a number");
                let i = super::abi::Intrinsic::from_name(&n).expect("an intrinsic");
                assert_eq!(i as u32, id, "{n}");
                assert!(super::collects(id), "{n}");
                seen.push(id);
            }
        }
        let all = (0..super::abi::Intrinsic::ALL.len() as u32).filter(|&id| super::collects(id)).count();
        assert_eq!(seen.len(), all);
    }
}
