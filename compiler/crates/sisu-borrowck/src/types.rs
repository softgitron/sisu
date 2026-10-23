//! What the checker needs to know about SIR types: the type of a place,
//! which values can hold a loan, which have drop glue that can observe a
//! loan, which must be consumed, and which implement `Drop`.

use sisu_sir::{Function, Place, Program, Type, TypeDecl};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

/// The state of one look at a type and what it reaches: the types being
/// looked at, and the answers for those done.
#[derive(Default)]
struct Walk {
    open: Vec<Type>,
    done: HashMap<Type, bool>,
}

pub struct Types<'p> {
    decls: HashMap<&'p str, &'p TypeDecl>,
    /// The program's functions by name (the first of a name, as
    /// `Program::function` finds it).
    fns: HashMap<&'p str, &'p Function>,
    program: &'p Program,
    /// [`Types::has_drop_glue`]'s answers so far: each depends on the type
    /// alone, and the checker asks again at every drop of every pass.
    glue: RefCell<HashMap<Type, bool>>,
}

/// The builtin types whose values can hold no loan and own nothing that
/// can.
const PLAIN_BUILTINS: &[&str] = &[
    "String",
    "Path",
    "DateTime",
    "bytes",
    "f32",
    "u8",
    "u16",
    "u32",
    "u64",
    "u128",
    "i8",
    "i16",
    "i32",
    "i128",
    "Fs",
    "Net",
    "Clock",
    "Rand",
    "Env",
    "Proc",
    "Stdin",
    "System",
    "HostFs",
    "HostNet",
    "HostClock",
    "HostRand",
    "HostEnv",
    "HostProc",
    "HostStdin",
    "TestClock",
    // What a `Cap` holds is neither a view nor must-consume (checked where
    // one is made).
    "Cap",
];

/// The builtin types whose values may borrow whatever their arguments are:
/// an iterator walks a collection it may borrow, an entry borrows its dict.
const BORROWING_BUILTINS: &[&str] =
    &["Iter", "Entry", "Iterable", "MutexGuard", "ReadGuard", "Lent", "Slice"];

impl<'p> Types<'p> {
    pub fn new(p: &'p Program) -> Types<'p> {
        let mut fns = HashMap::new();
        for f in &p.functions {
            fns.entry(f.name.as_str()).or_insert(f);
        }
        Types {
            decls: p.types.iter().map(|t| (t.name(), t)).collect(),
            fns,
            program: p,
            glue: RefCell::default(),
        }
    }

    /// Whether an intrinsic keeps or returns every value its argument `i`
    /// holds (the `keeps` of its declaration, from its contract).
    pub fn intrinsic_keeps(&self, name: &str, i: u32) -> bool {
        self.program.intrinsic(name).is_some_and(|d| d.keeps.contains(&i))
    }

    /// The program's function of that name.
    pub fn function(&self, name: &str) -> Option<&'p Function> {
        self.fns.get(name).copied()
    }

    /// The function a call through a trait runs for concrete type
    /// arguments, with its own type arguments: the impl whose patterns
    /// match, or the trait's provided method.
    pub fn trait_target(&self, name: &str, tys: &[Type]) -> Option<(String, Vec<Type>)> {
        let (trait_name, method) = name.rsplit_once('.')?;
        let self_ty = tys.first()?;
        for imp in &self.program.impls {
            if imp.trait_name != trait_name {
                continue;
            }
            let mut binds: Vec<Option<Type>> = vec![None; imp.params as usize];
            if !match_pattern(&imp.self_ty, self_ty, &mut binds)
                || !imp
                    .trait_args
                    .iter()
                    .zip(tys.iter().skip(1))
                    .all(|(p, t)| match_pattern(p, t, &mut binds))
            {
                continue;
            }
            let n = imp.trait_args.len();
            return match imp.methods.iter().find(|(m, _)| m == method) {
                Some((_, sisu_sir::ImplTarget::Fn(f))) => {
                    let mut targs: Vec<Type> = binds.into_iter().map(|b| b.unwrap_or(Type::Unit)).collect();
                    targs.extend(tys.iter().skip(1 + n).cloned());
                    Some((f.clone(), targs))
                }
                Some((_, sisu_sir::ImplTarget::Intrinsic(_))) => None,
                None => Some((name.to_string(), tys.to_vec())),
            };
        }
        None
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
        sisu_sir::types::place_ty(&|n| self.decl(n), f, p)
    }

    /// Field `i` of a struct or tuple, or of variant `v` of an enum.
    pub fn field_ty(&self, t: &Type, variant: Option<u32>, i: u32) -> Option<Type> {
        sisu_sir::types::field_ty(&|n| self.decl(n), t, variant, i)
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
                if matches!(n.as_str(), "list" | "Array" | "dict" | "set" | "Option" | "Result") {
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

    /// Whether a value of the type holds a capability handle or a `Cap`,
    /// whose object goes with the handle's last copy (decision 237): a
    /// result that points into the object borrows the place the call
    /// copied the handle from. A `Lent` holds loans of its own instead.
    pub fn holds_handle(&self, t: &Type) -> bool {
        self.holds_handle_depth(t, 0)
    }

    fn holds_handle_depth(&self, t: &Type, depth: u32) -> bool {
        if depth > 16 {
            return false;
        }
        match t {
            Type::Tuple(es) => es.iter().any(|e| self.holds_handle_depth(e, depth + 1)),
            Type::Named(n, args) => match n.as_str() {
                "Fs" | "Net" | "Clock" | "Rand" | "Env" | "Proc" | "Stdin" | "System" | "Cap" => true,
                "Lent" | "Shared" | "Weak" | "list" | "Array" | "dict" | "set" | "Iter" | "Entry"
                | "Iterable" | "Cell" | "Atomic" | "Mutex" | "RwLock" | "MutexGuard" | "ReadGuard"
                | "Arena" | "Slice" => false,
                "Option" | "Result" => args.iter().any(|a| self.holds_handle_depth(a, depth + 1)),
                _ => match self.decl(n) {
                    Some(TypeDecl::Struct { fields, .. }) => {
                        fields.iter().any(|f| self.holds_handle_depth(&f.ty.subst(args), depth + 1))
                    }
                    Some(TypeDecl::Enum { variants, .. }) => variants.iter().any(|v| {
                        v.fields.iter().any(|(_, ft)| self.holds_handle_depth(&ft.subst(args), depth + 1))
                    }),
                    None => false,
                },
            },
            _ => false,
        }
    }

    /// The index of a variant of an enum, by name.
    pub fn variant_index(&self, name: &str, variant: &str) -> Option<u32> {
        match self.decl(name)? {
            TypeDecl::Enum { variants, .. } => {
                variants.iter().position(|v| v.name == variant).map(|i| i as u32)
            }
            TypeDecl::Struct { .. } => None,
        }
    }

    /// The type a struct's field (or a variant's) is declared with, its
    /// type parameters unsubstituted.
    pub fn declared_field_ty(&self, name: &str, variant: Option<u32>, i: u32) -> Option<Type> {
        match (self.decl(name)?, variant) {
            (TypeDecl::Struct { fields, .. }, None) => fields.get(i as usize).map(|f| f.ty.clone()),
            (TypeDecl::Enum { variants, .. }, Some(v)) => {
                variants.get(v as usize)?.fields.get(i as usize).map(|(_, t)| t.clone())
            }
            _ => None,
        }
    }

    /// Whether every loan a value of a declared field type can hold is one
    /// of its type parameters' (`T`, `list[T]`, `I.Item`), so a generic
    /// type's arguments say what it holds; a function type's captures and
    /// a reference are not.
    pub fn carried_by_params(&self, t: &Type) -> bool {
        match t {
            Type::Param(_) | Type::Assoc(..) => true,
            Type::Tuple(es) => es.iter().all(|e| self.carried_by_params(e) || !self.may_hold(e)),
            Type::Named(n, args) => {
                !BORROWING_BUILTINS.contains(&n.as_str())
                    && !matches!(self.decl(n), Some(TypeDecl::Struct { view: true, .. }))
                    && args.iter().all(|a| self.carried_by_params(a) || !self.may_hold(a))
            }
            _ => !self.may_hold(t),
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
        if let Some(r) = self.glue.borrow().get(t) {
            return *r;
        }
        let r = self.drop_glue_in(t, 0, &mut Walk::default());
        self.glue.borrow_mut().insert(t.clone(), r);
        r
    }

    /// A type met again while it is being looked at is a recursive type,
    /// which counts as having glue (the depth bound's answer); one already
    /// looked at gives what it gave, so a recursive type takes one look per
    /// type it reaches.
    fn drop_glue_in(&self, t: &Type, depth: u32, w: &mut Walk) -> bool {
        if depth > 32 {
            return true;
        }
        match t {
            // Its type argument, or the value behind a `dyn`, may have one.
            Type::Param(_) | Type::Assoc(..) | Type::Dyn(..) => true,
            Type::Tuple(es) => es.iter().any(|e| self.drop_glue_in(e, depth + 1, w)),
            Type::Named(n, args) => {
                // Dropping a guard unlocks its mutex, through its loan.
                if n == "MutexGuard" || n == "ReadGuard" {
                    return true;
                }
                if let Some(r) = w.done.get(t) {
                    return *r;
                }
                if w.open.contains(t) {
                    return true;
                }
                w.open.push(t.clone());
                // The fields with the type's arguments in them.
                let r = args.iter().any(|a| self.drop_glue_in(a, depth + 1, w))
                    || match self.decl(n) {
                        Some(TypeDecl::Struct { drop, fields, .. }) => {
                            drop.is_some()
                                || fields.iter().any(|f| self.drop_glue_in(&f.ty.subst(args), depth + 1, w))
                        }
                        Some(TypeDecl::Enum { drop, variants, .. }) => {
                            drop.is_some()
                                || variants.iter().any(|v| {
                                    v.fields
                                        .iter()
                                        .any(|(_, t)| self.drop_glue_in(&t.subst(args), depth + 1, w))
                                })
                        }
                        None => false,
                    };
                w.open.pop();
                w.done.insert(t.clone(), r);
                r
            }
            Type::ErrSet(ms) => {
                ms.iter().any(|m| self.drop_glue_in(&Type::Named(m.clone(), Vec::new()), depth, w))
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
            Type::Ref(..) | Type::Fn(..) | Type::Const(_) | Type::ConstOp(..) => true,
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
        self.drop_fns_into(t, &mut out, 0, &mut HashSet::new());
        out
    }

    /// `seen`: the types looked at already, each once.
    fn drop_fns_into(&self, t: &Type, out: &mut Vec<String>, depth: u32, seen: &mut HashSet<Type>) {
        if depth > 32 {
            return;
        }
        match t {
            Type::Tuple(es) => es.iter().for_each(|e| self.drop_fns_into(e, out, depth + 1, seen)),
            Type::Named(n, args) => {
                if !seen.insert(t.clone()) {
                    return;
                }
                args.iter().for_each(|a| self.drop_fns_into(a, out, depth + 1, seen));
                match self.decl(n) {
                    Some(TypeDecl::Struct { drop, fields, .. }) => {
                        out.extend(drop.iter().filter(|d| !out.contains(d)).cloned().collect::<Vec<_>>());
                        fields.iter().for_each(|f| self.drop_fns_into(&f.ty, out, depth + 1, seen));
                    }
                    Some(TypeDecl::Enum { drop, variants, .. }) => {
                        out.extend(drop.iter().filter(|d| !out.contains(d)).cloned().collect::<Vec<_>>());
                        for v in variants {
                            v.fields.iter().for_each(|(_, t)| self.drop_fns_into(t, out, depth + 1, seen));
                        }
                    }
                    None => {}
                }
            }
            Type::ErrSet(ms) => {
                for m in ms {
                    self.drop_fns_into(&Type::Named(m.clone(), Vec::new()), out, depth + 1, seen);
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
        self.must_consume_in(t, 0, &mut HashSet::new())
    }

    /// `seen`: the types looked at already. One met again adds nothing:
    /// it is being looked at (a recursive type), or it was and is not
    /// (else the answer is already known), so a recursive type takes one
    /// look per type it reaches.
    fn must_consume_in(&self, t: &Type, depth: u32, seen: &mut HashSet<Type>) -> bool {
        if depth > 32 {
            return false;
        }
        match t {
            Type::Ref(..) => false,
            Type::Tuple(es) => es.iter().any(|e| self.must_consume_in(e, depth + 1, seen)),
            Type::Named(n, args) => {
                if PLAIN_BUILTINS.contains(&n.as_str()) || !seen.insert(t.clone()) {
                    return false;
                }
                if args.iter().any(|a| self.must_consume_in(a, depth + 1, seen)) {
                    return true;
                }
                match self.decl(n) {
                    Some(TypeDecl::Struct { must_consume, fields, .. }) => {
                        *must_consume || fields.iter().any(|f| self.must_consume_in(&f.ty, depth + 1, seen))
                    }
                    Some(TypeDecl::Enum { must_consume, variants, .. }) => {
                        *must_consume
                            || variants.iter().any(|v| {
                                v.fields.iter().any(|(_, t)| self.must_consume_in(t, depth + 1, seen))
                            })
                    }
                    None => false,
                }
            }
            Type::ErrSet(ms) => {
                ms.iter().any(|m| self.must_consume_in(&Type::Named(m.clone(), Vec::new()), depth + 1, seen))
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

/// Matches an impl's pattern (parameters `$0` and on) against a type.
fn match_pattern(pat: &Type, ty: &Type, binds: &mut [Option<Type>]) -> bool {
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
            matches!(ty, Type::Named(m, b) if m == n && a.len() == b.len()
                && a.iter().zip(b).all(|(x, y)| match_pattern(x, y, binds)))
        }
        Type::Tuple(a) => {
            matches!(ty, Type::Tuple(b) if a.len() == b.len() && a.iter().zip(b).all(|(x, y)| match_pattern(x, y, binds)))
        }
        Type::Ref(a, m) => matches!(ty, Type::Ref(b, k) if m == k && match_pattern(a, b, binds)),
        other => other == ty,
    }
}
