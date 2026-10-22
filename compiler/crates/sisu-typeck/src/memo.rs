//! The per-item query of the checker (docs/architecture.md, "The query
//! layer"; 10, "Compile speed as a requirement"): a body is checked again
//! only when its item's text, the program's interface or an inferred error
//! set it read changed. Resolution is deterministic, so two programs with
//! the same interface (a hash of everything resolution read, which the
//! query layer computes from the sources) have the same definitions; a
//! body's typed tree then depends on its own item's text alone, wherever
//! the item now starts, and on the error sets of the `T !` functions it
//! calls. An entry made where its item started elsewhere is moved there.

use crate::thir::{Body, BodyKind, Memo};
use sisu_diag::{Diagnostic, FileId, SourceMap, Span};
use sisu_resolve::{DefId, DefKind, Program};
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

/// The memo of checked bodies of one program, kept by the query layer
/// between checks of it.
#[derive(Default)]
pub struct BodyCache {
    entries: HashMap<(u64, DefId, BodyKind), Vec<Entry>>,
    generation: u64,
    /// The bodies the current query checked, and the ones it reused.
    pub checked: u32,
    pub reused: u32,
}

struct Entry {
    /// A hash of the item's text.
    content: u64,
    file: FileId,
    /// Where the item started when the entry was made (or last moved).
    start: u32,
    /// The inferred error sets the body read, as they were.
    reads: Vec<(DefId, Vec<DefId>)>,
    body: Arc<Body>,
    diags: Vec<Diagnostic>,
    absorbed: Vec<DefId>,
    used: u64,
}

/// The most entries kept for one body: the fixpoint of inferred error sets
/// may check a body against several sets in one query.
const PER_BODY: usize = 8;

impl BodyCache {
    /// Starts a query: what it does not use, [`BodyCache::sweep`] drops.
    pub fn begin(&mut self) {
        self.generation += 1;
        self.checked = 0;
        self.reused = 0;
    }

    /// Drops every entry the current query did not use.
    pub fn sweep(&mut self) {
        let g = self.generation;
        for list in self.entries.values_mut() {
            list.retain(|e| e.used == g);
        }
        self.entries.retain(|_, list| !list.is_empty());
    }

    /// How many bodies the memo holds.
    pub fn len(&self) -> usize {
        self.entries.values().map(Vec::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

pub(crate) struct Session<'c> {
    pub cache: &'c mut BodyCache,
    pub iface: u64,
}

fn line_start(text: &str, at: u32) -> u32 {
    let at = (at as usize).min(text.len());
    text.get(..at).and_then(|t| t.rfind('\n')).map(|i| i as u32 + 1).unwrap_or(0)
}

/// Where a body's item is: its file, and the bytes from the start of the
/// line its name (or its field, parameter or constant) is on to the end of
/// what is checked.
fn region(prog: &Program, sources: &SourceMap, def: DefId, kind: &BodyKind) -> Option<(FileId, u32, u32)> {
    let d = prog.def(def);
    let (file, from, end) = match kind {
        BodyKind::Fn => {
            let f = prog.fn_info(def)?;
            let b = f.body.as_ref()?;
            (f.file, d.span.range.start.min(b.range.start), b.range.end.max(d.span.range.end))
        }
        BodyKind::FieldDefault(s, i) => {
            let DefKind::Struct(info) = &prog.def(*s).kind else { return None };
            let field = info.fields.get(*i as usize)?;
            let e = field.default.as_ref()?;
            (field.span.file, field.span.range.start.min(e.range.start), e.range.end)
        }
        BodyKind::ParamDefault(fd, i) => {
            let f = prog.fn_info(*fd)?;
            let p = f.params.get(*i as usize)?;
            let e = p.default.as_ref()?;
            (f.file, p.span.range.start.min(e.range.start), e.range.end)
        }
        BodyKind::Const => {
            let DefKind::Const(c) = &d.kind else { return None };
            let e = c.value.as_ref()?;
            (c.file, d.span.range.start.min(e.range.start), e.range.end.max(d.span.range.end))
        }
    };
    if d.span.file != file && matches!(kind, BodyKind::Fn | BodyKind::Const) {
        return None;
    }
    let text = &sources.get(file)?.text;
    if end as usize > text.len() {
        return None;
    }
    Some((file, line_start(text, from), end))
}

fn hash_of(x: impl Hash) -> u64 {
    let mut h = DefaultHasher::new();
    x.hash(&mut h);
    h.finish()
}

/// Whether every span lies in the item (or in the standard library, whose
/// text never changes), so that the entry can move with the item.
fn movable(body: &mut Body, diags: &mut [Diagnostic], sources: &SourceMap, at: (FileId, u32, u32)) -> bool {
    let (file, start, end) = at;
    let mut ok = true;
    let mut check = |s: &mut Span| {
        if s.file == file {
            ok &= start <= s.range.start && s.range.end <= end;
        } else {
            ok &= sources.name(s.file).starts_with("<std>/");
        }
    };
    crate::spans::body(body, &mut check);
    for d in diags.iter_mut() {
        d.spans_mut().for_each(&mut check);
    }
    ok
}

/// Moves every span of `file` by `delta` bytes.
fn shift(body: &mut Arc<Body>, diags: &mut [Diagnostic], file: FileId, delta: i64) {
    if delta == 0 {
        return;
    }
    let body = Arc::make_mut(body);
    let mut mv = |s: &mut Span| {
        if s.file == file {
            s.range.start = (i64::from(s.range.start) + delta).max(0) as u32;
            s.range.end = (i64::from(s.range.end) + delta).max(0) as u32;
        }
    };
    crate::spans::body(body, &mut mv);
    for d in diags.iter_mut() {
        d.spans_mut().for_each(&mut mv);
    }
    body.memo.start = (i64::from(body.memo.start) + delta).max(0) as u32;
    body.memo.end = (i64::from(body.memo.end) + delta).max(0) as u32;
}

/// Checks one body (a function's, a default value's or a constant's),
/// through the memo when there is one: the typed body, its diagnostics and
/// the error set its `T !` absorbed.
pub(crate) fn memo_body(
    prog: &Program,
    inferred: &HashMap<DefId, Vec<DefId>>,
    def: DefId,
    kind: BodyKind,
    sources: &SourceMap,
    memo: &mut Option<Session>,
) -> (Arc<Body>, Vec<Diagnostic>, Vec<DefId>) {
    match memo_lookup(prog, inferred, def, &kind, sources, memo) {
        Lookup::Hit(found) => found,
        Lookup::Miss(slot) => {
            let checked = check_body(prog, inferred, def, &kind, sources);
            memo_store(inferred, def, kind, sources, memo, slot, checked)
        }
    }
}

/// What the memo holds for a body: what a check gave, or where to keep
/// what checking it gives (none when it cannot be kept).
pub(crate) enum Lookup {
    Hit((Arc<Body>, Vec<Diagnostic>, Vec<DefId>)),
    Miss(Option<Slot>),
}

/// Where a body's check is kept: the item's place and a hash of its text.
pub(crate) struct Slot {
    file: FileId,
    start: u32,
    end: u32,
    content: u64,
}

/// The memo's entry for a body, moved to where its item now is, or the
/// slot for one.
pub(crate) fn memo_lookup(
    prog: &Program,
    inferred: &HashMap<DefId, Vec<DefId>>,
    def: DefId,
    kind: &BodyKind,
    sources: &SourceMap,
    memo: &mut Option<Session>,
) -> Lookup {
    let at = region(prog, sources, def, kind);
    let (Some(m), Some((file, start, end))) = (memo.as_mut(), at) else {
        return Lookup::Miss(None);
    };
    let text = sources.get(file).map(|f| f.text.as_str()).unwrap_or("");
    // An instance of a template is told apart by what it instantiates too:
    // two instances may unfold to the same text with other types behind
    // its names.
    let instance = match kind {
        BodyKind::Fn => prog.fn_info(def).and_then(|f| f.instance_of.clone()),
        BodyKind::ParamDefault(f, _) => prog.fn_info(*f).and_then(|f| f.instance_of.clone()),
        _ => None,
    };
    let content = hash_of((text.get(start as usize..end as usize).unwrap_or(""), instance));
    let key = (m.iface, def, kind.clone());
    let generation = m.cache.generation;
    if let Some(list) = m.cache.entries.get_mut(&key)
        && let Some(e) = list.iter_mut().find(|e| {
            e.content == content
                && e.file == file
                && e.reads
                    .iter()
                    .all(|(d, set)| inferred.get(d).map(Vec::as_slice).unwrap_or(&[]) == set.as_slice())
        })
    {
        let delta = i64::from(start) - i64::from(e.start);
        shift(&mut e.body, &mut e.diags, file, delta);
        e.start = start;
        e.used = generation;
        m.cache.reused += 1;
        return Lookup::Hit((e.body.clone(), e.diags.clone(), e.absorbed.clone()));
    }
    Lookup::Miss(Some(Slot { file, start, end, content }))
}

/// Keeps what checking a body gave in its slot, when there is one and the
/// check can move with the item; the body, its diagnostics and the error
/// set it absorbed.
pub(crate) fn memo_store(
    inferred: &HashMap<DefId, Vec<DefId>>,
    def: DefId,
    kind: BodyKind,
    sources: &SourceMap,
    memo: &mut Option<Session>,
    slot: Option<Slot>,
    checked: (Body, Vec<Diagnostic>, Vec<DefId>, Vec<DefId>),
) -> (Arc<Body>, Vec<Diagnostic>, Vec<DefId>) {
    let (mut body, mut diags, absorbed, reads) = checked;
    let (Some(m), Some(Slot { file, start, end, content })) = (memo.as_mut(), slot) else {
        return (Arc::new(body), diags, absorbed);
    };
    m.cache.checked += 1;
    let generation = m.cache.generation;
    let key = (m.iface, def, kind.clone());
    let reads: Vec<(DefId, Vec<DefId>)> =
        reads.into_iter().map(|d| (d, inferred.get(&d).cloned().unwrap_or_default())).collect();
    if movable(&mut body, &mut diags, sources, (file, start, end)) {
        body.memo = Memo { key: hash_of((m.iface, def, &kind, content, &reads)), file, start, end };
        let list = m.cache.entries.entry(key).or_default();
        if list.len() >= PER_BODY {
            list.remove(0);
        }
        let body = Arc::new(body);
        list.push(Entry {
            content,
            file,
            start,
            reads,
            body: body.clone(),
            diags: diags.clone(),
            absorbed: absorbed.clone(),
            used: generation,
        });
        return (body, diags, absorbed);
    }
    (Arc::new(body), diags, absorbed)
}

/// Checks a body without the memo: with the inferred error sets it read.
pub(crate) fn check_body(
    prog: &Program,
    inferred: &HashMap<DefId, Vec<DefId>>,
    def: DefId,
    kind: &BodyKind,
    sources: &SourceMap,
) -> (Body, Vec<Diagnostic>, Vec<DefId>, Vec<DefId>) {
    let value = |kind: BodyKind, e: &sisu_syntax::ast::Expr, ty: &sisu_resolve::Ty, file: FileId| {
        let (b, d, reads) = crate::check_value(prog, inferred, def, kind, e, ty, file, sources);
        (b, d, Vec::new(), reads)
    };
    match kind {
        BodyKind::Fn => crate::check_fn(prog, inferred, def, sources),
        BodyKind::FieldDefault(s, i) => match &prog.def(*s).kind {
            DefKind::Struct(info) => match info.fields.get(*i as usize) {
                Some(f) => match &f.default {
                    Some(e) => value(kind.clone(), e, &f.ty, f.span.file),
                    None => crate::check_fn(prog, inferred, def, sources),
                },
                None => crate::check_fn(prog, inferred, def, sources),
            },
            _ => crate::check_fn(prog, inferred, def, sources),
        },
        BodyKind::ParamDefault(fd, i) => {
            match prog.fn_info(*fd).and_then(|f| f.params.get(*i as usize).map(|p| (f, p))) {
                Some((f, p)) => match &p.default {
                    Some(e) => value(kind.clone(), e, &p.ty, f.file),
                    None => crate::check_fn(prog, inferred, def, sources),
                },
                None => crate::check_fn(prog, inferred, def, sources),
            }
        }
        BodyKind::Const => match &prog.def(def).kind {
            DefKind::Const(c) => match &c.value {
                Some(e) => value(kind.clone(), e, &c.ty, c.file),
                None => crate::check_fn(prog, inferred, def, sources),
            },
            _ => crate::check_fn(prog, inferred, def, sources),
        },
    }
}
