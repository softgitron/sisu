//! `comptime assert` on `const` generic parameters (08, "comptime assert";
//! 03, "Const generics"): `comptime assert N > 0 and N & (N - 1) == 0,
//! "capacity {N} must be a power of two"` in a generic type's method can
//! be decided only for constant arguments, so it is checked at each call
//! that gives them, once the whole program is typed: its condition and
//! message are folded with the call's arguments for the parameters, and a
//! false condition is the call's compile-time error. Stage 0 folds what
//! such checks are made of, integer and boolean arithmetic, comparisons
//! and interpolated text; a condition that calls a function is checked
//! nowhere (the run never sees it either), which `sisu check --explain`
//! does not show yet.

use crate::purity::visit_block;
use crate::thir::{Body, BodyKind, Callee, Expr, ExprKind, GenericAssert, InterpPart, Typed};
use sisu_diag::{Catalog, Diagnostic, msg};
use sisu_resolve::{DefId, Ty};
use sisu_syntax::ast::{BinOp, CmpOp, UnOp};
use std::collections::HashMap;

/// A folded value.
#[derive(Clone, Debug, PartialEq)]
enum Value {
    Int(i128),
    Bool(bool),
    Text(String),
}

impl Value {
    fn show(&self) -> String {
        match self {
            Value::Int(n) => n.to_string(),
            Value::Bool(b) => if *b { "True" } else { "False" }.to_string(),
            Value::Text(t) => t.clone(),
        }
    }
}

/// The value of an expression with the call's type arguments for the
/// function's generic parameters, when it is made of constants.
fn fold(e: &Expr, args: &[Ty]) -> Option<Value> {
    let int = |x: &Expr| match fold(x, args)? {
        Value::Int(n) => Some(n),
        _ => None,
    };
    let boolean = |x: &Expr| match fold(x, args)? {
        Value::Bool(b) => Some(b),
        _ => None,
    };
    Some(match &e.kind {
        ExprKind::Int(n) => Value::Int(i128::from(*n)),
        ExprKind::Bool(b) => Value::Bool(*b),
        ExprKind::Str(s) => Value::Text(s.clone()),
        ExprKind::ConstParam(i) => match args.get(*i as usize)? {
            Ty::Const(n) => Value::Int(*n),
            _ => return None,
        },
        ExprKind::Coerce { expr, .. } => return fold(expr, args),
        ExprKind::Unary { op, operand } => match op {
            UnOp::Neg => Value::Int(int(operand)?.checked_neg()?),
            UnOp::BitNot => Value::Int(!int(operand)?),
            UnOp::Not => Value::Bool(!boolean(operand)?),
        },
        ExprKind::Binary { op, lhs, rhs } => {
            let (a, b) = (int(lhs)?, int(rhs)?);
            Value::Int(match op {
                BinOp::Add | BinOp::WrapAdd => a.checked_add(b)?,
                BinOp::Sub | BinOp::WrapSub => a.checked_sub(b)?,
                BinOp::Mul | BinOp::WrapMul => a.checked_mul(b)?,
                BinOp::FloorDiv if b != 0 => a.div_euclid(b) - i128::from(a.rem_euclid(b) != 0 && b < 0),
                BinOp::Mod if b != 0 => a - b * (a.div_euclid(b) - i128::from(a.rem_euclid(b) != 0 && b < 0)),
                BinOp::BitAnd => a & b,
                BinOp::BitOr => a | b,
                BinOp::BitXor => a ^ b,
                BinOp::Shl if (0..64).contains(&b) => a.checked_shl(b as u32)?,
                BinOp::Shr if (0..64).contains(&b) => a >> b,
                _ => return None,
            })
        }
        ExprKind::Logic { and, lhs, rhs } => {
            let a = boolean(lhs)?;
            Value::Bool(if *and { a && boolean(rhs)? } else { a || boolean(rhs)? })
        }
        ExprKind::Compare { first, rest } => {
            let mut left = int(first)?;
            for (op, x) in rest {
                let right = int(x)?;
                let holds = match op {
                    CmpOp::Eq => left == right,
                    CmpOp::Ne => left != right,
                    CmpOp::Lt => left < right,
                    CmpOp::Le => left <= right,
                    CmpOp::Gt => left > right,
                    CmpOp::Ge => left >= right,
                    CmpOp::In | CmpOp::NotIn => return None,
                };
                if !holds {
                    return Some(Value::Bool(false));
                }
                left = right;
            }
            Value::Bool(true)
        }
        ExprKind::Interp(parts) => {
            let mut out = String::new();
            for p in parts {
                match p {
                    InterpPart::Text(t) => out.push_str(t),
                    InterpPart::Value { expr, spec, .. } if spec.is_empty() => {
                        out.push_str(&fold(expr, args)?.show())
                    }
                    InterpPart::Value { .. } => return None,
                }
            }
            Value::Text(out)
        }
        _ => return None,
    })
}

/// The calls whose constant arguments make a `comptime assert` of the
/// callee false: one diagnostic at each.
pub(crate) fn check_calls(typed: &Typed) -> Vec<Diagnostic> {
    let asserts: HashMap<DefId, &[GenericAssert]> = typed
        .bodies
        .iter()
        .filter(|b| b.kind == BodyKind::Fn && !b.generic_asserts.is_empty())
        .map(|b| (b.def, b.generic_asserts.as_slice()))
        .collect();
    let mut out = Vec::new();
    if asserts.is_empty() {
        return out;
    }
    for body in &typed.bodies {
        let mut visit = |e: &Expr| {
            let ExprKind::Call { callee: Callee::Fn { def, type_args }, .. } = &e.kind else { return };
            let Some(list) = asserts.get(def) else { return };
            for a in list.iter() {
                if fold(&a.cond, type_args) != Some(Value::Bool(false)) {
                    continue;
                }
                let message = match a.message.as_ref().and_then(|m| fold(m, type_args)) {
                    Some(m) => Catalog::builtin().render(
                        msg::PANIC_ASSERT_MESSAGE.id(),
                        &[("expr", a.text.clone()), ("message", m.show())],
                    ),
                    None => Catalog::builtin().render(msg::PANIC_ASSERT.id(), &[("expr", a.text.clone())]),
                };
                out.push(Diagnostic::new(msg::COMPTIME_PANICKED, e.span).with("message", message));
            }
        };
        walk(body, &mut visit);
    }
    out
}

fn walk<'a>(body: &'a Body, f: &mut dyn FnMut(&'a Expr)) {
    visit_block(&body.block, f);
    for c in &body.closures {
        visit_block(&c.body, f);
    }
}
