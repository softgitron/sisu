//! A line diff, for a snapshot or a transcript that changed (17): the
//! lines both have, unmarked, and the others marked `-` (expected) and
//! `+` (found), from the longest common subsequence of lines.

/// `old` against `new`, one line per line, each marked `  `, `- ` or `+ `.
pub fn lines(old: &str, new: &str) -> String {
    let a: Vec<&str> = old.lines().collect();
    let b: Vec<&str> = new.lines().collect();
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
    out.join("\n")
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_diff_marks_what_changed() {
        assert_eq!(super::lines("a\nb\nc", "a\nx\nc"), "  a\n- b\n+ x\n  c");
        assert_eq!(super::lines("", "new"), "+ new");
        assert_eq!(super::lines("same", "same"), "  same");
    }
}
