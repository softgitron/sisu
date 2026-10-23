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

fn labels(r: &Json) -> Vec<String> {
    r.get("items")
        .and_then(Json::as_array)
        .unwrap_or(&[])
        .iter()
        .filter_map(|i| i.get("label").and_then(Json::as_str).map(str::to_string))
        .collect()
}

fn insert_text(r: &Json, label: &str) -> Option<String> {
    r.get("items")
        .and_then(Json::as_array)
        .unwrap_or(&[])
        .iter()
        .find(|i| i.get("label").and_then(Json::as_str) == Some(label))
        .and_then(|i| i.get("insertText").and_then(Json::as_str).map(str::to_string))
}

const SHAPES: &str = "\
struct Point:
    x: int
    y: int

    def norm(self) -> int:
        return self.x * self.x + self.y * self.y

def area(w: int, h: int) -> int:
    return w * h

def main():
    let p = Point(x=1, y=2)
    let total = area(p.x, p.y)
    print(total)
";

#[test]
fn completion_offers_members_and_names_in_scope() {
    let mut c = Client::start();
    let text = format!("{SHAPES}    print(p.)\n");
    let (uri, _dir) = document("complete.sisu", &text);
    c.open(&uri, &text);
    // After `p.`: the fields and methods of a `Point`, as snippets.
    let r = c.at("textDocument/completion", &uri, 14, 12);
    let names = labels(&r);
    assert!(names.contains(&"x".to_string()) && names.contains(&"y".to_string()), "{names:?}");
    assert_eq!(insert_text(&r, "norm").as_deref(), Some("norm()"), "{}", r.write());
    assert!(!names.contains(&"area".to_string()), "{names:?}");
    // At the start of a word: the names in scope that start with it.
    let text = format!("{SHAPES}    to\n    ar\n");
    c.change(&uri, 2, &text);
    let r = c.at("textDocument/completion", &uri, 14, 6);
    assert!(labels(&r).contains(&"total".to_string()), "{}", r.write());
    let r = c.at("textDocument/completion", &uri, 15, 6);
    assert_eq!(insert_text(&r, "area").as_deref(), Some("area(${1:w}, ${2:h})"), "{}", r.write());
    c.stop();
}

#[test]
fn rename_changes_every_use() {
    let mut c = Client::start();
    let (uri, _dir) = document("rename.sisu", SHAPES);
    c.open(&uri, SHAPES);
    let edits_of = |r: &Json| -> Vec<(u64, u64, String)> {
        r.at(&["changes", &uri])
            .and_then(Json::as_array)
            .unwrap_or(&[])
            .iter()
            .map(|e| {
                (
                    e.at(&["range", "start", "line"]).and_then(Json::as_u64).unwrap_or(0),
                    e.at(&["range", "start", "character"]).and_then(Json::as_u64).unwrap_or(0),
                    e.get("newText").and_then(Json::as_str).unwrap_or("").to_string(),
                )
            })
            .collect()
    };
    // A local: its declaration and its use.
    let pos = |line: u32, ch: u32| Json::obj(vec![("line", Json::num(line)), ("character", Json::num(ch))]);
    let prep = c.request(
        "textDocument/prepareRename",
        Json::obj(vec![
            ("textDocument", Json::obj(vec![("uri", Json::str(&uri))])),
            ("position", pos(12, 9)),
        ]),
    );
    assert_eq!(prep.get("placeholder").and_then(Json::as_str), Some("total"), "{}", prep.write());
    let r = c.request(
        "textDocument/rename",
        Json::obj(vec![
            ("textDocument", Json::obj(vec![("uri", Json::str(&uri))])),
            ("position", pos(12, 9)),
            ("newName", Json::str("sum")),
        ]),
    );
    assert_eq!(edits_of(&r), vec![(12, 8, "sum".to_string()), (13, 10, "sum".to_string())], "{}", r.write());
    // A function: its definition and its call.
    let r = c.request(
        "textDocument/rename",
        Json::obj(vec![
            ("textDocument", Json::obj(vec![("uri", Json::str(&uri))])),
            ("position", pos(7, 5)),
            ("newName", Json::str("surface")),
        ]),
    );
    assert_eq!(
        edits_of(&r),
        vec![(7, 4, "surface".to_string()), (12, 16, "surface".to_string())],
        "{}",
        r.write()
    );
    // A field: its declaration, the construction and the reads.
    let r = c.request(
        "textDocument/rename",
        Json::obj(vec![
            ("textDocument", Json::obj(vec![("uri", Json::str(&uri))])),
            ("position", pos(1, 4)),
            ("newName", Json::str("left")),
        ]),
    );
    let lines: Vec<u64> = edits_of(&r).iter().map(|e| e.0).collect();
    assert_eq!(lines, vec![1, 5, 5, 11, 12], "{}", r.write());
    // The references of `area`: its definition and its call.
    let r = c.request(
        "textDocument/references",
        Json::obj(vec![
            ("textDocument", Json::obj(vec![("uri", Json::str(&uri))])),
            ("position", pos(12, 17)),
            ("context", Json::obj(vec![("includeDeclaration", Json::Bool(true))])),
        ]),
    );
    let at: Vec<u64> = r
        .as_array()
        .unwrap_or(&[])
        .iter()
        .filter_map(|l| l.at(&["range", "start", "line"]).and_then(Json::as_u64))
        .collect();
    assert_eq!(at, vec![7, 12], "{}", r.write());
    // The standard library's `print` is not the program's to rename.
    let prep = c.request(
        "textDocument/prepareRename",
        Json::obj(vec![
            ("textDocument", Json::obj(vec![("uri", Json::str(&uri))])),
            ("position", pos(13, 5)),
        ]),
    );
    assert_eq!(prep, Json::Null);
    c.stop();
}

#[test]
fn code_actions_carry_the_catalogs_fixes() {
    let mut c = Client::start();
    let text = "def main():\n    let count = 3\n    print(cuont)\n";
    let (uri, _dir) = document("fix.sisu", text);
    c.open(&uri, text);
    let range = Json::obj(vec![
        ("start", Json::obj(vec![("line", Json::num(2)), ("character", Json::num(10))])),
        ("end", Json::obj(vec![("line", Json::num(2)), ("character", Json::num(15))])),
    ]);
    let r = c.request(
        "textDocument/codeAction",
        Json::obj(vec![
            ("textDocument", Json::obj(vec![("uri", Json::str(&uri))])),
            ("range", range),
            ("context", Json::obj(vec![("diagnostics", Json::Arr(Vec::new()))])),
        ]),
    );
    let action = r.as_array().and_then(|a| a.first()).cloned().unwrap_or(Json::Null);
    assert_eq!(action.get("kind").and_then(Json::as_str), Some("quickfix"), "{}", r.write());
    assert!(action.get("title").and_then(Json::as_str).is_some_and(|t| t.contains("count")), "{}", r.write());
    let edit =
        action.at(&["edit", "changes", &uri]).and_then(Json::as_array).and_then(|a| a.first()).cloned();
    let edit = edit.unwrap_or(Json::Null);
    assert_eq!(edit.get("newText").and_then(Json::as_str), Some("count"), "{}", r.write());
    assert_eq!(edit.at(&["range", "start", "character"]).and_then(Json::as_u64), Some(10));
    c.stop();
}

#[test]
fn inlay_hints_show_types_error_sets_and_conventions() {
    let mut c = Client::start();
    let text = "\
error Missing:
    Gone

def load(fs: Fs, name: Path) -> int !:
    let text = fs.read_to_string(name)?
    if text.is_empty():
        return Err(Gone)
    return Ok(len(text))

def bump(mut xs: list[int]):
    xs.append(1)

def main():
    var xs = [1, 2]
    bump(xs)
    let n = len(xs)
    print(n)
";
    let (uri, _dir) = document("hints.sisu", text);
    c.open(&uri, text);
    let range = Json::obj(vec![
        ("start", Json::obj(vec![("line", Json::num(0)), ("character", Json::num(0))])),
        ("end", Json::obj(vec![("line", Json::num(17)), ("character", Json::num(0))])),
    ]);
    let r = c.request(
        "textDocument/inlayHint",
        Json::obj(vec![("textDocument", Json::obj(vec![("uri", Json::str(&uri))])), ("range", range)]),
    );
    let hints: Vec<(u64, String)> = r
        .as_array()
        .unwrap_or(&[])
        .iter()
        .map(|h| {
            (
                h.at(&["position", "line"]).and_then(Json::as_u64).unwrap_or(0),
                h.get("label").and_then(Json::as_str).unwrap_or("").to_string(),
            )
        })
        .collect();
    assert!(hints.contains(&(4, ": String".to_string())), "{hints:?}");
    assert!(
        hints.contains(&(3, "{IoError, Missing}".to_string()))
            || hints.contains(&(3, "{Missing, IoError}".to_string())),
        "{hints:?}"
    );
    assert!(hints.contains(&(14, "mut".to_string())), "{hints:?}");
    assert!(hints.contains(&(15, ": int".to_string())), "{hints:?}");
    c.stop();
}

#[test]
fn explain_borrow_is_a_command() {
    let mut c = Client::start();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../conformance/cases/ownership/explain_borrow.sisu");
    let text = std::fs::read_to_string(&path).expect("the case");
    let (uri, _dir) = document("explain.sisu", &text);
    c.open(&uri, &text);
    let r = c.request(
        "workspace/executeCommand",
        Json::obj(vec![
            ("command", Json::str("sisu.explainBorrow")),
            (
                "arguments",
                Json::Arr(vec![Json::obj(vec![
                    ("textDocument", Json::obj(vec![("uri", Json::str(&uri))])),
                    ("position", Json::obj(vec![("line", Json::num(15)), ("character", Json::num(4))])),
                ])]),
            ),
        ]),
    );
    assert_eq!(r.get("line").and_then(Json::as_u64), Some(16), "{}", r.write());
    let loans = r.get("loans").and_then(Json::as_array).unwrap_or(&[]).to_vec();
    assert_eq!(loans.len(), 2, "{}", r.write());
    assert_eq!(loans[0].get("place").and_then(Json::as_str), Some("text"));
    assert_eq!(
        loans[0].get("message").and_then(Json::as_str),
        Some("a shared borrow of `text` from 15:29, held by `best`, `words`, used until 18:15")
    );
    // Where it starts, as a range of the document (line 15, column 29).
    assert_eq!(loans[0].at(&["start", "start", "line"]).and_then(Json::as_u64), Some(14));
    assert_eq!(loans[0].at(&["start", "start", "character"]).and_then(Json::as_u64), Some(28));
    c.stop();
}

#[test]
fn range_formatting_and_worksheet_values() {
    let mut c = Client::start();
    let messy = "def a( ):\n    print( 1+2 )\n\ndef b( ):\n    print( 3+4 )\n";
    let (uri, _dir) = document("range.sisu", messy);
    c.open(&uri, messy);
    let range = Json::obj(vec![
        ("start", Json::obj(vec![("line", Json::num(3)), ("character", Json::num(0))])),
        ("end", Json::obj(vec![("line", Json::num(4)), ("character", Json::num(0))])),
    ]);
    let r = c.request(
        "textDocument/rangeFormatting",
        Json::obj(vec![
            ("textDocument", Json::obj(vec![("uri", Json::str(&uri))])),
            ("range", range),
            ("options", Json::obj(vec![("tabSize", Json::num(4)), ("insertSpaces", Json::Bool(true))])),
        ]),
    );
    let edits = r.as_array().unwrap_or(&[]).to_vec();
    assert_eq!(edits.len(), 1, "{}", r.write());
    assert_eq!(edits[0].at(&["range", "start", "line"]).and_then(Json::as_u64), Some(3));
    assert_eq!(edits[0].get("newText").and_then(Json::as_str), Some("def b():\n    print(3 + 4)\n"));
    // A worksheet's values, at the ends of their statements' lines.
    let sheet = "let x = 1 + 2\nprint(\"x is {x}\")\nx * 2\n";
    let (uri, _dir2) = document("sheet.sisu", sheet);
    c.open(&uri, sheet);
    let r = c.request(
        "textDocument/inlineValue",
        Json::obj(vec![
            ("textDocument", Json::obj(vec![("uri", Json::str(&uri))])),
            (
                "range",
                Json::obj(vec![
                    ("start", Json::obj(vec![("line", Json::num(0)), ("character", Json::num(0))])),
                    ("end", Json::obj(vec![("line", Json::num(3)), ("character", Json::num(0))])),
                ]),
            ),
            ("context", Json::obj(vec![("frameId", Json::num(0))])),
        ]),
    );
    let values: Vec<(u64, String)> = r
        .as_array()
        .unwrap_or(&[])
        .iter()
        .map(|v| {
            (
                v.at(&["range", "start", "line"]).and_then(Json::as_u64).unwrap_or(99),
                v.get("text").and_then(Json::as_str).unwrap_or("").to_string(),
            )
        })
        .collect();
    assert_eq!(
        values,
        vec![(0, "# => 3".to_string()), (1, "# | x is 3".to_string()), (2, "# => 6".to_string())],
        "{}",
        r.write()
    );
    c.stop();
}

/// Where a body in the middle of a file starts: its first statement.
fn middle_body(text: &str) -> Option<usize> {
    let parse = sisu_parser::parse(sisu_diag::FileId(0), text);
    let module = sisu_syntax::ast::build::module(&parse.tree);
    let mut starts: Vec<usize> = Vec::new();
    for item in &module.items {
        let fns: Vec<&sisu_syntax::ast::FnDef> = match &item.kind {
            sisu_syntax::ast::ItemKind::Fn(f) => vec![f],
            sisu_syntax::ast::ItemKind::Struct(s) => s.methods.iter().collect(),
            sisu_syntax::ast::ItemKind::Impl(i) => i.methods.iter().collect(),
            _ => Vec::new(),
        };
        for f in fns {
            if f.generics.is_empty()
                && let Some(s) = f.body.as_ref().and_then(|b| b.stmts.first())
            {
                starts.push(s.range.start as usize);
            }
        }
    }
    starts.sort();
    starts.get(starts.len() / 2).copied()
}

/// The answers on every module of the standard library, opened as a
/// document: the diagnostics after an edit of one function body (a `pass`
/// line in a body in the middle of the file) and a hover, each against the
/// 100 ms an editor allows (10, "Compile speed as a requirement"). The
/// numbers are printed (`cargo test --release --test lsp latency --
/// --nocapture`); the test fails only past ten times that, since a debug
/// build on a busy machine is not the measure.
#[test]
fn latency() {
    let mut c = Client::start();
    let std_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../std");
    let mut names: Vec<PathBuf> = std::fs::read_dir(&std_dir)
        .expect("std")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "sisu"))
        .collect();
    names.sort();
    let mut worst = (0u128, String::new());
    for path in names {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        let text = std::fs::read_to_string(&path).expect("a std module");
        let uri = path_uri(&std::fs::canonicalize(&path).expect("a path"));
        let start = std::time::Instant::now();
        c.open(&uri, &text);
        let open = start.elapsed();
        let changed = match middle_body(&text) {
            Some(at) => {
                let line_start = text[..at].rfind('\n').map(|i| i + 1).unwrap_or(0);
                format!("{}{}pass\n{}", &text[..line_start], &text[line_start..at], &text[line_start..])
            }
            None => format!("{text}\n"),
        };
        let start = std::time::Instant::now();
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
        if change.as_millis() > worst.0 {
            worst = (change.as_millis(), name.clone());
        }
        assert!(change.as_millis() < 1000 && hover.as_millis() < 1000, "{name}: too slow");
        c.notify(
            "textDocument/didClose",
            Json::obj(vec![("textDocument", Json::obj(vec![("uri", Json::str(&uri))]))]),
        );
        // Closing clears the document's diagnostics.
        let cleared = c.notification("textDocument/publishDiagnostics");
        assert_eq!(cleared.get("uri").and_then(Json::as_str), Some(uri.as_str()));
    }
    println!("slowest change: std/{} in {} ms", worst.1, worst.0);
    c.stop();
}
