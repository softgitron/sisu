//! The inference table of one body: variables, unification and the
//! absorbing error-set variable of a function whose set is inferred.

use sisu_resolve::{DefId, FnTy, Ty};

#[derive(Clone, Debug, Default)]
pub struct InferTable {
    vars: Vec<Option<Ty>>,
    /// The variable standing for an inferred error set (`T !`): unifying it
    /// with an error set adds the set's members instead of binding it.
    absorbing: Option<u32>,
    absorbed: Vec<DefId>,
}

#[derive(Clone, Debug)]
pub struct Snapshot {
    vars: Vec<Option<Ty>>,
    absorbed: Vec<DefId>,
}

impl InferTable {
    pub fn fresh(&mut self) -> Ty {
        self.vars.push(None);
        Ty::Var(self.vars.len() as u32 - 1)
    }

    /// A fresh variable that absorbs error sets.
    pub fn fresh_absorbing(&mut self) -> Ty {
        let v = self.fresh();
        if let Ty::Var(i) = v {
            self.absorbing = Some(i);
        }
        v
    }

    pub fn absorbed(&self) -> Vec<DefId> {
        let mut v = self.absorbed.clone();
        v.sort();
        v.dedup();
        v
    }

    pub fn is_absorbing(&self, t: &Ty) -> bool {
        matches!(self.resolve(t), Ty::Var(i) if Some(i) == self.absorbing)
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot { vars: self.vars.clone(), absorbed: self.absorbed.clone() }
    }

    pub fn rollback(&mut self, s: Snapshot) {
        self.vars = s.vars;
        self.absorbed = s.absorbed;
    }

    /// Follows variable bindings at the top of a type.
    pub fn resolve(&self, t: &Ty) -> Ty {
        let mut cur = t.clone();
        while let Ty::Var(i) = cur {
            match self.vars.get(i as usize) {
                Some(Some(bound)) => cur = bound.clone(),
                _ => break,
            }
        }
        cur
    }

    /// Resolves every variable inside a type. The absorbing variable
    /// becomes the set absorbed so far; an unbound one stays a variable.
    pub fn zonk(&self, t: &Ty) -> Ty {
        match self.resolve(t) {
            Ty::Var(i) if Some(i) == self.absorbing => Ty::ErrSet(self.absorbed()),
            Ty::Adt(d, a) => Ty::Adt(d, a.iter().map(|x| self.zonk(x)).collect()),
            Ty::Tuple(e, l) => Ty::Tuple(e.iter().map(|x| self.zonk(x)).collect(), l),
            Ty::Fn(f) => Ty::Fn(Box::new(FnTy {
                params: f.params.iter().map(|x| self.zonk(x)).collect(),
                ret: self.zonk(&f.ret),
                pure: f.pure,
            })),
            Ty::Ref(t, m) => Ty::Ref(Box::new(self.zonk(&t)), m),
            Ty::Dyn(tr) => Ty::Dyn(self.zonk_trait(&tr)),
            Ty::Assoc(a) => Ty::Assoc(Box::new(sisu_resolve::AssocTy {
                self_ty: self.zonk(&a.self_ty),
                trait_: self.zonk_trait(&a.trait_),
                name: a.name.clone(),
            })),
            Ty::ConstOp(op, a) => Ty::const_op(op, a.iter().map(|x| self.zonk(x)).collect()),
            other => other,
        }
    }

    fn zonk_trait(&self, tr: &sisu_resolve::TraitRef) -> sisu_resolve::TraitRef {
        sisu_resolve::TraitRef { def: tr.def, args: tr.args.iter().map(|x| self.zonk(x)).collect() }
    }

    pub fn has_vars(&self, t: &Ty) -> bool {
        match self.zonk(t) {
            Ty::Var(_) => true,
            Ty::Adt(_, a) => a.iter().any(|x| self.has_vars(x)),
            Ty::Tuple(e, _) => e.iter().any(|x| self.has_vars(x)),
            Ty::Fn(f) => f.params.iter().any(|x| self.has_vars(x)) || self.has_vars(&f.ret),
            Ty::Ref(t, _) => self.has_vars(&t),
            Ty::Dyn(tr) => tr.args.iter().any(|x| self.has_vars(x)),
            Ty::Assoc(a) => self.has_vars(&a.self_ty) || a.trait_.args.iter().any(|x| self.has_vars(x)),
            Ty::ConstOp(_, a) => a.iter().any(|x| self.has_vars(x)),
            _ => false,
        }
    }

    fn occurs(&self, v: u32, t: &Ty) -> bool {
        match self.resolve(t) {
            Ty::Var(i) => i == v,
            Ty::Adt(_, a) => a.iter().any(|x| self.occurs(v, x)),
            Ty::Tuple(e, _) => e.iter().any(|x| self.occurs(v, x)),
            Ty::Fn(f) => f.params.iter().any(|x| self.occurs(v, x)) || self.occurs(v, &f.ret),
            Ty::Ref(t, _) => self.occurs(v, &t),
            Ty::Dyn(tr) => tr.args.iter().any(|x| self.occurs(v, x)),
            Ty::Assoc(a) => self.occurs(v, &a.self_ty) || a.trait_.args.iter().any(|x| self.occurs(v, x)),
            Ty::ConstOp(_, a) => a.iter().any(|x| self.occurs(v, x)),
            _ => false,
        }
    }

    /// Makes two types equal, binding variables. Returns false (possibly
    /// having bound some variables; callers that need atomicity snapshot)
    /// when they cannot be.
    pub fn unify(&mut self, a: &Ty, b: &Ty) -> bool {
        // A constant operation whose arguments are known is its value.
        let known = |me: &Self, t: &Ty| match me.resolve(t) {
            t @ Ty::ConstOp(..) => me.zonk(&t),
            t => t,
        };
        let a = known(self, a);
        let b = known(self, b);
        match (&a, &b) {
            (Ty::Var(x), Ty::Var(y)) if x == y => true,
            (Ty::Var(x), other) | (other, Ty::Var(x)) if Some(*x) == self.absorbing => match other {
                Ty::ErrSet(m) => {
                    self.absorbed.extend(m.iter().copied());
                    true
                }
                Ty::Var(y) => {
                    // Bind the other variable to the absorbing one.
                    if let Some(slot) = self.vars.get_mut(*y as usize) {
                        *slot = Some(Ty::Var(*x));
                    }
                    true
                }
                Ty::Error => true,
                _ => false,
            },
            (Ty::Var(x), other) | (other, Ty::Var(x)) => {
                if self.occurs(*x, other) {
                    return false;
                }
                if let Some(slot) = self.vars.get_mut(*x as usize) {
                    *slot = Some(other.clone());
                }
                true
            }
            (Ty::Error, _) | (_, Ty::Error) => true,
            (Ty::Adt(d1, a1), Ty::Adt(d2, a2)) => {
                d1 == d2 && a1.len() == a2.len() && a1.iter().zip(a2.iter()).all(|(x, y)| self.unify(x, y))
            }
            (Ty::Tuple(e1, _), Ty::Tuple(e2, _)) => {
                e1.len() == e2.len() && e1.iter().zip(e2.iter()).all(|(x, y)| self.unify(x, y))
            }
            (Ty::ErrSet(m1), Ty::ErrSet(m2)) => m1 == m2,
            (Ty::Fn(f1), Ty::Fn(f2)) => {
                f1.params.len() == f2.params.len()
                    && f1.params.iter().zip(f2.params.iter()).all(|(x, y)| self.unify(x, y))
                    && self.unify(&f1.ret, &f2.ret)
            }
            (Ty::Ref(t1, m1), Ty::Ref(t2, m2)) => m1 == m2 && self.unify(t1, t2),
            (Ty::Dyn(t1), Ty::Dyn(t2)) => {
                t1.def == t2.def
                    && t1.args.len() == t2.args.len()
                    && t1.args.iter().zip(t2.args.iter()).all(|(x, y)| self.unify(x, y))
            }
            // An operation on constants not known yet equals only the same
            // operation on equal arguments: `W * H` is not `H * W`.
            (Ty::ConstOp(o1, a1), Ty::ConstOp(o2, a2)) => {
                o1 == o2 && a1.len() == a2.len() && a1.iter().zip(a2.iter()).all(|(x, y)| self.unify(x, y))
            }
            // An associated type that is not known yet (its `Self` is a
            // generic parameter) equals only itself.
            (Ty::Assoc(a1), Ty::Assoc(a2)) => {
                a1.name == a2.name
                    && a1.trait_.def == a2.trait_.def
                    && a1.trait_.args.len() == a2.trait_.args.len()
                    && self.unify(&a1.self_ty, &a2.self_ty)
                    && a1.trait_.args.iter().zip(a2.trait_.args.iter()).all(|(x, y)| self.unify(x, y))
            }
            (x, y) => x == y,
        }
    }

    /// Unifies atomically: on failure nothing is bound.
    pub fn try_unify(&mut self, a: &Ty, b: &Ty) -> bool {
        let snap = self.snapshot();
        if self.unify(a, b) {
            true
        } else {
            self.rollback(snap);
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sisu_resolve::DefId;

    #[test]
    fn unify_binds_and_zonks() {
        let mut t = InferTable::default();
        let v = t.fresh();
        let list = Ty::Adt(DefId(1), vec![v.clone()]);
        assert!(t.unify(&list, &Ty::Adt(DefId(1), vec![Ty::Int])));
        assert_eq!(t.zonk(&v), Ty::Int);
        assert!(!t.try_unify(&v, &Ty::Bool));
        assert_eq!(t.zonk(&v), Ty::Int);
    }

    #[test]
    fn occurs_check() {
        let mut t = InferTable::default();
        let v = t.fresh();
        assert!(!t.unify(&v, &Ty::Adt(DefId(1), vec![v.clone()])));
    }

    #[test]
    fn absorbing_variable_collects_error_sets() {
        let mut t = InferTable::default();
        let e = t.fresh_absorbing();
        assert!(t.unify(&e, &Ty::ErrSet(vec![DefId(5)])));
        assert!(t.unify(&Ty::ErrSet(vec![DefId(3), DefId(5)]), &e));
        assert_eq!(t.zonk(&e), Ty::ErrSet(vec![DefId(3), DefId(5)]));
        assert!(!t.try_unify(&e, &Ty::Int));
    }
}
