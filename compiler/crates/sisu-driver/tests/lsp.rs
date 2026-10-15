//! `sisu lsp` over standard input and output: a client that starts the
//! server this package builds and speaks LSP to it, as an editor does
//! (10, "Language server"). Each test checks one feature; `latency` times
//! the answers on the standard library's own modules against the 100 ms
//! an editor's keystroke allows.

// A test's helpers may fail loudly, as its body may (the panic policy is
// for the crates, not their tests).
#![allow(clippy::expect_used)]

use sisu_driver::json::{Json, parse};
use sisu_driver::lsp::{path_uri, read_message, write_message};
use std::collections::VecDeque;
use std::io::BufReader;
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

struct Client {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    next: u64,
    /// Notifications read while waiting for a response.
    pending: VecDeque<Json>,
}

impl Client {
    fn start() -> Client {
        let mut child = Command::new(env!("CARGO_BIN_EXE_sisu"))
            .arg("lsp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("sisu lsp starts");
        let input = child.stdin.take().expect("stdin");
        let output = BufReader::new(child.stdout.take().expect("stdout"));
        let mut c = Client { child, input, output, next: 1, pending: VecDeque::new() };
        let r = c.request(
            "initialize",
            Json::obj(vec![
                ("processId", Json::Null),
                ("rootUri", Json::Null),
                ("capabilities", Json::obj(vec![])),
            ]),
        );
        assert!(r.at(&["capabilities", "hoverProvider"]).is_some(), "{}", r.write());
        c.notify("initialized", Json::obj(vec![]));
        c
    }

    fn send(&mut self, m: Json) {
        write_message(&mut self.input, &m.write());
    }

    fn read(&mut self) -> Json {
        let text = read_message(&mut self.output).expect("a message from the server");
        parse(&text).expect("JSON")
    }

    fn notify(&mut self, method: &str, params: Json) {
        self.send(Json::obj(vec![
            ("jsonrpc", Json::str("2.0")),
            ("method", Json::str(method)),
            ("params", params),
        ]));
    }

    /// Sends a request and waits for its response's result.
    fn request(&mut self, method: &str, params: Json) -> Json {
        let id = self.next;
        self.next += 1;
        self.send(Json::obj(vec![
            ("jsonrpc", Json::str("2.0")),
            ("id", Json::num(id as f64)),
            ("method", Json::str(method)),
            ("params", params),
        ]));
        loop {
            let m = self.read();
            if m.get("id").and_then(Json::as_u64) == Some(id) && m.get("method").is_none() {
                return m.get("result").cloned().unwrap_or(Json::Null);
            }
            self.pending.push_back(m);
        }
    }

    /// The next notification of `method`.
    fn notification(&mut self, method: &str) -> Json {
        if let Some(i) =
            self.pending.iter().position(|m| m.get("method").and_then(Json::as_str) == Some(method))
        {
            return self.pending.remove(i).and_then(|m| m.get("params").cloned()).unwrap_or(Json::Null);
        }
        loop {
            let m = self.read();
            if m.get("method").and_then(Json::as_str) == Some(method) {
                return m.get("params").cloned().unwrap_or(Json::Null);
            }
        }
    }

    fn open(&mut self, uri: &str, text: &str) -> Json {
        self.notify(
            "textDocument/didOpen",
            Json::obj(vec![(
                "textDocument",
                Json::obj(vec![
                    ("uri", Json::str(uri)),
                    ("languageId", Json::str("sisu")),
                    ("version", Json::num(1)),
                    ("text", Json::str(text)),
                ]),
            )]),
        );
        self.notification("textDocument/publishDiagnostics")
    }

    fn change(&mut self, uri: &str, version: u32, text: &str) -> Json {
        self.notify(
            "textDocument/didChange",
            Json::obj(vec![
                ("textDocument", Json::obj(vec![("uri", Json::str(uri)), ("version", Json::num(version))])),
                ("contentChanges", Json::Arr(vec![Json::obj(vec![("text", Json::str(text))])])),
            ]),
        );
        self.notification("textDocument/publishDiagnostics")
    }

    fn at(&mut self, method: &str, uri: &str, line: u32, character: u32) -> Json {
        self.request(
            method,
            Json::obj(vec![
                ("textDocument", Json::obj(vec![("uri", Json::str(uri))])),
                ("position", Json::obj(vec![("line", Json::num(line)), ("character", Json::num(character))])),
            ]),
        )
    }

    fn stop(mut self) {
        let r = self.request("shutdown", Json::Null);
        assert_eq!(r, Json::Null);
        self.notify("exit", Json::Null);
        let status = self.child.wait().expect("the server exits");
        assert!(status.success(), "exit after shutdown is status 0");
    }
}

/// A document's URI, in a fresh directory of its own (no package above).
fn document(name: &str, text: &str) -> (String, PathBuf) {
    let dir = std::env::temp_dir().join(format!("sisu-lsp-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a directory");
    let path = dir.join(name);
    std::fs::write(&path, text).expect("written");
    (path_uri(&path), dir)
}

fn hover_text(h: &Json) -> String {
    h.at(&["contents", "value"]).and_then(Json::as_str).unwrap_or("").to_string()
}

const PROGRAM: &str = "\
## The area of a rectangle.
pure def area(w: int, h: int) -> int:
    requires w >= 0 and h >= 0
    return w * h

error Missing:
    Gone

def load(fs: Fs, name: Path) -> int !:
    let text = fs.read_to_string(name)?
    if text.is_empty():
        return Err(Gone)
    return Ok(area(len(text), 2))

struct Point:
    x: int
    y: int

def main(sys: System):
    let p = Point(x=1, y=2)
    print(area(p.x, p.y))
    _ = load(sys.fs, \"a.txt\")
";

#[test]
fn diagnostics_carry_catalog_codes() {
    let mut c = Client::start();
    let (uri, _dir) = document("bad.sisu", "def main():\n    let x: int = \"text\"\n    print(y)\n");
    let d = c.open(&uri, "def main():\n    let x: int = \"text\"\n    print(y)\n");
    let diags = d.get("diagnostics").and_then(Json::as_array).unwrap_or(&[]).to_vec();
    let codes: Vec<&str> = diags.iter().filter_map(|x| x.get("code").and_then(Json::as_str)).collect();
    assert!(codes.contains(&"E0300") && codes.contains(&"E0200"), "{}", d.write());
    let ids: Vec<&str> = diags.iter().filter_map(|x| x.at(&["data", "id"]).and_then(Json::as_str)).collect();
    assert!(ids.contains(&"resolve.undefined-name"), "{ids:?}");
    // The range of `y`: line 2, characters 10 to 11.
    let y = diags.iter().find(|x| x.get("code").and_then(Json::as_str) == Some("E0200")).expect("E0200");
    assert_eq!(y.at(&["range", "start", "line"]).and_then(Json::as_u64), Some(2));
    assert_eq!(y.at(&["range", "start", "character"]).and_then(Json::as_u64), Some(10));
    // Fixed, the diagnostics go.
    let d = c.change(&uri, 2, "def main():\n    let x: int = 1\n    print(x)\n");
    assert_eq!(d.get("diagnostics").and_then(Json::as_array).map(<[Json]>::len), Some(0), "{}", d.write());
    c.stop();
}

#[test]
fn hover_shows_types_signatures_error_sets_capabilities_and_contracts() {
    let mut c = Client::start();
    let (uri, _dir) = document("prog.sisu", PROGRAM);
    let d = c.open(&uri, PROGRAM);
    assert_eq!(d.get("diagnostics").and_then(Json::as_array).map(<[Json]>::len), Some(0), "{}", d.write());
    // `area` in `print(area(p.x, p.y))`.
    let h = hover_text(&c.at("textDocument/hover", &uri, 20, 11));
    assert!(h.contains("pure def area(w: int, h: int) -> int"), "{h}");
    assert!(h.contains("requires `w >= 0 and h >= 0`"), "{h}");
    assert!(h.contains("The area of a rectangle."), "{h}");
    // `load`'s name: its inferred error set and its capability.
    let h = hover_text(&c.at("textDocument/hover", &uri, 8, 5));
    assert!(h.contains("error set (inferred)") && h.contains("Missing") && h.contains("IoError"), "{h}");
    assert!(h.contains("capabilities: `fs: Fs`"), "{h}");
    // A local, and a field.
    let h = hover_text(&c.at("textDocument/hover", &uri, 19, 8));
    assert!(h.contains("let p: Point"), "{h}");
    let h = hover_text(&c.at("textDocument/hover", &uri, 20, 17));
    assert!(h.contains("x: int"), "{h}");
    c.stop();
}

#[test]
fn definition_goes_to_the_program_and_to_the_standard_library() {
    let mut c = Client::start();
    let (uri, _dir) = document("prog.sisu", PROGRAM);
    c.open(&uri, PROGRAM);
    let d = c.at("textDocument/definition", &uri, 20, 11);
    assert_eq!(d.get("uri").and_then(Json::as_str), Some(uri.as_str()), "{}", d.write());
    assert_eq!(d.at(&["range", "start", "line"]).and_then(Json::as_u64), Some(1));
    // `print` is the prelude's.
    let d = c.at("textDocument/definition", &uri, 20, 5);
    let target = d.get("uri").and_then(Json::as_str).unwrap_or("");
    assert!(target.ends_with("/std/prelude.sisu"), "{}", d.write());
    c.stop();
}

#[test]
fn symbols_selection_ranges_and_formatting() {
    let mut c = Client::start();
    let (uri, _dir) = document("prog.sisu", PROGRAM);
    c.open(&uri, PROGRAM);
    let s = c.request(
        "textDocument/documentSymbol",
        Json::obj(vec![("textDocument", Json::obj(vec![("uri", Json::str(&uri))]))]),
    );
    let names: Vec<&str> =
        s.as_array().unwrap_or(&[]).iter().filter_map(|x| x.get("name").and_then(Json::as_str)).collect();
    assert_eq!(names, ["area", "Missing", "load", "Point", "main"], "{}", s.write());
    let point = &s.as_array().unwrap_or(&[])[3];
    let fields: Vec<&str> = point
        .get("children")
        .and_then(Json::as_array)
        .unwrap_or(&[])
        .iter()
        .filter_map(|x| x.get("name").and_then(Json::as_str))
        .collect();
    assert_eq!(fields, ["x", "y"]);
    // Around `p.x` in `print(area(p.x, p.y))`: the field, the argument, the
    // call, ..., the function, the file.
    let r = c.request(
        "textDocument/selectionRange",
        Json::obj(vec![
            ("textDocument", Json::obj(vec![("uri", Json::str(&uri))])),
            (
                "positions",
                Json::Arr(vec![Json::obj(vec![("line", Json::num(20)), ("character", Json::num(16))])]),
            ),
        ]),
    );
    let mut depth = 0;
    let mut cur = r.as_array().and_then(|a| a.first()).cloned().unwrap_or(Json::Null);
    let mut widths = Vec::new();
    while cur != Json::Null {
        widths.push(cur.write().len());
        depth += 1;
        cur = cur.get("parent").cloned().unwrap_or(Json::Null);
    }
    assert!(depth >= 5, "{}", r.write());
    // Formatting: an edit that writes the canonical text.
    let messy = "def main( ):\n    print( 1+2 )\n";
    let (uri2, _dir2) = document("messy.sisu", messy);
    c.open(&uri2, messy);
    let f = c.request(
        "textDocument/formatting",
        Json::obj(vec![
            ("textDocument", Json::obj(vec![("uri", Json::str(&uri2))])),
            ("options", Json::obj(vec![("tabSize", Json::num(4)), ("insertSpaces", Json::Bool(true))])),
        ]),
    );
    let edit = f.as_array().and_then(|a| a.first()).cloned().unwrap_or(Json::Null);
    assert_eq!(
        edit.get("newText").and_then(Json::as_str),
        Some("def main():\n    print(1 + 2)\n"),
        "{}",
        f.write()
    );
    c.stop();
}

/// The answers on the standard library's modules, opened as documents:
/// the diagnostics after a change and a hover, each against the 100 ms an
/// editor allows. The numbers are printed (`cargo test -- --nocapture`);
/// the test fails only past ten times that, since a debug build on a busy
/// machine is not the measure.
#[test]
fn latency() {
    let mut c = Client::start();
    let std_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../std");
    for name in ["num.sisu", "collections.sisu", "time.sisu", "json.sisu"] {
        let path = std_dir.join(name);
        let text = std::fs::read_to_string(&path).expect("a std module");
        let uri = path_uri(&std::fs::canonicalize(&path).expect("a path"));
        let start = std::time::Instant::now();
        c.open(&uri, &text);
        let open = start.elapsed();
        let start = std::time::Instant::now();
        let changed = format!("{text}\n");
        c.change(&uri, 2, &changed);
        let change = start.elapsed();
        // A hover in the middle of the file.
        let line = (text.lines().count() / 2) as u32;
        let start = std::time::Instant::now();
        let _ = c.at("textDocument/hover", &uri, line, 8);
        let hover = start.elapsed();
        println!(
            "std/{name}: open {} ms, change {} ms, hover {} ms",
            open.as_millis(),
            change.as_millis(),
            hover.as_millis()
        );
        assert!(change.as_millis() < 1000 && hover.as_millis() < 1000, "{name}: too slow");
    }
    c.stop();
}
