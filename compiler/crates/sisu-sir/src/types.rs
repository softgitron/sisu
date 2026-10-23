//! Types of places, for the passes that read SIR (the ownership checker,
//! the interpreter's drops), and the name of a type's drop glue.

use crate::{Function, Place, Proj, Type, TypeDecl};
use std::collections::HashMap;

/// The drop glue of the declared type `ty`: the function `ty#drop`, of
/// kind `glue`, taking the value with `own`.
pub fn glue_name(ty: &str) -> String {
    format!("{ty}#drop")
}

/// Whether dropping a value of the type can do anything: run a `Drop`,
/// change a count, unlock a mutex. `needs` says which declared types have
/// drop glue (lowering's, which leaves out a field whose drop does
/// nothing, so a native runtime frees such a field itself).
pub fn drops_something(t: &Type, needs: &HashMap<&str, bool>) -> bool {
    match t {
        Type::Int | Type::Float | Type::Bool | Type::Char | Type::Str | Type::Unit | Type::Never => false,
        // A reference owns nothing; a closure holds references, or copies
        // of `Copy` values (an `own` closure).
        Type::Ref(..) | Type::Fn(..) | Type::Const(_) | Type::ConstOp(..) => false,
        Type::Tuple(es) => es.iter().any(|e| drops_something(e, needs)),
        Type::Param(_) | Type::Assoc(..) | Type::Dyn(..) => true,
        Type::ErrSet(ms) => ms.iter().any(|m| needs.get(m.as_str()).copied().unwrap_or(true)),
        Type::Named(n, args) => match n.as_str() {
            // A capability handle's and a `Cap`'s copies are counted, and
            // the last one's drop drops the object (decision 237).
            "Shared" | "Weak" | "MutexGuard" | "ReadGuard" | "Fs" | "Net" | "Clock" | "Rand" | "Env"
            | "Proc" | "Stdin" | "System" | "Cap" => true,
            "String" | "Path" | "DateTime" | "HostFs" | "HostNet" | "HostClock" | "HostRand" | "HostEnv"
            | "HostProc" | "HostStdin" | "TestClock" | "Lent" | "f32" | "u8" | "u16" | "u32" | "u64"
            | "u128" | "i8" | "i16" | "i32" | "i128" | "bytes" | "Slice" => false,
            "list" | "Array" | "dict" | "set" | "Iter" | "Entry" | "Iterable" | "Option" | "Result"
            | "Cell" | "Atomic" | "Mutex" | "RwLock" | "Arena" => {
                args.iter().any(|a| drops_something(a, needs))
            }
            other => needs.get(other).copied().unwrap_or(true),
        },
    }
}

/// The type of a place in `f`, or `None` when SIR does not say (a payload
/// of a union of error sets). `decl` finds a declared type by name.
pub fn place_ty<'d>(decl: &dyn Fn(&str) -> Option<&'d TypeDecl>, f: &Function, p: &Place) -> Option<Type> {
    let mut t = f.locals.get(p.local.0 as usize)?.ty.clone();
    let mut variant: Option<u32> = None;
    for proj in &p.proj {
        t = match proj {
            Proj::Deref => match t {
                Type::Ref(inner, _) => *inner,
                // A `Shared` and a mutex or read guard lead to the value.
                Type::Named(n, args) if n == "Shared" || n == "MutexGuard" || n == "ReadGuard" => {
                    args.into_iter().next()?
                }
                _ => return None,
            },
            Proj::Downcast(v) => {
                variant = Some(*v);
                continue;
            }
            Proj::Field(i) => {
                let ft = field_ty(decl, &t, variant, *i)?;
                variant = None;
                ft
            }
            Proj::Index(_) | Proj::IndexFromEnd(_) => match t {
                Type::Named(n, args) if n == "list" || n == "Array" => args.into_iter().next()?,
                _ => return None,
            },
            Proj::Key(_) => match t {
                Type::Named(n, args) if n == "dict" => args.into_iter().nth(1)?,
                Type::Named(n, args) if n == "Arena" => args.into_iter().next()?,
                _ => return None,
            },
        };
    }
    Some(t)
}

/// Field `i` of a struct or tuple, or of variant `v` of an enum.
pub fn field_ty<'d>(
    decl: &dyn Fn(&str) -> Option<&'d TypeDecl>,
    t: &Type,
    variant: Option<u32>,
    i: u32,
) -> Option<Type> {
    match (t, variant) {
        (Type::Tuple(es), None) => es.get(i as usize).cloned(),
        (Type::Named(n, args), Some(v)) if n == "Option" => match v {
            0 if i == 0 => args.first().cloned(),
            _ => None,
        },
        (Type::Named(n, args), Some(v)) if n == "Result" => match (v, i) {
            (0, 0) => args.first().cloned(),
            (1, 0) => args.get(1).cloned(),
            _ => None,
        },
        (Type::Named(n, args), None) => match decl(n)? {
            TypeDecl::Struct { fields, .. } => fields.get(i as usize).map(|f| f.ty.subst(args)),
            TypeDecl::Enum { .. } => None,
        },
        (Type::Named(n, args), Some(v)) => match decl(n)? {
            TypeDecl::Enum { variants, .. } => {
                variants.get(v as usize)?.fields.get(i as usize).map(|(_, t)| t.subst(args))
            }
            TypeDecl::Struct { .. } => None,
        },
        (Type::ErrSet(ms), Some(v)) if ms.len() == 1 => match decl(&ms[0])? {
            TypeDecl::Enum { variants, .. } => {
                variants.get(v as usize)?.fields.get(i as usize).map(|(_, t)| t.clone())
            }
            TypeDecl::Struct { .. } => None,
        },
        _ => None,
    }
}
