//! A line diff, for a snapshot or a transcript that changed (17): the
//! lines both have, unmarked, and the others marked `-` (expected) and
//! `+` (found), from the longest common subsequence of lines.

/// `old` against `new`, one line per line, each marked `  `, `- ` or `+ `.
pub fn lines(old: &str, new: &str) -> String {
    let a: Vec<&str> = old.lines().collect();
    let b: Vec<&str> = new.lines().collect();
    marks(&a, &b).join("\n")
}

/// The lines of `a` and `b` marked, from their longest common subsequence.
fn marks(a: &[&str], b: &[&str]) -> Vec<String> {
    // The length of the longest common subsequence of each pair of
    // suffixes.
    let mut lcs = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            lcs[i][j] = if a[i] == b[j] { lcs[i + 1][j + 1] + 1 } else { lcs[i + 1][j].max(lcs[i][j + 1]) };
        }
    }
    let mut out = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < a.len() || j < b.len() {
        if i < a.len() && j < b.len() && a[i] == b[j] {
            out.push(format!("  {}", a[i]));
            i += 1;
            j += 1;
        } else if i < a.len() && (j == b.len() || lcs[i + 1][j] >= lcs[i][j + 1]) {
            out.push(format!("- {}", a[i]));
            i += 1;
        } else {
            out.push(format!("+ {}", b[j]));
            j += 1;
        }
    }
    out
}

/// Only the lines that changed, with `context` lines around each run of
/// them, under a header naming the first line shown (`@@ line N @@`):
/// what `sisu fmt --check` prints. The lines both texts start and end
/// with are set aside before the common subsequence is computed, so a
/// small change in a long file costs little.
pub fn hunks(old: &str, new: &str, context: usize) -> String {
    let a: Vec<&str> = old.lines().collect();
    let b: Vec<&str> = new.lines().collect();
    let prefix = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let suffix = a[prefix..].iter().rev().zip(b[prefix..].iter().rev()).take_while(|(x, y)| x == y).count();
    let middle = marks(&a[prefix..a.len() - suffix], &b[prefix..b.len() - suffix]);
    // Every line in order, with its line number in `old` when it has one.
    let mut marked: Vec<(Option<usize>, String)> = Vec::new();
    for (i, l) in a[..prefix].iter().enumerate() {
        marked.push((Some(i + 1), format!("  {l}")));
    }
    let mut at = prefix;
    for l in middle {
        if l.starts_with("+ ") {
            marked.push((None, l));
        } else {
            at += 1;
            marked.push((Some(at), l));
        }
    }
    for (i, l) in a[a.len() - suffix..].iter().enumerate() {
        marked.push((Some(a.len() - suffix + i + 1), format!("  {l}")));
    }
    let changed: Vec<usize> =
        marked.iter().enumerate().filter(|(_, (_, l))| !l.starts_with("  ")).map(|(i, _)| i).collect();
    let mut out: Vec<String> = Vec::new();
    let mut shown_to = 0;
    let mut k = 0;
    while k < changed.len() {
        let start = changed[k].saturating_sub(context).max(shown_to);
        let mut end = changed[k] + 1;
        while k + 1 < changed.len() && changed[k + 1] <= end + 2 * context {
            k += 1;
            end = changed[k] + 1;
        }
        let end = (end + context).min(marked.len());
        let line = marked[start..].iter().find_map(|(n, _)| *n).unwrap_or(a.len() + 1);
        out.push(format!("@@ line {line} @@"));
        out.extend(marked[start..end].iter().map(|(_, l)| l.clone()));
        shown_to = end;
        k += 1;
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    #[test]
    fn hunks_show_what_changed_with_context() {
        let old = "a\nb\nc\nd\ne\nf\ng\nh";
        let new = "a\nb\nc\nD\ne\nf\ng\nh";
        assert_eq!(super::hunks(old, new, 1), "@@ line 3 @@\n  c\n- d\n+ D\n  e");
        assert_eq!(super::hunks("x\n\n\ny", "x\n\ny", 1), "@@ line 2 @@\n  \n- \n  y");
        let far = super::hunks("1\n2\n3\n4\n5\n6\n7\n8\n9", "0\n2\n3\n4\n5\n6\n7\n8\n0", 1);
        assert_eq!(far, "@@ line 1 @@\n- 1\n+ 0\n  2\n@@ line 8 @@\n  8\n- 9\n+ 0");
    }

    #[test]
    fn a_diff_marks_what_changed() {
        assert_eq!(super::lines("a\nb\nc", "a\nx\nc"), "  a\n- b\n+ x\n  c");
        assert_eq!(super::lines("", "new"), "+ new");
        assert_eq!(super::lines("same", "same"), "  same");
    }
}
