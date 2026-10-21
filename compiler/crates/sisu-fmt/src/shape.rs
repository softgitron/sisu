//! The shape of a tree: what formatting must keep. Its nodes and their
//! significant tokens, with what the formatter changes on purpose made
//! equal: trailing commas, `T?` and `Option[T]`, `T ! E` and
//! `Result[T, E]`, unused imports, and the order of a run of imports and
//! of the names a `from` imports. `sisu fmt`
//! checks that its output has the input's shape, and `sisu parse --shape`
//! prints it, so the conformance runner can check any implementation's
//! formatter.

use sisu_syntax::SyntaxKind::{self, *};
use sisu_syntax::{Element, NodeId, Tree};
use std::collections::HashSet;

/// The shape of a file's text; `None` when it has a syntax error.
pub fn shape(text: &str) -> Option<String> {
    let parse = sisu_parser::parse(sisu_diag::FileId(0), text);
    if parse.diagnostics.iter().any(|d| d.is_error()) {
        return None;
    }
    Some(shape_of(&parse.tree))
}

/// The shape of a tree, one node per line, indented.
pub fn shape_of(tree: &Tree) -> String {
    let mut out = String::new();
    let pruned = crate::prune::unused(tree);
    node(tree, tree.root(), 0, &pruned, &mut out);
    out
}

fn is_close(k: SyntaxKind) -> bool {
    matches!(k, R_PAREN | R_BRACKET | R_BRACE)
}

fn node(tree: &Tree, n: NodeId, depth: usize, pruned: &HashSet<NodeId>, out: &mut String) {
    let kind = tree.kind(n);
    let pad = "  ".repeat(depth);
    if kind == STRING_EXPR {
        out.push_str(&format!("{pad}STRING {:?}\n", tree.node_text(n)));
        return;
    }
    // `Result[T, E]` and `T ! E` are one shape.
    if let Some((t, e)) = crate::lines::result_parts(tree, n) {
        out.push_str(&format!("{pad}RESULT_TYPE\n"));
        node(tree, t, depth + 1, pruned, out);
        out.push_str(&format!("{pad}  BANG\n"));
        node(tree, e, depth + 1, pruned, out);
        return;
    }
    // `Option[T]` and `T?` are one shape.
    if let Some(arg) = crate::lines::option_sugar(tree, n) {
        out.push_str(&format!("{pad}OPTION_TYPE\n"));
        node(tree, arg, depth + 1, pruned, out);
        return;
    }
    if kind == OPTION_TYPE {
        out.push_str(&format!("{pad}OPTION_TYPE\n"));
        for c in tree.child_nodes(n) {
            node(tree, c, depth + 1, pruned, out);
        }
        return;
    }
    out.push_str(&format!("{pad}{}\n", kind.name()));
    let children: Vec<Element> = tree
        .children(n)
        .iter()
        .copied()
        .filter(|e| !matches!(e, Element::Token(t) if tree.token_kind(*t).is_trivia()))
        .collect();
    // The children's shapes, each as its text.
    let mut parts: Vec<(Option<SyntaxKind>, String)> = Vec::new();
    for (i, e) in children.iter().enumerate() {
        match *e {
            // An unused import is left out.
            Element::Node(c) if pruned.contains(&c) => {}
            Element::Node(c) => {
                let mut s = String::new();
                node(tree, c, depth + 1, pruned, &mut s);
                parts.push((Some(tree.kind(c)), s));
            }
            Element::Token(t) => {
                let k = tree.token_kind(t);
                // The names a `from` imports are a set: their commas, which
                // removing one moves, are not part of it.
                if k == COMMA && kind == FROM_IMPORT {
                    continue;
                }
                // A trailing comma: one before a closing bracket, unless it
                // makes a tuple of one.
                if k == COMMA
                    && children
                        .get(i + 1)
                        .is_some_and(|x| matches!(x, Element::Token(c) if is_close(tree.token_kind(*c))))
                {
                    let elems = tree.child_nodes(n).count();
                    if !(matches!(kind, TUPLE_EXPR | TUPLE_TYPE | TUPLE_PAT) && elems == 1) {
                        continue;
                    }
                }
                let shown = match k {
                    IDENT | INT_NUMBER | FLOAT_NUMBER | CHAR_LIT | RAW_STRING | BYTE_STRING | FORMAT_SPEC
                    | STRING_TEXT => format!("{} {:?}", k.name(), tree.token_text(t)),
                    _ => k.name(),
                };
                parts.push((None, format!("{}  {shown}\n", pad)));
            }
        }
    }
    // A run of imports, and the names one `from` imports, in any order.
    if kind == SOURCE_FILE {
        let mut i = 0;
        while i < parts.len() {
            if matches!(parts[i].0, Some(IMPORT_ITEM | FROM_IMPORT)) {
                let mut j = i;
                while j < parts.len() && matches!(parts[j].0, Some(IMPORT_ITEM | FROM_IMPORT)) {
                    j += 1;
                }
                parts[i..j].sort_by(|a, b| a.1.cmp(&b.1));
                i = j;
            } else {
                i += 1;
            }
        }
    }
    if kind == FROM_IMPORT {
        let mut names: Vec<String> =
            parts.iter().filter(|p| p.0 == Some(IMPORT_NAME)).map(|p| p.1.clone()).collect();
        names.sort();
        let mut k = 0;
        for p in parts.iter_mut() {
            if p.0 == Some(IMPORT_NAME) {
                p.1 = names[k].clone();
                k += 1;
            }
        }
    }
    for (_, s) in parts {
        out.push_str(&s);
    }
}
