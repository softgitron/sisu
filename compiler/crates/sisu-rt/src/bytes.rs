//! `bytes` (decision 208): an immutable owned run of bytes, laid out as a
//! `String` is (a heap buffer, its length and capacity, [`Kind::Bytes`]),
//! with the interpreter's intrinsics, results and messages
//! (`sisu-interp/src/sized.rs`, `bytes_intrinsic`).

use crate::abi::{self, Intrinsic, Kind};
use crate::intrinsics::{Call, Part, write_err};
use crate::iter;
use crate::mem::{self, List, Text};
use crate::rt::{self, ty};
use sisu_diag::msg;

/// The bytes argument `i` is or refers to.
///
/// # Safety
/// Argument `i` must be `bytes` or a reference to it, alive while the
/// slice is used.
pub(crate) unsafe fn arg<'b>(c: &Call<'_>, i: usize) -> &'b [u8] {
    // SAFETY: as the caller guarantees.
    unsafe {
        let (p, t) = c.place(i);
        match ty(t).kind {
            Kind::Bytes => mem::at::<Text>(p).bytes(),
            _ => rt::internal("a `bytes` method on something that is not `bytes`"),
        }
    }
}

/// Writes new `bytes` holding `b` at `dst`.
///
/// # Safety
/// `dst` must have room for `bytes`.
pub(crate) unsafe fn write(dst: *mut u8, b: &[u8]) {
    // SAFETY: as the caller guarantees.
    unsafe { (dst as *mut Text).write(Text::from_bytes(b)) }
}

/// `b[i]` out of bounds, and a cut whose bounds are out of order or out of
/// bounds: the interpreter's panic.
fn out_of_bounds(len: usize, index: i64) -> u8 {
    rt::raise_msg(msg::PANIC_INDEX_OUT_OF_BOUNDS, &[("len", len.to_string()), ("index", index.to_string())])
}

/// The bounds of `b[start:end]`, where a bound left out is the least
/// `int` (the interpreter's `cut_bounds`).
fn cut_bounds(start: i64, end: i64, len: usize) -> Result<(usize, usize), u8> {
    let s = if start == i64::MIN { 0 } else { start };
    let e = if end == i64::MIN { len as i64 } else { end };
    if s < 0 || e < s || e > len as i64 {
        let bad = if s < 0 || s > len as i64 { s } else { e };
        return Err(out_of_bounds(len, bad));
    }
    Ok((s as usize, e as usize))
}

/// The intrinsics of `bytes`, and `str.encode`; `None` for any other.
///
/// # Safety
/// The arguments are of the types the descriptor gives.
pub(crate) unsafe fn call(i: Intrinsic, c: &Call<'_>) -> Option<u8> {
    use Intrinsic as I;
    // SAFETY: each reads its arguments as their types say.
    unsafe {
        match i {
            I::BytesNew => write(c.ret, &[]),
            I::BytesFromHex => {
                let hex = c.text(0);
                let mut out = Vec::with_capacity(hex.len() / 2);
                for pair in hex.chunks(2) {
                    let b = std::str::from_utf8(pair).ok().and_then(|h| u8::from_str_radix(h, 16).ok());
                    out.push(b.unwrap_or_else(|| rt::internal("bad hex in a byte string")));
                }
                write(c.ret, &out);
            }
            I::BytesFromList => {
                let (p, t) = c.place(0);
                let Kind::List(e) = ty(t).kind else {
                    rt::internal("`bytes.from_list` of something not a list")
                };
                let l = mem::at::<List>(p);
                let size = ty(e).size;
                let out: Vec<u8> = (0..l.len).map(|k| mem::word(l.at(k, size), 0) as u8).collect();
                write(c.ret, &out);
            }
            I::BytesAt | I::BytesGet => {
                let b = arg(c, 0);
                let k = c.int(1);
                let at = usize::try_from(k).ok().and_then(|k| b.get(k).copied());
                match (i, at) {
                    (I::BytesGet, Some(x)) => {
                        mem::set_word(c.ret, 0, 0);
                        mem::set_word(c.ret, 8, u64::from(x));
                    }
                    (I::BytesGet, None) => c.none(),
                    (_, Some(x)) => c.set_word(u64::from(x)),
                    (_, None) => return Some(out_of_bounds(b.len(), k)),
                }
            }
            I::BytesIsEmpty => c.set_bool(arg(c, 0).is_empty()),
            I::BytesSlice | I::BytesCut => {
                let b = arg(c, 0);
                let (s, e) = (c.int(1), c.int(2));
                if i == I::BytesSlice && (s == i64::MIN || e == i64::MIN) {
                    return Some(out_of_bounds(b.len(), s.min(e)));
                }
                match cut_bounds(s, e, b.len()) {
                    Ok((s, e)) => write(c.ret, &b[s..e]),
                    Err(st) => return Some(st),
                }
            }
            // The iterator owns a copy: `bytes.iter` borrows nothing.
            I::BytesIter => c.set_word(iter::into_word(iter::owned_bytes(arg(c, 0).to_vec()))),
            // A `Slice` holds copies of what it views, as the interpreter's
            // does (decision 225): of bytes, each a `u8` of a word.
            I::BytesToList | I::BytesAsSlice => {
                let b = arg(c, 0);
                let mut l = List::empty();
                l.reserve(b.len() as u64, 8);
                for (k, x) in b.iter().enumerate() {
                    mem::set_word(l.at(k as u64, 8), 0, u64::from(*x));
                }
                l.len = b.len() as u64;
                (c.ret as *mut List).write(l);
            }
            I::BytesFind | I::BytesStartsWith | I::BytesEndsWith => {
                let (b, p) = (arg(c, 0), arg(c, 1));
                match i {
                    I::BytesFind => {
                        let at = if p.is_empty() { Some(0) } else { b.windows(p.len()).position(|w| w == p) };
                        match at {
                            Some(k) => {
                                mem::set_word(c.ret, 0, 0);
                                mem::set_word(c.ret, 8, k as u64);
                            }
                            None => c.none(),
                        }
                    }
                    I::BytesStartsWith => c.set_bool(b.starts_with(p)),
                    _ => c.set_bool(b.ends_with(p)),
                }
            }
            I::BytesDecode => match std::str::from_utf8(arg(c, 0)) {
                Ok(s) => {
                    mem::set_word(c.ret, 0, 0);
                    crate::value::write_string(c.ret.add(8), s.as_bytes());
                }
                Err(e) => write_err(c, "Utf8Error", "InvalidUtf8", &[Part::Word(e.valid_up_to() as u64)]),
            },
            I::BytesConcat => {
                let mut out = arg(c, 0).to_vec();
                out.extend_from_slice(arg(c, 1));
                write(c.ret, &out);
            }
            I::StrEncode => write(c.ret, c.text(0)),
            _ => return None,
        }
    }
    Some(abi::OK)
}
