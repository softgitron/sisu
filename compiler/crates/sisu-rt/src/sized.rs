//! The sized integers an `i64` word holds (`u8` to `u64`, `i8` to `i32`):
//! the intrinsics `T.op`, computed in `i128` and trapping, wrapping or
//! saturating at the type's bounds, with the interpreter's messages
//! (`sisu-interp/src/sized.rs`).

use crate::abi::{self, SIZED_TYPES, SizedOp};
use crate::mem;
use crate::rt;
use sisu_diag::msg;

/// `flags` of `sisu_rt_sized`: the first operand is a `u64`.
pub(crate) const X_U64: u32 = 1;
/// `flags`: the second operand is a `u64`.
pub(crate) const Y_U64: u32 = 2;

fn width(kind: &str) -> (u32, bool) {
    match kind {
        "u8" => (8, false),
        "u16" => (16, false),
        "u32" => (32, false),
        "u64" => (64, false),
        "i8" => (8, true),
        "i16" => (16, true),
        _ => (32, true),
    }
}

fn bounds(kind: &str) -> (i128, i128, u32, bool) {
    match width(kind) {
        (bits, true) => (-(1i128 << (bits - 1)), (1i128 << (bits - 1)) - 1, bits, true),
        (bits, false) => (0, (1i128 << bits) - 1, bits, false),
    }
}

fn wrap(kind: &str, v: i128) -> i128 {
    let (_, _, bits, signed) = bounds(kind);
    let m = v & ((1i128 << bits) - 1);
    if signed && m >= 1i128 << (bits - 1) { m - (1i128 << bits) } else { m }
}

/// The word holding a value of the type.
fn make(v: i128) -> u64 {
    v as i64 as u64
}

/// An operation of a sized integer type (`kind`, by its number in
/// [`abi::SIZED_TYPES`]) on `x` and `y`, written at `out`.
///
/// # Safety
/// `out` must have room for the result: a word, or an option of one.
#[unsafe(no_mangle)]
unsafe extern "C" fn sisu_rt_sized(kind: u32, op: u32, flags: u32, x: i64, y: i64, out: *mut u8) -> u8 {
    // SAFETY: as the caller guarantees.
    crate::sched::system(|| unsafe { sized_op(kind, op, flags, x, y, out) })
}

/// # Safety
/// As `sisu_rt_sized`.
unsafe fn sized_op(kind: u32, op: u32, flags: u32, x: i64, y: i64, out: *mut u8) -> u8 {
    let Some(kind) = SIZED_TYPES.get(kind as usize).copied() else {
        rt::internal("an unknown sized integer type")
    };
    let Some(op) = SizedOp::from_code(op) else { rt::internal("an unknown sized integer operation") };
    let x = if flags & X_U64 != 0 { i128::from(x as u64) } else { i128::from(x) };
    let y = if flags & Y_U64 != 0 { i128::from(y as u64) } else { i128::from(y) };
    // SAFETY: `out` has room for the result.
    unsafe {
        match sized(kind, op, x, y) {
            Ok(Out::Word(w)) => mem::set_word(out, 0, w),
            Ok(Out::Some(w)) => {
                mem::set_word(out, 0, 0);
                mem::set_word(out, 8, w);
            }
            Ok(Out::None) => mem::set_word(out, 0, 1),
            Err(status) => return status,
        }
    }
    abi::OK
}

enum Out {
    Word(u64),
    Some(u64),
    None,
}

fn sized(kind: &str, op: SizedOp, x: i128, y: i128) -> Result<Out, u8> {
    let (lo, hi, bits, signed) = bounds(kind);
    let checked = |r: Option<i128>, sym: &str, y: i128| -> Result<Out, u8> {
        match r {
            Some(v) if (lo..=hi).contains(&v) => Ok(Out::Word(make(v))),
            _ => Err(rt::raise_msg(msg::PANIC_OVERFLOW, &[("expr", format!("{x} {sym} {y} as {kind}"))])),
        }
    };
    let word = |v: i128| Ok(Out::Word(make(v)));
    match op {
        SizedOp::Add => checked(x.checked_add(y), "+", y),
        SizedOp::Sub => checked(x.checked_sub(y), "-", y),
        SizedOp::Mul => checked(x.checked_mul(y), "*", y),
        SizedOp::FloorDiv | SizedOp::Rem => {
            if y == 0 {
                return Err(rt::raise_msg(msg::PANIC_DIVIDE_BY_ZERO, &[]));
            }
            let r = x.wrapping_rem(y);
            let inexact = r != 0 && ((x < 0) != (y < 0));
            if op == SizedOp::Rem {
                return word(if inexact { r + y } else { r });
            }
            checked(x.checked_div(y).map(|q| if inexact { q - 1 } else { q }), "//", y)
        }
        SizedOp::Pow => {
            if y < 0 {
                return Err(rt::raise_msg(msg::PANIC_NEGATIVE_EXPONENT, &[("expr", format!("{x} ** {y}"))]));
            }
            let r = match u32::try_from(y).ok().and_then(|e| x.checked_pow(e)) {
                Some(v) => Some(v),
                None if x == 0 || x == 1 => Some(x),
                None if x == -1 => Some(if y % 2 == 0 { 1 } else { -1 }),
                None => None,
            };
            checked(r, "**", y)
        }
        SizedOp::WrapAdd => word(wrap(kind, x.wrapping_add(y))),
        SizedOp::WrapSub => word(wrap(kind, x.wrapping_sub(y))),
        SizedOp::WrapMul => word(wrap(kind, x.wrapping_mul(y))),
        SizedOp::SatAdd => word(x.saturating_add(y).clamp(lo, hi)),
        SizedOp::SatSub => word(x.saturating_sub(y).clamp(lo, hi)),
        SizedOp::SatMul => word(x.saturating_mul(y).clamp(lo, hi)),
        SizedOp::Shl | SizedOp::Shr => {
            if !(0..i128::from(bits)).contains(&y) {
                return Err(rt::raise_msg(
                    msg::PANIC_SHIFT_OUT_OF_RANGE_SIZED,
                    &[("ty", kind.to_string()), ("amount", y.to_string()), ("max", (bits - 1).to_string())],
                ));
            }
            word(if op == SizedOp::Shl { wrap(kind, x << y) } else { x >> y })
        }
        SizedOp::And => word(x & y),
        SizedOp::Or => word(x | y),
        SizedOp::Xor => word(x ^ y),
        SizedOp::Not => word(if signed { !x } else { hi ^ x }),
        SizedOp::Neg => checked(x.checked_neg(), "-", 0),
        SizedOp::Cmp => Ok(Out::Word(match x.cmp(&y) {
            std::cmp::Ordering::Less => 0,
            std::cmp::Ordering::Equal => 1,
            std::cmp::Ordering::Greater => 2,
        })),
        SizedOp::ToInt if kind == "u64" => Ok(match i64::try_from(x) {
            Ok(v) => Out::Some(v as u64),
            Err(_) => Out::None,
        }),
        SizedOp::ToInt => word(x),
        SizedOp::FromLiteral => checked(Some(x), "as", 0),
        SizedOp::Widen => word(x),
        SizedOp::FromBits => Ok(Out::Word(x as i64 as u64)),
        SizedOp::WrappingFrom => word(wrap(kind, x)),
        SizedOp::ToSized => Ok(if (lo..=hi).contains(&x) { Out::Some(make(x)) } else { Out::None }),
        // Only a 128-bit literal is made from its text (`sisu_rt_wide`).
        SizedOp::FromText => rt::internal(&format!("no `{kind}` operation `from_text`")),
    }
}
