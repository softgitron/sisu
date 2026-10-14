//! `sisu check --explain FILE`: the file's declarations the way `sisu doc`
//! renders them (09, "Documentation"): each signature with its contracts
//! (07: contracts are "documented automatically"), a type's invariants, a
//! refinement's predicate and the conversion that checks it, and the error
//! set the compiler inferred for a `T !` function. Bodies, tests and imports
//! are left out; so are the declarations the compiler writes itself.
//!
//! The text is the declaration as written: a header written over several
//! lines is joined into one, comments dropped. A contract keeps its own
//! lines (an `ensures match result:` block), re-indented under its
//! signature. The output is for people, like `sisu doc`'s; it is compared
//! byte for byte by the conformance suite (`NAME.doc`) so that the
//! rendering stays stable, not because another tool reads it.

use sisu_resolve::{DefKind, Program, Ty};
use sisu_syntax::SyntaxKind;
use sisu_syntax::ast::{self, ItemKind};
use sisu_syntax::tree::Token;
use sisu_typeck::Typed;
use std::collections::HashSet;

/// The rendering of one file's root module.
pub fn explain(
    source: &str,
    tokens: &[Token],
    module: &ast::Module,
    prog: &Program,
    typed: &Typed,
) -> String {
    let mut with_invariant = HashSet::new();
    for item in &module.items {
        if let ItemKind::Struct(s) = &item.kind
            && !s.invariants.is_empty()
        {
            with_invariant.insert(s.name.text.clone());
        }
    }
    let r = Renderer { source, tokens, prog, typed, with_invariant };
    let mut blocks = Vec::new();
    for item in &module.items {
        let mut out = String::new();
        r.item(item, &mut out);
        if !out.is_empty() {
            blocks.push(out);
        }
    }
    blocks.join("\n")
}

struct Renderer<'a> {
    source: &'a str,
    tokens: &'a [Token],
    prog: &'a Program,
    typed: &'a Typed,
    /// The structs of the file that declare an `invariant`.
    with_invariant: HashSet<String>,
}

fn is_code(k: SyntaxKind) -> bool {
    !k.is_trivia()
        && !matches!(k, SyntaxKind::NEWLINE | SyntaxKind::INDENT | SyntaxKind::DEDENT | SyntaxKind::EOF)
}

const STEP: &str = "    ";

impl Renderer<'_> {
    fn text(&self, t: &Token) -> &str {
        self.source.get(t.range.start as usize..t.range.end as usize).unwrap_or("")
    }

    /// The index of the first token at or after `at`.
    fn token_at(&self, at: u32) -> usize {
        self.tokens.partition_point(|t| t.range.start < at)
    }

    /// The code tokens from `start` up to the end of the header line: the
    /// first line break outside brackets, or `end`.
    fn header(&self, start: u32, end: u32, skip: &[ast::Attr]) -> String {
        let mut depth = 0i32;
        let mut picked = Vec::new();
        let mut i = self.token_at(start);
        while let Some(t) = self.tokens.get(i) {
            i += 1;
            if t.range.start >= end {
                break;
            }
            if skip.iter().any(|a| a.range.start <= t.range.start && t.range.end <= a.range.end) {
                continue;
            }
            match t.kind {
                SyntaxKind::L_PAREN | SyntaxKind::L_BRACKET | SyntaxKind::L_BRACE => depth += 1,
                SyntaxKind::R_PAREN | SyntaxKind::R_BRACKET | SyntaxKind::R_BRACE => depth -= 1,
                SyntaxKind::NEWLINE | SyntaxKind::INDENT if depth <= 0 && !picked.is_empty() => break,
                _ => {}
            }
            if is_code(t.kind) {
                picked.push(*t);
            }
        }
        if picked.last().is_some_and(|t| t.kind == SyntaxKind::COLON) {
            picked.pop();
        }
        self.join(&picked)
    }

    /// The code tokens in `start..end`, joined into one line.
    fn compact(&self, start: u32, end: u32) -> String {
        let from = self.token_at(start);
        let picked: Vec<Token> = self.tokens[from..]
            .iter()
            .take_while(|t| t.range.end <= end)
            .filter(|t| is_code(t.kind))
            .copied()
            .collect();
        self.join(&picked)
    }

    /// Tokens as written, a gap between two of them kept as one space; a
    /// gap that was a line break inside brackets goes, and so does the
    /// trailing comma of a list written one element per line.
    fn join(&self, picked: &[Token]) -> String {
        let mut out = String::new();
        for (k, t) in picked.iter().enumerate() {
            if k > 0 {
                let prev = picked[k - 1];
                let gap = self.source.get(prev.range.end as usize..t.range.start as usize).unwrap_or("");
                let broken = gap.contains('\n');
                let closing =
                    matches!(t.kind, SyntaxKind::R_PAREN | SyntaxKind::R_BRACKET | SyntaxKind::R_BRACE);
                if broken && closing && prev.kind == SyntaxKind::COMMA {
                    out.pop();
                    if out.ends_with(' ') {
                        out.pop();
                    }
                }
                let opening =
                    matches!(prev.kind, SyntaxKind::L_PAREN | SyntaxKind::L_BRACKET | SyntaxKind::L_BRACE);
                if !gap.is_empty() && !(broken && (opening || closing)) {
                    out.push(' ');
                }
            }
            out.push_str(self.text(t));
        }
        out
    }

    /// The `##` lines just before `start`.
    fn docs_before(&self, start: u32) -> Vec<String> {
        let mut docs = Vec::new();
        let mut i = self.token_at(start);
        while i > 0 {
            i -= 1;
            let t = &self.tokens[i];
            match t.kind {
                SyntaxKind::DOC_COMMENT => {
                    let s = self.text(t);
                    let s = s.strip_prefix("##").unwrap_or(s);
                    docs.push(s.strip_prefix(' ').unwrap_or(s).to_string());
                }
                SyntaxKind::WHITESPACE
                | SyntaxKind::LINE_BREAK
                | SyntaxKind::NEWLINE
                | SyntaxKind::INDENT
                | SyntaxKind::DEDENT => {}
                _ => break,
            }
        }
        docs.reverse();
        docs
    }

    fn docs(&self, docs: &[String], indent: &str, out: &mut String) {
        for d in docs {
            if d.is_empty() {
                out.push_str(&format!("{indent}##\n"));
            } else {
                out.push_str(&format!("{indent}## {d}\n"));
            }
        }
    }

    fn attrs(&self, attrs: &[ast::Attr], indent: &str, out: &mut String) {
        for a in attrs {
            out.push_str(&format!("{indent}{}\n", self.compact(a.range.start, a.range.end)));
        }
    }

    /// A contract clause as written; the lines after its first keep their
    /// indentation relative to it.
    fn contract(&self, c: &ast::Contract, indent: &str, out: &mut String) {
        let start = c.range.start as usize;
        let text = self.source.get(start..c.range.end as usize).unwrap_or("");
        let line_start = self.source[..start].rfind('\n').map_or(0, |p| p + 1);
        let column = start - line_start;
        for (k, line) in text.lines().enumerate() {
            let line = line.trim_end();
            if k == 0 {
                out.push_str(&format!("{indent}{line}\n"));
            } else if !line.trim().is_empty() {
                let lead = line.len() - line.trim_start().len();
                let rest = &line[lead.min(column)..];
                out.push_str(&format!("{indent}{rest}\n"));
            }
        }
    }

    fn item(&self, item: &ast::Item, out: &mut String) {
        let start = item.range.start;
        let end = item.range.end;
        match &item.kind {
            ItemKind::Fn(f) => {
                self.docs(&item.docs, "", out);
                self.func(f, "", None, out);
            }
            ItemKind::Struct(s) => {
                self.docs(&item.docs, "", out);
                self.attrs(&item.attrs, "", out);
                out.push_str(&self.header(start, end, &item.attrs));
                out.push('\n');
                for field in &s.fields {
                    out.push_str(&format!("{STEP}{}\n", self.compact(field.range.start, field.range.end)));
                }
                for inv in &s.invariants {
                    self.contract(inv, STEP, out);
                }
                for c in &s.consts {
                    self.constant(c, STEP, out);
                }
                let owner = (!s.invariants.is_empty()).then_some(s.name.text.as_str());
                for m in &s.methods {
                    out.push('\n');
                    self.func(m, STEP, owner, out);
                }
            }
            ItemKind::Enum(e) | ItemKind::ErrorSet(e) => {
                self.docs(&item.docs, "", out);
                self.attrs(&item.attrs, "", out);
                out.push_str(&self.header(start, end, &item.attrs));
                out.push('\n');
                for v in &e.variants {
                    out.push_str(&format!("{STEP}{}\n", self.compact(v.range.start, v.range.end)));
                }
                if e.open {
                    out.push_str(&format!("{STEP}..\n"));
                }
                for m in &e.methods {
                    out.push('\n');
                    self.func(m, STEP, None, out);
                }
            }
            ItemKind::Trait(t) => {
                self.docs(&item.docs, "", out);
                self.attrs(&item.attrs, "", out);
                out.push_str(&self.header(start, end, &item.attrs));
                out.push('\n');
                for a in &t.assoc_types {
                    out.push_str(&format!("{STEP}type {}\n", a.name.text));
                }
                for m in &t.methods {
                    self.func(m, STEP, None, out);
                }
            }
            ItemKind::Impl(i) => {
                self.docs(&item.docs, "", out);
                self.attrs(&item.attrs, "", out);
                out.push_str(&self.header(start, end, &item.attrs));
                out.push('\n');
                // An inherent impl of a type with an invariant: its `mut
                // self` methods keep it.
                let owner = match &i.self_ty.kind {
                    ast::TypeKind::Path { segments, .. } if i.trait_ty.is_none() => {
                        segments.last().map(|s| s.text.as_str()).filter(|n| self.with_invariant.contains(*n))
                    }
                    _ => None,
                };
                for a in &i.assoc_types {
                    out.push_str(&format!("{STEP}type {}\n", a.name.text));
                }
                for c in &i.consts {
                    self.constant(c, STEP, out);
                }
                for m in &i.methods {
                    self.func(m, STEP, owner, out);
                }
            }
            ItemKind::Const(c) => {
                self.docs(&item.docs, "", out);
                self.constant(c, "", out);
            }
            ItemKind::TypeAlias(a) => {
                self.docs(&item.docs, "", out);
                self.attrs(&item.attrs, "", out);
                out.push_str(&self.header(start, end, &item.attrs));
                out.push('\n');
                // A refinement's one way in from its base (07).
                if a.refinement
                    && let Some(base) = &a.ty
                {
                    let name = &a.name.text;
                    let args = if a.generics.is_empty() {
                        String::new()
                    } else {
                        let names: Vec<&str> = a.generics.iter().map(|g| g.name.text.as_str()).collect();
                        format!("[{}]", names.join(", "))
                    };
                    let base = self.compact(base.range.start, base.range.end);
                    out.push_str(&format!(
                        "{STEP}def try_from(value: {base}) -> {name}{args} ! RefinementError\n"
                    ));
                }
            }
            ItemKind::Import(_)
            | ItemKind::FromImport(_)
            | ItemKind::Test(_)
            | ItemKind::Suite(_)
            | ItemKind::Bench(_)
            | ItemKind::Extern(_)
            | ItemKind::Stmt(_)
            | ItemKind::Error => {}
        }
    }

    fn constant(&self, c: &ast::ConstDef, indent: &str, out: &mut String) {
        match &c.ty {
            Some(t) => out.push_str(&format!(
                "{indent}const {}: {}\n",
                c.name.text,
                self.compact(t.range.start, t.range.end)
            )),
            None => out.push_str(&format!("{indent}const {}\n", c.name.text)),
        }
    }

    /// A function's signature and contracts; `invariant_of` names the type
    /// whose invariant a `mut self` method re-checks on return.
    fn func(&self, f: &ast::FnDef, indent: &str, invariant_of: Option<&str>, out: &mut String) {
        if indent.is_empty() {
            // A top-level function's `##` lines are on its item.
        } else {
            self.docs(&self.docs_before(f.range.start), indent, out);
        }
        self.attrs(&f.attrs, indent, out);
        let mut header = self.header(f.range.start, f.range.end, &f.attrs);
        // `T !`: the set the compiler inferred from the body (05).
        if header.ends_with('!')
            && let Some(set) = self.inferred(f)
        {
            header.push(' ');
            header.push_str(&set);
            header.push_str("  # inferred");
        }
        out.push_str(&format!("{indent}{header}\n"));
        let inner = format!("{indent}{STEP}");
        for c in f.contracts() {
            self.contract(c, &inner, out);
        }
        if let Some(owner) = invariant_of
            && f.params
                .first()
                .is_some_and(|p| matches!(p.kind, ast::ParamKind::SelfParam) && p.mode == ast::Mode::Mut)
        {
            out.push_str(&format!("{inner}# re-checks the invariant of `{owner}` on return\n"));
        }
    }

    /// The inferred error set of a `T !` function, as a type is written.
    fn inferred(&self, f: &ast::FnDef) -> Option<String> {
        let def = self.prog.defs.iter().position(|d| {
            d.module == self.prog.root && d.span.range == f.name.range && matches!(d.kind, DefKind::Fn(_))
        })?;
        let set = self.typed.inferred.get(&sisu_resolve::DefId(def as u32))?;
        if set.is_empty() {
            return Some("{}".into());
        }
        Some(self.prog.display(&Ty::ErrSet(set.clone())))
    }
}
