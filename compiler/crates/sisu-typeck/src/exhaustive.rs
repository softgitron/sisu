//! Exhaustiveness and reachability of `match` (03, "`match` is
//! exhaustive"), by the usefulness algorithm over constructor spaces
//! (Maranget, "Warnings for pattern matching", 2007). A witness of the
//! first missing case names it in the diagnostic.
//!
//! Types with infinitely many values (integers, strings) need a `case _`
//! or a binding; an open error set needs a wildcard (05). Lists are split
//! by length: with `L` one more than the longest fixed part of any list
//! pattern in the column, the constructors are "exactly 0", ..., "exactly
//! L - 1" and "L or more", which is how `[]`, `[x]` and `[a, b, *rest]`
//! together are exhaustive.

use crate::check::Checker;
use crate::thir::*;
use sisu_diag::{TextRange, msg};
use sisu_resolve::{DefId, DefKind, Ty};

#[derive(Clone, Debug, PartialEq)]
enum Ctor {
    Variant(DefId, u32),
    Struct(DefId),
    Tuple(usize),
    Bool(bool),
    /// A list of exactly `n` elements, or of `n` or more when the flag is set.
    List(usize, bool),
    /// A literal: never part of a complete signature.
    Other(String),
}

#[derive(Clone, Debug)]
enum Sp {
    Wild,
    C(Ctor, Vec<Sp>),
    /// A list pattern: its leading elements, and its trailing ones when it
    /// has a `*rest` (so it matches any longer list too).
    Slice(Vec<Sp>, Option<Vec<Sp>>),
}

/// The fewest elements a list pattern matches, and whether it matches more.
fn slice_len(sp: &Sp) -> Option<(usize, bool)> {
    match sp {
        Sp::Slice(p, None) => Some((p.len(), false)),
        Sp::Slice(p, Some(s)) => Some((p.len() + s.len(), true)),
        _ => None,
    }
}

/// Whether a column head matches every value of a constructor.
fn covers(head: &Sp, c: &Ctor) -> bool {
    match (head, c) {
        (Sp::C(k, _), c) => k == c,
        (Sp::Slice(..), Ctor::List(m, more)) => match slice_len(head) {
            Some((n, false)) => !*more && n == *m,
            Some((n, true)) => n <= *m,
            None => false,
        },
        _ => false,
    }
}

/// The sub-patterns of a list pattern seen as a list of `m` elements.
fn slice_fields(head: &Sp, m: usize) -> Option<Vec<Sp>> {
    match head {
        Sp::Slice(p, None) if p.len() == m => Some(p.clone()),
        Sp::Slice(p, Some(s)) if p.len() + s.len() <= m => {
            let mut v = p.clone();
            v.extend((0..m - p.len() - s.len()).map(|_| Sp::Wild));
            v.extend(s.iter().cloned());
            Some(v)
        }
        _ => None,
    }
}

impl Checker<'_> {
    /// The constructors of a type with their field types, or `None` for a
    /// type with infinitely many.
    fn ctors(&self, ty: &Ty) -> Option<Vec<(Ctor, Vec<Ty>)>> {
        let prog = self.prog;
        match self.infer.zonk(ty) {
            Ty::Bool => Some(vec![(Ctor::Bool(true), vec![]), (Ctor::Bool(false), vec![])]),
            Ty::Tuple(es, _) => Some(vec![(Ctor::Tuple(es.len()), es)]),
            Ty::Ref(t, _) => self.ctors(&t),
            Ty::Never | Ty::Error => Some(vec![]),
            Ty::ErrSet(members) => {
                let mut out = Vec::new();
                for d in members {
                    let e = prog.enum_info(d)?;
                    if e.open {
                        return None;
                    }
                    for (i, v) in e.variants.iter().enumerate() {
                        out.push((
                            Ctor::Variant(d, i as u32),
                            v.fields.iter().map(|(_, t)| t.clone()).collect(),
                        ));
                    }
                }
                Some(out)
            }
            Ty::Adt(d, args) => match &prog.def(d).kind {
                DefKind::Enum(e) => Some(
                    e.variants
                        .iter()
                        .enumerate()
                        .map(|(i, v)| {
                            (
                                Ctor::Variant(d, i as u32),
                                v.fields.iter().map(|(_, t)| t.subst(&args)).collect(),
                            )
                        })
                        .collect(),
                ),
                DefKind::Struct(s) if s.builtin.is_none() => {
                    Some(vec![(Ctor::Struct(d), s.fields.iter().map(|f| f.ty.subst(&args)).collect())])
                }
                _ => None,
            },
            _ => None,
        }
    }

    /// The constructors of a column's type, given the patterns at its head
    /// (lists are split by the lengths those patterns mention).
    fn ctors_in<'s>(&self, ty: &Ty, heads: impl Iterator<Item = &'s Sp>) -> Option<Vec<(Ctor, Vec<Ty>)>> {
        let prog = self.prog;
        let t = self.infer.zonk(ty);
        let t = match t {
            Ty::Ref(inner, _) => *inner,
            other => other,
        };
        if let Ty::Adt(d, args) = &t
            && Some(*d) == prog.wk.list
        {
            let elem = args.first().cloned().unwrap_or(Ty::Error);
            let lens: Vec<usize> = heads.filter_map(slice_len).map(|(n, _)| n).collect();
            let bound = lens.iter().max()? + 1;
            let mut out: Vec<(Ctor, Vec<Ty>)> =
                (0..bound).map(|m| (Ctor::List(m, false), vec![elem.clone(); m])).collect();
            out.push((Ctor::List(bound, true), vec![elem; bound]));
            return Some(out);
        }
        self.ctors(ty)
    }

    fn render(&self, c: &Ctor, args: &[String]) -> String {
        let prog = self.prog;
        let with_args =
            |name: String| if args.is_empty() { name } else { format!("{name}({})", args.join(", ")) };
        match c {
            Ctor::Variant(d, i) => with_args(
                prog.enum_info(*d)
                    .and_then(|e| e.variants.get(*i as usize))
                    .map(|v| v.name.clone())
                    .unwrap_or_default(),
            ),
            Ctor::Struct(d) => with_args(prog.def(*d).name.clone()),
            Ctor::Tuple(_) => format!("({})", args.join(", ")),
            Ctor::Bool(b) => {
                if *b {
                    "True".into()
                } else {
                    "False".into()
                }
            }
            Ctor::List(_, false) => format!("[{}]", args.join(", ")),
            Ctor::List(_, true) => {
                let mut parts = args.to_vec();
                parts.push("*_".into());
                format!("[{}]", parts.join(", "))
            }
            Ctor::Other(s) => s.clone(),
        }
    }

    /// A witness of a value no row matches, one rendered pattern per column.
    fn witness(&self, rows: &[Vec<Sp>], tys: &[Ty]) -> Option<Vec<String>> {
        let Some(head) = tys.first() else {
            return if rows.is_empty() { Some(Vec::new()) } else { None };
        };
        let used = |c: &Ctor| rows.iter().any(|r| r.first().is_some_and(|h| covers(h, c)));
        match self.ctors_in(head, rows.iter().filter_map(|r| r.first())) {
            Some(cs) if !cs.is_empty() && cs.iter().all(|(c, _)| used(c)) => {
                for (c, ftys) in &cs {
                    let spec = specialize(rows, c, ftys.len());
                    let mut sub_tys = ftys.clone();
                    sub_tys.extend_from_slice(&tys[1..]);
                    if let Some(w) = self.witness(&spec, &sub_tys) {
                        let (fw, rest) = w.split_at(ftys.len());
                        let mut out = vec![self.render(c, fw)];
                        out.extend_from_slice(rest);
                        return Some(out);
                    }
                }
                None
            }
            Some(cs) => {
                let def = default_rows(rows);
                let rest = self.witness(&def, &tys[1..])?;
                let missing = cs.iter().find(|(c, _)| !used(c)).map(|(c, ftys)| {
                    let wilds: Vec<String> = ftys.iter().map(|_| "_".to_string()).collect();
                    self.render(c, &wilds)
                });
                let mut out = vec![missing.unwrap_or_else(|| "_".into())];
                out.extend(rest);
                Some(out)
            }
            None => {
                let def = default_rows(rows);
                let rest = self.witness(&def, &tys[1..])?;
                let mut out = vec!["_".to_string()];
                out.extend(rest);
                Some(out)
            }
        }
    }

    /// Reports a non-exhaustive `match` and unreachable arms.
    pub fn check_exhaustive(&mut self, ty: &Ty, arms: &[Arm], range: TextRange) {
        if self.infer.zonk(ty).contains_error() || arms.iter().any(|a| pat_has_error(&a.pat)) {
            return;
        }
        let mut rows: Vec<Vec<Sp>> = Vec::new();
        for a in arms {
            let alts = expand(&a.pat);
            if a.guard.is_none() {
                let reachable = alts.iter().any(|sp| self.witness_useful(&rows, sp, ty));
                if !reachable && !rows.is_empty() {
                    let d = self.error(msg::TYPE_UNREACHABLE_ARM, a.pat.span.range);
                    self.report(d);
                }
                for sp in alts {
                    rows.push(vec![sp]);
                }
            }
        }
        if let Some(w) = self.witness(&rows, std::slice::from_ref(ty)) {
            let missing = format!("`{}`", w.join(", "));
            let open = matches!(self.infer.zonk(ty), Ty::ErrSet(ref m) if m.iter().any(|d| self.prog.enum_info(*d).is_some_and(|e| e.open)));
            let d = if open {
                self.error(msg::TYPE_OPEN_SET_NEEDS_WILDCARD, range).with("ty", self.show(ty))
            } else {
                self.error(msg::TYPE_NON_EXHAUSTIVE, range)
                    .with("missing", missing.clone())
                    .note(msg::TYPE_MISSING_PATTERNS, vec![("missing", missing)])
            };
            self.report(d);
        }
    }

    /// Whether a row adds a value not matched by the rows before it.
    fn witness_useful(&self, rows: &[Vec<Sp>], sp: &Sp, ty: &Ty) -> bool {
        // A row is useful when some value matches it and none of `rows`.
        useful(self, rows, std::slice::from_ref(sp), std::slice::from_ref(ty))
    }
}

fn useful(c: &Checker, rows: &[Vec<Sp>], v: &[Sp], tys: &[Ty]) -> bool {
    let Some(head) = v.first() else { return rows.is_empty() };
    match head {
        Sp::C(ctor, args) => {
            let spec = specialize(rows, ctor, args.len());
            let field_tys = c
                .ctors(&tys[0])
                .and_then(|cs| cs.into_iter().find(|(k, _)| k == ctor).map(|(_, f)| f))
                .unwrap_or_else(|| args.iter().map(|_| Ty::Error).collect());
            let mut nv = args.clone();
            nv.extend_from_slice(&v[1..]);
            let mut nt = field_tys;
            nt.extend_from_slice(&tys[1..]);
            useful(c, &spec, &nv, &nt)
        }
        Sp::Slice(..) => {
            // Useful if it adds a value for any list length it covers.
            let heads = rows.iter().filter_map(|r| r.first()).chain(std::iter::once(head));
            let Some(cs) = c.ctors_in(&tys[0], heads) else { return false };
            cs.iter().filter(|(k, _)| covers(head, k)).any(|(k, ftys)| {
                let Ctor::List(m, _) = k else { return false };
                let spec = specialize(rows, k, *m);
                let mut nv = slice_fields(head, *m).unwrap_or_default();
                nv.extend_from_slice(&v[1..]);
                let mut nt = ftys.clone();
                nt.extend_from_slice(&tys[1..]);
                useful(c, &spec, &nv, &nt)
            })
        }
        Sp::Wild => {
            let used = |k: &Ctor| rows.iter().any(|r| r.first().is_some_and(|h| covers(h, k)));
            match c.ctors_in(&tys[0], rows.iter().filter_map(|r| r.first())) {
                Some(cs) if !cs.is_empty() && cs.iter().all(|(k, _)| used(k)) => {
                    cs.iter().any(|(k, ftys)| {
                        let spec = specialize(rows, k, ftys.len());
                        let mut nv: Vec<Sp> = ftys.iter().map(|_| Sp::Wild).collect();
                        nv.extend_from_slice(&v[1..]);
                        let mut nt = ftys.clone();
                        nt.extend_from_slice(&tys[1..]);
                        useful(c, &spec, &nv, &nt)
                    })
                }
                _ => useful(c, &default_rows(rows), &v[1..], &tys[1..]),
            }
        }
    }
}

fn specialize(rows: &[Vec<Sp>], c: &Ctor, arity: usize) -> Vec<Vec<Sp>> {
    let mut out = Vec::new();
    for r in rows {
        match r.first() {
            Some(Sp::C(k, args)) if k == c => {
                let mut n = args.clone();
                n.extend_from_slice(&r[1..]);
                out.push(n);
            }
            Some(h @ Sp::Slice(..)) if covers(h, c) => {
                let mut n = slice_fields(h, arity).unwrap_or_default();
                n.extend_from_slice(&r[1..]);
                out.push(n);
            }
            Some(Sp::Wild) => {
                let mut n: Vec<Sp> = (0..arity).map(|_| Sp::Wild).collect();
                n.extend_from_slice(&r[1..]);
                out.push(n);
            }
            _ => {}
        }
    }
    out
}

fn default_rows(rows: &[Vec<Sp>]) -> Vec<Vec<Sp>> {
    rows.iter().filter(|r| matches!(r.first(), Some(Sp::Wild))).map(|r| r[1..].to_vec()).collect()
}

fn pat_has_error(p: &Pat) -> bool {
    match &p.kind {
        PatKind::Error => true,
        PatKind::Variant { fields, .. } | PatKind::Struct { fields, .. } => fields.iter().any(pat_has_error),
        PatKind::Tuple(ps) | PatKind::Or(ps) => ps.iter().any(pat_has_error),
        PatKind::Deref(p) => pat_has_error(p),
        PatKind::List { prefix, suffix, .. } => prefix.iter().chain(suffix).any(pat_has_error),
        _ => false,
    }
}

/// The alternatives of a pattern, or-patterns expanded at every depth.
fn expand(p: &Pat) -> Vec<Sp> {
    match &p.kind {
        PatKind::Wild | PatKind::Bind { .. } | PatKind::Error => vec![Sp::Wild],
        PatKind::Deref(inner) => expand(inner),
        PatKind::Lit(Lit::Bool(b)) => vec![Sp::C(Ctor::Bool(*b), vec![])],
        PatKind::Lit(l) => vec![Sp::C(Ctor::Other(format!("{l:?}")), vec![])],
        PatKind::List { prefix, rest, suffix } => {
            let all: Vec<Pat> = prefix.iter().chain(suffix.iter()).cloned().collect();
            product(&all)
                .into_iter()
                .map(|mut args| {
                    let suf = args.split_off(prefix.len());
                    Sp::Slice(args, if rest.is_some() { Some(suf) } else { None })
                })
                .collect()
        }
        PatKind::Or(ps) => ps.iter().flat_map(expand).collect(),
        PatKind::Variant { def, variant, fields } => {
            product(fields).into_iter().map(|args| Sp::C(Ctor::Variant(*def, *variant), args)).collect()
        }
        PatKind::Struct { def, fields } => {
            product(fields).into_iter().map(|args| Sp::C(Ctor::Struct(*def), args)).collect()
        }
        PatKind::Tuple(ps) => {
            product(ps).into_iter().map(|args| Sp::C(Ctor::Tuple(ps.len()), args)).collect()
        }
    }
}

fn product(fields: &[Pat]) -> Vec<Vec<Sp>> {
    let mut acc: Vec<Vec<Sp>> = vec![Vec::new()];
    for f in fields {
        let alts = expand(f);
        let mut next = Vec::new();
        for a in &acc {
            for alt in &alts {
                let mut v = a.clone();
                v.push(alt.clone());
                next.push(v);
            }
        }
        acc = next;
        if acc.len() > 4096 {
            // Pathological or-patterns: give up precision, never correctness of the report.
            acc.truncate(4096);
        }
    }
    acc
}
