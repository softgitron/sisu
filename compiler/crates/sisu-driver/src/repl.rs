//! `sisu repl` and `sisu ws` (10, "REPL and scripts").
//!
//! A REPL session is a script (`#!/usr/bin/env sisu`): each input the
//! session keeps is one more top-level statement or item of it, in order,
//! and an input that is an expression is shown (its `Debug` rendering,
//! unless it is `()`). Each input runs the whole script again from the
//! top, with the log of the evaluation before it standing in for the
//! world up to where that evaluation ended (`sisu_interp::record::Resume`),
//! so the earlier inputs print nothing and touch nothing again, and only
//! the new one runs live: the session has no heap to snapshot and no
//! hidden state. An input that does not check, panics or returns an
//! error is not kept.
//!
//! `:back N` forgets the last N inputs (REPL undo: the inputs before them
//! replay against their log). `:why NAME` answers where a value came
//! from: the input that bound it, with the value it has now; the values
//! of the session it was computed from, each explained the same way; the
//! capability results that input received, from its log; and the inputs
//! that changed it since. It follows the session's own bindings, not the
//! functions they call (a dynamic slice inside them is a later step).
//!
//! `sisu repl` in a package's directory, or naming one, loads it: the
//! session depends on the package, as a script's inline metadata would,
//! so it imports the package's modules by their paths.
//!
//! The session's program, and a worksheet's, is checked and lowered by
//! stage 1 when it serves them (`sisu1 0.8.0` and later, decision 563):
//! the program is written to a file of a directory of its own, stage 1's
//! `sir` gives its SIR, which the interpreter runs as it runs the Rust
//! front end's, and a program that does not check is reported by stage
//! 1's `check`, its places moved into the input as the Rust front end's
//! are. An older stage 1, or `--front-end=rust`, leaves them to the Rust
//! front end.
//!
//! A worksheet (`sisu ws FILE`, Q-48) is a script run top to bottom, with
//! each top-level statement's value and printed output written after it
//! as comments, never saved. It runs against the fake `System` of a test
//! (files in memory, no network, the test clock, seeded random numbers)
//! unless `--run` asks for the real one: code that can reach a real
//! capability runs only on an explicit run.

use crate::cli::{Captured, Input, NotRun, WithArgs, check_to_run, input_with_root, plain_error, text};
use crate::db::Db;
use crate::front::FrontEnd;
use sisu_diag::{Diagnostic, FileId, MsgId, SourceMap, Span, TextRange, msg};
use sisu_interp::record::{Header, Log, Recorder, Resume};
use sisu_interp::{Host, Options, exit};
use sisu_syntax::SyntaxKind::{self, *};
use sisu_syntax::{NodeId, Tree};

/// How diagnostics name the session's program and an input.
const SESSION: &str = "<repl>";
const INPUT: &str = "<input>";

/// What the session's program adds after its inputs: the function that
/// shows an expression's value.
const SHOW: &str = "\ndef __repl_show[T](value: T):\n    if comptime reflect.name_of(T) != \"()\":\n        print(\"{value:?}\")\n";

/// The worksheet's markers on standard output: a statement starts, and a
/// statement's value.
const AT: char = '\u{1}';
const VALUE: char = '\u{2}';
const WS_HELPERS: &str = "\ndef __ws_at(line: int):\n    print(\"\\u{1}{line}\")\n\ndef __ws_show[T](line: int, value: T):\n    if comptime reflect.name_of(T) != \"()\":\n        print(\"\\u{2}{line} {value:?}\")\n";

/// A function of the worksheet's program that reaches the fake `System`
/// it runs against, which stage 1 otherwise leaves out of the SIR.
const WS_FAKE: &str = "\ndef __ws_fake() -> System:\n    return System.fake(0)\n";

fn line(id: MsgId, params: &[(&str, String)]) -> String {
    format!("{}\n", text(id, params))
}

/// What checks and lowers a session's program, or a worksheet's: stage 1
/// (decision 563), with the directory its programs are written to, or
/// the Rust front end.
pub(crate) enum Checker {
    Rust,
    Stage1 { stage1: crate::front::Stage1, dir: std::path::PathBuf },
}

impl Checker {
    /// Stage 1 when it is the front end and serves the REPL; else the Rust
    /// front end.
    pub(crate) fn choose(front: FrontEnd, host: &mut dyn Host) -> Checker {
        if front != FrontEnd::Stage1 {
            return Checker::Rust;
        }
        let Ok(stage1) = crate::front::Stage1::ready(host) else { return Checker::Rust };
        if !stage1.serves_repl() {
            return Checker::Rust;
        }
        static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("sisu-repl-{}-{n}", std::process::id()));
        if std::fs::create_dir_all(&dir).is_err() {
            return Checker::Rust;
        }
        Checker::Stage1 { stage1, dir }
    }

    /// The program `text`, named `name` in what is shown of it, checked and
    /// lowered with the functions `keep` named kept, or why it does not
    /// run: the Rust front end's diagnostics, or stage 1's report, the
    /// program named `name` in it.
    fn compile(
        &self,
        name: &str,
        text: &str,
        cwd: &std::path::Path,
        keep: &[&str],
        host: &mut dyn Host,
    ) -> Result<sisu_sir::Program, NotRun> {
        let (stage1, dir) = match self {
            Checker::Rust => return rust_compile(name, text, cwd, keep, host),
            Checker::Stage1 { stage1, dir } => (stage1, dir),
        };
        use crate::front::Backend;
        let base = std::path::Path::new(name)
            .file_name()
            .and_then(|f| f.to_str())
            .filter(|f| f.ends_with(".sisu"))
            .unwrap_or("session.sisu");
        let path = dir.join(base);
        if let Err(e) = std::fs::write(&path, text) {
            return Err(NotRun::Message(format!("{}\n", text_of_io(&path, &e))));
        }
        let shown = path.display().to_string();
        let words = vec!["sir".to_string(), shown.clone()];
        let ran = stage1.run(&words, Backend::Unused, host).map_err(|_| NotRun::Message(String::new()))?;
        if ran.status == exit::OK && !ran.stdout.is_empty() {
            // The program's name, where its sources and the places its
            // errors record name it.
            let sir = ran.stdout.replace(&shown, name);
            return sisu_sir::read(&sir).map_err(|e| {
                NotRun::Message(line(
                    msg::SIR_PARSE,
                    &[("detail", format!("line {}: {}", e.line, e.message))],
                ))
            });
        }
        // Stage 1's `sir` prints nothing for a program that does not check;
        // its `check` says why.
        let mut report = ran.stderr;
        if report.trim().is_empty() {
            let words = vec!["check".to_string(), shown.clone()];
            if let Ok(r) = stage1.run(&words, Backend::Unused, host) {
                report = r.stderr;
            }
        }
        Err(NotRun::Message(report.replace(&shown, name)))
    }
}

impl Drop for Checker {
    fn drop(&mut self) {
        if let Checker::Stage1 { dir, .. } = self {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

fn text_of_io(path: &std::path::Path, e: &std::io::Error) -> String {
    text(msg::CLI_CANNOT_READ, &[("path", path.display().to_string()), ("reason", e.to_string())])
}

/// The Rust front end's check and lowering of a program named `name`.
fn rust_compile(
    name: &str,
    text: &str,
    cwd: &std::path::Path,
    keep: &[&str],
    host: &mut dyn Host,
) -> Result<sisu_sir::Program, NotRun> {
    let layout = crate::package::load_file(name, text, cwd);
    if layout.has_errors() {
        let mut map = SourceMap::new();
        for (n, t) in &layout.files {
            map.add(n.clone(), t.clone());
        }
        return Err(NotRun::Diagnostics(map, layout.diagnostics.clone()));
    }
    let input = if layout.packages.len() == 1 {
        Input::single(name, text.to_string())
    } else {
        input_with_root(&layout, host, false, Some(text)).map_err(|_| NotRun::Message(String::new()))?
    };
    let mut db = Db::new();
    input.load(&mut db);
    check_to_run(&mut db, name, keep)
}

/// A report stage 1 rendered for the program `name`, with the places in
/// its lines `first` to `last` moved to the text it shows them in, named
/// `shown`, whose line `shown_first` is the program's line `first`: each
/// `--> name:LINE:COL` and the line numbers of the gutter beside it, the
/// gutter as wide as its numbers then are. A diagnostic of another part
/// of the program is left as it is.
fn relocate(report: &str, name: &str, first: usize, last: usize, shown: &str, shown_first: usize) -> String {
    let moved = |l: usize| l - first + shown_first;
    let mut out = String::new();
    let lines: Vec<&str> = report.split_inclusive('\n').collect();
    let mut i = 0;
    while i < lines.len() {
        // A diagnostic: its header and the lines after it, which start
        // with a space or a gutter's number.
        let mut j = i + 1;
        while j < lines.len() && !lines[j].starts_with(|c: char| c.is_alphabetic()) {
            j += 1;
        }
        let block = &lines[i..j];
        i = j;
        let at = |l: &str| -> Option<(usize, usize)> {
            let rest = l.trim_start().strip_prefix("--> ")?.trim_end();
            let (file, pos) = rest.rsplit_once(':').and_then(|(a, col)| {
                let (file, line) = a.rsplit_once(':')?;
                Some((file, (line.parse::<usize>().ok()?, col.parse::<usize>().ok()?)))
            })?;
            (file == name).then_some(pos)
        };
        let inside = block.iter().filter_map(|l| at(l)).any(|(l, _)| (first..=last).contains(&l));
        if !inside {
            block.iter().for_each(|l| out.push_str(l));
            continue;
        }
        // The gutter: its bar's column, from the first line that is a bar
        // alone.
        let bar = block.iter().find_map(|l| {
            let t = l.trim_end();
            (t.ends_with('|') && t[..t.len() - 1].chars().all(|c| c == ' ')).then(|| t.len() - 1)
        });
        let number = |l: &str| -> Option<(usize, String)> {
            let b = bar?;
            let head = l.get(..b)?;
            let n: usize = head.trim().parse().ok()?;
            l.get(b..)?.starts_with('|').then(|| (n, l[b..].to_string()))
        };
        let width = block
            .iter()
            .filter_map(|l| number(l))
            .map(|(n, _)| if (first..=last).contains(&n) { moved(n) } else { n })
            .max()
            .map_or(0, |n| n.to_string().len());
        for l in block {
            if let Some((n, c)) = at(l) {
                let indent = &l[..l.len() - l.trim_start().len()];
                let n = if (first..=last).contains(&n) { moved(n) } else { n };
                out.push_str(&format!("{indent}--> {shown}:{n}:{c}\n"));
            } else if let Some((n, rest)) = number(l) {
                let n = if (first..=last).contains(&n) { moved(n) } else { n };
                out.push_str(&format!("{n:>width$} {rest}"));
            } else if let Some(b) = bar
                && l.len() > b
                && l[..b].chars().all(|c| c == ' ')
                && matches!(l.as_bytes()[b], b'|' | b'=')
            {
                out.push_str(&format!("{:width$} {}", "", &l[b..]));
            } else {
                out.push_str(l);
            }
        }
    }
    out
}

/// One input the session kept.
#[derive(Clone, Debug)]
struct Entry {
    /// The input as typed.
    text: String,
    /// The input as the session's program holds it: an expression is
    /// wrapped to be shown, so its text starts `prefix` bytes in.
    source: String,
    prefix: usize,
    /// How its diagnostics show it: the name, the text they point into,
    /// and where the input starts in that text.
    shown: (String, String, usize),
}

impl Entry {
    fn of(input: &str) -> Entry {
        let (tree, _) = parse_script(input);
        let stmts: Vec<NodeId> = top_level(&tree);
        let expression = stmts.len() == 1 && tree.kind(stmts[0]) == EXPR_STMT;
        let shown = (INPUT.to_string(), input.trim_end().to_string(), 0);
        if expression {
            // The call on a line of its own, so that the input keeps its
            // lines and columns in the session's program.
            let prefix = "__repl_show(\n";
            return Entry {
                text: input.to_string(),
                source: format!("{prefix}{}\n)\n", input.trim_end()),
                prefix: prefix.len(),
                shown,
            };
        }
        Entry { text: input.to_string(), source: format!("{}\n", input.trim_end()), prefix: 0, shown }
    }
}

/// A file loaded into a session (`sisu repl FILE`): its `#!` line and
/// inline metadata, which lead the session's program, and the rest, its
/// first input.
fn split_preamble(file: &str, text: &str) -> (String, Entry) {
    let mut end = 0;
    let mut open = false;
    let mut offset = 0;
    for l in text.split_inclusive('\n') {
        let bare = l.trim_end();
        offset += l.len();
        if !open && bare == "# /// script" {
            open = true;
        } else if open && bare == "# ///" {
            end = offset;
            break;
        }
    }
    if end == 0 && text.starts_with("#!") {
        end = text.find('\n').map(|i| i + 1).unwrap_or(text.len());
    }
    let mut preamble = text[..end].to_string();
    if !preamble.starts_with("#!") {
        preamble.insert_str(0, "#!/usr/bin/env sisu\n");
    }
    let body = text[end..].to_string();
    let entry = Entry {
        source: if body.ends_with('\n') || body.is_empty() { body.clone() } else { format!("{body}\n") },
        text: body,
        prefix: 0,
        shown: (file.to_string(), text.to_string(), end),
    };
    (preamble, entry)
}

/// An input parsed as the statements of a script, and the text parsed
/// (with the `#!` line in front).
fn parse_script(input: &str) -> (Tree, String) {
    let text = format!("#!/usr/bin/env sisu\n{input}\n");
    let parse = sisu_parser::parse(FileId(0), &text);
    (parse.tree, text)
}

/// The top-level statements and items of a parsed script.
fn top_level(tree: &Tree) -> Vec<NodeId> {
    tree.child_nodes(tree.root()).collect()
}

/// The names a `let` or `var` binds.
fn bound_names(tree: &Tree, stmt: NodeId) -> Vec<String> {
    if tree.kind(stmt) != LET_STMT {
        return Vec::new();
    }
    let mut out = Vec::new();
    for c in tree.child_nodes(stmt) {
        if matches!(tree.kind(c), IDENT_PAT | TUPLE_PAT) {
            for n in tree.preorder(c) {
                if tree.kind(n) == IDENT_PAT
                    && let Some(t) = tree.first_token(n, IDENT)
                {
                    out.push(tree.token_text(t).to_string());
                }
            }
        }
    }
    out
}

/// The initialiser of a `let`: its expression's node.
fn initialiser(tree: &Tree, stmt: NodeId) -> Option<NodeId> {
    let eq = tree.first_token(stmt, EQ)?;
    let at = tree.token_range(eq).end;
    tree.child_nodes(stmt).find(|&c| tree.range(c).start >= at)
}

/// The names an expression reads.
fn names_read(tree: &Tree, n: NodeId) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for c in tree.preorder(n) {
        if tree.kind(c) == NAME_REF
            && let Some(t) = tree.first_token(c, IDENT)
        {
            let name = tree.token_text(t).to_string();
            if !out.contains(&name) {
                out.push(name);
            }
        }
    }
    out
}

/// Whether a statement may change `name`: assigns to it (or into it), or
/// calls a method on it.
fn changes(tree: &Tree, stmt: NodeId, name: &str) -> bool {
    let root_name = |n: NodeId| -> Option<String> {
        let mut cur = n;
        loop {
            match tree.kind(cur) {
                NAME_REF => return tree.first_token(cur, IDENT).map(|t| tree.token_text(t).to_string()),
                FIELD_EXPR | INDEX_EXPR => cur = tree.child_nodes(cur).next()?,
                _ => return None,
            }
        }
    };
    match tree.kind(stmt) {
        ASSIGN_STMT => tree.child_nodes(stmt).next().and_then(root_name).as_deref() == Some(name),
        EXPR_STMT => tree.preorder(stmt).into_iter().any(|c| {
            tree.kind(c) == CALL_EXPR
                && tree
                    .child_nodes(c)
                    .next()
                    .filter(|&f| tree.kind(f) == FIELD_EXPR)
                    .and_then(|f| tree.child_nodes(f).next())
                    .and_then(root_name)
                    .as_deref()
                    == Some(name)
        }),
        _ => false,
    }
}

/// Whether more lines belong to an input: it opens a block, starts with
/// an attribute, or leaves a bracket open.
fn needs_more(input: &str) -> bool {
    let last = input.lines().last().unwrap_or("").trim_end();
    if last.ends_with(':') || input.trim_start().starts_with('@') {
        return true;
    }
    let mut depth: i32 = 0;
    let mut in_str = false;
    let mut prev = ' ';
    for c in input.chars() {
        match c {
            '"' if prev != '\\' => in_str = !in_str,
            '(' | '[' | '{' if !in_str => depth += 1,
            ')' | ']' | '}' if !in_str => depth -= 1,
            '#' if !in_str => {}
            _ => {}
        }
        prev = c;
    }
    depth > 0
}

/// A diagnostic's spans in the session's program moved into the text the
/// input is shown in (`FileId(0)` of a map holding only it, where the
/// input starts at `delta`), when they are in the input.
fn rebase(d: &Diagnostic, root: FileId, start: u32, len: u32, delta: u32) -> Option<Diagnostic> {
    let inside = |s: &Span| s.file == root && s.range.start >= start && s.range.end <= start + len;
    if !inside(&d.span) {
        return None;
    }
    let mv = |s: &mut Span| {
        if inside(s) {
            let r = TextRange::new(s.range.start - start + delta, s.range.end - start + delta);
            *s = Span::new(FileId(0), r);
        }
    };
    let mut d = d.clone();
    mv(&mut d.span);
    d.labels.iter_mut().for_each(|l| mv(&mut l.span));
    for f in &mut d.fixes {
        f.edits.iter_mut().for_each(|e| mv(&mut e.span));
    }
    Some(d)
}

/// A REPL session: the inputs it kept and the log of each evaluation.
struct Session {
    /// What leads the session's program: its `#!` line, and the inline
    /// metadata of a file loaded into it.
    preamble: String,
    entries: Vec<Entry>,
    /// `logs[k]`: the log of the evaluation that kept `entries[k]`.
    logs: Vec<Log>,
    /// The package the session loads: its name and directory.
    package: Option<(String, std::path::PathBuf)>,
    cwd: std::path::PathBuf,
    checker: Checker,
}

impl Session {
    /// The session's program: its first `upto` inputs, then `extra`.
    /// Returns the text and where `extra` starts in it.
    fn program(&self, upto: usize, extra: Option<&str>) -> (String, u32) {
        let mut text = self.preamble.clone();
        if let Some((name, dir)) = &self.package {
            let path = dir.display().to_string().replace('\\', "\\\\").replace('"', "\\\"");
            text.push_str(&format!(
                "# /// script\n# [dependencies]\n# {name} = {{ path = \"{path}\" }}\n# ///\n"
            ));
        }
        for e in &self.entries[..upto] {
            text.push_str(&e.source);
        }
        let at = text.len() as u32;
        if let Some(x) = extra {
            text.push_str(x);
        }
        // A statement, so the script has its `main` before any input does.
        text.push_str("pass\n");
        text.push_str(SHOW);
        (text, at)
    }

    fn compile(&self, program: &str, host: &mut dyn Host) -> Result<sisu_sir::Program, NotRun> {
        self.checker.compile(SESSION, program, &self.cwd, &[], host)
    }

    /// Why an input was not kept, as the input's own diagnostics where
    /// they point into it.
    fn explain(&self, why: NotRun, program: &str, at: u32, entry: &Entry) -> String {
        match why {
            // Stage 1's report, its places in the input moved there.
            NotRun::Message(m) => {
                let start = (at as usize + entry.prefix).min(program.len());
                let first = program[..start].matches('\n').count() + 1;
                let last = first + entry.text.trim_end().lines().count().max(1) - 1;
                let (name, shown, delta) = &entry.shown;
                let shown_first = shown[..(*delta).min(shown.len())].matches('\n').count() + 1;
                relocate(&m, SESSION, first, last, name, shown_first)
            }
            NotRun::Diagnostics(map, diags) => {
                let root =
                    map.files().iter().find(|f| f.name == SESSION).map(|f| f.id).unwrap_or(FileId(u32::MAX));
                let start = at + entry.prefix as u32;
                let len = entry.text.trim_end().len() as u32;
                let (name, shown, delta) = &entry.shown;
                let mut input_map = SourceMap::new();
                input_map.add(name.clone(), shown.clone());
                let (mut mine, mut other) = (Vec::new(), Vec::new());
                for d in &diags {
                    match rebase(d, root, start, len, *delta as u32) {
                        Some(r) => mine.push(r),
                        None => other.push(d.clone()),
                    }
                }
                let mut out = sisu_diag::render::render_human(&input_map, &mine);
                if !other.is_empty() {
                    out.push_str(&sisu_diag::render::render_human(&map, &other));
                }
                out
            }
        }
    }

    /// Runs the session with `entry` after its inputs, and keeps it if it
    /// ran to the end.
    fn evaluate(&mut self, entry: Entry, host: &mut dyn Host) {
        let n = self.entries.len();
        let (program, at) = self.program(n, Some(&entry.source));
        let prog = match self.compile(&program, host) {
            Ok(p) => p,
            Err(e) => {
                let shown = self.explain(e, &program, at, &entry);
                host.stderr(&shown);
                return;
            }
        };
        let (status, log, diverged) = run(&prog, self.logs.last(), host);
        if let Some(d) = diverged {
            host.stderr(&line(msg::CLI_REPL_DIVERGED, &[("detail", d)]));
        }
        match log {
            Some(log) if status == exit::OK => {
                self.entries.push(entry);
                self.logs.push(log);
            }
            _ => host.stderr(&line(msg::CLI_REPL_NOT_KEPT, &[])),
        }
    }

    /// The `Debug` rendering of `name` after the first `upto` inputs.
    fn value_after(&self, upto: usize, name: &str, host: &mut dyn Host) -> String {
        let show = format!("__repl_show({name})\n");
        let (program, _) = self.program(upto, Some(&show));
        let Ok(prog) = self.compile(&program, host) else { return "?".into() };
        let earlier = upto.checked_sub(1).and_then(|k| self.logs.get(k));
        let mut cap = Captured { inner: &mut *host, out: String::new(), err: String::new() };
        let _ = run(&prog, earlier, &mut cap);
        let out = cap.out.trim_end().to_string();
        if out.is_empty() { "?".into() } else { out }
    }

    /// `:why NAME`.
    fn why(&self, name: &str, host: &mut dyn Host) {
        let mut seen = Vec::new();
        let found = self.entries.iter().rposition(|e| {
            let (tree, _) = parse_script(&e.text);
            top_level(&tree).into_iter().any(|s| bound_names(&tree, s).iter().any(|b| b == name))
        });
        let Some(k) = found else {
            host.stdout(&line(msg::CLI_REPL_WHY_UNBOUND, &[("name", name.to_string())]));
            return;
        };
        let value = self.value_after(self.entries.len(), name, host);
        self.why_at(k, name, value, 0, &mut seen, host);
    }

    fn why_at(
        &self,
        k: usize,
        name: &str,
        value: String,
        depth: usize,
        seen: &mut Vec<String>,
        host: &mut dyn Host,
    ) {
        seen.push(name.to_string());
        let pad = "  ".repeat(depth);
        let (tree, _) = parse_script(&self.entries[k].text);
        let stmt = top_level(&tree).into_iter().find(|&s| bound_names(&tree, s).iter().any(|b| b == name));
        let init = stmt.and_then(|s| initialiser(&tree, s));
        let expr = init.map(|i| tree.node_text(i).trim().to_string()).unwrap_or_default();
        host.stdout(&format!(
            "{pad}{}",
            line(
                msg::CLI_REPL_WHY_VALUE,
                &[
                    ("name", name.to_string()),
                    ("value", value),
                    ("expr", expr),
                    ("input", (k + 1).to_string())
                ],
            )
        ));
        // The capability results the input received.
        let from = k.checked_sub(1).and_then(|j| self.logs.get(j)).map(Log::len).unwrap_or(0);
        if let Some(log) = self.logs.get(k) {
            for (n, call) in log.calls_from(from) {
                if call.starts_with("virtual_time") {
                    continue;
                }
                host.stdout(&format!(
                    "{pad}  {}",
                    line(msg::CLI_REPL_WHY_CALL, &[("call", call), ("record", n.to_string())])
                ));
            }
        }
        // The session's values it was computed from, as they were then.
        if let Some(i) = init {
            for dep in names_read(&tree, i) {
                if seen.contains(&dep) {
                    continue;
                }
                let binder = self.entries[..k].iter().rposition(|e| {
                    let (t, _) = parse_script(&e.text);
                    top_level(&t).into_iter().any(|s| bound_names(&t, s).contains(&dep))
                });
                if let Some(j) = binder {
                    let v = self.value_after(k, &dep, host);
                    self.why_at(j, &dep, v, depth + 1, seen, host);
                }
            }
        }
        // The inputs that changed it since.
        for (j, e) in self.entries.iter().enumerate().skip(k + 1) {
            let (t, _) = parse_script(&e.text);
            if top_level(&t).into_iter().any(|s| changes(&t, s, name)) {
                host.stdout(&format!(
                    "{pad}  {}",
                    line(
                        msg::CLI_REPL_WHY_CHANGE,
                        &[("text", e.text.trim().to_string()), ("input", (j + 1).to_string())]
                    )
                ));
            }
        }
    }
}

/// Runs a session's program: what `earlier` logged replays from it, the
/// rest runs live on `host`. Returns the status, the run's own log, and
/// the divergence from `earlier` if the replay met one.
fn run(
    prog: &sisu_sir::Program,
    earlier: Option<&Log>,
    host: &mut dyn Host,
) -> (i32, Option<Log>, Option<String>) {
    let mut live = WithArgs { inner: host, args: Vec::new(), watch: Vec::new() };
    let mut resume = Resume::new(earlier, &mut live).without_stdin();
    let mut rec = Recorder::new(&mut resume, Vec::new());
    let report = sisu_interp::run_main_report(prog, &mut rec, &Options::default());
    let text = rec.finish(&Header::default(), report.status);
    drop(rec);
    (report.status, Log::parse(&text).ok(), resume.divergence())
}

/// `sisu repl [DIR | FILE]`.
pub(crate) fn repl(args: &[String], front: FrontEnd, host: &mut dyn Host) -> i32 {
    if let Some(bad) = args.iter().find(|a| a.starts_with("--")).or(args.get(1)) {
        host.stderr(&plain_error(
            msg::CLI_UNKNOWN_OPTION,
            &[("option", bad.clone()), ("command", "repl".into())],
        ));
        return crate::cli::EXIT_USAGE;
    }
    let cwd = std::env::current_dir().unwrap_or_default();
    // `sisu repl FILE`: the file is the session's first input.
    let loaded = match args.first() {
        Some(f) if f.ends_with(".sisu") => match std::fs::read_to_string(f) {
            Ok(t) => Some(split_preamble(f, &t)),
            Err(e) => {
                host.stderr(&plain_error(
                    msg::CLI_CANNOT_READ,
                    &[("path", f.clone()), ("reason", e.to_string())],
                ));
                return exit::ERROR;
            }
        },
        _ => None,
    };
    let located =
        if loaded.is_some() { None } else { crate::package::locate(args.first().map(String::as_str), &cwd) };
    let package = match located {
        Some(crate::package::Target::Package(dir)) => {
            let layout = crate::package::load_package(&dir, &cwd);
            match layout.packages.first() {
                Some(p) if !layout.has_errors() => Some((p.name.clone(), p.dir.clone())),
                _ => {
                    let mut map = SourceMap::new();
                    for (n, t) in &layout.files {
                        map.add(n.clone(), t.clone());
                    }
                    host.stderr(&sisu_diag::render::render_human(&map, &layout.diagnostics));
                    return exit::ERROR;
                }
            }
        }
        _ => None,
    };
    host.stderr(&line(msg::CLI_REPL_BANNER, &[]));
    if let Some((name, _)) = &package {
        host.stderr(&line(msg::CLI_REPL_LOADED, &[("name", name.clone())]));
    }
    let mut s = Session {
        preamble: "#!/usr/bin/env sisu\n".into(),
        entries: Vec::new(),
        logs: Vec::new(),
        package,
        cwd,
        checker: Checker::choose(front, host),
    };
    if let Some((preamble, first)) = loaded {
        s.preamble = preamble;
        s.evaluate(first, host);
    }
    let prompt = format!("{} ", text(msg::CLI_REPL_PROMPT, &[]));
    let more = format!("{} ", text(msg::CLI_REPL_MORE, &[]));
    loop {
        host.stdout(&prompt);
        let Some(first) = host.read_line() else {
            host.stdout("\n");
            return exit::OK;
        };
        let first = first.trim_end().to_string();
        let trimmed = first.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(cmd) = trimmed.strip_prefix(':') {
            let mut words = cmd.split_whitespace();
            match (words.next(), words.next()) {
                (Some("quit" | "q"), None) => return exit::OK,
                (Some("back"), n) => {
                    let n: usize = match n.map(str::parse) {
                        None => 1,
                        Some(Ok(n)) => n,
                        Some(Err(_)) => {
                            host.stderr(&line(msg::CLI_REPL_BAD_COMMAND, &[("command", cmd.to_string())]));
                            continue;
                        }
                    };
                    if n > s.entries.len() {
                        host.stderr(&line(
                            msg::CLI_REPL_BACK_TOO_FAR,
                            &[("count", s.entries.len().to_string())],
                        ));
                        continue;
                    }
                    let keep = s.entries.len() - n;
                    s.entries.truncate(keep);
                    s.logs.truncate(keep);
                    host.stdout(&line(
                        msg::CLI_REPL_BACK,
                        &[("forgotten", n.to_string()), ("count", keep.to_string())],
                    ));
                }
                (Some("why"), Some(name)) => s.why(name, host),
                _ => host.stderr(&line(msg::CLI_REPL_BAD_COMMAND, &[("command", cmd.to_string())])),
            }
            continue;
        }
        let mut input = first.clone();
        if needs_more(&input) {
            loop {
                host.stdout(&more);
                match host.read_line() {
                    Some(l) if !l.trim().is_empty() => {
                        input.push('\n');
                        input.push_str(l.trim_end());
                    }
                    _ => break,
                }
            }
        }
        s.evaluate(Entry::of(&input), host);
    }
}

/// `sisu ws [--run] FILE`.
pub(crate) fn worksheet(args: &[String], front: FrontEnd, host: &mut dyn Host) -> i32 {
    let mut real = false;
    let mut file = None;
    for a in args {
        match a.as_str() {
            "--run" => real = true,
            f if !f.starts_with("--") && file.is_none() => file = Some(f.to_string()),
            other => {
                host.stderr(&plain_error(
                    msg::CLI_UNKNOWN_OPTION,
                    &[("option", other.to_string()), ("command", "ws".into())],
                ));
                return crate::cli::EXIT_USAGE;
            }
        }
    }
    let Some(file) = file else {
        host.stderr(&plain_error(msg::CLI_MISSING_FILE, &[("command", "ws".into())]));
        return crate::cli::EXIT_USAGE;
    };
    let source = match std::fs::read_to_string(&file) {
        Ok(t) => t,
        Err(e) => {
            host.stderr(&plain_error(
                msg::CLI_CANNOT_READ,
                &[("path", file.clone()), ("reason", e.to_string())],
            ));
            return exit::ERROR;
        }
    };
    let cwd = std::env::current_dir().unwrap_or_default();
    let checker = Checker::choose(front, host);
    let sheet = match evaluate_sheet(&file, &source, &cwd, real, None, &checker, host) {
        Ok(s) => s,
        Err(e) => {
            host.stderr(&e);
            return exit::ERROR;
        }
    };
    let mut shown = String::new();
    for (i, l) in source.lines().enumerate() {
        let n = i + 1;
        shown.push_str(l);
        if let Some(vs) = sheet.values.get(&n) {
            shown.push_str("  ");
            shown.push_str(&text(msg::CLI_WS_VALUE, &[("value", vs.join(", "))]));
        }
        shown.push('\n');
        for o in sheet.outputs.get(&n).into_iter().flatten() {
            shown.push_str(&line(msg::CLI_WS_OUTPUT, &[("text", o.clone())]));
        }
    }
    host.stdout(&shown);
    if !sheet.errors.is_empty() {
        host.stderr(&sheet.errors);
    }
    sheet.status
}

/// A worksheet's run: the values of each top-level statement and what it
/// printed, by the last line of the statement in the file as written
/// (1-based), with the run's standard error and exit status. The language
/// server shows them as inline values (10, "Worksheets").
pub(crate) struct Sheet {
    pub values: std::collections::BTreeMap<usize, Vec<String>>,
    pub outputs: std::collections::BTreeMap<usize, Vec<String>>,
    pub errors: String,
    pub status: i32,
}

/// Runs a worksheet (Q-48) against a test's fake `System`, or the real one
/// when `real`, for at most `fuel` statements when given: its text must
/// check, or the diagnostics are the error.
pub(crate) fn evaluate_sheet(
    file: &str,
    source: &str,
    cwd: &std::path::Path,
    real: bool,
    fuel: Option<u64>,
    checker: &Checker,
    host: &mut dyn Host,
) -> Result<Sheet, String> {
    // A worksheet is a script; one without its `#!` line gets one.
    let (sheet, shift) = if source.starts_with("#!") {
        (source.to_string(), 0)
    } else {
        (format!("#!/usr/bin/env sisu\n{source}"), 1)
    };
    let compile =
        |text: &str, keep: &[&str], host: &mut dyn Host| checker.compile(file, text, cwd, keep, host);
    let show = |e: NotRun| match e {
        NotRun::Diagnostics(map, diags) => sisu_diag::render::render_human(&map, &diags),
        // Stage 1's report, of the worksheet with its `#!` line.
        NotRun::Message(m) if shift > 0 => relocate(&m, file, 2, usize::MAX - 1, file, 1),
        NotRun::Message(m) => m,
    };
    // The worksheet as written must check; its instrumented form then does.
    compile(&sheet, &[], host).map_err(show)?;
    let parse = sisu_parser::parse(FileId(0), &sheet);
    let tree = &parse.tree;
    let line_of = |pos: u32| sheet[..pos as usize].matches('\n').count() + 1;
    let is_stmt = |k: SyntaxKind| !Tree::is_item_kind(k) && !matches!(k, ATTRIBUTE | ERROR);
    // Each statement: its first and last line, and the instrumented text.
    let mut spans: Vec<(usize, usize)> = Vec::new();
    let mut out = String::new();
    let mut last = 0usize;
    for n in top_level(tree) {
        let k = tree.kind(n);
        if !is_stmt(k) {
            continue;
        }
        let r = tree.range(n);
        let (start, end) = (r.start as usize, r.end as usize);
        let body = sheet[start..end].trim_end();
        let first = line_of(r.start);
        let last_line = first + body.matches('\n').count();
        spans.push((first, last_line));
        out.push_str(&sheet[last..start]);
        out.push_str(&format!("__ws_at({first})\n"));
        if k == EXPR_STMT {
            out.push_str(&format!("__ws_show({first}, {body})\n"));
        } else {
            out.push_str(body);
            out.push('\n');
            for name in bound_names(tree, n) {
                out.push_str(&format!("__ws_show({first}, {name})\n"));
            }
        }
        last = start + body.len();
        // The line break the statement ended with is the one written.
        if sheet[last..].starts_with('\n') {
            last += 1;
        }
    }
    out.push_str(&sheet[last..]);
    out.push_str(WS_HELPERS);
    // Stage 1 keeps what the program's own functions reach: the fakes too.
    if matches!(checker, Checker::Stage1 { .. }) {
        out.push_str(WS_FAKE);
    }
    let prog = compile(&out, &["System.fake"], host).map_err(show)?;
    let mut cap = Captured { inner: &mut *host, out: String::new(), err: String::new() };
    let report = {
        let mut live = WithArgs { inner: &mut cap, args: Vec::new(), watch: Vec::new() };
        let options = Options { fuel, ..Options::default() };
        if real {
            sisu_interp::run_main_report(&prog, &mut live, &options)
        } else {
            sisu_interp::run_main_faked(&prog, &mut live, &options)
        }
    };
    let (printed, errors) = (cap.out, cap.err);
    // What each statement printed and its values, by its first line.
    let mut values: std::collections::BTreeMap<usize, Vec<String>> = Default::default();
    let mut outputs: std::collections::BTreeMap<usize, Vec<String>> = Default::default();
    let mut current = 0usize;
    for l in printed.lines() {
        if let Some(n) = l.strip_prefix(AT) {
            current = n.trim().parse().unwrap_or(0);
        } else if let Some(rest) = l.strip_prefix(VALUE) {
            let (n, v) = rest.split_once(' ').unwrap_or((rest, ""));
            values.entry(n.parse().unwrap_or(0)).or_default().push(v.to_string());
        } else {
            outputs.entry(current).or_default().push(l.to_string());
        }
    }
    // By each statement's last line, in the file as written.
    let mut sheet_out =
        Sheet { values: Default::default(), outputs: Default::default(), errors, status: report.status };
    for (first, end) in spans {
        let Some(at) = end.checked_sub(shift).filter(|n| *n > 0) else { continue };
        if let Some(vs) = values.remove(&first) {
            sheet_out.values.entry(at).or_default().extend(vs);
        }
        if let Some(os) = outputs.remove(&first) {
            sheet_out.outputs.entry(at).or_default().extend(os);
        }
    }
    Ok(sheet_out)
}

#[cfg(test)]
mod tests {
    /// Stage 1's report of a session's program is moved into the input:
    /// the place, the gutter's numbers and its width (decision 563).
    #[test]
    fn a_report_of_stage_1_is_moved_into_the_input() {
        let report = "error[E0300]: expected `str`, found `int`\n  --> <repl>:14:14\n   |\n14 | let y: str = 3\n   |              ^\n   |        --- expected because of this\n\nerror: elsewhere\n  --> <repl>:2:1\n  |\n2 | x\n  | ^\n";
        let moved = super::relocate(report, "<repl>", 14, 14, "<input>", 1);
        assert_eq!(
            moved,
            "error[E0300]: expected `str`, found `int`\n  --> <input>:1:14\n  |\n1 | let y: str = 3\n  |              ^\n  |        --- expected because of this\n\nerror: elsewhere\n  --> <repl>:2:1\n  |\n2 | x\n  | ^\n"
        );
    }
}
