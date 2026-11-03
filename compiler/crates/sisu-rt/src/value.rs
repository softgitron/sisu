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

/// # Safety
/// `p` must hold an `f32`.
unsafe fn f32_at(p: *const u8) -> f32 {
    // SAFETY: an `f32` is the low half of its word.
    unsafe { (p as *const f32).read() }
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
            Kind::F32 => out.extend_from_slice(float32_repr(f32_at(p)).as_bytes()),
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
/// point or an exponent (the runtime in Sisu's, decision 358).
pub(crate) fn float_repr(f: f64) -> String {
    crate::bridge::float_text(f.to_bits(), crate::bridge::SHORTEST, 0)
}

/// The shortest text that reads back as the same `f32`, as [`float_repr`]
/// writes an `f64`.
pub(crate) fn float32_repr(f: f32) -> String {
    crate::bridge::float_text(u64::from(f.to_bits()), crate::bridge::SHORTEST32, 0)
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
