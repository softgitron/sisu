//! Which values can borrow, and which parameters a function's result may
//! borrow from (04, "Borrowing rules in detail"): the `from` clause, or
//! the three elision rules. Lowering writes the result into SIR, where
//! the ownership checker (`sisu-borrowck`) uses it at every call and checks
//! every `return` against it.

use crate::program::{DefId, DefKind, FnInfo, Program};
use crate::ty::Ty;
use sisu_syntax::ast::Mode;

/// How a function's result relates to its arguments' loans.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResultShape {
    /// The result cannot hold a loan.
    Plain,
    /// The result is or contains a reference or a view, which may point
    /// into the arguments it borrows from.
    Deep,
    /// The result is a generic `T` (or contains one, `T?`): it holds what
    /// the arguments' values hold, never a reference into the arguments.
    Inner,
}

/// The arguments (`self` is argument 0 of a method) a result may borrow from.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResultBorrows {
    pub deep: Vec<u32>,
    pub inner: Vec<u32>,
}

/// What a value of a result type can point into, beyond the loans the
/// arguments' values hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Reach {
    Nothing,
    /// Only text: a `str` views a `String` (or a `Path`).
    Text,
    /// Any place: a reference, a closure, an iterator, a generic value.
    Anything,
}

/// One parameter as the elision rules see it.
#[derive(Clone, Debug)]
pub struct SigParam {
    pub name: String,
    pub mode: Mode,
    pub ty: Ty,
}

impl Program {
    /// Whether a type is a view by its spelling (04, "Views"): a
    /// reference, `str`, a `view struct`, or a collection, option or tuple
    /// of views. Only a `view struct` may have a field of such a type.
    pub fn is_view_type(&self, ty: &Ty) -> bool {
        if let Ty::Adt(d, args) = ty
            && let Some(b) = self.refinement_base(*d, args)
        {
            return self.is_view_type(&b);
        }
        match ty {
            Ty::Ref(..) | Ty::Str => true,
            Ty::Tuple(es, _) => es.iter().any(|t| self.is_view_type(t)),
            Ty::Adt(d, args) => match &self.def(*d).kind {
                DefKind::Struct(s)
                    if matches!(
                        s.builtin.as_deref(),
                        Some("MutexGuard" | "ReadGuard" | "Lent" | "Slice")
                    ) =>
                {
                    true
                }
                DefKind::Struct(s) if s.builtin.is_some() => args.iter().any(|t| self.is_view_type(t)),
                // A generic type holds what its arguments hold.
                DefKind::Struct(s) => s.view || args.iter().any(|t| self.is_view_type(t)),
                DefKind::Enum(_) => args.iter().any(|t| self.is_view_type(t)),
                _ => false,
            },
            _ => false,
        }
    }

    /// Whether a value of the type can hold a loan at all: a view, and
    /// also a function value (a closure borrows its captures) and an
    /// iterator or dict entry (they borrow what they walk). A generic
    /// parameter can, since it may stand for any of these.
    pub fn can_borrow(&self, ty: &Ty) -> bool {
        if let Ty::Adt(d, args) = ty
            && let Some(b) = self.refinement_base(*d, args)
        {
            return self.can_borrow(&b);
        }
        match ty {
            Ty::Ref(..) | Ty::Str | Ty::Fn(_) | Ty::Param(_) | Ty::Var(_) | Ty::Error => true,
            Ty::Tuple(es, _) => es.iter().any(|t| self.can_borrow(t)),
            Ty::Adt(d, args) => match &self.def(*d).kind {
                DefKind::Struct(s) => match s.builtin.as_deref() {
                    Some("Iter" | "Entry" | "Iterable" | "MutexGuard" | "ReadGuard" | "Lent" | "Slice") => {
                        true
                    }
                    Some(_) => args.iter().any(|t| self.can_borrow(t)),
                    None => s.view || args.iter().any(|t| self.can_borrow(t)),
                },
                DefKind::Enum(_) => args.iter().any(|t| self.can_borrow(t)),
                _ => false,
            },
            _ => false,
        }
    }

    /// Whether a value of the type is a `Copy` handle on something that
    /// lives as long as the run: a `TestClock`, the number of one of the
    /// scheduler's clocks. A capability handle, a `Cap` and a `Lent` are
    /// not: their object goes with the handle's last copy (decision 237).
    pub fn is_run_handle(&self, ty: &Ty) -> bool {
        matches!(ty, Ty::Adt(d, _) if self.builtin(*d) == Some("TestClock"))
    }

    /// The shape of a declared result type (before substitution).
    pub fn result_shape(&self, ret: &Ty) -> ResultShape {
        fn deep(p: &Program, t: &Ty) -> bool {
            if let Ty::Adt(d, args) = t
                && let Some(b) = p.refinement_base(*d, args)
            {
                return deep(p, &b);
            }
            match t {
                Ty::Ref(..) | Ty::Str | Ty::Fn(_) => true,
                Ty::Tuple(es, _) => es.iter().any(|t| deep(p, t)),
                Ty::Adt(d, args) => match &p.def(*d).kind {
                    DefKind::Struct(s) => match s.builtin.as_deref() {
                        Some(
                            "Iter" | "Entry" | "Iterable" | "MutexGuard" | "ReadGuard" | "Lent" | "Slice",
                        ) => true,
                        Some(_) => args.iter().any(|t| deep(p, t)),
                        None => s.view || args.iter().any(|t| deep(p, t)),
                    },
                    DefKind::Enum(_) => args.iter().any(|t| deep(p, t)),
                    _ => false,
                },
                _ => false,
            }
        }
        fn generic(t: &Ty) -> bool {
            match t {
                Ty::Param(_) => true,
                Ty::Tuple(es, _) => es.iter().any(generic),
                Ty::Adt(_, args) => args.iter().any(generic),
                Ty::Ref(t, _) => generic(t),
                Ty::Fn(f) => f.params.iter().any(generic) || generic(&f.ret),
                _ => false,
            }
        }
        if deep(self, ret) {
            ResultShape::Deep
        } else if generic(ret) {
            ResultShape::Inner
        } else {
            ResultShape::Plain
        }
    }

    /// What a value of the type can point into (see [`Reach`]).
    fn reach(&self, t: &Ty, depth: u32) -> Reach {
        if depth > 16 {
            return Reach::Anything;
        }
        let most = |ts: &mut dyn Iterator<Item = &Ty>| {
            ts.map(|t| self.reach(t, depth + 1)).max().unwrap_or(Reach::Nothing)
        };
        if let Ty::Adt(d, args) = t
            && let Some(b) = self.refinement_base(*d, args)
        {
            return self.reach(&b, depth + 1);
        }
        match t {
            Ty::Ref(..) | Ty::Fn(_) | Ty::Param(_) | Ty::Var(_) | Ty::Error => Reach::Anything,
            Ty::Str => Reach::Text,
            Ty::Tuple(es, _) => most(&mut es.iter()),
            Ty::Adt(d, args) => {
                let of_args = most(&mut args.iter());
                match &self.def(*d).kind {
                    DefKind::Struct(s) => match s.builtin.as_deref() {
                        Some("Iter" | "Entry" | "Iterable" | "Lent" | "Slice") => Reach::Anything,
                        Some(_) => of_args,
                        None if s.view => most(&mut s.fields.iter().map(|f| &f.ty)).max(of_args),
                        None => of_args,
                    },
                    DefKind::Enum(_) => of_args,
                    _ => Reach::Nothing,
                }
            }
            _ => Reach::Nothing,
        }
    }

    /// Whether a value of the type owns text a `str` could view: a
    /// `String` or `Path` in its own storage, not behind a reference.
    fn owns_text(&self, t: &Ty, depth: u32) -> bool {
        if depth > 16 {
            return true;
        }
        if let Ty::Adt(d, args) = t
            && let Some(b) = self.refinement_base(*d, args)
        {
            return self.owns_text(&b, depth + 1);
        }
        match t {
            Ty::Ref(..) | Ty::Str | Ty::Fn(_) => false,
            Ty::Param(_) | Ty::Var(_) | Ty::Error => true,
            Ty::Tuple(es, _) => es.iter().any(|e| self.owns_text(e, depth + 1)),
            Ty::Adt(d, args) => {
                args.iter().any(|a| self.owns_text(a, depth + 1))
                    || match &self.def(*d).kind {
                        DefKind::Struct(s) => match s.builtin.as_deref() {
                            Some("String" | "Path" | "Iter" | "Entry" | "Iterable") => true,
                            Some(_) => false,
                            None => s.fields.iter().any(|f| self.owns_text(&f.ty, depth + 1)),
                        },
                        DefKind::Enum(e) => e
                            .variants
                            .iter()
                            .any(|v| v.fields.iter().any(|(_, t)| self.owns_text(t, depth + 1))),
                        _ => false,
                    }
            }
            _ => false,
        }
    }

    /// The parameters of a function as its arguments: `self` first.
    pub fn sig_params(&self, f: &FnInfo) -> Vec<SigParam> {
        let mut out = Vec::new();
        if let (Some(mode), Some(owner)) = (f.self_mode, &f.owner) {
            out.push(SigParam { name: "self".into(), mode, ty: owner.clone() });
        }
        out.extend(f.params.iter().map(|p| SigParam {
            name: p.name.clone(),
            mode: p.mode,
            ty: p.ty.clone(),
        }));
        out
    }

    /// 04: "A parameter counts as borrowed when it is passed without
    /// `own` and is not `Copy`, or when its type is a view, since a result
    /// may point into a view." A value that holds a capability handle or a
    /// `Cap` counts too, since a result may point into the handle's object
    /// (decision 237).
    pub fn is_borrowed_param(&self, p: &SigParam) -> bool {
        match p.mode {
            Mode::Mut => true,
            Mode::Own => false,
            Mode::Borrow => !self.is_copy(&p.ty) || self.can_borrow(&p.ty) || self.holds_capability(&p.ty),
        }
    }

    /// Whether a borrowed parameter can lend anything to a result of type
    /// `ret`. A function value lends only what its calls return, unless
    /// the result can hold the function itself (`Filter` holds `keep`):
    /// `cmp: (ref T, ref T) -> Ordering` lends nothing to a `list[T]`.
    pub fn can_lend(&self, p: &SigParam, ret: &Ty) -> bool {
        match &p.ty {
            Ty::Fn(f) => self.can_borrow(&f.ret) || self.holds_fn_value(ret, 0),
            _ => true,
        }
    }

    /// Whether a value of the type can hold a function value: a function
    /// type in it, or a `view struct`, whose fields may be anything.
    fn holds_fn_value(&self, t: &Ty, depth: u32) -> bool {
        if depth > 16 {
            return true;
        }
        if let Ty::Adt(d, args) = t
            && let Some(b) = self.refinement_base(*d, args)
        {
            return self.holds_fn_value(&b, depth + 1);
        }
        match t {
            Ty::Fn(_) => true,
            Ty::Ref(t, _) => self.holds_fn_value(t, depth + 1),
            Ty::Tuple(es, _) => es.iter().any(|e| self.holds_fn_value(e, depth + 1)),
            Ty::Adt(d, args) => {
                matches!(&self.def(*d).kind, DefKind::Struct(s) if s.view)
                    || args.iter().any(|a| self.holds_fn_value(a, depth + 1))
            }
            _ => false,
        }
    }

    /// The borrowed parameters a result comes from: the `from` clause; or
    /// the only borrowed parameter; or `self`; otherwise every borrowed
    /// parameter, which for a user function is an error reported by the
    /// resolver (a `from` clause is required).
    pub fn from_params(&self, f: &FnInfo) -> Vec<u32> {
        let params = self.sig_params(f);
        if !f.from.is_empty() {
            return params
                .iter()
                .enumerate()
                .filter(|(_, p)| f.from.iter().any(|(n, _)| *n == p.name))
                .map(|(i, _)| i as u32)
                .collect();
        }
        let borrowed: Vec<u32> = params
            .iter()
            .enumerate()
            .filter(|(_, p)| self.is_borrowed_param(p) && self.can_lend(p, &f.ret))
            .map(|(i, _)| i as u32)
            .collect();
        if borrowed.len() <= 1 {
            return borrowed;
        }
        if f.self_mode.is_some() && borrowed.first() == Some(&0) {
            return vec![0];
        }
        borrowed
    }

    /// The arguments a call's result may borrow from: the borrowed
    /// parameters of [`Program::from_params`] and every owned parameter
    /// whose value can hold a loan (a view moved in can come back out).
    pub fn result_borrows(&self, d: DefId) -> ResultBorrows {
        let Some(f) = self.fn_info(d) else { return ResultBorrows::default() };
        self.result_borrows_of(f)
    }

    pub fn result_borrows_of(&self, f: &FnInfo) -> ResultBorrows {
        let shape = self.result_shape(&f.ret);
        if shape == ResultShape::Plain {
            return ResultBorrows::default();
        }
        let params = self.sig_params(f);
        // A handle on what lives as long as the run (a test clock) lends
        // nothing of its own; what a result reaches through a capability
        // handle, a `Cap` or a `Lent` lives as long as the handle's copy
        // the caller passed (decision 237).
        let from: Vec<u32> = self
            .from_params(f)
            .into_iter()
            .filter(|i| !self.is_run_handle(&params[*i as usize].ty))
            .collect();
        let owned: Vec<u32> = params
            .iter()
            .enumerate()
            .filter(|(_, p)| p.mode == Mode::Own && self.can_borrow(&p.ty))
            .map(|(i, _)| i as u32)
            .collect();
        let mut deep: Vec<u32> = owned;
        let mut inner = Vec::new();
        match shape {
            // A result that can point into a borrowed argument's own
            // storage holds the loan of the argument itself; one that can
            // only view text, from an argument that owns none (a `str`
            // taken from a `list[str]`), holds only what the argument
            // holds.
            // Only an argument passed by reference has a loan of its own
            // apart from its value's; a view passed by value (`str`) is
            // its loans.
            ResultShape::Deep => {
                let reach = self.reach(&f.ret, 0);
                for i in from {
                    let p = &params[i as usize];
                    let by_ref = match p.mode {
                        Mode::Mut => true,
                        Mode::Own => false,
                        Mode::Borrow => !self.is_copy(&p.ty),
                    };
                    if !by_ref
                        || reach == Reach::Anything
                        || (reach == Reach::Text && self.owns_text(&p.ty, 0))
                    {
                        deep.push(i);
                    } else if !deep.contains(&i) {
                        inner.push(i);
                    }
                }
            }
            _ => inner.extend(from.into_iter().filter(|i| !deep.contains(i))),
        }
        deep.sort_unstable();
        deep.dedup();
        ResultBorrows { deep, inner }
    }
}
