//! A type's constant argument written as an expression (03, "Const
//! generics": `struct Grid[T, const W: int, const H: int]` with `cells:
//! Array[T, W * H]`). Name resolution and type checking need the value, so
//! the compiler computes it where the type is written: integers, `const`
//! parameters, the program's constants whose values are written that way,
//! and the integer operators (`+ - * // % ** << >> & | ^`, unary `-` and
//! `~`), with the run time's rules (`sisu_sir::ConstOp::fold`). An
//! operation on a `const` parameter stays an operation (`Ty::ConstOp`)
//! until a use gives the parameter a constant.

use crate::program::{DefKind, ModuleId, Program};
use crate::ty::Ty;
use sisu_diag::{Catalog, Diagnostic, Span, TextRange, msg};
use sisu_sir::{ConstFault, ConstOp};
use sisu_syntax::ast::{self, BinOp, ExprKind, UnOp};

/// Why an expression is not a constant argument.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConstError {
    /// Not made of integers, `const` parameters, constants and the integer
    /// operators (a call, a variable, `comptime`, `/`).
    NotConstant(TextRange),
    /// A value of another type than `int`, named.
    NotInt(TextRange, String),
    /// A name that names nothing.
    Undefined(TextRange, String),
    /// An integer literal beyond `int`.
    OutOfRange(TextRange),
    /// An operation without a value (overflow, a zero divisor, a negative
    /// exponent, a shift out of range), shown with its operands.
    Fault(TextRange, ConstFault, String),
}

/// What a name means where the argument is written, before the program's
/// items are looked at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Local {
    /// The `const` generic parameter with this index.
    ConstParam(u32),
    /// A type parameter, or a value known only at run time (a local, a
    /// parameter): not a constant.
    NotConstant,
    /// Nothing in scope: an item of the program, if anything.
    Item,
}

/// What a name of the scope where the argument is written means.
pub type ParamOf<'a> = &'a dyn Fn(&str) -> Local;

/// The constant (or the operation on `const` parameters) that `e`, written
/// in `module` where `param` names the generic parameters, computes.
pub fn const_arg(prog: &Program, module: ModuleId, e: &ast::Expr, param: ParamOf) -> Result<Ty, ConstError> {
    eval(prog, module, e, param, 0)
}

fn eval(
    prog: &Program,
    module: ModuleId,
    e: &ast::Expr,
    param: ParamOf,
    depth: u32,
) -> Result<Ty, ConstError> {
    let r = e.range;
    if depth > 32 {
        return Err(ConstError::NotConstant(r));
    }
    let not_int = |ty: &str| Err(ConstError::NotInt(r, ty.to_string()));
    match &e.kind {
        ExprKind::Int { value, suffix, .. } => match suffix.as_deref() {
            None | Some("int" | "i64") => match value.and_then(|v| i64::try_from(v).ok()) {
                Some(n) => Ok(Ty::Const(i128::from(n))),
                None => Err(ConstError::OutOfRange(r)),
            },
            Some("uint") => not_int("u64"),
            Some(s) => not_int(s),
        },
        ExprKind::Float { suffix, .. } => not_int(suffix.as_deref().unwrap_or("f64")),
        ExprKind::Char(_) => not_int("char"),
        ExprKind::Str(_) | ExprKind::RawStr(_) => not_int("str"),
        ExprKind::ByteStr(_) => not_int("bytes"),
        ExprKind::Compare { .. } => not_int("bool"),
        ExprKind::Name(n) if n == "True" || n == "False" => not_int("bool"),
        ExprKind::Paren(inner) => eval(prog, module, inner, param, depth),
        ExprKind::Name(n) => match param(n) {
            Local::ConstParam(i) => Ok(Ty::Param(i)),
            Local::NotConstant => Err(ConstError::NotConstant(r)),
            Local::Item => named(prog, module, n, r, depth),
        },
        ExprKind::Unary { op, expr } => {
            let op = match op {
                UnOp::Neg => ConstOp::Neg,
                UnOp::BitNot => ConstOp::BitNot,
                UnOp::Not => return not_int("bool"),
            };
            let x = eval(prog, module, expr, param, depth + 1)?;
            apply(op, vec![x], r)
        }
        ExprKind::Binary { op, lhs, rhs } => {
            let op = match op {
                BinOp::Add => ConstOp::Add,
                BinOp::Sub => ConstOp::Sub,
                BinOp::Mul => ConstOp::Mul,
                BinOp::FloorDiv => ConstOp::FloorDiv,
                BinOp::Mod => ConstOp::Mod,
                BinOp::Pow => ConstOp::Pow,
                BinOp::Shl => ConstOp::Shl,
                BinOp::Shr => ConstOp::Shr,
                BinOp::BitAnd => ConstOp::BitAnd,
                BinOp::BitOr => ConstOp::BitOr,
                BinOp::BitXor => ConstOp::BitXor,
                BinOp::And | BinOp::Or => return not_int("bool"),
                _ => return Err(ConstError::NotConstant(r)),
            };
            let x = eval(prog, module, lhs, param, depth + 1)?;
            let y = eval(prog, module, rhs, param, depth + 1)?;
            apply(op, vec![x, y], r)
        }
        _ => Err(ConstError::NotConstant(r)),
    }
}

/// The value of the program's constant `name`, written at `r` where a
/// type's constant argument goes: `SIZE` in `Array[int, SIZE]`.
pub fn const_item(prog: &Program, module: ModuleId, name: &str, r: TextRange) -> Result<Ty, ConstError> {
    named(prog, module, name, r, 0)
}

fn named(prog: &Program, module: ModuleId, n: &str, r: TextRange, depth: u32) -> Result<Ty, ConstError> {
    let Some(d) = prog.lookup(module, n) else { return Err(ConstError::Undefined(r, n.to_string())) };
    let DefKind::Const(info) = &prog.def(d).kind else { return Err(ConstError::NotConstant(r)) };
    let Some(value) = &info.value else { return Err(ConstError::NotConstant(r)) };
    // The constant's own value, computed where it is written; what is wrong
    // with it is wrong with this use of it.
    match eval(prog, prog.def(d).module, value, &|_| Local::Item, depth + 1) {
        Ok(t) => Ok(t),
        Err(ConstError::NotInt(_, ty)) => Err(ConstError::NotInt(r, ty)),
        Err(_) => Err(ConstError::NotConstant(r)),
    }
}

/// `op` on its operands: their value when they are constants, the
/// operation otherwise.
fn apply(op: ConstOp, args: Vec<Ty>, r: TextRange) -> Result<Ty, ConstError> {
    let values: Option<Vec<i128>> =
        args.iter().map(|a| if let Ty::Const(n) = a { Some(*n) } else { None }).collect();
    let Some(values) = values else { return Ok(Ty::ConstOp(op, args)) };
    op.fold(&values).map(Ty::Const).map_err(|fault| {
        let shown = match values.as_slice() {
            [x] => format!("{}({x})", op.symbol()),
            [x, y] => format!("{x} {} {y}", op.symbol()),
            _ => String::new(),
        };
        ConstError::Fault(r, fault, shown)
    })
}

/// The diagnostic of `err`, with `span` placing a range in the file.
pub fn diagnostic(err: &ConstError, span: impl Fn(TextRange) -> Span) -> Diagnostic {
    match err {
        ConstError::NotConstant(r) => Diagnostic::new(msg::RESOLVE_CONST_EXPRESSION, span(*r)),
        ConstError::NotInt(r, ty) => {
            Diagnostic::new(msg::RESOLVE_CONST_NOT_INT, span(*r)).with("ty", ty.clone())
        }
        ConstError::Undefined(r, name) => {
            Diagnostic::new(msg::RESOLVE_UNDEFINED_NAME, span(*r)).with("name", name.clone())
        }
        ConstError::OutOfRange(r) => Diagnostic::new(msg::TYPE_LITERAL_OUT_OF_RANGE, span(*r))
            .with("text", "the literal")
            .with("ty", "int"),
        ConstError::Fault(r, fault, shown) => {
            let message = match fault {
                ConstFault::Overflow => {
                    Catalog::builtin().render(msg::PANIC_OVERFLOW.id(), &[("expr", shown.clone())])
                }
                ConstFault::DivideByZero => Catalog::builtin().render(msg::PANIC_DIVIDE_BY_ZERO.id(), &[]),
                ConstFault::NegativeExponent => {
                    Catalog::builtin().render(msg::PANIC_NEGATIVE_EXPONENT.id(), &[("expr", shown.clone())])
                }
                ConstFault::ShiftOutOfRange => {
                    let amount = shown.rsplit(' ').next().unwrap_or("").to_string();
                    Catalog::builtin().render(msg::PANIC_SHIFT_OUT_OF_RANGE.id(), &[("amount", amount)])
                }
            };
            Diagnostic::new(msg::COMPTIME_PANICKED, span(*r)).with("message", message)
        }
    }
}
