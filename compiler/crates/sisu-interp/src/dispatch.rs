//! Calls through a trait (`call trait Trait.method[Self, ...]`): the
//! interpreter finds, at run time, the impl whose patterns match the
//! concrete type arguments (03, "Dispatch"; 18, Phase 4: "type arguments
//! are runtime values, impl lookup by concrete type at call time").
//!
//! A `Self` that is a reference dispatches on what it points to (the
//! reading traits pass through references); a `dyn` value dispatches on
//! the type it carries, with `self` pointing inside it. When no impl
//! matches, the trait is one the language implements itself for the
//! type (error sets, functions, long tuples), structurally.

use crate::value::{HEAP, Pointer, Step, Value};
use crate::{Interp, Unwind, ice};
use sisu_sir as sir;
use sisu_sir::{ImplTarget, Type};
use std::rc::Rc;

/// What a trait call runs, with its arguments and type arguments.
pub(crate) enum Target {
    Fn(usize, Vec<Value>, Vec<Type>),
    Intrinsic(String, Vec<Value>, Vec<Type>),
}

impl Interp<'_, '_> {
    pub(crate) fn dispatch(
        &mut self,
        name: &str,
        mut tys: Vec<Type>,
        mut args: Vec<Value>,
    ) -> Result<Target, Unwind> {
        let (trait_name, method) =
            name.rsplit_once('.').ok_or_else(|| ice(format!("bad trait method `{name}`")))?;
        let mut self_ty = tys.first().cloned().ok_or_else(|| ice(format!("`{name}` without `Self`")))?;
        loop {
            match &self_ty {
                Type::Ref(inner, _) => {
                    // `self` points at a reference: go through it.
                    let p = match args.first() {
                        Some(Value::Ref(p)) => (**p).clone(),
                        other => return Err(ice(format!("`{name}` on a reference without one: {other:?}"))),
                    };
                    let inner_ref = self.read(&p)?.clone();
                    args[0] = inner_ref;
                    self_ty = (**inner).clone();
                }
                // A `dyn` is either owned (a `Value::Dyn` holding the value
                // and its type, reached through a reference for a borrowed
                // `self`) or a `ref dyn` made from a `ref T` (a `Value::Dyn`
                // holding the reference, its type `ref T`).
                Type::Dyn(..) => match args.first_mut() {
                    Some(Value::Ref(p)) => {
                        let p = (**p).clone();
                        let (ty, arg) = match self.read(&p)? {
                            Value::Dyn(d) => match &d.ty {
                                Type::Ref(inner, _) => ((**inner).clone(), d.value.clone()),
                                ty => (ty.clone(), Value::Ref(Rc::new(p.join(Step::DynValue)))),
                            },
                            other => return Err(ice(format!("`{name}` on a `dyn` holding {other:?}"))),
                        };
                        args[0] = arg;
                        self_ty = ty;
                    }
                    Some(v @ Value::Dyn(_)) => {
                        let Value::Dyn(d) = std::mem::replace(v, Value::Moved) else { break };
                        *v = d.value;
                        self_ty = match d.ty {
                            Type::Ref(inner, _) => *inner,
                            ty => ty,
                        };
                    }
                    other => return Err(ice(format!("`{name}` on a `dyn` without one: {other:?}"))),
                },
                // A capability handle forwards to its object, a `dyn` in a
                // heap cell; a `Lent` is its handle.
                Type::Named(n, a) if n == "Lent" && a.len() == 1 => {
                    let inner = a[0].clone();
                    if let Some(Value::Ref(p)) = args.first() {
                        let v = self.read(p)?.clone();
                        args[0] = v;
                    }
                    self_ty = inner;
                }
                Type::Named(n, a)
                    if a.is_empty()
                        && matches!(
                            n.as_str(),
                            "Fs" | "Net" | "Clock" | "Rand" | "Env" | "Proc" | "Stdin"
                        ) =>
                {
                    let id = match self
                        .target(args.first().ok_or_else(|| ice("a handle call without `self`"))?)?
                    {
                        Value::Cap(id) => *id,
                        other => return Err(ice(format!("`{name}` on a handle without one: {other:?}"))),
                    };
                    let cell = self.heap.get(id as usize).ok_or_else(|| ice("a handle without its cell"))?;
                    let ty = match &cell.value {
                        Value::Dyn(d) => d.ty.clone(),
                        other => return Err(ice(format!("a handle's cell holds {other:?}"))),
                    };
                    let p = Pointer {
                        task: 0,
                        frame: HEAP,
                        serial: cell.serial,
                        local: id,
                        path: Vec::new(),
                        tag: 0,
                    };
                    args[0] = Value::Ref(Rc::new(p.join(Step::DynValue)));
                    self_ty = ty;
                }
                _ => break,
            }
        }
        if let Some(t) = tys.first_mut() {
            *t = self_ty.clone();
        }
        let prog = self.prog;
        for imp in &prog.impls {
            if imp.trait_name != trait_name {
                continue;
            }
            let n = imp.trait_args.len();
            let mut binds: Vec<Option<Type>> = vec![None; imp.params as usize];
            if !match_type(&imp.self_ty, &self_ty, &mut binds)
                || !imp.trait_args.iter().zip(tys.iter().skip(1)).all(|(p, t)| match_type(p, t, &mut binds))
            {
                continue;
            }
            let mut targs: Vec<Type> = binds.into_iter().map(|b| b.unwrap_or(Type::Unit)).collect();
            targs.extend(tys.iter().skip(1 + n).cloned());
            return match imp.methods.iter().find(|(m, _)| m == method) {
                Some((_, ImplTarget::Fn(f))) => {
                    let fi = self.fn_index(f)?;
                    self.adapt(fi, &targs, &mut args)?;
                    Ok(Target::Fn(fi, args, targs))
                }
                Some((_, ImplTarget::Intrinsic(i))) => Ok(Target::Intrinsic(i.clone(), args, targs)),
                // The trait's provided method.
                None => {
                    let fi = self.fn_index(name)?;
                    self.adapt(fi, &tys, &mut args)?;
                    Ok(Target::Fn(fi, args, tys))
                }
            };
        }
        // The language's own impls: structural.
        let structural = match (trait_name, method) {
            ("Display", "fmt") => "structural.display",
            ("Debug", "fmt_debug") => "structural.debug",
            ("Eq", "eq") => "structural.eq",
            ("PartialOrd", "partial_cmp") => "structural.partial_cmp",
            ("Ord", "cmp") => "structural.cmp",
            ("Hash", "hash") => "structural.hash",
            _ => return Err(ice(format!("no impl of `{trait_name}` for `{}`", sir::write::ty(&self_ty)))),
        };
        Ok(Target::Intrinsic(structural.into(), args, tys))
    }

    /// A trait passes a parameter whose declared type is a generic
    /// parameter by reference; the impl of a `Copy` type takes it by
    /// value. Each such argument becomes the value it points to.
    fn adapt(&self, fi: usize, tys: &[Type], args: &mut [Value]) -> Result<(), Unwind> {
        let f = self.func(fi).ok_or_else(|| ice("no such function"))?;
        for (i, a) in args.iter_mut().enumerate() {
            let Some(l) = f.locals.get(i + 1) else { continue };
            let by_value = !matches!(l.ty.subst(tys), Type::Ref(..));
            if by_value && let Value::Ref(p) = a {
                let v = self.read(p)?.clone();
                *a = v;
            }
        }
        Ok(())
    }
}

/// Matches an impl's pattern (its parameters are `$0` and on, bound in
/// `binds`) against a concrete type.
pub(crate) fn match_type(pat: &Type, ty: &Type, binds: &mut [Option<Type>]) -> bool {
    match pat {
        Type::Param(i) => match binds.get_mut(*i as usize) {
            Some(slot @ None) => {
                *slot = Some(ty.clone());
                true
            }
            Some(Some(b)) => b == ty,
            None => false,
        },
        Type::Named(n, a) => {
            matches!(ty, Type::Named(m, b) if m == n && a.len() == b.len() && a.iter().zip(b).all(|(x, y)| match_type(x, y, binds)))
        }
        Type::Tuple(a) => {
            matches!(ty, Type::Tuple(b) if a.len() == b.len() && a.iter().zip(b).all(|(x, y)| match_type(x, y, binds)))
        }
        Type::Ref(a, m) => matches!(ty, Type::Ref(b, k) if m == k && match_type(a, b, binds)),
        Type::Fn(ps, r, pure) => matches!(ty, Type::Fn(qs, s, q) if pure == q && ps.len() == qs.len()
            && ps.iter().zip(qs).all(|(x, y)| match_type(x, y, binds)) && match_type(r, s, binds)),
        Type::Dyn(n, a) => {
            matches!(ty, Type::Dyn(m, b) if m == n && a.len() == b.len() && a.iter().zip(b).all(|(x, y)| match_type(x, y, binds)))
        }
        other => other == ty,
    }
}
