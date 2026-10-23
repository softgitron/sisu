//! A line diff for readable failures: `-` lines are expected, `+` lines
//! are what the implementation printed.

/// The diff of two texts by lines, from a longest common subsequence.
pub fn diff(expected: &str, actual: &str) -> String {
    let a: Vec<&str> = expected.lines().collect();
    let b: Vec<&str> = actual.lines().collect();
    if a.len() * b.len() > 4_000_000 {
        return format!(
            "--- expected ({} lines)\n+++ actual ({} lines)\n(too large to diff)\n",
            a.len(),
            b.len()
        );
    }
    // lcs[i][j] = length of the LCS of a[i..] and b[j..].
    let mut lcs = vec![vec![0u32; b.len() + 1]; a.len() + 1];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            lcs[i][j] = if a[i] == b[j] { lcs[i + 1][j + 1] + 1 } else { lcs[i + 1][j].max(lcs[i][j + 1]) };
        }
    }
    let mut out = String::from("--- expected\n+++ actual\n");
    let (mut i, mut j) = (0, 0);
    while i < a.len() || j < b.len() {
        if i < a.len() && j < b.len() && a[i] == b[j] {
            out.push_str(&format!("  {}\n", a[i]));
            i += 1;
            j += 1;
        } else if j < b.len() && (i == a.len() || lcs[i][j + 1] >= lcs[i + 1][j]) {
            out.push_str(&format!("+ {}\n", b[j]));
            j += 1;
        } else {
            out.push_str(&format!("- {}\n", a[i]));
            i += 1;
        }
    }
    if expected.ends_with('\n') != actual.ends_with('\n') {
        out.push_str("(the texts differ in their final line break)\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::diff;

    #[test]
    fn marks_changed_lines() {
        let d = diff("a\nb\nc\n", "a\nx\nc\n");
        assert!(d.contains("- b\n"), "{d}");
        assert!(d.contains("+ x\n"), "{d}");
        assert!(d.contains("  a\n"), "{d}");
    }

    #[test]
    fn notices_a_missing_final_newline() {
        assert!(diff("a\n", "a").contains("final line break"));
    }
}
