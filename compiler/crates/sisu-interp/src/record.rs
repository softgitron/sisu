//! The capability log (N-04; 10, "Debugging and profiling"): `sisu run
//! --record FILE` runs a program on a [`Recorder`], which passes every call
//! of the host contract through to the real host and writes the call and
//! its result to the log, in order; `sisu replay FILE` runs it again on a
//! [`Replayer`], which answers each call from the log instead of the world.
//! In the interpreter every source of nondeterminism is a host call or the
//! seeded scheduler, whose seed the log's header holds, so a replay makes
//! the same calls in the same order and prints the same output. The first
//! call or output that differs from the log is a divergence: the replay
//! reports both and fails.
//!
//! # The format, version 1
//!
//! Text, one record per line, each a keyword and its fields separated by
//! spaces. A field is a quoted string (quoted as SIR quotes one), an
//! integer, or a word. The header comes first:
//!
//! ```text
//! sisu-log 1
//! toolchain "0.0.1"
//! program "main.sisu" 3f2a9c0d1e2b4a59
//! source "def main():\n    print(1)\n"
//! seed 0
//! hash-key 0
//! contracts all
//! ```
//!
//! `program` is the file's name and the content hash of its text (FNV-1a,
//! 64 bits, in hex), `source` the text itself, so a log replays without
//! the file; `seed` the scheduler's; `hash-key` the per-process key of
//! hashing (always 0 in stage 0, whose hash is not keyed per process);
//! `contracts` the level the run checked (07). A program of several
//! modules (09) adds, for each file besides the root module's, `file NAME
//! TEXT`, and then `layout TEXT`, its modules and packages as the driver
//! writes them, so it too replays without its files. Then, in order:
//!
//! - `call NAME FIELDS... -> RESULT...`: a call of the host contract and
//!   its result (`call env_var "HOME" -> some "/root"`, `call read_file
//!   "a.txt" -> err not-found`). Data the program wrote is logged by its
//!   content hash (`#9f3a...`), since a replay needs only the result.
//! - `out TEXT` and `err TEXT`: what the program printed.
//! - `redacted CAPABILITY`: a call whose result was left out (`--redact`):
//!   a replay that reaches one stops there, as a divergence.
//! - `edit POLL FILE TEXT`: under `sisu run --watch` (whose log has a
//!   `watch` line in its header), a save of one of the program's files
//!   that the run saw at its POLL-th poll (1-based; the interpreter polls
//!   at every task start and `live.boundary()`), with the file's new text.
//!   A replay applies it at the same poll, so the same epochs are
//!   published, the same rejected saves reported and the same restarts
//!   made.
//! - `exit STATUS`: the last record.

use crate::host::{Host, IoErrorKind, NetFail, Poll};
use std::cell::{Cell, RefCell};

/// The format's version, the second word of a log's first line.
pub const VERSION: u32 = 1;

/// FNV-1a over bytes, 64 bits: the content hash of a program's text and
/// of the data a program writes. Stable across machines and toolchains.
pub fn content_hash(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in data {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn q(s: &str) -> String {
    sisu_sir::write::quote(s)
}

fn hex(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok()).collect()
}

fn io_word(k: &IoErrorKind) -> String {
    match k {
        IoErrorKind::NotFound => "not-found".into(),
        IoErrorKind::PermissionDenied => "permission-denied".into(),
        IoErrorKind::AlreadyExists => "already-exists".into(),
        IoErrorKind::InvalidData => "invalid-data".into(),
        IoErrorKind::Interrupted => "interrupted".into(),
        IoErrorKind::Other(m) => format!("other {}", q(m)),
    }
}

fn net_word(k: &NetFail) -> String {
    match k {
        NetFail::Refused => "refused".into(),
        NetFail::NotAllowed => "not-allowed".into(),
        NetFail::Closed => "closed".into(),
        NetFail::Other(m) => format!("other {}", q(m)),
    }
}

/// The header of a log: what a replay needs besides the records.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Header {
    pub toolchain: String,
    pub file: String,
    pub hash: u64,
    pub source: String,
    pub seed: u64,
    pub hash_key: u64,
    pub contracts: String,
    /// For a program of several modules (09): the other files' names and
    /// texts, in the order of its layout.
    pub files: Vec<(String, String)>,
    /// And its layout (modules and packages), as the driver writes it.
    pub layout: Option<String>,
    /// The run was under `--watch`: its saves are `edit` records.
    pub watch: bool,
    /// `source` is the program's serialised SIR, which a replay runs
    /// without a front end: the run's front end was stage 1 (decision 301).
    pub sir: bool,
}

impl Header {
    pub fn write(&self) -> String {
        let mut out = format!(
            "sisu-log {VERSION}\ntoolchain {}\nprogram {} {:016x}\nsource {}\nseed {}\nhash-key {}\ncontracts {}\n",
            q(&self.toolchain),
            q(&self.file),
            self.hash,
            q(&self.source),
            self.seed,
            self.hash_key,
            self.contracts
        );
        for (name, text) in &self.files {
            out.push_str(&format!("file {} {}\n", q(name), q(text)));
        }
        if let Some(l) = &self.layout {
            out.push_str(&format!("layout {}\n", q(l)));
        }
        if self.watch {
            out.push_str("watch\n");
        }
        if self.sir {
            out.push_str("sir\n");
        }
        out
    }
}

/// Wraps a host and writes every call through it to a log.
pub struct Recorder<'h> {
    inner: &'h mut dyn Host,
    lines: RefCell<Vec<String>>,
    /// The capabilities whose results are left out of the log.
    redact: Vec<String>,
    /// The polls for saves so far (`--watch`).
    polls: u64,
}

impl<'h> Recorder<'h> {
    pub fn new(inner: &'h mut dyn Host, redact: Vec<String>) -> Recorder<'h> {
        Recorder { inner, lines: RefCell::new(Vec::new()), redact, polls: 0 }
    }

    /// The records written so far, one per line, and the last one.
    pub fn finish(&self, header: &Header, status: i32) -> String {
        let mut out = header.write();
        for l in self.lines.borrow().iter() {
            out.push_str(l);
            out.push('\n');
        }
        out.push_str(&format!("exit {status}\n"));
        out
    }

    fn log(&self, capability: &str, call: String, result: String) {
        if self.redact.iter().any(|r| r == capability) {
            self.lines.borrow_mut().push(format!("redacted {capability}"));
        } else if result.is_empty() {
            self.lines.borrow_mut().push(format!("call {call}"));
        } else {
            self.lines.borrow_mut().push(format!("call {call} -> {result}"));
        }
    }
}

fn unit(r: &Result<(), IoErrorKind>) -> String {
    match r {
        Ok(()) => "ok".into(),
        Err(k) => format!("err {}", io_word(k)),
    }
}

impl Host for Recorder<'_> {
    fn stdout(&mut self, text: &str) {
        self.lines.borrow_mut().push(format!("out {}", q(text)));
        self.inner.stdout(text)
    }
    fn stderr(&mut self, text: &str) {
        self.lines.borrow_mut().push(format!("err {}", q(text)));
        self.inner.stderr(text)
    }
    fn args(&self) -> Vec<String> {
        let a = self.inner.args();
        let shown: Vec<String> = a.iter().map(|x| q(x)).collect();
        self.log("env", "args".into(), format!("{} {}", a.len(), shown.join(" ")).trim_end().to_string());
        a
    }
    fn env_var(&self, name: &str) -> Option<String> {
        let v = self.inner.env_var(name);
        let r = match &v {
            Some(x) => format!("some {}", q(x)),
            None => "none".into(),
        };
        self.log("env", format!("env_var {}", q(name)), r);
        v
    }
    fn now(&self) -> i64 {
        let v = self.inner.now();
        self.log("clock", "now".into(), v.to_string());
        v
    }
    fn now_nanos(&self) -> i128 {
        let v = self.inner.now_nanos();
        self.log("clock", "now_nanos".into(), v.to_string());
        v
    }
    fn virtual_time(&self) -> bool {
        let v = self.inner.virtual_time();
        self.log("clock", "virtual_time".into(), v.to_string());
        v
    }
    fn sleep_nanos(&mut self, nanos: u64) {
        self.log("clock", format!("sleep_nanos {nanos}"), String::new());
        self.inner.sleep_nanos(nanos)
    }
    fn read_file(&mut self, path: &str) -> Result<String, IoErrorKind> {
        let v = self.inner.read_file(path);
        let r = match &v {
            Ok(t) => format!("ok {}", q(t)),
            Err(k) => format!("err {}", io_word(k)),
        };
        self.log("fs", format!("read_file {}", q(path)), r);
        v
    }
    fn write_file(&mut self, path: &str, text: &str) -> Result<(), IoErrorKind> {
        let v = self.inner.write_file(path, text);
        self.log("fs", format!("write_file {} #{:016x}", q(path), content_hash(text.as_bytes())), unit(&v));
        v
    }
    fn read_bytes(&mut self, path: &str) -> Result<Vec<u8>, IoErrorKind> {
        let v = self.inner.read_bytes(path);
        let r = match &v {
            Ok(b) => format!("ok {}", q(&hex(b))),
            Err(k) => format!("err {}", io_word(k)),
        };
        self.log("fs", format!("read_bytes {}", q(path)), r);
        v
    }
    fn write_bytes(&mut self, path: &str, data: &[u8]) -> Result<(), IoErrorKind> {
        let v = self.inner.write_bytes(path, data);
        self.log("fs", format!("write_bytes {} #{:016x}", q(path), content_hash(data)), unit(&v));
        v
    }
    fn exists(&self, path: &str) -> bool {
        let v = self.inner.exists(path);
        self.log("fs", format!("exists {}", q(path)), v.to_string());
        v
    }
    fn metadata(&self, path: &str) -> Result<(i64, i64, bool), IoErrorKind> {
        let v = self.inner.metadata(path);
        let r = match &v {
            Ok((len, modified, dir)) => format!("ok {len} {modified} {dir}"),
            Err(k) => format!("err {}", io_word(k)),
        };
        self.log("fs", format!("metadata {}", q(path)), r);
        v
    }
    fn remove_file(&mut self, path: &str) -> Result<(), IoErrorKind> {
        let v = self.inner.remove_file(path);
        self.log("fs", format!("remove_file {}", q(path)), unit(&v));
        v
    }
    fn list_dir(&self, path: &str) -> Result<Vec<String>, IoErrorKind> {
        let v = self.inner.list_dir(path);
        let r = match &v {
            Ok(names) => {
                let shown: Vec<String> = names.iter().map(|n| q(n)).collect();
                format!("ok {} {}", names.len(), shown.join(" ")).trim_end().to_string()
            }
            Err(k) => format!("err {}", io_word(k)),
        };
        self.log("fs", format!("list_dir {}", q(path)), r);
        v
    }
    fn create_dir(&mut self, path: &str) -> Result<(), IoErrorKind> {
        let v = self.inner.create_dir(path);
        self.log("fs", format!("create_dir {}", q(path)), unit(&v));
        v
    }
    fn sync_path(&mut self, path: &str) -> Result<(), IoErrorKind> {
        let v = self.inner.sync_path(path);
        self.log("fs", format!("sync_path {}", q(path)), unit(&v));
        v
    }
    fn random_bits(&mut self) -> u64 {
        let v = self.inner.random_bits();
        self.log("rand", "random_bits".into(), v.to_string());
        v
    }
    fn run_process(&mut self, program: &str, args: &[String]) -> Result<(i64, String, String), IoErrorKind> {
        let v = self.inner.run_process(program, args);
        let r = match &v {
            Ok((status, out, err)) => format!("ok {status} {} {}", q(out), q(err)),
            Err(k) => format!("err {}", io_word(k)),
        };
        let shown: Vec<String> = args.iter().map(|x| q(x)).collect();
        self.log(
            "proc",
            format!("run_process {} {} {}", q(program), args.len(), shown.join(" ")).trim_end().into(),
            r,
        );
        v
    }
    fn tcp_connect(&mut self, host: &str, port: u16) -> Result<u64, NetFail> {
        let v = self.inner.tcp_connect(host, port);
        let r = match &v {
            Ok(h) => format!("ok {h}"),
            Err(k) => format!("err {}", net_word(k)),
        };
        self.log("net", format!("tcp_connect {} {port}", q(host)), r);
        v
    }
    fn tcp_listen(&mut self, host: &str, port: u16) -> Result<(u64, u16), NetFail> {
        let v = self.inner.tcp_listen(host, port);
        let r = match &v {
            Ok((h, p)) => format!("ok {h} {p}"),
            Err(k) => format!("err {}", net_word(k)),
        };
        self.log("net", format!("tcp_listen {} {port}", q(host)), r);
        v
    }
    fn tcp_accept(&mut self, listener: u64) -> Result<Poll<(u64, String)>, NetFail> {
        let v = self.inner.tcp_accept(listener);
        let r = match &v {
            Ok(Poll::Ready((h, peer))) => format!("ok ready {h} {}", q(peer)),
            Ok(Poll::Wait) => "ok wait".into(),
            Err(k) => format!("err {}", net_word(k)),
        };
        self.log("net", format!("tcp_accept {listener}"), r);
        v
    }
    fn tcp_read(&mut self, conn: u64, max: usize) -> Result<Poll<Vec<u8>>, NetFail> {
        let v = self.inner.tcp_read(conn, max);
        let r = match &v {
            Ok(Poll::Ready(b)) => format!("ok ready {}", q(&hex(b))),
            Ok(Poll::Wait) => "ok wait".into(),
            Err(k) => format!("err {}", net_word(k)),
        };
        self.log("net", format!("tcp_read {conn} {max}"), r);
        v
    }
    fn tcp_write(&mut self, conn: u64, data: &[u8]) -> Result<Poll<usize>, NetFail> {
        let v = self.inner.tcp_write(conn, data);
        let r = match &v {
            Ok(Poll::Ready(n)) => format!("ok ready {n}"),
            Ok(Poll::Wait) => "ok wait".into(),
            Err(k) => format!("err {}", net_word(k)),
        };
        self.log("net", format!("tcp_write {conn} #{:016x}", content_hash(data)), r);
        v
    }
    fn tcp_close(&mut self, handle: u64) {
        self.log("net", format!("tcp_close {handle}"), String::new());
        self.inner.tcp_close(handle)
    }
    fn read_line(&mut self) -> Option<String> {
        let v = self.inner.read_line();
        let r = match &v {
            Some(l) => format!("some {}", q(l)),
            None => "none".into(),
        };
        self.log("stdin", "read_line".into(), r);
        v
    }
    fn poll_edits(&mut self) -> Vec<(String, String)> {
        self.polls += 1;
        let v = self.inner.poll_edits();
        for (file, text) in &v {
            self.lines.borrow_mut().push(format!("edit {} {} {}", self.polls, q(file), q(text)));
        }
        v
    }
}

/// One field of a record.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Field {
    Str(String),
    Word(String),
}

impl Field {
    fn text(&self) -> &str {
        match self {
            Field::Str(s) | Field::Word(s) => s,
        }
    }
}

/// The fields of one line: quoted strings and words.
fn fields(line: &str) -> Result<Vec<Field>, String> {
    let mut out = Vec::new();
    let mut chars = line.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c == ' ' {
            chars.next();
            continue;
        }
        if c == '"' {
            chars.next();
            let mut s = String::new();
            loop {
                match chars.next() {
                    None => return Err("an unterminated string".into()),
                    Some('"') => break,
                    Some('\\') => match chars.next() {
                        Some('n') => s.push('\n'),
                        Some('t') => s.push('\t'),
                        Some('r') => s.push('\r'),
                        Some('0') => s.push('\0'),
                        Some('\\') => s.push('\\'),
                        Some('"') => s.push('"'),
                        Some('u') => {
                            if chars.next() != Some('{') {
                                return Err("a bad escape".into());
                            }
                            let mut h = String::new();
                            for d in chars.by_ref() {
                                if d == '}' {
                                    break;
                                }
                                h.push(d);
                            }
                            let ch = Some(h)
                                .filter(|h| h.bytes().all(|b| b.is_ascii_hexdigit()))
                                .and_then(|h| u32::from_str_radix(&h, 16).ok())
                                .and_then(char::from_u32);
                            s.push(ch.ok_or("a bad escape")?);
                        }
                        _ => return Err("a bad escape".into()),
                    },
                    Some(ch) => s.push(ch),
                }
            }
            out.push(Field::Str(s));
        } else {
            let mut w = String::new();
            while let Some(&ch) = chars.peek() {
                if ch == ' ' {
                    break;
                }
                w.push(ch);
                chars.next();
            }
            out.push(Field::Word(w));
        }
    }
    Ok(out)
}

/// A record after the header.
#[derive(Clone, Debug)]
enum Record {
    /// The call as written (its name and fields), and its result's fields.
    Call {
        call: String,
        result: Vec<Field>,
    },
    Out(String),
    Err(String),
    Redacted(String),
    /// A save seen at a poll: the poll's number, the file and its text.
    Edit(u64, String, String),
    Exit(i32),
}

/// A log read back: its header and records.
#[derive(Clone, Debug)]
pub struct Log {
    pub header: Header,
    records: Vec<Record>,
}

impl Log {
    pub fn parse(text: &str) -> Result<Log, String> {
        let mut lines = text.lines().enumerate();
        let mut next = |what: &str| -> Result<(usize, Vec<Field>), String> {
            let (n, l) = lines.next().ok_or_else(|| format!("the log ends before its `{what}` line"))?;
            let f = fields(l).map_err(|e| format!("line {}: {e}", n + 1))?;
            if f.first().map(Field::text) != Some(what) {
                return Err(format!("line {}: expected `{what}`", n + 1));
            }
            Ok((n, f))
        };
        let (_, v) = next("sisu-log")?;
        let version = v.get(1).map(Field::text).unwrap_or("");
        if version != VERSION.to_string() {
            return Err(format!("this is a log of version {version}; stage 0 reads version {VERSION}"));
        }
        let str_at = |f: &[Field], i: usize| f.get(i).map(|x| x.text().to_string()).unwrap_or_default();
        let (_, t) = next("toolchain")?;
        let (_, p) = next("program")?;
        let (_, s) = next("source")?;
        let (_, sd) = next("seed")?;
        let (_, hk) = next("hash-key")?;
        let (_, c) = next("contracts")?;
        let header = Header {
            toolchain: str_at(&t, 1),
            file: str_at(&p, 1),
            hash: u64::from_str_radix(&str_at(&p, 2), 16).map_err(|_| "a bad content hash".to_string())?,
            source: str_at(&s, 1),
            seed: str_at(&sd, 1).parse().map_err(|_| "a bad seed".to_string())?,
            hash_key: str_at(&hk, 1).parse().map_err(|_| "a bad hash key".to_string())?,
            contracts: str_at(&c, 1),
            files: Vec::new(),
            layout: None,
            watch: false,
            sir: false,
        };
        let mut header = header;
        let mut records = Vec::new();
        for (n, l) in lines {
            let f = fields(l).map_err(|e| format!("line {}: {e}", n + 1))?;
            let Some(kind) = f.first() else { continue };
            // The files and layout of a program of several modules belong
            // to the header.
            match kind.text() {
                "file" if records.is_empty() => {
                    header.files.push((str_at(&f, 1), str_at(&f, 2)));
                    continue;
                }
                "layout" if records.is_empty() => {
                    header.layout = Some(str_at(&f, 1));
                    continue;
                }
                "watch" if records.is_empty() => {
                    header.watch = true;
                    continue;
                }
                "sir" if records.is_empty() => {
                    header.sir = true;
                    continue;
                }
                _ => {}
            }
            let rec = match kind.text() {
                "call" => {
                    let arrow = f.iter().position(|x| *x == Field::Word("->".into()));
                    let (call, result) = match arrow {
                        Some(a) => (&f[1..a], f[a + 1..].to_vec()),
                        None => (&f[1..], Vec::new()),
                    };
                    Record::Call { call: render(call), result }
                }
                "out" => Record::Out(str_at(&f, 1)),
                "err" => Record::Err(str_at(&f, 1)),
                "redacted" => Record::Redacted(str_at(&f, 1)),
                "edit" => Record::Edit(
                    str_at(&f, 1).parse().map_err(|_| format!("line {}: a bad poll number", n + 1))?,
                    str_at(&f, 2),
                    str_at(&f, 3),
                ),
                "exit" => {
                    Record::Exit(str_at(&f, 1).parse().map_err(|_| format!("line {}: a bad status", n + 1))?)
                }
                other => return Err(format!("line {}: an unknown record `{other}`", n + 1)),
            };
            records.push(rec);
        }
        Ok(Log { header, records })
    }

    /// How many records the log has after its header.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// The calls of the host contract from record `from` on, each with
    /// its number (1-based) and as the log writes it (`read_file "a.txt"
    /// -> ok "..."`).
    pub fn calls_from(&self, from: usize) -> Vec<(usize, String)> {
        self.records
            .iter()
            .enumerate()
            .skip(from)
            .filter_map(|(i, r)| match r {
                Record::Call { call, result } if result.is_empty() => Some((i + 1, call.clone())),
                Record::Call { call, result } => Some((i + 1, format!("{call} -> {}", render(result)))),
                _ => None,
            })
            .collect()
    }

    /// The status the recorded run exited with.
    pub fn status(&self) -> Option<i32> {
        self.records.iter().rev().find_map(|r| match r {
            Record::Exit(s) => Some(*s),
            _ => None,
        })
    }
}

/// A call's fields as the log writes them.
fn render(fields: &[Field]) -> String {
    fields
        .iter()
        .map(|f| match f {
            Field::Str(s) => q(s),
            Field::Word(w) => w.clone(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Answers each host call from a log, printing what the program prints to
/// `out`; the first call or output unlike the log's is the divergence.
pub struct Replayer<'h> {
    out: &'h mut dyn Host,
    records: Vec<Record>,
    pos: Cell<usize>,
    diverged: RefCell<Option<String>>,
    /// The polls for saves so far (`--watch`).
    polls: u64,
}

impl<'h> Replayer<'h> {
    pub fn new(log: &Log, out: &'h mut dyn Host) -> Replayer<'h> {
        let records = log.records.iter().filter(|r| !matches!(r, Record::Exit(_))).cloned().collect();
        Replayer { out, records, pos: Cell::new(0), diverged: RefCell::new(None), polls: 0 }
    }

    /// The first divergence, or the records the run never reached.
    pub fn divergence(&self) -> Option<String> {
        if let Some(d) = self.diverged.borrow().clone() {
            return Some(d);
        }
        let pos = self.pos.get();
        self.records
            .get(pos)
            .map(|r| format!("the run ended before record {} of the log: {}", pos + 1, show(r)))
    }

    fn diverge(&self, why: String) {
        let mut d = self.diverged.borrow_mut();
        if d.is_none() {
            *d = Some(why);
        }
    }

    /// The result fields of the next record if it is this call.
    fn take(&self, call: String) -> Option<Vec<Field>> {
        if self.diverged.borrow().is_some() {
            return None;
        }
        let pos = self.pos.get();
        match self.records.get(pos) {
            Some(Record::Call { call: c, result }) if *c == call => {
                self.pos.set(pos + 1);
                Some(result.clone())
            }
            Some(r) => {
                self.diverge(format!(
                    "record {} of the log is {}, but the program made the call {call}",
                    pos + 1,
                    show(r)
                ));
                None
            }
            None => {
                self.diverge(format!("the log ended, but the program made the call {call}"));
                None
            }
        }
    }

    fn printed(&mut self, err: bool, text: &str) {
        if self.diverged.borrow().is_none() {
            let pos = self.pos.get();
            match self.records.get(pos) {
                Some(Record::Out(t)) if !err && t == text => self.pos.set(pos + 1),
                Some(Record::Err(t)) if err && t == text => self.pos.set(pos + 1),
                Some(r) => {
                    let what = if err { "err" } else { "out" };
                    self.diverge(format!(
                        "record {} of the log is {}, but the program printed {what} {}",
                        pos + 1,
                        show(r),
                        q(text)
                    ));
                }
                None => {
                    let what = if err { "err" } else { "out" };
                    self.diverge(format!("the log ended, but the program printed {what} {}", q(text)));
                }
            }
        }
        // What a diverged run prints is not what the log recorded.
        if self.diverged.borrow().is_some() {
            return;
        }
        if err { self.out.stderr(text) } else { self.out.stdout(text) }
    }
}

fn show(r: &Record) -> String {
    match r {
        Record::Call { call, result } if result.is_empty() => format!("the call {call}"),
        Record::Call { call, result } => format!("the call {call} -> {}", render(result)),
        Record::Out(t) => format!("out {}", q(t)),
        Record::Err(t) => format!("err {}", q(t)),
        Record::Redacted(c) => format!("a call to `{c}` whose result was redacted"),
        Record::Edit(poll, f, _) => format!("a save of `{f}` seen at poll {poll}"),
        Record::Exit(s) => format!("exit {s}"),
    }
}

fn word(f: &[Field], i: usize) -> &str {
    f.get(i).map(Field::text).unwrap_or("")
}

fn int<T: std::str::FromStr + Default>(f: &[Field], i: usize) -> T {
    word(f, i).parse().unwrap_or_default()
}

fn io_err(f: &[Field], i: usize) -> IoErrorKind {
    match word(f, i) {
        "not-found" => IoErrorKind::NotFound,
        "permission-denied" => IoErrorKind::PermissionDenied,
        "already-exists" => IoErrorKind::AlreadyExists,
        "invalid-data" => IoErrorKind::InvalidData,
        "interrupted" => IoErrorKind::Interrupted,
        _ => IoErrorKind::Other(word(f, i + 1).to_string()),
    }
}

fn net_err(f: &[Field], i: usize) -> NetFail {
    match word(f, i) {
        "refused" => NetFail::Refused,
        "not-allowed" => NetFail::NotAllowed,
        "closed" => NetFail::Closed,
        _ => NetFail::Other(word(f, i + 1).to_string()),
    }
}

/// A diverged replay's answer to a call that fails: the most neutral one.
const GONE: IoErrorKind = IoErrorKind::Interrupted;

fn unit_of(r: Option<Vec<Field>>) -> Result<(), IoErrorKind> {
    match r {
        Some(f) if word(&f, 0) == "ok" => Ok(()),
        Some(f) => Err(io_err(&f, 1)),
        None => Err(GONE),
    }
}

impl Host for Replayer<'_> {
    fn stdout(&mut self, text: &str) {
        self.printed(false, text)
    }
    fn stderr(&mut self, text: &str) {
        self.printed(true, text)
    }
    fn args(&self) -> Vec<String> {
        match self.take("args".into()) {
            Some(f) => f.iter().skip(1).map(|x| x.text().to_string()).collect(),
            None => Vec::new(),
        }
    }
    fn env_var(&self, name: &str) -> Option<String> {
        let f = self.take(format!("env_var {}", q(name)))?;
        (word(&f, 0) == "some").then(|| word(&f, 1).to_string())
    }
    fn now(&self) -> i64 {
        self.take("now".into()).map(|f| int(&f, 0)).unwrap_or(0)
    }
    fn now_nanos(&self) -> i128 {
        self.take("now_nanos".into()).map(|f| int(&f, 0)).unwrap_or(0)
    }
    fn virtual_time(&self) -> bool {
        self.take("virtual_time".into()).is_some_and(|f| word(&f, 0) == "true")
    }
    fn sleep_nanos(&mut self, nanos: u64) {
        // The time passed when the run was recorded; a replay does not wait.
        let _ = self.take(format!("sleep_nanos {nanos}"));
    }
    fn read_file(&mut self, path: &str) -> Result<String, IoErrorKind> {
        match self.take(format!("read_file {}", q(path))) {
            Some(f) if word(&f, 0) == "ok" => Ok(word(&f, 1).to_string()),
            Some(f) => Err(io_err(&f, 1)),
            None => Err(GONE),
        }
    }
    fn write_file(&mut self, path: &str, text: &str) -> Result<(), IoErrorKind> {
        unit_of(self.take(format!("write_file {} #{:016x}", q(path), content_hash(text.as_bytes()))))
    }
    fn read_bytes(&mut self, path: &str) -> Result<Vec<u8>, IoErrorKind> {
        match self.take(format!("read_bytes {}", q(path))) {
            Some(f) if word(&f, 0) == "ok" => unhex(word(&f, 1)).ok_or(IoErrorKind::InvalidData),
            Some(f) => Err(io_err(&f, 1)),
            None => Err(GONE),
        }
    }
    fn write_bytes(&mut self, path: &str, data: &[u8]) -> Result<(), IoErrorKind> {
        unit_of(self.take(format!("write_bytes {} #{:016x}", q(path), content_hash(data))))
    }
    fn exists(&self, path: &str) -> bool {
        self.take(format!("exists {}", q(path))).is_some_and(|f| word(&f, 0) == "true")
    }
    fn metadata(&self, path: &str) -> Result<(i64, i64, bool), IoErrorKind> {
        match self.take(format!("metadata {}", q(path))) {
            Some(f) if word(&f, 0) == "ok" => Ok((int(&f, 1), int(&f, 2), word(&f, 3) == "true")),
            Some(f) => Err(io_err(&f, 1)),
            None => Err(GONE),
        }
    }
    fn remove_file(&mut self, path: &str) -> Result<(), IoErrorKind> {
        unit_of(self.take(format!("remove_file {}", q(path))))
    }
    fn list_dir(&self, path: &str) -> Result<Vec<String>, IoErrorKind> {
        match self.take(format!("list_dir {}", q(path))) {
            Some(f) if word(&f, 0) == "ok" => Ok(f.iter().skip(2).map(|x| x.text().to_string()).collect()),
            Some(f) => Err(io_err(&f, 1)),
            None => Err(GONE),
        }
    }
    fn create_dir(&mut self, path: &str) -> Result<(), IoErrorKind> {
        unit_of(self.take(format!("create_dir {}", q(path))))
    }
    fn sync_path(&mut self, path: &str) -> Result<(), IoErrorKind> {
        unit_of(self.take(format!("sync_path {}", q(path))))
    }
    fn random_bits(&mut self) -> u64 {
        self.take("random_bits".into()).map(|f| int(&f, 0)).unwrap_or(0)
    }
    fn run_process(&mut self, program: &str, args: &[String]) -> Result<(i64, String, String), IoErrorKind> {
        let shown: Vec<String> = args.iter().map(|x| q(x)).collect();
        let call =
            format!("run_process {} {} {}", q(program), args.len(), shown.join(" ")).trim_end().to_string();
        match self.take(call) {
            Some(f) if word(&f, 0) == "ok" => {
                Ok((int(&f, 1), word(&f, 2).to_string(), word(&f, 3).to_string()))
            }
            Some(f) => Err(io_err(&f, 1)),
            None => Err(GONE),
        }
    }
    fn tcp_connect(&mut self, host: &str, port: u16) -> Result<u64, NetFail> {
        match self.take(format!("tcp_connect {} {port}", q(host))) {
            Some(f) if word(&f, 0) == "ok" => Ok(int(&f, 1)),
            Some(f) => Err(net_err(&f, 1)),
            None => Err(NetFail::Closed),
        }
    }
    fn tcp_listen(&mut self, host: &str, port: u16) -> Result<(u64, u16), NetFail> {
        match self.take(format!("tcp_listen {} {port}", q(host))) {
            Some(f) if word(&f, 0) == "ok" => Ok((int(&f, 1), int(&f, 2))),
            Some(f) => Err(net_err(&f, 1)),
            None => Err(NetFail::Closed),
        }
    }
    fn tcp_accept(&mut self, listener: u64) -> Result<Poll<(u64, String)>, NetFail> {
        match self.take(format!("tcp_accept {listener}")) {
            Some(f) if word(&f, 0) == "ok" && word(&f, 1) == "ready" => {
                Ok(Poll::Ready((int(&f, 2), word(&f, 3).to_string())))
            }
            Some(f) if word(&f, 0) == "ok" => Ok(Poll::Wait),
            Some(f) => Err(net_err(&f, 1)),
            None => Err(NetFail::Closed),
        }
    }
    fn tcp_read(&mut self, conn: u64, max: usize) -> Result<Poll<Vec<u8>>, NetFail> {
        match self.take(format!("tcp_read {conn} {max}")) {
            Some(f) if word(&f, 0) == "ok" && word(&f, 1) == "ready" => {
                Ok(Poll::Ready(unhex(word(&f, 2)).unwrap_or_default()))
            }
            Some(f) if word(&f, 0) == "ok" => Ok(Poll::Wait),
            Some(f) => Err(net_err(&f, 1)),
            None => Err(NetFail::Closed),
        }
    }
    fn tcp_write(&mut self, conn: u64, data: &[u8]) -> Result<Poll<usize>, NetFail> {
        match self.take(format!("tcp_write {conn} #{:016x}", content_hash(data))) {
            Some(f) if word(&f, 0) == "ok" && word(&f, 1) == "ready" => Ok(Poll::Ready(int(&f, 2))),
            Some(f) if word(&f, 0) == "ok" => Ok(Poll::Wait),
            Some(f) => Err(net_err(&f, 1)),
            None => Err(NetFail::Closed),
        }
    }
    fn tcp_close(&mut self, handle: u64) {
        let _ = self.take(format!("tcp_close {handle}"));
    }
    fn read_line(&mut self) -> Option<String> {
        let f = self.take("read_line".into())?;
        (word(&f, 0) == "some").then(|| word(&f, 1).to_string())
    }
    /// The saves the recorded run saw at this poll: the `edit` records
    /// numbered with it.
    fn poll_edits(&mut self) -> Vec<(String, String)> {
        self.polls += 1;
        let mut out = Vec::new();
        if self.diverged.borrow().is_some() {
            return out;
        }
        let mut pos = self.pos.get();
        while let Some(Record::Edit(poll, file, text)) = self.records.get(pos) {
            if *poll != self.polls {
                break;
            }
            out.push((file.clone(), text.clone()));
            pos += 1;
        }
        self.pos.set(pos);
        out
    }
}

/// Prints nothing and has nothing: where a [`Resume`]'s replay prints.
struct Sink;

impl Host for Sink {
    fn stdout(&mut self, _: &str) {}
    fn stderr(&mut self, _: &str) {}
    fn args(&self) -> Vec<String> {
        Vec::new()
    }
    fn env_var(&self, _: &str) -> Option<String> {
        None
    }
    fn now(&self) -> i64 {
        0
    }
    fn read_file(&mut self, _: &str) -> Result<String, IoErrorKind> {
        Err(GONE)
    }
    fn write_file(&mut self, _: &str, _: &str) -> Result<(), IoErrorKind> {
        Err(GONE)
    }
    fn exists(&self, _: &str) -> bool {
        false
    }
}

/// A REPL's evaluation (10, "REPL and scripts"; REPL undo): the session's
/// earlier inputs run again with the log of the evaluation before them
/// standing in for the world, printing nothing again and touching
/// nothing again, and what follows that log runs against `live`. Wrapped
/// in a [`Recorder`], the evaluation's own log is the next one's.
pub struct Resume<'h> {
    replay: Replayer<'static>,
    live: &'h mut dyn Host,
    /// Whether the program may read the live standard input (a REPL's is
    /// the session's own).
    stdin: bool,
}

impl<'h> Resume<'h> {
    pub fn new(earlier: Option<&Log>, live: &'h mut dyn Host) -> Resume<'h> {
        // The sink is leaked: it holds nothing, and a REPL makes one per
        // evaluation.
        let sink: &'static mut Sink = Box::leak(Box::new(Sink));
        let empty = Log { header: Header::default(), records: Vec::new() };
        Resume { replay: Replayer::new(earlier.unwrap_or(&empty), sink), live, stdin: true }
    }

    /// The program reads no live standard input: it ends at once.
    pub fn without_stdin(mut self) -> Resume<'h> {
        self.stdin = false;
        self
    }

    /// Whether a host call is answered from the earlier log: while it has
    /// records, and has not diverged (an earlier input that no longer
    /// does what it did, which stage 0 does not expect).
    fn replaying(&self) -> bool {
        self.replay.diverged.borrow().is_none() && self.replay.pos.get() < self.replay.records.len()
    }

    /// The divergence from the earlier log, if the replay of the session
    /// met one.
    pub fn divergence(&self) -> Option<String> {
        self.replay.diverged.borrow().clone()
    }
}

macro_rules! resume_to {
    ($self:ident . $m:ident ( $($a:expr),* )) => {
        if $self.replaying() { $self.replay.$m($($a),*) } else { $self.live.$m($($a),*) }
    };
}

impl Host for Resume<'_> {
    fn stdout(&mut self, text: &str) {
        resume_to!(self.stdout(text))
    }
    fn stderr(&mut self, text: &str) {
        resume_to!(self.stderr(text))
    }
    fn args(&self) -> Vec<String> {
        resume_to!(self.args())
    }
    fn env_var(&self, name: &str) -> Option<String> {
        resume_to!(self.env_var(name))
    }
    fn now(&self) -> i64 {
        resume_to!(self.now())
    }
    fn now_nanos(&self) -> i128 {
        resume_to!(self.now_nanos())
    }
    fn virtual_time(&self) -> bool {
        resume_to!(self.virtual_time())
    }
    fn sleep_nanos(&mut self, nanos: u64) {
        resume_to!(self.sleep_nanos(nanos))
    }
    fn read_file(&mut self, path: &str) -> Result<String, IoErrorKind> {
        resume_to!(self.read_file(path))
    }
    fn write_file(&mut self, path: &str, text: &str) -> Result<(), IoErrorKind> {
        resume_to!(self.write_file(path, text))
    }
    fn read_bytes(&mut self, path: &str) -> Result<Vec<u8>, IoErrorKind> {
        resume_to!(self.read_bytes(path))
    }
    fn write_bytes(&mut self, path: &str, data: &[u8]) -> Result<(), IoErrorKind> {
        resume_to!(self.write_bytes(path, data))
    }
    fn exists(&self, path: &str) -> bool {
        resume_to!(self.exists(path))
    }
    fn metadata(&self, path: &str) -> Result<(i64, i64, bool), IoErrorKind> {
        resume_to!(self.metadata(path))
    }
    fn remove_file(&mut self, path: &str) -> Result<(), IoErrorKind> {
        resume_to!(self.remove_file(path))
    }
    fn list_dir(&self, path: &str) -> Result<Vec<String>, IoErrorKind> {
        resume_to!(self.list_dir(path))
    }
    fn create_dir(&mut self, path: &str) -> Result<(), IoErrorKind> {
        resume_to!(self.create_dir(path))
    }
    fn sync_path(&mut self, path: &str) -> Result<(), IoErrorKind> {
        resume_to!(self.sync_path(path))
    }
    fn random_bits(&mut self) -> u64 {
        resume_to!(self.random_bits())
    }
    fn run_process(&mut self, program: &str, args: &[String]) -> Result<(i64, String, String), IoErrorKind> {
        resume_to!(self.run_process(program, args))
    }
    fn tcp_connect(&mut self, host: &str, port: u16) -> Result<u64, NetFail> {
        resume_to!(self.tcp_connect(host, port))
    }
    fn tcp_listen(&mut self, host: &str, port: u16) -> Result<(u64, u16), NetFail> {
        resume_to!(self.tcp_listen(host, port))
    }
    fn tcp_accept(&mut self, listener: u64) -> Result<Poll<(u64, String)>, NetFail> {
        resume_to!(self.tcp_accept(listener))
    }
    fn tcp_read(&mut self, conn: u64, max: usize) -> Result<Poll<Vec<u8>>, NetFail> {
        resume_to!(self.tcp_read(conn, max))
    }
    fn tcp_write(&mut self, conn: u64, data: &[u8]) -> Result<Poll<usize>, NetFail> {
        resume_to!(self.tcp_write(conn, data))
    }
    fn tcp_close(&mut self, handle: u64) {
        resume_to!(self.tcp_close(handle))
    }
    fn read_line(&mut self) -> Option<String> {
        if !self.replaying() && !self.stdin {
            return None;
        }
        resume_to!(self.read_line())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::TestHost;

    fn header() -> Header {
        Header {
            files: Vec::new(),
            layout: None,
            watch: false,
            sir: false,
            toolchain: "0.0.1".into(),
            file: "a.sisu".into(),
            hash: 0xabc,
            source: "def main():\n    print(\"hi\")\n".into(),
            seed: 3,
            hash_key: 0,
            contracts: "all".into(),
        }
    }

    #[test]
    fn a_log_reads_back_and_replays_its_calls() {
        let mut real =
            TestHost { env: [("HOME".to_string(), "/h \"q\"".to_string())].into(), ..Default::default() };
        real.stdin.push_back("line one".into());
        let mut rec = Recorder::new(&mut real, Vec::new());
        assert_eq!(rec.env_var("HOME").as_deref(), Some("/h \"q\""));
        assert_eq!(rec.read_line().as_deref(), Some("line one"));
        assert_eq!(rec.read_line(), None);
        assert!(rec.read_file("missing").is_err());
        rec.stdout("hi\n");
        let text = rec.finish(&header(), 0);
        let log = Log::parse(&text).expect("the log parses");
        assert_eq!(log.header, header());
        assert_eq!(log.status(), Some(0));
        let mut out = TestHost::default();
        let mut rep = Replayer::new(&log, &mut out);
        assert_eq!(rep.env_var("HOME").as_deref(), Some("/h \"q\""));
        assert_eq!(rep.read_line().as_deref(), Some("line one"));
        assert_eq!(rep.read_line(), None);
        assert_eq!(rep.read_file("missing"), Err(IoErrorKind::NotFound));
        rep.stdout("hi\n");
        assert_eq!(rep.divergence(), None);
        assert_eq!(out.out, "hi\n");
    }

    #[test]
    fn a_different_call_is_a_divergence() {
        let mut real = TestHost::default();
        let rec = Recorder::new(&mut real, Vec::new());
        let _ = rec.env_var("A");
        let log = Log::parse(&rec.finish(&header(), 0)).expect("the log parses");
        let mut out = TestHost::default();
        let rep = Replayer::new(&log, &mut out);
        let _ = rep.env_var("B");
        let d = rep.divergence().expect("a divergence");
        assert!(d.contains("env_var \"A\"") && d.contains("env_var \"B\""), "{d}");
    }

    #[test]
    fn a_redacted_result_is_not_in_the_log() {
        let mut real =
            TestHost { env: [("SECRET".to_string(), "hunter2".to_string())].into(), ..Default::default() };
        let rec = Recorder::new(&mut real, vec!["env".into()]);
        let _ = rec.env_var("SECRET");
        let text = rec.finish(&header(), 0);
        assert!(!text.contains("hunter2"));
        assert!(text.contains("redacted env"));
    }
}
