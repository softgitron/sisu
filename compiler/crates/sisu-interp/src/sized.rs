//! The sized integers (03, "Numbers"): `u8` to `u128` and `i8` to `i128`
//! (`i64` is `int`). A `u64`, `u128` or `i128` is a value of its own kind;
//! the others are `int` values their type keeps in range. The intrinsics
//! `T.op` compute in a wider host integer (in `i128` up to 64 bits, in the
//! type's own for the 128-bit ones) and trap, wrap or saturate at `T`'s
//! bounds; the comparisons and hashing of the values are the structural
//! ones.

use crate::value::Value;
use crate::{Interp, Unwind, ice, panic};
use sisu_diag::msg;
use std::rc::Rc;

/// The sized integer types (as `sisu_resolve::std_sources::SIZED`).
const SIZED: &[&str] = &["u8", "u16", "u32", "u64", "u128", "i8", "i16", "i32", "i128"];

/// The bits of a sized integer type and whether it is signed.
fn int_width(ty: &str) -> Option<(u32, bool)> {
    Some(match ty {
        "u8" => (8, false),
        "u16" => (16, false),
        "u32" => (32, false),
        "u64" => (64, false),
        "u128" => (128, false),
        "i8" => (8, true),
        "i16" => (16, true),
        "i32" => (32, true),
        "i128" => (128, true),
        _ => return None,
    })
}

/// The bounds of a sized integer type, as far as `i128` holds them (all of
/// them but `u128`'s greatest values, which have arithmetic of their own).
fn bounds(kind: &str) -> (i128, i128, u32, bool) {
    match int_width(kind).unwrap_or((64, true)) {
        (128, true) => (i128::MIN, i128::MAX, 128, true),
        (128, false) => (0, i128::MAX, 128, false),
        (bits, true) => (-(1i128 << (bits - 1)), (1i128 << (bits - 1)) - 1, bits, true),
        (bits, false) => (0, (1i128 << bits) - 1, bits, false),
    }
}

/// The value of `v`, a sized integer of kind `kind`, in range.
fn make(kind: &str, v: i128) -> Value {
    match kind {
        "u64" => Value::U64(v as u64),
        "i128" => Value::I128(v),
        "u128" => Value::U128(v as u128),
        _ => Value::Int(v as i64),
    }
}

/// `v` reduced to `kind`'s width, as two's complement wrapping does; an
/// `i128` result is already reduced.
fn wrap(kind: &str, v: i128) -> i128 {
    let (_, _, bits, signed) = bounds(kind);
    if bits == 128 {
        return v;
    }
    let m = v & ((1i128 << bits) - 1);
    if signed && m >= 1i128 << (bits - 1) { m - (1i128 << bits) } else { m }
}

impl Interp<'_, '_> {
    fn sized_value(&self, v: &Value) -> Result<i128, Unwind> {
        match self.target(v)? {
            Value::Int(n) => Ok(i128::from(*n)),
            Value::U64(n) => Ok(i128::from(*n)),
            Value::I128(n) => Ok(*n),
            other => Err(ice(format!("expected an integer, found {other:?}"))),
        }
    }

    /// The value of a `u128` operand, or of a narrower unsigned one that
    /// widens to it; `None` for a negative `int` (a literal out of range).
    fn unsigned_value(&self, v: &Value) -> Result<Option<u128>, Unwind> {
        match self.target(v)? {
            Value::U128(n) => Ok(Some(*n)),
            Value::U64(n) => Ok(Some(u128::from(*n))),
            Value::Int(n) => Ok(u128::try_from(*n).ok()),
            other => Err(ice(format!("expected an unsigned integer, found {other:?}"))),
        }
    }

    /// The intrinsics of the sized integers; `None` for any other.
    pub(crate) fn sized_intrinsic(&mut self, name: &str, args: &[Value]) -> Option<Result<Value, Unwind>> {
        let (owner, method) = name.split_once('.')?;
        if owner == "int" {
            let kind = method.strip_prefix("to_").and_then(|k| SIZED.iter().copied().find(|s| *s == k));
            if method == "widen" {
                return Some(args.first().ok_or_else(|| ice("widen without a value")).cloned());
            }
            let kind = kind?;
            return Some((|| {
                let v = self.sized_value(&args[0])?;
                let (lo, hi, _, _) = bounds(kind);
                Ok(Value::option((lo..=hi).contains(&v).then(|| make(kind, v))))
            })());
        }
        let kind = SIZED.iter().copied().find(|s| *s == owner)?;
        if kind == "u128" {
            return Some(self.u128_op(method, args));
        }
        Some(self.sized_op(kind, method, args))
    }

    fn sized_op(&mut self, kind: &str, method: &str, args: &[Value]) -> Result<Value, Unwind> {
        let (lo, hi, bits, signed) = bounds(kind);
        if method == "from_text" {
            let text = self.text(args.first().ok_or_else(|| ice("from_text without a text"))?)?;
            let v = text.parse::<i128>().map_err(|_| ice(format!("`{text}` is not a `{kind}` literal")))?;
            return Ok(make(kind, v));
        }
        let arg = |me: &Self, i: usize| -> Result<i128, Unwind> {
            me.sized_value(args.get(i).ok_or_else(|| ice(format!("missing argument {i}")))?)
        };
        let x = arg(self, 0)?;
        let checked = |r: Option<i128>, sym: &str, y: i128| -> Result<Value, Unwind> {
            match r {
                Some(v) if (lo..=hi).contains(&v) => Ok(make(kind, v)),
                _ => Err(panic(msg::PANIC_OVERFLOW, &[("expr", format!("{x} {sym} {y} as {kind}"))])),
            }
        };
        match method {
            "add" => checked(x.checked_add(arg(self, 1)?), "+", arg(self, 1)?),
            "sub" => checked(x.checked_sub(arg(self, 1)?), "-", arg(self, 1)?),
            "mul" => checked(x.checked_mul(arg(self, 1)?), "*", arg(self, 1)?),
            "floordiv" | "rem" => {
                let y = arg(self, 1)?;
                if y == 0 {
                    return Err(panic(msg::PANIC_DIVIDE_BY_ZERO, &[]));
                }
                // `//` floors and `%` has the sign of the divisor, as for
                // `int`; only `i128`'s least value over -1 overflows.
                let r = x.wrapping_rem(y);
                let inexact = r != 0 && ((x < 0) != (y < 0));
                if method == "rem" {
                    return Ok(make(kind, if inexact { r + y } else { r }));
                }
                checked(x.checked_div(y).map(|q| if inexact { q - 1 } else { q }), "//", y)
            }
            "pow" => {
                let y = arg(self, 1)?;
                if y < 0 {
                    return Err(panic(msg::PANIC_NEGATIVE_EXPONENT, &[("expr", format!("{x} ** {y}"))]));
                }
                let r = u32::try_from(y).ok().and_then(|e| x.checked_pow(e));
                let r = match r {
                    Some(v) => Some(v),
                    None if x == 0 || x == 1 => Some(x),
                    None if x == -1 => Some(if y % 2 == 0 { 1 } else { -1 }),
                    None => None,
                };
                checked(r, "**", y)
            }
            "wrap_add" => Ok(make(kind, wrap(kind, x.wrapping_add(arg(self, 1)?)))),
            "wrap_sub" => Ok(make(kind, wrap(kind, x.wrapping_sub(arg(self, 1)?)))),
            "wrap_mul" => Ok(make(kind, wrap(kind, x.wrapping_mul(arg(self, 1)?)))),
            "sat_add" => Ok(make(kind, x.saturating_add(arg(self, 1)?).clamp(lo, hi))),
            "sat_sub" => Ok(make(kind, x.saturating_sub(arg(self, 1)?).clamp(lo, hi))),
            "sat_mul" => Ok(make(kind, x.saturating_mul(arg(self, 1)?).clamp(lo, hi))),
            "shl" | "shr" => {
                let y = arg(self, 1)?;
                if !(0..i128::from(bits)).contains(&y) {
                    return Err(self.shift_range(kind, &y.to_string(), bits));
                }
                Ok(make(kind, if method == "shl" { wrap(kind, x << y) } else { x >> y }))
            }
            "and" => Ok(make(kind, x & arg(self, 1)?)),
            "or" => Ok(make(kind, x | arg(self, 1)?)),
            "xor" => Ok(make(kind, x ^ arg(self, 1)?)),
            "not" => Ok(make(kind, if signed { !x } else { hi ^ x })),
            "neg" => checked(x.checked_neg(), "-", 0),
            "cmp" => Ok(Value::ordering(x.cmp(&arg(self, 1)?))),
            "to_int" if matches!(kind, "u64" | "i128") => {
                Ok(Value::option(i64::try_from(x).ok().map(Value::Int)))
            }
            "to_int" => Ok(Value::Int(x as i64)),
            "from_literal" => checked(Some(x), "as", 0),
            "widen" => Ok(make(kind, x)),
            "from_bits" => Ok(Value::U64(x as i64 as u64)),
            // `T.wrapping_from(n)` (`int.truncate_to[T]`): `n` reduced to
            // the type's width.
            "wrapping_from" => Ok(make(kind, wrap(kind, x))),
            other => Err(ice(format!("no `{kind}` operation `{other}`"))),
        }
    }

    fn shift_range(&self, kind: &str, amount: &str, bits: u32) -> Unwind {
        panic(
            msg::PANIC_SHIFT_OUT_OF_RANGE_SIZED,
            &[("ty", kind.to_string()), ("amount", amount.to_string()), ("max", (bits - 1).to_string())],
        )
    }

    /// The operations of `u128`, whose values beyond `i128` need its own
    /// arithmetic: as [`Self::sized_op`]'s, at the bounds 0 and 2**128 - 1.
    fn u128_op(&mut self, method: &str, args: &[Value]) -> Result<Value, Unwind> {
        if method == "from_text" {
            let text = self.text(args.first().ok_or_else(|| ice("from_text without a text"))?)?;
            let v = text.parse::<u128>().map_err(|_| ice(format!("`{text}` is not a `u128` literal")))?;
            return Ok(Value::U128(v));
        }
        if method == "wrapping_from" {
            // Two's complement: -1 is the greatest value.
            let n = self.sized_value(args.first().ok_or_else(|| ice("wrapping_from without a value"))?)?;
            return Ok(Value::U128(n as u128));
        }
        let get = |me: &Self, i: usize| -> Result<Option<u128>, Unwind> {
            me.unsigned_value(args.get(i).ok_or_else(|| ice(format!("missing argument {i}")))?)
        };
        let overflow = |expr: String| panic(msg::PANIC_OVERFLOW, &[("expr", expr)]);
        let Some(x) = get(self, 0)? else {
            let n = self.sized_value(&args[0])?;
            return Err(overflow(format!("{n} as u128")));
        };
        let y = || -> Result<u128, Unwind> {
            match get(self, 1)? {
                Some(y) => Ok(y),
                None => Err(ice("a negative `u128` operand")),
            }
        };
        let checked = |r: Option<u128>, sym: &str, y: u128| -> Result<Value, Unwind> {
            r.map(Value::U128).ok_or_else(|| overflow(format!("{x} {sym} {y} as u128")))
        };
        match method {
            "add" => {
                let y = y()?;
                checked(x.checked_add(y), "+", y)
            }
            "sub" => {
                let y = y()?;
                checked(x.checked_sub(y), "-", y)
            }
            "mul" => {
                let y = y()?;
                checked(x.checked_mul(y), "*", y)
            }
            "floordiv" | "rem" => {
                let y = y()?;
                if y == 0 {
                    return Err(panic(msg::PANIC_DIVIDE_BY_ZERO, &[]));
                }
                Ok(Value::U128(if method == "rem" { x % y } else { x / y }))
            }
            "pow" => {
                let y = y()?;
                let r = match u32::try_from(y).ok().and_then(|e| x.checked_pow(e)) {
                    Some(v) => Some(v),
                    None if x <= 1 => Some(x),
                    None => None,
                };
                checked(r, "**", y)
            }
            "wrap_add" => Ok(Value::U128(x.wrapping_add(y()?))),
            "wrap_sub" => Ok(Value::U128(x.wrapping_sub(y()?))),
            "wrap_mul" => Ok(Value::U128(x.wrapping_mul(y()?))),
            "sat_add" => Ok(Value::U128(x.saturating_add(y()?))),
            "sat_sub" => Ok(Value::U128(x.saturating_sub(y()?))),
            "sat_mul" => Ok(Value::U128(x.saturating_mul(y()?))),
            "shl" | "shr" => {
                let y = y()?;
                if y >= 128 {
                    return Err(self.shift_range("u128", &y.to_string(), 128));
                }
                Ok(Value::U128(if method == "shl" { x << y } else { x >> y }))
            }
            "and" => Ok(Value::U128(x & y()?)),
            "or" => Ok(Value::U128(x | y()?)),
            "xor" => Ok(Value::U128(x ^ y()?)),
            "not" => Ok(Value::U128(!x)),
            "neg" if x == 0 => Ok(Value::U128(0)),
            "neg" => Err(overflow(format!("-{x} as u128"))),
            "cmp" => Ok(Value::ordering(x.cmp(&y()?))),
            "to_int" => Ok(Value::option(i64::try_from(x).ok().map(Value::Int))),
            "from_literal" | "widen" => Ok(Value::U128(x)),
            other => Err(ice(format!("no `u128` operation `{other}`"))),
        }
    }
}

impl Interp<'_, '_> {
    fn byte_values(&self, v: &Value) -> Result<std::rc::Rc<[u8]>, Unwind> {
        match self.target(v)? {
            Value::Bytes(b) => Ok(b.clone()),
            other => Err(ice(format!("expected bytes, found {other:?}"))),
        }
    }

    /// The bounds of `b[start:end]` for a length, where a bound left out
    /// is the least `int`; out of order or out of bounds panics.
    fn cut_bounds(&self, start: i64, end: i64, len: usize) -> Result<(usize, usize), Unwind> {
        let s = if start == i64::MIN { 0 } else { start };
        let e = if end == i64::MIN { len as i64 } else { end };
        if s < 0 || e < s || e > len as i64 {
            let bad = if s < 0 || s > len as i64 { s } else { e };
            return Err(panic(
                msg::PANIC_INDEX_OUT_OF_BOUNDS,
                &[("len", len.to_string()), ("index", bad.to_string())],
            ));
        }
        Ok((s as usize, e as usize))
    }

    /// The intrinsics of `bytes` and `Slice`, and the text-to-bytes ones;
    /// `None` for any other.
    pub(crate) fn bytes_intrinsic(&mut self, name: &str, args: &[Value]) -> Option<Result<Value, Unwind>> {
        let r = match name {
            "bytes.new" => Ok(Value::Bytes(std::rc::Rc::from(Vec::new()))),
            "bytes.from_hex" => (|| {
                let hex = self.text(&args[0])?;
                let mut out = Vec::with_capacity(hex.len() / 2);
                for i in (0..hex.len()).step_by(2) {
                    let b = hex.get(i..i + 2).and_then(|h| u8::from_str_radix(h, 16).ok());
                    out.push(b.ok_or_else(|| ice("bad hex in a byte string"))?);
                }
                Ok(Value::Bytes(std::rc::Rc::from(out)))
            })(),
            "bytes.from_list" => (|| {
                let xs = match self.target(&args[0])? {
                    Value::List(xs) => xs.clone(),
                    other => return Err(ice(format!("bytes.from_list of {other:?}"))),
                };
                let mut out = Vec::with_capacity(xs.len());
                for x in &xs {
                    out.push(self.int(x)? as u8);
                }
                Ok(Value::Bytes(std::rc::Rc::from(out)))
            })(),
            "bytes.at" | "bytes.get" => (|| {
                let b = self.byte_values(&args[0])?;
                let i = self.int(&args[1])?;
                let at = usize::try_from(i).ok().and_then(|i| b.get(i).copied());
                match (name, at) {
                    ("bytes.get", at) => Ok(Value::option(at.map(|x| Value::Int(i64::from(x))))),
                    (_, Some(x)) => Ok(Value::Int(i64::from(x))),
                    (_, None) => Err(panic(
                        msg::PANIC_INDEX_OUT_OF_BOUNDS,
                        &[("len", b.len().to_string()), ("index", i.to_string())],
                    )),
                }
            })(),
            "bytes.is_empty" => self.byte_values(&args[0]).map(|b| Value::Bool(b.is_empty())),
            "bytes.slice" | "bytes.cut" => (|| {
                let b = self.byte_values(&args[0])?;
                let (s, e) = (self.int(&args[1])?, self.int(&args[2])?);
                let (s, e) = if name == "bytes.slice" && (s == i64::MIN || e == i64::MIN) {
                    return Err(panic(
                        msg::PANIC_INDEX_OUT_OF_BOUNDS,
                        &[("len", b.len().to_string()), ("index", s.min(e).to_string())],
                    ));
                } else {
                    self.cut_bounds(s, e, b.len())?
                };
                Ok(Value::Bytes(std::rc::Rc::from(&b[s..e])))
            })(),
            "bytes.iter" => self.byte_values(&args[0]).map(|b| {
                let items = b.iter().map(|x| Value::Int(i64::from(*x))).collect();
                Value::Iter(Box::new(crate::value::IterState::Items(items)))
            }),
            "bytes.to_list" | "bytes.as_slice" => self
                .byte_values(&args[0])
                .map(|b| Value::List(b.iter().map(|x| Value::Int(i64::from(*x))).collect())),
            "bytes.find" | "bytes.starts_with" | "bytes.ends_with" => (|| {
                let b = self.byte_values(&args[0])?;
                let p = self.byte_values(&args[1])?;
                Ok(match name {
                    "bytes.find" => Value::option(if p.is_empty() {
                        Some(Value::Int(0))
                    } else {
                        b.windows(p.len()).position(|w| w == &*p).map(|i| Value::Int(i as i64))
                    }),
                    "bytes.starts_with" => Value::Bool(b.starts_with(&p)),
                    _ => Value::Bool(b.ends_with(&p)),
                })
            })(),
            "bytes.decode" => self.byte_values(&args[0]).map(|b| match std::str::from_utf8(&b) {
                Ok(s) => Value::ok(Value::String(s.to_string())),
                Err(e) => {
                    let i = self.variants.get(&("Utf8Error", "InvalidUtf8")).copied().unwrap_or(0);
                    Value::err(Value::variant("Utf8Error", i, vec![Value::Int(e.valid_up_to() as i64)]))
                }
            }),
            "bytes.concat" => (|| {
                let a = self.byte_values(&args[0])?;
                let b = self.byte_values(&args[1])?;
                let mut out = a.to_vec();
                out.extend_from_slice(&b);
                Ok(Value::Bytes(std::rc::Rc::from(out)))
            })(),
            "str.encode" => self.text(&args[0]).map(|s| Value::Bytes(std::rc::Rc::from(s.as_bytes()))),
            "list.as_slice" | "Slice.to_list" => match self.target(&args[0]) {
                Ok(Value::List(xs)) => Ok(Value::List(xs.clone())),
                Ok(other) => Err(ice(format!("{name} of {other:?}"))),
                Err(e) => Err(e),
            },
            "Slice.at" | "Slice.get" => (|| {
                let p = self.pointer_of(&args[0])?;
                let len = match self.read(&p)? {
                    Value::List(xs) => xs.len(),
                    other => return Err(ice(format!("{name} of {other:?}"))),
                };
                let i = self.int(&args[1])?;
                let at = usize::try_from(i).ok().filter(|i| *i < len);
                match (name, at) {
                    ("Slice.get", at) => Ok(Value::option(
                        at.map(|i| Value::Ref(Rc::new(p.join(crate::value::Step::Index(i))))),
                    )),
                    (_, Some(i)) => Ok(Value::Ref(Rc::new(p.join(crate::value::Step::Index(i))))),
                    (_, None) => Err(panic(
                        msg::PANIC_INDEX_OUT_OF_BOUNDS,
                        &[("len", len.to_string()), ("index", i.to_string())],
                    )),
                }
            })(),
            "Slice.is_empty" => match self.target(&args[0]) {
                Ok(Value::List(xs)) => Ok(Value::Bool(xs.is_empty())),
                Ok(other) => Err(ice(format!("{name} of {other:?}"))),
                Err(e) => Err(e),
            },
            "Slice.slice" | "Slice.cut" => (|| {
                let xs = match self.target(&args[0])? {
                    Value::List(xs) => xs.clone(),
                    other => return Err(ice(format!("{name} of {other:?}"))),
                };
                let (s, e) = self.cut_bounds(self.int(&args[1])?, self.int(&args[2])?, xs.len())?;
                Ok(Value::List(xs[s..e].to_vec()))
            })(),
            "Slice.iter" => (|| {
                let base = self.pointer_of(&args[0])?;
                Ok(Value::Iter(Box::new(crate::value::IterState::ListRef { base, idx: 0 })))
            })(),
            _ => return None,
        };
        Some(r)
    }
}
