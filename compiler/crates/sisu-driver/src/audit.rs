//! `sisu audit --ownership` (18, Phase 3): how often each file reaches for
//! the escape hatches of the ownership model, for the corpus study's
//! measures of `unsafe` use and of escape-hatch density (18, T1.1 and
//! T1.2).
//!
//! The counts are of tokens, so they mean the same to any implementation:
//! the names `Shared`, `Weak`, `Cell`, `Arena` and `Key` where they mean
//! std's types (not after a `.`, and not in a file that declares the name
//! itself: a type, trait, function, constant, alias, variant or imported
//! name), the method calls `.copy()`, and the keyword `unsafe`. `lines`
//! counts the lines of code: those on which a token starts that is not
//! trivia (a comment, a blank) or layout.
//!
//! The output is data, one line per file: the file's name, then
//! `name=count` for `lines` and each column in [`COLUMNS`]' order, separated
//! by single spaces.

use sisu_syntax::SyntaxKind;
use sisu_syntax::ast::{self, ItemKind};
use sisu_syntax::tree::Token;
use std::collections::HashSet;

/// The counted names and constructs, in output order.
pub const COLUMNS: [&str; 7] = ["Shared", "Weak", "Cell", "Arena", "Key", "copy", "unsafe"];

/// The counts for one file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub lines: usize,
    /// In [`COLUMNS`]' order.
    pub counts: [usize; COLUMNS.len()],
}

impl Counts {
    /// The file's line of output, without the line break.
    pub fn line(&self, file: &str) -> String {
        let mut out = format!("{file} lines={}", self.lines);
        for (name, n) in COLUMNS.iter().zip(self.counts) {
            out.push_str(&format!(" {name}={n}"));
        }
        out
    }
}

/// Counts one file from its tokens and its module.
pub fn ownership(source: &str, tokens: &[Token], module: &ast::Module) -> Counts {
    let mut declared = HashSet::new();
    declared_names(&module.items, &mut declared);
    let text = |t: &Token| source.get(t.range.start as usize..t.range.end as usize).unwrap_or("");
    // The tokens the counts look at: no trivia, no layout.
    let code: Vec<&Token> = tokens
        .iter()
        .filter(|t| {
            !t.kind.is_trivia()
                && !matches!(
                    t.kind,
                    SyntaxKind::NEWLINE | SyntaxKind::INDENT | SyntaxKind::DEDENT | SyntaxKind::EOF
                )
        })
        .collect();
    // The offset after each line break: a token at offset `o` is on line
    // `breaks.partition_point(|b| *b <= o)`.
    let breaks: Vec<usize> = source.match_indices('\n').map(|(i, _)| i + 1).collect();
    let mut c = Counts::default();
    let mut lines = HashSet::new();
    for (i, t) in code.iter().enumerate() {
        lines.insert(breaks.partition_point(|b| *b <= t.range.start as usize));
        let after_dot = i > 0 && code[i - 1].kind == SyntaxKind::DOT;
        let column = match t.kind {
            SyntaxKind::UNSAFE_KW => Some(6),
            SyntaxKind::IDENT if after_dot => {
                let call = code.get(i + 1).is_some_and(|n| n.kind == SyntaxKind::L_PAREN);
                (text(t) == "copy" && call).then_some(5)
            }
            SyntaxKind::IDENT if !declared.contains(text(t)) => {
                COLUMNS[..5].iter().position(|n| *n == text(t))
            }
            _ => None,
        };
        if let Some(k) = column {
            c.counts[k] += 1;
        }
    }
    c.lines = lines.len();
    c
}

/// The names a file declares at its top level, which then mean its own
/// declarations rather than std's.
fn declared_names(items: &[ast::Item], out: &mut HashSet<String>) {
    for item in items {
        match &item.kind {
            ItemKind::Fn(f) => {
                out.insert(f.name.text.clone());
            }
            ItemKind::Struct(s) => {
                out.insert(s.name.text.clone());
            }
            ItemKind::Enum(e) | ItemKind::ErrorSet(e) => {
                out.insert(e.name.text.clone());
                out.extend(e.variants.iter().map(|v| v.name.text.clone()));
            }
            ItemKind::Trait(t) => {
                out.insert(t.name.text.clone());
            }
            ItemKind::Const(k) => {
                out.insert(k.name.text.clone());
            }
            ItemKind::TypeAlias(a) => {
                out.insert(a.name.text.clone());
            }
            ItemKind::FromImport(f) => {
                out.extend(f.names.iter().map(|(n, alias)| alias.as_ref().unwrap_or(n).text.clone()));
            }
            ItemKind::Extern(inner) => declared_names(inner, out),
            _ => {}
        }
    }
}
