//! Snapshot files (17, "Snapshots and transcripts"): the snapshots of
//! `PROGRAM.sisu` are kept in `PROGRAM.snap` beside it, which `sisu test
//! --update` writes and a person reviews like code.
//!
//! ```text
//! # The snapshots of `shop.sisu` (17): `sisu test --update` rewrites this file.
//!
//! --- help text is stable
//!   usage: shop COMMAND
//!   ...
//! ```
//!
//! Each snapshot is a line `--- NAME` (the test's name, then ` #2` and on
//! for a test's later snapshots), then its text, each line indented by
//! two spaces; an empty line between snapshots is not part of either.

use std::collections::BTreeMap;

/// The snapshots a file holds, by name.
pub fn parse(text: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut current: Option<(String, Vec<&str>)> = None;
    for line in text.lines() {
        if let Some(name) = line.strip_prefix("--- ") {
            if let Some((n, body)) = current.take() {
                out.insert(n, body.join("\n"));
            }
            current = Some((name.to_string(), Vec::new()));
        } else if let Some((_, body)) = &mut current
            && let Some(content) = line.strip_prefix("  ")
        {
            body.push(content);
        }
    }
    if let Some((n, body)) = current {
        out.insert(n, body.join("\n"));
    }
    out
}

/// The text of a snapshot file for `program`, its snapshots in `order`.
pub fn write(program: &str, snaps: &[(String, String)]) -> String {
    let mut s = format!("# The snapshots of `{program}` (17): `sisu test --update` rewrites this file.\n");
    for (name, text) in snaps {
        s.push_str(&format!("\n--- {name}\n"));
        for line in text.split('\n') {
            s.push_str("  ");
            s.push_str(line);
            s.push('\n');
        }
    }
    s
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_snapshot_file_reads_back() {
        let snaps = vec![
            ("one".to_string(), "a\n\n--- b\n".to_string()),
            ("one #2".to_string(), "Point(x=1)".to_string()),
        ];
        let text = super::write("p.sisu", &snaps);
        let back = super::parse(&text);
        assert_eq!(back.len(), 2);
        assert_eq!(back["one"], "a\n\n--- b\n");
        assert_eq!(back["one #2"], "Point(x=1)");
    }
}
