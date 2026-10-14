//! The auto traits `Send` and `Sync` (03, "Auto traits"; 06, "Send, Sync,
//! and shared state"), inferred structurally as Rust infers them, and
//! checked only where a value crosses a `spawn` and where a bound asks.
//!
//! `Send` means a value may move to another task, `Sync` that tasks may
//! share it through references. The leaves (numbers, text, paths, times,
//! the capability handles, scopes) are both. A reference `ref T` is both
//! when `T` is `Sync`; `ref mut T` is `Send` when `T` is `Send` and `Sync`
//! when `T` is `Sync`. A `Cell[T]` is `Send` when `T` is and never `Sync`;
//! a `Mutex[T]`, a channel half and a task handle are both when `T` is
//! `Send`; a mutex guard is never `Send` (it unlocks in the task that
//! locked) and `Sync` when `T` is; a `Shared[T]` and a `Weak[T]` are both
//! when `T` is `Sync`. A function or closure value is neither, since its
//! captures are not in its type; a `dyn Trait` is what its trait's
//! supertraits require; a type parameter is what its bounds say. Tuples,
//! collections, options, results and declared types are what all their
//! parts are.

use crate::program::{DefId, DefKind, Predicate, Program};
use crate::ty::Ty;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Auto {
    Send,
    Sync,
}

/// Why a type is not `Send` or `Sync`: the parts leading to the obstacle
/// (field names, or `[]` for an element), and the obstacle itself.
#[derive(Clone, Debug, PartialEq)]
pub struct Obstacle {
    pub path: Vec<String>,
    pub ty: Ty,
}

impl Program {
    /// Whether `ty` is `Send` (or `Sync`) in an environment of bounds;
    /// the first obstacle when it is not.
    pub fn auto_trait(&self, ty: &Ty, auto: Auto, env: &[Predicate]) -> Result<(), Obstacle> {
        let mut seen = Vec::new();
        self.auto_depth(ty, auto, env, &mut seen, 0)
    }

    fn auto_bound(&self, auto: Auto) -> Option<DefId> {
        match auto {
            Auto::Send => self.wk.send,
            Auto::Sync => self.wk.sync,
        }
    }

    fn auto_depth(
        &self,
        ty: &Ty,
        auto: Auto,
        env: &[Predicate],
        seen: &mut Vec<(DefId, Vec<Ty>, Auto)>,
        depth: u32,
    ) -> Result<(), Obstacle> {
        let no = || Err(Obstacle { path: Vec::new(), ty: ty.clone() });
        if depth > 64 {
            return Ok(());
        }
        let both = |me: &Self, t: &Ty, a: Auto, seen: &mut Vec<_>| me.auto_depth(t, a, env, seen, depth + 1);
        let elem = |r: Result<(), Obstacle>| {
            r.map_err(|mut o| {
                o.path.insert(0, "[]".into());
                o
            })
        };
        match ty {
            Ty::Int
            | Ty::Float
            | Ty::Bool
            | Ty::Char
            | Ty::Str
            | Ty::Unit
            | Ty::Never
            | Ty::Error
            | Ty::Var(_) => Ok(()),
            Ty::Ref(inner, mutable) => {
                let need = if *mutable && auto == Auto::Send { Auto::Send } else { Auto::Sync };
                match both(self, inner, need, seen) {
                    Ok(()) => Ok(()),
                    Err(o) => Err(o),
                }
            }
            Ty::Tuple(es, _) => {
                for (i, e) in es.iter().enumerate() {
                    both(self, e, auto, seen).map_err(|mut o| {
                        o.path.insert(0, i.to_string());
                        o
                    })?;
                }
                Ok(())
            }
            Ty::Fn(_) => no(),
            Ty::Dyn(tr) => {
                let want = self.auto_bound(auto);
                if self.elaborate(ty, tr).iter().any(|t| Some(t.def) == want) { Ok(()) } else { no() }
            }
            Ty::Param(_) | Ty::Assoc(_) => {
                let want = self.auto_bound(auto);
                if env.iter().any(|p| p.ty == *ty && Some(p.bound.def) == want) { Ok(()) } else { no() }
            }
            Ty::ErrSet(members) => {
                for d in members {
                    self.adt_auto(*d, &[], auto, env, seen, depth)?;
                }
                Ok(())
            }
            Ty::Adt(d, args) => {
                let arg = |i: usize| args.get(i).cloned().unwrap_or(Ty::Error);
                match self.builtin(*d) {
                    Some("Cell") => {
                        if auto == Auto::Sync {
                            return no();
                        }
                        elem(both(self, &arg(0), Auto::Send, seen))
                    }
                    Some("Mutex" | "Sender" | "Receiver" | "Handle") => {
                        elem(both(self, &arg(0), Auto::Send, seen))
                    }
                    Some("MutexGuard") => {
                        if auto == Auto::Send {
                            return no();
                        }
                        elem(both(self, &arg(0), Auto::Sync, seen))
                    }
                    Some("Shared" | "Weak") => elem(both(self, &arg(0), Auto::Sync, seen)),
                    Some(_) => {
                        for a in args {
                            elem(both(self, a, auto, seen))?;
                        }
                        Ok(())
                    }
                    None => self.adt_auto(*d, args, auto, env, seen, depth),
                }
            }
        }
    }

    /// A declared struct, enum or error set: every field, with the type's
    /// arguments in it. A type met again on the way is assumed to hold
    /// (coinduction, as for Rust's auto traits).
    fn adt_auto(
        &self,
        d: DefId,
        args: &[Ty],
        auto: Auto,
        env: &[Predicate],
        seen: &mut Vec<(DefId, Vec<Ty>, Auto)>,
        depth: u32,
    ) -> Result<(), Obstacle> {
        let key = (d, args.to_vec(), auto);
        if seen.contains(&key) {
            return Ok(());
        }
        seen.push(key);
        let fields: Vec<(String, Ty)> = match &self.def(d).kind {
            DefKind::Struct(s) => s.fields.iter().map(|f| (f.name.clone(), f.ty.subst(args))).collect(),
            DefKind::Enum(e) | DefKind::ErrorSet(e) => e
                .variants
                .iter()
                .flat_map(|v| {
                    v.fields.iter().enumerate().map(move |(i, (n, t))| {
                        let name = match n {
                            Some(n) => format!("{}.{n}", v.name),
                            None => format!("{}.{i}", v.name),
                        };
                        (name, t.subst(args))
                    })
                })
                .collect(),
            _ => Vec::new(),
        };
        for (name, t) in fields {
            self.auto_depth(&t, auto, env, seen, depth + 1).map_err(|mut o| {
                o.path.insert(0, name.clone());
                o
            })?;
        }
        Ok(())
    }
}
