//! The layout documents of one logical line and their printer: Wadler's
//! "prettier printer" with groups that print on one line when they fit and
//! break otherwise, as Prettier implements it. A broken group's line
//! breaks become new lines, indented by the group's indentation; a
//! trailing comment waits for the next line break; a comment on a line of
//! its own forces the lines around it.

/// One piece of a layout.
#[derive(Clone, Debug, PartialEq)]
pub enum Doc {
    Text(String),
    /// A space when its group is flat, a line break when it is broken.
    Line,
    /// Nothing when its group is flat, a line break when it is broken.
    Soft,
    /// Its contents `n` columns deeper after each of their line breaks.
    Indent(usize, Vec<Doc>),
    Group(Group),
    /// The text only when the enclosing group is broken: a trailing comma.
    IfBreak(String),
    /// A trailing comment: printed before the next line break.
    Suffix(String),
    /// A comment on a line of its own: a line break before it unless the
    /// line is empty, and one after it before anything else.
    Comment(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    pub docs: Vec<Doc>,
    /// Broken whatever the width: a magic trailing comma, a comment.
    pub force: bool,
    /// Flat only when the rest of the line fits flat after it too, rather
    /// than up to the rest's first possible line break: a definition's
    /// parameters break before its return type does (Black's left-hand
    /// split of a `def`).
    pub whole_line: bool,
}

impl Doc {
    pub fn group(docs: Vec<Doc>, force: bool) -> Doc {
        Doc::Group(Group { docs, force, whole_line: false })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Flat,
    Break,
}

/// The width of a text as the formatter counts it: characters.
fn width(s: &str) -> usize {
    s.chars().count()
}

/// Marks every group that holds a comment or a forced group as forced, so
/// it breaks whatever the width: a comment must end its line. Returns
/// whether the docs force their group.
fn propagate(docs: &mut [Doc]) -> bool {
    let mut force = false;
    for d in docs.iter_mut() {
        match d {
            Doc::Comment(_) | Doc::Suffix(_) => force = true,
            Doc::Indent(_, inner) => force |= propagate(inner),
            Doc::Group(g) => {
                let inner = propagate(&mut g.docs);
                g.force |= inner;
                force |= g.force;
            }
            _ => {}
        }
    }
    force
}

/// Whether the docs fit in `room` columns printed in `mode`, together
/// with what follows them up to its first line break (`rest`, each with
/// its own mode, or all flat when `rest_flat`).
fn fits(next: (Mode, &[Doc]), rest: &[(usize, Mode, &Doc)], room: isize, rest_flat: bool) -> bool {
    let mut room = room;
    let mut stack: Vec<(Mode, &Doc)> = next.1.iter().rev().map(|d| (next.0, d)).collect();
    let mut rest_at = rest.len();
    loop {
        if room < 0 {
            return false;
        }
        let Some((mode, doc)) = stack.pop() else {
            if rest_at == 0 {
                return true;
            }
            rest_at -= 1;
            let (_, m, d) = rest[rest_at];
            stack.push((if rest_flat { Mode::Flat } else { m }, d));
            continue;
        };
        match doc {
            Doc::Text(s) => room -= width(s) as isize,
            Doc::Line => match mode {
                Mode::Break => return true,
                Mode::Flat => room -= 1,
            },
            Doc::Soft => {
                if mode == Mode::Break {
                    return true;
                }
            }
            Doc::Comment(_) => return true,
            Doc::Indent(_, docs) => stack.extend(docs.iter().rev().map(|d| (mode, d))),
            Doc::Group(g) => {
                let m = if g.force { Mode::Break } else { mode };
                stack.extend(g.docs.iter().rev().map(|d| (m, d)));
            }
            Doc::IfBreak(s) => {
                if mode == Mode::Break {
                    room -= width(s) as isize;
                }
            }
            Doc::Suffix(_) => {}
        }
    }
}

/// The printer's position: the text so far, its column, and whether the
/// current line holds anything but its indentation.
struct Out {
    text: String,
    col: usize,
    fresh: bool,
    suffixes: Vec<String>,
}

impl Out {
    fn newline(&mut self, at: usize) {
        for s in self.suffixes.drain(..) {
            self.text.push_str(&s);
        }
        while self.text.ends_with(' ') {
            self.text.pop();
        }
        self.text.push('\n');
        self.text.push_str(&" ".repeat(at));
        self.col = at;
        self.fresh = true;
    }

    fn push(&mut self, s: &str) {
        self.text.push_str(s);
        self.col += width(s);
        self.fresh = false;
    }
}

/// Prints the docs of one logical line at indentation `indent` in a line
/// of `max` columns. The result has no final line break.
pub fn print(docs: &[Doc], indent: usize, max: usize) -> String {
    let mut docs = docs.to_vec();
    propagate(&mut docs);
    let mut out = Out { text: " ".repeat(indent), col: indent, fresh: true, suffixes: Vec::new() };
    // Set after a comment on a line of its own: what comes next starts a
    // new line.
    let mut need_break = false;
    let mut stack: Vec<(usize, Mode, &Doc)> = docs.iter().rev().map(|d| (indent, Mode::Break, d)).collect();
    while let Some((ind, mode, doc)) = stack.pop() {
        match doc {
            Doc::Text(s) => {
                if need_break {
                    out.newline(ind);
                    need_break = false;
                }
                out.push(s);
            }
            Doc::Line | Doc::Soft => {
                if mode == Mode::Break || need_break {
                    out.newline(ind);
                    need_break = false;
                } else if *doc == Doc::Line {
                    out.push(" ");
                }
            }
            Doc::Comment(c) => {
                if !out.fresh {
                    out.newline(ind);
                }
                out.push(c);
                need_break = true;
            }
            Doc::Indent(n, inner) => stack.extend(inner.iter().rev().map(|d| (ind + n, mode, d))),
            Doc::Group(g) => {
                let m = if g.force {
                    Mode::Break
                } else if mode == Mode::Flat {
                    Mode::Flat
                } else {
                    let room = max as isize - out.col as isize;
                    if fits((Mode::Flat, &g.docs), &stack, room, g.whole_line) {
                        Mode::Flat
                    } else {
                        Mode::Break
                    }
                };
                stack.extend(g.docs.iter().rev().map(|d| (ind, m, d)));
            }
            Doc::IfBreak(s) => {
                if mode == Mode::Break {
                    out.push(s);
                }
            }
            Doc::Suffix(s) => out.suffixes.push(s.clone()),
        }
    }
    let Out { mut text, suffixes, .. } = out;
    for s in suffixes {
        text.push_str(&s);
    }
    while text.ends_with(' ') {
        text.pop();
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> Doc {
        Doc::Text(s.into())
    }

    fn call(name: &str, args: &[&str]) -> Vec<Doc> {
        let mut inner = Vec::new();
        for (i, a) in args.iter().enumerate() {
            if i > 0 {
                inner.push(t(","));
                inner.push(Doc::Line);
            }
            inner.push(t(a));
        }
        inner.push(Doc::IfBreak(",".into()));
        vec![Doc::group(
            vec![
                t(&format!("{name}(")),
                Doc::Indent(4, vec![Doc::Soft, Doc::group(inner, false)]),
                Doc::Soft,
                t(")"),
            ],
            false,
        )]
    }

    #[test]
    fn flat_when_it_fits() {
        assert_eq!(print(&call("f", &["a", "b"]), 0, 100), "f(a, b)");
    }

    #[test]
    fn hugs_then_breaks_one_per_line() {
        assert_eq!(print(&call("function", &["aaaa", "bbbb"]), 0, 16), "function(\n    aaaa, bbbb\n)");
        assert_eq!(print(&call("function", &["aaaa", "bbbb"]), 0, 12), "function(\n    aaaa,\n    bbbb,\n)");
    }

    #[test]
    fn a_trailing_comment_waits_for_the_line_break() {
        let docs = vec![t("x"), Doc::Suffix("  # c".into())];
        assert_eq!(print(&docs, 4, 100), "    x  # c");
    }
}
