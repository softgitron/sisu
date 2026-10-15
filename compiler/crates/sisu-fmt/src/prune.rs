//! Unused imports, which `sisu fmt` removes (09, "Unused import/variable
//! as a hard error"). The formatter works without name resolution, so an
//! import is used when the file spells its name: as an identifier outside
//! the imports, or as a word of a doc comment, whose examples are compiled
//! as tests. That is enough, because nothing else needs an import: a
//! method is found through every trait its receiver implements, imported
//! or not. Kept whatever their use: a re-export (`pub` or `pub(package)`),
//! and an import with a comment inside it, which would have nowhere to go.

use sisu_syntax::SyntaxKind::*;
use sisu_syntax::{Element, NodeId, TokenId, Tree};
use std::collections::HashSet;

/// The imports to remove: whole `import` and `from` items, and single
/// names (`IMPORT_NAME` nodes) of a `from` item that keeps others.
pub fn unused(tree: &Tree) -> HashSet<NodeId> {
    let root = tree.root();
    let imports: Vec<NodeId> =
        tree.child_nodes(root).filter(|&n| matches!(tree.kind(n), IMPORT_ITEM | FROM_IMPORT)).collect();
    let mut out = HashSet::new();
    if imports.is_empty() {
        return out;
    }
    let inside = |pos: u32| {
        imports.iter().any(|&n| {
            let r = tree.range(n);
            r.start <= pos && pos < r.end
        })
    };
    let mut used: HashSet<&str> = HashSet::new();
    for i in 0..tree.token_count() {
        let t = TokenId(i as u32);
        let kind = tree.token_kind(t);
        let text = tree.token_text(t);
        match kind {
            IDENT if !inside(tree.token_range(t).start) => {
                used.insert(text);
            }
            DOC_COMMENT | MODULE_DOC_COMMENT => {
                for w in text.split(|c: char| !(c.is_alphanumeric() || c == '_')) {
                    if !w.is_empty() {
                        used.insert(w);
                    }
                }
            }
            _ => {}
        }
    }
    for n in imports {
        if tree.first_child(n, VISIBILITY).is_some() || has_comment(tree, n) {
            continue;
        }
        match tree.kind(n) {
            IMPORT_ITEM => {
                if binding(tree, n).is_some_and(|b| !used.contains(b)) {
                    out.insert(n);
                }
            }
            _ => {
                let names: Vec<NodeId> =
                    tree.child_nodes(n).filter(|&c| tree.kind(c) == IMPORT_NAME).collect();
                let unused: Vec<NodeId> = names
                    .iter()
                    .copied()
                    .filter(|&c| binding(tree, c).is_some_and(|b| !used.contains(b)))
                    .collect();
                if !names.is_empty() && unused.len() == names.len() {
                    out.insert(n);
                } else {
                    out.extend(unused);
                }
            }
        }
    }
    out
}

/// The name an import binds: the one after `as`, or else the last part of
/// the module path (`import std.json`) or the imported name.
fn binding(tree: &Tree, n: NodeId) -> Option<&str> {
    let toks: Vec<TokenId> = tree.child_tokens(n).collect();
    if let Some(at) = toks.iter().position(|&t| tree.token_kind(t) == AS_KW) {
        return toks[at + 1..].iter().find(|&&t| tree.token_kind(t) == IDENT).map(|&t| tree.token_text(t));
    }
    if tree.kind(n) == IMPORT_ITEM {
        let path = tree.first_child(n, MODULE_PATH)?;
        return tree
            .child_tokens(path)
            .filter(|&t| tree.token_kind(t) == IDENT)
            .last()
            .map(|t| tree.token_text(t));
    }
    toks.iter().find(|&&t| tree.token_kind(t) == IDENT).map(|&t| tree.token_text(t))
}

fn has_comment(tree: &Tree, n: NodeId) -> bool {
    tree.children(n).iter().any(|e| match *e {
        Element::Token(t) => matches!(tree.token_kind(t), COMMENT | DOC_COMMENT | MODULE_DOC_COMMENT),
        Element::Node(c) => has_comment(tree, c),
    })
}
