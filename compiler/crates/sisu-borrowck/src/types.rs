//! What the checker needs to know about SIR types: the type of a place,
//! which values can hold a loan, which have drop glue that can observe a
//! loan, which must be consumed, and which implement `Drop`.

use sisu_sir::{Function, Place, Program, Proj, Type, TypeDecl};
use std::collections::HashMap;

pub struct Types<'p> {
    decls: HashMap<&'p str, &'p TypeDecl>,
    program: &'p Program,
}

/// The builtin types whose values can hold no loan and own nothing that
/// can.
const PLAIN_BUILTINS: &[&str] = &["String", "Path", "DateTime", "Fs", "Env", "Clock", "System"];

/// The builtin types whose values may borrow whatever their arguments are:
/// an iterator walks a collection it may borrow, an entry borrows its dict.
const BORROWING_BUILTINS: &[&str] = &["Iter", "Entry", "Iterable"];

impl<'p> Types<'p> {
    pub fn new(p: &'p Program) -> Types<'p> {
        Types { decls: p.types.iter().map(|t| (t.name(), t)).collect(), program: p }
    }

    /// Where the type's own `Drop` is written: the `self` of its `drop`.
    pub fn drop_impl_loc(&self, t: &Type) -> Option<sisu_sir::Loc> {
        let Type::Named(n, _) = t else { return None };
        let d = match self.decl(n)? {
            TypeDecl::Struct { drop, .. } | TypeDecl::Enum { drop, .. } => drop.as_ref()?,
        };
        self.program.function(d)?.locals.get(1)?.loc
    }

    pub fn decl(&self, name: &str) -> Option<&'p TypeDecl> {
        self.decls.get(name).copied()
    }

    /// The type of a place, or `None` when SIR does not say (a payload of
    /// a union of error sets).
    pub fn place_ty(&self, f: &Function, p: &Place) -> Option<Type> {
        let mut t = f.locals.get(p.local.0 as usize)?.ty.clone();
        let mut variant: Option<u32> = None;
        for proj in &p.proj {
            t = match proj {
                Proj::Deref => match t {
                    Type::Ref(inner, _) => *inner,
                    _ => return None,
                },
                Proj::Downcast(v) => {
                    variant = Some(*v);
                    continue;
                }
                Proj::Field(i) => {
                    let ft = self.field_ty(&t, variant, *i)?;
                    variant = None;
                    ft
                }
                Proj::Index(_) | Proj::IndexFromEnd(_) => match t {
                    Type::Named(n, args) if n == "list" => args.into_iter().next()?,
                    _ => return None,
                },
                Proj::Key(_) => match t {
                    Type::Named(n, args) if n == "dict" => args.into_iter().nth(1)?,
                    _ => return None,
                },
            };
        }
        Some(t)
    }

    /// Field `i` of a struct or tuple, or of variant `v` of an enum.
    pub fn field_ty(&self, t: &Type, variant: Option<u32>, i: u32) -> Option<Type> {
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
            (Type::Named(n, args), None) => match self.decl(n)? {
                TypeDecl::Struct { fields, .. } => fields.get(i as usize).map(|f| f.ty.subst(args)),
                TypeDecl::Enum { .. } => None,
            },
            (Type::Named(n, args), Some(v)) => match self.decl(n)? {
                TypeDecl::Enum { variants, .. } => {
                    variants.get(v as usize)?.fields.get(i as usize).map(|(_, t)| t.subst(args))
                }
                TypeDecl::Struct { .. } => None,
            },
            (Type::ErrSet(ms), Some(v)) if ms.len() == 1 => match self.decl(&ms[0])? {
                TypeDecl::Enum { variants, .. } => {
                    variants.get(v as usize)?.fields.get(i as usize).map(|(_, t)| t.clone())
                }
                TypeDecl::Struct { .. } => None,
            },
            _ => None,
        }
    }

    /// The number of fields of a struct or tuple, or of variant `v`.
    pub fn field_count(&self, t: &Type, variant: Option<u32>) -> usize {
        let mut n = 0;
        while self.field_ty(t, variant, n).is_some() {
            n += 1;
        }
        n as usize
    }

    /// The number of variants of an enum type, if it is one.
    pub fn variant_count(&self, t: &Type) -> Option<usize> {
        match t {
            Type::Named(n, _) if n == "Option" || n == "Result" => Some(2),
            Type::Named(n, _) => match self.decl(n)? {
                TypeDecl::Enum { variants, .. } => Some(variants.len()),
                TypeDecl::Struct { .. } => None,
            },
            Type::ErrSet(ms) if ms.len() == 1 => match self.decl(&ms[0])? {
                TypeDecl::Enum { variants, .. } => Some(variants.len()),
                TypeDecl::Struct { .. } => None,
            },
            _ => None,
        }
    }

    /// Whether a value of the type can hold a loan: a reference, a `str`
    /// view, a function value (a closure holds its captures), an iterator
    /// or entry, a `view struct`, or a builtin container or tuple of such.
    /// A plain struct or enum cannot: the resolver rejects view fields in
    /// one, and the checker rejects storing a borrowing value in one.
    pub fn may_hold(&self, t: &Type) -> bool {
        self.may_hold_depth(t, 0)
    }

    fn may_hold_depth(&self, t: &Type, depth: u32) -> bool {
        if depth > 32 {
            return true;
        }
        match t {
            // A type parameter may stand for a view.
            Type::Ref(..) | Type::Str | Type::Fn(..) | Type::Param(_) | Type::Assoc(..) => true,
            Type::Tuple(es) => es.iter().any(|e| self.may_hold_depth(e, depth + 1)),
            Type::Named(n, args) => {
                if BORROWING_BUILTINS.contains(&n.as_str()) {
                    return true;
                }
                if PLAIN_BUILTINS.contains(&n.as_str()) {
                    return false;
                }
                if matches!(n.as_str(), "list" | "dict" | "set" | "Option" | "Result") {
                    return args.iter().any(|a| self.may_hold_depth(a, depth + 1));
                }
                // A generic type holds what its arguments hold.
                matches!(self.decl(n), Some(TypeDecl::Struct { view: true, .. }))
                    || args.iter().any(|a| self.may_hold_depth(a, depth + 1))
            }
            // Only a value that owns what it holds becomes `dyn`.
            _ => false,
        }
    }

    /// Whether the type is a `view struct` or holds one.
    pub fn is_plain_aggregate(&self, name: &str) -> bool {
        match self.decl(name) {
            Some(TypeDecl::Struct { view, .. }) => !*view,
            Some(TypeDecl::Enum { .. }) => true,
            None => false,
        }
    }

    /// Whether dropping a value of the type runs a user `Drop` (its own or
    /// a part's), which may read the references the value holds.
    pub fn has_drop_glue(&self, t: &Type) -> bool {
        self.drop_glue_depth(t, 0)
    }

    fn drop_glue_depth(&self, t: &Type, depth: u32) -> bool {
        if depth > 32 {
            return true;
        }
        match t {
            // Its type argument, or the value behind a `dyn`, may have one.
            Type::Param(_) | Type::Assoc(..) | Type::Dyn(..) => true,
            Type::Tuple(es) => es.iter().any(|e| self.drop_glue_depth(e, depth + 1)),
            Type::Named(n, args) => {
                if args.iter().any(|a| self.drop_glue_depth(a, depth + 1)) {
                    return true;
                }
                match self.decl(n) {
                    Some(TypeDecl::Struct { drop, fields, .. }) => {
                        drop.is_some() || fields.iter().any(|f| self.drop_glue_depth(&f.ty, depth + 1))
                    }
                    Some(TypeDecl::Enum { drop, variants, .. }) => {
                        drop.is_some()
                            || variants
                                .iter()
                                .any(|v| v.fields.iter().any(|(_, t)| self.drop_glue_depth(t, depth + 1)))
                    }
                    None => false,
                }
            }
            Type::ErrSet(ms) => {
                ms.iter().any(|m| self.drop_glue_depth(&Type::Named(m.clone(), Vec::new()), depth))
            }
            _ => false,
        }
    }

    /// Whether dropping a value of the type does nothing: scalars, views,
    /// references, functions and capability handles, and tuples, options
    /// and results of them.
    pub fn trivial_drop(&self, t: &Type) -> bool {
        match t {
            Type::Int | Type::Float | Type::Bool | Type::Char | Type::Str | Type::Unit | Type::Never => true,
            Type::Ref(..) | Type::Fn(..) => true,
            Type::Tuple(es) => es.iter().all(|e| self.trivial_drop(e)),
            Type::Named(n, args) => {
                matches!(n.as_str(), "Fs" | "Env" | "Clock" | "DateTime")
                    || (matches!(n.as_str(), "Option" | "Result")
                        && args.iter().all(|a| self.trivial_drop(a)))
            }
            Type::ErrSet(_) | Type::Param(_) | Type::Assoc(..) | Type::Dyn(..) => false,
        }
    }

    /// The `Drop` functions dropping a value of the type may run.
    pub fn drop_fns(&self, t: &Type) -> Vec<String> {
        let mut out = Vec::new();
        self.drop_fns_into(t, &mut out, 0);
        out
    }

    fn drop_fns_into(&self, t: &Type, out: &mut Vec<String>, depth: u32) {
        if depth > 32 {
            return;
        }
        match t {
            Type::Tuple(es) => es.iter().for_each(|e| self.drop_fns_into(e, out, depth + 1)),
            Type::Named(n, args) => {
                args.iter().for_each(|a| self.drop_fns_into(a, out, depth + 1));
                match self.decl(n) {
                    Some(TypeDecl::Struct { drop, fields, .. }) => {
                        out.extend(drop.iter().filter(|d| !out.contains(d)).cloned().collect::<Vec<_>>());
                        fields.iter().for_each(|f| self.drop_fns_into(&f.ty, out, depth + 1));
                    }
                    Some(TypeDecl::Enum { drop, variants, .. }) => {
                        out.extend(drop.iter().filter(|d| !out.contains(d)).cloned().collect::<Vec<_>>());
                        for v in variants {
                            v.fields.iter().for_each(|(_, t)| self.drop_fns_into(t, out, depth + 1));
                        }
                    }
                    None => {}
                }
            }
            Type::ErrSet(ms) => {
                for m in ms {
                    self.drop_fns_into(&Type::Named(m.clone(), Vec::new()), out, depth + 1);
                }
            }
            _ => {}
        }
    }

    /// Whether the type itself implements `Drop`.
    pub fn implements_drop(&self, t: &Type) -> bool {
        match t {
            Type::Named(n, _) => matches!(
                self.decl(n),
                Some(TypeDecl::Struct { drop: Some(_), .. } | TypeDecl::Enum { drop: Some(_), .. })
            ),
            _ => false,
        }
    }

    /// `@must_consume` (D-054), propagated through containers (D-064): a
    /// value of a marked type, or a collection, option, tuple, iterator or
    /// aggregate holding one. A reference never is: it owns nothing.
    pub fn must_consume(&self, t: &Type) -> bool {
        self.must_consume_depth(t, 0)
    }

    fn must_consume_depth(&self, t: &Type, depth: u32) -> bool {
        if depth > 32 {
            return false;
        }
        match t {
            Type::Ref(..) => false,
            Type::Tuple(es) => es.iter().any(|e| self.must_consume_depth(e, depth + 1)),
            Type::Named(n, args) => {
                if PLAIN_BUILTINS.contains(&n.as_str()) {
                    return false;
                }
                if args.iter().any(|a| self.must_consume_depth(a, depth + 1)) {
                    return true;
                }
                match self.decl(n) {
                    Some(TypeDecl::Struct { must_consume, fields, .. }) => {
                        *must_consume || fields.iter().any(|f| self.must_consume_depth(&f.ty, depth + 1))
                    }
                    Some(TypeDecl::Enum { must_consume, variants, .. }) => {
                        *must_consume
                            || variants
                                .iter()
                                .any(|v| v.fields.iter().any(|(_, t)| self.must_consume_depth(t, depth + 1)))
                    }
                    None => false,
                }
            }
            Type::ErrSet(ms) => {
                ms.iter().any(|m| self.must_consume_depth(&Type::Named(m.clone(), Vec::new()), depth + 1))
            }
            _ => false,
        }
    }

    /// The name of field `i` of a struct type, for diagnostics.
    pub fn field_name(&self, t: &Type, variant: Option<u32>, i: u32) -> Option<String> {
        match (t, variant) {
            (Type::Named(n, _), None) => match self.decl(n)? {
                TypeDecl::Struct { fields, .. } => fields.get(i as usize).map(|f| f.name.clone()),
                TypeDecl::Enum { .. } => None,
            },
            (Type::Named(n, _), Some(v)) => match self.decl(n)? {
                TypeDecl::Enum { variants, .. } => {
                    variants.get(v as usize)?.fields.get(i as usize)?.0.clone()
                }
                TypeDecl::Struct { .. } => None,
            },
            _ => None,
        }
    }
}
