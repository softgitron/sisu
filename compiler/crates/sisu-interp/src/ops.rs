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
                // again to `f32` (it has more than twice the bits); `%`
                // and `**` are `f64`'s, rounded once to `f32`.
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
            // Two `Shared` compare their values, as std's `Eq` impl does
            // (decision 464); a cell that is gone only to itself.
            (Value::Shared(x), Value::Shared(y)) => match (self.live_shared(*x), self.live_shared(*y)) {
                (Some(a), Some(b)) => self.equal(a, b)?,
                _ => x == y,
            },
            _ => false,
        })
    }

    /// The value a `Shared` cell holds while it is live.
    fn live_shared(&self, id: u32) -> Option<&Value> {
        self.heap.get(id as usize).filter(|c| c.strong > 0).map(|c| &c.value)
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
            Value::List(xs) => Key::List(xs.iter().map(|x| self.key_of(x)).collect::<Result<_, _>>()?),
            Value::Struct(a) | Value::Variant(a) => Key::Adt(
                a.ty.clone(),
                a.variant,
                a.fields.iter().map(|x| self.key_of(x)).collect::<Result<_, _>>()?,
            ),
            Value::Cap(_) | Value::Clock(_) => Key::Handle,
            // A `Shared` is the key its value is (decision 464).
            Value::Shared(id) => match self.live_shared(*id) {
                Some(inner) => self.key_of(inner)?,
                None => Key::Handle,
            },
            other => return Err(ice(format!("{other:?} is not hashable"))),
        })
    }

    /// The structural hash of `v` (decision 256): its parts mixed into
    /// `state` as `Hasher.write_int` mixes an `int`, `(state ^ n) *%
    /// 1099511628211`: an integer of up to 64 bits, a `char`'s code, a
    /// `bool` as 0 or 1 and a `DateTime`'s nanoseconds as one `int`; a
    /// 128-bit integer as its low word, then its high one; a text or
    /// `bytes` as its length in bytes, then each byte; a tuple's and a
    /// struct's fields in order; an enum's variant index, then its fields;
    /// a list's length, then its elements; a float as its bits; `()` and
    /// a handle as nothing. These are the parts the derived impls and the
    /// prelude's own impls hash, so a structural hash and a written one
    /// agree.
    pub(crate) fn structural_hash(&self, state: &mut i64, v: &Value) -> Result<(), Unwind> {
        fn mix(state: &mut i64, n: i64) {
            *state = (*state ^ n).wrapping_mul(1_099_511_628_211);
        }
        fn text(state: &mut i64, b: &[u8]) {
            mix(state, b.len() as i64);
            for x in b {
                mix(state, i64::from(*x));
            }
        }
        match self.target(v)? {
            Value::Unit | Value::Cap(_) | Value::Clock(_) => {}
            // A `Shared` hashes as its value (decision 464).
            Value::Shared(id) => {
                if let Some(inner) = self.live_shared(*id) {
                    self.structural_hash(state, inner)?;
                }
            }
            Value::Int(i) | Value::DateTime(i) => mix(state, *i),
            Value::U64(i) => mix(state, *i as i64),
            Value::I128(i) => {
                mix(state, *i as i64);
                mix(state, (*i >> 64) as i64);
            }
            Value::U128(i) => {
                mix(state, *i as i64);
                mix(state, (*i >> 64) as i64);
            }
            Value::Float(f) => mix(state, f.to_bits() as i64),
            Value::F32(f) => mix(state, i64::from(f.to_bits())),
            Value::Bool(b) => mix(state, i64::from(*b)),
            Value::Char(c) => mix(state, i64::from(u32::from(*c))),
            Value::Str(s, _) => text(state, s.as_bytes()),
            Value::String(s) => text(state, s.as_bytes()),
            Value::Path(s) => text(state, s.as_bytes()),
            Value::Bytes(b) => text(state, b),
            Value::Tuple(xs) => {
                for x in xs {
                    self.structural_hash(state, x)?;
                }
            }
            Value::List(xs) => {
                mix(state, xs.len() as i64);
                for x in xs {
                    self.structural_hash(state, x)?;
                }
            }
            Value::Struct(a) => {
                for x in &a.fields {
                    self.structural_hash(state, x)?;
                }
            }
            Value::Variant(a) => {
                mix(state, i64::from(a.variant));
                for x in &a.fields {
                    self.structural_hash(state, x)?;
                }
            }
            other => return Err(ice(format!("{other:?} is not hashable"))),
        }
        Ok(())
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
        BinOp::Mod => float_mod(x, y),
        BinOp::Pow => x.powf(y),
        _ => f64::NAN,
    }
}

/// `x % y` on floats (decision 18): the remainder of the exact division
/// (IEEE 754's `fmod`, which is exact), moved to the divisor's sign by
/// adding the divisor once, which is the only rounding; a zero remainder
/// takes the divisor's sign. Python's rule, so `1e17 % 3.0` is `1.0`.
pub(crate) fn float_mod(x: f64, y: f64) -> f64 {
    let r = x % y;
    if r == 0.0 {
        0.0f64.copysign(y)
    } else if (r < 0.0) != (y < 0.0) {
        r + y
    } else {
        r
    }
}
