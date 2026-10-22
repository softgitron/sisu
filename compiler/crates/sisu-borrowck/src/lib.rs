//! The ownership checker (proposal/18-next-steps.md, Phase 3; pipeline
//! step 6, "Checkers on SIR"). It runs on SIR after lowering, function by
//! function, and does five things:
//!
//! 1. **Initialisation and moves** ([`init`]): a forward dataflow over move
//!    paths (a local and its fields and variant payloads) decides, before
//!    every statement, which paths may be initialised and which may be
//!    moved out. A use of a path that may be moved is a use-after-move (or
//!    of a partly moved value); one that was never assigned is a use before
//!    assignment; a second assignment to a `let` declared without a value
//!    is reported too.
//! 2. **Drop placement** ([`elaborate`]): lowering emits every `drop`
//!    unconditionally; the dataflow removes the drops of values that are
//!    moved on every path, guards those of values moved on some path with
//!    a drop flag, and splits the drop of a partly moved value into drops
//!    of the parts that remain, in reverse field order.
//! 3. **Loans** ([`loans`]): every `ref`, `ref mut`, `strview` and copy of a
//!    `ref mut` creates a loan; a forward dataflow tracks which locals may
//!    hold each loan, a backward one which locals are live, and a loan is
//!    live where a local holding it is live (non-lexical). An access that
//!    conflicts with a live loan (a write against any loan, a read against
//!    a `mut` loan) is an error naming both places; so is the end of a
//!    local's storage while a loan of it is live, a return of a loan the
//!    signature does not allow, and storing a loan where it may not go.
//! 4. **Must-consume** ([`consume`]): a drop of a `@must_consume` value on
//!    a normal path is an error (D-054, D-064).
//! 5. **Purity** ([`purity`]): a `pure` function calls only pure functions
//!    (D-074).
//!
//! The checker is sound before it is complete: where it cannot tell, it
//! rejects, and docs/architecture.md lists each known over-rejection.
//! The dynamic ownership monitor in `sisu-interp` re-checks every run.

pub mod bitset;
mod cfg;
mod consume;
mod elaborate;
pub mod explain;
mod init;
mod loans;
mod names;
mod paths;
mod purity;
pub mod shared;
mod types;

use sisu_diag::{Diagnostic, Edit, FileId, LineCol, MsgId, SourceMap, Span, TextRange, msg};
use sisu_sir as sir;

/// A message's parameters, by name.
pub type Params = Vec<(&'static str, Arg)>;

/// A fix: its message, the message's parameters, and the edit that makes
/// it when there is one.
pub type FindingFix = (MsgId, Params, Option<(sir::Loc, String)>);

/// A parameter of a finding's message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Arg {
    Text(String),
    /// The source text at a location (a temporary's expression).
    Source(sir::Loc),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FindingLabel {
    pub loc: Option<sir::Loc>,
    pub msg: MsgId,
    pub params: Vec<(&'static str, Arg)>,
}

/// One diagnostic of the checker, located by SIR positions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub msg: MsgId,
    pub params: Vec<(&'static str, Arg)>,
    /// The SIR function and its source index.
    pub function: String,
    pub source: u32,
    pub loc: Option<sir::Loc>,
    pub labels: Vec<FindingLabel>,
    pub notes: Vec<(MsgId, Params)>,
    pub fixes: Vec<FindingFix>,
    /// The block and statement the finding is about, for the
    /// counterexample mode's account of what a run reached.
    pub site: Option<(u32, u32)>,
    /// The loan a conflict is about, so that one mistake is reported once.
    pub loan: Option<u32>,
}

impl Finding {
    /// Whether the finding is an ownership rule, which the dynamic monitor
    /// observes, rather than purity or a second assignment to a `let`,
    /// which it does not.
    pub fn is_ownership(&self) -> bool {
        ![msg::TYPE_IMPURE_CALL, msg::TYPE_IMPURE_VALUE_CALL, msg::TYPE_IMPURE_DROP, msg::TYPE_REASSIGN_LET]
            .contains(&self.msg)
    }

    pub fn new(msg: MsgId, f: &sir::Function, loc: Option<sir::Loc>) -> Finding {
        Finding {
            msg,
            params: Vec::new(),
            function: f.name.clone(),
            source: f.source,
            loc,
            labels: Vec::new(),
            notes: Vec::new(),
            fixes: Vec::new(),
            site: None,
            loan: None,
        }
    }

    pub fn with(mut self, name: &'static str, value: impl Into<String>) -> Finding {
        self.params.push((name, Arg::Text(value.into())));
        self
    }

    pub fn with_arg(mut self, name: &'static str, value: Arg) -> Finding {
        self.params.push((name, value));
        self
    }

    pub fn label(mut self, loc: Option<sir::Loc>, msg: MsgId, params: Vec<(&'static str, Arg)>) -> Finding {
        self.labels.push(FindingLabel { loc, msg, params });
        self
    }

    pub fn note(mut self, msg: MsgId, params: Vec<(&'static str, Arg)>) -> Finding {
        self.notes.push((msg, params));
        self
    }

    /// A fix, with the edit that makes it when there is one.
    pub fn fix(
        mut self,
        msg: MsgId,
        params: Vec<(&'static str, Arg)>,
        edit: Option<(sir::Loc, String)>,
    ) -> Finding {
        self.fixes.push((msg, params, edit));
        self
    }

    pub fn at(mut self, block: usize, stmt: usize) -> Finding {
        self.site = Some((block as u32, stmt as u32));
        self
    }
}

/// Rules that a fault-injection test can switch off, to show that the
/// cross-check with the monitor catches an unsound checker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    Moves,
    Conflicts,
    Escapes,
    MustConsume,
    /// The `Shared` escape analysis: switched off, every construction site
    /// is tagged task-local.
    SharedEscape,
}

#[derive(Clone, Debug, Default)]
pub struct Options {
    pub disabled: Vec<Rule>,
    /// The standard library's functions are taken as checked: their drops
    /// are elaborated, but their loans are not analysed again for every
    /// program (`sisu test --std`, and the test that checks every std
    /// function, check them).
    pub trust_std: bool,
}

impl Options {
    fn on(&self, r: Rule) -> bool {
        !self.disabled.contains(&r)
    }
}

/// The checked program: SIR with its drops elaborated and its `Shared`
/// construction sites tagged, and the findings.
#[derive(Clone, Debug)]
pub struct Checked {
    pub program: sir::Program,
    pub findings: Vec<Finding>,
    /// How many `Shared` sites were tagged task-local and may-cross.
    pub shared: shared::SharedStats,
}

impl Checked {
    pub fn has_errors(&self) -> bool {
        !self.findings.is_empty()
    }
}

/// Checks a lowered program and places its drops. The result's program
/// is what runs; it is complete and valid even when there are findings,
/// so the counterexample mode can run a rejected program.
pub fn check(p: &sir::Program, opts: &Options) -> Checked {
    check_with(p, opts, None)
}

/// The per-function memo of the checker, kept by the query layer between
/// checks of one program (docs/architecture.md, "The query layer"): a
/// function the checker found nothing in, whose analysis read no other
/// function's body, is taken as checked again while its SIR and the
/// program's interface (`ctx`) are the same, with the drops it placed.
#[derive(Default)]
pub struct FnCache {
    entries: std::collections::HashMap<String, (u64, sir::Function, sir::Function)>,
    /// The functions the last check analysed, and the ones it reused.
    pub checked: u32,
    pub reused: u32,
}

impl FnCache {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// [`check`], through the memo when there is one: `ctx` is a hash of
/// everything the analysis of one function reads besides its own SIR (the
/// interface of the program and the contract level). The result is the
/// same as [`check`]'s.
///
/// Each function is analysed on its own (decision 219): the analyses run
/// on threads, and their findings and checked functions are taken in the
/// program's order, so the result is the same as on one thread.
pub fn check_with(p: &sir::Program, opts: &Options, mut memo: Option<(&mut FnCache, u64)>) -> Checked {
    let mut findings = Vec::new();
    // The checked function the memo gave for each function it answered.
    let mut reused: Vec<Option<sir::Function>> = Vec::with_capacity(p.functions.len());
    let mut kept: std::collections::HashMap<String, (u64, sir::Function, sir::Function)> =
        std::collections::HashMap::new();
    if let Some((cache, _)) = memo.as_mut() {
        cache.checked = 0;
        cache.reused = 0;
    }
    for f in &p.functions {
        if let Some((cache, ctx)) = memo.as_mut()
            && let Some((c, input, output)) = cache.entries.remove(&f.name)
            && c == *ctx
            && input == *f
        {
            cache.reused += 1;
            reused.push(Some(output.clone()));
            kept.insert(f.name.clone(), (c, input, output));
            continue;
        }
        reused.push(None);
    }
    let answered: Vec<bool> = reused.iter().map(Option::is_some).collect();
    // Moves and drop placement.
    let placed = each_function(p, &p.functions, |i, f, types| {
        if answered[i] {
            return None;
        }
        let mut f = f.clone();
        let init = init::Analysis::run(&f, types);
        let found = if opts.on(Rule::Moves) { init.findings(&f, types) } else { Vec::new() };
        elaborate::elaborate(&mut f, &init, types);
        Some((found, f))
    });
    let mut functions = Vec::with_capacity(p.functions.len());
    for (placed, given) in placed.into_iter().zip(&mut reused) {
        match (placed, given.take()) {
            (Some((found, f)), _) => {
                findings.extend(found);
                functions.push(f);
            }
            (None, Some(f)) => functions.push(f),
            (None, None) => {}
        }
    }
    let mut program = sir::Program {
        sources: p.sources.clone(),
        entry: p.entry.clone(),
        types: p.types.clone(),
        intrinsics: p.intrinsics.clone(),
        functions,
        tests: p.tests.clone(),
        trait_methods: p.trait_methods.clone(),
        impls: p.impls.clone(),
    };
    let memoizing = memo.is_some();
    // Loans, escapes and must-consume, on the program with its drops.
    let checked = each_function(p, &program.functions, |i, f, types| {
        if answered[i] {
            return None;
        }
        let trusted =
            opts.trust_std && program.sources.get(f.source as usize).is_some_and(|s| s.starts_with("<std>/"));
        let mut found = Vec::new();
        let mut alone = true;
        if !trusted {
            let a = loans::Analysis::run_with(&program, f, types, opts.on(Rule::Escapes));
            found.extend(a.findings(&program, f, types, opts));
            if opts.on(Rule::MustConsume) {
                let (fs, own) = consume::check(f, types);
                found.extend(fs);
                alone = own;
            }
        }
        // What the memo keeps of a function it may take, copied here
        // rather than one by one after.
        let copies = (memoizing && found.is_empty() && alone).then(|| (p.functions[i].clone(), f.clone()));
        Some((found, alone, copies))
    });
    for (i, r) in checked.into_iter().enumerate() {
        let Some((found, alone, copies)) = r else { continue };
        let f = &program.functions[i];
        if let Some((cache, ctx)) = memo.as_mut() {
            cache.checked += 1;
            let clean = found.is_empty() && !findings.iter().any(|x: &Finding| x.function == f.name);
            if clean
                && alone
                && let Some((input, output)) = copies
            {
                kept.insert(f.name.clone(), (*ctx, input, output));
            }
        }
        findings.extend(found);
    }
    if let Some((cache, _)) = memo.as_mut() {
        cache.entries = kept;
    }
    findings.extend(purity::check(&program));
    dedup(&mut findings);
    let shared = if opts.on(Rule::SharedEscape) {
        shared::tag_sites(&mut program)
    } else {
        shared::SharedStats::default()
    };
    Checked { program, findings, shared }
}

/// `work` for each of `fs`, by index, on as many threads as the machine
/// runs at once (at most 8; this one is one of them, and works alone if no
/// other can be made), each with its own view of the program's types; the
/// results in the order of `fs`.
fn each_function<T: Send>(
    p: &sir::Program,
    fs: &[sir::Function],
    work: impl Fn(usize, &sir::Function, &types::Types) -> Option<T> + Sync,
) -> Vec<Option<T>> {
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get()).min(8).min(fs.len());
    if threads <= 1 {
        let types = types::Types::new(p);
        return fs.iter().enumerate().map(|(i, f)| work(i, f, &types)).collect();
    }
    // The largest first, so that no thread is left with a long one at the
    // end.
    let mut order: Vec<usize> = (0..fs.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(fs[i].blocks.iter().map(|b| b.stmts.len() + 1).sum::<usize>()));
    let next = std::sync::atomic::AtomicUsize::new(0);
    let run = || {
        let types = types::Types::new(p);
        let mut done = Vec::new();
        loop {
            let k = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let Some(&i) = order.get(k) else { break };
            let Some(f) = fs.get(i) else { break };
            done.push((i, work(i, f, &types)));
        }
        done
    };
    let mut out: Vec<Option<T>> = std::iter::repeat_with(|| None).take(fs.len()).collect();
    std::thread::scope(|s| {
        let others: Vec<_> =
            (1..threads).filter_map(|_| std::thread::Builder::new().spawn_scoped(s, run).ok()).collect();
        let mut done = run();
        for w in others {
            match w.join() {
                Ok(theirs) => done.extend(theirs),
                // A panic in an analysis is the checker's own: raised
                // again here, as one on this thread would be.
                Err(e) => std::panic::resume_unwind(e),
            }
        }
        for (i, r) in done {
            out[i] = r;
        }
    });
    out
}

fn dedup(fs: &mut Vec<Finding>) {
    type Key = (MsgId, Option<sir::Loc>, Params, String);
    let mut seen: Vec<Key> = Vec::new();
    fs.retain(|f| {
        let key = (f.msg, f.loc, f.params.clone(), f.function.clone());
        if seen.contains(&key) {
            false
        } else {
            seen.push(key);
            true
        }
    });
}

/// The span of a SIR location in a file of the source map.
pub fn span_of(map: &SourceMap, file: FileId, loc: Option<sir::Loc>) -> Span {
    let Some(l) = loc else { return Span::new(file, TextRange::empty(0)) };
    let Some(f) = map.get(file) else { return Span::new(file, TextRange::empty(0)) };
    let start = f.offset(LineCol { line: l.line, col: l.col });
    let end = if l.has_end() { f.offset(LineCol { line: l.end_line, col: l.end_col }) } else { start };
    Span::new(file, TextRange::new(start, end))
}

fn render_arg(map: &SourceMap, file: FileId, a: &Arg) -> String {
    match a {
        Arg::Text(t) => t.clone(),
        Arg::Source(l) => map.slice(span_of(map, file, Some(*l))).to_string(),
    }
}

/// A finding as a diagnostic; `files[i]` is the file of SIR source `i`.
pub fn to_diagnostic(f: &Finding, map: &SourceMap, files: &[FileId]) -> Diagnostic {
    let file = files.get(f.source as usize).copied().unwrap_or_default();
    let mut d = Diagnostic::new(f.msg, span_of(map, file, f.loc));
    for (k, v) in &f.params {
        d = d.with(k, render_arg(map, file, v));
    }
    for l in f.labels.iter().filter(|l| l.loc.is_some()) {
        let params = l.params.iter().map(|(k, v)| (*k, render_arg(map, file, v))).collect();
        d = d.label(span_of(map, file, l.loc), l.msg, params);
    }
    for (m, ps) in &f.notes {
        let params = ps.iter().map(|(k, v)| (*k, render_arg(map, file, v))).collect();
        d = d.note(*m, params);
    }
    for (m, ps, edit) in &f.fixes {
        let params = ps.iter().map(|(k, v)| (*k, render_arg(map, file, v))).collect();
        let edits = match edit {
            Some((loc, text)) => vec![Edit { span: span_of(map, file, Some(*loc)), text: text.clone() }],
            None => Vec::new(),
        };
        d = d.fix(*m, params, edits);
    }
    d
}

#[cfg(test)]
mod tests;
