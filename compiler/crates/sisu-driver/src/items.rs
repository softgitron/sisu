//! What the per-item queries know of a module before it is resolved
//! (docs/architecture.md, "The query layer"): its interface, a hash of
//! everything name resolution reads of its text. A function body's
//! statements are left out of it unless resolution reads them: those of a
//! generic or `comptime` function (a generic function that reflects is a
//! template, 08), a body's leading contract clauses (the compiler writes
//! property tests from them, 07), and a body that names `arbitrary`,
//! `exhaustive`, `Wrap`, `Fake` or `reflect` (the generators, interceptors
//! and fakes the resolver writes for what a program names). An edit inside
//! any other body leaves the interface as it was, so resolution gives the
//! same definitions and only the edited body is checked again.

use sisu_diag::TextRange;
use sisu_syntax::SyntaxKind;
use sisu_syntax::ast::{self, ItemKind, StmtKind};
use std::hash::{DefaultHasher, Hash, Hasher};

/// The names whose use in a body resolution reads.
const READ_BY_RESOLUTION: &[&str] = &["arbitrary", "exhaustive", "Wrap", "Fake", "reflect"];

/// The statements of a function's body that resolution does not read:
/// from the first one after its contract clauses to the body's end.
fn body_rest(f: &ast::FnDef) -> Option<TextRange> {
    if !f.generics.is_empty() || f.is_comptime {
        return None;
    }
    let b = f.body.as_ref()?;
    let first = b.stmts.iter().find(|s| !matches!(s.kind, StmtKind::Contract(_)))?;
    Some(TextRange::new(first.range.start, b.range.end))
}

fn bodies(items: &[ast::Item], out: &mut Vec<TextRange>) {
    for item in items {
        match &item.kind {
            ItemKind::Fn(f) => out.extend(body_rest(f)),
            ItemKind::Struct(s) => out.extend(s.methods.iter().filter_map(body_rest)),
            ItemKind::Enum(e) | ItemKind::ErrorSet(e) => out.extend(e.methods.iter().filter_map(body_rest)),
            ItemKind::Impl(i) => out.extend(i.methods.iter().filter_map(body_rest)),
            ItemKind::Trait(t) => out.extend(t.methods.iter().filter_map(body_rest)),
            ItemKind::Test(t) | ItemKind::Bench(t) => out.extend(t.body.as_ref().map(|b| b.range)),
            ItemKind::Suite(s) => bodies(&s.items, out),
            _ => {}
        }
    }
}

/// The module's interface hash: its tokens other than spaces, line breaks
/// and plain comments (doc comments are kept: they hold doc tests), in
/// order, with each body resolution does not read replaced by a marker.
pub fn interface_hash(tree: &sisu_syntax::Tree, module: &ast::Module) -> u64 {
    let mut skip = Vec::new();
    bodies(&module.items, &mut skip);
    skip.sort_by_key(|r| r.start);
    let tokens = tree.tokens();
    let text =
        |t: &sisu_syntax::Token| tree.text().get(t.range.start as usize..t.range.end as usize).unwrap_or("");
    // A body that names what resolution reads is kept.
    let mut read = vec![false; skip.len()];
    for t in tokens.iter().filter(|t| t.kind == SyntaxKind::IDENT && READ_BY_RESOLUTION.contains(&text(t))) {
        let i = skip.partition_point(|r| r.end <= t.range.start);
        if skip.get(i).is_some_and(|r| r.start <= t.range.start) {
            read[i] = true;
        }
    }
    let mut i = 0;
    skip.retain(|_| {
        i += 1;
        !read[i - 1]
    });
    let mut h = DefaultHasher::new();
    let mut k = 0;
    let mut marked = false;
    for t in &tokens {
        if matches!(t.kind, SyntaxKind::WHITESPACE | SyntaxKind::COMMENT | SyntaxKind::LINE_BREAK) {
            continue;
        }
        while k < skip.len() && skip[k].end <= t.range.start {
            k += 1;
            marked = false;
        }
        if let Some(r) = skip.get(k)
            && r.start <= t.range.start
            && t.range.end <= r.end
        {
            if !marked {
                "{body}".hash(&mut h);
                marked = true;
            }
            continue;
        }
        t.kind.hash(&mut h);
        text(t).hash(&mut h);
    }
    h.finish()
}
