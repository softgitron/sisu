//! Transcript tests (17, "Snapshots and transcripts"; D-070): `sisu test`
//! runs every `tests/*.transcript` beside the program. A `$` line runs the
//! program's `main` with the words after the program's name as its
//! arguments, the `>` lines after it are its standard input, and every
//! other line until the next `$` is what it must print (standard output
//! and error as they were written, but a failure's trace, whose lines move
//! with the program's); a run that exits with a status other than 0 ends
//! with ` (exit N)` on its last line. The runs share a fake
//! `System`: an in-memory `Fs` holding the files of `tests/NAME/` (for
//! `tests/NAME.transcript`), a clock frozen at the epoch, `Rand.seeded(0)`
//! and no network; each file starts afresh.
//!
//! ```text
//! $ shop price
//! > apples
//! apples: 0.40
//! $ shop buy pears 1
//! error: OutOfStock(item="pears") (exit 1)
//! ```

use std::path::{Path, PathBuf};

/// One `$` line and what follows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Run {
    /// The `$` line as written.
    pub command: String,
    pub args: Vec<String>,
    /// The `>` lines, without the `> `.
    pub stdin: Vec<String>,
    /// The `>` lines as written, which a rewrite keeps.
    pub input_lines: Vec<String>,
    pub expected: Vec<String>,
}

/// A transcript: what precedes the first `$` (kept as it is), and its runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transcript {
    pub preamble: Vec<String>,
    pub runs: Vec<Run>,
}

/// The transcripts of the program `file`, by path, in order of name.
pub fn find(file: &str) -> Vec<PathBuf> {
    let dir = Path::new(file).parent().map(Path::to_path_buf).unwrap_or_default();
    let tests = if dir.as_os_str().is_empty() { PathBuf::from("tests") } else { dir.join("tests") };
    let Ok(entries) = std::fs::read_dir(&tests) else { return Vec::new() };
    let mut out: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "transcript"))
        .collect();
    out.sort();
    out
}

/// The words of a `$` line: split at spaces, a word in double quotes kept
/// whole.
fn words(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut any = false;
    for c in line.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                any = true;
            }
            c if c.is_whitespace() && !quoted => {
                if any || !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                    any = false;
                }
            }
            c => cur.push(c),
        }
    }
    if any || !cur.is_empty() {
        out.push(cur);
    }
    out
}

pub fn parse(text: &str) -> Transcript {
    let mut t = Transcript { preamble: Vec::new(), runs: Vec::new() };
    for line in text.lines() {
        if let Some(cmd) = line.strip_prefix("$ ") {
            let mut ws = words(cmd);
            if !ws.is_empty() {
                ws.remove(0);
            }
            t.runs.push(Run {
                command: line.to_string(),
                args: ws,
                stdin: Vec::new(),
                input_lines: Vec::new(),
                expected: Vec::new(),
            });
            continue;
        }
        match t.runs.last_mut() {
            None => t.preamble.push(line.to_string()),
            Some(r) => match line.strip_prefix("> ").or_else(|| (line == ">").then_some("")) {
                Some(input) if r.expected.is_empty() => {
                    r.stdin.push(input.to_string());
                    r.input_lines.push(line.to_string());
                }
                _ => r.expected.push(line.to_string()),
            },
        }
    }
    // Blank lines at the end of the file belong to no run.
    if let Some(r) = t.runs.last_mut() {
        while r.expected.last().is_some_and(|l| l.is_empty()) {
            r.expected.pop();
        }
    }
    t
}

/// The lines a run printed, as a transcript writes them: a failure's
/// trace, the `at` lines after an `error:` or a `panic:` line, is left out,
/// so that a transcript does not change when the program's lines move
/// (decision 428), as 17's own transcript shows an error.
pub fn shown(output: &str, status: i32) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut failing = false;
    for l in output.lines() {
        if failing && is_frame(l) {
            continue;
        }
        failing |= l.starts_with("error: ") || l.starts_with("panic: ");
        lines.push(l.to_string());
    }
    if status != 0 {
        match lines.last_mut() {
            Some(l) => l.push_str(&format!(" (exit {status})")),
            None => lines.push(format!("(exit {status})")),
        }
    }
    lines
}

/// Whether a line is a frame of a trace: `  at FILE:LINE:COL`, a return
/// trace's, or `  at FUNCTION (FILE:LINE:COL)`, a panic's.
fn is_frame(line: &str) -> bool {
    let Some(rest) = line.strip_prefix("  at ") else { return false };
    let rest = rest.strip_suffix(')').unwrap_or(rest);
    let mut parts = rest.rsplitn(3, ':');
    let number = |p: Option<&str>| p.is_some_and(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()));
    number(parts.next()) && number(parts.next()) && parts.next().is_some_and(|f| !f.is_empty())
}

/// The transcript's text, with each run's expected lines as given.
pub fn write(t: &Transcript, outputs: &[Vec<String>]) -> String {
    let mut s = String::new();
    for l in &t.preamble {
        s.push_str(l);
        s.push('\n');
    }
    for (i, r) in t.runs.iter().enumerate() {
        s.push_str(&r.command);
        s.push('\n');
        for l in &r.input_lines {
            s.push_str(l);
            s.push('\n');
        }
        for l in outputs.get(i).unwrap_or(&r.expected) {
            s.push_str(l);
            s.push('\n');
        }
    }
    s
}

/// The fixture files of the transcript at `path`: those under the
/// directory of its name, by path relative to it, as text.
pub fn fixtures(path: &Path) -> Vec<(String, String)> {
    let dir = path.with_extension("");
    let mut out = Vec::new();
    let mut stack = vec![dir.clone()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else { continue };
        for e in entries.filter_map(Result::ok) {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if let (Ok(text), Ok(rel)) = (std::fs::read_to_string(&p), p.strip_prefix(&dir)) {
                out.push((rel.to_string_lossy().replace('\\', "/"), text));
            }
        }
    }
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_transcript_reads_and_writes_back() {
        let text = "# shop\n$ shop price \"green apples\"\n> apples\n>\napples: 0.40\n$ shop buy pears 1\nerror (exit 1)\n";
        let t = parse(text);
        assert_eq!(t.preamble, vec!["# shop"]);
        assert_eq!(t.runs.len(), 2);
        assert_eq!(t.runs[0].args, vec!["price", "green apples"]);
        assert_eq!(t.runs[0].stdin, vec!["apples", ""]);
        assert_eq!(t.runs[0].expected, vec!["apples: 0.40"]);
        assert_eq!(write(&t, &[]), text);
        assert_eq!(shown("a\nb\n", 1), vec!["a", "b (exit 1)"]);
        let failed = "error: NotFound(path=\"stock.txt\")\n  at shop.sisu:12:16\n  at shop.sisu:29:13\n";
        assert_eq!(shown(failed, 1), vec!["error: NotFound(path=\"stock.txt\") (exit 1)"]);
        let panicked = "panic: assertion failed: x\n  left:  1\n  at main (shop.sisu:3:5)\n";
        assert_eq!(shown(panicked, 101), vec!["panic: assertion failed: x", "  left:  1 (exit 101)"]);
        assert_eq!(shown("  at a.sisu:1:2\n", 0), vec!["  at a.sisu:1:2"]);
        assert_eq!(shown("", 2), vec!["(exit 2)"]);
    }
}
