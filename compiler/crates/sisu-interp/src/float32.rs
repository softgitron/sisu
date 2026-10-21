//! `f32` (03, "Numbers"): IEEE 754 binary32, a value of its own kind
//! (`Value::F32`). Its arithmetic is the intrinsics `f32.op`, each computed
//! in `f64` and rounded once to `f32`: `f64` holds every `f32` exactly and
//! has more than twice its bits, so `+`, `-`, `*`, `/` and the square
//! root round as `f32` arithmetic would (`%` is exact in both).

use crate::value::Value;
use crate::{Interp, Unwind, ice, panic};
use sisu_diag::msg;

impl Interp<'_, '_> {
    fn f32_of(&self, v: Option<&Value>) -> Result<f32, Unwind> {
        match v.map(|v| self.target(v)).transpose()? {
            Some(Value::F32(f)) => Ok(*f),
            other => Err(ice(format!("expected an f32, found {other:?}"))),
        }
    }

    /// The intrinsics `f32.*` and `f64.to_f32`; `None` for another.
    pub(crate) fn f32_intrinsic(&mut self, name: &str, args: &[Value]) -> Option<Result<Value, Unwind>> {
        let method =
            name.strip_prefix("f32.").or(if name == "f64.to_f32" { Some("from_f64") } else { None })?;
        Some(self.f32_op(method, args))
    }

    fn f32_op(&mut self, method: &str, args: &[Value]) -> Result<Value, Unwind> {
        let wide = |x: f32| f64::from(x);
        let binary = |s: &Self, f: fn(f64, f64) -> f64| -> Result<Value, Unwind> {
            let (a, b) = (s.f32_of(args.first())?, s.f32_of(args.get(1))?);
            Ok(Value::F32(f(wide(a), wide(b)) as f32))
        };
        Ok(match method {
            "add" => binary(self, |a, b| a + b)?,
            "sub" => binary(self, |a, b| a - b)?,
            "mul" => binary(self, |a, b| a * b)?,
            "div" => binary(self, |a, b| a / b)?,
            "rem" => binary(self, |a, b| a % b)?,
            "neg" => Value::F32(-self.f32_of(args.first())?),
            "cmp" => {
                let (a, b) = (self.f32_of(args.first())?, self.f32_of(args.get(1))?);
                Value::ordering(a.partial_cmp(&b).ok_or_else(|| panic(msg::PANIC_UNORDERED, &[]))?)
            }
            "to_f64" => Value::Float(wide(self.f32_of(args.first())?)),
            "sqrt" => Value::F32(wide(self.f32_of(args.first())?).sqrt() as f32),
            "floor" => Value::F32(self.f32_of(args.first())?.floor()),
            "ceil" => Value::F32(self.f32_of(args.first())?.ceil()),
            "round" => Value::F32(self.f32_of(args.first())?.round()),
            "abs" => Value::F32(self.f32_of(args.first())?.abs()),
            "is_nan" => Value::Bool(self.f32_of(args.first())?.is_nan()),
            "to_bits" => Value::Int(i64::from(self.f32_of(args.first())?.to_bits())),
            "from_bits" => {
                let bits = match args.first().map(|v| self.target(v)).transpose()? {
                    Some(Value::Int(n)) => u32::try_from(*n).map_err(|_| ice("f32.from_bits beyond u32"))?,
                    other => return Err(ice(format!("f32.from_bits of {other:?}"))),
                };
                Value::F32(f32::from_bits(bits))
            }
            // A literal, already the `f32` it names (the checker rounded
            // its text once), or an `f64` rounded to the nearest.
            "from_literal" | "from_f64" => match args.first().map(|v| self.target(v)).transpose()? {
                Some(Value::Float(f)) => Value::F32(*f as f32),
                other => return Err(ice(format!("f32.{method} of {other:?}"))),
            },
            other => return Err(ice(format!("unknown intrinsic `f32.{other}`"))),
        })
    }
}
