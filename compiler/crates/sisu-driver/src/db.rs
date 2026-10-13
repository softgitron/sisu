//! The query layer: `source -> tokens -> syntax tree -> resolved module ->
//! typed bodies -> SIR`, each step memoised on the text it was computed
//! from (18, Phase 0, "Query skeleton"). It is an implementation detail:
//! no result depends on whether a query was cached.
//!
//! Stage 0 has one user file per program, so a query's key is the file's
//! name and its input is the file's text; the standard library is fixed
//! for the life of the process. A later stage keys per item and reuses
//! more (the language server will need that), without changing callers.

use sisu_diag::{Diagnostic, FileId, SourceMap};
use sisu_parser::Parse;
use sisu_resolve::Program;
use sisu_sir as sir;
use sisu_syntax::ast;
use sisu_typeck::Typed;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::rc::Rc;

/// A parsed file.
pub struct Parsed {
    pub map: SourceMap,
    pub file: FileId,
    pub parse: Parse,
    pub module: ast::Module,
}

/// A resolved and type-checked program, with every diagnostic so far.
pub struct Checked {
    pub map: SourceMap,
    pub file: FileId,
    pub program: Program,
    pub typed: Typed,
    /// Syntax, name and type diagnostics, in source order.
    pub diagnostics: Vec<Diagnostic>,
}

impl Checked {
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.is_error())
    }
}

/// How many times each query ran (not counting cache hits).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counters {
    pub parse: u32,
    pub check: u32,
    pub lower: u32,
}

#[derive(Default)]
pub struct Db {
    texts: HashMap<String, Rc<str>>,
    parsed: HashMap<String, (u64, Rc<Parsed>)>,
    checked: HashMap<String, (u64, Rc<Checked>)>,
    lowered: HashMap<String, (u64, Rc<sir::Program>)>,
    pub counters: Counters,
}

fn hash(text: &str) -> u64 {
    let mut h = DefaultHasher::new();
    text.hash(&mut h);
    h.finish()
}

impl Db {
    pub fn new() -> Db {
        Db::default()
    }

    /// Sets a file's text; queries over it recompute on next use if the
    /// text changed.
    pub fn set_file(&mut self, name: &str, text: &str) {
        self.texts.insert(name.to_string(), Rc::from(text));
    }

    pub fn text(&self, name: &str) -> Option<Rc<str>> {
        self.texts.get(name).cloned()
    }

    fn key(&self, name: &str) -> Option<(Rc<str>, u64)> {
        let t = self.texts.get(name)?.clone();
        let h = hash(&t);
        Some((t, h))
    }

    pub fn parsed(&mut self, name: &str) -> Option<Rc<Parsed>> {
        let (text, h) = self.key(name)?;
        if let Some((k, p)) = self.parsed.get(name)
            && *k == h
        {
            return Some(p.clone());
        }
        self.counters.parse += 1;
        let mut map = SourceMap::new();
        let file = map.add(name, &*text);
        let parse = sisu_parser::parse(file, &text);
        let module = ast::build::module(&parse.tree);
        let p = Rc::new(Parsed { map, file, parse, module });
        self.parsed.insert(name.to_string(), (h, p.clone()));
        Some(p)
    }

    pub fn checked(&mut self, name: &str) -> Option<Rc<Checked>> {
        let (_, h) = self.key(name)?;
        if let Some((k, c)) = self.checked.get(name)
            && *k == h
        {
            return Some(c.clone());
        }
        let parsed = self.parsed(name)?;
        self.counters.check += 1;
        let mut map = parsed.map.clone();
        let r = sisu_resolve::resolve(&mut map, parsed.file, &parsed.module);
        let (typed, type_diags) = sisu_typeck::check(&r.program, &map);
        let mut diagnostics = parsed.parse.diagnostics.clone();
        // A line with a syntax error has no reliable meaning, so name and
        // type diagnostics on it would only repeat the syntax error.
        let broken_lines: Vec<u32> = diagnostics.iter().map(|d| map.line_col(d.span).0.line).collect();
        let on_broken_line = |d: &Diagnostic| {
            d.span.file == parsed.file && broken_lines.contains(&map.line_col(d.span).0.line)
        };
        diagnostics.extend(r.diagnostics.into_iter().filter(|d| !on_broken_line(d)));
        diagnostics.extend(type_diags.into_iter().filter(|d| !on_broken_line(d)));
        sisu_diag::diagnostic::sort(&mut diagnostics);
        let c = Rc::new(Checked { map, file: parsed.file, program: r.program, typed, diagnostics });
        self.checked.insert(name.to_string(), (h, c.clone()));
        Some(c)
    }

    /// The SIR of a program without errors.
    pub fn sir(&mut self, name: &str) -> Option<Rc<sir::Program>> {
        let (_, h) = self.key(name)?;
        if let Some((k, s)) = self.lowered.get(name)
            && *k == h
        {
            return Some(s.clone());
        }
        let c = self.checked(name)?;
        if c.has_errors() {
            return None;
        }
        self.counters.lower += 1;
        let s = Rc::new(sisu_lower::lower(&c.program, &c.typed, &c.map));
        self.lowered.insert(name.to_string(), (h, s.clone()));
        Some(s)
    }
}
