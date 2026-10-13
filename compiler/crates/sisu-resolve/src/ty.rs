//! Types as the checker sees them.

use crate::program::{DefId, DefKind, Program};

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Ty {
    Int,
    Float,
    Bool,
    Char,
    Str,
    Unit,
    Never,
    /// A struct, enum or builtin type with its type arguments.
    Adt(DefId, Vec<Ty>),
    /// Element types and, for a labelled tuple, the labels.
    Tuple(Vec<Ty>, Vec<String>),
    /// A union of error sets, as the sorted list of the `error` declarations.
    ErrSet(Vec<DefId>),
    Fn(Box<FnTy>),
    /// `ref T` (`false`) or `ref mut T` (`true`).
    Ref(Box<Ty>, bool),
    /// A generic parameter of the declaration being instantiated.
    Param(u32),
    /// An inference variable of the body being checked.
    Var(u32),
    /// A type that could not be determined; it unifies with everything, so
    /// one mistake is reported once (a poisoned type).
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FnTy {
    pub params: Vec<Ty>,
    pub ret: Ty,
    pub pure: bool,
}

/// The key under which methods are registered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TyKey {
    Int,
    Float,
    Bool,
    Char,
    Str,
    Adt(DefId),
}

impl Ty {
    pub fn tuple(elems: Vec<Ty>) -> Ty {
        Ty::Tuple(elems, Vec::new())
    }

    pub fn key(&self) -> Option<TyKey> {
        Some(match self {
            Ty::Int => TyKey::Int,
            Ty::Float => TyKey::Float,
            Ty::Bool => TyKey::Bool,
            Ty::Char => TyKey::Char,
            Ty::Str => TyKey::Str,
            Ty::Adt(d, _) => TyKey::Adt(*d),
            _ => return None,
        })
    }

    /// Replaces `Param(i)` by `args[i]`.
    pub fn subst(&self, args: &[Ty]) -> Ty {
        match self {
            Ty::Param(i) => args.get(*i as usize).cloned().unwrap_or(Ty::Error),
            Ty::Adt(d, a) => Ty::Adt(*d, a.iter().map(|t| t.subst(args)).collect()),
            Ty::Tuple(e, l) => Ty::Tuple(e.iter().map(|t| t.subst(args)).collect(), l.clone()),
            Ty::Fn(f) => Ty::Fn(Box::new(FnTy {
                params: f.params.iter().map(|t| t.subst(args)).collect(),
                ret: f.ret.subst(args),
                pure: f.pure,
            })),
            Ty::Ref(t, m) => Ty::Ref(Box::new(t.subst(args)), *m),
            other => other.clone(),
        }
    }

    pub fn contains_error(&self) -> bool {
        match self {
            Ty::Error => true,
            Ty::Adt(_, a) => a.iter().any(Ty::contains_error),
            Ty::Tuple(e, _) => e.iter().any(Ty::contains_error),
            Ty::Fn(f) => f.params.iter().any(Ty::contains_error) || f.ret.contains_error(),
            Ty::Ref(t, _) => t.contains_error(),
            _ => false,
        }
    }

    pub fn strip_ref(&self) -> &Ty {
        match self {
            Ty::Ref(t, _) => t.strip_ref(),
            t => t,
        }
    }
}

/// The union of two error sets.
pub fn union(a: &[DefId], b: &[DefId]) -> Vec<DefId> {
    let mut v: Vec<DefId> = a.iter().chain(b.iter()).copied().collect();
    v.sort();
    v.dedup();
    v
}

/// Whether every member of `sub` is in `sup` (05, error-set subtyping).
pub fn is_subset(sub: &[DefId], sup: &[DefId]) -> bool {
    sub.iter().all(|d| sup.contains(d))
}

impl Program {
    /// The canonical spelling of a type, as the formatter would write it
    /// (`T?`, `T ! E`), used in diagnostics and in SIR.
    pub fn display(&self, ty: &Ty) -> String {
        match ty {
            Ty::Int => "int".into(),
            Ty::Float => "f64".into(),
            Ty::Bool => "bool".into(),
            Ty::Char => "char".into(),
            Ty::Str => "str".into(),
            Ty::Unit => "()".into(),
            Ty::Never => "Never".into(),
            Ty::Error => "{unknown}".into(),
            Ty::Var(_) => "_".into(),
            Ty::Param(i) => format!("T{i}"),
            Ty::Adt(d, args) => {
                if Some(*d) == self.wk.option {
                    let inner = args.first().map(|a| self.display(a)).unwrap_or_default();
                    let inner = match args.first() {
                        Some(Ty::Fn(_)) | Some(Ty::Ref(..)) => format!("({inner})"),
                        _ => inner,
                    };
                    return format!("{inner}?");
                }
                if Some(*d) == self.wk.result {
                    let ok = args.first().cloned().unwrap_or(Ty::Unit);
                    let err = args.get(1).map(|e| self.display(e)).unwrap_or_default();
                    if ok == Ty::Unit {
                        return format!("! {err}");
                    }
                    return format!("{} ! {err}", self.display(&ok));
                }
                let name = self.qualified_name(*d);
                if args.is_empty() {
                    name
                } else {
                    let a: Vec<String> = args.iter().map(|t| self.display(t)).collect();
                    format!("{name}[{}]", a.join(", "))
                }
            }
            Ty::Tuple(elems, labels) => {
                let parts: Vec<String> = elems
                    .iter()
                    .enumerate()
                    .map(|(i, t)| match labels.get(i) {
                        Some(l) => format!("{l}: {}", self.display(t)),
                        None => self.display(t),
                    })
                    .collect();
                if parts.len() == 1 && labels.is_empty() {
                    format!("({},)", parts[0])
                } else {
                    format!("({})", parts.join(", "))
                }
            }
            Ty::ErrSet(members) => {
                let names: Vec<String> = members.iter().map(|d| self.qualified_name(*d)).collect();
                if names.len() == 1 { names[0].clone() } else { format!("{{{}}}", names.join(", ")) }
            }
            Ty::Fn(f) => {
                let p: Vec<String> = f.params.iter().map(|t| self.display(t)).collect();
                format!("{}({}) -> {}", if f.pure { "pure " } else { "" }, p.join(", "), self.display(&f.ret))
            }
            Ty::Ref(t, m) => format!("ref {}{}", if *m { "mut " } else { "" }, self.display(t)),
        }
    }

    /// A definition's name, qualified by its module unless it is in the
    /// root module or the prelude.
    pub fn qualified_name(&self, d: DefId) -> String {
        let def = self.def(d);
        if def.module == self.root || def.module == self.prelude {
            def.name.clone()
        } else {
            format!("{}.{}", self.modules[def.module.0 as usize].name, def.name)
        }
    }

    /// Whether values of the type are copied rather than moved (04, rule 1).
    pub fn is_copy(&self, ty: &Ty) -> bool {
        match ty {
            Ty::Int | Ty::Float | Ty::Bool | Ty::Char | Ty::Str | Ty::Unit | Ty::Never | Ty::Error => true,
            Ty::Ref(_, m) => !*m,
            Ty::Fn(_) => true,
            Ty::Tuple(e, _) => e.iter().all(|t| self.is_copy(t)),
            Ty::ErrSet(members) => members.iter().all(|d| self.def_is_copy(*d, &[])),
            Ty::Adt(d, args) => {
                if Some(*d) == self.wk.option {
                    return args.iter().all(|t| self.is_copy(t));
                }
                self.def_is_copy(*d, args)
            }
            Ty::Param(_) | Ty::Var(_) => false,
        }
    }

    fn def_is_copy(&self, d: DefId, _args: &[Ty]) -> bool {
        match &self.def(d).kind {
            DefKind::Struct(s) => {
                s.copy || matches!(s.builtin.as_deref(), Some("Fs" | "Env" | "Clock" | "System" | "DateTime"))
            }
            DefKind::Enum(e) | DefKind::ErrorSet(e) => e.copy,
            _ => false,
        }
    }

    /// Whether a type satisfies one of the builtin bounds of the prelude
    /// (the stand-in for traits until Phase 4).
    pub fn satisfies(&self, ty: &Ty, bound: &str) -> bool {
        match ty {
            Ty::Error | Ty::Never => return true,
            Ty::Ref(t, _) => return self.satisfies(t, bound),
            _ => {}
        }
        let builtin = |d: DefId| match &self.def(d).kind {
            DefKind::Struct(s) => s.builtin.clone(),
            _ => None,
        };
        match bound {
            "Display" | "Debug" | "Eq" => !matches!(ty, Ty::Fn(_)),
            "Len" => match ty {
                Ty::Str => true,
                Ty::Adt(d, _) => matches!(builtin(*d).as_deref(), Some("String" | "list" | "dict" | "set")),
                _ => false,
            },
            "Num" => matches!(ty, Ty::Int | Ty::Float),
            "Ord" => match ty {
                Ty::Int | Ty::Char | Ty::Str | Ty::Bool | Ty::Unit => true,
                Ty::Tuple(e, _) => e.iter().all(|t| self.satisfies(t, "Ord")),
                Ty::Adt(d, args) => {
                    matches!(builtin(*d).as_deref(), Some("String" | "Path"))
                        || (Some(*d) == self.wk.option && args.iter().all(|t| self.satisfies(t, "Ord")))
                        || (matches!(builtin(*d).as_deref(), Some("list"))
                            && args.iter().all(|t| self.satisfies(t, "Ord")))
                        || Some(*d) == self.wk.ordering
                }
                _ => false,
            },
            "Hash" => match ty {
                Ty::Int | Ty::Char | Ty::Str | Ty::Bool | Ty::Unit => true,
                Ty::Float | Ty::Fn(_) => false,
                Ty::Tuple(e, _) => e.iter().all(|t| self.satisfies(t, "Hash")),
                Ty::ErrSet(_) => true,
                Ty::Adt(d, args) => match builtin(*d).as_deref() {
                    Some("list" | "dict" | "set" | "Iter" | "Entry") => false,
                    _ => args.iter().all(|t| self.satisfies(t, "Hash")),
                },
                _ => false,
            },
            _ => false,
        }
    }
}
