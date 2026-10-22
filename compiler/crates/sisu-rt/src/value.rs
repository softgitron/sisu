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
        match ty(t).kind {
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
        match &entry.kind {
            Kind::String | Kind::Path => {
                mem::at::<Text>(p).release();
                abi::OK
            }
            Kind::List(e) => {
                let list = mem::at::<List>(p);
                let size = ty(*e).size;
                let mut status = abi::OK;
                if rt::needs_drop(*e) {
                    for i in 0..list.len {
                        status = after_part(status, drop_value(list.at(i, size), *e));
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
            Kind::Box(inner) => {
                let cell = mem::word(p, 0) as *mut u8;
                let status = drop_value(cell, *inner);
                mem::release(cell);
                status
            }
            Kind::Struct { glue: Some(g), .. } | Kind::Enum { glue: Some(g), .. } => match rt::function(*g) {
                Some(f) => {
                    let mut unit = 0u64;
                    f(&mut unit as *mut u64 as *mut u8, p)
                }
                None => rt::internal("drop glue missing from the function table"),
            },
            Kind::ErrSet(_) => {
                let decl = concrete(p, t);
                if decl == t { abi::OK } else { drop_value(p, decl) }
            }
            _ => abi::OK,
        }
    }
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

/// `.copy()`: writes at `dst` a deep copy of the value at `src`.
///
/// # Safety
/// `src` must hold a value of type `t`, and `dst` have room for one.
pub(crate) unsafe fn copy_value(dst: *mut u8, src: *mut u8, t: u32) {
    let entry = ty(t);
    // SAFETY: the parts are where the type says.
    unsafe {
        match &entry.kind {
            Kind::String | Kind::Path => {
                let b = mem::at::<Text>(src).bytes();
                (dst as *mut Text).write(Text::from_bytes(b));
            }
            Kind::List(e) => {
                let from = mem::at::<List>(src);
                let size = ty(*e).size;
                let mut to = List::empty();
                to.reserve(from.len, size);
                for i in 0..from.len {
                    copy_value(to.at(i, size), from.at(i, size), *e);
                }
                to.len = from.len;
                (dst as *mut List).write(to);
            }
            Kind::Option(payload) => {
                let tg = tag(src, entry);
                mem::set_word(dst, 0, tg);
                if tg == 0 {
                    copy_value(dst.add(8), src.add(8), *payload);
                }
            }
            Kind::Result(ok, err) => {
                let tg = tag(src, entry);
                mem::set_word(dst, 0, tg);
                copy_value(dst.add(8), src.add(8), if tg == 0 { *ok } else { *err });
            }
            Kind::Tuple(slots) => {
                for s in slots {
                    copy_value(dst.add(s.offset as usize), src.add(s.offset as usize), s.ty);
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
                    copy_value(dst.add(f.offset as usize), src.add(f.offset as usize), f.ty);
                }
            }
            Kind::Enum { variants, error, .. } => {
                // The number of its type and its trace (shared: a trace is
                // never changed in place), then the variant's fields.
                let off = entry.kind.tag_offset();
                mem::copy(dst, src, off as usize);
                let tg = tag(src, entry);
                mem::set_word(dst, off, tg);
                if let Some(v) = variants.get(tg as usize) {
                    for f in &v.fields {
                        copy_value(dst.add(f.offset as usize), src.add(f.offset as usize), f.ty);
                    }
                }
                let _ = error;
            }
            Kind::ErrSet(_) => {
                let decl = concrete(src, t);
                if decl == t {
                    mem::copy(dst, src, entry.size as usize);
                } else {
                    copy_value(dst, src, decl);
                }
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
        if let (Some(x), Some(y)) = (text(a, ta), text(b, tb)) {
            return x == y;
        }
        let (ea, eb) = (ty(ta), ty(tb));
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
        if let (Some(x), Some(y)) = (text(a, ta), text(b, tb)) {
            return Some(x.cmp(y));
        }
        let (ea, eb) = (ty(ta), ty(tb));
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
        if let Some(b) = text(p, t) {
            out.extend_from_slice(b);
            return;
        }
        match ty(t).kind {
            Kind::Char => push_char(out, mem::word(p, 0) as u32),
            _ => debug(out, p, t),
        }
    }
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
                out.extend_from_slice((mem::word(p, 0) as i64).to_string().as_bytes())
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
            Kind::Fn => out.extend_from_slice(b"<fn>"),
            Kind::Iter(_) => out.extend_from_slice(b"<iter>"),
            Kind::Opaque(n) => out.extend_from_slice(n.as_bytes()),
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
