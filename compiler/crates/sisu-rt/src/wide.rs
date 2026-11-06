//! `i128` and `u128` (03, "Numbers"): two words each, the low one first.
//! The intrinsics `i128.op` and `u128.op`, and `int.to_i128` and
//! `int.to_u128`, with the interpreter's arithmetic and messages
//! (`sisu-interp/src/sized.rs`, `sized_op` and `u128_op`): an operand may
//! be an integer of any type the operation widens from.

use crate::abi::{self, Kind, SizedOp, WIDE_TYPES};
use crate::mem;
use crate::rt::{self, ty};
use crate::value;
use sisu_diag::msg;
use std::cmp::Ordering;

/// The value of a 128-bit integer at `p`, as its bits.
///
/// # Safety
/// `p` must hold two words.
pub(crate) unsafe fn bits(p: *const u8) -> u128 {
    // SAFETY: as the caller guarantees.
    unsafe { u128::from(mem::word(p, 0)) | (u128::from(mem::word(p, 8)) << 64) }
}

/// Writes the bits of a 128-bit integer at `p`.
///
/// # Safety
/// `p` must have room for two words.
pub(crate) unsafe fn set_bits(p: *mut u8, v: u128) {
    // SAFETY: as the caller guarantees.
    unsafe {
        mem::set_word(p, 0, v as u64);
        mem::set_word(p, 8, (v >> 64) as u64);
    }
}

/// An integer of any type: a `u128` as itself, every other as `i128`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Num {
    Signed(i128),
    Unsigned(u128),
}

impl Num {
    /// Its value as an `i128`, which a `u128` beyond it is not.
    fn signed(self) -> Option<i128> {
        match self {
            Num::Signed(n) => Some(n),
            Num::Unsigned(n) => i128::try_from(n).ok(),
        }
    }

    /// Its value as a `u128`, which a negative one is not.
    fn unsigned(self) -> Option<u128> {
        match self {
            Num::Signed(n) => u128::try_from(n).ok(),
            Num::Unsigned(n) => Some(n),
        }
    }

    /// Whether it is below zero, and its magnitude.
    pub(crate) fn magnitude(self) -> (bool, u128) {
        match self {
            Num::Signed(n) => (n < 0, n.unsigned_abs()),
            Num::Unsigned(n) => (false, n),
        }
    }
}

impl std::fmt::Display for Num {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Num::Signed(n) => write!(f, "{n}"),
            Num::Unsigned(n) => write!(f, "{n}"),
        }
    }
}

/// The integer at `p`, of type `t` (through references); `None` for a
/// value that is not an integer.
///
/// # Safety
/// `p` must hold a value of type `t`.
pub(crate) unsafe fn num(p: *mut u8, t: u32) -> Option<Num> {
    // SAFETY: as the caller guarantees.
    unsafe {
        let (p, t) = value::target(p, t);
        Some(match ty(t).kind {
            Kind::Int | Kind::Sized(_) => Num::Signed(i128::from(value::scalar((p, t)) as i64)),
            Kind::U64 => Num::Signed(i128::from(mem::word(p, 0))),
            Kind::I128 => Num::Signed(bits(p) as i128),
            Kind::U128 => Num::Unsigned(bits(p)),
            _ => return None,
        })
    }
}

/// What an operation gives.
enum Out {
    /// A value of the type.
    Value(u128),
    /// `Some` of a value of the type (`int.to_i128`).
    SomeValue(u128),
    /// An `int`, or an `Ordering` by its tag.
    Word(u64),
    /// `Some` of an `int` (`to_int`).
    SomeWord(u64),
    None,
}

/// An operation of `i128` or `u128` (`kind`, by its number in
/// [`abi::WIDE_TYPES`]) on the values at `x` and `y` (null when the
/// operation takes one), written at `out`.
///
/// # Safety
/// `x` and `y` must hold values of their types, and `out` have room for
/// the result: at most an option of a 128-bit integer.
#[unsafe(no_mangle)]
unsafe extern "C" fn sisu_rt_wide(
    kind: u32,
    op: u32,
    x: *mut u8,
    xt: u32,
    y: *mut u8,
    yt: u32,
    out: *mut u8,
) -> u8 {
    // SAFETY: as the caller guarantees.
    crate::sched::system(|| unsafe { wide_op(kind, op, x, xt, y, yt, out) })
}

/// # Safety
/// As `sisu_rt_wide`.
unsafe fn wide_op(kind: u32, op: u32, x: *mut u8, xt: u32, y: *mut u8, yt: u32, out: *mut u8) -> u8 {
    let Some(kind) = WIDE_TYPES.get(kind as usize).copied() else {
        rt::internal("an unknown 128-bit integer type")
    };
    let Some(op) = SizedOp::from_code(op) else { rt::internal("an unknown 128-bit integer operation") };
    // SAFETY: the operands hold values of their types, and `out` has room
    // for the result.
    unsafe {
        let r = if op == SizedOp::FromText {
            let text = value::text(x, xt).unwrap_or(&[]);
            let text = std::str::from_utf8(text).unwrap_or("");
            let v = if kind == "u128" {
                text.parse::<u128>().ok()
            } else {
                text.parse::<i128>().ok().map(|v| v as u128)
            };
            match v {
                Some(v) => Ok(Out::Value(v)),
                None => rt::internal(&format!("`{text}` is not a `{kind}` literal")),
            }
        } else {
            let operand = |p: *mut u8, t: u32| {
                if p.is_null() {
                    return None;
                }
                num(p, t)
            };
            let Some(a) = operand(x, xt) else {
                rt::internal("a 128-bit integer operation without its operand")
            };
            let b = operand(y, yt);
            if kind == "u128" { unsigned_op(op, a, b) } else { signed_op(op, a, b) }
        };
        match r {
            Ok(Out::Value(v)) => set_bits(out, v),
            Ok(Out::SomeValue(v)) => {
                mem::set_word(out, 0, 0);
                set_bits(out.add(8), v);
            }
            Ok(Out::Word(w)) => mem::set_word(out, 0, w),
            Ok(Out::SomeWord(w)) => {
                mem::set_word(out, 0, 0);
                mem::set_word(out, 8, w);
            }
            Ok(Out::None) => mem::set_word(out, 0, 1),
            Err(status) => return status,
        }
    }
    abi::OK
}

fn ordering(o: Ordering) -> Out {
    Out::Word(match o {
        Ordering::Less => 0,
        Ordering::Equal => 1,
        Ordering::Greater => 2,
    })
}

fn to_int(x: Num) -> Out {
    match x.signed().and_then(|v| i64::try_from(v).ok()) {
        Some(v) => Out::SomeWord(v as u64),
        None => Out::None,
    }
}

fn shift_range(kind: &str, amount: String) -> u8 {
    rt::raise_msg(
        msg::PANIC_SHIFT_OUT_OF_RANGE_SIZED,
        &[("ty", kind.to_string()), ("amount", amount), ("max", "127".to_string())],
    )
}

/// `i128`'s operations, as the interpreter's `sized_op` computes them at
/// the bounds of `i128`.
fn signed_op(op: SizedOp, a: Num, b: Option<Num>) -> Result<Out, u8> {
    let Some(x) = a.signed() else { rt::internal("a `u128` operand of an `i128` operation") };
    let y = || match b.and_then(Num::signed) {
        Some(y) => y,
        None => rt::internal("an `i128` operation without its second operand"),
    };
    let checked = |r: Option<i128>, sym: &str, y: i128| -> Result<Out, u8> {
        match r {
            Some(v) => Ok(Out::Value(v as u128)),
            None => Err(rt::raise_msg(msg::PANIC_OVERFLOW, &[("expr", format!("{x} {sym} {y} as i128"))])),
        }
    };
    let value = |v: i128| Ok(Out::Value(v as u128));
    match op {
        SizedOp::Add => checked(x.checked_add(y()), "+", y()),
        SizedOp::Sub => checked(x.checked_sub(y()), "-", y()),
        SizedOp::Mul => checked(x.checked_mul(y()), "*", y()),
        SizedOp::FloorDiv | SizedOp::Rem => {
            let y = y();
            if y == 0 {
                return Err(rt::raise_msg(msg::PANIC_DIVIDE_BY_ZERO, &[]));
            }
            // `//` floors and `%` has the sign of the divisor; only the
            // least value over -1 overflows.
            let r = x.wrapping_rem(y);
            let inexact = r != 0 && ((x < 0) != (y < 0));
            if op == SizedOp::Rem {
                return value(if inexact { r + y } else { r });
            }
            checked(x.checked_div(y).map(|q| if inexact { q - 1 } else { q }), "//", y)
        }
        SizedOp::Pow => {
            let y = y();
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
        SizedOp::WrapAdd => value(x.wrapping_add(y())),
        SizedOp::WrapSub => value(x.wrapping_sub(y())),
        SizedOp::WrapMul => value(x.wrapping_mul(y())),
        SizedOp::SatAdd => value(x.saturating_add(y())),
        SizedOp::SatSub => value(x.saturating_sub(y())),
        SizedOp::SatMul => value(x.saturating_mul(y())),
        SizedOp::Shl | SizedOp::Shr => {
            let y = y();
            if !(0..128).contains(&y) {
                return Err(shift_range("i128", y.to_string()));
            }
            value(if op == SizedOp::Shl { x << y } else { x >> y })
        }
        SizedOp::And => value(x & y()),
        SizedOp::Or => value(x | y()),
        SizedOp::Xor => value(x ^ y()),
        SizedOp::Not => value(!x),
        SizedOp::Neg => checked(x.checked_neg(), "-", 0),
        SizedOp::Cmp => Ok(ordering(x.cmp(&y()))),
        SizedOp::ToInt => Ok(to_int(a)),
        SizedOp::FromLiteral | SizedOp::Widen | SizedOp::WrappingFrom => value(x),
        SizedOp::ToSized => Ok(Out::SomeValue(x as u128)),
        SizedOp::FromBits | SizedOp::FromText => {
            rt::internal(&format!("no `i128` operation `{}`", op.name()))
        }
    }
}

/// `u128`'s operations, as the interpreter's `u128_op` computes them.
fn unsigned_op(op: SizedOp, a: Num, b: Option<Num>) -> Result<Out, u8> {
    match op {
        // Two's complement: -1 is the greatest value.
        SizedOp::WrappingFrom => {
            return Ok(Out::Value(match a {
                Num::Signed(n) => n as u128,
                Num::Unsigned(n) => n,
            }));
        }
        SizedOp::ToSized => {
            return Ok(match a.unsigned() {
                Some(v) => Out::SomeValue(v),
                None => Out::None,
            });
        }
        _ => {}
    }
    let overflow = |expr: String| rt::raise_msg(msg::PANIC_OVERFLOW, &[("expr", expr)]);
    let Some(x) = a.unsigned() else { return Err(overflow(format!("{a} as u128"))) };
    let y = || match b.and_then(Num::unsigned) {
        Some(y) => y,
        None => rt::internal("a negative `u128` operand"),
    };
    let checked = |r: Option<u128>, sym: &str, y: u128| -> Result<Out, u8> {
        match r {
            Some(v) => Ok(Out::Value(v)),
            None => Err(overflow(format!("{x} {sym} {y} as u128"))),
        }
    };
    let value = |v: u128| Ok(Out::Value(v));
    match op {
        SizedOp::Add => checked(x.checked_add(y()), "+", y()),
        SizedOp::Sub => checked(x.checked_sub(y()), "-", y()),
        SizedOp::Mul => checked(x.checked_mul(y()), "*", y()),
        SizedOp::FloorDiv | SizedOp::Rem => {
            let y = y();
            if y == 0 {
                return Err(rt::raise_msg(msg::PANIC_DIVIDE_BY_ZERO, &[]));
            }
            value(if op == SizedOp::Rem { x % y } else { x / y })
        }
        SizedOp::Pow => {
            let y = y();
            let r = match u32::try_from(y).ok().and_then(|e| x.checked_pow(e)) {
                Some(v) => Some(v),
                None if x <= 1 => Some(x),
                None => None,
            };
            checked(r, "**", y)
        }
        SizedOp::WrapAdd => value(x.wrapping_add(y())),
        SizedOp::WrapSub => value(x.wrapping_sub(y())),
        SizedOp::WrapMul => value(x.wrapping_mul(y())),
        SizedOp::SatAdd => value(x.saturating_add(y())),
        SizedOp::SatSub => value(x.saturating_sub(y())),
        SizedOp::SatMul => value(x.saturating_mul(y())),
        SizedOp::Shl | SizedOp::Shr => {
            let y = y();
            if y >= 128 {
                return Err(shift_range("u128", y.to_string()));
            }
            value(if op == SizedOp::Shl { x << y } else { x >> y })
        }
        SizedOp::And => value(x & y()),
        SizedOp::Or => value(x | y()),
        SizedOp::Xor => value(x ^ y()),
        SizedOp::Not => value(!x),
        SizedOp::Neg if x == 0 => value(0),
        SizedOp::Neg => Err(overflow(format!("-{x} as u128"))),
        SizedOp::Cmp => Ok(ordering(x.cmp(&y()))),
        SizedOp::ToInt => Ok(to_int(a)),
        SizedOp::FromLiteral | SizedOp::Widen => value(x),
        SizedOp::WrappingFrom | SizedOp::ToSized | SizedOp::FromBits | SizedOp::FromText => {
            rt::internal(&format!("no `u128` operation `{}`", op.name()))
        }
    }
}
