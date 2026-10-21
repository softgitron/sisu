//! Operators, equality, ordering and hashing keys.

use crate::value::{Key, Value};
use crate::{Interp, Unwind, ice, msg, panic};
use sisu_sir::{BinOp, UnOp};
use std::cmp::Ordering;
use std::rc::Rc;

impl Interp<'_, '_> {
    pub(crate) fn binary(&mut self, op: BinOp, a: &Value, b: &Value) -> Result<Value, Unwind> {
        match op {
            BinOp::Eq => Ok(Value::Bool(self.equal(a, b)?)),
            BinOp::Ne => Ok(Value::Bool(!self.equal(a, b)?)),
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                let o = self.compare(a, b)?;
                Ok(Value::Bool(match o {
                    None => false,
                    Some(o) => match op {
                        BinOp::Lt => o == Ordering::Less,
                        BinOp::Le => o != Ordering::Greater,
                        BinOp::Gt => o == Ordering::Greater,
                        _ => o != Ordering::Less,
                    },
                }))
            }
            BinOp::In => Ok(Value::Bool(self.contains(b, a)?)),
            BinOp::Concat => {
                let mut s = self.text(a)?.to_string();
                s.push_str(self.text(b)?);
                Ok(Value::String(s))
            }
            _ => match (self.target(a)?, self.target(b)?) {
                (Value::Int(x), Value::Int(y)) => int_op(op, *x, *y).map(Value::Int),
                (Value::Float(x), Value::Float(y)) => Ok(Value::Float(float_op(op, *x, *y))),
                // In `f64`, which holds every `f32` and rounds the result
                // of `+`, `-`, `*` and `/` the same once it is rounded
                // again to `f32` (it has more than twice the bits).
                (Value::F32(x), Value::F32(y)) => {
                    Ok(Value::F32(float_op(op, f64::from(*x), f64::from(*y)) as f32))
                }
                (x, y) => Err(ice(format!("{} on {x:?} and {y:?}", op.name()))),
            },
        }
    }

    pub(crate) fn unary(&mut self, op: UnOp, a: &Value) -> Result<Value, Unwind> {
        match (op, self.target(a)?) {
            (UnOp::Neg, Value::Int(x)) => x
                .checked_neg()
                .map(Value::Int)
                .ok_or_else(|| panic(msg::PANIC_OVERFLOW, &[("expr", format!("-({x})"))])),
            (UnOp::Neg, Value::Float(x)) => Ok(Value::Float(-x)),
            (UnOp::Neg, Value::F32(x)) => Ok(Value::F32(-x)),
            (UnOp::Not, Value::Bool(b)) => Ok(Value::Bool(!b)),
            (UnOp::BitNot, Value::Int(x)) => Ok(Value::Int(!x)),
            (op, x) => Err(ice(format!("{} on {x:?}", op.name()))),
        }
    }

    /// Structural equality, through references; `str` and `String` with
    /// the same text are equal.
    pub(crate) fn equal(&self, a: &Value, b: &Value) -> Result<bool, Unwind> {
        let (a, b) = (self.target(a)?, self.target(b)?);
        Ok(match (a, b) {
            (Value::Int(x), Value::Int(y)) => x == y,
            (Value::U64(x), Value::U64(y)) => x == y,
            (Value::I128(x), Value::I128(y)) => x == y,
            (Value::U128(x), Value::U128(y)) => x == y,
            (Value::Bytes(x), Value::Bytes(y)) => x == y,
            (Value::Float(x), Value::Float(y)) => x == y,
            (Value::F32(x), Value::F32(y)) => x == y,
            (Value::Bool(x), Value::Bool(y)) => x == y,
            (Value::Char(x), Value::Char(y)) => x == y,
            (Value::Unit, Value::Unit) => true,
            (Value::DateTime(x), Value::DateTime(y)) => x == y,
            (
                Value::Str(..) | Value::String(_) | Value::Path(_),
                Value::Str(..) | Value::String(_) | Value::Path(_),
            ) => self.text(a)? == self.text(b)?,
            (Value::Tuple(xs), Value::Tuple(ys)) | (Value::List(xs), Value::List(ys)) => {
                self.all_equal(xs, ys)?
            }
            (Value::Struct(x), Value::Struct(y)) | (Value::Variant(x), Value::Variant(y)) => {
                x.ty == y.ty && x.variant == y.variant && self.all_equal(&x.fields, &y.fields)?
            }
            (Value::Set(x), Value::Set(y)) => {
                x.items.len() == y.items.len() && x.index.keys().all(|k| y.index.contains_key(k))
            }
            (Value::Dict(x), Value::Dict(y)) => {
                if x.entries.len() != y.entries.len() {
                    return Ok(false);
                }
                for (k, &i) in &x.index {
                    let Some(&j) = y.index.get(k) else { return Ok(false) };
                    if !self.equal(&x.entries[i].1, &y.entries[j].1)? {
                        return Ok(false);
                    }
                }
                true
            }
            (Value::Cap(x), Value::Cap(y)) | (Value::Clock(x), Value::Clock(y)) => x == y,
            (Value::Fn(x), Value::Fn(y)) => x == y,
            _ => false,
        })
    }

    fn all_equal(&self, xs: &[Value], ys: &[Value]) -> Result<bool, Unwind> {
        if xs.len() != ys.len() {
            return Ok(false);
        }
        for (x, y) in xs.iter().zip(ys) {
            if !self.equal(x, y)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// The order of two values of an `Ord` type; `None` for NaN and for
    /// values without an order.
    pub(crate) fn compare(&self, a: &Value, b: &Value) -> Result<Option<Ordering>, Unwind> {
        let (a, b) = (self.target(a)?, self.target(b)?);
        Ok(match (a, b) {
            (Value::Int(x), Value::Int(y)) => Some(x.cmp(y)),
            (Value::U64(x), Value::U64(y)) => Some(x.cmp(y)),
            (Value::I128(x), Value::I128(y)) => Some(x.cmp(y)),
            (Value::U128(x), Value::U128(y)) => Some(x.cmp(y)),
            (Value::Bytes(x), Value::Bytes(y)) => Some(x.cmp(y)),
            (Value::Float(x), Value::Float(y)) => x.partial_cmp(y),
            (Value::F32(x), Value::F32(y)) => x.partial_cmp(y),
            (Value::Bool(x), Value::Bool(y)) => Some(x.cmp(y)),
            (Value::Char(x), Value::Char(y)) => Some(x.cmp(y)),
            (Value::Unit, Value::Unit) => Some(Ordering::Equal),
            (Value::DateTime(x), Value::DateTime(y)) => Some(x.cmp(y)),
            (
                Value::Str(..) | Value::String(_) | Value::Path(_),
                Value::Str(..) | Value::String(_) | Value::Path(_),
            ) => Some(self.text(a)?.cmp(self.text(b)?)),
            (Value::Tuple(xs), Value::Tuple(ys)) | (Value::List(xs), Value::List(ys)) => {
                self.lexicographic(xs, ys)?
            }
            (Value::Struct(x), Value::Struct(y)) | (Value::Variant(x), Value::Variant(y)) => {
                match x.variant.cmp(&y.variant) {
                    Ordering::Equal => self.lexicographic(&x.fields, &y.fields)?,
                    o => Some(o),
                }
            }
            _ => None,
        })
    }

    fn lexicographic(&self, xs: &[Value], ys: &[Value]) -> Result<Option<Ordering>, Unwind> {
        for (x, y) in xs.iter().zip(ys) {
            match self.compare(x, y)? {
                Some(Ordering::Equal) => {}
                other => return Ok(other),
            }
        }
        Ok(Some(xs.len().cmp(&ys.len())))
    }

    /// The order for sorting: like [`Self::compare`], but NaN panics.
    pub(crate) fn total_compare(&self, a: &Value, b: &Value) -> Result<Ordering, Unwind> {
        self.compare(a, b)?.ok_or_else(|| panic(msg::PANIC_UNORDERED, &[]))
    }

    pub(crate) fn key_of(&self, v: &Value) -> Result<Key, Unwind> {
        Ok(match self.target(v)? {
            Value::Unit => Key::Unit,
            Value::Int(i) => Key::Int(*i),
            Value::U64(i) => Key::U64(*i),
            Value::I128(i) => Key::I128(*i),
            Value::U128(i) => Key::U128(*i),
            Value::Bytes(b) => Key::Bytes(b.clone()),
            Value::Bool(b) => Key::Bool(*b),
            Value::Char(c) => Key::Char(*c),
            Value::Str(s, _) => Key::Text(s.clone()),
            Value::String(s) => Key::Text(Rc::from(s.as_str())),
            Value::Path(p) => Key::Path(p.clone()),
            Value::DateTime(t) => Key::Int(*t),
            Value::Tuple(xs) => Key::Tuple(xs.iter().map(|x| self.key_of(x)).collect::<Result<_, _>>()?),
            Value::Struct(a) | Value::Variant(a) => Key::Adt(
                a.ty.clone(),
                a.variant,
                a.fields.iter().map(|x| self.key_of(x)).collect::<Result<_, _>>()?,
            ),
            Value::Cap(_) | Value::Clock(_) => Key::Handle,
            other => return Err(ice(format!("{other:?} is not hashable"))),
        })
    }

    /// `item in container`.
    pub(crate) fn contains(&self, container: &Value, item: &Value) -> Result<bool, Unwind> {
        match self.target(container)? {
            Value::List(xs) => {
                for x in xs {
                    if self.equal(x, item)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            Value::Set(s) => Ok(s.index.contains_key(&self.key_of(item)?)),
            Value::Dict(d) => Ok(d.index.contains_key(&self.key_of(item)?)),
            c @ (Value::Str(..) | Value::String(_)) => {
                let text = self.text(c)?;
                match self.target(item)? {
                    Value::Char(ch) => Ok(text.contains(*ch)),
                    other => Ok(text.contains(self.text(other)?)),
                }
            }
            other => Err(ice(format!("`in` on {other:?}"))),
        }
    }
}

fn symbol(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::FloorDiv => "//",
        BinOp::Mod => "%",
        BinOp::Pow => "**",
        BinOp::Shl => "<<",
        BinOp::Shr => ">>",
        _ => op.name(),
    }
}

/// Integer arithmetic: overflow and division by zero panic (D-009); `//`
/// floors and `%` has the sign of the divisor (02, "Operators").
pub(crate) fn int_op(op: BinOp, x: i64, y: i64) -> Result<i64, Unwind> {
    let overflow = || panic(msg::PANIC_OVERFLOW, &[("expr", format!("{x} {} {y}", symbol(op)))]);
    match op {
        BinOp::Add => x.checked_add(y).ok_or_else(overflow),
        BinOp::Sub => x.checked_sub(y).ok_or_else(overflow),
        BinOp::Mul => x.checked_mul(y).ok_or_else(overflow),
        BinOp::Div | BinOp::FloorDiv => {
            if y == 0 {
                return Err(panic(msg::PANIC_DIVIDE_BY_ZERO, &[]));
            }
            let q = x.checked_div(y).ok_or_else(overflow)?;
            Ok(if x % y != 0 && ((x < 0) != (y < 0)) { q - 1 } else { q })
        }
        BinOp::Mod => {
            if y == 0 {
                return Err(panic(msg::PANIC_DIVIDE_BY_ZERO, &[]));
            }
            let r = x.checked_rem(y).unwrap_or(0);
            Ok(if r != 0 && ((r < 0) != (y < 0)) { r + y } else { r })
        }
        BinOp::Pow => {
            if y < 0 {
                return Err(panic(msg::PANIC_NEGATIVE_EXPONENT, &[("expr", format!("{x} ** {y}"))]));
            }
            match u32::try_from(y) {
                Ok(e) => x.checked_pow(e).ok_or_else(overflow),
                Err(_) => match x {
                    0 | 1 => Ok(x),
                    -1 => Ok(if y % 2 == 0 { 1 } else { -1 }),
                    _ => Err(overflow()),
                },
            }
        }
        BinOp::WrapAdd => Ok(x.wrapping_add(y)),
        BinOp::WrapSub => Ok(x.wrapping_sub(y)),
        BinOp::WrapMul => Ok(x.wrapping_mul(y)),
        BinOp::SatAdd => Ok(x.saturating_add(y)),
        BinOp::SatSub => Ok(x.saturating_sub(y)),
        BinOp::SatMul => Ok(x.saturating_mul(y)),
        BinOp::Shl | BinOp::Shr => {
            if !(0..64).contains(&y) {
                return Err(panic(msg::PANIC_SHIFT_OUT_OF_RANGE, &[("amount", y.to_string())]));
            }
            Ok(if op == BinOp::Shl { x << y } else { x >> y })
        }
        BinOp::BitAnd => Ok(x & y),
        BinOp::BitOr => Ok(x | y),
        BinOp::BitXor => Ok(x ^ y),
        _ => Err(ice(format!("{} on integers", op.name()))),
    }
}

/// Float arithmetic is IEEE 754: no trap, division by zero gives an
/// infinity or NaN.
pub(crate) fn float_op(op: BinOp, x: f64, y: f64) -> f64 {
    match op {
        BinOp::Add => x + y,
        BinOp::Sub => x - y,
        BinOp::Mul => x * y,
        BinOp::Div => x / y,
        BinOp::FloorDiv => (x / y).floor(),
        BinOp::Mod => x - y * (x / y).floor(),
        BinOp::Pow => x.powf(y),
        _ => f64::NAN,
    }
}
