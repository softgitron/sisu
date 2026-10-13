//! The lossless syntax tree.
//!
//! Every byte of the source belongs to exactly one token, and every token
//! to exactly one node, so concatenating the tokens in tree order
//! reproduces the input byte for byte (`sisu parse --print`). Layout tokens
//! (INDENT, DEDENT, EOF) are zero-width. The tree is an arena: nodes and
//! tokens are addressed by indices, which keeps it free of shared pointers
//! and easy to transcribe.
//!
//! [`NodeId`] is an index into one tree. [`StableId`] names a node in a way
//! that survives edits elsewhere in the file: an item is named by its kind,
//! its name and its position among same-named siblings, and a node inside
//! an item by its pre-order index within that item.

use crate::kinds::SyntaxKind;
use sisu_diag::TextRange;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TokenId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Element {
    Node(NodeId),
    Token(TokenId),
}

/// A token as produced by the lexer: a kind and a byte range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token {
    pub kind: SyntaxKind,
    pub range: TextRange,
}

#[derive(Clone, Debug)]
struct NodeData {
    kind: SyntaxKind,
    range: TextRange,
    parent: Option<NodeId>,
    children: Vec<Element>,
}

#[derive(Clone, Debug)]
struct TokenData {
    kind: SyntaxKind,
    range: TextRange,
    parent: Option<NodeId>,
}

#[derive(Clone, Debug)]
pub struct Tree {
    text: String,
    nodes: Vec<NodeData>,
    tokens: Vec<TokenData>,
    root: NodeId,
}

impl Tree {
    pub fn root(&self) -> NodeId {
        self.root
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn kind(&self, node: NodeId) -> SyntaxKind {
        self.nodes.get(node.0 as usize).map(|n| n.kind).unwrap_or(SyntaxKind::ERROR)
    }

    pub fn range(&self, node: NodeId) -> TextRange {
        self.nodes.get(node.0 as usize).map(|n| n.range).unwrap_or_default()
    }

    pub fn parent(&self, node: NodeId) -> Option<NodeId> {
        self.nodes.get(node.0 as usize).and_then(|n| n.parent)
    }

    pub fn children(&self, node: NodeId) -> &[Element] {
        self.nodes.get(node.0 as usize).map(|n| n.children.as_slice()).unwrap_or(&[])
    }

    pub fn child_nodes(&self, node: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        self.children(node).iter().filter_map(|e| match e {
            Element::Node(n) => Some(*n),
            Element::Token(_) => None,
        })
    }

    /// Child tokens that are not trivia.
    pub fn child_tokens(&self, node: NodeId) -> impl Iterator<Item = TokenId> + '_ {
        self.children(node).iter().filter_map(move |e| match e {
            Element::Token(t) if !self.token_kind(*t).is_trivia() => Some(*t),
            _ => None,
        })
    }

    pub fn first_child(&self, node: NodeId, kind: SyntaxKind) -> Option<NodeId> {
        self.child_nodes(node).find(|&n| self.kind(n) == kind)
    }

    pub fn first_token(&self, node: NodeId, kind: SyntaxKind) -> Option<TokenId> {
        self.child_tokens(node).find(|&t| self.token_kind(t) == kind)
    }

    pub fn has_token(&self, node: NodeId, kind: SyntaxKind) -> bool {
        self.first_token(node, kind).is_some()
    }

    pub fn token_kind(&self, token: TokenId) -> SyntaxKind {
        self.tokens.get(token.0 as usize).map(|t| t.kind).unwrap_or(SyntaxKind::ERROR_TOKEN)
    }

    pub fn token_range(&self, token: TokenId) -> TextRange {
        self.tokens.get(token.0 as usize).map(|t| t.range).unwrap_or_default()
    }

    pub fn token_text(&self, token: TokenId) -> &str {
        let r = self.token_range(token);
        self.text.get(r.start as usize..r.end as usize).unwrap_or("")
    }

    pub fn token_parent(&self, token: TokenId) -> Option<NodeId> {
        self.tokens.get(token.0 as usize).and_then(|t| t.parent)
    }

    pub fn node_text(&self, node: NodeId) -> &str {
        let r = self.range(node);
        self.text.get(r.start as usize..r.end as usize).unwrap_or("")
    }

    pub fn token_count(&self) -> usize {
        self.tokens.len()
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// All tokens in source order, as the lexer produced them.
    pub fn tokens(&self) -> Vec<Token> {
        self.tokens.iter().map(|t| Token { kind: t.kind, range: t.range }).collect()
    }

    /// Reprints the source from the tree.
    pub fn to_source(&self) -> String {
        let mut out = String::with_capacity(self.text.len());
        self.write_source(self.root, &mut out);
        out
    }

    fn write_source(&self, node: NodeId, out: &mut String) {
        for e in self.children(node) {
            match e {
                Element::Node(n) => self.write_source(*n, out),
                Element::Token(t) => out.push_str(self.token_text(*t)),
            }
        }
    }

    /// Nodes of the subtree in pre-order, `node` first.
    pub fn preorder(&self, node: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        let mut stack = vec![node];
        while let Some(n) = stack.pop() {
            out.push(n);
            let kids: Vec<NodeId> = self.child_nodes(n).collect();
            for k in kids.into_iter().rev() {
                stack.push(k);
            }
        }
        out
    }

    /// An indented dump: one line per node and token, with byte ranges.
    pub fn dump(&self, with_trivia: bool) -> String {
        let mut out = String::new();
        self.dump_node(self.root, 0, with_trivia, &mut out);
        out
    }

    fn dump_node(&self, node: NodeId, depth: usize, with_trivia: bool, out: &mut String) {
        let r = self.range(node);
        out.push_str(&format!("{}{} {}..{}\n", "  ".repeat(depth), self.kind(node).name(), r.start, r.end));
        for e in self.children(node) {
            match e {
                Element::Node(n) => self.dump_node(*n, depth + 1, with_trivia, out),
                Element::Token(t) => {
                    let kind = self.token_kind(*t);
                    if kind.is_trivia() && !with_trivia {
                        continue;
                    }
                    let r = self.token_range(*t);
                    out.push_str(&format!(
                        "{}{} {}..{} {:?}\n",
                        "  ".repeat(depth + 1),
                        kind.name(),
                        r.start,
                        r.end,
                        self.token_text(*t)
                    ));
                }
            }
        }
    }

    /// The name token of an item node: the first IDENT after the item's
    /// keyword, or the string of a test.
    fn item_name(&self, node: NodeId) -> String {
        let mut seen_keyword = false;
        for e in self.children(node) {
            match e {
                Element::Token(t) => {
                    let k = self.token_kind(*t);
                    if k.is_keyword() {
                        seen_keyword = true;
                    } else if k == SyntaxKind::IDENT && seen_keyword {
                        return self.token_text(*t).to_string();
                    }
                }
                Element::Node(n) if seen_keyword && self.kind(*n) == SyntaxKind::STRING_EXPR => {
                    return self.node_text(*n).to_string();
                }
                Element::Node(n)
                    if self.kind(*n) == SyntaxKind::PATH_TYPE || self.kind(*n) == SyntaxKind::MODULE_PATH =>
                {
                    return self.node_text(*n).to_string();
                }
                _ => {}
            }
        }
        String::new()
    }

    /// Whether a node kind is a declaration (for stable identities and for
    /// telling declarations from top-level code that failed to parse).
    pub fn is_item_kind(kind: SyntaxKind) -> bool {
        Self::is_item(kind)
    }

    fn is_item(kind: SyntaxKind) -> bool {
        use SyntaxKind::*;
        matches!(
            kind,
            FN_DEF
                | STRUCT_DEF
                | ENUM_DEF
                | ERROR_DEF
                | IMPL_DEF
                | TRAIT_DEF
                | IMPORT_ITEM
                | FROM_IMPORT
                | TEST_DEF
                | SUITE_DEF
                | BENCH_DEF
                | CONST_DEF
                | TYPE_ALIAS
                | EXTERN_BLOCK
        )
    }

    /// The stable identity of a node; see the module documentation.
    pub fn stable_id(&self, node: NodeId) -> StableId {
        // Find the innermost item containing the node.
        let mut item = None;
        let mut cur = Some(node);
        while let Some(n) = cur {
            if Self::is_item(self.kind(n)) {
                item = Some(n);
                break;
            }
            cur = self.parent(n);
        }
        let Some(item) = item else {
            return StableId {
                item: Vec::new(),
                local: self.preorder(self.root).iter().position(|&n| n == node).unwrap_or(0) as u32,
            };
        };
        let local = self.preorder(item).iter().position(|&n| n == node).unwrap_or(0) as u32;
        StableId { item: self.item_path(item), local }
    }

    fn item_path(&self, item: NodeId) -> Vec<String> {
        let mut path = Vec::new();
        let mut cur = Some(item);
        while let Some(n) = cur {
            if Self::is_item(self.kind(n)) {
                let name = self.item_name(n);
                let kind = self.kind(n);
                // Ordinal among earlier siblings with the same kind and name.
                let ordinal = match self.parent(n) {
                    Some(p) => self
                        .child_nodes(p)
                        .take_while(|&s| s != n)
                        .filter(|&s| self.kind(s) == kind && self.item_name(s) == name)
                        .count(),
                    None => 0,
                };
                path.push(format!("{}:{}#{}", kind.name(), name, ordinal));
            }
            cur = self.parent(n);
        }
        path.reverse();
        path
    }
}

/// A node identity that survives edits outside the node's item.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StableId {
    /// The path of enclosing items, outermost first, as `KIND:name#ordinal`.
    pub item: Vec<String>,
    /// Pre-order index inside the innermost item.
    pub local: u32,
}

/// Builds a [`Tree`] from lexer tokens. The parser attaches tokens in order
/// and opens and closes nodes; `checkpoint` and `start_node_at` wrap
/// already-attached children, which is how left-recursive expressions are
/// built without backtracking.
#[derive(Debug)]
pub struct TreeBuilder {
    text: String,
    tokens: Vec<TokenData>,
    nodes: Vec<NodeData>,
    stack: Vec<(SyntaxKind, usize)>,
    children: Vec<Element>,
    next_token: usize,
    pos: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Checkpoint(usize);

impl TreeBuilder {
    pub fn new(text: String, tokens: &[Token]) -> TreeBuilder {
        TreeBuilder {
            text,
            tokens: tokens.iter().map(|t| TokenData { kind: t.kind, range: t.range, parent: None }).collect(),
            nodes: Vec::new(),
            stack: Vec::new(),
            children: Vec::new(),
            next_token: 0,
            pos: 0,
        }
    }

    pub fn start_node(&mut self, kind: SyntaxKind) {
        self.stack.push((kind, self.children.len()));
    }

    pub fn checkpoint(&self) -> Checkpoint {
        Checkpoint(self.children.len())
    }

    pub fn start_node_at(&mut self, checkpoint: Checkpoint, kind: SyntaxKind) {
        let floor = self.stack.last().map(|&(_, s)| s).unwrap_or(0);
        self.stack.push((kind, checkpoint.0.max(floor).min(self.children.len())));
    }

    /// Attaches the next lexer token.
    pub fn token(&mut self) {
        if self.next_token >= self.tokens.len() {
            return;
        }
        let id = TokenId(self.next_token as u32);
        self.pos = self.tokens[self.next_token].range.end;
        self.next_token += 1;
        self.children.push(Element::Token(id));
    }

    /// The number of tokens attached so far.
    pub fn tokens_attached(&self) -> usize {
        self.next_token
    }

    pub fn finish_node(&mut self) {
        let Some((kind, start)) = self.stack.pop() else {
            return;
        };
        let kids: Vec<Element> = self.children.drain(start..).collect();
        let id = NodeId(self.nodes.len() as u32);
        let mut range: Option<TextRange> = None;
        for e in &kids {
            let r = match e {
                Element::Node(n) => self.nodes[n.0 as usize].range,
                Element::Token(t) => self.tokens[t.0 as usize].range,
            };
            range = Some(match range {
                None => r,
                Some(acc) => acc.cover(r),
            });
            match e {
                Element::Node(n) => self.nodes[n.0 as usize].parent = Some(id),
                Element::Token(t) => self.tokens[t.0 as usize].parent = Some(id),
            }
        }
        let range = range.unwrap_or(TextRange::empty(self.pos));
        self.nodes.push(NodeData { kind, range, parent: None, children: kids });
        self.children.push(Element::Node(id));
    }

    /// Attaches any remaining tokens and closes every open node; the
    /// outermost node becomes the root.
    pub fn finish(mut self) -> Tree {
        while self.next_token < self.tokens.len() {
            self.token();
        }
        while !self.stack.is_empty() {
            self.finish_node();
        }
        if self.children.len() != 1 || !matches!(self.children.first(), Some(Element::Node(_))) {
            // Wrap whatever is left in a root node, so the tree is always well formed.
            self.stack.push((SyntaxKind::SOURCE_FILE, 0));
            self.finish_node();
        }
        let root = match self.children.first() {
            Some(Element::Node(n)) => *n,
            _ => NodeId(0),
        };
        Tree { text: self.text, nodes: self.nodes, tokens: self.tokens, root }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kinds::SyntaxKind::*;

    fn tok(kind: SyntaxKind, s: u32, e: u32) -> Token {
        Token { kind, range: TextRange::new(s, e) }
    }

    #[test]
    fn builds_and_reprints() {
        let text = "a + b".to_string();
        let tokens = [
            tok(IDENT, 0, 1),
            tok(WHITESPACE, 1, 2),
            tok(PLUS, 2, 3),
            tok(WHITESPACE, 3, 4),
            tok(IDENT, 4, 5),
        ];
        let mut b = TreeBuilder::new(text.clone(), &tokens);
        b.start_node(SOURCE_FILE);
        let cp = b.checkpoint();
        b.start_node(NAME_REF);
        b.token();
        b.finish_node();
        b.start_node_at(cp, BIN_EXPR);
        b.token();
        b.token();
        b.token();
        b.start_node(NAME_REF);
        b.token();
        b.finish_node();
        b.finish_node();
        b.finish_node();
        let tree = b.finish();
        assert_eq!(tree.to_source(), text);
        let root = tree.root();
        assert_eq!(tree.kind(root), SOURCE_FILE);
        let bin = tree.child_nodes(root).next().unwrap();
        assert_eq!(tree.kind(bin), BIN_EXPR);
        assert_eq!(tree.range(bin), TextRange::new(0, 5));
        assert_eq!(tree.child_nodes(bin).count(), 2);
        assert!(tree.dump(false).contains("BIN_EXPR 0..5"));
    }

    #[test]
    fn finish_wraps_leftovers() {
        let tokens = [tok(IDENT, 0, 1)];
        let tree = TreeBuilder::new("x".into(), &tokens).finish();
        assert_eq!(tree.to_source(), "x");
        assert_eq!(tree.kind(tree.root()), SOURCE_FILE);
    }
}
