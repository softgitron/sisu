//! `sisu build --native --explain-size` (10, "Binary size"; decision 944):
//! the parts of a linked executable, read from the file itself as decision
//! 682 measured them. The code is attributed by its function symbols: the
//! runtime in Sisu's are those its object defines (`target/rt/rt.o`), the
//! runtime in Rust's those of its crate (`sisu_rt`, by their mangled names,
//! and its `sisu_rt_` entry points), the program's own the rest that the
//! backend named (`sisu$...` and the C `main`), and what is left of the
//! code Rust's standard library's and the C runtime's. The debug
//! information is the `.debug_*` sections, the symbols the symbol table and
//! its names, and the rest is the data, the unwinding tables and the
//! headers, so that the parts add up to the file's size.

use std::collections::HashSet;

/// The sizes, in bytes, of an executable's parts.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Sizes {
    pub total: u64,
    pub program: u64,
    pub rt_sisu: u64,
    pub rt_rust: u64,
    pub other_code: u64,
    pub debug: u64,
    pub symbols: u64,
    pub rest: u64,
}

struct Section {
    name: String,
    kind: u32,
    flags: u64,
    offset: u64,
    size: u64,
    link: u32,
    entsize: u64,
}

struct Elf<'a> {
    data: &'a [u8],
    sections: Vec<Section>,
}

const SHT_SYMTAB: u32 = 2;
const SHT_NOBITS: u32 = 8;
const SHF_EXECINSTR: u64 = 4;
const STT_FUNC: u8 = 2;

fn word(d: &[u8], at: usize, n: usize) -> Option<u64> {
    let b = d.get(at..at + n)?;
    let mut v = 0u64;
    for (i, x) in b.iter().enumerate() {
        v |= u64::from(*x) << (8 * i);
    }
    Some(v)
}

impl<'a> Elf<'a> {
    /// A 64-bit little-endian ELF file's sections, or `None` for another
    /// file.
    fn read(data: &'a [u8]) -> Option<Elf<'a>> {
        if data.get(0..4)? != b"\x7fELF" || *data.get(4)? != 2 || *data.get(5)? != 1 {
            return None;
        }
        let shoff = word(data, 0x28, 8)? as usize;
        let shentsize = word(data, 0x3a, 2)? as usize;
        let shnum = word(data, 0x3c, 2)? as usize;
        let shstrndx = word(data, 0x3e, 2)? as usize;
        let mut raw = Vec::with_capacity(shnum);
        for i in 0..shnum {
            let at = shoff + i * shentsize;
            raw.push((
                word(data, at, 4)? as u32,
                Section {
                    name: String::new(),
                    kind: word(data, at + 4, 4)? as u32,
                    flags: word(data, at + 8, 8)?,
                    offset: word(data, at + 0x18, 8)?,
                    size: word(data, at + 0x20, 8)?,
                    link: word(data, at + 0x28, 4)? as u32,
                    entsize: word(data, at + 0x38, 8)?,
                },
            ));
        }
        let names = raw.get(shstrndx).map(|(_, s)| (s.offset as usize, s.size as usize));
        let mut sections = Vec::with_capacity(shnum);
        for (n, mut s) in raw {
            if let Some((off, len)) = names {
                s.name = cstr(data.get(off..off + len)?, n as usize);
            }
            sections.push(s);
        }
        Some(Elf { data, sections })
    }

    /// The functions the symbol table defines: each one's name, size and
    /// whether its section holds code.
    fn functions(&self) -> Vec<(String, u64)> {
        let mut out = Vec::new();
        for s in self.sections.iter().filter(|s| s.kind == SHT_SYMTAB && s.entsize >= 24) {
            let Some(strtab) = self.sections.get(s.link as usize) else { continue };
            let (so, sl) = (strtab.offset as usize, strtab.size as usize);
            let Some(names) = self.data.get(so..so + sl) else { continue };
            let n = s.size / s.entsize;
            for i in 0..n {
                let at = (s.offset + i * s.entsize) as usize;
                let (Some(name), Some(info), Some(shndx), Some(size)) = (
                    word(self.data, at, 4),
                    word(self.data, at + 4, 1),
                    word(self.data, at + 6, 2),
                    word(self.data, at + 16, 8),
                ) else {
                    continue;
                };
                if (info as u8) & 0xf != STT_FUNC || shndx == 0 || shndx >= 0xff00 {
                    continue;
                }
                let code = self.sections.get(shndx as usize).is_some_and(|c| c.flags & SHF_EXECINSTR != 0);
                if code {
                    out.push((cstr(names, name as usize), size));
                }
            }
        }
        out
    }
}

fn cstr(table: &[u8], at: usize) -> String {
    let rest = table.get(at..).unwrap_or(&[]);
    let end = rest.iter().position(|b| *b == 0).unwrap_or(rest.len());
    String::from_utf8_lossy(&rest[..end]).into_owned()
}

/// The sizes of the parts of the executable `exe`, whose runtime in Sisu
/// is the object `rt`; `None` when either is not an ELF file of 64 bits.
pub(crate) fn sizes(exe: &[u8], rt: &[u8]) -> Option<Sizes> {
    let elf = Elf::read(exe)?;
    let rt_names: HashSet<String> = Elf::read(rt)?.functions().into_iter().map(|(n, _)| n).collect();
    let mut s = Sizes { total: exe.len() as u64, ..Sizes::default() };
    let mut code = 0;
    for sec in &elf.sections {
        if sec.kind == SHT_NOBITS {
            continue;
        }
        if sec.name.starts_with(".debug") {
            s.debug += sec.size;
        } else if sec.name == ".symtab" || sec.name == ".strtab" {
            s.symbols += sec.size;
        } else if sec.flags & SHF_EXECINSTR != 0 {
            code += sec.size;
        }
    }
    for (name, size) in elf.functions() {
        if rt_names.contains(&name) {
            s.rt_sisu += size;
        } else if name.starts_with("sisu_rt_") || name.contains("7sisu_rt") {
            s.rt_rust += size;
        } else if name.starts_with("sisu$") || name == "main" {
            s.program += size;
        }
    }
    s.other_code = code.saturating_sub(s.program + s.rt_sisu + s.rt_rust);
    s.rest = s.total.saturating_sub(code + s.debug + s.symbols);
    Some(s)
}
