//! The trait solver (03, "Traits"): which impl, bound or language rule
//! makes a type implement a trait, what an associated type is for a given
//! type, and which traits a method name belongs to.
//!
//! Selection matches impl headers only: coherence (one impl per trait and
//! type, checked by the resolver) guarantees that at most one header
//! matches, so `where` clauses never choose between impls; they are
//! checked once the impl is chosen.

use crate::program::{DefId, DefKind, ImplInfo, Predicate, Program, TraitRef};
use crate::ty::{AssocTy, FnTy, Ty};

/// What makes a type implement a trait.
#[derive(Clone, Debug, PartialEq)]
pub enum ImplSource {
    /// An `impl`, with the impl's type arguments.
    Impl { def: DefId, args: Vec<Ty> },
    /// A bound in scope (a generic parameter's, a `where` clause's, a
    /// trait's `Self`).
    Param,
    /// `dyn Trait` implements its trait and the trait's supertraits.
    Dyn,
    /// A rule of the language: references implement what their referent
    /// does for the reading traits, error sets compare and hash
    /// structurally, every type is `Debug`, `Copy` is structural.
    Builtin,
}

/// The solver's answer.
#[derive(Clone, Debug, PartialEq)]
pub enum Answer {
    Yes(ImplSource),
    No,
    /// The type still has inference variables that decide it.
    Unknown,
}

impl Answer {
    pub fn holds(&self) -> bool {
        matches!(self, Answer::Yes(_))
    }
}

/// Recursion bound for impl predicates (`impl[T: Eq] Eq for list[T]`).
const DEPTH: u32 = 32;

impl Program {
    /// The trait and its supertraits, transitively, with `Self` = `subject`.
    pub fn elaborate(&self, subject: &Ty, tr: &TraitRef) -> Vec<TraitRef> {
        let mut out: Vec<TraitRef> = Vec::new();
        let mut work = vec![tr.clone()];
        while let Some(t) = work.pop() {
            if out.contains(&t) || out.len() > 64 {
                continue;
            }
            if let Some(info) = self.trait_info(t.def) {
                let mut args = vec![subject.clone()];
                args.extend(t.args.iter().cloned());
                for s in &info.supertraits {
                    work.push(s.subst(&args));
                }
            }
            out.push(t);
        }
        out
    }

    /// The predicates in `env` closed under supertraits.
    pub fn elaborate_env(&self, env: &[Predicate]) -> Vec<Predicate> {
        let mut out: Vec<Predicate> = Vec::new();
        for p in env {
            for b in self.elaborate(&p.ty, &p.bound) {
                if !out.iter().any(|q| q.ty == p.ty && q.bound == b) {
                    out.push(Predicate { ty: p.ty.clone(), bound: b, span: p.span });
                }
            }
        }
        out
    }

    /// Whether `ty` implements `tr` in an environment of bounds (already
    /// elaborated with [`Program::elaborate_env`]).
    pub fn implements(&self, ty: &Ty, tr: &TraitRef, env: &[Predicate]) -> Answer {
        self.implements_depth(ty, tr, env, 0)
    }

    fn implements_depth(&self, ty: &Ty, tr: &TraitRef, env: &[Predicate], depth: u32) -> Answer {
        if depth > DEPTH {
            return Answer::No;
        }
        let ty = self.normalize(ty, env);
        match &ty {
            Ty::Error | Ty::Never => return Answer::Yes(ImplSource::Builtin),
            Ty::Var(_) => return Answer::Unknown,
            _ => {}
        }
        if tr.args.iter().any(|a| matches!(a, Ty::Var(_))) {
            return Answer::Unknown;
        }
        // A bound in scope.
        if env.iter().any(|p| p.ty == ty && p.bound == *tr) {
            return Answer::Yes(ImplSource::Param);
        }
        let wk = &self.wk;
        let is = |d: Option<DefId>| d == Some(tr.def);
        // `Copy` is structural (04, rule 1; `@derive(Copy)` opts a type in).
        if is(wk.copy) {
            return if matches!(ty, Ty::Param(_) | Ty::Assoc(_)) {
                Answer::No
            } else if self.is_copy(&ty) {
                Answer::Yes(ImplSource::Builtin)
            } else {
                Answer::No
            };
        }
        match &ty {
            Ty::Dyn(d) if self.elaborate(&ty, d).contains(tr) => return Answer::Yes(ImplSource::Dyn),
            // A reference reads what it points to.
            Ty::Ref(inner, _) if self.passes_through_refs(tr.def) => {
                return match self.implements_depth(inner, tr, env, depth + 1) {
                    Answer::Yes(_) => Answer::Yes(ImplSource::Builtin),
                    other => other,
                };
            }
            // Error sets compare, hash and print structurally.
            Ty::ErrSet(_) if is(wk.eq) || is(wk.hash) || is(wk.debug) || is(wk.display) => {
                return Answer::Yes(ImplSource::Builtin);
            }
            // Functions and closures print as `<fn name>` and `<closure>`.
            Ty::Fn(_) if is(wk.debug) => return Answer::Yes(ImplSource::Builtin),
            // Every type has `Debug` (11): a `dyn` value prints as the value
            // it holds, through that value's type.
            Ty::Dyn(_) if is(wk.debug) => return Answer::Yes(ImplSource::Builtin),
            _ => {}
        }
        let mut unknown = false;
        for &i in &self.impls {
            let Some(info) = self.impl_info(i) else { continue };
            if info.trait_ref.def != tr.def {
                continue;
            }
            let mut binds: Vec<Option<Ty>> = vec![None; info.generics.len()];
            let header = match_ty(&info.self_ty, &ty, &mut binds).and(
                info.trait_ref
                    .args
                    .iter()
                    .zip(&tr.args)
                    .fold(Match::Yes, |m, (p, t)| m.and(match_ty(p, t, &mut binds))),
            );
            match header {
                Match::No => continue,
                Match::Unknown => {
                    unknown = true;
                    continue;
                }
                Match::Yes => {}
            }
            let args: Vec<Ty> = binds.into_iter().map(|b| b.unwrap_or(Ty::Error)).collect();
            // The impl's own bounds, for these arguments.
            let mut ok = Answer::Yes(ImplSource::Impl { def: i, args: args.clone() });
            for p in &info.predicates {
                match self.implements_depth(&p.ty.subst(&args), &p.bound.subst(&args), env, depth + 1) {
                    Answer::Yes(_) => {}
                    Answer::No => {
                        ok = Answer::No;
                        break;
                    }
                    Answer::Unknown => ok = Answer::Unknown,
                }
            }
            return ok;
        }
        if unknown { Answer::Unknown } else { Answer::No }
    }

    /// The traits that a `ref T` implements whenever `T` does: those whose
    /// methods only read (`Display`, `Debug`, `Eq`, `Ord`, `PartialOrd`,
    /// `Hash`, `Len`).
    pub fn passes_through_refs(&self, t: DefId) -> bool {
        let wk = &self.wk;
        [wk.display, wk.debug, wk.eq, wk.partial_ord, wk.ord, wk.hash, wk.len].contains(&Some(t))
    }

    /// The impl that makes `ty` implement `tr`, when one does.
    pub fn find_impl(&self, ty: &Ty, tr: &TraitRef, env: &[Predicate]) -> Option<(DefId, Vec<Ty>)> {
        match self.implements(ty, tr, env) {
            Answer::Yes(ImplSource::Impl { def, args }) => Some((def, args)),
            _ => None,
        }
    }

    /// Replaces associated types of known types by what their impls bind.
    pub fn normalize(&self, ty: &Ty, env: &[Predicate]) -> Ty {
        self.normalize_depth(ty, env, 0)
    }

    fn normalize_depth(&self, ty: &Ty, env: &[Predicate], depth: u32) -> Ty {
        if depth > DEPTH {
            return ty.clone();
        }
        let n = |t: &Ty| self.normalize_depth(t, env, depth + 1);
        match ty {
            Ty::Assoc(a) => {
                let self_ty = n(&a.self_ty);
                let trait_ = TraitRef { def: a.trait_.def, args: a.trait_.args.iter().map(&n).collect() };
                if let Some((def, args)) = self.find_impl(&self_ty, &trait_, env)
                    && let Some(info) = self.impl_info(def)
                {
                    if let Some((_, t)) = info.assoc.iter().find(|(name, _)| *name == a.name) {
                        return n(&t.subst(&args));
                    }
                    // The trait's default for the associated type.
                    if let Some(d) = self.trait_info(trait_.def).and_then(|t| {
                        t.assoc.iter().find(|x| x.name == a.name).and_then(|x| x.default.clone())
                    }) {
                        let mut targs = vec![self_ty.clone()];
                        targs.extend(trait_.args.iter().cloned());
                        return n(&d.subst(&targs));
                    }
                }
                Ty::Assoc(Box::new(AssocTy { self_ty, trait_, name: a.name.clone() }))
            }
            Ty::Adt(d, args) => Ty::Adt(*d, args.iter().map(n).collect()),
            Ty::Tuple(e, l) => Ty::Tuple(e.iter().map(n).collect(), l.clone()),
            Ty::Fn(f) => Ty::Fn(Box::new(FnTy {
                params: f.params.iter().map(n).collect(),
                ret: n(&f.ret),
                pure: f.pure,
            })),
            Ty::Ref(t, m) => Ty::Ref(Box::new(n(t)), *m),
            Ty::Dyn(tr) => Ty::Dyn(TraitRef { def: tr.def, args: tr.args.iter().map(n).collect() }),
            other => other.clone(),
        }
    }

    /// The traits whose methods named `name` a value of type `ty` can call:
    /// for a generic parameter, its bounds; for `dyn T`, `T` and its
    /// supertraits; otherwise every trait the type implements. Each comes
    /// with the trait's arguments.
    pub fn trait_methods_for(&self, ty: &Ty, name: &str, env: &[Predicate]) -> Vec<(TraitRef, DefId)> {
        let mut out: Vec<(TraitRef, DefId)> = Vec::new();
        let mut push = |prog: &Program, tr: &TraitRef| {
            if let Some(m) = prog.trait_method(tr.def, name)
                && !out.iter().any(|(t, _)| t == tr)
            {
                out.push((tr.clone(), m));
            }
        };
        match ty {
            Ty::Param(_) | Ty::Assoc(_) => {
                for p in env {
                    if p.ty == *ty {
                        push(self, &p.bound);
                    }
                }
            }
            Ty::Dyn(tr) => {
                for t in self.elaborate(ty, tr) {
                    push(self, &t);
                }
                // Every `dyn` value has `Debug`.
                if let Some(d) = self.wk.debug {
                    push(self, &TraitRef { def: d, args: Vec::new() });
                }
            }
            _ => {
                for (i, def) in self.defs.iter().enumerate() {
                    let DefKind::Trait(info) = &def.kind else { continue };
                    let d = DefId(i as u32);
                    if self.trait_method(d, name).is_none() {
                        continue;
                    }
                    if info.generics.is_empty() {
                        let tr = TraitRef { def: d, args: Vec::new() };
                        if self.implements(ty, &tr, env).holds() {
                            push(self, &tr);
                        }
                    } else {
                        // A generic trait: the impls for this type say which
                        // arguments it is implemented with.
                        for &im in &self.impls {
                            let Some(ii) = self.impl_info(im) else { continue };
                            if ii.trait_ref.def != d {
                                continue;
                            }
                            let mut binds = vec![None; ii.generics.len()];
                            if match_ty(&ii.self_ty, ty, &mut binds) == Match::Yes {
                                let args: Vec<Ty> =
                                    binds.into_iter().map(|b| b.unwrap_or(Ty::Error)).collect();
                                push(self, &ii.trait_ref.subst(&args));
                            }
                        }
                    }
                }
            }
        }
        out
    }

    /// Whether `ty` implements the trait `tr` (one of `Display`, `Debug`,
    /// `Eq`, `PartialOrd`, `Ord`, `Hash`) structurally, the way the
    /// interpreter compares, orders, hashes and prints values: by the
    /// language's rules, the standard library's impls and derived impls,
    /// all the way down. Lowering then uses the interpreter's operation
    /// directly instead of calling the trait; the results are the same.
    pub fn is_structural(&self, ty: &Ty, tr: DefId) -> bool {
        self.structural_depth(ty, tr, 0)
    }

    fn structural_depth(&self, ty: &Ty, tr: DefId, depth: u32) -> bool {
        if depth > 16 {
            return false;
        }
        let rec = |t: &Ty| self.structural_depth(t, tr, depth + 1);
        match ty {
            Ty::Int | Ty::Float | Ty::Bool | Ty::Char | Ty::Str | Ty::Unit | Ty::Never | Ty::Error => true,
            Ty::Ref(t, _) => rec(t),
            Ty::Tuple(es, _) => es.iter().all(rec),
            Ty::Fn(_) => Some(tr) == self.wk.debug,
            Ty::ErrSet(_) => true,
            Ty::Adt(d, args) => {
                let tref = TraitRef { def: tr, args: Vec::new() };
                let Some((imp, iargs)) = self.find_impl(ty, &tref, &[]) else { return false };
                let Some(info) = self.impl_info(imp) else { return false };
                let module = &self.modules[self.def(imp).module.0 as usize];
                if info.derived {
                    // The derived impl compares or prints the fields.
                    let fields: Vec<Ty> = match &self.def(*d).kind {
                        DefKind::Struct(s) => s.fields.iter().map(|f| f.ty.subst(args)).collect(),
                        DefKind::Enum(e) => e
                            .variants
                            .iter()
                            .flat_map(|v| v.fields.iter().map(|(_, t)| t.subst(args)))
                            .collect(),
                        _ => Vec::new(),
                    };
                    fields.iter().all(rec) && iargs.iter().all(rec)
                } else if module.is_std {
                    args.iter().all(rec)
                } else {
                    false
                }
            }
            Ty::Param(_) | Ty::Assoc(_) | Ty::Dyn(_) | Ty::Var(_) => false,
        }
    }

    /// The method `name` a trait declares, if any.
    pub fn trait_method(&self, tr: DefId, name: &str) -> Option<DefId> {
        self.trait_info(tr)?.methods.iter().copied().find(|m| self.def(*m).name == name)
    }

    /// The function a call of trait method `method` runs for `self_ty`
    /// (and the trait's arguments): the impl's method with the impl's
    /// arguments, or the trait's provided body with `Self` and the trait's
    /// arguments. `None` when only the run time can tell.
    pub fn resolve_method(
        &self,
        method: DefId,
        trait_args: &[Ty],
        self_ty: &Ty,
        env: &[Predicate],
    ) -> Option<(DefId, Vec<Ty>)> {
        let f = self.fn_info(method)?;
        let tr = TraitRef { def: f.trait_?, args: trait_args.to_vec() };
        let (imp, args) = self.find_impl(self_ty, &tr, env)?;
        let info: &ImplInfo = self.impl_info(imp)?;
        let name = &self.def(method).name;
        if let Some((_, m)) = info.methods.iter().find(|(n, _)| n == name) {
            return Some((*m, args));
        }
        // The provided method: `Self` and the trait's arguments.
        let mut targs = vec![self_ty.clone()];
        targs.extend(trait_args.iter().cloned());
        Some((method, targs))
    }
}

/// The result of matching an impl header against a type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Match {
    Yes,
    No,
    Unknown,
}

impl Match {
    fn and(self, other: Match) -> Match {
        match (self, other) {
            (Match::No, _) | (_, Match::No) => Match::No,
            (Match::Unknown, _) | (_, Match::Unknown) => Match::Unknown,
            _ => Match::Yes,
        }
    }
}

/// Matches `pat` (in terms of the impl's parameters, `Param(i)` binding
/// `binds[i]`) against `ty`.
pub fn match_ty(pat: &Ty, ty: &Ty, binds: &mut Vec<Option<Ty>>) -> Match {
    if let Ty::Param(i) = pat
        && (*i as usize) < binds.len()
    {
        return match &binds[*i as usize] {
            None => {
                binds[*i as usize] = Some(ty.clone());
                Match::Yes
            }
            Some(b) if b == ty => Match::Yes,
            Some(b) if matches!(b, Ty::Var(_)) || matches!(ty, Ty::Var(_)) => Match::Unknown,
            Some(_) => Match::No,
        };
    }
    if matches!(ty, Ty::Var(_)) {
        return Match::Unknown;
    }
    match (pat, ty) {
        (Ty::Adt(d1, a1), Ty::Adt(d2, a2)) if d1 == d2 && a1.len() == a2.len() => {
            a1.iter().zip(a2).fold(Match::Yes, |m, (p, t)| m.and(match_ty(p, t, binds)))
        }
        (Ty::Tuple(e1, _), Ty::Tuple(e2, _)) if e1.len() == e2.len() => {
            e1.iter().zip(e2).fold(Match::Yes, |m, (p, t)| m.and(match_ty(p, t, binds)))
        }
        (Ty::Ref(p, m1), Ty::Ref(t, m2)) if m1 == m2 => match_ty(p, t, binds),
        (Ty::Fn(f1), Ty::Fn(f2)) if f1.params.len() == f2.params.len() => f1
            .params
            .iter()
            .zip(&f2.params)
            .fold(match_ty(&f1.ret, &f2.ret, binds), |m, (p, t)| m.and(match_ty(p, t, binds))),
        (Ty::Dyn(t1), Ty::Dyn(t2)) if t1.def == t2.def && t1.args.len() == t2.args.len() => {
            t1.args.iter().zip(&t2.args).fold(Match::Yes, |m, (p, t)| m.and(match_ty(p, t, binds)))
        }
        (a, b) => {
            if a == b {
                Match::Yes
            } else {
                Match::No
            }
        }
    }
}

/// Whether two impl headers can describe the same type: their patterns
/// unify, with the parameters of each renamed apart.
pub fn headers_overlap(a: &Ty, a_params: usize, b: &Ty, b_params: usize) -> bool {
    let shift = |t: &Ty, by: u32| -> Ty { shift_params(t, by) };
    let b = shift(b, a_params as u32);
    let mut binds: Vec<Option<Ty>> = vec![None; a_params + b_params];
    unify_patterns(a, &b, &mut binds)
}

fn shift_params(t: &Ty, by: u32) -> Ty {
    match t {
        Ty::Param(i) => Ty::Param(i + by),
        Ty::Adt(d, a) => Ty::Adt(*d, a.iter().map(|x| shift_params(x, by)).collect()),
        Ty::Tuple(e, l) => Ty::Tuple(e.iter().map(|x| shift_params(x, by)).collect(), l.clone()),
        Ty::Ref(x, m) => Ty::Ref(Box::new(shift_params(x, by)), *m),
        Ty::Fn(f) => Ty::Fn(Box::new(FnTy {
            params: f.params.iter().map(|x| shift_params(x, by)).collect(),
            ret: shift_params(&f.ret, by),
            pure: f.pure,
        })),
        Ty::Dyn(tr) => {
            Ty::Dyn(TraitRef { def: tr.def, args: tr.args.iter().map(|x| shift_params(x, by)).collect() })
        }
        other => other.clone(),
    }
}

fn resolve_bind(t: &Ty, binds: &[Option<Ty>]) -> Ty {
    let mut cur = t.clone();
    for _ in 0..64 {
        match &cur {
            Ty::Param(i) => match binds.get(*i as usize).and_then(|b| b.clone()) {
                Some(b) => cur = b,
                None => return cur,
            },
            _ => return cur,
        }
    }
    cur
}

/// Two-way unification of patterns whose parameters are all variables.
fn unify_patterns(a: &Ty, b: &Ty, binds: &mut Vec<Option<Ty>>) -> bool {
    let a = resolve_bind(a, binds);
    let b = resolve_bind(b, binds);
    match (&a, &b) {
        (Ty::Param(i), Ty::Param(j)) if i == j => true,
        (Ty::Param(i), other) | (other, Ty::Param(i)) if (*i as usize) < binds.len() => {
            binds[*i as usize] = Some(other.clone());
            true
        }
        // An associated type could be anything.
        (Ty::Assoc(_), _) | (_, Ty::Assoc(_)) => true,
        (Ty::Adt(d1, a1), Ty::Adt(d2, a2)) => {
            d1 == d2 && a1.len() == a2.len() && a1.iter().zip(a2).all(|(x, y)| unify_patterns(x, y, binds))
        }
        (Ty::Tuple(e1, _), Ty::Tuple(e2, _)) => {
            e1.len() == e2.len() && e1.iter().zip(e2).all(|(x, y)| unify_patterns(x, y, binds))
        }
        (Ty::Ref(x, m1), Ty::Ref(y, m2)) => m1 == m2 && unify_patterns(x, y, binds),
        (Ty::Fn(f1), Ty::Fn(f2)) => {
            f1.params.len() == f2.params.len()
                && f1.params.iter().zip(&f2.params).all(|(x, y)| unify_patterns(x, y, binds))
                && unify_patterns(&f1.ret, &f2.ret, binds)
        }
        (Ty::Dyn(t1), Ty::Dyn(t2)) => {
            t1.def == t2.def
                && t1.args.len() == t2.args.len()
                && t1.args.iter().zip(&t2.args).all(|(x, y)| unify_patterns(x, y, binds))
        }
        (x, y) => x == y,
    }
}
