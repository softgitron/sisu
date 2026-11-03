//! Where two ELF files differ, for the checks that compare executables and
//! objects byte for byte (the fixed point of `stage2/bootstrap.sh`, the
//! driver's tests of the Sisu backend): the sizes, how many bytes differ,
//! and, by section of the first file, how many of them are there and the
//! first run of them with the bytes on both sides. A pair that is the same
//! gives an empty text.

use std::fmt::Write;

/// A description of how `a` and `b` differ, empty when they are the same.
pub fn differences(a: &[u8], b: &[u8]) -> String {
    if a == b {
        return String::new();
    }
    let mut out = String::new();
    let _ = writeln!(out, "sizes {} and {} bytes", a.len(), b.len());
    let common = a.len().min(b.len());
    let differing = (0..common).filter(|&i| a[i] != b[i]).count() + (a.len().max(b.len()) - common);
    let _ = writeln!(out, "{differing} bytes differ");
    // The runs of differing bytes in the part both have.
    let mut runs: Vec<(usize, usize)> = Vec::new();
    let mut i = 0;
    while i < common {
        if a[i] == b[i] {
            i += 1;
            continue;
        }
        let start = i;
        while i < common && a[i] != b[i] {
            i += 1;
        }
        runs.push((start, i));
    }
    let sections = sections(a);
    let mut shown_outside = false;
    let mut rest = Vec::new();
    for s in &sections {
        let inside: Vec<&(usize, usize)> =
            runs.iter().filter(|(start, _)| s.offset <= *start && *start < s.offset + s.size).collect();
        if inside.is_empty() {
            continue;
        }
        let count: usize = inside.iter().map(|(start, end)| end - start).sum();
        let (start, end) = **inside.first().unwrap_or(&&(0, 0));
        let _ = writeln!(
            out,
            "  {:<22} {count:>9} bytes in {} runs, the first at +{:#x} ({start:#x}..{end:#x}): {} | {}",
            s.name,
            inside.len(),
            start - s.offset,
            hex(&a[start..(start + 16).min(end)]),
            hex(&b[start..(start + 16).min(end)])
        );
    }
    for (start, end) in &runs {
        if !sections.iter().any(|s| s.offset <= *start && *start < s.offset + s.size) {
            rest.push((*start, *end));
        }
    }
    if !rest.is_empty() {
        let count: usize = rest.iter().map(|(start, end)| end - start).sum();
        let (start, end) = rest.first().copied().unwrap_or((0, 0));
        let where_ = if sections.is_empty() { "in all" } else { "outside every section" };
        let _ = writeln!(
            out,
            "  {where_:<22} {count:>9} bytes in {} runs, the first at {start:#x}..{end:#x}: {} | {}",
            rest.len(),
            hex(&a[start..(start + 16).min(end)]),
            hex(&b[start..(start + 16).min(end)])
        );
        shown_outside = true;
    }
    if a.len() != b.len() {
        let _ = writeln!(out, "  the first {common} bytes compared; the rest is only in the longer");
    }
    if !sections.is_empty() && !shown_outside && runs.is_empty() {
        let _ = writeln!(out, "  (the part both have is the same)");
    }
    out
}

/// Bytes as hex, two digits each.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ")
}

/// A section of an ELF file: its name and where its bytes are in the file.
struct Section {
    name: String,
    offset: usize,
    size: usize,
}

/// The sections of a 64-bit little-endian ELF file that take room in it,
/// by offset; none when the bytes are not such a file (then the
/// differences are reported by offset alone).
fn sections(file: &[u8]) -> Vec<Section> {
    let word = |at: usize| -> Option<usize> {
        let bytes: [u8; 8] = file.get(at..at + 8)?.try_into().ok()?;
        usize::try_from(u64::from_le_bytes(bytes)).ok()
    };
    let half = |at: usize| -> Option<usize> {
        let bytes: [u8; 2] = file.get(at..at + 2)?.try_into().ok()?;
        Some(usize::from(u16::from_le_bytes(bytes)))
    };
    let quad = |at: usize| -> Option<usize> {
        let bytes: [u8; 4] = file.get(at..at + 4)?.try_into().ok()?;
        usize::try_from(u32::from_le_bytes(bytes)).ok()
    };
    let header = || -> Option<Vec<Section>> {
        if file.get(0..4)? != b"\x7fELF" || *file.get(4)? != 2 || *file.get(5)? != 1 {
            return None;
        }
        let shoff = word(0x28)?;
        let shentsize = half(0x3a)?;
        let shnum = half(0x3c)?;
        let shstrndx = half(0x3e)?;
        if shentsize < 0x40 || shnum == 0 {
            return None;
        }
        let at = |k: usize| shoff + k * shentsize;
        // Section names are in the string table the header names.
        let names_offset = word(at(shstrndx) + 0x18)?;
        let names_size = word(at(shstrndx) + 0x20)?;
        let names = file.get(names_offset..names_offset + names_size)?;
        let mut out = Vec::with_capacity(shnum);
        for k in 0..shnum {
            let name_at = quad(at(k))?;
            let kind = quad(at(k) + 4)?;
            let offset = word(at(k) + 0x18)?;
            let size = word(at(k) + 0x20)?;
            let name = names
                .get(name_at..)
                .and_then(|rest| rest.split(|&b| b == 0).next())
                .map(|n| String::from_utf8_lossy(n).into_owned())
                .unwrap_or_default();
            // SHT_NOBITS (8) takes no room in the file.
            if kind == 8 || name.is_empty() || size == 0 {
                continue;
            }
            out.push(Section { name, offset, size });
        }
        out.sort_by_key(|s| s.offset);
        Some(out)
    };
    header().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::differences;

    #[test]
    fn the_same_bytes_give_nothing() {
        assert_eq!(differences(b"abc", b"abc"), "");
    }

    #[test]
    fn differing_bytes_are_located() {
        let a = b"the quick brown fox".to_vec();
        let mut b = a.clone();
        b[4] = b'Q';
        b[10] = b'B';
        b[11] = b'R';
        let text = differences(&a, &b);
        assert!(text.contains("sizes 19 and 19 bytes"), "{text}");
        assert!(text.contains("3 bytes differ"), "{text}");
        assert!(
            text.contains("in all                         3 bytes in 2 runs, the first at 0x4..0x5: 71 | 51"),
            "{text}"
        );
    }

    #[test]
    fn a_longer_file_is_said_so() {
        let text = differences(b"abc", b"abcd");
        assert!(text.contains("sizes 3 and 4 bytes"), "{text}");
        assert!(text.contains("1 bytes differ"), "{text}");
        assert!(text.contains("the rest is only in the longer"), "{text}");
    }
}
