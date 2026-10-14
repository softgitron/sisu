//! The query layer: `source -> tokens -> syntax tree -> resolved module ->
//! typed bodies -> SIR -> checked SIR`, each step memoised on the text it
//! was computed from (18, Phase 0, "Query skeleton"). It is an
//! implementation detail: no result depends on whether a query was cached.
//! The last step is the ownership checker, which places the drops; its
//! SIR is the one that runs, even when it reports errors, so that the
//! counterexample mode can run a rejected program.
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

    /// Whether an error stops lowering: every error but the ownership
    /// rules about declarations (`E04xx` from name resolution, such as a
    /// borrow in a plain struct), which leave the program well-formed, so
    /// the counterexample mode can still run it.
    pub fn blocks_lowering(&self) -> bool {
        self.diagnostics.iter().any(|d| d.is_error() && !is_ownership_rule(d))
    }
}

/// A diagnostic of the ownership family (`E04xx`).
pub fn is_ownership_rule(d: &Diagnostic) -> bool {
    d.code().starts_with("E04")
}

/// How many times each query ran (not counting cache hits).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counters {
    pub parse: u32,
    pub check: u32,
    pub lower: u32,
}

/// A program without front-end errors, lowered and through the
/// ownership checker.
pub struct Lowered {
    /// The SIR with its drops placed: what runs.
    pub sir: sir::Program,
    /// The ownership checker's findings, by SIR position.
    pub findings: Vec<sisu_borrowck::Finding>,
    /// Each finding as a diagnostic, in the findings' order.
    pub diagnostics_by_finding: Vec<Diagnostic>,
    /// The diagnostics in source order.
    pub diagnostics: Vec<Diagnostic>,
    /// The errors of compile-time evaluation (also in `diagnostics`), which
    /// runs once the ownership checker accepts the program.
    pub comptime: Vec<Diagnostic>,
}

impl Lowered {
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.is_error())
    }
}

#[derive(Default)]
pub struct Db {
    texts: HashMap<String, Rc<str>>,
    parsed: HashMap<String, (u64, Rc<Parsed>)>,
    checked: HashMap<String, (u64, Rc<Checked>)>,
    lowered: HashMap<String, (u64, Rc<Lowered>)>,
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

    /// The lowered and ownership-checked program, for a program whose
    /// front end found no error, or only ownership errors about
    /// declarations.
    pub fn lowered(&mut self, name: &str) -> Option<Rc<Lowered>> {
        let (_, h) = self.key(name)?;
        if let Some((k, s)) = self.lowered.get(name)
            && *k == h
        {
            return Some(s.clone());
        }
        let c = self.checked(name)?;
        if c.blocks_lowering() {
            return None;
        }
        self.counters.lower += 1;
        let lowered = sisu_lower::lower(&c.program, &c.typed, &c.map);
        let mut checked = sisu_borrowck::check(&lowered, &sisu_borrowck::Options::default());
        let files = source_files(&c.map, &checked.program);
        let by_finding: Vec<Diagnostic> =
            checked.findings.iter().map(|f| sisu_borrowck::to_diagnostic(f, &c.map, &files)).collect();
        // The compiler runs the `comptime` expressions of a program the
        // ownership checker accepts (08).
        let comptime = if checked.findings.is_empty() {
            let errors = sisu_interp::comptime::evaluate(&mut checked.program);
            errors.iter().map(|e| comptime_diagnostic(e, &c.map, &files, &checked.program)).collect()
        } else {
            Vec::new()
        };
        let mut diagnostics = by_finding.clone();
        diagnostics.extend(comptime.iter().cloned());
        sisu_diag::diagnostic::sort(&mut diagnostics);
        let l = Rc::new(Lowered {
            sir: checked.program,
            findings: checked.findings,
            diagnostics_by_finding: by_finding,
            diagnostics,
            comptime,
        });
        self.lowered.insert(name.to_string(), (h, l.clone()));
        Some(l)
    }

    /// The SIR of a program without front-end errors, drops placed.
    pub fn sir(&mut self, name: &str) -> Option<Rc<sir::Program>> {
        self.lowered(name).map(|l| Rc::new(l.sir.clone()))
    }

    /// Every diagnostic of a file: the front end's, then, when it found no
    /// error, the ownership checker's, in source order.
    pub fn diagnostics(&mut self, name: &str) -> Vec<Diagnostic> {
        let Some(c) = self.checked(name) else { return Vec::new() };
        let mut all = c.diagnostics.clone();
        if !c.has_errors()
            && let Some(l) = self.lowered(name)
        {
            all.extend(l.diagnostics.iter().cloned());
            sisu_diag::diagnostic::sort(&mut all);
        }
        all
    }
}

/// A failed `comptime` expression as a diagnostic at the expression.
fn comptime_diagnostic(
    e: &sisu_interp::comptime::Error,
    map: &SourceMap,
    files: &[FileId],
    p: &sir::Program,
) -> Diagnostic {
    use sisu_diag::msg;
    use sisu_interp::comptime::Failure;
    let f = p.function(&e.function);
    let file = f.and_then(|f| files.get(f.source as usize)).copied().unwrap_or_default();
    let span = sisu_borrowck::span_of(map, file, f.and_then(|f| f.loc));
    // The item that holds the expression: `main` for `main#comptime0`.
    let item = e.function.split("#comptime").next().unwrap_or(&e.function).to_string();
    match &e.failure {
        Failure::Panicked(m) => Diagnostic::new(msg::COMPTIME_PANICKED, span).with("message", m.clone()),
        Failure::Budget(n) => {
            Diagnostic::new(msg::COMPTIME_BUDGET, span).with("item", item).with("steps", n.to_string())
        }
        Failure::World(what) => Diagnostic::new(msg::COMPTIME_IO, span).with("what", *what),
        Failure::NotKept(what) => Diagnostic::new(msg::COMPTIME_NOT_KEPT, span).with("what", what.clone()),
        Failure::Internal(m) => Diagnostic::new(msg::STAGE0_INTERNAL, span)
            .with("stage", "compile-time evaluation")
            .with("detail", m.clone()),
    }
}

/// The file of each SIR source, by name.
pub fn source_files(map: &SourceMap, p: &sir::Program) -> Vec<FileId> {
    p.sources
        .iter()
        .map(|n| map.files().iter().find(|f| f.name == *n).map(|f| f.id).unwrap_or_default())
        .collect()
}
