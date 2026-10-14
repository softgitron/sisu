//! Places as the programmer wrote them, for diagnostics: `u.name` for
//! `_1.*.0`, `xs[...]` for an element. A temporary has no name, so it is
//! described by the source text of the expression that made it.

use crate::Arg;
use crate::types::Types;
use sisu_sir::{Function, LocalId, Operand, Place, Proj, Rvalue, StatementKind, Terminator};

/// The name of a local, or `_N` for an unnamed one.
pub fn local_name(f: &Function, l: LocalId) -> String {
    match f.locals.get(l.0 as usize).and_then(|d| d.name.as_ref()) {
        Some(n) => n.clone(),
        None => format!("_{}", l.0),
    }
}

/// Whether a local has a source name.
pub fn is_named(f: &Function, l: LocalId) -> bool {
    f.locals.get(l.0 as usize).is_some_and(|d| d.name.as_ref().is_some_and(|n| !n.is_empty()))
}

/// Where an unnamed local gets its value: its first assignment.
pub fn temp_source(f: &Function, l: LocalId) -> Option<sisu_sir::Loc> {
    for b in &f.blocks {
        for s in &b.stmts {
            let d = match &s.kind {
                StatementKind::Assign(p, _) | StatementKind::Call { dest: p, .. } => p,
                _ => continue,
            };
            if d.local == l && d.proj.is_empty() {
                return s.loc;
            }
        }
    }
    None
}

/// A place in source terms.
pub fn place_name(types: &Types, f: &Function, p: &Place) -> Arg {
    if !is_named(f, p.local)
        && let Some(loc) = temp_source(f, p.local)
        && loc.has_end()
    {
        return Arg::Source(loc);
    }
    Arg::Text(place_text(types, f, p))
}

pub fn place_text(types: &Types, f: &Function, p: &Place) -> String {
    let mut s = local_name(f, p.local);
    let mut ty = f.locals.get(p.local.0 as usize).map(|d| d.ty.clone());
    let mut variant: Option<u32> = None;
    for proj in &p.proj {
        match proj {
            Proj::Deref => {
                ty = match ty {
                    Some(sisu_sir::Type::Ref(inner, _)) => Some(*inner),
                    _ => None,
                };
            }
            Proj::Downcast(v) => variant = Some(*v),
            Proj::Field(i) => {
                let name = ty.as_ref().and_then(|t| types.field_name(t, variant, *i));
                // A payload field of an option or result is the value inside.
                let is_payload = variant.is_some() && name.is_none();
                if !is_payload {
                    s.push('.');
                    s.push_str(&name.unwrap_or_else(|| i.to_string()));
                }
                ty = ty.and_then(|t| types.field_ty(&t, variant, *i));
                variant = None;
            }
            Proj::Index(_) | Proj::IndexFromEnd(_) | Proj::Key(_) => {
                s.push_str("[...]");
                ty = None;
            }
        }
    }
    s
}

/// Whether a drop is the first half of an assignment over the place (the
/// old value goes), rather than the end of the place's scope. Elaboration
/// may have narrowed the drop to the parts that hold something, so the
/// assignment may be to a place containing the dropped one.
pub fn is_overwrite(f: &Function, b: usize, place: &Place) -> bool {
    let Terminator::Goto(t) = &f.blocks[b].term else { return false };
    let Some(next) = f.blocks.get(t.0 as usize) else { return false };
    next.stmts
        .iter()
        .find(|s| !matches!(s.kind, StatementKind::Assign(_, Rvalue::Use(Operand::Const(_)))))
        .is_some_and(|s| {
            matches!(&s.kind, StatementKind::Assign(p, _)
                if p.local == place.local && place.proj.starts_with(&p.proj))
        })
}
