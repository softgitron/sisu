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
//! The session's program, and a worksheet's, is built by stage 1 when it
//! serves them (`sisu1 0.8.0` and later, decisions 563 and 620): `sir
//! --repl SESSION INPUT --kept OUT` gives the SIR of the session with an
//! input after it and the session's program that keeps it, or the
//! diagnostics with the input's places drawn in it; `sir --worksheet
//! FILE` the worksheet's SIR with its statements marked. One run of stage
//! 1 for each input. `:why NAME` reads the inputs' trees from stage 1's
//! `parse --tree`; a stage 1 of 0.9.0 is told where the program stands
//! and what it is called (`--dir`, `--name`), and `:why` reads its `parse
//! --bindings` instead (decision 640). An older stage 1, or
//! `--front-end=rust`, leaves them to the Rust front end, which builds the
//! same programs.
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

/// How diagnostics name the session's program and an input.
const SESSION: &str = "<repl>";
const INPUT: &str = "<input>";

/// What the session's program adds after its inputs, for the Rust front
/// end: the function that shows an expression's value. Stage 1 adds its
/// own (RULES.md, R79).
const SHOW: &str = "\ndef __repl_show[T](value: T):\n    if comptime reflect.name_of(T) != \"()\":\n        print(\"{value:?}\")\n";

/// The worksheet's markers on standard output: a statement starts, and a
/// statement's value.
const AT: char = '\u{1}';
const VALUE: char = '\u{2}';
const WS_HELPERS: &str = "\ndef __ws_at(line: int):\n    print(\"\\u{1}{line}\")\n\ndef __ws_show[T](line: int, value: T):\n    if comptime reflect.name_of(T) != \"()\":\n        print(\"\\u{2}{line} {value:?}\")\n";

/// The `#!` line a session's program, and a worksheet without one, starts
/// with.
const SHEBANG: &str = "#!/usr/bin/env sisu\n";

fn line(id: MsgId, params: &[(&str, String)]) -> String {
    format!("{}\n", text(id, params))
}

/// What builds a session's program, or a worksheet's: stage 1 (decisions
/// 563 and 620), with the directory its inputs are written to, or the
/// Rust front end.
pub(crate) enum Checker {
    Rust,
    Stage1 {
        stage1: crate::front::Stage1,
        dir: std::path::PathBuf,
        /// Stage 1 takes `--dir`, `--name` and `parse --bindings`
        /// (`sisu1 0.9.0`, decision 640).
        options: bool,
        /// The process of stage 1 that serves the session's commands
        /// (`sir --repl-serve`, `sisu1 0.10.0`, decision 720), started at
        /// the first.
        server: std::cell::RefCell<Served>,
    },
}

/// Where a session's commands of stage 1 go: to a server not started yet,
/// to the one started, or to a process of their own each (a stage 1 older
/// than 0.10.0, `SISU_STAGE1_REPL_SERVE=0`, or a server that went away).
pub(crate) enum Served {
    Unstarted,
    Running(crate::front::Server),
    Off,
}

/// Where a session's program, or a worksheet, stands for stage 1 (`--dir`,
/// which its `embed` and its inline metadata's paths resolve in), and
/// what its diagnostics and its SIR call it (`--name`), when stage 1 takes
/// them (decision 640).
#[derive(Clone, Debug, Default)]
pub(crate) struct Place {
    pub dir: Option<std::path::PathBuf>,
    pub name: Option<String>,
}

impl Place {
    /// The options of stage 1 that say it.
    fn words(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(d) = &self.dir {
            out.push("--dir".to_string());
            out.push(d.display().to_string());
        }
        if let Some(n) = &self.name {
            out.push("--name".to_string());
            out.push(n.clone());
        }
        out
    }
}

/// A program built for a session, or why not: its SIR and, from stage 1,
/// the session's program that keeps the input; or the report to show.
type Built = Result<(sisu_sir::Program, String), String>;

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
        let options = stage1.serves_repl_options();
        let served = if options && stage1.serves_repl_server() { Served::Unstarted } else { Served::Off };
        Checker::Stage1 { stage1, dir, options, server: std::cell::RefCell::new(served) }
    }

    /// Runs the stage-1 command `words`: through the session's server when
    /// there is one, started at the first command, or as a process of its
    /// own. A server that cannot answer is given up for the session, and
    /// the command runs alone, which reports what went wrong.
    fn stage1_run(&self, words: &[String], host: &mut dyn Host) -> Result<crate::front::Ran, i32> {
        let Checker::Stage1 { stage1, server, .. } = self else { return Err(exit::ERROR) };
        {
            let mut served = server.borrow_mut();
            if matches!(*served, Served::Unstarted) {
                *served = stage1.serve().map_or(Served::Off, Served::Running);
            }
            if let Served::Running(s) = &mut *served {
                if let Some(ran) = s.ask(words) {
                    return Ok(ran);
                }
                if s.is_lost() {
                    *served = Served::Off;
                }
            }
        }
        stage1.run(words, crate::front::Backend::Unused, host)
    }

    /// Stage 1's `sir --repl SESSION INPUT --kept OUT` (RULES.md, R79): the
    /// session's program `session` with `input` after it, or with the file
    /// `load` loaded into it (`--load`): the SIR and OUT, or the
    /// diagnostics stage 1 drew, the input's in its text.
    fn repl(
        &self,
        session: &str,
        input: &str,
        load: Option<&str>,
        place: &Place,
        host: &mut dyn Host,
    ) -> Built {
        let Checker::Stage1 { dir, options, .. } = self else { return Err(String::new()) };
        let written = |name: &str, text: &str| -> Result<String, String> {
            let path = dir.join(name);
            std::fs::write(&path, text).map_err(|e| format!("{}\n", text_of_io(&path, &e)))?;
            Ok(path.display().to_string())
        };
        let session_file = written("session.sisu", session)?;
        let input_file = match load {
            Some(f) => f.to_string(),
            None => written("input.sisu", input)?,
        };
        let kept = dir.join("kept.sisu");
        let _ = std::fs::remove_file(&kept);
        let mut words: Vec<String> =
            ["sir", "--repl", &session_file, &input_file, "--kept"].iter().map(|w| w.to_string()).collect();
        words.push(kept.display().to_string());
        if load.is_some() {
            words.push("--load".into());
        }
        if *options {
            words.extend(place.words());
        }
        let ran = self.stage1_run(&words, host).map_err(|_| String::new())?;
        if ran.status != exit::OK {
            return Err(ran.stderr);
        }
        let prog = read_sir(&ran.stdout)?;
        let kept = std::fs::read_to_string(&kept).map_err(|e| format!("{}\n", text_of_io(&kept, &e)))?;
        Ok((prog, kept))
    }

    /// Stage 1's `sir --worksheet FILE`: the worksheet's SIR with its
    /// statements marked, or its diagnostics in the file as written.
    fn worksheet(&self, file: &str, place: &Place, host: &mut dyn Host) -> Result<sisu_sir::Program, String> {
        let Checker::Stage1 { options, .. } = self else { return Err(String::new()) };
        let mut words = vec!["sir".to_string(), "--worksheet".to_string(), file.to_string()];
        if *options {
            words.extend(place.words());
        }
        let ran = self.stage1_run(&words, host).map_err(|_| String::new())?;
        if ran.status != exit::OK {
            return Err(ran.stderr);
        }
        read_sir(&ran.stdout)
    }

    /// What `:why` reads of an input as the statements of a script: from
    /// stage 1's `parse --bindings` (decision 640), or from the tree of its
    /// `parse --tree` or of the Rust parser (decision 621).
    fn outline(&self, input: &str, host: &mut dyn Host) -> Option<Outline> {
        let text = format!("{SHEBANG}{input}\n");
        match self {
            Checker::Rust => {
                Syntax::read(text.clone(), &sisu_parser::parse(FileId(0), &text).tree.dump(false))
                    .map(|t| Outline::of(&t))
            }
            Checker::Stage1 { dir, options, .. } => {
                let path = dir.join("tree.sisu");
                std::fs::write(&path, &text).ok()?;
                let how = if *options { "--bindings" } else { "--tree" };
                let words = vec!["parse".to_string(), how.to_string(), path.display().to_string()];
                let printed = self.stage1_run(&words, host).ok()?.stdout;
                if *options {
                    Outline::read(text, &printed)
                } else {
                    Syntax::read(text, &printed).map(|t| Outline::of(&t))
                }
            }
        }
    }
}

impl Drop for Checker {
    fn drop(&mut self) {
        if let Checker::Stage1 { dir, .. } = self {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

fn read_sir(sir: &str) -> Result<sisu_sir::Program, String> {
    sisu_sir::read(sir)
        .map_err(|e| line(msg::SIR_PARSE, &[("detail", format!("line {}: {}", e.line, e.message))]))
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

/// A syntax tree as `sisu parse --tree` prints it, over the text parsed:
/// each node's kind and byte range, its tokens (those with text) beside
/// its nodes. What `:why` and the Rust front end's programs read of an
/// input.
#[derive(Clone, Debug)]
struct Syntax {
    text: String,
    nodes: Vec<SyntaxNode>,
}

#[derive(Clone, Debug)]
struct SyntaxNode {
    kind: String,
    start: usize,
    end: usize,
    token: bool,
    children: Vec<usize>,
}

/// The kinds of the items of a module, which a worksheet does not show.
const ITEM_KINDS: &[&str] = &[
    "FN_DEF",
    "STRUCT_DEF",
    "ENUM_DEF",
    "ERROR_DEF",
    "IMPL_DEF",
    "TRAIT_DEF",
    "IMPORT_ITEM",
    "FROM_IMPORT",
    "TEST_DEF",
    "SUITE_DEF",
    "BENCH_DEF",
    "CONST_DEF",
    "TYPE_ALIAS",
    "EXTERN_BLOCK",
];

impl Syntax {
    /// The tree `dump` prints of `text`: a line for each node and token, two
    /// spaces deeper than its parent, `KIND START..END`, a token's text
    /// after it.
    fn read(text: String, dump: &str) -> Option<Syntax> {
        let mut nodes: Vec<SyntaxNode> = Vec::new();
        let mut open: Vec<(usize, usize)> = Vec::new();
        for l in dump.lines() {
            let rest = l.trim_start_matches(' ');
            let depth = (l.len() - rest.len()) / 2;
            let mut words = rest.splitn(3, ' ');
            let kind = words.next()?.to_string();
            let (a, b) = words.next()?.split_once("..")?;
            let (start, end) = (a.parse().ok()?, b.parse().ok()?);
            let token = words.next().is_some();
            while open.last().is_some_and(|&(d, _)| d >= depth) {
                open.pop();
            }
            let id = nodes.len();
            if let Some(&(_, parent)) = open.last() {
                nodes[parent].children.push(id);
            } else if id != 0 {
                return None;
            }
            nodes.push(SyntaxNode { kind, start, end, token, children: Vec::new() });
            if !token {
                open.push((depth, id));
            }
        }
        (!nodes.is_empty()).then_some(Syntax { text, nodes })
    }

    fn kind(&self, n: usize) -> &str {
        &self.nodes[n].kind
    }

    /// The node's text.
    fn text_of(&self, n: usize) -> &str {
        self.text.get(self.nodes[n].start..self.nodes[n].end).unwrap_or("")
    }

    /// The node's child nodes, not its tokens.
    fn child_nodes(&self, n: usize) -> impl Iterator<Item = usize> + '_ {
        self.nodes[n].children.iter().copied().filter(|&c| !self.nodes[c].token)
    }

    fn first_token(&self, n: usize, kind: &str) -> Option<usize> {
        self.nodes[n].children.iter().copied().find(|&c| self.nodes[c].token && self.nodes[c].kind == kind)
    }

    /// The node and the nodes under it, in order.
    fn preorder(&self, n: usize) -> Vec<usize> {
        let mut out = vec![n];
        let mut i = 0;
        while i < out.len() {
            let at = out[i];
            let kids: Vec<usize> = self.child_nodes(at).collect();
            out.splice(i + 1..i + 1, kids);
            i += 1;
        }
        out
    }

    /// The top-level statements and items.
    fn top_level(&self) -> Vec<usize> {
        self.child_nodes(0).collect()
    }

    /// The names a `let` or `var` binds.
    fn bound_names(&self, stmt: usize) -> Vec<String> {
        if self.kind(stmt) != "LET_STMT" {
            return Vec::new();
        }
        let mut out = Vec::new();
        for c in self.child_nodes(stmt) {
            if matches!(self.kind(c), "IDENT_PAT" | "TUPLE_PAT") {
                for n in self.preorder(c) {
                    if self.kind(n) == "IDENT_PAT"
                        && let Some(t) = self.first_token(n, "IDENT")
                    {
                        out.push(self.text_of(t).to_string());
                    }
                }
            }
        }
        out
    }

    /// The initialiser of a `let`: its expression's node.
    fn initialiser(&self, stmt: usize) -> Option<usize> {
        let at = self.nodes[self.first_token(stmt, "EQ")?].end;
        self.child_nodes(stmt).find(|&c| self.nodes[c].start >= at)
    }

    /// The names an expression reads.
    fn names_read(&self, n: usize) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for c in self.preorder(n) {
            if self.kind(c) == "NAME_REF"
                && let Some(t) = self.first_token(c, "IDENT")
            {
                let name = self.text_of(t).to_string();
                if !out.contains(&name) {
                    out.push(name);
                }
            }
        }
        out
    }

    /// The names a statement may change: what it assigns to (or into), and
    /// what it calls a method on.
    fn changed(&self, stmt: usize) -> Vec<String> {
        let root_name = |n: usize| -> Option<String> {
            let mut cur = n;
            loop {
                match self.kind(cur) {
                    "NAME_REF" => return self.first_token(cur, "IDENT").map(|t| self.text_of(t).to_string()),
                    "FIELD_EXPR" | "INDEX_EXPR" => cur = self.child_nodes(cur).next()?,
                    _ => return None,
                }
            }
        };
        let mut out: Vec<String> = Vec::new();
        match self.kind(stmt) {
            "ASSIGN_STMT" => out.extend(self.child_nodes(stmt).next().and_then(root_name)),
            "EXPR_STMT" => {
                for c in self.preorder(stmt) {
                    if self.kind(c) != "CALL_EXPR" {
                        continue;
                    }
                    let receiver = self
                        .child_nodes(c)
                        .next()
                        .filter(|&f| self.kind(f) == "FIELD_EXPR")
                        .and_then(|f| self.child_nodes(f).next())
                        .and_then(root_name);
                    if let Some(r) = receiver
                        && !out.contains(&r)
                    {
                        out.push(r);
                    }
                }
            }
            _ => {}
        }
        out
    }
}

/// What `:why` reads of an input: for each top-level statement or item its
/// kind (`let`, `assign`, `expr`, `item` or `other`) and range, the names
/// it binds, its initialiser's range, the names that reads, and the names
/// it may change. Stage 1's `parse --bindings` prints it (decision 640);
/// for an older stage 1 and the Rust front end it is read off the tree.
#[derive(Clone, Debug, Default, PartialEq)]
struct Outline {
    text: String,
    stmts: Vec<Bindings>,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Bindings {
    kind: String,
    start: usize,
    end: usize,
    binds: Vec<String>,
    initialiser: Option<(usize, usize)>,
    reads: Vec<String>,
    changes: Vec<String>,
}

impl Outline {
    /// What `parse --bindings` prints of `text`: a line `KIND START END`
    /// for each top-level node, then indented lines `binds N...`,
    /// `initialiser START END`, `reads N...` and `changes N...`.
    fn read(text: String, printed: &str) -> Option<Outline> {
        let mut stmts: Vec<Bindings> = Vec::new();
        for l in printed.lines() {
            if l.trim().is_empty() {
                continue;
            }
            let indented = l.starts_with(' ');
            let mut words = l.split_whitespace();
            let head = words.next()?;
            let names = |w: std::str::SplitWhitespace| w.map(str::to_string).collect::<Vec<_>>();
            if !indented {
                let start = words.next()?.parse().ok()?;
                let end = words.next()?.parse().ok()?;
                stmts.push(Bindings { kind: head.to_string(), start, end, ..Bindings::default() });
                continue;
            }
            let b = stmts.last_mut()?;
            match head {
                "binds" => b.binds = names(words),
                "reads" => b.reads = names(words),
                "changes" => b.changes = names(words),
                "initialiser" => {
                    let start = words.next()?.parse().ok()?;
                    let end = words.next()?.parse().ok()?;
                    b.initialiser = Some((start, end));
                }
                _ => {}
            }
        }
        Some(Outline { text, stmts })
    }

    /// The same, read off an input's tree (decision 621).
    fn of(t: &Syntax) -> Outline {
        let stmts = t
            .top_level()
            .into_iter()
            .map(|s| {
                let kind = match t.kind(s) {
                    "LET_STMT" => "let",
                    "ASSIGN_STMT" => "assign",
                    "EXPR_STMT" => "expr",
                    k if ITEM_KINDS.contains(&k) => "item",
                    _ => "other",
                };
                let init = t.initialiser(s);
                Bindings {
                    kind: kind.to_string(),
                    start: t.nodes[s].start,
                    end: t.nodes[s].end,
                    binds: t.bound_names(s),
                    initialiser: init.map(|i| (t.nodes[i].start, t.nodes[i].end)),
                    reads: init.map(|i| t.names_read(i)).unwrap_or_default(),
                    changes: t.changed(s),
                }
            })
            .collect();
        Outline { text: t.text.clone(), stmts }
    }

    /// Whether a top-level statement binds `name`.
    fn binds(&self, name: &str) -> bool {
        self.binder(name).is_some()
    }

    /// The statement that binds `name`.
    fn binder(&self, name: &str) -> Option<&Bindings> {
        self.stmts.iter().find(|b| b.binds.iter().any(|n| n == name))
    }

    /// Whether a statement may change `name`.
    fn changes(&self, name: &str) -> bool {
        self.stmts.iter().any(|b| b.changes.iter().any(|n| n == name))
    }

    /// The text of an initialiser.
    fn text_of(&self, (start, end): (usize, usize)) -> &str {
        self.text.get(start..end).unwrap_or("")
    }
}

/// The Rust parser's tree of a text.
fn rust_syntax(text: &str) -> Option<Syntax> {
    Syntax::read(text.to_string(), &sisu_parser::parse(FileId(0), text).tree.dump(false))
}

/// One input the session kept.
#[derive(Clone, Debug)]
struct Entry {
    /// The input as typed (a loaded file's text after its preamble).
    text: String,
    /// The file loaded into the session, which stage 1 reads itself.
    load: Option<String>,
    /// For the Rust front end: the input as the session's program holds
    /// it (an expression is wrapped to be shown, so its text starts
    /// `prefix` bytes in), and how its diagnostics show it: the name, the
    /// text they point into, and where the input starts in that text.
    source: String,
    prefix: usize,
    shown: (String, String, usize),
    /// From stage 1: the session's program that keeps the input.
    kept: String,
    /// What `:why` reads of the input, read when it asks.
    outline: std::cell::OnceCell<Option<Outline>>,
}

impl Entry {
    /// An input as typed; `rust` when the Rust front end builds it, which
    /// the session wraps itself.
    fn of(input: &str, rust: bool) -> Entry {
        let shown = (INPUT.to_string(), input.trim_end().to_string(), 0);
        let mut entry = Entry {
            text: input.to_string(),
            load: None,
            source: format!("{}\n", input.trim_end()),
            prefix: 0,
            shown,
            kept: String::new(),
            outline: Default::default(),
        };
        let expression = rust
            && rust_syntax(&format!("{SHEBANG}{input}\n")).is_some_and(|t| {
                let stmts = t.top_level();
                stmts.len() == 1 && t.kind(stmts[0]) == "EXPR_STMT"
            });
        if expression {
            // As stage 1 wraps it (RULES.md, R79): the call's `)` on a line
            // of its own, after any comment the input ends with.
            let prefix = "__repl_show(";
            entry.source = format!("{prefix}{}\n)\n", input.trim_end());
            entry.prefix = prefix.len();
        }
        entry
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
        preamble.insert_str(0, SHEBANG);
    }
    let body = text[end..].to_string();
    let entry = Entry {
        source: if body.ends_with('\n') || body.is_empty() { body.clone() } else { format!("{body}\n") },
        text: body,
        load: Some(file.to_string()),
        prefix: 0,
        shown: (file.to_string(), text.to_string(), end),
        kept: String::new(),
        outline: Default::default(),
    };
    (preamble, entry)
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

/// The inline metadata that makes a session depend on the package in
/// `dir`, named `name`.
fn package_metadata(name: &str, dir: &std::path::Path) -> String {
    let path = dir.display().to_string().replace('\\', "\\\\").replace('"', "\\\"");
    format!("# /// script\n# [dependencies]\n# {name} = {{ path = \"{path}\" }}\n# ///\n")
}

/// A REPL session: the inputs it kept and the log of each evaluation.
struct Session {
    /// What leads the session's program: its `#!` line, the inline
    /// metadata of a package or of a file loaded into it.
    preamble: String,
    entries: Vec<Entry>,
    /// `logs[k]`: the log of the evaluation that kept `entries[k]`.
    logs: Vec<Log>,
    /// The package the session loads: its name and directory.
    package: Option<(String, std::path::PathBuf)>,
    cwd: std::path::PathBuf,
    checker: Checker,
    /// Where the session's program stands and what it is called, for a
    /// stage 1 that takes them (decision 640).
    place: Place,
}

impl Session {
    fn rust(&self) -> bool {
        matches!(self.checker, Checker::Rust)
    }

    /// The Rust front end's program of the session: its first `upto`
    /// inputs, then `extra`. Returns the text and where `extra` starts in
    /// it.
    fn program(&self, upto: usize, extra: Option<&str>) -> (String, u32) {
        let mut text = self.preamble.clone();
        if let Some((name, dir)) = &self.package {
            text.push_str(&package_metadata(name, dir));
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

    /// Stage 1's SESSION after the first `upto` inputs: what the input
    /// that kept the last of them left, or the package's metadata for a
    /// session that loads one.
    fn kept(&self, upto: usize) -> String {
        match upto.checked_sub(1).and_then(|k| self.entries.get(k)) {
            Some(e) => e.kept.clone(),
            None => match &self.package {
                Some((name, dir)) => format!("{SHEBANG}{}", package_metadata(name, dir)),
                None => String::new(),
            },
        }
    }

    /// The session's first `upto` inputs with `entry` after them, built.
    fn build(&self, upto: usize, entry: &Entry, host: &mut dyn Host) -> Built {
        if !self.rust() {
            return self.checker.repl(
                &self.kept(upto),
                &entry.text,
                entry.load.as_deref(),
                &self.place,
                host,
            );
        }
        let (program, at) = self.program(upto, Some(&entry.source));
        match rust_compile(SESSION, &program, &self.cwd, &[], host) {
            Ok(p) => Ok((p, String::new())),
            Err(e) => Err(self.explain(e, at, entry)),
        }
    }

    /// Why an input was not kept, as the input's own diagnostics where
    /// they point into it.
    fn explain(&self, why: NotRun, at: u32, entry: &Entry) -> String {
        match why {
            NotRun::Message(m) => m,
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
    fn evaluate(&mut self, mut entry: Entry, host: &mut dyn Host) {
        let n = self.entries.len();
        let (prog, kept) = match self.build(n, &entry, host) {
            Ok(p) => p,
            Err(shown) => {
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
                entry.kept = kept;
                self.entries.push(entry);
                self.logs.push(log);
            }
            _ => host.stderr(&line(msg::CLI_REPL_NOT_KEPT, &[])),
        }
    }

    /// The `Debug` rendering of `name` after the first `upto` inputs.
    fn value_after(&self, upto: usize, name: &str, host: &mut dyn Host) -> String {
        let shown = Entry::of(name, self.rust());
        let Ok((prog, _)) = self.build(upto, &shown, host) else { return "?".into() };
        let earlier = upto.checked_sub(1).and_then(|k| self.logs.get(k));
        let mut cap = Captured { inner: &mut *host, out: String::new(), err: String::new() };
        let _ = run(&prog, earlier, &mut cap);
        let out = cap.out.trim_end().to_string();
        if out.is_empty() { "?".into() } else { out }
    }

    /// What `:why` reads of the input `k` kept, read once.
    fn outline(&self, k: usize, host: &mut dyn Host) -> Option<&Outline> {
        let e = &self.entries[k];
        e.outline.get_or_init(|| self.checker.outline(&e.text, host)).as_ref()
    }

    /// The last of the first `upto` inputs that binds `name`.
    fn binder(&self, upto: usize, name: &str, host: &mut dyn Host) -> Option<usize> {
        (0..upto).rev().find(|&k| self.outline(k, host).is_some_and(|t| t.binds(name)))
    }

    /// `:why NAME`.
    fn why(&self, name: &str, host: &mut dyn Host) {
        let mut seen = Vec::new();
        let Some(k) = self.binder(self.entries.len(), name, host) else {
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
        let tree = self.outline(k, host).cloned();
        let stmt = tree.as_ref().and_then(|t| t.binder(name)).cloned();
        let init = stmt.as_ref().and_then(|s| s.initialiser);
        let expr = tree.as_ref().zip(init).map(|(t, i)| t.text_of(i).trim().to_string()).unwrap_or_default();
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
        if let Some(s) = stmt.as_ref().filter(|s| s.initialiser.is_some()) {
            for dep in s.reads.clone() {
                if seen.contains(&dep) {
                    continue;
                }
                if let Some(j) = self.binder(k, &dep, host) {
                    let v = self.value_after(k, &dep, host);
                    self.why_at(j, &dep, v, depth + 1, seen, host);
                }
            }
        }
        // The inputs that changed it since.
        for j in k + 1..self.entries.len() {
            let changed = self.outline(j, host).is_some_and(|t| t.changes(name));
            if changed {
                host.stdout(&format!(
                    "{pad}  {}",
                    line(
                        msg::CLI_REPL_WHY_CHANGE,
                        &[("text", self.entries[j].text.trim().to_string()), ("input", (j + 1).to_string())]
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
    // `--echo` writes each input after its prompt, so that a session read
    // from a pipe is captured as a terminal shows it (decision 685).
    let echo = args.iter().any(|a| a == "--echo");
    let args: Vec<String> = args.iter().filter(|a| *a != "--echo").cloned().collect();
    let args = args.as_slice();
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
    // A session stands in the working directory, and one that loads a
    // file is called by it (decision 640).
    let place = Place { dir: Some(cwd.clone()), name: loaded.as_ref().and_then(|(_, e)| e.load.clone()) };
    let mut s = Session {
        preamble: SHEBANG.into(),
        entries: Vec::new(),
        logs: Vec::new(),
        package,
        cwd,
        checker: Checker::choose(front, host),
        place,
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
        if echo {
            host.stdout(&format!("{first}\n"));
        }
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
                let next = host.read_line();
                if echo {
                    host.stdout(&format!("{}\n", next.as_deref().unwrap_or_default().trim_end()));
                }
                match next {
                    Some(l) if !l.trim().is_empty() => {
                        input.push('\n');
                        input.push_str(l.trim_end());
                    }
                    _ => break,
                }
            }
        }
        let rust = s.rust();
        s.evaluate(Entry::of(&input, rust), host);
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
/// check, or the diagnostics are the error. Stage 1 marks the statements
/// (`sir --worksheet`, RULES.md R79) by the line each ends on in the file
/// as written; for the Rust front end the driver marks them by their
/// first line in the program, then moves each to its last in the file.
pub(crate) fn evaluate_sheet(
    file: &str,
    source: &str,
    cwd: &std::path::Path,
    real: bool,
    fuel: Option<u64>,
    checker: &Checker,
    host: &mut dyn Host,
) -> Result<Sheet, String> {
    let (prog, spans) = match checker {
        Checker::Stage1 { .. } => {
            // A worksheet stands in its file's directory (decision 640).
            let dir = std::path::Path::new(file).parent().map(|d| cwd.join(d));
            (checker.worksheet(file, &Place { dir, name: None }, host)?, None)
        }
        Checker::Rust => {
            let (prog, spans) = rust_sheet(file, source, cwd, host)?;
            (prog, Some(spans))
        }
    };
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
    // What each statement printed and its values, by its marker's line.
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
    let Some((spans, shift)) = spans else {
        return Ok(Sheet { values, outputs, errors, status: report.status });
    };
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

/// Each statement's first and last line in the Rust front end's worksheet
/// program, and the lines its `#!` line added.
type Marks = (Vec<(usize, usize)>, usize);

/// The Rust front end's worksheet: the program with each statement marked
/// by its first line, and each statement's first and last line in it,
/// with the lines its `#!` line added (0 or 1).
fn rust_sheet(
    file: &str,
    source: &str,
    cwd: &std::path::Path,
    host: &mut dyn Host,
) -> Result<(sisu_sir::Program, Marks), String> {
    // A worksheet is a script; one without its `#!` line gets one.
    let (sheet, shift) =
        if source.starts_with("#!") { (source.to_string(), 0) } else { (format!("{SHEBANG}{source}"), 1) };
    let show = |e: NotRun| match e {
        NotRun::Diagnostics(map, diags) => sisu_diag::render::render_human(&map, &diags),
        NotRun::Message(m) => m,
    };
    // The worksheet as written must check; its instrumented form then does.
    rust_compile(file, &sheet, cwd, &[], host).map_err(show)?;
    let tree = rust_syntax(&sheet).ok_or_else(String::new)?;
    let line_of = |pos: usize| sheet[..pos].matches('\n').count() + 1;
    let is_stmt = |k: &str| !ITEM_KINDS.contains(&k) && !matches!(k, "ATTRIBUTE" | "ERROR");
    // Each statement: its first and last line, and the instrumented text.
    let mut spans: Vec<(usize, usize)> = Vec::new();
    let mut out = String::new();
    let mut last = 0usize;
    for n in tree.top_level() {
        if !is_stmt(tree.kind(n)) {
            continue;
        }
        let (start, end) = (tree.nodes[n].start, tree.nodes[n].end);
        let body = sheet[start..end].trim_end();
        let first = line_of(start);
        let last_line = first + body.matches('\n').count();
        spans.push((first, last_line));
        out.push_str(&sheet[last..start]);
        out.push_str(&format!("__ws_at({first})\n"));
        if tree.kind(n) == "EXPR_STMT" {
            out.push_str(&format!("__ws_show({first}, {body})\n"));
        } else {
            out.push_str(body);
            out.push('\n');
            for name in tree.bound_names(n) {
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
    let prog = rust_compile(file, &out, cwd, &["System.fake"], host).map_err(show)?;
    Ok((prog, (spans, shift)))
}

#[cfg(test)]
mod tests {
    /// The tree `sisu parse --tree` prints is read back: its nodes, their
    /// tokens, and what `:why` asks of them (decision 620).
    #[test]
    fn a_printed_tree_is_read_back() {
        let text = "#!/usr/bin/env sisu\nlet (a, b) = (f(x), 2)\nseen.push(a)\n".to_string();
        let dump = sisu_parser::parse(super::FileId(0), &text).tree.dump(false);
        let t = super::Syntax::read(text, &dump).expect("a tree");
        let stmts = t.top_level();
        assert_eq!(stmts.len(), 2);
        assert_eq!(t.bound_names(stmts[0]), vec!["a".to_string(), "b".to_string()]);
        let init = t.initialiser(stmts[0]).expect("an initialiser");
        assert_eq!(t.text_of(init), "(f(x), 2)");
        assert_eq!(t.names_read(init), vec!["f".to_string(), "x".to_string()]);
        assert_eq!(t.changed(stmts[1]), vec!["seen".to_string()]);
        assert!(super::Syntax::read(String::new(), "A 0..1\nB 0..1\n").is_none());
    }

    /// What `parse --bindings` prints is read as the tree gives it (decision
    /// 640): the same outline either way.
    #[test]
    fn printed_bindings_are_read_as_the_tree_gives_them() {
        let text =
            "#!/usr/bin/env sisu\nlet (a, b) = (f(x), 2)\nseen.push(a)\ndef g():\n    pass\n".to_string();
        let dump = sisu_parser::parse(super::FileId(0), &text).tree.dump(false);
        let tree = super::Syntax::read(text.clone(), &dump).expect("a tree");
        let from_tree = super::Outline::of(&tree);
        let printed = "let 20 43\n  binds a b\n  initialiser 33 42\n  reads f x\n  changes\n\
                       expr 43 55\n  binds\n  reads\n  changes seen\nitem 56 72\n";
        let read = super::Outline::read(text, printed).expect("an outline");
        assert_eq!(read.stmts.len(), 3);
        assert_eq!(read.stmts[0], from_tree.stmts[0]);
        assert_eq!(read.stmts[1].changes, from_tree.stmts[1].changes);
        assert_eq!(read.stmts[2].kind, from_tree.stmts[2].kind);
        assert_eq!(read.text_of(read.stmts[0].initialiser.expect("one")), "(f(x), 2)");
        assert!(read.binds("b") && !read.binds("seen"));
        assert!(read.changes("seen") && !read.changes("a"));
    }
}
