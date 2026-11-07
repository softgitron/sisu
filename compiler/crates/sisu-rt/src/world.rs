//! The capability log of a native run (N-04; decision 1321): what the
//! interpreter's `record::Recorder` and `record::Replayer` do for its host,
//! done here for the runtime's calls of the host contract, in the same
//! form, so that a log a native run wrote replays in the interpreter and
//! one the interpreter wrote replays natively where the program makes the
//! same calls.
//!
//! `SISU_RT_RECORD=PATH` makes the run write each call of the host contract
//! with its result (`call NAME FIELDS -> RESULT`, data the program wrote by
//! its content hash), each text it printed (`out`, `err`) and a call of a
//! capability `SISU_RT_REDACT` names as `redacted CAP`, one a line, to
//! `PATH` when it ends: the records of the interpreter's log, which the
//! driver puts after the log's header and before its `exit` line.
//! `SISU_RT_REPLAY=PATH` makes the run answer each call from the records
//! the driver wrote there (each a list of fields, a field its length in
//! bytes, a colon and its bytes: the record's kind, the call or text, how
//! the interpreter's replay shows it, and the result's fields), and print
//! only what the log printed; the first call or text unlike the log's is
//! the divergence, after which calls get the interpreter's neutral answers
//! and nothing is printed, and `SISU_RT_REPLAY_REPORT` names the file the
//! run writes `ok` or `diverged` and the divergence to when it ends.
//!
//! The calls are the interpreter's host contract's, in its order: the
//! host's clock asked whether it is virtual when the run starts, the real
//! clock read for its time (`now_nanos`) and its monotonic reading, the
//! scheduler waiting for a timer of it (`sleep_nanos`), the file system,
//! the environment, standard input, random bits, processes and sockets.

use std::sync::Mutex;
use std::sync::atomic::{AtomicU8, Ordering};

/// What the run does with its calls: nothing, record them, or replay them.
static MODE: AtomicU8 = AtomicU8::new(OFF);
const OFF: u8 = 0;
const RECORD: u8 = 1;
const REPLAY: u8 = 2;

/// A record of a log being replayed: its kind (`c`, `o`, `e`, `r`, `w`),
/// the call or the text, how the interpreter's replay shows it, and the
/// result's fields.
struct Rec {
    kind: String,
    what: String,
    shown: String,
    result: Vec<String>,
}

#[derive(Default)]
struct World {
    /// The record's path and the lines so far.
    record: Option<std::path::PathBuf>,
    lines: Vec<String>,
    redact: Vec<String>,
    /// The replay's records, the next one, the divergence and the report's
    /// path.
    records: Vec<Rec>,
    pos: usize,
    diverged: Option<String>,
    report: Option<std::path::PathBuf>,
}

static WORLD: Mutex<Option<World>> = Mutex::new(None);

fn with<R>(f: impl FnOnce(&mut World) -> R) -> R {
    let mut w = WORLD.lock().unwrap_or_else(|p| p.into_inner());
    f(w.get_or_insert_with(World::default))
}

/// Reads `SISU_RT_RECORD` and `SISU_RT_REPLAY` when the run starts, and
/// asks the host's clock whether it is virtual, as the interpreter does
/// first: a native run's host clock never is.
pub(crate) fn start() {
    if let Some(path) = std::env::var_os("SISU_RT_REPLAY") {
        let text = std::fs::read(&path).unwrap_or_default();
        let records = parse(&text);
        with(|w| {
            w.records = records;
            w.report = std::env::var_os("SISU_RT_REPLAY_REPORT").map(Into::into);
        });
        MODE.store(REPLAY, Ordering::SeqCst);
    } else if let Some(path) = std::env::var_os("SISU_RT_RECORD") {
        let redact = std::env::var("SISU_RT_REDACT").unwrap_or_default();
        with(|w| {
            w.record = Some(path.into());
            w.redact = redact.split(',').filter(|c| !c.is_empty()).map(str::to_string).collect();
        });
        MODE.store(RECORD, Ordering::SeqCst);
    }
    let _ =
        call("clock", "virtual_time".into(), || false, |v| v.to_string(), |f| word(f, 0) == "true", || false);
}

/// Writes the record, or the replay's report, when the run ends (at its
/// end or when it aborts).
pub(crate) fn finish() {
    match MODE.swap(OFF, Ordering::SeqCst) {
        RECORD => with(|w| {
            if let Some(path) = &w.record {
                let mut text = String::new();
                for l in &w.lines {
                    text.push_str(l);
                    text.push('\n');
                }
                let _ = std::fs::write(path, text);
            }
        }),
        REPLAY => with(|w| {
            let detail = w.diverged.clone().or_else(|| {
                w.records
                    .get(w.pos)
                    .map(|r| format!("the run ended before record {} of the log: {}", w.pos + 1, r.shown))
            });
            if let Some(path) = &w.report {
                let text = match detail {
                    Some(d) => format!("diverged\n{d}"),
                    None => "ok\n".to_string(),
                };
                let _ = std::fs::write(path, text);
            }
        }),
        _ => {}
    }
}

/// A text quoted as the log quotes it (SIR's quoting).
pub(crate) fn quote(s: &str) -> String {
    let mut q = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => q.push_str("\\\""),
            '\\' => q.push_str("\\\\"),
            '\n' => q.push_str("\\n"),
            '\t' => q.push_str("\\t"),
            '\r' => q.push_str("\\r"),
            '\0' => q.push_str("\\0"),
            c if (c as u32) < 0x20 => q.push_str(&format!("\\u{{{:x}}}", c as u32)),
            c => q.push(c),
        }
    }
    q.push('"');
    q
}

/// FNV-1a over bytes, 64 bits: the content hash of data the program wrote.
pub(crate) fn content_hash(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in data {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

pub(crate) fn hex(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02x}")).collect()
}

pub(crate) fn unhex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok()).collect()
}

/// The `i`-th field of a result, or nothing.
pub(crate) fn word(f: &[String], i: usize) -> &str {
    f.get(i).map(String::as_str).unwrap_or("")
}

pub(crate) fn int<T: std::str::FromStr + Default>(f: &[String], i: usize) -> T {
    word(f, i).parse().unwrap_or_default()
}

/// The records of a replay, as the driver writes them.
fn parse(mut text: &[u8]) -> Vec<Rec> {
    let mut item = || -> Option<String> {
        let colon = text.iter().position(|b| *b == b':')?;
        let n: usize = std::str::from_utf8(&text[..colon]).ok()?.parse().ok()?;
        let body = text.get(colon + 1..colon + 1 + n)?;
        let s = String::from_utf8_lossy(body).into_owned();
        text = &text[colon + 1 + n..];
        Some(s)
    };
    let mut out = Vec::new();
    while let Some(kind) = item() {
        let (Some(what), Some(shown), Some(n)) = (item(), item(), item()) else { break };
        let n: usize = n.parse().unwrap_or(0);
        let result = (0..n).map_while(|_| item()).collect();
        out.push(Rec { kind, what, shown, result });
    }
    out
}

/// A call of the host contract by the capability `cap`, written `call`:
/// `real` makes it, which a replay does not; `show` writes its result as
/// the log does (empty for none), `read` reads it back from the log's
/// fields, and `gone` is the answer after a divergence.
pub(crate) fn call<T>(
    cap: &str,
    call: String,
    real: impl FnOnce() -> T,
    show: impl FnOnce(&T) -> String,
    read: impl FnOnce(&[String]) -> T,
    gone: impl FnOnce() -> T,
) -> T {
    match MODE.load(Ordering::Relaxed) {
        RECORD => {
            let v = real();
            let result = show(&v);
            with(|w| {
                let line = if w.redact.iter().any(|r| r == cap) {
                    format!("redacted {cap}")
                } else if result.is_empty() {
                    format!("call {call}")
                } else {
                    format!("call {call} -> {result}")
                };
                w.lines.push(line);
            });
            v
        }
        REPLAY => match take(&call) {
            Some(f) => read(&f),
            None => gone(),
        },
        _ => real(),
    }
}

/// The result's fields of the next record if it is this call; else the
/// divergence.
fn take(call: &str) -> Option<Vec<String>> {
    with(|w| {
        if w.diverged.is_some() {
            return None;
        }
        match w.records.get(w.pos) {
            Some(r) if r.kind == "c" && r.what == call => {
                w.pos += 1;
                Some(r.result.clone())
            }
            Some(r) => {
                w.diverged = Some(format!(
                    "record {} of the log is {}, but the program made the call {call}",
                    w.pos + 1,
                    r.shown
                ));
                None
            }
            None => {
                w.diverged = Some(format!("the log ended, but the program made the call {call}"));
                None
            }
        }
    })
}

/// What the program printed, on its standard output or error: recorded, or
/// checked against the log; whether it is to be printed (not after a
/// replay diverged).
pub(crate) fn printed(err: bool, text: &[u8]) -> bool {
    let mode = MODE.load(Ordering::Relaxed);
    if mode == OFF {
        return true;
    }
    let text = String::from_utf8_lossy(text);
    let what = if err { "err" } else { "out" };
    with(|w| {
        if mode == RECORD {
            w.lines.push(format!("{what} {}", quote(&text)));
            return true;
        }
        if w.diverged.is_none() {
            let kind = if err { "e" } else { "o" };
            match w.records.get(w.pos) {
                Some(r) if r.kind == kind && r.what == text => w.pos += 1,
                Some(r) => {
                    w.diverged = Some(format!(
                        "record {} of the log is {}, but the program printed {what} {}",
                        w.pos + 1,
                        r.shown,
                        quote(&text)
                    ));
                }
                None => {
                    w.diverged =
                        Some(format!("the log ended, but the program printed {what} {}", quote(&text)));
                }
            }
        }
        w.diverged.is_none()
    })
}

/// The program's arguments after its name: the process's, or the log's.
pub(crate) fn args() -> &'static [String] {
    let real = crate::entry::args;
    let shown = |a: &&'static [String]| {
        let q: Vec<String> = a.iter().map(|x| quote(x)).collect();
        format!("{} {}", a.len(), q.join(" ")).trim_end().to_string()
    };
    // A replay's arguments live as long as the process, as its own do.
    let read = |f: &[String]| -> &'static [String] {
        Box::leak(f.iter().skip(1).cloned().collect::<Vec<_>>().into_boxed_slice())
    };
    call("env", "args".into(), real, shown, read, || &[])
}

/// The time of the host's real clock in nanoseconds since the epoch.
pub(crate) fn now_nanos() -> i128 {
    call(
        "clock",
        "now_nanos".into(),
        || {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as i128)
                .unwrap_or(0)
        },
        |v| v.to_string(),
        |f| int(f, 0),
        || 0,
    )
}

/// The host's monotonic reading, from the run's first.
pub(crate) fn monotonic_nanos() -> i128 {
    static ORIGIN: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    call(
        "clock",
        "monotonic_nanos".into(),
        || ORIGIN.get_or_init(std::time::Instant::now).elapsed().as_nanos() as i128,
        |v| v.to_string(),
        |f| int(f, 0),
        || 0,
    )
}

/// Waits `nanos` of real time for a timer of the host's clock; a replay
/// does not wait, the time having passed when the run was recorded.
pub(crate) fn sleep_nanos(nanos: u64) {
    call(
        "clock",
        format!("sleep_nanos {nanos}"),
        || std::thread::sleep(std::time::Duration::from_nanos(nanos)),
        |()| String::new(),
        |_| (),
        || (),
    )
}

/// A file operation's failure as the log writes it (`not-found`, `other
/// "..."`).
pub(crate) fn io_word(k: &crate::intrinsics::IoKind) -> String {
    use crate::intrinsics::IoKind as K;
    match k {
        K::NotFound => "not-found".into(),
        K::PermissionDenied => "permission-denied".into(),
        K::AlreadyExists => "already-exists".into(),
        K::InvalidData => "invalid-data".into(),
        K::Interrupted => "interrupted".into(),
        K::Other(m) => format!("other {}", quote(m)),
    }
}

/// The failure the log's fields from `i` on write.
pub(crate) fn io_read(f: &[String], i: usize) -> crate::intrinsics::IoKind {
    use crate::intrinsics::IoKind as K;
    match word(f, i) {
        "not-found" => K::NotFound,
        "permission-denied" => K::PermissionDenied,
        "already-exists" => K::AlreadyExists,
        "invalid-data" => K::InvalidData,
        "interrupted" => K::Interrupted,
        _ => K::Other(word(f, i + 1).to_string()),
    }
}

/// A file operation's result of nothing: `ok`, or its failure.
pub(crate) fn unit_word(r: &Result<(), crate::intrinsics::IoKind>) -> String {
    match r {
        Ok(()) => "ok".into(),
        Err(k) => format!("err {}", io_word(k)),
    }
}

pub(crate) fn unit_read(f: &[String]) -> Result<(), crate::intrinsics::IoKind> {
    if word(f, 0) == "ok" { Ok(()) } else { Err(io_read(f, 1)) }
}

/// The answer to a file operation after a divergence: the most neutral
/// failure, as the interpreter's replay gives.
pub(crate) fn io_gone<T>() -> Result<T, crate::intrinsics::IoKind> {
    Err(crate::intrinsics::IoKind::Interrupted)
}
