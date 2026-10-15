//! The tree's tokens regrouped into logical lines, each laid out with the
//! printer; and the comments and blank lines between them.
//!
//! The rules, so another implementation writes the same text:
//!
//! - A logical line is what the lexer ends with `NEWLINE`; it is written at
//!   four spaces per block level, and broken only inside brackets and
//!   before the binary operators of its outermost expressions (a line that
//!   starts with an operator continues the one before, 02), each broken
//!   bracket's contents one level deeper and each broken operator's line
//!   one level deeper (two in a block header, so the condition stands out
//!   from the block).
//! - A bracket's contents go on their own line when the whole does not
//!   fit, and one element per line, each with a comma after it, when they
//!   do not fit on that line either. A trailing comma in the source (the
//!   "magic trailing comma") keeps them one per line; a tuple of one keeps
//!   its comma. A comprehension that does not fit puts each `for` and `if`
//!   clause on a line of its own instead.
//! - Brackets are tried from the left, each kept on its line when the rest
//!   up to the next bracket fits, except a definition's parameters: they
//!   stay on the `def` line only when the whole line fits, so a long
//!   signature breaks there before its return type does (Black's
//!   left-hand split).
//! - A comment at the end of a line does not count toward its width.
//! - Strings are written as they are.
//! - `Option[T]` is written `T?`, unless `T` is itself optional (`T??`
//!   would read as `??`), a result or a function type, or the option is
//!   behind `ref` or optional itself. `Result[T, E]` is kept: whether `E`
//!   is an error set, which `!` needs, is not in the syntax.
//! - A comment keeps its text. One after code stays after it, two spaces
//!   away; one on a line of its own stays on a line of its own, at the
//!   level of the line after it, or of an enclosing block whose
//!   indentation it was written at.
//! - Blank lines: none at the start of a block or of the file; one before
//!   and after every top-level `def`, `struct`, `enum`, `error`, `trait`,
//!   `impl`, `test`, `suite`, `bench` and `extern` (with the comments
//!   right above it); elsewhere at most one, where the source had any.
//! - A run of `import` and `from ... import` lines, with no comment or
//!   blank line between them, is sorted: `import`s first, each kind by
//!   module path; the names a `from` imports are sorted too. An unused
//!   import is removed (`prune.rs` says which are).

use crate::doc::{self, Doc};
use crate::space;
use sisu_syntax::SyntaxKind::{self, *};
use sisu_syntax::{Element, NodeId, Tree};
use std::collections::HashSet;

/// A token of a line.
#[derive(Clone, Debug)]
pub struct Tok {
    pub kind: SyntaxKind,
    pub text: String,
    /// The kind of the node it belongs to.
    pub parent: SyntaxKind,
    /// That node.
    pub node: Option<NodeId>,
    /// Where the token is in the source (for a token the formatter writes,
    /// where the node it stands for is).
    pub pos: u32,
    /// The comments after it: on its line (`false`) or on lines of their
    /// own (`true`), before the next token.
    pub comments: Vec<(bool, String)>,
    /// For an operator of a chain of binary operators: the chain's root.
    pub op_root: Option<NodeId>,
}

impl Default for Tok {
    fn default() -> Tok {
        Tok {
            kind: ERROR_TOKEN,
            text: String::new(),
            parent: ERROR,
            node: None,
            pos: 0,
            comments: Vec::new(),
            op_root: None,
        }
    }
}

/// A comment on a line of its own between logical lines.
#[derive(Clone, Debug)]
struct CommentLine {
    text: String,
    col: usize,
    blank_before: usize,
    depth: usize,
}

#[derive(Clone, Debug)]
struct Line {
    depth: usize,
    toks: Vec<Tok>,
    /// The blank lines just before it (after any comments before it).
    blank_before: usize,
    comments_before: Vec<CommentLine>,
    /// The column of its first token in the source.
    col: usize,
    /// The top-level item it belongs to.
    item: Option<NodeId>,
    item_kind: SyntaxKind,
}

struct Walker<'t> {
    tree: &'t Tree,
    text: &'t str,
    lines: Vec<Line>,
    cur: Vec<Tok>,
    cur_col: usize,
    depth: usize,
    blank: usize,
    fresh: bool,
    pending: Vec<CommentLine>,
    /// The comments after the last line.
    tail: Vec<CommentLine>,
    /// The unused imports, left out.
    pruned: HashSet<NodeId>,
}

fn col_of(text: &str, pos: usize) -> usize {
    let before = &text[..pos.min(text.len())];
    let start = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
    before[start..].chars().count()
}

/// The child nodes of `n` that are types.
fn type_children(tree: &Tree, n: NodeId) -> Vec<NodeId> {
    tree.child_nodes(n).filter(|&c| is_type(tree.kind(c))).collect()
}

fn is_type(k: SyntaxKind) -> bool {
    matches!(
        k,
        PATH_TYPE
            | OPTION_TYPE
            | RESULT_TYPE
            | ERROR_SET_TYPE
            | TUPLE_TYPE
            | PAREN_TYPE
            | FN_TYPE
            | REF_TYPE
            | DYN_TYPE
    )
}

/// `Option[T]`'s `T`, when the formatter writes it `T?`.
pub(crate) fn option_sugar(tree: &Tree, n: NodeId) -> Option<NodeId> {
    if tree.kind(n) != PATH_TYPE {
        return None;
    }
    let idents: Vec<_> = tree.child_tokens(n).collect();
    if idents.len() != 1 || tree.token_text(idents[0]) != "Option" {
        return None;
    }
    let args = tree.first_child(n, GENERIC_ARGS)?;
    let ts = type_children(tree, args);
    if ts.len() != 1 || tree.child_tokens(args).count() != 2 {
        return None;
    }
    let arg = ts[0];
    // Nothing in it but the type: a comment would have nowhere to go.
    if tree.children(n).iter().any(|e| matches!(e, Element::Token(t) if tree.token_kind(*t).is_trivia()))
        || tree
            .children(args)
            .iter()
            .any(|e| matches!(e, Element::Token(t) if tree.token_kind(*t).is_trivia()))
    {
        return None;
    }
    let arg_kind = if option_sugar(tree, arg).is_some() { OPTION_TYPE } else { tree.kind(arg) };
    if !matches!(arg_kind, PATH_TYPE | TUPLE_TYPE | REF_TYPE | DYN_TYPE | PAREN_TYPE) {
        return None;
    }
    let parent = tree.parent(n).map(|p| tree.kind(p));
    if matches!(parent, Some(REF_TYPE | OPTION_TYPE)) {
        return None;
    }
    // `Option[Option[T]]` written `Option[T]?` would be `T??`.
    if parent == Some(GENERIC_ARGS)
        && let Some(outer) = tree.parent(n).and_then(|p| tree.parent(p))
        && option_sugar(tree, outer) == Some(n)
    {
        return None;
    }
    Some(arg)
}

impl<'t> Walker<'t> {
    fn node(&mut self, n: NodeId) {
        let kind = self.tree.kind(n);
        // A string, with its interpolations, is written as it is.
        if matches!(kind, STRING_EXPR) {
            let r = self.tree.range(n);
            self.push(Tok {
                kind: STRING_START,
                text: self.tree.node_text(n).to_string(),
                parent: kind,
                node: Some(n),
                pos: r.start,
                ..Tok::default()
            });
            return;
        }
        if let Some(arg) = option_sugar(self.tree, n) {
            self.node(arg);
            let r = self.tree.range(n);
            self.push(Tok {
                kind: QUESTION,
                text: "?".into(),
                parent: OPTION_TYPE,
                node: Some(n),
                pos: r.end,
                ..Tok::default()
            });
            return;
        }
        let mut children: Vec<Element> = self.tree.children(n).to_vec();
        if kind == FROM_IMPORT {
            children = without_pruned(self.tree, children, &self.pruned);
        }
        for e in children {
            match e {
                Element::Node(c) if self.pruned.contains(&c) => {}
                Element::Node(c) => self.node(c),
                Element::Token(t) => self.token(t, n),
            }
        }
    }

    fn push(&mut self, t: Tok) {
        if self.cur.is_empty() {
            self.cur_col = col_of(self.text, t.pos as usize);
        }
        self.cur.push(t);
        self.fresh = false;
    }

    fn token(&mut self, t: sisu_syntax::TokenId, parent: NodeId) {
        let kind = self.tree.token_kind(t);
        let range = self.tree.token_range(t);
        match kind {
            WHITESPACE | EOF => {}
            LINE_BREAK => {
                if self.cur.is_empty() && self.fresh {
                    self.blank += 1;
                }
                self.fresh = true;
            }
            COMMENT | DOC_COMMENT | MODULE_DOC_COMMENT => {
                let text = self.tree.token_text(t).trim_end().to_string();
                if self.cur.is_empty() {
                    let col = col_of(self.text, range.start as usize);
                    self.pending.push(CommentLine { text, col, blank_before: self.blank, depth: 0 });
                    self.blank = 0;
                } else if let Some(last) = self.cur.last_mut() {
                    last.comments.push((self.fresh, text));
                }
                self.fresh = false;
            }
            NEWLINE => {
                let toks = std::mem::take(&mut self.cur);
                let item = toks.first().and_then(|t| t.node).and_then(|n| top_item(self.tree, n));
                let item_kind = item.map(|i| self.tree.kind(i)).unwrap_or(ERROR);
                self.lines.push(Line {
                    depth: self.depth,
                    toks,
                    blank_before: self.blank,
                    comments_before: std::mem::take(&mut self.pending),
                    col: self.cur_col,
                    item,
                    item_kind,
                });
                self.blank = 0;
                self.fresh = true;
            }
            INDENT => self.depth += 1,
            DEDENT => self.depth = self.depth.saturating_sub(1),
            _ => {
                let pk = self.tree.kind(parent);
                let op_root = matches!(pk, BIN_EXPR | COMPARE_EXPR).then(|| chain_root(self.tree, parent));
                self.push(Tok {
                    kind,
                    text: self.tree.token_text(t).to_string(),
                    parent: pk,
                    node: Some(parent),
                    pos: range.start,
                    comments: Vec::new(),
                    op_root,
                });
            }
        }
    }
}

/// A `from` import's children without its unused names, each with the
/// comma after it (or, for the last, the one before it).
fn without_pruned(tree: &Tree, children: Vec<Element>, pruned: &HashSet<NodeId>) -> Vec<Element> {
    let mut drop = vec![false; children.len()];
    let is_comma = |e: &Element| matches!(e, Element::Token(t) if tree.token_kind(*t) == COMMA);
    let significant = |e: &Element| !matches!(e, Element::Token(t) if tree.token_kind(*t).is_trivia());
    for (i, e) in children.iter().enumerate() {
        let Element::Node(c) = e else { continue };
        if !pruned.contains(c) {
            continue;
        }
        drop[i] = true;
        let after = (i + 1..children.len()).find(|&k| significant(&children[k]));
        if let Some(k) = after
            && is_comma(&children[k])
            && !drop[k]
        {
            drop[k] = true;
            continue;
        }
        let before = (0..i).rev().find(|&k| significant(&children[k]) && !drop[k]);
        if let Some(k) = before
            && is_comma(&children[k])
        {
            drop[k] = true;
        }
    }
    children.into_iter().zip(drop).filter(|(_, d)| !d).map(|(e, _)| e).collect()
}

/// The child of the file that holds `n`.
fn top_item(tree: &Tree, n: NodeId) -> Option<NodeId> {
    let mut cur = n;
    loop {
        let p = tree.parent(cur)?;
        if tree.kind(p) == SOURCE_FILE {
            return Some(cur);
        }
        cur = p;
    }
}

/// The class of a binary operator: operators of one class chain (`a + b -
/// c`), and a chain is broken before each of its operators.
fn op_class(tree: &Tree, n: NodeId) -> u8 {
    let Some(t) = tree.child_tokens(n).find(|&t| !matches!(tree.token_kind(t), L_PAREN | R_PAREN)) else {
        return 0;
    };
    match tree.token_kind(t) {
        OR_KW => 1,
        AND_KW => 2,
        QUESTION_QUESTION => 3,
        PLUS | MINUS | PLUS_PERCENT | MINUS_PERCENT | PLUS_PIPE | MINUS_PIPE => 4,
        STAR | SLASH | SLASH_SLASH | PERCENT | STAR_PERCENT | STAR_PIPE => 5,
        SHL | SHR => 6,
        AMP | PIPE | CARET => 7,
        STAR_STAR => 8,
        _ => 9,
    }
}

/// The root of the chain of binary operators that `n`'s operator is in:
/// the outermost expression above it of the same class.
fn chain_root(tree: &Tree, n: NodeId) -> NodeId {
    let mut cur = n;
    if tree.kind(n) == COMPARE_EXPR {
        return n;
    }
    let class = op_class(tree, n);
    while let Some(p) = tree.parent(cur) {
        if tree.kind(p) == BIN_EXPR && op_class(tree, p) == class {
            cur = p;
        } else {
            break;
        }
    }
    cur
}

fn is_open(k: SyntaxKind) -> bool {
    matches!(k, L_PAREN | L_BRACKET | L_BRACE)
}

fn is_close(k: SyntaxKind) -> bool {
    matches!(k, R_PAREN | R_BRACKET | R_BRACE)
}

/// The brackets whose contents take a trailing comma when they are written
/// one per line.
fn takes_trailing_comma(k: SyntaxKind) -> bool {
    matches!(
        k,
        ARG_LIST
            | PARAM_LIST
            | LIST_EXPR
            | DICT_EXPR
            | SET_EXPR
            | TUPLE_EXPR
            | GENERIC_ARGS
            | GENERIC_PARAMS
            | TUPLE_TYPE
            | LIST_PAT
            | TUPLE_PAT
            | PAT_ARGS
            | FN_TYPE
            | FROM_IMPORT
            | CLOSURE_PARAMS
    )
}

/// The layout of one logical line.
struct Layout<'a> {
    toks: &'a [Tok],
    mate: Vec<usize>,
    space: Vec<bool>,
    tree: &'a Tree,
    /// The extra indentation of a broken chain's lines.
    cont: usize,
}

impl Layout<'_> {
    fn comments(&self, i: usize, out: &mut Vec<Doc>) {
        for (own, c) in &self.toks[i].comments {
            if *own {
                out.push(Doc::Comment(c.clone()));
            } else {
                out.push(Doc::Suffix(format!("  {c}")));
            }
        }
    }

    /// The tokens `lo..hi`, laid out.
    fn build(&self, lo: usize, hi: usize, out: &mut Vec<Doc>) {
        let mut i = lo;
        while i < hi {
            if i > lo && self.space[i] {
                out.push(Doc::Text(" ".into()));
            }
            if let Some((root, end)) = self.chain_at(i, hi) {
                out.push(self.chain(root, i, end));
                i = end;
                continue;
            }
            let t = &self.toks[i];
            if is_open(t.kind) && self.mate[i] > i && self.mate[i] < hi {
                let j = self.mate[i];
                out.push(self.bracket(i, j));
                i = j + 1;
                continue;
            }
            out.push(Doc::Text(t.text.clone()));
            self.comments(i, out);
            i += 1;
        }
    }

    /// The outermost chain of binary operators that starts at token `i`
    /// and ends before `hi`, with its end.
    fn chain_at(&self, i: usize, hi: usize) -> Option<(NodeId, usize)> {
        let mut best: Option<(NodeId, usize)> = None;
        for k in i..hi {
            let Some(root) = self.toks[k].op_root else { continue };
            let r = self.tree.range(root);
            // The chain's tokens are those inside its root's range.
            let first =
                (0..self.toks.len()).find(|&x| self.toks[x].pos >= r.start && self.toks[x].pos < r.end);
            if first != Some(i) {
                continue;
            }
            let end = (i..self.toks.len()).take_while(|&x| self.toks[x].pos < r.end).last().map(|x| x + 1);
            let Some(end) = end else { continue };
            if end > hi || end <= i + 1 {
                continue;
            }
            if best.is_none_or(|(_, e)| end > e) {
                best = Some((root, end));
            }
        }
        best
    }

    fn chain(&self, root: NodeId, lo: usize, hi: usize) -> Doc {
        // The operators of this chain, outside any bracket of it.
        let mut ops = Vec::new();
        let mut k = lo;
        while k < hi {
            let t = &self.toks[k];
            if is_open(t.kind) && self.mate[k] > k {
                k = self.mate[k] + 1;
                continue;
            }
            if t.op_root == Some(root) {
                ops.push(k);
                // `not in` is one operator.
                if t.kind == NOT_KW && self.toks.get(k + 1).is_some_and(|n| n.kind == IN_KW) {
                    k += 1;
                }
            }
            k += 1;
        }
        let Some(&first_op) = ops.first() else {
            let mut out = Vec::new();
            self.build_plain(lo, hi, &mut out);
            return Doc::group(out, false);
        };
        let mut head = Vec::new();
        self.build(lo, first_op, &mut head);
        let mut rest = Vec::new();
        for (n, &op) in ops.iter().enumerate() {
            let next_op = ops.get(n + 1).copied().unwrap_or(hi);
            rest.push(Doc::Line);
            let mut j = op;
            let mut text = self.toks[op].text.clone();
            if self.toks[op].kind == NOT_KW && self.toks.get(op + 1).is_some_and(|t| t.kind == IN_KW) {
                self.comments(op, &mut rest);
                j += 1;
                text.push_str(" in");
            }
            rest.push(Doc::Text(text));
            self.comments(j, &mut rest);
            if j + 1 < next_op {
                if self.space[j + 1] {
                    rest.push(Doc::Text(" ".into()));
                }
                self.build(j + 1, next_op, &mut rest);
            }
        }
        head.push(Doc::Indent(self.cont, rest));
        Doc::group(head, false)
    }

    /// The tokens as they come, without looking for chains (inside a chain
    /// that has none of its own).
    fn build_plain(&self, lo: usize, hi: usize, out: &mut Vec<Doc>) {
        for i in lo..hi {
            if i > lo && self.space[i] {
                out.push(Doc::Text(" ".into()));
            }
            out.push(Doc::Text(self.toks[i].text.clone()));
            self.comments(i, out);
        }
    }

    fn bracket(&self, i: usize, j: usize) -> Doc {
        let open = &self.toks[i];
        let close = &self.toks[j];
        let mut after_open = Vec::new();
        self.comments(i, &mut after_open);
        let mut after_close = Vec::new();
        self.comments(j, &mut after_close);
        if j == i + 1 {
            let mut docs = vec![Doc::Text(format!("{}{}", open.text, close.text))];
            if !after_open.is_empty() {
                docs = vec![Doc::Text(open.text.clone()), Doc::Indent(crate::INDENT, after_open), Doc::Soft];
                docs.push(Doc::Text(close.text.clone()));
            }
            docs.extend(after_close);
            return Doc::group(docs, false);
        }
        if open.parent == COMPREHENSION {
            return self.comprehension(i, j, after_open, after_close);
        }
        // The elements: split at this bracket's own commas.
        let mut elems: Vec<(usize, usize)> = Vec::new();
        let mut commas: Vec<usize> = Vec::new();
        let mut start = i + 1;
        let mut k = i + 1;
        while k < j {
            let t = &self.toks[k];
            if is_open(t.kind) && self.mate[k] > k && self.mate[k] < j {
                k = self.mate[k] + 1;
                continue;
            }
            if t.kind == COMMA && t.node == open.node {
                elems.push((start, k));
                commas.push(k);
                start = k + 1;
            }
            k += 1;
        }
        elems.push((start, j));
        let kind = open.parent;
        // A trailing comma: magic, except the one a tuple of one needs.
        let mut trailing: Option<usize> = None;
        if let Some(&(s, e)) = elems.last()
            && s == e
            && elems.len() > 1
        {
            elems.pop();
            trailing = commas.pop();
        }
        let one_tuple =
            matches!(kind, TUPLE_EXPR | TUPLE_TYPE | TUPLE_PAT) && elems.len() == 1 && trailing.is_some();
        let eligible = takes_trailing_comma(kind)
            && !matches!(kind, TUPLE_EXPR | TUPLE_TYPE | TUPLE_PAT if elems.len() < 2);
        let magic = trailing.is_some() && !one_tuple && takes_trailing_comma(kind);
        let mut inner: Vec<Doc> = Vec::new();
        let last = elems.len().saturating_sub(1);
        for (n, &(s, e)) in elems.iter().enumerate() {
            if n > 0 {
                let c = commas[n - 1];
                inner.push(Doc::Text(",".into()));
                self.comments(c, &mut inner);
                inner.push(Doc::Line);
            }
            if s >= e {
                continue;
            }
            if n == last {
                // The last element's own comments come after its comma.
                let mut docs = Vec::new();
                self.build_without_last_comments(s, e, &mut docs);
                inner.extend(docs);
                if one_tuple {
                    inner.push(Doc::Text(",".into()));
                } else if eligible && (elems.len() >= 2 || magic) {
                    inner.push(Doc::IfBreak(",".into()));
                }
                self.comments(e - 1, &mut inner);
                if let Some(t) = trailing {
                    self.comments(t, &mut inner);
                }
            } else {
                self.build(s, e, &mut inner);
            }
        }
        let mut body = after_open;
        body.push(Doc::Soft);
        body.push(Doc::group(inner, magic));
        let mut docs = vec![Doc::Text(open.text.clone()), Doc::Indent(crate::INDENT, body), Doc::Soft];
        docs.push(Doc::Text(close.text.clone()));
        docs.extend(after_close);
        // A definition's parameters break first (Black's left-hand split).
        Doc::Group(doc::Group { docs, force: false, whole_line: kind == PARAM_LIST })
    }

    /// A comprehension: when it does not fit on a line, each `for` and `if`
    /// clause starts a line of its own, as Black writes them.
    fn comprehension(&self, i: usize, j: usize, after_open: Vec<Doc>, after_close: Vec<Doc>) -> Doc {
        let open = &self.toks[i];
        let mut cuts = vec![i + 1];
        let mut k = i + 1;
        while k < j {
            let t = &self.toks[k];
            if is_open(t.kind) && self.mate[k] > k && self.mate[k] < j {
                k = self.mate[k] + 1;
                continue;
            }
            if matches!(t.kind, FOR_KW | IF_KW)
                && matches!(t.parent, COMP_FOR | COMP_IF)
                && t.node.and_then(|n| self.tree.parent(n)) == open.node
            {
                cuts.push(k);
            }
            k += 1;
        }
        cuts.push(j);
        let mut inner = Vec::new();
        for w in cuts.windows(2) {
            if w[0] > i + 1 {
                inner.push(Doc::Line);
            }
            self.build(w[0], w[1], &mut inner);
        }
        let mut body = after_open;
        body.push(Doc::Soft);
        body.push(Doc::group(inner, false));
        let mut docs = vec![Doc::Text(open.text.clone()), Doc::Indent(crate::INDENT, body), Doc::Soft];
        docs.push(Doc::Text(self.toks[j].text.clone()));
        docs.extend(after_close);
        Doc::group(docs, false)
    }

    /// [`Layout::build`], leaving out the comments of the last token.
    fn build_without_last_comments(&self, lo: usize, hi: usize, out: &mut Vec<Doc>) {
        let mut docs = Vec::new();
        self.build(lo, hi, &mut docs);
        let n = self.toks[hi - 1].comments.len();
        if n > 0 {
            // The last token's comments are the last docs pushed at the
            // innermost level: strip them where they are.
            strip_tail_comments(&mut docs, n);
        }
        out.extend(docs);
    }
}

/// Removes the last `n` comment docs, wherever the last ones are nested.
fn strip_tail_comments(docs: &mut Vec<Doc>, mut n: usize) {
    while n > 0 {
        match docs.last_mut() {
            Some(Doc::Suffix(_) | Doc::Comment(_)) => {
                docs.pop();
                n -= 1;
            }
            Some(Doc::Group(g)) => {
                strip_tail_comments(&mut g.docs, n);
                return;
            }
            Some(Doc::Indent(_, inner)) => {
                strip_tail_comments(inner, n);
                return;
            }
            _ => return,
        }
    }
}

fn line_text(tree: &Tree, line: &Line) -> String {
    let toks = &line.toks;
    let mut mate = vec![usize::MAX; toks.len()];
    let mut stack = Vec::new();
    for (i, t) in toks.iter().enumerate() {
        if is_open(t.kind) {
            stack.push(i);
        } else if is_close(t.kind)
            && let Some(o) = stack.pop()
        {
            mate[o] = i;
            mate[i] = o;
        }
    }
    let space: Vec<bool> = (0..toks.len()).map(|i| i > 0 && space::between(&toks[i - 1], &toks[i])).collect();
    let header = toks.last().is_some_and(|t| t.kind == COLON);
    let layout =
        Layout { toks, mate, space, tree, cont: if header { 2 * crate::INDENT } else { crate::INDENT } };
    let mut docs = Vec::new();
    // A line with a string over several lines is written as it is.
    if toks.iter().any(|t| t.text.contains('\n')) {
        layout.build_plain(0, toks.len(), &mut docs);
    } else if !toks.is_empty() {
        // The comment that ends the line is outside every group: it needs
        // no line break, and so forces none.
        layout.build_without_last_comments(0, toks.len(), &mut docs);
        layout.comments(toks.len() - 1, &mut docs);
    }
    doc::print(&docs, line.depth * crate::INDENT, crate::WIDTH)
}

/// The kinds of top-level item that stand apart, with a blank line before
/// and after.
fn is_block_item(k: SyntaxKind) -> bool {
    matches!(
        k,
        FN_DEF
            | STRUCT_DEF
            | ENUM_DEF
            | ERROR_DEF
            | IMPL_DEF
            | TRAIT_DEF
            | TEST_DEF
            | SUITE_DEF
            | BENCH_DEF
            | EXTERN_BLOCK
    )
}

fn is_import_line(l: &Line) -> bool {
    l.depth == 0
        && matches!(l.item_kind, IMPORT_ITEM | FROM_IMPORT)
        && l.toks.first().is_some_and(|t| matches!(t.kind, IMPORT_KW | FROM_KW))
}

/// The key a line sorts by among imports.
fn import_key(l: &Line) -> (u8, String, String) {
    let kind = if l.toks.first().is_some_and(|t| t.kind == IMPORT_KW) { 0 } else { 1 };
    let path: String = l
        .toks
        .iter()
        .skip(1)
        .take_while(|t| matches!(t.kind, IDENT | DOT))
        .map(|t| t.text.as_str())
        .collect();
    let all: Vec<&str> = l.toks.iter().map(|t| t.text.as_str()).collect();
    (kind, path, all.join(" "))
}

/// Sorts the names a `from ... import` line imports.
fn sort_names(l: &mut Line) {
    if l.toks.iter().any(|t| !t.comments.is_empty()) || l.toks.first().is_none_or(|t| t.kind != FROM_KW) {
        return;
    }
    let Some(at) = l.toks.iter().position(|t| t.kind == IMPORT_KW) else { return };
    let mut body: Vec<Tok> = l.toks[at + 1..].to_vec();
    let parens = body.first().is_some_and(|t| t.kind == L_PAREN);
    if parens {
        body.remove(0);
        if body.last().is_some_and(|t| t.kind == R_PAREN) {
            body.pop();
        }
    }
    let comma = body.iter().find(|t| t.kind == COMMA).cloned();
    let mut names: Vec<Vec<Tok>> =
        body.split(|t| t.kind == COMMA).map(|g| g.to_vec()).filter(|g| !g.is_empty()).collect();
    if names.len() < 2 {
        return;
    }
    let had_trailing = body.last().is_some_and(|t| t.kind == COMMA);
    names.sort_by(|a, b| a.first().map(|t| t.text.as_str()).cmp(&b.first().map(|t| t.text.as_str())));
    let mut rebuilt: Vec<Tok> = l.toks[..=at].to_vec();
    if parens {
        rebuilt.push(l.toks[at + 1].clone());
    }
    let comma = comma.unwrap_or_default();
    for (i, n) in names.into_iter().enumerate() {
        if i > 0 {
            rebuilt.push(comma.clone());
        }
        rebuilt.extend(n);
    }
    if had_trailing {
        rebuilt.push(comma);
    }
    if parens && let Some(close) = l.toks.last() {
        rebuilt.push(close.clone());
    }
    l.toks = rebuilt;
}

fn sort_imports(lines: &mut [Line]) {
    let mut i = 0;
    while i < lines.len() {
        if !is_import_line(&lines[i]) {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while j < lines.len()
            && is_import_line(&lines[j])
            && lines[j].comments_before.is_empty()
            && lines[j].blank_before == 0
        {
            j += 1;
        }
        for l in lines[i..j].iter_mut() {
            sort_names(l);
        }
        // The run's leading comments and blank lines stay at its head.
        let head_comments = std::mem::take(&mut lines[i].comments_before);
        let head_blank = lines[i].blank_before;
        lines[i..j].sort_by_key(import_key);
        lines[i].comments_before = head_comments;
        lines[i].blank_before = head_blank;
        for l in lines[i + 1..j].iter_mut() {
            l.blank_before = 0;
        }
        i = j;
    }
}

/// The formatted text of a parsed file.
pub fn format_tree(tree: &Tree) -> String {
    let mut w = Walker {
        tree,
        text: tree.text(),
        lines: Vec::new(),
        cur: Vec::new(),
        cur_col: 0,
        depth: 0,
        blank: 0,
        fresh: true,
        pending: Vec::new(),
        tail: Vec::new(),
        pruned: crate::prune::unused(tree),
    };
    w.node(tree.root());
    w.tail = std::mem::take(&mut w.pending);
    let tail_blank = w.blank;
    let mut lines = w.lines;
    let mut tail = w.tail;
    // Each comment between lines takes the level of the line after it, or
    // of a block it was written inside (by its column).
    let mut indent_of: Vec<usize> = Vec::new();
    let mut prev_depth = 0usize;
    for l in lines.iter_mut() {
        place_comments(&mut l.comments_before, prev_depth, l.depth, &indent_of);
        if indent_of.len() <= l.depth {
            indent_of.resize(l.depth + 1, 0);
        }
        indent_of[l.depth] = l.col;
        indent_of.truncate(l.depth + 1);
        prev_depth = l.depth;
    }
    place_comments(&mut tail, prev_depth, 0, &indent_of);
    sort_imports(&mut lines);
    let mut out = Output { text: String::new(), first: true };
    let mut prev_depth = 0usize;
    let mut prev_block_item = false;
    let mut prev_item: Option<NodeId> = None;
    let mut prev_attr = false;
    for l in &lines {
        let starts_item = l.depth == 0 && l.item != prev_item;
        let block = is_block_item(l.item_kind);
        // The blank lines the line needs before it (and before the comments
        // right above it), when the rules fix them.
        let needed: Option<usize> = if l.depth > prev_depth {
            Some(0)
        } else if starts_item && (block || prev_block_item) {
            Some(1)
        } else if l.depth == 0 && !starts_item && prev_attr {
            Some(0)
        } else {
            None
        };
        // Comments deeper than the line close the block before it; the
        // others are above the line.
        let split = l.comments_before.iter().rposition(|c| c.depth > l.depth).map(|k| k + 1).unwrap_or(0);
        let (inner, above) = l.comments_before.split_at(split);
        for c in inner {
            out.comment(c, c.blank_before.min(1));
        }
        let attached = l.blank_before == 0;
        for (n, c) in above.iter().enumerate() {
            let blank = match (n, needed) {
                (0, Some(k)) if attached || k == 1 => k,
                _ => c.blank_before.min(1),
            };
            out.comment(c, blank);
        }
        let blank = if !above.is_empty() && attached { 0 } else { needed.unwrap_or(l.blank_before.min(1)) };
        out.line(&line_text(tree, l), blank);
        if starts_item {
            prev_block_item = block;
        }
        if l.depth == 0 {
            prev_item = l.item;
            prev_attr = l.toks.first().is_some_and(|t| t.kind == AT);
        }
        prev_depth = l.depth;
    }
    for (n, c) in tail.iter().enumerate() {
        let blank = if n == 0 && prev_block_item && c.depth == 0 { 1 } else { c.blank_before.min(1) };
        out.comment(c, blank);
    }
    let _ = tail_blank;
    out.text
}

/// The formatted text so far.
struct Output {
    text: String,
    first: bool,
}

impl Output {
    fn blank(&mut self, n: usize) {
        if !self.first {
            for _ in 0..n {
                self.text.push('\n');
            }
        }
        self.first = false;
    }

    fn comment(&mut self, c: &CommentLine, blank: usize) {
        self.blank(blank);
        self.text.push_str(&" ".repeat(c.depth * crate::INDENT));
        self.text.push_str(&c.text);
        self.text.push('\n');
    }

    fn line(&mut self, text: &str, blank: usize) {
        self.blank(blank);
        self.text.push_str(text);
        self.text.push('\n');
    }
}

/// The levels of the comments before a line at `next` after one at
/// `prev`: the next line's, or the deepest enclosing block's whose
/// indentation the comment is written at or beyond.
fn place_comments(comments: &mut [CommentLine], prev: usize, next: usize, indent_of: &[usize]) {
    for c in comments.iter_mut() {
        c.depth = next;
        if prev > next {
            for d in (next + 1..=prev).rev() {
                if indent_of.get(d).is_some_and(|&i| c.col >= i) {
                    c.depth = d;
                    break;
                }
            }
        }
    }
}
