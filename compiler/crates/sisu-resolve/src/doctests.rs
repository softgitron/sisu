//! Doc examples as tests (10, "Testing": "Doc examples are tests"; 17,
//! "Doc examples in `##` comments are compiled ... and rendered as the
//! Examples section").
//!
//! An example is a fenced block in a run of `##` lines, opened by a line
//! "```" or "```sisu" and closed by a line "```"; its lines are the body
//! of a test named after the item the run documents
//! (`gcd (doc example)`, `Deque.push_back (doc example 2)`). The tests are
//! written as a module of their own whose text keeps every example line
//! where it is in the file, with the `##` turned into spaces, so a
//! diagnostic or a failure in an example points at the example itself;
//! the module's scope is the documented module's.

/// One fenced example.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Example {
    /// The documented item's name, `Type.method` for a method.
    pub item: String,
    /// Which example of that item it is, from 1.
    pub ordinal: usize,
    /// The line (0-based) of its opening fence, and the column (in
    /// characters) of the `##` there.
    pub fence_line: usize,
    pub indent: usize,
    /// Each line of the example: its line (0-based), the column its text
    /// starts at, and the text.
    pub lines: Vec<(usize, usize, String)>,
}

impl Example {
    /// The test's name.
    pub fn test_name(&self) -> String {
        if self.ordinal == 1 {
            format!("{} (doc example)", self.item)
        } else {
            format!("{} (doc example {})", self.item, self.ordinal)
        }
    }
}

/// The text of a `##` comment line after the marker, with the column (in
/// characters) the marker starts at; `None` for any other line (`##!` is a
/// module's doc, not an item's).
fn doc_line(line: &str) -> Option<(usize, &str)> {
    let indent = line.len() - line.trim_start_matches(' ').len();
    let rest = &line[indent..];
    if !rest.starts_with("##") || rest.starts_with("##!") {
        return None;
    }
    Some((indent, &rest[2..]))
}

/// The examples in a module's text. `items` gives the byte offset where
/// each documented item starts and its name; a run of `##` lines documents
/// the first item starting after it.
pub fn examples(text: &str, items: &[(u32, String)]) -> Vec<Example> {
    let mut out: Vec<Example> = Vec::new();
    let mut offset = 0usize;
    // The examples of the current run of `##` lines, waiting for the item
    // they document.
    let mut run: Vec<Example> = Vec::new();
    let mut open: Option<Example> = None;
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut finish_run = |run: &mut Vec<Example>, at: usize, out: &mut Vec<Example>| {
        if run.is_empty() {
            return;
        }
        let item = items
            .iter()
            .filter(|(start, _)| *start as usize >= at)
            .min_by_key(|(start, _)| *start)
            .map(|(_, n)| n.clone());
        for mut e in run.drain(..) {
            if let Some(name) = &item {
                let n = counts.entry(name.clone()).or_insert(0);
                *n += 1;
                e.item = name.clone();
                e.ordinal = *n;
                out.push(e);
            }
        }
    };
    for (index, line) in text.split_inclusive('\n').enumerate() {
        let start = offset;
        offset += line.len();
        let bare = line.trim_end_matches(['\n', '\r']);
        match doc_line(bare) {
            Some((indent, content)) => {
                let fence = content.trim();
                match open.take() {
                    Some(e) if fence == "```" => run.push(e),
                    Some(mut e) => {
                        // `## text`: the text starts after the one space.
                        let (col, body) = match content.strip_prefix(' ') {
                            Some(b) => (indent + 3, b),
                            None => (indent + 2, content),
                        };
                        if !body.trim().is_empty() {
                            e.lines.push((index, col, body.to_string()));
                        }
                        open = Some(e);
                    }
                    None if fence == "```" || fence == "```sisu" => {
                        open = Some(Example {
                            item: String::new(),
                            ordinal: 0,
                            fence_line: index,
                            indent,
                            lines: Vec::new(),
                        });
                    }
                    None => {}
                }
            }
            None => {
                // The run ends at any other line; an example left open is
                // not one.
                open = None;
                finish_run(&mut run, start, &mut out);
            }
        }
    }
    finish_run(&mut run, text.len(), &mut out);
    out.retain(|e| !e.lines.is_empty());
    out
}

/// A module of tests, one per example, whose text has as many lines as
/// `text` and keeps every example line where it is: the opening fence's
/// line holds the test's header, the example's lines its body, and every
/// other line is empty.
pub fn test_module(text: &str, examples: &[Example]) -> String {
    let count = text.split_inclusive('\n').count().max(1);
    let mut lines: Vec<String> = vec![String::new(); count];
    for e in examples {
        // Every test is an item of the module: its header starts the line,
        // and the example's lines are indented past it wherever they are.
        if let Some(l) = lines.get_mut(e.fence_line) {
            let name = sisu_diag::render::quote(&e.test_name());
            *l = format!("test {name}:");
        }
        for (line, col, body) in &e.lines {
            if let Some(l) = lines.get_mut(*line) {
                *l = format!("{}{body}", " ".repeat(*col));
            }
        }
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_examples_and_their_items() {
        let text = "## Greatest common divisor.\n## ```\n## assert gcd(4, 6) == 2\n##\n## assert gcd(1, 1) == 1\n## ```\n## ```text\n## not code\n## ```\ndef gcd(a: int, b: int) -> int:\n    return a\n\nstruct S:\n    ## ```sisu\n    ## let s = S()\n    ## ```\n    def m(self):\n        pass\n";
        let def = text.find("def gcd").unwrap() as u32;
        let m = text.find("def m").unwrap() as u32;
        let items = vec![(def, "gcd".to_string()), (m, "S.m".to_string())];
        let ex = examples(text, &items);
        assert_eq!(ex.len(), 2);
        assert_eq!(ex[0].item, "gcd");
        assert_eq!(
            ex[0].lines,
            vec![(2, 3, "assert gcd(4, 6) == 2".into()), (4, 3, "assert gcd(1, 1) == 1".into())]
        );
        assert_eq!(ex[1].item, "S.m");
        assert_eq!(ex[1].lines[0], (14, 7, "let s = S()".into()));
        let module = test_module(text, &ex);
        let lines: Vec<&str> = module.lines().collect();
        assert_eq!(lines[1], "test \"gcd (doc example)\":");
        assert_eq!(lines[2], "   assert gcd(4, 6) == 2");
        assert_eq!(lines[13], "test \"S.m (doc example)\":");
        assert_eq!(text.lines().count(), module.lines().count());
    }

    #[test]
    fn an_unclosed_fence_is_not_an_example() {
        let text = "## ```\n## assert False\ndef f():\n    pass\n";
        assert!(examples(text, &[(text.find("def").unwrap() as u32, "f".into())]).is_empty());
    }
}
